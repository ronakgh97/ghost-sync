mod model;

use crate::model::Pokedex;
use ::rand::rng;
#[allow(unused_imports)]
use ::rand::RngExt;
use dashmap::DashMap;
use ghost_sync::{Client, ServerEvent, Uuid};
use macroquad::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, OnceLock};
use tokio::sync::mpsc;
use wincode::{SchemaRead, SchemaWrite};

struct Anim {
    texture: Texture2D,
    frame_w: f32,
    frame_h: f32,
    frame_durations: &'static [f32],
}

#[derive(SchemaRead, SchemaWrite, Debug, Clone, Copy, PartialEq, Eq)]
enum AnimKind {
    Idle,
    Walk,
}

#[derive(SchemaRead, SchemaWrite, Clone, Copy)]
#[repr(usize)]
enum Facing {
    South = 0,
    SouthWest = 1,
    West = 2,
    NorthWest = 3,
    North = 4,
    NorthEast = 5,
    East = 6,
    SouthEast = 7,
}

impl Facing {
    #[inline]
    fn from_axes(x: i8, y: i8, fallback: Self) -> Self {
        match (x.signum(), y.signum()) {
            (0, 1) => Self::South,
            (-1, 1) => Self::SouthWest,
            (-1, 0) => Self::West,
            (-1, -1) => Self::NorthWest,
            (0, -1) => Self::North,
            (1, -1) => Self::NorthEast,
            (1, 0) => Self::East,
            (1, 1) => Self::SouthEast,
            _ => fallback,
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(SchemaRead, SchemaWrite, Debug, Clone, Copy, PartialEq, Eq)]
enum PokemonKind {
    Lugia,
    Latios,
    Latias,
    Articuno,
    Zapdos,
    Moltres,
}

static DEFAULT_ADDR: &str = "127.0.0.1:7777";
static SERVER_ADDR: OnceLock<String> = OnceLock::new();
const ROOM_ID: &str = "test-room";

/// Player state that can be broadcast over the network.
/// This is a compact representation of the player's current
#[allow(unused)]
#[derive(SchemaRead, SchemaWrite, Clone, Copy)]
struct PlayerState {
    x: f32,
    y: f32,
    facing: Facing,
    anim_kind: AnimKind,
    frame_index: usize,
    frame_timer: f32,
    pokemon_kind: PokemonKind,
}

/// Events that go from network task TO game thread.
enum NetEvent {
    /// Local player successfully joined the room
    LocalJoin { client_id: Uuid },
    /// A player left the room
    RemoteLeft { client_id: Uuid },
    /// Got broadcast from another player
    RemoteState { sender_id: Uuid, state: PlayerState },
    /// Network error occurred
    Error(String),
    /// Connection lost
    Disconnected,
}

struct Player {
    pos: Vec2,
    target_pos: Vec2,
    facing: Facing,
    anim_kind: AnimKind,
    frame_index: usize,
    frame_timer: f32,
    pokemon_kind: PokemonKind,
}

impl Player {
    fn new(pos: Vec2, pokemon_kind: PokemonKind) -> Self {
        Self {
            pos,
            target_pos: pos,
            facing: Facing::South,
            anim_kind: AnimKind::Idle,
            frame_index: 0,
            frame_timer: 0.0,
            pokemon_kind,
        }
    }

    /// Form a state of local players that can be sent over the network
    fn to_state(&self) -> PlayerState {
        PlayerState {
            x: self.pos.x,
            y: self.pos.y,
            facing: self.facing,
            anim_kind: self.anim_kind,
            frame_index: self.frame_index,
            frame_timer: self.frame_timer,
            pokemon_kind: self.pokemon_kind,
        }
    }

    /// Get a state from the network
    /// Can be used to smooth interpolation of remote players in client side
    fn from_state(state: &PlayerState) -> Self {
        let pos = Vec2::new(state.x, state.y);
        Player {
            pos,
            target_pos: pos,
            facing: state.facing,
            anim_kind: state.anim_kind,
            frame_index: state.frame_index,
            frame_timer: state.frame_timer,
            pokemon_kind: state.pokemon_kind,
        }
    }

    /// Overwrite the player state with the received state from network
    fn apply_state(&mut self, state: &PlayerState) {
        self.target_pos = vec2(state.x, state.y);
        self.facing = state.facing;
        self.anim_kind = state.anim_kind;
        self.frame_index = state.frame_index;
        self.frame_timer = state.frame_timer;
        self.pokemon_kind = state.pokemon_kind;
    }
}

/// A Master state holding all game data, including local player and remote players received from network
struct GameState {
    local_player: Player,
    remote_players: DashMap<Uuid, Player>,
    self_id: Option<Uuid>,
    state_tx: Option<mpsc::UnboundedSender<PlayerState>>,
    net_rx: Option<mpsc::UnboundedReceiver<NetEvent>>,
    tx_count: AtomicU64,
    rx_count: AtomicU64,
}

impl GameState {
    fn offline(local_player: Player) -> Self {
        Self {
            local_player,
            remote_players: DashMap::new(),
            self_id: None,
            state_tx: None,
            net_rx: None,
            tx_count: AtomicU64::new(0),
            rx_count: AtomicU64::new(0),
        }
    }

    /// Start in online mode,  connects to server, runs echo test, joins room, then returns.
    /// BLOCKS until connection + echo test + room join all succeed.
    async fn online(local_player: Player) -> Result<Self, String> {
        let (state_tx, state_rx) = mpsc::unbounded_channel::<PlayerState>();
        let (net_tx, mut net_rx) = mpsc::unbounded_channel::<NetEvent>();

        // Spawn a background network thread, from here we will poll events into our local game thread via net_rx
        // and send our state every frame via state_tx
        ASYNC_RUNTIME.spawn(network_loop(state_rx, net_tx));

        // Startup loop, runs for one EVENT only, either JOIN room or ERROR return
        loop {
            match net_rx.recv().await {
                Some(NetEvent::LocalJoin { client_id }) => {
                    return Ok(Self {
                        local_player,
                        remote_players: DashMap::new(),
                        self_id: Some(client_id),
                        state_tx: Some(state_tx),
                        net_rx: Some(net_rx),
                        tx_count: AtomicU64::new(0),
                        rx_count: AtomicU64::new(0),
                    });
                }
                Some(NetEvent::Error(msg)) => return Err(msg),
                Some(NetEvent::Disconnected) => {
                    return Err("Disconnected while waiting for room join".to_string())
                }
                None => return Err("Network setup channel closed".to_string()),
                _ => {}
            }
        }
    }

    /// Update local player + network sync.
    /// Runs every frame: input -> local update -> poll network -> send state -> lerp remotes.
    fn update(&mut self, dt: f32, pokedex: &Pokedex) {
        update_player(&mut self.local_player, dt, pokedex);

        self.poll_network_events();
        self.send_local_state();
        self.lerp_remote_players(dt);
    }

    #[inline]
    /// Get current player state for network broadcast.
    fn get_local_state(&self) -> PlayerState {
        self.local_player.to_state()
    }

    /// True if we've successfully joined a room (have a client_id).
    fn has_joined(&self) -> bool {
        self.self_id.is_some()
    }

    /// Pull all pending network events from channel into local queue.
    #[inline]
    fn poll_network_events(&mut self) {
        let mut pending = Vec::new();

        if let Some(net_rx) = self.net_rx.as_mut() {
            while let Ok(evt) = net_rx.try_recv() {
                pending.push(evt);
            }
        }

        for evt in pending {
            self.process_network_event(evt);
        }
    }

    /// Send our current state to server (if joined). Called every frame.
    fn send_local_state(&mut self) {
        if !self.has_joined() {
            return;
        }

        if let Some(state_tx) = &self.state_tx {
            let _ = state_tx.send(self.get_local_state());
            self.tx_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[inline]
    /// Handle incoming network event - update self_id, remote players, or log error.
    fn process_network_event(&mut self, event: NetEvent) {
        match event {
            NetEvent::LocalJoin { client_id } => {
                self.self_id = Some(client_id);
            }
            NetEvent::RemoteState { sender_id, state } => {
                self.rx_count.fetch_add(1, Ordering::Relaxed);
                // Ignore self
                if self.self_id == Some(sender_id) {
                    return;
                }

                if let Some(mut remote) = self.remote_players.get_mut(&sender_id) {
                    remote.apply_state(&state);
                } else {
                    self.remote_players
                        .insert(sender_id, Player::from_state(&state));
                }
            }
            NetEvent::RemoteLeft { client_id } => {
                self.remote_players.remove(&client_id);
            }
            NetEvent::Disconnected => {
                self.self_id = None;
                self.remote_players.clear();
                self.state_tx = None;
                self.net_rx = None;
            }
            _ => {}
        }
    }

    /// A global render fn for rendering both remote and local players
    /// Local player will direct Player and remotes use PlayerState received from network to render
    fn render_players(&self, pokedex: &Pokedex) {
        for remote in self.remote_players.iter() {
            render_player(remote.value(), pokedex);
        }
        render_player(&self.local_player, pokedex);
    }

    fn lerp_remote_players(&mut self, dt: f32) {
        let _alpha = dt.clamp(0.0, 1.0);
        for mut remote in self.remote_players.iter_mut() {
            remote.pos = remote.pos.lerp(remote.target_pos, 0.5);
        }
    }

    fn draw_debug(&self, pokedex: &Pokedex, dt: f32) {
        render_debug(&self.local_player, dt, pokedex, self);
    }
}

#[inline]
fn read_axis_input() -> (i8, i8) {
    let mut x = 0i8;
    let mut y = 0i8;

    if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
        x -= 1;
    }
    if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
        x += 1;
    }
    if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
        y -= 1;
    }
    if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
        y += 1;
    }

    (x.clamp(-1, 1), y.clamp(-1, 1))
}

#[inline]
fn update_player(player: &mut Player, dt: f32, pokedex: &Pokedex) {
    let (axis_x, axis_y) = read_axis_input();
    let moving = axis_x != 0 || axis_y != 0;

    let def = pokedex.get(player.pokemon_kind);

    // Move in facing direction
    if moving {
        player.facing = Facing::from_axes(axis_x, axis_y, player.facing);
        let direction = vec2(axis_x as f32, axis_y as f32).normalize();
        let new_pos = player.pos + direction * def.speed * dt;
        player.pos = player.pos.lerp(new_pos, 0.95);
    }

    let desired_anim = if moving {
        AnimKind::Walk
    } else {
        AnimKind::Idle
    };

    // Reset anim state
    if desired_anim != player.anim_kind {
        player.anim_kind = desired_anim;
        player.frame_index = 0;
        player.frame_timer = 0.0;
    }

    let clip = match player.anim_kind {
        AnimKind::Idle => &def.idle,
        AnimKind::Walk => &def.walk,
    };

    // Advance anim frame
    player.frame_timer += dt;
    while player.frame_timer >= clip.frame_durations[player.frame_index] {
        player.frame_timer -= clip.frame_durations[player.frame_index];
        player.frame_index = (player.frame_index + 1) % clip.frame_durations.len();
        // Mod that
    }

    // TODO: We need better logic here? Maybe a dedicated Collision box for each
    // Clamp at frame size
    // let half_w = (clip.frame_w * player.pokemon.scale()) * 0.5;
    // let half_h = (clip.frame_h * player.pokemon.scale()) * 0.5;
    player.pos.x = player
        .pos
        .x
        .clamp(clip.frame_w, screen_width() - clip.frame_w);
    player.pos.y = player
        .pos
        .y
        .clamp(clip.frame_h, screen_height() - clip.frame_h);
}

#[inline]
fn render_player(player: &Player, pokedex: &Pokedex) {
    let def = pokedex.get(player.pokemon_kind);
    let clip = match player.anim_kind {
        AnimKind::Idle => &def.idle,
        AnimKind::Walk => &def.walk,
    };

    // Get which row to render
    let row = def.facing_rows[player.facing as usize] as f32;
    // Get the frame offset
    let source = Rect::new(
        player.frame_index as f32 * clip.frame_w,
        row * clip.frame_h,
        clip.frame_w,
        clip.frame_h,
    );

    // Scale it
    let dest_w = clip.frame_w * def.scale;
    let dest_h = clip.frame_h * def.scale;

    draw_texture_ex(
        &clip.texture,
        player.pos.x - dest_w * 0.5,
        player.pos.y - dest_h * 0.5,
        WHITE,
        DrawTextureParams {
            source: Some(source),                  // Render that exact frame
            dest_size: Some(vec2(dest_w, dest_h)), // Exact scale
            ..Default::default()
        },
    );

    let label = def.name;
    let text_dims = measure_text(label, None, 24, 1.0);
    draw_text(
        label,
        player.pos.x - text_dims.width * 0.5,
        player.pos.y - dest_h * 0.5 - 8.0,
        24.0,
        WHITE,
    );
}

#[inline]
fn render_world() {
    clear_background(Color::from_rgba(22, 30, 44, 255));

    let random = rng().random_range(1.0..5.0);

    let grid = 48.0;
    let width = screen_width();
    let height = screen_height();
    let line_color = Color::from_rgba(36, 46, 64, 255);

    let mut x = 0.0;
    while x <= width {
        draw_line(x, 0.0, x, height, random, line_color);
        x += grid;
    }

    let mut y = 0.0;
    while y <= height {
        draw_line(0.0, y, width, y, random, line_color);
        y += grid;
    }
}

#[inline]
fn render_debug(player: &Player, dt: f32, pokedex: &Pokedex, game_state: &GameState) {
    let (axis_x, axis_y) = read_axis_input();
    let def = pokedex.get(player.pokemon_kind);
    let clip = match player.anim_kind {
        AnimKind::Idle => &def.idle,
        AnimKind::Walk => &def.walk,
    };

    let row = def.facing_rows[player.facing as usize];
    let source = Rect::new(
        player.frame_index as f32 * clip.frame_w,
        row as f32 * clip.frame_h,
        clip.frame_w,
        clip.frame_h,
    );

    let tex_w = clip.texture.width();
    let tex_h = clip.texture.height();
    let cols = (tex_w / clip.frame_w).round() as usize;
    let rows = (tex_h / clip.frame_h).round() as usize;

    let source_in_bounds = source.x >= 0.0
        && source.y >= 0.0
        && source.x + source.w <= tex_w + 0.001
        && source.y + source.h <= tex_h + 0.001;
    let row_in_bounds = row < rows;
    let frame_in_bounds = player.frame_index < cols;
    let frame_timer_in_bounds = player.frame_index < clip.frame_durations.len()
        && player.frame_timer <= clip.frame_durations[player.frame_index] + 0.001;

    let debug_panel_x = 12.0;
    let debug_panel_y = 12.0;
    let debug_panel_w = 660.0;
    let debug_panel_h = 245.0;

    draw_rectangle(
        debug_panel_x,
        debug_panel_y,
        debug_panel_w,
        debug_panel_h,
        Color::new(0.05, 0.07, 0.11, 0.84),
    );
    draw_rectangle_lines(
        debug_panel_x,
        debug_panel_y,
        debug_panel_w,
        debug_panel_h,
        2.0,
        Color::from_rgba(140, 168, 212, 255),
    );

    let mut y = debug_panel_y + 22.0;
    let lh = 18.0;
    let fs = 20.0;

    draw_text(
        "DEBUG OVERLAY (animation/runtime)",
        debug_panel_x + 10.0,
        y,
        fs,
        Color::from_rgba(188, 227, 255, 255),
    );
    y += lh;

    draw_text(
        &format!(
            "FPS: {:>3.0}  Delta: {:.4}s ({:.2}ms)  Screen: {:.0}x{:.0}",
            get_fps(),
            dt,
            dt * 1000.0,
            screen_width(),
            screen_height()
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        WHITE,
    );
    y += lh;

    draw_text(
        &format!(
            "Pokemon: {:<8}  Anim: {:?}  Facing_idx: {}  Mapped_row: {}",
            def.name, player.anim_kind, player.facing as usize, row
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        WHITE,
    );
    y += lh;

    draw_text(
        &format!(
            "Input axis: ({:>2}, {:>2})  Pos: ({:>7.2}, {:>7.2})  Speed: {:.1}  Scale: {:.2}",
            axis_x, axis_y, player.pos.x, player.pos.y, def.speed, def.scale
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        BLUE,
    );
    y += lh;

    draw_text(
        &format!(
            "Texture: {:.0}x{:.0}  Frame: {:.0}x{:.0}  Grid: cols={} rows={}",
            tex_w, tex_h, clip.frame_w, clip.frame_h, cols, rows
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        WHITE,
    );
    y += lh;

    draw_text(
        &format!(
            "Frame_index: {} / {}  Frame_timer: {:.3} / {:.3}",
            player.frame_index,
            clip.frame_durations.len().saturating_sub(1),
            player.frame_timer,
            clip.frame_durations
                .get(player.frame_index)
                .copied()
                .unwrap_or_default()
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        GRAY,
    );
    y += lh;

    draw_text(
        &format!(
            "Source Rect: x={:.1} y={:.1} w={:.1} h={:.1}",
            source.x, source.y, source.w, source.h
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        BEIGE,
    );
    y += lh;

    let u0 = source.x / tex_w.max(1.0);
    let v0 = source.y / tex_h.max(1.0);
    let u1 = (source.x + source.w) / tex_w.max(1.0);
    let v1 = (source.y + source.h) / tex_h.max(1.0);
    draw_text(
        &format!("UV Rect: ({:.3}, {:.3}) -> ({:.3}, {:.3})", u0, v0, u1, v1),
        debug_panel_x + 10.0,
        y,
        fs,
        WHITE,
    );
    y += lh;

    let valid = source_in_bounds && row_in_bounds && frame_in_bounds && frame_timer_in_bounds;
    let status_color = if valid { GREEN } else { RED };
    draw_text(
        &format!(
            "Sprites bounds: Src: {}  Row: {} Frame: {}  Timer: {}",
            source_in_bounds, row_in_bounds, frame_in_bounds, frame_timer_in_bounds
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        status_color,
    );
    y += lh;

    let id_text = game_state
        .self_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "<offline>".to_string());
    draw_text(
        &format!(
            "Network: ID: {} Addr: '{}'",
            id_text,
            SERVER_ADDR.get().unwrap_or(&DEFAULT_ADDR.to_string())
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        PURPLE,
    );
    y += lh;

    draw_text(
        &format!(
            "Remote Clients: '{}' TX: {}  RX: {}",
            game_state.remote_players.len(),
            game_state.tx_count.load(Ordering::Relaxed),
            game_state.rx_count.load(Ordering::Relaxed),
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        YELLOW,
    );

    let rows_str = def
        .facing_rows
        .iter()
        .enumerate()
        .map(|(i, r)| format!("{}:{}", i, r))
        .collect::<Vec<_>>()
        .join("  ");
    draw_text(
        &format!("Facing row map -> {rows_str}"),
        debug_panel_x + 10.0,
        debug_panel_y + debug_panel_h - 10.0,
        16.0,
        Color::from_rgba(160, 186, 225, 255),
    );

    let dest_w = clip.frame_w * def.scale;
    let dest_h = clip.frame_h * def.scale;
    let left = player.pos.x - dest_w * 0.5;
    let top = player.pos.y - dest_h * 0.5;

    draw_rectangle_lines(
        left,
        top,
        dest_w,
        dest_h,
        2.0,
        Color::from_rgba(255, 105, 105, 255),
    );
    draw_circle_lines(
        player.pos.x,
        player.pos.y,
        6.0,
        2.0,
        Color::from_rgba(80, 255, 255, 255),
    );
    draw_line(
        player.pos.x - 8.0,
        player.pos.y,
        player.pos.x + 8.0,
        player.pos.y,
        1.0,
        Color::from_rgba(80, 255, 255, 255),
    );
    draw_line(
        player.pos.x,
        player.pos.y - 8.0,
        player.pos.x,
        player.pos.y + 8.0,
        1.0,
        Color::from_rgba(80, 255, 255, 255),
    );

    let facing_dir = match player.facing {
        Facing::South => vec2(0.0, 1.0),
        Facing::SouthWest => vec2(-1.0, 1.0).normalize(),
        Facing::West => vec2(-1.0, 0.0),
        Facing::NorthWest => vec2(-1.0, -1.0).normalize(),
        Facing::North => vec2(0.0, -1.0),
        Facing::NorthEast => vec2(1.0, -1.0).normalize(),
        Facing::East => vec2(1.0, 0.0),
        Facing::SouthEast => vec2(1.0, 1.0).normalize(),
    };

    let arrow_len = 42.0;
    let end = player.pos + facing_dir * arrow_len;
    draw_line(
        player.pos.x,
        player.pos.y,
        end.x,
        end.y,
        2.0,
        Color::from_rgba(255, 210, 80, 255),
    );
    draw_circle(end.x, end.y, 3.0, Color::from_rgba(255, 210, 80, 255));

    let preview_x = screen_width() - 300.0;
    let preview_y = 12.0;
    let preview_w = 288.0;
    let preview_h = 288.0;

    draw_rectangle(
        preview_x,
        preview_y,
        preview_w,
        preview_h,
        Color::new(0.04, 0.06, 0.09, 0.84),
    );
    draw_rectangle_lines(
        preview_x,
        preview_y,
        preview_w,
        preview_h,
        2.0,
        Color::from_rgba(140, 168, 212, 255),
    );
    draw_text(
        "SPRITES SHEET PREVIEW",
        preview_x + 8.0,
        preview_y + 20.0,
        20.0,
        Color::from_rgba(188, 227, 255, 255),
    );

    let tex_scale = ((preview_w - 16.0) / tex_w).min((preview_h - 34.0) / tex_h);
    let scaled_w = tex_w * tex_scale;
    let scaled_h = tex_h * tex_scale;
    let tex_draw_x = preview_x + (preview_w - scaled_w) * 0.5;
    let tex_draw_y = preview_y + 30.0 + (preview_h - 34.0 - scaled_h) * 0.5;

    draw_texture_ex(
        &clip.texture,
        tex_draw_x,
        tex_draw_y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(scaled_w, scaled_h)),
            ..Default::default()
        },
    );

    draw_rectangle_lines(
        tex_draw_x + source.x * tex_scale,
        tex_draw_y + source.y * tex_scale,
        source.w * tex_scale,
        source.h * tex_scale,
        2.0,
        Color::from_rgba(255, 110, 110, 255),
    );

    let timeline_x = screen_width() - 300.0;
    let timeline_y = 312.0;
    let timeline_w = 288.0;
    let timeline_h = 94.0;

    draw_rectangle(
        timeline_x,
        timeline_y,
        timeline_w,
        timeline_h,
        Color::new(0.04, 0.06, 0.09, 0.84),
    );
    draw_rectangle_lines(
        timeline_x,
        timeline_y,
        timeline_w,
        timeline_h,
        2.0,
        Color::from_rgba(140, 168, 212, 255),
    );
    draw_text(
        "ANIM TIMELINE",
        timeline_x + 8.0,
        timeline_y + 20.0,
        20.0,
        Color::from_rgba(188, 227, 255, 255),
    );

    let total: f32 = clip.frame_durations.iter().sum::<f32>().max(0.0001);
    let mut cx = timeline_x + 10.0;
    let bar_y = timeline_y + 34.0;
    let bar_h = 16.0;
    let bar_w_total = timeline_w - 20.0;

    for (i, dur) in clip.frame_durations.iter().copied().enumerate() {
        let w = (dur / total) * bar_w_total;
        let color = if i == player.frame_index {
            Color::from_rgba(255, 200, 90, 255)
        } else {
            Color::from_rgba(90, 130, 200, 255)
        };
        draw_rectangle(cx, bar_y, w.max(1.0), bar_h, color);
        draw_rectangle_lines(cx, bar_y, w.max(1.0), bar_h, 1.0, BLACK);

        draw_text(&format!("{}", i), cx + 2.0, bar_y + 13.0, 14.0, BLACK);

        cx += w;
    }

    if let Some(&cur) = clip.frame_durations.get(player.frame_index) {
        let prog = (player.frame_timer / cur.max(0.0001)).clamp(0.0, 1.0);
        draw_text(
            &format!("Frame progress: {:>5.1}%", prog * 100.0),
            timeline_x + 10.0,
            timeline_y + 70.0,
            18.0,
            WHITE,
        );
    }
}

/// Network task running in background.
/// Flow: connect -> echo test -> join room -> broadcast loop.
#[inline]
async fn network_loop(
    mut state_rx: mpsc::UnboundedReceiver<PlayerState>, // from game thread: our state
    net_tx: mpsc::UnboundedSender<NetEvent>,            // to game thread: remote events
) {
    let mut client =
        match Client::connect(SERVER_ADDR.get().unwrap_or(&DEFAULT_ADDR.to_string())).await {
            Ok(c) => c,
            Err(e) => {
                let _ = net_tx.send(NetEvent::Error(format!("connect failed: {e}")));
                let _ = net_tx.send(NetEvent::Disconnected);
                return;
            }
        };

    // Send known payload, verify byte-by-byte response matches exactly.
    // This catches corrupted connections early.
    let echo_payload = b"ghost-sync-mmo-echo-test";
    if let Err(e) = client.echo_test(echo_payload).await {
        let _ = net_tx.send(NetEvent::Error(format!("echo test send failed: {e}")));
    }

    // Echo recv
    match client.recv().await {
        Ok(Some(ServerEvent::EchoTest { data })) => {
            if data != echo_payload.as_slice() {
                println!("[Net] Echo FAILED: payload mismatch");
                let _ = net_tx.send(NetEvent::Error(
                    "Echo test failed: payload mismatch".to_string(),
                ));
                let _ = net_tx.send(NetEvent::Disconnected);
                return;
            }
            println!("[Net] Echo OK");
        }
        Ok(None) => {
            println!("[Net] Echo: connection closed");
            let _ = net_tx.send(NetEvent::Disconnected);
        }
        Err(e) => {
            println!("[Net] Echo: recv error - {e}");
            let _ = net_tx.send(NetEvent::Error(format!(
                "Recv failed during echo test: {e}"
            )));
        }
        _ => {
            println!("[Net] Echo: unexpected event");
            let _ = net_tx.send(NetEvent::Disconnected);
        }
    }

    if let Err(e) = client.join(ROOM_ID, None).await {
        let _ = net_tx.send(NetEvent::Error(format!("join failed: {e}")));
        let _ = net_tx.send(NetEvent::Disconnected);
        return;
    }

    match client.recv().await {
        Ok(Some(ServerEvent::Joined { client_id, .. })) => {
            println!("[Net] Player joined room as {}", client_id);
            let _ = net_tx.send(NetEvent::LocalJoin { client_id });
        }
        Ok(Some(ServerEvent::Error(msg))) => {
            println!("[Net] Join error: {msg}");
            let _ = net_tx.send(NetEvent::Error(format!("join error: {msg}")));
            let _ = net_tx.send(NetEvent::Disconnected);
            return;
        }
        Ok(Some(evt)) => {
            println!("[Net] Join: unexpected event: {:?}", evt);
            let _ = net_tx.send(NetEvent::Error(
                "Unexpected event while joining room".to_string(),
            ));
        }
        Ok(None) => {
            println!("[Net] Join: connection closed");
            let _ = net_tx.send(NetEvent::Disconnected);
            return;
        }
        Err(e) => {
            println!("[Net] Join: recv error - {e}");
            let _ = net_tx.send(NetEvent::Error(format!("recv failed during join: {e}")));
            let _ = net_tx.send(NetEvent::Disconnected);
            return;
        }
    } // <-- The Startup loop job is done after this point,
      // we should have successfully joined a room and can enter the main broadcast loop.

    // Main broadcast loop: send our state / receive remote states.
    loop {
        tokio::select! {
            // Outbound: got state from game thread -> broadcast over network
            local_state = state_rx.recv() => {
                let Some(state) = local_state else {
                    break;
                };

                let payload = match wincode::serialize(&state) {
                    Ok(p) => p,
                    Err(_) => {
                        //let _ = net_tx.send(NetEvent::Error(format!("serialize player state failed: {e:?}")));
                        continue;
                    }
                };

                if client.broadcast(&payload).await.is_err() {
                    //let _ = net_tx.send(NetEvent::Error(format!("broadcast failed: {e}")));
                    let _ = net_tx.send(NetEvent::Disconnected);
                    break;
                }
            }

            // Inbound: got event from server -> forward to game thread
            recv_result = client.recv() => {
                match recv_result {
                    Ok(Some(ServerEvent::Broadcast { sender_id, data })) => {
                        match wincode::deserialize::<PlayerState>(&data) {
                            Ok(state) => {
                                let _ = net_tx.send(NetEvent::RemoteState { sender_id, state });
                            }
                            Err(e) => {
                                println!("[Net] Broadcast: deserialize failed - {e:?}");
                                // let _ = net_tx.send(NetEvent::Error(format!("deserialize remote state failed: {e:?}")));
                            }
                        }
                    }
                    Ok(Some(ServerEvent::PlayerLeft { client_id })) => {
                        println!("[Net] RemotePlayer {} left", client_id);
                        let _ = net_tx.send(NetEvent::RemoteLeft { client_id });
                    }
                    Ok(Some(ServerEvent::PlayerJoined { client_id })) => {
                        println!("[Net] RemotePlayer {} joined", client_id);
                    }
                    Ok(Some(ServerEvent::Error(msg))) => {
                        println!("[Net] Server error: {msg}");
                        // let _ = net_tx.send(NetEvent::Error(msg));
                    }
                    Ok(Some(ServerEvent::Joined { client_id, .. })) => {
                        println!("[Net] Player re-joined as {}", client_id);
                    }
                    Ok(Some(ServerEvent::EchoTest { .. })) => {
                        println!("[Net] Unexpected EchoTest in main loop");
                    }
                    Ok(None) => {
                        println!("[Net] Connection closed");
                        let _ = net_tx.send(NetEvent::Disconnected);
                        break;
                    }
                    Err(e) => {
                        println!("[Net] Recv error: {e}");
                        // let _ = net_tx.send(NetEvent::Error(format!("recv failed: {e}")));
                        let _ = net_tx.send(NetEvent::Disconnected);
                        break;
                    }
                }
            }
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() -> anyhow::Result<()> {
    // TODO: This does not work as I intended to
    let init = ASYNC_RUNTIME.block_on(init());
    let (mode, pokemon_select) = match init {
        Ok(res) => res,
        Err(e) => {
            cliclack::log::error(format!("Initialization error: {e}"))?;
            std::process::exit(1);
        }
    };
    let pokedex = Pokedex::load();
    let mut rng = ::rand::rng();

    let rand_x = rng.random_range(-100.0..100.0);
    let rand_y = rng.random_range(-100.0..100.0);

    let local_player = Player::new(
        vec2(
            screen_width() / 2.0 + rand_x,
            screen_height() / 2.0 + rand_y,
        ),
        pokemon_select,
    );

    // Use block_on BEFORE entering macroquad's game loop
    // Macroquad doesn't support async/await inside its main loop
    let mut game_state = match mode {
        true => {
            // Run async network init in blocking way BEFORE macroquad starts
            let online_state = ASYNC_RUNTIME.block_on(GameState::online(local_player));
            match online_state {
                Ok(state) => state,
                Err(err) => {
                    cliclack::log::error(format!("Failed to init async game runtime: {err}"))?;
                    std::process::exit(1);
                }
            }
        }
        false => GameState::offline(local_player),
    };

    set_default_filter_mode(FilterMode::Nearest);
    loop {
        let dt = get_frame_time();

        render_world();
        game_state.update(dt, &pokedex);
        game_state.render_players(&pokedex);
        if is_key_down(KeyCode::Space) {
            game_state.draw_debug(&pokedex, dt);
        }
        if is_key_down(KeyCode::Escape) {
            break;
        }
        next_frame().await;
    }
    Ok(())
}

fn window_conf() -> Conf {
    Conf {
        window_title: String::from("Ghost Sync - MMO Test"),
        window_width: 1280,
        window_height: 768,
        window_resizable: true,
        fullscreen: false,
        sample_count: 4096,
        ..Default::default()
    }
}

async fn init() -> anyhow::Result<(bool, PokemonKind)> {
    use cliclack::*;

    let mode = select("Select mode")
        .item(true, "Online", "")
        .item(false, "Offline", "")
        .interact()?;

    let addr: String = input("Game server address?")
        .placeholder("127.0.0.1:7777")
        .default_input(DEFAULT_ADDR)
        .required(true)
        .validate(|input: &String| {
            if input.is_empty() {
                Err("Addr is required!")
            } else {
                Ok(())
            }
        })
        .interact()?;

    SERVER_ADDR
        .set(addr)
        .map_err(|_| anyhow::anyhow!("Failed to set server address"))?;

    let pokemon_select = select("Select your Pokemon")
        .item(PokemonKind::Lugia, "Lugia", "")
        .item(PokemonKind::Latios, "Latios", "")
        .item(PokemonKind::Latias, "Latios", "")
        .item(PokemonKind::Articuno, "Articuno", "")
        .item(PokemonKind::Zapdos, "Zapdos", "")
        .item(PokemonKind::Moltres, "Moltres", "")
        .initial_value(PokemonKind::Lugia)
        .interact()?;

    Ok((mode, pokemon_select))
}

static ASYNC_RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(16)
        .build()
        .expect("Failed to create Tokio runtime")
});
