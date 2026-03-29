use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cliargs::parse();

    match cli.command {
        Some(Command::Create {
            room,
            addr,
            password,
            max_player,
        }) => {
            let room = validate_room_name(&room)?;
            let ip = addr
                .parse::<SocketAddr>()
                .map_err(|e| anyhow!("Invalid address: {e}"))?;
            let created = create_room(&room, ip, password.as_deref(), max_player)
                .await
                .map_err(|e| anyhow!("Failed to create room: {e}"))?;
            println!("Created room: {created}");
        }
        Some(Command::List { addr }) => {
            let ip = addr.parse::<SocketAddr>()?;
            let rooms = list_rooms(ip).await?;
            if rooms.is_empty() {
                println!("No rooms found");
            } else {
                println!("Rooms:");
                for room in rooms {
                    let private_marker = if room.is_private { " [private]" } else { "" };
                    println!(
                        "  {} ({} players){}",
                        room.room_id, room.players, private_marker
                    );
                }
            }
        }
        Some(Command::Echo { payload, addr }) => {
            let ip = addr.parse::<SocketAddr>()?;
            let recv = echo_test(&payload, ip).await?;
            if !recv.eq(&payload) {
                println!("Echo mismatch! Sent: '{payload}', Received: '{recv}'");
            } else {
                println!("Echo successful");
            }
        }
        None => {
            println!("No command provided");
        }
    }
    Ok(())
}

const MAX_ROOM_NAME_LEN: usize = 8;

fn validate_room_name(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Room name is empty"));
    }
    if trimmed.len() > MAX_ROOM_NAME_LEN {
        return Err(anyhow!(
            "Room name too long (max {} chars)",
            MAX_ROOM_NAME_LEN
        ));
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(anyhow!(
            "Room name can only contain letters, digits, '-' and '_'"
        ));
    }
    Ok(trimmed.to_ascii_lowercase())
}

#[derive(Debug)]
struct ControlAPIResponse {
    ok: bool,
    body_lines: Vec<String>,
    error_message: Option<String>,
}

async fn list_rooms(addr: SocketAddr) -> Result<Vec<RoomInfo>> {
    let response = control_command("LIST_ROOMS\n", addr).await?;
    if !response.ok {
        return Err(anyhow!(
            "{}",
            response
                .error_message
                .unwrap_or_else(|| "LIST_ROOMS failed".to_string())
        ));
    }

    let mut rooms = Vec::new();
    for line in response.body_lines {
        if let Some(room) = parse_room_line(&line) {
            rooms.push(room);
        }
    }

    Ok(rooms)
}

async fn create_room(
    room_id: &str,
    addr: SocketAddr,
    password: Option<&str>,
    max_player: Option<usize>,
) -> Result<String> {
    let mut command = format!("CREATE_ROOM\n{room_id}");
    if let Some(pw) = password {
        command.push_str(&format!("\n{pw}"));
    } else {
        command.push('\n');
    }
    if let Some(mp) = max_player {
        command.push_str(&format!("\n{mp}"));
    }
    command.push('\n');

    let response = control_command(&command, addr).await?;
    if !response.ok {
        return Err(anyhow!(
            "{}",
            response
                .error_message
                .unwrap_or_else(|| "CREATE_ROOM failed".to_string())
        ));
    }

    if let Some(line) = response.body_lines.first() {
        return Ok(line.trim().to_string());
    }

    Ok(room_id.to_string())
}

async fn echo_test(payload: &str, addr: SocketAddr) -> Result<String> {
    let command = format!("ECHO_TEST\n{payload}\n");
    let response = control_command(&command, addr).await?;
    if !response.ok {
        return Err(anyhow!(
            "{}",
            response
                .error_message
                .unwrap_or_else(|| "ECHO_TEST failed".to_string())
        ));
    }

    if let Some(line) = response.body_lines.first() {
        return Ok(line.trim().to_string());
    }

    Ok(payload.to_string())
}

#[inline]
fn parse_room_line(line: &str) -> Option<RoomInfo> {
    // LIST_ROOMS body format from daemon API: "{room-id},{client-count}"
    let (room_id, rest) = line.split_once(',')?;
    let room_id = room_id.trim();
    if room_id.is_empty() {
        return None;
    }

    // Parse client count (may have trailing ",private" marker)
    let (count_str, is_private) = if let Some((count, marker)) = rest.split_once(',') {
        (count.trim(), marker.trim() == "private")
    } else {
        (rest.trim(), false)
    };

    let players = count_str.parse::<usize>().ok()?;

    Some(RoomInfo {
        room_id: room_id.to_string(),
        players,
        is_private,
    })
}

#[derive(Debug)]
struct RoomInfo {
    room_id: String,
    players: usize,
    is_private: bool,
}

async fn control_command(command: &str, addr: SocketAddr) -> Result<ControlAPIResponse> {
    let mut stream = TcpStream::connect(addr)
        .await
        .map_err(|e| anyhow!("control connect failed: {e}"))?;

    stream.set_nodelay(true).ok();

    write_frame(&mut stream, command.as_bytes()).await?;
    let response_payload = read_frame(&mut stream).await?;
    parse_control_response(&response_payload)
}

fn parse_control_response(payload: &[u8]) -> Result<ControlAPIResponse> {
    let text = std::str::from_utf8(payload).map_err(|e| anyhow!("invalid UTF-8 response: {e}"))?;
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    if lines.is_empty() {
        return Err(anyhow!("empty control response"));
    }

    let status = lines.remove(0);
    match status.as_str() {
        "OK" => Ok(ControlAPIResponse {
            ok: true,
            body_lines: lines,
            error_message: None,
        }),
        "ERROR" => Ok(ControlAPIResponse {
            ok: false,
            body_lines: Vec::new(),
            error_message: Some(lines.join(" ")),
        }),
        _ => Err(anyhow!("unknown control status: {status}")),
    }
}

async fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let len = stream.read_u32().await? as usize;
    if len > 1024 * 1024 {
        return Err(anyhow!("Frame too large: {len}"));
    }

    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).await?;
    Ok(payload)
}

async fn write_frame(stream: &mut TcpStream, payload: &[u8]) -> Result<()> {
    stream.write_u32(payload.len() as u32).await?;
    stream.write_all(payload).await?;
    stream.flush().await?;
    Ok(())
}

#[derive(Parser)]
#[command(
    name = "gs-daemon-client",
    version = "1.0.0-beta",
    about = "Daemon client for `ghost_sync` generic daemon server",
    long_about = None
)]
struct Cliargs {
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Commands to interact with Control API Server
#[derive(Subcommand)]
enum Command {
    /// Create a room
    Create {
        room: String,

        #[clap(short, long)]
        addr: String,

        /// Room password (optional - creates private room if provided)
        #[clap(short, long)]
        password: Option<String>,

        /// Maximum players allowed (optional)
        #[clap(short = 'n', long)]
        max_player: Option<usize>,
    },

    /// List all rooms
    List {
        #[clap(short, long)]
        addr: String,
    },

    /// Echo test - send a payload and receive it back
    Echo {
        /// The payload to echo
        payload: String,

        #[clap(short, long)]
        addr: String,
    },
}
