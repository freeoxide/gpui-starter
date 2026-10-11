# Windows packaging

Three release artifacts, all produced on a Windows host with Git Bash (the
GHA `windows-2025` runner; ISCC and the MSVC toolchain are preinstalled
there, see `docs/research/icons-and-windows.md` B1-B3):

| Artifact | Builder | Name |
|---|---|---|
| Portable zip | `scripts/build-windows-portable.sh` | `GPUI-Starter-<ver>-x86_64-windows.zip` |
| Bare exe (updater target) | same script | `GPUI-Starter-<ver>-x86_64-windows.exe` |
| Installer | `scripts/build-windows-installer.sh` | `GPUI-Starter-<ver>-x86_64-windows-setup.exe` |

Everything lands in `target/release/` with a `.sha256` sidecar next to it.
The portable zip holds `gpui-starter.exe`, `LICENSE`, and `README.md` at its
root. The bare exe must stay a release asset of its own: the in-app updater
swaps single files, not archives, and the update manifest's `windows-x86_64`
entry points at it.

Both scripts run under Git Bash and hard-fail unless
`GPUI_UPDATE_MANIFEST_URL` is set (portable) or the built exe exists
(installer). Neither is runnable on the Linux dev box; the justfile
recipes `win-portable` / `win-installer` exist for Windows CI and Windows
dev machines.

## Installer design

`gpui-starter.iss` follows Zed's `zed.iss`: per-user install
(`PrivilegesRequired=lowest`, `{autopf}` = `%LOCALAPPDATA%\Programs`), the
committed `assets/app-icon/gpui-starter.ico` as `SetupIconFile`, an
optional desktop-icon task (unchecked), and an optional `SignTool` behind
`/DSignToolName`. The `AppId` GUID is `uuid5(DNS, "com.gpui-starter.app")`
and must never change: Inno treats a new GUID as a different app, so
upgrades would install side by side instead of replacing in place.

The installer deliberately registers no protocols and no file
associations. The app is its own integration layer and runs it on every
startup, installer or not:

- `src/platform/toast_identity.rs` writes `gpui-starter.lnk` into the
  user's Start Menu Programs folder carrying
  `System.AppUserModel.ID = com.gpui-starter.app`; without that property
  Windows silently drops every toast. The installer's Start Menu shortcut
  uses the same filename and AUMID, so first launch rewrites (or confirms)
  the same file instead of adding a second entry.
- `src/platform/deep_link_registration.rs` registers `gpui-starter://`
  under `HKCU\Software\Classes` for the running exe. Consequently the
  uninstaller does not touch that key; a stale registration pointing at a
  removed exe is the accepted trade-off of the self-service model (same as
  the Linux tarball).

The exe icon comes from `build.rs` at the repo root: `winresource` embeds
`assets/app-icon/gpui-starter.ico` into the .exe resource section whenever
`CARGO_CFG_TARGET_OS=windows`, which makes title bar, taskbar, and the
portable exe show the app icon with no runtime work. On non-Windows hosts
the build script is a no-op (the crate still compiles as a host build-dep,
nothing invokes it). MSVC builds need `rc.exe` from the Windows SDK; the
GHA images ship it, and `scripts/windows-dev-env.sh --setup` fetches one
for bare dev machines.

## Updater on Windows: known gap (not implemented here)

`src/services/updater/apply.rs` performs the pending swap with a single
`std::fs::rename(new, current)`; only macOS has a special branch. Windows
does not allow replacing a running exe, so on Windows the updater
downloads and verifies an update and then fails at the swap step. The fix,
deliberately out of scope for the release-infrastructure work that added
this packaging (it lives in `src/`), is a two-step rename dance behind a
windows cfg branch in `apply.rs`:

1. rename the running `gpui-starter.exe` to `gpui-starter.exe.old`
   (renaming a running exe is allowed),
2. move the downloaded exe into the old name,
3. delete `gpui-starter.exe.old` on the next launch.

Per-user installation (`%LOCALAPPDATA%\Programs`) is what keeps the
destination writable without elevation; a Program Files install would
break the updater outright. Until that patch lands, treat Windows
auto-update as download-verify-only.

## Code signing

Default: unsigned, plus the SHA256SUMS and Ed25519-signed manifest the
release workflow already publishes. Every GPUI app except Zed ships this
way; unsigned exes show the SmartScreen click-through until download
reputation builds.

Options when signing becomes worthwhile:

- Azure Artifact Signing (formerly Trusted Signing): Microsoft-trusted
  certificates that build SmartScreen reputation. Basic tier USD 9.99/month
  for 5,000 signatures; individual developers can onboard only in the USA
  and Canada (organizations also EU/UK). Wire it as a CI step gated on
  `AZURE_*` secrets in the pattern of Zed's `sign.ps1`, then compile with
  `ISCC /DSignToolName=azure /Sazure=<sign command>` so the dormant
  `SignTool` line in the .iss activates.
- osslsigncode with a self-made certificate: not recommended. It adds
  warning friction and earns no reputation, which is strictly worse than
  shipping unsigned.

## winget

A winget listing is a community PR to `microsoft/winget-pkgs` after the
first tagged release: YAML manifests with `PackageIdentifier`
(`Publisher.Package`), `InstallerUrl`, and `InstallerSha256` that must
match the hosted binary byte for byte. winget supports ZIP portable
packages, so the entry can point at `GPUI-Starter-<ver>-x86_64-windows.zip`
(take the hash from the release's `.sha256` sidecar). Hold off until a
stable download URL exists, then submit; publisher/domain consistency
checks run under moderation.
