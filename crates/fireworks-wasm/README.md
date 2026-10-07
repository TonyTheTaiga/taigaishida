# Mini particle engine

A small Rust particle core compiled into the existing fireworks WASM module.
There are no new runtime dependencies. JavaScript drives one batched `tick()`
call and uploads packed point and trail buffers; individual particles stay in WASM.

## Modules

The engine models fireworks from the composition up. Nothing above the
chemistry layer sets a colour curve, lifetime, trajectory, or burst height
directly; those follow from measured inputs (see [Calibration](#calibration)).

- `src/chemistry.rs`: the smallest unit. A `Composition` has a density, an
  in-flight burn rate, a light source (emitter bands such as SrCl red, BaCl
  green, CuCl blue, Na yellow, or incandescence at a temperature), optional
  `SparkFuel`, and an optional `Effect` (strobe, thrust, split charge). A star
  recipe is a list of `Layer`s, from the outside in. Colours are computed from
  spectra with the CIE 1931 colour-matching functions, partially adapted the
  way a dark-adapted audience sees (CIECAM02), and encoded as sRGB.
  Incandescent luminance is the Planck spectrum weighted by the eye's ȳ(λ).
- `src/star.rs`: `Star` is a layered sphere whose flame front regresses inward.
  Mass, diameter, drag, light (∝ burning area), spark output (∝ mass burned),
  and thrust (mass flow × exhaust speed) change continuously. Crossing a layer
  boundary changes colour or fires that layer's effect; a split charge breaks
  the remaining core into equal fragments, conserving mass and momentum.
  `Spark` is an incandescent particle: it smoulders, burns, and then cools by
  conduction (air conductivity at the film temperature) and grey-body
  radiation, with its own heat capacity. Its colour and brightness come only
  from its temperature. `Puff` covers burst flashes and drifting smoke.
- `src/shell.rs`: `ShellDesign` combines a casing, a lift charge, a bursting
  charge with its heat of explosion, an optional rising comet, and a payload
  of stars or sub-shells packed at fractional radii, as spheres or rings. The
  lift sets the muzzle speed (½·M·v² = η·m·Q) and drag sets the burst height;
  the burst shares η·m·Q among the petals, each moving in proportion to its
  packing radius. The time fuse glows on the way up and is consumed by the
  burst.
- `src/designs.rs`: the ten shells, described below.
- `src/particle.rs`: 3D bodies with gravity, wind, and Reynolds-dependent drag
  in air that thins with altitude. Drag is integrated implicitly so 0.2 mm
  sparks stay stable. Also holds the fixed-step clock and bounded storage.
- `src/projection.rs`: a perspective camera with a ~45° field of view. Metres
  are square on screen, and framing fits the show's stage (540 m × 360 m on
  desktop, 220 m × 360 m on phones) to any viewport, which puts the viewer
  0.5–1.8 km from the barge, where spectators stand.
- `src/show.rs`: the two show programmes, described below. Cues name a burst
  time, a mortar position, and a tilt; launch times are worked back from each
  shell's predicted flight, and the time fuse is cut to its apex.
- `src/trail.rs`: eight-sample motion history (persistence of vision) per star.
- `src/lib.rs`: the exported engine, the simulation step, and render output.

Physics uses metres, kilograms, kelvin, and seconds. `tick(seconds)` runs up to
three 1/60 s steps. Each simulated spark stands for many real ones. Once half
the spark budget is in use, spark sampling thins evenly instead of starving
whichever stars update last.

## The ten shells

Sizes are Japanese gō: 4-gō is 114 mm, 5-gō 142 mm, 6-gō 171 mm, 7-gō 199 mm.

1. **Yae-zaki chrysanthemum** (7-gō): 220 outer 18 mm stars leave a charcoal
   tail, then burn red → green → blue. A green-into-violet middle petal and a
   white-into-red pistil open at smaller radii.
2. **Kamuro golden crown** (7-gō): a weakened burst and 19 mm charcoal stars
   that burn for about 9.5 s; their fire dust hangs in a drooping gold crown.
3. **Crossette** (4-gō): sixteen 25 mm titanium comets with an internal split
   charge. Each core breaks into four smaller comets thrown sideways.
4. **Saturn** (6-gō): a violet-to-blue planet inside a tilted ring of gold
   glitter from the same shell.
5. **Senrin thousand flowers** (6-gō): forty-five 30 mm small shells, each on
   its own fuse, blooming into small red, green, or glitter flowers.
6. **Swimming koi** (5-gō poka): ninety pierced stars propelled by their own
   exhaust, with a tumbling nozzle direction, around a green peony.
7. **Dragon eggs** (5-gō): gold tails dissolve into bismuth microstars that
   smoulder dark, then pop.
8. **Twinkling crown** (6-gō): silver titanium tails, then Shimizu's green
   twinkler flashing at 3.1 Hz for about six seconds.
9. **Brocade palm** (5-gō poka): a rising brocade comet and ten heavy 25 mm
   comets with long range and a slow droop.
10. **Ghost shell** (7-gō): dark delay layers between violet, green, and
    orange. The sphere vanishes and reappears in a new colour, twice.

Star mines use smaller shells: 3-gō peonies (86 mm, Shimizu's standard 150 ×
9 mm stars) in red, green, blue, silver, violet, yellow, and red-to-green, and
a 4-gō gold chrysanthemum of all-charcoal stars.

## The two shows

The page picks a show when the engine is created: phones (a coarse pointer and
a screen under 768 px on its short side) get the mobile show, and tablets and
computers get the desktop show. Add `?show=mobile` or `?show=desktop` to the
URL to preview either.

|            | Desktop                                     | Mobile                                |
| ---------- | ------------------------------------------- | ------------------------------------- |
| Stage      | 540 m × 360 m, mortars across a 450 m barge | 220 m × 360 m, one centred column     |
| Length     | 158 s loop, ~175 shells                     | 87 s loop, ~53 shells                 |
| Budget     | 6,000 stars, 26,000 sparks, 800 puffs       | 3,000 stars, 12,000 sparks, 400 puffs |
| Frame cost | 0.18 ms median, 2.7 ms p95 (Node)           | 0.10 ms median, 1.4 ms p95 (Node)     |

The desktop show runs in six acts:

1. **Opening**: a five-shell silver salute, red and blue fans from the wings,
   three Yae-zaki across the barge, a gold sweep, and twin crossettes.
2. **Tanpatsu**: the signature shells one at a time, each given room to
   finish, ending on a lone kamuro.
3. **Star mine**: peony sweeps in both directions, a five-crossette fan, colour
   call and response from the wings, a gold salvo, and a rapid mine.
4. **Garden**: layered heights, including a five-tier ladder in which 3- to
   7-gō shells fired from one spot burst together at 125–250 m.
5. **Crescendo**: three ghosts, twin twinkling crowns over a silver fan, a
   five-shell Yae-zaki sweep, crossette fans, and an accelerating mine.
6. **Finale**: full-width salvos, then a seven-shell kamuro curtain that hangs
   until the loop restarts.

The mobile show stacks by height instead of spreading by width: centred solos
with small shells layered under long-lived ones, a three-tier ladder, narrow
star-mine sweeps, paired Yae-zaki, and a three-shell kamuro that fills the
portrait frame. Each shell follows the last as it fades.

## Output buffers

The Glow renderer reads `f32` points with eight values each:
`[x, y, radius, red, green, blue, alpha, kind]`, where kind is 0 (light),
1 (smoke), or 2 (detonation flash). Trail segments have ten values each:
`[x1, y1, x2, y2, width, red, green, blue, alpha, reserved]`. Stars draw four
history segments. Each spark draws one streak covering 80 ms of motion, and
very bright sparks (glitter, microstar pops) also draw a point. Lengths count
floats, not bytes. Recreate output views after `tick()` or `resize()`, because
WASM memory can move.

## Calibration

| Quantity                             | Model                                                                                   | Source                                                                                                                                                                                                  |
| ------------------------------------ | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Shell diameters by size              | 114–199 mm                                                                              | Japan Fireworks Association table ([MLIT](https://www.mlit.go.jp/kaiji/kikenbutu/26-3-3-1-1.pdf))                                                                                                       |
| Empty casing masses                  | 0.141–0.477 kg                                                                          | [Ooki et al. 2006b](https://www.jes.or.jp/mag/stem/Vol.67/documents/Vol.67,No.1,p.39-42.pdf), Table 1                                                                                                   |
| Main stars, counts, bursting charges | 13–18 mm, 180–220, 135–550 g                                                            | Shimizu, _Fireworks: The Art, Science and Technique_, Table 23; consistent with [Wakabayashi et al. 2006](https://www.jes.or.jp/mag/stem/Vol.67/documents/Vol.67,No.3,p.91-95.pdf)                      |
| Lift charges                         | 41.5–120 g                                                                              | Shimizu Table 27                                                                                                                                                                                        |
| Lift efficiency                      | η = 0.088                                                                               | one calibration: a 7-gō reaches the Association's 250 m                                                                                                                                                 |
| Burst heights                        | emergent: 195–251 m warimono                                                            | tested against the Association's 165–250 m within 12%                                                                                                                                                   |
| Muzzle speeds, climb times           | emergent: 114–149 m/s, 4–6 s                                                            | Kosanke: about 112 m/s ("250 mph") and 3–6 s fuses ([Fire Engineering](https://www.fireengineering.com/firefighting/fireworks-and-their-hazards/))                                                      |
| Heats of explosion                   | KP 690, black powder 400 kcal/kg                                                        | Shimizu Table 19                                                                                                                                                                                        |
| Burst efficiency                     | η = 0.0014                                                                              | Shimizu Table 19: 63.5 m/s stars from a 6-inch shell                                                                                                                                                    |
| Star speeds, flower diameters        | emergent: 50–66 m/s, 130–180 m (kamuro crown 230 m)                                     | Shimizu Table 19 (57–71 m/s) and Table 2; Association table                                                                                                                                             |
| Star density, burn rates, Cd         | 1550 kg/m³, 1.9–3.75 mm/s, 0.5                                                          | [Ooki et al. 2006a](https://www.jes.or.jp/mag/stem/Vol.67/documents/Vol.67,No.1,p.43-47.pdf) (fired stars burn 1.6× slower than on a plate)                                                             |
| Kamuro and palm star lives           | 9.5 s, 7.8 s                                                                            | Shimizu §18.3 (8–10 s) and §2 (slow stars 7–8 s)                                                                                                                                                        |
| Twinkler                             | 3.1 Hz, 1.31 g/cm³, 1.7 mm/s on a plate                                                 | Shimizu Table 22                                                                                                                                                                                        |
| Spark temperatures                   | charcoal 1780 K, titanium 2400 K                                                        | Shimizu §10.2 ("dark red to bright orange"), [Sandia 2023](https://www.sandia.gov/research/publications/details/evolution-of-titanium-particle-combustion-in-potassium-perchlorate-and-air-2023-07-01/) |
| Flame temperatures                   | colour 2200 °C, magnesium 2500–3500 °C                                                  | Shimizu §9                                                                                                                                                                                              |
| Emitter bands                        | SrCl/SrOH 606–682 nm, BaCl 514/524 nm, CuCl 428–488 nm, Na 589 nm, CaCl/CaOH 554–622 nm | flame and plasma spectroscopy; Shimizu §9.4                                                                                                                                                             |
| Colour vision                        | CIE 1931 2° fit, adaptation D ≈ 0.66 toward 2523 K                                      | [Wyman et al. 2013](https://jcgt.org/published/0002/02/01/); CIECAM02 dark surround; Shimizu §8.2 (silver-white at 2250 °C)                                                                             |
| Air                                  | 1.204 kg/m³, 8.4 km scale height, μ = 1.81e-5 Pa·s                                      | standard atmosphere                                                                                                                                                                                     |

Estimates, chosen within plausible ranges because no measurement was found:
green, yellow, and orange burn rates; glitter and microstar timings and flash
temperatures; the twinkler's flash fraction; fish-star exhaust speed; the
crossette split efficiency; the crossette, palm, and small-flower charges;
relative luminous intensities; and the number of sparks sampled per gram.
Light shells use a lift of 6% of their mass, matching the 5–8% in Shimizu
Table 27. The wind is a constant 2.2 m/s with no shear.

## Browser API

```ts
const engine = new FireworkEngine(cols, rows, mobile); // always simulates in 3D
engine.mobile(); // which programme is playing
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

Use pnpm 10 with the current lockfile. Native tests use a deterministic RNG.
They check that every stored colour matches its spectrum, layered mass and
burn-through, crossette mass and momentum conservation, spark cooling, strobe
and thrust, each shell's burst height, muzzle speed, climb time, star speed,
and flower diameter against the published tables above, the viewer distance,
that every cue bursts on time and inside the frame on its venue's viewports,
that the mobile show stays in one column without crowding, and that both shows
loop within budget. Browser verification should cover both shows, resizing and
rotation, and WASM initialization/cleanup.

The renderer tests enforce draw-call and allocation budgets with a mocked GPU;
they do not measure browser FPS or validate shader output. In full-show WASM
benchmarks under Node, a desktop frame took 0.18 ms at the median, 2.7 ms at
the 95th percentile, and at most 3.9 ms on the development machine; a mobile
frame took 0.10, 1.4, and 1.7 ms. Actual frame rate also depends on the
browser and GPU.
