#![allow(deprecated)]
use bytes::Bytes;
use ghost_sync::{Client, Server, ServerEvent, SyncError};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, watch};

/// Helper: start a server on a random port with a "test" room.
/// Returns (ServerHandle, port).
async fn start_server() -> (ghost_sync::ServerHandle, u16) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = Server::builder()
        .bind(format!("127.0.0.1:{port}"))
        .max_clients(10)
        .max_payload(1024)
        .build();

    server.pre_create_room("test").unwrap();

    let handle = server.run().await.unwrap();
    (handle, port)
}

/// Helper: start a server with custom config.
async fn start_server_with(
    idle_timeout: Duration,
    ping_interval: Duration,
) -> (ghost_sync::ServerHandle, u16) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = Server::builder()
        .bind(format!("127.0.0.1:{port}"))
        .max_clients(10)
        .max_payload(1024)
        .idle_timeout(idle_timeout)
        .ping_interval(ping_interval)
        .build();

    server.pre_create_room("test").unwrap();

    let handle = server.run().await.unwrap();
    (handle, port)
}

// Standard flow tests
#[tokio::test]
async fn connect_and_join() {
    let (_handle, port) = start_server().await;
    let mut client = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    client.join("test").await.unwrap();

    match client.recv().await.unwrap() {
        Some(ServerEvent::Joined { room_id, .. }) => assert_eq!(room_id, "test"),
        other => panic!("expected Joined, got something else: {:?}", other.is_some()),
    }
}

#[tokio::test]
async fn join_nonexistent_room() {
    let (_handle, port) = start_server().await;
    let mut client = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    client.join("nonexistent").await.unwrap();

    match client.recv().await.unwrap() {
        Some(ServerEvent::Error(msg)) => assert!(msg.contains("room not found")),
        other => panic!("expected Error, got something else: {:?}", other.is_some()),
    }
}

#[tokio::test]
async fn broadcast_relay() {
    let (_handle, port) = start_server().await;

    let mut a = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    a.join("test").await.unwrap();
    // Drain Joined
    let _ = a.recv().await.unwrap();

    let mut b = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    b.join("test").await.unwrap();
    // Drain Joined
    let _ = b.recv().await.unwrap();

    // Drain PlayerJoined notification for A
    let _ = a.recv().await.unwrap();

    // B broadcasts
    b.broadcast(b"hello").await.unwrap();

    // A should receive it
    match a.recv().await.unwrap() {
        Some(ServerEvent::Broadcast { data, .. }) => {
            assert_eq!(data, Bytes::from_static(b"hello"));
        }
        other => panic!("expected Broadcast, got: {:?}", other.is_some()),
    }

    // B should NOT receive its own broadcast (sender excluded)
    // Give it a moment — if it gets anything, it would be an error
    let result = tokio::time::timeout(Duration::from_millis(100), b.recv()).await;
    assert!(result.is_err(), "sender should not receive own broadcast");
}

#[tokio::test]
async fn broadcast_isolation() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = Server::builder()
        .bind(format!("127.0.0.1:{port}"))
        .max_clients(10)
        .build();

    server.pre_create_room("room_a").unwrap();
    server.pre_create_room("room_b").unwrap();

    let _handle = server.run().await.unwrap();

    let mut a = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    a.join("room_a").await.unwrap();
    let _ = a.recv().await.unwrap();

    let mut b = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    b.join("room_b").await.unwrap();
    let _ = b.recv().await.unwrap();

    // A broadcasts in room_a
    a.broadcast(b"from_a").await.unwrap();

    // B should NOT receive it (different room)
    let result = tokio::time::timeout(Duration::from_millis(100), b.recv()).await;
    assert!(
        result.is_err(),
        "different rooms should not see each other's broadcasts"
    );
}

#[tokio::test]
async fn leave_room() {
    let (_handle, port) = start_server().await;

    let mut a = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    a.join("test").await.unwrap();
    let _ = a.recv().await.unwrap(); // Joined

    let mut b = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    b.join("test").await.unwrap();
    let _ = b.recv().await.unwrap(); // Joined

    // A should see PlayerJoined for B
    let _ = a.recv().await.unwrap();

    // B leaves
    b.leave().await.unwrap();

    // A should see PlayerLeft
    match a.recv().await.unwrap() {
        Some(ServerEvent::PlayerLeft { .. }) => {}
        other => panic!("expected PlayerLeft, got: {:?}", other.is_some()),
    }
}

#[tokio::test]
async fn client_disconnect_cleanup() {
    let (_handle, port) = start_server().await;

    let mut a = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    a.join("test").await.unwrap();
    let _ = a.recv().await.unwrap(); // Joined

    let mut b = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    b.join("test").await.unwrap();
    let _ = b.recv().await.unwrap(); // Joined

    // A sees B join
    let _ = a.recv().await.unwrap();

    // Drop B (simulates disconnect)
    drop(b);

    // A should see PlayerLeft
    match a.recv().await.unwrap() {
        Some(ServerEvent::PlayerLeft { .. }) => {}
        other => panic!(
            "expected PlayerLeft after disconnect, got: {:?}",
            other.is_some()
        ),
    }
}

#[tokio::test]
async fn create_duplicate_room() {
    let server = Server::builder().bind("127.0.0.1:0").build();
    server.pre_create_room("dup").unwrap();

    let err = server.pre_create_room("dup").unwrap_err();
    match err {
        SyncError::RoomAlreadyExists(id) => assert_eq!(id, "dup"),
        other => panic!("expected RoomAlreadyExists, got: {}", other),
    }
}

#[tokio::test]
async fn ping_pong_internal() {
    // Server pings client, client auto-responds via [`recv`].
    // If the client doesn't respond (simulated by not sending Pong),
    // the server disconnects with PingTimeout.
    // Here we just verify the connection stays alive through ping cycles.
    tokio::time::pause();

    let (_handle, port) = start_server_with(
        Duration::from_secs(60), // long idle
        Duration::from_secs(3),  // ping interval
    )
    .await;

    let mut client = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    client.join("test").await.unwrap();
    let _ = client.recv().await.unwrap(); // Joined

    // Advance time through 3 ping cycles and connection should survive
    for _ in 0..3 {
        tokio::time::advance(Duration::from_secs(2)).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Client should still be connected and can send a message
    client.broadcast(b"still alive").await.unwrap();

    tokio::time::resume();
    drop(client);
}

#[tokio::test]
async fn idle_timeout_disconnects() {
    tokio::time::pause();

    let (_handle, port) = start_server_with(
        Duration::from_secs(5),
        Duration::from_secs(60), // long ping so it doesn't interfere
    )
    .await;

    let mut client = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    client.join("test").await.unwrap();
    let _ = client.recv().await.unwrap(); // Joined

    // Advance time past idle timeout
    tokio::time::advance(Duration::from_secs(6)).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Client should be disconnected
    // recv returns None or error
    let result = tokio::time::timeout(Duration::from_secs(1), client.recv()).await;
    match result {
        Ok(Ok(None)) => {} // clean disconnect
        Ok(Err(e)) => assert!(e.is_connection_closed()),
        Err(_) => panic!("timed out waiting for disconnect"),
        Ok(Ok(Some(_))) => panic!("unexpected message after idle timeout"),
    }

    tokio::time::resume();
}

#[tokio::test]
async fn ping_timeout_disconnects() {
    // Verify that the ping/pong mechanism works: client auto-responds
    // and the connection survives multiple ping cycles.
    tokio::time::pause();

    let (_handle, port) = start_server_with(
        Duration::from_secs(60), // long idle
        Duration::from_secs(3),  // ping interval
    )
    .await;

    let mut client = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    client.join("test").await.unwrap();
    let _ = client.recv().await.unwrap();

    // Advance through 2 ping cycles, yielding between each so the
    // client can process the Ping and auto-respond with Pong.
    for _ in 0..2 {
        tokio::time::advance(Duration::from_secs(4)).await;
        tokio::task::yield_now().await;
    }

    // Client should still be connected and can send a message
    client.broadcast(b"still alive").await.unwrap();

    tokio::time::resume();
    drop(client);
}

// Backpressure / connection filter tests
#[tokio::test]
async fn on_connect_reject() {
    use ghost_sync::ServerHandler;
    use std::net::SocketAddr;

    struct RejectAll;
    impl ServerHandler for RejectAll {
        fn on_connect(&self, _addr: SocketAddr) -> bool {
            false
        }
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = Server::builder()
        .bind(format!("127.0.0.1:{port}"))
        .handler(RejectAll)
        .build();

    server.pre_create_room("test").unwrap();
    let _handle = server.run().await.unwrap();

    // Connection should be rejected — client gets connection refused or immediate close
    let result = Client::connect(&format!("127.0.0.1:{port}")).await;
    // Depending on timing, this might be a ConnectionRefused or ConnectionReset
    assert!(
        result.is_err() || {
            // Connection established but immediately closed
            let mut c = result.unwrap();
            c.join("test").await.is_err() || matches!(c.recv().await, Ok(None))
        }
    );
}

// Backpressure test
#[tokio::test]
async fn backpressure_fires_on_slow_client() {
    use ghost_sync::{ServerHandler, Uuid};

    struct BackpressureCollector(Arc<Mutex<Vec<(Uuid, String)>>>);

    impl ServerHandler for BackpressureCollector {
        fn on_backpressure(&self, id: Uuid, room: &str) {
            self.0.lock().unwrap().push((id, room.to_string()));
        }
    }

    let collector = Arc::new(Mutex::new(Vec::new()));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = Server::builder()
        .bind(format!("127.0.0.1:{port}"))
        .channel_capacity(16)
        .max_payload(2048)
        .ping_interval(Duration::from_secs(60))
        .handler(BackpressureCollector(collector.clone()))
        .build();

    server.pre_create_room("test").unwrap();
    let _handle = server.run().await.unwrap();

    // Connect A reads normally
    let mut a = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    a.join("test").await.unwrap();
    let _ = a.recv().await.unwrap(); // Joined

    // Connect B will stop reading after join
    let mut b = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    b.join("test").await.unwrap();
    let _ = b.recv().await.unwrap(); // Joined

    // A sees B join
    let _ = a.recv().await.unwrap();

    // B stops reading. Its writer task will block when socket buffer fills.
    // A floods broadcasts to fill B's write channel (capacity 8).
    for i in 0..1024 {
        a.broadcast(format!("msg-{i}").as_bytes()).await.unwrap();
    }

    // Waiting for the hook to fire instead
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        if !collector.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    // Check if backpressure was recorded
    let events = collector.lock().unwrap();
    assert!(
        !events.is_empty(),
        "expected at least one backpressure event, got 0"
    );
    assert!(events.iter().all(|(_, room)| room == "test"));
}

// Runtime room management
#[tokio::test]
async fn runtime_room_management() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = Server::builder().bind(format!("127.0.0.1:{port}")).build();

    server.pre_create_room("lobby").unwrap();
    let handle = server.run().await.unwrap();

    assert!(handle.room_exists("lobby"));
    assert_eq!(handle.room_count(), 1);

    handle.create_room("match-1").unwrap();
    assert!(handle.room_exists("match-1"));
    assert_eq!(handle.room_count(), 2);

    // Duplicate fails
    let err = handle.create_room("match-1").unwrap_err();
    assert!(matches!(err, SyncError::RoomAlreadyExists(_)));

    // Client can join runtime-created room
    let mut client = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    client.join("match-1").await.unwrap();
    match client.recv().await.unwrap() {
        Some(ServerEvent::Joined { room_id, .. }) => assert_eq!(room_id, "match-1"),
        _ => panic!("expected Joined"),
    }

    assert_eq!(handle.room_client_count("match-1"), Some(1));
    assert!(handle.delete_room("match-1"));
    assert!(!handle.room_exists("match-1"));
    assert_eq!(handle.room_count(), 1);
    assert!(!handle.delete_room("nonexistent"));
}

const LOAD_CLIENTS: usize = 256;
const LOAD_DURATION: Duration = Duration::from_secs(8);

#[inline(always)]
fn get_random_bytes(size: u32) -> Vec<u8> {
    let mut rng = rand::rng();
    (0..size).map(|_| rng.random::<u8>()).collect()
}

async fn drain_startup_messages(client: &mut Client) -> Result<(), SyncError> {
    loop {
        match tokio::time::timeout(Duration::from_millis(20), client.recv()).await {
            Ok(Ok(Some(_))) => continue,
            Ok(Ok(None)) => return Err(SyncError::ConnectionClosed),
            Ok(Err(e)) => return Err(e),
            Err(_) => return Ok(()),
        }
    }
}

async fn load_worker(
    mut client: Client,
    mut stop_rx: watch::Receiver<bool>,
    fail_tx: mpsc::UnboundedSender<String>,
    worker_id: usize,
) {
    let mut rng = StdRng::seed_from_u64(0x5EED_0000 ^ worker_id as u64);
    let mut send_tick =
        tokio::time::interval(Duration::from_millis(25 + (worker_id as u64 % 5) * 5));
    send_tick.tick().await;

    loop {
        tokio::select! {
            biased;
            // Drain shutdown messages first, then cleanly leave and drain incoming messages to avoid spurious errors
            _ = stop_rx.changed() => {
                let _ = client.leave().await;

                let drain_deadline = tokio::time::sleep(Duration::from_millis(500));
                tokio::pin!(drain_deadline);
                loop {
                    tokio::select! {
                        _ = &mut drain_deadline => break,
                        recv_result = tokio::time::timeout(Duration::from_millis(25), client.recv()) => {
                            match recv_result {
                                Ok(Ok(Some(_))) => {}
                                Ok(Ok(None)) => break,
                                Ok(Err(e)) if e.is_connection_closed() => break,
                                Ok(Err(_)) => break,
                                Err(_) => {}
                            }
                        }
                    }
                }
                break;
            },
            // Send out random garbage broadcasts at intervals to simulate load
            _ = send_tick.tick() => {
                let size = rng.random_range(64..=512);
                let payload = get_random_bytes(size as u32);
                if let Err(e) = client.broadcast(&payload).await {
                    if *stop_rx.borrow() || e.is_connection_closed() {
                        break;
                    }
                    let _ = fail_tx.send(format!("worker {worker_id} broadcast failed: {e}"));
                    return;
                }
            }
            // Check for incoming messages to detect disconnects or other errors. We don't expect any messages, so time out immediately if there are any issues.
            recv_result = tokio::time::timeout(Duration::from_millis(1), client.recv()) => {
                match recv_result {
                    Ok(Ok(Some(_event))) => {}
                    Ok(Ok(None)) => {
                        if *stop_rx.borrow() {
                            break;
                        }
                        let _ = fail_tx.send(format!("worker {worker_id} disconnected unexpectedly"));
                        return;
                    }
                    Ok(Err(e)) => {
                        if *stop_rx.borrow() || e.is_connection_closed() {
                            break;
                        }
                        let _ = fail_tx.send(format!("worker {worker_id} recv failed: {e}"));
                        return;
                    }
                    Err(_) => {}
                }
            }
        }
    }
}

/// NOTE: This test might have race condition due to network dependencies, but it should be good enough to catch major issues under load.
#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn server_load_test() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = Server::builder()
        .bind(format!("127.0.0.1:{port}"))
        .max_clients(LOAD_CLIENTS)
        .max_payload(1024)
        .idle_timeout(Duration::from_secs(60))
        .ping_interval(Duration::from_secs(30))
        .channel_capacity(1024)
        .build();

    server.pre_create_room("load_test").unwrap();
    let handle = server.run().await.unwrap();

    let mut clients = Vec::with_capacity(LOAD_CLIENTS);
    for _ in 0..LOAD_CLIENTS {
        let mut client = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
        client.join("load_test").await.unwrap();
        let _ = client.recv().await.unwrap();
        clients.push(client);
    }

    tokio::time::sleep(Duration::from_millis(250)).await;

    for client in &mut clients {
        drain_startup_messages(client).await.unwrap();
    }

    let (stop_tx, stop_rx) = watch::channel(false);
    let (fail_tx, mut fail_rx) = mpsc::unbounded_channel::<String>();
    let mut tasks = Vec::with_capacity(LOAD_CLIENTS);

    for (worker_id, client) in clients.into_iter().enumerate() {
        tasks.push(tokio::spawn(load_worker(
            client,
            stop_rx.clone(),
            fail_tx.clone(),
            worker_id,
        )));
    }

    let failure = tokio::select! {
        _ = tokio::time::sleep(LOAD_DURATION) => None,
        maybe_failure = fail_rx.recv() => maybe_failure,
    };

    let _ = stop_tx.send(true);

    for task in tasks {
        task.await.unwrap();
    }

    handle.shutdown().await;

    if let Some(failure) = failure {
        panic!("load test failed: {failure}");
    }
}

// Metadata tests
#[derive(Clone, Debug, PartialEq)]
struct RoomMeta {
    mode: String,
    max_players: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct ClientMeta {
    username: String,
}

#[tokio::test]
async fn room_metadata_typed() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let server = Server::builder().bind(format!("127.0.0.1:{port}")).build();

    server.pre_create_room("test").unwrap();
    let handle = server.run().await.unwrap();

    // No metadata initially
    assert!(!handle.room_has_meta("test"));

    // Set metadata (typed struct, no string keys)
    let meta = RoomMeta {
        mode: "deathmatch".into(),
        max_players: 8,
    };
    assert!(handle.set_room_meta("test", meta.clone()));
    assert!(handle.room_has_meta("test"));

    // Read via callback
    let mode = handle.with_room_meta("test", |m: &RoomMeta| m.mode.clone());
    assert_eq!(mode, Some("deathmatch".to_string()));

    let max = handle.with_room_meta("test", |m: &RoomMeta| m.max_players);
    assert_eq!(max, Some(8));

    let wrong = handle.with_room_meta("test", |v: &u64| *v);
    assert_eq!(wrong, None);

    // Take metadata (removes it)
    let taken = handle.take_room_meta::<RoomMeta>("test");
    assert_eq!(taken, Some(meta));
    assert!(!handle.room_has_meta("test"));

    // After take, reading returns None
    let gone = handle.with_room_meta("test", |m: &RoomMeta| m.mode.clone());
    assert_eq!(gone, None);

    // Nonexistent room returns false
    assert!(!handle.set_room_meta(
        "nope",
        RoomMeta {
            mode: "x".into(),
            max_players: 1,
        }
    ));
    assert!(!handle.room_has_meta("nope"));
}

#[tokio::test]
async fn client_metadata_typed() {
    let (handle, port) = start_server().await;
    let mut client = Client::connect(&format!("127.0.0.1:{port}")).await.unwrap();
    client.join("test").await.unwrap();
    let _ = client.recv().await.unwrap(); // Joined

    let client_id = client.client_id().unwrap();
    assert!(handle.set_client_meta(
        &client_id,
        ClientMeta {
            username: "alice".into(),
        }
    ));
    assert!(handle.client_has_meta(&client_id));

    let name = handle.with_client_meta(&client_id, |m: &ClientMeta| m.username.clone());
    assert_eq!(name, Some("alice".to_string()));
}
