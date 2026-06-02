use crate::model::Pokedex;
use crate::{read_axis_input, Anim, Facing, GameState, Player, DEFAULT_ADDR, SERVER_ADDR};
use macroquad::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, RwLock};
use std::time::Instant;

static MSG_SENT: AtomicU64 = AtomicU64::new(0);
static MSG_RECV: AtomicU64 = AtomicU64::new(0);
static TIMER: LazyLock<Arc<RwLock<Instant>>> =
    LazyLock::new(|| Arc::new(RwLock::new(Instant::now())));

#[inline]
pub fn render_debug(player: &Player, dt: f32, pokedex: &Pokedex, game_state: &GameState) {
    let (axis_x, axis_y) = read_axis_input();
    let pokemon = pokedex.get(player.pokemon_kind);
    let clip = Anim::clip_for(pokemon, player.anim_kind);

    let row = pokemon.facing_rows[player.facing as usize];
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
        format!(
            "FPS: {:>3.0}  Delta: {:.4}s Screen: {:.0}x{:.0} Particles: {}",
            get_fps(),
            dt,
            screen_width(),
            screen_height(),
            game_state.particles.particles.len(),
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        WHITE,
    );
    y += lh;

    draw_text(
        format!(
            "Pokemon: {:<8}  Anim: {:?}  Facing_idx: {}  Mapped_row: {}",
            pokemon.name, player.anim_kind, player.facing as usize, row
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        WHITE,
    );
    y += lh;

    draw_text(
        format!(
            "Input axis: ({:>2}, {:>2})  Pos: ({:.1}, {:.1})  Move: {}  Atk: {}",
            axis_x,
            axis_y,
            player.pos.x,
            player.pos.y,
            if player.is_moving { "Y" } else { "N" },
            if player.is_attacking { "Y" } else { "N" }
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        BLUE,
    );
    y += lh;

    draw_text(
        format!(
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
        format!(
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
        format!(
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
        format!("UV Rect: ({:.3}, {:.3}) -> ({:.3}, {:.3})", u0, v0, u1, v1),
        debug_panel_x + 10.0,
        y,
        fs,
        WHITE,
    );
    y += lh;

    let valid = source_in_bounds && row_in_bounds && frame_in_bounds && frame_timer_in_bounds;
    let status_color = if valid { GREEN } else { RED };
    draw_text(
        format!(
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
        format!(
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

    // TODO: Fix this
    if TIMER.read().unwrap().elapsed().as_secs_f64() >= 1.0 {
        MSG_SENT.store(
            game_state.tx_count.swap(0, Ordering::Relaxed),
            Ordering::Relaxed,
        );
        MSG_RECV.store(
            game_state.rx_count.swap(0, Ordering::Relaxed),
            Ordering::Relaxed,
        );
        *TIMER.write().unwrap() = Instant::now();
    }

    draw_text(
        format!(
            "Remote Clients: '{}' TX: {}/s  RX: {}/s",
            game_state.remote_players.len(),
            MSG_SENT.load(Ordering::Relaxed),
            MSG_RECV.load(Ordering::Relaxed),
        ),
        debug_panel_x + 10.0,
        y,
        fs,
        YELLOW,
    );

    let rows_str = pokemon
        .facing_rows
        .iter()
        .enumerate()
        .map(|(i, r)| format!("{}:{}", i, r))
        .collect::<Vec<_>>()
        .join("  ");
    draw_text(
        format!("Facing row map -> {rows_str}"),
        debug_panel_x + 10.0,
        debug_panel_y + debug_panel_h - 10.0,
        16.0,
        Color::from_rgba(160, 186, 225, 255),
    );

    let dest_w = clip.frame_w * pokemon.scale;
    let dest_h = clip.frame_h * pokemon.scale;
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
    // Visualize the attack particle spawn position continuously
    let def = pokedex.get(player.pokemon_kind);
    let cfg = &def.particle_config;
    let spawn_pos = player.pos + cfg.offsets[player.facing as usize];
    draw_circle(
        spawn_pos.x,
        spawn_pos.y,
        4.0,
        Color::from_rgba(255, 80, 80, 255),
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

        draw_text(format!("{}", i), cx + 2.0, bar_y + 13.0, 14.0, BLACK);

        cx += w;
    }

    if let Some(&cur) = clip.frame_durations.get(player.frame_index) {
        let prog = (player.frame_timer / cur.max(0.0001)).clamp(0.0, 1.0);
        draw_text(
            format!("Frame progress: {:>5.1}%", prog * 100.0),
            timeline_x + 10.0,
            timeline_y + 70.0,
            18.0,
            WHITE,
        );
    }
}
