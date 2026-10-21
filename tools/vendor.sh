#!/usr/bin/env bash
# tools/vendor.sh — восстановить vendor/ из vendor-upstream/manifest.toml.
# Работает на любой машине: git clone проекта → tools/vendor.sh → cargo build.
# Никаких crates.io — только git.
#
# Требования:
#   - git, curl, tar
#   - python3.11+ (tomllib) или pip install tomli
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
VU="$REPO_ROOT/vendor-upstream"
VENDOR="$REPO_ROOT/vendor"
MANIFEST="$VU/manifest.toml"
CACHE="${VENDOR_UPSTREAM_CACHE:-${HOME}/.cache/gpui-editors-upstream}"

if [[ ! -f "$MANIFEST" ]]; then
    echo "ОШИБКА: $MANIFEST не найден." >&2
    exit 1
fi

mkdir -p "$VENDOR" "$CACHE"

python3 - "$MANIFEST" "$VENDOR" "$VU" "$CACHE" "$REPO_ROOT" <<'PY'
import sys, pathlib, subprocess, os, shutil, tarfile, urllib.request, re

manifest_path = pathlib.Path(sys.argv[1])
vendor = pathlib.Path(sys.argv[2])
vu = pathlib.Path(sys.argv[3])
cache = pathlib.Path(sys.argv[4])
repo_root_py = pathlib.Path(sys.argv[5])

try:
    import tomllib as toml
except ImportError:
    try:
        import tomli as toml
    except ImportError:
        print("ОШИБКА: нужен python3.11+ (tomllib) или pip install tomli", file=sys.stderr)
        sys.exit(1)

data = toml.loads(manifest_path.read_text())
crates = data.get("crate", [])
if not crates:
    print("ОШИБКА: в manifest.toml нет [[crate]]", file=sys.stderr)
    sys.exit(1)


def run(cmd, cwd=None):
    print("  $", " ".join(cmd))
    subprocess.run(cmd, check=True, cwd=cwd)


def toml_value(v):
    """Рендер Python-значения в TOML-литерал (для inline-table и скаляров)."""
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        return str(v)
    if isinstance(v, str):
        return '"' + v.replace("\\", "\\\\").replace('"', '\\"') + '"'
    if isinstance(v, list):
        return "[" + ", ".join(toml_value(x) for x in v) + "]"
    if isinstance(v, dict):
        return "{ " + ", ".join(f"{k} = {toml_value(x)}" for k, x in v.items()) + " }"
    raise TypeError(f"unsupported TOML value: {v!r}")


def find_existing_repo(url, cache_dir):
    """Найти существующий клон по 4 шаблонам: <repo>, <owner>_<repo>, <repo>.git, <owner>_<repo>.git."""
    parts = url.rstrip("/").replace(".git", "").split("/")
    repo_name = parts[-1]
    owner_name = parts[-2] if len(parts) >= 2 else ""
    candidates = [
        repo_name,
        f"{owner_name}_{repo_name}",
        f"{repo_name}.git",
        f"{owner_name}_{repo_name}.git",
    ]
    for c in candidates:
        cand = cache_dir / c
        if cand.is_dir() and (cand / ".git").exists():
            print(f"  found existing repo: {cand}")
            return cand
    return None


def remap_path_dependencies(crate_dir: pathlib.Path, vendor_dir: pathlib.Path):
    """Заменить `path = "crates/<name>"` на `../<name>-local` для крейтов, уже присутствующих в vendor/."""
    crate_cargo = crate_dir / "Cargo.toml"
    if not crate_cargo.exists():
        return

    text = crate_cargo.read_text()
    changed = False

    # Явные алиасы: upstream-имя -> локальное имя в vendor/ (если не совпадает с <name>-local)
    aliases = {
        "gpui": "gpui-pre-local",
        "twrite-core": "twrite-core-local",
        "twrite-gpui": "twrite-gpui-local",
    }

    def repl(m):
        nonlocal changed
        prefix, name = m.group(1), m.group(2)
        if name.endswith("-local"):
            return m.group(0)
        # 1) явный алиас
        target = aliases.get(name)
        if target and (vendor_dir / target).is_dir() and (vendor_dir / target / "Cargo.toml").exists():
            changed = True
            return f'{prefix}path = "../{target}"'
        # 2) <name>-local и <name с _ -> ->-local (bench_metrics -> bench-metrics-local)
        candidates = [f"{name}-local", f"{name.replace('_', '-')}-local"]
        for cand_name in candidates:
            candidate = vendor_dir / cand_name
            if candidate.is_dir() and (candidate / "Cargo.toml").exists():
                changed = True
                return f'{prefix}path = "../{cand_name}"'
        return m.group(0)

    new_text = re.sub(r'(\bpath\s*=\s*)"crates/([\w-]+)"', repl, text)


    if changed:
        crate_cargo.write_text(new_text)
        print(f"  OK: remap path-зависимостей в {crate_cargo.relative_to(crate_dir.parent.parent)}")


def clone_git(url, rev, cache_dir):
    repo = find_existing_repo(url, cache_dir)
    if repo is None:
        name = url.rstrip("/").split("/")[-1].replace(".git", "")
        repo = cache_dir / name
        print(f"  git clone {url}")
        run(["git", "clone", "--quiet", url, str(repo)])
    subprocess.run(["git", "-C", str(repo), "fetch", "--depth", "1", "origin", rev],
                   check=False)
    run(["git", "-C", str(repo), "checkout", "--quiet", rev])
    return repo


def expand_workspace_inheritance(crate_dir: pathlib.Path, repo_root: pathlib.Path, crate_name: str):
    """Развернуть `*.workspace = true` в Cargo.toml в конкретные значения из корневого workspace."""
    root_cargo = repo_root / "Cargo.toml"
    crate_cargo = crate_dir / "Cargo.toml"

    if not root_cargo.exists() or not crate_cargo.exists():
        return

    root_data = toml.loads(root_cargo.read_text())
    ws_pkg = root_data.get("workspace", {}).get("package", {}) or {}
    ws_deps = root_data.get("workspace", {}).get("dependencies", {}) or {}
    ws_lints = root_data.get("workspace", {}).get("lints", {}) or {}

    text = crate_cargo.read_text()

    # 1. Простые поля: `edition.workspace = true` → `edition = "<значение>"`
    for field in ("edition", "version", "license", "repository", "rust-version", "authors", "publish"):
        pattern = rf'^{field}\.workspace\s*=\s*true\s*$'
        if re.search(pattern, text, re.MULTILINE):
            if field in ws_pkg:
                value_str = toml_value(ws_pkg[field])
                text = re.sub(pattern, f'{field} = {value_str}', text, count=1, flags=re.MULTILINE)

    # 2. `foo.workspace = true` внутри [dependencies], [dev-dependencies], [build-dependencies], [*-dependencies] секций:
    #    `foo = { workspace = true }` или `foo.workspace = true`
    lines = text.splitlines()
    in_deps_section = False
    out_lines = []
    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        if re.match(r'^\[.*dependencies.*\]', stripped):
            in_deps_section = True
        elif stripped.startswith("[") and not re.match(r'^\[.*dependencies.*\]', stripped):
            in_deps_section = False

        if in_deps_section:
            # G: склеить многострочную inline-таблицу `foo = { workspace = true, ... }`
            # Начало: `foo = { workspace = true` без закрывающей `}` до конца строки.
            m_open = re.match(r'^\s*[\w-]+\s*=\s*\{\s*workspace\s*=\s*true\b', line)
            if m_open and "}" not in line[m_open.end():]:
                buf = [line]
                j = i + 1
                found_close = False
                while j < len(lines):
                    buf.append(lines[j])
                    if "}" in lines[j]:
                        found_close = True
                        break
                    j += 1
                if found_close:
                    line = " ".join(s.strip() for s in buf)
                    i = j  # перепрыгнуть потреблённые строки (следующий шаг — i += 1 ниже)
                else:
                    # не нашли закрывающую } — оставляем как было
                    out_lines.append(line)
                    i += 1
                    continue

            # 2a. `foo = { workspace = true, ... }`
            m = re.match(r'^(\s*)([\w-]+)\s*=\s*\{\s*workspace\s*=\s*true\s*(.*?)\}\s*$', line)
            if m:
                indent, dep, rest = m.group(1), m.group(2), m.group(3)
                ws_dep = ws_deps.get(dep)
                if ws_dep is None:
                    out_lines.append(line)
                    i += 1
                    continue
                if isinstance(ws_dep, str):
                    base = {"version": ws_dep}
                    if rest.strip():
                        base.update(toml.loads("x = {" + rest.strip().strip(" ,") + "}")["x"])
                    if len(base) == 1 and "version" in base:
                        new_val = f'"{base["version"]}"'
                    else:
                        new_val = toml_value(base)
                else:
                    merged = dict(ws_dep)
                    if rest.strip():
                        extra = toml.loads("x = {" + rest.strip().strip(" ,") + "}")["x"]
                        ws_feats = list(merged.get("features", []))
                        for f in extra.get("features", []):
                            if f not in ws_feats:
                                ws_feats.append(f)
                        if ws_feats:
                            merged["features"] = ws_feats
                        for k, v in extra.items():
                            if k != "features":
                                merged[k] = v
                    new_val = toml_value(merged)
                out_lines.append(f'{indent}{dep} = {new_val}')
                i += 1
                continue
            # 2b. `foo.workspace = true`
            m = re.match(r'^(\s*)([\w-]+)\.workspace\s*=\s*true\s*$', line)
            if m:
                indent, dep = m.group(1), m.group(2)
                ws_dep = ws_deps.get(dep)
                if ws_dep is None:
                    out_lines.append(line)
                    i += 1
                    continue
                if isinstance(ws_dep, str):
                    new_val = f'"{ws_dep}"'
                else:
                    parts = []
                    if "version" in ws_dep:
                        parts.append(f'version = "{ws_dep["version"]}"')
                    if "package" in ws_dep:
                        parts.append(f'package = "{ws_dep["package"]}"')
                    if "features" in ws_dep:
                        feats = ", ".join(f'"{f}"' for f in ws_dep["features"])
                        parts.append(f'features = [{feats}]')
                    new_val = "{ " + ", ".join(parts) + " }"
                out_lines.append(f'{indent}{dep} = {new_val}')
                i += 1
                continue

        out_lines.append(line)
        i += 1

    text = "\n".join(out_lines)

    # 3. [lints] workspace = true → развернуть или удалить
    if re.search(r'^\[lints\]\s*\nworkspace\s*=\s*true\s*$', text, re.MULTILINE):
        if ws_lints:
            # Заменяем на [lints.rust] и [lints.clippy] из workspace
            lints_text = []
            for section, rules in ws_lints.items():
                lints_text.append(f"[lints.{section}]")
                for rule, level in rules.items():
                    lints_text.append(f'{rule} = {toml_value(level)}')
                lints_text.append("")
            text = re.sub(r'^\[lints\]\s*\nworkspace\s*=\s*true\s*$',
                          "\n".join(lints_text), text, flags=re.MULTILINE)
        else:
            text = re.sub(r'^\[lints\]\s*\nworkspace\s*=\s*true\s*\n?', '', text, flags=re.MULTILINE)

    crate_cargo.write_text(text)
    print(f"  OK: развёрнут workspace inheritance в {crate_cargo.relative_to(crate_dir.parent.parent)}")


fail = False
for c in crates:
    name = c["name"]
    src = c["source"]
    dst = vendor / name
    print(f"\n=== {name} ({src}) ===")

    if (dst / "Cargo.toml").exists():
        print(f"  OK: уже есть")
        continue

    if dst.exists():
        print(f"  ОШИБКА: {dst} существует, но Cargo.toml нет. Уберите вручную.", file=sys.stderr)
        fail = True
        continue

    if src == "git":
        repo = clone_git(c["url"], c["rev"], cache)
        subdir = repo / c["subdir"]
        if not (subdir / "Cargo.toml").exists():
            print(f"  ОШИБКА: {subdir}/Cargo.toml нет после checkout", file=sys.stderr)
            fail = True
            continue
        shutil.copytree(subdir, dst)
        print(f"  OK: git → {dst}")
        # Разворачиваем workspace inheritance (repo_root — корень клона)
        expand_workspace_inheritance(dst, repo, name)

    elif src == "generated":
        gen = vu / c["generator"]
        if not gen.exists():
            print(f"  ОШИБКА: генератор {gen} не найден", file=sys.stderr)
            fail = True
            continue
        run(["bash", str(gen), str(dst)])
        print(f"  OK: generated → {dst}")

    else:
        print(f"  ОШИБКА: неизвестный source={src} (crates-io запрещён)", file=sys.stderr)
        fail = True
        continue

    if "patch" in c:
        patch_file = vu / c["patch"]
        if not patch_file.exists():
            print(f"  ОШИБКА: патч {patch_file} не найден", file=sys.stderr)
            fail = True
            continue
        print(f"  git apply {patch_file}")
        rel_dst = dst.relative_to(repo_root_py)
        r = subprocess.run(
            ["git", "-C", str(repo_root_py), "apply",
             "--directory", str(rel_dst), "-p1", str(patch_file)],
            capture_output=True, text=True,
        )
        if r.returncode != 0:
            print(f"  ОШИБКА git apply:\n{r.stderr}", file=sys.stderr)
            fail = True

# K1: финальный remap path-зависимостей по всем vendor/*/Cargo.toml (после того, как все крейты на месте)
for d in sorted(vendor.iterdir()):
    if d.is_dir() and (d / "Cargo.toml").exists():
        try:
            remap_path_dependencies(d, vendor)
        except Exception as e:
            print(f"  ВНИМАНИЕ: remap {d.name}: {e}", file=sys.stderr)

sys.exit(1 if fail else 0)
PY

echo ""
echo "OK: vendor/ восстановлен."
