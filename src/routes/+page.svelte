<script lang="ts">
	import { onMount } from 'svelte';

	const CHARS = [' ', '.', '\u00B7', '+', '*', '\u2726', '\u2502', '\u257D', 'o', '~', "'", 'x', '%'];
	const CELL_W = 14;
	const CELL_H = 18;
	const CELL_SIZE = 5;

	let canvas: HTMLCanvasElement;

	onMount(async () => {
		const wasm = await import('$lib/fireworks-wasm/fireworks_wasm.js');
		const wasmExports = await wasm.default();

		const ctx = canvas.getContext('2d')!;
		const { FireworkEngine } = wasm;

		function computeGrid() {
			const dpr = window.devicePixelRatio || 1;
			const w = window.innerWidth;
			const h = window.innerHeight;
			canvas.width = w * dpr;
			canvas.height = h * dpr;
			canvas.style.width = w + 'px';
			canvas.style.height = h + 'px';
			ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
			return {
				cols: Math.floor(w / CELL_W),
				rows: Math.floor(h / CELL_H),
			};
		}

		let { cols, rows } = computeGrid();
		let engine = new FireworkEngine(cols, rows);

		const onResize = () => {
			({ cols, rows } = computeGrid());
			engine.resize(cols, rows);
		};
		window.addEventListener('resize', onResize);

		let lastTime = performance.now();
		let animId = 0;
		let running = true;

		function tick() {
			if (!running) return;

			const now = performance.now();
			const dtSec = (now - lastTime) / 1000;
			lastTime = now;

			engine.tick(dtSec);

			// Read grid buffer directly from WASM linear memory (zero-copy)
			const ptr = engine.grid_ptr();
			const len = engine.grid_len();
			const buf = new Uint8Array(wasmExports.memory.buffer, ptr, len);

			const curCols = engine.cols();
			const curRows = engine.rows();

			// Render
			ctx.fillStyle = '#080810';
			ctx.fillRect(0, 0, window.innerWidth, window.innerHeight);
			ctx.font = `${CELL_H}px monospace`;
			ctx.textBaseline = 'top';

			for (let r = 0; r < curRows; r++) {
				for (let c = 0; c < curCols; c++) {
					const idx = (r * curCols + c) * CELL_SIZE;
					const charIdx = buf[idx];
					const alpha = buf[idx + 4];
					if (charIdx === 0 || alpha === 0) continue;

					const cr = buf[idx + 1];
					const cg = buf[idx + 2];
					const cb = buf[idx + 3];
					const a = Math.min(1, alpha / 255);

					ctx.fillStyle = `rgba(${cr},${cg},${cb},${a})`;
					ctx.fillText(CHARS[charIdx], c * CELL_W, r * CELL_H);
				}
			}

			// Water reflection: mirror bottom ~15% of grid
			const reflectionRows = Math.floor(curRows * 0.15);
			const reflectionStart = curRows - reflectionRows;
			for (let rr = 0; rr < reflectionRows; rr++) {
				const sourceRow = reflectionStart - 1 - rr;
				if (sourceRow < 0) continue;
				const destRow = reflectionStart + rr;
				for (let cc = 0; cc < curCols; cc++) {
					const srcIdx = (sourceRow * curCols + cc) * CELL_SIZE;
					const charIdx = buf[srcIdx];
					const alpha = buf[srcIdx + 4];
					if (charIdx === 0 || alpha === 0) continue;

					const cr = buf[srcIdx + 1];
					const cg = buf[srcIdx + 2];
					const cb = buf[srcIdx + 3];
					const a = Math.min(1, (alpha / 255) * 0.35);

					// Blue-shift the reflected color
					const br = Math.floor(cr * 0.5);
					const bg = Math.floor(cg * 0.6);
					const bb = Math.min(255, Math.floor(cb * 1.2 + 40));

					ctx.fillStyle = `rgba(${br},${bg},${bb},${a})`;
					ctx.fillText(CHARS[charIdx] || '.', cc * CELL_W, destRow * CELL_H);
				}
			}

			animId = requestAnimationFrame(tick);
		}

		animId = requestAnimationFrame(tick);

		return () => {
			running = false;
			cancelAnimationFrame(animId);
			window.removeEventListener('resize', onResize);
			engine.free();
		};
	});
</script>

<svelte:head>
	<title>Taiga Ishida</title>
</svelte:head>

<div class="fixed inset-0 bg-black overflow-hidden">
	<canvas bind:this={canvas} class="block w-full h-full"></canvas>
</div>

<style>
	:global(body) {
		overflow: hidden;
		margin: 0;
	}
</style>
