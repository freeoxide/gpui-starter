import { latestVersion } from './changelog';

// Single source of truth for every download link on the site.
//
// Two channels exist:
//   - stable: the newest tagged release (v{latestVersion} — the same version
//     the changelog and hero advertise).
//   - latest: a rolling release, tag `latest`, deleted and recreated by the
//     Latest Build workflow (.github/workflows/latest.yml) on every push to
//     master.
//
// Asset names MUST stay in lockstep with release.yml / latest.yml — the DMG
// loop and the Linux tarball step define these filenames. If a workflow
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
  };
}

export const downloadLinks: Record<Channel, ChannelLinks> = {
  stable: channel(`v${latestVersion}`, latestVersion),
  latest: channel('latest', 'latest'),
};

/** Links for any past tagged release, e.g. channelFor('0.2.0'). */
export const channelFor = (version: string): ChannelLinks =>
  channel(`v${version}`, version);

export { RELEASES as RELEASES_URL };
