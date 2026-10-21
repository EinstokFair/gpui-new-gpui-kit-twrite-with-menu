#!/usr/bin/env bash
# Генератор vendor/gpui-local — обёртка над gpui-pre (re-export + features wayland/x11).
set -euo pipefail
TARGET="${1:?usage: gpui-local.sh <target-dir>}"
mkdir -p "$TARGET/src"
cat > "$TARGET/Cargo.toml" <<'TOML'
[package]
name = "gpui"
version = "0.2.2"
edition = "2021"

[features]
default = []
wayland = ["gpui-pre/wayland"]
x11 = ["gpui-pre/x11"]

[dependencies]
gpui-pre = { path = "../gpui-pre-local" }
TOML
echo 'pub use gpui_pre::*;' > "$TARGET/src/lib.rs"
echo "OK: $TARGET"
