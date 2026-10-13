# Showcase plan index

Master index for the 27 feature plans under `docs/plans/`. Read this, then
execute plans in the batch order below without reading all of them first.
Inputs read in full this session: `_survey/seams.md`, `_survey/ecosystem.md`,
and plans 01-20. Auth-track update (2026-10-12, second session): read in full
`_survey/auth-zed.md`, `_survey/auth-ecosystem.md`, and plans 21-24; those
plans cite gpui-pre 0.3.9 / gpui-kit 0.7.3 sources directly (the versions the
app pins), so the 0.7.1/0.3.8 citation caveat in §4 applies to plans 01-20
only. Widgets-track update (2026-10-12, third session): read in full
`_survey/desktop-widgets.md` and its four evidence files; plan 25 (widget
window shims) cites gpui-pre 0.3.9 sources directly, amends 02's contract
(append-only note), and coordinates with plan 26
(`26-layer-shell-widgets.md`, parallel session). Widgets registration pass
(2026-10-12, this edit): plans 26 and 27 read in full; both cite the pinned
0.3.9/0.7.3 sources directly (26.md:46-53, 27.md:72-81; sources verified
present on disk this session), and their INDEX rows land here in one edit per
27 seam 11 — 26 deferred its own rows to the track owner to avoid
parallel-session row edits (26.md:1000-1014). Repo baseline facts re-verified on disk: 10 sidebar pages
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
capability of the gpui-kit stack. A second track, auth (plans 21-24), applies
the same gallery discipline to identity: an extractable `crates/gpui-auth`
foundation, email and social providers over its contract, and an account UI.
A third track, desktop widgets (plans 25-27), applies it to the desktop
itself: OS-level window shims for click-through and runtime topmost (25), a
Wayland layer-shell surface option on the float path (26), and a persisted
widget manager page (27).
Standing rule for every plan: the feature is
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
| 21 | Auth foundation crate | `crates/gpui-auth`: TokenSet/SecretString, TokenStore + InMemoryStore, PKCE/state, AuthProvider + MockProvider, AuthController entity, app adapter `src/services/auth.rs` | none | none (24 owns UI) | native keyring (keyring gains `linux-native`); wasm InMemoryStore + degraded capabilities | 8 |
| 22 | Email + password auth | `AuthBackend` trait + DemoBackend (argon2, deterministic tokens, TOTP), controller email flows, optional axum dev server | 21 | none (24 owns UI) | argon2/totp-rs in crate not-wasm table; wasm keeps mock, `auth_email` flips unsupported | 7 |
| 23 | Social login (OIDC/OAuth) | `src/services/auth_social/`: Google + generic OIDC, GitHub, loopback capture + opt-in scheme mode, per-provider controllers | 21 | none (24 owns UI) | module native-only (cfg not-wasm); wasm aggregate capability degraded | 7 |
| 24 | Auth UI and account shell | Sign-in page (provider chooser + gpui-form email form), title-bar account menu, settings account section, diagnostics rows, SignIn/SignOut commands | 21 (15 optional, Phase 10) | sidebar page + title bar + settings + commands | none (wasm routable; social buttons capability-gated) | 11 |
| 25 | Widget window shims | cfg-gated `src/ui/floating/shims/` (macOS NSWindow reach-in, Win32 ex-style + SetWindowPos, X11 EWMH + XShape, Wayland set_input_region): click-through + runtime topmost toggles; 02 Windows PopUp mapping pinned | 02 | gallery section | shims native-only per OS; wasm no-op | 4 |
| 26 | Layer shell widgets | Wayland layer-shell surface option on 02 floats (Layer::Background/Top), `set_input_passthrough`, zed#46378 deferred spawn, capability rows; Mutter fallback to a 02 float | 02 | none (27 owns the widget UI) | native Wayland only; X11/mac/win/wasm untouched | 4 |
| 27 | Desktop widgets manager | sidebar page: widget kinds with spawn/despawn, Placement9 picker, always-on-top + click-through (25), Wayland layer (26), persisted layout, respawn-on-launch, capability rows; built-ins: clock, battery, quick sliders (17), CPU/mem sparkline (04) | 02, 25, 26 (17/04 data feeds) | sidebar page | native windows + Wayland layer-shell; wasm overlay panes | 7 |

Peer (non-dependency) coordinations, stated in the plans themselves:
13↔19 (19's engine can replace 13's Phase 6 rodio; 13 Dependencies),
18↔19 (single `cpal = "0.18"` not-wasm entry + `libasound2-dev`; 18/19 Seam
Touches), 18↔20 (byte-identical `NSMicrophoneUsageDescription` string,
grep-guarded; 18 Dependencies, 20 Step 1.2), 05/11/14 (one `image` entry, see
§5), 02→03 (03 needs 02's `NSScreen` feature line), 25↔26 (both amend 02 and
share `src/ui/floating/` files; the Cargo.toml Linux-table `x11rb` line is
25's alone — 26 makes no manifest change, 26.md:459-460; the Wayland
click-through dispatch site is the one coordination point, 25 Dependencies),
27→25/26 (consumes both siblings' amendments at 02.md:429-484 and
02.md:1135-1207; adds no 02 amendment of its own, 27 Dependencies).
Auth track: 21→22/23/24 (all three consume `21:Interface Contract` and the
`src/services/auth.rs` adapter); 22↔23 (`AccountInfo.avatar_url`: 23's
amendment needs one line in 22's literal, 23 seam rows 9-10; either order);
08/21/22(P6) (workspace members: append, never rewrite); 23→24 (24's "Plans
22/23 do not exist yet" sentences at 24.md:50 and 24.md:246-247 are stale;
browser buttons light up only for the registered `provider_id`, 24 risk 3).

## 3. Dependency DAG and implementation batches

```text
foundations (no deps)         single-dep consumers                two-dep
01 canvas ────────────────┬── 05 showcase     06 infinite
                          ├── 08 multiuser    14 image editor
11 wgpu ──────────────────┴── 07 shaders (needs 01 + 11; 11 before Phase 4)
02 floating ─┬─ 03 notch
             ├─ 04 menubar (Phases 1-3 build without 02; Phase 4 gated)
             ├─ 25 widget shims (amends 02 append-only; Wayland dispatch shared with 26)
             ├─ 26 layer shell (amends 02 append-only; no dep on 25 — parallel after 02)
25 + 26 ─────────────────────── 27 desktop widgets (data-fed kinds also need 17/04)
             └─ 09 floating ui
01 + 02 ───────────────────── 10 floating canvas (stretch 6 also needs 11)
16 waveform ─┬─ 18 microphone
             └─ 19 audio playback
12 analytics · 13 music · 15 tutorial · 17 system controls · 20 bundling
  (no plan deps; publish contracts or coordinate)
21 auth foundation ─┬─ 22 email/password  (amends 21 contract: AuthError variants,
                    │                    AuthState/AuthEvent, controller surface)
                    ├─ 23 social oidc    (amends AccountInfo: avatar_url; app-side module)
                    └─ 24 auth ui        (needs 21 alone; 22/23 need 24-side wiring — batch 6)
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
| 6. auth track | 21, 22, 23, 24 | 21 strictly first — 22/23/24 consume its crate, adapter, and contract. Then 22 and 23 in either order (disjoint crate files: 22 amends error/controller/lib + crate manifest, 23 amends token.rs; both edit `src/services/auth.rs`'s capability block, so land serially and grep first). 24 any time after 21 — it works with the foundation alone (mock provider + demo-complete control). Wiring for later landings (2026-10-12 fix round folded the email half INTO 24): (a) email — 24's submit and provider row already dispatch by provider: `sign_in_with_password` when 22 is registered, `start_sign_in` only on the mock (22's provider has no browser flow, `AuthError::NoBrowserFlow`, 22.md:367-372); (b) social — 23's per-provider controllers live app-side in `SocialFlows` (23.md:281-291) while 24 mirrors the adapter controller, so the Google/GitHub rows dispatch through the provider-registry seam (24 Step 3.2) but `provider_id` stays "mock"/"email" and real social sign-in needs 24's follow-up to call `auth_social::start` — 24 risk 3 names this design space; whoever lands 24 after 23 owns that one wiring edit. Batch 6 has no plan-dep on batches 1-5 and interleaves freely with them; only shared-file order matters (§5: init.rs auth hooks 21 → 23/24 adjacent; workspace members append) |
| 7. widgets track | 25, 26, 27 | after 02 (batch 1); no dep on batches 2-6 except 27's data feeds (17 batch 5, 04 batch 2; 27's reduced-build fallback covers out-of-order landings). 25 and 26 both depend on 02 alone — parallel-friendly, disjoint new submodules (`shims/` vs `layer_shell/`) — but both append to 02's contract and co-edit 02's `mod/spec/registry/spawn` files, so land serially, second lander greps first (§5). 25 before 26 keeps the Wayland click-through dispatch inline in `shims/mod.rs`; 26 first means 25's Step 2.2 forwards to `layer_shell::set_input_passthrough` (25 Dependencies). 02's Windows `AlwaysOnTop`→`PopUp` mapping pin rides with 25 (amendment note; Step 2.6 verifies the landed code and fixes it only if it drifted from 02.md:667; consumer impact check over 03/04/09/10 — all four already expect the PopUp posture, so no consumer edits). 27 last, after 25 and 26 (and after 17/04 for the battery/sliders/metrics kinds) |

Page-count mechanics for every page-adding plan: read the current
`tests/e2e_navigation.rs` literal, add 1, update the comment and `expected`
array (rule stated identically in 05 seam 5, 06 seam 5, 11 seam 8, 19 seam 17,
24 seam 16).
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
| 21 | — (no keys; capability reason strings are English literals) | - |
| 22 | — (no keys; wasm capability reason literal) | - |
| 23 | — (no keys; capability reasons + loopback browser page live outside the catalogs) | - |
| 24 | `sign_in_*`, `settings_auth_*`, `titlebar_account_*`, `diagnostics_auth_*`, `command_sign_*` (+ `command_reason_signed_out`, shared `auth_no_expiry`, nested `sign_in_form:` block); deletes 3 `settings_session_*` demo keys from both files | `nav_sign_in` |
| 25 | — (no keys; gallery copy literal English, capability reason literal) | - |
| 26 | — (no keys; capability reason literals) | - |
| 27 | `widgets_*` (+ `command_desktop_widgets*`) | `nav_desktop_widgets` |

No catalog keys: 01, 10 (gallery copy is literal English), 25, 26, 16, 20, 21,
22, 23. Scanner
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
IDENTICAL, so §4's icon policy and stand-in table hold unchanged. Plans 21-27
were drafted against 0.7.3/0.3.9 directly; no re-verification caveat applies
to their citations. A future pin bump re-opens the 25/26/27 Research Digests
(25 and 27 name it in their open questions, 26 in its risk 4; §8).

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
AudioLines→Play, Google→Globe (24). Escape hatch if a plan truly needs a rich glyph: ship a
section-local SVG inside the feature module (gallery-section pattern) —
in-module, no asset-source seam; swapping the app to `AllAssets` is a
deliberate, single-owner decision no plan takes unilaterally.

Capability registry. Snake_case names, one owning feature each:

`floating_windows`, `floating_always_on_top`, `notch_geometry` (02) ·
`floating_click_through` (25) · `layer_shell_widgets`,
`layer_shell_input_passthrough` (26; written by `layer_shell::init` at the
end of `floating::init`, per-session values — a Mutter spawn flips
`layer_shell_widgets` to degraded) · `desktop_widgets` (27) ·
`notch_floater` (03) · `menubar_charts`, `menubar_charts_anchored` (04) ·
`canvas_shaders` (07) · `collab_canvas` (08) · `floating_ui_palettes` (09) ·
`wgpu_embed` (11) · `music_playback`, `music_audio_output` (13; second only in
Phase 6 builds) · `image_editor` (14) · `tutorial` (15) ·
`system_controls_battery/_bluetooth/_volume/_brightness` (17) ·
`microphone_input` (18) · `audio_playback` (19) · `auth_email`, `auth_social`
(21; set via `capabilities::set` inside `install`, NOT in
`DEFAULT_ENABLED_CAPABILITIES`) · `auth_social_google/_github/_oidc` (23;
native only — the wasm `auth_social` aggregate stays in 21's adapter behind
cfg). 22 adds no id (amends `auth_email` in place). No entries: 01, 05, 06,
10, 12, 16, 20, 24 (each states why: no platform variance or init failure to
report; 24 reads 21's entries only).

Auth secrets policy (plans 21-24, restated in each). Auth secrets — tokens,
passwords, provider client secrets, TOTP secrets — never enter `AppConfig`
(no plan adds a field; snapshot schema pinned), never enter logs
(`SecretString`'s Debug is `[redacted]`, controllers log provider ids and
states only, `redacted_summary` exists for token logging, 24's
`demo_authorize_url` is excluded from every tracing macro), and never enter
source. The OS-keyring `TokenStore` (21's `KeyringStore`, service
`gpui-starter.auth`) is the only sanctioned token store; wasm degrades to
`InMemoryStore` and loses tokens on reload (21's recorded posture). Provider
client secrets live only in the user-authored, never-committed
`auth-providers.json` with a 0600 perms lint (23); TOTP secrets stay in the
demo backend's process memory, never in the `TokenStore` (22). The one
pre-existing exposure — the two single-instance forward log lines printing
the full deep link verbatim, putting a callback's single-use authorization
`code` in logs (`src/platform/process/single_instance.rs:296-297, 415-416`)
— is RESOLVED in the 2026-10-12 fix round: 23 seam row 11 / Step 6.0
sanitizes both log fields to path-only (the full link still flows to
`events::emit` untouched); 21 flagged it, 22 carried the tripwire, 23 fixes
it, and 23's manual QA item 8 audits the redaction.

Crate-extraction convention (from 21). Reusable foundations land as workspace
members under `crates/` with their own manifest (`publish = false`, documented
publish path), zero `crate::`/app imports, no i18n coupling, no UI (21's README
extraction checklist is the template). The app side stays one adapter file per
feature (`src/services/auth.rs`); `tests/qa_migration.rs` scans the app
package only, so the crate imports gpui directly while the adapter keeps
kit-facade imports (21 Research Digest). Configuration-bound, non-reusable
code stays app-side behind cfg — 23's `src/services/auth_social/` is the
recorded inverse decision, with its reasons.

Desktop-widgets decision record (`_survey/desktop-widgets.md` is the
reference doc). GPUI-native panels are the widget path on every platform:
`WindowKind::PopUp` plus 25's shims on macOS/Windows/X11, `WindowKind::
LayerShell` on Wayland (26). OS-surface integrations — macOS WidgetKit, the
Windows 11 widget board, KDE plasmoids, GNOME extensions — are EXPLICITLY NOT
PLANNED: those surfaces render the OS's own UI (SwiftUI, Adaptive Cards,
QML, GJS), so GPUI pixels cannot appear in them and the app's role would be
a data bridge only. Survey trigger condition, quoted: treat them as "optional
data-bridge extensions, one platform at a time, only when placement in the OS
widget gallery is a hard requirement" (survey Recommendation). 25 and 27
restate this as out of scope; the survey's effort numbers are the
justification.

Wasm degradation rules (survey §6, applied per plan): prefer a
capability-registry entry over a compile failure; only deps with no wasm32
story go behind the not-wasm table; no `std::time` clocks (use
`crate::platform::clock`); `just wasm-check` is the local gate. Concrete
postures: in-window fallback surfaces (02 overlay floats, 03 demo float, 09
panels, 10 overlay panes), demo data instead of hardware (08 demo peers, 13
simulated engine, 18 synthetic signal), reason panels with the page still
routable (07, 11, 19), disabled controls with a stated reason (14, 04's
command), all-unsupported rows (17), auth surfaces kept present with an
in-memory token store and capability-degraded entries carrying reasons
(21/22/23) while the sign-in page stays routable (24), widgets as 02 overlay
panes with a routable manager page and no-data widget states (27).

## 5. Cross-feature seam-collision map

Files touched by more than one plan, with the merge/application rule. Within
one file, each plan's touches are additive unless noted.

| file | plans | rule |
|---|---|---|
| `src/shell/sidebar.rs` | 05 06 07 08 09 12 13 14 17 18 19 24 27 | Each adds a `Page` variant + the four match arms + an `all()` entry. All append after `Gallery` (index >= 10, no digit rebind) except 17, which inserts at index 9 between `About` and `Gallery` (also no digit rebind; land 17 early to minimize `expected`-array churn). 13 additionally swaps the file's `IconName` import to `gpui_kit::assets::IconName` (13 seam 6; all existing variants exist in that enum, 13 Research Digest); land that swap once, with 13. 24 appends `Page::SignIn` last (standard arms, `nav_sign_in`, `IconName::User`); 27 appends `Page::DesktopWidgets` after `Gallery` (standard arms, `nav_desktop_widgets`, `IconName::PanelBottom`; relative order with 24's `SignIn` is landing order, both beyond the digit-bind range) |
| `src/shell/route.rs` (`VALID_HOSTS`) | same 11 plans + 24 + 27 | Append one host line each; order irrelevant. 24's line is the page host `sign-in`; 27's is the page host `desktop-widgets`. The auth callback `gpui-starter://auth/callback` is NOT a host here: those links are consumed before route parsing — 21's `handle_callback_link` guard runs as the first statement of the `DeepLinkReceived` and `ForwardedCommand::DeepLink` drain arms (21 seam 8), and from 23 on the guard's own first statement routes to `auth_social::route_callback` first (23 seam 6); `parse_deep_link` would reject `auth` as an unknown host |
| `src/shell/root/app_root/state.rs` | 11 page plans + 24 + 27 + 02 + 09 + 13 + 15 + 21 | Page plans: field + construct + `unchecked_active_page_view` arm (survey §1 rows 10-12). 02 adds `floating_host`; 09 adds `floating_ui_session`/`floating_ui_host` + the `PaletteChanged` drain arm + the `RootWindow` restore observer; 13 wires `music::init`; 15 adds the `tutorial` field + `TourHandle` global set; 21 adds a one-line `if auth::handle_callback_link(&link, cx) { continue; }` guard as the first statement of the two deep-link drain arms (inside the lines 64-113 drain loop); 24 adds the standard page wiring for `sign_in_page`; 27 adds the standard page wiring for `desktop_widgets_page`. All additive |
| `src/shell/root/app_root/render.rs` | 02, 09, 13, 15 | Stacking is tree order (no z_index, survey / 02 digest). Apply in order 02 → 09 → 15: 02's `floating_host` child, then 09's `floating_ui_host` after it (09 seam 8), then 15's `tutorial::target("tour-viewport", …)` root wrap with the tour controller as last child (15 seam 3). 13's `now_playing_bar` goes between `content_area` and the status bar (13 seam 12), independent of the overlay order |
| `src/features/pages/mod.rs` | 05 06 07 08 09 12 13 14 17 18 19 24 27 | `mod x;` + `pub use` per plan. Three deviations: 07 also re-exports `CAPABILITY_SHADERS` (E0603 under wasm otherwise, 07 seam 4); 13 needs `pub mod music;` (public module for `views::music::init`, 13 seam 1); 08 uses `pub mod multiuser_canvas;` rather than a private `mod` (08 seam 6). 24 is the standard shape (`pub mod sign_in;` + `pub use`). 27 is `pub mod desktop_widgets;` + `pub use` (`pub mod` because `app/init.rs` calls `desktop_widgets::init` by path, 27 seam 1) |
| `src/features/mod.rs` | 03, 04, 11, 15 | One `pub mod` line each (`notch`, `menubar_charts`, `wgpu`, `tutorial`) |
| `src/app/init.rs` | 02 03 04 07 09 11 13 14 15 17 18 19 21 23 24 27 | Boot hooks: 02 `floating::init` immediately after `notifications::initialize`; 03 and 09 state their `init` lines go after 02's (03 seam 2, 09 seam 11); apply 02 → 03 → 09 in that order. 26 adds no line of its own — `layer_shell::init(cx)` runs at the end of 02's `floating::init` (26 seam 2); 27's `desktop_widgets::init(cx);` goes directly after 02's `floating::init` line (27 seam 5; independent of 03/09's adjacent lines). 17/18/19 add `initialize` calls in or beside the `runtime_services_init` block per their seams; 04 adds `menubar_charts::init` after the capabilities loop. Auth hooks: 21's `services::auth::install(cx);` after `crate::storage::initialize(cx);` (after tokio 207 / secure storage 216 / session 217, 21 seam 7), then 23's `auth_social::install` (cfg'd out on wasm) and 24's `auth_ui::initialize` each directly after 21's line, either order between them. `DEFAULT_ENABLED_CAPABILITIES` gains: `canvas_shaders` (07), `wgpu_embed` (11), `music_playback` (13), `tutorial` (15); 07 and 11 each add a `#[cfg(target_family = "wasm")]` unsupported block after the loop. 14 adds its `image_editor` capability via `capabilities::set` beside the other startup capability steps (14 seam 8), not in the default list; 21/23 set `auth_email`/`auth_social` (+ per-provider entries) the same way, inside their `install`; 25/26/27 likewise register inside `floating::init` (probe)/`layer_shell::init`/`desktop_widgets::init` — the default list stays untouched by 21-27 |
| `locales/en.yml` + `zh-CN.yml` | 02 03 04 05 06 07 08 09 11 12 13 14 15 17 18 19 24 27 | Prefixes are unique (§4); append each plan's block, `nav_*` keys inside the nav block. 04 also deletes `tray_open_launcher` (Step 4.3, after its reader swap in 4.2; do not delete early). 24 adds ~40 keys (incl. the nested `sign_in_form:` block and hand-added `nav_sign_in` — scanner blind spot) and deletes the three `settings_session_*` demo keys from both files with its settings rewrite (24 seam 15). 27 adds ~40 `widgets_*` keys incl. the hand-added `nav_desktop_widgets` (same scanner blind spot) plus `command_desktop_widgets*`. 21/22/23/25/26 add none. Lockstep test is the gate |
| `Cargo.toml` not-wasm table | 04 07 08 11 13(P6) 17 18 19 21 23 | grep-first duplicates: `wgpu`/`naga` shared by 07 and 11 (identical lines; second lander skips); `cpal` shared by 18 and 19 (second lander skips). 11 also puts `image` here (`default-features = false`), which does not collide with the common-table entries (a crate may appear in several tables; features union). 21 appends `"linux-native"` to the keyring feature list (`Cargo.toml:337`; target-gated inside keyring, inert on macOS/Windows). 23 adds `oauth2` + `openidconnect` (`default-features = false`, `reqwest`+`rustls-tls` — subsets the app's reqwest line, one copy). 22's argon2/totp-rs lines sit in the crate's own not-wasm table; 22 ALSO adds the same two crates to the app's NEW `[target.'cfg(not(target_family = "wasm"))'.dev-dependencies]` table (Step 5.3's tests construct `argon2::Params` and generate TOTP codes; dev-deps never reach production or wasm builds) |
| `Cargo.toml` common `[dependencies]` | 05, 14, 16, 19, 25 | 05 adds `image` png-only; 14 adds `image` png+jpeg; one key only, so land the union (`png+jpeg` satisfies 05's png need) and note the divergence in both PRs (see §8). 16 adds `rtrb` + `rustfft`; 19 adds `hound` + `symphonia` (no collisions). 25 moves `raw-window-handle = "0.6"` here from the macOS table (wasm-clean; reverts with 25) |
| `Cargo.toml` OS tables + `[dev-dependencies]` | 02, 17, 18, 22, 25 | 02 extends the macOS `objc2-app-kit` features (`NSScreen`); 03 deliberately adds nothing (03 seam 5). 17 adds macOS/Linux/Windows entries and features; 18 adds `objc2-av-foundation` (macOS). 25 adds the `x11rb` line (shape + allow-unsafe-code) to the Linux table; macOS and Windows tables untouched by 25. Dev-deps: `naga` (07/11, grep-first); 22 adds argon2 + totp-rs under a NEW not-wasm dev-deps table (see the not-wasm row above) |
| `[workspace]` members + `justfile` | 08, 21, 22 (Phase 6, optional) | Append, never rewrite; grep `members` first (21 risk 6, 22 risk 7). 08: `collab-protocol`, `collab-server`, `collab-server` recipe. 21: `crates/gpui-auth`. 22 Phase 6: `auth-dev-server` + `auth-dev-server` recipe |
| `tests/e2e_navigation.rs` | 11 page plans + 24 + 27 | count = 10 + pages landed; read the literal, +1, fix comment and `expected` (and route-parse case where the plan lists one; 24's is `gpui-starter://sign-in`, 27's is `gpui-starter://desktop-widgets`) |
| `src/services/commands.rs` | 04, 15, 17, 24, 27 | One `CommandId` variant + availability/registry/execute arms each; palette picks them up automatically. 24's `SignIn`/`SignOut` follow the same shape (SignOut availability is snapshot-driven, not capability-driven). 27's `OpenDesktopWidgets` too (always enabled, `IconName::PanelBottom`) |
| `src/state/config_store.rs` + `tests/snapshot_tests.rs` | 09, 18, 27 | 09 adds `floating_palettes`; 18 adds `mic_device_id`/`mic_gain_db`; 27 adds `widgets_layout: Vec<PersistedWidget>` (sanitized at read, the 09 `floating_palettes` shape). All `#[serde(default)]`, all update the two snapshot fixtures (27: inline yaml gains `widgets_layout: []` + every exhaustive `AppConfig {` literal). Additive schema, no renames (survey §4). 21-26 add no fields — auth secrets never enter `AppConfig` (§4 auth secrets policy), and 25/26 carry no persisted state |
| `src/runtime/events.rs` | 09 only | `PaletteChanged` variant; AppRoot's drain is the single exhaustive match (09 Research Digest). 21-24 add no variants (21 consumes `DeepLinkReceived` and the `RemoteCommand(ForwardedCommand::DeepLink)` arm in the drain, `src/runtime/events.rs:10-16`; 24 reuses `Navigate`) |
| `src/platform/process/single_instance.rs` | 23 | Two log-field edits only (seam row 11 / Step 6.0): both forwarded-link `tracing::info!` sites log `path = link.split('?')...` instead of the verbatim link, keeping the auth callback's `code` out of logs; the emitted event is untouched and removal may keep the redaction (strict improvement, 23 removal step 5) |
| `src/ui/mod.rs` | 01, 16 | `pub mod canvas;` and `pub mod waveform;`; removal of either leaves the other's line (16 removal step 1 states this) |
| `src/ui/floating/{mod,spec,registry,spawn,probe}.rs` + `src/ui/floating/shims/` + `src/ui/floating/layer_shell/` | 02 (creates), 25, 26 | 02 builds the module. 25 adds the `click_through` field/builder/`set_level` (spec.rs), `set_entry_level` (registry.rs), the two runtime fns + spawn-time apply (spawn.rs), the `floating_click_through` registration (probe.rs), the `mod shims;` line + re-exports (mod.rs), and the whole `shims/` submodule. 26 adds the layer-shell builder + `surface` field and its own init line, and the whole `layer_shell/` submodule. Module ownership split: 25 owns `shims/` (dispatcher + macos/windows/x11), 26 owns `layer_shell/` (spec/state/passthrough/defer/wayland) — the new submodules are disjoint; the co-edited surface is 02's mod/spec/registry/spawn, all touches additive — probe.rs is 25's alone (26 registers via `layer_shell::init`, not probe's `register_capabilities`; 26 seam table has no probe.rs row). 27 touches none of these files (public-API consumer only). Land 25 and 26 serially, second lander greps first |
| `src/services/mod.rs` + `src/lib.rs` services re-export | 17, 18, 19, 21, 23, 24 | `system_controls`, `microphone`, `audio`, `auth` (21), cfg'd `auth_social` (23), `auth_ui` (24); one line/identifier each |
| `scripts/install-linux-deps.sh` | 17, 18, 19 | 17 adds `libpulse-dev`; 18 adds `libasound2-dev` (19 only if 18 has not landed, 19 seam 2b). Keep both when both features exist |
| `scripts/create-release-app.sh` vs `scripts/macos-dev-app.sh` | 20, 18 | 18 owns the dev-bundle plist copy; 20 owns the release-script heredoc and inserts the byte-identical mic string grep-guarded (20 Step 1.2). 18 must not touch the release script; 20 must not touch the dev script |
| `src/gallery/sections/overlays/mod.rs` | 02, 03, 10, 25 | Four additive section hooks (`floating`, `notch_floater`, `floating_canvas`, `window_shims`), alphabetical `mod` lines. Note: 10's Research Digest claims no other plan edits this file; 03's seam table does. The claims reconcile (all additions are additive), but 10's sentence is stale (§8) |
| `src/features/pages/home.rs`, `src/gallery/page.rs` | 15 | Tour targets + Start-tour button; layout-transparent wraps only |
| `docs/accessibility-checklist.md` | 15 | One bullet; `qa_docs` sentinels must survive (survey §9) |
| `src/platform/desktop_shell/tray/{macos,windows}.rs` + `windows.test.rs` | 04 | 04 only |
| `src/services/auth.rs` | 21 (creates), 22, 23 (amend) | 21 builds the whole adapter (`install`/`install_with`, `controller`, `handle_callback_link`, `KeyringStore`, capability entries). 22 widens `install_with` to (store, io, provider, password, clock), constructs `EmailPasswordProvider` over `DemoBackend` natively, flips the wasm `auth_email` entry to an unsupported literal, and spawns the 60 s refresh loop. 23 inserts a cfg'd `route_callback` guard as `handle_callback_link`'s first statement and moves the native `auth_social` entry into `auth_social::install` (wasm entry stays in the adapter behind cfg). 22 and 23 both edit the capability block — land serially, second lander greps first |
| `crates/gpui-auth/src/*` + crate manifest | 21 (creates), 22, 23 (amend) | 21's Interface Contract block is line-stable; amendments append-only: 22 appends 11 `AuthError` variants, 2 `AuthState` variants, 1 `AuthEvent` variant, controller fields/methods, and the argon2/totp-rs lines in the crate not-wasm table; 23 adds `AccountInfo.avatar_url` (`#[serde(default)]`; old keyring rows parse unchanged). No file overlap between 22's edits (error/controller/lib/manifest) and 23's (token) |
| `src/shell/title_bar.rs` + `src/shell/mod.rs` | 24 | 24 only: `account: Entity<AccountMenu>` field + construct + last-child render in the right cluster; `pub mod account_menu;` (24 risk 5: no plan 01-20 touches title_bar) |
| `src/features/pages/settings/mod.rs` + `dev_sections/runtime_sections.rs` | 24 | 24 only: `session_section` module + child call + `AuthUiState` observer; deletes the three session demo buttons and their `use crate::session` import (restored on removal, 24 removal step 4) |
| `src/features/pages/diagnostics/{mod.rs,rows.rs}` | 24 | 24 only: one `observe_global_in` observer + four rows after `diagnostics_session` (23's per-provider capability rows need no page edit — the registry renders generically, `rows.rs:387-396`) |
| `docs/plans/21-auth-foundation.md` + `22-auth-email.md` | 22, 23 | Cross-plan doc notes, prose-append only: 22 appends a pointer paragraph under 21's contract (22 seam 8); 23 appends 21's amendment note (23 seam 9) and adds `avatar_url: None` to 22's literal (22.md:630-632, 23 seam 10) |
| `docs/plans/02-floating-window-toolkit.md` | 25, 26 | Cross-plan amendment notes, prose-append only, never reflow 02's contract block or original lines: 25's shims amendment sits at the end of the Interface Contract section (Windows PopUp mapping pin, added API, consumer check; 02.md:429); 26's layer-shell amendment sits at file end (02.md:1135). 25's removal step 6 deletes its note; 27 appends nothing — it amends no 02 file (27 Dependencies) |
| `docs/plans/INDEX.md` | 27 | The widgets-track registration pass: 26's deferred rows plus 27's own (§2/§3/§4/§5/§6/§7), executed per 27 seam 11 / Step 6.1. 27's removal step 12 drops the rows |

## 6. Full-showcase removal recipe

Goal state: the repo back to the pre-showcase boilerplate. Execute each plan's
own Removal Recipe, in dependency-reverse order (consumers before
foundations), one plan per commit, gates green between steps:

1. Leaves with no dependents, any order: 27, 24, 23, 22, 20, 19, 18, 17, 15,
   14, 13, 12, 10, 09, 08, 07, 06, 05, 04, 03. Then the widget-track pair in
   order: 25, then 26 (26's removal recipe waits for both its consumers —
   27 above, and 25, whose Wayland dispatch site may forward into 26's
   `set_input_passthrough`; 25 also lands first in batch 7).
2. Foundations last: 21, 16, 11, 02, 01 (01's removal step 3: dependents must
   be gone first; same rule in 16 step 4; 21's removal step 4 requires
   22/23/24 gone first).
3. Shared resources revert with their last user: `cpal` + `libasound2-dev`
   only after both 18 and 19 are gone (18 removal step 10); `wgpu`/`naga` only
   after both 07 and 11 (07 removal step 2); the common `image` entry only
   after 05 and 14 (11's `image` line sits in the not-wasm table and reverts
   with 11, per 11 seam 1); the macOS `NSScreen` feature with 02 (03 carries
   nothing to revert); `NSMicrophoneUsageDescription` in
   `scripts/macos-dev-app.sh` with 18 and in `scripts/create-release-app.sh`
   with 20 (18 removal step 11). Auth shared resources: the keyring
   `linux-native` feature, the `crates/gpui-auth` member, and the two drain
   guards revert with 21; `oauth2`/`openidconnect` and
   `src/services/auth_social/` with 23; the `auth-dev-server` member + recipe
   with 22 Phase 6 if landed. Widgets shared resources: the Linux-table
   `x11rb` line and the common-table `raw-window-handle` move revert with 25
   (25 removal step 5); 25's and 26's amendment notes on plan 02 delete with
   their plans (25 removal step 6, 26 removal step 6).
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
| 01 | canvas-foundation | 1 | implemented |
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
| 21 | auth-foundation | 6 | |
| 22 | auth-email | 6 | |
| 23 | auth-social-oidc | 6 | |
| 24 | auth-ui | 6 | |
| 25 | widget-window-shims | 7 | |
| 26 | layer-shell-widgets | 7 | |
| 27 | desktop-widgets | 7 | |

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
- Plan 02's Windows `FloatLevel::AlwaysOnTop` → `WindowKind::PopUp` mapping:
  02's plan text already maps uniformly (02.md:667); 25's amendment pins it
  and Step 2.6 fixes only landed code that drifted. The consumer impact
  check over 03/04/09/10 (25 Dependencies, grep-stated) found all four
  already expect the PopUp posture, so the pin changes nothing they rely on.
- Plan 02's Step 3.2 calls `spec.focus()`-style readers that cannot coexist
  with its same-named consume-self builders (no method overloading in Rust;
  02.md:667-671, found during 25's planning, 25 risk 5). 02's implementer
  renames the readers (`is_focus()` style) or makes the builders
  `Option`-taking; 25's own additions avoid the collision
  (`is_click_through`).
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
- Auth workspace members: 08, 21, and 22 (Phase 6) all edit `[workspace]` —
  the append-never-rewrite rule now lives in §5 (21 risk 6, 22 risk 7).
- `AccountInfo.avatar_url` ripple (22↔23): 23's serde-default amendment
  breaks 22's exhaustive literal (22.md:630-632) until it gains
  `avatar_url: None` (23 seam rows 9-10); either landing order costs one
  line, and whoever lands second owns the sync.
- 24's staleness: its Dependencies still say "Plans 22/23 do not exist yet"
  (24.md:50) and its contract notes "when they land" (24.md:246-247) — both
  predate 22/23 landing on disk (23 open questions). Load-bearing third
  instance: 24.md:214-215 claims "the same page button calls `start_sign_in`
  and the browser flow replaces the demo step" once 22/23 land — wrong for
  both (22's provider returns `NoBrowserFlow`, 22.md:367-372; 23's
  controllers are `SocialFlows`', not the adapter's, 23.md:281-291), which
  is why batch 6 names the two 24-side wiring edits. Whoever executes or
  next edits 24 drops all three sentences and applies the wiring.
- 21's `install_with` visibility is unstated ("the internal fn both call",
  Phase 8.2); 24's tests need it `pub(crate)` and 22 widens its signature —
  24's setup helper adopts whichever signature exists when it lands
  (24 risk 1, 22 seam 6). One-word widening is the only edit inside 21's
  files.

Auth track (21-24), deduplicated; each attributed:

- No cargo gate was run during 21-24 planning or their surveys (doc-only
  scope, matching plans 01-20). Earliest compile evidence points: 21 Steps
  1.1/7.2, 22 Steps 2.1/4.1, 23 Step 1.1/7.2, 24 Steps 1.4/2.2.
- Never-compiled dependency sets: getrandom 0.4 `wasm_js` union + reqwest
  json-only on wasm (21 risk 1; js-sys fallback lever stated there); totp-rs
  6.0.0 minimal feature set `["alloc", "otpauth", "zeroize"]` — feature table
  docs.rs-verified 2026-10-12, crate absent from the registry cache (22 risk
  1; fallback: ~30-line hand-rolled RFC 6238 verify over hmac+sha1); oauth2
  5.0.0 / openidconnect 4.0.1 lock resolution — APIs tarball-verified, the
  feature graph unproven (23 risk 1; fallback levers listed there). argon2
  pinned 0.5.3 because 0.6.0 is not in the registry cache (22 open
  questions).
- Provider facts from single docs fetches, unexercised: GitHub loopback
  any-port, DELETE-token Basic auth, Google revoke endpoint, Google desktop
  custom-scheme ambiguity (23 open questions; survey Open Questions). Confirm
  with throwaway registrations during 23's manual QA (items 3/6).
- macOS/Windows: gpui-pre 0.3.9 macos/windows/apple platform sources are
  absent from this host (0.3.8 latest present), so scheme-mode and
  `register_url_scheme` claims need a per-OS pass (23; survey); 24's account
  menu keyboard nav is kit surface with Linux-only QA here (24).
- Deep-link authorization-code logging (pre-existing): see §4 auth secrets
  policy — 21 flagged it, 22 re-assigned it to 23, 23 accepts it unchanged
  and audits logs (manual QA item 8).
- Duplicate majors accepted: totp-rs pulls sha2 0.11 + sha1 0.11 next to the
  crate's sha2 0.10 (22 risk 6); oauth2/openidconnect carry thiserror 1.x
  next to the app's 2.0.18 (23). `qa_migration` guards only the gpui family.
- Linux keyutils store is session-scoped: tokens do not survive reboot; the
  persistent upgrade lever (`linux-native-sync-persistent`, needs
  `libdbus-1-dev`) is documented, not taken (21 risk 2).
- Questions the auth plans direct at 24: QR vs manual-entry rendering for
  TOTP enrollment (22 kept the `qr` feature off; the enrollment data
  supports both, 22 open questions); TOTP-secret serde only if a future
  backend persists secrets (22 open questions); a toast for
  `route_callback` cold links that hit NoPendingFlow (23 open questions).
- 24-local: tutorial wraps conditional on plan 15 (grep guard; five reserved
  target ids, 24 Phase 10); `Button::loading` not verified to block clicks,
  so the submit also sets `disabled` (24 risk 6); zh-CN copy is
  draft-quality, no native-reviewer convention in the repo (24);
  `auth_no_expiry` deliberately breaks the page-prefix convention because
  three surfaces share the string (24 open questions).

Widgets track (25-27), deduplicated; each attributed:

- No cargo gate was run during 25/26/27 planning or their survey (doc-only
  scope, matching every other track). Earliest compile evidence points:
  25 Step 2.1, 26 Step 1.1, 27 Step 1.1.
- Windows click-through on the DirectComposition path (`WS_EX_TRANSPARENT`
  on `WS_EX_NOREDIRECTIONBITMAP` windows) is unverified at runtime — survey
  Part 2, 25 risk 1, 27 risk 1. No Windows host existed in any planning
  session. Closure: 25 manual QA 6 / 27 QA 8 record the outcome in the PR;
  the fallback levers (window recreation, `WS_EX_LAYERED`) are documented,
  not applied blindly (layered needs a redirection surface, the opposite
  direction of DComp).
- Mutter's honoring of empty input regions is unverified — survey Part 3,
  25 risk 3, 26 risk 1, 27 risk 2. No GNOME session was run. Closure: 25 QA
  12 / 26 QA 5 / 27 QA 13; the capability row stays protocol-level either
  way.
- `kCGDesktopWindowLevel` citation unverified: desktop-below placement stays
  out of scope on macOS; X11 `_NET_WM_STATE_BELOW` exists in 25's pure
  message builder but no public API sends it; desktop-pinned is Wayland-only
  and the 27 page states so (survey Part 1; 25/27 open questions).
- Sparse-MSIX widget registration and every OS-surface integration
  (WidgetKit, Windows widget board, plasmoids, GNOME extensions):
  explicitly not planned — §4 decision record; survey Recommendation holds
  the effort numbers.
- EWMH honoring is WM policy on X11 (25 risk 2, 27 risk 3): toggles return
  true when the client messages are flushed; non-EWMH WMs ignore them, and
  PopUp-kind floats are override-redirect there anyway (already above,
  taskbar-free).
- Cross-thread xcb use while gpui's event loop polls the same connection:
  unexercised under load (25 risk 4). Documented lever if a hang ever
  appears: pre-intern atoms once at first float spawn.
- zed#46378 startup output selection (26 risk 2, 27 risk 6): workaround
  shipped as `spawn_floating_after_displays` (100 ms retries, max 10);
  residual compositor-chosen output accepted and logged.
- 25↔26 contract reconciliation (26 risk 3, raised when the siblings were
  absent from disk) is resolved: both plans are on disk and 02 carries both
  amendments (02.md:429, 02.md:1135); 25's Wayland arm stays inline with the
  forward option documented, and 27 consumes both contracts as written.
- macOS/Windows shim modules compile only on their CI lanes; there is no
  macOS test lane (25 risk 6; same posture as 03's cfg'd platform code).
- 27-local accepted trade-offs: respawn rebuilds widget views (02 has no
  move API; layer/level are creation-time on Wayland), wasm overlay panes
  inherit 02's coordinate-staleness risk 5, and a 27-before-17/04 landing
  uses the reduced-build fallback (kinds hidden from the catalog, never
  fake data).
- A gpui-pre pin bump re-opens all three Research Digests (25/27 open
  questions; 26 risk 4's future-bump note; §4 pin-update rule).

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
  block. Same discipline applies to 02 (consumers 03/04/09/10 cite `02:NNN`),
  16 (consumers 18/19 cite `16:NNN`), and 21 (consumers 22/23/24 cite
  `21:Interface Contract`; 22/23 append amendment pointer notes to 21's doc,
  prose only, and both keep their own additions append-only). 25 and 26 keep
  their 02 amendments append-only for the same reason — 27 cites
  `02.md:429-484` and `02.md:1135-1207` into them.
- `Hsla` has no `Serialize`; dependents persisting canvas scenes must map to
  a serializable color form (01 open question; 08 does this with `WireColor`).
- Survey/plan doc-only scope: no cargo gate was run during any planning
  session; every Verification section lists the exact commands (survey, 01,
  and each plan).
