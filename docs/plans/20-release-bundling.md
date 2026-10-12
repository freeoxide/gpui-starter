# 20 Release Bundling Plan

## Goal

A build downloaded from GitHub Actions (PR-check artifact or the rolling `latest` release) launches as a real app: the macOS artifact is a zipped `.app` with `CFBundleIdentifier`, bundle icon, and usage-description keys, so native notifications initialize instead of bailing with "macOS bundle identifier missing"; the Linux debug tarball carries the desktop entry, hicolor icons, and `install-linux.sh` so notifications resolve identity and icon; every release lane gains a smoke step that unpacks its own artifact and asserts identifier/icon presence. The web download page's `latest` channel points at assets that actually exist (today its DMG links 404 because `publish-latest` never uploads DMGs).

## Dependencies

- **Plan 18 (`docs/plans/18-microphone.md`)** — coordinated, not consumed, with DISJOINT files. Plan 18's Step 6.1 (`18-microphone.md:1029-1039`) adds `NSMicrophoneUsageDescription` with the pinned string "Captures microphone audio to display the live input level and waveform." to `scripts/macos-dev-app.sh` ONLY — its step text explicitly says `create-release-app.sh` "is plan 20's surface … this plan must not touch it" (`:1031-1034`), and its seam item 3 (`:451-454`) says the same. Plan 20's Step 1.2 is therefore the ONLY writer of the key into `create-release-app.sh` (release + host-debug bundles), and it quotes the same byte-identical string plan 18 pins (`18:26-34` records the split of ownership, citing this plan's Step 1.2 at `20:149-160`). The insert is grep-guarded for idempotence on re-runs, not for merge order. Changing the wording later means editing both plans.
- **Plan 19 (`docs/plans/19-audio-playback.md`)** — exists on disk, no interaction (no packaging surface of its own).
- Published for dependents: every CI-produced macOS `.app` ships the key (Interface Contract item 2 — presence guaranteed by whichever of plans 18/20 lands first), and the debug-channel asset names and layouts (items 3–4) for any plan testing native notifications from CI builds.

## Research Digest

Survey (`docs/plans/_survey/seams.md`, `_survey/ecosystem.md`) used as the map; every load-bearing claim below re-verified on disk this session unless marked "web".

### Root cause of the missing bundle identifier (repo files)

- `src/services/notifications/backend/user_notify.rs:237-254`: on macOS the user-notify backend bails unless `NSBundle::mainBundle().bundleIdentifier()` resolves; the error string at `:249-252` is exactly "macOS bundle identifier missing; launch the bundled app from scripts/macos-dev-app.sh instead of raw cargo run". A bare `target/debug/gpui-starter` has no `NSBundle` identifier.
- The backend registers under `APP_ID = "com.gpui-starter.app"` (`user_notify.rs:17`); `DESKTOP_ENTRY_ID = "com.gpui-starter.app"` is the shared constant (`src/services/notifications/service/types.rs:20`).
- `.github/workflows/pr-builds.yml:184-227` (build-macos lane): uploads the bare binary `target/debug/gpui-starter` (`:215`) — the artifact that reports the missing identifier. Its own comment says "plain binary, no .app bundle / codesign / DMG — those are release.yml's job" (`:185-188`).
- The stable channel is already correct: `scripts/create-release-app.sh:57-84` writes `Info.plist` with `CFBundleIdentifier` (`:68-69`), `CFBundleIconFile=gpui-starter.icns` (`:66-67`), copies the icns (`:36`), and the workflow ad-hoc or Developer-ID signs each `.app` (`.github/workflows/release.yml:154-159`) before DMG creation. Missing from that plist: `NSMicrophoneUsageDescription`.
- `scripts/macos-dev-app.sh:19-44`: dev-bundle plist has the identifier but no icon and no usage keys, and it runs its own `cargo build` (`:12`) without `--features artifact`, so pr-builds cannot reuse it as-is (its build would overwrite the `artifact`-feature binary with a non-embedding one; `artifact` = `rust-embed/debug-embed`, `Cargo.toml:145-147`).

### Windows: icon and AUMID already handled (repo files)

- Icon: `build.rs:4-10` embeds `assets/app-icon/gpui-starter.ico` via winresource for every `target_os = "windows"` build — debug and release alike (no profile gate).
- Toasts: `src/platform/win_toast.rs:139-141` sends through `ToastNotificationManager::CreateToastNotifierWithId(DESKTOP_ENTRY_ID)`. Unpackaged apps need an AUMID shortcut or Windows silently drops toasts; `src/platform/toast_identity.rs:42-54` (`ensure_toast_identity`) installs the Start Menu shortcut and process AUMID at backend init (`user_notify.rs:40-43`), idempotent and best-effort. The Inno installer also pre-creates the shortcut with `AppUserModelID: "com.gpui-starter.app"` (`packaging/windows/gpui-starter.iss:56-60`). Conclusion: no Windows workflow change beyond smoke assertions; a runtime AUMID check cannot run headless in CI, so the source-level guarantee (iss lines + toast_identity call site) is asserted by grep instead.

### Linux: release channel complete, debug channel stripped (repo files)

- Release tarball stages desktop entry + full hicolor set + `install-linux.sh` (`.github/workflows/release.yml:303-306`); deb/rpm asset maps carry the same files (`Cargo.toml:100-106`, `:116-123`); AppImage passes the desktop file and 512px icon to linuxdeploy (`scripts/build-appimage.sh:130-131`); flatpak installs desktop + metainfo + every hicolor size (`packaging/flatpak/com.gpui-starter.app.yml:30-33`) with the `notifications-portal` backend feature (`:28`). These recipes are verified against the files; none changes in this plan.
- Notification identity: notify-rust sends the `desktop-entry` hint (`src/services/notifications/backend/notify_rust.rs:109-112`) and resolves the icon from an exe-relative probe that includes `exe_dir/hicolor/<s>x<s>/apps/gpui-starter.png` (`notify_rust.rs:50-66`) — the exact layout `release.yml:304` stages. `scripts/install-linux.sh:34-63` installs the entry and hicolor tree per-user.
- Debug tarball gap: `pr-builds.yml:89-110` bundles binary + libs only ("trimmed from the release.yml bundling step", `:58-60`). Adding the same four staging lines gives the debug tarball working notification identity.

### macOS notification "usage strings" (web + plan 18)

- No Apple-documented Info.plist key customizes the UNUserNotificationCenter permission prompt; the alert text is system-generated from the bundle name. `NSUserNotificationsUsageDescription` is not a real Apple key. Verified by web search 2026-10-12 ([Apple Info.plist privacy keys](https://developer.apple.com), [Stack Overflow discussion](https://stackoverflow.max-everyday.com), [Apple Developer Forums — User Notifications](https://developer.apple.com)). Consequence: the only usage string the plist needs is `NSMicrophoneUsageDescription` (a real key), and its wording is pinned by plan 18, not chosen here.
- Plan 18 owns the copy: its Step 6.1 (`18-microphone.md:1029-1039`) pins "Captures microphone audio to display the live input level and waveform." into the DEV bundle (`scripts/macos-dev-app.sh` only — it explicitly never touches `create-release-app.sh`), and its Dependencies note (`18-microphone.md:26-34`) correctly records this plan as on disk and assigns `create-release-app.sh` to it. Both plans quote the same bytes; a reword means editing both.

### Artifact plumbing the plan must not break (repo files)

- `pr-builds.yml:246-250`: `publish-latest` downloads run artifacts with `pattern: gpui-starter-*-debug` + `merge-multiple: true`. A glob, not a list: the macOS artifact name `gpui-starter-macos-arm64-debug-app` (upload name from Step 1.3) does NOT match (pattern requires the trailing `-debug`), so without a pattern change the zip never reaches `./artifacts` and Step 5.1's `cp` fails. Verified by reading the step this session.
- `assets/app-icon/gpui-starter.ico` begins with the ICONDIR header: `od -An -tx1 -N8 assets/app-icon/gpui-starter.ico` → `00 00 01 00 07 00 00 00` (reserved=0, type=1, count=7) — run this session. The RT_GROUP_ICON resource winresource embeds carries the same 6-byte header shape, which is what the Windows smoke steps scan for.
- `[System.Drawing.Icon]::ExtractAssociatedIcon` cannot gate icon presence: it returns the shell's generic associated-exe icon (non-null) even when the exe has no embedded resource. Not runnable on this Linux planner to reproduce; flagged by review against .NET documented behavior. The smoke steps therefore scan the exe bytes for the ICONDIR signature instead.
- `hdiutil attach -mountpoint` requires the mount directory to exist; `MNT="$(mktemp -d)/dmg"` creates only the parent. Not verifiable on Linux; the smoke step creates the dir explicitly (cheap, removes the variable).

### Web download widgets (repo files)

- `web/util/downloads.ts` builds both channels from one template: the `latest` channel advertises `GPUI-Starter-latest.dmg` / `gpui-starter-latest-x86_64-linux.tar.gz` (`channel()` at `:31-41`, wired in `downloadLinks` at `:43-45`). `pr-builds.yml` `publish-latest` uploads none of those; it uploads `gpui-starter-linux-x86_64-debug.tar.gz`, `gpui-starter-windows-x64-debug.exe`, `gpui-starter-macos-arm64-debug` (`pr-builds.yml:252-261`, `:281-284`). Every `latest`-channel asset link on the site is therefore a 404 today.
- `web/pages/download.astro:18-42` renders the same card set for both flavors; `:151` passes `workflow="latest.yml"` to `BuildStatus`, whose default is also `latest.yml` (`web/components/BuildStatus.astro:16`, `:53`) — a workflow file that does not exist (`.github/workflows/` contains cache-cleanup, ci, deploy-docs, pr-builds, release). The live status strip queries `/actions/workflows/latest.yml/runs` (`BuildStatus.astro:77`) and 404s, so it hides.
- Live size/count enrichment matches cards to release assets by exact filename (`download.astro:230-240`), so renamed cards must equal the real asset names.
- `scripts/generate-manifest.sh:52-71` consumes only `GPUI-Starter-<ver>.dmg` and the bare Windows exe; the stable asset list must not change (and does not in this plan).

### Tools available on runners (to be confirmed on first run; low risk)

- `ditto`, `hdiutil`, `/usr/libexec/PlistBuddy`, `codesign` ship on macOS runners (used by existing steps: `release.yml:165` create-dmg.sh calls hdiutil; `create-dmg.sh` itself runs on the runner today).
- `tar -tzf`, `desktop-file-validate` (installed at `release.yml:241`), and PowerShell 7 (`pwsh`, inbox on windows runners) for the Latin1 byte scan — no external tooling.

No new crates. No gpui/gpui-kit API is touched, so no registry verification was needed beyond confirming the feature adds no Rust API surface (`grep -rn "gpui" scripts/ .github/` — none of the touched files compile Rust).

## Architecture

No runtime entities, state, or events change; the app binary is untouched. What changes is the artifact-production pipeline and the web surface that advertises it.

Data flow after the change:

```
push/PR ──> pr-builds.yml
  build-linux   ──> debug tarball (binary+lib+hicolor+desktop+install-linux.sh) ──┐
  build-windows ──> gpui-starter.exe (icon embedded by build.rs) ─────────────────┤─> run artifacts
  build-macos   ──> create-release-app.sh host-debug ──> ad-hoc signed .app       │    + per-lane
                    ──> ditto zip ──> smoke (plist+icns assert) ──────────────────┘    smoke asserts
  publish-latest (master pushes) ──> `latest` release, 3 stable-named assets
                                          │
tag v* ──> release.yml                    v
  build-macos   ──> 3 DMGs (plist now with NSMicrophoneUsageDescription)
                    ──> smoke: mount DMG, PlistBuddy + icns + codesign asserts
  build-linux   ──> tarball/deb/rpm/AppImage (unchanged recipes) + tar-listing smoke
  build-windows ──> exe/zip/installer (unchanged) + zip-unpack icon smoke + iss grep
  build-flatpak ──> unchanged
  publish-release ──> 18 assets + SHA256SUMS + manifest.json (names unchanged)
                                          │
web download page ──< downloads.ts (stable: DMG/tarball names; latest: 3 debug assets)
                  ──< BuildStatus queries pr-builds.yml runs (was latest.yml, a 404)
```

Ownership: workflow files own artifact assembly; `scripts/create-release-app.sh` owns the single `Info.plist` heredoc used by both release and debug lanes (one source of truth for bundle identity); `web/util/downloads.ts` owns every download URL on the site (comment at `:5-10` already declares this contract).

## Interface Contract

What dependents (plan 18, future notification-testing plans, web) can rely on after this plan:

1. `scripts/create-release-app.sh <version> [universal|aarch64|x86_64|host-debug]` — `host-debug` is new: consumes `target/debug/gpui-starter` (host arch, no lipo), prints the staged `.app` path on stdout exactly like the existing arches (`create-release-app.sh:86`).
2. Every CI-produced macOS `.app` `Info.plist` contains: `CFBundleIdentifier=com.gpui-starter.app`, `CFBundleIconFile=gpui-starter.icns` (file present in `Contents/Resources/`), `CFBundleName=GPUI Starter`, plus `NSMicrophoneUsageDescription` with the string pinned by plan 18 Step 6.1 (`docs/plans/18-microphone.md:968-977`); plan 20's Step 1.2 inserts it grep-guarded, so merge order with plan 18 does not matter, and rewording means editing both plans. Matches the identifier the notifications backend registers under (`user_notify.rs:17`).
3. Debug-channel (`latest` release) asset names, stable across pushes: `gpui-starter-macos-arm64-debug-app.zip` (contains `GPUI Starter.app`, ad-hoc signed), `gpui-starter-linux-x86_64-debug.tar.gz`, `gpui-starter-windows-x64-debug.exe`.
4. Debug Linux tarball layout: top dir `gpui-starter/` containing the binary, `lib/`, `hicolor/` (full size set), `com.gpui-starter.app.desktop`, `install-linux.sh` — the layout `notify_rust.rs:50-66` and `install-linux.sh:39-44` probe.
5. Stable-channel release assets: byte-for-byte the same 18 filenames as today (`docs/releasing.md:41-60` table); `generate-manifest.sh` and `SHA256SUMS` untouched.
6. Web: `ChannelLinks` (`web/util/downloads.ts:18-25`) gains `debugMacArm`, `debugLinuxX64`, `debugWinX64` (empty string on the stable channel); `downloadLinks.latest` fills those three and leaves the DMG/tarball fields empty; `assetCards(links, flavor)` returns the 3 debug cards when `flavor === 'latest'`.

## Module Layout

No new files. All work edits existing workflows, one script, web components/util, and docs. (New-file table intentionally empty; the smoke steps are inline workflow steps because each is ~10 lines of platform-specific shell/PowerShell and has no second caller — AGENTS.md de-bloat rule.)

## Seam Touches

| File | Change | Why minimal |
|---|---|---|
| `scripts/create-release-app.sh` | Add `host-debug` arch arm (binary lookup + case arm); add `NSMicrophoneUsageDescription` to the plist heredoc | Single plist heredoc stays the one source of bundle identity for release AND debug lanes |
| `.github/workflows/pr-builds.yml` | macOS lane: stage `.app` + ad-hoc sign + ditto zip + upload zip + smoke; Linux lane: add the 4 staging lines + tar-listing smoke; Windows lane: embedded-icon smoke; `publish-latest`: download pattern `gpui-starter-*-debug` → `gpui-starter-*-debug*` (`:248`), stage/ship the mac zip instead of the bare binary; 3 job-summary strings updated | Each lane changes only its packaging tail; build steps untouched. The pattern widening is forced: the new artifact name `gpui-starter-macos-arm64-debug-app` does not match the old trailing-`-debug` glob |
| `.github/workflows/release.yml` | `build-macos`: DMG-mount smoke step; `build-linux`: tar-listing smoke; `build-windows`: zip-unpack icon smoke + iss grep | Read-only verification steps inserted before upload; zero effect on produced bytes |
| `web/util/downloads.ts` | `ChannelLinks` + 3 debug fields; `latest` channel built from the real debug asset names; comment block fixed (`latest.yml` → `pr-builds.yml`) | File is already the declared single source of download links |
| `web/pages/download.astro` | `assetCards` flavor branch for the 3 debug cards + hints; `BuildStatus workflow="pr-builds.yml"` | Cards, enrichment script, and structure reused as-is |
| `web/components/DownloadCta.astro` | `platformMap` picks debug links when the channel is `latest` | One map, one label set |
| `web/components/BuildStatus.astro` | Default `workflow` prop `latest.yml` → `pr-builds.yml` (2 places + doc comment) | Dead default today; keeps the component working without a prop |
| `docs/releasing.md` | New "The debug channel" subsection; amend the "Skipped: … macOS `.app`-in-zip" line to scope it to the stable channel; note the new plist key and smoke steps in the macOS section | Documents the contract this plan creates |

No edits to `src/`, `Cargo.toml`, `Cargo.lock`, or `packaging/`.

### Removal Recipe

Mirror of `src/gallery/mod.rs:1-7` style. Full removal of feature 20:

1. `pr-builds.yml`: revert build-macos to `path: target/debug/gpui-starter` upload and the old summary line; delete the macOS/Linux/Windows smoke steps and the 4 Linux staging lines; in `publish-latest`, restore `pattern: gpui-starter-*-debug`, restore `cp artifacts/gpui-starter release/gpui-starter-macos-arm64-debug` and the matching `files:` entry.
2. `release.yml`: delete the smoke steps (build-macos "Verify DMG bundle", build-linux "Verify tarball contents", build-windows "Verify installer AUMID source" and "Verify portable zip embeds an icon").
3. `scripts/create-release-app.sh`: delete the `host-debug` case arm and binary lookup; delete the `NSMicrophoneUsageDescription` key/value pair from the heredoc.
4. `web/util/downloads.ts`: drop `debugMacArm`/`debugLinuxX64`/`debugWinX64` from `ChannelLinks` and the debug builder; restore the previous comment block.
5. `web/pages/download.astro`: restore the single `assetCards` path (5 DMG/tarball cards both flavors) and `workflow="latest.yml"`.
6. `web/components/DownloadCta.astro`: restore the single `platformMap`.
7. `web/components/BuildStatus.astro`: restore `latest.yml` defaults and comment.
8. `docs/releasing.md`: delete "The debug channel" section, restore the "Skipped:" line and the macOS section note.

Everything above is revert-in-place; there is no new file or runtime state to delete.

## Implementation Phases

### Phase 1: macOS `.app` for debug artifacts

#### Step 1.1: `host-debug` arch in create-release-app.sh

Files: `scripts/create-release-app.sh`.
Changes:
- After the `X86_BIN` assignment (`:18-19`), add `DEBUG_BIN="$ROOT_DIR/target/debug/gpui-starter"`.
- Replace the existence loop (`:21-23`) so it checks `$DEBUG_BIN` only when `ARCH = host-debug`, else the two release binaries.
- Add case arm before `universal` (`:40-54`): `host-debug) cp "$DEBUG_BIN" "$MACOS_DIR/$APP_NAME" ;;` and extend the usage/error string (`:8`, `:51`) to list `host-debug`.
- Leave `APP_DIR` staging under `target/release/dist/host-debug/` as-is (`:28`) — path is internal.
Tests: none (shell). Done-check: `bash -n scripts/create-release-app.sh` passes and `ARCH=host-debug grep` shows the new arm; a local fake run: `mkdir -p target/debug && touch target/debug/gpui-starter && ./scripts/create-release-app.sh 0.0.0-test host-debug` prints a path whose `Contents/MacOS/GPUI Starter` exists.

#### Step 1.2: plist usage key (guarded, string pinned to plan 18)

Files: `scripts/create-release-app.sh`.
Changes: in the heredoc (`:57-84`), after `NSHighResolutionCapable`, add — ONLY if the heredoc does not already carry the key (idempotence guard for re-runs; plan 18 never edits this script — it lands the same key/string in the DEV bundle `scripts/macos-dev-app.sh`, its Step 6.1 at `18-microphone.md:1029-1039`):

```
  <key>NSMicrophoneUsageDescription</key>
  <string>Captures microphone audio to display the live input level and waveform.</string>
```

The string is byte-identical to plan 18's so both merge orders converge on one plist; do not reword without editing `18-microphone.md` too.
Done-check: `grep -A2 'NSMicrophoneUsageDescription' scripts/create-release-app.sh | grep -q 'Captures microphone audio to display the live input level and waveform'` exits 0 (-A2 tolerates either the two-line key/value form above or plan 18's wrapped single-pair form).

#### Step 1.3: pr-builds macOS lane bundles, signs, zips

Files: `.github/workflows/pr-builds.yml` (build-macos, `:190-227`).
Replace the `Upload macOS artifact` step with three steps (keep the build step untouched):

```yaml
      - name: Bundle debug .app and zip
        run: |
          set -euo pipefail
          APP="$(./scripts/create-release-app.sh "${GITHUB_SHA:0:12}" host-debug)"
          codesign --force --deep --sign - "$APP"
          mkdir -p stage
          ditto -c -k --sequesterRsrc --keepParent "$APP" \
            "stage/gpui-starter-macos-arm64-debug-app.zip"

      - name: Verify bundled app (identifier + icon)
        run: |
          set -euo pipefail
          rm -rf /tmp/appcheck && mkdir -p /tmp/appcheck
          ditto -x -k --rsrc stage/gpui-starter-macos-arm64-debug-app.zip /tmp/appcheck
          APP="/tmp/appcheck/GPUI Starter.app"
          PLIST="$APP/Contents/Info.plist"
          [ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$PLIST")" = "com.gpui-starter.app" ]
          [ -n "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIconFile' "$PLIST")" ]
          [ -n "$(/usr/libexec/PlistBuddy -c 'Print :NSMicrophoneUsageDescription' "$PLIST")" ]
          [ -f "$APP/Contents/Resources/gpui-starter.icns" ]
          codesign --verify --deep --strict "$APP"

      - name: Upload macOS artifact
        uses: actions/upload-artifact@v7
        with:
          name: gpui-starter-macos-arm64-debug-app
          path: stage/gpui-starter-macos-arm64-debug-app.zip
          if-no-files-found: error
```

Update the job-summary step (`:218-227`) to: "Artifact: gpui-starter-macos-arm64-debug-app — zip containing the ad-hoc signed GPUI Starter.app (arm64 debug). Unzip, right-click the app, Open."
Rationale pinned in a comment on the bundle step: the bare binary cannot pass `platform_primary_runtime_status` (`user_notify.rs:237-254`), so notification testing from CI artifacts requires a bundle.
Done-check: `grep -c "GPUI Starter.app" .github/workflows/pr-builds.yml` ≥ 2; YAML parses (`python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/pr-builds.yml'))"` or `rust-gha`-equivalent visual check); on a mac, the verify block passes against a locally produced zip.

### Phase 2: release macOS smoke

#### Step 2.1: DMG mount verification

Files: `.github/workflows/release.yml` (build-macos).
Insert after the `Notarize` step (`:171-191`), before `Upload DMG artifacts` (`:193-198`):

```yaml
      - name: Verify DMG bundle (identifier + icon)
        run: |
          set -euo pipefail
          VER="${{ steps.version.outputs.value }}"
          MNT="$(mktemp -d)/dmg"
          mkdir -p "$MNT"   # hdiutil -mountpoint needs an existing dir
          for DMG in \
            "target/release/GPUI-Starter-${VER}.dmg" \
            "target/release/GPUI-Starter-${VER}-aarch64.dmg" \
            "target/release/GPUI-Starter-${VER}-x86_64.dmg"; do
            hdiutil attach -readonly -nobrowse -mountpoint "$MNT" "$DMG" >/dev/null
            PLIST="$MNT/GPUI Starter.app/Contents/Info.plist"
            [ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$PLIST")" = "com.gpui-starter.app" ]
            [ -n "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIconFile' "$PLIST")" ]
            [ -n "$(/usr/libexec/PlistBuddy -c 'Print :NSMicrophoneUsageDescription' "$PLIST")" ]
            [ -f "$MNT/GPUI Starter.app/Contents/Resources/gpui-starter.icns" ]
            codesign --verify --deep --strict "$MNT/GPUI Starter.app"
            hdiutil detach "$MNT" >/dev/null
          done
```

`/usr/libexec/PlistBuddy` is spelled absolutely: it is not in macOS default `/etc/paths`, so a bare `PlistBuddy` depends on runner PATH setup. The mic-key assertion is safe regardless of plan-18 merge order because Step 1.2 guarantees the key exists once THIS plan lands.

Done-check: step present between Notarize and Upload; the DMG list matches the notarize loop exactly (`:181-184`).

### Phase 3: Linux debug-tarball parity + smoke

#### Step 3.1: stage desktop entry, icons, installer into debug tarball

Files: `.github/workflows/pr-builds.yml` (build-linux Relink step, `:89-110`).
Append after the lib-bundling loop, mirroring `release.yml:303-306`:

```bash
          cp resources/com.gpui-starter.app.desktop "$STAGE/" 2>/dev/null || true
          cp -r assets/app-icon/hicolor "$STAGE/hicolor" 2>/dev/null || true
          cp scripts/install-linux.sh "$STAGE/" 2>/dev/null || true
          chmod +x "$STAGE/install-linux.sh" 2>/dev/null || true
```

(The `|| true` guards match the release lane so a missing optional file degrades, not fails — but the Step 3.2 assertions make absence fatal anyway.)
Done-check: `diff <(sed -n '303,306p' .github/workflows/release.yml) <(grep -A3 'com.gpui-starter.app.desktop' .github/workflows/pr-builds.yml | head -4)` shows the same cp commands.

#### Step 3.2: tarball content assertions (both workflows)

Files: `.github/workflows/pr-builds.yml` (build-linux, after the tar step `:110`), `.github/workflows/release.yml` (build-linux, after `Create Linux tarball` `:318-329`).
Same block in both, parameterized by tarball name:

```bash
          LIST="$(tar -tzf "$TARBALL")"
          echo "$LIST" | grep -qx 'gpui-starter/com.gpui-starter.app.desktop'
          echo "$LIST" | grep -q '^gpui-starter/hicolor/48x48/apps/gpui-starter\.png$'
          echo "$LIST" | grep -q '^gpui-starter/hicolor/512x512/apps/gpui-starter\.png$'
          echo "$LIST" | grep -qx 'gpui-starter/install-linux.sh'
```

In pr-builds `TARBALL` is the sha-named file from `:110`; in release.yml use `stage/${{ steps.tarball.outputs.path }}`.
Done-check: locally, `tar -tzf` over a tarball built by the modified lane contains all four entries (reproduce staging by hand in `/tmp` if no runner).

### Phase 4: Windows smoke

Icon presence is asserted by scanning the exe bytes for the ICONDIR signature `00 00 01 00` (reserved=0, type=1), the header of the RT_GROUP_ICON resource `build.rs:4-10` embeds and of the committed ico itself (`od -An -tx1 -N8 assets/app-icon/gpui-starter.ico` → `00 00 01 00 07 00 00 00`, run this session). `[System.Drawing.Icon]::ExtractAssociatedIcon` is NOT used as a gate: it returns the shell's generic associated-exe icon when nothing is embedded, so a null check cannot fail (Research Digest). The scan is a heuristic — a 4-byte pattern can appear in code by chance (~2% for a 100 MB exe) — but a broken `build.rs` embed makes it fail deterministically, which is the regression the spec wants caught. AUMID stays a source-level guarantee: runtime registration (`toast_identity.rs:42-54`) cannot run headless, so the iss carries the check.

#### Step 4.1: release portable-zip embedded-icon assertion + AUMID source check

Files: `.github/workflows/release.yml` (build-windows, after `Build portable artifacts` `:444-446`). Two steps, NOT one combined bash step — a single step that `cd`s into an extraction temp dir loses the workspace-relative zip path (`target/release/…` would resolve under the temp dir / MSYS root, and `cygpath` around `$PWD` does not fix a wrong directory), so the grep runs in bash at the workspace root and the unzip + scan runs in a `shell: pwsh` step whose default cwd IS the workspace:

```yaml
      - name: Verify installer AUMID source
        shell: bash
        run: grep -q 'AppUserModelID: "com.gpui-starter.app"' packaging/windows/gpui-starter.iss

      - name: Verify portable zip embeds an icon
        shell: pwsh
        env:
          VER: ${{ steps.version.outputs.value }}
        run: |
          $dst = "$env:RUNNER_TEMP/zipcheck"
          Remove-Item -Recurse -Force $dst -ErrorAction SilentlyContinue
          Expand-Archive -Path "target/release/GPUI-Starter-$env:VER-x86_64-windows.zip" -DestinationPath $dst
          # ICONDIR signature scan; Latin1 round-trip keeps bytes 1:1 and the
          # string search vectorized instead of a per-byte PowerShell loop.
          $exe = [System.IO.File]::ReadAllBytes("$dst/gpui-starter.exe")
          $text = [System.Text.Encoding]::GetEncoding(28591).GetString($exe)
          if (-not $text.Contains([string][char]0 + [char]0 + [char]1 + [char]0)) {
            throw "no ICONDIR signature in gpui-starter.exe - build.rs icon embed broken"
          }
```

Done-check: `grep -c 'GetEncoding(28591)' .github/workflows/release.yml` = 1; the iss grep line matches `packaging/windows/gpui-starter.iss:59-60` content.

#### Step 4.2: pr-builds Windows embedded-icon assertion

Files: `.github/workflows/pr-builds.yml` (build-windows, before Upload `:168`).

```yaml
      - name: Verify exe embeds an icon
        shell: pwsh
        run: |
          $exe = [System.IO.File]::ReadAllBytes("C:\t\debug\gpui-starter.exe")
          $text = [System.Text.Encoding]::GetEncoding(28591).GetString($exe)
          if (-not $text.Contains([string][char]0 + [char]0 + [char]1 + [char]0)) {
            throw "no ICONDIR signature - build.rs icon embed broken"
          }
```

Note in a 1-line comment: icon is embedded by `build.rs` for all Windows builds; AUMID is self-registered at app start (`toast_identity.rs:42-54`), not assertable headless.
Done-check: step present; path matches the lane's `CARGO_TARGET_DIR: 'C:\t'` (`pr-builds.yml:147`).

### Phase 5: publish-latest assets and web widgets

#### Step 5.1: ship the mac zip on the latest release

Files: `.github/workflows/pr-builds.yml` (publish-latest, `:246-261` and `:281-284`).
- Widen the download pattern (`:248`): `pattern: gpui-starter-*-debug` → `pattern: gpui-starter-*-debug*`. The old glob requires the artifact name to END in `-debug`; the new macOS artifact is named `gpui-starter-macos-arm64-debug-app` (Step 1.3), which does not match — without this change the zip is never downloaded and the `cp` below fails.
- Replace `cp artifacts/gpui-starter release/gpui-starter-macos-arm64-debug` with:

```bash
          [ -f artifacts/gpui-starter-macos-arm64-debug-app.zip ] \
            || { echo "Error: mac zip not in ./artifacts (pattern mismatch?)" >&2; exit 1; }
          cp artifacts/gpui-starter-macos-arm64-debug-app.zip \
            release/gpui-starter-macos-arm64-debug-app.zip
```

- Replace the `files:` entry `release/gpui-starter-macos-arm64-debug` with `release/gpui-starter-macos-arm64-debug-app.zip`.
- Delete the stale bare-binary asset, which `softprops/action-gh-release@v3` never removes on its own (it overwrites same-named assets but does not delete names that fall out of `files:` — every previous master push uploaded `gpui-starter-macos-arm64-debug`, so it persists on the `latest` release forever unless removed). Add a step BEFORE the release action, using the `GITHUB_TOKEN` the job already has:

```yaml
          gh release delete-asset latest gpui-starter-macos-arm64-debug \
            --repo "$GITHUB_REPOSITORY" --yes || true
```

  `|| true` because the first post-change run may race a release that has only old assets (the asset exists → deleted) or the deletion may already have happened (→ not found, non-fatal); every later run is a no-op. The `gh` CLI is preinstalled on GitHub-hosted runners; `permissions: contents: write` is already granted to this job by the existing release upload.
Done-check: `grep -n 'pattern:' .github/workflows/pr-builds.yml` shows `gpui-starter-*-debug*`; `grep -n "macos-arm64-debug" .github/workflows/pr-builds.yml` shows no bare-binary name in publish-latest; the staging step's explicit `[ -f … ]` guard makes a silent glob miss impossible (the step goes red naming the pattern, instead of a confusing `cp: cannot stat`).

#### Step 5.2: downloads.ts debug channel

Files: `web/util/downloads.ts`.
- Extend `ChannelLinks` (`:19-29`) with `debugMacArm: string; debugLinuxX64: string; debugWinX64: string;` (doc comment: debug-channel assets, empty on stable).
- `channel()` (`:31-41`) sets the three debug fields to `''`.
- Add:

```ts
function debugChannel(): ChannelLinks {
  const t = 'latest';
  return {
    version: t,
    page: `${RELEASES}/tag/${t}`,
    macUniversal: '', macArm: '', macIntel: '', linuxX64: '', linuxArm: '',
    debugMacArm: `${RELEASES}/download/${t}/gpui-starter-macos-arm64-debug-app.zip`,
    debugLinuxX64: `${RELEASES}/download/${t}/gpui-starter-linux-x86_64-debug.tar.gz`,
    debugWinX64: `${RELEASES}/download/${t}/gpui-starter-windows-x64-debug.exe`,
  };
}
```

- `downloadLinks.latest = debugChannel();`
- Fix the header comment (`:5-10`): the rolling release is published by `pr-builds.yml`'s `publish-latest` job.
Done-check: `bun run typecheck` (in `web/`) passes; the three URLs match Step 5.1 names byte-for-byte.

#### Step 5.3: download.astro latest cards + status workflow

Files: `web/pages/download.astro`.
- In `assetCards` (`:18-42`): first line `if (flavor === 'latest')` return the three debug cards:

```ts
    { platform: 'macOS', arch: 'Apple Silicon', file: 'gpui-starter-macos-arm64-debug-app.zip',
      hint: 'Unzip, then right-click GPUI Starter.app and choose Open — ad-hoc signed debug build.',
      href: links.debugMacArm },
    { platform: 'Linux', arch: 'x86_64', file: 'gpui-starter-linux-x86_64-debug.tar.gz',
      hint: 'Extract, run ./install-linux.sh once to register the desktop entry, then gpui-starter/gpui-starter.',
      href: links.debugLinuxX64 },
    { platform: 'Windows', arch: 'x64', file: 'gpui-starter-windows-x64-debug.exe',
      hint: 'Run gpui-starter.exe (debug; needs the VC++ runtime). The app writes its own Start Menu shortcut for toasts.',
      href: links.debugWinX64 },
```

  The `macHint` ternary (`:19-22`) collapses to the stable string.
- `BuildStatus workflow="latest.yml"` (`:151`) → `workflow="pr-builds.yml"`.
Done-check: `bun run docs:typecheck` passes; `grep -c 'data-asset' web/pages/download.astro` unchanged (cards still carry `data-asset={a.file}` so the live-size enrichment resolves against real asset names, `download.astro:230-240`).

#### Step 5.4: DownloadCta debug map

Files: `web/components/DownloadCta.astro`.
- Build `platformMap` (`:20-25`) per channel: stable keeps the current four entries; latest uses `{ mac: links.debugMacArm, linuxArm: links.page, linuxX64: links.debugLinuxX64, other: links.debugWinX64 }` with labels `Download the master build for macOS` / `… for Linux (x86_64 debug)` / `… for Windows (x64 debug)`; the no-JS `href`/label fallback (`:28-30`) becomes the mac debug link for the latest channel.
Done-check: `bun run typecheck` + `bun run lint` pass; with JS disabled the latest-channel button still resolves to an existing asset.

#### Step 5.5: BuildStatus default workflow

Files: `web/components/BuildStatus.astro`.
- Default prop (`:16`), fallback (`:53`), and the doc comment (`:5`): `latest.yml` → `pr-builds.yml`.
Done-check: `grep -c "pr-builds.yml" web/components/BuildStatus.astro` = 3.

### Phase 6: docs

#### Step 6.1: releasing.md debug-channel section

Files: `docs/releasing.md`.
- Amend line 27 "Skipped: snap, macOS `.app`-in-zip, …" to "Skipped: snap, macOS `.app`-in-zip (stable channel; the debug `latest` channel ships one), macOS `.pkg`, …".
- New subsection after "## The 18 release assets" block (after `:68`): "## The debug channel (`latest` release)" — one paragraph: three assets (names from Interface Contract item 3), produced by `pr-builds.yml` `publish-latest` on every master push, macOS entry is an ad-hoc signed `.app` zip with the full plist (identifier, icns, `NSMicrophoneUsageDescription`), Linux entry mirrors the release tarball staging, Windows entry is the icon-embedded debug exe; each lane smoke-verifies its own artifact; the download page's `latest` cards and `DownloadCta` map to exactly these names via `web/util/downloads.ts`.
- macOS per-format section (`:258-266`): add one sentence — every shipped plist carries `NSMicrophoneUsageDescription` (microphone is plan 18; the UN notification prompt itself has no plist usage key, system text).
Done-check: `grep -c "debug channel" docs/releasing.md` ≥ 1; asset names in the section match Interface Contract item 3.

#### Step 6.2: full gates

Files: none (verification only).
Run, in order, from repo root: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-features`; `cargo test --workspace --all-features`; `just wasm-check`. No Rust changed, so these prove non-regression only. In `web/`: `bun run lint`, `bun run typecheck`, `bun run docs:typecheck`, `bun run docs:build`. `bash -n` every script touched.
Done-check: all commands exit 0 (exact commands above, run verbatim).

## Platform & Wasm Gating

- No `cfg` gates, no `Cargo.toml` changes, no capability-registry entries: the app binary is untouched ("CI-workflow and packaging changes only" per the spec; no gap required app code — Windows AUMID and icon are already runtime/build-time handled, `build.rs:4-10`, `toast_identity.rs:42-54`).
- Native-only dependency table: unchanged; nothing added under `[target.'cfg(not(target_family = "wasm"))'.dependencies]` (`Cargo.toml:310-353`).
- Wasm degradation path: none needed — the wasm harness (`wasm/`) is not distributed through these artifacts. `just wasm-check` (`justfile:24-25`) still passes because no Rust source changes.
- Per-platform behavior summary (spec requires every platform covered):
  - macOS release: unchanged bytes except plist key; smoke added.
  - macOS debug: was bare binary (notifications backend refused, no icon) → now ad-hoc signed `.app` zip, hidden-capability reason removed.
  - Linux release: unchanged recipes, verified + smoke; Linux debug: gains desktop/icon/installer payload.
  - Windows release + debug: unchanged bytes; smoke added; AUMID guaranteed by `gpui-starter.iss:56-60` (install) and runtime self-registration (portable/debug).
  - Flatpak: untouched lane (`release.yml:470-520`), portal notifications already the sandbox path (`packaging/flatpak/com.gpui-starter.app.yml:28`).

## i18n

No app-facing strings: the locale catalogs (`locales/en.yml`, `locales/zh-CN.yml`) are untouched, and the crate facade (`crate::i18n::localize`, `src/services/i18n.rs:3-13`) is not called from anything this plan changes. The web download page is English-only (no locale layer exists under `web/`), so the new card hints are single-language site copy. Empty table, by construction:

| Key | en | zh-CN |
|---|---|---|
| (none) | — | — |

## Design Compliance

- No GPUI surface is created or changed; layout, density, tokens, interaction states, overlays, motion sections of the design guides do not apply to CI artifacts.
- Interface language (`gpui-kit-design-guides/references/design-guides.md`, "Interface language", `:692+`): new download-card hints use nouns for objects and verbs for commands, shortest accurate wording ("Unzip, then right-click GPUI Starter.app and choose Open"), no wrappers like "Management"/"System"; button labels describe the result ("Download the master build for macOS", matching the existing `Download v{ver} for …` pattern, `DownloadCta.astro:21-24`).
- Context economy (`design-guides.md`, "Let context carry context"): the debug cards live under the "Latest build" heading, so hints state only what changes the decision (ad-hoc signing, install step, VC++ runtime) and do not restate what the section already establishes.
- Progressive-enhancement contract preserved: `BuildStatus` still hides on API failure (`BuildStatus.astro:90-94`) and the static page stands alone (`download.astro:216-219` comment); fixing the workflow name restores the live strip without changing its states.
- A11y checklist (`design-guides.md:848+`): no focus order, contrast, or name changes; new anchors keep descriptive link text ("Download" cells unchanged in structure).

## Verification

Repo gates (AGENTS.md), run verbatim even though no Rust changes:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Web gates (in `web/`): `bun run lint`, `bun run typecheck`, `bun run docs:typecheck`, `bun run docs:build`.
Script syntax: `bash -n scripts/create-release-app.sh` (+ unchanged scripts incidentally).

Feature-specific manual QA (acceptance criteria):

1. Dispatch `pr-builds.yml` on a branch (or push to master): each lane's smoke step is green; the run page carries `gpui-starter-macos-arm64-debug-app` (zip), Linux tarball, Windows exe.
2. On the `latest` release: exactly `gpui-starter-macos-arm64-debug-app.zip`, `gpui-starter-linux-x86_64-debug.tar.gz`, `gpui-starter-windows-x64-debug.exe` and no `gpui-starter-macos-arm64-debug` bare-binary asset — the delete-asset step in Step 5.1 removes it on the first post-change run (`action-gh-release` never deletes fallen-off names itself).
3. macOS: download the zip on an Apple Silicon mac, unzip, right-click → Open (Gatekeeper click-through for ad-hoc), app shows the bundle icon in the Dock; open the notifications page and send a native notification — the permission prompt appears and the notification arrives (backend no longer logs "macOS bundle identifier missing"; `user_notify.rs:249-252`).
4. Linux: extract the debug tarball, run `./install-linux.sh`, launch `gpui-starter/gpui-starter`, send a notification from the notifications page — banner shows the GPUI Starter icon and groups under the desktop entry (`notify_rust.rs:109-117`).
5. Windows: run the debug exe, send a notification — toast appears (first launch writes the AUMID shortcut, `toast_identity.rs:126-153`); exe shows the embedded icon in Explorer.
6. Web: `bun run docs:build && bun run docs:preview` — the Latest section shows the 3 debug cards with live sizes (proves filename match), the BuildStatus strip renders (proves `pr-builds.yml` runs query resolves), and every Download link returns 200.
7. Stable channel regression: tag `v0.2.x` dispatch on the fork — 18 assets unchanged, `SHA256SUMS.txt` and `manifest.json` generate as before (names pinned by `release.yml:571-589`, `generate-manifest.sh:52-53`).

Checks this plan could NOT run at planning time (no CI execution from a docs-only scope): steps 1–2 and the runner-tool assumptions (`/usr/libexec/PlistBuddy`, `ditto` flags, `hdiutil -mountpoint` behavior, the pwsh Latin1 scan on a ~100 MB exe). They are the first-run watch list, same discipline as `docs/releasing.md:154-158`.

## Risks & Open Questions

Ranked:

1. **Runner tool drift** (`/usr/libexec/PlistBuddy`, `ditto` flags, `hdiutil -mountpoint`, pwsh encoding behavior on windows-2025). Mitigation: absolute paths where the tool is not on default PATH, and `create-dmg.sh` already exercises hdiutil on the same runner. First dispatch must watch the smoke steps (Verification step 1).
2. **ICONDR scan false-pass**: the 4-byte signature can occur by chance in a large exe (~2% at 100 MB), so the Windows gate could theoretically pass with a broken embed. Accepted: absence fails deterministically (the embed copies the ico's own header bytes verbatim into RT_GROUP_ICON), and a false pass requires the exact pattern in unrelated data. Alternative (PE resource-section parser in pwsh) was rejected as disproportionate for a smoke step.
3. **Double-zip UX**: `actions/upload-artifact` wraps the ditto zip again, so the macOS download is a zip containing a zip. Accepted — matches the existing convention of uploading packaged files (`.tar.gz`, `.dmg`) and keeps artifact names stable.
4. **Gatekeeper on ad-hoc debug apps**: browser download quarantines the zip; right-click Open is required and cannot be pre-verified in CI. Mitigated by the card hint and job-summary text.
5. **`latest` release consumers**: `grep` across repo (excluding `node_modules`, `dist`, workflow-state dirs) found debug asset names only inside `pr-builds.yml` and (after this plan) `web/util/downloads.ts`; the AppImage zsync target names stable-channel AppImages only (`build-appimage.sh:119`). Renaming the mac asset breaks nothing else.
6. **String divergence with plan 18**: the two plans edit DIFFERENT scripts — 18 lands `NSMicrophoneUsageDescription` in `scripts/macos-dev-app.sh` (dev bundle), 20 in `scripts/create-release-app.sh` (release + host-debug bundles); no merge-order conflict exists. Residual risk: one implementer rewording the pinned string so the dev and release plists diverge. Both plans pin the same bytes and each one's done-check greps its own script for the exact string, so a reword fails the OTHER plan's check only if that plan re-runs — each plan's text says "do not reword without editing both plans".

Open questions (each with what was tried):

1. **Is a universal debug DMG wanted on `latest`?** Tried: reading `pr-builds.yml` for an x86_64 mac lane (none; macos-latest is arm64-only, `:185`). Adding one doubles the mac lane cost for marginal benefit; deferred — the zip serves notification/bundle testing, which is the spec's target.
2. **Does Apple's notification permission prompt read any plist string?** Tried: web search 2026-10-12 across Apple docs, forums, Stack Overflow — no such key exists on macOS; prompt text is system-generated. Plan therefore ships only `NSMicrophoneUsageDescription`; if Apple ships a notification key later, the single heredoc in `create-release-app.sh` is the place to add it.
3. **Plan 18 string ownership.** Verified this session: `docs/plans/18-microphone.md:1029-1039` (Step 6.1, `scripts/macos-dev-app.sh` only) pins "Captures microphone audio to display the live input level and waveform."; ownership split recorded at `:26-34`, seam item at `:451-454`. If its implementer rewords, the dev and release plists diverge — each plan's done-check greps only its own script, so divergence is caught by re-running the other plan's check; the fix is a one-line sync. Tried: no shared constant is possible across two bash heredocs in different scripts without a refactor neither plan budgets.
4. **`hdiutil attach` under concurrent runner load** (the "Resource busy" failure mode `release.yml:51-54` disables Spotlight for). Tried: reading the existing workflow — the disable-Spotlight step runs before all build-macos steps, which covers the smoke step too; residual risk is mount-point collisions, avoided by `mktemp -d`.
