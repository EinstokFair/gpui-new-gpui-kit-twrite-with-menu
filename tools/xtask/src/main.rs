use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug)]
struct Mode {
    offline: bool,
    update: bool,
}

impl Mode {
    fn from_env_and_args(args: &[String]) -> Mode {
        let offline = std::env::var("VENDOR_UPSTREAM_CACHE").is_ok();
        let update = args.iter().any(|a| a == "--update");
        Mode { offline, update }
    }
}

fn main() -> Result<()> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let cmd = argv
        .iter()
        .find(|a| !a.starts_with("--"))
        .context("usage: cargo xtask <close|expand|remap|postprocess> [--update] [root]")?
        .clone();

    let root = argv
        .iter()
        .skip_while(|a| a.as_str() != cmd.as_str())
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let mode = Mode::from_env_and_args(&argv);

    match cmd.as_str() {
        "close" => cmd_close(&root, mode),
        "expand" => cmd_expand(&root, mode),
        "remap" => cmd_remap(&root),
        "postprocess" => cmd_postprocess(&root),
        other => bail!("неизвестная подкоманда: {other}"),
    }
}

fn resolve_cache() -> PathBuf {
    std::env::var("VENDOR_UPSTREAM_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").expect("HOME not set");
            PathBuf::from(home).join(".cache/gpui-editors-upstream")
        })
}

fn update_all_clones(crates: &[toml_edit::Table], cache: &Path) -> Result<()> {
    let mut urls: std::collections::HashSet<String> = std::collections::HashSet::new();
    for c in crates {
        if c.get("source").and_then(|v| v.as_str()) == Some("git") {
            if let Some(url) = c.get("url").and_then(|v| v.as_str()) {
                urls.insert(url.to_string());
            }
        }
    }
    let mut sorted: Vec<String> = urls.into_iter().collect();
    sorted.sort();
    for url in &sorted {
        match find_clone(cache, url) {
            Some(repo) => {
                println!("  git fetch origin (--update): {}", repo.display());
                let st = Command::new("git")
                    .args(["-C"])
                    .arg(&repo)
                    .args(["fetch", "origin"])
                    .status()
                    .with_context(|| format!("git fetch origin для {}", repo.display()))?;
                if !st.success() {
                    eprintln!(
                        "  ВНИМАНИЕ: git fetch origin вернул {} для {}",
                        st,
                        repo.display()
                    );
                }
            }
            None => {
                println!("  skip fetch (нет клона): {}", url);
            }
        }
    }
    Ok(())
}

// ============ close ============

fn cmd_close(root: &Path, mode: Mode) -> Result<()> {
    let manifest = root.join("vendor-upstream/manifest.toml");
    let cache = resolve_cache();
    if !manifest.exists() {
        bail!("manifest not found: {}", manifest.display());
    }
    if !cache.is_dir() {
        bail!(
            "cache not found: {} (set VENDOR_UPSTREAM_CACHE)",
            cache.display()
        );
    }

    let text = std::fs::read_to_string(&manifest)?;
    let mut doc: toml_edit::DocumentMut = text.parse()?;

    let crates_snapshot: Vec<toml_edit::Table> = doc
        .get("crate")
        .and_then(|i| i.as_array_of_tables())
        .map(|arr| arr.iter().cloned().collect())
        .unwrap_or_default();

    if mode.update {
        println!("== --update: обновляю все клоны в кэше ==");
        update_all_clones(&crates_snapshot, &cache)?;
    }

    let mut known_local: std::collections::HashSet<String> = std::collections::HashSet::new();
    for t in &crates_snapshot {
        if let Some(name) = t.get("name").and_then(|v| v.as_str()) {
            known_local.insert(name.to_string());
        }
    }

    let to_upstream = |local: &str| -> Option<String> {
        match local {
            "gpui-pre-local" => Some("gpui".into()),
            "gpui-pre-linux" => Some("gpui_linux".into()),
            "gpui-base-local" => Some("base".into()),
            "gpui-kit-local" => Some("kit".into()),
            "gpui-wgpu-local" => Some("gpui_wgpu".into()),
            "gpui-platform-local" => Some("gpui_platform".into()),
            "gpui-macros-local" => Some("gpui_macros".into()),
            "gpui-local" => None,
            other if other.ends_with("-local") => Some(other[..other.len() - 6].into()),
            other => Some(other.into()),
        }
    };

    let mut known_upstream: std::collections::HashSet<String> =
        known_local.iter().filter_map(|n| to_upstream(n)).collect();

    let mut clones: std::collections::HashMap<String, (PathBuf, String)> = Default::default();
    for t in &crates_snapshot {
        if t.get("source").and_then(|v| v.as_str()) != Some("git") {
            continue;
        }
        let (Some(url), Some(rev)) = (
            t.get("url").and_then(|v| v.as_str()),
            t.get("rev").and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        if clones.contains_key(url) {
            continue;
        }
        if let Some(p) = find_clone(&cache, url) {
            clones.insert(url.to_string(), (p, rev.to_string()));
        }
    }

    let locate_new = |name: &str| -> Option<(String, String, String)> {
        for (url, (clone_path, rev)) in &clones {
            let direct = clone_path.join(format!("crates/{name}"));
            if direct.join("Cargo.toml").exists() {
                return Some((url.clone(), rev.clone(), format!("crates/{name}")));
            }
            if let Ok(entries) = std::fs::read_dir(clone_path.join("crates")) {
                for e in entries.flatten() {
                    let sub = e.path();
                    if sub.is_dir() && sub.join(name).join("Cargo.toml").exists() {
                        let sub_name = sub.file_name()?.to_string_lossy().to_string();
                        return Some((
                            url.clone(),
                            rev.clone(),
                            format!("crates/{sub_name}/{name}"),
                        ));
                    }
                }
            }
        }
        None
    };

    let mut added: Vec<String> = Vec::new();
    let mut iteration = 0;
    loop {
        iteration += 1;
        let mut new_entries: Vec<(String, String, String, String, String)> = Vec::new();
        let mut snapshot: Vec<String> = known_upstream.iter().cloned().collect();
        snapshot.sort();
        for name in &snapshot {
            let Some((url, _rev, subdir)) = locate_new(name) else {
                continue;
            };
            let clone_path = &clones[&url].0;
            let cargo = clone_path.join(&subdir).join("Cargo.toml");
            for raw in path_crates_of(&cargo)? {
                let dep_name = raw.rsplit('/').next().unwrap_or(&raw).to_string();
                if known_upstream.contains(&dep_name) {
                    continue;
                }
                let Some((dep_url, dep_rev, dep_subdir)) = locate_new(&dep_name) else {
                    eprintln!("ВНИМАНИЕ: '{dep_name}' (нужен '{name}') не найден ни в одном клоне");
                    continue;
                };
                let local_name = format!("{}-local", dep_name.replace('_', "-"));
                new_entries.push((
                    local_name,
                    dep_url,
                    dep_rev,
                    dep_subdir,
                    dep_name.clone(),
                ));
                known_upstream.insert(dep_name);
            }
        }
        if new_entries.is_empty() {
            println!("Итерация {iteration}: новых нет — замыкание закрыто");
            break;
        }
        println!("Итерация {iteration}: +{} новых", new_entries.len());
        for (local_name, url, rev, subdir, name) in &new_entries {
            let mut t = toml_edit::Table::new();
            t["name"] = toml_edit::value(local_name.as_str());
            t["source"] = toml_edit::value("git");
            t["url"] = toml_edit::value(url.as_str());
            t["rev"] = toml_edit::value(rev.as_str());
            t["subdir"] = toml_edit::value(subdir.as_str());
            let arr = doc
                .entry("crate")
                .or_insert(toml_edit::Item::ArrayOfTables(
                    toml_edit::ArrayOfTables::new(),
                ))
                .as_array_of_tables_mut()
                .context("crate not array-of-tables")?;
            arr.push(t);
            println!(
                "  + {local_name}  <- {url}@{} {subdir}",
                &rev[..8.min(rev.len())]
            );
            added.push(name.clone());
        }
    }
    std::fs::write(&manifest, doc.to_string())?;
    println!("\nИТОГО добавлено: {}", added.len());
    for n in &added {
        println!("  + {n}");
    }
    Ok(())
}

fn find_clone(cache: &Path, url: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = url
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .split('/')
        .collect();
    let repo = parts.last()?;
    let owner = if parts.len() >= 2 {
        parts[parts.len() - 2]
    } else {
        ""
    };
    for c in [
        repo.to_string(),
        format!("{owner}_{repo}"),
        format!("{repo}.git"),
        format!("{owner}_{repo}.git"),
    ] {
        let cand = cache.join(&c);
        if cand.is_dir() && cand.join(".git").exists() {
            return Some(cand);
        }
    }
    None
}

fn path_crates_of(cargo: &Path) -> Result<Vec<String>> {
    if !cargo.exists() {
        return Ok(vec![]);
    }
    let text = std::fs::read_to_string(cargo)?;
    let re = regex::Regex::new(r#"path\s*=\s*"crates/([\w/-]+)""#)?;
    Ok(re.captures_iter(&text).map(|c| c[1].to_string()).collect())
}

// ============ expand ============

fn cmd_expand(root: &Path, mode: Mode) -> Result<()> {
    let vu = root.join("vendor-upstream");
    let vendor = root.join("vendor");
    let manifest_path = vu.join("manifest.toml");
    let cache = resolve_cache();

    if !manifest_path.exists() {
        bail!("ОШИБКА: {} не найден.", manifest_path.display());
    }
    std::fs::create_dir_all(&vendor)?;
    std::fs::create_dir_all(&cache)?;

    let text = std::fs::read_to_string(&manifest_path)?;
    let doc: toml_edit::DocumentMut = text.parse()?;
    let crates: Vec<toml_edit::Table> = doc
        .get("crate")
        .and_then(|i| i.as_array_of_tables())
        .map(|arr| arr.iter().cloned().collect())
        .context("в manifest.toml нет [[crate]]")?;

    if mode.update {
        println!("== --update: обновляю все клоны в кэше ==");
        update_all_clones(&crates, &cache)?;
    }

    let mut fail = false;

    for c in &crates {
        let name = c.get("name").and_then(|v| v.as_str()).context("crate.name")?;
        let src = c
            .get("source")
            .and_then(|v| v.as_str())
            .context("crate.source")?;
        let dst = vendor.join(name);
        println!("\n=== {name} ({src}) ===");

        if dst.join("Cargo.toml").exists() {
            println!("  OK: уже есть");
            continue;
        }
        if dst.exists() {
            eprintln!(
                "  ОШИБКА: {} существует, но Cargo.toml нет. Уберите вручную.",
                dst.display()
            );
            fail = true;
            continue;
        }

        match src {
            "git" => {
                let url = c.get("url").and_then(|v| v.as_str()).context("git.url")?;
                let rev = c.get("rev").and_then(|v| v.as_str()).context("git.rev")?;
                let subdir = c
                    .get("subdir")
                    .and_then(|v| v.as_str())
                    .context("git.subdir")?;
                let repo = match clone_git(url, rev, &cache, mode) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("  ОШИБКА clone_git: {e:#}");
                        fail = true;
                        continue;
                    }
                };
                let sub = repo.join(subdir);
                if !sub.join("Cargo.toml").exists() {
                    eprintln!("  ОШИБКА: {}/Cargo.toml нет после checkout", sub.display());
                    fail = true;
                    continue;
                }
                if let Err(e) = copy_dir_all(&sub, &dst) {
                    eprintln!("  ОШИБКА copy_dir_all: {e:#}");
                    fail = true;
                    continue;
                }
                println!("  OK: git → {}", dst.display());

                let root_cargo = repo.join("Cargo.toml");
                if root_cargo.exists() {
                    if let Err(e) = expand_workspace_inheritance(&dst, &root_cargo) {
                        eprintln!("  ВНИМАНИЕ: expand_workspace_inheritance: {e:#}");
                    }
                    if let Err(e) = expand_workspace_sections(&dst, &root_cargo) {
                        eprintln!("  ВНИМАНИЕ: expand_workspace_sections: {e:#}");
                    }
                }
            }
            "generated" => {
                let gen_rel = c
                    .get("generator")
                    .and_then(|v| v.as_str())
                    .context("generated.generator")?;
                let gen = vu.join(gen_rel);
                if !gen.exists() {
                    eprintln!("  ОШИБКА: генератор {} не найден", gen.display());
                    fail = true;
                    continue;
                }
                let status = Command::new("bash")
                    .arg(&gen)
                    .arg(&dst)
                    .status()
                    .with_context(|| format!("запуск {}", gen.display()))?;
                if !status.success() {
                    eprintln!("  ОШИБКА: генератор вернул {}", status);
                    fail = true;
                    continue;
                }
                println!("  OK: generated → {}", dst.display());
            }
            other => {
                eprintln!("  ОШИБКА: неизвестный source={other} (crates-io запрещён)");
                fail = true;
                continue;
            }
        }

        if let Some(patch_rel) = c.get("patch").and_then(|v| v.as_str()) {
            let patch_file = vu.join(patch_rel);
            if !patch_file.exists() {
                eprintln!("  ОШИБКА: патч {} не найден", patch_file.display());
                fail = true;
                continue;
            }
            println!("  git apply {}", patch_file.display());
            let rel_dst = match dst.strip_prefix(root) {
                Ok(p) => p.to_path_buf(),
                Err(_) => {
                    eprintln!("  ОШИБКА: {} не внутри {}", dst.display(), root.display());
                    fail = true;
                    continue;
                }
            };
            let out = Command::new("git")
                .arg("-C")
                .arg(root)
                .arg("apply")
                .arg("--directory")
                .arg(&rel_dst)
                .arg("-p1")
                .arg(&patch_file)
                .output()
                .context("git apply")?;
            if !out.status.success() {
                eprintln!(
                    "  ОШИБКА git apply:\n{}",
                    String::from_utf8_lossy(&out.stderr)
                );
                fail = true;
            }
        }
    }

    let remap_res = cmd_remap(root);
    if let Err(e) = remap_res {
        eprintln!("  ВНИМАНИЕ: remap: {e:#}");
    }

    if fail {
        bail!("expand завершился с ошибками");
    }
    println!("\nOK: vendor/ восстановлен.");
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else if ty.is_symlink() {
            std::fs::copy(&from, &to)
                .with_context(|| format!("copy symlink {}", from.display()))?;
        } else {
            std::fs::copy(&from, &to)
                .with_context(|| format!("copy {} -> {}", from.display(), to.display()))?;
        }
    }
    Ok(())
}

fn has_rev(repo: &Path, rev: &str) -> bool {
    Command::new("git")
        .args(["-C"])
        .arg(repo)
        .args(["cat-file", "-e", rev])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn clone_git(url: &str, rev: &str, cache: &Path, mode: Mode) -> Result<PathBuf> {
    let repo = match find_clone(cache, url) {
        Some(p) => p,
        None => {
            if mode.offline && !mode.update {
                bail!(
                    "офлайн-режим (VENDOR_UPSTREAM_CACHE задана): клона {} нет в кэше {} (задайте --update, чтобы сходить в сеть)",
                    url,
                    cache.display()
                );
            }
            let name = url
                .trim_end_matches('/')
                .split('/')
                .last()
                .unwrap_or("repo")
                .trim_end_matches(".git")
                .to_string();
            let p = cache.join(name);
            println!("  git clone {url}");
            let st = Command::new("git")
                .args(["clone", "--quiet", url])
                .arg(&p)
                .status()
                .context("git clone")?;
            if !st.success() {
                bail!("git clone failed: {url}");
            }
            p
        }
    };

    match (mode.offline, mode.update) {
        (true, false) => {
            if !has_rev(&repo, rev) {
                bail!(
                    "офлайн-режим: коммита {} нет в кэше {} (задайте --update, чтобы сходить в сеть)",
                    rev,
                    repo.display()
                );
            }
        }
        (_, true) => {
            if !has_rev(&repo, rev) {
                println!("  git fetch origin (--update): {}", repo.display());
                let _ = Command::new("git")
                    .args(["-C"])
                    .arg(&repo)
                    .args(["fetch", "origin"])
                    .status();
                if !has_rev(&repo, rev) {
                    bail!(
                        "после --update коммита {} всё ещё нет в {}",
                        rev,
                        repo.display()
                    );
                }
            }
        }
        (false, false) => {
            if !has_rev(&repo, rev) {
                let _ = Command::new("git")
                    .args(["-C"])
                    .arg(&repo)
                    .args(["fetch", "--depth", "1", "origin", rev])
                    .status();
            }
            if !has_rev(&repo, rev) {
                bail!("не удалось получить коммит {} из {}", rev, url);
            }
        }
    }

    let st = Command::new("git")
        .args(["-C"])
        .arg(&repo)
        .args(["checkout", "--quiet", rev])
        .status()
        .context("git checkout")?;
    if !st.success() {
        bail!("git checkout failed: {} @ {}", url, rev);
    }
    Ok(repo)
}

// ============ workspace inheritance ============

fn read_ws_tables(
    root_cargo: &Path,
) -> Result<(
    toml_edit::Table,
    toml_edit::Table,
    toml_edit::Table,
)> {
    let text = std::fs::read_to_string(root_cargo)?;
    let doc: toml_edit::DocumentMut = text.parse()?;
    let ws = doc
        .get("workspace")
        .and_then(|i| i.as_table())
        .cloned()
        .unwrap_or_default();
    let pkg = ws
        .get("package")
        .and_then(|i| i.as_table())
        .cloned()
        .unwrap_or_default();
    let deps = ws
        .get("dependencies")
        .and_then(|i| i.as_table())
        .cloned()
        .unwrap_or_default();
    let lints = ws
        .get("lints")
        .and_then(|i| i.as_table())
        .cloned()
        .unwrap_or_default();
    Ok((pkg, deps, lints))
}

fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn render_value(v: &toml_edit::Value) -> String {
    match v {
        toml_edit::Value::String(s) => format!("\"{}\"", escape_toml_string(s.value())),
        toml_edit::Value::InlineTable(it) => {
            if it.is_empty() {
                "{}".to_string()
            } else {
                let mut parts = Vec::new();
                for (k, val) in it.iter() {
                    parts.push(format!("{k} = {}", render_value(val)));
                }
                format!("{{ {} }}", parts.join(", "))
            }
        }
        toml_edit::Value::Array(a) => {
            let mut parts = Vec::new();
            for val in a.iter() {
                parts.push(render_value(val));
            }
            format!("[{}]", parts.join(", "))
        }
        _ => v.to_string().trim().to_string(),
    }
}

fn render_item(v: &toml_edit::Item) -> Option<String> {
    match v {
        toml_edit::Item::Value(val) => Some(render_value(val)),
        toml_edit::Item::Table(t) => {
            if t.is_empty() {
                None
            } else {
                let mut parts = Vec::new();
                for (k, val) in t.iter() {
                    if let Some(rendered) = render_item(val) {
                        parts.push(format!("{k} = {rendered}"));
                    }
                }
                if parts.is_empty() {
                    None
                } else {
                    Some(format!("{{ {} }}", parts.join(", ")))
                }
            }
        }
        toml_edit::Item::ArrayOfTables(_) => None,
        toml_edit::Item::None => None,
    }
}

fn render_ws_dep(ws_dep: &toml_edit::Item) -> Option<String> {
    if let toml_edit::Item::Value(toml_edit::Value::String(s)) = ws_dep {
        return Some(format!(
            "{{ version = \"{}\" }}",
            escape_toml_string(s.value())
        ));
    }
    render_item(ws_dep)
}

fn parse_extra(s: &str) -> Option<toml_edit::InlineTable> {
    let synthetic = format!("x = {{ {} }}", s.trim().trim_matches(',').trim());
    let doc: toml_edit::DocumentMut = synthetic.parse().ok()?;
    doc.get("x").and_then(|i| i.as_inline_table()).cloned()
}

fn merge_ws_and_extra(
    ws_dep: &toml_edit::Item,
    extra: Option<&toml_edit::InlineTable>,
) -> Option<String> {
    if let toml_edit::Item::Value(toml_edit::Value::String(s)) = ws_dep {
        match extra {
            None => {
                return Some(format!(
                    "{{ version = \"{}\" }}",
                    escape_toml_string(s.value())
                ))
            }
            Some(extra_it) if extra_it.is_empty() => {
                return Some(format!(
                    "{{ version = \"{}\" }}",
                    escape_toml_string(s.value())
                ))
            }
            Some(extra_it) => {
                let mut parts: Vec<String> = vec![format!(
                    "version = \"{}\"",
                    escape_toml_string(s.value())
                )];
                let mut seen: std::collections::HashSet<String> =
                    std::collections::HashSet::from(["version".to_string()]);
                for (k, v) in extra_it.iter() {
                    if !seen.insert(k.to_string()) {
                        continue;
                    }
                    parts.push(format!("{k} = {}", render_value(v)));
                }
                return Some(format!("{{ {} }}", parts.join(", ")));
            }
        }
    }

    let mut base: toml_edit::InlineTable = match ws_dep {
        toml_edit::Item::Value(toml_edit::Value::InlineTable(it)) => it.clone(),
        toml_edit::Item::Table(t) => {
            let mut it = toml_edit::InlineTable::new();
            for (k, v) in t.iter() {
                if let Some(val) = v.as_value() {
                    it.insert(k, val.clone());
                }
            }
            it
        }
        _ => toml_edit::InlineTable::new(),
    };

    if let Some(extra_it) = extra {
        for (k, v) in extra_it.iter() {
            if k == "features" {
                let mut merged: Vec<String> = Vec::new();
                if let Some(existing) = base.get("features").and_then(|x| x.as_array()) {
                    for f in existing.iter() {
                        if let Some(s) = f.as_str() {
                            merged.push(s.to_string());
                        }
                    }
                }
                if let Some(add) = v.as_array() {
                    for f in add.iter() {
                        if let Some(s) = f.as_str() {
                            if !merged.iter().any(|x| x == s) {
                                merged.push(s.to_string());
                            }
                        }
                    }
                }
                let mut arr = toml_edit::Array::new();
                for f in merged {
                    arr.push(f);
                }
                base.insert("features", toml_edit::Value::Array(arr));
            } else {
                base.insert(k, v.clone());
            }
        }
    }

    if base.is_empty() {
        None
    } else {
        let mut parts = Vec::new();
        for (k, v) in base.iter() {
            parts.push(format!("{k} = {}", render_value(v)));
        }
        Some(format!("{{ {} }}", parts.join(", ")))
    }
}

fn expand_workspace_inheritance(crate_dir: &Path, root_cargo: &Path) -> Result<()> {
    let crate_cargo = crate_dir.join("Cargo.toml");
    if !crate_cargo.exists() || !root_cargo.exists() {
        return Ok(());
    }
    let (ws_pkg, ws_deps, ws_lints) = read_ws_tables(root_cargo)?;

    let mut text = std::fs::read_to_string(&crate_cargo)?;

    for field in [
        "edition",
        "version",
        "license",
        "repository",
        "rust-version",
        "authors",
        "publish",
    ] {
        let re = regex::Regex::new(&format!(
            r"(?m)^{}\.workspace\s*=\s*true\s*$",
            regex::escape(field)
        ))?;
        if re.is_match(&text) {
            if let Some(v) = ws_pkg.get(field) {
                if let Some(val) = render_item(v) {
                    text = re
                        .replace(&text, format!("{field} = {val}"))
                        .to_string();
                }
            }
        }
    }

    let lines: Vec<&str> = text.lines().collect();
    let mut in_deps = false;
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;

    let section_re = regex::Regex::new(r"^\[.*dependencies.*\]")?;
    let open_inline_re = regex::Regex::new(r"^\s*[\w-]+\s*=\s*\{\s*workspace\s*=\s*true\b")?;
    let inline_re =
        regex::Regex::new(r"^(\s*)([\w-]+)\s*=\s*\{\s*workspace\s*=\s*true\s*(.*?)\}\s*$")?;
    let dotted_re = regex::Regex::new(r"^(\s*)([\w-]+)\.workspace\s*=\s*true\s*$")?;

    while i < lines.len() {
        let line = lines[i];
        let stripped = line.trim();
        if section_re.is_match(stripped) {
            in_deps = true;
        } else if stripped.starts_with('[') && !section_re.is_match(stripped) {
            in_deps = false;
        }

        if in_deps {
            let mut joined: Option<String> = None;
            if open_inline_re.is_match(line)
                && !line[open_inline_re.find(line).unwrap().end()..].contains('}')
            {
                let mut buf = vec![line.to_string()];
                let mut j = i + 1;
                let mut found = false;
                while j < lines.len() {
                    buf.push(lines[j].to_string());
                    if lines[j].contains('}') {
                        found = true;
                        break;
                    }
                    j += 1;
                }
                if found {
                    joined = Some(buf.iter().map(|s| s.trim()).collect::<Vec<_>>().join(" "));
                    i = j;
                }
            }
            let effective_line = joined.clone().unwrap_or_else(|| line.to_string());

            if let Some(cap) = inline_re.captures(&effective_line) {
                let indent = &cap[1];
                let dep = &cap[2];
                let rest = &cap[3];
                if let Some(ws_dep) = ws_deps.get(dep) {
                    let extra = if rest.trim().is_empty() {
                        None
                    } else {
                        parse_extra(rest)
                    };
                    if let Some(rendered) = merge_ws_and_extra(ws_dep, extra.as_ref()) {
                        out.push(format!("{indent}{dep} = {rendered}"));
                    } else {
                        out.push(effective_line);
                    }
                    i += 1;
                    continue;
                }
                out.push(effective_line);
                i += 1;
                continue;
            }

            if let Some(cap) = dotted_re.captures(&effective_line) {
                let indent = &cap[1];
                let dep = &cap[2];
                if let Some(ws_dep) = ws_deps.get(dep) {
                    if let Some(rendered) = render_ws_dep(ws_dep) {
                        out.push(format!("{indent}{dep} = {rendered}"));
                        i += 1;
                        continue;
                    }
                }
            }
        }

        out.push(line.to_string());
        i += 1;
    }

    text = out.join("\n");

    let lints_re = regex::Regex::new(r"(?m)^\[lints\]\s*\nworkspace\s*=\s*true\s*$")?;
    if lints_re.is_match(&text) {
        if !ws_lints.is_empty() {
            let mut lints_text: Vec<String> = Vec::new();
            for (section, rules) in ws_lints.iter() {
                let rules_t = match rules.as_table() {
                    Some(t) => t,
                    None => continue,
                };
                lints_text.push(format!("[lints.{section}]"));
                for (rule, level) in rules_t.iter() {
                    if let Some(rendered) = render_item(level) {
                        lints_text.push(format!("{rule} = {rendered}"));
                    }
                }
                lints_text.push(String::new());
            }
            text = lints_re.replace(&text, lints_text.join("\n")).to_string();
        } else {
            let empty_re = regex::Regex::new(r"(?m)^\[lints\]\s*\nworkspace\s*=\s*true\s*\n?")?;
            text = empty_re.replace(&text, "").to_string();
        }
    }

    std::fs::write(&crate_cargo, text)?;
    println!(
        "  OK: развёрнут workspace inheritance в {}",
        crate_cargo.display()
    );
    Ok(())
}

fn expand_workspace_sections(crate_dir: &Path, root_cargo: &Path) -> Result<()> {
    let crate_cargo = crate_dir.join("Cargo.toml");
    if !crate_cargo.exists() || !root_cargo.exists() {
        return Ok(());
    }
    let (_ws_pkg, ws_deps, _ws_lints) = read_ws_tables(root_cargo)?;

    let text = std::fs::read_to_string(&crate_cargo)?;
    let lines: Vec<String> = text.lines().map(String::from).collect();

    let section_re = regex::Regex::new(
        r"^\s*\[\s*([^\]]*?\.)?(dependencies|dev-dependencies|build-dependencies)\.([\w-]+)\s*\]\s*$",
    )?;

    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    let mut changed = false;

    while i < lines.len() {
        let line = lines[i].clone();
        if let Some(cap) = section_re.captures(&line) {
            let prefix = cap
                .get(1)
                .map(|m| m.as_str().trim_end_matches('.'))
                .unwrap_or("");
            let kind = &cap[2];
            let name = &cap[3];

            let mut body: Vec<String> = Vec::new();
            let mut j = i + 1;
            while j < lines.len() {
                let s = lines[j].trim();
                if s.starts_with('[') && s.ends_with(']') {
                    break;
                }
                body.push(lines[j].clone());
                j += 1;
            }

            let has_ws = body.iter().any(|l| l.trim() == "workspace = true");
            if has_ws {
                if let Some(ws_dep) = ws_deps.get(name) {
                    let mut extras: Vec<String> = Vec::new();
                    for l in &body {
                        let s = l.trim();
                        if s == "workspace = true" || s.is_empty() {
                            continue;
                        }
                        if s.contains('=') {
                            extras.push(s.to_string());
                        }
                    }
                    let extra_inline = if extras.is_empty() {
                        None
                    } else {
                        parse_extra(&extras.join(", "))
                    };
                    if let Some(rendered) = merge_ws_and_extra(ws_dep, extra_inline.as_ref()) {
                        let header_kind = if prefix.is_empty() {
                            format!("[{kind}]")
                        } else {
                            format!("[{prefix}.{kind}]")
                        };
                        out.push(header_kind);
                        out.push(format!("{name} = {rendered}"));
                        i = j;
                        changed = true;
                        continue;
                    }
                }
            }
            out.push(line);
            i += 1;
            continue;
        }
        out.push(line);
        i += 1;
    }

    if changed {
        let joined = out.join("\n") + "\n";
        std::fs::write(&crate_cargo, joined)?;
        println!(
            "  OK: развёрнуты секции workspace = true в {}",
            crate_cargo.display()
        );
    }
    Ok(())
}

// ============ remap ============

fn cmd_remap(root: &Path) -> Result<()> {
    let vendor = root.join("vendor");
    if !vendor.is_dir() {
        bail!("vendor/ not found: {}", vendor.display());
    }
    for entry in std::fs::read_dir(&vendor)? {
        let entry = entry?;
        let manifest = entry.path().join("Cargo.toml");
        if manifest.exists() {
            remap_manifest(&manifest, &vendor)?;
        }
    }
    println!("remap: done");
    Ok(())
}

fn remap_manifest(manifest: &Path, vendor: &Path) -> Result<()> {
    let text = std::fs::read_to_string(manifest)?;
    let re = regex::Regex::new(r#"(\bpath\s*=\s*)"crates/([\w/-]+)""#)?;
    let mut changed = false;
    let mut out = String::new();
    for line in text.lines() {
        let mut done = false;
        if let Some(cap) = re.captures(line) {
            let prefix = &cap[1];
            let raw = &cap[2];
            let leaf = raw.rsplit('/').next().unwrap_or(raw);
            if !leaf.ends_with("-local") {
                let alias: Option<&str> = match leaf {
                    "gpui_linux" => Some("gpui-pre-linux"),
                    "gpui_macros" => Some("gpui-macros-local"),
                    "gpui-base" => Some("gpui-base-local"),
                    "base" => Some("gpui-base-local"),
                    "twrite-core" => Some("twrite-core-local"),
                    "twrite-gpui" => Some("twrite-gpui-local"),
                    _ => None,
                };
                let cand_name = if let Some(a) = alias {
                    a.to_string()
                } else {
                    format!("{leaf}-local")
                };
                let cand = vendor.join(&cand_name);
                if cand.is_dir() {
                    let new_line = format!("{prefix}\"../{cand_name}\"");
                    let replaced = re.replace(line, new_line.as_str()).to_string();
                    out.push_str(&replaced);
                    out.push('\n');
                    changed = true;
                    done = true;
                }
            }
        }
        if !done {
            out.push_str(line);
            out.push('\n');
        }
    }
    if changed {
        std::fs::write(manifest, out)?;
        println!("  remap: {}", manifest.display());
    }
    Ok(())
}

// ============ postprocess ============

fn cmd_postprocess(root: &Path) -> Result<()> {
    let vendor = root.join("vendor");
    if !vendor.is_dir() {
        bail!("vendor/ not found: {}", vendor.display());
    }
    for entry in std::fs::read_dir(&vendor)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let manifest = path.join("Cargo.toml");
            if manifest.exists() {
                dedup_target_sections(&manifest)?;
            }
        }
    }
    rename_package(&vendor.join("gpui-pre-local/Cargo.toml"), "gpui-pre", "0.3.9")?;
    rename_package(
        &vendor.join("gpui-pre-linux/Cargo.toml"),
        "gpui-pre-linux",
        "0.3.9",
    )?;
    for entry in std::fs::read_dir(&vendor)? {
        let entry = entry?;
        let manifest = entry.path().join("Cargo.toml");
        if manifest.exists() {
            fix_crates_paths(&manifest, &vendor)?;
        }
    }
    for entry in std::fs::read_dir(&vendor)? {
        let entry = entry?;
        let manifest = entry.path().join("Cargo.toml");
        if manifest.exists() {
            fix_gpui_pre_refs(&manifest)?;
        }
    }
    fix_ztracing(&vendor.join("ztracing-local/Cargo.toml"))?;
    ensure_patches(&root.join("Cargo.toml"))?;
    println!("postprocess: done");
    Ok(())
}

fn dedup_target_sections(manifest: &Path) -> Result<()> {
    let text = std::fs::read_to_string(manifest)?;
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out = String::new();
    let mut changed = false;
    let re = regex::Regex::new(
        r"^\[target\.'cfg\([^)]*\)'\.(dependencies|build-dependencies|dev-dependencies)\]$",
    )?;
    for line in text.lines() {
        if re.is_match(line) && !seen.insert(line.to_string()) {
            changed = true;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    if changed {
        std::fs::write(manifest, out)?;
        println!("  dedup: {}", manifest.display());
    }
    Ok(())
}

fn rename_package(manifest: &Path, new_name: &str, new_version: &str) -> Result<()> {
    if !manifest.exists() {
        println!("  skip rename (no file): {}", manifest.display());
        return Ok(());
    }
    let mut doc = std::fs::read_to_string(manifest)?.parse::<toml_edit::DocumentMut>()?;
    let pkg = doc.get_mut("package").context("no [package]")?;
    let tbl = pkg.as_table_mut().context("[package] not a table")?;
    tbl["name"] = toml_edit::value(new_name);
    tbl["version"] = toml_edit::value(new_version);
    std::fs::write(manifest, doc.to_string())?;
    println!(
        "  rename: {} -> {} @ {}",
        manifest.display(),
        new_name,
        new_version
    );
    Ok(())
}

fn fix_crates_paths(manifest: &Path, vendor: &Path) -> Result<()> {
    let text = std::fs::read_to_string(manifest)?;
    let re = regex::Regex::new(r#"path = "crates/([^"]+)""#)?;
    let mut changed = false;
    let mut out = String::new();
    for line in text.lines() {
        if let Some(cap) = re.captures(line) {
            let raw = &cap[1];
            let leaf = raw.rsplit('/').next().unwrap_or(raw);
            // Пробуем оба варианта: <leaf>-local и <leaf.replace('_','-')>-local
            let candidates = [
                format!("{leaf}-local"),
                format!("{}-local", leaf.replace('_', "-")),
            ];
            let mut found: Option<String> = None;
            for cand_name in &candidates {
                if vendor.join(cand_name).is_dir() {
                    found = Some(cand_name.clone());
                    break;
                }
            }
            let replacement = match found {
                Some(cand_name) => format!(r#"path = "../{cand_name}""#),
                None => String::new(), // снимаем path, если каталога нет
            };
            let new_line = re.replace(line, replacement.as_str()).to_string();
            if new_line != line {
                changed = true;
                out.push_str(&new_line);
                out.push('\n');
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if changed {
        std::fs::write(manifest, out)?;
        println!("  crates/ path: {}", manifest.display());
    }
    Ok(())
}

fn fix_gpui_pre_refs(manifest: &Path) -> Result<()> {
    let text = std::fs::read_to_string(manifest)?;
    let re = regex::Regex::new(r#"^gpui = \{ path = "\.\./gpui-pre-local""#)?;
    let mut changed = false;
    let mut out = String::new();
    for line in text.lines() {
        if re.is_match(line) {
            let new_line = line.replacen(
                r#"gpui = { path = "../gpui-pre-local""#,
                r#"gpui = { package = "gpui-pre", path = "../gpui-pre-local""#,
                1,
            );
            out.push_str(&new_line);
            out.push('\n');
            changed = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if changed {
        std::fs::write(manifest, out)?;
        println!("  package=gpui-pre: {}", manifest.display());
    }
    Ok(())
}

fn fix_ztracing(manifest: &Path) -> Result<()> {
    if !manifest.exists() {
        println!("  skip ztracing (no file)");
        return Ok(());
    }
    let mut doc = std::fs::read_to_string(manifest)?.parse::<toml_edit::DocumentMut>()?;
    let deps = doc.get_mut("dependencies").context("no [dependencies]")?;
    let tbl = deps.as_table_mut().context("[dependencies] not a table")?;
    let mut it = toml_edit::InlineTable::new();
    it.insert("package", "ztracing-macro-gpui-unofficial".into());
    it.insert("version", "1.25.0-pre".into());
    tbl["ztracing_macro"] = toml_edit::value(it);
    std::fs::write(manifest, doc.to_string())?;
    println!("  ztracing_macro -> ztracing-macro-gpui-unofficial");
    Ok(())
}

fn ensure_patches(cargo: &Path) -> Result<()> {
    let mut doc = std::fs::read_to_string(cargo)?.parse::<toml_edit::DocumentMut>()?;
    let patches = doc
        .entry("patch")
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
        .as_table_mut()
        .context("[patch] not a table")?;
    let crates_io = patches
        .entry("crates-io")
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
        .as_table_mut()
        .context("[patch.crates-io] not a table")?;
    for (name, path) in [
        ("gpui-kit-assets", "vendor/assets-local"),
        ("gpui-component", "vendor/component-local"),
    ] {
        if !crates_io.contains_key(name) {
            let mut t = toml_edit::InlineTable::new();
            t.insert("path", path.into());
            crates_io[name] = toml_edit::Item::Value(toml_edit::Value::InlineTable(t));
            println!("  [patch] + {name}");
        }
    }
    std::fs::write(cargo, doc.to_string())?;
    Ok(())
}