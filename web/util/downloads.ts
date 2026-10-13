import { latestVersion } from './changelog';

// Single source of truth for every download link on the site.
//
// Two channels exist:
//   - stable: the newest tagged release (v{latestVersion} — the same version
//     the changelog and hero advertise).
//   - latest: a rolling release, tag `latest`, whose assets the
//     publish-latest job of .github/workflows/pr-builds.yml replaces on
//     every push to master.
//
// Asset names MUST stay in lockstep with release.yml / pr-builds.yml — the
// DMG loop and the two tarball steps define these filenames. If a workflow
// renames an asset, rename it here too.
const RELEASES = 'https://github.com/freeoxide/gpui-starter/releases';

export type Channel = 'stable' | 'latest';

export interface ChannelLinks {
  /** Asset tag embedded in filenames: "0.3.0" for stable, "latest" for the rolling release. */
  version: string;
  /** The GitHub release page (release notes, per-asset listing). */
  page: string;
  macUniversal: string;
  macArm: string;
  macIntel: string;
  linuxX64: string;
  linuxArm: string;
  /** Debug-channel assets on the rolling `latest` release; empty on stable. */
  debugMacArm: string;
  debugLinuxX64: string;
  debugWinX64: string;
}

function channel(tag: string, t: string): ChannelLinks {
  return {
    version: t,
    page: `${RELEASES}/tag/${tag}`,
    macUniversal: `${RELEASES}/download/${tag}/GPUI-Starter-${t}.dmg`,
    macArm: `${RELEASES}/download/${tag}/GPUI-Starter-${t}-aarch64.dmg`,
    macIntel: `${RELEASES}/download/${tag}/GPUI-Starter-${t}-x86_64.dmg`,
    linuxX64: `${RELEASES}/download/${tag}/gpui-starter-${t}-x86_64-linux.tar.gz`,
    linuxArm: `${RELEASES}/download/${tag}/gpui-starter-${t}-aarch64-linux.tar.gz`,
    debugMacArm: '',
    debugLinuxX64: '',
    debugWinX64: '',
  };
}

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

export const downloadLinks: Record<Channel, ChannelLinks> = {
  stable: channel(`v${latestVersion}`, latestVersion),
  latest: debugChannel(),
};

/** Links for any past tagged release, e.g. channelFor('0.2.0'). */
export const channelFor = (version: string): ChannelLinks =>
  channel(`v${version}`, version);

export { RELEASES as RELEASES_URL };
