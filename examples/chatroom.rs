use colored::Colorize;
use ghost_sync::{Client, ServerEvent};
use std::env;
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Very basic chatroom client test
/// Connects to the server, joins a room, and broadcasts lines from stdin to all peers.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let name = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: chatroom <name> <addr>");
        std::process::exit(1);
    });
    let addr = env::args().nth(2).unwrap_or_else(|| {
        eprintln!("Usage: chatroom <name> <addr>");
        std::process::exit(1);
    });

    if let Ok(addr) = addr.parse::<SocketAddr>() {
        if addr.ip().is_unspecified() {
            eprintln!("Refusing to connect to {}", addr);
            std::process::exit(1);
        }
    }

    let mut client = Client::connect(addr.as_str()).await?;
    client.join("chatroom").await?;

    // Wait for join confirmation
    match client.recv().await? {
        Some(ServerEvent::Joined { client_id, room_id }) => {
            println!("Joined room '{room_id}' as {name} (id: {client_id})");
        }
        _ => {
            eprintln!("Unexpected response from server");
            return Ok(());
        }
    }

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    loop {
        tokio::select! {
            // Read from stdin
            result = lines.next_line() => {
                match result {
                    Ok(Some(text)) if !text.is_empty() => {
                        // Show our own message locally
                        // println!("[{name}]: {text}");
                        // Broadcast to peers
                        let msg = format!("{name}: {text}");
                        client.broadcast(msg.as_bytes()).await?;
                    }
                    _ => break, // EOF
                }
            }

            // Receive from server
            result = client.recv() => {
                match result {
                    #[allow(clippy::single_match)]
                    Ok(Some(event)) => match event {
                        ServerEvent::Broadcast { data, .. } => {
                            let text = String::from_utf8_lossy(&data);
                            println!("{text}");
                        }
                        // ServerEvent::PlayerJoined { client_id } => {
                        //     println!(">> {client_id} joined the room");
                        // }
                        // ServerEvent::PlayerLeft { client_id } => {
                        //     println!(">> {client_id} left the room");
                        // }
                        _ => {}
                    },
                    Ok(None) => {
                        println!("{}","Disconnected from server.".red());
                        break;
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
