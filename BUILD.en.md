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
