#!/usr/bin/env python3
"""Factory-floor asset-copy tool for the CyberÆgg badge.

Listens (via inotify) for a freshly-mounted CyberÆgg USB-MSC volume
(label ``CYBRxxxx`` where ``xxxx`` is the device-ID hex) and, when it
finds an *empty* one, copies every file from ``assets/to-badge/`` into
the volume's root.  Designed to slot into the factory workflow:

    1. Worker plugs badge into USB.
    2. First-boot factory test runs to completion (auto-marks KV pass,
       renders the ship-image, halts).
    3. Worker power-cycles the badge — KV pass means it skips the test
       and boots normally.  The App's USB-MSC task enumerates the FAT12
       partition under e.g. ``/run/media/$USER/CYBRA3F7/``.
    4. This script (running continuously on the factory laptop) wakes
       on the inotify event for the new mount, confirms it's empty,
       copies the asset bundle, and reports success.
    5. Worker unplugs + packs.

Usage
-----

    scripts/copy_assets.py            # watch forever
    scripts/copy_assets.py --once     # exit after the first successful copy
    scripts/copy_assets.py --quiet    # suppress per-file progress

Detection
---------

Volume-label regex: ``^CYBR[0-9A-F]{4}$`` (matches ``fw::fat12::format``
in the firmware).  Both ``/run/media/$USER/`` and ``/media/$USER/`` are
watched, plus the legacy bare ``/media/`` for distros that mount there.

A volume is considered "empty" when its root contains no non-system
entries (``.Trash-*``, ``System Volume Information`` ignored).  Anything
else → script SKIPs it as "already provisioned".

Implementation notes
--------------------

Event-driven via the Linux ``inotify(7)`` syscall, called through a
small ``ctypes`` wrapper — no third-party Python deps.  Falls back to
a 1 s poll loop only if ``inotify_init1`` itself errors (very rare,
e.g. running on a non-Linux host).

After a successful copy ``sync(1)`` is invoked so the assets are
flushed before the worker unplugs.  No automatic unmount: the host
desktop environment will release the volume when the cable is pulled.
"""

import argparse
import ctypes
import ctypes.util
import os
import re
import shutil
import struct
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
ASSETS_DIR = REPO_ROOT / "assets" / "to-badge"
VOLUME_PATTERN = re.compile(r"^CYBR[0-9A-F]{4}$")
IGNORED_ENTRIES = {".Trash-1000", ".trash", "System Volume Information"}

# inotify constants (linux/inotify.h)
IN_CREATE = 0x00000100
IN_MOVED_TO = 0x00000080
# Header layout: { int32 wd; uint32 mask; uint32 cookie; uint32 len; char name[len]; }
_EVENT_HEADER_FMT = "iIII"
_EVENT_HEADER_SIZE = struct.calcsize(_EVENT_HEADER_FMT)


# ---------------------------------------------------------------------------
# Mount-root discovery
# ---------------------------------------------------------------------------

def candidate_mount_roots():
    user = os.environ.get("USER", "")
    if user:
        yield Path(f"/run/media/{user}")
        yield Path(f"/media/{user}")
    yield Path("/media")


def is_volume_empty(volume):
    try:
        for entry in volume.iterdir():
            if entry.name in IGNORED_ENTRIES:
                continue
            return False
    except PermissionError:
        return False
    return True


def copy_assets_to(volume, *, quiet):
    files_copied = 0
    total_bytes = 0
    for src in sorted(ASSETS_DIR.iterdir()):
        if not src.is_file():
            continue
        dst = volume / src.name
        if not quiet:
            print(f"    {src.name}")
        shutil.copy2(src, dst)
        files_copied += 1
        total_bytes += src.stat().st_size
    subprocess.run(["sync"], check=False)
    print(f"  → {files_copied} files, {total_bytes // 1024} KiB copied + flushed")
    return files_copied > 0


def process_candidate(path, *, seen, quiet):
    """Apply the volume-pattern + empty checks to a single path."""
    if not VOLUME_PATTERN.match(path.name):
        return False
    if not path.is_dir():
        return False
    if path in seen:
        return False
    seen.add(path)
    if not is_volume_empty(path):
        print(f"SKIP {path}  (not empty — already provisioned?)")
        return False
    print(f"FRESH {path}")
    if copy_assets_to(path, quiet=quiet):
        print(f"DONE  {path}  ✓\n")
        return True
    return False


# ---------------------------------------------------------------------------
# inotify front-end (preferred)
# ---------------------------------------------------------------------------

class _Inotify:
    """Minimal ctypes inotify wrapper — yields (path, name) per event."""

    def __init__(self):
        libc_path = ctypes.util.find_library("c") or "libc.so.6"
        self._libc = ctypes.CDLL(libc_path, use_errno=True)
        self._libc.inotify_init1.argtypes = [ctypes.c_int]
        self._libc.inotify_init1.restype = ctypes.c_int
        self._libc.inotify_add_watch.argtypes = [
            ctypes.c_int, ctypes.c_char_p, ctypes.c_uint32
        ]
        self._libc.inotify_add_watch.restype = ctypes.c_int
        self._fd = self._libc.inotify_init1(0)
        if self._fd < 0:
            err = ctypes.get_errno()
            raise OSError(err, f"inotify_init1 failed: {os.strerror(err)}")
        self._wd_to_path = {}

    def watch(self, path):
        wd = self._libc.inotify_add_watch(
            self._fd, str(path).encode(), IN_CREATE | IN_MOVED_TO
        )
        if wd < 0:
            err = ctypes.get_errno()
            raise OSError(err, f"inotify_add_watch({path}): {os.strerror(err)}")
        self._wd_to_path[wd] = path

    def events(self):
        while True:
            buf = os.read(self._fd, 4096)
            offset = 0
            while offset < len(buf):
                wd, _mask, _cookie, name_len = struct.unpack_from(
                    _EVENT_HEADER_FMT, buf, offset
                )
                name = (
                    buf[offset + _EVENT_HEADER_SIZE : offset + _EVENT_HEADER_SIZE + name_len]
                    .rstrip(b"\x00")
                    .decode(errors="replace")
                )
                offset += _EVENT_HEADER_SIZE + name_len
                root = self._wd_to_path.get(wd)
                if root is not None and name:
                    yield root / name


def watch_inotify(quiet, once):
    seen = set()
    inot = _Inotify()
    watched = []
    for root in candidate_mount_roots():
        if not root.is_dir():
            continue
        try:
            inot.watch(root)
            watched.append(root)
        except OSError as e:
            print(f"WARN  cannot watch {root}: {e}", file=sys.stderr)
    if not watched:
        raise FileNotFoundError(
            "no mount roots exist yet — plug a USB stick once so your "
            "desktop creates /run/media/$USER/, then re-run"
        )
    print(f"Watching (inotify) {', '.join(str(p) for p in watched)}\n")
    # First-pass: handle anything that was already mounted when we started.
    for root in watched:
        for entry in root.iterdir():
            if process_candidate(entry, seen=seen, quiet=quiet) and once:
                return
    # Then block on events.
    for path in inot.events():
        if process_candidate(path, seen=seen, quiet=quiet) and once:
            return


# ---------------------------------------------------------------------------
# Polling fallback
# ---------------------------------------------------------------------------

def watch_polling(quiet, once):
    print("(falling back to 1 s polling — inotify unavailable)\n")
    seen = set()
    while True:
        for root in candidate_mount_roots():
            if not root.is_dir():
                continue
            for entry in root.iterdir():
                if process_candidate(entry, seen=seen, quiet=quiet) and once:
                    return
        time.sleep(1.0)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Auto-copy assets to fresh CyberÆgg badges.")
    parser.add_argument("--once", action="store_true",
                        help="Exit after the first successful copy.")
    parser.add_argument("--quiet", action="store_true",
                        help="Suppress per-file progress output.")
    parser.add_argument("--poll", action="store_true",
                        help="Force polling instead of inotify (debug).")
    args = parser.parse_args()

    if not ASSETS_DIR.is_dir():
        print(f"ERROR: assets directory {ASSETS_DIR} not found", file=sys.stderr)
        sys.exit(1)
    if not any(p.suffix.lower() == ".pcx" for p in ASSETS_DIR.iterdir() if p.is_file()):
        print(f"WARNING: {ASSETS_DIR} has no .PCX files — did the asset bundle build?",
              file=sys.stderr)

    print(f"Asset bundle: {ASSETS_DIR}/")
    try:
        if args.poll:
            watch_polling(quiet=args.quiet, once=args.once)
        else:
            try:
                watch_inotify(quiet=args.quiet, once=args.once)
            except (OSError, FileNotFoundError) as e:
                print(f"WARN  inotify unavailable: {e}", file=sys.stderr)
                watch_polling(quiet=args.quiet, once=args.once)
    except KeyboardInterrupt:
        print("\nBye.")


if __name__ == "__main__":
    main()
