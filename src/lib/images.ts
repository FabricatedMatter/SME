// Image cache for game sprites served over the sme:// protocol, with optional hue shifting
// (Starbound's `?hueshift=` directive, applied in HSV like the game).

import { assetUrl } from "./api";

type Img = ImageBitmap | HTMLCanvasElement;

const cache = new Map<string, Img | null>();
const pending = new Map<string, Promise<Img | null>>();
const listeners = new Set<() => void>();
let notifyQueued = false;

/** Called (batched) whenever new images finish loading, so views can redraw. */
export function onImagesLoaded(fn: () => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

function notify() {
  if (notifyQueued) return;
  notifyQueued = true;
  requestAnimationFrame(() => {
    notifyQueued = false;
    listeners.forEach((f) => f());
  });
}

export function loadImage(ref: string): Promise<Img | null> {
  if (cache.has(ref)) return Promise.resolve(cache.get(ref)!);
  let p = pending.get(ref);
  if (!p) {
    p = (async () => {
      try {
        const r = await fetch(assetUrl(ref));
        if (!r.ok) throw new Error(String(r.status));
        const bmp = await createImageBitmap(await r.blob());
        cache.set(ref, bmp);
        return bmp;
      } catch {
        cache.set(ref, null);
        return null;
      } finally {
        pending.delete(ref);
        notify();
      }
    })();
    pending.set(ref, p);
  }
  return p;
}

/** Returns the image if loaded (else starts loading and returns undefined; null = missing). */
export function getImage(ref: string): Img | null | undefined {
  if (cache.has(ref)) return cache.get(ref);
  void loadImage(ref);
  return undefined;
}

function rgbToHsv(r: number, g: number, b: number): [number, number, number] {
  const max = Math.max(r, g, b),
    min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  if (d !== 0) {
    if (max === r) h = ((g - b) / d) % 6;
    else if (max === g) h = (b - r) / d + 2;
    else h = (r - g) / d + 4;
    h /= 6;
    if (h < 0) h += 1;
  }
  return [h, max === 0 ? 0 : d / max, max];
}

function hsvToRgb(h: number, s: number, v: number): [number, number, number] {
  const i = Math.floor(h * 6);
  const f = h * 6 - i;
  const p = v * (1 - s),
    q = v * (1 - f * s),
    t = v * (1 - (1 - f) * s);
  switch (((i % 6) + 6) % 6) {
    case 0:
      return [v, t, p];
    case 1:
      return [q, v, p];
    case 2:
      return [p, v, t];
    case 3:
      return [p, q, v];
    case 4:
      return [t, p, v];
    default:
      return [v, p, q];
  }
}

/** Hue-shifted copy of an image (degrees), cached. */
export function getHued(ref: string, deg: number): Img | null | undefined {
  if (!deg) return getImage(ref);
  const key = `${ref}?hue=${deg}`;
  if (cache.has(key)) return cache.get(key);
  const base = getImage(ref);
  if (!base) return base;
  const c = document.createElement("canvas");
  c.width = base.width;
  c.height = base.height;
  const ctx = c.getContext("2d")!;
  ctx.drawImage(base, 0, 0);
  const img = ctx.getImageData(0, 0, c.width, c.height);
  const d = img.data;
  const shift = deg / 360;
  for (let i = 0; i < d.length; i += 4) {
    if (d[i + 3] === 0) continue;
    const [h, s, v] = rgbToHsv(d[i] / 255, d[i + 1] / 255, d[i + 2] / 255);
    const [r, g, b] = hsvToRgb((h + shift) % 1, s, v);
    d[i] = r * 255;
    d[i + 1] = g * 255;
    d[i + 2] = b * 255;
  }
  ctx.putImageData(img, 0, 0);
  cache.set(key, c);
  return c;
}
