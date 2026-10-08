# Mini particle engine

A small Rust particle core compiled into the existing fireworks WASM module.
There are no new runtime dependencies. JavaScript drives one batched `tick()`
call and uploads packed point, trail, and mesh buffers; individual particles
stay in WASM.

## Modules

The engine models fireworks from the composition up. Nothing above the
chemistry layer sets a colour curve, lifetime, trajectory, or burst height
directly; those follow from measured inputs (see [Calibration](#calibration)).

- `src/chemistry.rs`: the smallest unit. A `Composition` has a density, an
  in-flight burn rate, a light source (emitter bands such as SrCl red, BaCl
  green, CuCl blue, Na yellow, or incandescence at a temperature), optional
  `SparkFuel`, and an optional `Effect` (strobe, thrust, split charge, the
  tumbling flutter of a falling leaf, or a gerb's fixed upward jet). A star
  recipe is a list of `Layer`s, from the outside in. Colours are computed from
  spectra with the CIE 1931 colour-matching functions, partially adapted the
  way a dark-adapted audience sees (CIECAM02), and encoded as sRGB. Aqua,
  lemon, and pink come from mixed emitters (BaCl with a tenth of CuCl; Na with
  a quarter of BaCl; SrCl with traces of CuCl and Na). Incandescent luminance
  is the Planck spectrum weighted by the eye's ȳ(λ).
- `src/star.rs`: `Star` is a layered sphere whose flame front regresses inward.
  Mass, diameter, drag, light (∝ burning area), spark output (∝ mass burned),
  and thrust (mass flow × exhaust speed) change continuously. Crossing a layer
  boundary changes colour or fires that layer's effect; a split charge breaks
  the remaining core into equal fragments, conserving mass and momentum.
  Hand-made stars differ: each varies in burn rate (about 6%) and brightness
  (about 10%), flutters by a few percent, and lights up to 0.1 s after the
  burst. `Spark` is an incandescent particle: it smoulders, burns, and then
  cools by conduction (air conductivity at the film temperature) and grey-body
  radiation, with its own heat capacity. Its colour and brightness come only
  from its temperature. `Puff` covers burst and muzzle flashes and smoke that
  spreads as √t and lingers for about 20 s, long enough for later bursts to
  light it.
- `src/shell.rs`: `ShellDesign` combines a casing, a lift charge, a bursting
  charge with its heat of explosion, an optional rising comet, and a payload
  of stars or sub-shells packed at fractional radii. Payloads open as spheres,
  rings, sectors and slices (lit in turn by dark delays), flat picture
  templates, or a half-dome on the water. The lift sets the muzzle speed
  (½·M·v² = η·m·Q) and drag sets the burst height; the burst shares η·m·Q
  among the petals, each moving in proportion to its packing radius. No two
  bursts match: the charge's strength varies, the casing tears unevenly, the
  flower comes out a little flattened or drawn out, and about one star in
  fifty never lights. Pattern shells load upright and mostly face the
  audience; water shells burst when they land.
- `src/comet.rs`: shell-less devices. A comet is one large star shot from its
  own tube; a mine throws loose stars from a mortar as a cone; a gerb never
  leaves its tube and jets sparks upward.
- `src/designs.rs`: every shell and device, described below.
- `src/fleet.rs`: the firing barges and their tugs, at about half real scale.
  Each face of every hull gathers the light of every burning star and flash by
  the inverse-square law and Lambert's cosine.
- `src/particle.rs`: 3D bodies with gravity, wind, and Reynolds-dependent drag
  in air that thins with altitude. Drag is integrated implicitly so 0.2 mm
  sparks stay stable. Also holds the fixed-step clock and bounded storage.
- `src/projection.rs`: a perspective camera with a ~45° field of view. Metres
  are square on screen, the sky fills the top 80% of the frame, and framing
  fits the show's stage (520 m × 300 m on desktop, 190 m × 300 m on phones) to
  any viewport, which puts the viewer 0.45–1.3 km from the barges.
- `src/show.rs`: venues, budgets, fleets, and cues. `src/show/sheet.rs` is
  the cue-sheet vocabulary (shots, salvos, fans, chases, cakes, candles,
  gerbs, water fans, star mines, and a bed of low effects), and
  `src/show/programme.rs` is the one programme every venue plays, described
  below.
- `src/render.rs`: packs light into the output buffers.
- `src/trail.rs`: eight-sample motion history (persistence of vision) per star.
- `src/world.rs`: the `World` that owns every shell, star, spark, and smoke
  parcel and advances them one fixed step at a time. Anything burning reports
  what it creates through a single `Spawn`, which also carries the dice.
- `src/rng.rs`: a seeded generator. Every random draw in a show is passed
  down explicitly, so a seed replays a show exactly and engines never share
  state; each loop of the programme draws from its own stream.
- `src/lib.rs`: the exported engine: it fires the programme's cues into the
  world and packs each frame.

Physics uses metres, kilograms, kelvin, and seconds. `tick(seconds)` runs up to
three 1/60 s steps. Each simulated spark stands for many real ones. Once half
the spark budget is in use, spark sampling thins evenly instead of starving
whichever stars update last.

## Shells and devices

Sizes are Japanese gō: 3-gō is 86 mm, 4-gō 114 mm, 5-gō 142 mm, 6-gō 171 mm,
7-gō 199 mm.

The ten signature shells:

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

Modern styles from recent Japanese competitions and Western pyromusicals:

- **Jikansa-botan** (5-gō): four quarters behind dark delays 0.36 s apart
  light around the clock in red, lemon, aqua, and pink, then go out together.
- **Slide-botan** (6-gō): six slices light 0.25 s apart, so a band of aqua
  sweeps across the flower and comes back in pink.
- **Yondan henka-giku** (7-gō): a four-core changing chrysanthemum whose outer
  petal ends in white strobe around a white-strobe heart.
- **Strobe pistils** (4-gō): red, pink, purple, or aqua petals around a
  shimmering 12 Hz white strobe core.
- **Katamono** (4-gō): hearts, smiley faces, stars, and butterflies, fired in
  twos and threes as makers do, since some open edge-on.
- **Double ring** (4-gō), **spider** (5-gō straight gold lines), **horsetail**
  (5-gō falling plume), **time rain** (6-gō sizzling glitter), **falling
  leaves** (5-gō tumbling flakes that fall at about 4 m/s), **bees** (4-gō
  corkscrewing pierced stars), and **titanium salutes** (4-gō flash and white
  sparks).
- **Colpi** (5-gō): an Italian multi-break, red then green then a salute, each
  break lit by its own fuse 0.7–0.8 s after the last.
- **Brocade crown with coloured tips** (6-gō) and **water fans**: water shells
  lobbed from the barges that open on the surface as half-domes.

Star mines use 3-gō peonies (Shimizu's standard 150 × 9 mm stars) in eight
colours and a 4-gō gold chrysanthemum. From the decks: 25 mm silver, gold,
crossette, and crackling comets and 16 mm strobe comets (lifts of 3% of their
weight, about 89 m/s); 18 mm pearls from candles (about 45 m/s); colour,
crackling, silver, willow, and strobe mines (about 62 m/s); and silver and
gold gerbs whose 1.5 mm titanium granules jet about 20 m.

## The two shows

The page picks a show when the engine is created: phones (a coarse pointer and
a screen under 768 px on its short side) get the mobile show, and tablets and
computers get the desktop show. Add `?show=mobile` or `?show=desktop` to the
URL to preview either.

Modern displays keep every layer of the sky busy (Macy's fires about 2,000
effects a minute) and finish with their densest minute. Both shows layer
ground effects low, star-mine chases and pistils in the middle, and feature
shells high, over a bed of pearls, mines, and comets that never pauses. A test
fails if the sky goes more than 0.6 s without a new effect.

|            | Desktop                                  | Mobile                                |
| ---------- | ---------------------------------------- | ------------------------------------- |
| Fleet      | five barges 110 m apart (a 480 m line)   | three barges 55 m apart               |
| Length     | 183 s loop, about 2,400 cues             | 127 s loop, about 1,250 cues          |
| Pace       | 14.0 effects/s, 16.8/s in the finale     | 8.9/s, 15.7/s in the finale           |
| Budget     | 10,000 stars, 36,000 sparks, 2,000 puffs | 4,500 stars, 15,000 sparks, 900 puffs |
| Frame cost | 1.1 ms median, 3.1 ms p95 (Node)         | 0.7 ms median, 1.5 ms p95 (Node)      |

The desktop show runs in five sections:

1. **Overture**: gerbs light the decks, comet chases race the line both ways,
   fan cakes fire from every barge, and pistils, four-core chrysanthemums,
   hearts, and salutes open over Z cakes and a rainbow star mine.
2. **Colour waves**: rainbow chases sweep the line in alternate directions,
   pistils answer, and a feature shell opens every four seconds (slide and
   time-difference peonies, butterflies, Saturns, ghosts, stars, smileys)
   over candles of pearls.
3. **Showcase**: the signature shells, spiders, horsetails, time rain,
   leaves, and bees high, over a pulse of 4-gō shells, water fans, and mines.
4. **Pulse**: fan cakes hop between barges every 1.6 s over snaking Z cakes,
   V chases open from the centre, colpi climb in steps, and an accelerating
   star mine ends on a wall of salutes.
5. **Gold and silver, then the finale**: hanging crowns and water fans; then
   chases every two seconds, the densest star mine, crossette fans from every
   barge, salute chases, a strobe wall, and five kamuro left hanging.

The phone show plays the same programme. Each section is written once
against the venue's barges, and feature shells open as many abreast as the
line has room for (a flower spans about 1,050 times its shell's diameter),
so a pair of 7-gō shells on desktop is one centred shell on a phone. A small
`Plan` per venue sets only section lengths and densities: five colour waves
instead of seven, ten fan-cake hops instead of twenty, a shorter barrage, no
gold-and-silver interlude, and star mines firing at 55% of the desktop
rate.

## Output buffers

Layouts are documented in `src/render.rs` and mirrored by
`src/lib/renderers/glow.ts`. Lengths count floats, not bytes. Positions,
radii, and widths are CSS pixels from the viewport's top left.

- Points, eight values: `[x, y, radius, red, green, blue, intensity, kind]`.
  Kind 0 is a burning emitter, 1 smoke (radius is the parcel's size, intensity
  its optical depth), 2 a flash, and 3 a lamp. Colours are sRGB 0–255;
  intensity is linear light, unbounded.
- Trail segments, ten values: `[x1, y1, x2, y2, width, red, green, blue,
intensity, reserved]`. Stars draw four history segments; each spark draws
  one streak covering 80 ms of motion, and very bright sparks also draw a
  point.
- Mesh vertices, six values: `[x, y, red, green, blue, coverage]`, linear
  radiance, three per triangle, back to front.

The renderer accumulates light in a half-float target (8-bit at 1/16 scale
where the GPU cannot render floats), blooms it through a seven-level pyramid,
lights the smoke and air with the smoothed result, reflects the show in the
water about the waterline beneath the barges, and tone maps once. Recreate
output views after `tick()` or `resize()`, because WASM memory can move.

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
green, yellow, orange, aqua, lemon, and pink burn rates; glitter, microstar,
and time-rain timings and flash temperatures; the twinkler's flash fraction
and the 12 Hz white strobe (strobes run at 7–20 Hz); fish- and bee-star
exhaust speeds; the crossette split efficiency; the crossette, palm, spider,
horsetail, leaf, bee, salute, colpi, water-shell, and small-flower charges;
the leaf's effective density; gerb granule sizes; comet, pearl, and mine
lifts; the variation between stars and between bursts; smoke spread and
lifetime; relative luminous intensities; and the number of sparks sampled per
gram. Light shells use a lift of 6% of their mass, matching the 5–8% in
Shimizu Table 27. The wind is a constant 2.2 m/s with no shear.

## Browser API

```ts
const engine = new FireworkEngine(width, height, mobile, seed); // CSS px; seed optional
engine.mobile(); // which programme is playing
engine.seed(); // the seed in use: pass it back to replay the show
engine.tick(elapsedSeconds);
// Recreate views after tick/resize because WASM memory can move.
const points = new Float32Array(
  exports.memory.buffer,
  engine.points_ptr(),
  engine.points_len(),
); // likewise trails_ptr/len and mesh_ptr/len
engine.horizon(); // CSS px from the top: where sky meets water
engine.waterline(); // CSS px from the top: the water beneath the barges
engine.star_count(); // also spark_count, smoke_count, shell_count
engine.resize(newWidth, newHeight); // CSS px
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

Use pnpm 10 with the current lockfile. Native tests use fixed seeds, and one
checks that a seed replays a show frame for frame while another engine runs.
On the page, `?seed=123` replays a show; otherwise the seed is logged to the
console.
They check that every stored colour matches its spectrum, layered mass and
burn-through, crossette mass and momentum conservation, spark cooling, strobe
and thrust, each shell's burst height, muzzle speed, climb time, star speed,
and flower diameter against the published tables above, hull lighting by the
inverse-square law, the viewer distance, that every cue fires from a barge
deck and bursts on time and inside the frame on its venue's viewports, that
both shows feature every signature and modern shell, never leave the sky
empty for more than 0.6 s, and end densest, and that both loop within budget.
Browser verification should cover both shows, resizing and rotation, and WASM
initialization/cleanup.

The renderer tests enforce draw-call, render-target, and allocation budgets
with a mocked GPU; they do not measure browser FPS or validate shader output.
In full-show WASM benchmarks under Node, a desktop frame took 1.1 ms at the
median, 3.1 ms at the 95th percentile, and at most 4.0 ms on the development
machine; a mobile frame took 0.7, 1.5, and 2.0 ms. Actual frame rate also
depends on the browser and GPU.
