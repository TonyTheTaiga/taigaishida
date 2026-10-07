use wasm_bindgen::prelude::*;

mod particle;
mod trail;
use trail::Trail;
mod projection;
use particle::{Body, Clock, Particles, Vec3};
use projection::Camera;

// ─── Constants ──────────────────────────────────────────────────────

const SHOW_DURATION: f64 = 68.0;
const MAX_PARTICLES: usize = 3200;
const MAX_TRAIL_SEGMENTS: usize = 12800;

// Character table — JS decodes by index
// 0=' ' 1='.' 2='·' 3='+' 4='*' 5='✦' 6='│' 7='╽' 8='o' 9='~' 10=''' 11='x' 12='%'
fn char_for_age(frac: f64) -> u8 {
    if frac > 0.7 {
        5
    }
    // ✦
    else if frac > 0.5 {
        4
    }
    // *
    else if frac > 0.3 {
        3
    }
    // +
    else if frac > 0.15 {
        2
    }
    // ·
    else if frac > 0.05 {
        1
    }
    // .
    else {
        0
    } // ' '
}

// ─── RNG helpers ────────────────────────────────────────────────────

#[cfg(not(test))]
fn rand_f64() -> f64 {
    js_sys::Math::random()
}

// Native tests exercise complete shows without requiring a JavaScript runtime.
#[cfg(test)]
fn rand_f64() -> f64 {
    use std::cell::Cell;
    thread_local! { static SEED: Cell<u64> = const { Cell::new(42) }; }
    SEED.with(|seed| {
        let next = seed.get().wrapping_mul(6364136223846793005).wrapping_add(1);
        seed.set(next);
        (next >> 11) as f64 / ((1u64 << 53) as f64)
    })
}

fn rand(min: f64, max: f64) -> f64 {
    rand_f64() * (max - min) + min
}

fn rand_int(min: i32, max: i32) -> i32 {
    (rand_f64() * ((max - min + 1) as f64)).floor() as i32 + min
}

fn pick<T: Copy>(arr: &[T]) -> T {
    arr[(rand_f64() * arr.len() as f64).floor() as usize % arr.len()]
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

// ─── Color palette ──────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

const GOLD: Color = Color {
    r: 255,
    g: 215,
    b: 0,
};
const AMBER: Color = Color {
    r: 255,
    g: 165,
    b: 0,
};
const SILVER: Color = Color {
    r: 192,
    g: 192,
    b: 192,
};
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};
const RED: Color = Color {
    r: 255,
    g: 68,
    b: 68,
};
const SOFT_RED: Color = Color {
    r: 255,
    g: 107,
    b: 107,
};
const BLUE: Color = Color {
    r: 68,
    g: 136,
    b: 255,
};
const SOFT_BLUE: Color = Color {
    r: 102,
    g: 187,
    b: 255,
};
const GREEN: Color = Color {
    r: 68,
    g: 255,
    b: 136,
};
const LIME: Color = Color {
    r: 136,
    g: 255,
    b: 68,
};
const PURPLE: Color = Color {
    r: 187,
    g: 102,
    b: 255,
};
const WARM_WHITE: Color = Color {
    r: 255,
    g: 240,
    b: 200,
};
const COPPER: Color = Color {
    r: 184,
    g: 115,
    b: 51,
};
const PINK: Color = Color {
    r: 255,
    g: 150,
    b: 200,
};
const DIM_GREY: Color = Color {
    r: 80,
    g: 75,
    b: 70,
};

const COLOR_GROUPS: &[&[Color]] = &[
    &[GOLD, AMBER],
    &[SILVER, WHITE],
    &[RED, SOFT_RED],
    &[BLUE, SOFT_BLUE],
    &[GREEN, LIME],
    &[PURPLE, SOFT_BLUE],
    &[GOLD, RED],
    &[SILVER, BLUE],
    &[WARM_WHITE, PINK],
    &[COPPER, GOLD],
];

// ─── ParticleKind ───────────────────────────────────────────────────

#[allow(dead_code)]
enum ParticleKind {
    Normal,
    Flash,
    CracklingSource,
    CracklingSpark,
    ColorChanging {
        r2: u8,
        g2: u8,
        b2: u8,
        r3: u8,
        g3: u8,
        b3: u8,
    },
    GlitterTrail,
    GlitterDot,
    Strobe {
        phase: f64,
    },
    Crossette {
        split_dist: f64,
        dist_traveled: f64,
    },
    Tourbillion {
        angular_vel: f64,
        angle: f64,
        origin_x: f64,
        origin_y: f64,
    },
    Brocade,
    Smoke,
    Ring,
}

// ─── Particle ───────────────────────────────────────────────────────

struct Particle {
    body: Body,
    trail: Trail,
    long_trail: bool,
    char_idx: u8,
    r: u8,
    g: u8,
    b: u8,
    kind: ParticleKind,
}

impl Particle {
    /// Effect tunings are authored in metres per simulation tick and converted
    /// here to SI velocity/lifetime before entering the physical integrator.
    fn new(
        x: f64,
        y: f64,
        vx_per_tick: f64,
        vy_per_tick: f64,
        life_ticks: f64,
        color: Color,
    ) -> Self {
        Self {
            body: Body::new(
                Vec3::new(x, y, 0.0),
                Vec3::new(vx_per_tick * 60.0, vy_per_tick * 60.0, 0.0),
                life_ticks / 60.0,
            ),
            trail: Trail::new(),
            long_trail: false,
            char_idx: 5, // ✦
            r: color.r,
            g: color.g,
            b: color.b,
            kind: ParticleKind::Normal,
        }
    }

    fn update(&mut self, dt: f64) -> bool {
        if !matches!(
            self.kind,
            ParticleKind::Smoke | ParticleKind::GlitterDot | ParticleKind::Flash
        ) {
            self.trail
                .record(self.body.position, if self.long_trail { 5 } else { 2 });
        }
        // The core handles ballistic motion; procedural effects override position.
        match &mut self.kind {
            ParticleKind::Tourbillion {
                angular_vel,
                angle,
                origin_x,
                origin_y,
            } => {
                *angle += *angular_vel * dt;
                *origin_y += 0.3 * dt;
                let frac_elapsed = 1.0 - (self.body.life / self.body.max_life);
                let radius = frac_elapsed * 15.0;
                self.body.position.x = *origin_x + radius * angle.cos();
                self.body.position.y = *origin_y + radius * angle.sin();
                self.body.step(dt);
            }
            ParticleKind::GlitterDot => {
                self.body.life -= dt;
            }
            ParticleKind::Smoke => {
                self.body.step(dt);
            }
            ParticleKind::Crossette { dist_traveled, .. } => {
                *dist_traveled += self.body.step(dt);
            }
            _ => {
                self.body.step(dt);
            }
        }

        // The water plane is a real collision surface in world space.
        if !matches!(self.kind, ParticleKind::Flash) && self.body.position.y <= 0.0 {
            self.body.life = 0.0;
            self.char_idx = 0;
            return false;
        }

        // Visual (kind-specific char_idx)
        let frac = (self.body.life / self.body.max_life).max(0.0);
        match &self.kind {
            ParticleKind::Smoke => {
                self.char_idx = if frac > 0.5 { 9 } else { 1 };
            }
            ParticleKind::GlitterDot => {
                self.char_idx = 10;
            }
            ParticleKind::Brocade => {
                self.char_idx = if frac > 0.3 {
                    12
                } else if frac > 0.1 {
                    2
                } else {
                    1
                };
            }
            ParticleKind::Ring => {
                self.char_idx = if frac > 0.3 {
                    8
                } else if frac > 0.1 {
                    2
                } else {
                    1
                };
            }
            ParticleKind::Strobe { phase } => {
                let t = (self.body.max_life - self.body.life) * 18.0 + phase;
                self.char_idx = if t.sin() > 0.0 { char_for_age(frac) } else { 0 };
            }
            _ => {
                self.char_idx = char_for_age(frac);
            }
        }

        // Alive check
        match &self.kind {
            ParticleKind::Strobe { .. } => self.body.life > 0.0,
            _ => self.body.life > 0.0 && self.char_idx != 0,
        }
    }

    fn appearance(&self, scale: f64) -> (u8, u8, u8, f64) {
        let frac = (self.body.life / self.body.max_life).max(0.0);

        // Color-changing: lerp through 3 colors over lifetime
        let (base_r, base_g, base_b) = match &self.kind {
            ParticleKind::ColorChanging {
                r2,
                g2,
                b2,
                r3,
                g3,
                b3,
            } => {
                if frac > 0.66 {
                    let t = (1.0 - frac) / 0.34;
                    (
                        lerp(self.r as f64, *r2 as f64, t).round() as u8,
                        lerp(self.g as f64, *g2 as f64, t).round() as u8,
                        lerp(self.b as f64, *b2 as f64, t).round() as u8,
                    )
                } else if frac > 0.33 {
                    let t = (0.66 - frac) / 0.33;
                    (
                        lerp(*r2 as f64, *r3 as f64, t).round() as u8,
                        lerp(*g2 as f64, *g3 as f64, t).round() as u8,
                        lerp(*b2 as f64, *b3 as f64, t).round() as u8,
                    )
                } else {
                    (*r3, *g3, *b3)
                }
            }
            _ => (self.r, self.g, self.b),
        };

        // Stars burn brightly before cooling to amber; they do not fade linearly
        // from the instant of detonation. Per-star lifetime variation breaks up the edge.
        let fade = (frac * 2.5).min(1.0);
        let r = lerp(40.0, base_r as f64, fade).round() as u8;
        let g = lerp(20.0, base_g as f64, fade).round() as u8;
        let b = lerp(15.0, base_b as f64, fade).round() as u8;

        let alpha = match &self.kind {
            ParticleKind::Smoke => frac.min(0.18),
            ParticleKind::Flash => frac.powi(3),
            _ => {
                (frac * 3.0).min(1.0)
                    * (0.87
                        + 0.13
                            * ((self.body.max_life - self.body.life) * 102.0 + self.body.max_life)
                                .sin())
            }
        };

        // Distance dims remote particles while retaining their color and glow.
        let alpha = alpha * (scale / Camera::FRAMING).clamp(0.35, 1.0);
        (r, g, b, alpha)
    }

    fn write_points(&self, points: &mut Vec<f32>, camera: &Camera) {
        if self.char_idx == 0 {
            return;
        }
        let position = self.body.position;
        let Some((x, y, scale)) = camera.project(position.x, position.y, position.z) else {
            return;
        };
        let (r, g, b, alpha) = self.appearance(scale);
        let smoke = matches!(self.kind, ParticleKind::Smoke);
        let flash = matches!(self.kind, ParticleKind::Flash);
        let radius = if smoke {
            12.0
        } else if flash {
            22.0
        } else {
            1.6 + 1.4 * alpha
        };
        // Eight f32 values per point: projected position, radius, color, alpha and kind.
        points.extend_from_slice(&[
            x as f32,
            y as f32,
            (radius * scale.clamp(0.3, 3.0)) as f32,
            r as f32,
            g as f32,
            b as f32,
            alpha as f32,
            if smoke {
                1.0
            } else if flash {
                2.0
            } else {
                0.0
            },
        ]);
    }

    fn write_trails(&self, output: &mut Vec<f32>, camera: &Camera) {
        if self.char_idx == 0
            || matches!(
                self.kind,
                ParticleKind::Smoke | ParticleKind::GlitterDot | ParticleKind::Flash
            )
        {
            return;
        }
        let p = self.body.position;
        let Some(mut head) = camera.project(p.x, p.y, p.z) else {
            return;
        };
        let (r, g, b, alpha) = self.appearance(head.2);
        // Four tapered segments per star. History is sampled by simulation time,
        // so trails are stable at 30, 60, and 144 Hz and survive renderer switches.
        for (age, tail) in self
            .trail
            .samples()
            .enumerate()
            .filter(|(age, _)| age % 2 == 1)
        {
            if output.len() / 10 >= MAX_TRAIL_SEGMENTS {
                break;
            }
            let Some(end) = camera.project(tail.x, tail.y, tail.z) else {
                break;
            };
            let fade = (1.0 - age as f64 / 9.0).powi(2);
            output.extend_from_slice(&[
                head.0 as f32,
                head.1 as f32,
                end.0 as f32,
                end.1 as f32,
                ((if self.long_trail { 2.0 } else { 1.4 }) * head.2) as f32,
                r as f32,
                g as f32,
                b as f32,
                (alpha * fade * 0.8) as f32,
                0.0,
            ]);
            head = end;
        }
    }

}

// ─── Burst generators ───────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum FireworkType {
    Kiku,
    Botan,
    Yanagi,
    Kamuro,
    Senrin,
    Starmine,
    Ring,
    Crossette,
    Tourbillion,
    Brocade,
    Palm,
    Crown,
}

#[cfg(test)]
const ALL_TYPES: &[FireworkType] = &[
    FireworkType::Kiku,
    FireworkType::Botan,
    FireworkType::Yanagi,
    FireworkType::Kamuro,
    FireworkType::Senrin,
    FireworkType::Starmine,
    FireworkType::Ring,
    FireworkType::Crossette,
    FireworkType::Tourbillion,
    FireworkType::Brocade,
    FireworkType::Palm,
    FireworkType::Crown,
];

fn burst_kiku(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let count = rand_int(220, 280) as usize;
    let speed = rand(1.05, 1.35);
    let mut particles = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f64 / count as f64) * std::f64::consts::TAU + rand(-0.05, 0.05);
        let v = speed * rand(0.94, 1.06);
        let color = pick(colors);
        particles.push(Particle::new(
            cx,
            cy,
            angle.cos() * v,
            angle.sin() * v,
            rand(110.0, 160.0),
            color,
        ));
    }
    particles
}

fn burst_botan(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let count = rand_int(160, 220) as usize;
    let speed = rand(0.6, 1.0);
    let mut particles = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f64 / count as f64) * std::f64::consts::TAU + rand(-0.1, 0.1);
        let v = speed * rand(0.92, 1.08);
        let color = pick(colors);
        particles.push(Particle::new(
            cx,
            cy,
            angle.cos() * v,
            angle.sin() * v,
            rand(75.0, 115.0),
            color,
        ));
    }
    particles
}

fn burst_yanagi(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let count = rand_int(180, 240) as usize;
    let mut particles = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f64 / count as f64) * std::f64::consts::TAU + rand(-0.08, 0.08);
        let v = rand(0.5, 1.0);
        let color = pick(colors);
        particles.push(Particle::new(
            cx,
            cy,
            angle.cos() * v,
            angle.sin() * v * 0.7,
            rand(170.0, 240.0),
            color,
        ));
    }
    particles
}

fn burst_kamuro(cx: f64, cy: f64) -> Vec<Particle> {
    let count = rand_int(220, 300) as usize;
    let mut particles = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f64 / count as f64) * std::f64::consts::TAU + rand(-0.05, 0.05);
        let v = rand(0.3, 0.9);
        let color = if rand_f64() > 0.3 { GOLD } else { AMBER };
        particles.push(Particle::new(
            cx,
            cy,
            angle.cos() * v,
            angle.sin() * v - 0.2,
            rand(170.0, 240.0),
            color,
        ));
    }
    particles
}

fn burst_senrin(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let num_centers = rand_int(4, 8);
    let mut particles = Vec::new();

    for _ in 0..num_centers {
        let ox = cx + rand(-8.0, 8.0);
        let oy = cy + rand(-6.0, 6.0);
        let count = rand_int(15, 25) as usize;
        let speed = rand(0.3, 0.6);

        for i in 0..count {
            let angle = (i as f64 / count as f64) * std::f64::consts::TAU + rand(-0.15, 0.15);
            let v = speed * rand(0.5, 1.2);
            let color = pick(colors);
            particles.push(Particle::new(
                ox,
                oy,
                angle.cos() * v,
                angle.sin() * v,
                rand(25.0, 50.0),
                color,
            ));
        }
    }
    particles
}

fn burst_ring(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let count = 120;
    let speed = rand(0.95, 1.15);
    (0..count)
        .map(|i| {
            let angle = i as f64 / count as f64 * std::f64::consts::TAU;
            let mut p = Particle::new(
                cx,
                cy,
                angle.cos() * speed,
                angle.sin() * speed,
                rand(100.0, 140.0),
                pick(colors),
            );
            p.kind = ParticleKind::Ring;
            p
        })
        .collect()
}

fn burst_crossette(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let count = rand_int(8, 16) as usize;
    let speed = rand(0.8, 1.2);
    let mut particles = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f64 / count as f64) * std::f64::consts::TAU + rand(-0.1, 0.1);
        let v = speed * rand(0.8, 1.2);
        let color = pick(colors);
        let mut p = Particle::new(
            cx,
            cy,
            angle.cos() * v,
            angle.sin() * v,
            rand(50.0, 80.0),
            color,
        );
        p.kind = ParticleKind::Crossette {
            split_dist: rand(8.0, 15.0),
            dist_traveled: 0.0,
        };
        particles.push(p);
    }
    particles
}

fn burst_tourbillion(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let num_arms = rand_int(3, 5);
    let particles_per_arm = rand_int(20, 35) as usize;
    let mut particles = Vec::new();

    for arm in 0..num_arms {
        let base_angle = (arm as f64 / num_arms as f64) * std::f64::consts::TAU;
        let angular_vel = rand(0.03, 0.06) * 60.0 * if rand_f64() > 0.5 { 1.0 } else { -1.0 };
        let color = pick(colors);

        for j in 0..particles_per_arm {
            let phase_offset = j as f64 * 0.15;
            let mut p = Particle::new(cx, cy, 0.0, 0.0, rand(50.0, 90.0), color);
            p.kind = ParticleKind::Tourbillion {
                angular_vel,
                angle: base_angle + phase_offset,
                origin_x: cx,
                origin_y: cy,
            };
            particles.push(p);
        }
    }
    particles
}

fn burst_brocade(cx: f64, cy: f64) -> Vec<Particle> {
    let count = rand_int(240, 300) as usize;
    let mut particles = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f64 / count as f64) * std::f64::consts::TAU + rand(-0.08, 0.08);
        let v = rand(0.3, 0.8);
        let color = if rand_f64() > 0.3 { GOLD } else { COPPER };
        let mut p = Particle::new(
            cx,
            cy,
            angle.cos() * v,
            angle.sin() * v,
            rand(180.0, 260.0),
            color,
        );
        p.kind = ParticleKind::Brocade;
        particles.push(p);
    }
    particles
}

fn burst_palm(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let arms = rand_int(9, 13) as usize;
    let mut particles = Vec::with_capacity(arms * 11);
    for arm in 0..arms {
        let angle = arm as f64 / arms as f64 * std::f64::consts::TAU + rand(-0.035, 0.035);
        let color = pick(colors);
        let stars = rand_int(8, 12) as usize;
        for _ in 0..stars {
            let speed = rand(0.48, 0.88);
            let mut p = Particle::new(
                cx,
                cy,
                angle.cos() * speed,
                angle.sin() * speed,
                rand(190.0, 270.0),
                color,
            );
            p.kind = ParticleKind::Brocade;
            p.long_trail = true;
            particles.push(p);
        }
    }
    particles
}

fn burst_crown(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let rings = [(112, rand(0.62, 0.76)), (176, rand(1.0, 1.12))];
    let mut particles = Vec::with_capacity(rings[0].0 + rings[1].0);
    for (ring_index, (count, speed)) in rings.into_iter().enumerate() {
        let color = pick(colors);
        for i in 0..count {
            let angle = (i as f64 / count as f64) * std::f64::consts::TAU;
            let mut p = Particle::new(
                cx,
                cy,
                angle.cos() * speed,
                angle.sin() * speed,
                if ring_index == 0 { 105.0 } else { 145.0 },
                color,
            );
            p.kind = ParticleKind::Ring;
            p.long_trail = true;
            particles.push(p);
        }
    }
    particles
}

fn spawn_smoke(cx: f64, cy: f64) -> Vec<Particle> {
    let count = rand_int(10, 20) as usize;
    let mut particles = Vec::with_capacity(count);

    for _ in 0..count {
        let mut p = Particle::new(
            cx + rand(-3.0, 3.0),
            cy + rand(-2.0, 2.0),
            rand(-0.03, 0.03),
            0.0,
            rand(80.0, 150.0),
            DIM_GREY,
        );
        p.kind = ParticleKind::Smoke;
        particles.push(p);
    }
    particles
}

fn spawn_explosion_sparks(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let count = rand_int(20, 40) as usize;
    let mut particles = Vec::with_capacity(count);

    for _ in 0..count {
        let angle = rand(0.0, std::f64::consts::TAU);
        let v = rand(1.2, 2.5);
        let color = pick(colors);
        let mut p = Particle::new(
            cx,
            cy,
            angle.cos() * v,
            angle.sin() * v,
            rand(4.0, 10.0),
            color,
        );
        p.kind = ParticleKind::CracklingSpark;
        particles.push(p);
    }
    particles
}

fn spawn_burst(cx: f64, cy: f64, fw_type: FireworkType, colors: &[Color]) -> Vec<Particle> {
    let mut particles = match fw_type {
        FireworkType::Kiku => burst_kiku(cx, cy, colors),
        FireworkType::Botan | FireworkType::Starmine => burst_botan(cx, cy, colors),
        FireworkType::Yanagi => burst_yanagi(cx, cy, colors),
        FireworkType::Kamuro => burst_kamuro(cx, cy),
        FireworkType::Senrin => burst_senrin(cx, cy, colors),
        FireworkType::Ring => burst_ring(cx, cy, colors),
        FireworkType::Crossette => burst_crossette(cx, cy, colors),
        FireworkType::Tourbillion => burst_tourbillion(cx, cy, colors),
        FireworkType::Brocade => burst_brocade(cx, cy),
        FireworkType::Palm => burst_palm(cx, cy, colors),
        FireworkType::Crown => burst_crown(cx, cy, colors),
    };

    for p in &mut particles {
        p.long_trail = matches!(
            fw_type,
            FireworkType::Kiku
                | FireworkType::Yanagi
                | FireworkType::Kamuro
                | FireworkType::Brocade
                | FireworkType::Palm
                | FireworkType::Crown
        );
    }
    if matches!(
        fw_type,
        FireworkType::Kiku | FireworkType::Kamuro | FireworkType::Brocade
    ) {
        for i in 0..70 {
            let angle = i as f64 / 70.0 * std::f64::consts::TAU;
            let v = rand(0.35, 0.42);
            particles.push(Particle::new(
                cx,
                cy,
                angle.cos() * v,
                angle.sin() * v,
                rand(55.0, 85.0),
                WARM_WHITE,
            ));
        }
    }

    // Secondary effects (~38% of bursts)
    let roll = rand_f64();
    if roll < 0.15 {
        // Crackling sparks: tag ~40% of Normal particles
        for p in &mut particles {
            if rand_f64() < 0.4 && matches!(p.kind, ParticleKind::Normal) {
                p.kind = ParticleKind::CracklingSource;
            }
        }
    } else if roll < 0.25 {
        // Color-changing: lerp through 3 colors
        let c2 = pick(pick(COLOR_GROUPS));
        let c3 = pick(pick(COLOR_GROUPS));
        for p in &mut particles {
            if matches!(p.kind, ParticleKind::Normal) {
                p.kind = ParticleKind::ColorChanging {
                    r2: c2.r,
                    g2: c2.g,
                    b2: c2.b,
                    r3: c3.r,
                    g3: c3.g,
                    b3: c3.b,
                };
            }
        }
    } else if roll < 0.32 {
        // Glitter trails: drop stationary dots
        for p in &mut particles {
            if matches!(p.kind, ParticleKind::Normal) {
                p.kind = ParticleKind::GlitterTrail;
            }
        }
    } else if roll < 0.38 {
        // Strobe: blink on/off at ~3Hz
        for p in &mut particles {
            if matches!(p.kind, ParticleKind::Normal) {
                p.kind = ParticleKind::Strobe {
                    phase: rand(0.0, std::f64::consts::TAU),
                };
            }
        }
    }

    let mut flash = Particle::new(cx, cy, 0.0, 0.0, 10.0, WARM_WHITE);
    flash.kind = ParticleKind::Flash;
    particles.push(flash);

    // Explosion sparks on detonation
    particles.extend(spawn_explosion_sparks(cx, cy, colors));

    // Add smoke at burst center
    particles.extend(spawn_smoke(cx, cy));

    particles
}

// Give every burst volume and orient its motion in three dimensions.
fn give_depth(particles: &mut [Particle], cx: f64, cy: f64, z: f64, fw_type: FireworkType) {
    let tilt = rand(0.4, 1.1);
    for p in particles {
        p.body.position.z = z;
        if matches!(p.kind, ParticleKind::Flash) {
            continue;
        }
        if matches!(p.kind, ParticleKind::Smoke) {
            p.body.position.z += rand(-3.0, 3.0);
            p.body.velocity.z = rand(-0.03, 0.03) * 60.0;
            p.body.acceleration.y = 10.10665; // slight net buoyancy for smoke
        } else if matches!(fw_type, FireworkType::Ring | FireworkType::Crown)
            && matches!(p.kind, ParticleKind::Ring)
        {
            // Tilt a circular ring into a plane in 3D.
            let dy = p.body.position.y - cy;
            p.body.position.y = cy + dy * tilt.cos();
            p.body.position.z += dy * tilt.sin();
            p.body.velocity.z = p.body.velocity.y * tilt.sin();
            p.body.velocity.y *= tilt.cos();
        } else if fw_type == FireworkType::Tourbillion
            && matches!(p.kind, ParticleKind::Tourbillion { .. })
        {
            // Spinning arms spread forward/backward into a corkscrew.
            p.body.velocity.z = rand(-0.25, 0.25) * 60.0;
        } else {
            spread_velocity(p);
            // Multi-center bursts also occupy volume.
            p.body.position.z += (p.body.position.x - cx) * tilt.sin();
        }
        p.body.velocity.y *= 14.0 / 18.0;
    }
}

fn spread_velocity(p: &mut Particle) {
    let depth = rand(-1.0, 1.0);
    let planar = (1.0 - depth * depth).sqrt();
    let speed = p.body.velocity.x.hypot(p.body.velocity.y);
    p.body.velocity.x *= planar;
    p.body.velocity.y *= planar;
    p.body.velocity.z = speed * depth;
}

// ─── Launch Trail ───────────────────────────────────────────────────

struct LaunchTrail {
    body: Body,
    target_y: f64,
    wobble: f64,
    particles: Vec<Particle>,
    bursted: bool,
    fw_type: FireworkType,
    colors: Vec<Color>,
}

fn shell_apex(initial_x_speed: f64, initial_y_speed: f64, mass: f64, diameter: f64) -> Vec3 {
    let mut body = Body::new(
        Vec3::default(),
        Vec3::new(initial_x_speed, initial_y_speed, 0.0),
        30.0,
    );
    body.mass = mass;
    body.diameter = diameter;
    for _ in 0..30 * 60 {
        body.step(Clock::STEP_SECONDS);
        if body.velocity.y <= 0.0 {
            break;
        }
    }
    body.position
}

fn launch_speed_for_height(height: f64, mass: f64, diameter: f64, horizontal_speed: f64) -> f64 {
    let mut low = 0.0;
    let mut high = (2.0 * 9.80665 * height).sqrt().max(1.0);
    while shell_apex(horizontal_speed, high, mass, diameter).y < height {
        high *= 1.5;
    }
    for _ in 0..18 {
        let middle = (low + high) * 0.5;
        if shell_apex(horizontal_speed, middle, mass, diameter).y < height {
            low = middle;
        } else {
            high = middle;
        }
    }
    (low + high) * 0.5
}

impl LaunchTrail {
    fn new(target_x: f64, target_y: f64, fw_type: FireworkType, colors: Vec<Color>) -> Self {
        let shell_mass = 0.5;
        let shell_diameter = 0.10;
        let approximate_flight_time = 2.0 * (2.0 * target_y.max(1.0) / 9.80665).sqrt();
        let horizontal_speed = target_x / approximate_flight_time;
        let initial_speed = launch_speed_for_height(
            target_y.max(1.0),
            shell_mass,
            shell_diameter,
            horizontal_speed,
        );
        let mut body = Body::new(
            Vec3::default(),
            Vec3::new(horizontal_speed, initial_speed, 0.0),
            30.0,
        );
        // Representative medium aerial shell: drag uses its projected area and mass.
        body.mass = shell_mass;
        body.diameter = shell_diameter;
        Self {
            body,
            target_y,
            wobble: rand(0.0, std::f64::consts::TAU),
            particles: Vec::new(),
            bursted: false,
            fw_type,
            colors,
        }
    }

    /// Returns true if this launch just bursted this frame
    fn update(&mut self, dt: f64) -> bool {
        let mut just_bursted = false;

        if !self.bursted {
            self.body.step(dt);
            self.wobble += 9.0 * dt;
            self.body.position.x += self.wobble.sin() * 3.0 * dt;

            // Spark trail
            for _ in 0..2 {
                let mut spark = Particle::new(
                    self.body.position.x,
                    self.body.position.y,
                    rand(-0.05, 0.05),
                    -rand(0.05, 0.15),
                    rand(20.0, 35.0),
                    AMBER,
                );
                spark.body.position.z = self.body.position.z;
                self.particles.push(spark);
            }

            if self.body.velocity.y <= 0.0 || self.body.position.y >= self.target_y {
                self.bursted = true;
                just_bursted = true;
            }
        }

        self.particles.retain_mut(|p| p.update(dt));
        just_bursted
    }

    fn is_alive(&self) -> bool {
        !self.bursted || !self.particles.is_empty()
    }

}

// ─── Show Schedule ──────────────────────────────────────────────────

struct ScheduledBurst {
    time: f64,
    fw_type: FireworkType,
    x: i32,
    y: i32,
}

struct ShowState {
    time: f64,
    schedule: Vec<ScheduledBurst>,
    schedule_index: usize,
}

fn build_schedule(cols: usize, rows: usize) -> Vec<ScheduledBurst> {
    let mut schedule = Vec::new();
    let margin = (cols as f64 * 0.1).floor() as i32;
    // Spread launch targets vertically in portrait layouts so the show uses
    // the taller frame; landscape layouts keep the wider, layered composition.
    // Grid coordinates represent 14px horizontally and 18px vertically.
    let aspect = viewport_aspect(cols, rows);
    let vertical_spread = ((0.9 - aspect) * 0.36).clamp(0.0, 0.18);
    let top_zone = (rows as f64 * (0.45 - vertical_spread)).floor() as i32;
    let mid_zone = (rows as f64 * (0.60 + vertical_spread)).floor() as i32;
    let cols_i = cols as i32;

    let rx = || rand_int(margin, cols_i - margin);
    let ry = || rand_int(top_zone, mid_zone);

    // Opening (0-8s): Single kiku bursts
    schedule.push(ScheduledBurst {
        time: 0.15,
        fw_type: FireworkType::Kiku,
        x: cols_i / 2,
        y: top_zone,
    });
    let mut t = 3.5_f64;
    while t < 8.0 {
        schedule.push(ScheduledBurst {
            time: t,
            fw_type: FireworkType::Kiku,
            x: rx(),
            y: ry(),
        });
        t += rand(1.8, 2.5);
    }

    // Act 1 (8-25s): Mixed types + Ring
    t = 8.0;
    while t < 25.0 {
        let fw_type = pick(&[
            FireworkType::Kiku,
            FireworkType::Botan,
            FireworkType::Yanagi,
            FireworkType::Botan,
            FireworkType::Ring,
            FireworkType::Crown,
        ]);
        schedule.push(ScheduledBurst {
            time: t,
            fw_type,
            x: rx(),
            y: ry(),
        });
        if rand_f64() > 0.4 {
            schedule.push(ScheduledBurst {
                time: t + rand(0.1, 0.4),
                fw_type: pick(&[
                    FireworkType::Botan,
                    FireworkType::Kiku,
                    FireworkType::Ring,
                    FireworkType::Crown,
                ]),
                x: rx(),
                y: ry(),
            });
        }
        t += rand(1.2, 2.0);
    }

    // Act 2 (25-40s): Faster pace, senrin & kamuro + Crossette, Brocade
    t = 25.0;
    while t < 40.0 {
        let fw_type = pick(&[
            FireworkType::Senrin,
            FireworkType::Kamuro,
            FireworkType::Kiku,
            FireworkType::Yanagi,
            FireworkType::Botan,
            FireworkType::Crossette,
            FireworkType::Brocade,
            FireworkType::Palm,
            FireworkType::Crown,
        ]);
        schedule.push(ScheduledBurst {
            time: t,
            fw_type,
            x: rx(),
            y: ry(),
        });
        if rand_f64() > 0.3 {
            schedule.push(ScheduledBurst {
                time: t + rand(0.05, 0.3),
                fw_type: pick(&[
                    FireworkType::Senrin,
                    FireworkType::Botan,
                    FireworkType::Crossette,
                    FireworkType::Palm,
                ]),
                x: rx(),
                y: ry(),
            });
        }
        t += rand(0.8, 1.5);
    }

    // Starmine (40-50s): Rapid-fire + Tourbillion (~20%)
    t = 40.0;
    while t < 50.0 {
        let fw_type = if rand_f64() < 0.2 {
            FireworkType::Tourbillion
        } else {
            FireworkType::Starmine
        };
        schedule.push(ScheduledBurst {
            time: t,
            fw_type,
            x: rx(),
            y: ry(),
        });
        t += rand(0.3, 0.7);
    }

    // Finale: coordinated volleys, then a gold curtain with time to hang and fade.
    t = 50.0;
    while t < 62.0 {
        let fw_type = pick(&[
            FireworkType::Kiku,
            FireworkType::Botan,
            FireworkType::Crossette,
            FireworkType::Palm,
            FireworkType::Crown,
        ]);
        for lane in [0.22, 0.5, 0.78] {
            schedule.push(ScheduledBurst {
                time: t,
                fw_type,
                x: (cols as f64 * lane) as i32,
                y: ry(),
            });
        }
        t += 1.1;
    }
    for lane in [0.2, 0.35, 0.5, 0.65, 0.8] {
        schedule.push(ScheduledBurst {
            time: 63.0,
            fw_type: FireworkType::Brocade,
            x: (cols as f64 * lane) as i32,
            y: top_zone,
        });
    }

    schedule.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    schedule
}

fn viewport_aspect(cols: usize, rows: usize) -> f64 {
    // Grid coordinates represent 14px horizontally and 18px vertically.
    cols as f64 * 14.0 / (rows.max(1) as f64 * 18.0)
}

// ─── FireworkEngine (exported) ──────────────────────────────────────

#[wasm_bindgen]
pub struct FireworkEngine {
    clock: Clock,
    cols: usize,
    rows: usize,
    points: Vec<f32>,
    trails: Vec<f32>,
    particles: Particles<Particle>,
    launches: Vec<LaunchTrail>,
    show: ShowState,
}

#[wasm_bindgen]
impl FireworkEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32) -> Self {
        let cols = cols as usize;
        let rows = rows as usize;
        Self {
            clock: Clock::default(),
            cols,
            rows,
        points: Vec::with_capacity(MAX_PARTICLES * 8),
        trails: Vec::with_capacity(MAX_TRAIL_SEGMENTS * 10),
            particles: Particles::new(MAX_PARTICLES),
            launches: Vec::new(),
            show: ShowState {
                time: 0.0,
                schedule: build_schedule(cols, rows),
                schedule_index: 0,
            },
        }
    }

    pub fn tick(&mut self, dt_sec: f64) {
        for _ in 0..self.clock.advance(dt_sec) {
            self.step();
        }
        self.render();
    }

    fn step(&mut self) {
        let dt_sec = Clock::STEP_SECONDS;
        let dt = dt_sec;

        // 1. Advance show time and spawn new launches
        self.show.time += dt_sec;
        self.spawn_from_schedule();

        // 2. Update launches — burst on arrival
        let mut new_particles = Vec::new();
        for lt in &mut self.launches {
            let just_bursted = lt.update(dt);
            if just_bursted {
                let cx = lt.body.position.x.round();
                let cy = lt.body.position.y.round();
                let mut burst = spawn_burst(cx, cy, lt.fw_type, &lt.colors);
                give_depth(&mut burst, cx, cy, lt.body.position.z, lt.fw_type);
                new_particles.extend(burst);
            }
        }
        self.launches.retain(|lt| lt.is_alive());
        self.particles.emit(new_particles);

        // 3. Update particles with secondary spawning
        let mut secondary = Vec::new();
        let mut i = 0;
        while i < self.particles.items.len() {
            let alive = self.particles.items[i].update(dt);

            if alive {
                // GlitterTrail: 30% chance per frame to drop a stationary dot
                if matches!(self.particles.items[i].kind, ParticleKind::GlitterTrail)
                    && rand_f64() < 0.30
                {
                    let mut dot = Particle::new(
                        self.particles.items[i].body.position.x,
                        self.particles.items[i].body.position.y,
                        0.0,
                        0.0,
                        rand(30.0, 60.0),
                        Color {
                            r: self.particles.items[i].r,
                            g: self.particles.items[i].g,
                            b: self.particles.items[i].b,
                        },
                    );
                    dot.body.position.z = self.particles.items[i].body.position.z;
                    dot.kind = ParticleKind::GlitterDot;
                    secondary.push(dot);
                }

                // Crossette: split after traveling set distance
                let should_split = if let ParticleKind::Crossette {
                    split_dist,
                    dist_traveled,
                } = &self.particles.items[i].kind
                {
                    *dist_traveled >= *split_dist
                } else {
                    false
                };

                if should_split {
                    let cx = self.particles.items[i].body.position.x;
                    let cy = self.particles.items[i].body.position.y;
                    let pr = self.particles.items[i].r;
                    let pg = self.particles.items[i].g;
                    let pb = self.particles.items[i].b;
                    let count = rand_int(4, 6);
                    for j in 0..count {
                        let angle =
                            (j as f64 / count as f64) * std::f64::consts::TAU + rand(-0.2, 0.2);
                        let v = rand(0.3, 0.6);
                        let mut child = Particle::new(
                            cx,
                            cy,
                            angle.cos() * v,
                            angle.sin() * v,
                            rand(20.0, 40.0),
                            Color {
                                r: pr,
                                g: pg,
                                b: pb,
                            },
                        );
                        child.body.position.z = self.particles.items[i].body.position.z;
                        spread_velocity(&mut child);
                        secondary.push(child);
                    }
                    self.particles.items.swap_remove(i);
                    continue;
                }

                i += 1;
            } else {
                // CracklingSource: spawn sparks on death
                if matches!(self.particles.items[i].kind, ParticleKind::CracklingSource) {
                    let cx = self.particles.items[i].body.position.x;
                    let cy = self.particles.items[i].body.position.y;
                    let count = rand_int(3, 6);
                    for _ in 0..count {
                        let angle = rand(0.0, std::f64::consts::TAU);
                        let v = rand(0.5, 1.2);
                        let mut spark = Particle::new(
                            cx,
                            cy,
                            angle.cos() * v,
                            angle.sin() * v,
                            rand(5.0, 12.0),
                            WARM_WHITE,
                        );
                        spark.body.position.z = self.particles.items[i].body.position.z;
                        spread_velocity(&mut spark);
                        spark.kind = ParticleKind::CracklingSpark;
                        secondary.push(spark);
                    }
                }

                self.particles.items.swap_remove(i);
                // Don't increment i — swap_remove put a new element at i
            }
        }
        self.particles.emit(secondary);

        // 6. Loop show
        if self.show.time > SHOW_DURATION + 3.0 {
            self.show.time = 0.0;
            self.clock = Clock::default();
            self.show.schedule_index = 0;
            self.show.schedule = build_schedule(self.cols, self.rows);
        }
    }

    fn render(&mut self) {
        self.points.clear();
        self.trails.clear();
        let camera = Camera::new(self.cols, self.rows);
        for lt in &self.launches {
            if !lt.bursted {
                if let Some((x, y, scale)) =
                    camera.project(lt.body.position.x, lt.body.position.y, lt.body.position.z)
                {
                    self.points.extend_from_slice(&[
                        x as f32,
                        y as f32,
                        (2.4 * scale) as f32,
                        255.0,
                        225.0,
                        170.0,
                        (scale / Camera::FRAMING).min(1.0) as f32,
                        0.0,
                    ]);
                }
            }
            for p in &lt.particles {
                p.write_points(&mut self.points, &camera);
                p.write_trails(&mut self.trails, &camera);
            }
        }
        for p in &self.particles.items {
            p.write_points(&mut self.points, &camera);
            p.write_trails(&mut self.trails, &camera);
        }
    }

    pub fn resize(&mut self, cols: u32, rows: u32) {
        let cols = cols as usize;
        let rows = rows as usize;
        self.cols = cols;
        self.rows = rows;
        self.points.clear();
        self.trails.clear();
        self.show.schedule = build_schedule(cols, rows);
        self.show.schedule_index = self
            .show
            .schedule
            .partition_point(|burst| burst.time <= self.show.time);
    }

    /// Packed point data for smooth renderers. Length is in f32 elements, not bytes.
    pub fn points_ptr(&self) -> *const f32 {
        self.points.as_ptr()
    }

    pub fn points_len(&self) -> usize {
        self.points.len()
    }

    pub fn trails_ptr(&self) -> *const f32 {
        self.trails.as_ptr()
    }

    pub fn trails_len(&self) -> usize {
        self.trails.len()
    }

    pub fn cols(&self) -> u32 {
        self.cols as u32
    }

    pub fn rows(&self) -> u32 {
        self.rows as u32
    }

    fn spawn_from_schedule(&mut self) {
        while self.show.schedule_index < self.show.schedule.len()
            && self.show.schedule[self.show.schedule_index].time <= self.show.time
        {
            let burst = &self.show.schedule[self.show.schedule_index];
            self.show.schedule_index += 1;

            let colors = pick(COLOR_GROUPS).to_vec();
            let camera = Camera::new(self.cols, self.rows);
            let (target_x, target_y) = camera.world_at_screen(burst.x as f64, burst.y as f64);
            if burst.fw_type == FireworkType::Starmine {
                let mut new = spawn_burst(target_x, target_y, burst.fw_type, &colors);
                give_depth(&mut new, target_x, target_y, 0.0, burst.fw_type);
                self.particles.emit(new);
            } else {
                self.launches
                    .push(LaunchTrail::new(target_x, target_y, burst.fw_type, colors));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_burst_has_depth_and_expires() {
        for &kind in ALL_TYPES {
            let mut particles = spawn_burst(50.0, 20.0, kind, &[GOLD]);
            give_depth(&mut particles, 50.0, 20.0, 15.0, kind);
            assert!(!particles.is_empty());
            assert!(particles.iter().any(|p| p.body.position.z != 0.0));
            for _ in 0..300 {
                particles.retain_mut(|p| p.update(1.0));
                for p in &particles {
                    assert!(p.body.position.x.is_finite());
                    assert!(p.body.position.y.is_finite());
                    assert!(p.body.position.z.is_finite());
                }
            }
            assert!(particles.is_empty());
        }
    }

    #[test]
    fn launch_layout_adapts_to_portrait_and_landscape_viewports() {
        let portrait = build_schedule(40, 100);
        let landscape = build_schedule(180, 80);
        assert!(portrait.iter().all(|burst| (27..=78).contains(&burst.y)));
        assert!(landscape.iter().all(|burst| (36..=48).contains(&burst.y)));
        // Correct for the renderer's 14x18 pixel grid, not the number of cells.
        assert!((viewport_aspect(27, 46) - 0.457).abs() < 0.002);
        assert!((viewport_aspect(90, 40) - 1.75).abs() < 0.001);
    }

    #[test]
    fn complete_show_uses_depth_by_default_and_stays_bounded() {
        let mut engine = FireworkEngine::new(100, 60);
        let mut rendered = false;
        let mut has_depth = false;
        for _ in 0..(72 * 60) {
            engine.tick(Clock::STEP_SECONDS);
            has_depth |= engine
                .particles
                .items
                .iter()
                .any(|p| p.body.position.z != 0.0 && p.body.velocity.z != 0.0);
            assert!(engine.particles.items.len() <= MAX_PARTICLES);
            assert_eq!(engine.points_len() % 8, 0);
            assert_eq!(engine.trails_len() % 10, 0);
            assert!(engine.trails_len() <= MAX_TRAIL_SEGMENTS * 10);
            assert!(engine.trails.iter().all(|v| v.is_finite()));
            assert!(engine.points.iter().all(|value| value.is_finite()));
            for point in engine.points.chunks_exact(8) {
                assert!(point[2] > 0.0);
                assert!((0.0..=1.0).contains(&point[6]));
            }
            rendered |= !engine.points.is_empty();
        }
        assert!(rendered);
        assert!(has_depth);
        assert!(engine.show.time < 2.0, "show must loop");
    }

    #[test]
    fn point_output_preserves_subcell_positions_and_matches_appearance() {
        let mut particle = Particle::new(20.25, 10.75, 0.0, 0.0, 60.0, GOLD);
        particle.body.position.z = 15.0;
        let camera = Camera::new(100, 60);
        let mut points = Vec::new();
        particle.write_points(&mut points, &camera);
        assert_eq!(points.len(), 8);
        let (x, y, scale) = camera.project(20.25, 10.75, 15.0).unwrap();
        let (r, g, b, alpha) = particle.appearance(scale);
        assert_eq!(&points[..2], &[x as f32, y as f32]);
        assert_ne!(points[0].fract(), 0.0);
        assert_eq!(&points[3..7], &[r as f32, g as f32, b as f32, alpha as f32]);
        assert_eq!(points[7], 0.0);
        particle.char_idx = 0;
        particle.write_points(&mut points, &camera);
        assert_eq!(points.len(), 8, "invisible strobes must not emit points");
    }

    #[test]
    fn ring_expands_from_center_and_launch_slows_before_burst() {
        let ring = burst_ring(50.0, 20.0, &[GOLD]);
        assert!(ring
            .iter()
            .all(|p| p.body.position == Vec3::new(50.0, 20.0, 0.0)));
        assert!(ring.iter().all(|p| p.body.velocity.length() > 0.9));
        let mut launch = LaunchTrail::new(0.0, 15.0, FireworkType::Kiku, vec![GOLD]);
        let initial_speed = launch.body.velocity.y.abs();
        for _ in 0..30 * 60 {
            if launch.update(Clock::STEP_SECONDS) {
                break;
            }
        }
        assert!(launch.bursted);
        assert!(launch.body.velocity.y.abs() < initial_speed * 0.2);
        assert!((launch.body.position.y - 15.0).abs() < 1.0);
    }

    #[test]
    fn palm_is_a_long_trailing_fan_and_crown_has_two_shells() {
        let palm = burst_palm(50.0, 20.0, &[GOLD]);
        assert!((72..=156).contains(&palm.len()));
        assert!(palm.iter().all(|p| p.long_trail));
        assert!(palm.iter().all(|p| matches!(p.kind, ParticleKind::Brocade)));

        let crown = burst_crown(50.0, 20.0, &[GOLD]);
        assert_eq!(crown.len(), 288);
        let speeds = crown
            .iter()
            .map(|p| p.body.velocity.x.hypot(p.body.velocity.y))
            .collect::<Vec<_>>();
        assert!(speeds[..112].iter().all(|speed| (37.0..=46.0).contains(speed)));
        assert!(speeds[112..].iter().all(|speed| *speed >= 60.0));
        assert!(crown.iter().all(|p| p.long_trail));
    }

    #[test]
    fn trails_follow_world_history_and_cool_before_expiring() {
        let mut star = Particle::new(50.0, 20.0, 1.0, -0.5, 100.0, GOLD);
        star.body.position.z = 12.0;
        star.body.velocity.z = 0.5;
        let camera = Camera::new(100, 60);
        for _ in 0..30 {
            star.update(Clock::STEP_SECONDS);
        }
        assert!(star.appearance(Camera::FRAMING).3 > 0.7);
        let mut trails = Vec::new();
        star.write_trails(&mut trails, &camera);
        assert_eq!(trails.len(), 40);
        assert!(trails.chunks_exact(10).all(|s| s[0] > s[2]));
        assert!(trails[8] > trails[38]);
        for _ in 0..60 {
            star.update(Clock::STEP_SECONDS);
        }
        assert!(star.appearance(Camera::FRAMING).3 < 0.35);
    }

    #[test]
    fn resizing_does_not_replay_past_launches() {
        let mut engine = FireworkEngine::new(100, 60);
        engine.show.time = 30.0;
        engine.resize(40, 80);
        assert_eq!(engine.cols(), 40);
        assert_eq!(engine.rows(), 80);
        assert!(engine.show.schedule[..engine.show.schedule_index]
            .iter()
            .all(|b| b.time <= 30.0));
        assert!(engine.show.schedule[engine.show.schedule_index..]
            .iter()
            .all(|b| b.time > 30.0));
        engine.resize(0, 0);
        engine.tick(Clock::STEP_SECONDS);
        assert_eq!(engine.cols(), 0);
        assert_eq!(engine.rows(), 0);
    }
}
