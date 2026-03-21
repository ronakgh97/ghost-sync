use std::time::Duration;
use uuid::Uuid;
use wincode::{SchemaRead, SchemaWrite};

/// Messages clients send to the server.
#[derive(SchemaWrite, SchemaRead)]
pub enum ClientWire {
    JoinRoom { room_id: String },
    LeaveRoom,
    Ping,
    Pong,
    Broadcast { data: Vec<u8> },
}

/// Messages the server sends to clients.
#[derive(SchemaWrite, SchemaRead)]
pub enum ServerWire {
    Joined { client_id: Uuid, room_id: String },
    PlayerJoined { client_id: Uuid },
    PlayerLeft { client_id: Uuid },
    Ping,
    Pong,
    Error(String),
    Broadcast { sender_id: Uuid, data: Vec<u8> },
}

#[allow(rustdoc::private_intra_doc_links)]
/// Client-facing event (clean API over [`ServerWire`])
/// Events the client receives from the server.
pub enum ServerEvent {
    /// Room join confirmed.
    Joined { client_id: Uuid, room_id: String },
    /// Another player joined your room.
    PlayerJoined { client_id: Uuid },
    /// Another player left your room.
    PlayerLeft { client_id: Uuid },
    /// Server error.
    Error(String),
    /// Relayed data from a peer.
    Broadcast { sender_id: Uuid, data: Vec<u8> },
}

/// Server configuration.
#[derive(Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub max_clients: usize,
    pub max_payload: usize,
    /// Disconnect clients that send nothing for this duration.
    pub idle_timeout: Duration,
    /// How often the server pings clients. If no Pong arrives before the
    /// next ping tick, the client is disconnected with [`SyncError::PingTimeout`].
    pub ping_interval: Duration,
    /// Per-client write channel capacity. Frames are dropped when the channel
    /// is full (with `on_backpressure` hook). Higher values buffer more for
    /// bursty games; lower values keep latency tight.
    pub channel_capacity: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:7777".into(),
            max_clients: 1024,
            max_payload: 64 * 1024,
            // Non-divisible defaults to avoid idle_timeout and ping_interval
            idle_timeout: Duration::from_secs(31),
            ping_interval: Duration::from_secs(13),
            channel_capacity: 64,
        }
    }
}

// Errors
pub enum SyncError {
    PayloadTooLarge { size: usize, max: usize },
    IdleTimeout,
    PingTimeout,
    RoomNotFound,
    RoomAlreadyExists(String),
    NotInRoom,
    MaxClientsReached,
    ConnectionClosed,
    ConnectionRefused,
    Protocol(String),
    Io(std::io::Error),
}

impl SyncError {
    /// Returns true if this is a "normal" connection close (not an actual error).
    /// This includes EOF, connection reset, broken pipe, and connection aborted.
    pub fn is_connection_closed(&self) -> bool {
        match self {
            Self::ConnectionClosed => true,
            Self::Io(e) => is_graceful_io_error(e),
            _ => false,
        }
    }
}

/// Returns true if an IO error represents a normal connection close.
///
/// Detects:
/// - `UnexpectedEof` — remote closed cleanly
/// - `ConnectionReset` — remote forcibly closed (10054 Windows / ECONNRESET Unix)
/// - `ConnectionAborted` — connection aborted (10053 Windows / ECONNABORTED Unix)
/// - `BrokenPipe` — broken pipe (EPIPE Unix)
/// - Any IO error with OS error codes matching the above
pub(crate) fn is_graceful_io_error(e: &std::io::Error) -> bool {
    match e.kind() {
        std::io::ErrorKind::UnexpectedEof
        | std::io::ErrorKind::ConnectionReset
        | std::io::ErrorKind::ConnectionAborted
        | std::io::ErrorKind::BrokenPipe => true,
        _ => {
            // Also check raw OS error codes for cross-platform coverage
            matches!(
                e.raw_os_error(),
                Some(
                    10054 | // Windows WSAECONNRESET
                    10053 | // Windows WSAECONNABORTED
                    10058 // Windows WSAESHUTDOWN
                )
            )
        }
    }
}

impl std::fmt::Debug for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadTooLarge { size, max } => {
                write!(f, "payload too large: {} bytes (max {})", size, max)
            }
            Self::IdleTimeout => write!(f, "client idle timeout"),
            Self::PingTimeout => write!(f, "no pong received before next ping tick"),
            Self::RoomNotFound => write!(f, "room not found"),
            Self::RoomAlreadyExists(id) => write!(f, "room already exists: {id}"),
            Self::NotInRoom => write!(f, "client not in a room"),
            Self::MaxClientsReached => write!(f, "max clients reached"),
            Self::ConnectionClosed => write!(f, "connection closed"),
            Self::ConnectionRefused => write!(f, "connection refused — is the server running?"),
            Self::Protocol(msg) => write!(f, "protocol error: {}", msg),
            Self::Io(e) => write!(f, "io error: {}", e),
        }
    }
}

impl std::error::Error for SyncError {}

impl From<std::io::Error> for SyncError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, SyncError>;
