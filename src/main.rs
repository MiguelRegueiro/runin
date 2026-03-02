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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    let config_path = config_path()?;
    let mut config = ensure_and_load_config(&config_path)?;

    if let Some(Commands::Config {
        search_root,
        default_command,
    }) = cli.subcommand
    {
        if search_root.is_none() && default_command.is_none() {
            interactive_config(&mut config)?;
        } else {
            if let Some(value) = search_root {
                config.search_root = value;
            }
            if let Some(value) = default_command {
                config.default_command = value;
            }
        }
        write_config(&config_path, &config)?;
        println!("Updated config at {}", config_path.display());
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
    let fd_output = Command::new("fd")
        .arg("--type")
        .arg("directory")
        .arg("--absolute-path")
        .arg(".")
        .arg(search_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("Failed to run fd: {e}"))?;

    if !fd_output.status.success() {
        return Err("fd failed while listing directories".to_string());
    }

    let mut fzf_child = Command::new("fzf")
        .arg("--height")
        .arg("60%")
        .arg("--layout")
        .arg("reverse")
        .arg("--border")
        .arg("--prompt")
        .arg("search_root > ")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn fzf: {e}"))?;

    {
        let mut stdin = fzf_child
            .stdin
            .take()
            .ok_or("Failed to capture fzf stdin")?;
        writeln!(stdin, "{search_root}").map_err(|e| format!("Failed writing to fzf: {e}"))?;
        stdin
            .write_all(&fd_output.stdout)
            .map_err(|e| format!("Failed writing fd output to fzf: {e}"))?;
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

fn interactive_config(cfg: &mut Config) -> Result<(), String> {
    println!("================");
    println!("  runin config");
    println!("================");
    println!("Press Enter to keep the current value.");
    println!();

    if let Some(new_root) = prompt_search_root(&cfg.search_root)? {
        cfg.search_root = new_root;
    }
    println!();
    if let Some(new_command) = prompt_with_default("default_command", &cfg.default_command)? {
        cfg.default_command = new_command;
    }
    Ok(())
}

fn prompt_search_root(current: &str) -> Result<Option<String>, String> {
    println!("search_root");
    println!("  current: {current}");
    println!("  choose:");
    println!("    - Change search_root (fd+fzf)");
    println!("    - Keep current search_root");
    println!("  (Enter confirms, arrow keys move)");

    let choice = fzf_select_option(
        vec![
            "Change search_root (fd+fzf)",
            "Keep current search_root",
        ],
        "action > ",
    )?;

    let Some(choice) = choice else {
        return Ok(None);
    };

    if choice == "Keep current search_root" {
        println!("  selected: {current}");
        return Ok(None);
    }

    if choice == "Change search_root (fd+fzf)" {
        let home = env::var("HOME").map_err(|_| "HOME environment variable is not set".to_string())?;
        let picked = fzf_select_directory(&home)?;
        if let Some(path) = picked {
            let path_str = path.to_string_lossy().to_string();
            println!("  selected: {path_str}");
            return Ok(Some(path_str));
        }
        println!("  selected: {current}");
        return Ok(None);
    }

    Ok(None)
}

fn prompt_with_default(field: &str, current: &str) -> Result<Option<String>, String> {
    println!("{field}");
    println!("  current: {current}");
    print!("  new value: ");
    io::stdout()
        .flush()
        .map_err(|e| format!("Failed flushing stdout: {e}"))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed reading {field}: {e}"))?;

    let value = input.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value.to_string()))
    }
}

fn fzf_select_option(options: Vec<&str>, prompt: &str) -> Result<Option<String>, String> {
    let mut fzf_child = Command::new("fzf")
        .arg("--height")
        .arg("40%")
        .arg("--layout")
        .arg("reverse")
        .arg("--border")
        .arg("--prompt")
        .arg(prompt)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn fzf: {e}"))?;

    {
        let mut stdin = fzf_child
            .stdin
            .take()
            .ok_or("Failed to capture fzf stdin")?;
        for option in options {
            writeln!(stdin, "{option}").map_err(|e| format!("Failed writing to fzf: {e}"))?;
        }
    }

    let mut selection = String::new();
    {
        let mut stdout = fzf_child
            .stdout
            .take()
            .ok_or("Failed to capture fzf stdout")?;
        BufReader::new(&mut stdout)
            .read_line(&mut selection)
            .map_err(|e| format!("Failed reading fzf output: {e}"))?;
    }

    let status = fzf_child
        .wait()
        .map_err(|e| format!("Failed to wait on fzf: {e}"))?;

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

    let picked = selection.trim();
    if picked.is_empty() {
        Ok(None)
    } else {
        Ok(Some(picked.to_string()))
    }
}

fn expand_home(path: &str) -> String {
    if let Some(home) = env::var_os("HOME") {
        path.replace("$HOME", &home.to_string_lossy())
    } else {
        path.to_string()
    }
}
