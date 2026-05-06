# Contacts screen — meshcore on-badge UI redesign

The badge is more than a passive Bluetooth companion — it has a working chat
UI on its own.  This doc captures the design we agreed on for restructuring
the meshcore-related screens around a single **Contacts** view that surfaces
contacts and adverts together, with a popup menu for per-contact actions.

It's a redesign of the existing `SCREEN_ADVERT` (which today shows only the
*last* advert as a single record) into a list-driven discovery surface.

## Goals

- **Discovery first.**  At a hacker camp the badge's most useful job is "who
  is near me right now?"  The screen is sorted so live nodes float to the top.
- **One press to chat.**  Friend's badge sends an advert → it appears in the
  list → two button presses (Fire, Fire) opens a PM thread.
- **Stay consistent with the rest of the badge.**  Up/Down scrolls, Left/Right
  is the global screen-swipe carousel — no per-screen rebinding of nav keys.
  All per-contact actions live behind a popup menu (a "click").
- **Extensible.**  Per-contact actions (PM, Info, Favorite, future Block /
  Forget / Join channel / etc.) live in a single popup so adding actions
  doesn't burn buttons.

## Architecture

`SCREEN_ADVERT` is replaced by a **Contacts** screen.  The advert *is* a
contact event: every received advert updates `contacts.rs`'s slot for that
`pub_key` (creating it on first sight), and the Contacts screen renders the
contact store sorted by `last_advert_ts` descending.

```
┌────────────────────────────────────────────────┐
│ Contacts                                [85%]  │  ← header (existing pattern)
├────────────────────────────────────────────────┤
│ ●  ☰  alice                              3m    │
│    ☰  bob                                12m   │
│       carol                              1h    │
│       dave (3)                           ydy   │  ← (N) = unread PM count
│    ⚙  borncamp-room-1                    2h    │  ← role glyph (room server)
│       eve                                3d    │
│       (older entries…)                          │
└────────────────────────────────────────────────┘
```

### Row layout

Each row is ~18–20 px tall (≈ 7 visible rows on a 152×152 display once the
header is subtracted).  Three real fields, in priority order:

| Element              | When                                          |
|----------------------|-----------------------------------------------|
| Live dot ●           | `last_advert_ts` is within last few minutes (red) |
| Role glyph           | Only when `node_type ≠ Chat Node`             |
| Name (bold)          | Always; `(N)` suffix shows unread PM count     |
| Last-seen, right-aligned | Always — terse: `3m / 12m / 1h / ydy / 3d` |

Sort: `last_advert_ts` desc.  When `FLAG_FAVORITE` lands in meshcore (the
flag bit is already reserved in `contacts.rs`), favorites become a
tiebreaker / star marker — no sort surgery needed.

### Unknown adverts

Every advert auto-adds a contact slot.  To keep the slot ring from filling
up at a 7-day camp:

- **Live indicator** `last_advert_ts` ≤ 5 minutes lights the red dot.
- **Stale sweep** at boot (and periodically): drop non-favorite contacts not
  heard in > 24 h.  Favorites and any contact with PM history are exempt.

### Passive-screen indicator

Add a small `+N` glyph to the passive (main) screen header showing the
count of new contacts heard since the user last opened the Contacts screen.
No LED chirp by default — easy to get spammy at a busy venue — but a
Settings toggle can turn the chirp on for those who want it.

## Interaction

### List view (top level)

| Button       | Action                                             |
|--------------|----------------------------------------------------|
| Up / Down    | Scroll the list                                    |
| Left / Right | **Screen swipe to neighbour screen — unchanged.**  |
| Fire         | Open popup for selected contact                    |
| Execute      | (reserved — same as Fire for now)                  |
| Cancel       | Back to home / main screen                         |

### Popup menu (modal overlay, ~120×80 px centred)

```
┌──────────────┐
│ alice        │  ← title = selected contact name
├──────────────┤
│ ▶ PM         │  ← preselected (primary action)
│   Info       │
│   ★ Favorite │  ← shows current state ★/☆
│   < Cancel   │
└──────────────┘
```

| Button        | Action                            |
|---------------|-----------------------------------|
| Up / Down     | Pick item                         |
| Fire / Execute| Confirm                           |
| Cancel        | Dismiss popup, back to list       |

#### Role-aware contents

The popup is rebuilt from `node_type`.  Primary item is preselected.

| Role            | Primary       | Secondary | Tertiary    |
|-----------------|---------------|-----------|-------------|
| Chat Node (1)   | **PM**        | Info      | ★ Favorite  |
| Repeater (2)    | **Info**      | ★ Favorite| (Forget)    |
| Room Server (3) | **Join room** | Info      | ★ Favorite  |
| Sensor (4)      | **Read** *(if/when supported)* | Info | ★ Favorite |
| Unknown         | Info          | ★ Favorite| —           |

For Room Server, **Join room** deep-links into the existing channel browser
scoped to that server's channels — no new screen.

### Detail view (clicked-in via popup → Info)

Reachable from the popup's `Info` item.  Per-contact info screen.

```
┌────────────────────────────────────────────────┐
│ alice                                   [85%]  │
├────────────────────────────────────────────────┤
│  Chat Node                                     │
│  Last seen: 14:32                              │
│  Hops: 2                                       │
│                                                │
│  Key:                                          │
│  573b 0ec3 0476 993d                           │
└────────────────────────────────────────────────┘
```

| Button        | Action                                       |
|---------------|----------------------------------------------|
| Fire / Execute| Open PM thread (chat nodes only)             |
| Left / Right  | Prev / next contact (now safe — sub-mode)    |
| Cancel        | Back to Contacts list                          |

All paths return to the Contacts list on Cancel — predictable.

## Implementation notes

- The contacts store (`fw/mesh/contacts.rs`) already holds `name`, `pub_key`,
  `node_type`, `out_path_len`, `last_advert_ts`, `flags` (`FLAG_FAVORITE`),
  and persists across reboots.  The Contacts screen is a new **view** over
  that data — no schema change required.
- The existing `LAST_ADVERT` global / `SCREEN_ADVERT` rendering can be
  removed once the new screen takes over.
- Existing PM and channel-browser screens stay as-is; they're reached via
  the popup actions instead of from a separate top-level entry.
- Stale-sweep runs in an existing async task (boot + periodic).

## Out of scope (for v1)

- Live-only filter — the live-dot already does this visually.
- LED chirp on new advert — Settings toggle, off by default.
- Sensor "Read" action — depends on sensor support landing in meshcore.
- Companion-app companion changes — purely on-device redesign.
