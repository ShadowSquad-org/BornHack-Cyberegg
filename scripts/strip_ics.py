#!/usr/bin/env python3
"""Strip an iCalendar (.ics) file down to what the badge actually parses.

The on-device parser in `src/watch/ics.rs` only consumes:

  * `BEGIN:VEVENT` / `END:VEVENT` brackets
  * `DTSTART` (with optional `;TZID=...:` parameter, optional trailing `Z`)
  * `DTEND`   (same shape)
  * `SUMMARY`

Everything else — `DESCRIPTION`, `UID`, `DTSTAMP`, `LOCATION`, `URL`,
`PRODID`, `X-WR-*`, `CATEGORIES`, etc. — is read into the badge's RAM
buffer and then ignored.  The official Bornhack programme dump is
~70–100 KB because each event carries a multi-line `DESCRIPTION` with
URL boilerplate; stripping those lines collapses it to ~25× smaller,
which actually fits the badge's 16 KiB read buffer.

This script also lets you filter events by date range so you can ship
just the days you care about (e.g. only the festival proper, or
only the day you're leaving).

Usage:

    # Pipe-style — read stdin, write stdout
    curl -sL https://bornhack.dk/bornhack-2025/program/ics/ \\
        | python3 scripts/strip_ics.py > assets/to-badge/ALARMS.ICS

    # File-style with date range and max event cap
    python3 scripts/strip_ics.py \\
        --from 2025-07-16 --to 2025-07-22 --max 60 \\
        program.ics assets/to-badge/ALARMS.ICS

    # Print only stats (no output written)
    python3 scripts/strip_ics.py --stats program.ics

The output uses CRLF line endings (RFC 5545 default), no folded
continuation lines, and no parameters except `TZID` which is preserved
as-is so the badge can keep treating those values as floating local
time.
"""

from __future__ import annotations

import argparse
import re
import sys
from datetime import date, datetime
from pathlib import Path

# Property names we keep verbatim inside each VEVENT.  Anything else is
# dropped, including its continuation lines (lines starting with a
# space or tab — RFC 5545 line folding).
KEEP = {b"DTSTART", b"DTEND", b"SUMMARY"}

# Pattern used to extract the property name from a line.  Handles both
# `NAME:value` (bare) and `NAME;TZID=...:value` (with parameters).
PROP_RE = re.compile(rb"^([A-Z][A-Z0-9-]*)[;:]")


def unfold(lines: list[bytes]) -> list[bytes]:
    """Merge RFC 5545 folded lines.  Continuation lines start with a
    single space (or tab); they belong to the property started on the
    previous line."""
    out: list[bytes] = []
    for line in lines:
        if line[:1] in (b" ", b"\t") and out:
            # Strip the leading whitespace and append.
            out[-1] += line[1:]
        else:
            out.append(line)
    return out


def parse_dtstart_date(value: bytes) -> date | None:
    """Pull a `date` out of a DTSTART value.  Handles both bare
    `YYYYMMDDTHHMMSS[Z]` and `YYYYMMDD` (all-day) forms; ignores
    parameters."""
    # Strip parameters: "TZID=Europe/Copenhagen:20250716T..." → "20250716T..."
    if b":" in value:
        value = value.rsplit(b":", 1)[1]
    if len(value) < 8 or not value[:8].isdigit():
        return None
    try:
        return date(
            int(value[0:4]),
            int(value[4:6]),
            int(value[6:8]),
        )
    except ValueError:
        return None


def parse_iso_date(s: str) -> date:
    return datetime.strptime(s, "%Y-%m-%d").date()


def strip(
    src: bytes,
    date_from: date | None = None,
    date_to: date | None = None,
    max_events: int | None = None,
) -> tuple[bytes, dict]:
    """Return (stripped_ics, stats).  Stats: in_events, out_events,
    in_bytes, out_bytes."""
    # ICS lines may be CRLF or LF; normalise on bytes.
    raw_lines = src.replace(b"\r\n", b"\n").split(b"\n")
    lines = unfold(raw_lines)

    out: list[bytes] = []
    out.append(b"BEGIN:VCALENDAR")
    out.append(b"VERSION:2.0")
    out.append(b"PRODID:-//bornhack-aegg//strip_ics//EN")

    in_event = False
    event_buf: list[bytes] = []
    in_count = 0
    out_count = 0

    for line in lines:
        line = line.rstrip(b"\r")
        if line == b"BEGIN:VEVENT":
            in_event = True
            event_buf = [b"BEGIN:VEVENT"]
            continue
        if line == b"END:VEVENT":
            in_event = False
            in_count += 1
            event_buf.append(b"END:VEVENT")
            event = filter_event(event_buf, date_from, date_to)
            if event is not None:
                if max_events is not None and out_count >= max_events:
                    continue
                out.extend(event)
                out_count += 1
            continue
        if in_event:
            event_buf.append(line)
        # Lines outside VEVENT (PRODID, VERSION, X-WR-*, VTIMEZONE, ...)
        # are dropped; we emit our own minimal calendar header above.

    out.append(b"END:VCALENDAR")

    body = b"\r\n".join(out) + b"\r\n"
    stats = {
        "in_events": in_count,
        "out_events": out_count,
        "in_bytes": len(src),
        "out_bytes": len(body),
    }
    return body, stats


def filter_event(
    buf: list[bytes],
    date_from: date | None,
    date_to: date | None,
) -> list[bytes] | None:
    """Decide whether to keep this VEVENT, and emit only the kept
    properties.  Returns the slimmed-down lines or `None` to drop."""
    kept: list[bytes] = []
    dtstart_value: bytes | None = None
    saw_dtstart = False

    for line in buf:
        if line in (b"BEGIN:VEVENT", b"END:VEVENT"):
            kept.append(line)
            continue
        m = PROP_RE.match(line)
        if not m:
            continue
        name = m.group(1)
        if name not in KEEP:
            continue
        # Extract the value (after the first ':') for date filtering.
        colon = line.index(b":")
        value = line[colon + 1 :]
        if name == b"DTSTART":
            saw_dtstart = True
            dtstart_value = value
        kept.append(line)

    if not saw_dtstart:
        return None  # malformed — DTSTART required for a useful event

    if date_from is not None or date_to is not None:
        d = parse_dtstart_date(dtstart_value or b"")
        if d is None:
            return None
        if date_from is not None and d < date_from:
            return None
        if date_to is not None and d > date_to:
            return None

    return kept


def main() -> int:
    p = argparse.ArgumentParser(
        description="Strip an iCalendar file down to what the badge parses.",
    )
    p.add_argument(
        "input",
        nargs="?",
        type=Path,
        help="Input .ics file (default: stdin).",
    )
    p.add_argument(
        "output",
        nargs="?",
        type=Path,
        help="Output file (default: stdout).",
    )
    p.add_argument(
        "--from",
        dest="date_from",
        type=parse_iso_date,
        help="Drop events before this date (YYYY-MM-DD).",
    )
    p.add_argument(
        "--to",
        dest="date_to",
        type=parse_iso_date,
        help="Drop events after this date (YYYY-MM-DD).",
    )
    p.add_argument(
        "--max",
        dest="max_events",
        type=int,
        help="Cap output to N events (after date filtering).",
    )
    p.add_argument(
        "--stats",
        action="store_true",
        help="Print stats to stderr regardless of output target.",
    )
    args = p.parse_args()

    if args.input is None:
        src = sys.stdin.buffer.read()
    else:
        src = args.input.read_bytes()

    body, stats = strip(
        src,
        date_from=args.date_from,
        date_to=args.date_to,
        max_events=args.max_events,
    )

    if args.output is None:
        sys.stdout.buffer.write(body)
    else:
        args.output.write_bytes(body)

    if args.stats or args.output is not None:
        in_kb = stats["in_bytes"] / 1024
        out_kb = stats["out_bytes"] / 1024
        ratio = stats["in_bytes"] / max(stats["out_bytes"], 1)
        print(
            f"events: {stats['in_events']} → {stats['out_events']}   "
            f"size: {in_kb:.1f} KiB → {out_kb:.1f} KiB ({ratio:.1f}× smaller)",
            file=sys.stderr,
        )

    return 0


if __name__ == "__main__":
    sys.exit(main())
