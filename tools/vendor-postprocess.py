#!/usr/bin/env python3
"""Идемпотентный пост-процессинг vendor/ после tools/vendor.sh.

Что делает:
  1. Убирает дубликаты [target.'cfg(...)'.dependencies] в манифестах vendor/.
  2. Переименовывает пакеты под имена, ожидаемые [patch.crates-io]:
       gpui-pre-local   : name = "gpui-pre"        (version = 0.3.9)
       gpui-pre-linux   : name = "gpui-pre-linux"  (version = 0.3.9)
  3. Заменяет path = "crates/..." на ../<name>-local или снимает path.
  4. Проставляет package = "gpui-pre" во всех gpui = { path = "../gpui-pre-local" }.
  5. ztracing_macro -> package = "ztracing-macro-gpui-unofficial", version = "1.25.0-pre".
  6. Добавляет в [patch.crates-io] записи gpui-kit-assets и gpui-component.
Можно запускать повторно: повторный запуск ничего не меняет.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
VENDOR = ROOT / "vendor"
CARGO = ROOT / "Cargo.toml"

def read(p): return p.read_text()
def write(p, s): p.write_text(s)

# ---------- 1. Дубликаты target-секций ----------
def dedup_target_sections(path: pathlib.Path):
    lines = read(path).splitlines(keepends=True)
    seen = set()
    out = []
    changed = False
    for ln in lines:
        s = ln.rstrip("\n")
        m = re.match(r"^\[target\.'cfg\([^)]*\)'\.(dependencies|build-dependencies|dev-dependencies)\]$", s)
        if m:
            if s in seen:
                changed = True
                continue
            seen.add(s)
        out.append(ln)
    if changed:
        write(path, "".join(out))
        print(f"  dedup target sections: {path.relative_to(ROOT)}")

# ---------- 2. Переименование пакетов ----------
RENAMES = {
    "gpui-pre-local":   ("gpui",       "gpui-pre",       "0.3.9"),
    "gpui-pre-linux":   ("gpui_linux", "gpui-pre-linux", "0.3.9"),
}
def rename_package(dirname, old_name, new_name, version):
    p = VENDOR / dirname / "Cargo.toml"
    if not p.exists():
        print(f"  SKIP rename (no dir): {dirname}")
        return
    s = read(p)
    s2 = re.sub(r'^name = ".*"$', f'name = "{new_name}"', s, count=1, flags=re.M)
    s2 = re.sub(r'^version = ".*"$', f'version = "{version}"', s2, count=1, flags=re.M)
    if s2 != s:
        write(p, s2)
        print(f"  rename {dirname}: {old_name} -> {new_name} @ {version}")

# ---------- 3. path = "crates/..." ----------
# Для путей без ../ — ищем соответствующий *-local в vendor/.
def fix_crates_paths():
    for manifest in VENDOR.glob("*/Cargo.toml"):
        if manifest.name == "Cargo.toml.orig":
            continue
        s = read(manifest)
        def repl(m):
            crate = m.group(1)  # напр. "base" или "refineable/derive_refineable"
            leaf = crate.rsplit("/", 1)[-1]
            # ищем каталог vendor/<leaf>-local
            cand = VENDOR / f"{leaf}-local"
            if cand.is_dir():
                return f'path = "../{leaf}-local"'
            # если есть version — снимаем path (fallback на [patch]/registry)
            return ""  # маркер: удалить
        new = re.sub(r'path = "crates/([^"]+)"', repl, s)
        # убираем пустые { , } после удаления path
        new = re.sub(r"\{ ,\s*", "{ ", new)
        new = re.sub(r",\s*,\s*", ", ", new)
        new = re.sub(r"\{ \}", "{}", new)
        if new != s:
            write(manifest, new)
            print(f"  fix crates/ path: {manifest.relative_to(ROOT)}")

# ---------- 4. package = "gpui-pre" для gpui = { path = "../gpui-pre-local" } ----------
def fix_gpui_pre_refs():
    for manifest in VENDOR.glob("*/Cargo.toml"):
        s = read(manifest)
        new = re.sub(
            r'^gpui = \{ path = "\.\./gpui-pre-local"',
            'gpui = { package = "gpui-pre", path = "../gpui-pre-local"',
            s, flags=re.M)
        if new != s:
            write(manifest, new)
            print(f"  package=gpui-pre: {manifest.relative_to(ROOT)}")

# ---------- 5. ztracing_macro ----------
def fix_ztracing():
    p = VENDOR / "ztracing-local" / "Cargo.toml"
    if not p.exists():
        print("  SKIP ztracing (no dir)")
        return
    s = read(p)
    new = re.sub(
        r'^ztracing_macro = \{.*\}$',
        'ztracing_macro = { package = "ztracing-macro-gpui-unofficial", version = "1.25.0-pre" }',
        s, count=1, flags=re.M)
    if new != s:
        write(p, new)
        print("  ztracing_macro -> ztracing-macro-gpui-unofficial")

# ---------- 6. [patch.crates-io] ----------
PATCHES = {
    "gpui-kit-assets":  "vendor/assets-local",
    "gpui-component":   "vendor/component-local",
}
def ensure_patches():
    s = read(CARGO)
    changed = False
    for name, path in PATCHES.items():
        line = f'{name} = {{ path = "{path}" }}'
        if re.search(rf"^{re.escape(name)} = \{{", s, flags=re.M):
            continue
        # вставляем в конец секции [patch.crates-io]
        m = re.search(r"^\[patch\.crates-io\]\n((?:.+\n)+?)(?=\n?\[|\Z)", s, flags=re.M)
        if not m:
            print(f"  WARN: [patch.crates-io] not found; add {name} manually")
            continue
        block = m.group(1)
        s = s[:m.end(1)] + line + "\n" + s[m.end(1):]
        changed = True
        print(f"  [patch] + {name}")
    if changed:
        write(CARGO, s)

def main():
    if not VENDOR.is_dir():
        sys.exit("vendor/ not found")
    print("== dedup target sections ==")
    for m in VENDOR.glob("*/Cargo.toml"):
        dedup_target_sections(m)
    print("== rename packages ==")
    for d, (o, n, v) in RENAMES.items():
        rename_package(d, o, n, v)
    print("== fix crates/ paths ==")
    fix_crates_paths()
    print("== package = gpui-pre ==")
    fix_gpui_pre_refs()
    print("== ztracing ==")
    fix_ztracing()
    print("== [patch.crates-io] ==")
    ensure_patches()
    print("DONE")

if __name__ == "__main__":
    main()
