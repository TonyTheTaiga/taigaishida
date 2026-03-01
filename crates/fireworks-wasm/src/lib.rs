use wasm_bindgen::prelude::*;

// ─── Constants ──────────────────────────────────────────────────────

const SHOW_DURATION: f64 = 68.0;
const FPS_FACTOR: f64 = 60.0;

// Character table — JS decodes by index
// 0=' ' 1='.' 2='·' 3='+' 4='*' 5='✦' 6='│' 7='╽' 8='o' 9='~' 10=''' 11='x' 12='%'
fn char_for_age(frac: f64) -> u8 {
    if frac > 0.7 { 5 }       // ✦
    else if frac > 0.5 { 4 }  // *
    else if frac > 0.3 { 3 }  // +
    else if frac > 0.15 { 2 } // ·
    else if frac > 0.05 { 1 } // .
    else { 0 }                // ' '
}

// ─── RNG helpers ────────────────────────────────────────────────────

fn rand_f64() -> f64 {
    js_sys::Math::random()
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

const GOLD: Color = Color { r: 255, g: 215, b: 0 };
const AMBER: Color = Color { r: 255, g: 165, b: 0 };
const SILVER: Color = Color { r: 192, g: 192, b: 192 };
const WHITE: Color = Color { r: 255, g: 255, b: 255 };
const RED: Color = Color { r: 255, g: 68, b: 68 };
const SOFT_RED: Color = Color { r: 255, g: 107, b: 107 };
const BLUE: Color = Color { r: 68, g: 136, b: 255 };
const SOFT_BLUE: Color = Color { r: 102, g: 187, b: 255 };
const GREEN: Color = Color { r: 68, g: 255, b: 136 };
const LIME: Color = Color { r: 136, g: 255, b: 68 };
const PURPLE: Color = Color { r: 187, g: 102, b: 255 };
const WARM_WHITE: Color = Color { r: 255, g: 240, b: 200 };
const COPPER: Color = Color { r: 184, g: 115, b: 51 };
const PINK: Color = Color { r: 255, g: 150, b: 200 };
const DIM_GREY: Color = Color { r: 80, g: 75, b: 70 };

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
    CracklingSource,
    CracklingSpark,
    ColorChanging { r2: u8, g2: u8, b2: u8, r3: u8, g3: u8, b3: u8 },
    GlitterTrail,
    GlitterDot,
    Strobe { phase: f64 },
    Crossette { split_dist: f64, dist_traveled: f64 },
    Tourbillion { angular_vel: f64, angle: f64, origin_x: f64, origin_y: f64 },
    Brocade,
    Smoke,
    Ring,
}

// ─── CharGrid ───────────────────────────────────────────────────────

// Each cell: [char_index, r, g, b, alpha_u8]
const CELL_SIZE: usize = 5;

struct CharGrid {
    cols: usize,
    rows: usize,
    buf: Vec<u8>,
}

impl CharGrid {
    fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            buf: vec![0u8; cols * rows * CELL_SIZE],
        }
    }

    fn clear(&mut self) {
        self.buf.fill(0);
    }

    fn set(&mut self, col: i32, row: i32, char_idx: u8, r: u8, g: u8, b: u8, alpha: f64) {
        if col < 0 || row < 0 {
            return;
        }
        let c = col as usize;
        let ro = row as usize;
        if c >= self.cols || ro >= self.rows {
            return;
        }
        let idx = (ro * self.cols + c) * CELL_SIZE;
        let alpha_u8 = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
        let existing_alpha = self.buf[idx + 4];
        if alpha_u8 > existing_alpha || (alpha_u8 == existing_alpha && char_idx != 0) {
            self.buf[idx] = char_idx;
            self.buf[idx + 1] = r;
            self.buf[idx + 2] = g;
            self.buf[idx + 3] = b;
            self.buf[idx + 4] = alpha_u8;
        }
    }
}

// ─── Particle ───────────────────────────────────────────────────────

struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    max_life: f64,
    char_idx: u8,
    r: u8,
    g: u8,
    b: u8,
    gravity: f64,
    drag: f64,
    kind: ParticleKind,
}

impl Particle {
    fn new(x: f64, y: f64, vx: f64, vy: f64, life: f64, color: Color, gravity: f64, drag: f64) -> Self {
        Self {
            x, y, vx, vy,
            life,
            max_life: life,
            char_idx: 5, // ✦
            r: color.r,
            g: color.g,
            b: color.b,
            gravity,
            drag,
            kind: ParticleKind::Normal,
        }
    }

    fn update(&mut self, dt: f64) -> bool {
        // Physics (kind-specific)
        match &mut self.kind {
            ParticleKind::Tourbillion { angular_vel, angle, origin_x, origin_y } => {
                *angle += *angular_vel * dt;
                *origin_y += 0.005 * dt;
                let frac_elapsed = 1.0 - (self.life / self.max_life);
                let radius = frac_elapsed * 15.0;
                self.x = *origin_x + radius * angle.cos();
                self.y = *origin_y + radius * angle.sin();
                self.life -= dt;
            }
            ParticleKind::GlitterDot => {
                self.life -= dt;
            }
            ParticleKind::Smoke => {
                self.x += self.vx * dt;
                self.vx *= self.drag;
                self.y -= 0.015 * dt;
                self.life -= dt;
            }
            ParticleKind::Crossette { dist_traveled, .. } => {
                self.vy += self.gravity * dt;
                self.vx *= self.drag;
                self.vy *= self.drag;
                let dx = self.vx * dt;
                let dy = self.vy * dt;
                self.x += dx;
                self.y += dy;
                *dist_traveled += (dx * dx + dy * dy).sqrt();
                self.life -= dt;
            }
            _ => {
                self.vy += self.gravity * dt;
                self.vx *= self.drag;
                self.vy *= self.drag;
                self.x += self.vx * dt;
                self.y += self.vy * dt;
                self.life -= dt;
            }
        }

        // Visual (kind-specific char_idx)
        let frac = (self.life / self.max_life).max(0.0);
        match &self.kind {
            ParticleKind::Smoke => {
                self.char_idx = if frac > 0.5 { 9 } else { 1 };
            }
            ParticleKind::GlitterDot => {
                self.char_idx = 10;
            }
            ParticleKind::Brocade => {
                self.char_idx = if frac > 0.3 { 12 } else if frac > 0.1 { 2 } else { 1 };
            }
            ParticleKind::Ring => {
                self.char_idx = if frac > 0.3 { 8 } else if frac > 0.1 { 2 } else { 1 };
            }
            ParticleKind::Strobe { phase } => {
                let t = (self.max_life - self.life) * 0.3 + phase;
                self.char_idx = if t.sin() > 0.0 { char_for_age(frac) } else { 0 };
            }
            _ => {
                self.char_idx = char_for_age(frac);
            }
        }

        // Alive check
        match &self.kind {
            ParticleKind::Strobe { .. } => self.life > 0.0,
            _ => self.life > 0.0 && self.char_idx != 0,
        }
    }

    fn write(&self, grid: &mut CharGrid) {
        if self.char_idx == 0 { return; }

        let col = self.x.round() as i32;
        let row = self.y.round() as i32;
        let frac = (self.life / self.max_life).max(0.0);

        // Color-changing: lerp through 3 colors over lifetime
        let (base_r, base_g, base_b) = match &self.kind {
            ParticleKind::ColorChanging { r2, g2, b2, r3, g3, b3 } => {
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

        let fade = frac.sqrt();
        let r = lerp(40.0, base_r as f64, fade).round() as u8;
        let g = lerp(20.0, base_g as f64, fade).round() as u8;
        let b = lerp(15.0, base_b as f64, fade).round() as u8;

        let alpha = match &self.kind {
            ParticleKind::Smoke => frac.min(0.25),
            _ => frac,
        };

        grid.set(col, row, self.char_idx, r, g, b, alpha);

        // Bloom/glow: bright particles bleed into 4 adjacent cells
        let skip_bloom = matches!(
            self.kind,
            ParticleKind::Smoke | ParticleKind::GlitterDot | ParticleKind::CracklingSpark
        );
        if alpha > 0.5 && self.char_idx >= 4 && !skip_bloom {
            let bloom_alpha = alpha * 0.25;
            let br = (r as f64 * 0.6).round() as u8;
            let bg = (g as f64 * 0.6).round() as u8;
            let bb = (b as f64 * 0.6).round() as u8;
            grid.set(col - 1, row, 2, br, bg, bb, bloom_alpha);
            grid.set(col + 1, row, 2, br, bg, bb, bloom_alpha);
            grid.set(col, row - 1, 2, br, bg, bb, bloom_alpha);
            grid.set(col, row + 1, 2, br, bg, bb, bloom_alpha);
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
}

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
];

fn burst_kiku(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let count = rand_int(80, 120) as usize;
    let speed = rand(0.8, 1.4);
    let mut particles = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f64 / count as f64) * std::f64::consts::TAU + rand(-0.05, 0.05);
        let v = speed * rand(0.7, 1.3);
        let color = pick(colors);
        particles.push(Particle::new(
            cx, cy,
            angle.cos() * v, angle.sin() * v,
            rand(60.0, 100.0), color, 0.015, 0.985,
        ));
    }
    particles
}

fn burst_botan(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let count = rand_int(60, 90) as usize;
    let speed = rand(0.6, 1.0);
    let mut particles = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f64 / count as f64) * std::f64::consts::TAU + rand(-0.1, 0.1);
        let v = speed * rand(0.6, 1.2);
        let color = pick(colors);
        particles.push(Particle::new(
            cx, cy,
            angle.cos() * v, angle.sin() * v,
            rand(30.0, 55.0), color, 0.025, 0.97,
        ));
    }
    particles
}

fn burst_yanagi(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let count = rand_int(50, 80) as usize;
    let mut particles = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f64 / count as f64) * std::f64::consts::TAU + rand(-0.08, 0.08);
        let v = rand(0.5, 1.0);
        let color = pick(colors);
        particles.push(Particle::new(
            cx, cy,
            angle.cos() * v, angle.sin() * v * 0.7,
            rand(70.0, 120.0), color, 0.04, 0.99,
        ));
    }
    particles
}

fn burst_kamuro(cx: f64, cy: f64) -> Vec<Particle> {
    let count = rand_int(100, 150) as usize;
    let mut particles = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f64 / count as f64) * std::f64::consts::TAU + rand(-0.05, 0.05);
        let v = rand(0.3, 0.9);
        let color = if rand_f64() > 0.3 { GOLD } else { AMBER };
        particles.push(Particle::new(
            cx, cy,
            angle.cos() * v, angle.sin() * v - 0.2,
            rand(80.0, 140.0), color, 0.05, 0.992,
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
                ox, oy,
                angle.cos() * v, angle.sin() * v,
                rand(25.0, 50.0), color, 0.02, 0.975,
            ));
        }
    }
    particles
}

fn burst_ring(cx: f64, cy: f64, colors: &[Color]) -> Vec<Particle> {
    let count = rand_int(60, 80) as usize;
    let radius = rand(6.0, 12.0);
    let mut particles = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f64 / count as f64) * std::f64::consts::TAU;
        let x = cx + angle.cos() * radius;
        let y = cy + angle.sin() * radius;
        let tangent = angle + std::f64::consts::FRAC_PI_2;
        let drift = rand(-0.05, 0.05);
        let color = pick(colors);
        let mut p = Particle::new(
            x, y,
            tangent.cos() * drift, tangent.sin() * drift,
            rand(40.0, 70.0), color, 0.01, 0.99,
        );
        p.kind = ParticleKind::Ring;
        particles.push(p);
    }
    particles
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
            cx, cy,
            angle.cos() * v, angle.sin() * v,
            rand(50.0, 80.0), color, 0.015, 0.985,
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
        let angular_vel = rand(0.03, 0.06) * if rand_f64() > 0.5 { 1.0 } else { -1.0 };
        let color = pick(colors);

        for j in 0..particles_per_arm {
            let phase_offset = j as f64 * 0.15;
            let mut p = Particle::new(
                cx, cy,
                0.0, 0.0,
                rand(50.0, 90.0), color, 0.01, 0.99,
            );
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
    let count = rand_int(120, 180) as usize;
    let mut particles = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as f64 / count as f64) * std::f64::consts::TAU + rand(-0.08, 0.08);
        let v = rand(0.3, 0.8);
        let color = if rand_f64() > 0.3 { GOLD } else { COPPER };
        let mut p = Particle::new(
            cx, cy,
            angle.cos() * v, angle.sin() * v,
            rand(100.0, 180.0), color, 0.04, 0.995,
        );
        p.kind = ParticleKind::Brocade;
        particles.push(p);
    }
    particles
}

fn spawn_smoke(cx: f64, cy: f64) -> Vec<Particle> {
    let count = rand_int(10, 20) as usize;
    let mut particles = Vec::with_capacity(count);

    for _ in 0..count {
        let mut p = Particle::new(
            cx + rand(-3.0, 3.0), cy + rand(-2.0, 2.0),
            rand(-0.03, 0.03), 0.0,
            rand(80.0, 150.0), DIM_GREY, 0.0, 0.998,
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
            cx, cy,
            angle.cos() * v, angle.sin() * v,
            rand(4.0, 10.0), color, 0.008, 0.90,
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
    };

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
                    r2: c2.r, g2: c2.g, b2: c2.b,
                    r3: c3.r, g3: c3.g, b3: c3.b,
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
                p.kind = ParticleKind::Strobe { phase: rand(0.0, std::f64::consts::TAU) };
            }
        }
    }

    // Explosion sparks on detonation
    particles.extend(spawn_explosion_sparks(cx, cy, colors));

    // Add smoke at burst center
    particles.extend(spawn_smoke(cx, cy));

    particles
}

// ─── Launch Trail ───────────────────────────────────────────────────

struct LaunchTrail {
    x: f64,
    y: f64,
    target_y: f64,
    vy: f64,
    wobble: f64,
    particles: Vec<Particle>,
    bursted: bool,
    fw_type: FireworkType,
    colors: Vec<Color>,
}

impl LaunchTrail {
    fn new(x: f64, start_y: f64, target_y: f64, fw_type: FireworkType, colors: Vec<Color>) -> Self {
        Self {
            x,
            y: start_y,
            target_y,
            vy: rand(-1.2, -0.8),
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
            self.y += self.vy * dt;
            self.wobble += 0.15 * dt;
            self.x += self.wobble.sin() * 0.05 * dt;

            // Spark trail
            if rand_f64() > 0.3 {
                self.particles.push(Particle::new(
                    self.x, self.y,
                    rand(-0.05, 0.05), rand(0.05, 0.15),
                    rand(8.0, 18.0), AMBER, 0.01, 0.95,
                ));
            }

            if self.y <= self.target_y {
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

    fn write(&self, grid: &mut CharGrid) {
        if !self.bursted {
            let col = self.x.round() as i32;
            let row = self.y.round() as i32;
            grid.set(col, row, 6, 220, 200, 150, 1.0); // │
            grid.set(col, row - 1, 7, 255, 240, 180, 0.8); // ╽
        }
        for p in &self.particles {
            p.write(grid);
        }
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
    let top_zone = (rows as f64 * 0.15).floor() as i32;
    let mid_zone = (rows as f64 * 0.4).floor() as i32;
    let cols_i = cols as i32;

    let rx = || rand_int(margin, cols_i - margin);
    let ry = || rand_int(top_zone, mid_zone);

    // Opening (0-8s): Single kiku bursts
    let mut t = 1.0_f64;
    while t < 8.0 {
        schedule.push(ScheduledBurst { time: t, fw_type: FireworkType::Kiku, x: rx(), y: ry() });
        t += rand(1.8, 2.5);
    }

    // Act 1 (8-25s): Mixed types + Ring
    t = 8.0;
    while t < 25.0 {
        let fw_type = pick(&[
            FireworkType::Kiku, FireworkType::Botan,
            FireworkType::Yanagi, FireworkType::Botan, FireworkType::Ring,
        ]);
        schedule.push(ScheduledBurst { time: t, fw_type, x: rx(), y: ry() });
        if rand_f64() > 0.4 {
            schedule.push(ScheduledBurst {
                time: t + rand(0.1, 0.4),
                fw_type: pick(&[FireworkType::Botan, FireworkType::Kiku, FireworkType::Ring]),
                x: rx(), y: ry(),
            });
        }
        t += rand(1.2, 2.0);
    }

    // Act 2 (25-40s): Faster pace, senrin & kamuro + Crossette, Brocade
    t = 25.0;
    while t < 40.0 {
        let fw_type = pick(&[
            FireworkType::Senrin, FireworkType::Kamuro, FireworkType::Kiku,
            FireworkType::Yanagi, FireworkType::Botan,
            FireworkType::Crossette, FireworkType::Brocade,
        ]);
        schedule.push(ScheduledBurst { time: t, fw_type, x: rx(), y: ry() });
        if rand_f64() > 0.3 {
            schedule.push(ScheduledBurst {
                time: t + rand(0.05, 0.3),
                fw_type: pick(&[FireworkType::Senrin, FireworkType::Botan, FireworkType::Crossette]),
                x: rx(), y: ry(),
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
        schedule.push(ScheduledBurst { time: t, fw_type, x: rx(), y: ry() });
        t += rand(0.3, 0.7);
    }

    // Grand Finale (50-65s): All 10 types
    t = 50.0;
    while t < 65.0 {
        let fw_type = pick(ALL_TYPES);
        schedule.push(ScheduledBurst { time: t, fw_type, x: rx(), y: ry() });
        t += rand(0.15, 0.4);
    }

    schedule.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    schedule
}

// ─── FireworkEngine (exported) ──────────────────────────────────────

#[wasm_bindgen]
pub struct FireworkEngine {
    grid: CharGrid,
    particles: Vec<Particle>,
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
            grid: CharGrid::new(cols, rows),
            particles: Vec::new(),
            launches: Vec::new(),
            show: ShowState {
                time: 0.0,
                schedule: build_schedule(cols, rows),
                schedule_index: 0,
            },
        }
    }

    pub fn tick(&mut self, dt_sec: f64) {
        let dt_sec = dt_sec.min(0.05);
        let dt = dt_sec * FPS_FACTOR;

        // 1. Advance show time and spawn new launches
        self.show.time += dt_sec;
        self.spawn_from_schedule();

        // 2. Update launches — burst on arrival
        let mut new_particles = Vec::new();
        for lt in &mut self.launches {
            let just_bursted = lt.update(dt);
            if just_bursted {
                let cx = lt.x.round();
                let cy = lt.y.round();
                new_particles.extend(spawn_burst(cx, cy, lt.fw_type, &lt.colors));
            }
        }
        self.launches.retain(|lt| lt.is_alive());
        self.particles.append(&mut new_particles);

        // 3. Update particles with secondary spawning
        let mut secondary = Vec::new();
        let mut i = 0;
        while i < self.particles.len() {
            let alive = self.particles[i].update(dt);

            if alive {
                // GlitterTrail: 30% chance per frame to drop a stationary dot
                if matches!(self.particles[i].kind, ParticleKind::GlitterTrail) && rand_f64() < 0.30 {
                    let mut dot = Particle::new(
                        self.particles[i].x, self.particles[i].y,
                        0.0, 0.0,
                        rand(30.0, 60.0), Color { r: self.particles[i].r, g: self.particles[i].g, b: self.particles[i].b },
                        0.0, 1.0,
                    );
                    dot.kind = ParticleKind::GlitterDot;
                    secondary.push(dot);
                }

                // Crossette: split after traveling set distance
                let should_split = if let ParticleKind::Crossette { split_dist, dist_traveled } = &self.particles[i].kind {
                    *dist_traveled >= *split_dist
                } else {
                    false
                };

                if should_split {
                    let cx = self.particles[i].x;
                    let cy = self.particles[i].y;
                    let pr = self.particles[i].r;
                    let pg = self.particles[i].g;
                    let pb = self.particles[i].b;
                    let count = rand_int(4, 6);
                    for j in 0..count {
                        let angle = (j as f64 / count as f64) * std::f64::consts::TAU + rand(-0.2, 0.2);
                        let v = rand(0.3, 0.6);
                        secondary.push(Particle::new(
                            cx, cy,
                            angle.cos() * v, angle.sin() * v,
                            rand(20.0, 40.0), Color { r: pr, g: pg, b: pb },
                            0.02, 0.97,
                        ));
                    }
                    self.particles.swap_remove(i);
                    continue;
                }

                i += 1;
            } else {
                // CracklingSource: spawn sparks on death
                if matches!(self.particles[i].kind, ParticleKind::CracklingSource) {
                    let cx = self.particles[i].x;
                    let cy = self.particles[i].y;
                    let count = rand_int(3, 6);
                    for _ in 0..count {
                        let angle = rand(0.0, std::f64::consts::TAU);
                        let v = rand(0.5, 1.2);
                        let mut spark = Particle::new(
                            cx, cy,
                            angle.cos() * v, angle.sin() * v,
                            rand(5.0, 12.0), WARM_WHITE,
                            0.01, 0.92,
                        );
                        spark.kind = ParticleKind::CracklingSpark;
                        secondary.push(spark);
                    }
                }

                self.particles.swap_remove(i);
                // Don't increment i — swap_remove put a new element at i
            }
        }
        self.particles.append(&mut secondary);

        // 4. Particle cap
        if self.particles.len() > 2000 {
            self.particles.sort_by(|a, b| b.life.partial_cmp(&a.life).unwrap());
            self.particles.truncate(1500);
        }

        // 5. Clear grid and write
        self.grid.clear();
        for lt in &self.launches {
            lt.write(&mut self.grid);
        }
        for p in &self.particles {
            p.write(&mut self.grid);
        }

        // 6. Loop show
        if self.show.time > SHOW_DURATION + 3.0 {
            self.show.time = 0.0;
            self.show.schedule_index = 0;
            self.show.schedule = build_schedule(self.grid.cols, self.grid.rows);
        }
    }

    pub fn resize(&mut self, cols: u32, rows: u32) {
        let cols = cols as usize;
        let rows = rows as usize;
        self.grid = CharGrid::new(cols, rows);
        self.show.schedule = build_schedule(cols, rows);
    }

    pub fn grid_ptr(&self) -> *const u8 {
        self.grid.buf.as_ptr()
    }

    pub fn grid_len(&self) -> usize {
        self.grid.buf.len()
    }

    pub fn cols(&self) -> u32 {
        self.grid.cols as u32
    }

    pub fn rows(&self) -> u32 {
        self.grid.rows as u32
    }

    fn spawn_from_schedule(&mut self) {
        while self.show.schedule_index < self.show.schedule.len()
            && self.show.schedule[self.show.schedule_index].time <= self.show.time
        {
            let burst = &self.show.schedule[self.show.schedule_index];
            self.show.schedule_index += 1;

            let colors = pick(COLOR_GROUPS).to_vec();
            if burst.fw_type == FireworkType::Starmine {
                let new = spawn_burst(burst.x as f64, burst.y as f64, burst.fw_type, &colors);
                self.particles.extend(new);
            } else {
                self.launches.push(LaunchTrail::new(
                    burst.x as f64,
                    self.grid.rows as f64,
                    burst.y as f64,
                    burst.fw_type,
                    colors,
                ));
            }
        }
    }
}
