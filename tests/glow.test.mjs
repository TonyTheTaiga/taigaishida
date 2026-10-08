import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

// Test GPU command budgets and resource lifecycle without requiring a browser.
// Shader compilation and visual output still require a real WebGL 2 context.
const source = await readFile(
  new URL("../src/lib/renderers/glow.ts", import.meta.url),
  "utf8",
);
const js = ts.transpileModule(source, {
  compilerOptions: {
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ES2022,
  },
}).outputText;
const glowModuleUrl = `data:text/javascript;base64,${Buffer.from(js).toString("base64")}`;
const { GlowRenderer } = await import(glowModuleUrl);
function fixture({ shaderFailure = false, floatTargets = false } = {}) {
  let id = 0;
  const calls = [];
  const constants = new Map();
  const state = { lost: false };
  const gl = new Proxy(
    {},
    {
      get(_, name) {
        if (/^[A-Z_0-9]+$/.test(name)) {
          if (!constants.has(name)) constants.set(name, ++id);
          return constants.get(name);
        }
        return (...args) => {
          calls.push({ name, args });
          if (name.startsWith("create") || name === "getUniformLocation")
            return { id: ++id };
          if (name === "getShaderParameter") return !shaderFailure;
          if (name === "getProgramParameter") return true;
          if (name === "getExtension")
            return floatTargets && args[0] === "EXT_color_buffer_float"
              ? {}
              : null;
          if (name === "checkFramebufferStatus") return gl.FRAMEBUFFER_COMPLETE;
          if (name === "isContextLost") return state.lost;
        };
      },
    },
  );
  const canvas = { width: 0, height: 0, getContext: () => gl };
  globalThis.window = { devicePixelRatio: 3 };
  return { canvas, gl, calls, state };
}

// Matches the desktop budgets: 10,000 stars + 2,000 puffs + lamps, 76,000
// trail segments, and the fleet's mesh.
const frame = {
  points: new Float32Array(12030 * 8),
  trails: new Float32Array(76000 * 10),
  mesh: new Float32Array(900 * 6),
  horizon: 864,
  waterline: 880,
};
const BLOOM_LEVELS = 7;

test("one frame: vessels, light, smoke, a bloom pyramid, and a composite", () => {
  const { canvas, calls } = fixture();
  const renderer = new GlowRenderer(canvas);
  renderer.resize(1920, 1080);
  calls.length = 0;
  renderer.draw(frame, 1920, 1080);
  const draws = calls.filter((c) => c.name.startsWith("draw"));
  // Mesh, trails, lights, smoke, the pyramid down and back up, composite.
  assert.equal(draws.length, 4 + BLOOM_LEVELS + (BLOOM_LEVELS - 1) + 1);
  assert.deepEqual(
    draws.filter((c) => c.name === "drawArraysInstanced").map((c) => c.args[3]),
    [76000, 12030, 12030],
  );
  assert.equal(draws[0].args[2], 900);
  // Each buffer is uploaded once; smoke reuses the point buffer.
  assert.equal(calls.filter((c) => c.name === "bufferSubData").length, 3);
  assert.equal(calls.filter((c) => c.name === "drawElements").length, 0);
  const targets = calls.filter((c) => c.name === "bindFramebuffer");
  assert.notEqual(targets[0].args[1], null);
  assert.equal(targets.at(-1).args[1], null);
  assert.ok(canvas.width * canvas.height < 1_805_000);
  renderer.dispose();
});

test("light is stored in half floats when the GPU can render them", () => {
  for (const floatTargets of [true, false]) {
    const { canvas, calls, gl } = fixture({ floatTargets });
    const renderer = new GlowRenderer(canvas);
    renderer.resize(1280, 720);
    const formats = calls
      .filter((c) => c.name === "texImage2D")
      .map((c) => c.args[2]);
    assert.equal(formats.length, 2 + BLOOM_LEVELS);
    assert.ok(
      formats.every((f) => f === (floatTargets ? gl.RGBA16F : gl.RGBA)),
    );
    renderer.dispose();
  }
});

test("steady frames reuse GPU storage, shaders, and render targets", () => {
  const { canvas, calls } = fixture();
  const renderer = new GlowRenderer(canvas);
  renderer.resize(1920, 1080);
  renderer.draw(frame, 1920, 1080);
  calls.length = 0;
  renderer.draw(frame, 1920, 1080);
  assert.equal(calls.filter((c) => c.name === "bufferData").length, 0);
  assert.equal(calls.filter((c) => c.name.startsWith("create")).length, 0);
  assert.equal(calls.filter((c) => c.name === "texImage2D").length, 0);
  renderer.dispose();
  assert.equal(calls.filter((c) => c.name === "deleteProgram").length, 7);
  assert.equal(calls.filter((c) => c.name === "deleteBuffer").length, 3);
  assert.equal(
    calls.filter((c) => c.name === "deleteTexture").length,
    2 + BLOOM_LEVELS,
  );
});

test("lost contexts skip drawing, and invalid shaders fail clearly", () => {
  const { canvas, calls, state } = fixture();
  const renderer = new GlowRenderer(canvas);
  state.lost = true;
  calls.length = 0;
  renderer.draw(frame, 1920, 1080);
  assert.equal(calls.filter((c) => c.name.startsWith("draw")).length, 0);
  renderer.dispose();
  const failure = fixture({ shaderFailure: true });
  assert.throws(
    () => new GlowRenderer(failure.canvas),
    /shader compilation failed/,
  );
  assert.equal(
    failure.calls.filter((c) => c.name === "deleteProgram").length,
    1,
  );
  assert.equal(
    failure.calls.filter((c) => c.name === "deleteShader").length,
    1,
  );
});
