# BornPets v3, plan 9: new-state sprites and throwing up Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** the pet looks the way it feels. Bartholomeus, Cat, Panda, Badger and Starfish each get one new frame for each of ten states: tipsy, wasted, hung over, sore, injured, groggy, stuffed, thirsty, wired and throwing up. Relax finally shows the Relaxing art that is already on the badge. The Slug, and any pet without a file, keeps today's stand-ins. Sport on a stuffed pet always ends in "Bleurgh!": the session is cut short, the belly empties, the floor gets messy and Wash is wanted. The floor litter of plan 7 (crumbs, banana peel, can, sock, pizza box) is redrawn in the pets' sprite style, larger, and baked into flash.

**Architecture:** two parts.

**Part A, the art.** It touches only `assets/` sources, `scripts/` and docs, uses no cargo, and can run now.
- The existing OpenAI pipeline (`scripts/bornpets_ai_frames.py`) makes each state frame from the pet's own shipped frames as reference images. Its output is cut back to the exact frame and palette.
- A new checker (`scripts/bornpets_art_check.py`) proves the export, measures the FAT12 cost, flags outline and size drift, and writes the files the controller turns into the owner's gallery.
- asset-gen `migrate` re-lays out each pet's sheet for the new json5 rows.
- A litter generator (`scripts/bornpets_litter.py`) makes the five floor items the same way, from plan 7's hand bitmaps and shipped frames, into one 152 x 10 strip image (`assets/bornpets-litter.png`).
- Nothing reaches `assets/to-badge` and no sheet or strip is committed as final until the owner has approved the gallery. The litter shares the pets' gallery rounds.

**Part B, the firmware.** It runs after plans 5 and 7 are merged.
- Ten new `DisplayAnim` variants, plus `Relaxing`, on ids `0x29`–`0x32` (and `0x0E`). The presence table is indexed through a slot map, so RAM shrinks by 18 B.
- A pure `GameState::shown_anim(has_art)` picks the art from the status line's first word, else today's `display_anim()` as the stand-in.
- The throw-up is one branch inside the Exercise dose, with six tunables.
- Then the 50 PCX are shipped and the space guard is run.
- The litter strip is generated into `src/game/litter_art.rs` (new slots, one ink plane), replacing plan 7's hand bitmaps in `background.rs`. Then the guide line is added.

**Tech stack:**
- Part A: Python 3 with Pillow, `curl`, OpenAI `POST /v1/images/edits`, asset-gen (read-only sibling repo).
- Part B: Rust, `no_std` firmware (embassy, embedded-graphics), host tests through the `simulator` feature, Typst guide (`make guide`).

**Spec:** `BORNPETS_V3_ART_DESIGN.md`:
- §1 holds the owner's decisions, verbatim;
- §2 the findings;
- §3 the planner's calls.

Litter: design §1.9 (owner) and §3.5 (planner's calls). Also: `BORNPETS_V3_DESIGN.md` §8.3 (the existing-art table) and §14 (backgrounds, for floor items). Roadmap row 9. Interfaces consumed from `BORNPETS_V3_PLAN_5_BATTLES.md` (worktree `../BornHack-Cyberegg-v3battles`) and `BORNPETS_V3_PLAN_7_BACKGROUNDS.md` (`../BornHack-Cyberegg-v3bg`). Format exemplar: `BORNPETS_V3_PLAN_3_DEMO.md`.

**Code base:** worktree `BornHack-Cyberegg-v3art`, branch `ranzbak/bornpets-v3-art`, from local `main` `5e59574`. Part B first rebases onto `main` with plans 5 and 7 merged, then re-anchors these lines (they are at `5e59574`):
- `src/game/engine/to_display.rs`: `DisplayAnim` 28, `display_anim` 90, status groups 141 to 166.
- `src/game/engine/anim_files.rs`: `anim_id` 26 (reserved-range comment 45), `anim_id_for` 157, `frame_count` 170.
- `src/game/sprite_loader.rs`: `PP_MAX` 50, `AA_MAX` 64, `ANIM_PRESENCE` 73, `init` write 120, `count_anim_frames` read 173.
- `src/game/pacing.rs`: `is_action_anim` 23.
- `src/game/engine/doses.rs`: `apply` 217, `exercise` 288 (plan 5 Task 3 moves its injury block into `strain_up`).
- `src/game/engine/causes.rs`: `Cause` 82, `PLAIN` 106, `what` 126, `why` 150, `all` 180, `pack` 190, `unpack` 215, `TORN_BASE` 232.
- `src/game/engine/mod.rs`: `finish_action` 736, `start_action` 816, `exercise` 900, `preview` 985, `visible` 1446.
- `src/game/engine/thresholds.rs`: `thresholds_table!` 68 (`DOSE_EXERCISE_*` 293 to 308), `CLASSIC` 394, `CASUAL` 719, `BORNPETS_CFG_KEYS` 1150, `clamp_pct` 1935, `validate` 1941.
- `src/game/lifecycle.rs`: `display_anim` 757 (sleep floor 765 to 777).
- `src/game/mod.rs`: toast `what`/`why` 393 and 1402, debug anim names 1153.
- `src/game/modal.rs`: `title_text` 975 ("risk: ").
- `src/guide/words.rs`: causes 112.
- `guide/chapters/05-care-actions.typ`: `== Exercise` 346 to 384.
- `tests/engine_v3.rs`: exercise tests 1791 to 1900, `template_parses_to_classic_with_no_edits` 3262.
- `USER_GAMES.md`: "Custom balance" 54.
- Assets: `assets/bornpets-bartholomeus.json5` (rows to `0x1E`), `assets/bornpets-sponsors-{cat,panda,badger,seastar}.json5` (cat to `0x1F`, others to `0x28`), their `assets/bornpets-sprites-*.png`. The README export list is at `README.md` 803 to 817.

## Global Constraints

Every task's requirements include this section.

**Owner rules (verbatim in the design §1; repeated here as the rules a task checks):**
- **Delta engine.** No polling and no fixed-step loop on the badge. A throw-up is one event inside the Exercise dose. No new solver line or timer.
- **Throw-up.**
  - Sport while `fullness > needs::full_hi()` always throws up. There is no dice.
  - Battles are unchanged. Demo pets never pick Sport while stuffed.
  - Toast `Bleurgh!`. Diary `threw up` / `full belly`.
- **The Slug (02).** "The slug is hand drawn, and should not be touched, it should use the standins as today." No Slug file or source changes. The Jellyfish is plan 7's.
- **Stand-ins kept:** wash, grubby, filthy, waterlogged, overweight, underweight, unfit, exhausted. **Shared frames:** sloppy uses Wasted, parched uses Thirsty (design §3.2).
- **Hand-added PCX** (61 files: `0020xx`–`0028xx`, `0120xx`–`0128xx`, `0220xx`–`0228xx`, `0406xx`, `0407xx`): never touch, delete, regenerate or call stale. Never regenerate `MANIFEST.TXT`; edit it by hand. Copy only the 50 new files from a scratch export.
- **Per-badge files:** `000000.PCX`, `010000.PCX`, `PETS.CFG` (and the player's `BPETS_V3.CFG`) are never overwritten.
- **Outline.** An outline change must be uniform all round, on every frame. No frame is patched with a partial outline. A frame that drifts is regenerated.
- **Palette and size.** Black, white, red and transparent by the `classify` rules (asset-gen `template.rs::sample_frame`). Every new frame is 152 x 75.
- **OpenAI key** (the scratchpad `openai.key`, mode 0600):
  - It is read from `--key-file PATH` or `OPENAI_API_KEY_FILE`.
  - It is never printed, logged, put on argv, committed or copied into the repo. The scripts refuse a key file inside the repo or one readable by group or others.
  - Never `cat` it and never echo it in a command.
- **asset-gen** (`../asset-gen`, HEAD `ce65374`, uncommitted owner changes) is read-only.
  - Never edit it, commit, stash or reset there, and never write into its tree.
  - Run its existing `target/release/asset-gen`. Build it only if that lacks `migrate`, and then out of tree (Task A3).
- **GALLERY GATE (owner, 2026-11-14).** Rules:
  - No PCX goes into `assets/to-badge` and no sheet is committed as final until the owner has reviewed the gallery.
  - Part A writes the review files under `target/v3art/review/` in this worktree. The controller builds the gallery (a private claude.ai Artifact with a per-frame feedback field). This plan builds no web page.
  - Owner feedback per frame → regenerate only the flagged frames → a new gallery round.
  - **At most 3 rounds.** After round 3 with frames still open, stop and the controller asks the owner.
- **Space** (FAT12, 2 KiB clusters, 502 total, 511 root entries):
  - The 50 new PCX may use at most **55 clusters** (derivation in Self-review).
  - Every frame over 2048 bytes is simplified first, Badger and Starfish first, by the island merge of Task A2.
  - Plan 7's `test_repo_assets_fit_the_volume` (≥ 16 spare, ≤ 511 entries) must pass.
- **Floor litter (owner, design §1.9):**
  - The five items are generated with the pets' method (reference frames, fixed rectangle, exact cut-back, palette snap, uniform 1 px outline) and go through the same gallery rounds and the same 3-round cap. Manifest entries have `pet = "floor"`.
  - Palette on the badge: black ink only, white and transparent both mean "floor shows". No red (plan 7's `only_the_cross_is_red`); red the model draws is snapped to black and flagged.
  - Slots (planner's call, design §3.5): crumbs (2, 105) 12 x 5, peel (116, 101) 16 x 9, can (16, 100) 12 x 10, sock (134, 100) 14 x 10, pizza box (30, 102) 24 x 8.
  - Baked into flash (one ink plane, 92 B), never a PCX. `scene.rs`, `FLOOR_ITEMS`, the dirt lines, the tufts and the cross are unchanged.
  - The Slug's `021500`–`021700` are references only; never modified.
- **Anim ids:** `0x29`–`0x32` in the design §3.1 order. `Relaxing` is `0x0E`. Ids `0x15`–`0x1F` stay unused and uncatalogued, and their files stay.
- **Save:** none. The save stays `state_v3`, format 3, 177 bytes, with no new field. Never write kv `state`, `game/name` or `BORNPETS.CFG`.
- **Tunables:**
  - The six `THROWUP_*` values are `Thresholds` fields with a Classic and a Casual value and a `BPETS_V3.CFG` key.
  - The template is regenerated (`BPETS_REGEN=1`, plan 3's hook) and the `USER_GAMES.md` key list updated in the same commit.
  - They go in one block after the last field plan 5 and plan 7 added.
- **Width budgets:** toast ≤ 15 columns; cause `what` ≤ 12, `why` ≤ 15. Every new on-screen word goes in a `words()` list; `make guide` must pass. Never touch `GAME.md`.
- **Integer math** in `src/game/engine/` (except `sim.rs`). No `alloc` in engine code. Levels are `u16`, with `P = 655` per percent.
- **R15:** code made dead is removed in the same commit (`AA_MAX` goes in B1). R24: every refusal shows its reason (this plan adds none). R26: "BornPets v3".
- **Cargo** (Part B only): `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo ... -j 2`, one cargo command at a time. Never run `cargo fmt` on the whole tree. Never `git stash`. Python runs as `python3 -I`.
- **Test policy:** each task runs only its own focused tests. The full suite, both firmware builds, the flash gate and the guide build run once, in B6, on the GitHub mirror. B7 fixes what B6 finds.
- **Pushing.** Part A never pushes. Part B pushes only in B6 and B7, to the `github` remote only (`ranzbak/bornhack-firmware-2026-ce`), never Codeberg. Prefix SSH git commands with `SSH_AUTH_SOCK=/run/user/1000/gcr/ssh`.
- **Parallel stages.** Plan 5 owns `doses::exercise`'s `strain_up` split (the controller calls it `tear_if_over`; use the name that landed). Plan 7 owns `copy_assets.py`, `asset-changes.txt`, the scene and the Jellyfish removal in `bornpets_ai_frames.py` (Part A must not edit the `JELLYFISH_*` blocks).
- **Commit footer** on every commit: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Verify (V)**, the last step of every task:
1. Run the task's focused tests and nothing else.
2. `git status --short` shows only the files the task names.
3. No push (except B6 and B7).
4. Report the test names and their results.

## Review Focus

The failure modes most likely to reach a player unnoticed, most likely first. Each is pinned by a test.

1. **A pet shows the wrong picture or none.**
   - A pet missing a file shows a blank pet area or the debug name instead of its stand-in.
   - The Slug changes look.
   - A state frame overrides an action, sleep or leaving.

   Pinned in B2 by `a_pet_without_art_shows_todays_stand_in`, `art_never_overrides_actions_sleep_or_leaving` and `the_sleep_floor_still_wins`, and in B4 by `the_slug_keeps_its_stand_ins` and `five_pets_have_every_state_frame`.
2. **The art breaks the shipped assets.**
   - `migrate` shifts an existing frame.
   - The export overwrites a hand-added, per-badge or placeholder file.
   - A fresh copy or `--update` runs out of space.

   Pinned in A3 by the round-trip `diff` (no changed or removed file), in A4/A5 by `budget` (≤ 55), and in B4 by plan 7's `test_repo_assets_fit_the_volume` and `test_hand_added_pcx_are_kept` plus `test_only_new_art_was_added`.
3. **The throw-up misfires at its edges.**
   - Exactly on the line.
   - The Casual line.
   - Stuffed and over the strain line together.
   - A CFG value that leaves the pet stuffed (it would throw up every session).
   - A battle or a demo pet throwing up.
   - A reboot mid-session.

   Pinned in B3 by `at_the_line_it_does_not`, `casual_uses_its_own_line`, `stuffed_and_strained_only_throws_up`, `a_full_value_that_stays_stuffed_falls_back`, `battles_never_throw_up`, `demo_pet_never_sports_while_stuffed` and `a_reboot_mid_session_still_throws_up`.
4. **The badge sleeps through a picture change, or wakes for nothing.** The shown frame must change only when `Visible` does. Pinned in B2 by `shown_art_wake_is_exact` (2000 seeded states).
5. **The OpenAI key leaks** onto argv, stdout, stderr or a file in the repo. Pinned in A2 by `test_key_never_reaches_argv_or_output`, `test_key_file_inside_repo_is_refused` and `test_key_file_readable_by_others_is_refused`, and in A2b by `test_litter_uses_the_same_key_rules`.
6. **The new litter breaks plan 7's floor.** An item leaves the strip, meets another, floats above the floor, reads as a speck, carries red, or the baked art drifts from the approved strip. Pinned in A2b by `test_slots_fit_the_strip_and_never_meet` and `item_drift`, in B4b by plan 7's own floor tests re-run on the new data, `every_item_rests_on_the_floor`, `ink_rows_match_their_slots` and `test_litter_art_is_current`.

## File map

| File | Part | Change |
|---|---|---|
| `scripts/bornpets_art_check.py` | A | create: `decode_pcx`, `clusters`, `diff_exports`, `new_files`, `budget`, `edge_runs`, `drift`, `write_review`, `LITTER_DIR`; CLI `diff`, `budget`, `review` |
| `scripts/test_bornpets_art_check.py` | A | create |
| `scripts/bornpets_ai_frames.py` | A | `--key-file` and key guards; default model `gpt-image-2.6-sunburst`; profiles bartholomeus, cat, panda; `STATE_BRIEFS`, `STATE_REFS`, `ref_png`, `state_prompt`, `cut_back`, `merge_islands`; subcommands `state`, `simplify`, `list --states`; `frame_index` without the Panda layout pairing for reference-built profiles |
| `scripts/test_bornpets_ai_frames.py` | A | create (mocked `subprocess.run`; no API call) |
| `scripts/bornpets_litter.py` | A | create: `ITEMS`, `SLOTS`, `PLAN7_SKETCH`, `LITTER_REFS`, `item_scale`, `sketch_png`, `ref_canvas`, `litter_prompt`, `cut_item`, `item_drift`, `strip`, `ink_rows`, `rust_source`; CLI `generate`, `assemble`, `rust`, `check` |
| `scripts/test_bornpets_litter.py` | A, B | create (A2b); `test_litter_art_is_current` (B4b) |
| `assets/bornpets-litter.png` | A | create: the 152 x 10 strip (committed after approval) |
| `scripts/README.md` | A | rows for the three scripts and the art workflow |
| `assets/bornpets-bartholomeus.json5`, `assets/bornpets-sponsors-{cat,panda,badger,seastar}.json5` | A | new rows (and placeholder rows for Bartholomeus and Cat) |
| `assets/bornpets-sprites-{bartholomeus,cat,panda,badger,seastar}.png` | A | migrated layout, then the approved art |
| `src/game/litter_art.rs` | B | create, generated: `FLOOR_SLOTS`, `FLOOR_INK` |
| `src/game/background.rs` | B | plan 7's `CRUMBS` … `PIZZA_BOX`, `FLOOR_BITMAPS` and slot values removed; `FLOOR_SLOTS` re-exported; `bitmap()` on byte rows; `TUFT` as bytes; two tests |
| `src/game/engine/to_display.rs` | B | 11 variants; `art_for_word`; `GameState::{is_stuffed, shown_anim}` |
| `src/game/engine/anim_files.rs` | B | ids `0x0E`, `0x29`–`0x32`; `PRESENCE_SLOTS`, `presence_slot` |
| `src/game/sprite_loader.rs` | B | presence table of `PRESENCE_SLOTS`; `AA_MAX` removed |
| `src/game/pacing.rs` | B | `Relaxing`, `ThrowingUp` are action animations |
| `src/game/lifecycle.rs` | B | `display_anim` resolves through `shown_anim` with the pet's files |
| `src/game/mod.rs` | B | debug names for the new variants; toast headline from `toast_what`; `mod litter_art;` |
| `src/game/engine/causes.rs` | B | `Cause::ThrewUp`, `toast_what`, pack code 64 |
| `src/game/engine/doses.rs` | B | `exercise` → `throw_up` when stuffed |
| `src/game/engine/thresholds.rs`, `assets/to-badge/BPETS_V3.CFG`, `USER_GAMES.md` | B | six `THROWUP_*` keys |
| `src/game/engine/sim.rs` | B | `Outcome::throw_ups` |
| `tests/state_art.rs` | B | create: display mapping and file tests |
| `tests/engine_v3_throwup.rs` | B | create: throw-up tests |
| `assets/to-badge/PP{29..32}00.PCX` (50), `assets/to-badge/MANIFEST.TXT` | B | add; 50 manifest lines by hand |
| `assets/asset-changes.txt` | B | the `replace MANIFEST.TXT` line lists the version it replaces |
| `scripts/test_copy_assets.py` | B | new-art tests |
| `.github/workflows/tests.yml` | B | `guide` job: the two `test_bornpets_*` suites |
| `src/guide/words.rs`, `guide/chapters/05-care-actions.typ` | B | `toast_what` words; the full-belly line and joke |
| `guide/chapters/03-reading-your-pet.typ` | B | only if plan 7's `== Sky and floor` text describes how an item looks |
| `BORNPETS_V3_ROADMAP.md` | docs | row 9 |

**Shared names** (both scripts import them from `bornpets_art_check.py`):
```python
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
```
The frame id of a state frame is `f"{seq_prefix}_{state}_0"` (for example `badger_throwing_up_0`). Its file is `f"{prefix}{0x29 + i:02X}00.PCX"` (for example `063200.PCX`).

---

# Part A: the art (runs now; no cargo)

### Task A1: the art checker

**Files:**
- Create: `scripts/bornpets_art_check.py`, `scripts/test_bornpets_art_check.py`.
- Modify: `scripts/README.md` (one table row).

**Interfaces:**
- Consumes: `scripts/fix_badge_pcx.py` `rle_decode_line` (reuse it, do not copy it), `scripts/bornpets_sheet.py`.
- Produces (besides the shared names):
```python
def decode_pcx(path) -> tuple[int, int, list[list[int]]]    # (w, h, rows); 0 black, 1 red, 2 white, 3 transparent
def clusters(nbytes: int) -> int                              # ceil(nbytes / 2048); 0 for 0
def new_files() -> list[str]                                  # the 50 shipped names, sorted
def diff_exports(old_dir, new_dir, prefixes=tuple(p for p, *_ in ART_PETS.values())) -> dict
    # {"changed": [...], "removed": [...], "added": [...]}; only names starting with a prefix; byte compare
def budget(export_dir) -> dict
    # {"files": [(name, bytes, clusters)], "total": int, "over": [names > 2048 B], "fits": total <= NEW_PCX_BUDGET}
def edge_runs(rows: list[list[int]]) -> tuple[int, int]     # (median black run inward, % of edges starting black); see drift
def drift(new: list[list[int]], idle: list[list[int]]) -> dict
    # {"outline": (idle, new), "coverage": (idle, new), "height": (idle, new), "dy": int, "cut_off": bool, "flags": [...]}
def write_review(export_dir, round_no: int, out_dir=REVIEW_DIR, litter_dir=LITTER_DIR) -> Path   # returns the manifest path
LITTER_DIR = REPO / "target/v3art/litter"   # written by bornpets_litter.py (Task A2b)
```
- **`drift`.** Opaque means index ≠ 3.
  - *Outline.* At every transparent-to-opaque step, scanning each row both ways and each column both ways (the frame edge counts as transparent), count the run of black pixels inward. Edges whose first pixel is not black are left out of the width. `outline` is the median run (an integer). `coverage` is the percentage of edges that start black.
  - *Pet part.* The pet is the largest 8-connected opaque component: `height` is its bbox height, and `dy` its bbox centre row minus idle's.
  - `cut_off` is true when the new frame has opaque pixels on a frame border that idle has none on.
  - *Flags:*
    - `"outline"` when `abs(new - idle) >= 1`;
    - `"coverage"` when `new < idle - 10`;
    - `"size"` when `abs(new - idle) * 100 > 15 * idle`;
    - `"position"` when `abs(dy) > 8`;
    - `"cut_off"`.
- **`write_review`** writes under `out_dir`:
  - per pet: `<pet>/sheet.png` (a copy of the working sheet) and `<pet>/idle.png` (its `PP0100.PCX` from `assets/to-badge`);
  - per state: `<pet>/<state>.png`, decoded from the export's PCX, on white;
  - every frame PNG at 4x nearest-neighbour;
  - `manifest.json`:
```json
{"round": 1, "budget": {"total": 51, "limit": 55, "fits": true},
 "frames": [{"pet": "badger", "prefix": "06", "state": "tipsy", "anim_id": "0x29",
             "frame_id": "badger_tipsy_0", "file": "badger/tipsy.png", "idle": "badger/idle.png",
             "sheet": "badger/sheet.png", "pcx": "062900.PCX", "bytes": 1834, "clusters": 1,
             "flags": [], "metrics": {"outline": [2, 2], "coverage": [94, 91], "height": [61, 58], "dy": 1, "cut_off": false}}]}
```
  - **floor** (when `litter_dir/items/` holds the five items, Task A2b): `floor/<item>.png` (the item on white), `floor/<item>_plan7.png` (plan 7's bitmap in its old slot), `floor/strip.png` (the 152 x 10 strip on white) and `floor/scene.png` (the strip under the Panda's idle frame `050100`, 152 x 75, so the size shows against a pet), all 4x. Five more `frames` entries, read from `litter_dir/metrics.json`:
```json
{"pet": "floor", "prefix": null, "state": "crumbs", "anim_id": null, "frame_id": "floor_crumbs",
 "file": "floor/crumbs.png", "idle": "floor/crumbs_plan7.png", "sheet": "floor/strip.png", "pcx": null,
 "bytes": 10, "clusters": 0, "slot": [2, 105, 12, 5], "note": "slot size is the planner's call (design §3.5)",
 "flags": [], "metrics": {"outline": 1, "coverage": 100, "bottom": true, "ink_pct": 41, "red": 0}}
```
  `bytes` is the item's flash cost (its ink plane). The budget line is unchanged: the litter costs no cluster.
  Paths are relative to `out_dir`. It overwrites the PNGs of the round and keeps older `feedback-*.json` files.
- **CLI:**
  - `diff OLD NEW`: prints the three lists; exits 1 on any changed or removed file, or on an added file that is not in `new_files()`.
  - `budget EXPORT`: prints the table; exits 1 unless it fits.
  - `review EXPORT --round N`: prints the manifest path.

- [ ] **Step 1: write the failing tests** in `scripts/test_bornpets_art_check.py` (fixtures are PCX files written by a small encoder in the test, packed like `gen_placeholder_pcx.py`):
```python
def test_decode_round_trips_a_written_pcx(self): ...        # 152x75, all four indices -> decode == input
def test_clusters_at_the_boundary(self): ...                # clusters(0)=0, (1)=1, (2048)=1, (2049)=2
def test_new_files_are_the_fifty(self): ...                 # len 50; "002900.PCX" and "073200.PCX" in; no "02"-prefixed name
def test_diff_flags_changed_removed_and_foreign_added(self): ...  # tmp dirs: one byte changed, one missing, "060E05.PCX" added -> all three listed
def test_diff_ignores_other_prefixes(self): ...             # a changed "030000.PCX" is not reported
def test_budget_counts_and_lists_files_over_2048(self): ...  # 49 files of 1000 B + one 2049 B -> total 51, over == [that name], fits
def test_budget_over_the_limit_does_not_fit(self): ...      # 50 files of 2500 B -> total 100, not fits
def test_drift_same_frame_has_no_flags(self): ...           # a 30x20 white box, 2-px black outline, against itself -> flags == []
def test_drift_thicker_outline_is_flagged(self): ...        # same box, 3-px outline -> "outline" in flags
def test_drift_gap_in_outline_is_flagged(self): ...         # 40 % of the outline removed -> "coverage" in flags
def test_drift_bigger_pet_is_flagged(self): ...             # box 20 % taller -> "size"; moved 10 px down -> "position"
def test_drift_art_on_a_new_border_is_cut_off(self): ...    # opaque pixel on row 0, idle clear -> "cut_off"
def test_review_writes_manifest_and_files(self): ...        # tmp export with the 50 names -> manifest has 50 frames,
                                                            # every "file", "idle" and "sheet" exists and is 4x the frame
def test_review_lists_the_floor_items(self): ...            # tmp litter_dir with five items + metrics.json -> 55 frames;
                                                            # the five have pet "floor", clusters 0, a "slot"; floor/scene.png exists;
                                                            # budget total unchanged; no litter_dir items -> 50 frames
```
- [ ] **Step 2: run and see them fail.** `python3 -I -m unittest discover -s scripts -p 'test_bornpets_art_check.py'`.
- [ ] **Step 3: implement**, then the `scripts/README.md` row.
- [ ] **Step 4: run them and see them pass.** Same command: PASS.
- [ ] **Step 5: commit** `feat(assets): art checker for export diff, space budget and review files`.
- [ ] **Step 6: V.**

### Task A2: the generator makes state frames from the pet's own frames

Owner: "use other frames from the sprite sheet to get good consistency." The current docs (developers.openai.com, read 2026-11-14) mark `gpt-image-2` deprecated and recommend `gpt-image-2.6-sunburst` / `-flare`. `POST /v1/images/edits` takes several `image[]` files, the first one primary. Sizes are `1024x1024`, `1536x1024` and `1024x1536`. Quality is `low` … `max`. `input_fidelity` is not sent.

**Files:**
- Modify: `scripts/bornpets_ai_frames.py`, `scripts/README.md`.
- Create: `scripts/test_bornpets_ai_frames.py`.

**Interfaces:**
- Consumes: A1 `STATES`, `ART_PETS`, `FRAME`, `decode_pcx`; the existing `call_api`, `classify`, `despeckle`, `dekey`, `style`, `PROFILES`.
- Produces:
```python
MODEL = os.environ.get("BORNPETS_MODEL", "gpt-image-2.6-sunburst")
REF_SCALE = 6
CANVAS = (1536, 1024)
REF_ORIGIN = ((1536 - 152 * 6) // 2, (1024 - 75 * 6) // 2)   # (312, 287)
STATE_REFS: dict[str, tuple[int, ...]]   # the design §3.4 table: ids after idle (0x01), frame 0 of each
STATE_BRIEFS: dict[str, str]             # one per state, "{pet}" filled with the profile noun
def api_key(key_file: str | None = None) -> str
def ref_png(prefix: str, aa: int, path: Path) -> None
def state_refs(prefix: str, state: str, work: Path) -> list[Path]   # idle first, then STATE_REFS[state]
def state_prompt(state: str, note: str | None = None) -> str
def cut_back(raw: Image.Image) -> Image.Image      # exact FRAME, badge palette
def merge_islands(img: Image.Image, min_island: int) -> Image.Image
```
- **`api_key`** tries, in order: `key_file` (from the global `--key-file`), `OPENAI_API_KEY_FILE`, `OPENAI_API_KEY`, then `~/.config/openai/api-key`. A file is refused with `sys.exit` when its resolved path is inside `REPO`, or when `st_mode & 0o077`. The message names the path and never the content. The key still reaches curl only through `-K -` on stdin, as today.
- **Profiles** `bartholomeus`, `cat`, `panda`:
  - `noun`, `markings`, `identity` (4 to 6 sentences describing the shipped idle frame, in the style of `BADGER_IDENTITY`), `freedom` (pet-native props), `poses = {}`, `strips = {}`;
  - plus `"cfg"` and `"sheet"` from `ART_PETS`.
  - `PET_SHEET` / `PET_CFG` read `PROFILE.get("sheet")` / `get("cfg")` before the naming convention.
  - `frame_index()` pairs with the Panda only for profiles that have an `anchor_prompt`, so the five pets' differing row layouts load.
- **`ref_png`** decodes `assets/to-badge/{prefix}{aa:02X}00.PCX`. It paints transparent as the magenta `KEY` and upscales x`REF_SCALE` with nearest-neighbour onto a `CANVAS` of `KEY` at `REF_ORIGIN`. Every reference sits in the same rectangle, so the model sees one scale and one place.
- **`state_prompt`** is: identity; then "IMAGE 1 is this {pet} in its idle pose: same character, same outline thickness all round, same size and the same place inside the same rectangle. IMAGES 2 to N are other frames of the same pet: match their line weight and style, not their poses."; the brief; freedom; `style()`; then, if given, "Owner feedback on the previous attempt: {note}".
- **`cut_back`** asserts `raw.size == CANVAS`. It crops the `REF_ORIGIN` rectangle (912 x 450) and resizes it to `FRAME` with `Image.BOX`: an exact 6 x 6 average, so a reference fed straight back returns its own pixels (LANCZOS rings at the red/black edges). Then `dekey`, `classify` per pixel and `despeckle`. There is no rescale to the art's bbox, so the size follows idle.
- **`merge_islands`** repeats until nothing changes: every 4-connected component of one opaque colour with fewer than `min_island` pixels whose 4-neighbours outside it are all one other opaque colour is recoloured to that colour. A component with any transparent neighbour (or one on the frame edge) is never changed, so the silhouette and the outline stay as they are.
- **Subcommands:**
  - `state [--states a,b] [--quality q] [--force] [--note-file F]`: per state, `call_api(state_refs(..), state_prompt(..), q, CANVAS)` → `raw/<fid>.png` → `cut_back` → `frames/<fid>.png`. A cached raw is skipped unless `--force`. `--note-file` is a feedback JSON `{frame_id: note}`: the frame's note goes into its prompt, and `"ok"` frames are skipped.
  - `simplify --only ids --min-island N`: `frames/<fid>.png` → `merge_islands` → `overrides/<fid>.png` (overrides already win in `quantize`/assemble).
  - `list --states`: prints the 10 state frame ids comma-joined.
- **`STATE_REFS`** (design §3.4):
  - tipsy `(0x25, 0x02)`, wasted `(0x25, 0x0B, 0x04)`, hung over `(0x06, 0x07, 0x25)`, sore `(0x03, 0x23, 0x07)`, injured `(0x03, 0x0D, 0x23)`;
  - groggy `(0x0A, 0x08, 0x10)`, stuffed `(0x0C, 0x0B)`, thirsty `(0x09, 0x25)`, wired `(0x0F, 0x02, 0x23)`, throwing up `(0x03, 0x0C, 0x23)`.
- **`STATE_BRIEFS`** (exact copy; every brief ends "Nothing else in the frame."):
  - tipsy: "Tipsy. The {pet} sways happily with a small glass, a silly grin, one little bubble floating up."
  - wasted: "Wasted. The {pet} slumps against nothing, swirly eyes, a hiccup bubble, an empty bottle lying on its side."
  - hung over: "Hung over. The {pet} squints in pain under a red ice pack, a glass of water in its paw, a tiny storm cloud over its head."
  - sore: "Sore. The {pet} winces and rubs one aching shoulder, two small red ache marks beside it."
  - injured: "Injured. The {pet} stands on one leg with the other in a white bandage and a red cross plaster, leaning on a crutch."
  - groggy: "Groggy. Overslept: bleary half-open eyes, messy bed hair, a big yawn, clutching a coffee mug."
  - stuffed: "Stuffed. The {pet} leans back with a round, overfull belly, an empty plate beside it and a small burp puff."
  - thirsty: "Thirsty. The {pet} pants with its tongue out beside an empty glass, one sweat drop on its brow."
  - wired: "Wired. Overexcited: wide eyes, fur on end, jittery motion lines, a red energy-drink can in its paw."
  - throwing up: "Throwing up after sport on a full belly. The {pet} bends over with dizzy eyes and a cartoon splash of sick drawn in WHITE with a thick BLACK outline in front of it, a few wobbly lines. No red anywhere in the sick."

- [ ] **Step 1: write the failing tests** in `scripts/test_bornpets_ai_frames.py`. Set `os.environ["BORNPETS_PET"] = "panda"` before the import. `subprocess.run` is patched to record its call and return a fake `{"data": [{"b64_json": ...}]}` of a `CANVAS` magenta PNG.
```python
def test_key_never_reaches_argv_or_output(self): ...      # key file 0600 in a tmp dir outside REPO with a unique token;
                                                          # call_api(...) -> token not in any argv element, only in `input`;
                                                          # captured stdout/stderr have no token
def test_key_file_inside_repo_is_refused(self): ...       # a 0600 file under REPO/target -> SystemExit; message has no token
def test_key_file_readable_by_others_is_refused(self): ...  # mode 0644 -> SystemExit
def test_default_model_is_current(self): ...              # MODEL == "gpt-image-2.6-sunburst" without BORNPETS_MODEL
def test_state_request_sends_idle_first_then_table_refs(self): ...  # state "throwing_up" for "06": image[] order is
                                                          # 060100, 060300, 060C00, 062300 refs; size=1536x1024
def test_every_state_has_a_brief_and_two_to_four_refs(self): ...   # all STATES; 2 <= len(STATE_REFS[s]) <= 4;
                                                          # every ref PCX exists for all five prefixes in assets/to-badge
def test_ref_png_puts_the_frame_in_the_fixed_rectangle(self): ...   # pixels outside REF_ORIGIN+912x450 are KEY
def test_cut_back_returns_the_exact_frame_and_palette(self): ...    # ref_png(idle) fed back as raw -> FRAME, only the four
                                                          # badge colours, and decode(idle) == cut_back(..) pixel for pixel
def test_merge_islands_merges_enclosed_specks(self): ...  # 2-px black speck inside white -> white
def test_merge_islands_never_touches_the_outline(self): ...  # a 1-px gap pixel next to transparency stays as it was
def test_note_file_skips_ok_frames_and_adds_notes(self): ...  # {"panda_tipsy_0": "ok", "panda_wired_0": "bigger can"} ->
                                                          # one call, its prompt ends with the note
```
- [ ] **Step 2: run and see them fail.** `python3 -I -m unittest discover -s scripts -p 'test_bornpets_ai_frames.py'`.
- [ ] **Step 3: implement.** Do not touch the `JELLYFISH_*` blocks (plan 7 removes them). Add the workflow to `scripts/README.md`: key rules, the commands of A4, and the review directory.
- [ ] **Step 4: run them and see them pass.** Same command: PASS. Then `BORNPETS_PET=badger python3 -I scripts/bornpets_ai_frames.py --help`: still runs.
- [ ] **Step 5: commit** `feat(assets): state frames from the pet's own frames, key file guards`.
- [ ] **Step 6: V.**

### Task A2b: the litter generator

Owner (design §1.9): the floor litter in the pets' sprite style, from existing frames as references, in the same gallery rounds. Slots and storage are the planner's calls of design §3.5.

**Files:**
- Create: `scripts/bornpets_litter.py`, `scripts/test_bornpets_litter.py`.
- Modify: `scripts/README.md` (one row).

**Interfaces:**
- Consumes: A1 `decode_pcx`, `edge_runs`, `LITTER_DIR`; A2 `api_key`, `call_api`, `KEY`, `dekey`, `despeckle` (import `bornpets_ai_frames` with `BORNPETS_PET` set to `panda` when unset, since that module picks its profile at import).
- Produces:
```python
ITEMS = ("crumbs", "peel", "can", "sock", "pizza_box")       # plan 7 slot order
SLOTS = {"crumbs": (2, 105, 12, 5), "peel": (116, 101, 16, 9), "can": (16, 100, 12, 10),
         "sock": (134, 100, 14, 10), "pizza_box": (30, 102, 24, 8)}   # x, y, w, h on the screen
OLD_SLOTS = {"crumbs": (4, 107, 7, 3), "peel": (120, 104, 11, 6), "can": (16, 104, 10, 6),
             "sock": (136, 102, 9, 8), "pizza_box": (29, 105, 16, 5)}  # plan 7 at 7fdea40
STRIP_TOP, STRIP = 100, (152, 10)
SOURCE = REPO / "assets/bornpets-litter.png"
RUST_OUT = REPO / "src/game/litter_art.rs"
PLAN7_SKETCH: dict[str, tuple[str, ...]]   # plan 7's rows as bit strings, an exact copy of CRUMBS ... PIZZA_BOX
LITTER_REFS = {"crumbs": ("001500", "050C00", "021500"), "peel": ("001500", "010C00", "021500"),
               "can": ("052500", "062500", "001600"), "sock": ("001700", "021700", "051000"),
               "pizza_box": ("001500", "060C00", "021500")}
LITTER_BRIEFS: dict[str, str]
def item_scale(item: str) -> int            # min(1400 // w, 900 // h): crumbs 116, peel 87, can 90, sock 90, pizza_box 58
def item_box(item: str) -> tuple[int, int, int, int]   # (x0, y0, w * S, h * S), centred on CANVAS
def sketch_png(item: str, path: Path) -> None          # PLAN7_SKETCH stretched nearest onto w x h, x S into item_box, on KEY
def ref_canvas(name: str, path: Path) -> None          # decode_pcx(assets/to-badge/<name>.PCX), largest integer scale that fits
                                                       # 1400 x 900, centred on a KEY CANVAS, transparent as KEY
def litter_prompt(item: str, note: str | None = None) -> str
def cut_item(raw: Image.Image, item: str) -> tuple[Image.Image, int]   # (w x h in the 4 badge colours, red cells snapped)
def item_drift(img: Image.Image, red: int) -> dict   # {"outline", "coverage", "bottom", "ink_pct", "red", "flags"}
def strip(items: dict[str, Image.Image]) -> Image.Image   # 152 x 10, transparent, each item at (x, y - STRIP_TOP)
def ink_rows(img: Image.Image) -> bytes                # black = 1, MSB first, rows padded to (w + 7) // 8 bytes
def rust_source(strip_img: Image.Image) -> str         # the text of src/game/litter_art.rs
```
- **`litter_prompt`** is: "IMAGE 1 is a rough sketch of a {noun} lying on the floor, inside the box it must fill exactly. IMAGES 2 to 4 are frames from the same game: match their black line weight and flat fills, not their subjects."; then the brief; then "Chunky pixel art on a {w} x {h} grid, each grid cell one flat colour. A 1 pixel black outline all round. Black and white only, no red, no grey, no shading. Nothing outside the box."; then, if given, "Owner feedback on the previous attempt: {note}". The briefs (exact copy):
  - crumbs: "A few scattered bread crumbs, three or four small lumps."
  - peel: "A banana peel lying flat, three floppy peel arms."
  - can: "An empty drink can lying on its side, a dent in the middle."
  - sock: "One dirty sock, an L shape, a hole at the toe."
  - pizza_box: "A flat closed pizza box, a grease stain on the lid."
- **`cut_item`** asserts `raw.size == CANVAS`, crops `item_box`, and reduces it by exactly `S` with `Image.BOX` on the dekeyed RGBA. A cell is black when at least 25 % of its area was dark (luma < 96) or red, white when at least half of it was opaque, else transparent. Red cells are counted and become black. Then `despeckle`.
- **`item_drift`** uses A1 `edge_runs` (the slot edge counts as transparent). Flags:
  - `"outline"` when the median black run is not 1;
  - `"coverage"` when black edges are under 90 %;
  - `"floating"` when no black cell is on the bottom row;
  - `"speck"` when black cells are under 15 % of `w * h`;
  - `"red"` when `red > 0`.
- **`rust_source`** emits exactly:
```rust
//! Generated by scripts/bornpets_litter.py from assets/bornpets-litter.png. Do not edit.
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use super::engine::scene::FLOOR_ITEMS;

/// Slot boxes in slot order: crumbs, peel, can, sock, pizza box (bottom row y 109).
pub const FLOOR_SLOTS: [Rectangle; FLOOR_ITEMS] = [
    Rectangle::new(Point::new(2, 105), Size::new(12, 5)),
    // ... one line per item
];
/// Black pixels of each item, MSB first, `(width + 7) / 8` bytes a row.
pub const FLOOR_INK: [&[u8]; FLOOR_ITEMS] = [
    &[0x00, 0x00, /* ... */],
    // ... one line per item, 16 bytes to a source line
];
```
- **Subcommands** (work dir `LITTER_DIR`):
  - `generate [--items a,b] [--quality q] [--force] [--note-file F]`: per item, `call_api([sketch, refs...], litter_prompt(..), q, CANVAS)` → `raw/<item>.png` → `cut_item` → `items/<item>.png`; then `metrics.json` with each item's `slot`, `bytes` (`len(ink_rows)`), metrics and flags. A cached raw is skipped unless `--force`. The note file is the gallery feedback JSON: key `floor_<item>`; `"ok"` items are skipped.
  - `assemble`: `strip(items/*)` → `SOURCE` (working tree; the gallery gate decides when it is committed).
  - `rust`: `rust_source(SOURCE)` → `RUST_OUT`.
  - `check`: exit 1 unless `RUST_OUT` equals `rust_source(SOURCE)`.

- [ ] **Step 1: write the failing tests** in `scripts/test_bornpets_litter.py` (`subprocess.run` patched as in A2; no API call):
```python
def test_slots_fit_the_strip_and_never_meet(self): ...   # every SLOTS box in y 100..=109, x 0..=151, bottom row 109 or above;
                                                          # pairwise disjoint; same order and count as OLD_SLOTS
def test_sketch_is_plan_7_in_the_fixed_box(self): ...     # sketch_png("can") outside item_box is KEY; the box holds black
                                                          # where the stretched PLAN7_SKETCH has 1
def test_generate_sends_the_sketch_first_then_refs(self): ...  # generate --items sock: image[] order sketch, 001700, 021700, 051000
def test_litter_uses_the_same_key_rules(self): ...        # key file 0644 -> SystemExit; token never in argv or output
def test_cut_item_keeps_a_thin_outline(self): ...         # a CANVAS whose box has a black line S // 5 wide on each cell edge
                                                          # -> every border cell black, the inside white
def test_cut_item_snaps_red_and_counts_it(self): ...      # three red cells -> black, red == 3
def test_drift_flags_speck_floating_and_thick(self): ...  # one black cell -> "speck"; item off the bottom row -> "floating";
                                                          # 2-px outline -> "outline"; a clean 1-px box -> no flags
def test_ink_rows_pad_to_bytes(self): ...                 # 12 x 5 all black -> 10 bytes, each row 0xFF 0xF0
def test_rust_source_round_trips(self): ...               # parse FLOOR_SLOTS and FLOOR_INK out of rust_source(strip) ->
                                                          # SLOTS and ink_rows of each item; total ink bytes 92
def test_note_file_skips_ok_items(self): ...              # {"floor_can": "ok", "floor_sock": "a red stripe"} -> one call (sock)
```
- [ ] **Step 2: run and see them fail.** `python3 -I -m unittest discover -s scripts -p 'test_bornpets_litter.py'`.
- [ ] **Step 3: implement.** Copy `PLAN7_SKETCH` from `../BornHack-Cyberegg-v3bg/src/game/background.rs` at `7fdea40` (read only), then the `scripts/README.md` row.
- [ ] **Step 4: run them and see them pass.** Same command: PASS.
- [ ] **Step 5: commit** `feat(assets): floor litter generator in the pets' style`.
- [ ] **Step 6: V.**

### Task A3: sheet definitions and `migrate`

Owner: "Also update the sprite sheet definition." Controller: edit each json5, `asset-gen migrate`, fill the blank slots, export.

**Files:**
- Modify: the five json5 files and the five sheet PNGs (`ART_PETS`); `scripts/test_bornpets_ai_frames.py` (one test).

**Interfaces:**
- Consumes: A1 `diff_exports`, `ART_PETS`; A2 `frame_index`, `list --states`.
- Produces:
  - each json5 has rows up to `0x32`;
  - row `0x29 + i` is `{ type: "sequence", name: "<seq>_<STATES[i]>", comment: "v3 state art (plan 9)", frames: 1, frame_width: 152, frame_height: 75, }`, in the field order `bornpets_sheet._ENTRY` parses.
- **Bartholomeus** first gets the rows it lacks:
  - `0x1F` `reserved_1f` (152 x 75);
  - `0x20`–`0x28` `hand_added_20` … `hand_added_28`, with comment `"hand-added PCX in assets/to-badge; keep this box empty"`, sizes 72 x 72 for `0x20`–`0x22` and 152 x 75 for `0x23`–`0x28` (as the Panda's battle and action rows).
- **Cat** gets the same `0x20`–`0x28` rows.
- Blank boxes export nothing, so those files stay hand-added.

- [ ] **Step 1: find the tool.** `A=../asset-gen/target/release/asset-gen; $A --help | grep -q migrate`. If that fails, build once, out of its tree:
  - pick a moment when `pgrep -a cargo` shows no plan-5/7 build;
  - run `CARGO_TARGET_DIR=$PWD/target/asset-gen CARGO_BUILD_JOBS=2 nice -n 19 taskset -c 0,1 cargo build --release --locked -j 2 --manifest-path ../asset-gen/Cargo.toml`;
  - then use `A=target/asset-gen/release/asset-gen`.

  Write nothing into `../asset-gen`.
- [ ] **Step 2: export the old set.** Run `$A export <the 9 configs in README.md order> -o target/v3art/export-old` before any edit. Copy each of the five json5 files with its PNG to `target/v3art/old/`, so each config's `image` still resolves.
- [ ] **Step 3: write the failing test** in `scripts/test_bornpets_ai_frames.py`:
```python
def test_frame_index_has_the_states(self): ...  # for every ART_PETS pet: load_sequences(json5) has 51 rows; rows
                                                # 0x29..0x32 are "<seq>_<state>" 152x75 x1; frame_index() holds the 10 state ids
```
  Run `python3 -I -m unittest discover -s scripts -p 'test_bornpets_ai_frames.py'`: FAIL.
- [ ] **Step 4: edit the five json5 files** as above.
- [ ] **Step 5: migrate.** Per pet: `$A migrate target/v3art/old/<json5> assets/<json5>`. Expect "Migrated N/M frames … 10 new frames …" (Bartholomeus 20 new, Cat 19).
- [ ] **Step 6: round trip.** Run `$A export <the 9 configs> -o target/v3art/export-migrated`, then `python3 -I scripts/bornpets_art_check.py diff target/v3art/export-old target/v3art/export-migrated`. Expect changed, removed and added all empty: no frame moved and the new boxes are blank. On any change, stop: the layout of the binary does not match the sheet, so report to the controller.
- [ ] **Step 7: the test passes.** Same unittest command: PASS.
- [ ] **Step 8: commit** `feat(assets): sheet rows for the v3 states (layout only, boxes blank)`. This is not final art, so the gate allows it.
- [ ] **Step 9: V.**

### Task A4: generate, fit, first gallery round

**Files:**
- Modify: the five sheet PNGs, in the working tree only; create `assets/bornpets-litter.png`, also uncommitted. Neither is committed (gallery gate).

**Interfaces:**
- Consumes: A1 `budget`, `drift`, `write_review`, `diff`; A2 `state`, `simplify`, `assemble`, `list --states`; A3 the layout.
- Produces: `target/v3art/review/manifest.json` with round 1, and the files beside it.

- [ ] **Step 1: smoke.** Read https://developers.openai.com/api/docs/models and confirm `gpt-image-2.6-sunburst` is listed; otherwise set `BORNPETS_MODEL` to the current id and report it. Then make one call: `BORNPETS_PET=panda python3 -I scripts/bornpets_ai_frames.py --key-file <scratchpad>/openai.key state --states tipsy --quality low`. Expect a 1536 x 1024 raw and a 152 x 75 frame.
- [ ] **Step 2: generate.** For each pet in `ART_PETS`: `BORNPETS_PET=<pet> python3 -I scripts/bornpets_ai_frames.py --key-file <path> state --quality medium` (50 calls). Rerun any listed as failed.
- [ ] **Step 2b: litter.** `python3 -I scripts/bornpets_litter.py --key-file <path> generate --quality medium` (5 calls), then `assemble`. Regenerate (`--force --items <i>`) any item flagged `floating` or `speck` once before review, with the note "bigger, resting on the floor".
- [ ] **Step 3: assemble.** Per pet: `... --only $(... list --states) assemble`. Only the state boxes change.
- [ ] **Step 4: export and diff.** Run `$A export <the 9 configs> -o target/v3art/export-r1`, then `bornpets_art_check.py diff target/v3art/export-old target/v3art/export-r1`. Expect `added` to be exactly `new_files()`, with nothing changed or removed.
- [ ] **Step 5: fit.** Run `bornpets_art_check.py budget target/v3art/export-r1`.
  - For each name in `over`, Badger (`06`) and Starfish (`07`) first, run `simplify --only <fid> --min-island N` for N = 4, 8, 16, 32, re-assembling and re-exporting after each N, until the file is ≤ 2048 B or N is 32. A frame still over then gets `state --force --states <s> --note-file` with the note "flat fills, no texture, fewer small details".
  - Stop when `budget` fits. Frames from the other pets may stay over 2048 B while the total fits.
- [ ] **Step 6: review files.** Run `bornpets_art_check.py review target/v3art/export-r1 --round 1`. Hand the absolute path of `target/v3art/review/` to the controller, with the budget line, the flagged frames, and the litter's slot sizes and flash bytes (marked as the planner's call). **Stop here**: the controller builds the gallery and collects the owner's feedback.
- [ ] **Step 7: V** (no commit in this task; `git status --short` shows the five sheet PNGs modified and `assets/bornpets-litter.png` untracked, nothing else).

### Task A5: owner review rounds, then the final sheets

**Files:**
- Modify: the five sheet PNGs and `assets/bornpets-litter.png` (committed at the end).

**Interfaces:**
- Consumes: the controller's `target/v3art/review/feedback-<round>.json` (`{frame_id: "ok" | note}`), A2, A1.
- Produces: the committed sheets with every state frame approved. `target/v3art/export-final` (Part B re-exports from the committed sheets).

- [ ] **Step 1: one round** (round `r` = 2 or 3). Per pet with a frame not `"ok"`, run `state --force --note-file target/v3art/review/feedback-<r-1>.json` (only those frames are called). Then `assemble --only <those ids>`, export to `export-r<r>`, `diff`, `budget` (with A4 step 5 if it no longer fits). For a `floor_<item>` not `"ok"`: `bornpets_litter.py --key-file <path> generate --force --note-file target/v3art/review/feedback-<r-1>.json`, then `assemble`. Then `review export-r<r> --round <r>`. If the owner asks for a different slot size, change `SLOTS` (and `OLD_SLOTS` stays), rerun `test_bornpets_litter.py`, and regenerate that item. Hand the path back to the controller and stop.
- [ ] **Step 2: cap.** If round 3 comes back with any frame not `"ok"`, stop. The controller asks the owner. Do not start round 4.
- [ ] **Step 3: finish** when every frame of the latest feedback is `"ok"`:
  - export to `target/v3art/export-final`;
  - `diff target/v3art/export-old target/v3art/export-final` (only the 50 added);
  - `budget` (fits);
  - `python3 -I scripts/check_badge_pcx.py target/v3art/export-final` (exit 0);
  - `bornpets_litter.py assemble` from the approved items, so `assets/bornpets-litter.png` is the approved strip.
- [ ] **Step 4: commit** `feat(assets): v3 state art and floor litter (owner-approved, round <r>)`, with the five sheets and the litter strip only (and `SLOTS` if the owner changed a size). The body lists the budget total and the date of the owner's approval.
- [ ] **Step 5: V.**

---

# Part B: the firmware (after plans 5 and 7 are merged into `main`)

**Before B1:** rebase `ranzbak/bornpets-v3-art` onto `main`, and re-anchor the line numbers above.

Part B consumes these from the merged plans:
- **Plan 5:**
  - `doses::exercise` as split by its Task 3: `strain_up(s, strain, why) -> bool` (or the landed name);
  - `GameState::fight_over(Fought { won, hp_left_pct, costs, paid }) -> u32` and the `Fought` struct;
  - the `BATTLE_*` block in `thresholds.rs`, which ends the existing field list.
- **Plan 7:**
  - `scene::floor_lines() -> [u8; FLOOR_ITEMS]` (index 0 is `FLOOR_CRUMBS_DIRT_PCT`, 40);
  - the merged `sprite_loader.rs`;
  - `scripts/copy_assets.py` `load_changes` and `volume_budget`, plus `assets/asset-changes.txt` with its `replace MANIFEST.TXT` line;
  - the tests `test_repo_assets_fit_the_volume` and `test_hand_added_pcx_are_kept`;
  - the `guide` CI job with its `test_copy_assets.py` step.

### Task B1: anim ids and a lean presence table

**Files:**
- Modify: `src/game/engine/to_display.rs` (variants), `src/game/engine/anim_files.rs`, `src/game/sprite_loader.rs`, `src/game/pacing.rs`, `src/game/mod.rs` (debug names).
- Test: `anim_files.rs` `mod pcx_name_tests` (renamed `mod tests`).

**Interfaces:**
- Produces:
```rust
// to_display.rs: DisplayAnim gains (after Idle)
Relaxing, Tipsy, Wasted, HungOver, Sore, Injured, Groggy, Stuffed, Thirsty, Wired, ThrowingUp,
#[cfg(test)] pub const ALL: [DisplayAnim; 34];   // every variant once (payload variants with 0)
// anim_files.rs
// anim_id: Relaxing 0x0E, Tipsy 0x29, Wasted 0x2A, HungOver 0x2B, Sore 0x2C, Injured 0x2D,
//          Groggy 0x2E, Stuffed 0x2F, Thirsty 0x30, Wired 0x31, ThrowingUp 0x32
pub const PRESENCE_SLOTS: usize = 40;
/// Presence-table slot of an anim id: 0x00..=0x14 -> 0..=20, 0x20..=0x32 -> 21..=39, else None.
/// 0x15..=0x1F are MANIFEST-reserved, shipped, and read by nothing, so they are not catalogued.
pub const fn presence_slot(aa: u8) -> Option<usize>;
```
- `sprite_loader`: `ANIM_PRESENCE: [[AtomicU16; PRESENCE_SLOTS]; PP_MAX]` (720 B, was 738). `init` and `count_anim_frames` index through `presence_slot` (`None`: skip / 0). `AA_MAX` and its comment go (R15). The reserved-range comment in `anim_id` moves into the `presence_slot` doc.
- `pacing::is_action_anim` gains `Relaxing` and `ThrowingUp`. The other nine new variants are idle (the 5-minute calm pace).
- Debug names (`game/mod.rs` ~1153), used only when a pet has no file at all:
  - "RELAXING", "TIPSY", "WASTED", "HUNG OVER", "SORE", "INJURED", "GROGGY", "STUFFED", "THIRSTY", "WIRED", "THROW UP";
  - these are debug text like the existing names, not `words()` entries.

- [ ] **Step 1: write the failing tests** in `anim_files.rs` `mod tests`:
```rust
#[test] fn new_state_ids_are_0x29_to_0x32() { /* anim_id_for(Tipsy) == 0x29 ... ThrowingUp == 0x32; Relaxing == 0x0E */ }
#[test] fn presence_slot_skips_the_reserved_range() { /* 0x14 -> Some(20), 0x15..=0x1F -> None, 0x20 -> Some(21), 0x32 -> Some(39), 0x33 -> None */ }
#[test] fn every_anim_has_its_own_slot() { /* DisplayAnim::ALL: presence_slot(anim_id_for(a)) is Some and < PRESENCE_SLOTS; distinct ids give distinct slots */ }
#[test] fn hand_added_and_battle_ids_keep_slots() { /* 0x20..=0x28 all Some */ }
```
In `pacing.rs` tests: `relaxing_and_throwing_up_pace_as_actions`, `state_art_paces_as_idle` (`Tipsy`, 2 frames → `IDLE_FRAME_SECS`).
- [ ] **Step 2: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --lib -- anim_files pacing`.
- [ ] **Step 3: implement.**
- [ ] **Step 4: run them and see them pass.** Same command: PASS. Then `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo check -j 2 --bin embassy --no-default-features --features embassy-1680-full --target thumbv7em-none-eabihf`, and the same with `--features embassy-game`: both clean.
- [ ] **Step 5: commit** `feat(bornpets): state anim ids and a presence table without the reserved range`.
- [ ] **Step 6: V.**

### Task B2: what the pet shows, with today's stand-ins

**Files:**
- Modify: `src/game/engine/to_display.rs`, `src/game/lifecycle.rs`.
- Create: `tests/state_art.rs` (`mod mapping`).

**Interfaces:**
- Consumes: B1 variants; `status::{status_line, Word}`; `needs::full_hi`.
- Produces:
```rust
/// The state frame for the status line's first word, or None (that word keeps its stand-in).
pub fn art_for_word(w: Word) -> Option<DisplayAnim>;
impl GameState {
    /// Fullness over the stuffed line.
    pub fn is_stuffed(&self) -> bool;            // self.fullness as i32 > needs::full_hi()
    /// What the pet area shows when the pet has the files `has_art` reports; else `display_anim()`.
    pub fn shown_anim(&self, has_art: impl Fn(DisplayAnim) -> bool) -> DisplayAnim;
}
```
- `art_for_word`:
  - Tipsy → `Tipsy`; Sloppy and Wasted → `Wasted`; HungOver, Sore, Injured, Groggy, Stuffed and Wired → their own variant; Thirsty and Parched → `Thirsty`;
  - every other `Word` → `None`.
- `shown_anim`: let `base = self.display_anim()`. Then `art` is:
  - `ThrowingUp` when base is `Exercising` and `is_stuffed()`;
  - `Relaxing` when base is `Playing` and `active_action == Some(Action::Relax)`;
  - for base in {`Critical*`, `Warning*`, `Happy`, `Idle`}: `art_for_word(status_line(self).first)`;
  - else `None`.

  Return `art.filter(|&a| has_art(a)).unwrap_or(base)`. `display_anim()` itself is unchanged and stays the stand-in (the Slug's view).
- `lifecycle::display_anim`:
  - the sleep floor tests the stand-in `s.display_anim()` against its neutral list, as today, and returns `Sleeping` when it applies;
  - otherwise it returns `s.shown_anim(|a| anim_files::frame_count(pet_kind, a) > 0)`;
  - every display caller (`render`, the simulator blit, `embassy.rs`, `input.rs`, shots) already goes through it.

- [ ] **Step 1: write the failing tests** in `tests/state_art.rs` `mod mapping`:
```rust
#[test] fn art_follows_the_first_word() { /* for w in Word::ALL: art_for_word(w) == the table above, exhaustively */ }
#[test] fn a_pet_without_art_shows_todays_stand_in() { /* 2000 seeded states (the oracle seeds): shown_anim(|_| false) == display_anim() */ }
#[test] fn art_wins_when_the_pet_has_it() { /* hydration under the thirsty line, first word Thirsty: shown_anim(|_| true) == Thirsty;
                                              has_art only for Wasted: == display_anim() */ }
#[test] fn sloppy_shows_wasted_and_parched_shows_thirsty() { /* booze at 50 %: Wasted; hydration under the parched line: Thirsty */ }
#[test] fn art_never_overrides_actions_sleep_or_leaving() { /* tipsy pet asleep -> Sleeping; feeding -> Feeding; Leaving -> Leaving; Gone, Hatching unchanged */ }
#[test] fn relax_shows_relaxing() { /* relax() started: Relaxing with art, Playing without; wash() started: Playing even with art */ }
#[test] fn a_stuffed_session_shows_throwing_up() { /* fullness 79 % Classic, exercise() started: ThrowingUp; 78 %: Exercising; no art: Exercising */ }
#[test] fn shown_art_wake_is_exact() { /* 2000 seeded states, has_art = |_| true: shown_anim is the same at every minute
                                         from last_update_tick to next_wake_tick() - 1 (update minute by minute on a clone) */ }
```
In `lifecycle.rs` tests: `the_sleep_floor_still_wins` (the floor set, a tipsy pet, art present → `Sleeping`).
- [ ] **Step 2: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test state_art mapping::` and `... --lib -- lifecycle::tests::the_sleep_floor_still_wins`.
- [ ] **Step 3: implement.**
- [ ] **Step 4: run them and see them pass.** Same commands, plus `... --test guide_shots` (shots still draw): PASS.
- [ ] **Step 5: commit** `feat(bornpets): pets show state art, with today's stand-ins as the fallback`.
- [ ] **Step 6: V.**

### Task B3: throwing up

**Files:**
- Modify: `src/game/engine/doses.rs`, `causes.rs`, `thresholds.rs`, `sim.rs`, `src/game/mod.rs` (toast headline, two places), `assets/to-badge/BPETS_V3.CFG`, `USER_GAMES.md`.
- Create: `tests/engine_v3_throwup.rs`.

**Interfaces:**
- Consumes: B2 `is_stuffed`; plan 5 `exercise` / `strain_up` and `fight_over`; plan 7 `scene::floor_lines`.
- Produces:
```rust
// causes.rs
Cause::ThrewUp            // what "threw up", why "full belly"; pack 64 (const MORE_BASE: u8 = 64); in Cause::all()
pub fn toast_what(self) -> &'static str;   // ThrewUp -> "Bleurgh!", else what()
// doses.rs
fn throw_up(s: &mut GameState);           // the first thing exercise() does when s.is_stuffed(); then return
// sim.rs
pub throw_ups: u32,                       // Outcome: ThrewUp causes in the run (note_cause)
```
- `throw_up`:
  - fullness is **set** to `lvl(THROWUP_FULL_PCT)`;
  - weight `-lvl(THROWUP_WEIGHT_PCT)`, mood `-lvl(THROWUP_MOOD_PCT)`, hydration `-lvl(THROWUP_WATER_PCT)`;
  - dirt `= max(dirt + lvl(THROWUP_DIRT_PCT), lvl(scene::floor_lines()[0]))`, clamped;
  - energy `-lvl(DOSE_EXERCISE_ENERGY_PCT) * THROWUP_ENERGY_SHARE_PCT / 100`;
  - push `Cause::ThrewUp`.

  Nothing else: no strain, no fitness, no fun, no booze, no hangover or groggy change, and no `strain_up` call.
- The toast draws `toast_what()` as its bold line and `why()` below. The diary and the menu's "risk: " title keep `what()`.
- **Tunables** (Classic / Casual, `clamp_pct`). One block after the last plan-5/7 field, in the table, both presets and `BORNPETS_CFG_KEYS`:

| Key | Classic | Casual |
|---|---|---|
| `THROWUP_FULL_PCT` | 45 | 45 |
| `THROWUP_WEIGHT_PCT` | 2 | 2 |
| `THROWUP_MOOD_PCT` | 10 | 10 |
| `THROWUP_WATER_PCT` | 10 | 10 |
| `THROWUP_DIRT_PCT` | 30 | 30 |
| `THROWUP_ENERGY_SHARE_PCT` | 50 | 50 |

  `validate`: `THROWUP_FULL_PCT` not strictly between `FULL_LO_PCT` and `FULL_HI_PCT` → preset value.

- [ ] **Step 1: write the failing tests** in `tests/engine_v3_throwup.rs` (Classic unless named):
```rust
#[test] fn sport_while_stuffed_always_throws_up() {
    // fullness 79 %, strain 0, dirt 10 %, weight 50 %, mood 60 %, hydration 60 %, energy 70 %; exercise() and finish:
    // fullness == 45 * P; weight -= 2 * P; mood -= 10 * P; hydration -= 10 * P; dirt == 40 * P (max(10 + 30, 40));
    // energy -= 5 * P; fitness and strain unchanged; injured_min == 0; take_causes() == [ThrewUp]
}
#[test] fn the_mess_rises_from_a_dirty_floor() { /* dirt 30 % -> 60 % */ }
#[test] fn at_the_line_it_does_not() { /* fullness == full_hi() exactly: a normal session, fitness += 12 * P, no ThrewUp */ }
#[test] fn casual_uses_its_own_line() { /* Casual: 80 % normal session; 84 % throws up */ }
#[test] fn stuffed_and_strained_only_throws_up() { /* strain 55 % (would tear): no injury, no TornMuscle, only ThrewUp */ }
#[test] fn after_throwing_up_the_pet_is_not_stuffed() { /* !is_stuffed(); status_line has no Stuffed; a second session is normal */ }
#[test] fn throw_up_numbers_are_tunables() { /* CFG THROWUP_FULL_PCT=50, THROWUP_ENERGY_SHARE_PCT=100: fullness 50 %, energy -10 % */ }
#[test] fn a_full_value_that_stays_stuffed_falls_back() { /* CFG THROWUP_FULL_PCT=80 -> validate gives 45 */ }
#[test] fn a_reboot_mid_session_still_throws_up() { /* exercise() started stuffed, to_bytes, from_bytes, finish: ThrewUp; SAVE_SIZE 177 */ }
#[test] fn battles_never_throw_up() { /* fullness 90 %: fight_over(Fought { won: true, hp_left_pct: 50, costs: true, paid: false })
                                        -> no ThrewUp; fullness unchanged by the fight */ }
#[test] fn demo_pet_never_sports_while_stuffed() { /* 2000 seeded demo states with fullness 79..=100 %, update 12 h each:
                                                     no ThrewUp, active_action never Exercise */ }
#[test] fn the_menu_marks_sport_risky_when_stuffed() { /* preview(Action::Exercise, None, None) == Some(ThrewUp); at 70 %: not ThrewUp */ }
#[test] fn throw_up_texts_fit() { /* toast_what <= 15 and <= 12 cols, what <= 12, why <= 15; pack/unpack round trip;
                                     Cause::all() contains ThrewUp exactly once */ }
```
In `tests/engine_v3.rs`, nothing new: the existing exercise and template tests must stay green.
- [ ] **Step 2: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test engine_v3_throwup`.
- [ ] **Step 3: implement.**
  - Regenerate the template: `BPETS_REGEN=1 CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test engine_v3 template_`.
  - Add the six keys to the `USER_GAMES.md` key list.
  - Add `throw_ups` to `Outcome` and its printout.
- [ ] **Step 4: run them and see them pass.** `... --test engine_v3_throwup`, `... --test engine_v3 exercise`, `... --test engine_v3 template_`, `... --test battle_v3 rules::`, `... --test save_format`: PASS.
- [ ] **Step 5: commit** `feat(bornpets): sport on a full belly throws up`.
- [ ] **Step 6: V.**

### Task B4: ship the art

**Files:**
- Add: `assets/to-badge/PP{29..32}00.PCX` for PP 00, 01, 05, 06, 07 (50).
- Modify: `assets/to-badge/MANIFEST.TXT`, `assets/asset-changes.txt`, `scripts/test_copy_assets.py`, `.github/workflows/tests.yml`.
- Test: `tests/state_art.rs` `mod files`.

**Interfaces:**
- Consumes: A1 `new_files`, `diff`, `budget`; A5's committed sheets; plan 7's `asset-changes.txt` format, `volume_budget` and tests.

- [ ] **Step 1: write the failing tests.** In `scripts/test_copy_assets.py`:
```python
def test_new_state_art_is_shipped(self): ...      # every name of bornpets_art_check.new_files() is in ASSETS_DIR
def test_manifest_lists_the_new_art(self): ...    # each of the 50 names has one MANIFEST line "<NAME> <SEQ>_F1" (upper case)
def test_only_new_art_was_added(self): ...        # `git diff --name-status <merge-base main> -- assets/to-badge`: 50 "A" lines
                                                  # (the new names), "M" only for MANIFEST.TXT and BPETS_V3.CFG, nothing else
def test_manifest_replace_never_lists_the_current_version(self): ...  # sha256(MANIFEST.TXT) not on the replace line
```
In `tests/state_art.rs` `mod files` (simulator `count_anim_frames` reads `assets/to-badge`):
```rust
#[test] fn five_pets_have_every_state_frame() { /* PetKind(0|1|5|6|7): frame_count == 1 for the 10 state variants; Relaxing >= 1 */ }
#[test] fn the_slug_keeps_its_stand_ins() { /* PetKind(2): frame_count == 0 for the 10; a stuffed-session and a tipsy Slug through
                                               lifecycle::display_anim show display_anim()'s stand-in */ }
#[test] fn new_frames_are_badge_pcx() { /* each of the 50: header 152x75, 2 bpp, 1 plane, RLE */ }
```
- [ ] **Step 2: run and see them fail.** `python3 -I -m unittest discover -s scripts -p 'test_copy_assets.py'` and `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test state_art files::`.
- [ ] **Step 3: export and copy.**
  - Export the configs in the README order after plan 7 (no Jellyfish) to `target/v3art/export-ship`. `bornpets_art_check.py diff target/v3art/export-old target/v3art/export-ship` must list only the 50 as added.
  - Copy exactly those 50 files into `assets/to-badge/`.
  - Append their 50 lines from the export's `MANIFEST.TXT` after the last line of each prefix. This is a hand edit; do not regenerate the manifest.
  - In `asset-changes.txt`, append the sha256 of the `MANIFEST.TXT` that was on `main` to the `replace MANIFEST.TXT` line.
  - In the `guide` CI job, after the `test_copy_assets.py` step, add `python3 -m pip install --user pillow` (if the job lacks it) and `python3 -m unittest discover -s scripts -p 'test_bornpets_*.py'`.
- [ ] **Step 4: run them and see them pass.**
  - The two commands of step 2, with plan 7's `test_repo_assets_fit_the_volume` and `test_hand_added_pcx_are_kept` included in the unittest run: PASS.
  - `python3 -I scripts/check_badge_pcx.py`: exit 0.
  - Report the volume budget (clusters and entries).
  - If the guard fails, the art goes back to A4 step 5. Do not loosen the guard.
- [ ] **Step 5: commit** `feat(assets): ship the v3 state art`.
- [ ] **Step 6: V.**

### Task B4b: the litter in the firmware

Replaces plan 7's hand bitmaps with the approved strip (design §3.5). `scene.rs`, `FLOOR_ITEMS`, the `FLOOR_*_DIRT_PCT` keys, the tufts and the cross stay as plan 7 landed them.

**Files:**
- Create: `src/game/litter_art.rs` (generated by `bornpets_litter.py rust`; never edited by hand).
- Modify: `src/game/background.rs`, `src/game/mod.rs` (`mod litter_art;` beside `pub mod background;`), `scripts/test_bornpets_litter.py`.

**Interfaces:**
- Consumes: A2b `rust_source`, `check`; A5's committed `assets/bornpets-litter.png`; plan 7's `background.rs`.
- Produces: `background::FLOOR_SLOTS` (same name and type, now `pub use super::litter_art::FLOOR_SLOTS;`) and `litter_art::FLOOR_INK`.
- `bitmap(display, origin, width, rows: &[u8])` reads `(width + 7) / 8` bytes a row, MSB first, and draws only black. `TUFT` becomes `[u8; 4]` (`0b10101000`, `0b10101000`, `0b01110000`, `0b00100000`). R15: `CRUMBS`, `PEEL`, `CAN`, `SOCK`, `PIZZA_BOX`, `FLOOR_BITMAPS`, the old slot values and the `u16` path are removed in the same commit.

- [ ] **Step 1: write the failing tests.** In `src/game/background.rs` `mod draw`:
```rust
#[test] fn every_item_rests_on_the_floor() {
    // Sky::Plain, floor 5: for each slot, the black pixels drawn inside it are exactly the set bits of FLOOR_INK[k]
    // at (x + col, y + row); at least one of them is on the slot's bottom row; ink >= 15 % of the slot
}
#[test] fn ink_rows_match_their_slots() {
    // FLOOR_INK[k].len() == h * ((w + 7) / 8); the padding bits past w are 0; total 92 bytes
}
```
In `scripts/test_bornpets_litter.py`:
```python
def test_litter_art_is_current(self): ...   # RUST_OUT == rust_source(Image.open(SOURCE)): the baked art is the approved strip
```
- [ ] **Step 2: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --lib background::` and `python3 -I -m unittest discover -s scripts -p 'test_bornpets_litter.py'`.
- [ ] **Step 3: implement.** `python3 -I scripts/bornpets_litter.py rust`, then the `background.rs` changes above. Update plan 7's doc comments that describe the old bitmaps.
- [ ] **Step 4: run them and see them pass.** The two commands of step 2: PASS. These plan-7 tests run **unchanged** (no line of their code changes; they are written over `FLOOR_SLOTS`, `FLOOR_TOP`/`FLOOR_BOTTOM`, `TUFT_XS` and the cross, so they now check the new data):
  - `background.rs` `mod draw`: `floor_items_are_cumulative_and_never_move`, `slots_and_cross_never_overlap`, `grass_steps_around_the_items`, `only_the_cross_is_red`, `background_stays_in_the_pet_area` (all in the `background::` run above);
  - `background.rs` `mod budget`: `the_sky_switch_is_cheap`, `ssd1675_promotions_stay_bounded`. The items are static and the floor is white day and night, so their numbers should not move: report the before and after `eprintln!` values;
  - `src/game/mod.rs`: `ui_boxes_stay_clear` (dirt 95, all five items): `... cargo test -j 2 --features simulator --lib ui_boxes_stay_clear`;
  - `tests/guide_shots.rs`: `every_shot_is_drawn_at_noon_unless_it_says_night`: `... --test guide_shots`.

  A plan-7 test that fails is a slot or art problem. Fix the slot (back to A5 with the owner) or the art, never the test.
- [ ] **Step 5: commit** `feat(game): floor litter in the pets' sprite style`.
- [ ] **Step 6: V.**

### Task B5: the guide

**Files:**
- Modify: `guide/chapters/05-care-actions.typ` (the `== Exercise` section, after the *Injury* paragraph), `src/guide/words.rs`; `guide/chapters/03-reading-your-pet.typ` only as step 3b says.

- [ ] **Step 1: words.** In `build`, after the causes loop, add `toast_what()` for each cause whose `toast_what() != what()`.
- [ ] **Step 2: the line and the joke** (exact copy; numbers through `v()`, words through `w()`, no bare digits):
```typst
*Full belly.* Exercise on a #w("stuffed") pet (fullness over #v("FULL_HI_PCT")) always ends the
same way: #w("Bleurgh!"). The session stops early: it costs #v("THROWUP_ENERGY_SHARE_PCT") of
the usual energy and gives no fitness and no strain. Fullness drops to #v("THROWUP_FULL_PCT"),
weight by #v("THROWUP_WEIGHT_PCT"), mood by #v("THROWUP_MOOD_PCT") and water by
#v("THROWUP_WATER_PCT"), and the floor gets #v("THROWUP_DIRT_PCT") dirtier. The diary says
#w("threw up"), #w("full belly"). Battles do not mind a full belly.

#tip[Never take a stuffed pet to the gym. It will show you its lunch, and you are the one
holding the mop. Wash comes next.]
```
- [ ] **Step 3b: the floor.** Plan 7's `"floor"` shot (`debug_set_dirt_pct(96)`, expect `floor == 5`) keeps its definition; it is redrawn with the new litter. In the built guide, look at `screen("floor")`: all five items show and none is hidden by the status backing more than in plan 7's shot. Read ch3's `== Sky and floor` table: the item names and `#v("FLOOR_*_DIRT_PCT")` stay. Change a sentence only where it describes an item's look (for example "a tiny can"), to match the approved art.
- [ ] **Step 3: check.** Run `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test guide_words` and then `make guide` (the build uses the same pin through `CARGO_BUILD_JOBS`; run nothing else in parallel). Both pass. `GAME.md` is untouched.
- [ ] **Step 4: commit** `docs(guide): ch5 a full belly and the gym`.
- [ ] **Step 5: V.**

### Task B6: regression and integration on CI

- [ ] **Step 1:** rebase on `main` if it moved, then push `ranzbak/bornpets-v3-art` to `github`: `SSH_AUTH_SOCK=/run/user/1000/gcr/ssh git push github ranzbak/bornpets-v3-art`.
- [ ] **Step 2:** watch every job (`gh run watch` on the mirror).
- [ ] **Step 3: report:**
  - every job's result, and the failing test names with their messages;
  - the SSD1680 release image against `main` before this branch (estimate +1.6 KiB, of which the litter about +32 B);
  - the debug firmware's `.bss` against `main` (expect −18 B);
  - `Outcome::throw_ups` per day for each standard sim routine (a routine that throws up daily is a balance change to report, not to hide);
  - the volume budget.
- [ ] **Step 4:** no fix in this task. Every finding goes to B7.

### Task B7: bug-fix round

- [ ] **Step 1:** fix each B6 finding in its own commit, with the focused test that reproduces it first.
- [ ] **Step 2:** if a sim routine now throws up every day, tell the controller. The routine models a player who eats right before sport; changing it is a balance call for the owner, not a fix.
- [ ] **Step 3:** if flash goes over 960 KiB, remove Lights Out in its own commit (the owner's allowance).
- [ ] **Step 4:** push and watch as in B6. Repeat until green. Report the final numbers.

## On-screen words

| Where | Text |
|---|---|
| Toast (`Cause::ThrewUp`) | `Bleurgh!` / `full belly` |
| Diary | `threw up` / `full belly` |
| Menu title (risk) | `risk: threw up` |
| Debug names (no file at all) | `RELAXING`, `TIPSY`, `WASTED`, `HUNG OVER`, `SORE`, `INJURED`, `GROGGY`, `STUFFED`, `THIRSTY`, `WIRED`, `THROW UP` |

## Self-review

**Spec coverage:**

| Requirement (design §) | Where |
|---|---|
| 1.1 throw-up trigger, every effect, no dice | B3 |
| 1.1 toast and diary texts | B3 (`toast_what`, `what`, `why`), B5 |
| 1.1 demo never sports while stuffed; battles unchanged | B3 (`demo_pet_never_sports_while_stuffed`, `battles_never_throw_up`) |
| 1.1 guide line and joke | B5 |
| 1.1 tunables, Classic and Casual | B3 |
| 1.2 ten states × five pets, one frame each | A2 to A5, B4 |
| 1.2 Slug untouched, stand-ins | B2, B4 (`the_slug_keeps_its_stand_ins`); no Slug file in any task |
| 1.2 shared words; kept stand-ins | B2 (`art_for_word`) |
| 1.2 Relaxing `0E`; Drained `06`/`0A` | B1 (id), B2 (`relax_shows_relaxing`); Drained replaced by the new frames (design §3.2) |
| 1.2 json5 and sheets; `migrate` | A3 |
| 1.2 live badger version | design §2.1 (unsuffixed, 51/51); A3 edits that pair |
| 1.3 measure; simplify Badger/Starfish; guard; root entries | A1 `budget`, A4 step 5, B4 |
| 1.4 hand-added and per-badge files | A3 (blank placeholder rows), A4/A5 `diff`, B4 copy of 50 only, `test_only_new_art_was_added` |
| 1.5 style, size, palette, uniform outline | A2 (references, `cut_back`, `merge_islands`), A1 `drift` |
| 1.6 key handling, current model | A2 |
| 1.6 gallery files, gate, 3-round cap | A1 `write_review`, A4 step 6, A5; Global Constraints |
| 1.6 reference frames per state; drift check | A2 `STATE_REFS`, A1 `drift` |
| 1.7 reserved ids, RAM, ids | design §3.1, B1 |
| 1.8 rules | Global Constraints; V |
| 1.9 litter in the pets' style, from references, same gallery rounds, `pet = "floor"` | A2b, A1 `write_review` (floor), A4 step 2b, A5 |
| 1.9 slot sizes in the strip, clear of the UI, shown as the planner's call | design §3.5; A2b `test_slots_fit_the_strip_and_never_meet`; B4b plan 7's tests re-run |
| 1.9 storage baked into flash | design §3.5 (one ink plane, argued); B4b |
| 1.9 replace plan 7's bitmaps, keep `scene.rs` and thresholds, tests and guide | B4b (the re-run list), B5 step 3b |

**Planner's calls:** all are in the design §3 and marked there:
- ids and the slot map;
- the first-word source of the picture, the fixed sloppy → Wasted mapping, the Slug's own Relaxing frames;
- the throw-up numbers, the floor-line minimum for dirt, what a throw-up leaves alone, the session length, and the toast's second line;
- the reference-frame table and the cut-back;
- the island merge as the simplifier;
- the review directory;
- the litter slots, the no-red palette, one ink plane instead of two, the plan-7 bitmap as image 1, the 25 % cut-back rule and the litter drift flags (design §3.5).

**Type consistency:** checked across tasks:
- Python: `STATES`, `ART_PETS`, `FRAME`, `NEW_PCX_BUDGET`, `REVIEW_DIR`, `new_files`, `diff_exports`, `budget`, `drift`, `write_review`, `STATE_REFS`, `STATE_BRIEFS`, `ref_png`, `state_refs`, `state_prompt`, `cut_back`, `merge_islands`, `api_key(key_file)`;
- litter: `ITEMS`, `SLOTS`, `OLD_SLOTS`, `PLAN7_SKETCH`, `LITTER_REFS`, `item_scale`, `item_box`, `sketch_png`, `cut_item`, `item_drift`, `edge_runs`, `ink_rows`, `rust_source`, `LITTER_DIR`, frame ids `floor_<item>`;
- Rust: `FLOOR_SLOTS`, `FLOOR_INK`, `PRESENCE_SLOTS`, `presence_slot`, the 11 variant names, `art_for_word`, `is_stuffed`, `shown_anim`, `Cause::ThrewUp`, `toast_what`, `throw_up`, `Outcome::throw_ups` and the six key names.

Frame ids use `seq_prefix` (`bartho_`, not `bartholomeus_`).

**Review Focus:** items 1 to 6 are pinned in B2/B4, A3/A4/B4, B3, B2, A2/A2b and A2b/B4b.

**Proportion:** 6 art tasks and 8 firmware tasks. The code is short (one branch, one mapping, one table change). Most of the risk is in the assets, and the plan spends its words there.

**Flash (Part B):** about +1.6 KiB, so about 878 KiB of 960 after plans 5 and 7:
- 11 variants and their arms in `anim_id`, pacing and the debug names: about 0.5 KiB;
- `shown_anim` and `art_for_word`: about 0.3 KiB;
- `throw_up`, the cause and its texts: about 0.2 KiB;
- six keys in two presets with setters and key strings: about 0.4 KiB;
- `presence_slot`: under 0.1 KiB;
- the litter: +92 B of ink, −56 B of plan 7 bitmaps, −4 B of tuft, so about +32 B.

No font. The only image data in flash is the 92 B of litter ink. Measured in B6.

**RAM:** the presence table goes from 738 B to 720 B (−18 B). The alternatives were growing `AA_MAX` to 51 (+180 B) and reusing `0x15`–`0x1E` (0 B, but 18 to 19 shipped files change, including 8 Slug files). No other static or stack grows; `shown_anim` takes a closure by value and allocates nothing.

**FAT12 budget:**

| Item | Clusters | Root entries |
|---|---|---|
| After plan 7 (Jellyfish removed) | 429 used, 73 free | 372 |
| `BPETS_V3.CFG` with plan 5's 48 keys and these 6 (about 13.6 KB) | +1 (6 → 7) | 0 |
| `MANIFEST.TXT` +50 lines × 31 B (9.3 → 10.9 KB) | +1 (5 → 6) | 0 |
| Plan 7 guard | 16 kept free | |
| **Left for the 50 new PCX** | **55** | **+50 → 422 of 511** |
| Owner's estimate at today's per-pet averages | about 57 | |
| Plan target | ≤ 55 (each Badger/Starfish frame ≤ 2048 B first) | |

An updated old badge frees the Jellyfish's 75 clusters through plan 7 and then adds these 50 files as plain additions.

## Needs the owner

One question, conditional. Ask it only if A4 step 5 cannot reach 55 clusters after simplifying and one regeneration:
- May the 18 now-unmapped Drained files (`06`/`0A` of prefixes 00, 01, 05, 06, 07; about 20 clusters) be removed through plan 7's `remove` lines?
- They are shipped art that nothing shows after this plan.

Everything else, including the 3-round gallery cap, is already decided.
