# Showcase plan index

Master index for the 20 feature plans under `docs/plans/`. Read this, then
execute plans in the batch order below without reading all of them first.
Inputs read in full this session: `_survey/seams.md`, `_survey/ecosystem.md`,
and plans 01-20. Repo baseline facts re-verified on disk: 10 sidebar pages
(`src/shell/sidebar.rs:9-20`), page-count assert 10 (`tests/e2e_navigation.rs:14-19`),
`src/ui/mod.rs` and `src/features/mod.rs` are single/three-line module lists,
digit keys bind only `i < 9` (`src/shell/root/app_root/state.rs:144-152`),
`CapabilityStatus` has five pub fields (`src/runtime/capabilities.rs:6-12`).
gpui/gpui-kit sources located on disk per survey key (`ls
~/.cargo/registry/src/*/ | grep -E 'gpui-pre-0.3.8|gpui-kit-0.7.1'` → both
present); this index restates no kit API beyond what the survey and plans
already cite.

## 1. Mission

The showcase turns the gpui-starter boilerplate into a gallery of removable
feature demonstrations: a shared canvas engine, a floating-window toolkit,
GPU/audio/system surfaces, and full app-style pages, each exercising one
capability of the gpui-kit stack. Standing rule for every plan: the feature is
removable, gallery-style. Each plan carries an Interface Contract, a Seam
Touches table, and a Removal Recipe modeled on the `src/gallery/mod.rs:1-7`
header (delete the feature directory, revert the enumerated seam touches, gates
stay green). Nothing routes through a feature that cannot be deleted in one
afternoon. Survey `_survey/seams.md` is the seam canon; plans cite it rather
than re-derive it.

## 2. Plan catalog

Phases counted from `## Phase N` headings (20 uses `### Phase`; 6). "Deps"
lists only plans each plan's own Dependencies section names as consumed.

| id | title | scope (one line) | deps | UI | platform gates | phases |
|---|---|---|---|---|---|---|
| 01 | Canvas foundation | `src/ui/canvas/` engine: custom Element, scene/layers, tools, pan/zoom, shape cache | none | widget only | none (wasm-clean) | 7 |
| 02 | Floating window toolkit | `src/ui/floating/`: spawn/close/respawn borderless floats, probe, wasm overlay fallback | none | gallery section | macos/win/linux native; wasm overlay | 7 |
| 03 | Notch floater | macOS notch-area hover panel (clock, mock widgets); demo float elsewhere | 02 | gallery section | macOS real; others demo float | 6 |
| 04 | Menu bar charts | tray-click CPU/memory charts popover (macOS) / sheet (Win, Linux via command) | 02 (Phase 4 only) | popover/sheet/tray | macOS anchored; wasm no entry | 7 |
| 05 | Canvas showcase | sidebar page, six isolated canvas demos (shapes, bezier, nodes, particles, text, image) | 01 | sidebar page | none | 7 |
| 06 | Infinite canvas | sidebar page, 1k/5k/20k procedural note field, minimap, LOD, perf budget | 01 | sidebar page | none (pinch inert on wasm) | 7 |
| 07 | Canvas shaders | sidebar page, four WGSL demos blitted through the wgpu bridge | 01, 11 (before Phase 4) | sidebar page | native full; wasm reason panel | 8 |
| 08 | Multiuser canvas | sidebar page, socket.io live sync + LWW board, demo peers, collab-server crate | 01 | sidebar page | live sync native; wasm demo peers | 8 |
| 09 | Floating UI | sidebar page, three tool palettes as always-on-top windows or in-window panels, persisted | 02 | sidebar page | native windows; wasm panels | 9 |
| 10 | Floating canvas | gallery section: shared-state mini-paint float, live HUD floats, CPU shader panel | 01, 02 (07+11 stretch only) | gallery section | none (plan 02 handles wasm) | 6 (5 + stretch) |
| 11 | WGPU embed | `src/features/wgpu/` bridge (`WgpuEngine`, frame helpers) + particle/mesh/post demos page | none | sidebar page | native full; wasm reason panel | 8 |
| 12 | Real app UI (analytics) | sidebar page: KPI cards, filters, charts, sortable table, detail sheet, state previews | none | sidebar page | none | 7 |
| 13 | Music player | Music page, album grid, playlist reorder, queue, persistent now-playing bar, simulated engine | none | sidebar page + shell bar | none (Phase 6 rodio native-only, optional) | 6 (5 + optional) |
| 14 | Image editor | sidebar page: open image, brush/shapes/crop/rotate/filters, undo, save via desktop_actions | 01 | sidebar page | io native-only; wasm disabled buttons | 7 |
| 15 | Tutorial mode | spotlight overlay + target registry + demo tour over Home/Gallery | none | overlay + command | none (a11y live regions inert on wasm) | 6 |
| 16 | Waveform foundation | `src/ui/waveform/`: SPSC ring, min/max/RMS pyramid, FFT spectrum, Element | none | widget only | none (wasm-clean) | 6 |
| 17 | System controls | sidebar page: battery/bluetooth/volume/brightness rows, capability-gated | none | sidebar page | per-OS matrix; wasm all-unsupported | 8 |
| 18 | Microphone | Microphone page: cpal capture, permission, gain, level meter, waveform | 16 | sidebar page | capture native; wasm synthetic demo | 6 |
| 19 | Audio playback | `src/services/audio/` decode+output engine + Audio page (scrub, devices, monitor) | 16 | sidebar page | playback native; wasm read-only | 8 |
| 20 | Release bundling | debug artifacts as real bundles (.app zip, linux payload), smoke steps, web download fixes | none (coordinates with 18) | none (CI/web only) | none | 6 |

Peer (non-dependency) coordinations, stated in the plans themselves:
13↔19 (19's engine can replace 13's Phase 6 rodio; 13 Dependencies),
18↔19 (single `cpal = "0.18"` not-wasm entry + `libasound2-dev`; 18/19 Seam
Touches), 18↔20 (byte-identical `NSMicrophoneUsageDescription` string,
grep-guarded; 18 Dependencies, 20 Step 1.2), 05/11/14 (one `image` entry, see
§5), 02→03 (03 needs 02's `NSScreen` feature line).

## 3. Dependency DAG and implementation batches

```text
foundations (no deps)         single-dep consumers                two-dep
01 canvas ────────────────┬── 05 showcase     06 infinite
                          ├── 08 multiuser    14 image editor
11 wgpu ──────────────────┴── 07 shaders (needs 01 + 11; 11 before Phase 4)
02 floating ─┬─ 03 notch
             ├─ 04 menubar (Phases 1-3 build without 02; Phase 4 gated)
             └─ 09 floating ui
01 + 02 ───────────────────── 10 floating canvas (stretch 6 also needs 11)
16 waveform ─┬─ 18 microphone
             └─ 19 audio playback
12 analytics · 13 music · 15 tutorial · 17 system controls · 20 bundling
  (no plan deps; publish contracts or coordinate)
```

Every edge above is drawn from the dependent plan's own Dependencies section;
no edges were inferred. Batches are sized so one implementer lands one feature
per branch/PR (repo PR process, CONTRIBUTING.md; Conventional Commits per
AGENTS.md), gates green at each merge:

| batch | plans | order inside batch |
|---|---|---|
| 1. foundations | 01, 02, 11, 16 | any order; all four unblock later batches. 11 before 07; 02 before 03/04(P4)/09/10; 16 before 18/19 |
| 2. floating consumers | 03, 04, 05, 06 | any order after 02/01. 04 Phases 1-3 may start before 02 lands; land 04's Phase 4 (tray routing, dead-key cleanup) only after 02 |
| 3. canvas consumers | 08, 09, 14, 10 | any order. 10 needs both 01 and 02; 10 Phase 6 (stretch) deferred until 11 (6.1) and 07's extraction (6.2) exist |
| 4. gpu + audio consumers | 07, 18, 19 | 07 after 01+11. 18 then 19 (18 lands the cpal line and `libasound2-dev`; 19's seam rows 2/2b become no-ops). If 19 lands first, invert |
| 5. standalone | 12, 13, 15, 17, 20 | any order, any time after batch 1 (12/15/17/20 have no plan deps and may run parallel to earlier batches). If 19 landed, drop 13's Phase 6. 20's plist step is grep-guarded against 18 either way |

Page-count mechanics for every page-adding plan: read the current
`tests/e2e_navigation.rs` literal, add 1, update the comment and `expected`
array (rule stated identically in 05 seam 5, 06 seam 5, 11 seam 8, 19 seam 17).
Digit bindings never move: appended pages sit at index >= 10 and the bind loop
covers `i < 9` (`src/shell/root/app_root/state.rs:144-152`, re-verified).

## 4. Shared conventions

Removal contract (seam canon: `_survey/seams.md`; pattern:
`src/gallery/mod.rs:1-7`). Every feature module opens with a header naming its
seam touches and a step-by-step removal recipe. The consolidated
multi-plan-file map is §5; apply touches in the stated order there.

i18n. Facade `crate::i18n::localize` / `localize_with_args`; both catalogs in
lockstep (`tests/qa_localization.rs:188-198`); one nesting level, quoted
scalars. Prefix per feature (each plan's i18n table is canonical for exact
keys and values):

| plan | prefix | nav key |
|---|---|---|
| 02 | `floating_close` (1 key) | - |
| 03 | `notch_*` | - |
| 04 | `menubar_charts_*`, `command_menubar_charts*`; deletes dead `tray_open_launcher` (Step 4.3) | - |
| 05 | `showcase_*` (deliberately not `canvas_*`) | `nav_canvas_showcase` |
| 06 | `canvas_*` (claimed before 05 chose `showcase_*`) | `nav_infinite_canvas` |
| 07 | `shader_*` | `nav_shader_canvas` |
| 08 | `multiuser_*` | `nav_multiuser_canvas` |
| 09 | `floating_ui_*` | `nav_floating_ui` |
| 11 | `wgpu_*` | `nav_wgpu` |
| 12 | `analytics_*` | `nav_analytics` |
| 13 | `music_*` | `nav_music` |
| 14 | `image_editor_*` | `nav_image_editor` |
| 15 | `tutorial_*` (+ `tutorial_command*`) | - (no page) |
| 17 | `system_controls_*` (+ `command_system_controls*`) | `nav_system_controls` |
| 18 | `microphone_*` | `nav_microphone` |
| 19 | `audio_*` | `nav_audio` |

No catalog keys: 01, 10 (gallery copy is literal English), 16, 20. Scanner
blind spot: indirect `localize(self.title_key())` calls are invisible to
`qa_localization`; every nav key above must land in both catalogs by hand
(survey §5).

Pin update (post-review). These plans were drafted and reviewed against
gpui-kit 0.7.1 / gpui-pre 0.3.8 sources; the app now pins gpui-kit 0.7.3 /
gpui-pre 0.3.9. All `path:line` citations still name the 0.7.1/0.3.8
registry sources they were verified against — treat them as the historical
record and re-verify load-bearing claims against the 0.7.3/0.3.9 sources at
implementation time (the registry keeps both). Known deltas from the 0.7.3
changelog that touch surfaces the plans rely on: `visible_rows_changed`
now fires `0..1`/`0..0` for one-row/empty tables (plans 12/13 table usage),
TextView/Markdown layout defaults changed (visual only), and
`TooltipRequest::placement` is deprecated in favor of `with_placement`
(no plan uses it). The default icon set was diffed 0.7.1 → 0.7.3 and is
IDENTICAL, so §4's icon policy and stand-in table hold unchanged.

Icon policy (applied across all plans post-review). `gpui_kit::component::
IconName` is generated from the DEFAULT 106 only
(`gpui-kit-assets-0.7.1/default-icons.txt`), and the app's native asset
source embeds exactly that set (`src/app/assets.rs` wraps
`gpui_kit::assets::Assets`, whose native form is `default_assets.rs`;
`AllAssets` is the unregistered full source). Rules: name only default
variants; an svg existing in `assets/icons/` proves nothing; full-catalog
variants compile through `gpui_kit::assets::IconName` but render BLANK on
native (wasm's CDN source is the only one serving them). Stand-ins shipped by
the plans: Music→Play, Volume2→Settings2, SkipBack/Forward→ChevronLeft/
Right, ListMusic→Menu, Gpu→Cpu, Activity→ChartPie, PenTool→Palette,
Image→Frame, SlidersHorizontal→Settings2, X→Close, wifi family→Network/Ban,
AudioLines→Play. Escape hatch if a plan truly needs a rich glyph: ship a
section-local SVG inside the feature module (gallery-section pattern) —
in-module, no asset-source seam; swapping the app to `AllAssets` is a
deliberate, single-owner decision no plan takes unilaterally.

Capability registry. Snake_case names, one owning feature each:

`floating_windows`, `floating_always_on_top`, `notch_geometry` (02) ·
`notch_floater` (03) · `menubar_charts`, `menubar_charts_anchored` (04) ·
`canvas_shaders` (07) · `collab_canvas` (08) · `floating_ui_palettes` (09) ·
`wgpu_embed` (11) · `music_playback`, `music_audio_output` (13; second only in
Phase 6 builds) · `image_editor` (14) · `tutorial` (15) ·
`system_controls_battery/_bluetooth/_volume/_brightness` (17) ·
`microphone_input` (18) · `audio_playback` (19). No entries: 01, 05, 06, 10,
12, 16, 20 (each states why: no platform variance or init failure to report).

Wasm degradation rules (survey §6, applied per plan): prefer a
capability-registry entry over a compile failure; only deps with no wasm32
story go behind the not-wasm table; no `std::time` clocks (use
`crate::platform::clock`); `just wasm-check` is the local gate. Concrete
postures: in-window fallback surfaces (02 overlay floats, 03 demo float, 09
panels, 10 overlay panes), demo data instead of hardware (08 demo peers, 13
simulated engine, 18 synthetic signal), reason panels with the page still
routable (07, 11, 19), disabled controls with a stated reason (14, 04's
command), all-unsupported rows (17).

## 5. Cross-feature seam-collision map

Files touched by more than one plan, with the merge/application rule. Within
one file, each plan's touches are additive unless noted.

| file | plans | rule |
|---|---|---|
| `src/shell/sidebar.rs` | 05 06 07 08 09 12 13 14 17 18 19 | Each adds a `Page` variant + the four match arms + an `all()` entry. All append after `Gallery` (index >= 10, no digit rebind) except 17, which inserts at index 9 between `About` and `Gallery` (also no digit rebind; land 17 early to minimize `expected`-array churn). 13 additionally swaps the file's `IconName` import to `gpui_kit::assets::IconName` (13 seam 6; all existing variants exist in that enum, 13 Research Digest); land that swap once, with 13 |
| `src/shell/route.rs` (`VALID_HOSTS`) | same 11 plans | Append one host line each; order irrelevant |
| `src/shell/root/app_root/state.rs` | 11 page plans + 02 + 09 + 13 + 15 | Page plans: field + construct + `unchecked_active_page_view` arm (survey §1 rows 10-12). 02 adds `floating_host`; 09 adds `floating_ui_session`/`floating_ui_host` + the `PaletteChanged` drain arm + the `RootWindow` restore observer; 13 wires `music::init`; 15 adds the `tutorial` field + `TourHandle` global set. All additive |
| `src/shell/root/app_root/render.rs` | 02, 09, 13, 15 | Stacking is tree order (no z_index, survey / 02 digest). Apply in order 02 → 09 → 15: 02's `floating_host` child, then 09's `floating_ui_host` after it (09 seam 8), then 15's `tutorial::target("tour-viewport", …)` root wrap with the tour controller as last child (15 seam 3). 13's `now_playing_bar` goes between `content_area` and the status bar (13 seam 12), independent of the overlay order |
| `src/features/pages/mod.rs` | 05 06 07 08 09 12 13 14 17 18 19 | `mod x;` + `pub use` per plan. Three deviations: 07 also re-exports `CAPABILITY_SHADERS` (E0603 under wasm otherwise, 07 seam 4); 13 needs `pub mod music;` (public module for `views::music::init`, 13 seam 1); 08 uses `pub mod multiuser_canvas;` rather than a private `mod` (08 seam 6) |
| `src/features/mod.rs` | 03, 04, 11, 15 | One `pub mod` line each (`notch`, `menubar_charts`, `wgpu`, `tutorial`) |
| `src/app/init.rs` | 02 03 04 07 09 11 13 14 15 17 18 19 | Boot hooks: 02 `floating::init` immediately after `notifications::initialize`; 03 and 09 state their `init` lines go after 02's (03 seam 2, 09 seam 11); apply 02 → 03 → 09 in that order. 17/18/19 add `initialize` calls in or beside the `runtime_services_init` block per their seams; 04 adds `menubar_charts::init` after the capabilities loop. `DEFAULT_ENABLED_CAPABILITIES` gains: `canvas_shaders` (07), `wgpu_embed` (11), `music_playback` (13), `tutorial` (15); 07 and 11 each add a `#[cfg(target_family = "wasm")]` unsupported block after the loop. 14 adds its `image_editor` capability via `capabilities::set` beside the other startup capability steps (14 seam 8), not in the default list |
| `locales/en.yml` + `zh-CN.yml` | 02 03 04 05 06 07 08 09 11 12 13 14 15 17 18 19 | Prefixes are unique (§4); append each plan's block, `nav_*` keys inside the nav block. 04 also deletes `tray_open_launcher` (Step 4.3, after its reader swap in 4.2; do not delete early). Lockstep test is the gate |
| `Cargo.toml` not-wasm table | 04 07 08 11 13(P6) 17 18 19 | grep-first duplicates: `wgpu`/`naga` shared by 07 and 11 (identical lines; second lander skips); `cpal` shared by 18 and 19 (second lander skips). 11 also puts `image` here (`default-features = false`), which does not collide with the common-table entries (a crate may appear in several tables; features union) |
| `Cargo.toml` common `[dependencies]` | 05, 14, 16, 19 | 05 adds `image` png-only; 14 adds `image` png+jpeg; one key only, so land the union (`png+jpeg` satisfies 05's png need) and note the divergence in both PRs (see §8). 16 adds `rtrb` + `rustfft`; 19 adds `hound` + `symphonia` (no collisions) |
| `Cargo.toml` OS tables + `[dev-dependencies]` | 02, 17, 18 | 02 extends the macOS `objc2-app-kit` features (`NSScreen`); 03 deliberately adds nothing (03 seam 5). 17 adds macOS/Linux/Windows entries and features; 18 adds `objc2-av-foundation` (macOS). Dev-deps: `naga` (07/11, grep-first) |
| `[workspace]` members + `justfile` | 08 | 08 only (`collab-protocol`, `collab-server`, `collab-server` recipe) |
| `tests/e2e_navigation.rs` | 11 page plans | count = 10 + pages landed; read the literal, +1, fix comment and `expected` (and route-parse case where the plan lists one) |
| `src/services/commands.rs` | 04, 15, 17 | One `CommandId` variant + availability/registry/execute arms each; palette picks them up automatically |
| `src/state/config_store.rs` + `tests/snapshot_tests.rs` | 09, 18 | 09 adds `floating_palettes`; 18 adds `mic_device_id`/`mic_gain_db`. Both `#[serde(default)]`, both update the two snapshot fixtures. Additive schema, no renames (survey §4) |
| `src/runtime/events.rs` | 09 only | `PaletteChanged` variant; AppRoot's drain is the single exhaustive match (09 Research Digest) |
| `src/ui/mod.rs` | 01, 16 | `pub mod canvas;` and `pub mod waveform;`; removal of either leaves the other's line (16 removal step 1 states this) |
| `src/services/mod.rs` + `src/lib.rs` services re-export | 17, 18, 19 | `system_controls`, `microphone`, `audio`; one line/identifier each |
| `scripts/install-linux-deps.sh` | 17, 18, 19 | 17 adds `libpulse-dev`; 18 adds `libasound2-dev` (19 only if 18 has not landed, 19 seam 2b). Keep both when both features exist |
| `scripts/create-release-app.sh` vs `scripts/macos-dev-app.sh` | 20, 18 | 18 owns the dev-bundle plist copy; 20 owns the release-script heredoc and inserts the byte-identical mic string grep-guarded (20 Step 1.2). 18 must not touch the release script; 20 must not touch the dev script |
| `src/gallery/sections/overlays/mod.rs` | 02, 03, 10 | Three additive section hooks (`floating`, `notch_floater`, `floating_canvas`), alphabetical `mod` lines. Note: 10's Research Digest claims no other plan edits this file; 03's seam table does. The claims reconcile (all additions are additive), but 10's sentence is stale (§8) |
| `src/features/pages/home.rs`, `src/gallery/page.rs` | 15 | Tour targets + Start-tour button; layout-transparent wraps only |
| `docs/accessibility-checklist.md` | 15 | One bullet; `qa_docs` sentinels must survive (survey §9) |
| `src/platform/desktop_shell/tray/{macos,windows}.rs` + `windows.test.rs` | 04 | 04 only |

## 6. Full-showcase removal recipe

Goal state: the repo back to the pre-showcase boilerplate. Execute each plan's
own Removal Recipe, in dependency-reverse order (consumers before
foundations), one plan per commit, gates green between steps:

1. Leaves with no dependents, any order: 20, 19, 18, 17, 15, 14, 13, 12, 10,
   09, 08, 07, 06, 05, 04, 03.
2. Foundations last: 16, 11, 02, 01 (01's removal step 3: dependents must be
   gone first; same rule in 16 step 4).
3. Shared resources revert with their last user: `cpal` + `libasound2-dev`
   only after both 18 and 19 are gone (18 removal step 10); `wgpu`/`naga` only
   after both 07 and 11 (07 removal step 2); the common `image` entry only
   after 05 and 14 (11's `image` line sits in the not-wasm table and reverts
   with 11, per 11 seam 1); the macOS `NSScreen` feature with 02 (03 carries
   nothing to revert); `NSMicrophoneUsageDescription` in
   `scripts/macos-dev-app.sh` with 18 and in `scripts/create-release-app.sh`
   with 20 (18 removal step 11).
4. Decrement `tests/e2e_navigation.rs` by one per page plan removed; drop the
   `expected` entries and route cases; the count returns to 10.
5. Delete `docs/plans/` (including `_survey/`) and this INDEX.
6. Run the gates (`cargo fmt --all -- --check`,
   `cargo clippy --workspace --all-features`,
   `cargo test --workspace --all-features`, `just wasm-check`); grep the
   removed feature names across `src/ tests/ locales/ scripts/`: zero hits.

## 7. Tracking table

| id | plan | batch | status |
|---|---|---|---|
| 01 | canvas-foundation | 1 | |
| 02 | floating-window-toolkit | 1 | |
| 11 | wgpu-embed | 1 | |
| 16 | waveform-foundation | 1 | |
| 03 | notch-floater | 2 | |
| 04 | menubar-charts | 2 | |
| 05 | canvas-showcase | 2 | |
| 06 | infinite-canvas | 2 | |
| 08 | multiuser-canvas | 3 | |
| 09 | floating-ui | 3 | |
| 14 | image-editor | 3 | |
| 10 | floating-canvas | 3 | |
| 07 | canvas-shaders | 4 | |
| 18 | microphone | 4 | |
| 19 | audio-playback | 4 | |
| 12 | real-app-ui | 5 | |
| 13 | music-player | 5 | |
| 15 | tutorial-mode | 5 | |
| 17 | system-controls | 5 | |
| 20 | release-bundling | 5 | |

## 8. Open questions roll-up

Deduplicated across plans; each attributed. None blocks batch 1.

Cross-plan coordination (resolved by this index where possible):

- Implementation order of page plans / e2e count. Raised by 07, 11, 19 (and
  noted in 05/06/08). Batch table above is the answer; the count rule is
  "read the literal, +1".
- `image` feature-set divergence: 05 specifies png-only in the common table,
  14 png+jpeg in the same table, 11 `default-features = false` in the not-wasm
  table. Found while building §5 (this index). Union rule stated there;
  landing plans should note the deviation in their PR.
- 10's claim that no other plan edits `overlays/mod.rs` conflicts with 03's
  seam touch 3. Found while building §5; additions are additive, no action
  beyond awareness.
- Plan 02's text says `CapabilityStatus` has six pub fields; the struct has
  five (`src/runtime/capabilities.rs:6-12`, verified this session). 09's
  reviewer note already flagged it. Implementers of 02 should write five-field
  literals.
- Plan 14's dependency list names `update_shape` and `top_layer` as
  `CanvasState` items; 01's contract defines both on `CanvasDocument`
  (01:187, 01:195), reached through `document()`/`edit_document`. Slip was in
  14's Dependencies text only; FIXED post-review — 14 now attributes both to
  `CanvasDocument` via the state's readers.
- Plan 01's open question "plan 14 absent" is stale: `14-image-editor.md`
  exists and consumes 01's contract (14 Dependencies). Resolved.
- 18↔20 mic string reword risk and 18↔19 cpal duplication: both handled by
  grep-guards stated in the plans; residual risk is an implementer rewording
  one side (20 risk 6, 18 risk 2).
- Amplitude convention: 16 assumes ±1.0 normalized f32 input (cpal's f32
  convention); 16's open question asks 18/19 to confirm or normalize before
  push. Both consumers decode to f32 in ±1 (18 callback conversion, 19
  `DecodedAudio`), so the convention holds as planned.
- TCC prompt localization: `NSMicrophoneUsageDescription` localization via
  InfoPlist.strings is a bundling concern 18 assigns to 20 (18 open
  questions); 20 ships the English string only. Follow-up if localized
  bundles are ever wanted.
- `collab-protocol` on wasm: 08 assumes the serde-only crate compiles for
  wasm32 but never ran `just wasm-check` during planning (08 open questions;
  same class as 16's rustfft item below). Covered by 08's step done-checks.

Test-harness limits (accept the documented workarounds; do not invent tooling):

- Kit `test-support` feature (TestWindowExt/ElementSnapshot) not enabled; only
  gpui's `test-support` via the dev-dep. Raised by survey Open Questions, 01,
  06, 10, 12, 16; workaround everywhere is `#[gpui_kit::test]` +
  `TestAppContext` state assertions plus manual QA.
- No scroll-wheel or held-key simulation in the test harness. 01 (wheel
  router), 06 (space-pan); covered by unit math + manual QA.
- GPU-dependent tests are `#[ignore]`d; CI has no GPU job. 07, 11 (also 11's
  1M-particle perf target unmeasured, and lavapipe availability on runners
  unknown, 07).

Platform verification gaps (first-run watch lists inside each plan):

- Icon variants — RESOLVED post-review. The recurring risk (07 `Orbit`, 11
  `Gpu`, 12/04 chart icons, 06 HUD icons, plus audio-plan full-catalog picks)
  turned out to be worse than compile-only: full-catalog variants compile via
  the assets enum but render blank on native because the app embeds only the
  default 106 (icon policy in §4, grounded in `src/app/assets.rs` +
  `gpui-kit-assets`' `Assets` vs `AllAssets` split). All plans now name
  verified default variants; `cargo check` remains the arbiter for the
  generated spellings, and §4's stand-in table is the record.
- macOS paths compiled only on a Mac: 03 (notch reach-in; also
  `FullScreenAuxiliary` semantics sourced from web search, not Apple's page),
  17 (objc2-io-bluetooth feature gates, CoreAudio element fallbacks, `COINIT`
  feature module), 18 (AVFoundation statics). Windows: 04 (tray tests run
  only on the Windows lane), 17 (MMDevice COM). Linux: 17 (brightness
  permissions, AppImage libpulse bundling), 18/19 (alsa headers on runners).
- External crates verified docs-only (no compile this session): 19 hound; 13
  rodio 0.23 (Phase 6 only); 08 socket.io handshake proven solely by its
  Phase 2 loopback test; 16 rustfft wasm compile (fallback:
  `default-features = false` + per-target tables).
- Runner tooling for 20 (PlistBuddy path, ditto/hdiutil flags, pwsh Latin1
  scan) unverified until first dispatch (20 Verification).

Contract-stability notes:

- 01's Interface Contract code block is line-stable because 08 cites line
  numbers into it (01 Dependencies). Edits extend prose, never reflow the
  block. Same discipline applies to 02 (consumers 03/04/09/10 cite `02:NNN`)
  and 16 (consumers 18/19 cite `16:NNN`).
- `Hsla` has no `Serialize`; dependents persisting canvas scenes must map to
  a serializable color form (01 open question; 08 does this with `WireColor`).
- Survey/plan doc-only scope: no cargo gate was run during any planning
  session; every Verification section lists the exact commands (survey, 01,
  and each plan).
