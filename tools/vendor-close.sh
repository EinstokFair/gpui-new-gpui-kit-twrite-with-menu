#!/usr/bin/env bash
# tools/vendor-close.sh — обёртка над `cargo xtask close`.
# Дополняет vendor-upstream/manifest.toml недостающими upstream-крейтами (замыкание).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
exec cargo xtask close "$REPO_ROOT"
