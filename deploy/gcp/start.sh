#!/bin/bash
set -euo pipefail
if [[ $EUID -ne 0 ]]; then
  echo 'Run with sudo on the deployment VM.' >&2
  exit 1
fi
cd /opt/zool/app
compose=(docker compose --env-file /opt/zool/config/production.env -f deploy/gcp/compose.yaml)
"${compose[@]}" config --quiet
"${compose[@]}" build
python3 zsplit-backend/scripts/download-proving-params.py
install -d -m 0755 /opt/zool/state/zcash/params
cp zsplit-backend/data/params/*.params /opt/zool/state/zcash/params/
chown -R 1000:1000 /opt/zool/state/zcash
if [[ ! -f /opt/zool/state/zcash/coordinator.seed ]]; then
  if [[ -f /opt/zool/state/zcash/wallet/wallet.sqlite3 ]]; then
    echo 'Existing wallet has no seed: restore its original seed; refusing to replace it.' >&2
    exit 1
  fi
  "${compose[@]}" run --rm --no-deps --entrypoint zsplit-wallet api seed
fi
"${compose[@]}" up -d
"${compose[@]}" ps
