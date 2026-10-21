#!/usr/bin/env bash
# tools/vendor.sh — восстановить vendor/ из vendor-upstream/manifest.toml.
# Работает на любой машине: git clone проекта → tools/vendor.sh → cargo build.
#
# Требования:
#   - git
#   - curl
#   - python3 с tomllib (Python 3.11+)  ИЛИ  tomli (pip install tomli)
#   - tar
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
VU="$REPO_ROOT/vendor-upstream"
VENDOR="$REPO_ROOT/vendor"
MANIFEST="$VU/manifest.toml"
CACHE="${HOME}/.cache/gpui-editors-upstream"

if [[ ! -f "$MANIFEST" ]]; then
    echo "ОШИБКА: $MANIFEST не найден." >&2
    exit 1
fi

mkdir -p "$VENDOR" "$CACHE"

# Парсер TOML: пробуем tomllib (3.11+), иначе tomli
python3 - "$MANIFEST" "$VENDOR" "$VU" "$CACHE" <<'PY'
import sys, pathlib, subprocess, os, shutil, tarfile, urllib.request

manifest_path = pathlib.Path(sys.argv[1])
vendor = pathlib.Path(sys.argv[2])
vu = pathlib.Path(sys.argv[3])
cache = pathlib.Path(sys.argv[4])

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

def download_crate(crate_name, version, dest_dir):
    file_name = f"{crate_name}-{version}.crate"
    local = cache / file_name
    if not local.exists():
        url = f"https://static.crates.io/crates/{crate_name}/{file_name}"
        print(f"  curl {url}")
        urllib.request.urlretrieve(url, local)
    dest_dir.mkdir(parents=True, exist_ok=True)
    with tarfile.open(local, "r:gz") as tf:
        tf.extractall(dest_dir, filter="data")
    return dest_dir / f"{crate_name}-{version}"

def clone_git(url, rev, cache_dir):
    name = url.rstrip("/").split("/")[-1].replace(".git", "")
    repo = cache_dir / name
    if not repo.exists():
        print(f"  git clone {url}")
        run(["git", "clone", "--quiet", url, str(repo)])
    print(f"  git fetch --depth 1 origin {rev}")
    subprocess.run(["git", "-C", str(repo), "fetch", "--depth", "1", "origin", rev],
                   check=False)
    run(["git", "-C", str(repo), "checkout", "--quiet", rev])
    return repo

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

    if src == "crates-io":
        extracted = download_crate(c["crate"], c["version"], cache)
        shutil.copytree(extracted, dst)
        print(f"  OK: crates-io → {dst}")

    elif src == "git":
        repo = clone_git(c["url"], c["rev"], cache)
        subdir = repo / c["subdir"]
        if not (subdir / "Cargo.toml").exists():
            print(f"  ОШИБКА: {subdir}/Cargo.toml нет после checkout", file=sys.stderr)
            fail = True
            continue
        shutil.copytree(subdir, dst)
        print(f"  OK: git → {dst}")

    elif src == "generated":
        gen = vu / c["generator"]
        if not gen.exists():
            print(f"  ОШИБКА: генератор {gen} не найден", file=sys.stderr)
            fail = True
            continue
        run(["bash", str(gen), str(dst)])
        print(f"  OK: generated → {dst}")

    else:
        print(f"  ОШИБКА: неизвестный source={src}", file=sys.stderr)
        fail = True
        continue

    # Применяем патч, если есть
    if "patch" in c:
        patch_file = vu / c["patch"]
        if not patch_file.exists():
            print(f"  ОШИБКА: патч {patch_file} не найден", file=sys.stderr)
            fail = True
            continue
        print(f"  git apply {patch_file}")
        # Патч должен применяться с -p1 из директории dst
        r = subprocess.run(["git", "apply", "--directory", str(dst), "-p1", str(patch_file)],
                           capture_output=True, text=True)
        if r.returncode != 0:
            print(f"  ОШИБКА git apply:\n{r.stderr}", file=sys.stderr)
            fail = True

sys.exit(1 if fail else 0)
PY

echo ""
echo "OK: vendor/ восстановлен."
