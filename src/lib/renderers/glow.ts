// GPU pipeline for the fireworks engine's buffers. Layouts mirror
// crates/fireworks-wasm/src/render.rs:
// - points, 8 floats: x, y (grid cells), radius (CSS px), red, green, blue
//   (sRGB 0–255), intensity (linear), kind (0 emitter, 1 smoke, 2 flash,
//   3 lamp);
// - trails, 10 floats: x0, y0, x1, y1 (grid cells), width (CSS px), red,
//   green, blue, intensity, unused;
// - mesh, 6 floats per vertex: x, y (grid cells), red, green, blue (linear
//   radiance), coverage.
// Light accumulates unclipped in a half-float scene target, spreads through a
// bloom pyramid, lights the smoke and haze, reflects in the water, and is
// tone mapped once at the end.
const QUAD = `
const vec2 corners[6] = vec2[6](
  vec2(-1., -1.), vec2(1., -1.), vec2(-1., 1.),
  vec2(-1., 1.), vec2(1., -1.), vec2(1., 1.)
);`;

const TO_CLIP = `
vec4 clip(vec2 pixel) {
  return vec4(pixel / u_size * vec2(2., -2.) + vec2(-1., 1.), 0., 1.);
}`;

/** Grid cells are 14 × 18 CSS px. */
const CELL = `const vec2 CELL = vec2(14., 18.);`;

const POINT_VERTEX = `#version 300 es
precision highp float;
precision highp int;
layout(location=0) in vec4 a_position; // x, y, radius, red
layout(location=1) in vec4 a_color; // green, blue, intensity, kind
uniform vec2 u_size;
uniform float u_smoke;
out vec2 v_local;
out vec3 v_color;
out float v_intensity;
out float v_kind;
${QUAD}
${CELL}
${TO_CLIP}
void main() {
  float kind = a_color.w;
  bool smoke = abs(kind - 1.) < .5;
  v_local = corners[gl_VertexID];
  v_color = pow(vec3(a_position.w, a_color.xy) / 255., vec3(2.2));
  v_intensity = a_color.z;
  v_kind = kind;
  if (smoke != (u_smoke > .5)) {
    gl_Position = vec4(2., 2., 2., 1.);
    return;
  }
  gl_Position = clip(a_position.xy * CELL + v_local * a_position.z);
}`;

const LIGHT_FRAGMENT = `#version 300 es
precision highp float;
precision highp int;
in vec2 v_local;
in vec3 v_color;
in float v_intensity;
in float v_kind;
uniform float u_encode;
out vec4 color;
void main() {
  float r2 = dot(v_local, v_local);
  if (r2 >= 1.) discard;
  vec3 light;
  if (v_kind > 1.5 && v_kind < 2.5) {
    light = v_color * v_intensity * exp(-r2 * 5.);
  } else {
    float energy = min(v_intensity, 6.) * exp(-r2 * (v_kind > 2.5 ? 10. : 6.));
    // An overexposed core reads white whatever the flame colour.
    light = v_color * energy + vec3(max(energy - 2., 0.) * .12);
  }
  color = vec4(light * u_encode, 0.);
}`;

const SMOKE_FRAGMENT = `#version 300 es
precision highp float;
precision highp int;
in vec2 v_local;
in vec3 v_color;
in float v_intensity;
in float v_kind;
out vec4 color;
void main() {
  float r2 = dot(v_local, v_local);
  if (r2 >= 1.) discard;
  float puff = 1. - r2;
  color = vec4(v_intensity * puff * puff, 0., 0., 0.);
}`;

const TRAIL_VERTEX = `#version 300 es
precision highp float;
precision highp int;
layout(location=0) in vec4 a_ends;
layout(location=1) in vec4 a_style; // width, red, green, blue
layout(location=2) in vec2 a_alpha; // intensity, unused
uniform vec2 u_size;
out vec3 v_color;
out float v_intensity;
out float v_edge;
${QUAD}
${CELL}
${TO_CLIP}
void main() {
  vec2 corner = corners[gl_VertexID];
  vec2 start = a_ends.xy * CELL;
  vec2 end = a_ends.zw * CELL;
  vec2 direction = end - start;
  float distance = length(direction);
  vec2 normal = vec2(-direction.y, direction.x) / max(distance, .001);
  float halfWidth = max(.6, a_style.x * .5);
  gl_Position = clip(mix(start, end, corner.x * .5 + .5) + normal * corner.y * halfWidth);
  v_color = pow(a_style.yzw / 255., vec3(2.2));
  v_intensity = a_alpha.x;
  v_edge = corner.y;
}`;

const TRAIL_FRAGMENT = `#version 300 es
precision highp float;
precision highp int;
in vec3 v_color;
in float v_intensity;
in float v_edge;
uniform float u_encode;
out vec4 color;
void main() {
  float profile = 1. - smoothstep(.2, 1., abs(v_edge));
  color = vec4(v_color * v_intensity * profile * u_encode, 0.);
}`;

const MESH_VERTEX = `#version 300 es
precision highp float;
precision highp int;
layout(location=0) in vec2 a_position;
layout(location=1) in vec4 a_light; // red, green, blue, coverage
uniform vec2 u_size;
out vec4 v_light;
${CELL}
${TO_CLIP}
void main() {
  v_light = a_light;
  gl_Position = clip(a_position * CELL);
}`;

const MESH_FRAGMENT = `#version 300 es
precision highp float;
precision highp int;
in vec4 v_light;
uniform float u_encode;
out vec4 color;
void main() {
  color = vec4(v_light.rgb * u_encode * v_light.a, v_light.a);
}`;

const FULLSCREEN_VERTEX = `#version 300 es
precision highp float;
precision highp int;
out vec2 v_uv;
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  v_uv = p;
  gl_Position = vec4(p * 2. - 1., 0., 1.);
}`;

// The 13-tap downsample from Jimenez's "Next Generation Post Processing in
// Call of Duty: Advanced Warfare" (2014). The first pass weights each group by
// its brightness (a Karis average), so lone sparks cannot flicker the bloom.
const DOWNSAMPLE_FRAGMENT = `#version 300 es
precision highp float;
precision highp int;
uniform sampler2D u_source;
uniform vec2 u_texel;
uniform float u_decode;
uniform float u_karis;
in vec2 v_uv;
out vec4 color;
vec3 tap(float x, float y) {
  return texture(u_source, v_uv + vec2(x, y) * u_texel).rgb * u_decode;
}
float weight(vec3 c) {
  return u_karis > .5 ? 1. / (1. + dot(c, vec3(.2126, .7152, .0722))) : 1.;
}
void main() {
  vec3 a = tap(-2., 2.), b = tap(0., 2.), c = tap(2., 2.);
  vec3 d = tap(-2., 0.), e = tap(0., 0.), f = tap(2., 0.);
  vec3 g = tap(-2., -2.), h = tap(0., -2.), i = tap(2., -2.);
  vec3 j = tap(-1., 1.), k = tap(1., 1.), l = tap(-1., -1.), m = tap(1., -1.);
  vec3 groups[5] = vec3[5](
    (j + k + l + m) * .25,
    (a + b + d + e) * .25, (b + c + e + f) * .25,
    (d + e + g + h) * .25, (e + f + h + i) * .25
  );
  float shares[5] = float[5](.5, .125, .125, .125, .125);
  vec3 sum = vec3(0.);
  float total = 0.;
  for (int n = 0; n < 5; n++) {
    float w = shares[n] * weight(groups[n]);
    sum += groups[n] * w;
    total += w;
  }
  color = vec4(sum / total, 1.);
}`;

// 3 × 3 tent filter, added onto the next larger level.
const UPSAMPLE_FRAGMENT = `#version 300 es
precision highp float;
precision highp int;
uniform sampler2D u_source;
uniform vec2 u_texel;
in vec2 v_uv;
out vec4 color;
vec3 tap(float x, float y) {
  return texture(u_source, v_uv + vec2(x, y) * u_texel).rgb;
}
void main() {
  vec3 sum = tap(0., 0.) * 4.
    + (tap(0., 1.) + tap(-1., 0.) + tap(1., 0.) + tap(0., -1.)) * 2.
    + tap(-1., 1.) + tap(1., 1.) + tap(-1., -1.) + tap(1., -1.);
  color = vec4(sum / 16., 1.);
}`;

const COMPOSITE_FRAGMENT = `#version 300 es
precision highp float;
precision highp int;
uniform sampler2D u_scene;
uniform sampler2D u_bloom;
uniform sampler2D u_smoke;
uniform float u_time;
uniform vec2 u_size;
uniform float u_horizon;
uniform float u_waterline;
uniform float u_decode;
in vec2 v_uv;
out vec4 color;
// Halation around every light, from the summed bloom pyramid. The same
// smoothed light stands in for what the air and smoke scatter: sampling the
// pyramid's coarse levels directly shows their texels as rectangles.
const float BLOOM = .035;
const float HAZE = .02;
const float SMOKE_LIGHT = .12;
const vec3 SMOKE_AMBIENT = vec3(.0016, .0019, .0026);
// Calm water reflects about a quarter of grazing light.
const float REFLECT = .26;
const float EXPOSURE = .85;
float hash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
float noise(vec2 p) {
  vec2 i=floor(p), f=fract(p); f=f*f*(3.-2.*f);
  return mix(mix(hash(i),hash(i+vec2(1,0)),f.x),
    mix(hash(i+vec2(0,1)),hash(i+vec2(1)),f.x),f.y);
}
float stars(vec2 uv, float scale, float threshold, float radius) {
  vec2 p=uv*scale, cell=floor(p), local=fract(p)-.5;
  float h=hash(cell);
  float dotLight=1.-smoothstep(radius*.25,radius,length(local));
  return step(threshold,h)*dotLight*(.55+.45*sin(u_time*1.3+h*6.28));
}
vec3 light(vec2 uv) {
  return texture(u_scene, uv).rgb * u_decode + texture(u_bloom, uv).rgb * BLOOM;
}
// Narkowicz's fit of the ACES filmic curve, then the display's gamma.
vec3 tonemap(vec3 x) {
  x *= EXPOSURE;
  vec3 mapped = (x * (2.51 * x + .03)) / (x * (2.43 * x + .59) + .14);
  return pow(clamp(mapped, 0., 1.), vec3(1. / 2.2));
}
void main() {
  float screenY=1.-v_uv.y;
  float horizon=u_horizon/u_size.y;
  float waterline=u_waterline/u_size.y;
  float shore=.94;
  float aspect=u_size.x/u_size.y;
  float x=(v_uv.x-.5)*aspect;
  vec4 scene=texture(u_scene,v_uv);
  vec3 glow=texture(u_bloom,v_uv).rgb;
  vec3 hdr=scene.rgb*u_decode+glow*BLOOM;
  vec3 haze=glow;
  vec3 background;

  if (screenY < horizon) {
    float skyHeight=screenY/horizon;
    vec3 zenith=vec3(.008,.016,.031);
    vec3 lowSky=vec3(.035,.052,.065);
    background=mix(lowSky,zenith,smoothstep(.02,.92,skyHeight));
    float glow=exp(-pow((screenY-(horizon-.012))*42.,2.));
    background+=glow*vec3(.035,.026,.016);
    float cloud=noise(vec2(x*2.8+u_time*.002,skyHeight*10.));
    float cloudBand=smoothstep(.64,.82,cloud)*smoothstep(.12,.45,skyHeight)
      *(1.-smoothstep(.62,.86,skyHeight));
    background=mix(background,background+vec3(.012,.018,.022),cloudBand*.48);
    float star=stars(v_uv,145.,.991,.19)+stars(v_uv,82.,.996,.13)*.8;
    background+=vec3(.42,.53,.78)*star*.009;
    hdr+=haze*HAZE;
  } else {
    float depth=clamp((screenY-horizon)/(shore-horizon),0.,1.);
    float waveX=x*(5.+depth*30.);
    float wavePhase=waveX+sin(depth*19.+x*3.)*.48+u_time*(.18+depth*.32);
    float wave=pow(max(0.,sin(wavePhase)),22.);
    float glintNoise=noise(vec2(waveX*2.4+u_time*.035,depth*22.-u_time*.018));
    float glints=smoothstep(.73,.94,glintNoise)*wave;
    vec3 farWater=vec3(.016,.031,.039);
    vec3 nearWater=vec3(.003,.009,.013);
    background=mix(farWater,nearWater,smoothstep(0.,1.,depth));
    background+=vec3(.012,.016,.016)*wave*(.12+.48*depth);
    background+=vec3(.10,.12,.105)*glints*(.12+.55*depth);

    float shoreline=.94+.003*sin(x*17.)+.0015*(noise(vec2(x*9.,1.))-0.5);
    float sandMask=smoothstep(shoreline-.004,shoreline+.004,screenY);
    // Mirror the show about the water beneath the barges. Ripples break it
    // into wavering columns that stretch toward the viewer.
    if (screenY > waterline) {
      float reflectedY=2.*waterline-screenY;
      float distortion=sin(wavePhase*1.7+depth*11.)*.0022*depth;
      vec2 mirrored=vec2(v_uv.x+distortion,1.-reflectedY);
      float stretch=.004+.01*depth;
      vec3 reflected=(light(mirrored)+light(mirrored+vec2(0.,stretch))
        +light(mirrored-vec2(0.,stretch)))/3.;
      hdr+=(reflected*REFLECT+haze*HAZE*.35)*(.6+.4*wave)*(1.-sandMask);
    }

    float foam=exp(-pow((screenY-shoreline)*250.,2.));
    float sandGrain=noise(vec2(x*180.,screenY*140.));
    vec3 wetSand=vec3(.018,.015,.012)+sandGrain*.006;
    wetSand=mix(wetSand,vec3(.034,.025,.017),smoothstep(.955,1.,screenY)*.45);
    background=mix(background,wetSand,sandMask);
    background+=vec3(.10,.115,.105)*foam*(1.-sandMask*.65);
    hdr+=haze*HAZE*.2*sandMask;
  }

  float smoke=texture(u_smoke,v_uv).r;
  hdr=hdr*exp(-smoke*.3)+smoke*(SMOKE_AMBIENT+glow*SMOKE_LIGHT);

  float vignette=1.-.24*dot(vec2(x,screenY-.5),vec2(x,screenY-.5));
  vec3 display=(tonemap(hdr)+background*(1.-scene.a))*vignette;
  display+=(hash(gl_FragCoord.xy+fract(u_time*7.))-.5)/255.;
  color=vec4(display,1.);
}`;

function program(gl: WebGL2RenderingContext, vertex: string, fragment: string) {
  const result = gl.createProgram();
  if (!result) throw new Error("Could not allocate a shader program");
  for (const [kind, source] of [
    [gl.VERTEX_SHADER, vertex],
    [gl.FRAGMENT_SHADER, fragment],
  ] as const) {
    const shader = gl.createShader(kind);
    if (!shader) {
      gl.deleteProgram(result);
      throw new Error("Could not allocate a shader");
    }
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      const detail = gl.getShaderInfoLog(shader);
      gl.deleteShader(shader);
      gl.deleteProgram(result);
      throw new Error(`Fireworks shader compilation failed: ${detail}`);
    }
    gl.attachShader(result, shader);
    gl.deleteShader(shader);
  }
  gl.linkProgram(result);
  if (!gl.getProgramParameter(result, gl.LINK_STATUS)) {
    const detail = gl.getProgramInfoLog(result);
    gl.deleteProgram(result);
    throw new Error(`Fireworks shader linking failed: ${detail}`);
  }
  return result;
}

class Batch {
  readonly buffer: WebGLBuffer;
  readonly vao: WebGLVertexArrayObject;
  private capacity = 0;

  constructor(
    private gl: WebGL2RenderingContext,
    readonly stride: number,
    attributes: number[],
    instanced: boolean,
  ) {
    const buffer = gl.createBuffer();
    const vao = gl.createVertexArray();
    if (!buffer || !vao) {
      gl.deleteBuffer(buffer);
      gl.deleteVertexArray(vao);
      throw new Error("Could not allocate particle buffers");
    }
    this.buffer = buffer;
    this.vao = vao;
    gl.bindVertexArray(vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    let offset = 0;
    attributes.forEach((size, index) => {
      gl.enableVertexAttribArray(index);
      gl.vertexAttribPointer(
        index,
        size,
        gl.FLOAT,
        false,
        stride * 4,
        offset * 4,
      );
      if (instanced) gl.vertexAttribDivisor(index, 1);
      offset += size;
    });
    gl.bindVertexArray(null);
  }

  upload(data: Float32Array) {
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buffer);
    if (data.byteLength > this.capacity) {
      this.capacity =
        2 ** Math.ceil(Math.log2(Math.max(1024, data.byteLength)));
      gl.bufferData(gl.ARRAY_BUFFER, this.capacity, gl.DYNAMIC_DRAW);
    }
    if (data.byteLength) gl.bufferSubData(gl.ARRAY_BUFFER, 0, data);
    gl.bindVertexArray(this.vao);
  }

  dispose() {
    this.gl.deleteBuffer(this.buffer);
    this.gl.deleteVertexArray(this.vao);
  }
}

interface Target {
  texture: WebGLTexture;
  framebuffer: WebGLFramebuffer;
  width: number;
  height: number;
}

/** One engine frame: its buffers and where the water lies, in CSS px. */
export interface GlowFrame {
  points: Float32Array;
  trails: Float32Array;
  mesh: Float32Array;
  horizon: number;
  waterline: number;
}

/** Bloom pyramid depth: the smallest level spans about 1/128 of the frame. */
const BLOOM_LEVELS = 7;
/** Without float targets, light is stored at 1/16 so 8-bit channels hold it. */
const LOW_PRECISION_SCALE = 16;

export class GlowRenderer {
  private gl: WebGL2RenderingContext;
  private programs: Record<
    "light" | "smoke" | "trail" | "mesh" | "down" | "up" | "composite",
    WebGLProgram
  >;
  private points: Batch;
  private trails: Batch;
  private mesh: Batch;
  private targets: Target[] = [];
  private scene?: Target;
  private smoke?: Target;
  private bloom: Target[] = [];
  private float: boolean;
  private uniforms = new Map<string, WebGLUniformLocation | null>();

  constructor(private canvas: HTMLCanvasElement) {
    const gl = canvas.getContext("webgl2", {
      alpha: false,
      antialias: false,
      depth: false,
      stencil: false,
      preserveDrawingBuffer: false,
    });
    if (!gl) throw new Error("WebGL 2 is unavailable");
    this.gl = gl;
    this.float =
      gl.getExtension("EXT_color_buffer_float") !== null ||
      gl.getExtension("EXT_color_buffer_half_float") !== null;
    const programs: WebGLProgram[] = [];
    const batches: Batch[] = [];
    const build = (vertex: string, fragment: string) => {
      const result = program(gl, vertex, fragment);
      programs.push(result);
      return result;
    };
    try {
      this.programs = {
        light: build(POINT_VERTEX, LIGHT_FRAGMENT),
        smoke: build(POINT_VERTEX, SMOKE_FRAGMENT),
        trail: build(TRAIL_VERTEX, TRAIL_FRAGMENT),
        mesh: build(MESH_VERTEX, MESH_FRAGMENT),
        down: build(FULLSCREEN_VERTEX, DOWNSAMPLE_FRAGMENT),
        up: build(FULLSCREEN_VERTEX, UPSAMPLE_FRAGMENT),
        composite: build(FULLSCREEN_VERTEX, COMPOSITE_FRAGMENT),
      };
      this.points = new Batch(gl, 8, [4, 4], true);
      batches.push(this.points);
      this.trails = new Batch(gl, 10, [4, 4, 2], true);
      batches.push(this.trails);
      this.mesh = new Batch(gl, 6, [2, 4], false);
      batches.push(this.mesh);
    } catch (error) {
      programs.forEach((p) => gl.deleteProgram(p));
      batches.forEach((b) => b.dispose());
      throw error;
    }
    gl.useProgram(this.programs.composite);
    ["u_scene", "u_bloom", "u_smoke"].forEach((name, unit) =>
      gl.uniform1i(gl.getUniformLocation(this.programs.composite, name), unit),
    );
  }

  private uniform(target: WebGLProgram, name: string) {
    const key = `${Object.entries(this.programs).find(([, p]) => p === target)?.[0]}:${name}`;
    if (!this.uniforms.has(key))
      this.uniforms.set(key, this.gl.getUniformLocation(target, name));
    return this.uniforms.get(key) ?? null;
  }

  private target(width: number, height: number): Target {
    const gl = this.gl;
    const texture = gl.createTexture();
    const framebuffer = gl.createFramebuffer();
    if (!texture || !framebuffer) {
      gl.deleteTexture(texture);
      gl.deleteFramebuffer(framebuffer);
      throw new Error("Could not allocate the fireworks render targets");
    }
    const result = { texture, framebuffer, width, height };
    this.targets.push(result);
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      this.float ? gl.RGBA16F : gl.RGBA,
      width,
      height,
      0,
      gl.RGBA,
      this.float ? gl.HALF_FLOAT : gl.UNSIGNED_BYTE,
      null,
    );
    gl.bindFramebuffer(gl.FRAMEBUFFER, framebuffer);
    gl.framebufferTexture2D(
      gl.FRAMEBUFFER,
      gl.COLOR_ATTACHMENT0,
      gl.TEXTURE_2D,
      texture,
      0,
    );
    const complete =
      gl.checkFramebufferStatus(gl.FRAMEBUFFER) === gl.FRAMEBUFFER_COMPLETE;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    if (!complete) throw new Error("The fireworks framebuffer is incomplete");
    return result;
  }

  private releaseTargets() {
    for (const target of this.targets) {
      this.gl.deleteTexture(target.texture);
      this.gl.deleteFramebuffer(target.framebuffer);
    }
    this.targets = [];
    this.bloom = [];
    this.scene = undefined;
    this.smoke = undefined;
  }

  private allocate(width: number, height: number) {
    this.releaseTargets();
    this.scene = this.target(width, height);
    this.smoke = this.target(
      Math.max(1, Math.round(width / 4)),
      Math.max(1, Math.round(height / 4)),
    );
    for (let level = 1; level <= BLOOM_LEVELS; level++) {
      this.bloom.push(
        this.target(
          Math.max(1, Math.round(width / 2 ** level)),
          Math.max(1, Math.round(height / 2 ** level)),
        ),
      );
    }
  }

  resize(width: number, height: number) {
    // Keep fragment work bounded across Retina, portrait, and ultrawide screens.
    const ratio = Math.min(
      window.devicePixelRatio || 1,
      1.35,
      Math.sqrt(1_800_000 / Math.max(1, width * height)),
    );
    this.canvas.width = Math.max(1, Math.round(width * ratio));
    this.canvas.height = Math.max(1, Math.round(height * ratio));
    try {
      this.allocate(this.canvas.width, this.canvas.height);
    } catch (error) {
      if (!this.float) throw error;
      // Some drivers advertise float targets but cannot render to them.
      this.float = false;
      this.allocate(this.canvas.width, this.canvas.height);
    }
  }

  private bind(target: Target | null) {
    const gl = this.gl;
    gl.bindFramebuffer(gl.FRAMEBUFFER, target?.framebuffer ?? null);
    gl.viewport(
      0,
      0,
      target?.width ?? this.canvas.width,
      target?.height ?? this.canvas.height,
    );
  }

  draw(frame: GlowFrame, width: number, height: number) {
    const gl = this.gl;
    const { scene, smoke, bloom } = this;
    if (gl.isContextLost() || !scene || !smoke) return;
    const encode = this.float ? 1 : 1 / LOW_PRECISION_SCALE;
    const { programs } = this;

    // Vessels, then light added on top without touching their coverage.
    this.bind(scene);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.useProgram(programs.mesh);
    gl.uniform2f(this.uniform(programs.mesh, "u_size"), width, height);
    gl.uniform1f(this.uniform(programs.mesh, "u_encode"), encode);
    this.mesh.upload(frame.mesh);
    gl.drawArrays(gl.TRIANGLES, 0, frame.mesh.length / 6);

    gl.blendFuncSeparate(gl.ONE, gl.ONE, gl.ZERO, gl.ONE);
    gl.useProgram(programs.trail);
    gl.uniform2f(this.uniform(programs.trail, "u_size"), width, height);
    gl.uniform1f(this.uniform(programs.trail, "u_encode"), encode);
    this.trails.upload(frame.trails);
    gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, frame.trails.length / 10);

    gl.useProgram(programs.light);
    gl.uniform2f(this.uniform(programs.light, "u_size"), width, height);
    gl.uniform1f(this.uniform(programs.light, "u_encode"), encode);
    gl.uniform1f(this.uniform(programs.light, "u_smoke"), 0);
    this.points.upload(frame.points);
    gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, frame.points.length / 8);

    // Smoke density at quarter resolution, from the same point buffer.
    this.bind(smoke);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.blendFunc(gl.ONE, gl.ONE);
    gl.useProgram(programs.smoke);
    gl.uniform2f(this.uniform(programs.smoke, "u_size"), width, height);
    gl.uniform1f(this.uniform(programs.smoke, "u_smoke"), 1);
    gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, frame.points.length / 8);
    gl.bindVertexArray(null);

    // Bloom: filter down the pyramid, then tent-filter back up, summing.
    gl.disable(gl.BLEND);
    gl.useProgram(programs.down);
    gl.activeTexture(gl.TEXTURE0);
    gl.uniform1i(this.uniform(programs.down, "u_source"), 0);
    let source = scene;
    bloom.forEach((level, index) => {
      this.bind(level);
      gl.bindTexture(gl.TEXTURE_2D, source.texture);
      gl.uniform2f(
        this.uniform(programs.down, "u_texel"),
        1 / source.width,
        1 / source.height,
      );
      gl.uniform1f(
        this.uniform(programs.down, "u_decode"),
        index === 0 ? 1 / encode : 1,
      );
      gl.uniform1f(this.uniform(programs.down, "u_karis"), index === 0 ? 1 : 0);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
      source = level;
    });
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE);
    gl.useProgram(programs.up);
    gl.uniform1i(this.uniform(programs.up, "u_source"), 0);
    for (let index = bloom.length - 1; index > 0; index--) {
      const from = bloom[index];
      this.bind(bloom[index - 1]);
      gl.bindTexture(gl.TEXTURE_2D, from.texture);
      gl.uniform2f(
        this.uniform(programs.up, "u_texel"),
        1 / from.width,
        1 / from.height,
      );
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    }
    gl.disable(gl.BLEND);

    // Composite the sky, water, vessels, smoke, and light.
    this.bind(null);
    gl.useProgram(programs.composite);
    [scene, bloom[0], smoke].forEach((target, unit) => {
      gl.activeTexture(gl.TEXTURE0 + unit);
      gl.bindTexture(gl.TEXTURE_2D, target.texture);
    });
    gl.activeTexture(gl.TEXTURE0);
    gl.uniform2f(this.uniform(programs.composite, "u_size"), width, height);
    gl.uniform1f(
      this.uniform(programs.composite, "u_time"),
      performance.now() * 0.001,
    );
    gl.uniform1f(this.uniform(programs.composite, "u_horizon"), frame.horizon);
    gl.uniform1f(
      this.uniform(programs.composite, "u_waterline"),
      frame.waterline,
    );
    gl.uniform1f(this.uniform(programs.composite, "u_decode"), 1 / encode);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  dispose() {
    this.points.dispose();
    this.trails.dispose();
    this.mesh.dispose();
    this.releaseTargets();
    Object.values(this.programs).forEach((p) => this.gl.deleteProgram(p));
  }
}
