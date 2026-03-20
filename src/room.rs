use crate::types::{Result, ServerWire, SyncError};
use dashmap::DashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

/// A broadcast frame to be sent to a client.
pub type BroadcastFrame = Vec<u8>;

/// A single room holding connected clients.
///
/// Rooms are created explicitly via [`RoomManager::create`] and hold
/// connected clients identified by their UUIDs. Each client has a
/// write channel for outgoing frames.
pub struct Room {
    clients: DashMap<Uuid, mpsc::Sender<BroadcastFrame>>,
}

#[allow(dead_code)]
impl Room {
    fn new() -> Self {
        Self {
            clients: DashMap::new(),
        }
    }

    #[inline]
    pub fn insert(&self, id: Uuid, tx: mpsc::Sender<BroadcastFrame>) {
        self.clients.insert(id, tx);
    }

    #[inline]
    pub fn remove(&self, id: &Uuid) -> bool {
        self.clients.remove(id).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    pub fn len(&self) -> usize {
        self.clients.len()
    }

    /// Serialize a ServerWire message and broadcast to all clients except `sender`.
    /// Returns the UUIDs of clients whose write channels were full (frame dropped).
    #[inline(always)]
    pub async fn broadcast(&self, sender: Uuid, msg: &ServerWire) -> Vec<Uuid> {
        let payload = match wincode::serialize(msg) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        self.broadcast_raw(sender, &payload).await
    }

    /// Send raw bytes to all clients except `sender`.
    /// Returns the UUIDs of clients whose write channels were full (frame dropped).
    #[inline(always)]
    pub async fn broadcast_raw(&self, sender: Uuid, payload: &[u8]) -> Vec<Uuid> {
        let mut dropped = Vec::new();
        for entry in self.clients.iter() {
            if *entry.key() != sender && entry.value().try_send(payload.to_vec()).is_err() {
                dropped.push(*entry.key());
            }
        }
        dropped
    }

    /// Send a ServerWire message to a specific client.
    #[inline(always)]
    pub async fn send_to(&self, id: &Uuid, msg: &ServerWire) {
        if let Some(tx) = self.clients.get(id) {
            let payload = match wincode::serialize(msg) {
                Ok(p) => p,
                Err(_) => return,
            };
            let _ = tx.try_send(payload);
        }
    }

    /// Get all client IDs in this room.
    #[inline]
    pub fn client_ids(&self) -> Vec<Uuid> {
        self.clients.iter().map(|e| *e.key()).collect()
    }
}

/// Manages all active rooms.
pub struct RoomManager {
    rooms: DashMap<String, Room>,
}

impl RoomManager {
    /// Init a room with 64s pre-allocated
    pub fn new() -> Self {
        Self {
            rooms: DashMap::with_capacity(64),
        }
    }

    /// Create a room with the given ID. Fails if the room already exists.
    #[inline(always)]
    pub fn create(&self, room_id: &str) -> Result<()> {
        if self.rooms.contains_key(room_id) {
            return Err(SyncError::RoomAlreadyExists(room_id.to_string()));
        }
        self.rooms.insert(room_id.to_string(), Room::new());
        Ok(())
    }

    /// Delete a room. Returns true if the room existed.
    #[inline(always)]
    pub fn delete(&self, room_id: &str) -> bool {
        self.rooms.remove(room_id).is_some()
    }

    /// Remove a client from a room. Does NOT auto-delete the room.
    #[inline(always)]
    pub fn remove_client(&self, room_id: &str, client_id: &Uuid) {
        if let Some(room) = self.rooms.get(room_id) {
            room.remove(client_id);
        }
    }

    /// Get a reference to a room. Returns None if room doesn't exist.
    #[inline]
    pub fn get(&self, room_id: &str) -> Option<dashmap::mapref::one::Ref<'_, String, Room>> {
        self.rooms.get(room_id)
    }

    /// Number of active rooms.
    #[inline]
    pub fn len(&self) -> usize {
        self.rooms.len()
    }
}
