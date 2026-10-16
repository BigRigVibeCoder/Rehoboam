#!/usr/bin/env bash
# Points git at the repository's hooks in .githooks/, so that every push first runs
# bin/check_all.sh (SPR-001 T-05; RUN-001 §5). Undo: git config --unset core.hooksPath
set -eu
cd "$(dirname "$0")/.."
chmod +x .githooks/*
git config core.hooksPath .githooks
echo "Installed: core.hooksPath = .githooks (pre-push runs bin/check_all.sh)."
