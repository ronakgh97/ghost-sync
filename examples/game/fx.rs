use macroquad::prelude::{
    draw_texture_ex, vec2, Color, DrawTextureParams, FilterMode, ImageFormat, Texture2D,
};
use rand::rng;
use rand::RngExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleEffect {
    Fire,
    Flame,
    Smoke,
    Spark,
    Scorch,
}

pub struct ParticleTextureSet {
    pub textures: Vec<Texture2D>,
}

impl ParticleTextureSet {
    pub fn load_fire() -> Self {
        let tex1 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Fire_01.png"),
            Some(ImageFormat::Png),
        );
        tex1.set_filter(FilterMode::Nearest);

        let tex2 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Fire_02.png"),
            Some(ImageFormat::Png),
        );
        tex2.set_filter(FilterMode::Nearest);

        Self {
            textures: vec![tex1, tex2],
        }
    }

    pub fn load_flame() -> Self {
        let tex1 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Flame_01.png"),
            Some(ImageFormat::Png),
        );
        tex1.set_filter(FilterMode::Nearest);
        let tex2 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Flame_02.png"),
            Some(ImageFormat::Png),
        );
        tex2.set_filter(FilterMode::Nearest);
        let tex3 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Flame_03.png"),
            Some(ImageFormat::Png),
        );
        tex3.set_filter(FilterMode::Nearest);

        let tex4 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Flame_04.png"),
            Some(ImageFormat::Png),
        );
        tex4.set_filter(FilterMode::Nearest);
        Self {
            textures: vec![tex1, tex2, tex3, tex4],
        }
    }

    pub fn load_smoke() -> Self {
        let tex1 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Smoke_01.png"),
            Some(ImageFormat::Png),
        );
        tex1.set_filter(FilterMode::Nearest);

        let tex2 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Smoke_02.png"),
            Some(ImageFormat::Png),
        );
        tex2.set_filter(FilterMode::Nearest);

        let tex3 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Smoke_03.png"),
            Some(ImageFormat::Png),
        );
        tex3.set_filter(FilterMode::Nearest);
        Self {
            textures: vec![tex1, tex2, tex3],
        }
    }

    pub fn load_spark() -> Self {
        let tex1 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Spark_01.png"),
            Some(ImageFormat::Png),
        );
        tex1.set_filter(FilterMode::Nearest);

        let tex2 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Spark_02.png"),
            Some(ImageFormat::Png),
        );
        tex2.set_filter(FilterMode::Nearest);

        let tex3 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Spark_03.png"),
            Some(ImageFormat::Png),
        );
        tex3.set_filter(FilterMode::Nearest);
        Self {
            textures: vec![tex1, tex2, tex3],
        }
    }

    pub fn load_scorch() -> Self {
        let tex1 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Scorch_01.png"),
            Some(ImageFormat::Png),
        );
        tex1.set_filter(FilterMode::Nearest);
        let tex2 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Scorch_02.png"),
            Some(ImageFormat::Png),
        );
        tex2.set_filter(FilterMode::Nearest);
        let tex3 = Texture2D::from_file_with_format(
            include_bytes!("../../assets/sprites/Particles/Scorch_03.png"),
            Some(ImageFormat::Png),
        );
        tex3.set_filter(FilterMode::Nearest);

        Self {
            textures: vec![tex1, tex2, tex3],
        }
    }
}

pub struct ParticleEffectTextures {
    pub fire: ParticleTextureSet,
    pub flame: ParticleTextureSet,
    pub smoke: ParticleTextureSet,
    pub spark: ParticleTextureSet,
    pub scorer: ParticleTextureSet,
}

impl ParticleEffectTextures {
    pub fn load() -> Self {
        Self {
            fire: ParticleTextureSet::load_fire(),
            flame: ParticleTextureSet::load_flame(),
            smoke: ParticleTextureSet::load_smoke(),
            spark: ParticleTextureSet::load_spark(),
            scorer: ParticleTextureSet::load_scorch(),
        }
    }

    pub fn get(&self, effect: ParticleEffect) -> &ParticleTextureSet {
        match effect {
            ParticleEffect::Fire => &self.fire,
            ParticleEffect::Flame => &self.flame,
            ParticleEffect::Smoke => &self.smoke,
            ParticleEffect::Spark => &self.spark,
            ParticleEffect::Scorch => &self.scorer,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub lifetime: f32,
    pub lifetime_max: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub scale: f32,
    pub effect: ParticleEffect,
    pub texture_index: usize,
}

impl Particle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
        lifetime: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        scale: f32,
        effect: ParticleEffect,
        texture_index: usize,
    ) -> Self {
        Self {
            x,
            y,
            vx,
            vy,
            lifetime,
            lifetime_max: lifetime,
            r,
            g,
            b,
            a,
            scale,
            effect,
            texture_index,
        }
    }

    #[inline]
    pub fn update(&mut self, delta: f32) {
        self.x += self.vx * delta;
        self.y += self.vy * delta;
        self.lifetime -= delta;
    }

    #[inline]
    pub fn update_exponential(&mut self, delta: f32) {
        self.x += self.vx * delta;
        self.y += self.vy * delta;
        self.lifetime -= delta * (self.lifetime / self.lifetime_max).clamp(0.0, 1.0);
    }

    #[inline]
    pub fn is_alive(&self) -> bool {
        self.lifetime > 0.0
    }

    #[inline]
    pub fn alpha(&self) -> f32 {
        if self.lifetime_max > 0.0 {
            (self.lifetime / self.lifetime_max).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }
}

pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub textures: ParticleEffectTextures,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(1024),
            textures: ParticleEffectTextures::load(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn spawn(
        &mut self,
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
        lifetime: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        scale: f32,
        effect: ParticleEffect,
        texture_index: usize,
    ) {
        self.particles.push(Particle::new(
            x,
            y,
            vx,
            vy,
            lifetime,
            r,
            g,
            b,
            a,
            scale,
            effect,
            texture_index,
        ));
    }

    #[inline]
    pub fn update(&mut self, delta: f32) {
        let rand_bool = rng().random_bool(0.5);

        for p in &mut self.particles {
            if rand_bool {
                p.update(delta);
            } else {
                p.update_exponential(delta);
            }
        }

        self.particles.retain(|p| p.is_alive());
    }

    #[inline]
    pub fn render(&self) {
        for p in &self.particles {
            let alpha = p.alpha();
            if alpha <= 0.0 {
                continue;
            }

            // Get particle texture set
            let texture_set = self.textures.get(p.effect);
            // Get a particular texture variant for this particle effect
            let texture = &texture_set.textures[p.texture_index];

            let size = p.scale * 32.0;
            draw_texture_ex(
                texture,
                p.x - size * 0.5,
                p.y - size * 0.5,
                Color::new(p.r, p.g, p.b, p.a * alpha),
                DrawTextureParams {
                    dest_size: Some(vec2(size, size)),
                    ..Default::default()
                },
            );
        }
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}
