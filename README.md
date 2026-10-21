# gpui-new-gpui-kit-twrite-with-menu

Текстовый редактор на [GPUI](https://github.com/zed-industries/zed) с меню, диалогами сохранения/открытия и встроенным `twrite`-редактором.

## Что это

Приложение на Rust, использующее:

- **GPUI** — GPU-ускоренный UI-фреймворк от Zed;
- **gpui-kit** — компоненты и темы (Longbridge);
- **gpui-component** — набор UI-компонентов;
- **twrite** — движок текстового редактора (ToonionOfficial).

Все зависимости зафиксированы по `rev` в `vendor-upstream/manifest.toml` и разворачиваются локально в `vendor/` — без crates.io для git-крейтов.

## Сборка

См. [BUILD.md](BUILD.md).

Кратко:

    git clone https://github.com/EinstokFair/gpui-new-gpui-kit-twrite-with-menu.git
    cd gpui-new-gpui-kit-twrite-with-menu
    tools/vendor.sh
    cargo build --release

## Структура

    src/                      — исходники приложения
      main.rs                 — точка входа, окно, меню
      about_window.rs         — окно «О программе»
      save_dialog.rs          — диалог сохранения
      settings.rs             — настройки
    tools/
      vendor.sh               — восстановить vendor/ (обёртка над cargo xtask expand/postprocess)
      vendor-close.sh         — дополнить manifest.toml (обёртка над cargo xtask close)
      xtask/                  — Rust-утилита: close, expand, remap, postprocess
      gen-docs.sh             — этот скрипт (генерация README/BUILD/LICENSE)
    vendor-upstream/
      manifest.toml           — рецепт: откуда брать каждый крейт (git/generated)
      generate/               — генераторы (gpui-local)
      patches/                — патчи к upstream-крейтам
    vendor/                   — генерируемый каталог (в git не входит)

## Как это работает

1. `vendor-upstream/manifest.toml` — список крейтов: имя, источник (`git`/`generated`), url, rev, subdir.
2. `tools/vendor.sh` вызывает `cargo xtask expand` — разворачивает `vendor/` из кэша `~/.cache/gpui-editors-upstream` (или клонирует, если кэша нет), разворачивает `workspace = true`, ремапит `path = "crates/..."` на `../<x>-local`.
3. `cargo xtask postprocess` — доводит манифесты `vendor/` до собираемого вида (переименования, слияние `features`, `[patch.crates-io]`).
4. `cargo build --release` — собирает.

Подробности — в [BUILD.md](BUILD.md).

## Лицензия

GPLv3 — см. [LICENSE](LICENSE).

## Зависимости и их лицензии

Проект использует сторонние крейты (GPUI, gpui-kit, twrite и др.) под их собственными лицензиями (Apache-2.0, MIT и др.). См. `Cargo.lock` и `vendor-upstream/manifest.toml`.
