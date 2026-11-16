#!/usr/bin/env python3
"""Generates the SME example mod (a few small microdungeons) as plain Tiled JSON maps.

usage: make_examples.py <extracted assets dir> <starbound install> <output mod dir>
Maps are drawn as ASCII art: each character maps to a front/back brush via a legend.
"""
import json, os, shutil, struct, sys

ASSETS, INSTALL, OUT = sys.argv[1:4]
TS = os.path.join(ASSETS, "tilesets/packed")

TILESETS = [
    "materials.json", "supports.json", "miscellaneous.json", "liquids.json",
    "objects-by-category/storage.json", "objects-by-category/light.json",
    "objects-by-category/door.json", "objects-by-category/furniture.json",
    "objects-by-category/crafting.json", "objects-by-category/wire.json",
    "objects-by-category/decorative.json",
]
tsdata = {t: json.load(open(os.path.join(TS, t))) for t in TILESETS}


def find(ts, key, value):
    for k, v in tsdata[ts]["tileproperties"].items():
        if v.get(key) == value and v.get("invalid") != "true":
            return int(k)
    raise KeyError(f"{value} not in {ts}")


def tilecount(ts):
    d = tsdata[ts]
    return d.get("tilecount") or (max(int(k) for k in d["tileproperties"]) + 1)


def png_size(name):
    with open(os.path.join(INSTALL, "tiled/packed/objects", name + ".png"), "rb") as f:
        h = f.read(24)
    return struct.unpack(">II", h[16:24])


MISC = {"air": 0, "pink": 1, "playerstart": 3, "anchor_air": 4, "anchor_solid": 5,
        "biomeitems": 6, "biometree": 7}


class Map:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.layers = {"back": [None] * (w * h), "front": [None] * (w * h)}
        self.groups = {g: [] for g in ["mods", "objects", "wiring - lights & guns", "monsters & npcs", "anchors etc", "items"]}
        self.next_id = 1

    # brushes are (tileset, index)
    def set(self, layer, x, y, brush):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.layers[layer][y * self.w + x] = brush

    def fill(self, layer, x0, y0, x1, y1, brush):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.set(layer, x, y, brush)

    def art(self, rows, legend, ox=0, oy=0):
        """legend: char -> {"front": brush, "back": brush}; ' ' means leave untouched."""
        for y, row in enumerate(rows):
            for x, ch in enumerate(row):
                for layer, b in legend.get(ch, {}).items():
                    self.set(layer, ox + x, oy + y, b)

    def _id(self):
        self.next_id += 1
        return self.next_id - 1

    def obj(self, name, tx, ty, params=None, flip=False, group="objects"):
        """Places object `name` with its anchor tile at editor column tx / row ty (rows top-down)."""
        for ts in TILESETS:
            if not ts.startswith("objects"):
                continue
            for k, v in tsdata[ts]["tileproperties"].items():
                if v.get("object") == name:
                    ix = int(v.get("imagePositionX", 0)) // 8
                    iy = int(v.get("imagePositionY", 0)) // 8
                    w, h = png_size(name)
                    o = {"id": self._id(), "gid": ("o", ts, int(k), flip), "x": (tx + ix) * 8, "y": (ty + 1 - iy) * 8,
                         "width": w, "height": h, "name": "", "type": "", "rotation": 0, "visible": True, "properties": {}}
                    if params:
                        o["properties"]["parameters"] = json.dumps(params)
                    self.groups[group].append(o)
                    return
        raise KeyError(name)

    def marker(self, kind, tx, ty):
        self.groups["anchors etc"].append({"id": self._id(), "gid": ("o", "miscellaneous.json", MISC[kind], False),
                                           "x": tx * 8, "y": (ty + 1) * 8, "width": 8, "height": 8, "name": "", "type": "",
                                           "rotation": 0, "visible": True, "properties": {}})

    def area(self, group, tx, ty, w, h, props):
        self.groups[group].append({"id": self._id(), "x": tx * 8, "y": ty * 8, "width": w * 8, "height": h * 8,
                                   "name": "", "type": "", "rotation": 0, "visible": True, "properties": dict(props)})

    def wire(self, a, b):
        """a, b: (col, row) node tiles."""
        x0, y0 = a[0] * 8 + 4, a[1] * 8 + 4
        self.groups["wiring - lights & guns"].append({
            "id": self._id(), "x": x0, "y": y0, "width": 0, "height": 0, "name": "", "type": "", "rotation": 0,
            "visible": True, "properties": {},
            "polyline": [{"x": 0, "y": 0}, {"x": b[0] * 8 + 4 - x0, "y": b[1] * 8 + 4 - y0}]})

    def save(self, path, depth):
        used = sorted({b[0] for l in self.layers.values() for b in l if b} |
                      {o["gid"][1] for g in self.groups.values() for o in g if "gid" in o},
                      key=TILESETS.index)
        first, tsj, nxt = {}, [], 1
        for ts in used:
            first[ts] = nxt
            tsj.append({"firstgid": nxt, "source": "../" * depth + "tilesets/packed/" + ts})
            nxt += tilecount(ts)
        layers = []
        for name in ["back", "front"]:
            data = [first[b[0]] + b[1] if b else 0 for b in self.layers[name]]
            layers.append({"data": data, "height": self.h, "name": name, "opacity": 0.5 if name == "back" else 1,
                           "type": "tilelayer", "visible": True, "width": self.w, "x": 0, "y": 0})
        for gname, objs in self.groups.items():
            out = []
            for o in objs:
                o = dict(o)
                if "gid" in o:
                    _, ts, idx, flip = o["gid"]
                    o["gid"] = first[ts] + idx + (0x80000000 if flip else 0)
                out.append(o)
            layers.append({"draworder": "topdown", "name": gname, "objects": out, "opacity": 1,
                           "type": "objectgroup", "visible": True, "x": 0, "y": 0})
        doc = {"height": self.h, "width": self.w, "tileheight": 8, "tilewidth": 8, "orientation": "orthogonal",
               "renderorder": "right-down", "version": 1, "nextobjectid": self.next_id, "properties": {},
               "tilesets": tsj, "layers": layers}
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w") as f:
            json.dump(doc, f, indent=1)


def mat(name):
    return ("materials.json", find("materials.json", "material", name))


def plat(name):
    return ("supports.json", find("supports.json", "material", name))


def misc(kind):
    return ("miscellaneous.json", MISC[kind])


def liquid(name):
    return ("liquids.json", find("liquids.json", "liquid", name))


AIR, PINK = misc("air"), misc("pink")


def write_json(rel, value):
    p = os.path.join(OUT, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w") as f:
        json.dump(value, f, indent=2)
        f.write("\n")


def dungeon(name, parts, anchors=None, extra_meta=None):
    meta = {"name": name, "species": "generic", "rules": [], "anchor": anchors or [p for p, _ in parts],
            "gravity": 80, "maxRadius": 1000000, "maxParts": 100, "protected": False}
    meta.update(extra_meta or {})
    write_json(f"dungeons/microdungeons/{name}/{name}.dungeon", {
        "metadata": meta,
        "parts": [{"name": p, "rules": [["maxSpawnCount", [1]]], "def": ["tmx", f"{p}.json"]} for p, _ in parts]})
    for p, m in parts:
        m.save(os.path.join(OUT, f"dungeons/microdungeons/{name}/{p}.json"), 3)


def ground_anchors(m, ground_row, xs):
    for x in xs:
        m.marker("anchor_solid", x, ground_row)
        m.marker("anchor_air", x, ground_row - 1)


# ─────────────────────────── 1. smallhut (two variants) ───────────────────────────
def hut(variant):
    W, H = 26, 18
    m = Map(W, H)
    m.fill("back", 0, 0, W - 1, H - 1, PINK)  # keep whatever background the world has
    wall, roof, floor, backwall = (("darkwood", "rooftiles", "fullwood1", "woodpanelling") if variant == 1
                                   else ("cobblestonebrick", "thatch", "fullwood2", "wallpaper"))
    L = {
        "#": {"front": mat(wall), "back": mat(wall)},
        "R": {"front": mat(roof), "back": mat(roof)},
        "F": {"front": mat(floor), "back": mat(floor)},
        "S": {"front": mat("cobblestone"), "back": mat("cobblestone")},
        "D": {"front": mat("dirt"), "back": mat("dirt")},
        ".": {"front": AIR, "back": mat(backwall)},
        "w": {"front": AIR, "back": mat("glass")},
        "d": {"front": AIR, "back": mat(wall)},  # doorway
        "o": {"front": AIR},
    }
    art = [
        "                          ",
        "                          ",
        "          RRRRRR          ",
        "       RRRRRRRRRRRR       ",
        "    RRRRRRRRRRRRRRRRRR    ",
        "   RRRRRRRRRRRRRRRRRRRR   ",
        "    #................#    ",
        "    #................#    ",
        "    #..ww........ww..#    ",
        "    #..ww........ww..#    ",
        "    d................d    ",
        "    d................d    ",
        "    d................d    ",
        "  SSFFFFFFFFFFFFFFFFFFSS  ",
        "  SSSSSSSSSSSSSSSSSSSSSS  ",
        "   DDDDDDDDDDDDDDDDDDDD   ",
        "                          ",
        "                          ",
    ]
    m.art(art, L)
    m.obj("wooddoor", 4, 12)
    m.obj("wooddoor", 21, 12, flip=True)
    pool = "smeexampleHutTreasure"
    if variant == 1:
        m.obj("woodenchest", 12, 12, params={"treasurePools": [pool]})
        m.obj("woodenbed", 19, 12)
        m.obj("woodenchair", 8, 12)
        m.obj("torch", 12, 8)
    else:
        m.obj("woodenchest", 8, 12, params={"treasurePools": [pool]})
        m.obj("barrel", 18, 12)
        m.obj("torch", 13, 8)
        m.area("monsters & npcs", 12, 12, 1, 1, {"npc": "human", "typeName": "villager"})
    ground_anchors(m, 14, [1, 24])
    return m


dungeon("smallhut", [("smallhut1", hut(1)), ("smallhut2", hut(2))])


# ─────────────────────────── 2. lookout tower (wiring) ───────────────────────────
def tower():
    W, H = 16, 38
    m = Map(W, H)
    m.fill("back", 0, 0, W - 1, H - 1, PINK)
    L = {
        "#": {"front": mat("cobblestonebrick"), "back": mat("cobblestonebrick")},
        "B": {"front": mat("stonetiles"), "back": mat("stonetiles")},
        "S": {"front": mat("cobblestone"), "back": mat("cobblestone")},
        "=": {"front": plat("woodenplatform"), "back": mat("darksmoothstone")},
        ".": {"front": AIR, "back": mat("darksmoothstone")},
        "d": {"front": AIR, "back": mat("cobblestonebrick")},
        "c": {"front": AIR},
    }
    art = [
        "                ",
        "  B B B  B B B  ",
        "  BBBBBBBBBBBB  ",
        "   #........#   ",
        "   #........#   ",
        "   #........#   ",
        "   #........#   ",
        "   #=======.#   ",
        "   #........#   ",
        "   #........#   ",
        "   #........#   ",
        "   #........#   ",
        "   #.=======#   ",
        "   #........#   ",
        "   #........#   ",
        "   #........#   ",
        "   #........#   ",
        "   #=======.#   ",
        "   #........#   ",
        "   #........#   ",
        "   #........#   ",
        "   #........#   ",
        "   #.=======#   ",
        "   #........#   ",
        "   #........#   ",
        "   #........#   ",
        "   #........#   ",
        "   #=======.#   ",
        "   #........#   ",
        "   #........#   ",
        "   #........#   ",
        "   d........d   ",
        "   d........d   ",
        "   d........d   ",
        "  SSSSSSSSSSSS  ",
        "  SSSSSSSSSSSS  ",
        "                ",
        "                ",
    ]
    m.art(art, L)
    m.obj("wooddoor", 3, 33)
    m.obj("wooddoor", 12, 33, flip=True)
    # a wall switch by the door wired to two standing lights
    m.obj("smallwallswitch", 6, 31)
    m.obj("tier3light", 10, 33)
    m.obj("tier3light", 5, 26)
    # smallwallswitch: output node [0,-1] (y up) -> one row below; tier3light: input node [0,0]
    m.wire((6, 32), (10, 33))
    m.wire((6, 32), (5, 26))
    m.obj("woodenchest", 8, 6, params={"treasurePools": ["smeexampleTowerTreasure"]})
    m.obj("torch", 6, 4)
    m.area("monsters & npcs", 7, 16, 1, 1, {"npc": "human", "typeName": "guard"})
    ground_anchors(m, 34, [1, 14])
    return m


dungeon("lookouttower", [("lookouttower1", tower())])


# ─────────────────────────── 3. oasis (liquid + biome markers) ───────────────────────────
def oasis():
    W, H = 32, 14
    m = Map(W, H)
    m.fill("back", 0, 0, W - 1, H - 1, PINK)
    L = {
        "s": {"front": mat("sand"), "back": mat("sand")},
        "S": {"front": mat("sandstoneblock"), "back": mat("sandstoneblock")},
        "~": {"front": liquid("water"), "back": mat("sand")},
        "o": {"front": AIR},
    }
    art = [
        "                                ",
        "                                ",
        "                                ",
        "                                ",
        "  oooooooooooooooooooooooooooo  ",
        "  oooooooooooooooooooooooooooo  ",
        "  oooooooooooooooooooooooooooo  ",
        "  oooooooooooooooooooooooooooo  ",
        "  ssssss~~~~~~~~~~~~~~~~~sssss  ",
        "  sssssss~~~~~~~~~~~~~~~ssssss  ",
        "  SSSSSSSS~~~~~~~~~~~~~SSSSSSS  ",
        "   SSSSSSSSSSSSSSSSSSSSSSSSSS   ",
        "                                ",
        "                                ",
    ]
    m.art(art, L)
    m.marker("biometree", 4, 7)
    m.marker("biometree", 27, 7)
    for x in (13, 18, 22):
        m.marker("biomeitems", x, 7)
    m.obj("campfire", 25, 7)
    m.obj("barrel", 6, 7, params={"treasurePools": ["smeexampleOasisTreasure"]})
    ground_anchors(m, 8, [1, 30])
    return m


dungeon("oasis", [("oasis1", oasis())])


# ─────────────────────────── 4. buried camp (underground) ───────────────────────────
def camp():
    W, H = 30, 16
    m = Map(W, H)
    L = {
        "o": {"front": AIR, "back": PINK},
        "P": {"front": plat("woodenplatform"), "back": PINK},
        "W": {"front": mat("fullwood1"), "back": mat("fullwood1")},
        "G": {"front": mat("gravel"), "back": mat("gravel")},
    }
    art = [
        "                              ",
        "                              ",
        "          oooooooooo          ",
        "      ooooooooooooooooooo     ",
        "    oooooooooooooooooooooooo  ",
        "   oooWooooooooooooooWoooooo  ",
        "  ooooWooooooooooooooWooooooo ",
        "  ooooWooooooooooooooWooooooo ",
        "  ooooPPPPPPPPPPPPPPPPoooooo  ",
        "  ooooWooooooooooooooWoooooo  ",
        "   oooWooooooooooooooWooooo   ",
        "    GGGGGGGGGGGGGGGGGGGGGG    ",
        "     GGGGGGGGGGGGGGGGGGGG     ",
        "                              ",
        "                              ",
        "                              ",
    ]
    m.art(art, L)
    m.obj("campfire", 14, 10)
    m.obj("woodenchest", 9, 7, params={"treasurePools": ["smeexampleCampTreasure"]})
    m.obj("torch", 8, 9)
    m.obj("torch", 19, 9)
    m.obj("woodencrate1", 24, 10)
    m.area("monsters & npcs", 17, 10, 1, 1, {"npc": "human", "typeName": "bandit"})
    # anchors: solid rock under the floor, open air in the cave
    for x in (6, 15, 23):
        m.marker("anchor_solid", x, 12)
    for x in (4, 26):
        m.marker("anchor_air", x, 6)
    return m


dungeon("buriedcamp", [("buriedcamp1", camp())])

# ─────────────────────────── mod files ───────────────────────────
write_json("_metadata", {
    "name": "sme_examples",
    "friendlyName": "SME Example Microdungeons",
    "author": "SME",
    "version": "1.0",
    "description": "Four small example microdungeons made with SME: a hut with two variants, a wired lookout tower, a desert oasis and an underground camp.",
    "priority": 0,
})

write_json("treasure/smeexamples.treasurepools", {
    "smeexampleHutTreasure": [[0, {
        "fill": [{"item": ["money", 30]}],
        "pool": [{"weight": 0.4, "item": ["bandage", 3]}, {"weight": 0.3, "item": ["torch", 5]},
                 {"weight": 0.2, "item": ["bread", 2]}, {"weight": 0.1, "pool": "basicTreasure"}],
        "poolRounds": [[0.5, 1], [0.35, 2], [0.15, 3]]}]],
    "smeexampleTowerTreasure": [[0, {
        "fill": [{"item": ["money", 80]}],
        "pool": [{"weight": 0.6, "item": ["ironbar", 4]}, {"weight": 0.4, "pool": "weapon"}],
        "poolRounds": [[0.6, 1], [0.4, 2]]}]],
    "smeexampleOasisTreasure": [[0, {
        "pool": [{"weight": 0.5, "item": ["cactusjuice", 2]}, {"weight": 0.5, "item": ["bread", 2]}],
        "poolRounds": [[1, 1]]}]],
    "smeexampleCampTreasure": [[0, {
        "fill": [{"item": ["money", 50]}],
        "pool": [{"weight": 0.5, "item": ["copperbar", 5]}, {"weight": 0.3, "item": ["torch", 10]},
                 {"weight": 0.2, "item": ["bandage", 4]}],
        "poolRounds": [[0.5, 2], [0.5, 3]]}]],
})


def spawn(biome, layer, dungeons):
    path = f"biomes/surface/{biome}.biome.patch"  # a surface biome file holds both layers
    ops = []
    p = os.path.join(OUT, path)
    if os.path.exists(p):
        ops = json.load(open(p))
    ops.append({"op": "add", "path": f"/{layer}Placeables/items/-", "value": {
        "mode": "floor", "priority": 1, "variants": 1,
        "distribution": "/biomes/distributions.config:mainBiomeMicrodungeon",
        "type": "microdungeon", "microdungeons": dungeons}})
    write_json(path, ops)


for b in ("forest", "garden"):
    spawn(b, "surface", ["smallhut"])
for b in ("forest", "snow", "tundra"):
    spawn(b, "surface", ["lookouttower"])
spawn("desert", "surface", ["oasis"])
for b in ("forest", "desert", "savannah"):
    spawn(b, "underground", ["buriedcamp"])
print("written to", OUT)
