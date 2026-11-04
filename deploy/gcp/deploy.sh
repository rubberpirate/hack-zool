#!/usr/bin/env bash
# Deploy a committed revision without exporting local secrets or changing the checkout.
set -euo pipefail
usage() {
  cat <<'HELP'
Usage: ./deploy/gcp/deploy.sh [--check] [GIT_REF]

Default: fetch and deploy origin/main. Use HEAD for the latest local commit,
or a commit/tag for a specific revision. Uncommitted files are never deployed.
--check validates the committed archive without connecting to the VM.

Optional environment: ZOOL_SSH_TARGET (default zooldeploy@34.47.183.93),
ZOOL_SSH_KEY (default ~/.ssh/zool_gcp_deploy).
HELP
}
check=false
if [[ ${1:-} == --help || ${1:-} == -h ]]; then usage; exit 0; fi
if [[ ${1:-} == --check ]]; then check=true; shift; fi
if (( $# > 1 )); then usage >&2; exit 2; fi
ref=${1:-origin/main}
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo=$(git -C "$script_dir" rev-parse --show-toplevel)
cd "$repo"
if [[ -n $(git status --porcelain) ]]; then
  echo 'Note: uncommitted changes are excluded. Commit/push current fixes before deploying.' >&2
fi
if [[ $ref == origin/* ]]; then
  branch=${ref#origin/}
  git check-ref-format --branch "$branch" >/dev/null
  git fetch --no-tags origin "$branch"
  ref=FETCH_HEAD
fi
revision=$(git rev-parse --verify --end-of-options "${ref}^{commit}")
staging=$(mktemp -d)
trap 'rm -rf -- "$staging"' EXIT
archive=$staging/source.tar.gz
git archive --format=tar.gz --output="$archive" "$revision"
python3 - "$archive" <<'PY'
import sys, tarfile
from pathlib import PurePosixPath
required = {
    'Dockerfile', 'package-lock.json', 'zsplit-backend/Dockerfile',
    'deploy/gcp/compose.yaml', 'deploy/gcp/Caddyfile', 'deploy/gcp/release.sh',
}
with tarfile.open(sys.argv[1]) as archive:
    files = {entry.name for entry in archive.getmembers() if entry.isfile()}
    missing = required - files
    if missing:
        raise SystemExit('Selected commit is missing deployment files: ' + ', '.join(sorted(missing))
                         + '. Commit and push the current deployment fixes first.')
    for entry in archive.getmembers():
        parts = PurePosixPath(entry.name).parts
        if not parts or entry.name.startswith('/') or '..' in parts:
            raise SystemExit('Unsafe archive path; refusing deployment.')
        for part in parts:
            if ((part == '.env' or part.startswith('.env.')) and part != '.env.example') or part.endswith(('.seed', '.sqlite', '.sqlite3', '.sqlite3-wal', '.sqlite3-shm', '.pem', '.key')):
                raise SystemExit('Sensitive file is tracked in the selected commit; refusing deployment: ' + entry.name)
    archive.extractfile('deploy/gcp/release.sh').read()  # Ensure helper is a regular readable entry.
PY
printf 'Selected commit: %s\n' "$revision"
if "$check"; then
  echo 'Committed archive validated. No VM changes made.'
  exit 0
fi
target=${ZOOL_SSH_TARGET:-zooldeploy@34.47.183.93}
key=${ZOOL_SSH_KEY:-$HOME/.ssh/zool_gcp_deploy}
if [[ ! $target =~ ^[a-z_][a-z0-9_-]*@[a-zA-Z0-9][a-zA-Z0-9.-]*$ || ! -r $key ]]; then
  echo 'Set a valid ZOOL_SSH_TARGET and readable ZOOL_SSH_KEY.' >&2
  exit 2
fi
options=(-i "$key" -o BatchMode=yes -o StrictHostKeyChecking=yes)
remote_temp=$(ssh "${options[@]}" "$target" 'mktemp -d /tmp/zool-deploy.XXXXXXXX')
if [[ ! $remote_temp =~ ^/tmp/zool-deploy\.[a-zA-Z0-9]+$ ]]; then
  echo 'Unexpected remote staging path; refusing deployment.' >&2
  exit 1
fi
# Extract only the committed helper; do not run a different working-tree version remotely.
tar -xOf "$archive" deploy/gcp/release.sh > "$staging/release.sh"
status=0
scp "${options[@]}" "$archive" "$staging/release.sh" "$target:$remote_temp/" || status=$?
if (( status == 0 )); then
  ssh "${options[@]}" "$target" "sudo bash $remote_temp/release.sh $revision $remote_temp/source.tar.gz" || status=$?
fi
ssh "${options[@]}" "$target" "rm -rf -- $remote_temp" || true
exit "$status"
