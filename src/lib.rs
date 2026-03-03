mod config;
mod config_ui;

#[cfg(test)]
mod tests;

use clap::builder::styling::AnsiColor;
use clap::{builder::Styles, Parser, Subcommand};
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, Stdio};

use config::{config_path, ensure_and_load_config, expand_home, write_config};

#[derive(Parser)]
#[command(name = "runin")]
#[command(color = clap::ColorChoice::Auto)]
#[command(version)]
#[command(allow_external_subcommands = false)]
#[command(about = "runin — quickly select a project directory and run a command inside it")]
#[command(
    styles = Styles::styled()
        .header(AnsiColor::BrightCyan.on_default().bold())
        .usage(AnsiColor::BrightGreen.on_default().bold())
        .literal(AnsiColor::Yellow.on_default())
)]
#[command(
    after_help = "Usage:
  runin [OPTIONS] [CMD]...
  runin config

Examples:
  runin
  runin nvim .
  runin tmux new-session
  runin -H nvim .
  runin -H
  runin config"
)]
struct Cli {
    #[command(subcommand)]
    subcommand: Option<Commands>,

    #[arg(
        short = 'H',
        long = "hidden",
        global = true,
        help = "Include hidden directories in search (fd --hidden)"
    )]
    hidden: bool,

    #[arg(
        value_name = "CMD",
        trailing_var_arg = true,
        help = "Command to execute in the selected directory\n(defaults to configured command)"
    )]
    cmd: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Open interactive configuration")]
    Config {
        #[arg(long)]
        search_root: Option<String>,
        #[arg(long)]
        default_command: Option<String>,
        #[arg(long)]
        include_hidden: Option<bool>,
    },
}

pub fn main_entry() {
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
        include_hidden,
    }) = cli.subcommand
    {
        let old_config = config.clone();

        if search_root.is_none() && default_command.is_none() && include_hidden.is_none() {
            config_ui::interactive_config(
                &mut config.search_root,
                &mut config.default_command,
                &mut config.include_root,
                &mut config.include_hidden,
            )?;
        } else {
            if let Some(value) = search_root {
                config.search_root = value;
            }
            if let Some(value) = default_command {
                config.default_command = value;
            }
            if let Some(value) = include_hidden {
                config.include_hidden = value;
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

    let include_hidden = resolve_include_hidden(cli.hidden, config.include_hidden);
    let selected_dir = select_directory(
        &expand_home(&config.search_root),
        config.include_root,
        include_hidden,
    )?;
    let Some(selected_dir) = selected_dir else {
        return Ok(());
    };

    if cli.cmd.is_empty() {
        let parts = shell_words::split(&config.default_command)
            .map_err(|e| format!("Invalid default_command in config: {e}"))?;
        if parts.is_empty() {
            return Err("default_command cannot be empty".to_string());
        }
        exec_command(&selected_dir, parts);
    } else {
        exec_command(&selected_dir, cli.cmd);
    }
}

fn select_directory(
    search_root: &str,
    include_root: bool,
    include_hidden: bool,
) -> Result<Option<PathBuf>, String> {
    fzf_select_directory(search_root, include_root, include_hidden)
}

fn fzf_select_directory(
    search_root: &str,
    include_root: bool,
    include_hidden: bool,
) -> Result<Option<PathBuf>, String> {
    let mut fd_cmd = Command::new("fd");
    fd_cmd
        .arg("--type")
        .arg("directory")
        .arg("--absolute-path");
    if include_hidden {
        fd_cmd.arg("--hidden");
    }
    let mut fd_child = fd_cmd
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
        if include_root {
            let root = absolute_root_path(search_root)?;
            writeln!(fzf_stdin, "{root}")
                .map_err(|e| format!("Failed writing root option to fzf: {e}"))?;
        }

        let mut fd_stdout = fd_stdout;
        io::copy(&mut fd_stdout, &mut fzf_stdin)
            .map_err(|e| format!("Failed streaming directories to fzf: {e}"))?;
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

    Ok(parse_selection(&selection))
}

fn absolute_root_path(search_root: &str) -> Result<String, String> {
    let root = PathBuf::from(search_root);
    if root.is_absolute() {
        return Ok(root.to_string_lossy().into_owned());
    }

    let cwd = env::current_dir().map_err(|e| format!("Failed to read current directory: {e}"))?;
    Ok(cwd.join(root).to_string_lossy().into_owned())
}

fn parse_selection(selection: &str) -> Option<PathBuf> {
    let selected = selection.trim();
    if selected.is_empty() {
        return None;
    }

    Some(PathBuf::from(selected))
}

fn resolve_include_hidden(hidden: bool, default_include_hidden: bool) -> bool {
    if hidden {
        true
    } else {
        default_include_hidden
    }
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
