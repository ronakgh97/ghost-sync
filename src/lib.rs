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
