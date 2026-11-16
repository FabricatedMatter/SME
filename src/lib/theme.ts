import { invoke } from "@tauri-apps/api/core";
import "@fontsource/pixelify-sans/400.css";

/** Load the Starbound UI font from the user's own install; falls back to Pixelify Sans. */
export async function loadGameFont(): Promise<boolean> {
  try {
    const buf = await invoke<ArrayBuffer>("ui_font");
    const face = new FontFace("Hobo", buf);
    await face.load();
    document.fonts.add(face);
    document.documentElement.classList.add("game-font");
    return true;
  } catch {
    return false;
  }
}

// Small deterministic PRNG so the sky doesn't change between launches.
function rng(seed: number) {
  return () => {
    seed = (seed * 1664525 + 1013904223) >>> 0;
    return seed / 4294967296;
  };
}

const BAYER = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5].map((v) => (v + 0.5) / 16);

/**
 * Paints a pixel-art starfield with a dithered nebula into a fixed background canvas.
 * One canvas pixel = 3 screen pixels.
 */
export function paintSky(canvas: HTMLCanvasElement) {
  const PX = 3;
  const w = Math.ceil(window.innerWidth / PX);
  const h = Math.ceil(window.innerHeight / PX);
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d")!;
  const img = ctx.createImageData(w, h);
  const r = rng(1701);

  // value noise on a coarse grid, bilinear
  const G = 48;
  const gw = Math.ceil(w / G) + 2;
  const gh = Math.ceil(h / G) + 2;
  const grid = Array.from({ length: gw * gh }, () => r());
  const noise = (x: number, y: number) => {
    const gx = x / G,
      gy = y / G;
    const x0 = Math.floor(gx),
      y0 = Math.floor(gy);
    const fx = gx - x0,
      fy = gy - y0;
    const s = (t: number) => t * t * (3 - 2 * t);
    const at = (gx: number, gy: number) => grid[(((gy % gh) + gh) % gh) * gw + (((gx % gw) + gw) % gw)];
    const a = at(x0, y0),
      b = at(x0 + 1, y0);
    const c = at(x0, y0 + 1),
      d = at(x0 + 1, y0 + 1);
    return a + (b - a) * s(fx) + (c - a) * s(fy) + (a - b - c + d) * s(fx) * s(fy);
  };
  const levels = [
    [6, 7, 18],
    [12, 13, 38],
    [24, 19, 60],
    [44, 26, 78],
  ];
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      // nebula is denser toward the lower right
      const n = noise(x, y) * 0.65 + noise(x * 2.3 + 91, y * 2.3 + 17) * 0.35;
      const bias = (x / w) * 0.25 + (y / h) * 0.35;
      const v = Math.max(0, Math.min(0.999, (n - 0.35 + bias) * 1.6));
      const t = BAYER[(y % 4) * 4 + (x % 4)];
      const li = Math.min(levels.length - 1, Math.floor(v * levels.length + (t - 0.5)));
      const [cr, cg, cb] = levels[Math.max(0, li)];
      const i = (y * w + x) * 4;
      img.data[i] = cr;
      img.data[i + 1] = cg;
      img.data[i + 2] = cb;
      img.data[i + 3] = 255;
    }
  }
  ctx.putImageData(img, 0, 0);

  const starColors = ["#ffffff", "#cfd8ff", "#ffe9a8", "#a8c4ff"];
  const count = Math.floor((w * h) / 260);
  for (let i = 0; i < count; i++) {
    const x = Math.floor(r() * w),
      y = Math.floor(r() * h);
    const c = starColors[Math.floor(r() * starColors.length)];
    ctx.globalAlpha = 0.25 + r() * 0.75;
    ctx.fillStyle = c;
    ctx.fillRect(x, y, 1, 1);
  }
  // a few bright twinkle crosses
  ctx.globalAlpha = 1;
  for (let i = 0; i < Math.floor(count / 40) + 3; i++) {
    const x = Math.floor(r() * w),
      y = Math.floor(r() * h);
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(x, y, 1, 1);
    ctx.fillStyle = "rgba(200,215,255,0.55)";
    ctx.fillRect(x - 1, y, 1, 1);
    ctx.fillRect(x + 1, y, 1, 1);
    ctx.fillRect(x, y - 1, 1, 1);
    ctx.fillRect(x, y + 1, 1, 1);
  }
}
