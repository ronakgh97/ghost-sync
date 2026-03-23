use ::rand::RngExt;
use dashmap::DashMap;
use ghost_sync::{Client, ServerEvent};
use macroquad::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;
use wincode::{SchemaRead, SchemaWrite};

const ADDR: &str = "127.0.0.1:7777";
const ROOM: &str = "test-room";
const BOT_COUNT: usize = 128;
const TICK_RATE: u64 = 60;
const MAX_SPEED: f32 = 400.0;
const SCREEN: f32 = 800.0;

static BYTES_SENT: AtomicU64 = AtomicU64::new(0);
static BYTES_RECV: AtomicU64 = AtomicU64::new(0);

fn winconfig() -> Conf {
    Conf {
        window_title: "Bot Stress Test".into(),
        window_width: SCREEN as i32,
        window_height: SCREEN as i32,
        window_resizable: false,
        sample_count: 512,
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
    pub fn new(rng: &mut impl RngExt) -> Self {
        let angle: f32 = rng.random_range(0.0..std::f32::consts::TAU);
        Self {
            pos: Vec2::new(rng.random_range(0.0..SCREEN), rng.random_range(0.0..SCREEN)),
            vel: Vec2::new(angle.cos(), angle.sin()) * MAX_SPEED,
        }
    }

    #[inline(always)]
    pub fn random_velo(&mut self, rng: &mut impl RngExt) {
        let angle: f32 = rng.random_range(0.0..std::f32::consts::TAU);
        self.vel = Vec2::new(angle.cos(), angle.sin()) * MAX_SPEED;
    }

    #[inline(always)]
    pub fn update(&mut self, dt: f32) {
        let target_pos = self.pos + self.vel * dt;
        self.pos = self.pos.lerp(target_pos, 0.225); // Smooth visual (render only)
        self.pos.x = self.pos.x.rem_euclid(SCREEN);
        self.pos.y = self.pos.y.rem_euclid(SCREEN);
    }

    #[inline(always)]
    pub fn render(&self) {
        draw_circle(self.pos.x, self.pos.y, 5.0, RED);
    }
}

type BotMap = Arc<DashMap<Uuid, Bot>>;

#[inline(always)]
async fn bot_loop(map: BotMap, bot_index: usize) {
    let mut rng = ::rand::rng();

    let mut client = match Client::connect(ADDR).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Bot: [{bot_index}] connect error: {e}");
            return;
        }
    };
    if let Err(e) = client.join(ROOM).await {
        eprintln!("Bot: [{bot_index}] join error: {e}");
        return;
    }

    let self_id = match client.recv().await {
        Ok(Some(ServerEvent::Joined { client_id, .. })) => client_id,
        _ => return,
    };

    map.insert(self_id, Bot::new(&mut rng));

    // Tick limit to avoid flooding the server with updates. The bot's movement is smoothed
    let send_interval = Duration::from_millis(1000 / TICK_RATE);
    let mut last_send = Instant::now();

    loop {
        if last_send.elapsed() >= send_interval {
            last_send = Instant::now();

            if let Some(mut bot) = map.get_mut(&self_id) {
                let payload = Payload {
                    id: self_id,
                    vx: bot.vel.x,
                    vy: bot.vel.y,
                };
                let data = wincode::serialize(&payload).unwrap();
                BYTES_SENT.fetch_add(data.len() as u64, Ordering::Relaxed);
                let _ = client.broadcast(&data).await;

                bot.random_velo(&mut rng);
            }
        }

        loop {
            // Timeout for recv to avoid stalling the bot loop if the server is unresponsive.
            // The bot's movement is still updated and rendered during this time
            match tokio::time::timeout(Duration::from_millis(100), client.recv()).await {
                Ok(Ok(Some(ServerEvent::Broadcast { sender_id, data })))
                    if sender_id != self_id =>
                {
                    BYTES_RECV.fetch_add(data.len() as u64, Ordering::Relaxed);
                    if let Ok(v) = wincode::deserialize::<Payload>(&data) {
                        map.entry(v.id)
                            .and_modify(|b| b.vel = Vec2::new(v.vx, v.vy))
                            .or_insert(Bot {
                                pos: Vec2::new(
                                    rng.random_range(0.0..SCREEN),
                                    rng.random_range(0.0..SCREEN),
                                ),
                                vel: Vec2::new(v.vx, v.vy),
                            });
                    }
                }
                Ok(Ok(Some(ServerEvent::PlayerLeft { client_id }))) => {
                    map.remove(&client_id);
                    eprintln!("Bot: {client_id} disconnected, removed from map");
                }
                Ok(Ok(Some(ServerEvent::Joined { client_id, .. }))) => {
                    eprintln!("Bot: {client_id} joined");
                }
                _ => break,
            }
        }

        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[macroquad::main(winconfig)]
async fn main() {
    let map: BotMap = Arc::new(DashMap::with_capacity(BOT_COUNT));

    for i in 0..BOT_COUNT {
        let m = map.clone();
        thread::spawn(move || match tokio::runtime::Runtime::new() {
            Ok(rt) => rt.block_on(bot_loop(m, i)),
            Err(e) => eprintln!("Tokio runtime error: {e}"),
        });
    }

    let mut last_bandwidth = Instant::now();
    let mut sent_acc = 0u64;
    let mut recv_acc = 0u64;

    loop {
        let delta = get_frame_time();

        clear_background(BLACK);

        let connected = map.len();
        draw_text(
            &format!("Bots: {connected}/{BOT_COUNT}"),
            10.0,
            24.0,
            20.0,
            WHITE,
        );
        draw_text(
            &format!("FPS: {:.0}", 1.0 / delta.max(0.001)),
            10.0,
            48.0,
            20.0,
            WHITE,
        );
        draw_text(&format!("Tick rate: {TICK_RATE}"), 10.0, 72.0, 20.0, WHITE);

        if last_bandwidth.elapsed().as_secs_f32() >= 1.0 {
            sent_acc = BYTES_SENT.swap(0, Ordering::Relaxed);
            recv_acc = BYTES_RECV.swap(0, Ordering::Relaxed);
            last_bandwidth = Instant::now();
        }
        draw_text(
            &format!("TX: {} KB/s", sent_acc / 1024),
            10.0,
            96.0,
            20.0,
            GREEN,
        );
        draw_text(
            &format!("RX: {} KB/s", recv_acc / 1024),
            10.0,
            120.0,
            20.0,
            BLUE,
        );

        for mut entry in map.iter_mut() {
            let bot = entry.value_mut();
            bot.update(delta);
            bot.render();
        }

        next_frame().await;
    }
}
