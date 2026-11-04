#!/usr/bin/env bash
set -euo pipefail

# scripts/export-backend.sh
# Cleanly exports the Z-Split Rust backend into an isolated, standalone repository
# directory free of historical frontend commits or pre-event artifacts.

TARGET_DIR="${1:-../zsplit-standalone-backend}"

echo "=== Exporting Z-Split Backend to $TARGET_DIR ==="
mkdir -p "$TARGET_DIR"

# Copy backend root files and directories, excluding target/ and temporary files
rsync -av \
  --exclude="target/" \
  --exclude="data/" \
  --exclude=".DS_Store" \
  zsplit-backend/ "$TARGET_DIR/"

# Also copy CI configuration
mkdir -p "$TARGET_DIR/.github/workflows"
if [ -f .github/workflows/ci.yml ]; then
  cp .github/workflows/ci.yml "$TARGET_DIR/.github/workflows/ci.yml"
fi

cat << 'EOF' > "$TARGET_DIR/PROVENANCE.md"
# Z-Split Backend — Provenance & Hackathon Integrity

This repository contains the standalone Rust backend for Z-Split, developed during the
Free Society Hack (Nov 2026).

- Built with Rust + Tokio + Axum + official librustzcash crates + PostgreSQL SQLx.
- Cleanly isolated from prior web experimental mock interfaces.
- Contains no synthetic transaction IDs, mock blockchain success assertions, or fake funding.
EOF

echo "Export completed successfully at $TARGET_DIR"
