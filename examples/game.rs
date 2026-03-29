use ::rand::rng;
#[allow(unused_imports)]
use ::rand::RngExt;
use macroquad::prelude::*;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AnimKind {
    Idle,
    Walk,
}

#[derive(Clone, Copy)]
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

trait Pokemon: Send + Sync {
    fn name(&self) -> &'static str;
    fn idle(&self) -> &Anim;
    fn walk(&self) -> &Anim;
    fn facing_rows(&self) -> &'static [usize; 8];
    fn speed(&self) -> f32;
    fn scale(&self) -> f32;
}

struct Lugia {
    idle: Anim,
    walk: Anim,
}

impl Lugia {
    const FACING_ROWS: [usize; 8] = [0, 7, 6, 5, 4, 3, 2, 1];
    const IDLE_DURATIONS: [f32; 2] = [0.5, 0.5];
    const WALK_DURATIONS: [f32; 2] = [1.0 / 5.0, 1.0 / 5.0];

    fn new() -> Self {
        const LUGIA_IDLE_ANIM: &[u8] = include_bytes!("../assets/sprites/Lugia/Idle-Anim.png");
        const LUGIA_WALK_ANIM: &[u8] = include_bytes!("../assets/sprites/Lugia/Walk-Anim.png");

        let idle = Texture2D::from_file_with_format(LUGIA_IDLE_ANIM, Some(ImageFormat::Png));
        idle.set_filter(FilterMode::Nearest);

        let walk = Texture2D::from_file_with_format(LUGIA_WALK_ANIM, Some(ImageFormat::Png));
        walk.set_filter(FilterMode::Nearest);

        Self {
            idle: Anim {
                texture: idle,
                frame_w: 72.0,
                frame_h: 96.0,
                frame_durations: &Self::IDLE_DURATIONS,
            },
            walk: Anim {
                texture: walk,
                frame_w: 80.0,
                frame_h: 96.0,
                frame_durations: &Self::WALK_DURATIONS,
            },
        }
    }
}

impl Pokemon for Lugia {
    fn name(&self) -> &'static str {
        "Lugia"
    }

    fn idle(&self) -> &Anim {
        &self.idle
    }

    fn walk(&self) -> &Anim {
        &self.walk
    }

    fn facing_rows(&self) -> &'static [usize; 8] {
        &Self::FACING_ROWS
    }

    fn speed(&self) -> f32 {
        200.0
    }

    fn scale(&self) -> f32 {
        2.0
    }
}

struct Latios {
    idle: Anim,
    walk: Anim,
}

impl Latios {
    const FACING_ROWS: [usize; 8] = [0, 7, 6, 5, 4, 3, 2, 1];
    const IDLE_DURATIONS: [f32; 6] = [
        1.0 / 8.0,
        1.0 / 8.0,
        1.0 / 8.0,
        1.0 / 8.0,
        1.0 / 8.0,
        1.0 / 8.0,
    ];
    const WALK_DURATIONS: [f32; 12] = [
        1.0 / 10.0,
        1.0 / 10.0,
        1.0 / 10.0,
        1.0 / 10.0,
        1.0 / 10.0,
        1.0 / 10.0,
        1.0 / 10.0,
        1.0 / 10.0,
        1.0 / 10.0,
        1.0 / 10.0,
        1.0 / 10.0,
        1.0 / 10.0,
    ];

    fn new() -> Self {
        const LATIOS_IDLE_ANIM: &[u8] = include_bytes!("../assets/sprites/Latios/Idle-Anim.png");
        const LATIOS_WALK_ANIM: &[u8] = include_bytes!("../assets/sprites/Latios/Walk-Anim.png");

        let idle = Texture2D::from_file_with_format(LATIOS_IDLE_ANIM, Some(ImageFormat::Png));
        idle.set_filter(FilterMode::Nearest);

        let walk = Texture2D::from_file_with_format(LATIOS_WALK_ANIM, Some(ImageFormat::Png));
        walk.set_filter(FilterMode::Nearest);

        Self {
            idle: Anim {
                texture: idle,
                frame_w: 64.0,
                frame_h: 80.0,
                frame_durations: &Self::IDLE_DURATIONS,
            },
            walk: Anim {
                texture: walk,
                frame_w: 64.0,
                frame_h: 80.0,
                frame_durations: &Self::WALK_DURATIONS,
            },
        }
    }
}

impl Pokemon for Latios {
    fn name(&self) -> &'static str {
        "Latios"
    }

    fn idle(&self) -> &Anim {
        &self.idle
    }

    fn walk(&self) -> &Anim {
        &self.walk
    }

    fn facing_rows(&self) -> &'static [usize; 8] {
        &Self::FACING_ROWS
    }

    fn speed(&self) -> f32 {
        240.0
    }

    fn scale(&self) -> f32 {
        2.0
    }
}

struct Latias {
    idle: Anim,
    walk: Anim,
}

impl Latias {
    const FACING_ROWS: [usize; 8] = [0, 7, 6, 5, 4, 3, 2, 1];
    const IDLE_DURATIONS: [f32; 6] = [
        1.0 / 8.0,
        1.0 / 8.0,
        1.0 / 8.0,
        1.0 / 8.0,
        1.0 / 8.0,
        1.0 / 8.0,
    ];
    const WALK_DURATIONS: [f32; 12] = [
        1.0 / 12.0,
        1.0 / 12.0,
        1.0 / 12.0,
        1.0 / 12.0,
        1.0 / 12.0,
        1.0 / 12.0,
        1.0 / 12.0,
        1.0 / 12.0,
        1.0 / 12.0,
        1.0 / 12.0,
        1.0 / 12.0,
        1.0 / 12.0,
    ];

    pub fn new() -> Self {
        const LATIAS_IDLE_ANIM: &[u8] = include_bytes!("../assets/sprites/Latias/Idle-Anim.png");
        const LATIAS_WALK_ANIM: &[u8] = include_bytes!("../assets/sprites/Latias/Walk-Anim.png");

        let idle = Texture2D::from_file_with_format(LATIAS_IDLE_ANIM, Some(ImageFormat::Png));
        idle.set_filter(FilterMode::Nearest);

        let walk = Texture2D::from_file_with_format(LATIAS_WALK_ANIM, Some(ImageFormat::Png));
        walk.set_filter(FilterMode::Nearest);

        Self {
            idle: Anim {
                texture: idle,
                frame_w: 48.0,
                frame_h: 64.0,
                frame_durations: &Self::IDLE_DURATIONS,
            },
            walk: Anim {
                texture: walk,
                frame_w: 48.0,
                frame_h: 64.0,
                frame_durations: &Self::WALK_DURATIONS,
            },
        }
    }
}

impl Pokemon for Latias {
    fn name(&self) -> &'static str {
        "Latias"
    }
    fn idle(&self) -> &Anim {
        &self.idle
    }
    fn walk(&self) -> &Anim {
        &self.walk
    }
    fn facing_rows(&self) -> &'static [usize; 8] {
        &Self::FACING_ROWS
    }
    fn speed(&self) -> f32 {
        220.0
    }
    fn scale(&self) -> f32 {
        2.0
    }
}

struct Player {
    pos: Vec2,
    facing: Facing,
    anim_kind: AnimKind,
    frame_index: usize,
    frame_timer: f32,
    pokemon: Arc<dyn Pokemon>,
}

impl Player {
    fn new(pos: Vec2, pokemon: Arc<dyn Pokemon>) -> Self {
        Self {
            pos,
            facing: Facing::South,
            anim_kind: AnimKind::Idle,
            frame_index: 0,
            frame_timer: 0.0,
            pokemon,
        }
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
fn update_player(player: &mut Player, dt: f32) {
    let (axis_x, axis_y) = read_axis_input();
    let moving = axis_x != 0 || axis_y != 0;

    // Move in facing direction
    if moving {
        player.facing = Facing::from_axes(axis_x, axis_y, player.facing);

        let direction = vec2(axis_x as f32, axis_y as f32).normalize();
        player.pos += direction * player.pokemon.speed() * dt;
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
        AnimKind::Idle => player.pokemon.idle(),
        AnimKind::Walk => player.pokemon.walk(),
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
fn render_player(player: &Player) {
    let clip = match player.anim_kind {
        AnimKind::Idle => player.pokemon.idle(),
        AnimKind::Walk => player.pokemon.walk(),
    };

    // Get which row to render
    let row = player.pokemon.facing_rows()[player.facing as usize] as f32;
    // Get the frame offset
    let source = Rect::new(
        player.frame_index as f32 * clip.frame_w,
        row * clip.frame_h,
        clip.frame_w,
        clip.frame_h,
    );

    // Scale it
    let dest_w = clip.frame_w * player.pokemon.scale();
    let dest_h = clip.frame_h * player.pokemon.scale();

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

    draw_rectangle_lines(
        player.pos.x - dest_w * 0.5,
        player.pos.y - dest_h * 0.5,
        dest_w,
        dest_h,
        1.0,
        RED,
    );

    let label = player.pokemon.name();
    let text_dims = measure_text(label, None, 24, 1.0);
    draw_text(
        label,
        player.pos.x - text_dims.width * 0.5,
        player.pos.y - dest_h * 0.5 - 8.0,
        24.0,
        WHITE,
    );
}

fn render_world() {
    clear_background(Color::from_rgba(22, 30, 44, 255));

    let random = rng().random_range(1.0..3.0);

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

#[macroquad::main(window_conf)]
async fn main() {
    let lugia: Arc<dyn Pokemon> = Arc::new(Lugia::new());
    let _latios: Arc<dyn Pokemon> = Arc::new(Latios::new());
    let _latias: Arc<dyn Pokemon> = Arc::new(Latias::new());

    let _rng = ::rand::rng();

    let mut local_player = Player::new(vec2(screen_width() / 2.0, screen_height() / 2.0), lugia);

    loop {
        let dt = get_frame_time();

        update_player(&mut local_player, dt);
        render_world();
        render_player(&local_player);
        next_frame().await;

        if is_key_down(KeyCode::Escape) {
            break;
        }
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: String::from("Ghost Sync - MMO Test"),
        window_width: 800,
        window_height: 680,
        window_resizable: true,
        fullscreen: true,
        sample_count: 1024,
        ..Default::default()
    }
}
