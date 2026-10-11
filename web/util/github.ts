// Purely client-side GitHub API access for the live CI/release widgets.
//
// api.github.com is public and CORS-enabled for public repos: the browser
// queries it directly with unauthenticated fetch() — no server, no keys, no
// proxy. Unauthenticated requests are capped at 60/hour per IP, so responses
// are cached in sessionStorage keyed by path and revalidated with ETag
// conditional requests — a 304 costs nothing against the rate limit.

export const GH_REPO = 'freeoxide/gpui-starter';
export const GH_API = `https://api.github.com/repos/${GH_REPO}`;

export interface GhAsset {
  name: string;
  size: number;
  download_count: number;
  browser_download_url: string;
}

export interface GhRelease {
  tag_name: string;
  published_at: string;
  html_url: string;
  assets: GhAsset[];
}

export interface GhWorkflowRun {
  status: string;
  conclusion: string | null;
  head_sha: string;
  head_branch: string | null;
  display_title: string;
  html_url: string;
  created_at: string;
  run_started_at?: string;
}

export interface GhRunsResponse {
  total_count: number;
  workflow_runs: GhWorkflowRun[];
}

export interface GhCompare {
  status: string;
  ahead_by: number;
  behind_by: number;
}

export type GhResult<T> =
  | { ok: true; data: T }
  | { ok: false; reason: 'not-found' | 'rate-limit' | 'error' };

export async function ghFetch<T>(path: string): Promise<GhResult<T>> {
  const url = path.startsWith('https://') ? path : `${GH_API}${path}`;
  const cacheKey = `gh-etag:${path}`;
  const headers: Record<string, string> = {
    Accept: 'application/vnd.github+json',
    'X-GitHub-Api-Version': '2022-11-28',
  };

  let cached: { etag?: string; body?: T } = {};
  try {
    const raw = sessionStorage.getItem(cacheKey);
    if (raw) {
      cached = JSON.parse(raw) as { etag?: string; body?: T };
      if (cached.etag) headers['If-None-Match'] = cached.etag;
    }
  } catch {
    /* storage unavailable (private mode) — fetch fresh */
  }

  try {
    const res = await fetch(url, { headers });
    if (res.status === 304 && cached.body !== undefined) return { ok: true, data: cached.body };
    if (res.status === 404) return { ok: false, reason: 'not-found' };
    if (res.status === 403 && res.headers.get('x-ratelimit-remaining') === '0') {
      return { ok: false, reason: 'rate-limit' };
    }
    if (!res.ok) return { ok: false, reason: 'error' };

    const data = (await res.json()) as T;
    const etag = res.headers.get('ETag');
    if (etag) {
      try {
        sessionStorage.setItem(cacheKey, JSON.stringify({ etag, body: data }));
      } catch {
        /* storage full/unavailable — cache is best-effort */
      }
    }
    return { ok: true, data };
  } catch {
    return { ok: false, reason: 'error' };
  }
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB'];
  let value = bytes / 1024;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i++;
  }
  return `${value >= 100 ? Math.round(value) : value.toFixed(1)} ${units[i]}`;
}

export function formatCount(n: number): string {
  return n >= 1000 ? `${(n / 1000).toFixed(n >= 10000 ? 0 : 1)}k` : String(n);
}

export function timeAgo(iso: string, now = Date.now()): string {
  const seconds = Math.max(1, Math.round((now - new Date(iso).getTime()) / 1000));
  const rtf = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });
  if (seconds < 60) return rtf.format(-seconds, 'second');
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return rtf.format(-minutes, 'minute');
  const hours = Math.round(minutes / 60);
  if (hours < 24) return rtf.format(-hours, 'hour');
  return rtf.format(-Math.round(hours / 24), 'day');
}
