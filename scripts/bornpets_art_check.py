#!/usr/bin/env python3
"""BornPets v3 art checker: export diff, FAT12 space budget, drift metrics, review files.

Subcommands:
  diff OLD NEW             list changed / removed / added PCX of the five art pets;
                           exit 1 on any changed or removed file, or an added file
                           that is not one of the 50 shipped new-state names.
  budget EXPORT            FAT12 cost (2 KiB clusters) of the 50 new PCX; exit 1
                           unless it fits NEW_PCX_BUDGET (or a file is missing).
  review EXPORT --round N  write the owner-gallery files under target/v3art/review
                           and print the manifest path.

This module is the shared home of STATES / ART_PETS / FRAME / decode_pcx / edge_runs
and the floor names. bornpets_ai_frames.py and bornpets_litter.py import from here;
it never imports either of them (no cycle). The litter generator writes its working
files under LITTER_DIR and write_review only reads them.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import statistics
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from PIL import Image  # noqa: E402

from fix_badge_pcx import rle_decode_line  # noqa: E402

REPO = Path(__file__).resolve().parent.parent
TO_BADGE = REPO / "assets/to-badge"

STATES = ("tipsy", "wasted", "hung_over", "sore", "injured",
          "groggy", "stuffed", "thirsty", "wired", "throwing_up")   # STATES[i] has anim id 0x29 + i
ART_PETS = {  # pet -> (prefix, json5, sheet, sequence prefix)
    "bartholomeus": ("00", "bornpets-bartholomeus.json5", "bornpets-sprites-bartholomeus.png", "bartho"),
    "cat":          ("01", "bornpets-sponsors-cat.json5", "bornpets-sprites-cat.png", "cat"),
    "panda":        ("05", "bornpets-sponsors-panda.json5", "bornpets-sprites-panda.png", "panda"),
    "badger":       ("06", "bornpets-sponsors-badger.json5", "bornpets-sprites-badger.png", "badger"),
    "seastar":      ("07", "bornpets-sponsors-seastar.json5", "bornpets-sprites-seastar.png", "seastar"),
}
FRAME = (152, 75)
CLUSTER_BYTES = 2048
NEW_PCX_BUDGET = 55
REVIEW_DIR = REPO / "target/v3art/review"
LITTER_DIR = REPO / "target/v3art/litter"   # written by bornpets_litter.py (Task A2b)
FIRST_STATE_ANIM = 0x29
FLOOR_ITEMS = ("crumbs", "peel", "can", "sock", "pizza_box")   # plan 7 slot order
FLOOR_STRIP = (152, 10)
SCALE = 4
WHITE_RGB = (255, 255, 255)


# ---------------------------------------------------------------- PCX

def decode_pcx(path) -> tuple[int, int, list[list[int]]]:
    """(w, h, rows) of a 2bpp 1-plane badge PCX; 0 black, 1 red, 2 white, 3 transparent."""
    raw = Path(path).read_bytes()
    if len(raw) < 128 or raw[0] != 0x0A:
        raise ValueError(f"{path}: not a PCX")
    if raw[3] != 2 or raw[65] != 1:
        raise ValueError(f"{path}: need 2bpp, 1 plane")
    w = int.from_bytes(raw[8:10], "little") - int.from_bytes(raw[4:6], "little") + 1
    h = int.from_bytes(raw[10:12], "little") - int.from_bytes(raw[6:8], "little") + 1
    bpl = int.from_bytes(raw[66:68], "little")
    pos, rows = 128, []
    for _ in range(h):
        line, pos = rle_decode_line(raw, pos, bpl)
        px = []
        for byte in line:
            for k in range(4):
                px.append((byte >> (6 - 2 * k)) & 3)
        rows.append(px[:w])
    return w, h, rows


def clusters(nbytes: int) -> int:
    return -(-nbytes // CLUSTER_BYTES)


def new_files() -> list[str]:
    """The 50 shipped new-state names, sorted."""
    return sorted(f"{prefix}{FIRST_STATE_ANIM + i:02X}00.PCX"
                  for prefix, *_ in ART_PETS.values() for i in range(len(STATES)))


def diff_exports(old_dir, new_dir, prefixes=tuple(p for p, *_ in ART_PETS.values())) -> dict:
    def names(d):
        return {p.name for p in Path(d).iterdir() if p.is_file() and p.name.startswith(tuple(prefixes))}
    old, new = names(old_dir), names(new_dir)
    changed = sorted(n for n in old & new
                     if (Path(old_dir) / n).read_bytes() != (Path(new_dir) / n).read_bytes())
    return {"changed": changed, "removed": sorted(old - new), "added": sorted(new - old)}


def budget(export_dir) -> dict:
    files, missing = [], []
    for name in new_files():
        p = Path(export_dir) / name
        if p.is_file():
            n = p.stat().st_size
            files.append((name, n, clusters(n)))
        else:
            missing.append(name)
    total = sum(c for _, _, c in files)
    return {"files": files, "total": total,
            "over": [n for n, b, _ in files if b > CLUSTER_BYTES],
            "missing": missing, "fits": total <= NEW_PCX_BUDGET}


# ---------------------------------------------------------------- drift

def edge_runs(rows: list[list[int]]) -> tuple[int, int]:
    """(median black run inward, % of transparent-to-opaque edges that start black).

    Scans every row both ways and every column both ways; the frame edge counts as
    transparent. Edges whose first pixel is not black stay out of the run width.
    """
    h = len(rows)
    w = len(rows[0]) if h else 0
    lines = [rows[y] for y in range(h)] + [[rows[y][x] for y in range(h)] for x in range(w)]
    runs, edges = [], 0
    for line in lines:
        for seq in (line, line[::-1]):
            prev = 3
            for i, v in enumerate(seq):
                if v != 3 and prev == 3:
                    edges += 1
                    if v == 0:
                        j = i
                        while j < len(seq) and seq[j] == 0:
                            j += 1
                        runs.append(j - i)
                prev = v
    if not edges or not runs:
        return 0, 0
    return statistics.median_low(runs), (100 * len(runs)) // edges


def _pet_part(rows):
    """(top, bottom) rows of the largest 8-connected opaque component; None if empty."""
    h = len(rows)
    w = len(rows[0]) if h else 0
    seen = [[False] * w for _ in range(h)]
    best, best_n = None, 0
    for y0 in range(h):
        for x0 in range(w):
            if rows[y0][x0] == 3 or seen[y0][x0]:
                continue
            stack, n, top, bot = [(x0, y0)], 0, y0, y0
            seen[y0][x0] = True
            while stack:
                x, y = stack.pop()
                n += 1
                top, bot = min(top, y), max(bot, y)
                for dy in (-1, 0, 1):
                    for dx in (-1, 0, 1):
                        nx, ny = x + dx, y + dy
                        if 0 <= nx < w and 0 <= ny < h and not seen[ny][nx] and rows[ny][nx] != 3:
                            seen[ny][nx] = True
                            stack.append((nx, ny))
            if n > best_n:
                best, best_n = (top, bot), n
    return best


def _borders(rows):
    h, w = len(rows), len(rows[0])
    return (any(v != 3 for v in rows[0]), any(v != 3 for v in rows[-1]),
            any(r[0] != 3 for r in rows), any(r[w - 1] != 3 for r in rows))


def drift(new: list[list[int]], idle: list[list[int]]) -> dict:
    n_out, n_cov = edge_runs(new)
    i_out, i_cov = edge_runs(idle)
    npart, ipart = _pet_part(new), _pet_part(idle)
    n_h = npart[1] - npart[0] + 1 if npart else 0
    i_h = ipart[1] - ipart[0] + 1 if ipart else 0
    dy = ((npart[0] + npart[1]) // 2 - (ipart[0] + ipart[1]) // 2) if npart and ipart else 0
    cut_off = any(n and not i for n, i in zip(_borders(new), _borders(idle)))
    flags = []
    if abs(n_out - i_out) >= 1:
        flags.append("outline")
    if n_cov < i_cov - 10:
        flags.append("coverage")
    if abs(n_h - i_h) * 100 > 15 * i_h:
        flags.append("size")
    if abs(dy) > 8:
        flags.append("position")
    if cut_off:
        flags.append("cut_off")
    return {"outline": (i_out, n_out), "coverage": (i_cov, n_cov), "height": (i_h, n_h),
            "dy": dy, "cut_off": cut_off, "flags": flags}


# ---------------------------------------------------------------- review files

_PCX_COLOURS = {0: (0, 0, 0), 1: (255, 0, 0), 2: (255, 255, 255), 3: (255, 255, 255)}


def rows_to_image(rows: list[list[int]], scale: int = SCALE) -> Image.Image:
    """Badge indices on white (transparent shows white), nearest-neighbour scaled."""
    h, w = len(rows), len(rows[0])
    img = Image.new("RGB", (w, h))
    img.putdata([_PCX_COLOURS[v] for r in rows for v in r])
    return img.resize((w * scale, h * scale), Image.NEAREST)


def rgba_on_white(path, scale: int = SCALE) -> Image.Image:
    src = Image.open(path).convert("RGBA")
    bg = Image.new("RGBA", src.size, WHITE_RGB + (255,))
    bg.alpha_composite(src)
    return bg.convert("RGB").resize((src.width * scale, src.height * scale), Image.NEAREST)


def _floor_entries(litter_dir: Path, out_dir: Path) -> list[dict]:
    """Floor files and manifest entries; [] unless all five items are present."""
    items = litter_dir / "items"
    if not all((items / f"{i}.png").is_file() for i in FLOOR_ITEMS):
        return []
    floor = out_dir / "floor"
    floor.mkdir(parents=True, exist_ok=True)
    metrics = {}
    mpath = litter_dir / "metrics.json"
    if mpath.is_file():
        metrics = json.loads(mpath.read_text(encoding="utf-8"))
        metrics = metrics.get("items", metrics)
    entries = []
    for item in FLOOR_ITEMS:
        rgba_on_white(items / f"{item}.png").save(floor / f"{item}.png")
        old = litter_dir / "plan7" / f"{item}.png"
        if old.is_file():
            rgba_on_white(old).save(floor / f"{item}_plan7.png")
        m = metrics.get(item, {})
        inline = {k: v for k, v in m.items() if k not in ("slot", "bytes", "flags", "metrics")}
        entries.append({
            "pet": "floor", "prefix": None, "state": item, "anim_id": None,
            "frame_id": f"floor_{item}", "file": f"floor/{item}.png",
            "idle": f"floor/{item}_plan7.png", "sheet": "floor/strip.png", "pcx": None,
            "bytes": m.get("bytes", 0), "clusters": 0, "slot": m.get("slot"),
            "note": "slot size is the planner's call (design §3.5)",
            "flags": m.get("flags", []), "metrics": m.get("metrics", inline)})
    strip_path = litter_dir / "strip.png"
    if strip_path.is_file():
        rgba_on_white(strip_path).save(floor / "strip.png")
        strip = Image.open(strip_path).convert("RGBA")
        _, _, idle = decode_pcx(TO_BADGE / "050100.PCX")
        scene = Image.new("RGBA", (FRAME[0], FRAME[1]), WHITE_RGB + (255,))
        scene.alpha_composite(strip, (0, FRAME[1] - strip.height))
        pet = Image.new("RGBA", FRAME, (0, 0, 0, 0))
        pet.putdata([(0, 0, 0, 255) if v == 0 else (255, 0, 0, 255) if v == 1
                     else (255, 255, 255, 255) if v == 2 else (0, 0, 0, 0)
                     for r in idle for v in r])
        scene.alpha_composite(pet)
        scene.convert("RGB").resize((FRAME[0] * SCALE, FRAME[1] * SCALE), Image.NEAREST) \
            .save(floor / "scene.png")
    return entries


def write_review(export_dir, round_no: int, out_dir=REVIEW_DIR, litter_dir=LITTER_DIR) -> Path:
    """Write the gallery files and manifest.json under out_dir; returns the manifest path.

    PNGs are overwritten each round; older feedback-*.json files are left alone.
    """
    out_dir, export_dir, litter_dir = Path(out_dir), Path(export_dir), Path(litter_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    bud = budget(export_dir)
    frames = []
    for pet, (prefix, _cfg, sheet, seq) in ART_PETS.items():
        pdir = out_dir / pet
        pdir.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(REPO / "assets" / sheet, pdir / "sheet.png")
        _, _, idle = decode_pcx(TO_BADGE / f"{prefix}0100.PCX")
        rows_to_image(idle).save(pdir / "idle.png")
        for i, state in enumerate(STATES):
            anim = FIRST_STATE_ANIM + i
            name = f"{prefix}{anim:02X}00.PCX"
            path = export_dir / name
            if not path.is_file():
                continue
            _, _, rows = decode_pcx(path)
            rows_to_image(rows).save(pdir / f"{state}.png")
            d = drift(rows, idle)
            size = path.stat().st_size
            frames.append({
                "pet": pet, "prefix": prefix, "state": state, "anim_id": f"0x{anim:02X}",
                "frame_id": f"{seq}_{state}_0", "file": f"{pet}/{state}.png",
                "idle": f"{pet}/idle.png", "sheet": f"{pet}/sheet.png", "pcx": name,
                "bytes": size, "clusters": clusters(size), "flags": d["flags"],
                "metrics": {"outline": list(d["outline"]), "coverage": list(d["coverage"]),
                            "height": list(d["height"]), "dy": d["dy"], "cut_off": d["cut_off"]}})
    frames += _floor_entries(litter_dir, out_dir)
    manifest = {"round": round_no,
                "budget": {"total": bud["total"], "limit": NEW_PCX_BUDGET, "fits": bud["fits"]},
                "frames": frames}
    mpath = out_dir / "manifest.json"
    mpath.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return mpath


# ---------------------------------------------------------------- CLI

def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    d = sub.add_parser("diff")
    d.add_argument("old")
    d.add_argument("new")
    b = sub.add_parser("budget")
    b.add_argument("export")
    r = sub.add_parser("review")
    r.add_argument("export")
    r.add_argument("--round", type=int, required=True, dest="round_no")
    a = ap.parse_args(argv)

    if a.cmd == "diff":
        res = diff_exports(a.old, a.new)
        for k in ("changed", "removed", "added"):
            print(f"{k}: {len(res[k])}")
            for n in res[k]:
                print(f"  {n}")
        foreign = [n for n in res["added"] if n not in new_files()]
        if foreign:
            print(f"not a shipped new-state name: {', '.join(foreign)}")
        return 1 if res["changed"] or res["removed"] or foreign else 0
    if a.cmd == "budget":
        res = budget(a.export)
        for name, n, c in res["files"]:
            print(f"{name}  {n:6d} B  {c} cl{'  OVER 2048' if n > CLUSTER_BYTES else ''}")
        for name in res["missing"]:
            print(f"{name}  MISSING")
        print(f"total {res['total']} / {NEW_PCX_BUDGET} clusters: "
              f"{'fits' if res['fits'] else 'DOES NOT FIT'}")
        return 0 if res["fits"] and not res["missing"] else 1
    print(write_review(a.export, a.round_no))
    return 0


if __name__ == "__main__":
    sys.exit(main())
