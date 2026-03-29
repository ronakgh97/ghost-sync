//! # ghost-sync [GitHub](https://github.com/ronakgh97/ghost-sync)
//!
//! A composable Tokio TCP relay for room-based multiplayer messaging.
//!
//! `ghost-sync` provides a lightweight, backpressure-aware server and an asynchronous client for
//! building real-time multiplayer applications where state sync is managed explicitly via rooms.
//!
//! ## Features
//!
//! - **Room Management**: Clients join isolated rooms. All messages within a room are broadcast to all connected peers.
//! - **Efficient Broadcasts**: Messages are serialized exactly once per room and zero-copy shared among all recipients.
//! - **Backpressure**: Prevents slow clients from consuming unbounded server memory via drop-on-full semantics. Hook into backpressure events via [`ServerHandler::on_backpressure`].
//! - **Custom Handlers**: Inject your own application logic using the [`ServerHandler`] trait (e.g., `on_connect`, `on_join`, `on_leave`, `on_broadcast`).
//!
//! ## Quick Examples
//!
//! ### Server
//! ```rust,no_run
//! use ghost_sync::{Server, NoopHandler};
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = Server::builder()
//!         .bind("127.0.0.1:8080")
//!         .handler(NoopHandler)
//!         .build();
//!
//!     // Creates a background task running the server
//!     let handle = server.run().await.unwrap();
//!
//!     // Keep the server alive
//!     std::future::pending::<()>().await;
//! }
//! ```
//!
//! ### Client
//! ```rust,no_run
//! use ghost_sync::{Client, ServerEvent};
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut client = Client::connect("127.0.0.1:8080").await.unwrap();
//!
//!     // Join a room to start receiving and sending broadcasts
//!     client.join("test-room", None).await.unwrap();
//!
//!     while let Ok(Some(event)) = client.recv().await {
//!         match event {
//!             ServerEvent::Joined { client_id, room_id } => println!("Joined room: {}", room_id),
//!             ServerEvent::Broadcast { sender_id, data } => {
//!                 println!("Received broadcast: {:?}", data);
//!             },
//!             _ => {}
//!         }
//!     }
//! }
//! ```

#![allow(deprecated)]
mod client;
mod handler;
pub mod log;
mod protocol;
mod room;
mod server;
mod storage;
mod types;

pub use client::{Client, ClientBuilder};
pub use handler::{NoopHandler, ServerHandler};
pub use log::*;
pub use server::{Server, ServerBuilder, ServerHandle};
pub use types::{Result, ServerConfig, ServerEvent, SyncError};

pub use uuid::Uuid;
