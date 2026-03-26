use ::rand::rngs::StdRng;
use ::rand::{RngExt, SeedableRng};
use dashmap::DashMap;
use ghost_sync::{Client, ServerEvent};
use macroquad::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;
use wincode::{SchemaRead, SchemaWrite};

const ADDR: &str = "127.0.0.1:7777";
const ROOM: &str = "test-room";
const BOT_COUNT: usize = 12;
const TICK_RATE: u64 = 24;
const MAX_SPEED: f32 = 400.0;
const SCREEN: f32 = 600.0;

static MSG_SENT: AtomicU64 = AtomicU64::new(0);
static MSG_RECV: AtomicU64 = AtomicU64::new(0);
static BYTES_SENT: AtomicU64 = AtomicU64::new(0);
static BYTES_RECV: AtomicU64 = AtomicU64::new(0);
static DESERIALIZE_FAIL: AtomicU64 = AtomicU64::new(0);
static SERIALIZE_FAIL: AtomicU64 = AtomicU64::new(0);
static RUNTIME: std::sync::LazyLock<tokio::runtime::Runtime> = std::sync::LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(32)
        .enable_io()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime")
});

fn winconfig() -> Conf {
    Conf {
        window_title: String::from("Bot Stress Test"),
        window_width: SCREEN as i32,
        window_height: SCREEN as i32,
        window_resizable: false,
        sample_count: 1,
        ..Default::default()
    }
}

#[derive(SchemaRead, SchemaWrite)]
struct Payload {
    id: Uuid,
    vx: f32,
    vy: f32,
}

struct Bot {
    pos: Vec2,
    vel: Vec2,
}

impl Bot {
    #[inline(always)]
    pub fn new(rng: &mut impl RngExt) -> Self {
        let angle: f32 = rng.random_range(0.0..std::f32::consts::TAU);
        Self {
            pos: Vec2::new(rng.random_range(0.0..SCREEN), rng.random_range(0.0..SCREEN)),
            vel: Vec2::new(angle.cos(), angle.sin()) * MAX_SPEED,
        }
    }

    #[inline(always)]
    pub fn random_velocity(rng: &mut impl RngExt) -> Vec2 {
        let angle: f32 = rng.random_range(0.0..std::f32::consts::TAU);
        let vx = angle.cos() * MAX_SPEED;
        let vy = angle.sin() * MAX_SPEED;

        Vec2::new(vx, vy)
    }

    #[inline(always)]
    pub fn update(&mut self, dt: f32) {
        let target_pos = self.pos + self.vel * dt;
        self.pos = self.pos.lerp(target_pos, 0.275);
        self.pos.x = self.pos.x.rem_euclid(SCREEN);
        self.pos.y = self.pos.y.rem_euclid(SCREEN);
    }

    #[inline(always)]
    pub fn render(&self) {
        draw_circle(self.pos.x, self.pos.y, 5.0, RED);
    }
}

type BotMap = Arc<DashMap<Uuid, Bot>>;

/// Bot entry point: connects, joins room, runs select! loop.
/// One connection per bot
/// Single shared multi-threaded runtime (n workers)
/// select! with biased ensures send tick fires on time
#[inline(always)]
async fn bot_loop(map: BotMap, bot_index: usize) {
    let mut client = match Client::connect(ADDR).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Bot connect error: {e}");
            return;
        }
    };

    if let Err(e) = client.join(ROOM).await {
        eprintln!("Bot join error: {e}");
        return;
    }

    // Keep receiving until we get our Joined event
    let self_id = loop {
        match client.recv().await {
            Ok(Some(ServerEvent::Joined { client_id, .. })) => break client_id,
            Ok(Some(ServerEvent::Error(msg))) => {
                eprintln!("Bot join failed: {msg}");
                return;
            }
            Ok(Some(_)) => continue, // Ignore other events
            Ok(None) => {
                eprintln!("Bot connection closed during join");
                return;
            }
            Err(e) => {
                eprintln!("Bot recv error during join: {e}");
                return;
            }
        }
    };

    // RNG after all awaits
    let mut rng = StdRng::seed_from_u64(bot_index as u64);
    map.insert(self_id, Bot::new(&mut rng));

    // Set Tick limiter
    let mut send_interval = tokio::time::interval(Duration::from_millis(1000 / TICK_RATE));
    send_interval.tick().await;

    loop {
        tokio::select! {
            // Biased to prioritize sending on schedule, preventing backlog buildup and disconnects under load
            biased;

            // Send tick fires on schedule
            _ = send_interval.tick() => {
                if let Some(mut bot) = map.get_mut(&self_id) {
                    let velocity_vec = Bot::random_velocity(&mut rng);

                    let payload = Payload {
                        id: self_id,
                        vx: bot.vel.x,
                        vy: bot.vel.y,
                    };
                    let data = wincode::serialize(&payload).unwrap_or_else(|_| {
                        SERIALIZE_FAIL.fetch_add(1, Ordering::Relaxed);
                        Vec::new()
                    });

                    MSG_SENT.fetch_add(1, Ordering::Relaxed);
                    BYTES_SENT.fetch_add(data.len() as u64, Ordering::Relaxed);

                    let _ = client.broadcast(&data).await;

                    bot.vel = Vec2::new(velocity_vec.x, velocity_vec.y);
                }
            }

            // Recv with higher timeout to prevent starvation, but still detect disconnects
            result = client.recv() => {
                match result {
                    Ok(Some(ServerEvent::Broadcast { sender_id, data })) if sender_id != self_id => {
                        MSG_RECV.fetch_add(1, Ordering::Relaxed);
                        BYTES_RECV.fetch_add(data.len() as u64, Ordering::Relaxed);

                        // Update bot data with new broadcasted data, we will render them in main thread
                        if let Ok(v) = wincode::deserialize::<Payload>(&data) {
                            map.entry(v.id)
                                .and_modify(|b| b.vel = Vec2::new(v.vx, v.vy))
                                .or_insert(Bot {
                                    pos: Vec2::new(rng.random_range(0.0..SCREEN), rng.random_range(0.0..SCREEN)),
                                    vel: Vec2::new(v.vx, v.vy),
                                });
                        } else {
                            DESERIALIZE_FAIL.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Ok(Some(ServerEvent::Broadcast { .. })) => {}
                    Ok(Some(ServerEvent::PlayerLeft { client_id })) => {
                        eprintln!("Bot {} disconnected", self_id);
                        map.remove(&client_id);
                    }
                    Ok(Some(ServerEvent::PlayerJoined { .. })) | Ok(Some(ServerEvent::Error(_))) | Ok(Some(ServerEvent::Joined { .. })) => {}
                    Ok(None) | Err(_)  => {
                        eprintln!("Bot {} disconnected", self_id);
                        map.remove(&self_id);
                        break;
                    }
                }
            }
        }
    }
}

#[macroquad::main(winconfig)]
async fn main() {
    let map: BotMap = Arc::new(DashMap::with_capacity(BOT_COUNT));

    // Spawn all bots on the shared multi-threaded runtime
    for i in 0..BOT_COUNT {
        let m = map.clone();
        RUNTIME.spawn(bot_loop(m, i));
    }

    let mut timer = Instant::now();
    let mut msg_sent = 0u64;
    let mut msg_recv = 0u64;
    let mut bytes_sent = 0u64;
    let mut bytes_recv = 0u64;
    let mut deserialize_fail = 0u64;
    let mut serialize_fail = 0u64;

    loop {
        let delta = get_frame_time();

        clear_background(BLACK);

        let connected = map.len();
        let mut y = 24.0;
        let step = 24.0;

        // Update metrics every second
        if timer.elapsed().as_secs_f32() >= 1.0 {
            msg_sent = MSG_SENT.swap(0, Ordering::Relaxed);
            msg_recv = MSG_RECV.swap(0, Ordering::Relaxed);
            bytes_sent = BYTES_SENT.swap(0, Ordering::Relaxed);
            bytes_recv = BYTES_RECV.swap(0, Ordering::Relaxed);
            deserialize_fail = DESERIALIZE_FAIL.swap(0, Ordering::Relaxed);
            serialize_fail = SERIALIZE_FAIL.swap(0, Ordering::Relaxed);
            timer = Instant::now();
        }

        draw_text(
            &format!("Bots: {connected}/{BOT_COUNT}"),
            10.0,
            y,
            20.0,
            WHITE,
        );
        y += step;

        draw_text(
            &format!("FPS: {:.0}", 1.0 / delta.max(0.001)),
            10.0,
            y,
            20.0,
            WHITE,
        );
        y += step;

        draw_text(&format!("Tick: ~{} Hz", TICK_RATE), 10.0, y, 20.0, WHITE);
        y += step;

        // TX
        draw_text(
            &format!("TX: {} msg/s  {} KB/s", msg_sent, bytes_sent / 1024),
            10.0,
            y,
            20.0,
            GREEN,
        );
        y += step;

        // RX
        draw_text(
            &format!("RX: {} msg/s  {} KB/s", msg_recv, bytes_recv / 1024),
            10.0,
            y,
            20.0,
            BLUE,
        );
        y += step;

        let recipients = connected.saturating_sub(1) as u64;
        let expected_msg_recv = msg_sent.saturating_mul(recipients);
        let expected_bytes_recv = bytes_sent.saturating_mul(recipients);

        let msg_loss_pct = if expected_msg_recv > 0 {
            expected_msg_recv
                .saturating_sub(msg_recv)
                .saturating_mul(100)
                / expected_msg_recv
        } else {
            0
        };

        draw_text(
            &format!("Msg Loss: {}%", msg_loss_pct),
            10.0,
            y,
            20.0,
            YELLOW,
        );

        y += step;

        let byte_loss_pct = if expected_bytes_recv > 0 {
            expected_bytes_recv
                .saturating_sub(bytes_recv)
                .saturating_mul(100)
                / expected_bytes_recv
        } else {
            0
        };

        draw_text(
            &format!("Byte Loss: {}%", byte_loss_pct),
            10.0,
            y,
            20.0,
            YELLOW,
        );

        y += step;

        // Serialize fails
        draw_text(
            &format!("Serialize fails: {serialize_fail}/s"),
            10.0,
            y,
            20.0,
            RED,
        );

        y += step;

        // Deserialize fails
        draw_text(
            &format!("Deserialize fails: {deserialize_fail}/s"),
            10.0,
            y,
            20.0,
            RED,
        );
        // Render all bots in main thread
        for mut entry in map.iter_mut() {
            let bot = entry.value_mut();
            bot.update(delta);
            bot.render();
        }

        next_frame().await;
    }
}
