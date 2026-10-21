#!/usr/bin/env bash
# tools/vendor-close.sh — дописать в vendor-upstream/manifest.toml все upstream-крейты,
# достижимые по path-зависимостям от уже известных. Идемпотентно.
#
# Роль: ПОДГОТОВКА manifest.toml. Не трогает vendor/. Не копирует. Только manifest.
# После него: tools/vendor.sh (развернуть) -> cargo build.
#
# Требования: python3.11+ (tomllib), /tmp/upstream/{zed,gpui-kit,twrite} (или VENDOR_UPSTREAM_CACHE).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
MANIFEST="$REPO_ROOT/vendor-upstream/manifest.toml"
CACHE="${VENDOR_UPSTREAM_CACHE:-${HOME}/.cache/gpui-editors-upstream}"

if [[ ! -f "$MANIFEST" ]]; then
    echo "ОШИБКА: $MANIFEST не найден" >&2
    exit 1
fi
if [[ ! -d "$CACHE" ]]; then
    echo "ОШИБКА: кэш $CACHE не найден (задайте VENDOR_UPSTREAM_CACHE)" >&2
    exit 1
fi

python3 - "$MANIFEST" "$CACHE" <<'PY'
import pathlib, re, sys

manifest_path = pathlib.Path(sys.argv[1])
cache = pathlib.Path(sys.argv[2])

try:
    import tomllib as toml
except ImportError:
    try:
        import tomli as toml
    except ImportError:
        print("ОШИБКА: нужен python3.11+ (tomllib) или pip install tomli", file=sys.stderr)
        sys.exit(1)

# Клоны upstream в кэше: ищем по 4 шаблонам
def find_clone(url):
    parts = url.rstrip("/").replace(".git", "").split("/")
    repo_name = parts[-1]
    owner_name = parts[-2] if len(parts) >= 2 else ""
    for c in [repo_name, f"{owner_name}_{repo_name}", f"{repo_name}.git", f"{owner_name}_{repo_name}.git"]:
        cand = cache / c
        if cand.is_dir() and (cand / ".git").exists():
            return cand
    return None

# Локальное имя -> upstream-имя (алиасы, где они не совпадают с <name>-local)
ALIASES = {
    "gpui-pre-local": "gpui",
    "gpui-pre-linux": "gpui_linux",
    "gpui-base-local": "base",
    "gpui-kit-local": "kit",
    "gpui-wgpu-local": "gpui_wgpu",
    "gpui-platform-local": "gpui_platform",
    "gpui-macros-local": "gpui_macros",
    "gpui-local": None,  # generated, не upstream
}
def to_upstream(local_name):
    if local_name in ALIASES:
        return ALIASES[local_name]
    if local_name.endswith("-local"):
        return local_name[:-len("-local")]
    return local_name

manifest_text = manifest_path.read_text()
data = toml.loads(manifest_text)
known_local = {c["name"] for c in data.get("crate", [])}
known_upstream = {u for u in (to_upstream(n) for n in known_local) if u is not None}

# locate upstream-крейта в клонах: возвращает (url, rev, subdir, cargo_path)
def locate(name, cache):
    # ищем по кэшу: пробуем все клоны
    for clone_dir in cache.iterdir():
        if not (clone_dir.is_dir() and (clone_dir / ".git").exists()):
            continue
        # url и rev берём из .git/config и HEAD
        # проще: subdir = crates/<name> или crates/<...>/<name> — но мы не знаем url/rev
        pass
    return None

# Вместо перебора кэша — используем известные url/rev из manifest и их клоны.
# Для каждого upstream-крейта знаем url/rev/subdir из manifest. Для НОВЫХ — определяем по кэшу.
# Определяем клоны по url из manifest.
clones = {}  # url -> (path, rev)
for c in data.get("crate", []):
    if c.get("source") != "git":
        continue
    url = c.get("url")
    rev = c.get("rev")
    if url and url not in clones:
        p = find_clone(url)
        if p:
            clones[url] = (p, rev)

def locate_new(name, clones):
    """Найти url/rev/subdir для upstream-имени name во всех клонах."""
    for url, (clone_path, rev) in clones.items():
        for subdir in [f"crates/{name}"]:
            cargo = clone_path / subdir / "Cargo.toml"
            if cargo.exists():
                return (url, rev, subdir)
        # вложенные: crates/<...>/<name>/Cargo.toml — ищем вглубь 2 уровня
        for sub in (clone_path / "crates").glob("*"):
            if sub.is_dir():
                cargo = sub / name / "Cargo.toml"
                if cargo.exists():
                    return (url, rev, f"crates/{sub.name}/{name}")
    return None

def path_crates_of(cargo_path):
    if not cargo_path.exists():
        return []
    text = cargo_path.read_text()
    # собираем path = "crates/<...>" (включая вложенные)
    return re.findall(r'\bpath\s*=\s*"crates/([\w/-]+)"', text)

added = []
iteration = 0
while True:
    iteration += 1
    new_entries = []
    # для каждого upstream-имени из known_upstream — открыть его cargo из clones
    for name in sorted(known_upstream):
        loc = locate_new(name, clones)
        if loc is None:
            continue
        url, rev, subdir = loc
        clone_path = clones[url][0]
        cargo = clone_path / subdir / "Cargo.toml"
        for raw in path_crates_of(cargo):
            dep_name = raw.rsplit("/", 1)[-1]
            if dep_name in known_upstream:
                continue
            dep_loc = locate_new(dep_name, clones)
            if dep_loc is None:
                print(f"ВНИМАНИЕ: '{dep_name}' (нужен '{name}') не найден ни в одном клоне", file=sys.stderr)
                continue
            dep_url, dep_rev, dep_subdir = dep_loc
            local_name = f"{dep_name.replace('_', '-')}-local"
            new_entries.append((local_name, dep_url, dep_rev, dep_subdir, dep_name))
            known_upstream.add(dep_name)
    if not new_entries:
        print(f"Итерация {iteration}: новых нет — замыкание закрыто")
        break
    print(f"Итерация {iteration}: +{len(new_entries)} новых")
    block = ""
    for local_name, url, rev, subdir, name in new_entries:
        block += f'\n[[crate]]\nname = "{local_name}"\nsource = "git"\nurl = "{url}"\nrev = "{rev}"\nsubdir = "{subdir}"\n'
        print(f"  + {local_name}  <- {url}@{rev[:8]} {subdir}")
        added.append(name)
    manifest_text = manifest_text.rstrip() + "\n" + block
    manifest_path.write_text(manifest_text)

print()
print(f"ИТОГО добавлено: {len(added)}")
for n in sorted(added):
    print(f"  + {n}")
PY
