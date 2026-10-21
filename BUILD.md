# Сборка

## Требования

- **Rust** (stable, edition 2021). Проверено на Rust 1.85+.
- **git**, **curl**, **tar** — для разворачивания `vendor/`.
- **Linux**: системные библиотеки для GPUI:
  - `libwayland-dev`, `libxkbcommon-dev`, `libxkbcommon-x11-dev`, `libx11-dev`, `libxcb1-dev`
  - `libfontconfig1-dev`, `libfreetype6-dev`
  - `libvulkan-dev`, `libegl1-mesa-dev`, `libgles2-mesa-dev`
  - `libasound2-dev` (для аудио, если используется)

Установка на Debian/Ubuntu (пример):

    sudo apt install build-essential git curl \
      libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-dev libxcb1-dev \
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
