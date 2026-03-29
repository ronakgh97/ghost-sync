use anyhow::Result;
use clap::{Parser, Subcommand};
use dashmap::DashMap;
use ghost_sync::{info, warn, Server, ServerHandle, ServerHandler, Uuid};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::Duration;

struct Daemon {
    metrics: Metrics,
}

struct Metrics {
    start_time: chrono::DateTime<chrono::Local>,
    total_connections: AtomicUsize,
    // Key - ClientID, Value - Number of frames dropped
    backpressure_tracker: DashMap<Uuid, AtomicUsize>,
}

impl Metrics {
    pub fn uptime_time_hrs(&self) -> f64 {
        let now = chrono::Local::now();
        now.signed_duration_since(self.start_time).num_hours() as f64
    }
}

impl Daemon {
    fn new() -> Self {
        Self {
            metrics: Metrics {
                start_time: chrono::Local::now(),
                total_connections: AtomicUsize::new(0),
                backpressure_tracker: DashMap::with_capacity(1024),
            },
        }
    }
}

#[derive(Clone)]
struct DaemonHandler {
    daemon: Arc<Daemon>,
    server_handle: Arc<RwLock<Option<Arc<ServerHandle>>>>,
}

impl DaemonHandler {
    fn new(daemon: Arc<Daemon>) -> Self {
        Self {
            daemon,
            server_handle: Arc::new(RwLock::new(None)),
        }
    }

    #[inline]
    fn set_server_handle(&self, handle: Arc<ServerHandle>) {
        *self.server_handle.write().unwrap() = Some(handle);
    }

    #[inline]
    fn handle(&self) -> Arc<ServerHandle> {
        self.server_handle
            .read()
            .unwrap()
            .as_ref()
            .expect("ServerHandle not initialized")
            .clone()
    }
}

#[derive(Debug, Clone, Default)]
struct RoomMeta {
    password: Option<String>,
    max_player: Option<usize>,
}

impl ServerHandler for DaemonHandler {
    fn on_connect(&self, addr: SocketAddr) -> bool {
        let count = self
            .daemon
            .metrics
            .total_connections
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        info!("New connection from {} (total: {})", addr, count);
        true
    }

    fn on_join(
        &self,
        client_id: Uuid,
        room_id: &str,
        addr: SocketAddr,
        data: &[u8],
    ) -> (bool, Option<String>) {
        let handle = self.handle();

        // Check room metadata for password and max_player
        let join_result = handle.with_room_meta::<RoomMeta, Result<(), String>>(room_id, |meta| {
            // Check password
            if let Some(expected_password) = &meta.password {
                let provided_password = String::from_utf8_lossy(data);
                if provided_password != *expected_password {
                    warn!(
                        "Client {} ({}) failed to join room '{}' - wrong password",
                        client_id, addr, room_id
                    );
                    return Err("Wrong password".to_string());
                }
            }

            // Check max_player
            if let Some(max) = meta.max_player {
                let current_count = handle.room_client_count(room_id).unwrap_or(0);
                if current_count >= max {
                    warn!(
                        "Client {} ({}) failed to join room '{}' - room full ({}/{})",
                        client_id, addr, room_id, current_count, max
                    );
                    return Err(format!("Room is full ({}/{})", current_count, max));
                }
            }

            Ok(())
        });

        match join_result {
            Some(Ok(())) => {
                info!("Client {} ({}) joined room '{}'", client_id, addr, room_id);
                (true, None)
            }
            Some(Err(msg)) => (false, Some(msg)),
            None => {
                // Room doesn't exist or no metadata
                info!("Client {} ({}) joined room '{}'", client_id, addr, room_id);
                (true, None)
            }
        }
    }

    fn on_leave(&self, client_id: Uuid, room_id: &str) {
        info!("Client {} left room '{}'", client_id, room_id);
    }

    fn on_room_create(&self, room_id: &str) {
        // Set default empty metadata
        self.handle().set_room_meta(room_id, RoomMeta::default());
        info!("Room created: '{}'", room_id);
    }

    fn on_room_delete(&self, room_id: &str) {
        // Metadata is automatically cleaned up internally
        info!("Room deleted: '{}'", room_id);
    }

    fn on_backpressure(&self, client_id: Uuid, room_id: &str) {
        warn!(
            "Backpressure: Client {} in room '{}' has a full write channel, frame dropped",
            client_id, room_id
        );
        let tracker = &self.daemon.metrics.backpressure_tracker;
        _ = tracker
            .entry(client_id)
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::Relaxed)
            + 1;
    }
}

#[derive(Debug)]
struct ControlCmd {
    name: String,
    args: Vec<String>,
    // TODO: Add binary payload
}

impl ControlCmd {
    #[inline(always)]
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

    #[inline(always)]
    fn require_arg(&self, index: usize) -> Result<&str> {
        self.args
            .get(index)
            .map(|s| s.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Missing argument at position {}", index))
    }
}

#[inline(always)]
async fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let len = stream.read_u32().await? as usize;
    if len > 1024 * 1024 {
        return Err(anyhow::anyhow!("Frame too large: {} bytes", len));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

#[inline(always)]
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
                ));
            }
            Err(e) => warn!("Failed to accept control connection: {}", e),
        }
    }
}

#[inline]
async fn handle_control_connection(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    daemon: Arc<Daemon>,
    server_handle: Arc<ServerHandle>,
) {
    loop {
        let frame = match read_frame(&mut stream).await {
            Ok(f) => f,
            Err(e) => {
                let error_str = e.to_string();
                // Only warn on non-EOF errors (EOF is graceful disconnect)
                if !error_str.contains("EOF")
                    && !error_str.contains("early eof")
                    && !error_str.contains("unexpected end of file")
                {
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

        let response = handle_command(&cmd, &daemon, &server_handle, peer_addr);

        if write_frame(&mut stream, response.as_bytes()).await.is_err() {
            break;
        }
    }
}
/// API Ref:
///
/// ECHO_TEST -> {
/// OK
/// {echo_payload}
/// }
///
/// CREATE_ROOM -> {
/// Request:
///   {room-id}
///   {password}     <-- `optional - empty for public`
///   {max_player}   <-- `optional - empty for no limit`
/// Response:
/// OK
/// {room-id}
/// {private}        <-- `if password was provided, empty (no \n) otherwise`
/// {max_player}     <-- `if max_player was set, no \n if empty`
/// }
///
/// DELETE_ROOM -> {
/// OK
/// {room-id}
/// }
///
/// LIST_ROOMS -> {
/// OK
/// {room-id},{client-count},{private} <-- `empty after comma if public`
/// ...
/// }
///
/// ROOM_CLIENTS -> {
/// OK
/// {room-id}
/// {client-id}
/// ...
/// }
///
/// KICK_CLIENT -> {
/// OK
/// {client-id}
/// }
///
/// METRICS -> {
/// OK
/// {total-connections}
/// {uptime-hrs}
/// {total-room}
/// {total-clients}
///
/// {client-id},{channel-queue-len}
/// {client-id},{channel-queue-len}
/// ...
/// }
///
#[inline]
fn handle_command(
    cmd: &ControlCmd,
    daemon: &Daemon,
    handle: &ServerHandle,
    _peer_addr: SocketAddr,
) -> String {
    match cmd.name.as_str() {
        "ECHO_TEST" => match cmd.require_arg(0) {
            Ok(echo_payload) => {
                format!("OK\n{}", echo_payload)
            }
            Err(e) => {
                format!("ERROR\n{}", e)
            }
        },

        "CREATE_ROOM" => match cmd.require_arg(0) {
            Ok(room_id) => {
                // Optional args password (arg 1), max_player (arg 2)
                let password = cmd
                    .args
                    .get(1)
                    .map(|s| s.as_str())
                    .filter(|s| !s.is_empty());
                let max_player = cmd.args.get(2).and_then(|s| s.trim().parse::<usize>().ok());

                let meta = RoomMeta {
                    password: password.map(|p| p.to_string()),
                    max_player,
                };

                match handle.create_room(room_id) {
                    Ok(()) => {
                        // Set room metadata via ServerHandle
                        handle.set_room_meta(room_id, meta);

                        let mut response = format!("OK\n{}", room_id);
                        if password.is_some() {
                            response.push_str("\nprivate");
                        }
                        if let Some(mp) = max_player {
                            response.push_str(&format!("\nmax_player:{}", mp));
                        }
                        response
                    }
                    Err(e) => format!("ERROR\n{}", e),
                }
            }
            Err(e) => format!("ERROR\n{}", e),
        },

        "DELETE_ROOM" => match cmd.require_arg(0) {
            Ok(room_id) => {
                if handle.delete_room(room_id) {
                    format!("OK\n{}", room_id)
                } else {
                    format!("ERROR\nRoom '{}' not found", room_id)
                }
            }
            Err(e) => format!("ERROR\n{}", e),
        },

        "LIST_ROOMS" => {
            let mut response = String::from("OK\n");
            for room_id in handle.get_room_ids() {
                let clients = handle.room_client_count(&room_id).unwrap_or(0);
                let is_private = handle
                    .with_room_meta::<RoomMeta, bool>(&room_id, |m| m.password.is_some())
                    .unwrap_or(false);
                if is_private {
                    response.push_str(&format!("{},{},private\n", room_id, clients));
                } else {
                    response.push_str(&format!("{},{}\n", room_id, clients));
                }
            }
            response
        }

        "ROOM_CLIENTS" => match cmd.require_arg(0) {
            Ok(room_id) => match handle.get_room_clients(room_id) {
                Some(clients) => {
                    let mut response = format!("OK\n{}\n", room_id);
                    for client in clients {
                        response.push_str(&format!("{}\n", client));
                    }
                    response
                }
                None => format!("ERROR\nRoom '{}' not found", room_id),
            },
            Err(e) => format!("ERROR\n{}", e),
        },

        "KICK_CLIENT" => match cmd.require_arg(0) {
            Ok(client_id_str) => match Uuid::parse_str(client_id_str) {
                Ok(client_id) => {
                    if handle.kick_client(&client_id) {
                        format!("OK\n{}", client_id)
                    } else {
                        "ERROR\nClient not found or not in a room".to_string()
                    }
                }
                Err(_) => "ERROR\nInvalid client ID format".to_string(),
            },
            Err(e) => format!("ERROR\n{}", e),
        },

        "METRICS" => {
            let mut response = format!(
                "OK\n{}\n{}\n{}\n{}\n",
                daemon.metrics.total_connections.load(Ordering::Relaxed),
                daemon.metrics.uptime_time_hrs(),
                handle.room_count(),
                handle.get_client_count(),
            );

            // Show channel queue depths for all clients
            for room_id in handle.get_room_ids() {
                if let Some(lens) = handle.get_room_channel_lens(&room_id) {
                    if !lens.is_empty() {
                        response.push('\n');
                        for (id, len) in lens {
                            response.push_str(&format!("{},{}\n", &id.to_string(), len));
                        }
                    }
                }
            }

            response
        }

        _ => format!("ERROR\nUnknown command: {}", cmd.name),
    }
}

// TODO: More daemon feature...
// TODO: Use Admin token to manage access control for control API
#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() -> Result<()> {
    let args = Cliargs::parse();
    match args.command {
        Some(Command::Run {
            game_addr,
            api_addr,
            channel_capacity,
            max_client,
        }) => {
            let game_server_ip = game_addr.unwrap_or_else(|| "0.0.0.0:7777".to_string());
            let control_addr = match api_addr {
                Some(control_addr) => control_addr.parse::<SocketAddr>()?,
                None => SocketAddr::from(([0, 0, 0, 0], 7878)),
            };
            let max_client = max_client.unwrap_or(1024);
            let channel_capacity = channel_capacity.unwrap_or(256 * 1024 * 1024);

            info!("Starting Game Server Daemon");

            let daemon = Arc::new(Daemon::new());
            let handler = DaemonHandler::new(daemon.clone());

            let tcp_server = Server::builder()
                .bind(&game_server_ip)
                .max_clients(max_client)
                .max_payload(256 * 1024)
                .idle_timeout(Duration::from_secs(25))
                .ping_interval(Duration::from_secs(15))
                .channel_capacity(channel_capacity)
                .handler(handler.clone())
                .build();

            let gameserver_handle = Arc::new(tcp_server.run().await?);
            info!("Game relay server listening on {}", &game_server_ip);

            // Set the server handle on the handler for room metadata access
            handler.set_server_handle(gameserver_handle.clone());

            tokio::spawn(cleanup_job(gameserver_handle.clone()));

            let ctrl_listener = TcpListener::bind(control_addr).await?;
            info!("Control server listening on {}", control_addr);

            let ctrl_handle = tokio::spawn(run_control_server(
                ctrl_listener,
                daemon.clone(),
                gameserver_handle.clone(),
            ));

            gameserver_handle.create_room("test-room")?;
            let test_meta = RoomMeta {
                password: None,
                max_player: None,
            };
            gameserver_handle.set_room_meta("test-room", test_meta);

            info!("Game Server Daemon is running. Press Ctrl+C to stop...");

            shutdown_signal().await;
            gameserver_handle.shutdown().await;
            ctrl_handle.abort();
            info!("Game Server Daemon stopped.");
        }
        None => {
            ascii().await;
        }
    };

    Ok(())
}

async fn cleanup_job(handle: Arc<ServerHandle>) -> Result<()> {
    loop {
        tokio::time::sleep(Duration::from_mins(12)).await;
        let mut empty_room = vec![];
        {
            for room_id in handle.get_room_ids() {
                if let Some(clients) = handle.get_room_clients(&room_id) {
                    if clients.is_empty() {
                        empty_room.push(room_id);
                    }
                }
            }
        }
        {
            for id in empty_room {
                handle.delete_room(&id);
            }
        }
    }
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

async fn ascii() {
    let r = "
      ▄▄                                                       ▄▄
      ██                 ██                                    ██
▄████ ████▄ ▄███▄ ▄█▀▀▀ ▀██▀▀ ▄█▀▀▀ ██ ██ ████▄ ▄████       ▄████  ▀▀█▄ ▄█▀█▄ ███▄███▄ ▄███▄ ████▄
██ ██ ██ ██ ██ ██ ▀███▄  ██   ▀███▄ ██▄██ ██ ██ ██    ▀▀▀▀▀ ██ ██ ▄█▀██ ██▄█▀ ██ ██ ██ ██ ██ ██ ██
▀████ ██ ██ ▀███▀ ▄▄▄█▀  ██   ▄▄▄█▀  ▀██▀ ██ ██ ▀████       ▀████ ▀█▄██ ▀█▄▄▄ ██ ██ ██ ▀███▀ ██ ██
   ██                                 ██
 ▀▀▀                                ▀▀▀
";
    println!("{}", r);
}

#[derive(Parser)]
#[command(
    name = "gs-daemon",
    version = "1.0.0-beta",
    about = "Generic Game Server Daemon using `ghost_sync`",
    long_about = None
)]
struct Cliargs {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run daemon server
    Run {
        /// Addr to bind inner game server to
        #[clap(short, long)]
        game_addr: Option<String>,

        /// Addr to bind control layer to
        #[clap(short, long)]
        api_addr: Option<String>,

        #[clap(short, long)]
        max_client: Option<usize>,

        /// Channel bandwidth limit for each client (in bytes). Default is 256MB.
        #[clap(short, long)]
        channel_capacity: Option<usize>,
    },
}
