#!/usr/bin/env python3
"""Renders the pixel hut from src/lib/logo.json into src-tauri/icons/icon-src.png (a night-sky
tile with a gold frame); then run `npx tauri icon src-tauri/icons/icon-src.png`."""
import json, os, random
from PIL import Image

root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
logo = json.load(open(os.path.join(root, "src/lib/logo.json")))
pal = {k: tuple(int(v[i:i + 2], 16) for i in (1, 3, 5)) + (255,) for k, v in logo["palette"].items()}
N = 22  # tile size in art pixels
img = Image.new("RGBA", (N, N), (0, 0, 0, 0))
px = img.load()
for y in range(N):
    for x in range(N):
        edge = min(x, y, N - 1 - x, N - 1 - y)
        corner = (x in (0, N - 1)) and (y in (0, N - 1))
        if corner:
            continue
        if edge == 0:
            px[x, y] = (27, 16, 9, 255)
        elif edge == 1:
            px[x, y] = (232, 176, 40, 255) if (x + y) % 2 == 0 or edge else (0, 0, 0, 255)
        else:
            t = y / N
            px[x, y] = (int(18 + 20 * t), int(20 + 18 * t), int(52 + 40 * t), 255)
random.seed(7)
for x, y in [(4, 4), (8, 3), (17, 4), (5, 9), (17, 10), (12, 4)]:
    px[x, y] = (255, 255, 230, 255)
ox, oy = 3, 4
for y, row in enumerate(logo["art"]):
    for x, ch in enumerate(row):
        if ch != ".":
            px[ox + x, oy + y] = pal[ch]
# ground continues to the frame edges
for x in range(2, N - 2):
    for y in range(oy + 14, N - 2):
        if px[x, y][:3] not in [pal["G"][:3], pal["g"][:3]]:
            px[x, y] = pal["g"] if y > oy + 14 else pal["G"]
out = img.resize((N * 46, N * 46), Image.NEAREST)
canvas = Image.new("RGBA", (1024, 1024), (0, 0, 0, 0))
canvas.paste(out, ((1024 - out.width) // 2, (1024 - out.height) // 2))
canvas.save(os.path.join(root, "src-tauri/icons/icon-src.png"))
print("wrote src-tauri/icons/icon-src.png")
