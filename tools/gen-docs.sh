#!/usr/bin/env bash
# tools/gen-docs.sh — сгенерировать README.md, BUILD.md, LICENSE (GPLv3).
# Идемпотентно: перезаписывает файлы. Не трогает код.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# ---------- README.md ----------
cat > README.md <<'EOF'
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

## Запуск

    ./target/release/gpui-new-gpui-kit-twrite-editor [--encoding <метка>] [<файл>]

Без аргументов — открывается пустой редактор с приветственным текстом.

- `<файл>` — путь к файлу, который нужно открыть при старте. Если не указан, редактор стартует пустым.
- `--encoding <метка>` — кодировка, в которой читать файл: `utf-8` (по умолчанию), `windows-1251`, `ibm866` и другие метки, понимаемые `encoding_rs`. На сохранение не влияет: файл всегда пишется в UTF-8.
- `-h`, `--help` — показать справку и выйти.

Примеры:

    # открыть файл (UTF-8 по умолчанию)
    ./target/release/gpui-new-gpui-kit-twrite-editor /tmp/notes.txt

    # открыть файл в кодировке windows-1251
    ./target/release/gpui-new-gpui-kit-twrite-editor --encoding windows-1251 /tmp/notes-cp1251.txt

    # справка
    ./target/release/gpui-new-gpui-kit-twrite-editor --help

При ошибке чтения файла редактор не падает: сообщение уходит в stderr, окно открывается с пустым буфером.

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
EOF

# ---------- BUILD.md ----------
cat > BUILD.md <<'EOF'
# Сборка

## Требования

- **Rust** (stable, edition 2021). Проверено на Rust 1.85+.
- **git**, **curl**, **tar** — для разворачивания `vendor/`.
- **Linux**: системные библиотеки для GPUI:
  - `libwayland-dev`, `libxkbcommon-dev`, `libx11-dev`, `libxcb1-dev`
  - `libfontconfig1-dev`, `libfreetype6-dev`
  - `libvulkan-dev`, `libegl1-mesa-dev`, `libgles2-mesa-dev`
  - `libasound2-dev` (для аудио, если используется)

Установка на Debian/Ubuntu (пример):

    sudo apt install build-essential git curl \
      libwayland-dev libxkbcommon-dev libx11-dev libxcb1-dev \
      libfontconfig1-dev libfreetype6-dev \
      libvulkan-dev libegl1-mesa-dev libgles2-mesa-dev \
      libasound2-dev

## Быстрый старт

    git clone https://github.com/EinstokFair/gpui-new-gpui-kit-twrite-with-menu.git
    cd gpui-new-gpui-kit-twrite-with-menu

    # 1. Восстановить vendor/ (скачает upstream-крейты в кэш, развернёт в vendor/)
    tools/vendor.sh

    # 2. Собрать
    cargo build --release

Готовый бинарь: `target/release/gpui-new-gpui-kit-twrite-editor`.

## Режимы вендоринга

`tools/vendor.sh` (и `cargo xtask expand`) понимает переменную окружения `VENDOR_UPSTREAM_CACHE`:

| `VENDOR_UPSTREAM_CACHE` | `--update` | Поведение |
|---|---|---|
| не задана | нет | Онлайн: клонирует upstream в `~/.cache/gpui-editors-upstream` при необходимости |
| не задана | да | Онлайн + `git fetch origin` для всех клонов («тащить всё новое») |
| задана | нет | **Офлайн**: только кэш, сеть не трогается |
| задана | да | `--update` перебивает офлайн: идёт в сеть |

Примеры:

    # Офлайн (только кэш, без сети)
    VENDOR_UPSTREAM_CACHE=~/.cache/gpui-editors-upstream tools/vendor.sh

    # Обновить кэш до последних коммитов upstream (rev в manifest.toml не меняются)
    VENDOR_UPSTREAM_CACHE=~/.cache/gpui-editors-upstream tools/vendor.sh --update

**Важно:** `--update` обновляет **кэш клонов**, но `vendor/` всё равно собирается по `rev` из `vendor-upstream/manifest.toml`. `rev` меняет **только человек** — это осознанное решение, чтобы сборка была воспроизводимой.

## Обновление зависимостей

`rev` в `manifest.toml` фиксированы. Если нужно обновить:

1. Вручную поменять `rev` (и `url`/`subdir`, если изменились) в `vendor-upstream/manifest.toml`.
2. Если upstream добавил новые path-крейты — запустить `tools/vendor-close.sh` (дополнит манифест).
3. Удалить `vendor/` и пересобрать: `rm -rf vendor && tools/vendor.sh`.
4. Проверить, что `cargo build --release` проходит.
5. `Cargo.lock` обновится автоматически — закоммитить.

## Структура vendor/

    vendor-upstream/manifest.toml   — рецепт (в git)
    vendor/                         — генерируется, в git не входит
    ~/.cache/gpui-editors-upstream/ — кэш клонов upstream (вне репозитория)

`vendor/` восстанавливается за ~30 секунд из кэша (если кэш есть). Сеть нужна только для первого клона или при `--update`.

## Отладка

Если `cargo build` падает на TOML-ошибке в `vendor/`:

    rm -rf vendor
    tools/vendor.sh

Если ошибка повторяется — вероятно, upstream изменил структуру, и `xtask postprocess` нужно дополнить. См. `tools/xtask/src/main.rs`, функции `fix_*` / `merge_*`.

## Лицензия

GPLv3 — см. [LICENSE](LICENSE).
EOF

# ---------- LICENSE (GPLv3) ----------
echo "=== скачиваю GPLv3 ==="
curl -fsSL https://www.gnu.org/licenses/gpl-3.0.txt -o LICENSE
echo "LICENSE: $(wc -l < LICENSE) строк"

echo ""
echo "=== готово ==="
ls -la README.md BUILD.md LICENSE
