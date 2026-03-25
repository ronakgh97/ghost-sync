# ghost-sync docs

Usage guide for the `ghost-sync` Tokio TCP relay library.

## What ghost-sync is

`ghost-sync` is an opinionated room-based TCP relay for real-time multiplayer messaging in Rust.

- Transport: length-prefixed frames over TCP + `wincode` serialization.
- Core model: clients join rooms, broadcast payload bytes, receive room events.
- Extension model: custom policies via `ServerHandler` hooks.

This is a learning-focused library built to explore the Tokio ecosystem and networking patterns.
It aims to be genuinely usable despite its scope.

---

## Usage models

### Embedded in your server (primary model)

Wrap `ghost-sync` in your own backend service. Use `ServerHandler` hooks to inject your game logic.

This is the intended way to use the library. It gives you full control over:

- who connects (`on_connect`)
- who joins what room (`on_join`)
- room lifecycle (`on_room_create`, `on_room_delete`)
- what happens when a client is slow (`on_backpressure`)
- and more... (TODO)

### Hybrid

Run relay as a separate process. Keep game logic and control plane in your main backend.

---

## Little Overview

### Server

```rust
use ghost_sync::{Server, ServerHandler, Uuid};
use std::net::SocketAddr;

struct GameHandler;

impl ServerHandler for GameHandler {
    fn on_connect(&self, addr: SocketAddr) -> bool {
        // reject non-local in dev
        addr.ip().is_loopback()
    }

    fn on_join(&self, client_id: Uuid, room_id: &str, _addr: SocketAddr) -> bool {
        println!("{client_id} joined {room_id}");
        true
    }

    fn on_backpressure(&self, client_id: Uuid, room_id: &str) {
        eprintln!("slow client {client_id} in {room_id}");
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = Server::builder()
        .bind("127.0.0.1:7777")
        .max_clients(128)
        .max_payload(64 * 1024)
        .channel_capacity(64)
        .handler(GameHandler)
        .build();

    let handle = server.run().await?;

    // rooms are managed at runtime via the handle
    handle.create_room("lobby")?;
    handle.create_room("deathmatch")?;

    tokio::signal::ctrl_c().await?;
    handle.shutdown().await;
    Ok(())
}
```

### Client

```rust
use ghost_sync::{Client, ServerEvent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut client = Client::connect("127.0.0.1:7777").await?;
    client.join("lobby").await?;

    // join() sends the request; confirmation comes via recv()
    match client.recv().await? {
        Some(ServerEvent::Joined { client_id, room_id }) => {
            println!("joined {room_id} as {client_id}");
        }
        Some(ServerEvent::Error(msg)) => {
            eprintln!("join failed: {msg}");
            return Ok(());
        }
        _ => return Ok(()),
    }

    client.broadcast(b"hello room").await?;

    // recv() loop is required — it also handles keepalive internally
    while let Some(event) = client.recv().await? {
        match event {
            ServerEvent::Broadcast { sender_id, data } => {
                println!("{sender_id}: {}", String::from_utf8_lossy(&data));
            }
            ServerEvent::PlayerJoined { client_id } => {
                println!(">> {client_id} joined");
            }
            ServerEvent::PlayerLeft { client_id } => {
                println!(">> {client_id} left");
            }
            _ => {}
        }
    }

    Ok(())
}
```

---

## ServerHandler hooks

The `ServerHandler` trait is your main extension point. All methods have no-op defaults.

| Hook                                     | When                         | Can reject?          |
|------------------------------------------|------------------------------|----------------------|
| `on_connect(addr)`                       | TCP connection arrives       | Yes — return `false` |
| `on_join(client_id, room_id, addr)`      | Client requests join         | Yes — return `false` |
| `on_leave(client_id, room_id)`           | Client leaves or disconnects | No                   |
| `on_broadcast(client_id, room_id, data)` | Message relayed to room      | No                   |
| `on_backpressure(client_id, room_id)`    | Frame dropped (slow client)  | No                   |
| `on_room_create(room_id)`                | Room created                 | No                   |
| `on_room_delete(room_id)`                | Room deleted                 | No                   |

### Example: max players per room

```rust
use ghost_sync::{ServerHandler, Uuid, ServerHandle};
use std::net::SocketAddr;

struct MaxPlayers {
    limit: usize,
    handle: ServerHandle,
}

impl ServerHandler for MaxPlayers {
    fn on_join(&self, _client_id: Uuid, room_id: &str, _addr: SocketAddr) -> bool {
        match self.handle.room_client_count(room_id) {
            Some(count) => count < self.limit,
            None => false,
        }
    }
}
```

### Example: reject bad room IDs

```rust
use ghost_sync::{ServerHandler, Uuid};
use std::net::SocketAddr;

struct StrictRooms;

impl ServerHandler for StrictRooms {
    fn on_join(&self, _client_id: Uuid, room_id: &str, _addr: SocketAddr) -> bool {
        // only allow pre-approved room IDs
        matches!(room_id, "lobby" | "deathmatch" | "duel")
    }
}
```

---

## Room lifecycle

- Rooms are created at runtime via `ServerHandle::create_room()`.
- Clients **cannot** join rooms that don't exist.
- Empty rooms are not auto-deleted.
- Delete is **soft-delete**: connected clients in the room are not kicked immediately. Their next broadcast or join in
  that room will fail.
- Rooms can hold metadata via `set_room_meta()` / `get_room_meta()`.

## Keepalive and liveness

There are two independent liveness checks running simultaneously.

### Read inactivity timeout (`idle_timeout`)

If the server does not receive **any frame** from a client for longer than
`idle_timeout`, the server disconnects that client with `SyncError::IdleTimeout`.

A Pong response to a Ping counts as activity. So if the client is driving
`recv()` and responding to server pings, this timer resets on every pong
the client sends.

### Ping-based liveness (`ping_interval`)

Server sends a `Ping` to every client every `ping_interval`. The client
auto-responds with `Pong` inside `recv()`. If the server does not receive
a `Pong` before the **next** ping tick, it disconnects that client with
`SyncError::PingTimeout`.

The effective pong deadline is one full `ping_interval`.

### Why two checks?

They catch different failure modes:

- `idle_timeout` catches clients that go completely silent (network drop, client crash).
- `ping_interval` catches clients that are still connected but not responding to
  protocol-level pings (e.g., client stopped calling `recv()`).

### Critical: client must drive `recv()`

The client auto-responds to pings **inside `recv()`**. If the client stops calling
`recv()`:
- it stops sending Pong responses,
- server sees no activity for `idle_timeout` seconds,
- server disconnects with `IdleTimeout` (or `PingTimeout`, whichever fires first).

**You must keep calling `recv()` in a loop.** Even if you have no messages to send. (or use explicit `ping()` method in the future)

### Disconnect reasons summary

| Error              | Cause                                                     |
|--------------------|-----------------------------------------------------------|
| `IdleTimeout`      | No frame received from client for `idle_timeout` duration |
| `PingTimeout`      | Client did not send Pong before next ping tick            |
| `ConnectionClosed` | Server shutdown, or client closed connection cleanly      |
| `ConnectionReset`  | Client disappeared (network drop, process killed)         |
| `PayloadTooLarge`  | Client sent a frame larger than `max_payload`             |
| `Protocol`         | Deserialization failure (malformed frame)                 |

### How `recv()` handles this internally

`recv()` blocks until a server event arrives. While blocking, it automatically
responds to server `Ping` with `Pong` and swallows `Pong` responses. The caller
never sees ping/pong traffic — it is transparent.

---

## Backpressure

Per-client write queues are bounded by `channel_capacity` (default: 64).

When the queue is full, the frame is **dropped** (non-blocking `try_send`).
The `on_backpressure` hook fires so you can log, warn, or disconnect the slow client.

This is intentional for real-time game relay. One slow client should never
stall the server or delay messages to other clients.

**Semantics are lossy.** If you need reliable delivery, add retransmission at
the application level above ghost-sync.

```rust
impl ServerHandler for MyHandler {
    fn on_backpressure(&self, client_id: Uuid, room_id: &str) {
        // log it, kick the client, or downgrade their experience :(
        eprintln!("dropped frame for {client_id} in {room_id}");
    }
}
```

Tuning guidance:

| Scenario                          | `channel_capacity` | Trade-off                       |
|-----------------------------------|--------------------|---------------------------------|
| Latency-sensitive (FPS, fighting) | 16-32              | Lower latency, more drops       |
| Bursty but tolerant (lobby, chat) | 64-128             | Buffer more, fewer drops        |
| State sync (RPG, RTS)             | 128+               | Tolerate bursts, higher latency |

---

## Configuration reference

Set via `ServerBuilder`:

| Option                | Default          | Description                                                         |
|-----------------------|------------------|---------------------------------------------------------------------|
| `bind(addr)`          | `"0.0.0.0:7777"` | TCP listen address                                                  |
| `max_clients(n)`      | `1024`           | Max concurrent connections                                          |
| `max_payload(n)`      | `64 * 1024`      | Max frame size in bytes                                             |
| `idle_timeout(d)`     | `31s`            | Max time without receiving any frame from client                    |
| `ping_interval(d)`    | `13s`            | Ping cadence; also the pong deadline (miss = disconnect)            |
| `channel_capacity(n)` | `1024`           | Per-client write queue size; frames dropped when full               |
| `handler(h)`          | `NoopHandler`    | Custom `ServerHandler` implementation                               |

- `idle_timeout` and `ping_interval` are independent. A client that stops
  calling `recv()` will eventually fail one or both checks.
- Defaults are intentionally non-divisible (`31s` / `13s`) to avoid the two
  timers always firing at the same moment.

---

## Right way

1. **Always drive `recv()` in a loop.** It handles keepalive internally.
2. **Wait for `ServerEvent::Joined` after `join()`** before sending messages.
3. **Use `on_join` to enforce room rules** (max players, private rooms, bans).
4. **Treat `on_backpressure` as a signal.** Log it, warn the client, or disconnect them.
5. **Set realistic timeouts** for your network. Defaults are conservative (LAN).
6. **Create rooms explicitly** before clients can join them.
7. **Use room metadata** to track game state without separate storage.

---

## When not to use ghost-sync


- You hate me :(


- Reliable ordered delivery with retransmission.
- Built-in auth, accounts, or session management.
- Built-in TLS (use a reverse proxy instead).
- Authoritative game state simulation in the server.
- A packaged production daemon with full ops tooling.
- WebSocket support for browser clients.

---

## Anti-patterns for browsers

ghost-sync uses raw TCP. Browsers cannot connect directly.
If you need browser support, put a WebSocket proxy in front (e.g., `tungstenite`, `tokio-tungstenite`,
or a reverse proxy like Nginx with WebSocket upgrade).

