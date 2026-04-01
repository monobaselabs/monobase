#!/usr/bin/env bash
# Run E2E tests against the Rust API server.
# Expects: cargo run in services/api-rs (port 7213)
set -euo pipefail

export API_URL="${API_URL:-http://localhost:7213}"
export DATABASE_URL="${DATABASE_URL:-postgresql://postgres:password@127.0.0.1:5432/monobase}"
export AUTH_ADMIN_EMAILS="admin-test@test.local"

echo "Running E2E tests against Rust server at $API_URL"
cd "$(dirname "$0")/.."
bun test "$@"
