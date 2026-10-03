#!/usr/bin/env python3
"""Pack the assets the Rust code references into web-friendly files under assets/.

Inputs (produced by tools/extract.sh from a COPY of the game data):
  extract/raw/{sprites.json,masks.json,fonts.json,sounds.json,object_depths.json,objects.json}
  extract/raw/{sprites,fonts,sounds}/
  extract/mus/  (copied music files)
Only sprites/sounds whose names appear in src/ (plus EXTRA_* below) are packed.
"""
import json, os, re, shutil, sys
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RAW = os.path.join(ROOT, "extract", "raw")
OUT = os.path.join(ROOT, "assets")
PAGE = 2048

EXTRA_SPRITES = set()
EXTRA_SOUNDS = set()
MUSIC = ["knight.ogg"]


def referenced(prefix):
    names = set()
    pat = re.compile(r'"(' + prefix + r'[A-Za-z0-9_]+)"')
    for d, _, files in os.walk(os.path.join(ROOT, "src")):
        for f in files:
            if f.endswith(".rs") and f != "objdata.rs":
                names |= set(pat.findall(open(os.path.join(d, f), encoding="utf-8").read()))
    return names


def main():
    sprites = json.load(open(os.path.join(RAW, "sprites.json")))
    masks = json.load(open(os.path.join(RAW, "masks.json")))
    fonts = json.load(open(os.path.join(RAW, "fonts.json")))
    sounds = json.load(open(os.path.join(RAW, "sounds.json")))
    objects = json.load(open(os.path.join(RAW, "objects.json")))
    depths = json.load(open(os.path.join(RAW, "object_depths.json")))
    nslice = json.load(open(os.path.join(RAW, "nineslice.json")))

    want_objs = referenced("obj_")
    want_spr = (referenced("spr_") | EXTRA_SPRITES)
    for o in want_objs:
        info = objects.get(o)
        if info:
            for k in ("sprite", "mask"):
                if info[k]:
                    want_spr.add(info[k])
    missing = sorted(s for s in want_spr if s not in sprites)
    if missing:
        print("warning: unknown sprites:", missing, file=sys.stderr)
    want_spr = sorted(s for s in want_spr if s in sprites)

    # load + trim frames
    frames = []  # (name, idx, img, offx, offy)
    for name in want_spr:
        for i in range(sprites[name]["frames"]):
            p = os.path.join(RAW, "sprites", f"{name}_{i}.png")
            if not os.path.exists(p):
                frames.append((name, i, None, 0, 0))
                continue
            im = Image.open(p).convert("RGBA")
            bb = im.getbbox()
            if bb is None:
                frames.append((name, i, None, 0, 0))
                continue
            frames.append((name, i, im.crop(bb), bb[0], bb[1]))

    # shelf pack, tallest first
    order = sorted(range(len(frames)), key=lambda k: -(frames[k][2].height if frames[k][2] else 0))
    pages = [Image.new("RGBA", (PAGE, PAGE), (0, 0, 0, 0))]
    placed = {}
    x = y = shelf = 0
    for k in order:
        name, i, im, ox, oy = frames[k]
        if im is None:
            placed[k] = None
            continue
        w, h = im.width + 2, im.height + 2
        if w > PAGE or h > PAGE:
            # oversized: give it its own page
            big = Image.new("RGBA", (w, h), (0, 0, 0, 0))
            big.paste(im, (1, 1))
            pages.append(big)
            placed[k] = (len(pages) - 1, 1, 1)
            pages.append(Image.new("RGBA", (PAGE, PAGE), (0, 0, 0, 0)))
            x = y = shelf = 0
            continue
        if x + w > PAGE:
            x, y, shelf = 0, y + shelf, 0
        if y + h > PAGE:
            pages.append(Image.new("RGBA", (PAGE, PAGE), (0, 0, 0, 0)))
            x = y = shelf = 0
        pages[-1].paste(im, (x + 1, y + 1))
        placed[k] = (len(pages) - 1, x + 1, y + 1)
        x += w
        shelf = max(shelf, h)

    if os.path.isdir(OUT):
        shutil.rmtree(OUT)
    os.makedirs(os.path.join(OUT, "snd"))
    page_files = []
    for n, pg in enumerate(pages):
        bb = pg.getbbox()
        if bb is None:
            continue
        f = f"atlas{n}.png"
        pg.crop((0, 0, bb[2], bb[3])).save(os.path.join(OUT, f), optimize=True)
        page_files.append((n, f))
    remap = {n: i for i, (n, _) in enumerate(page_files)}

    out_spr = {}
    for k, (name, i, im, ox, oy) in enumerate(frames):
        s = sprites[name]
        e = out_spr.setdefault(name, {
            "w": s["w"], "h": s["h"], "ox": s["ox"], "oy": s["oy"],
            "bl": s["bl"], "br": s["br"], "bt": s["bt"], "bb": s["bb"],
            "kind": s["sepmasks"], "speed": s["speed"], "speedtype": s["speedtype"],
            "frames": [None] * s["frames"],
        })
        if name in masks and s["sepmasks"] == 1:
            e["mask"] = masks[name]
        if name in nslice:
            e["ns"] = nslice[name]
        p = placed.get(k)
        e["frames"][i] = None if p is None else [remap[p[0]], p[1], p[2], im.width, im.height, ox, oy]

    out_fonts = {}
    os.makedirs(os.path.join(OUT, "fnt"), exist_ok=True)
    for name, f in fonts.items():
        if name.startswith("fnt_ja"):
            continue
        shutil.copy(os.path.join(RAW, "fonts", name + ".png"), os.path.join(OUT, "fnt", name + ".png"))
        out_fonts[name] = f

    want_snd = (referenced("snd_") | EXTRA_SOUNDS)
    out_snd = {}
    for name in sorted(want_snd):
        s = sounds.get(name)
        if not s or not s["file"]:
            # streamed/external file in the chapter dir
            if s and s.get("ext"):
                ext = os.path.join(ROOT, "extract", "ext", s["ext"])
                if os.path.exists(ext):
                    shutil.copy(ext, os.path.join(OUT, "snd", s["ext"]))
                    out_snd[name] = {"file": "snd/" + s["ext"], "vol": s["vol"]}
                    continue
            print("warning: sound missing:", name, file=sys.stderr)
            continue
        shutil.copy(os.path.join(RAW, "sounds", s["file"]), os.path.join(OUT, "snd", s["file"]))
        out_snd[name] = {"file": "snd/" + s["file"], "vol": s["vol"]}
    os.makedirs(os.path.join(OUT, "mus"), exist_ok=True)
    for m in MUSIC:
        shutil.copy(os.path.join(ROOT, "extract", "mus", m), os.path.join(OUT, "mus", m))

    obj_depth = {o: depths.get(o, 0.0) for o in objects}
    obj_info = {o: {"sprite": v["sprite"], "mask": v["mask"], "parent": v["parent"], "visible": v["visible"], "depth": obj_depth[o]}
                for o, v in objects.items() if o in want_objs or any(o == objects.get(w, {}).get("parent") for w in want_objs)}
    json.dump({"pages": [f for _, f in page_files], "sprites": out_spr, "fonts": out_fonts, "sounds": out_snd},
              open(os.path.join(OUT, "assets.json"), "w"), separators=(",", ":"))
    print(f"packed {len(out_spr)} sprites / {len(frames)} frames into {len(page_files)} pages, {len(out_snd)} sounds")


if __name__ == "__main__":
    main()
