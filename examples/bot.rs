use ::rand::RngExt;
use dashmap::DashMap;
use ghost_sync::{Client, ServerEvent};
use macroquad::prelude::*;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;
use wincode::{SchemaRead, SchemaWrite};

const ADDR: &str = "127.0.0.1:7777";
const ROOM: &str = "room-128";
const BOT_COUNT: usize = 128;
const TICK_RATE: u64 = 20;
const MAX_SPEED: f32 = 100.0;
const SCREEN: f32 = 800.0;

fn winconfig() -> Conf {
    Conf {
        window_title: "Bot Stress Test".into(),
        window_width: SCREEN as i32,
        window_height: SCREEN as i32,
        window_resizable: false,
        sample_count: BOT_COUNT as i32,
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
        self.pos = self.pos.lerp(target_pos, 0.225); // Smooth movement visual
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
            eprintln!("[{bot_index}] connect error: {e}");
            return;
        }
    };
    if let Err(e) = client.join(ROOM).await {
        eprintln!("[{bot_index}] join error: {e}");
        return;
    }

    let self_id = match client.recv().await {
        Ok(Some(ServerEvent::Joined { client_id, .. })) => client_id,
        _ => return,
    };

    map.insert(self_id, Bot::new(&mut rng));

    // Tick rate limiter
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
                let _ = client
                    .broadcast(&wincode::serialize(&payload).unwrap())
                    .await;

                bot.random_velo(&mut rng);
            }
        }

        loop {
            // Draining incoming messages with a short timeout to avoid stalling the loop
            match tokio::time::timeout(Duration::from_micros(10), client.recv()).await {
                Ok(Ok(Some(ServerEvent::Broadcast { sender_id, data })))
                // Update the bot map, we will render them later
                    if sender_id != self_id =>
                {
                    if let Ok(v) = wincode::deserialize::<Payload>(&data) {
                        map.entry(v.id)
                            .and_modify(|b| b.vel = Vec2::new(v.vx, v.vy))
                            .or_insert(Bot {
                                pos: Vec2::new(
                                    rng.random::<f32>() * SCREEN,
                                    rng.random::<f32>() * SCREEN,
                                ),
                                vel: Vec2::new(v.vx, v.vy),
                            });
                    }
                }
                _ => break,
            }
        }

        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[macroquad::main(winconfig)]
async fn main() {
    let map: BotMap = Arc::new(DashMap::new());

    for i in 0..BOT_COUNT {
        let m = map.clone();
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(bot_loop(m, i));
        });
    }

    let delta = get_frame_time();

    loop {
        clear_background(BLACK);
        draw_text(
            &format!("Bots:{BOT_COUNT} Fps:{:.0}", 1.0 / delta.max(0.001)),
            10.0,
            24.0,
            20.0,
            WHITE,
        );
        draw_text(&format!("Tick rate: {TICK_RATE}"), 10.0, 48.0, 20.0, WHITE);

        for mut entry in map.iter_mut() {
            let bot = entry.value_mut();
            bot.update(delta);
            bot.render();
        }

        next_frame().await;
    }
}
