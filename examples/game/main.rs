mod debug;
mod fx;
mod model;

use crate::debug::render_debug;
use crate::fx::ParticleSystem;
use crate::model::Pokedex;
use ::rand::rng;
use ::rand::RngExt;
use dashmap::DashMap;
use ghost_sync::{Client, ServerEvent, Uuid};
use macroquad::prelude::*;
use std::f32::consts::PI;
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

impl Anim {
    #[inline]
    fn clip_for(def: &model::Pokemon, anim_kind: AnimKind) -> &Anim {
        match anim_kind {
            AnimKind::Idle => &def.idle,
            AnimKind::Walk => &def.walk,
            AnimKind::Attack => &def.atk,
        }
    }
}

#[derive(SchemaRead, SchemaWrite, Debug, Clone, Copy, PartialEq, Eq)]
enum AnimKind {
    Idle,
    Walk,
    Attack,
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

    #[inline]
    fn facing_to_vec(facing: Self) -> Vec2 {
        match facing {
            Facing::North => vec2(0.0, -1.0),
            Facing::NorthEast => vec2(0.707, -0.707),
            Facing::East => vec2(1.0, 0.0),
            Facing::SouthEast => vec2(0.707, 0.707),
            Facing::South => vec2(0.0, 1.0),
            Facing::SouthWest => vec2(-0.707, 0.707),
            Facing::West => vec2(-1.0, 0.0),
            Facing::NorthWest => vec2(-0.707, -0.707),
        }
    }
}

#[derive(SchemaRead, SchemaWrite, Debug, Clone, Copy, PartialEq, Eq)]
enum PokemonKind {
    Lugia,
    Latias,
    Latios,
    Articuno,
    Zapdos,
    Moltres,
}

static DEFAULT_ADDR: &str = "127.0.0.1:7777";
static SERVER_ADDR: OnceLock<String> = OnceLock::new();
const ROOM_ID: &str = "test-room";

/// Player state that can be broadcast over the network.
/// This is a compact representation of the player's current
#[derive(SchemaRead, SchemaWrite, Clone, Copy)]
struct PlayerState {
    x: f32,
    y: f32,
    facing: Facing,
    anim_kind: AnimKind,
    frame_index: usize,
    frame_timer: f32,
    pokemon_kind: PokemonKind,
    is_attacking: bool,
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
    is_moving: bool,
    is_attacking: bool,
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
            is_moving: false,
            is_attacking: false,
        }
    }

    #[inline]
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
            is_attacking: self.is_attacking,
        }
    }

    #[inline]
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
            is_moving: false,
            is_attacking: false,
        }
    }

    #[inline]
    /// Overwrite the player state with the received state from network
    fn apply_state(&mut self, state: &PlayerState) {
        self.target_pos = vec2(state.x, state.y);
        self.facing = state.facing;
        self.anim_kind = state.anim_kind;
        self.frame_index = state.frame_index;
        self.frame_timer = state.frame_timer;
        self.pokemon_kind = state.pokemon_kind;
        self.is_moving = false;
        self.is_attacking = state.is_attacking;
    }
}

/// A Master state holding all game data, including local player and remote players received from network
struct GameState {
    local_player: Player,
    #[allow(dead_code)]
    is_fullscreen: bool,
    remote_players: DashMap<Uuid, Player>,
    self_id: Option<Uuid>,
    state_tx: Option<mpsc::UnboundedSender<PlayerState>>,
    net_rx: Option<mpsc::UnboundedReceiver<NetEvent>>,
    tx_count: AtomicU64,
    rx_count: AtomicU64,
    particles: ParticleSystem,
}

impl GameState {
    fn offline(local_player: Player) -> Self {
        Self {
            local_player,
            is_fullscreen: false,
            remote_players: DashMap::new(),
            self_id: None,
            state_tx: None,
            net_rx: None,
            tx_count: AtomicU64::new(0),
            rx_count: AtomicU64::new(0),
            particles: ParticleSystem::new(),
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
                        is_fullscreen: false,
                        remote_players: DashMap::new(),
                        self_id: Some(client_id),
                        state_tx: Some(state_tx),
                        net_rx: Some(net_rx),
                        tx_count: AtomicU64::new(0),
                        rx_count: AtomicU64::new(0),
                        particles: ParticleSystem::new(),
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
        update_player(&mut self.local_player, dt, pokedex, &mut self.particles);

        // Spawn particles for remote players if they are attacking
        for remote in self.remote_players.iter_mut() {
            if remote.is_attacking {
                spawn_attack_particles(&mut self.particles, &remote, pokedex, dt);
            }
        }

        self.poll_network_events();
        self.send_local_state();
        self.lerp_remote_players(dt);
        self.particles.update(dt);
    }

    #[inline]
    /// Get current player state for network broadcast.
    fn get_local_state(&self) -> PlayerState {
        self.local_player.to_state()
    }

    /// True if we have joined a room (Server assigns a client_id).
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

    #[inline]
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

    fn render_particles(&self) {
        self.particles.render();
    }

    fn lerp_remote_players(&mut self, dt: f32) {
        let _alpha = dt.clamp(0.0, 1.0);
        for mut remote in self.remote_players.iter_mut() {
            remote.pos = remote.pos.lerp(remote.target_pos, 0.5);
        }
    }

    fn draw_debug(&self, pokedex: &Pokedex, dt: f32) {
        render_debug(&self.local_player, dt, pokedex, self);

        // let def = pokedex.get(self.local_player.pokemon_kind);
        // let cfg = &def.particle_config;
        // let spawn_pos = self.local_player.pos + cfg.offsets[self.local_player.facing as usize];
        // draw_circle(spawn_pos.x, spawn_pos.y, 5.0, RED);
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
/// Spawn particles for a Pokémon's attack in the direction they're facing
fn spawn_attack_particles(
    particles: &mut ParticleSystem,
    player: &Player,
    pokedex: &Pokedex,
    dt: f32,
) {
    let def = pokedex.get(player.pokemon_kind);
    let cfg = &def.particle_config;

    let clip = Anim::clip_for(def, AnimKind::Attack);

    // Attack delay before particles start spawning, so they sync with the animation frames.
    // Calculated by summing frame durations up to current frame + timer.
    {
        let mut elapsed = 0.0;
        for i in 0..player.frame_index {
            elapsed += clip.frame_durations[i];
        }
        elapsed += player.frame_timer;

        if elapsed < cfg.attack_delay {
            return;
        }
    }

    let dir = Facing::facing_to_vec(player.facing);

    let spawn_pos = player.pos + cfg.offsets[player.facing as usize];

    // Spawn rate is particles per second
    // So we multiply by dt to get particles this frame
    let num_particles = (cfg.spawn_rate * dt).ceil() as usize;

    // Iterate all effects and spawn particles for each
    for effect in cfg.effects {
        for _ in 0..num_particles {
            let speed = rand::gen_range(cfg.speed_min, cfg.speed_max);
            let lifetime = rand::gen_range(cfg.lifetime_min, cfg.lifetime_max);
            let scale = rand::gen_range(cfg.scale_min, cfg.scale_max);
            let texture_index = rng().random_range(0..2);

            // TODO: Use this for spread
            let radians = (3.0 * PI) / 180.0;
            // Add some randomness spread
            let angle_offset = rand::gen_range(-radians, radians);
            let angle = dir.y.atan2(dir.x) + angle_offset;
            let vel = vec2(angle.cos(), angle.sin()) * speed;

            particles.spawn(
                spawn_pos.x,
                spawn_pos.y,
                vel.x,
                vel.y,
                lifetime,
                cfg.r,
                cfg.g,
                cfg.b,
                cfg.a,
                scale,
                *effect,
                texture_index,
            );
        }
    }
}

#[inline]
fn update_player(player: &mut Player, dt: f32, pokedex: &Pokedex, particles: &mut ParticleSystem) {
    let (axis_x, axis_y) = read_axis_input();
    player.is_moving = axis_x != 0 || axis_y != 0;

    let def = pokedex.get(player.pokemon_kind);

    let attack_triggered = is_key_pressed(KeyCode::J) && !player.is_attacking;

    if attack_triggered {
        start_anim(player, AnimKind::Attack);
        player.is_attacking = true;
    }

    // If attacking, run one-shot attack (locks movement)
    if player.is_attacking {
        // Spawn attack particles continuously during attack
        spawn_attack_particles(particles, player, pokedex, dt);

        let clip = Anim::clip_for(def, AnimKind::Attack);
        let finished = advance_once(player, clip.frame_durations, dt);

        if finished {
            player.is_attacking = false;
            start_anim(
                player,
                if player.is_moving {
                    AnimKind::Walk
                } else {
                    AnimKind::Idle
                },
            );
        }
    }
    // Normal locomotion
    else {
        if player.is_moving {
            player.facing = Facing::from_axes(axis_x, axis_y, player.facing);
            let direction = vec2(axis_x as f32, axis_y as f32).normalize();
            let new_pos = player.pos + direction * def.speed * dt;
            player.pos = player.pos.lerp(new_pos, 0.90);
        }

        // Switch animation only on state change
        let desired = if player.is_moving {
            AnimKind::Walk
        } else {
            AnimKind::Idle
        };
        if desired != player.anim_kind {
            start_anim(player, desired);
        }

        // Loop idle/walk
        let clip = Anim::clip_for(def, player.anim_kind);
        advance_loop(player, clip.frame_durations, dt);
    }

    // Clamp position to screen bounds
    let clip = Anim::clip_for(def, player.anim_kind);
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
fn start_anim(player: &mut Player, kind: AnimKind) {
    player.anim_kind = kind;
    player.frame_index = 0;
    player.frame_timer = 0.0;
}

#[inline]
/// Forward the animation by dt, return true if it has reached the end of the clip (for one-shot animations like attack)
fn advance_once(player: &mut Player, durations: &[f32], dt: f32) -> bool {
    player.frame_timer += dt;
    while player.frame_timer >= durations[player.frame_index] {
        // Keep consuming time until we find the current frame
        player.frame_timer -= durations[player.frame_index];
        player.frame_index += 1;
        if player.frame_index >= durations.len() {
            // We've reached the end of the clip, return true for completion
            return true;
        }
    }
    false
}

#[inline]
/// Forward the animation by dt, looping back to start after the end of the clip
fn advance_loop(player: &mut Player, durations: &[f32], dt: f32) {
    player.frame_timer += dt;
    while player.frame_timer >= durations[player.frame_index] {
        player.frame_timer -= durations[player.frame_index]; // Consume the time for the current frame
        player.frame_index = (player.frame_index + 1) % durations.len(); // Loop back to start after the end
    }
}

#[inline]
fn render_player(player: &Player, pokedex: &Pokedex) {
    let def = pokedex.get(player.pokemon_kind);
    let clip = Anim::clip_for(def, player.anim_kind);

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
    draw_text(
        label,
        player.pos.x - dest_w * 0.5 - 10.0,
        player.pos.y - dest_h * 0.5 - 15.0,
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

    // Do a little Echo test
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

        // if is_key_pressed(KeyCode::F) {
        //     game_state.is_fullscreen = !game_state.is_fullscreen;
        //     if game_state.is_fullscreen {
        //         set_fullscreen(true);
        //     } else {
        //         set_fullscreen(false);
        //     }
        // }

        render_world();
        game_state.update(dt, &pokedex);
        game_state.render_particles();
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
        window_width: 998,
        window_height: 768,
        window_resizable: true,
        fullscreen: false,
        sample_count: 4096,
        ..Default::default()
    }
}

async fn init() -> anyhow::Result<(bool, PokemonKind)> {
    use cliclack::*;

    let mode = select("Got friends?")
        .item(true, "Online", "Yes!!")
        .item(false, "Offline", "No?")
        .interact()?;

    let addr: String = input("Server address?")
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

    let pokemon_select = select("Pokemon?")
        .item(PokemonKind::Lugia, "Lugia", "")
        .item(PokemonKind::Latias, "Latias", "")
        .item(PokemonKind::Latios, "Latios", "")
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
