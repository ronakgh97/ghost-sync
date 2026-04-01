use ::rand::{rng, RngExt};
use macroquad::prelude::*;
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleShape {
    Circle,
    Poly,
    Ellipse,
    Line,
}

#[allow(clippy::too_many_arguments)]
#[derive(Clone, Debug)]
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
    pub shape: ParticleShape,
}

#[allow(clippy::too_many_arguments)]
impl Particle {
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
        shape: ParticleShape,
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
            shape,
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
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(1024),
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
        shape: ParticleShape,
    ) {
        self.particles.push(Particle::new(
            x, y, vx, vy, lifetime, r, g, b, a, scale, shape,
        ));
    }

    #[allow(unused)]
    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn spawn_range(
        &mut self,
        x: f32,
        y: f32,
        vx_min: f32,
        vx_max: f32,
        vy_min: f32,
        vy_max: f32,
        lifetime_min: f32,
        lifetime_max: f32,
        scale_min: f32,
        scale_max: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        shape: ParticleShape,
    ) {
        let mut rng = rng();

        let vx = rng.random_range(vx_min..vx_max);
        let vy = rng.random_range(vy_min..vy_max);
        let lifetime = rng.random_range(lifetime_min..lifetime_max);
        let scale = rng.random_range(scale_min..scale_max);

        self.spawn(x, y, vx, vy, lifetime, r, g, b, a, scale, shape);
    }

    #[inline]
    pub fn update(&mut self, delta: f32) {
        let random_bool = rng().random_bool(0.5);

        if random_bool {
            for p in &mut self.particles {
                p.update(delta);
            }
        } else {
            for p in &mut self.particles {
                p.update_exponential(delta);
            }
        }

        self.particles.retain(|p| p.is_alive());
    }

    #[inline]
    pub fn render(&self) {
        let mut rng = rng();
        for p in &self.particles {
            let alpha = p.alpha();
            if alpha <= 0.0 {
                continue;
            }
            let color = Color::new(p.r, p.g, p.b, p.a * alpha);
            let radius = p.scale * 4.0;

            match p.shape {
                ParticleShape::Circle => {
                    draw_circle(p.x, p.y, radius, color);
                }
                ParticleShape::Poly => {
                    let sides: u8 = rng.random_range(6..12);
                    let rot: f32 = rng.random_range(0.0..PI * 2.0);
                    draw_poly(p.x, p.y, sides, radius, rot, color);
                }
                ParticleShape::Ellipse => {
                    let rot: f32 = rng.random_range(0.0..PI * 2.0);
                    draw_ellipse(p.x, p.y, radius, radius, rot, color);
                }
                ParticleShape::Line => {
                    let speed = (p.vx * p.vx + p.vy * p.vy).sqrt();
                    let length = if speed > 0.0 {
                        (radius * 2.0).max(4.0)
                    } else {
                        radius
                    };
                    let thickness = radius * 0.4;
                    let end_x = p.x + p.vx * (length / speed.max(0.01));
                    let end_y = p.y + p.vy * (length / speed.max(0.01));
                    draw_line(p.x, p.y, end_x, end_y, thickness, color);
                }
            }
        }
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}
