use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const DEFAULT_SEARCH_ROOT: &str = "$HOME/Documents";
pub(crate) const DEFAULT_COMMAND: &str = "nvim .";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) search_root: String,
    pub(crate) default_command: String,
    #[serde(default)]
    pub(crate) include_root: bool,
    #[serde(default)]
    pub(crate) include_hidden: bool,
    #[serde(default = "default_cd_after_run")]
    pub(crate) cd_after_run: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            search_root: DEFAULT_SEARCH_ROOT.to_string(),
            default_command: DEFAULT_COMMAND.to_string(),
            include_root: false,
            include_hidden: false,
            cd_after_run: default_cd_after_run(),
        }
    }
}

fn default_cd_after_run() -> bool {
    true
}

pub(crate) fn config_path() -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME environment variable is not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("runin")
        .join("config.toml"))
}

pub(crate) fn config_exists(path: &Path) -> bool {
    path.exists()
}

pub(crate) fn load_config(path: &Path) -> Result<Config, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Failed reading config {}: {e}", path.display()))?;
    toml::from_str(&raw).map_err(|e| format!("Failed parsing config {}: {e}", path.display()))
}

pub(crate) fn write_config(path: &Path, cfg: &Config) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed creating config directory {}: {e}", parent.display()))?;
    }

    let content =
        toml::to_string_pretty(cfg).map_err(|e| format!("Failed serializing config: {e}"))?;
    fs::write(path, content).map_err(|e| format!("Failed writing config {}: {e}", path.display()))
}

pub(crate) fn expand_home(path: &str) -> String {
    if let Some(home) = env::var_os("HOME") {
        expand_home_with(path, &home.to_string_lossy())
    } else {
        path.to_string()
    }
}

pub(crate) fn expand_home_with(path: &str, home: &str) -> String {
    if let Some(rest) = path.strip_prefix("$HOME") {
        format!("{home}{rest}")
    } else if let Some(rest) = path.strip_prefix("${HOME}") {
        format!("{home}{rest}")
    } else if path == "~" {
        home.to_string()
    } else if let Some(rest) = path.strip_prefix("~/") {
        format!("{home}/{rest}")
    } else {
        path.to_string()
    }
}
