# BornPets v3, stage 9: new-state sprites and throwing up (design)

Plan: `BORNPETS_V3_PLAN_9_ART.md`. Roadmap row 9. Base: local `main` `5e59574`, worktree `BornHack-Cyberegg-v3art`, branch `ranzbak/bornpets-v3-art`.

This spec has three kinds of content, kept apart:
- **§1 Owner decisions**, copied verbatim. They bind.
- **§2 Findings**, measured in the tree at `5e59574`.
- **§3 Planner's calls**, each one marked *(planner's call)*. The owner may overrule any of them.

## 1. Owner decisions (2026-11-14), verbatim

### 1.1 Throw-up mechanic

**Throw-up mechanic** ("funny gross-out moment", not a harsh penalty). Sport (Exercise) while the pet is stuffed, meaning fullness above the stuffed line `needs::full_hi()` (78 % in Classic), always throws up. There is no dice.
- Fullness drops to about 45 %, so the pet is no longer stuffed.
- Weight goes down about 2 %.
- Mood takes a hit about the size of the woken-grumpy hit.
- Hydration goes down about 10 %.
- Dirt goes up: a mess, so Wash is wanted. Plan 7's floor items show it.
- The session is cut short: half the energy cost, no fitness gain, no strain or injury risk.
- Toast: "Bleurgh!". Diary cause: "threw up" / "full belly".
- Demo pets never pick Sport while stuffed.
- Battles are unchanged.
- The guide gets a line and a joke in the sport chapter.
- All numbers are tunables in thresholds, Classic and Casual, in the existing pattern.

Each "about" is a value the plan pins.

### 1.2 New art

**New art, with an OpenAI image token available.** One frame per state per pet. The states are: tipsy, wasted, hung over, sore, injured, groggy, stuffed, thirsty, wired, throwing up (10).
- The pets are Bartholomeus (prefix 00), Cat (01), Panda (05), Badger (06) and Seastar/Starfish (07).
- Owner: "The slug is hand drawn, and should not be touched, it should use the standins as today." The Slug (02) gets no new art. The Jellyfish (08) is removed by plan 7.
- Words that share a frame (controller call, owner saw it): sloppy uses the nearer of tipsy and wasted; parched uses thirsty.
- These keep today's stand-ins: wash, grubby, filthy, waterlogged, overweight, underweight, unfit, exhausted.
- Also map the art that is already on the badge but unmapped: Relaxing `0E` for Relax, and Drained `06`/`0A` for hung over and groggy. The design §8.3 table says this art exists; check whether new hung over and groggy frames replace that mapping.
- Owner: "Also update the sprite sheet definition". The `assets/bornpets-sponsors-<pet>.json5` definitions and the `assets/bornpets-sprites-<pet>.png` sheets must gain the new sequences, so that `../asset-gen` export produces the new PCX. Find which badger sheet/json5 version is live (badger, -v2 or -v3) by comparing with what is in `assets/to-badge`.
- Any pet without art for a new state, such as the Slug, shows that state's current stand-in. Today that is `to_display.rs` `display_anim`, which maps the words onto the Warning/Critical groups.

Relayed by the controller (owner, 2026-11-14): asset-gen lives at `../asset-gen` (separate git repo, HEAD `ce65374`). The sheet definition is updated through its `migrate` command: edit each pet's json5, run `asset-gen migrate` to get the re-laid-out sheet with empty slots, fill those slots with the generated art, then export. asset-gen has uncommitted owner changes. It is read-only to this stage: no edits, no commit, stash or reset. Building and running it is fine, with outputs outside its tree.

### 1.3 Space

**Space (FAT12, 2 KiB clusters, 502 total).**
- After plan 7 removes the Jellyfish there are 429 used and 73 free. Plan 7 keeps a guard of ≥16 spare, and BPETS_V3.CFG grows with plans 5 and 7 (about 7 clusters in total).
- Measured cost per pose frame is 00: 1.00, 01: 1.05, 05: 1.00, 06: 1.21 and 07: 1.37 clusters. So 50 frames is about 57 clusters, which leaves no margin.
- The plan must measure the exported PCX. If the guard would break, it simplifies the Badger and Starfish frames (less fine dither) until it fits. The guard test from plan 7 must pass.
- Also check root-directory entries (limit 511).

### 1.4 Hand-added PCX

**Hand-added PCX: never touch them.** There are 61 files: rows 0x20–0x28 of prefixes 00, 01 and 02, plus menu icons 0406xx and 0407xx. Their source configs lack those rows, so a plain asset-gen export of Bartholomeus or Cat would drop them. Copy only the new rows from a scratch export. Note also that `000000.PCX`, `010000.PCX` and `PETS.CFG` differ per badge.

### 1.5 Look and feel

**Look and feel.** Memory rule: an outline change must be uniform all round, on every frame. A partial bold outline was tried and reverted. New frames must match each pet's existing style, size, palette (black/white/red/transparent, the classification in asset-gen `template.rs::sample_frame`) and frame size. Python helpers exist in `scripts/bornpets_sheet.py`. Earlier helper scripts may be in the session scratchpad (png_to_badge_pcx.py, bold_outline.py, check_badge_pcx.py). Read them, and if the art pipeline needs them, plan to bring them into `scripts/`.

### 1.6 OpenAI key and review

**OpenAI key.** It is at the scratchpad path `openai.key`, mode 0600.
- Plan generation scripts that read it from a path given on the command line or in an env var.
- Never print it, log it, commit it, or put it in the repo.
- Use the current OpenAI image model and API, and look up current docs if needed. Include an owner-visible review step: a contact sheet PNG of all new frames per pet, so the owner can look at them before export.

Owner, later the same day (supersedes the contact-sheet form of the review step): "When you have generated the sprite sheets, show them to me in a gallery so I can review them and give feedback." Relayed by the controller:
- Per pet, the generation task writes the migrated sheet PNG with the new frames filled in.
- Per new frame it writes a PNG crop, upscaled about 4x with nearest-neighbour, with the same pet's existing idle frame beside it for comparison.
- A `manifest.json` lists pet, state, anim id, file path and the cluster cost after PCX export.
- Everything goes under one directory. The controller builds the gallery (a private claude.ai Artifact with a per-frame feedback field). The plan builds no web page.
- Gate: no PCX goes into `assets/to-badge` and no sheet is committed as final until the owner has reviewed the gallery. The loop is: owner feedback per frame, regenerate only the flagged frames, new gallery round. At most 3 rounds, then the controller asks the owner.

Owner, later the same day: "use other frames from the sprite sheet to get good consistency." Relayed by the controller:
- Every new frame is made from the pet's own existing frames as reference images, not from a text prompt alone, through the current model's image-edit reference input.
- The reference set per pet is the idle frame plus the 2–4 existing frames closest to the new state. Examples: tipsy/wasted use the drinking and miserable frames, injured/sore use critical sick, stuffed uses feeding, groggy and hung over use the drained/tired frames, and throw-up uses critical sick and feeding. The plan names the set per state in a table.
- Post-processing forces each output onto the exact frame size and the black/white/red/transparent palette.
- The outline must be uniform all round on every frame. A check step compares the outline width and the pet's bounding box against the idle frame, and flags frames that drift so the gallery can mark them.

### 1.7 Anim ids and RAM

**Anim ids and RAM.** `src/game/sprite_loader.rs` has `AA_MAX = 41` (ids 0x00..=0x28) and a RAM-tight debug build. MANIFEST.TXT reserves 0x15–0x1F for env/icon features; Bartholomeus and Slug ship files 0015xx–001Exx (`ENV_*`/`ICON_*`, `PLACEHOLDER`).
- Find out whether the firmware uses those, and choose ids for the 10 new states.
- Cost the RAM of growing `AA_MAX` against reusing reserved ids.
- If reusing changes or deletes shipped files, say so and keep it inside plan 7's `copy_assets.py --update` replace/remove mechanism.

### 1.8 Rules for every task

Delta engine, no polling. A throw-up is one event inside the Exercise dose. Focused tests per task, then a CI regression on the GitHub mirror and a bug-fix round. Cargo uses 2 cores. Integer math, no alloc. Never write kv `state`, `game/name` or `BORNPETS.CFG`, and the save is `state_v3`. Toast ≤ 15 columns, cause what/why ≤ 12/≤ 15. Every new on-screen word goes in a `words()` list, and `make guide` must pass. Never touch GAME.md. R15: code made dead is removed in the same commit.

## 2. Findings at `5e59574`

1. **Live badger sources.** `bornpets-sponsors-badger.json5` with `bornpets-sprites-badger.png` (no suffix). Decoded pixel by pixel, all 51 shipped `06*.PCX` match its sheet. The `-v2` and `-v3` sheets match 0 of 51; they are stale.
2. **The other sheets reproduce what ships.** Panda 48/48, Seastar 50/50. Bartholomeus 36/38 and Cat 41/43: the four misses are the per-badge start screens `000000`/`010000` and the placeholders `001E00`/`011F00`, which differ from their sheet rows. So even a full export of these two pets would overwrite four files besides dropping the 61 hand-added ones. Only the new files are copied (§1.4).
3. **Row layout.** asset-gen numbers rows by their position in the json5. Panda, Badger and Seastar end at row `0x28`. Bartholomeus ends at `0x1E` and Cat at `0x1F`; rows `0x20`–`0x28` of those two exist only as hand-added PCX.
4. **Reserved ids 0x15–0x1F are unused.** No firmware code reads them. They are shipped as files: `001500`–`001E00` (10 files), `011F00` (1 file) and `021500`–`021C00` (8 Slug files).
5. **RAM.** The presence table `ANIM_PRESENCE` is `PP_MAX 9 × AA_MAX 41 × 2 B = 738 B` of static RAM.
6. **Space.** Shipped PCX cost per prefix (clusters per file): 00 1.02, 01 1.05, 05 1.02, 06 1.20 (10 of 51 files over 2048 B), 07 1.36 (18 of 50 over). A PCX of at most 2048 bytes is exactly one cluster. `MANIFEST.TXT` averages 31 B a line. `BPETS_V3.CFG` averages 38 B a key row (282 keys, 11 208 B at `5e59574`).
7. **Existing pipeline.** `scripts/bornpets_ai_frames.py` already does anchor → `/v1/images/edits` → quantize (the `sample_frame` classification) → assemble, with `curl -K -` so the key never reaches argv. It has profiles for badger, seastar and jellyfish only. It defaults to `gpt-image-2` and asserts that each pet's rows match the Panda's row layout. `png_to_badge_pcx.py` and `check_badge_pcx.py` are already in `scripts/`. The scratchpad copy of `bold_outline.py` is a Seastar-only partial bold outline, the reverted approach.
8. **Current OpenAI image models** (developers.openai.com, read 2026-11-14): `gpt-image-2` is marked deprecated. The docs say "For new integrations, use GPT-Image-2.6 Sunburst or Flare": `gpt-image-2.6-sunburst` (detailed work) and `gpt-image-2.6-flare` (fast). Quality `low|medium|high|xhigh|max|auto`; recommended sizes `1024x1024`, `1536x1024`, `1024x1536`; `input_fidelity` is not sent.
9. **The dose lands at the action's end** (`finish_action` → `doses::apply`), and needs do not age during an action (R23). So "stuffed when Sport starts" and "stuffed when the dose lands" are the same test.
10. **Plain cause codes are full.** Codes 0 to 15 are used and `TORN_BASE` is 16. Cause codes never reach flash: the diary is RAM only.

## 3. Planner's calls

### 3.1 Anim ids and the presence table *(planner's call)*

| id | State | `DisplayAnim` | id | State | `DisplayAnim` |
|---|---|---|---|---|---|
| `0x29` | tipsy | `Tipsy` | `0x2E` | groggy | `Groggy` |
| `0x2A` | wasted | `Wasted` | `0x2F` | stuffed | `Stuffed` |
| `0x2B` | hung over | `HungOver` | `0x30` | thirsty | `Thirsty` |
| `0x2C` | sore | `Sore` | `0x31` | wired | `Wired` |
| `0x2D` | injured | `Injured` | `0x32` | throwing up | `ThrowingUp` |

Relax gets `DisplayAnim::Relaxing` on the existing id `0x0E`.

The RAM options were costed against each other:

| Option | Static RAM | Shipped files changed |
|---|---|---|
| Grow `AA_MAX` to 51 | +180 B (918 B) | none |
| Reuse `0x15`–`0x1E` | 0 B | 18 to 19. The Slug's 8 files would show as its "tipsy" etc. and must be deleted, which touches the Slug. Bartholomeus' 10 would be replaced. A badge whose copies do not hash-match keeps them and shows a 20 x 20 food icon as a pose. |
| **Chosen:** ids `0x29`–`0x32`, presence table indexed through a slot map that skips the unused `0x15`–`0x1F` | **−18 B (720 B, 40 slots)** | none |

The slot map is `0x00..=0x14 → 0..=20` and `0x20..=0x32 → 21..=39`. Ids `0x15`–`0x1F` are not catalogued; no code reads them (§2.4). Their files stay on the badge untouched.

### 3.2 What the pet shows *(planner's call)*

- **Source of the picture.** The art follows the first word of the status line (`status_line(s).first`), so the picture matches the word under it. The stand-in stays exactly today's `display_anim()` (from `status()`), so the Slug and any pet without a file look as they do today.
- **Word to frame:**
  - tipsy → Tipsy;
  - sloppy and wasted → Wasted (the Classic sloppy band 38–70 has its middle at 54, which is 16 from the wasted line and 44 from the tipsy line; a fixed mapping, so no new solver line);
  - hung over, sore, injured, groggy, stuffed and wired → their own frame;
  - thirsty and parched → Thirsty.
  - Every other word keeps its stand-in.
- **Priority.** Only the content and status groups (today's groups 4 to 6) are replaced. An action, sleep, leaving, hatching, gone and hibernating always win, and the sleep display floor still applies.
- **Throwing up.** It shows for the whole Exercise action when the pet was stuffed. It needs no new state (§2.9). Without the file, the stand-in is today's Exercising.
- **Relax** shows Relaxing `0E` for every pet that has it, including the Slug's own hand-drawn `020E` frames, which are not touched. Without it, the stand-in is today's Playing. Wash keeps Playing (owner, §1.2).
- **Drained `06`/`0A`.** The new hung over and groggy frames replace that mapping. The Drained art shows an empty battery, which reads as "tired", not as a hangover. Only the Slug would ever use it, and the Slug keeps today's stand-ins. So `06`/`0A` stay unmapped and their files stay on the badge.
- **Exactness.** The shown animation is a function of the status line, the action and the fullness of a running Exercise. All of these are already in the engine's `Visible` view (fullness does not change during an action). The wake stays exact with no new line or timer.

### 3.3 Throw-up numbers *(planner's call; owner's "about" values pinned)*

| Key | Classic | Casual | Meaning |
|---|---|---|---|
| `THROWUP_FULL_PCT` | 45 | 45 | fullness is set to this; `validate`: `FULL_LO_PCT < v < FULL_HI_PCT`, else preset |
| `THROWUP_WEIGHT_PCT` | 2 | 2 | weight drop |
| `THROWUP_MOOD_PCT` | 10 | 10 | mood drop (= `GRUMPY_PCT`, the grumpy-wake hit) |
| `THROWUP_WATER_PCT` | 10 | 10 | hydration drop |
| `THROWUP_DIRT_PCT` | 30 | 30 | dirt rise; dirt also ends at least on the first floor-item line (plan 7 `FLOOR_CRUMBS_DIRT_PCT`, 40), so the mess shows and Wash is open |
| `THROWUP_ENERGY_SHARE_PCT` | 50 | 50 | share of `DOSE_EXERCISE_ENERGY_PCT` the cut-short session costs |

Casual equals Classic. Its later stuffed line (83) already makes it gentler, as with `GRUMPY_PCT`.

- **Trigger.** Strictly above the line: `fullness > needs::full_hi()`.
- **What a throw-up does not change.** Fun, booze, hangover, groggy, fullness loss from sport, fitness and strain all stay as they are. Leaving booze alone keeps throwing up from becoming a way to sober up.
- **Session length.** The action keeps its `EXERCISE_DURATION`. "Cut short" is in the effects.
- **Texts.**
  - New `Cause::ThrewUp`: `what` "threw up", `why` "full belly", toast headline "Bleurgh!" through a new `Cause::toast_what`.
  - The toast reads "Bleurgh!" over "full belly", and the diary "threw up" / "full belly". The menu's risk title reads "risk: threw up" (the one `what` text, like "risk: torn muscle").
  - Pack code 64, a new range after `BAD_NIGHT_BASE` (48 + 8).
- **Menu and hint.** The menu and hint warn for free. `preview` runs the real dose on a clone, so Sport on a stuffed pet is marked risky, and the hint prefers a clean remedy when one exists. This is existing behaviour: nothing is added.
- **Demo mode.** The demo pet never exercises (plan 3), which covers the owner's rule. A test pins it so a later policy change cannot break it.
- **Save.** No save byte. The throw-up is decided from saved fields, and the cause queue is RAM only.

### 3.4 Art pipeline *(planner's call)*

- **Generator.** Extend the existing `scripts/bornpets_ai_frames.py`; no second generator.
  - Profiles are added for bartholomeus, cat and panda.
  - The 10 state briefs are shared, with pet-specific props from each profile's freedom text.
  - The default model becomes `gpt-image-2.6-sunburst`.
  - The key is read from `--key-file PATH` or `OPENAI_API_KEY_FILE`. The script refuses a key file inside the repo or one readable by group or others.
- **Reference images (owner, §1.6).**
  - Every state frame is one `/v1/images/edits` call with the pet's idle frame as `image[]` 1 and the frames of the table below as further `image[]` entries (at most 5 in all; the docs take several, and the first is the primary).
  - The references are decoded from the shipped `assets/to-badge` PCX, which is the badge truth and also covers Bartholomeus' and Cat's hand-added rows `0x23`/`0x25`.
  - Each reference is pasted at one fixed integer scale and position on the request canvas. The output is cut back out of that same rectangle and scaled down by the same factor, so the new frame keeps the idle frame's size and place exactly.

| State | References after idle (frame 0 of each row) |
|---|---|
| tipsy | drinking `25`, happy `02` |
| wasted | drinking `25`, warning miserable `0B`, critical tired `04` |
| hung over | critical drained `06`, warning sick `07`, drinking `25` |
| sore | critical sick `03`, exercising `23`, warning sick `07` |
| injured | critical sick `03`, healing `0D`, exercising `23` |
| groggy | warning drained `0A`, warning tired `08`, sleeping `10` |
| stuffed | feeding `0C`, warning miserable `0B` |
| thirsty | warning hungry `09`, drinking `25` |
| wired | playing `0F`, happy `02`, exercising `23` |
| throwing up | critical sick `03`, feeding `0C`, exercising `23` |

- **Post-processing.** The palette goes through the existing `classify` (the `sample_frame` rules) and `despeckle`. The size is exact by the cut-back (above).
- **Drift check.** It compares each new frame with the pet's idle frame. Each check flags the frame for the gallery:
  - median outline width, from the silhouette inward: drift of 1 px or more;
  - outline coverage (share of silhouette edges that start black): more than 10 points below idle;
  - the pet's bounding-box height (its largest opaque 8-connected part): more than 15 % off;
  - the bounding-box centre: more than 8 px off;
  - any art cut off at the rectangle edge.

  A flagged frame is regenerated, never patched: no automatic outline edit, because a partial outline change was reverted (§1.5).
- **Not brought into `scripts/`.** `bold_outline.py` (the reverted outline approach) and the ad-hoc scratchpad viewers stay out. A new `scripts/bornpets_art_check.py` does the export diff, the space budget and the review files.
- **asset-gen.** Use the existing `../asset-gen/target/release/asset-gen`, which is newer than its sources. Build it once, out of tree, only if that binary lacks `migrate`. A round-trip export check proves that `migrate` changed no existing frame.
- **Space rule.**
  - Budget for the 50 new PCX: 73 free after plan 7, minus 1 for `BPETS_V3.CFG` (7 clusters with plan 5's 48 keys and these 6), minus 1 for `MANIFEST.TXT` (+50 lines, 5 to 6 clusters), minus the 16 guard = **55 clusters**.
  - Every frame over 2048 B is simplified, Badger and Starfish first. The simplifier only merges small enclosed islands of one colour into the colour around them. It never touches transparency or anything next to it, so the silhouette and outline stay as they are (§1.5).
  - Root entries: 372 + 50 = 422 of 511.
- **Shipping.** Only the 50 files `PP29xx`–`PP32xx` for PP 00, 01, 05, 06, 07 are copied. `MANIFEST.TXT` is hand-edited and its new version listed in plan 7's `asset-changes.txt` `replace` line. New files need no change line, because `--update` adds missing files.
- **Review directory.** `target/v3art/review/` in this worktree (gitignored).
