"""Tests for copy_assets.py.  Temp dirs only; never touches a real badge.

Run from the repo root:  python3 -m unittest discover -s scripts -p 'test_copy_assets.py'
"""

import errno
import hashlib
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import copy_assets  # noqa: E402


class CopyAssetsTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.assets = Path(tmp.name) / "assets"
        self.vol = Path(tmp.name) / "vol"
        self.assets.mkdir()
        self.vol.mkdir()
        (self.assets / "000000.PCX").write_bytes(b"asset-000000")
        (self.assets / "010000.PCX").write_bytes(b"asset-010000")
        (self.assets / "PETS.CFG").write_bytes(b"asset-pets")
        (self.assets / "BPETS_V3.CFG").write_bytes(b"# template\n")

    def update(self):
        return copy_assets.update_assets_to(self.vol, self.assets)

    def test_update_leaves_old_config_untouched(self):
        (self.vol / "BORNPETS.CFG").write_bytes(b"MOOD_STEP=40\r\n")
        self.update()
        self.assertEqual((self.vol / "BORNPETS.CFG").read_bytes(), b"MOOD_STEP=40\r\n")
        self.assertEqual(sorted(p.name for p in self.vol.iterdir() if p.suffix.upper() == ".BAK"), [])

    def test_update_adds_v3_config_when_missing(self):
        result = self.update()
        self.assertEqual((self.vol / copy_assets.CONFIG_NAME).read_bytes(), b"# template\n")
        self.assertIn("BPETS_V3.CFG", result["added"])

    def test_update_keeps_edited_v3_config(self):
        (self.vol / "BPETS_V3.CFG").write_bytes(b"MOOD_STEP=40\n")
        result = self.update()
        self.assertEqual((self.vol / "BPETS_V3.CFG").read_bytes(), b"MOOD_STEP=40\n")
        self.assertNotIn("BPETS_V3.CFG", result["added"])

    def test_update_never_overwrites_other_files(self):
        (self.vol / "PETS.CFG").write_bytes(b"mine")
        (self.vol / "000000.PCX").write_bytes(b"badge-specific")
        result = self.update()
        self.assertEqual((self.vol / "PETS.CFG").read_bytes(), b"mine")
        self.assertEqual((self.vol / "000000.PCX").read_bytes(), b"badge-specific")
        self.assertEqual((self.vol / "010000.PCX").read_bytes(), b"asset-010000")
        self.assertEqual(sorted(result["added"]), ["010000.PCX", "BPETS_V3.CFG"])

    def test_update_matches_names_case_insensitively(self):
        (self.vol / "pets.cfg").write_bytes(b"mine")
        self.update()
        self.assertEqual((self.vol / "pets.cfg").read_bytes(), b"mine")
        names = [p.name for p in self.vol.iterdir() if p.name.lower() == "pets.cfg"]
        self.assertEqual(names, ["pets.cfg"])

    def test_update_survives_file_created_after_listing(self):
        real_iterdir = Path.iterdir
        racer = self.assets / "010000.PCX"

        def racing_iterdir(path):
            entries = list(real_iterdir(path))
            if path == self.vol:
                # The badge writes the file after we listed the volume.
                (self.vol / racer.name).write_bytes(b"raced")
            return iter(entries)

        with mock.patch.object(Path, "iterdir", racing_iterdir):
            result = self.update()
        self.assertEqual((self.vol / "010000.PCX").read_bytes(), b"raced")
        self.assertNotIn("010000.PCX", result["added"])

    def test_update_case_variants_in_assets_do_not_overwrite(self):
        (self.assets / "extra.pcx").write_bytes(b"lower")
        (self.assets / "EXTRA.PCX").write_bytes(b"upper")
        result = self.update()
        self.assertEqual(len([n for n in result["added"] if n.lower() == "extra.pcx"]), 1)

    def test_factory_mode_unchanged(self):
        with mock.patch.object(copy_assets, "ASSETS_DIR", self.assets):
            ok = copy_assets.copy_assets_to(self.vol, label="T", quiet=True)
        self.assertTrue(ok)
        for src in self.assets.iterdir():
            self.assertEqual((self.vol / src.name).read_bytes(), src.read_bytes())

    def test_process_device_update_accepts_non_empty_volume(self):
        (self.vol / "000000.PCX").write_bytes(b"badge-specific")
        empty = self.assets.parent / "changes.txt"
        empty.write_text("# none\n")
        with mock.patch.object(copy_assets, "ASSETS_DIR", self.assets), \
                mock.patch.object(copy_assets, "CHANGES_FILE", empty), \
                mock.patch.object(copy_assets, "mount_via_udisks", return_value=self.vol), \
                mock.patch.object(copy_assets, "unmount_via_udisks"):
            self.assertFalse(copy_assets.process_device("/dev/x", "CYBR0001", quiet=True))
            self.assertTrue(copy_assets.process_device("/dev/x", "CYBR0001", quiet=True, update=True))
        self.assertEqual((self.vol / "000000.PCX").read_bytes(), b"badge-specific")
        self.assertTrue((self.vol / "BPETS_V3.CFG").exists())

    # ---- removals, replacements, no-space handling ----

    @staticmethod
    def sha(data):
        return hashlib.sha256(data).hexdigest()

    def test_update_removes_retired_file_with_known_hash(self):
        (self.vol / "080100.PCX").write_bytes(b"shipped")
        changes = [("remove", "080100.PCX", frozenset({self.sha(b"shipped")}))]
        result = copy_assets.update_assets_to(self.vol, self.assets, changes)
        self.assertFalse((self.vol / "080100.PCX").exists())
        self.assertEqual(result["removed"], ["080100.PCX"])

    def test_update_keeps_retired_file_changed_on_badge(self):
        (self.vol / "080100.PCX").write_bytes(b"my own pet")
        changes = [("remove", "080100.PCX", frozenset({self.sha(b"shipped")}))]
        result = copy_assets.update_assets_to(self.vol, self.assets, changes)
        self.assertEqual((self.vol / "080100.PCX").read_bytes(), b"my own pet")
        self.assertEqual(result["kept"], ["080100.PCX"])
        self.assertEqual(result["removed"], [])

    def test_update_replaces_only_a_known_old_version(self):
        (self.assets / "MANIFEST.TXT").write_bytes(b"new manifest")
        changes = [("replace", "MANIFEST.TXT", frozenset({self.sha(b"old manifest")}))]
        (self.vol / "MANIFEST.TXT").write_bytes(b"old manifest")
        result = copy_assets.update_assets_to(self.vol, self.assets, changes)
        self.assertEqual((self.vol / "MANIFEST.TXT").read_bytes(), b"new manifest")
        self.assertEqual(result["replaced"], ["MANIFEST.TXT"])
        self.assertFalse((self.vol / "MANIFEST.TMP").exists())
        (self.vol / "MANIFEST.TXT").write_bytes(b"edited by hand")
        result = copy_assets.update_assets_to(self.vol, self.assets, changes)
        self.assertEqual((self.vol / "MANIFEST.TXT").read_bytes(), b"edited by hand")
        self.assertEqual(result["kept"], ["MANIFEST.TXT"])

    def test_replace_without_asset_file_raises(self):
        (self.vol / "GONE.TXT").write_bytes(b"old")
        changes = [("replace", "GONE.TXT", frozenset({self.sha(b"old")}))]
        with self.assertRaises(ValueError):
            copy_assets.update_assets_to(self.vol, self.assets, changes)

    def test_changes_refuse_protected_names(self):
        h = "a" * 64
        for name in ["PETS.CFG", "000000.PCX", "010000.PCX", "BPETS_V3.CFG", "bornpets.cfg"]:
            for op in ("remove", "replace"):
                f = self.assets.parent / "c.txt"
                f.write_text(f"{op} {name} {h}\n")
                with self.assertRaises(ValueError, msg=f"{op} {name}"):
                    copy_assets.load_changes(f)

    def test_load_changes_parses_and_rejects_bad_lines(self):
        h1, h2 = "a" * 64, "B" * 64
        f = self.assets.parent / "c.txt"
        f.write_text(f"# c\n\nremove 080100.PCX {h1} {h2}  # why\nreplace MANIFEST.TXT {h1}\n")
        self.assertEqual(copy_assets.load_changes(f), [
            ("remove", "080100.PCX", frozenset({h1, h2.lower()})),
            ("replace", "MANIFEST.TXT", frozenset({h1})),
        ])
        for bad in ["delete X.PCX " + h1, "remove X.PCX", "remove X.PCX nothex"]:
            f.write_text(bad + "\n")
            with self.assertRaises(ValueError, msg=bad):
                copy_assets.load_changes(f)

    def test_update_removes_before_it_adds(self):
        (self.vol / "080100.PCX").write_bytes(b"shipped")
        changes = [("remove", "080100.PCX", frozenset({self.sha(b"shipped")}))]
        events = []
        real_remove, real_copy = os.remove, shutil.copyfileobj

        def rec_remove(p, *a, **k):
            events.append("remove")
            return real_remove(p, *a, **k)

        def rec_copy(*a, **k):
            events.append("add")
            return real_copy(*a, **k)

        with mock.patch.object(os, "remove", rec_remove), \
                mock.patch.object(shutil, "copyfileobj", rec_copy):
            copy_assets.update_assets_to(self.vol, self.assets, changes)
        self.assertIn("remove", events)
        self.assertIn("add", events)
        last_remove = max(i for i, e in enumerate(events) if e == "remove")
        first_add = min(i for i, e in enumerate(events) if e == "add")
        self.assertLess(last_remove, first_add)

    def test_update_without_changes_behaves_as_before(self):
        result = self.update()
        self.assertEqual(sorted(result["added"]), ["000000.PCX", "010000.PCX", "BPETS_V3.CFG", "PETS.CFG"])
        for key in ("removed", "replaced", "kept", "no_space"):
            self.assertEqual(result[key], [])

    def test_update_removes_a_partial_file_on_no_space(self):
        real_copy = shutil.copyfileobj

        def full_disk(fin, fout, *a, **k):
            if fout.name.endswith("010000.PCX"):
                fout.write(b"0123456789")
                raise OSError(errno.ENOSPC, "No space left on device")
            return real_copy(fin, fout, *a, **k)

        with mock.patch.object(shutil, "copyfileobj", full_disk):
            result = self.update()
        self.assertFalse((self.vol / "010000.PCX").exists())
        self.assertEqual(result["no_space"], ["010000.PCX"])
        self.assertIn("PETS.CFG", result["added"])
        self.assertIn("BPETS_V3.CFG", result["added"])

    def test_replace_no_space_keeps_old_file(self):
        (self.assets / "MANIFEST.TXT").write_bytes(b"new manifest")
        (self.vol / "MANIFEST.TXT").write_bytes(b"old manifest")
        changes = [("replace", "MANIFEST.TXT", frozenset({self.sha(b"old manifest")}))]
        real_copy = shutil.copyfileobj

        def full_disk(fin, fout, *a, **k):
            if fout.name.endswith("MANIFEST.TMP"):
                fout.write(b"01234")
                raise OSError(errno.ENOSPC, "No space left on device")
            return real_copy(fin, fout, *a, **k)

        with mock.patch.object(shutil, "copyfileobj", full_disk):
            result = copy_assets.update_assets_to(self.vol, self.assets, changes)
        self.assertEqual((self.vol / "MANIFEST.TXT").read_bytes(), b"old manifest")
        self.assertFalse((self.vol / "MANIFEST.TMP").exists())
        self.assertEqual(result["no_space"], ["MANIFEST.TXT"])

    def test_volume_budget_counts_clusters_and_entries(self):
        d = self.assets.parent / "budget"
        d.mkdir()
        (d / "a").write_bytes(b"x")
        (d / "b").write_bytes(b"x" * 2048)
        (d / "c").write_bytes(b"x" * 2049)
        (d / "z").write_bytes(b"")
        self.assertEqual(copy_assets.volume_budget(d), (1 + 1 + 2 + 0, 4))

    def test_process_device_reports_no_space(self):
        changes_file = self.assets.parent / "changes.txt"
        changes_file.write_text("# none\n")
        real_copy = shutil.copyfileobj

        def full_disk(fin, fout, *a, **k):
            if fout.name.endswith("PETS.CFG"):
                raise OSError(errno.ENOSPC, "full")
            return real_copy(fin, fout, *a, **k)

        with mock.patch.object(copy_assets, "ASSETS_DIR", self.assets), \
                mock.patch.object(copy_assets, "CHANGES_FILE", changes_file), \
                mock.patch.object(copy_assets, "mount_via_udisks", return_value=self.vol), \
                mock.patch.object(copy_assets, "unmount_via_udisks"), \
                mock.patch.object(copy_assets, "_say") as say, \
                mock.patch.object(shutil, "copyfileobj", full_disk):
            copy_assets.process_device("/dev/x", "CYBR0001", quiet=True, update=True)
        out = "\n".join(str(c.args[0]) for c in say.call_args_list)
        self.assertIn("WARN", out)
        self.assertIn("PETS.CFG", out)

    def test_non_enospc_error_removes_partial_and_propagates(self):
        real_copy = shutil.copyfileobj

        def eio(fin, fout, *a, **k):
            if fout.name.endswith("010000.PCX"):
                fout.write(b"0123")
                raise OSError(errno.EIO, "io error")
            return real_copy(fin, fout, *a, **k)

        with mock.patch.object(shutil, "copyfileobj", eio):
            with self.assertRaises(OSError) as cm:
                self.update()
        self.assertEqual(cm.exception.errno, errno.EIO)
        self.assertFalse((self.vol / "010000.PCX").exists())

        (self.assets / "MANIFEST.TXT").write_bytes(b"new manifest")
        (self.vol / "MANIFEST.TXT").write_bytes(b"old manifest")
        changes = [("replace", "MANIFEST.TXT", frozenset({self.sha(b"old manifest")}))]

        def eio_tmp(fin, fout, *a, **k):
            if fout.name.endswith("MANIFEST.TMP"):
                fout.write(b"01")
                raise OSError(errno.EIO, "io error")
            return real_copy(fin, fout, *a, **k)

        with mock.patch.object(shutil, "copyfileobj", eio_tmp):
            with self.assertRaises(OSError):
                copy_assets.update_assets_to(self.vol, self.assets, changes)
        self.assertFalse((self.vol / "MANIFEST.TMP").exists())
        self.assertEqual((self.vol / "MANIFEST.TXT").read_bytes(), b"old manifest")

    def test_replace_deletes_its_own_stale_tmp(self):
        (self.assets / "MANIFEST.TXT").write_bytes(b"new manifest")
        (self.vol / "MANIFEST.TXT").write_bytes(b"old manifest")
        (self.vol / "MANIFEST.TMP").write_bytes(b"half")
        changes = [("replace", "MANIFEST.TXT", frozenset({self.sha(b"old manifest")}))]
        result = copy_assets.update_assets_to(self.vol, self.assets, changes)
        self.assertEqual((self.vol / "MANIFEST.TXT").read_bytes(), b"new manifest")
        self.assertFalse((self.vol / "MANIFEST.TMP").exists())
        self.assertEqual(result["replaced"], ["MANIFEST.TXT"])
        self.assertEqual(result["kept"], [])

    def test_process_device_bad_changes_file_reports_and_unmounts(self):
        bad = self.assets.parent / "changes.txt"
        bad.write_text("explode X.PCX " + "a" * 64 + "\n")
        with mock.patch.object(copy_assets, "ASSETS_DIR", self.assets), \
                mock.patch.object(copy_assets, "CHANGES_FILE", bad), \
                mock.patch.object(copy_assets, "mount_via_udisks", return_value=self.vol), \
                mock.patch.object(copy_assets, "unmount_via_udisks") as unmount, \
                mock.patch("sys.stderr") as err:
            ok = copy_assets.process_device("/dev/x", "CYBR0001", quiet=True, update=True)
        self.assertFalse(ok)
        unmount.assert_called_once_with("/dev/x")
        out = "".join(str(c.args[0]) for c in err.write.call_args_list)
        self.assertIn("ERR", out)
        self.assertIn("/dev/x", out)
        self.assertIn("explode", out)


if __name__ == "__main__":
    unittest.main()
