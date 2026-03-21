use std::net::SocketAddr;
use uuid::Uuid;

/// Optional hook trait for server lifecycle events.
/// All methods have no-op defaults — implement only what you need.
///
/// # Composability
///
/// These hooks are the extension point for game logic. The library never
/// interprets room metadata or makes game-specific decisions. Use the hooks
/// to implement private rooms, max player limits, matchmaking, bans, etc.
///
/// # Security
///
/// Use [`on_connect`](ServerHandler::on_connect) to filter connections by address
/// before they're fully accepted.
///
/// # Backpressure
///
/// When a client's write channel is full, the frame is dropped and
/// [`on_backpressure`](ServerHandler::on_backpressure) is called. This is
/// intentional for real-time game relay — a slow client shouldn't stall the server.
pub trait ServerHandler: Send + Sync + 'static {
    /// Called when a new TCP connection arrives. Return `false` to reject it.
    /// Called before any resources are allocated for the connection.
    fn on_connect(&self, _addr: SocketAddr) -> bool {
        true
    }

    /// Called when a client requests to join a room. Return `false` to reject
    /// the join - the client receives a "join rejected" error.
    ///
    /// Use this to implement private rooms (check password), max player limits
    /// (check client count), bans (check addr), add room_limit or any custom join logic you wish
    fn on_join(&self, _client_id: Uuid, _room_id: &str, _addr: SocketAddr) -> bool {
        true
    }

    /// Called when a client leaves a room (or disconnects).
    fn on_leave(&self, _client_id: Uuid, _room_id: &str) {}

    /// Called when a broadcast message is relayed.
    /// client_id is the sender, and the message has already been relayed to all peers.
    fn on_broadcast(&self, _client_id: Uuid, _room_id: &str, _data: &[u8]) {}

    /// Called when a frame is dropped because a client's write channel is full.
    /// The affected clients are identified by their UUIDs.
    fn on_backpressure(&self, _client_id: Uuid, _room_id: &str) {}

    /// Called after a room is created (including via [`ServerHandle::create_room_runtime`](crate::ServerHandle::create_room)).
    fn on_room_create(&self, _room_id: &str) {}

    /// Called after a room is deleted.
    fn on_room_delete(&self, _room_id: &str) {}
}

/// Default no-op handler. Accepts all connections, ignores all events.
pub struct NoopHandler;

impl ServerHandler for NoopHandler {}
