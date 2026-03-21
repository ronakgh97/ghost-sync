use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use dashmap::DashMap;
use tokio::io::BufWriter;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::handler::{NoopHandler, ServerHandler};
use crate::protocol;
use crate::room::RoomManager;
use crate::types::{ClientWire, Result, ServerConfig, ServerWire, SyncError};
use crate::{error, info, warn};

/// Tracks per-client state on the server side.
struct ClientState {
    room_id: Option<String>,
    addr: SocketAddr,
}

/// A broadcast relay game server.
///
/// Create a server with [`Server::builder`], optionally create rooms with
/// [`Server::pre_create_room`], then call [`Server::run`] to start accepting connections
/// or use [`ServerHandle::create_room`] to perform at runtime, you create a daemon wrapper this way :)
///
/// ```no_run
/// # async fn example() -> std::io::Result<()> {
/// use ghost_sync::Server;
///
/// let server = Server::builder()
///     .bind("0.0.0.0:7777")
///     .build();
///
/// // Spin up the server with an initial rooms (This is not standard, but you have freedom)
/// server.pre_create_room("standoff").unwrap();
///
/// let handle = server.run().await?;
/// // Rooms can also be managed at runtime via the handle:
/// // handle.create_room_runtime("match-001").unwrap();
/// # Ok(())
/// # }
/// ```
pub struct Server {
    config: ServerConfig,
    handler: Arc<dyn ServerHandler>,
    rooms: Arc<RoomManager>,
    clients: Arc<DashMap<Uuid, ClientState>>,
    client_count: Arc<std::sync::atomic::AtomicUsize>,
    shutdown_tx: broadcast::Sender<()>,
}

/// Handle to a running server.
///
/// Provides runtime control: shutdown, room management and state queries
/// Rooms can be created and deleted at any time, new clients will see the
/// updated room list, and existing clients in deleted rooms get errors on
/// their next broadcast or join attempt.
pub struct ServerHandle {
    shutdown_tx: broadcast::Sender<()>,
    rooms: Arc<RoomManager>,
    clients: Arc<DashMap<Uuid, ClientState>>,
    handler: Arc<dyn ServerHandler>,
    client_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl ServerHandle {
    /// Signal the server to stop accepting connections and disconnect all clients.
    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// Create a room at runtime. Fails if the room already exists.
    #[inline(always)]
    pub fn create_room(&self, id: &str) -> Result<()> {
        self.rooms.create(id)?;
        self.handler.on_room_create(id);
        Ok(())
    }

    /// Delete a room at runtime. Returns `true` if the room existed.
    ///
    /// Soft delete: connected clients are not kicked, but their next
    /// broadcast or join will fail with a [`SyncError::RoomNotFound`].
    #[inline(always)]
    pub fn delete_room(&self, id: &str) -> bool {
        let existed = self.rooms.delete(id);
        if existed {
            self.handler.on_room_delete(id);
        }
        existed
    }

    /// Check if a room exists.
    pub fn room_exists(&self, id: &str) -> bool {
        self.rooms.get(id).is_some()
    }

    /// Number of active rooms.
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    /// List all room IDs.
    pub fn get_room_ids(&self) -> Vec<String> {
        self.rooms.room_ids()
    }

    /// Client IDs in a room. Returns `None` if the room doesn't exist.
    pub fn get_room_clients(&self, id: &str) -> Option<Vec<Uuid>> {
        self.rooms.get(id).map(|r| r.client_ids())
    }

    /// Number of clients in a room. Returns `None` if the room doesn't exist.
    pub fn room_client_count(&self, id: &str) -> Option<usize> {
        self.rooms.get(id).map(|r| r.len())
    }

    /// Get a metadata value from a room.
    pub fn get_room_meta(&self, room_id: &str, key: &str) -> Option<String> {
        self.rooms.get(room_id)?.get_meta(key)
    }

    /// Set a metadata value on a room. Returns `false` if the room doesn't exist.
    pub fn set_room_meta(&self, room_id: &str, key: &str, value: &str) -> bool {
        if let Some(room) = self.rooms.get(room_id) {
            room.set_meta(key, value);
            true
        } else {
            false
        }
    }

    /// Get all metadata from a room as a HashMap. Returns `None` if room doesn't exist.
    pub fn get_room_meta_all(&self, room_id: &str) -> Option<HashMap<String, String>> {
        self.rooms.get(room_id).map(|r| r.get_all_meta())
    }

    /// Room a client is currently in. Returns `None` if not in a room or not connected.
    pub fn get_client_room(&self, client_id: Uuid) -> Option<String> {
        self.clients.get(&client_id)?.room_id.clone()
    }

    /// Remote address of a connected client.
    pub fn get_client_addr(&self, client_id: Uuid) -> Option<SocketAddr> {
        self.clients.get(&client_id).map(|c| c.addr)
    }

    /// Total number of connected clients.
    pub fn get_client_count(&self) -> usize {
        self.client_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Server {
    /// Start building a server.
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Create a room. Call this before [`Server::run`]. (pre-run)
    /// Clients can only join rooms that have been explicitly created.
    /// Returns [`SyncError::RoomAlreadyExists`] if a room with this ID exists.
    #[deprecated(note = "Use ServerHandle::create_room instead, at runtime")]
    pub fn pre_create_room(&self, id: &str) -> Result<()> {
        self.rooms.create(id)?;
        self.handler.on_room_create(id);
        Ok(())
    }

    /// Delete a room. Call this before [`Server::run`]. (pre-run)
    /// Returns `true` if the room existed.
    #[deprecated(note = "Use ServerHandle::delete_room instead, at runtime")]
    pub fn pre_delete_room(&self, id: &str) -> bool {
        let existed = self.rooms.delete(id);
        if existed {
            self.handler.on_room_delete(id);
        }
        existed
    }

    /// Start accepting connections.
    ///
    /// Binds the TCP listener synchronously. If the bind fails, the error is
    /// returned immediately — nothing is spawned on failure.
    ///
    /// On success, spawns the accept loop on the current runtime and returns
    /// a [`ServerHandle`] for shutdown and runtime control.
    pub async fn run(self) -> std::io::Result<ServerHandle> {
        let listener = TcpListener::bind(&self.config.bind_addr).await?;
        info!("listening on {}", self.config.bind_addr);

        let (shutdown_tx, _) = broadcast::channel::<()>(2);
        let handle = ServerHandle {
            shutdown_tx: shutdown_tx.clone(),
            rooms: self.rooms.clone(),
            clients: self.clients.clone(),
            handler: self.handler.clone(),
            client_count: self.client_count.clone(),
        };

        let mut shutdown_rx = shutdown_tx.subscribe();
        let mut server = self;
        server.shutdown_tx = shutdown_tx;

        let server = Arc::new(server);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                if !server.handler.on_connect(addr) {
                                    warn!("connection rejected by handler: {addr}");
                                    drop(stream);
                                    continue;
                                }

                                let count = server.client_count.load(std::sync::atomic::Ordering::Relaxed);
                                if count >= server.config.max_clients {
                                    warn!("max clients reached, rejecting {addr}");
                                    drop(stream);
                                    continue;
                                }

                                server.client_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                let srv = server.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = srv.handle_connection(stream, addr).await {
                                        if !e.is_connection_closed() {
                                            warn!("client error: {e}");
                                        }
                                    }
                                    srv.client_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                                });
                            }
                            Err(e) => {
                                error!("accept error: {e}");
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("shutting down");
                        break;
                    }
                }
            }
        });

        Ok(handle)
    }

    #[inline]
    async fn handle_connection(
        self: &Arc<Self>,
        stream: TcpStream,
        addr: SocketAddr,
    ) -> Result<()> {
        let client_id = Uuid::new_v4();
        let (read_half, write_half) = stream.into_split();
        let mut reader = tokio::io::BufReader::new(read_half);
        let mut writer = BufWriter::new(write_half);

        let (write_tx, mut write_rx) = mpsc::channel::<Bytes>(self.config.channel_capacity);

        // Spawn writer task
        let writer_handle = tokio::spawn(async move {
            while let Some(frame) = write_rx.recv().await {
                if let Err(e) = protocol::write_frame_raw(&mut writer, &frame).await {
                    #[allow(clippy::needless_ifs)]
                    if !e.is_connection_closed() {}
                    warn!("write error: {e}");
                    break;
                }
            }
        });

        let result = self
            .client_loop(client_id, addr, &mut reader, &write_tx)
            .await;

        // Cleanup
        if let Some(state) = self.clients.remove(&client_id) {
            if let Some(room_id) = state.1.room_id {
                self.cleanup_client(client_id, &room_id).await;
            }
        }

        drop(write_tx);
        let _ = writer_handle.await;

        result
    }

    #[inline(always)]
    async fn client_loop(
        self: &Arc<Self>,
        client_id: Uuid,
        addr: SocketAddr,
        reader: &mut tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>,
        write_tx: &mpsc::Sender<Bytes>,
    ) -> Result<()> {
        // Store client state with address
        self.clients.insert(
            client_id,
            ClientState {
                room_id: None,
                addr,
            },
        );

        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let mut ping_interval = tokio::time::interval(self.config.ping_interval);
        let mut awaiting_pong = false;

        // Skip the first immediate tick so the client has time to JoinRoom
        ping_interval.tick().await;

        loop {
            tokio::select! {
                // Read incoming frame with inactivity timeout
                // If no frame (any message) arrives within idle_timeout, disconnect.
                // Pong responses reset this timer.
                result = tokio::time::timeout(
                    self.config.idle_timeout,
                    protocol::read_frame_raw(reader, self.config.max_payload),
                ) => {
                    match result {
                        Ok(Ok(payload)) => {
                            let msg: ClientWire = wincode::deserialize(&payload)
                                .map_err(|e| SyncError::Protocol(format!("deserialize failed: {:?}", e)))?;

                            // Client sent Pong — clear awaiting flag
                            if matches!(msg, ClientWire::Pong) {
                                awaiting_pong = false;
                                continue;
                            }

                            self.process_message(client_id, msg, write_tx).await?;
                        }
                        Ok(Err(SyncError::Io(ref e)))
                            if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                        {
                            return Ok(()); // clean disconnect
                        }
                        // Any other error from read_frame_raw is a protocol or IO error
                        Ok(Err(e)) => return Err(e),
                        // Timeout means the client was idle for too long
                        Err(_timeout) => return Err(SyncError::IdleTimeout),
                    }
                }

                // Ping tick — server sends Ping to client
                // Pong deadline = next tick. If still awaiting_pong, disconnect.
                _ = ping_interval.tick() => {
                    if awaiting_pong {
                        // Client didn't respond with Pong before this tick
                        return Err(SyncError::PingTimeout);
                    }
                    let ping = ServerWire::Ping;
                    self.send_to_client(client_id, write_tx, &ping).await;
                    awaiting_pong = true;
                }

                // Shutdown signal
                _ = shutdown_rx.recv() => {
                    return Err(SyncError::ConnectionClosed);
                }
            }
        }
    }

    #[inline(always)]
    async fn process_message(
        self: &Arc<Self>,
        client_id: Uuid,
        msg: ClientWire,
        write_tx: &mpsc::Sender<Bytes>,
    ) -> Result<()> {
        match msg {
            ClientWire::JoinRoom { room_id } => {
                // Get client addr for the hook
                let addr = match self.clients.get(&client_id) {
                    Some(state) => state.addr,
                    None => return Ok(()),
                };

                // Check room exists
                if self.rooms.get(&room_id).is_none() {
                    let err = ServerWire::Error(format!("room not found: {room_id}"));
                    self.send_to_client(client_id, write_tx, &err).await;
                    return Ok(());
                }

                // Handler hook — can reject the join
                if !self.handler.on_join(client_id, &room_id, addr) {
                    let err = ServerWire::Error("join rejected".into());
                    self.send_to_client(client_id, write_tx, &err).await;
                    return Ok(());
                }

                // Leave previous room if any
                if let Some(mut state) = self.clients.get_mut(&client_id) {
                    if let Some(old_room) = state.room_id.take() {
                        drop(state);
                        self.cleanup_client(client_id, &old_room).await;
                    }
                }

                // Insert into room
                {
                    let room = self.rooms.get(&room_id).unwrap();
                    room.insert(client_id, write_tx.clone());
                }

                // Update client state
                if let Some(mut state) = self.clients.get_mut(&client_id) {
                    state.room_id = Some(room_id.clone());
                }

                let joined = ServerWire::Joined {
                    client_id,
                    room_id: room_id.clone(),
                };
                self.send_to_client(client_id, write_tx, &joined).await;

                let notify = ServerWire::PlayerJoined { client_id };
                if let Some(room) = self.rooms.get(&room_id) {
                    let dropped = room.broadcast(client_id, &notify).await;
                    for id in dropped {
                        self.handler.on_backpressure(id, &room_id);
                    }
                }
            }

            ClientWire::LeaveRoom => {
                if let Some(mut state) = self.clients.get_mut(&client_id) {
                    if let Some(room_id) = state.room_id.take() {
                        drop(state);
                        self.cleanup_client(client_id, &room_id).await;
                    }
                }
            }

            ClientWire::Ping => {
                let pong = ServerWire::Pong;
                self.send_to_client(client_id, write_tx, &pong).await;
            }

            ClientWire::Pong => {
                // Handled in client_loop select to clear awaiting_pong
            }

            ClientWire::Broadcast { data } => {
                let room_id = match self.clients.get(&client_id) {
                    Some(state) => match &state.room_id {
                        Some(id) => id.clone(),
                        None => {
                            let err = ServerWire::Error("not in a room".into());
                            self.send_to_client(client_id, write_tx, &err).await;
                            return Ok(());
                        }
                    },
                    None => return Ok(()),
                };

                let broadcast = ServerWire::Broadcast {
                    sender_id: client_id,
                    data,
                };
                let payload = wincode::serialize(&broadcast)
                    .map_err(|e| SyncError::Protocol(format!("serialize failed: {:?}", e)))?;
                let payload = Bytes::from(payload);

                if let Some(room) = self.rooms.get(&room_id) {
                    let dropped = room.broadcast_raw(client_id, payload).await;
                    for id in dropped {
                        self.handler.on_backpressure(id, &room_id);
                    }
                }

                if let ServerWire::Broadcast { ref data, .. } = broadcast {
                    self.handler.on_broadcast(client_id, &room_id, data);
                }
            }
        }

        Ok(())
    }

    #[inline]
    async fn cleanup_client(self: &Arc<Self>, client_id: Uuid, room_id: &str) {
        let notify = ServerWire::PlayerLeft { client_id };
        if let Some(room) = self.rooms.get(room_id) {
            let dropped = room.broadcast(client_id, &notify).await;
            for id in dropped {
                self.handler.on_backpressure(id, room_id);
            }
        }

        self.rooms.remove_client(room_id, &client_id);

        self.handler.on_leave(client_id, room_id);
    }

    #[inline(always)]
    async fn send_to_client(&self, client_id: Uuid, tx: &mpsc::Sender<Bytes>, msg: &ServerWire) {
        if let Ok(payload) = wincode::serialize(msg) {
            if tx.try_send(Bytes::from(payload)).is_err() {
                if let Some(state) = self.clients.get(&client_id) {
                    if let Some(ref room_id) = state.room_id {
                        self.handler.on_backpressure(client_id, room_id);
                    }
                }
            }
        }
    }
}

/// Builder for configuring a [`Server`].
///
/// ```no_run
/// use ghost_sync::Server;
///
/// let server = Server::builder()
///     .bind("0.0.0.0:7777")
///     .max_clients(128)
///     .max_payload(128 * 1024)
///     .build();
/// ```
pub struct ServerBuilder {
    config: ServerConfig,
    handler: Arc<dyn ServerHandler>,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
            handler: Arc::new(NoopHandler),
        }
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerBuilder {
    /// Set the bind address (e.g., `"0.0.0.0:7777"`).
    pub fn bind(mut self, addr: impl Into<String>) -> Self {
        self.config.bind_addr = addr.into();
        self
    }

    /// Set max concurrent clients. Connections beyond this are rejected.
    pub fn max_clients(mut self, n: usize) -> Self {
        self.config.max_clients = n;
        self
    }

    /// Set max incoming payload size in bytes. Frames larger than this are rejected.
    pub fn max_payload(mut self, n: usize) -> Self {
        self.config.max_payload = n;
        self
    }

    /// Disconnect clients that send nothing for this duration.
    /// Any frame (including Pong) resets this timer. This is separate from
    /// ping-based liveness — see [`ping_interval`](Self::ping_interval).
    pub fn idle_timeout(mut self, d: Duration) -> Self {
        self.config.idle_timeout = d;
        self
    }

    /// How often the server pings clients. Pong deadline is one full
    /// `ping_interval` — if no Pong arrives before the next tick, the
    /// client is disconnected. This is separate from read inactivity —
    /// see [`idle_timeout`](Self::idle_timeout).
    pub fn ping_interval(mut self, d: Duration) -> Self {
        self.config.ping_interval = d;
        self
    }

    /// Per-client write channel capacity. Frames are dropped when full
    /// (with `on_backpressure` hook). Higher values buffer more for bursty
    /// games, lower values keep latency tight. Default: 64.
    pub fn channel_capacity(mut self, n: usize) -> Self {
        self.config.channel_capacity = n;
        self
    }

    /// Set a custom event handler for lifecycle hooks.
    pub fn handler(mut self, h: impl ServerHandler) -> Self {
        self.handler = Arc::new(h);
        self
    }

    /// Build the server. Call [`Server::run`] to start accepting connections.
    pub fn build(self) -> Server {
        let (tx, _) = broadcast::channel(1);
        Server {
            config: self.config.clone(),
            handler: self.handler,
            rooms: Arc::new(RoomManager::new()),
            clients: Arc::new(DashMap::with_capacity(self.config.max_clients)),
            client_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            shutdown_tx: tx,
        }
    }
}
