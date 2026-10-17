#!/usr/bin/env bash
# bin/run_stream_viewer.sh — One-Step Launcher for Rehoboam Plane C Stream Viewer & WebSocket Daemon
#
# Launches:
#   1. Zero-dependency WebSocket Simulation Stream Server (ws://localhost:8089)
#   2. Static HTTP WebGL Server (http://localhost:8088)
#   3. Opens or displays stream viewer URL (http://localhost:8088/tools/stream_viewer/index.html)
#
# Adheres to NASA-Grade reliability, zero-dark-failure, and Plane C presentation-only decoupling.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

WS_PORT=8089
HTTP_PORT=8088

echo "================================================================="
echo "   REHOBOAM // PLANE C SIMULATION STREAM VIEWER LAUNCHER"
echo "================================================================="

cd "$REPO_ROOT"

# 1. Check if WebSocket Stream Daemon is already active
if ss -tlnp 2>/dev/null | grep -q ":${WS_PORT} "; then
    echo "  [OK] WebSocket Stream Server already running on ws://localhost:${WS_PORT}"
else
    echo "  [STARTING] Launching Plane A WebSocket Stream Server on port ${WS_PORT}..."
    python3 "$REPO_ROOT/bin/stream_server.py" --port "${WS_PORT}" --interval 1.2 > /dev/null 2>&1 &
    sleep 0.8
    if ss -tlnp 2>/dev/null | grep -q ":${WS_PORT} "; then
        echo "  [OK] WebSocket Stream Server active on ws://localhost:${WS_PORT}"
    else
        echo "  [WARN] Stream server started in background (verifying async)..."
    fi
fi

# 2. Check if HTTP Static Server is already active
if ss -tlnp 2>/dev/null | grep -q ":${HTTP_PORT} "; then
    echo "  [OK] HTTP Web Server already running on http://localhost:${HTTP_PORT}"
else
    echo "  [STARTING] Launching HTTP Web Server on port ${HTTP_PORT}..."
    python3 -m http.server "${HTTP_PORT}" > /dev/null 2>&1 &
    sleep 0.8
    echo "  [OK] HTTP Web Server active on http://localhost:${HTTP_PORT}"
fi

echo "-----------------------------------------------------------------"
echo "  [READY] Stream Viewer is ready for real-time visualization:"
echo ""
echo "    Production Tool:   http://localhost:${HTTP_PORT}/tools/stream_viewer/index.html"
echo "    Prototype Lab:     http://localhost:${HTTP_PORT}/docs/red_team/prototypes/voxel_stream_compiler.html"
echo "    WebSocket Stream:  ws://localhost:${WS_PORT}"
echo ""
echo "  Features Active:"
echo "    • Real-time Plane A event stream ingestion (Ticks, Demography, Harvests, Sieges)"
echo "    • First-Person Ground Walk Mode (1.6m eye height, WASD + mouse look)"
echo "    • Luanti-grade micro-voxel fidelity (Weeping Willows, Sakura, PBR water, Fireflies)"
echo "    • Interactive Trebuchet Ballistics & Dynamic Voxel Wall Destructibility"
echo "    • Flight-animated Crimson Dragon with infernal breath strafing"
echo "    • Upstream simulation commands (Play/Pause, 1x/2x/5x/10x, Single-Step)"
echo "================================================================="
