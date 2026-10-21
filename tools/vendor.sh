#!/usr/bin/env bash
# tools/vendor.sh — обёртка над `cargo xtask expand` + `cargo xtask postprocess`.
# Восстанавливает vendor/ по vendor-upstream/manifest.toml.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
cargo xtask expand "$REPO_ROOT"
cargo xtask postprocess "$REPO_ROOT"
