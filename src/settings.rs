use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeKind {
    Light,
    Dark,
}

impl ThemeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeKind::Light => "light",
            ThemeKind::Dark => "dark",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "light" => Some(ThemeKind::Light),
            "dark" => Some(ThemeKind::Dark),
            _ => None,
        }
    }
}

fn config_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg));
        }
    }
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".config"))
}

fn settings_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("gpui-new-gpui-kit-twrite-editor").join("settings.txt"))
}

pub fn load_theme() -> Option<ThemeKind> {
    let path = settings_path()?;
    let text = fs::read_to_string(path).ok()?;
    ThemeKind::parse(&text)
}

pub fn save_theme(kind: ThemeKind) {
    let Some(path) = settings_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, kind.as_str());
}