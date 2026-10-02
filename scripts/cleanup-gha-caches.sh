#!/usr/bin/env bash
#
# Delete GitHub Actions caches that can no longer speed up a future build.
# Complements GitHub's own 10 GB LRU eviction, which only fires once the
# quota is already full:
#
#   refs/pull/*/merge            PR-scoped caches, useless once the PR goes
#                                idle (visible only inside that PR anyway)
#   refs/tags/*                  only restorable by re-runs of the same tag
#   refs/heads/<other>           only restorable inside that branch
#   refs/heads/<default branch>  kept while recently accessed; older ones are
#                                superseded by newer per-commit rust-cache
#                                entries and left for LRU only as fallback
#
# TTLs are days since last access, configurable via env:
#   PR_TTL_DAYS (3), TAG_TTL_DAYS (14), BRANCH_TTL_DAYS (7),
#   DEFAULT_TTL_DAYS (30)
#
# RETIRED_KEY_PREFIXES (comma-separated) are cache families whose writer no
# longer exists in any workflow; they can never be read again, so they are
# deleted regardless of TTL or ref. sccache/ was retired by the kache
# migration and its ~1.8k stale master entries otherwise sit out the
# 30-day default-branch TTL.
#
# DRY_RUN=1 lists what would be deleted without deleting anything.
# Requires gh with GH_TOKEN set; deletion needs the actions:write scope.

set -euo pipefail

: "${GH_TOKEN:?GH_TOKEN must be set}"

REPO="${GITHUB_REPOSITORY:-$(gh repo view --json nameWithOwner --jq .nameWithOwner)}"
DEFAULT_BRANCH="${DEFAULT_BRANCH:-$(gh repo view "$REPO" --json defaultBranchRef --jq .defaultBranchRef.name)}"

PR_TTL_DAYS="${PR_TTL_DAYS:-3}"
TAG_TTL_DAYS="${TAG_TTL_DAYS:-14}"
BRANCH_TTL_DAYS="${BRANCH_TTL_DAYS:-7}"
DEFAULT_TTL_DAYS="${DEFAULT_TTL_DAYS:-30}"
RETIRED_KEY_PREFIXES="${RETIRED_KEY_PREFIXES:-sccache/}"

NOW="$(date +%s)"
DELETED_LOG="$(mktemp)"
trap 'rm -f "$DELETED_LOG"' EXIT

echo "Cache cleanup for ${REPO} (default branch: ${DEFAULT_BRANCH})"
echo "TTLs: pr=${PR_TTL_DAYS}d tag=${TAG_TTL_DAYS}d branch=${BRANCH_TTL_DAYS}d default=${DEFAULT_TTL_DAYS}d${DRY_RUN:+ (dry run)}"
echo "Retired key prefixes: ${RETIRED_KEY_PREFIXES//,/ }"
echo

gh api --paginate "repos/${REPO}/actions/caches?per_page=100" \
  --jq '.actions_caches[] | [.id, (.ref // ""), .last_accessed_at, .key] | @tsv' |
while IFS=$'\t' read -r id ref accessed key; do
  retired=0
  for prefix in ${RETIRED_KEY_PREFIXES//,/ }; do
    case "$key" in "$prefix"*) retired=1; break ;; esac
  done

  if (( retired )); then
    echo "delete retired             ${ref:-<no-ref>}   ${key}"
    if [ -z "${DRY_RUN:-}" ]; then
      gh api -X DELETE "repos/${REPO}/actions/caches/${id}" >/dev/null
    fi
    echo "$key" >>"$DELETED_LOG"
    continue
  fi

  age_days=$(( (NOW - "$(date -d "$accessed" +%s)") / 86400 ))

  case "$ref" in
    refs/pull/*) ttl="$PR_TTL_DAYS" ;;
    refs/tags/*) ttl="$TAG_TTL_DAYS" ;;
    "refs/heads/$DEFAULT_BRANCH") ttl="$DEFAULT_TTL_DAYS" ;;
    *) ttl="$BRANCH_TTL_DAYS" ;;
  esac

  if (( age_days < ttl )); then
    echo "keep   ${age_days}d < ${ttl}d   ${ref:-<no-ref>}   ${key}"
    continue
  fi

  echo "delete ${age_days}d >= ${ttl}d   ${ref:-<no-ref>}   ${key}"
  if [ -z "${DRY_RUN:-}" ]; then
    gh api -X DELETE "repos/${REPO}/actions/caches/${id}" >/dev/null
  fi
  echo "$key" >>"$DELETED_LOG"
done

echo
echo "Deleted $(wc -l <"$DELETED_LOG") cache(s)${DRY_RUN:+ (dry run: nothing was actually deleted)}"
