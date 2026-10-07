# Mini particle engine

A small Rust particle core compiled into the existing fireworks WASM module.
There are no new runtime dependencies. JavaScript drives one batched `tick()`
call and uploads packed point and trail buffers; individual particles stay in WASM.

## Modules

- `src/particle.rs`: 3D bodies (position, velocity, acceleration, drag, lifetime),
  a fixed-step clock, and contiguous storage with an emission budget.
- `src/projection.rs`: a fixed perspective camera with near-plane clipping.
  The camera stays level and centered,
  with a long lens and a pulled-back view that leaves sky around the display.
- `src/trail.rs`: eight-sample, fixed-size motion history for each moving star.
- `src/lib.rs`: fireworks emitters, secondary effects, show scheduling, and render
  output. Custom effects can override ballistic motion without adding
  firework-specific concepts to the core.

Physics uses metres, metres per second, and seconds. Effect patterns remain
authored in screen-scale distances and per-tick speeds; emitters convert those
values to SI units, and the camera maps metres into viewport coordinates. `tick(seconds)`
accumulates elapsed time and runs up to three 1/60-second steps. Excess time
after a long pause is discarded. The main particle budget is 3,200; new emissions
are dropped when full. Launch shells and their trails are maintained separately.

The world origin `(0, 0, 0)` is the launch point. X runs across the display, Y is
altitude (positive up), and Z points away from the viewer. The show schedule starts
each shell at the origin and converts its screen-space target back to world metres.
Bursts expand in depth, rings tilt, and child sparks/glitter inherit their parent
depth. A perspective camera projects the XYZ simulation for the Glow renderer.
Portrait screens spread launch targets farther vertically; landscape screens keep
the broader horizontal staging.

The Glow renderer reads contiguous `f32` buffers with eight values per point:
`[x, y, radius, red, green, blue, alpha, kind]`. X/Y are unrounded projected
simulation coordinates; radius is in CSS pixels, RGB in 0–255, alpha in 0–1,
and kind is 0 (star), 1 (smoke), or 2 (detonation flash). `points_len()` counts
floats, not bytes. Recreate output views after `tick()` or `resize()` because WASM
memory can move.

A second `f32` buffer exposes motion trails through `trails_ptr()` and
`trails_len()`. Each segment has ten values:
`[x1, y1, x2, y2, width, red, green, blue, alpha, reserved]`.
The reserved value is currently zero. Up to 12,800 segments are emitted each frame;
history sampling runs at the simulation rate, independent of the display rate.

`src/lib/renderers/glow.ts` uses WebGL 2 instancing to draw trajectory segments,
smoke, and glowing particles in two batches. The full-frame particle field is
composited over a beach-at-night scene with a low ocean horizon, perspective
wave glints, a subtle shoreline, and rippled reflections sampled from the
fireworks buffer. The WASM buffers upload directly into
reusable GPU buffers; no per-star Canvas paths or canvas self-copies are needed.
Glow rendering is capped at 1.35× device scale and 1.8 million pixels
to bound GPU work on Retina/4K displays. The page owns viewport sizing and the
render loop. WebGL 2 is required for Glow rendering; if it is unavailable, the
page reports that the renderer could not start. Hidden tabs pause animation.

## Fireworks behavior

Launches decelerate under gravity and burst near their apex. Chrysanthemums use
dense outer shells and a smaller white core; peonies burn more briefly; willows
and brocades hold long, falling gold trails. Palm bursts fan into separated,
hanging branches, while crowns layer two expanding rings. Rings can tilt in
depth. Stars hold their color before cooling and flickering out.

Star trajectories use gravity and Reynolds-dependent quadratic sphere drag,
with wind-relative velocity, a representative 1 g / 10 mm star, and a fixed
2 m/s crosswind plus 0.8 m/s depth wind. Launch shells use the same drag model
with a representative 0.5 kg / 100 mm shell; launch speed is solved so drag still
allows the shell to reach its authored burst height. These sizes and wind values
are tunable stand-ins, not universal firework measurements. Burst patterns,
light, smoke, color, and combustion timing remain designed visual effects rather
than a chemical or explosive-energy simulation.
The representative star size and drag fit follow measurements reported by
[Ooki et al. (2006)](https://www.jes.or.jp/mag/stem/Vol.67/documents/Vol.67%2CNo.1%2Cp.43-47.pdf);
the Reynolds-dependent shell-drag approach follows
[Ding et al. (2007)](https://www.jes.or.jp/mag/stem/Vol.68/No.1.03.html).

The 71-second loop opens with a centered shell, builds through mixed effects,
and closes with coordinated volleys and a five-shell gold curtain.

## Browser API

```ts
const engine = new FireworkEngine(cols, rows); // always simulates in 3D
engine.tick(elapsedSeconds);
// Recreate views after tick/resize because WASM memory can move.
const points = new Float32Array(
  exports.memory.buffer,
  engine.points_ptr(),
  engine.points_len(),
);
engine.resize(newCols, newRows);
engine.free(); // release when unmounting
```

The simulation always runs in 3D. The site uses the Glow particle renderer. A
movable camera, collision detection, and a scene graph are outside this core's
scope.

## Validation

From the repository root:

```sh
cargo test --manifest-path crates/fireworks-wasm/Cargo.toml
node --test tests/glow.test.mjs
pnpm run wasm:build
pnpm check
pnpm build:production
```

Use pnpm 10 with the current lockfile. Native tests use a deterministic RNG and
cover physics, timestep behavior, capacity, projection, all burst types, resizing,
and complete looping 3D shows. Browser verification should cover
renderer switching, resizing, and WASM initialization/cleanup.

The renderer tests enforce draw-call and allocation budgets with a mocked GPU;
they do not measure browser FPS or validate shader output. In a full-show WASM
benchmark the simulation remained below 0.31 ms at the 95th percentile on the
development machine. Actual frame rate also depends on the browser and GPU.
