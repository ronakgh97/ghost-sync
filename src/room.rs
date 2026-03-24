use std::collections::HashMap;

use bytes::Bytes;
use dashmap::DashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::types::{Result, ServerWire, SyncError};

/// A broadcast frame to be sent to a client. `Bytes` is reference-counted,
/// so cloning for multiple recipients is an Arc bump, not a memcpy.
pub type BroadcastFrame = Bytes;

/// A single room holding connected clients.
///
/// Rooms are created explicitly via [`RoomManager::create`] and hold
/// connected clients identified by their UUIDs. Each client has a
/// write channel for outgoing frames.
///
/// Rooms carry a string metadata map. The library does not interpret metadata —
/// users store whatever they need (passwords, max players, game mode, etc.)
/// and check it in their [`ServerHandler`](crate::ServerHandler) hooks.
pub struct Room {
    clients: DashMap<Uuid, mpsc::Sender<BroadcastFrame>>,
    metadata: DashMap<String, String>,
}

#[allow(dead_code)]
impl Room {
    fn new() -> Self {
        Self {
            clients: DashMap::with_capacity(64),
            metadata: DashMap::with_capacity(128),
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

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.clients.len()
    }

    /// Get the write channel queue length for a specific client.
    ///
    /// Returns `None` if the client is not in this room.
    /// Higher values indicate the client's writer task is falling behind.
    #[inline(always)]
    pub fn channel_len(&self, id: &Uuid) -> Option<usize> {
        self.clients
            .get(id)
            .map(|tx| tx.max_capacity() - tx.capacity())
    }

    /// Get all clients' write channel queue lengths.
    ///
    /// Returns `(uuid, channel_len)` pairs. Useful for monitoring
    /// backpressure and identifying slow clients.
    #[inline(always)]
    pub fn all_channel_lens(&self) -> Vec<(Uuid, usize)> {
        self.clients
            .iter()
            .map(|e| {
                let tx = e.value();
                (*e.key(), tx.max_capacity() - tx.capacity())
            })
            .collect()
    }

    /// Serialize a ServerWire message and broadcast to all clients except `sender`.
    /// Returns the UUIDs of clients whose write channels were full (frame dropped).
    #[inline(always)]
    pub async fn broadcast(&self, sender: Uuid, msg: &ServerWire) -> Vec<Uuid> {
        let payload = match wincode::serialize(msg) {
            Ok(p) => Bytes::from(p),
            Err(_) => return Vec::new(),
        };
        self.broadcast_raw(sender, payload).await
    }

    /// Send pre-serialized bytes to all clients except `sender`.
    /// Returns the UUIDs of clients whose write channels were full (frame dropped).
    ///
    /// Cloning `Bytes` is an Arc pointer bump — no data copy regardless of payload size.
    #[inline(always)]
    pub async fn broadcast_raw(&self, sender: Uuid, payload: Bytes) -> Vec<Uuid> {
        let mut dropped = Vec::new();
        for entry in self.clients.iter() {
            if *entry.key() != sender && entry.value().try_send(payload.clone()).is_err() {
                dropped.push(*entry.key());
            }
        }
        dropped
    }

    /// Send a ServerWire message to a specific client.
    #[inline(always)]
    pub async fn send_to(&self, id: &Uuid, msg: &ServerWire) {
        if let Some(tx) = self.clients.get(id) {
            if let Ok(payload) = wincode::serialize(msg) {
                let _ = tx.try_send(Bytes::from(payload));
            }
        }
    }

    /// Get all client IDs in this room.
    #[inline]
    pub fn client_ids(&self) -> Vec<Uuid> {
        self.clients.iter().map(|e| *e.key()).collect()
    }

    /// Get a metadata value by key.
    #[inline]
    pub fn get_meta(&self, key: &str) -> Option<String> {
        self.metadata.get(key).map(|v| v.clone())
    }

    /// Set a metadata key-value pair.
    #[inline]
    pub fn set_meta(&self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Remove a metadata key.
    #[inline]
    pub fn remove_meta(&self, key: &str) -> bool {
        self.metadata.remove(key).is_some()
    }

    /// Get all metadata as a HashMap.
    #[inline]
    pub fn get_all_meta(&self) -> HashMap<String, String> {
        self.metadata
            .iter()
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect()
    }
}

/// Manages all active rooms.
pub struct RoomManager {
    rooms: DashMap<String, Room>,
}

impl RoomManager {
    /// Init a room manager with 64 pre-allocated slots.
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

    /// List all room IDs.
    #[inline]
    pub fn room_ids(&self) -> Vec<String> {
        self.rooms.iter().map(|e| e.key().clone()).collect()
    }
}
