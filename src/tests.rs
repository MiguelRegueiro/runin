use crate::config::{
    ensure_and_load_config, expand_home_with, write_config, Config, DEFAULT_COMMAND,
    DEFAULT_SEARCH_ROOT,
};
use crate::{absolute_root_path, parse_selection, resolve_config_toggle, resolve_include_hidden};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new() -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("runin-test-{ts}-{seq}"));
        fs::create_dir_all(&path).expect("failed to create temp test dir");
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn ensure_and_load_config_creates_default_when_missing() {
    let dir = TestDir::new();
    let config_path = dir.path.join("config.toml");

    let cfg = ensure_and_load_config(&config_path).expect("should create and load config");

    assert_eq!(cfg.search_root, DEFAULT_SEARCH_ROOT);
    assert_eq!(cfg.default_command, DEFAULT_COMMAND);
    assert!(!cfg.include_root);
    assert!(config_path.exists());
}

#[test]
fn write_and_load_config_roundtrip() {
    let dir = TestDir::new();
    let config_path = dir.path.join("config.toml");
    let expected = Config {
        search_root: "/home/regueiro".to_string(),
        default_command: "qwen".to_string(),
        include_root: true,
        include_hidden: true,
    };

    write_config(&config_path, &expected).expect("write config should succeed");
    let loaded = ensure_and_load_config(&config_path).expect("load config should succeed");

    assert_eq!(loaded, expected);
}

#[test]
fn ensure_and_load_config_returns_error_for_invalid_toml() {
    let dir = TestDir::new();
    let config_path = dir.path.join("config.toml");
    fs::write(&config_path, "not-valid-toml = [").expect("failed to write invalid test config");

    let err = ensure_and_load_config(&config_path).expect_err("invalid TOML should fail");
    assert!(err.contains("Failed parsing config"));
}

#[test]
fn ensure_and_load_config_defaults_toggles_when_missing() {
    let dir = TestDir::new();
    let config_path = dir.path.join("config.toml");
    fs::write(
        &config_path,
        "search_root = \"/home/regueiro\"\ndefault_command = \"qwen\"\n",
    )
    .expect("failed to write config without include_root");

    let cfg = ensure_and_load_config(&config_path).expect("load config should succeed");
    assert!(!cfg.include_root);
    assert!(!cfg.include_hidden);
}

#[test]
fn parse_selection_handles_root_path() {
    let parsed = parse_selection("/home/regueiro\n").expect("should parse root");
    assert_eq!(parsed, PathBuf::from("/home/regueiro"));
}

#[test]
fn parse_selection_handles_regular_path() {
    let parsed = parse_selection("/home/regueiro/project\n").expect("should parse path");
    assert_eq!(parsed, PathBuf::from("/home/regueiro/project"));
}

#[test]
fn parse_selection_ignores_empty_input() {
    assert_eq!(parse_selection("  \n"), None);
}

#[test]
fn absolute_root_path_keeps_absolute_paths() {
    let root = absolute_root_path("/tmp").expect("should resolve absolute path");
    assert_eq!(root, "/tmp");
}

#[test]
fn absolute_root_path_resolves_relative_paths() {
    let root = absolute_root_path("relative-root").expect("should resolve relative path");
    assert!(Path::new(&root).is_absolute());
    assert!(root.ends_with("relative-root"));
}

#[test]
fn resolve_include_hidden_uses_hidden_override() {
    assert!(resolve_include_hidden(true, false));
}

#[test]
fn resolve_include_hidden_falls_back_to_default() {
    assert!(resolve_include_hidden(false, true));
    assert!(!resolve_include_hidden(false, false));
}

#[test]
fn resolve_config_toggle_interprets_enable_disable_flags() {
    assert_eq!(resolve_config_toggle(true, false), Some(true));
    assert_eq!(resolve_config_toggle(false, true), Some(false));
    assert_eq!(resolve_config_toggle(false, false), None);
}

#[test]
fn expand_home_with_expands_supported_prefixes_only() {
    let home = "/home/regueiro";
    assert_eq!(
        expand_home_with("$HOME/Projects", home),
        "/home/regueiro/Projects"
    );
    assert_eq!(
        expand_home_with("${HOME}/Projects", home),
        "/home/regueiro/Projects"
    );
    assert_eq!(
        expand_home_with("~/Projects", home),
        "/home/regueiro/Projects"
    );
    assert_eq!(expand_home_with("~", home), "/home/regueiro");
    assert_eq!(expand_home_with("/tmp/$HOME", home), "/tmp/$HOME");
}
