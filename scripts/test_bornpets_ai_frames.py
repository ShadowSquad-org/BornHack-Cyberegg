#!/usr/bin/env python3
"""Tests for the state-frame flow of scripts/bornpets_ai_frames.py.

No real API call: subprocess.run is patched wherever call_api can be reached, and
the key is a fake token in a tmp dir outside the repo.
"""
import base64
import contextlib
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

os.environ["BORNPETS_PET"] = "panda"
os.environ.pop("BORNPETS_MODEL", None)
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from PIL import Image  # noqa: E402

import bornpets_ai_frames as afm  # noqa: E402
import bornpets_art_check as ac  # noqa: E402

REAL_RUN = subprocess.run          # kept for the per-pet subprocess check
TOKEN = "sk-test-UNIQUE-TOKEN-0123456789"
SCRIPTS = Path(__file__).resolve().parent


def canvas_png_b64() -> str:
    buf = io.BytesIO()
    Image.new("RGBA", afm.CANVAS, afm.KEY).save(buf, "PNG")
    return base64.b64encode(buf.getvalue()).decode()


class FakeRun:
    """Records every curl call and answers with a magenta CANVAS PNG."""

    def __init__(self):
        self.calls = []

    def __call__(self, cmd, **kw):
        self.calls.append((list(cmd), kw))
        out = json.dumps({"data": [{"b64_json": canvas_png_b64()}]}).encode()
        return subprocess.CompletedProcess(cmd, 0, out, b"")


def make_key_file(directory, mode=0o600, text=TOKEN):
    p = Path(directory) / "key"
    p.write_text(text + "\n")
    p.chmod(mode)
    return p


class Base(unittest.TestCase):
    def setUp(self):
        self._env = mock.patch.dict(os.environ)
        self._env.start()
        for k in ("OPENAI_API_KEY", "OPENAI_API_KEY_FILE"):
            os.environ.pop(k, None)
        self.addCleanup(self._env.stop)
        self._kf = mock.patch.object(afm, "KEY_FILE", None)
        self._kf.start()
        self.addCleanup(self._kf.stop)


class KeyHandling(Base):
    def test_key_never_reaches_argv_or_output(self):
        with tempfile.TemporaryDirectory() as t:
            kf = make_key_file(t)
            fake = FakeRun()
            out, err = io.StringIO(), io.StringIO()
            with mock.patch.object(subprocess, "run", fake), \
                    contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                afm.KEY_FILE = str(kf)
                afm.call_api([], "prompt", "low", afm.CANVAS)
            cmd, kw = fake.calls[0]
            self.assertFalse([a for a in cmd if TOKEN in a])
            self.assertIn(TOKEN, kw["input"].decode())
            self.assertIn("-K", cmd)
            self.assertNotIn(TOKEN, out.getvalue() + err.getvalue())

    def test_key_file_inside_repo_is_refused(self):
        (afm.REPO / "target").mkdir(exist_ok=True)
        d = tempfile.mkdtemp(dir=afm.REPO / "target")
        self.addCleanup(shutil.rmtree, d, True)
        kf = make_key_file(d)
        with self.assertRaises(SystemExit) as cm:
            afm.api_key(str(kf))
        self.assertNotIn(TOKEN, str(cm.exception))
        self.assertIn(str(kf), str(cm.exception))

    def test_key_file_readable_by_others_is_refused(self):
        with tempfile.TemporaryDirectory() as t:
            kf = make_key_file(t, mode=0o644)
            with self.assertRaises(SystemExit) as cm:
                afm.api_key(str(kf))
            self.assertNotIn(TOKEN, str(cm.exception))

    def test_key_source_order(self):
        with tempfile.TemporaryDirectory() as t:
            a = Path(t, "a")
            a.write_text("from-arg\n")
            a.chmod(0o600)
            g = Path(t, "g")
            g.write_text("from-global\n")
            g.chmod(0o600)
            e = Path(t, "e")
            e.write_text("from-envfile\n")
            e.chmod(0o600)
            os.environ["OPENAI_API_KEY"] = "from-env"
            os.environ["OPENAI_API_KEY_FILE"] = str(e)
            self.assertEqual(afm.api_key(str(a)), "from-arg")
            afm.KEY_FILE = str(g)
            self.assertEqual(afm.api_key(), "from-global")
            afm.KEY_FILE = None
            self.assertEqual(afm.api_key(), "from-envfile")
            del os.environ["OPENAI_API_KEY_FILE"]
            self.assertEqual(afm.api_key(), "from-env")

    def test_default_model_is_current(self):
        self.assertEqual(afm.MODEL, "gpt-image-2.6-sunburst")


class StateFlow(Base):
    def test_state_request_sends_idle_first_then_table_refs(self):
        with tempfile.TemporaryDirectory() as t:
            refs = afm.state_refs("06", "throwing_up", Path(t))
            self.assertEqual([r.stem for r in refs],
                             ["060100", "060300", "060C00", "062300"])
            fake = FakeRun()
            with mock.patch.object(subprocess, "run", fake):
                os.environ["OPENAI_API_KEY"] = TOKEN
                afm.call_api(refs, "p", "low", afm.CANVAS)
            cmd = fake.calls[0][0]
            imgs = [a for a in cmd if a.startswith("image[]=@")]
            self.assertEqual([Path(a.split("@", 1)[1].split(";")[0]).stem for a in imgs],
                             ["060100", "060300", "060C00", "062300"])
            self.assertIn("size=1536x1024", cmd)

    def test_every_state_has_a_brief_and_two_to_four_refs(self):
        for s in ac.STATES:
            self.assertIn(s, afm.STATE_BRIEFS)
            self.assertTrue(afm.STATE_BRIEFS[s].endswith("Nothing else in the frame."), s)
            self.assertGreaterEqual(len(afm.STATE_REFS[s]), 2)
            self.assertLessEqual(len(afm.STATE_REFS[s]), 4)
            for prefix, *_ in ac.ART_PETS.values():
                for aa in (0x01,) + tuple(afm.STATE_REFS[s]):
                    self.assertTrue((ac.TO_BADGE / f"{prefix}{aa:02X}00.PCX").is_file(),
                                    f"{prefix}{aa:02X}00.PCX")
        self.assertEqual(set(afm.STATE_REFS), set(ac.STATES))
        self.assertEqual(set(afm.STATE_BRIEFS), set(ac.STATES))

    def test_prompt_shape(self):
        p = afm.state_prompt("wired", "bigger can")
        self.assertIn(afm.IDENTITY, p)
        self.assertIn("IMAGE 1 is this panda in its idle pose", p)
        self.assertIn("IMAGES 2 to 4", p)
        self.assertIn(afm.STATE_BRIEFS["wired"].format(pet="panda"), p)
        self.assertTrue(p.endswith("Owner feedback on the previous attempt: bigger can"))
        self.assertNotIn("Owner feedback", afm.state_prompt("wired"))

    def test_ref_png_puts_the_frame_in_the_fixed_rectangle(self):
        with tempfile.TemporaryDirectory() as t:
            p = Path(t) / "r.png"
            afm.ref_png("05", 0x01, p)
            im = Image.open(p).convert("RGBA")
        self.assertEqual(im.size, afm.CANVAS)
        ox, oy = afm.REF_ORIGIN
        self.assertEqual((ox, oy), (312, 287))
        px = im.load()
        for y in range(im.height):
            for x in range(im.width):
                if not (ox <= x < ox + 912 and oy <= y < oy + 450):
                    self.assertEqual(px[x, y], afm.KEY)
        self.assertNotEqual(px[ox + 456, oy + 300], afm.KEY)   # the panda is inside

    def test_cut_back_returns_the_exact_frame_and_palette(self):
        with tempfile.TemporaryDirectory() as t:
            p = Path(t) / "r.png"
            afm.ref_png("05", 0x01, p)
            out = afm.cut_back(Image.open(p).convert("RGBA"))
        self.assertEqual(out.size, ac.FRAME)
        self.assertLessEqual(set(out.getdata()), {afm.BLACK, afm.WHITE, afm.RED, afm.TRANSPARENT})
        self.assertEqual(self._as_rows(out), ac.decode_pcx(ac.TO_BADGE / "050100.PCX")[2])

    def test_cut_back_rejects_a_wrong_canvas(self):
        with self.assertRaises(AssertionError):
            afm.cut_back(Image.new("RGBA", (1024, 1024), afm.KEY))

    @staticmethod
    def _as_rows(img):
        idx = {afm.BLACK: 0, afm.RED: 1, afm.WHITE: 2, afm.TRANSPARENT: 3}
        w, h = img.size
        px = img.load()
        return [[idx[px[x, y]] for x in range(w)] for y in range(h)]

    def test_cut_back_round_trips_idle_for_all_five_pets(self):
        code = (
            "import sys; sys.path.insert(0, %r)\n"
            "import tempfile\nfrom pathlib import Path\nfrom PIL import Image\n"
            "import bornpets_ai_frames as afm, bornpets_art_check as ac\n"
            "idx={afm.BLACK:0,afm.RED:1,afm.WHITE:2,afm.TRANSPARENT:3}\n"
            "with tempfile.TemporaryDirectory() as t:\n"
            "    p=Path(t)/'r.png'; afm.ref_png(sys.argv[1], 1, p)\n"
            "    out=afm.cut_back(Image.open(p).convert('RGBA'))\n"
            "px=out.load()\n"
            "rows=[[idx[px[x,y]] for x in range(152)] for y in range(75)]\n"
            "want=ac.decode_pcx(ac.TO_BADGE/(sys.argv[1]+'0100.PCX'))[2]\n"
            "sys.exit(0 if rows==want else 1)\n" % str(SCRIPTS)
        )
        for pet, (prefix, *_) in ac.ART_PETS.items():
            env = {k: v for k, v in os.environ.items() if k != "BORNPETS_PET"}
            env["BORNPETS_PET"] = pet
            res = REAL_RUN([sys.executable, "-I", "-c", code, prefix],
                           env=env, capture_output=True, text=True)
            self.assertEqual(res.returncode, 0, f"{pet}: {res.stdout}{res.stderr}")

    def test_merge_islands_merges_enclosed_specks(self):
        img = Image.new("RGBA", (12, 12), afm.TRANSPARENT)
        for y in range(2, 10):
            for x in range(2, 10):
                img.putpixel((x, y), afm.WHITE)
        img.putpixel((5, 5), afm.BLACK)
        img.putpixel((6, 5), afm.BLACK)
        out = afm.merge_islands(img, 3)
        self.assertEqual(out.getpixel((5, 5)), afm.WHITE)
        self.assertEqual(out.getpixel((6, 5)), afm.WHITE)

    def test_merge_islands_never_touches_the_outline(self):
        img = Image.new("RGBA", (12, 12), afm.TRANSPARENT)
        for y in range(2, 10):
            for x in range(2, 10):
                img.putpixel((x, y), afm.WHITE)
        img.putpixel((2, 5), afm.BLACK)     # on the silhouette edge, next to transparency
        img.putpixel((0, 0), afm.RED)       # on the frame edge
        out = afm.merge_islands(img, 3)
        self.assertEqual(out.getpixel((2, 5)), afm.BLACK)
        self.assertEqual(out.getpixel((0, 0)), afm.RED)
        self.assertEqual(out.getpixel((5, 11)), afm.TRANSPARENT)

    def test_merge_islands_leaves_a_mixed_neighbourhood(self):
        img = Image.new("RGBA", (6, 3), afm.WHITE)
        for y in range(3):
            img.putpixel((3, y), afm.BLACK)       # a black wall right of x=2
        img.putpixel((2, 1), afm.RED)             # red speck touching white and black
        out = afm.merge_islands(img, 3)
        self.assertEqual(out.getpixel((2, 1)), afm.RED)


class StateCommand(Base):
    def run_state(self, work, note=None, extra=()):
        argv = ["x", "--work", str(work), "state", "--quality", "low", *extra]
        if note is not None:
            nf = Path(work) / "notes.json"
            nf.write_text(json.dumps(note))
            argv += ["--note-file", str(nf)]
        fake = FakeRun()
        os.environ["OPENAI_API_KEY"] = TOKEN
        with mock.patch.object(subprocess, "run", fake), \
                mock.patch.object(sys, "argv", argv), \
                contextlib.redirect_stdout(io.StringIO()):
            afm.main()
        return fake

    def test_note_file_skips_ok_frames_and_adds_notes(self):
        with tempfile.TemporaryDirectory() as t:
            fake = self.run_state(t, {"panda_tipsy_0": "ok", "panda_wired_0": "bigger can",
                                      "floor_crumbs": "too dark"})
            self.assertEqual(len(fake.calls), 1)
            prompt = [a for a in fake.calls[0][0] if a.startswith("prompt=")][0]
            self.assertTrue(prompt.endswith("bigger can"))
            self.assertTrue(Path(t, "frames/panda_wired_0.png").is_file())
            self.assertFalse(Path(t, "frames/panda_tipsy_0.png").exists())
            self.assertFalse(Path(t, "frames/panda_sore_0.png").exists())   # absent: skipped

    def test_cached_raw_is_skipped_unless_forced(self):
        with tempfile.TemporaryDirectory() as t:
            self.assertEqual(len(self.run_state(t, extra=["--states", "tipsy,sore"]).calls), 2)
            self.assertEqual(len(self.run_state(t, extra=["--states", "tipsy,sore"]).calls), 0)
            self.assertEqual(len(self.run_state(t, extra=["--states", "tipsy", "--force"]).calls), 1)

    def test_simplify_rewrites_frames_in_place(self):
        with tempfile.TemporaryDirectory() as t:
            (Path(t) / "frames").mkdir()
            img = Image.new("RGBA", ac.FRAME, afm.TRANSPARENT)
            for y in range(10, 30):
                for x in range(10, 30):
                    img.putpixel((x, y), afm.WHITE)
            img.putpixel((20, 20), afm.BLACK)
            img.save(Path(t) / "frames/panda_tipsy_0.png")
            with mock.patch.object(sys, "argv", ["x", "--work", t, "simplify",
                                                 "--only", "panda_tipsy_0", "--min-island", "2"]), \
                    contextlib.redirect_stdout(io.StringIO()):
                afm.main()
            out = Image.open(Path(t) / "frames/panda_tipsy_0.png").convert("RGBA")
            self.assertEqual(out.getpixel((20, 20)), afm.WHITE)
            self.assertFalse((Path(t) / "overrides").exists())

    def test_list_states_prints_ten_ids(self):
        buf = io.StringIO()
        with mock.patch.object(sys, "argv", ["x", "list", "--states"]), \
                contextlib.redirect_stdout(buf):
            afm.main()
        ids = buf.getvalue().strip().split(",")
        self.assertEqual(len(ids), 10)
        self.assertEqual(ids[0], "panda_tipsy_0")
        self.assertEqual(ids[-1], "panda_throwing_up_0")


class Profiles(unittest.TestCase):
    def test_new_profiles_load_and_pair_without_panda(self):
        code = (
            "import sys; sys.path.insert(0, %r)\n"
            "import bornpets_ai_frames as afm\n"
            "idx=afm.frame_index()\n"
            "assert idx and 'panda_idle_0' not in idx or sys.argv[1]=='panda', sorted(idx)[:3]\n"
            "assert all('panda_id' not in v for v in idx.values()) or sys.argv[1] in ('badger','seastar','jellyfish','panda')\n"
            "print(len(idx))\n" % str(SCRIPTS)
        )
        for pet in ("bartholomeus", "cat", "panda", "badger", "seastar", "jellyfish"):
            env = {k: v for k, v in os.environ.items() if k != "BORNPETS_PET"}
            env["BORNPETS_PET"] = pet
            res = REAL_RUN([sys.executable, "-I", "-c", code, pet],
                           env=env, capture_output=True, text=True)
            self.assertEqual(res.returncode, 0, f"{pet}: {res.stdout}{res.stderr}")


if __name__ == "__main__":
    unittest.main()
