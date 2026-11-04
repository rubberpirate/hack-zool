#!/usr/bin/env bash
set -euo pipefail

# scripts/demo.sh
# Runs the 10-step end-to-end Z-Split demonstration workflow.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BACKEND_DIR="$PROJECT_ROOT/zsplit-backend"

echo "=== Running Z-Split End-to-End Workflow Demonstration ==="
cd "$BACKEND_DIR"

cargo run --package zsplit-api --example workflow_demo
