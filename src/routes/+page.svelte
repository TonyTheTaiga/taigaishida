<script lang="ts">
  import { onMount } from "svelte";
  import { GlowRenderer } from "$lib/renderers/glow";

  // Phones get the portrait-first programme on a smaller particle budget;
  // tablets and computers get the panoramic show. `?show=mobile` or
  // `?show=desktop` overrides detection for previews.
  function prefersMobileShow() {
    const requested = new URLSearchParams(window.location.search).get("show");
    if (requested === "mobile" || requested === "desktop") {
      return requested === "mobile";
    }
    const touch = window.matchMedia("(pointer: coarse)").matches;
    const phoneSized =
      Math.min(window.screen.width, window.screen.height) < 768;
    return touch && phoneSized;
  }

  // `?seed=123` replays a show exactly; without it every visit differs.
  function requestedSeed() {
    const seed = new URLSearchParams(window.location.search).get("seed");
    return seed !== null && /^\d{1,10}$/.test(seed)
      ? Number(seed) >>> 0
      : undefined;
  }

  let glowCanvas: HTMLCanvasElement;
  let rendererNote = $state("");
  let error = $state("");
  let counts = $state({ total: 0, stars: 0, sparks: 0, smoke: 0 });
  const formatCount = (n: number) => n.toLocaleString("en-US");

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

      const initial = viewport();
      const engine = new FireworkEngine(
        initial.width,
        initial.height,
        prefersMobileShow(),
        requestedSeed(),
      );
      console.info(
        `Fireworks seed ${engine.seed()}; add ?seed=${engine.seed()} to replay this show.`,
      );

      const onResize = () => {
        const { width, height } = viewport();
        glow?.resize(width, height);
        engine.resize(width, height);
      };
      const resizeObserver = new ResizeObserver(onResize);
      resizeObserver.observe(glowCanvas);
      window.addEventListener("resize", onResize);

      let lastTime = performance.now();
      let lastCount = 0;
      let animId = 0;
      let running = true;

      function tick() {
        if (!running) return;

        const now = performance.now();
        const dtSec = (now - lastTime) / 1000;
        lastTime = now;

        engine.tick(dtSec);

        // A few updates a second keep the counter readable and cheap.
        if (now - lastCount > 250) {
          lastCount = now;
          const stars = engine.star_count();
          const sparks = engine.spark_count();
          const smoke = engine.smoke_count();
          counts = { total: stars + sparks + smoke, stars, sparks, smoke };
        }

        // Re-read after tick: WASM memory or any output buffer can move.
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
        const mesh = new Float32Array(
          wasmExports.memory.buffer,
          engine.mesh_ptr(),
          engine.mesh_len(),
        );
        const size = viewport();
        glow?.draw(
          {
            points,
            trails,
            mesh,
            horizon: engine.horizon(),
            waterline: engine.waterline(),
          },
          size.width,
          size.height,
        );

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
  <div
    class="pointer-events-none absolute top-4 right-4 text-right font-mono text-xs text-white/55 tabular-nums"
    aria-hidden="true"
  >
    <div class="text-sm text-white/80">
      {formatCount(counts.total)} particles
    </div>
    <div>
      {formatCount(counts.stars)} stars · {formatCount(counts.sparks)} sparks ·
      {formatCount(counts.smoke)} smoke
    </div>
  </div>
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
