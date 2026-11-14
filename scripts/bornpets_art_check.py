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

Floor input contract (the one shape A2b must write under LITTER_DIR):
  items/<item>.png    1x RGBA, the item at its slot size, for each of FLOOR_ITEMS
  plan7/<item>.png    1x RGBA, plan 7's old bitmap (optional; the gallery omits "idle" without it)
  strip.png           1x RGBA, 152 x 10 (optional; the gallery omits "sheet" and scene.png without it)
  metrics.json        {"<item>": {"slot": [x, y, w, h], "bytes": int, "flags": [str, ...],
                                  "metrics": {"outline": int, "coverage": int, "bottom": bool,
                                              "ink_pct": int, "red": int}}}
                      keys are exactly FLOOR_ITEMS names; any other top-level key (for example
                      an "items" wrapper) or any other per-item key is a ValueError.
Floor entries appear once any items/<item>.png exists. An item without its PNG, or without a
metrics.json entry, still gets an entry: "missing": true, or the flag "no_metrics".

Completeness: write_review never drops a frame silently. A missing export PCX or floor item
gets a manifest entry with "missing": true and the flag "missing"; budget.fits is false then.

Protected files (PROTECTED / is_protected): the Slug (02xxxx), hand-added rows 0x20-0x28 of
prefixes 00, 01 and 02, and the menu icons 0406xx / 0407xx. diff_exports never reports them as
changed, removed or added (they come back under "protected"); budget reports the clusters they
use under "protected_clusters" so they still count as existing volume usage.
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
# FAT12 volume: 502 usable 2 KiB clusters, minus 429 used (after plan 7 removes the
# Jellyfish), minus 2 for the files that grow (BPETS_V3.CFG, plans 5 and 7), minus
# a 16-cluster guard = 55 for the 50 new PCX. The 429 is an assumption about the
# volume: used_clusters() recomputes it from the real tree and budget reports drift.
USABLE_CLUSTERS = 502
ASSUMED_USED = 429
NEW_PCX_BUDGET = 55
REVIEW_DIR = REPO / "target/v3art/review"
LITTER_DIR = REPO / "target/v3art/litter"   # written by bornpets_litter.py (Task A2b)
FIRST_STATE_ANIM = 0x29
FLOOR_ITEMS = ("crumbs", "peel", "can", "sock", "pizza_box")   # plan 7 slot order
FLOOR_STRIP = (152, 10)
SCALE = 4
WHITE_RGB = (255, 255, 255)

# Files this tool must never flag: Slug art, hand-added rows, menu icons.
PROTECTED = {
    "prefixes": ("02",),                          # every 02xxxx (the Slug)
    "hand_added": (("00", "01", "02"), range(0x20, 0x29)),   # prefix, anim id
    "icons": ("0406", "0407"),                    # name starts with
}


def is_protected(name: str) -> bool:
    n = name.upper()
    if not n.endswith(".PCX") or len(n) != 10:
        return False
    if n.startswith(PROTECTED["prefixes"]) or n.startswith(PROTECTED["icons"]):
        return True
    prefixes, anims = PROTECTED["hand_added"]
    try:
        return n[:2] in prefixes and int(n[2:4], 16) in anims
    except ValueError:
        return False


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
        return {p.name for p in Path(d).iterdir() if p.is_file()}
    both = names(old_dir) | names(new_dir)
    protected = sorted(n for n in both if is_protected(n))
    old = {n for n in names(old_dir) if n.startswith(tuple(prefixes)) and not is_protected(n)}
    new = {n for n in names(new_dir) if n.startswith(tuple(prefixes)) and not is_protected(n)}
    changed = sorted(n for n in old & new
                     if (Path(old_dir) / n).read_bytes() != (Path(new_dir) / n).read_bytes())
    return {"changed": changed, "removed": sorted(old - new), "added": sorted(new - old),
            "protected": protected}


def used_clusters(to_badge_dir=TO_BADGE) -> int:
    """Clusters the files of a to-badge tree occupy (what the 429 assumption counts)."""
    return sum(clusters(p.stat().st_size) for p in Path(to_badge_dir).iterdir() if p.is_file())


def budget(export_dir, to_badge_dir=TO_BADGE) -> dict:
    if not Path(export_dir).is_dir():
        raise ValueError(f"export dir {export_dir} does not exist")
    files, missing = [], []
    for name in new_files():
        p = Path(export_dir) / name
        if p.is_file():
            n = p.stat().st_size
            files.append((name, n, clusters(n)))
        else:
            missing.append(name)
    total = sum(c for _, _, c in files)
    prot = sum(clusters(p.stat().st_size) for p in Path(export_dir).iterdir()
               if p.is_file() and is_protected(p.name))
    return {"files": files, "total": total,
            "over": [n for n, b, _ in files if b > CLUSTER_BYTES],
            "missing": missing, "protected_clusters": prot,
            "assumed_used": ASSUMED_USED, "measured_used": used_clusters(to_badge_dir),
            "fits": total <= NEW_PCX_BUDGET and not missing}


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


_METRIC_KEYS = {"slot", "bytes", "flags", "metrics"}


def _load_floor_metrics(litter_dir: Path) -> dict:
    mpath = litter_dir / "metrics.json"
    if not mpath.is_file():
        return {}
    data = json.loads(mpath.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"{mpath}: expected an object keyed by item")
    for item, m in data.items():
        if item not in FLOOR_ITEMS:
            raise ValueError(f"{mpath}: unknown key {item!r}; expected only {list(FLOOR_ITEMS)}")
        if not isinstance(m, dict) or set(m) != _METRIC_KEYS or not isinstance(m["metrics"], dict):
            raise ValueError(f"{mpath}: {item!r} must be exactly {sorted(_METRIC_KEYS)} "
                             "with metrics an object")
    return data


def _floor_entries(litter_dir: Path, out_dir: Path) -> list[dict]:
    """Floor files and entries; [] only when the litter has no item PNG at all."""
    items = litter_dir / "items"
    if not any((items / f"{i}.png").is_file() for i in FLOOR_ITEMS):
        return []
    metrics = _load_floor_metrics(litter_dir)
    floor = out_dir / "floor"
    floor.mkdir(parents=True, exist_ok=True)
    strip_path = litter_dir / "strip.png"
    have_strip = strip_path.is_file()
    if have_strip:
        rgba_on_white(strip_path).save(floor / "strip.png")
        strip = Image.open(strip_path).convert("RGBA")
        _, _, idle = decode_pcx(TO_BADGE / "050100.PCX")
        scene = Image.new("RGBA", FRAME, WHITE_RGB + (255,))
        scene.alpha_composite(strip, (0, FRAME[1] - strip.height))
        pet = Image.new("RGBA", FRAME, (0, 0, 0, 0))
        pet.putdata([(0, 0, 0, 255) if v == 0 else (255, 0, 0, 255) if v == 1
                     else (255, 255, 255, 255) if v == 2 else (0, 0, 0, 0)
                     for r in idle for v in r])
        scene.alpha_composite(pet)
        scene.convert("RGB").resize((FRAME[0] * SCALE, FRAME[1] * SCALE), Image.NEAREST) \
            .save(floor / "scene.png")
    entries = []
    for item in FLOOR_ITEMS:
        base = {"pet": "floor", "prefix": None, "state": item, "anim_id": None,
                "frame_id": f"floor_{item}", "pcx": None, "clusters": 0,
                "note": "slot size is the planner's call (design §3.5)"}
        png = items / f"{item}.png"
        if not png.is_file():
            entries.append({**base, "missing": True, "file": None, "idle": None, "sheet": None,
                            "bytes": 0, "slot": None, "flags": ["missing"], "metrics": None})
            continue
        rgba_on_white(png).save(floor / f"{item}.png")
        old = litter_dir / "plan7" / f"{item}.png"
        if old.is_file():
            rgba_on_white(old).save(floor / f"{item}_plan7.png")
        m = metrics.get(item)
        entries.append({
            **base, "missing": False, "file": f"floor/{item}.png",
            "idle": f"floor/{item}_plan7.png" if old.is_file() else None,
            "sheet": "floor/strip.png" if have_strip else None,
            "bytes": m["bytes"] if m else 0, "slot": m["slot"] if m else None,
            "flags": list(m["flags"]) if m else ["no_metrics"],
            "metrics": m["metrics"] if m else None})
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
            base = {"pet": pet, "prefix": prefix, "state": state, "anim_id": f"0x{anim:02X}",
                    "frame_id": f"{seq}_{state}_0", "pcx": name, "idle": f"{pet}/idle.png",
                    "sheet": f"{pet}/sheet.png"}
            if not path.is_file():
                frames.append({**base, "missing": True, "file": None, "bytes": 0, "clusters": 0,
                               "flags": ["missing"], "metrics": None})
                continue
            w, h, rows = decode_pcx(path)
            rows_to_image(rows).save(pdir / f"{state}.png")
            d = drift(rows, idle)
            size = path.stat().st_size
            flags = d["flags"] + (["frame_size"] if (w, h) != FRAME else [])
            frames.append({
                **base, "missing": False, "file": f"{pet}/{state}.png",
                "bytes": size, "clusters": clusters(size), "flags": flags,
                "metrics": {"outline": list(d["outline"]), "coverage": list(d["coverage"]),
                            "height": list(d["height"]), "dy": d["dy"], "cut_off": d["cut_off"]}})
    floor = _floor_entries(litter_dir, out_dir)
    frames += floor
    incomplete = bool(bud["missing"]) or any(f["missing"] for f in floor)
    manifest = {"round": round_no,
                "budget": {"total": bud["total"], "limit": NEW_PCX_BUDGET,
                           "fits": bud["fits"] and not incomplete},
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
        for k in ("changed", "removed", "added", "protected"):
            print(f"{k}: {len(res[k])}")
            for n in res[k]:
                print(f"  {n}")
        foreign = [n for n in res["added"] if n not in new_files()]
        if foreign:
            print(f"not a shipped new-state name: {', '.join(foreign)}")
        return 1 if res["changed"] or res["removed"] or foreign else 0
    if a.cmd == "budget":
        try:
            res = budget(a.export)
        except ValueError as exc:
            print(exc, file=sys.stderr)
            return 1
        for name, n, c in res["files"]:
            print(f"{name}  {n:6d} B  {c} cl{'  OVER 2048' if n > CLUSTER_BYTES else ''}")
        for name in res["missing"]:
            print(f"{name}  MISSING")
        if res["measured_used"] != res["assumed_used"]:
            print(f"note: assumed_used {res['assumed_used']}, measured_used {res['measured_used']} "
                  "(plan 7 removes the Jellyfish; re-derive the budget if this stays off)")
        print(f"total {res['total']} / {NEW_PCX_BUDGET} clusters: "
              f"{'fits' if res['fits'] else 'DOES NOT FIT'}")
        return 0 if res["fits"] and not res["missing"] else 1
    print(write_review(a.export, a.round_no))
    return 0


if __name__ == "__main__":
    sys.exit(main())
