use tokio::io::{BufReader, BufWriter};
use tokio::net::TcpStream;

use crate::protocol;
use crate::types::{ClientWire, ServerEvent, ServerWire, SyncError};

/// A connected relay client.
pub struct Client {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    max_payload: usize,
}

impl Client {
    /// Connect to a relay server.
    pub async fn connect(addr: &str) -> Result<Self, SyncError> {
        Self::builder().connect(addr).await
    }

    /// Start building a client.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Join a room. If room does not exist, returns [`SyncError::RoomNotFound`]
    pub async fn join(&mut self, room_id: &str) -> Result<(), SyncError> {
        let msg = ClientWire::JoinRoom {
            room_id: room_id.into(),
        };
        self.send(&msg).await
    }

    /// Leave the current room.
    pub async fn leave(&mut self) -> Result<(), SyncError> {
        self.send(&ClientWire::LeaveRoom).await
    }

    /// Send a ping (keep-alive).
    #[inline(always)]
    pub async fn ping(&mut self) -> Result<(), SyncError> {
        self.send(&ClientWire::Ping).await
    }

    /// Broadcast raw bytes to all peers in the current room.
    #[inline]
    pub async fn broadcast(&mut self, data: &[u8]) -> Result<(), SyncError> {
        let msg = ClientWire::Broadcast {
            data: data.to_vec(),
        };
        self.send(&msg).await
    }

    /// Receive the next server event.
    /// Ping/pong keepalive is handled internally
    /// Returns `Ok(None)` on clean disconnect.
    #[inline]
    pub async fn recv(&mut self) -> Result<Option<ServerEvent>, SyncError> {
        loop {
            let payload = match protocol::read_frame_raw(&mut self.reader, self.max_payload).await {
                Ok(p) => p,
                Err(ref e) if e.is_connection_closed() => {
                    return Ok(None);
                }
                Err(e) => return Err(e),
            };

            let wire: ServerWire = wincode::deserialize(&payload)
                .map_err(|e| SyncError::Protocol(format!("deserialize failed: {:?}", e)))?;

            // Internal: auto-respond to server pings
            if matches!(wire, ServerWire::Ping) {
                self.send(&ClientWire::Pong).await?;
                continue;
            }

            // Internal: swallow pong responses
            if matches!(wire, ServerWire::Pong) {
                continue;
            }

            return Ok(Some(Self::wire_to_event(wire)));
        }
    }

    async fn send(&mut self, msg: &ClientWire) -> Result<(), SyncError> {
        protocol::write_frame(&mut self.writer, msg).await
    }

    fn wire_to_event(wire: ServerWire) -> ServerEvent {
        match wire {
            ServerWire::Joined { client_id, room_id } => ServerEvent::Joined { client_id, room_id },
            ServerWire::PlayerJoined { client_id } => ServerEvent::PlayerJoined { client_id },
            ServerWire::PlayerLeft { client_id } => ServerEvent::PlayerLeft { client_id },
            ServerWire::Error(msg) => ServerEvent::Error(msg),
            ServerWire::Broadcast { sender_id, data } => ServerEvent::Broadcast { sender_id, data },
            // Ping/Pong are handled internally before reaching here
            _ => unreachable!("ping/pong should be handled in recv()"),
        }
    }
}

/// Builder for configuring a [`Client`].
pub struct ClientBuilder {
    max_payload: usize,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            max_payload: 256 * 1024,
        }
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientBuilder {
    /// Set max payload size for incoming frames.
    pub fn max_payload(mut self, n: usize) -> Self {
        self.max_payload = n;
        self
    }

    /// Connect to the server.
    pub async fn connect(self, addr: &str) -> Result<Client, SyncError> {
        let stream = TcpStream::connect(addr).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::ConnectionRefused {
                SyncError::ConnectionRefused
            } else {
                SyncError::Io(e)
            }
        })?;
        stream.set_nodelay(true).ok(); // Ignore error, not critical
        let (read_half, write_half) = stream.into_split();

        Ok(Client {
            reader: BufReader::with_capacity(128 * 1024, read_half),
            writer: BufWriter::with_capacity(128 * 1024, write_half),
            max_payload: self.max_payload,
        })
    }
}
