<script lang="ts">
  import { onMount } from "svelte";
  import { GlowRenderer } from "$lib/renderers/glow";

  const CELL_W = 14;
  const CELL_H = 18;

  let glowCanvas: HTMLCanvasElement;
  let rendererNote = $state("");
  let error = $state("");
  let diagnosticsOpen = $state(true);
  let diagnostics = $state({
    fps: 0,
    frameMs: 0,
    simulationMs: 0,
    submitMs: 0,
    points: 0,
    trailSegments: 0,
    width: 0,
    height: 0,
  });

  onMount(() => {
    let disposed = false;
    let cleanup = () => {};

    async function initialize() {
      const wasm = await import("$lib/fireworks-wasm/fireworks_wasm.js");
      const wasmExports = await wasm.default();
      if (disposed) return;

      let glow: GlowRenderer | undefined;
      const viewport = () => ({
        width: Math.max(1, glowCanvas.clientWidth),
        height: Math.max(1, glowCanvas.clientHeight),
      });
      function createGlow() {
        try {
          glow?.dispose();
          glow = new GlowRenderer(glowCanvas);
          const size = viewport();
          glow.resize(size.width, size.height);
          rendererNote = "";
        } catch (cause) {
          glow?.dispose();
          glow = undefined;
          rendererNote = `Glow particles are unavailable: ${cause instanceof Error ? cause.message : String(cause)}`;
          console.warn("Glow renderer unavailable", cause);
        }
      }
      createGlow();
      const { FireworkEngine } = wasm;

      function computeGrid() {
        const { width: w, height: h } = viewport();
        glow?.resize(w, h);
        return {
          cols: Math.floor(w / CELL_W),
          rows: Math.floor(h / CELL_H),
        };
      }

      let { cols, rows } = computeGrid();
      const engine = new FireworkEngine(cols, rows);

      const onResize = () => {
        ({ cols, rows } = computeGrid());
        engine.resize(cols, rows);
      };
      const resizeObserver = new ResizeObserver(onResize);
      resizeObserver.observe(glowCanvas);
      window.addEventListener("resize", onResize);

      let lastTime = performance.now();
      let statsWindowStart = lastTime;
      let statsFrames = 0;
      let statsSimulationMs = 0;
      let statsSubmitMs = 0;
      let animId = 0;
      let running = true;

      function tick() {
        if (!running) return;

        const now = performance.now();
        const frameMs = now - lastTime;
        const dtSec = frameMs / 1000;
        lastTime = now;

        const simulationStart = performance.now();
        engine.tick(dtSec);
        const simulationMs = performance.now() - simulationStart;
        const submitStart = performance.now();
        const pointCount = engine.points_len() / 8;
        const trailSegmentCount = engine.trails_len() / 10;

        // Re-read after tick: WASM memory or either output buffer can move.
        const points = new Float32Array(
          wasmExports.memory.buffer,
          engine.points_ptr(),
          engine.points_len(),
        );
        const trails = new Float32Array(
          wasmExports.memory.buffer,
          engine.trails_ptr(),
          engine.trails_len(),
        );
        const size = viewport();
        glow?.draw(points, trails, size.width, size.height);
        const submitMs = performance.now() - submitStart;
        statsFrames += 1;
        statsSimulationMs += simulationMs;
        statsSubmitMs += submitMs;
        if (now - statsWindowStart >= 250) {
          const elapsed = now - statsWindowStart;
          diagnostics = {
            fps: (statsFrames * 1000) / elapsed,
            frameMs: elapsed / statsFrames,
            simulationMs: statsSimulationMs / statsFrames,
            submitMs: statsSubmitMs / statsFrames,
            points: pointCount,
            trailSegments: trailSegmentCount,
            width: glowCanvas.width,
            height: glowCanvas.height,
          };
          statsWindowStart = now;
          statsFrames = 0;
          statsSimulationMs = 0;
          statsSubmitMs = 0;
        }

        animId = requestAnimationFrame(tick);
      }

      const onContextLost = (event: Event) => {
        event.preventDefault();
        rendererNote = "Glow particles paused while the GPU renderer recovers.";
      };
      const onContextRestored = () => {
        createGlow();
      };
      const onVisibility = () => {
        cancelAnimationFrame(animId);
        lastTime = performance.now();
        statsWindowStart = lastTime;
        statsFrames = 0;
        statsSimulationMs = 0;
        statsSubmitMs = 0;
        if (!document.hidden) animId = requestAnimationFrame(tick);
      };
      glowCanvas.addEventListener("webglcontextlost", onContextLost);
      glowCanvas.addEventListener("webglcontextrestored", onContextRestored);
      document.addEventListener("visibilitychange", onVisibility);
      if (!document.hidden) animId = requestAnimationFrame(tick);

      cleanup = () => {
        running = false;
        cancelAnimationFrame(animId);
        window.removeEventListener("resize", onResize);
        resizeObserver.disconnect();
        engine.free();
        glow?.dispose();
        glowCanvas.removeEventListener("webglcontextlost", onContextLost);
        glowCanvas.removeEventListener(
          "webglcontextrestored",
          onContextRestored,
        );
        document.removeEventListener("visibilitychange", onVisibility);
      };
    }

    void initialize().catch((cause: unknown) => {
      if (!disposed) {
        error = "The fireworks could not load. Please refresh to try again.";
        console.error("Fireworks initialization failed", cause);
      }
    });
    return () => {
      disposed = true;
      cleanup();
    };
  });
</script>

<svelte:head>
  <title>Taiga Ishida</title>
</svelte:head>

<div class="viewport-shell fixed inset-0 overflow-hidden bg-black">
  <canvas bind:this={glowCanvas} class="block h-full w-full"></canvas>
  <aside
    class="absolute z-40 font-mono"
    style="bottom: max(1.25rem, env(safe-area-inset-bottom)); left: max(1.25rem, env(safe-area-inset-left));"
  >
    {#if diagnosticsOpen}
      <section
        aria-label="Live rendering diagnostics"
        class="max-h-[calc(100dvh-2.5rem)] w-[min(16rem,calc(100vw-2.5rem))] overflow-y-auto rounded-md border border-white/15 bg-black/75 p-3 text-[10px] text-white/75 shadow-xl backdrop-blur-md"
      >
        <header
          class="mb-3 flex items-center justify-between border-b border-white/10 pb-2 text-[9px] tracking-[0.2em] text-white/50"
        >
          <span>LIVE DIAGNOSTICS</span>
          <button
            class="px-1 text-sm leading-none text-white/55 hover:text-white"
            aria-label="Hide diagnostics"
            onclick={() => (diagnosticsOpen = false)}>×</button
          >
        </header>
        <div class="grid grid-cols-2 gap-x-4 gap-y-2">
          <div>
            <div class="text-white/40">FPS</div>
            <div class="text-sm tabular-nums text-white">
              {diagnostics.fps.toFixed(0)}
            </div>
          </div>
          <div>
            <div class="text-white/40">FRAME</div>
            <div class="tabular-nums text-white">
              {diagnostics.frameMs.toFixed(1)} ms
            </div>
          </div>
          <div>
            <div class="text-white/40">WASM SIM</div>
            <div class="tabular-nums text-white">
              {diagnostics.simulationMs.toFixed(2)} ms
            </div>
          </div>
          <div>
            <div class="text-white/40">RENDER CPU</div>
            <div
              class="tabular-nums text-white"
              title="Main-thread time creating particle views and submitting renderer commands; GPU execution is not included"
            >
              {diagnostics.submitMs.toFixed(2)} ms
            </div>
          </div>
          <div>
            <div class="text-white/40">POINTS</div>
            <div class="tabular-nums text-white">
              {diagnostics.points.toLocaleString()}
            </div>
          </div>
          <div>
            <div class="text-white/40">TRAIL SEGMENTS</div>
            <div class="tabular-nums text-white">
              {diagnostics.trailSegments.toLocaleString()}
            </div>
          </div>
          <div class="col-span-2 border-t border-white/10 pt-2 text-white/40">
            BUFFER {diagnostics.width} × {diagnostics.height}
          </div>
        </div>
      </section>
    {:else}
      <button
        class="rounded border border-white/15 bg-black/75 px-3 py-2 text-[10px] tracking-wider text-white/70 backdrop-blur-md hover:text-white"
        aria-label="Show diagnostics"
        onclick={() => (diagnosticsOpen = true)}
      >
        FPS {diagnostics.fps.toFixed(0)} · STATS
      </button>
    {/if}
  </aside>
  {#if rendererNote}
    <p
      role="status"
      class="absolute inset-x-6 top-20 text-center text-sm text-white/70"
    >
      {rendererNote}
    </p>
  {/if}
  {#if error}
    <p
      role="alert"
      class="absolute inset-x-6 top-20 text-center text-sm text-white"
    >
      {error}
    </p>
  {/if}
</div>

<style>
  .viewport-shell {
    height: 100vh;
    height: 100dvh;
  }

  :global(body) {
    overflow: hidden;
    margin: 0;
  }
</style>
