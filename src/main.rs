mod config_ui;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};

const DEFAULT_SEARCH_ROOT: &str = "$HOME/Documents";
const DEFAULT_COMMAND: &str = "code .";

#[derive(Parser)]
#[command(name = "runin")]
#[command(about = "Pick a project directory with fd+fzf and run a command")]
struct Cli {
    #[command(subcommand)]
    subcommand: Option<Commands>,

    #[arg(value_name = "command", trailing_var_arg = true)]
    command: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    Config {
        #[arg(long)]
        search_root: Option<String>,
        #[arg(long)]
        default_command: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Config {
    search_root: String,
    default_command: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            search_root: DEFAULT_SEARCH_ROOT.to_string(),
            default_command: DEFAULT_COMMAND.to_string(),
        }
    }
}

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    ensure_dependencies()?;

    let config_path = config_path()?;
    let mut config = ensure_and_load_config(&config_path)?;

    if let Some(Commands::Config {
        search_root,
        default_command,
    }) = cli.subcommand
    {
        let old_config = config.clone();

        if search_root.is_none() && default_command.is_none() {
            config_ui::interactive_config(&mut config.search_root, &mut config.default_command)?;
        } else {
            if let Some(value) = search_root {
                config.search_root = value;
            }
            if let Some(value) = default_command {
                config.default_command = value;
            }
        }

        if config != old_config {
            write_config(&config_path, &config)?;
            println!("saved");
        } else {
            println!("unchanged");
        }
        return Ok(());
    }

    let selected_dir = select_directory(&expand_home(&config.search_root))?;
    let Some(selected_dir) = selected_dir else {
        return Ok(());
    };

    if cli.command.is_empty() {
        let parts = shell_words::split(&config.default_command)
            .map_err(|e| format!("Invalid default_command in config: {e}"))?;
        if parts.is_empty() {
            return Err("default_command cannot be empty".to_string());
        }
        exec_command(&selected_dir, parts);
    } else {
        exec_command(&selected_dir, cli.command);
    }
}

fn select_directory(search_root: &str) -> Result<Option<PathBuf>, String> {
    fzf_select_directory(search_root)
}

fn fzf_select_directory(search_root: &str) -> Result<Option<PathBuf>, String> {
    let mut fd_child = Command::new("fd")
        .arg("--type")
        .arg("directory")
        .arg("--absolute-path")
        .arg(".")
        .arg(search_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to run fd: {e}"))?;

    let fd_stdout = fd_child
        .stdout
        .take()
        .ok_or("Failed to capture fd stdout")?;

    let mut fzf_child = Command::new("fzf")
        .arg("--height")
        .arg("60%")
        .arg("--layout")
        .arg("reverse")
        .arg("--border")
        .arg("--info")
        .arg("inline-right")
        .arg("--header")
        .arg("Type to filter | Enter select | Esc cancel")
        .arg("--prompt")
        .arg("project > ")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn fzf: {e}"))?;

    {
        let mut fzf_stdin = fzf_child
            .stdin
            .take()
            .ok_or("Failed to capture fzf stdin")?;
        writeln!(fzf_stdin, "{search_root}").map_err(|e| format!("Failed writing to fzf: {e}"))?;

        let mut fd_stdout = fd_stdout;
        io::copy(&mut fd_stdout, &mut fzf_stdin)
            .map_err(|e| format!("Failed streaming directories to fzf: {e}"))?;
    }

    let mut selection = String::new();
    {
        let mut stdout = fzf_child.stdout.take().ok_or("Failed to capture fzf stdout")?;
        BufReader::new(&mut stdout)
            .read_line(&mut selection)
            .map_err(|e| format!("Failed reading fzf output: {e}"))?;
    }

    let status = fzf_child
        .wait()
        .map_err(|e| format!("Failed to wait on fzf: {e}"))?;
    let fd_status = fd_child
        .wait()
        .map_err(|e| format!("Failed to wait on fd: {e}"))?;

    if !fd_status.success() {
        return Err("fd failed while listing directories".to_string());
    }

    if let Some(code) = status.code() {
        if code == 130 {
            process::exit(130);
        }
        if code != 0 {
            return Ok(None);
        }
    } else if !status.success() {
        return Err("fzf terminated by signal".to_string());
    }

    let selected = selection.trim();
    if selected.is_empty() {
        return Ok(None);
    }

    Ok(Some(PathBuf::from(selected)))
}

fn ensure_dependencies() -> Result<(), String> {
    let required = ["fd", "fzf"];
    let missing: Vec<&str> = required
        .into_iter()
        .filter(|binary| {
            Command::new(binary)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_err()
        })
        .collect();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Missing required dependencies: {}.\nInstall `fd` and `fzf`, and ensure both are available in PATH.",
            missing.join(", ")
        ))
    }
}

fn exec_command(selected_dir: &Path, mut parts: Vec<String>) -> ! {
    let program = parts.remove(0);
    let err = Command::new(&program)
        .args(parts)
        .current_dir(selected_dir)
        .exec();

    eprintln!("Failed to execute '{program}': {err}");
    process::exit(1);
}

fn config_path() -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME environment variable is not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("runin")
        .join("config.toml"))
}

fn ensure_and_load_config(path: &Path) -> Result<Config, String> {
    if !path.exists() {
        let cfg = Config::default();
        write_config(path, &cfg)?;
        return Ok(cfg);
    }

    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Failed reading config {}: {e}", path.display()))?;
    toml::from_str(&raw).map_err(|e| format!("Failed parsing config {}: {e}", path.display()))
}

fn write_config(path: &Path, cfg: &Config) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed creating config directory {}: {e}", parent.display()))?;
    }

    let content = toml::to_string_pretty(cfg)
        .map_err(|e| format!("Failed serializing config: {e}"))?;
    fs::write(path, content).map_err(|e| format!("Failed writing config {}: {e}", path.display()))
}

fn expand_home(path: &str) -> String {
    if let Some(home) = env::var_os("HOME") {
        path.replace("$HOME", &home.to_string_lossy())
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_and_load_config, write_config, Config, DEFAULT_COMMAND, DEFAULT_SEARCH_ROOT};
    use std::fs;
    use std::path::PathBuf;
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
        assert!(config_path.exists());
    }

    #[test]
    fn write_and_load_config_roundtrip() {
        let dir = TestDir::new();
        let config_path = dir.path.join("config.toml");
        let expected = Config {
            search_root: "/home/regueiro".to_string(),
            default_command: "qwen".to_string(),
        };

        write_config(&config_path, &expected).expect("write config should succeed");
        let loaded = ensure_and_load_config(&config_path).expect("load config should succeed");

        assert_eq!(loaded, expected);
    }

    #[test]
    fn ensure_and_load_config_returns_error_for_invalid_toml() {
        let dir = TestDir::new();
        let config_path = dir.path.join("config.toml");
        fs::write(&config_path, "not-valid-toml = [")
            .expect("failed to write invalid test config");

        let err = ensure_and_load_config(&config_path).expect_err("invalid TOML should fail");
        assert!(err.contains("Failed parsing config"));
    }
}
