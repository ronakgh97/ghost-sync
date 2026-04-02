use crate::fx::ParticleEffect;
use crate::{Anim, PokemonKind};
use macroquad::prelude::*;

// TODO: Configs needs more tuning!!

/// Animation tick rate (how many ticks per second)
/// XML Duration values are in ticks, so we do tick / TICK_RATE
const TICK_RATE: f32 = 33.0;

/// Particle configuration for Pokémon attack effects
pub struct ParticleConfig {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    pub scale_min: f32,
    pub scale_max: f32,
    pub spawn_rate: f32,
    pub attack_delay: f32,
    pub offsets: [Vec2; 8],
    pub effects: &'static [ParticleEffect],
}

pub struct Pokemon {
    pub name: &'static str,
    pub idle: Anim,
    pub walk: Anim,
    pub atk: Anim,
    pub facing_rows: [usize; 8],
    pub speed: f32,
    pub scale: f32,
    pub particle_config: ParticleConfig,
}

pub struct Pokedex {
    lugia: Pokemon,
    latias: Pokemon,
    latios: Pokemon,
    articuno: Pokemon,
    zapdos: Pokemon,
    moltres: Pokemon,
}

impl Pokedex {
    #[inline]
    pub fn load() -> Self {
        Self {
            lugia: Self::load_lugia(),
            latios: Self::load_latios(),
            latias: Self::load_latias(),
            articuno: Self::load_articuno(),
            zapdos: Self::load_zapdos(),
            moltres: Self::load_moltres(),
        }
    }

    pub fn get(&self, kind: PokemonKind) -> &Pokemon {
        match kind {
            PokemonKind::Lugia => &self.lugia,
            PokemonKind::Latios => &self.latios,
            PokemonKind::Latias => &self.latias,
            PokemonKind::Articuno => &self.articuno,
            PokemonKind::Zapdos => &self.zapdos,
            PokemonKind::Moltres => &self.moltres,
        }
    }

    pub fn load_lugia() -> Pokemon {
        let idle = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Lugia/Idle-Anim.png"),
            Some(ImageFormat::Png),
        );
        idle.set_filter(FilterMode::Nearest);

        let walk = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Lugia/Walk-Anim.png"),
            Some(ImageFormat::Png),
        );
        walk.set_filter(FilterMode::Nearest);

        let atk = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Lugia/Shoot-Anim.png"),
            Some(ImageFormat::Png),
        );
        atk.set_filter(FilterMode::Nearest);

        Pokemon {
            name: "Lugia",
            idle: Anim {
                texture: idle,
                frame_w: 72.0,
                frame_h: 96.0,
                frame_durations: &[30.0 / TICK_RATE, 30.0 / TICK_RATE],
            },
            walk: Anim {
                texture: walk,
                frame_w: 80.0,
                frame_h: 96.0,
                frame_durations: &[4.0 / TICK_RATE, 4.0 / TICK_RATE],
            },
            atk: Anim {
                texture: atk,
                frame_w: 88.0,
                frame_h: 128.0,
                frame_durations: &[
                    2.0 / TICK_RATE,
                    6.0 / TICK_RATE,
                    1.0 / TICK_RATE,
                    1.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                ],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 202.0,
            scale: 2.0,
            particle_config: ParticleConfig {
                r: 0.8,
                g: 0.9,
                b: 1.0,
                a: 0.85,
                speed_min: 180.0,
                speed_max: 480.0,
                lifetime_min: 0.4,
                lifetime_max: 1.0,
                scale_min: 0.4,
                scale_max: 1.6,
                spawn_rate: 2200.0,
                attack_delay: 8.0 / TICK_RATE,
                offsets: [
                    vec2(0.0, 10.0),    // South
                    vec2(-55.0, -15.0), // SouthWest
                    vec2(-75.0, -40.0), // West
                    vec2(-55.0, -75.0), // NorthWest
                    vec2(0.0, -75.0),   // North
                    vec2(55.0, -75.0),  // NorthEast
                    vec2(75.0, -40.0),  // East
                    vec2(55.0, -15.0),  // SouthEast
                ],
                effects: &[
                    ParticleEffect::Smoke,
                    ParticleEffect::Scorch,
                    ParticleEffect::Smoke,
                ],
            },
        }
    }

    pub fn load_latios() -> Pokemon {
        let idle = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Latios/Idle-Anim.png"),
            Some(ImageFormat::Png),
        );
        idle.set_filter(FilterMode::Nearest);

        let walk = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Latios/Walk-Anim.png"),
            Some(ImageFormat::Png),
        );
        walk.set_filter(FilterMode::Nearest);

        let atk = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Latios/Shoot-Anim.png"),
            Some(ImageFormat::Png),
        );
        atk.set_filter(FilterMode::Nearest);

        Pokemon {
            name: "Latios",
            idle: Anim {
                texture: idle,
                frame_w: 64.0,
                frame_h: 80.0,
                frame_durations: &[
                    8.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                ],
            },
            walk: Anim {
                texture: walk,
                frame_w: 64.0,
                frame_h: 80.0,
                frame_durations: &[
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                ],
            },
            atk: Anim {
                texture: atk,
                frame_w: 72.0,
                frame_h: 88.0,
                frame_durations: &[
                    2.0 / TICK_RATE,
                    6.0 / TICK_RATE,
                    1.0 / TICK_RATE,
                    1.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                ],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 202.0,
            scale: 2.0,
            particle_config: ParticleConfig {
                r: 0.97,
                g: 0.17,
                b: 0.71,
                a: 0.9,
                speed_min: 150.0,
                speed_max: 480.0,
                lifetime_min: 0.2,
                lifetime_max: 0.8,
                scale_min: 1.2,
                scale_max: 1.8,
                spawn_rate: 1000.0,
                attack_delay: 8.0 / TICK_RATE,
                offsets: [
                    vec2(0.0, 10.0),    // South
                    vec2(-60.0, 10.0),  // SouthWest
                    vec2(-75.0, -40.0), // West
                    vec2(-60.0, -80.0), // NorthWest
                    vec2(0.0, -75.0),   // North
                    vec2(60.0, -80.0),  // NorthEast
                    vec2(75.0, -40.0),  // East
                    vec2(60.0, 10.0),   // SouthEast
                ],
                effects: &[
                    ParticleEffect::Spark,
                    ParticleEffect::Smoke,
                    ParticleEffect::Scorch,
                ],
            },
        }
    }

    pub fn load_latias() -> Pokemon {
        let idle = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Latias/Idle-Anim.png"),
            Some(ImageFormat::Png),
        );
        idle.set_filter(FilterMode::Nearest);

        let walk = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Latias/Walk-Anim.png"),
            Some(ImageFormat::Png),
        );
        walk.set_filter(FilterMode::Nearest);

        let atk = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Latias/Shoot-Anim.png"),
            Some(ImageFormat::Png),
        );
        atk.set_filter(FilterMode::Nearest);

        Pokemon {
            name: "Latias",
            idle: Anim {
                texture: idle,
                frame_w: 48.0,
                frame_h: 64.0,
                frame_durations: &[
                    8.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                ],
            },
            walk: Anim {
                texture: walk,
                frame_w: 48.0,
                frame_h: 64.0,
                frame_durations: &[
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                    4.0 / TICK_RATE,
                ],
            },
            atk: Anim {
                texture: atk,
                frame_w: 64.0,
                frame_h: 80.0,
                frame_durations: &[
                    2.0 / TICK_RATE,
                    6.0 / TICK_RATE,
                    1.0 / TICK_RATE,
                    1.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                ],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 202.0,
            scale: 2.0,
            particle_config: ParticleConfig {
                r: 0.97,
                g: 0.17,
                b: 0.71,
                a: 0.9,
                speed_min: 150.0,
                speed_max: 480.0,
                lifetime_min: 0.2,
                lifetime_max: 0.8,
                scale_min: 1.2,
                scale_max: 1.8,
                spawn_rate: 1000.0,
                attack_delay: 8.0 / TICK_RATE,
                offsets: [
                    vec2(0.0, 10.0),    // South
                    vec2(-60.0, 10.0),  // SouthWest
                    vec2(-75.0, -25.0), // West
                    vec2(-60.0, -80.0), // NorthWest
                    vec2(0.0, -75.0),   // North
                    vec2(60.0, -80.0),  // NorthEast
                    vec2(75.0, -25.0),  // East
                    vec2(60.0, 10.0),   // SouthEast
                ],
                effects: &[
                    ParticleEffect::Spark,
                    ParticleEffect::Smoke,
                    ParticleEffect::Scorch,
                ],
            },
        }
    }

    pub fn load_articuno() -> Pokemon {
        let idle = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Articuno/Idle-Anim.png"),
            Some(ImageFormat::Png),
        );
        idle.set_filter(FilterMode::Nearest);

        let walk = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Articuno/Walk-Anim.png"),
            Some(ImageFormat::Png),
        );
        walk.set_filter(FilterMode::Nearest);

        let atk = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Articuno/Shoot-Anim.png"),
            Some(ImageFormat::Png),
        );
        atk.set_filter(FilterMode::Nearest);

        Pokemon {
            name: "Articuno",
            idle: Anim {
                texture: idle,
                frame_w: 88.0,
                frame_h: 88.0,
                frame_durations: &[
                    8.0 / TICK_RATE,
                    10.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                    10.0 / TICK_RATE,
                ],
            },
            walk: Anim {
                texture: walk,
                frame_w: 88.0,
                frame_h: 88.0,
                frame_durations: &[
                    8.0 / TICK_RATE,
                    12.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                    12.0 / TICK_RATE,
                ],
            },
            atk: Anim {
                texture: atk,
                frame_w: 104.0,
                frame_h: 104.0,
                frame_durations: &[
                    1.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    1.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    1.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    1.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    1.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    1.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    6.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                ],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 157.0,
            scale: 2.0,
            particle_config: ParticleConfig {
                r: 0.5,
                g: 0.8,
                b: 1.0,
                a: 0.85,
                speed_min: 100.0,
                speed_max: 280.0,
                lifetime_min: 0.4,
                lifetime_max: 1.6,
                scale_min: 0.6,
                scale_max: 2.0,
                spawn_rate: 1800.0,
                attack_delay: 24.0 / TICK_RATE,
                offsets: [
                    vec2(0.0, 10.0),    // South
                    vec2(-40.0, 5.0),   // SouthWest
                    vec2(-50.0, -25.0), // West
                    vec2(-40.0, -50.0), // NorthWest
                    vec2(0.0, -75.0),   // North
                    vec2(40.0, -50.0),  // NorthEast
                    vec2(50.0, -25.0),  // East
                    vec2(40.0, 5.0),    // SouthEast
                ],
                effects: &[
                    ParticleEffect::Scorch,
                    ParticleEffect::Flame,
                    ParticleEffect::Smoke,
                ],
            },
        }
    }

    pub fn load_zapdos() -> Pokemon {
        let idle = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Zapdos/Idle-Anim.png"),
            Some(ImageFormat::Png),
        );
        idle.set_filter(FilterMode::Nearest);
        let walk = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Zapdos/Walk-Anim.png"),
            Some(ImageFormat::Png),
        );
        walk.set_filter(FilterMode::Nearest);

        let atk = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Zapdos/Shoot-Anim.png"),
            Some(ImageFormat::Png),
        );
        atk.set_filter(FilterMode::Nearest);
        Pokemon {
            name: "Zapdos",
            idle: Anim {
                texture: idle,
                frame_w: 56.0,
                frame_h: 96.0,
                frame_durations: &[
                    8.0 / TICK_RATE,
                    10.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                    10.0 / TICK_RATE,
                ],
            },
            walk: Anim {
                texture: walk,
                frame_w: 56.0,
                frame_h: 96.0,
                frame_durations: &[
                    6.0 / TICK_RATE,
                    6.0 / TICK_RATE,
                    6.0 / TICK_RATE,
                    6.0 / TICK_RATE,
                ],
            },
            atk: Anim {
                texture: atk,
                frame_w: 72.0,
                frame_h: 112.0,
                frame_durations: &[
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    6.0 / TICK_RATE,
                    1.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                ],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 184.0,
            scale: 2.0,
            particle_config: ParticleConfig {
                r: 1.0,
                g: 0.9,
                b: 0.2,
                a: 0.95,
                speed_min: 200.0,
                speed_max: 300.0,
                lifetime_min: 0.2,
                lifetime_max: 1.5,
                scale_min: 0.6,
                scale_max: 1.8,
                spawn_rate: 1600.0,
                attack_delay: 12.0 / TICK_RATE,
                offsets: [
                    vec2(0.0, 20.0),    // South
                    vec2(-50.0, 15.0),  // SouthWest
                    vec2(-75.0, -15.0), // West
                    vec2(-60.0, -50.0), // NorthWest
                    vec2(0.0, -90.0),   // North
                    vec2(60.0, -50.0),  // NorthEast
                    vec2(75.0, -15.0),  // East
                    vec2(50.0, 15.0),   // SouthEast
                ],
                effects: &[
                    ParticleEffect::Spark,
                    ParticleEffect::Spark,
                    ParticleEffect::Scorch,
                ],
            },
        }
    }

    pub fn load_moltres() -> Pokemon {
        let idle = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Moltres/Idle-Anim.png"),
            Some(ImageFormat::Png),
        );
        idle.set_filter(FilterMode::Nearest);
        let walk = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Moltres/Walk-Anim.png"),
            Some(ImageFormat::Png),
        );
        walk.set_filter(FilterMode::Nearest);

        let atk = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Moltres/Shoot-Anim.png"),
            Some(ImageFormat::Png),
        );
        atk.set_filter(FilterMode::Nearest);
        Pokemon {
            name: "Moltres",
            idle: Anim {
                texture: idle,
                frame_w: 80.0,
                frame_h: 96.0,
                frame_durations: &[
                    8.0 / TICK_RATE,
                    12.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                    12.0 / TICK_RATE,
                ],
            },
            walk: Anim {
                texture: walk,
                frame_w: 80.0,
                frame_h: 96.0,
                frame_durations: &[
                    8.0 / TICK_RATE,
                    10.0 / TICK_RATE,
                    8.0 / TICK_RATE,
                    10.0 / TICK_RATE,
                ],
            },
            atk: Anim {
                texture: atk,
                frame_w: 96.0,
                frame_h: 96.0,
                frame_durations: &[
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                    2.0 / TICK_RATE,
                ],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 166.0,
            scale: 2.0,
            particle_config: ParticleConfig {
                r: 1.0,
                g: 0.4,
                b: 0.1,
                a: 0.9,
                speed_min: 120.0,
                speed_max: 280.0,
                lifetime_min: 0.8,
                lifetime_max: 1.4,
                scale_min: 0.2,
                scale_max: 1.8,
                spawn_rate: 2500.0,
                attack_delay: 12.0 / TICK_RATE,
                offsets: [
                    vec2(0.0, 10.0),    // South
                    vec2(-55.0, 2.5),   // SouthWest
                    vec2(-75.0, -25.0), // West
                    vec2(-40.0, -50.0), // NorthWest
                    vec2(0.0, -80.0),   // North
                    vec2(40.0, -50.0),  // NorthEast
                    vec2(75.0, -25.0),  // East
                    vec2(55.0, 2.5),    // SouthEast
                ],
                effects: &[
                    ParticleEffect::Fire,
                    ParticleEffect::Flame,
                    ParticleEffect::Scorch,
                ],
            },
        }
    }
}
