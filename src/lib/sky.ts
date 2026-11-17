// The animated Starbound-style background: a dithered nebula with pixel stars, drawn in
// device pixels so every "art pixel" is exactly an integer number of screen pixels (sharp at
// any resolution / display scaling).
//
// Layers (back to front), each a canvas inside #sky:
//   0  nebula + faint far stars   (static, very slight parallax)
//   1  mid stars                  (some twinkle)
//   2  near stars + sparkles      (some twinkle, most parallax)
//   fx shooting stars             (created only while one is visible)
// A new sky is generated at every launch (random seed); resizing keeps the same one. Dark dust
// clouds ("Dunkelwolken") sit in front of the glowing nebula and the far/mid stars: they
// swallow stars, and redden and dim the ones near their edges.
// Parallax moves the layers with CSS transforms, so mouse movement never repaints them;
// twinkling repaints only the few stars that twinkle, ~20 times a second.

const ART = 3; // CSS pixels per art pixel (rounded to whole device pixels)
const MARGIN = 4; // art pixels of overscan per side, for parallax
const PARALLAX = [0.8, 1.8, 3.2]; // max shift per layer, in art pixels (very subtle)
const TWINKLE_MS = 50;

function rng(seed: number) {
  return () => {
    seed = (seed * 1664525 + 1013904223) >>> 0;
    return seed / 4294967296;
  };
}

const BAYER = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5].map((v) => (v + 0.5) / 16);
// Nebula colour ramps (dark → bright). Clouds blend between them, dithered.
const RAMPS: [number, number, number][][] = [
  // deep blue
  [[5, 6, 14], [9, 13, 32], [14, 24, 56], [22, 38, 86], [38, 64, 128], [80, 120, 186]],
  // teal
  [[5, 6, 14], [7, 17, 28], [10, 33, 46], [15, 54, 68], [30, 92, 104], [84, 160, 168]],
  // violet
  [[5, 6, 14], [14, 10, 30], [29, 17, 52], [48, 26, 80], [80, 42, 116], [150, 96, 176]],
];

// Hash-based value noise (no tiling, no allocations).
function hash(x: number, y: number, seed: number): number {
  let h = Math.imul(x, 374761393) + Math.imul(y, 668265263) + Math.imul(seed, 1442695041);
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}
function vnoise(x: number, y: number, seed: number): number {
  const xi = Math.floor(x),
    yi = Math.floor(y);
  let fx = x - xi,
    fy = y - yi;
  fx = fx * fx * (3 - 2 * fx);
  fy = fy * fy * (3 - 2 * fy);
  const a = hash(xi, yi, seed),
    b = hash(xi + 1, yi, seed),
    c = hash(xi, yi + 1, seed),
    d = hash(xi + 1, yi + 1, seed);
  return a + (b - a) * fx + (c - a) * fy + (a - b - c + d) * fx * fy;
}
function fbm(x: number, y: number, seed: number, octaves = 4): number {
  let sum = 0,
    amp = 0.5,
    norm = 0;
  for (let i = 0; i < octaves; i++) {
    sum += amp * vnoise(x, y, seed + i * 31);
    norm += amp;
    x = x * 2.03 + 17.1;
    y = y * 2.03 - 9.7;
    amp *= 0.5;
  }
  return sum / norm;
}
const smooth = (a: number, b: number, t: number) => {
  const x = Math.max(0, Math.min(1, (t - a) / (b - a)));
  return x * x * (3 - 2 * x);
};

// coverage targets (quantiles of the seed's noise): tuned so skies look like the hand-picked one
// (measured over 30 seeds: ~47% glowing gas, mean density ~0.19, ~11% dark clouds)
const MASK_Q = 0.6,
  GAS_Q = 0.55,
  DARKMASK_Q = 0.62,
  LANE_Q = 0.68;
const DUST: [number, number, number] = [4, 3, 6];
const RIM: [number, number, number] = [52, 22, 14];

/** Paints the nebula into `data` (W×H RGBA, one art pixel each). Returns the glowing cloud
 *  density and the dark-cloud extinction (both 0..1 per art pixel). */
/** (exported for offline tuning/statistics) */
export function paintNebula(data: Uint8ClampedArray, W: number, H: number, seed: number): { density: Float32Array; dark: Float32Array } {
  const density = new Float32Array(W * H);
  const dark = new Float32Array(W * H);
  const F = 1 / 150; // base frequency (art pixels)
  const sr = rng(seed);
  const ox = sr() * 1000,
    oy = sr() * 1000; // where in the infinite noise field this sky sits
  const hueShift = sr() * 1.2 - 0.3;
  const hueScale = 1.6 + sr() * 1.2;
  const S = (seed % 100000) * 7;
  // adaptive thresholds: sample the patch noise coarsely so every seed gets a similar amount
  // of glowing gas (~40–45% of the sky) and of dark clouds
  const maskN = (u: number, v: number) => fbm(u * 0.45 + 40, v * 0.45 - 12, S + 14, 2);
  const darkN = (u: number, v: number) => fbm(u * 0.33 - 61, v * 0.33 + 25, S + 23, 2);
  const quantile = (f: (u: number, v: number) => number, q: number) => {
    const vals: number[] = [];
    for (let y = 0; y < H; y += 12) for (let x = 0; x < W; x += 12) vals.push(f(x * F + ox, y * F + oy));
    vals.sort((a, b) => a - b);
    return vals[Math.floor(q * (vals.length - 1))] ?? 0.5;
  };
  const gasN = (u: number, v: number) => fbm(u + 1.6 * fbm(u + 3.1, v + 1.7, S + 11, 3), v + 1.6 * fbm(u - 5.2, v + 8.3, S + 12, 3), S + 13, 5);
  const laneN = (u: number, v: number) => {
    const dw = fbm(u * 0.9 + 7.7, v * 0.9 - 3.3, S + 21, 3);
    return fbm(u * 1.25 + 1.4 * dw, v * 1.25 - 1.4 * dw, S + 22, 5);
  };
  // thresholds from this seed's own noise, so every sky has a similar amount of gas and dust
  const m0 = quantile(maskN, MASK_Q) - 0.15; // ramp midpoint at the quantile
  const n0 = quantile(gasN, GAS_Q) - 0.18;
  const k0 = quantile(darkN, DARKMASK_Q) - 0.11;
  const l0 = quantile(laneN, LANE_Q) - 0.09;
  for (let y = 0; y < H; y++)
    for (let x = 0; x < W; x++) {
      const u = x * F + ox,
        v = y * F + oy;
      // domain warp gives the wispy, swirled cloud shapes
      const wx = fbm(u + 3.1, v + 1.7, S + 11, 3),
        wy = fbm(u - 5.2, v + 8.3, S + 12, 3);
      const n = fbm(u + 1.6 * wx, v + 1.6 * wy, S + 13, 5);
      // ridged noise along the warped field gives thin bright filaments
      const rn = 1 - Math.abs(2 * fbm(u * 2.2 + 2.4 * wx, v * 2.2 + 2.4 * wy, S + 16, 4) - 1);
      // large patches: much of the sky stays empty
      const mask = smooth(m0, m0 + 0.3, maskN(u, v));
      const d = Math.min(1, (smooth(n0, n0 + 0.36, n) * 0.8 + rn ** 10 * 0.45) * mask);
      density[y * W + x] = d;
      const t = BAYER[(y & 3) * 4 + (x & 3)];
      // colour region: blend factor between two ramps, dithered
      const hue = fbm(u * 0.6 - 20, v * 0.6 + 33, S + 15, 2) * hueScale + hueShift; // ~0..2
      const hi = Math.max(0, Math.min(1, Math.floor(hue)));
      const ramp = RAMPS[hue - hi > t ? hi + 1 : hi];
      const lvl = Math.max(0, Math.min(ramp.length - 1, Math.floor(d * (ramp.length - 0.6) + (t - 0.5) * 0.9)));
      let c = ramp[lvl];

      // dark clouds: warped, lumpy lanes, more likely where the nebula glows
      const dmask = smooth(k0, k0 + 0.22, darkN(u, v)) * (0.45 + 0.55 * mask);
      const e = smooth(l0, l0 + 0.18, laneN(u, v)) * dmask;
      dark[y * W + x] = e;
      if (e > 0.02) {
        // dithered steps of opacity keep the pixel-art look
        const q = Math.min(1, Math.floor(e * 4 + t) / 4);
        // warm glow at the edge, only where glowing gas is behind the dust
        const rim = Math.max(0, 1 - Math.abs(e - 0.3) / 0.22) * 0.55 * smooth(0.05, 0.4, d);
        c = [
          c[0] * (1 - q) + DUST[0] * q + (RIM[0] - c[0]) * rim * (1 - q),
          c[1] * (1 - q) + DUST[1] * q + (RIM[1] - c[1]) * rim * (1 - q),
          c[2] * (1 - q) + DUST[2] * q + (RIM[2] - c[2]) * rim * (1 - q),
        ];
      }
      const i = (y * W + x) * 4;
      data[i] = c[0];
      data[i + 1] = c[1];
      data[i + 2] = c[2];
      data[i + 3] = 255;
    }
  return { density, dark };
}

/** A star colour pulled toward deep orange-red by dust extinction `e` (0..1). */
function redden(hex: string, e: number): string {
  const k = Math.min(1, e * 1.7);
  const n = parseInt(hex.slice(1), 16);
  const r = (n >> 16) & 255,
    g = (n >> 8) & 255,
    b = n & 255;
  const mix = (a: number, z: number) => Math.round(a + (z - a) * k);
  return `rgb(${mix(r, 255)}, ${mix(g, 110)}, ${mix(b, 60)})`;
}

export interface SkyOptions {
  parallax: boolean;
  /** twinkling and shooting stars */
  motion: boolean;
}
const OPTS_KEY = "sme.sky";
export function loadSkyOptions(): SkyOptions {
  try {
    return { parallax: true, motion: true, ...JSON.parse(localStorage.getItem(OPTS_KEY) ?? "{}") };
  } catch {
    return { parallax: true, motion: true };
  }
}
let skyOpts = loadSkyOptions();
/** Changes the background options now and remembers them for the next launch. */
export function setSkyOptions(o: SkyOptions) {
  skyOpts = { ...o };
  try {
    localStorage.setItem(OPTS_KEY, JSON.stringify(skyOpts));
  } catch {
    /* storage unavailable: still applies for this session */
  }
}

const STAR_COLORS = ["#ffffff", "#cfd8ff", "#ffe9a8", "#a8c4ff"];

interface Twinkler {
  x: number;
  y: number;
  color: string;
  alpha: number;
  amp: number;
  speed: number;
  phase: number;
  sparkle: boolean;
}

interface Layer {
  canvas: HTMLCanvasElement;
  ctx: CanvasRenderingContext2D;
  twinklers: Twinkler[];
  shift: string;
}

export function startSky(root: HTMLElement) {
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)");
  const still = () => reduce.matches || !skyOpts.motion;
  // a fresh sky each launch; `#seed=123` in the URL pins one (for testing)
  const pinned = /seed=(\d+)/.exec(location.hash);
  const seed = pinned ? Number(pinned[1]) : (Math.random() * 2 ** 31) >>> 0;
  let layers: Layer[] = [];
  let P = 3; // device pixels per art pixel
  let dpr = 1;
  let aw = 0,
    ah = 0; // viewport size in art pixels
  let target = { x: 0, y: 0 },
    cur = { x: 0, y: 0 };
  let lastTwinkle = 0;
  // the first one comes after 15–45 s; `#shoot` in the URL fires one right away (for testing)
  let nextShot = performance.now() + (location.hash.includes("shoot") ? 300 : 15000 + Math.random() * 30000);
  let shot: { x: number; y: number; dx: number; dy: number; start: number; dur: number; len: number } | null = null;
  let fx: { canvas: HTMLCanvasElement; ctx: CanvasRenderingContext2D; dirty: [number, number, number, number] | null } | null = null;

  function makeCanvas(wArt: number, hArt: number, offsetArt: number): [HTMLCanvasElement, CanvasRenderingContext2D] {
    const c = document.createElement("canvas");
    c.width = wArt * P;
    c.height = hArt * P;
    c.style.width = `${c.width / dpr}px`;
    c.style.height = `${c.height / dpr}px`;
    c.style.left = c.style.top = `${(-offsetArt * P) / dpr}px`;
    const ctx = c.getContext("2d")!;
    ctx.imageSmoothingEnabled = false;
    root.appendChild(c);
    return [c, ctx];
  }

  function build() {
    root.replaceChildren();
    fx = null;
    dpr = window.devicePixelRatio || 1;
    P = Math.max(1, Math.round(ART * dpr));
    aw = Math.ceil((window.innerWidth * dpr) / P);
    ah = Math.ceil((window.innerHeight * dpr) / P);
    const W = aw + 2 * MARGIN,
      H = ah + 2 * MARGIN;
    const r = rng(seed ^ 0x5bd1e995);
    layers = [0, 1, 2].map(() => {
      const [canvas, ctx] = makeCanvas(W, H, MARGIN);
      return { canvas, ctx, twinklers: [], shift: "" };
    });

    // ── nebula, rendered at art resolution and scaled up by an integer factor ──
    const small = document.createElement("canvas");
    small.width = W;
    small.height = H;
    const sctx = small.getContext("2d")!;
    const img = sctx.createImageData(W, H);
    const { density, dark } = paintNebula(img.data, W, H, seed);
    sctx.putImageData(img, 0, 0);
    layers[0].ctx.drawImage(small, 0, 0, W * P, H * P);

    // ── stars: far (static, on the nebula), mid and near (some twinkle) ──
    const taken = new Set<number>();
    const free = (x: number, y: number, rad: number) => {
      for (let dy = -rad; dy <= rad; dy++) for (let dx = -rad; dx <= rad; dx++) if (taken.has((y + dy) * W + x + dx)) return false;
      return true;
    };
    const total = Math.floor((W * H) / 170);
    const plan: [number, number, number, number][] = [
      // layer, count, min alpha, alpha range
      [0, Math.floor(total * 0.55), 0.2, 0.45],
      [1, Math.floor(total * 0.32), 0.35, 0.55],
      [2, Math.floor(total * 0.13), 0.6, 0.4],
    ];
    for (const [li, count, a0, ar] of plan) {
      const L = layers[li];
      for (let i = 0; i < count; i++) {
        const x = Math.floor(r() * W),
          y = Math.floor(r() * H);
        if (!free(x, y, 0)) continue;
        // more stars inside the glowing clouds, fewer in empty space
        if (r() > 0.45 + 1.4 * density[y * W + x]) continue;
        // dark clouds are in front of the far and mid stars: swallow or redden them
        const e = li < 2 ? dark[y * W + x] : 0;
        if (e > (li === 0 ? 0.5 : 0.7)) continue;
        taken.add(y * W + x);
        let color = STAR_COLORS[Math.floor(r() * STAR_COLORS.length)];
        let alpha = a0 + r() * ar;
        if (e > 0.03) {
          color = redden(color, e);
          alpha *= 1 - e * 0.8;
        }
        if (li > 0 && r() < 0.35) {
          L.twinklers.push({ x, y, color, alpha, amp: 0.3 + r() * 0.45, speed: 0.4 + r() * 1.4, phase: r() * Math.PI * 2, sparkle: false });
        } else {
          L.ctx.globalAlpha = alpha;
          L.ctx.fillStyle = color;
          L.ctx.fillRect(x * P, y * P, P, P);
        }
      }
    }
    // bright sparkles (a pixel cross) on the near layer
    const near = layers[2];
    for (let i = 0; i < Math.floor(total / 40) + 3; i++) {
      const x = 1 + Math.floor(r() * (W - 2)),
        y = 1 + Math.floor(r() * (H - 2));
      if (!free(x, y, 1)) continue;
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) taken.add((y + dy) * W + x + dx);
      near.twinklers.push({ x, y, color: "#ffffff", alpha: 1, amp: 0.35, speed: 0.3 + r() * 0.8, phase: r() * Math.PI * 2, sparkle: true });
    }
    for (const L of layers) L.ctx.globalAlpha = 1;
    lastTwinkle = 0;
    twinkle(performance.now(), true);
    applyParallax(true);
  }

  function drawTwinkler(ctx: CanvasRenderingContext2D, s: Twinkler, k: number) {
    // k in [0,1]: current brightness factor
    if (s.sparkle) {
      ctx.clearRect((s.x - 1) * P, (s.y - 1) * P, 3 * P, 3 * P);
      ctx.globalAlpha = 1;
      ctx.fillStyle = s.color;
      ctx.fillRect(s.x * P, s.y * P, P, P);
      ctx.globalAlpha = 0.25 + 0.5 * k;
      ctx.fillStyle = "#c8d7ff";
      ctx.fillRect((s.x - 1) * P, s.y * P, P, P);
      ctx.fillRect((s.x + 1) * P, s.y * P, P, P);
      ctx.fillRect(s.x * P, (s.y - 1) * P, P, P);
      ctx.fillRect(s.x * P, (s.y + 1) * P, P, P);
    } else {
      ctx.clearRect(s.x * P, s.y * P, P, P);
      ctx.globalAlpha = s.alpha * k;
      ctx.fillStyle = s.color;
      ctx.fillRect(s.x * P, s.y * P, P, P);
    }
  }

  function twinkle(now: number, force = false) {
    if (!force && now - lastTwinkle < TWINKLE_MS) return;
    lastTwinkle = now;
    const t = now / 1000;
    const calm = still();
    for (const L of layers) {
      for (const s of L.twinklers) {
        // smooth, slightly irregular scintillation: two sines at unrelated rates
        const w = calm ? 0.5 : 0.5 + 0.35 * Math.sin(t * s.speed + s.phase) + 0.15 * Math.sin(t * s.speed * 2.7 + s.phase * 1.3);
        drawTwinkler(L.ctx, s, 1 - s.amp * Math.max(0, Math.min(1, w)));
      }
      L.ctx.globalAlpha = 1;
    }
  }

  function applyParallax(force = false) {
    layers.forEach((L, i) => {
      // whole device pixels only, so the layer stays pixel-aligned (sharp)
      const dx = Math.round(-cur.x * PARALLAX[i] * P),
        dy = Math.round(-cur.y * PARALLAX[i] * P);
      const s = `translate(${dx / dpr}px, ${dy / dpr}px)`;
      if (force || s !== L.shift) {
        L.shift = s;
        L.canvas.style.transform = s;
      }
    });
  }

  function shootingStar(now: number) {
    if (!shot) {
      if (now < nextShot || still()) return;
      const dir = Math.random() < 0.5 ? 1 : -1; // travelling right or left
      const angle = 0.3 + Math.random() * 0.35; // radians below horizontal
      const speed = 0.14 + Math.random() * 0.08; // art pixels per ms
      shot = {
        x: dir > 0 ? Math.random() * aw * 0.6 : aw * 0.4 + Math.random() * aw * 0.6,
        y: Math.random() * ah * 0.45,
        dx: Math.cos(angle) * speed * dir,
        dy: Math.sin(angle) * speed,
        start: now,
        dur: 600 + Math.random() * 500,
        len: 10 + Math.floor(Math.random() * 8),
      };
      const [canvas, ctx] = makeCanvas(aw, ah, 0);
      fx = { canvas, ctx, dirty: null };
    }
    const f = fx!;
    if (f.dirty) f.ctx.clearRect(...f.dirty);
    const e = now - shot.start;
    if (e > shot.dur) {
      f.canvas.remove();
      fx = null;
      shot = null;
      nextShot = now + 25000 + Math.random() * 45000;
      return;
    }
    const fade = Math.min(1, e / 120, (shot.dur - e) / 250);
    const hx = shot.x + shot.dx * e,
      hy = shot.y + shot.dy * e;
    const nx = -shot.dx / Math.hypot(shot.dx, shot.dy),
      ny = -shot.dy / Math.hypot(shot.dx, shot.dy);
    let x0 = Infinity,
      y0 = Infinity,
      x1 = -Infinity,
      y1 = -Infinity;
    const seen = new Set<number>();
    for (let i = 0; i < shot.len; i++) {
      const px = Math.round(hx + nx * i),
        py = Math.round(hy + ny * i);
      if (seen.has(py * 100000 + px)) continue;
      seen.add(py * 100000 + px);
      const a = fade * (1 - i / shot.len) ** 1.5;
      f.ctx.globalAlpha = a;
      f.ctx.fillStyle = i < 2 ? "#ffffff" : i < 5 ? "#e4ecff" : "#9fb8ff";
      f.ctx.fillRect(px * P, py * P, P, P);
      x0 = Math.min(x0, px);
      y0 = Math.min(y0, py);
      x1 = Math.max(x1, px);
      y1 = Math.max(y1, py);
    }
    f.ctx.globalAlpha = 1;
    f.dirty = [x0 * P, y0 * P, (x1 - x0 + 1) * P, (y1 - y0 + 1) * P];
  }

  function frame(now: number) {
    const goal = reduce.matches || !skyOpts.parallax ? { x: 0, y: 0 } : target;
    if (Math.abs(goal.x - cur.x) + Math.abs(goal.y - cur.y) > 1e-4) {
      cur.x += (goal.x - cur.x) * 0.04;
      cur.y += (goal.y - cur.y) * 0.04;
      applyParallax();
    }
    twinkle(now);
    shootingStar(now);
    requestAnimationFrame(frame);
  }

  window.addEventListener("pointermove", (e) => {
    target = { x: (e.clientX / window.innerWidth) * 2 - 1, y: (e.clientY / window.innerHeight) * 2 - 1 };
  });
  let t: number | undefined;
  window.addEventListener("resize", () => {
    clearTimeout(t);
    t = window.setTimeout(build, 150);
  });
  build();
  requestAnimationFrame(frame);
}
