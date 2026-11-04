#!/usr/bin/env bash
# Runs on the existing VM; never initializes or replaces wallet/credential files.
set -euo pipefail
umask 077
if (( $# != 2 )) || [[ ! $1 =~ ^[0-9a-f]{40,64}$ ]]; then
  echo 'Usage: release.sh COMMIT_SHA SOURCE_ARCHIVE' >&2
  exit 2
fi
revision=$1
archive=$2
base=${ZOOL_DEPLOY_ROOT:-/opt/zool}
config=$base/config/production.env
[[ -r $config && -f $base/state/zcash/coordinator.seed ]] || {
  echo 'Existing production configuration and seed are required. This is an update, not first-time setup.' >&2
  exit 1
}
exec 9>"$base/deploy.lock"
flock -n 9 || { echo 'Another deployment is already running.' >&2; exit 1; }
previous=$base/current
[[ -L $previous ]] || previous=$base/app
[[ -f $previous/deploy/gcp/compose.yaml ]] || { echo 'Previous deployment is missing.' >&2; exit 1; }
release=$base/releases/$revision
mkdir -p "$release"
tar --extract --gzip --file "$archive" --directory "$release" --no-same-owner
compose=(docker compose --project-name zool --env-file "$config" -f "$release/deploy/gcp/compose.yaml")
old=(docker compose --project-name zool --env-file "$config" -f "$previous/deploy/gcp/compose.yaml")
export ZOOL_RELEASE=$revision
"${compose[@]}" config --quiet
printf 'Building %s while current services remain running.\n' "$revision"
"${compose[@]}" build api web
domain=$(python3 - "$config" <<'PY'
import re, sys
for line in open(sys.argv[1]):
    if line.startswith('DOMAIN='):
        value = line.split('=', 1)[1].strip().strip('"').strip("'")
        if re.fullmatch(r'[a-zA-Z0-9][a-zA-Z0-9.-]*', value):
            print(value)
            break
else:
    raise SystemExit('Missing or invalid DOMAIN configuration')
PY
)
[[ -n $domain ]] || { echo 'Invalid deployment domain.' >&2; exit 1; }
docker run --rm --entrypoint caddy -e "DOMAIN=$domain" \
  -v "$release/deploy/gcp/Caddyfile:/etc/caddy/Caddyfile:ro" \
  caddy:2-alpine validate --config /etc/caddy/Caddyfile
backup=$base/backups/$(date -u +%Y%m%dT%H%M%SZ)-${revision:0:12}-$$
mkdir -p "$backup"
stopped=false
applied=false
finish() {
  result=$?
  if (( result != 0 )); then
    if "$stopped" && ! "$applied"; then
      echo 'Update failed before application; restarting the previous API.' >&2
      "${old[@]}" start api || true
    fi
    echo "Deployment failed. Backup/recovery evidence: $backup" >&2
    if "$applied"; then
      echo 'New services may be running. No automatic rollback: migrations may have changed the database.' >&2
    fi
  fi
  exit "$result"
}
trap finish EXIT
container=$("${old[@]}" ps -q api)
if [[ -n $container ]]; then
  echo 'Waiting for the current wallet worker to shut down gracefully (no forced timeout).'
  docker container stop --timeout -1 "$container" >/dev/null
  stopped=true
fi
# No wallet writer runs during the native backup; pg_dump provides a consistent DB snapshot.
"${old[@]}" exec -T postgres pg_dump -U zsplit -d zsplit --format=custom > "$backup/postgres.dump"
tar -czf "$backup/zcash.tar.gz" -C "$base/state" zcash
cp -- "$config" "$backup/production.env"
printf '%s\n' "$(readlink -f "$previous")" > "$backup/previous-source"
touch "$backup/complete"
applied=true
"${compose[@]}" up -d --no-build
for path in /readyz /; do
  curl --fail --silent --show-error --retry 30 --retry-all-errors --retry-delay 2 \
    --max-time 15 "https://$domain$path" >/dev/null
done
ln -s "$release" "$base/.current-$revision-$$"
mv -Tf "$base/.current-$revision-$$" "$base/current"
printf '%s\n' "$revision" > "$base/deployed-commit"
"${compose[@]}" ps
printf 'Deployed %s to https://%s\nBackup: %s\n' "$revision" "$domain" "$backup"
