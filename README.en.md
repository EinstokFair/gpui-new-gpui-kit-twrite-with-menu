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
