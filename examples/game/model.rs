use crate::{Anim, PokemonKind};
use macroquad::prelude::*;

pub struct Pokemon {
    pub name: &'static str,
    pub idle: Anim,
    pub walk: Anim,
    pub facing_rows: [usize; 8],
    pub speed: f32,
    pub scale: f32,
}
pub struct Pokedex {
    lugia: Pokemon,
    latios: Pokemon,
    latias: Pokemon,
    articuno: Pokemon,
    zapdos: Pokemon,
    moltres: Pokemon,
}

impl Pokedex {
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

        Pokemon {
            name: "Lugia",
            idle: Anim {
                texture: idle,
                frame_w: 72.0,
                frame_h: 96.0,
                frame_durations: &[0.3, 0.3],
            },
            walk: Anim {
                texture: walk,
                frame_w: 80.0,
                frame_h: 96.0,
                frame_durations: &[
                    1.0 / 2.0,
                    1.0 / 3.0,
                    1.0 / 2.0,
                    1.0 / 3.0,
                    1.0 / 2.0,
                    1.0 / 3.0,
                    1.0 / 2.0,
                    1.0 / 3.0,
                    1.0 / 2.0,
                    1.0 / 3.0,
                    1.0 / 2.0,
                    1.0 / 3.0,
                ],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 110.0,
            scale: 2.0,
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
                frame_durations: &[1.0 / 4.0; 12],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 110.0,
            scale: 2.0,
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
                frame_durations: &[1.0 / 4.0; 12],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 110.0,
            scale: 2.0,
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

        Pokemon {
            name: "Articuno",
            idle: Anim {
                texture: idle,
                frame_w: 88.0,
                frame_h: 88.0,
                frame_durations: &[1.0 / 8.0, 1.0 / 10.0, 1.0 / 8.0, 1.0 / 16.0],
            },
            walk: Anim {
                texture: walk,
                frame_w: 88.0,
                frame_h: 88.0,
                frame_durations: &[1.0 / 8.0, 1.0 / 10.0, 1.0 / 8.0, 1.0 / 10.0],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 85.0,
            scale: 2.0,
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

        Pokemon {
            name: "Zapdos",
            idle: Anim {
                texture: idle,
                frame_w: 56.0,
                frame_h: 96.0,
                frame_durations: &[1.0 / 8.0, 1.0 / 10.0, 1.0 / 8.0, 1.0 / 10.0],
            },
            walk: Anim {
                texture: walk,
                frame_w: 56.0,
                frame_h: 96.0,
                frame_durations: &[1.0 / 6.0, 1.0 / 6.0, 1.0 / 6.0, 1.0 / 6.0],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 100.0,
            scale: 2.0,
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

        Pokemon {
            name: "Moltres",
            idle: Anim {
                texture: idle,
                frame_w: 80.0,
                frame_h: 96.0,
                frame_durations: &[1.0 / 8.0, 1.0 / 12.0, 1.0 / 8.0, 1.0 / 12.0],
            },
            walk: Anim {
                texture: walk,
                frame_w: 80.0,
                frame_h: 96.0,
                frame_durations: &[1.0 / 8.0, 1.0 / 10.0, 1.0 / 8.0, 1.0 / 10.0],
            },
            facing_rows: [0, 7, 6, 5, 4, 3, 2, 1],
            speed: 90.0,
            scale: 2.0,
        }
    }
}
