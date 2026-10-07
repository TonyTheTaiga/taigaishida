# Repository Guidelines

## Project Structure & Module Organization

This is a Svelte 5/SvelteKit personal site with a Rust/WebAssembly fireworks engine.

- `src/routes/+page.svelte` owns the full-viewport Glow canvas, diagnostics, and animation loop; `+layout.svelte` provides shared layout.
- `src/app.css` contains global styles; `src/lib/` holds shared modules and bundled assets.
- `crates/fireworks-wasm/src/lib.rs` implements the simulation and exported WASM API.
- `static/` contains directly served assets, including favicons and `robots.txt`.
- `src/lib/fireworks-wasm/` is generated and ignored by Git. Edit the Rust source rather than generated bindings.
- `cloudbuild.yaml` and `app.yaml` configure Google Cloud Build and App Engine deployment.

## Build, Test, and Development Commands

Use pnpm and Node.js 20, matching Cloud Build. Install Rust, `wasm-pack`, and the `wasm32-unknown-unknown` target before building.

- `pnpm install --frozen-lockfile`: install dependencies from the committed lockfile.
- `pnpm run wasm:build`: compile Rust and generate browser WASM bindings. Run before checking a fresh checkout and after Rust changes.
- `pnpm dev`: build WASM, then start the Vite development server.
- `pnpm check`: synchronize SvelteKit types and run Svelte/TypeScript diagnostics.
- `pnpm lint`: run Prettier checks and ESLint; `pnpm format` applies formatting.
- `pnpm build:production`: run diagnostics, then build WASM and the site.
- `pnpm preview`: preview the build; `pnpm start` runs the Node adapter output.

## Coding Style & Naming Conventions

Use TypeScript in Svelte components and preserve strict typing. Match nearby formatting: Svelte scripts use tabs, JavaScript configuration uses two spaces, and Rust uses four spaces. Use camelCase for JavaScript variables/functions, snake_case for Rust functions, PascalCase for Rust types, and UPPER_SNAKE_CASE for constants. Preserve SvelteKit route filenames and use `$lib` imports for shared code.

Keep the Rust point/trail buffer layouts synchronized with the WebGL batch strides in `src/lib/renderers/glow.ts`.

## Testing Guidelines

Renderer tests use Node's built-in test runner (`node --test tests/glow.test.mjs`); Rust simulation tests use `cargo test --manifest-path crates/fireworks-wasm/Cargo.toml`. No coverage threshold is configured. Run checks, lint, and a production build before submitting. For animation changes, verify playback, portrait and landscape resizing, and browser console errors locally.

## Commit & Pull Request Guidelines

History uses short, imperative subjects such as `Fix Cloud Build` and `Port fireworks engine from TypeScript to Rust/WASM`. Follow that style and keep commits focused. PRs should describe behavior changes, list validation performed, link relevant issues, and include screenshots or recordings for visual changes.
