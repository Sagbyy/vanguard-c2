#!/usr/bin/env bash
# Launch the whole stack locally for development:
#   ./dev.sh        — NATS + vanguard-map + vanguard-control + dashboard
#                     (Ctrl-C stops everything)
#
# Fast iteration: cargo builds incrementally and the dashboard runs with HMR.
# For a zero-setup, fully containerised run instead, use: docker compose up --build
set -uo pipefail
cd "$(dirname "$0")"

export NATS_URL="${NATS_URL:-nats://127.0.0.1:4222}"
export RECOGNITION_RANGE_M="${RECOGNITION_RANGE_M:-4000}"

# Stop any previous run of OUR binaries (a shared NATS is left alone / reused).
pkill -f 'target/debug/vanguard-map'     2>/dev/null || true
pkill -f 'target/debug/vanguard-control' 2>/dev/null || true
sleep 1

pids=()
nats_pid=""
cleanup() {
  echo; echo "› stopping stack…"
  [ -n "$nats_pid" ] && kill "$nats_pid" 2>/dev/null || true
  for p in "${pids[@]:-}"; do kill "$p" 2>/dev/null || true; done
  pkill -f 'target/debug/vanguard-map'     2>/dev/null || true
  pkill -f 'target/debug/vanguard-control' 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# 1) NATS — reuse if already listening on 4222, else start (binary or Docker).
if lsof -iTCP:4222 -sTCP:LISTEN >/dev/null 2>&1; then
  echo "› NATS already running on 4222 — reusing"
elif command -v nats-server >/dev/null 2>&1; then
  echo "› starting NATS (nats-server)"
  nats-server -c nats.conf >/tmp/nats.log 2>&1 & nats_pid=$!
else
  echo "› starting NATS (Docker)"
  docker run --rm -p 4222:4222 -p 8080:8080 \
    -v "$PWD/nats.conf:/etc/nats/nats.conf:ro" nats:latest -c /etc/nats/nats.conf \
    >/tmp/nats.log 2>&1 & nats_pid=$!
fi
sleep 1

# 2) Backend — build once, then run map + control.
echo "› building backend…"
cargo build -q -p vanguard-map -p vanguard-control || { echo "✗ build failed"; exit 1; }
./target/debug/vanguard-map     >/tmp/map.log     2>&1 & pids+=($!)
./target/debug/vanguard-control >/tmp/control.log 2>&1 & pids+=($!)

# 3) Dashboard — Vite dev server with hot reload.
[ -d webui/node_modules ] || ( cd webui && pnpm install )
( cd webui && pnpm dev >/tmp/vite.log 2>&1 ) & pids+=($!)

cat <<'EOF'

  ✔ stack up
     dashboard : http://localhost:5173
     logs      : tail -f /tmp/nats.log /tmp/map.log /tmp/control.log /tmp/vite.log
     stop      : Ctrl-C

EOF
wait
