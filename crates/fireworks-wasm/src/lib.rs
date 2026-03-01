use wasm_bindgen::prelude::*;

// ─── Constants ──────────────────────────────────────────────────────

const SHOW_DURATION: f64 = 68.0;
const FPS_FACTOR: f64 = 60.0;

// Character table — JS decodes by index
// 0=' ' 1='.' 2='·' 3='+' 4='*' 5='✦' 6='│' 7='╽'
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

const COLOR_GROUPS: &[&[Color]] = &[
    &[GOLD, AMBER],
    &[SILVER, WHITE],
    &[RED, SOFT_RED],
    &[BLUE, SOFT_BLUE],
    &[GREEN, LIME],
    &[PURPLE, SOFT_BLUE],
    &[GOLD, RED],
    &[SILVER, BLUE],
];

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
        }
    }

    fn update(&mut self, dt: f64) -> bool {
        self.vy += self.gravity * dt;
        self.vx *= self.drag;
        self.vy *= self.drag;
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.life -= dt;

        let frac = (self.life / self.max_life).max(0.0);
        self.char_idx = char_for_age(frac);

        self.life > 0.0 && self.char_idx != 0
    }

    fn write(&self, grid: &mut CharGrid) {
        let col = self.x.round() as i32;
        let row = self.y.round() as i32;
        let frac = (self.life / self.max_life).max(0.0);

        let fade = frac.sqrt();
        let r = lerp(40.0, self.r as f64, fade).round() as u8;
        let g = lerp(20.0, self.g as f64, fade).round() as u8;
        let b = lerp(15.0, self.b as f64, fade).round() as u8;

        grid.set(col, row, self.char_idx, r, g, b, frac);
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
}

const ALL_TYPES: &[FireworkType] = &[
    FireworkType::Kiku,
    FireworkType::Botan,
    FireworkType::Yanagi,
    FireworkType::Kamuro,
    FireworkType::Senrin,
    FireworkType::Starmine,
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

fn spawn_burst(cx: f64, cy: f64, fw_type: FireworkType, colors: &[Color]) -> Vec<Particle> {
    match fw_type {
        FireworkType::Kiku => burst_kiku(cx, cy, colors),
        FireworkType::Botan | FireworkType::Starmine => burst_botan(cx, cy, colors),
        FireworkType::Yanagi => burst_yanagi(cx, cy, colors),
        FireworkType::Kamuro => burst_kamuro(cx, cy),
        FireworkType::Senrin => burst_senrin(cx, cy, colors),
    }
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

    // Act 1 (8-25s): Mixed types, 2-3 at a time
    t = 8.0;
    while t < 25.0 {
        let fw_type = pick(&[FireworkType::Kiku, FireworkType::Botan, FireworkType::Yanagi, FireworkType::Botan]);
        schedule.push(ScheduledBurst { time: t, fw_type, x: rx(), y: ry() });
        if rand_f64() > 0.4 {
            schedule.push(ScheduledBurst {
                time: t + rand(0.1, 0.4),
                fw_type: pick(&[FireworkType::Botan, FireworkType::Kiku]),
                x: rx(), y: ry(),
            });
        }
        t += rand(1.2, 2.0);
    }

    // Act 2 (25-40s): Faster pace, senrin & kamuro
    t = 25.0;
    while t < 40.0 {
        let fw_type = pick(&[FireworkType::Senrin, FireworkType::Kamuro, FireworkType::Kiku, FireworkType::Yanagi, FireworkType::Botan]);
        schedule.push(ScheduledBurst { time: t, fw_type, x: rx(), y: ry() });
        if rand_f64() > 0.3 {
            schedule.push(ScheduledBurst {
                time: t + rand(0.05, 0.3),
                fw_type: pick(&[FireworkType::Senrin, FireworkType::Botan]),
                x: rx(), y: ry(),
            });
        }
        t += rand(0.8, 1.5);
    }

    // Starmine (40-50s): Rapid-fire across width
    t = 40.0;
    while t < 50.0 {
        schedule.push(ScheduledBurst { time: t, fw_type: FireworkType::Starmine, x: rx(), y: ry() });
        t += rand(0.3, 0.7);
    }

    // Grand Finale (50-65s): Everything
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

        // 3. Update particles
        self.particles.retain_mut(|p| p.update(dt));

        // 4. Clear grid and write
        self.grid.clear();
        for lt in &self.launches {
            lt.write(&mut self.grid);
        }
        for p in &self.particles {
            p.write(&mut self.grid);
        }

        // 5. Loop show
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
