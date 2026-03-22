use anyhow::Result;
use ghost_sync::{info, warn, Server, ServerHandle, ServerHandler, Uuid};
use std::env;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;
use tokio::time::Duration;

struct Daemon {
    metrics: Metrics,
}

struct Metrics {
    total_connections: AtomicUsize,
    start_time: Instant,
}

impl Daemon {
    fn new() -> Self {
        Self {
            metrics: Metrics {
                total_connections: AtomicUsize::new(0),
                start_time: Instant::now(),
            },
        }
    }
}

struct DaemonHandler(Arc<Daemon>);

impl DaemonHandler {
    fn new(demon: Arc<Daemon>) -> Self {
        Self(demon)
    }
}

impl ServerHandler for DaemonHandler {
    fn on_connect(&self, addr: SocketAddr) -> bool {
        let count = self
            .0
            .metrics
            .total_connections
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        info!("New connection from {} (total: {})", addr, count);
        true
    }

    fn on_join(&self, client_id: Uuid, room_id: &str, addr: SocketAddr) -> bool {
        info!("Client {} ({}) joined room '{}'", client_id, addr, room_id);
        true
    }

    fn on_leave(&self, client_id: Uuid, room_id: &str) {
        info!("Client {} left room '{}'", client_id, room_id);
    }

    fn on_backpressure(&self, client_id: Uuid, room_id: &str) {
        warn!(
            "Backpressure: client {} in room '{}' has a full write channel, frame dropped",
            client_id, room_id
        );
    }

    fn on_room_create(&self, room_id: &str) {
        info!("Room created: '{}'", room_id);
    }

    fn on_room_delete(&self, room_id: &str) {
        info!("Room deleted: '{}'", room_id);
    }
}

#[derive(Debug)]
struct ControlCmd {
    name: String,
    args: Vec<String>,
}

impl ControlCmd {
    fn parse(data: &[u8]) -> Result<Self> {
        let s = String::from_utf8(data.to_vec()).map_err(|_| anyhow::anyhow!("Invalid UTF-8"))?;

        let mut parts: Vec<&str> = s.split('\n').collect();

        // Remove trailing empty from trailing \n
        if parts.last().is_some_and(|s| s.is_empty()) {
            parts.pop();
        }

        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty command"));
        }

        Ok(ControlCmd {
            name: parts[0].trim().to_uppercase(),
            args: parts[1..].iter().map(|s| s.trim().to_string()).collect(),
        })
    }

    fn require_arg(&self, index: usize) -> Result<&str> {
        self.args
            .get(index)
            .map(|s| s.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Missing argument at position {}", index))
    }
}

async fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let len = stream.read_u32().await? as usize;
    if len > 1024 * 1024 {
        return Err(anyhow::anyhow!("Frame too large: {} bytes", len));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn write_frame(stream: &mut TcpStream, data: &[u8]) -> Result<()> {
    stream.write_u32(data.len() as u32).await?;
    stream.write_all(data).await?;
    stream.flush().await?;
    Ok(())
}

async fn run_control_server(
    listener: TcpListener,
    daemon: Arc<Daemon>,
    server_handle: Arc<ServerHandle>,
    shutdown_token: String,
    shutdown_notify: Arc<Notify>,
) {
    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                stream.set_nodelay(true).ok();
                tokio::spawn(handle_control_connection(
                    stream,
                    peer_addr,
                    daemon.clone(),
                    server_handle.clone(),
                    shutdown_token.clone(),
                    shutdown_notify.clone(),
                ));
            }
            Err(e) => warn!("Failed to accept control connection: {}", e),
        }
    }
}

async fn handle_control_connection(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    daemon: Arc<Daemon>,
    server_handle: Arc<ServerHandle>,
    shutdown_token: String,
    shutdown_notify: Arc<Notify>,
) {
    loop {
        let frame = match read_frame(&mut stream).await {
            Ok(f) => f,
            Err(e) => {
                if !e.to_string().contains("EOF") && !e.to_string().contains("early eof") {
                    warn!("Control read error from {}: {}", peer_addr, e);
                }
                break;
            }
        };

        let cmd = match ControlCmd::parse(&frame) {
            Ok(c) => c,
            Err(e) => {
                write_frame(&mut stream, format!("ERROR\n{}", e).as_bytes())
                    .await
                    .ok();
                continue;
            }
        };

        let response = handle_command(&cmd, &daemon, &server_handle, &shutdown_token, peer_addr);

        if write_frame(&mut stream, response.as_bytes()).await.is_err() {
            break;
        }

        // Check for shutdown
        if cmd.name == "SHUTDOWN" && cmd.require_arg(0).is_ok_and(|t| t == shutdown_token) {
            info!("Shutdown requested via control from {}", peer_addr);
            shutdown_notify.notify_one();
            return;
        }
    }
}

fn handle_command(
    cmd: &ControlCmd,
    daemon: &Daemon,
    handle: &ServerHandle,
    token: &str,
    peer_addr: SocketAddr,
) -> String {
    match cmd.name.as_str() {
        "LIST_ROOMS" => {
            let mut response = String::from("OK\n");
            for room_id in handle.get_room_ids() {
                let clients = handle.room_client_count(&room_id).unwrap_or(0);
                response.push_str(&format!("{}: {} clients\n", room_id, clients));
            }
            response
        }

        "CREATE_ROOM" => match cmd.require_arg(0) {
            Ok(room_id) => match handle.create_room(room_id) {
                Ok(()) => format!("OK\nRoom '{}' created", room_id),
                Err(e) => format!("ERROR\n{}", e),
            },
            Err(e) => format!("ERROR\n{}", e),
        },

        "DELETE_ROOM" => match cmd.require_arg(0) {
            Ok(room_id) => {
                if handle.delete_room(room_id) {
                    format!("OK\nRoom '{}' deleted", room_id)
                } else {
                    format!("ERROR\nRoom '{}' not found", room_id)
                }
            }
            Err(e) => format!("ERROR\n{}", e),
        },

        "ROOM_CLIENTS" => match cmd.require_arg(0) {
            Ok(room_id) => match handle.get_room_clients(room_id) {
                Some(clients) => {
                    let mut response = format!("OK\nClients in '{}':\n", room_id);
                    for client in clients {
                        response.push_str(&format!("{}\n", client));
                    }
                    response
                }
                None => format!("ERROR\nRoom '{}' not found", room_id),
            },
            Err(e) => format!("ERROR\n{}", e),
        },

        "METRICS" => format!(
            "OK\nconnections={}\nrooms={}\nclients={}\nuptime={}s",
            daemon.metrics.total_connections.load(Ordering::Relaxed),
            handle.room_count(),
            handle.get_client_count(),
            daemon.metrics.start_time.elapsed().as_secs()
        ),

        "SHUTDOWN" => match cmd.require_arg(0) {
            Ok(arg) if arg == token => format!("OK\nShutting down from {}", peer_addr),
            Ok(_) => "ERROR\nInvalid token".to_string(),
            Err(e) => format!("ERROR\n{}", e),
        },

        _ => format!("ERROR\nUnknown command: {}", cmd.name),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let shutdown_token =
        env::var("GHOST_SYNC_TOKEN").unwrap_or_else(|_| Uuid::new_v4().to_string());

    info!("Starting Game Server Daemon");
    info!("Shutdown token: {}", shutdown_token);

    let daemon = Arc::new(Daemon::new());
    let shutdown_notify = Arc::new(Notify::new());

    let tcp_server = Server::builder()
        .bind("0.0.0.0:7777")
        .max_clients(1000)
        .max_payload(64 * 1024)
        .idle_timeout(Duration::from_secs(30))
        .ping_interval(Duration::from_secs(10))
        .channel_capacity(128)
        .handler(DaemonHandler::new(daemon.clone()))
        .build();

    let server_handle = Arc::new(tcp_server.run().await?);
    info!("TCP relay server listening on 0.0.0.0:7777");

    for i in 1..=10 {
        server_handle.create_room(&format!("room-{}", i))?;
    }

    let ctrl_listener = TcpListener::bind("0.0.0.0:8888").await?;
    info!("Control server listening on 0.0.0.0:8888");

    let ctrl_handle = tokio::spawn(run_control_server(
        ctrl_listener,
        daemon,
        server_handle.clone(),
        shutdown_token,
        shutdown_notify.clone(),
    ));

    // Wait for either Ctrl+C or control shutdown command
    info!("Daemon ready. Press Ctrl+C to stop.");
    tokio::select! {
        _ = shutdown_signal() => {
            info!("Ctrl+C received. Shutting down...");
        }
        _ = shutdown_notify.notified() => {
            info!("Shutdown command received. Shutting down...");
        }
    }

    server_handle.shutdown().await;
    ctrl_handle.abort();
    info!("Game Server Daemon stopped.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
