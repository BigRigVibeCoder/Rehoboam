#!/bin/sh
# Powers adjutant off after 30 minutes with no GitHub Actions job and no interactive login
# (RUN-002 §3, the backstop against a forgotten VM; SPR-009 T-04). Run every 5 minutes by
# rehoboam-idle-shutdown.timer, as root.
#
# Busy means: a runner job is running (a Runner.Worker process), or someone is logged in (`who`).
# Each busy check touches the marker; when the marker is older than IDLE_MINUTES, the VM powers off.
set -eu
IDLE_MINUTES="${IDLE_MINUTES:-30}"
MARKER="${REHOBOAM_IDLE_MARKER:-/run/rehoboam-last-busy}"
if pgrep -f Runner.Worker >/dev/null 2>&1 || [ -n "$(who)" ] || [ ! -e "$MARKER" ]; then
  touch "$MARKER"
  exit 0
fi
if [ -n "$(find "$MARKER" -mmin +"$IDLE_MINUTES")" ]; then
  logger -t rehoboam-idle "idle for ${IDLE_MINUTES} minutes: powering off"
  systemctl poweroff
fi
