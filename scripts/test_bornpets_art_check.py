#!/usr/bin/env python3
"""Unit tests for scripts/bornpets_art_check.py (no API calls, no repo-local state)."""
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from PIL import Image  # noqa: E402

import bornpets_art_check as ac  # noqa: E402
from fix_badge_pcx import rle_encode_line  # noqa: E402


def write_pcx(path, rows):
    h, w = len(rows), len(rows[0])
    bpl = (w * 2 + 7) // 8
    hdr = bytearray(128)
    hdr[0], hdr[1], hdr[2], hdr[3] = 0x0A, 5, 1, 2
    hdr[8:10] = (w - 1).to_bytes(2, "little")
    hdr[10:12] = (h - 1).to_bytes(2, "little")
    hdr[65] = 1
    hdr[66:68] = bpl.to_bytes(2, "little")
    body = bytearray()
    for row in rows:
        line = bytearray()
        for x in range(0, w, 4):
            b = 0
            for k in range(4):
                if x + k < w:
                    b |= (row[x + k] & 3) << (6 - 2 * k)
            line.append(b)
        body += rle_encode_line(line)
    Path(path).write_bytes(bytes(hdr) + bytes(body))


def box(w=30, h=20, outline=2, ox=40, oy=20, fw=152, fh=75):
    """A white box with a black outline on a transparent frame."""
    rows = [[3] * fw for _ in range(fh)]
    for y in range(h):
        for x in range(w):
            edge = min(x, y, w - 1 - x, h - 1 - y) < outline
            rows[oy + y][ox + x] = 0 if edge else 2
    return rows


def pad_file(path, size):
    Path(path).write_bytes(b"\0" * size)


class Checker(unittest.TestCase):
    def test_decode_round_trips_a_written_pcx(self):
        rows = [[(x * 7 + y * 3 + x // 5) % 4 for x in range(152)] for y in range(75)]
        with tempfile.TemporaryDirectory() as t:
            p = Path(t) / "a.PCX"
            write_pcx(p, rows)
            self.assertEqual(ac.decode_pcx(p), (152, 75, rows))

    def test_clusters_at_the_boundary(self):
        self.assertEqual([ac.clusters(n) for n in (0, 1, 2048, 2049)], [0, 1, 1, 2])

    def test_new_files_are_the_fifty(self):
        names = ac.new_files()
        self.assertEqual(len(names), 50)
        self.assertEqual(names, sorted(names))
        self.assertIn("002900.PCX", names)
        self.assertIn("073200.PCX", names)
        self.assertFalse([n for n in names if n.startswith("02")])

    def test_diff_flags_changed_removed_and_foreign_added(self):
        with tempfile.TemporaryDirectory() as o, tempfile.TemporaryDirectory() as n:
            for name in ("000100.PCX", "000200.PCX", "010100.PCX"):
                Path(o, name).write_bytes(b"abc")
            Path(n, "000100.PCX").write_bytes(b"abd")   # changed
            Path(n, "010100.PCX").write_bytes(b"abc")   # same; 000200 removed
            Path(n, "060E05.PCX").write_bytes(b"x")     # added, not a shipped name
            res = ac.diff_exports(o, n)
            self.assertEqual(res, {"changed": ["000100.PCX"], "removed": ["000200.PCX"],
                                   "added": ["060E05.PCX"]})
            self.assertEqual(ac.main(["diff", o, n]), 1)

    def test_diff_ok_when_only_shipped_names_are_added(self):
        with tempfile.TemporaryDirectory() as o, tempfile.TemporaryDirectory() as n:
            Path(o, "000100.PCX").write_bytes(b"abc")
            Path(n, "000100.PCX").write_bytes(b"abc")
            Path(n, "002900.PCX").write_bytes(b"new")
            self.assertEqual(ac.main(["diff", o, n]), 0)

    def test_diff_ignores_other_prefixes(self):
        with tempfile.TemporaryDirectory() as o, tempfile.TemporaryDirectory() as n:
            Path(o, "030000.PCX").write_bytes(b"a")
            Path(n, "030000.PCX").write_bytes(b"b")
            self.assertEqual(ac.diff_exports(o, n), {"changed": [], "removed": [], "added": []})

    def test_budget_counts_and_lists_files_over_2048(self):
        with tempfile.TemporaryDirectory() as t:
            names = ac.new_files()
            for name in names[:49]:
                pad_file(Path(t, name), 1000)
            pad_file(Path(t, names[49]), 2049)
            res = ac.budget(t)
            self.assertEqual(res["total"], 51)
            self.assertEqual(res["over"], [names[49]])
            self.assertTrue(res["fits"])
            self.assertEqual(ac.main(["budget", t]), 0)

    def test_budget_over_the_limit_does_not_fit(self):
        with tempfile.TemporaryDirectory() as t:
            for name in ac.new_files():
                pad_file(Path(t, name), 2500)
            res = ac.budget(t)
            self.assertEqual(res["total"], 100)
            self.assertFalse(res["fits"])
            self.assertEqual(ac.main(["budget", t]), 1)

    def test_budget_with_a_missing_file_fails_the_cli(self):
        with tempfile.TemporaryDirectory() as t:
            for name in ac.new_files()[1:]:
                pad_file(Path(t, name), 100)
            self.assertEqual(ac.budget(t)["missing"], [ac.new_files()[0]])
            self.assertEqual(ac.main(["budget", t]), 1)

    def test_drift_same_frame_has_no_flags(self):
        self.assertEqual(ac.drift(box(), box())["flags"], [])

    def test_drift_thicker_outline_is_flagged(self):
        d = ac.drift(box(outline=3), box(outline=2))
        self.assertIn("outline", d["flags"])
        self.assertEqual(d["outline"], (2, 3))

    def test_drift_gap_in_outline_is_flagged(self):
        new = box()
        for y in range(20):                       # whiten the outline on the top 40 %
            for x in range(30):
                if new[20 + y][40 + x] == 0 and (y < 8 or x < 12):
                    new[20 + y][40 + x] = 2
        self.assertIn("coverage", ac.drift(new, box())["flags"])

    def test_drift_bigger_pet_is_flagged(self):
        d = ac.drift(box(h=24), box(h=20))
        self.assertIn("size", d["flags"])
        self.assertEqual(d["height"], (20, 24))
        d = ac.drift(box(oy=30), box(oy=20))
        self.assertIn("position", d["flags"])
        self.assertEqual(d["dy"], 10)

    def test_drift_art_on_a_new_border_is_cut_off(self):
        new = box()
        new[0][10] = 0
        d = ac.drift(new, box())
        self.assertTrue(d["cut_off"])
        self.assertIn("cut_off", d["flags"])

    def test_drift_border_art_in_idle_is_not_cut_off(self):
        idle = box()
        idle[0][10] = 0
        new = box()
        new[0][11] = 0
        self.assertFalse(ac.drift(new, idle)["cut_off"])

    def _export(self, t):
        rows = box()
        for name in ac.new_files():
            write_pcx(Path(t, name), rows)

    def test_review_writes_manifest_and_files(self):
        with tempfile.TemporaryDirectory() as e, tempfile.TemporaryDirectory() as o, \
                tempfile.TemporaryDirectory() as lit:
            self._export(e)
            old = Path(o, "feedback-1.json")
            old.write_text("{}")
            mpath = ac.write_review(e, 1, out_dir=o, litter_dir=lit)
            self.assertEqual(mpath, Path(o) / "manifest.json")
            m = json.loads(mpath.read_text())
            self.assertEqual(m["round"], 1)
            self.assertEqual(m["budget"]["limit"], 55)
            self.assertEqual(m["budget"]["total"], 50)
            self.assertEqual(len(m["frames"]), 50)
            self.assertTrue(old.exists())
            f0 = m["frames"][0]
            self.assertEqual((f0["pet"], f0["state"], f0["anim_id"], f0["frame_id"], f0["pcx"]),
                             ("bartholomeus", "tipsy", "0x29", "bartho_tipsy_0", "002900.PCX"))
            for f in m["frames"]:
                frame = Image.open(Path(o, f["file"]))
                self.assertEqual(frame.size, (152 * 4, 75 * 4))
                self.assertEqual(Image.open(Path(o, f["idle"])).size, (152 * 4, 75 * 4))
                self.assertTrue(Path(o, f["sheet"]).is_file())
                self.assertEqual(set(f["metrics"]), {"outline", "coverage", "height", "dy", "cut_off"})

    def test_review_without_litter_has_no_floor(self):
        with tempfile.TemporaryDirectory() as e, tempfile.TemporaryDirectory() as o, \
                tempfile.TemporaryDirectory() as lit:
            self._export(e)
            m = json.loads(ac.write_review(e, 2, out_dir=o, litter_dir=lit).read_text())
            self.assertEqual(len(m["frames"]), 50)
            self.assertFalse(Path(o, "floor").exists())

    def test_review_lists_the_floor_items(self):
        with tempfile.TemporaryDirectory() as e, tempfile.TemporaryDirectory() as o, \
                tempfile.TemporaryDirectory() as lit:
            self._export(e)
            lit = Path(lit)
            (lit / "items").mkdir()
            (lit / "plan7").mkdir()
            metrics = {}
            for i, item in enumerate(ac.FLOOR_ITEMS):
                w, h = 12 + i, 5 + i
                im = Image.new("RGBA", (w, h), (0, 0, 0, 0))
                im.paste((0, 0, 0, 255), (0, h - 1, w, h))
                im.save(lit / "items" / f"{item}.png")
                Image.new("RGBA", (152, 10), (0, 0, 0, 0)).save(lit / "plan7" / f"{item}.png")
                metrics[item] = {"slot": [2, 105, w, h], "bytes": 10, "flags": [],
                                 "metrics": {"outline": 1, "coverage": 100, "bottom": True,
                                             "ink_pct": 41, "red": 0}}
            Image.new("RGBA", (152, 10), (0, 0, 0, 0)).save(lit / "strip.png")
            (lit / "metrics.json").write_text(json.dumps(metrics))
            m = json.loads(ac.write_review(e, 1, out_dir=o, litter_dir=lit).read_text())
            self.assertEqual(len(m["frames"]), 55)
            self.assertEqual(m["budget"]["total"], 50)
            floor = [f for f in m["frames"] if f["pet"] == "floor"]
            self.assertEqual([f["state"] for f in floor], list(ac.FLOOR_ITEMS))
            for f in floor:
                self.assertEqual(f["clusters"], 0)
                self.assertEqual(f["bytes"], 10)
                self.assertEqual(len(f["slot"]), 4)
                self.assertIsNone(f["pcx"])
                self.assertEqual(f["frame_id"], "floor_" + f["state"])
                for key in ("file", "idle", "sheet"):
                    self.assertTrue(Path(o, f[key]).is_file(), f[key])
            self.assertEqual(Image.open(Path(o, "floor/scene.png")).size, (152 * 4, 75 * 4))
            self.assertEqual(Image.open(Path(o, "floor/strip.png")).size, (152 * 4, 10 * 4))


if __name__ == "__main__":
    unittest.main()
