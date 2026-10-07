// GPU batches consume the WASM buffers directly: eight floats per point,
// ten per trail segment. No per-particle JS objects, paths, or sprite canvases.
const QUAD = `
const vec2 corners[6] = vec2[6](
  vec2(-1., -1.), vec2(1., -1.), vec2(-1., 1.),
  vec2(-1., 1.), vec2(1., -1.), vec2(1., 1.)
);`;

const POINT_VERTEX = `#version 300 es
precision highp float;
precision highp int;
layout(location=0) in vec4 a_position; // x, y, radius, red
layout(location=1) in vec4 a_color; // green, blue, alpha, kind
uniform vec2 u_size;
out vec2 v_local;
out vec3 v_color;
out float v_alpha;
out float v_kind;
${QUAD}
void main() {
  bool smoke = a_color.w > 0.5 && a_color.w < 1.5;
  v_local = corners[gl_VertexID];
  vec2 center = a_position.xy * vec2(14., 18.);
  float radius = a_position.z * (smoke ? 2. : 4.);
  vec2 pixel = center + v_local * radius;
  gl_Position = vec4(pixel / u_size * vec2(2., -2.) + vec2(-1., 1.), 0., 1.);
  v_color = vec3(a_position.w, a_color.xy) / 255.;
  v_alpha = a_color.z;
  v_kind = a_color.w;
}`;

const POINT_FRAGMENT = `#version 300 es
precision highp float;
precision highp int;
in vec2 v_local;
in vec3 v_color;
in float v_alpha;
in float v_kind;
uniform float u_flash;
out vec4 color;
void main() {
  float radius = length(v_local);
  if (radius >= 1.) discard;
  float glow = exp(-radius * 6.) * (1. - smoothstep(0.7, 1., radius));
  float alpha = v_alpha * glow;
  vec3 light = mix(v_color, vec3(1., .98, .92), exp(-radius * 28.));
  if (v_kind > 0.5 && v_kind < 1.5) {
    light = v_color;
    alpha *= .35 * (.4 + min(1., u_flash));
  }
  color = vec4(light * alpha, alpha);
}`;

const TRAIL_VERTEX = `#version 300 es
precision highp float;
precision highp int;
layout(location=0) in vec4 a_ends;
layout(location=1) in vec4 a_style; // width, red, green, blue
layout(location=2) in vec2 a_alpha;
uniform vec2 u_size;
out vec3 v_color;
out float v_alpha;
out float v_edge;
${QUAD}
void main() {
  vec2 corner = corners[gl_VertexID];
  vec2 start = a_ends.xy * vec2(14., 18.);
  vec2 end = a_ends.zw * vec2(14., 18.);
  vec2 direction = end - start;
  float distance = length(direction);
  vec2 normal = vec2(-direction.y, direction.x) / max(distance, .001);
  float halfWidth = max(.6, a_style.x * .5);
  vec2 pixel = mix(start, end, corner.x * .5 + .5) + normal * corner.y * halfWidth;
  gl_Position = vec4(pixel / u_size * vec2(2., -2.) + vec2(-1., 1.), 0., 1.);
  v_color = a_style.yzw / 255.;
  v_alpha = a_alpha.x;
  v_edge = corner.y;
}`;

const TRAIL_FRAGMENT = `#version 300 es
precision highp float;
precision highp int;
in vec3 v_color;
in float v_alpha;
in float v_edge;
out vec4 color;
void main() {
  float alpha = v_alpha * (1. - smoothstep(.4, 1., abs(v_edge)));
  color = vec4(v_color * alpha, alpha);
}`;

const COMPOSITE_VERTEX = `#version 300 es
precision highp float;
precision highp int;
out vec2 v_uv;
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  v_uv = p;
  gl_Position = vec4(p * 2. - 1., 0., 1.);
}`;

const COMPOSITE_FRAGMENT = `#version 300 es
precision highp float;
precision highp int;
uniform sampler2D u_sky;
uniform float u_time;
uniform vec2 u_size;
in vec2 v_uv;
out vec4 color;
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
void main() {
  float screenY=1.-v_uv.y;
  float horizon=.75;
  float shore=.94;
  float aspect=u_size.x/u_size.y;
  float x=(v_uv.x-.5)*aspect;
  vec3 scene;

  if (screenY < horizon) {
    float skyHeight=screenY/horizon;
    vec3 zenith=vec3(.008,.016,.031);
    vec3 lowSky=vec3(.035,.052,.065);
    scene=mix(lowSky,zenith,smoothstep(.02,.92,skyHeight));
    float haze=exp(-pow((screenY-(horizon-.012))*42.,2.));
    scene+=haze*vec3(.035,.026,.016);
    float cloud=noise(vec2(x*2.8+u_time*.002,skyHeight*10.));
    float cloudBand=smoothstep(.64,.82,cloud)*smoothstep(.12,.45,skyHeight)
      *(1.-smoothstep(.62,.86,skyHeight));
    scene=mix(scene,scene+vec3(.012,.018,.022),cloudBand*.48);
    float star=stars(v_uv,145.,.991,.19)+stars(v_uv,82.,.996,.13)*.8;
    scene+=vec3(.42,.53,.78)*star*.009;
  } else {
    float depth=clamp((screenY-horizon)/(shore-horizon),0.,1.);
    float waveX=x*(5.+depth*30.);
    float wavePhase=waveX+sin(depth*19.+x*3.)*.48+u_time*(.18+depth*.32);
    float wave=pow(max(0.,sin(wavePhase)),22.);
    float glintNoise=noise(vec2(waveX*2.4+u_time*.035,depth*22.-u_time*.018));
    float glints=smoothstep(.73,.94,glintNoise)*wave;
    vec3 farWater=vec3(.016,.031,.039);
    vec3 nearWater=vec3(.003,.009,.013);
    scene=mix(farWater,nearWater,smoothstep(0.,1.,depth));
    scene+=vec3(.012,.016,.016)*wave*(.12+.48*depth);
    scene+=vec3(.10,.12,.105)*glints*(.12+.55*depth);

    // Mirror the bright particle buffer around the horizon and let surface
    // ripples break it into narrow, wavering reflections.
    float reflectedY=2.*horizon-screenY;
    float distortion=sin(wavePhase*1.7+depth*11.)*.0018*depth;
    vec2 reflectedUv=vec2(v_uv.x+distortion,1.-reflectedY);
    vec3 reflected=vec3(0.);
    reflected+=max(texture(u_sky,reflectedUv+vec2(-.002,0.)).rgb-vec3(.0196,.0275,.0627),0.);
    reflected+=max(texture(u_sky,reflectedUv).rgb-vec3(.0196,.0275,.0627),0.);
    reflected+=max(texture(u_sky,reflectedUv+vec2(.002,0.)).rgb-vec3(.0196,.0275,.0627),0.);
    scene+=reflected*(.24*(1.-depth*.55))*(.55+.45*wave);

    float shoreline=.94+.003*sin(x*17.)+.0015*(noise(vec2(x*9.,1.))-0.5);
    float sandMask=smoothstep(shoreline-.004,shoreline+.004,screenY);
    float foam=exp(-pow((screenY-shoreline)*250.,2.));
    float sandGrain=noise(vec2(x*180.,screenY*140.));
    vec3 wetSand=vec3(.018,.015,.012)+sandGrain*.006;
    wetSand=mix(wetSand,vec3(.034,.025,.017),smoothstep(.955,1.,screenY)*.45);
    scene=mix(scene,wetSand,sandMask);
    scene+=vec3(.10,.115,.105)*foam*(1.-sandMask*.65);
  }

  float vignette=1.-.24*dot(vec2(x,screenY-.5),vec2(x,screenY-.5));
  scene*=vignette;
  color=vec4(texture(u_sky,v_uv).rgb+scene,1.);
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
    stride: number,
    attributes: number[],
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
      gl.vertexAttribDivisor(index, 1);
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

export class GlowRenderer {
  private gl: WebGL2RenderingContext;
  private pointProgram: WebGLProgram;
  private trailProgram: WebGLProgram;
  private compositeProgram: WebGLProgram;
  private points: Batch;
  private trails: Batch;
  private texture: WebGLTexture;
  private framebuffer: WebGLFramebuffer;
  private pointSize: WebGLUniformLocation | null;
  private pointFlash: WebGLUniformLocation | null;
  private trailSize: WebGLUniformLocation | null;
  private compositeSize: WebGLUniformLocation | null;
  private compositeTime: WebGLUniformLocation | null;

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
    const programs: WebGLProgram[] = [];
    const batches: Batch[] = [];
    const allocatedTargets: {
      texture: WebGLTexture;
      framebuffer: WebGLFramebuffer;
    }[] = [];
    try {
      this.pointProgram = program(gl, POINT_VERTEX, POINT_FRAGMENT);
      programs.push(this.pointProgram);
      this.trailProgram = program(gl, TRAIL_VERTEX, TRAIL_FRAGMENT);
      programs.push(this.trailProgram);
      this.compositeProgram = program(gl, COMPOSITE_VERTEX, COMPOSITE_FRAGMENT);
      programs.push(this.compositeProgram);
      this.points = new Batch(gl, 8, [4, 4]);
      batches.push(this.points);
      this.trails = new Batch(gl, 10, [4, 4, 2]);
      batches.push(this.trails);
      const texture = gl.createTexture();
      const framebuffer = gl.createFramebuffer();
      if (!texture || !framebuffer) {
        gl.deleteTexture(texture);
        gl.deleteFramebuffer(framebuffer);
        throw new Error("Could not allocate the particle framebuffer");
      }
      this.texture = texture;
      this.framebuffer = framebuffer;
      allocatedTargets.push({ texture, framebuffer });
    } catch (error) {
      programs.forEach((p) => gl.deleteProgram(p));
      batches.forEach((b) => b.dispose());
      for (const target of allocatedTargets) {
        gl.deleteTexture(target.texture);
        gl.deleteFramebuffer(target.framebuffer);
      }
      throw error;
    }
    this.pointSize = gl.getUniformLocation(this.pointProgram, "u_size");
    this.pointFlash = gl.getUniformLocation(this.pointProgram, "u_flash");
    this.trailSize = gl.getUniformLocation(this.trailProgram, "u_size");
    this.compositeSize = gl.getUniformLocation(this.compositeProgram, "u_size");
    this.compositeTime = gl.getUniformLocation(this.compositeProgram, "u_time");
    gl.useProgram(this.compositeProgram);
    gl.uniform1i(gl.getUniformLocation(this.compositeProgram, "u_sky"), 0);
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.framebuffer);
    gl.framebufferTexture2D(
      gl.FRAMEBUFFER,
      gl.COLOR_ATTACHMENT0,
      gl.TEXTURE_2D,
      this.texture,
      0,
    );
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
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
    const gl = this.gl;
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      gl.RGBA,
      this.canvas.width,
      this.canvas.height,
      0,
      gl.RGBA,
      gl.UNSIGNED_BYTE,
      null,
    );
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.framebuffer);
    gl.framebufferTexture2D(
      gl.FRAMEBUFFER,
      gl.COLOR_ATTACHMENT0,
      gl.TEXTURE_2D,
      this.texture,
      0,
    );
    if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE)
      throw new Error("The fireworks framebuffer is incomplete");
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  }

  draw(
    points: Float32Array,
    trails: Float32Array,
    width: number,
    height: number,
  ) {
    const gl = this.gl;
    if (gl.isContextLost()) return;
    let flash = 0;
    for (let i = 0; i < points.length; i += 8)
      if (points[i + 7] === 2) flash += points[i + 6];
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.framebuffer);
    gl.disable(gl.SCISSOR_TEST);
    gl.clearColor(
      0.0196 + Math.min(0.08, flash * 0.035) * 0.25,
      0.0275,
      0.0627,
      1,
    );
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE);
    gl.useProgram(this.trailProgram);
    gl.uniform2f(this.trailSize, width, height);
    this.trails.upload(trails);
    gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, trails.length / 10);

    gl.useProgram(this.pointProgram);
    gl.uniform2f(this.pointSize, width, height);
    gl.uniform1f(this.pointFlash, flash);
    this.points.upload(points);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, points.length / 8);

    gl.bindVertexArray(null);
    gl.disable(gl.BLEND);
    gl.disable(gl.SCISSOR_TEST);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.useProgram(this.compositeProgram);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    const now = performance.now() * 0.001;
    gl.uniform2f(this.compositeSize, width, height);
    gl.uniform1f(this.compositeTime, now);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  dispose() {
    this.points.dispose();
    this.trails.dispose();
    const gl = this.gl;
    gl.deleteTexture(this.texture);
    gl.deleteFramebuffer(this.framebuffer);
    gl.deleteProgram(this.pointProgram);
    gl.deleteProgram(this.trailProgram);
    gl.deleteProgram(this.compositeProgram);
  }
}
