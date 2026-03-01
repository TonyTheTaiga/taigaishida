// ─── Types ───────────────────────────────────────────────────────────

interface Cell {
	char: string;
	r: number;
	g: number;
	b: number;
	alpha: number;
}

interface Particle {
	x: number;
	y: number;
	vx: number;
	vy: number;
	life: number;
	maxLife: number;
	char: string;
	r: number;
	g: number;
	b: number;
	gravity: number;
	drag: number;
}

type FireworkType = 'kiku' | 'botan' | 'yanagi' | 'kamuro' | 'senrin' | 'starmine';

interface ScheduledBurst {
	time: number;
	type: FireworkType;
	x: number;
	y: number;
}

// ─── Color Palette ───────────────────────────────────────────────────

const PALETTE = {
	gold: { r: 255, g: 215, b: 0 },
	amber: { r: 255, g: 165, b: 0 },
	silver: { r: 192, g: 192, b: 192 },
	white: { r: 255, g: 255, b: 255 },
	red: { r: 255, g: 68, b: 68 },
	softRed: { r: 255, g: 107, b: 107 },
	blue: { r: 68, g: 136, b: 255 },
	softBlue: { r: 102, g: 187, b: 255 },
	green: { r: 68, g: 255, b: 136 },
	lime: { r: 136, g: 255, b: 68 },
	purple: { r: 187, g: 102, b: 255 },
};

type PaletteKey = keyof typeof PALETTE;

const COLOR_GROUPS: PaletteKey[][] = [
	['gold', 'amber'],
	['silver', 'white'],
	['red', 'softRed'],
	['blue', 'softBlue'],
	['green', 'lime'],
	['purple', 'softBlue'],
	['gold', 'red'],
	['silver', 'blue'],
];

// ─── Utilities ───────────────────────────────────────────────────────

function rand(min: number, max: number): number {
	return Math.random() * (max - min) + min;
}

function randInt(min: number, max: number): number {
	return Math.floor(rand(min, max + 1));
}

function pick<T>(arr: T[]): T {
	return arr[Math.floor(Math.random() * arr.length)];
}

function lerp(a: number, b: number, t: number): number {
	return a + (b - a) * t;
}

function charForAge(frac: number): string {
	if (frac > 0.7) return '✦';
	if (frac > 0.5) return '*';
	if (frac > 0.3) return '+';
	if (frac > 0.15) return '·';
	if (frac > 0.05) return '.';
	return ' ';
}

// ─── CharGrid ────────────────────────────────────────────────────────

class CharGrid {
	cols: number;
	rows: number;
	cells: Cell[][];

	constructor(cols: number, rows: number) {
		this.cols = cols;
		this.rows = rows;
		this.cells = [];
		for (let r = 0; r < rows; r++) {
			const row: Cell[] = [];
			for (let c = 0; c < cols; c++) {
				row.push({ char: ' ', r: 0, g: 0, b: 0, alpha: 0 });
			}
			this.cells.push(row);
		}
	}

	clear() {
		for (let r = 0; r < this.rows; r++) {
			for (let c = 0; c < this.cols; c++) {
				const cell = this.cells[r][c];
				cell.char = ' ';
				cell.r = 0;
				cell.g = 0;
				cell.b = 0;
				cell.alpha = 0;
			}
		}
	}

	set(col: number, row: number, char: string, r: number, g: number, b: number, alpha: number) {
		if (col < 0 || col >= this.cols || row < 0 || row >= this.rows) return;
		const cell = this.cells[row][col];
		if (alpha > cell.alpha || (alpha === cell.alpha && char !== ' ')) {
			cell.char = char;
			cell.r = r;
			cell.g = g;
			cell.b = b;
			cell.alpha = alpha;
		}
	}
}

// ─── Renderer ────────────────────────────────────────────────────────

class Renderer {
	private ctx: CanvasRenderingContext2D;
	private cellW: number;
	private cellH: number;

	constructor(ctx: CanvasRenderingContext2D, cellW: number, cellH: number) {
		this.ctx = ctx;
		this.cellW = cellW;
		this.cellH = cellH;
	}

	draw(grid: CharGrid, canvasW: number, canvasH: number) {
		const ctx = this.ctx;

		ctx.fillStyle = '#080810';
		ctx.fillRect(0, 0, canvasW, canvasH);

		ctx.font = `${this.cellH}px monospace`;
		ctx.textBaseline = 'top';

		for (let r = 0; r < grid.rows; r++) {
			for (let c = 0; c < grid.cols; c++) {
				const cell = grid.cells[r][c];
				if (cell.char === ' ' || cell.alpha <= 0) continue;

				const a = Math.min(1, cell.alpha);
				ctx.fillStyle = `rgba(${cell.r},${cell.g},${cell.b},${a})`;
				ctx.fillText(cell.char, c * this.cellW, r * this.cellH);
			}
		}
	}
}

// ─── Particle system ─────────────────────────────────────────────────

function createParticle(
	x: number,
	y: number,
	vx: number,
	vy: number,
	life: number,
	color: { r: number; g: number; b: number },
	gravity: number = 0.03,
	drag: number = 0.98
): Particle {
	return {
		x, y, vx, vy,
		life, maxLife: life,
		char: '✦',
		r: color.r, g: color.g, b: color.b,
		gravity, drag,
	};
}

function updateParticle(p: Particle, dt: number): boolean {
	p.vy += p.gravity * dt;
	p.vx *= p.drag;
	p.vy *= p.drag;
	p.x += p.vx * dt;
	p.y += p.vy * dt;
	p.life -= dt;

	const frac = Math.max(0, p.life / p.maxLife);
	p.char = charForAge(frac);

	return p.life > 0 && p.char !== ' ';
}

function writeParticle(p: Particle, grid: CharGrid) {
	const col = Math.round(p.x);
	const row = Math.round(p.y);
	const frac = Math.max(0, p.life / p.maxLife);

	const fade = Math.pow(frac, 0.5);
	const r = Math.round(lerp(40, p.r, fade));
	const g = Math.round(lerp(20, p.g, fade));
	const b = Math.round(lerp(15, p.b, fade));

	grid.set(col, row, p.char, r, g, b, frac);
}

// ─── Firework Burst Generators ───────────────────────────────────────

function burstKiku(cx: number, cy: number, colors: PaletteKey[]): Particle[] {
	const particles: Particle[] = [];
	const count = randInt(80, 120);
	const speed = rand(0.8, 1.4);

	for (let i = 0; i < count; i++) {
		const angle = (i / count) * Math.PI * 2 + rand(-0.05, 0.05);
		const v = speed * rand(0.7, 1.3);
		const color = PALETTE[pick(colors)];
		particles.push(createParticle(cx, cy, Math.cos(angle) * v, Math.sin(angle) * v, rand(60, 100), color, 0.015, 0.985));
	}
	return particles;
}

function burstBotan(cx: number, cy: number, colors: PaletteKey[]): Particle[] {
	const particles: Particle[] = [];
	const count = randInt(60, 90);
	const speed = rand(0.6, 1.0);

	for (let i = 0; i < count; i++) {
		const angle = (i / count) * Math.PI * 2 + rand(-0.1, 0.1);
		const v = speed * rand(0.6, 1.2);
		const color = PALETTE[pick(colors)];
		particles.push(createParticle(cx, cy, Math.cos(angle) * v, Math.sin(angle) * v, rand(30, 55), color, 0.025, 0.97));
	}
	return particles;
}

function burstYanagi(cx: number, cy: number, colors: PaletteKey[]): Particle[] {
	const particles: Particle[] = [];
	const count = randInt(50, 80);

	for (let i = 0; i < count; i++) {
		const angle = (i / count) * Math.PI * 2 + rand(-0.08, 0.08);
		const v = rand(0.5, 1.0);
		const color = PALETTE[pick(colors)];
		particles.push(createParticle(cx, cy, Math.cos(angle) * v, Math.sin(angle) * v * 0.7, rand(70, 120), color, 0.04, 0.99));
	}
	return particles;
}

function burstKamuro(cx: number, cy: number): Particle[] {
	const particles: Particle[] = [];
	const count = randInt(100, 150);

	for (let i = 0; i < count; i++) {
		const angle = (i / count) * Math.PI * 2 + rand(-0.05, 0.05);
		const v = rand(0.3, 0.9);
		const color = Math.random() > 0.3 ? PALETTE.gold : PALETTE.amber;
		particles.push(createParticle(cx, cy, Math.cos(angle) * v, Math.sin(angle) * v - 0.2, rand(80, 140), color, 0.05, 0.992));
	}
	return particles;
}

function burstSenrin(cx: number, cy: number, colors: PaletteKey[]): Particle[] {
	const particles: Particle[] = [];
	const numCenters = randInt(4, 8);

	for (let c = 0; c < numCenters; c++) {
		const ox = cx + rand(-8, 8);
		const oy = cy + rand(-6, 6);
		const count = randInt(15, 25);
		const speed = rand(0.3, 0.6);

		for (let i = 0; i < count; i++) {
			const angle = (i / count) * Math.PI * 2 + rand(-0.15, 0.15);
			const v = speed * rand(0.5, 1.2);
			const color = PALETTE[pick(colors)];
			particles.push(createParticle(ox, oy, Math.cos(angle) * v, Math.sin(angle) * v, rand(25, 50), color, 0.02, 0.975));
		}
	}
	return particles;
}

// ─── Launch Trail ────────────────────────────────────────────────────

interface LaunchTrail {
	x: number;
	y: number;
	targetY: number;
	vy: number;
	wobble: number;
	particles: Particle[];
	bursted: boolean;
	type: FireworkType;
	colors: PaletteKey[];
}

function createLaunch(
	x: number,
	startY: number,
	targetY: number,
	type: FireworkType,
	colors: PaletteKey[]
): LaunchTrail {
	return {
		x, y: startY, targetY,
		vy: rand(-1.2, -0.8),
		wobble: rand(0, Math.PI * 2),
		particles: [],
		bursted: false,
		type, colors,
	};
}

function updateLaunch(lt: LaunchTrail, dt: number): boolean {
	if (!lt.bursted) {
		lt.y += lt.vy * dt;
		lt.wobble += 0.15 * dt;
		lt.x += Math.sin(lt.wobble) * 0.05 * dt;

		// Spark trail
		if (Math.random() > 0.3) {
			lt.particles.push(
				createParticle(lt.x, lt.y, rand(-0.05, 0.05), rand(0.05, 0.15), rand(8, 18), PALETTE.amber, 0.01, 0.95)
			);
		}

		if (lt.y <= lt.targetY) {
			lt.bursted = true;
			return true; // signal: just arrived, needs burst
		}
	}

	// Update trail particles
	lt.particles = lt.particles.filter((p) => updateParticle(p, dt));

	// Keep alive while trail sparks remain
	return !lt.bursted || lt.particles.length > 0;
}

function writeLaunch(lt: LaunchTrail, grid: CharGrid) {
	if (!lt.bursted) {
		const col = Math.round(lt.x);
		const row = Math.round(lt.y);
		grid.set(col, row, '│', 220, 200, 150, 1);
		if (row > 0) {
			grid.set(col, row - 1, '╽', 255, 240, 180, 0.8);
		}
	}
	for (const p of lt.particles) {
		writeParticle(p, grid);
	}
}

// ─── Show Director ───────────────────────────────────────────────────

const SHOW_DURATION = 68;
const FPS_FACTOR = 60;

interface ShowState {
	time: number;
	schedule: ScheduledBurst[];
	scheduleIndex: number;
}

function buildSchedule(cols: number, rows: number): ScheduledBurst[] {
	const schedule: ScheduledBurst[] = [];
	const margin = Math.floor(cols * 0.1);
	const topZone = Math.floor(rows * 0.15);
	const midZone = Math.floor(rows * 0.4);

	function rx() { return randInt(margin, cols - margin); }
	function ry() { return randInt(topZone, midZone); }

	// Opening (0-8s): Single kiku bursts
	for (let t = 1; t < 8; t += rand(1.8, 2.5)) {
		schedule.push({ time: t, type: 'kiku', x: rx(), y: ry() });
	}

	// Act 1 (8-25s): Mixed types, 2-3 at a time
	for (let t = 8; t < 25; t += rand(1.2, 2.0)) {
		const type = pick<FireworkType>(['kiku', 'botan', 'yanagi', 'botan']);
		schedule.push({ time: t, type, x: rx(), y: ry() });
		if (Math.random() > 0.4) {
			schedule.push({ time: t + rand(0.1, 0.4), type: pick(['botan', 'kiku']), x: rx(), y: ry() });
		}
	}

	// Act 2 (25-40s): Faster pace, senrin & kamuro
	for (let t = 25; t < 40; t += rand(0.8, 1.5)) {
		const type = pick<FireworkType>(['senrin', 'kamuro', 'kiku', 'yanagi', 'botan']);
		schedule.push({ time: t, type, x: rx(), y: ry() });
		if (Math.random() > 0.3) {
			schedule.push({ time: t + rand(0.05, 0.3), type: pick(['senrin', 'botan']), x: rx(), y: ry() });
		}
	}

	// Starmine (40-50s): Rapid-fire across width
	for (let t = 40; t < 50; t += rand(0.3, 0.7)) {
		schedule.push({ time: t, type: 'starmine', x: rx(), y: ry() });
	}

	// Grand Finale (50-65s): Everything
	for (let t = 50; t < 65; t += rand(0.15, 0.4)) {
		const type = pick<FireworkType>(['kiku', 'botan', 'yanagi', 'kamuro', 'senrin', 'starmine']);
		schedule.push({ time: t, type, x: rx(), y: ry() });
	}

	schedule.sort((a, b) => a.time - b.time);
	return schedule;
}

// ─── FireworkEngine ──────────────────────────────────────────────────

export class FireworkEngine {
	private grid: CharGrid;
	private renderer: Renderer;
	private particles: Particle[] = [];
	private launches: LaunchTrail[] = [];
	private show: ShowState;
	private canvas: HTMLCanvasElement;
	private ctx: CanvasRenderingContext2D;
	private animId: number = 0;
	private running = false;
	private cellW: number;
	private cellH: number;
	private lastTime: number = 0;

	constructor(canvas: HTMLCanvasElement) {
		this.canvas = canvas;
		this.ctx = canvas.getContext('2d')!;
		this.cellW = 14;
		this.cellH = 18;

		const { cols, rows } = this.computeGridSize();
		this.grid = new CharGrid(cols, rows);
		this.renderer = new Renderer(this.ctx, this.cellW, this.cellH);
		this.show = {
			time: 0,
			schedule: buildSchedule(cols, rows),
			scheduleIndex: 0,
		};
	}

	private computeGridSize() {
		const dpr = window.devicePixelRatio || 1;
		const w = window.innerWidth;
		const h = window.innerHeight;
		this.canvas.width = w * dpr;
		this.canvas.height = h * dpr;
		this.canvas.style.width = w + 'px';
		this.canvas.style.height = h + 'px';
		this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

		return {
			cols: Math.floor(w / this.cellW),
			rows: Math.floor(h / this.cellH),
		};
	}

	resize() {
		const { cols, rows } = this.computeGridSize();
		this.grid = new CharGrid(cols, rows);
		this.renderer = new Renderer(this.ctx, this.cellW, this.cellH);
		this.show.schedule = buildSchedule(cols, rows);
	}

	start() {
		this.running = true;
		this.lastTime = performance.now();
		this.tick();
	}

	stop() {
		this.running = false;
		cancelAnimationFrame(this.animId);
	}

	private tick = () => {
		if (!this.running) return;

		const now = performance.now();
		const rawDt = (now - this.lastTime) / 1000;
		this.lastTime = now;
		const dtSec = Math.min(rawDt, 0.05);
		const dt = dtSec * FPS_FACTOR;

		// 1. Advance show time and spawn new launches
		this.show.time += dtSec;
		this.spawnFromSchedule();

		// 2. Update launches — burst on arrival
		for (const lt of this.launches) {
			const wasNotBursted = !lt.bursted;
			updateLaunch(lt, dt);
			if (wasNotBursted && lt.bursted) {
				this.spawnBurst(Math.round(lt.x), Math.round(lt.y), lt.type, lt.colors);
			}
		}
		this.launches = this.launches.filter((lt) => !lt.bursted || lt.particles.length > 0);

		// 3. Update particles
		this.particles = this.particles.filter((p) => updateParticle(p, dt));

		// 4. Clear grid and write
		this.grid.clear();
		for (const lt of this.launches) {
			writeLaunch(lt, this.grid);
		}
		for (const p of this.particles) {
			writeParticle(p, this.grid);
		}

		// 5. Render
		this.renderer.draw(this.grid, window.innerWidth, window.innerHeight);

		// 6. Loop show
		if (this.show.time > SHOW_DURATION + 3) {
			this.show.time = 0;
			this.show.scheduleIndex = 0;
			this.show.schedule = buildSchedule(this.grid.cols, this.grid.rows);
		}

		this.animId = requestAnimationFrame(this.tick);
	};

	private spawnFromSchedule() {
		const s = this.show;
		while (s.scheduleIndex < s.schedule.length && s.schedule[s.scheduleIndex].time <= s.time) {
			const burst = s.schedule[s.scheduleIndex];
			s.scheduleIndex++;

			const colors = pick(COLOR_GROUPS);
			if (burst.type === 'starmine') {
				this.spawnBurst(burst.x, burst.y, burst.type, colors);
			} else {
				this.launches.push(createLaunch(burst.x, this.grid.rows, burst.y, burst.type, colors));
			}
		}
	}

	private spawnBurst(cx: number, cy: number, type: FireworkType, colors: PaletteKey[]) {
		let newParticles: Particle[];

		switch (type) {
			case 'kiku':
				newParticles = burstKiku(cx, cy, colors);
				break;
			case 'botan':
			case 'starmine':
				newParticles = burstBotan(cx, cy, colors);
				break;
			case 'yanagi':
				newParticles = burstYanagi(cx, cy, colors);
				break;
			case 'kamuro':
				newParticles = burstKamuro(cx, cy);
				break;
			case 'senrin':
				newParticles = burstSenrin(cx, cy, colors);
				break;
		}

		this.particles.push(...newParticles);
	}

}
