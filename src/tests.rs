use crate::config::{
    Config, DEFAULT_COMMAND, DEFAULT_SEARCH_ROOT, DirectorySource, expand_home_with, load_config,
    write_config,
};
use crate::{
    Shell, abbreviate_home_with, abbreviate_zoxide_candidates_with, absolute_root_path,
    is_broken_pipe, missing_config_non_interactive_error, parse_selection, remove_managed_block,
    resolve_config_toggle, resolve_include_hidden, selection_path, shell_init, source_block,
    upsert_managed_block, write_cd_target, write_shell_integration,
};
use std::fs;
use std::io;
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
fn write_and_load_default_config_when_missing() {
    let dir = TestDir::new();
    let config_path = dir.path.join("config.toml");

    let expected = Config::default();
    write_config(&config_path, &expected).expect("should create default config");
    let cfg = load_config(&config_path).expect("should load config");

    assert_eq!(cfg.search_root, DEFAULT_SEARCH_ROOT);
    assert_eq!(cfg.default_command, DEFAULT_COMMAND);
    assert_eq!(cfg.directory_source, DirectorySource::Zoxide);
    assert!(!cfg.include_root);
    assert!(!cfg.include_hidden);
    assert!(cfg.cd_after_run);
    assert!(config_path.exists());
}

#[test]
fn write_and_load_config_roundtrip() {
    let dir = TestDir::new();
    let config_path = dir.path.join("config.toml");
    let expected = Config {
        search_root: "/home/antonio".to_string(),
        default_command: "qwen".to_string(),
        directory_source: DirectorySource::Fd,
        include_root: true,
        include_hidden: true,
        cd_after_run: false,
    };

    write_config(&config_path, &expected).expect("write config should succeed");
    let loaded = load_config(&config_path).expect("load config should succeed");

    assert_eq!(loaded, expected);
}

#[test]
fn load_config_returns_error_for_invalid_toml() {
    let dir = TestDir::new();
    let config_path = dir.path.join("config.toml");
    fs::write(&config_path, "not-valid-toml = [").expect("failed to write invalid test config");

    let err = load_config(&config_path).expect_err("invalid TOML should fail");
    assert!(err.contains("Failed parsing config"));
}

#[test]
fn load_config_defaults_toggles_when_missing() {
    let dir = TestDir::new();
    let config_path = dir.path.join("config.toml");
    fs::write(
        &config_path,
        "search_root = \"/home/antonio\"\ndefault_command = \"qwen\"\n",
    )
    .expect("failed to write config without include_root");

    let cfg = load_config(&config_path).expect("load config should succeed");
    assert!(!cfg.include_root);
    assert!(!cfg.include_hidden);
    assert!(cfg.cd_after_run);
    assert_eq!(cfg.directory_source, DirectorySource::Zoxide);
}

#[test]
fn write_cd_target_writes_selected_directory() {
    let dir = TestDir::new();
    let target_path = dir.path.join("target");

    write_cd_target(&target_path, Path::new("/home/antonio/project"))
        .expect("write cd target should succeed");

    let target = fs::read_to_string(target_path).expect("target should be readable");
    assert_eq!(target, "/home/antonio/project\n");
}

#[test]
fn shell_init_embeds_binary_path_instead_of_requiring_path_lookup() {
    let bash = shell_init(Shell::Bash, Path::new("/tmp/runin"));
    let fish = shell_init(Shell::Fish, Path::new("/tmp/runin"));

    assert!(bash.contains("'/tmp/runin' --emit-cd-path"));
    assert!(fish.contains("'/tmp/runin' --emit-cd-path"));
    assert!(bash.contains("export RUNIN_SHELL_INTEGRATION=1"));
    assert!(fish.contains("set -gx RUNIN_SHELL_INTEGRATION 1"));
    assert!(!bash.contains("command runin"));
    assert!(!fish.contains("command runin"));
}

#[test]
fn managed_shell_block_is_idempotent_and_removable() {
    let block = source_block(Path::new("/tmp/runin/runin.bash"));
    let original = "alias ll='ls -la'\n";

    let once = upsert_managed_block(original, &block);
    let twice = upsert_managed_block(&once, &block);

    assert_eq!(once, twice);
    assert!(once.contains("alias ll='ls -la'"));
    assert!(once.contains("# >>> runin shell integration >>>"));
    assert_eq!(remove_managed_block(&once), original);
}

#[test]
fn write_shell_integration_creates_parent_directories() {
    let dir = TestDir::new();
    let install_path = dir.path.join("fish").join("conf.d").join("runin.fish");

    write_shell_integration(Shell::Fish, &install_path, Path::new("/tmp/runin"))
        .expect("shell integration should be written");

    let content = fs::read_to_string(install_path).expect("script should be readable");
    assert!(content.contains("function runin"));
    assert!(content.contains("'/tmp/runin' --emit-cd-path"));
}

#[test]
fn parse_selection_handles_root_path() {
    let parsed = parse_selection("/home/antonio\n").expect("should parse root");
    assert_eq!(parsed, PathBuf::from("/home/antonio"));
}

#[test]
fn parse_selection_handles_regular_path() {
    let parsed = parse_selection("/home/antonio/project\n").expect("should parse path");
    assert_eq!(parsed, PathBuf::from("/home/antonio/project"));
}

#[test]
fn parse_selection_ignores_empty_input() {
    assert_eq!(parse_selection("  \n"), None);
}

#[test]
fn selection_path_expands_home_after_trimming_fzf_output() {
    let selected = selection_path("~\n").expect("should parse selection");
    assert_eq!(
        selected,
        PathBuf::from(std::env::var("HOME").expect("HOME should be set"))
    );
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
fn is_broken_pipe_matches_broken_pipe_only() {
    assert!(is_broken_pipe(&io::Error::from(io::ErrorKind::BrokenPipe)));
    assert!(!is_broken_pipe(&io::Error::from(
        io::ErrorKind::PermissionDenied
    )));
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
fn missing_config_error_allows_interactive_setup_when_tty_available() {
    let path = Path::new("/tmp/runin-config.toml");
    assert_eq!(missing_config_non_interactive_error(path, true, true), None);
}

#[test]
fn missing_config_error_blocks_when_stdin_is_not_tty() {
    let path = Path::new("/tmp/runin-config.toml");
    let err =
        missing_config_non_interactive_error(path, false, true).expect("expected non-tty error");
    assert!(err.contains("Config not found"));
    assert!(err.contains("runin config"));
}

#[test]
fn missing_config_error_blocks_when_stdout_is_not_tty() {
    let path = Path::new("/tmp/runin-config.toml");
    let err =
        missing_config_non_interactive_error(path, true, false).expect("expected non-tty error");
    assert!(err.contains("Config not found"));
    assert!(err.contains("runin config"));
}

#[test]
fn expand_home_with_expands_supported_prefixes_only() {
    let home = "/home/antonio";
    assert_eq!(
        expand_home_with("$HOME/Projects", home),
        "/home/antonio/Projects"
    );
    assert_eq!(
        expand_home_with("${HOME}/Projects", home),
        "/home/antonio/Projects"
    );
    assert_eq!(
        expand_home_with("~/Projects", home),
        "/home/antonio/Projects"
    );
    assert_eq!(expand_home_with("~", home), "/home/antonio");
    assert_eq!(expand_home_with("/tmp/$HOME", home), "/tmp/$HOME");
}

#[test]
fn abbreviate_home_with_replaces_only_a_home_path_prefix() {
    assert_eq!(
        abbreviate_home_with("/home/antonio/project", Some("/home/antonio")),
        "~/project"
    );
    assert_eq!(
        abbreviate_home_with("/home/antonio", Some("/home/antonio")),
        "~"
    );
    assert_eq!(
        abbreviate_home_with("/home/antonio-other", Some("/home/antonio")),
        "/home/antonio-other"
    );
}

#[test]
fn abbreviate_zoxide_candidates_keeps_scores_and_shortens_home_paths() {
    assert_eq!(
        abbreviate_zoxide_candidates_with(" 218.0 /home/antonio/project\n", Some("/home/antonio")),
        " 218.0\t~/project\t/home/antonio/project"
    );
}
