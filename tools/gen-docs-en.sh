#!/usr/bin/env bash
# tools/gen-docs-en.sh — generate README.en.md and BUILD.en.md.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# ---------- README.en.md ----------
cat > README.en.md <<'EOF'
# gpui-new-gpui-kit-twrite-with-menu

A text editor built on [GPUI](https://github.com/zed-industries/zed) with menus, save/open dialogs, and an embedded `twrite` editor.

## What it is

A Rust application using:

- **GPUI** — GPU-accelerated UI framework by Zed;
- **gpui-kit** — components and themes (Longbridge);
- **gpui-component** — UI component set;
- **twrite** — text editor engine (ToonionOfficial).

All dependencies are pinned by `rev` in `vendor-upstream/manifest.toml` and unpacked locally into `vendor/` — no crates.io for git crates.

## Building

See [BUILD.en.md](BUILD.en.md).

Quick start:

    git clone https://github.com/EinstokFair/gpui-new-gpui-kit-twrite-with-menu.git
    cd gpui-new-gpui-kit-twrite-with-menu
    tools/vendor.sh
    cargo build --release

## Layout

    src/                      — application sources
      main.rs                 — entry point, window, menu
      about_window.rs         — About window
      save_dialog.rs          — Save dialog
      settings.rs             — settings
    tools/
      vendor.sh               — restore vendor/ (wraps cargo xtask expand/postprocess)
      vendor-close.sh         — extend manifest.toml (wraps cargo xtask close)
      xtask/                  — Rust helper: close, expand, remap, postprocess
      gen-docs.sh             — generate README/BUILD/LICENSE (Russian)
      gen-docs-en.sh          — generate README.en/BUILD.en (this script)
    vendor-upstream/
      manifest.toml           — recipe: where each crate comes from (git/generated)
      generate/               — generators (gpui-local)
      patches/                — patches for upstream crates
    vendor/                   — generated directory (not in git)

## How it works

1. `vendor-upstream/manifest.toml` — list of crates: name, source (`git`/`generated`), url, rev, subdir.
2. `tools/vendor.sh` runs `cargo xtask expand` — unpacks `vendor/` from the cache `~/.cache/gpui-editors-upstream` (or clones if missing), expands `workspace = true`, remaps `path = "crates/..."` to `../<x>-local`.
3. `cargo xtask postprocess` — normalizes `vendor/` manifests (renames, `features` merge, `[patch.crates-io]`).
4. `cargo build --release` — builds.

Details — in [BUILD.en.md](BUILD.en.md).

## License

GPLv3 — see [LICENSE](LICENSE).

## Third-party licenses

The project uses third-party crates (GPUI, gpui-kit, twrite, etc.) under their own licenses (Apache-2.0, MIT, etc.). See `Cargo.lock` and `vendor-upstream/manifest.toml`.
EOF

# ---------- BUILD.en.md ----------
cat > BUILD.en.md <<'EOF'
# Building

## Requirements

- **Rust** (stable, edition 2021). Tested on Rust 1.85+.
- **git**, **curl**, **tar** — to unpack `vendor/`.
- **Linux**: system libraries for GPUI:
  - `libwayland-dev`, `libxkbcommon-dev`, `libx11-dev`, `libxcb1-dev`
  - `libfontconfig1-dev`, `libfreetype6-dev`
  - `libvulkan-dev`, `libegl1-mesa-dev`, `libgles2-mesa-dev`
  - `libasound2-dev` (for audio, if used)

Install on Debian/Ubuntu (example):

    sudo apt install build-essential git curl \
      libwayland-dev libxkbcommon-dev libx11-dev libxcb1-dev \
      libfontconfig1-dev libfreetype6-dev \
      libvulkan-dev libegl1-mesa-dev libgles2-mesa-dev \
      libasound2-dev

## Quick start

    git clone https://github.com/EinstokFair/gpui-new-gpui-kit-twrite-with-menu.git
    cd gpui-new-gpui-kit-twrite-with-menu

    # 1. Restore vendor/ (fetch upstream crates into cache, unpack into vendor/)
    tools/vendor.sh

    # 2. Build
    cargo build --release

Binary: `target/release/gpui-new-gpui-kit-twrite-editor`.

## Vendoring modes

`tools/vendor.sh` (and `cargo xtask expand`) reads the `VENDOR_UPSTREAM_CACHE` environment variable:

| `VENDOR_UPSTREAM_CACHE` | `--update` | Behavior |
|---|---|---|
| unset | no | Online: clones upstream into `~/.cache/gpui-editors-upstream` if needed |
| unset | yes | Online + `git fetch origin` for all clones ("pull everything new") |
| set | no | **Offline**: cache only, no network |
| set | yes | `--update` overrides offline: goes online |

Examples:

    # Offline (cache only, no network)
    VENDOR_UPSTREAM_CACHE=~/.cache/gpui-editors-upstream tools/vendor.sh

    # Refresh cache to latest upstream commits (rev in manifest.toml unchanged)
    VENDOR_UPSTREAM_CACHE=~/.cache/gpui-editors-upstream tools/vendor.sh --update

**Note:** `--update` refreshes the **clone cache**, but `vendor/` is still built from `rev` in `vendor-upstream/manifest.toml`. Only a human changes `rev` — this is intentional, to keep builds reproducible.

## Updating dependencies

`rev` in `manifest.toml` is pinned. To update:

1. Manually change `rev` (and `url`/`subdir` if needed) in `vendor-upstream/manifest.toml`.
2. If upstream added new path crates — run `tools/vendor-close.sh` (extends the manifest).
3. Remove `vendor/` and rebuild: `rm -rf vendor && tools/vendor.sh`.
4. Verify `cargo build --release` passes.
5. `Cargo.lock` updates automatically — commit it.

## vendor/ layout

    vendor-upstream/manifest.toml   — recipe (in git)
    vendor/                         — generated, not in git
    ~/.cache/gpui-editors-upstream/ — upstream clone cache (outside repo)

`vendor/` is restored in ~30 seconds from cache (if present). Network is needed only for the first clone or with `--update`.

## Debugging

If `cargo build` fails on a TOML error in `vendor/`:

    rm -rf vendor
    tools/vendor.sh

If the error repeats — upstream likely changed its layout, and `xtask postprocess` needs an update. See `tools/xtask/src/main.rs`, functions `fix_*` / `merge_*`.

## License

GPLv3 — see [LICENSE](LICENSE).
EOF

echo "=== generated ==="
ls -la README.en.md BUILD.en.md

# ---------- .github/README.md (real file, ../ paths) ----------
mkdir -p .github
sed -e 's#](BUILD.en.md)#](../BUILD.en.md)#g' \
    -e 's#](LICENSE)#](../LICENSE)#g' \
    README.en.md > .github/README.md

# ---------- README.en.md (symlink to .github/README.md) ----------
rm -f README.en.md
ln -s .github/README.md README.en.md

echo "=== .github/README.md ==="
ls -la .github/README.md
readlink README.en.md || echo "(README.en.md — не симлинк)"
