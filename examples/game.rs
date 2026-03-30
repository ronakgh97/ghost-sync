use ::rand::rng;
#[allow(unused_imports)]
use ::rand::RngExt;
use macroquad::prelude::*;
use wincode::{SchemaRead, SchemaWrite};

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

struct Anim {
    texture: Texture2D,
    frame_w: f32,
    frame_h: f32,
    frame_durations: &'static [f32],
}

struct Pokemon {
    name: &'static str,
    idle: Anim,
    walk: Anim,
    facing_rows: [usize; 8],
    speed: f32,
    scale: f32,
}

struct Pokedex {
    lugia: Pokemon,
    latios: Pokemon,
    latias: Pokemon,
    articuno: Pokemon,
}

impl Pokedex {
    fn load() -> Self {
        Self {
            lugia: Self::load_lugia(),
            latios: Self::load_latios(),
            latias: Self::load_latias(),
            articuno: Self::load_articuno(),
        }
    }

    fn get(&self, kind: PokemonKind) -> &Pokemon {
        match kind {
            PokemonKind::Lugia => &self.lugia,
            PokemonKind::Latios => &self.latios,
            PokemonKind::Latias => &self.latias,
            PokemonKind::Articuno => &self.articuno,
        }
    }

    fn load_lugia() -> Pokemon {
        let idle = Texture2D::from_file_with_format(
            include_bytes!("../assets/sprites/Lugia/Idle-Anim.png"),
            Some(ImageFormat::Png),
        );
        idle.set_filter(FilterMode::Nearest);

        let walk = Texture2D::from_file_with_format(
            include_bytes!("../assets/sprites/Lugia/Walk-Anim.png"),
            Some(ImageFormat::Png),
        );
        walk.set_filter(FilterMode::Nearest);

        Pokemon {
            name: "Lugia",
            idle: Anim {
                texture: idle,
                frame_w: 72.0,
                frame_h: 96.0,
                frame_durations: &[0.5, 0.5],
            },
            walk: Anim {
                texture: walk,
                frame_w: 80.0,
                frame_h: 96.0,
                frame_durations: &[1.0 / 5.0, 1.0 / 5.0],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 200.0,
            scale: 2.0,
        }
    }

    fn load_latios() -> Pokemon {
        let idle = Texture2D::from_file_with_format(
            include_bytes!("../assets/sprites/Latios/Idle-Anim.png"),
            Some(ImageFormat::Png),
        );
        idle.set_filter(FilterMode::Nearest);

        let walk = Texture2D::from_file_with_format(
            include_bytes!("../assets/sprites/Latios/Walk-Anim.png"),
            Some(ImageFormat::Png),
        );
        walk.set_filter(FilterMode::Nearest);

        Pokemon {
            name: "Latios",
            idle: Anim {
                texture: idle,
                frame_w: 64.0,
                frame_h: 80.0,
                frame_durations: &[1.0 / 8.0; 6],
            },
            walk: Anim {
                texture: walk,
                frame_w: 64.0,
                frame_h: 80.0,
                frame_durations: &[1.0 / 10.0; 12],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 240.0,
            scale: 2.0,
        }
    }

    fn load_latias() -> Pokemon {
        let idle = Texture2D::from_file_with_format(
            include_bytes!("../assets/sprites/Latias/Idle-Anim.png"),
            Some(ImageFormat::Png),
        );
        idle.set_filter(FilterMode::Nearest);

        let walk = Texture2D::from_file_with_format(
            include_bytes!("../assets/sprites/Latias/Walk-Anim.png"),
            Some(ImageFormat::Png),
        );
        walk.set_filter(FilterMode::Nearest);

        Pokemon {
            name: "Latias",
            idle: Anim {
                texture: idle,
                frame_w: 48.0,
                frame_h: 64.0,
                frame_durations: &[1.0 / 8.0; 6],
            },
            walk: Anim {
                texture: walk,
                frame_w: 48.0,
                frame_h: 64.0,
                frame_durations: &[1.0 / 12.0; 12],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 220.0,
            scale: 2.0,
        }
    }

    fn load_articuno() -> Pokemon {
        let idle = Texture2D::from_file_with_format(
            include_bytes!("../assets/sprites/Articuno/Idle-Anim.png"),
            Some(ImageFormat::Png),
        );
        idle.set_filter(FilterMode::Nearest);

        let walk = Texture2D::from_file_with_format(
            include_bytes!("../assets/sprites/Articuno/Walk-Anim.png"),
            Some(ImageFormat::Png),
        );
        walk.set_filter(FilterMode::Nearest);

        Pokemon {
            name: "Articuno",
            idle: Anim {
                texture: idle,
                frame_w: 88.0,
                frame_h: 88.0,
                frame_durations: &[1.0 / 3.0, 1.0 / 4.0, 1.0 / 6.0, 1.0 / 3.0],
            },
            walk: Anim {
                texture: walk,
                frame_w: 88.0,
                frame_h: 88.0,
                frame_durations: &[1.0 / 14.0, 1.0 / 10.0, 1.0 / 12.0, 1.0 / 10.0],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 260.0,
            scale: 2.0,
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
}

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

struct Player {
    pos: Vec2,
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
            facing: Facing::South,
            anim_kind: AnimKind::Idle,
            frame_index: 0,
            frame_timer: 0.0,
            pokemon_kind,
        }
    }

    /// Form a state of local players
    #[allow(unused)]
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
    #[allow(unused)]
    fn from_state(state: &PlayerState) -> Self {
        Player {
            pos: Vec2::new(state.x, state.y),
            facing: state.facing,
            anim_kind: state.anim_kind,
            frame_index: 0,
            frame_timer: 0.0,
            pokemon_kind: state.pokemon_kind,
        }
    }
}

struct GameState {
    local_player: Player,
    remote_players: Vec<Player>,
}

impl GameState {
    /// Start with a local player
    fn new(local_player: Player) -> Self {
        Self {
            local_player,
            remote_players: Vec::new(),
        }
    }

    /// Add a remote player for Game state to manage it
    #[allow(unused)]
    fn add_remote_player(&mut self, remote_player: Player) {
        self.remote_players.push(remote_player);
    }

    /// Update local player state based on input and elapsed time,
    fn update(&mut self, dt: f32, pokedex: &Pokedex) {
        update_player(&mut self.local_player, dt, pokedex);
    }

    /// A global render fn for rendering both remote and local players
    /// Local player will direct Player and remotes use PlayerState received from network to render
    fn render_players(&self, pokedex: &Pokedex) {
        for remote in &self.remote_players {
            render_player(remote, pokedex);
        }
        render_player(&self.local_player, pokedex);
    }

    fn draw_debug(&self, pokedex: &Pokedex, dt: f32) {
        debug_draw(&self.local_player, dt, pokedex);
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
        player.pos = new_pos.lerp(player.pos, 0.05);
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
        .clamp(clip.frame_w, screen_height() - clip.frame_w);
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
fn debug_draw(player: &Player, dt: f32, pokedex: &Pokedex) {
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
    let debug_panel_w = 656.0;
    let debug_panel_h = 225.0;

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
        WHITE,
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
        WHITE,
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
        WHITE,
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
    let status_color = if valid {
        Color::from_rgba(96, 245, 160, 255)
    } else {
        Color::from_rgba(255, 110, 110, 255)
    };
    draw_text(
        &format!(
            "Src_bounds: {}  Row_bounds: {} Frame_bounds: {}  Timer_bounds: {}",
            source_in_bounds, row_in_bounds, frame_in_bounds, frame_timer_in_bounds
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        status_color,
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

#[macroquad::main(window_conf)]
async fn main() {
    let pokedex = Pokedex::load();
    let _rng = ::rand::rng();

    let local_player = Player::new(
        vec2(screen_width() / 2.0, screen_height() / 2.0),
        PokemonKind::Latias,
    );

    let mut game_state = GameState::new(local_player);

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
}

fn window_conf() -> Conf {
    Conf {
        window_title: String::from("Ghost Sync - MMO Test"),
        window_width: 1024,
        window_height: 768,
        window_resizable: true,
        fullscreen: false,
        sample_count: 1024,
        ..Default::default()
    }
}
