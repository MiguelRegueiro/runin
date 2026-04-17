mod config;
mod config_ui;

#[cfg(test)]
mod tests;

use clap::builder::styling::AnsiColor;
use clap::{builder::Styles, Parser, Subcommand, ValueEnum};
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, IsTerminal, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, Stdio};

use config::{config_exists, config_path, expand_home, load_config, write_config, Config};

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
    override_usage = "runin [OPTIONS] [CMD]...\n       runin config [OPTIONS]\n       runin init [SHELL]"
)]
#[command(after_help = "Examples:
  runin
  runin nvim .
  runin tmux new-session
  runin -H nvim .
  runin -H
  runin config
  runin init bash")]
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
        long = "cd",
        conflicts_with = "no_cd",
        help = "Change the parent shell to the selected directory after the command exits (requires shell integration)"
    )]
    cd: bool,

    #[arg(
        long = "no-cd",
        conflicts_with = "cd",
        help = "Do not change the parent shell directory after the command exits"
    )]
    no_cd: bool,

    #[arg(long = "emit-cd-path", hide = true, value_name = "FILE")]
    emit_cd_path: Option<PathBuf>,

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
        #[arg(long, conflicts_with = "no_include_root")]
        include_root: bool,
        #[arg(long = "no-include-root", conflicts_with = "include_root")]
        no_include_root: bool,
        #[arg(long, conflicts_with = "no_include_hidden")]
        include_hidden: bool,
        #[arg(long = "no-include-hidden", conflicts_with = "include_hidden")]
        no_include_hidden: bool,
        #[arg(long = "cd-after-run", conflicts_with = "no_cd_after_run")]
        cd_after_run: bool,
        #[arg(long = "no-cd-after-run", conflicts_with = "cd_after_run")]
        no_cd_after_run: bool,
    },
    #[command(about = "Print shell integration for persistent cd behavior")]
    Init {
        #[arg(value_enum)]
        shell: Option<Shell>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Shell {
    Bash,
    Zsh,
    Fish,
}

pub fn main_entry() {
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    let Cli {
        subcommand,
        hidden,
        cd,
        no_cd,
        emit_cd_path,
        cmd,
    } = cli;
    let config_path = config_path()?;

    match subcommand {
        Some(Commands::Config {
            search_root,
            default_command,
            include_root,
            no_include_root,
            include_hidden,
            no_include_hidden,
            cd_after_run,
            no_cd_after_run,
        }) => {
            let existed = config_exists(&config_path);
            let mut config = if existed {
                load_config(&config_path)?
            } else {
                Config::default()
            };
            let old_config = config.clone();
            let include_root = resolve_config_toggle(include_root, no_include_root);
            let include_hidden = resolve_config_toggle(include_hidden, no_include_hidden);
            let cd_after_run = resolve_config_toggle(cd_after_run, no_cd_after_run);

            if search_root.is_none()
                && default_command.is_none()
                && include_root.is_none()
                && include_hidden.is_none()
                && cd_after_run.is_none()
            {
                config_ui::interactive_config(
                    &mut config.search_root,
                    &mut config.default_command,
                    &mut config.include_root,
                    &mut config.include_hidden,
                    &mut config.cd_after_run,
                )?;
            } else {
                if let Some(value) = search_root {
                    config.search_root = value;
                }
                if let Some(value) = default_command {
                    config.default_command = value;
                }
                if let Some(value) = include_root {
                    config.include_root = value;
                }
                if let Some(value) = include_hidden {
                    config.include_hidden = value;
                }
                if let Some(value) = cd_after_run {
                    config.cd_after_run = value;
                }
            }

            if !existed || config != old_config {
                write_config(&config_path, &config)?;
                println!("saved");
            } else {
                println!("unchanged");
            }
            return Ok(());
        }
        Some(Commands::Init { shell }) => {
            let shell = shell
                .or_else(infer_shell)
                .ok_or("Could not infer shell. Run `runin init bash`, `runin init zsh`, or `runin init fish`.")?;
            print_shell_init(shell)?;
            return Ok(());
        }
        None => {}
    }

    ensure_dependencies()?;

    let config = load_or_bootstrap_runtime_config(&config_path)?;
    let include_hidden = resolve_include_hidden(hidden, config.include_hidden);
    let cd_after_run = resolve_config_toggle(cd, no_cd).unwrap_or(config.cd_after_run);
    let selected_dir = select_directory(
        &expand_home(&config.search_root),
        config.include_root,
        include_hidden,
    )?;
    let Some(selected_dir) = selected_dir else {
        return Ok(());
    };

    let parts = if cmd.is_empty() {
        let parts = shell_words::split(&config.default_command)
            .map_err(|e| format!("Invalid default_command in config: {e}"))?;
        if parts.is_empty() {
            return Err("default_command cannot be empty".to_string());
        }
        parts
    } else {
        cmd
    };

    if cd_after_run {
        if let Some(path) = &emit_cd_path {
            write_cd_target(path, &selected_dir)?;
        }
    }

    exec_command(&selected_dir, parts, emit_cd_path.as_deref());
}

fn load_or_bootstrap_runtime_config(config_path: &Path) -> Result<Config, String> {
    if config_exists(config_path) {
        return load_config(config_path);
    }

    if let Some(err) = missing_config_non_interactive_error(
        config_path,
        io::stdin().is_terminal(),
        io::stdout().is_terminal(),
    ) {
        return Err(err);
    }

    eprintln!(
        "No config found at {}. Launching first-run setup.",
        config_path.display()
    );

    let mut config = Config::default();
    config_ui::interactive_config(
        &mut config.search_root,
        &mut config.default_command,
        &mut config.include_root,
        &mut config.include_hidden,
        &mut config.cd_after_run,
    )?;
    write_config(config_path, &config)?;
    println!("saved");
    Ok(config)
}

fn missing_config_non_interactive_error(
    config_path: &Path,
    stdin_is_tty: bool,
    stdout_is_tty: bool,
) -> Option<String> {
    if stdin_is_tty && stdout_is_tty {
        None
    } else {
        Some(format!(
            "Config not found at {}.\nRun `runin config` in an interactive terminal to create it.",
            config_path.display()
        ))
    }
}

fn infer_shell() -> Option<Shell> {
    let shell = env::var_os("SHELL")?;
    let name = Path::new(&shell).file_name()?.to_str()?;
    match name {
        "bash" => Some(Shell::Bash),
        "zsh" => Some(Shell::Zsh),
        "fish" => Some(Shell::Fish),
        _ => None,
    }
}

fn print_shell_init(shell: Shell) -> Result<(), String> {
    let runin_bin =
        env::current_exe().map_err(|e| format!("Failed to locate runin binary: {e}"))?;
    print!("{}", shell_init(shell, &runin_bin));
    Ok(())
}

fn shell_init(shell: Shell, runin_bin: &Path) -> String {
    match shell {
        Shell::Bash | Shell::Zsh => posix_shell_init(runin_bin),
        Shell::Fish => fish_shell_init(runin_bin),
    }
}

fn posix_shell_init(runin_bin: &Path) -> String {
    let runin_bin = sh_single_quote(&runin_bin.to_string_lossy());
    format!(
        r#"# runin shell integration
runin() {{
    local _runin_target _runin_status _runin_dir

    _runin_target="$(mktemp "${{TMPDIR:-/tmp}}/runin-cd.XXXXXX")" || return
    {runin_bin} --emit-cd-path "$_runin_target" "$@"
    _runin_status=$?

    if [ -s "$_runin_target" ]; then
        IFS= read -r _runin_dir < "$_runin_target"
        rm -f "$_runin_target"
        if [ -n "$_runin_dir" ]; then
            cd -- "$_runin_dir" || return $?
        fi
    else
        rm -f "$_runin_target"
    fi

    return "$_runin_status"
}}
"#
    )
}

fn fish_shell_init(runin_bin: &Path) -> String {
    let runin_bin = fish_single_quote(&runin_bin.to_string_lossy());
    format!(
        r#"# runin shell integration
function runin
    set -l _runin_tmpdir
    if set -q TMPDIR
        set _runin_tmpdir $TMPDIR
    else
        set _runin_tmpdir /tmp
    end

    set -l _runin_target (mktemp "$_runin_tmpdir/runin-cd.XXXXXX")
    or return

    {runin_bin} --emit-cd-path "$_runin_target" $argv
    set -l _runin_status $status

    if test -s "$_runin_target"
        set -l _runin_dir
        read -l _runin_dir < "$_runin_target"
        rm -f "$_runin_target"
        if test -n "$_runin_dir"
            cd "$_runin_dir"
            or return $status
        end
    else
        rm -f "$_runin_target"
    end

    return $_runin_status
end
"#
    )
}

fn sh_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn fish_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
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
    let root_path = Path::new(search_root);
    if !root_path.exists() {
        return Err(format!("Search root does not exist: {search_root}"));
    }
    if !root_path.is_dir() {
        return Err(format!("Search root is not a directory: {search_root}"));
    }

    let mut fd_cmd = Command::new("fd");
    fd_cmd.arg("--type").arg("directory").arg("--absolute-path");
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

    let mut fzf_closed_stdin_early = false;
    {
        let mut fzf_stdin = fzf_child
            .stdin
            .take()
            .ok_or("Failed to capture fzf stdin")?;
        if include_root {
            let root = absolute_root_path(search_root)?;
            if let Err(err) = writeln!(fzf_stdin, "{root}") {
                if is_broken_pipe(&err) {
                    fzf_closed_stdin_early = true;
                } else {
                    return Err(format!("Failed writing root option to fzf: {err}"));
                }
            }
        }

        if !fzf_closed_stdin_early {
            let mut fd_stdout = fd_stdout;
            if let Err(err) = io::copy(&mut fd_stdout, &mut fzf_stdin) {
                if is_broken_pipe(&err) {
                    fzf_closed_stdin_early = true;
                } else {
                    return Err(format!("Failed streaming directories to fzf: {err}"));
                }
            }
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
    let fd_status = fd_child
        .wait()
        .map_err(|e| format!("Failed to wait on fd: {e}"))?;

    if !fd_status.success() && !fzf_closed_stdin_early {
        return Err(format!(
            "fd failed while listing directories (search_root: {search_root}, include_hidden: {include_hidden})"
        ));
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

fn is_broken_pipe(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::BrokenPipe
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

fn resolve_config_toggle(enable: bool, disable: bool) -> Option<bool> {
    if enable {
        Some(true)
    } else if disable {
        Some(false)
    } else {
        None
    }
}

fn write_cd_target(path: &Path, selected_dir: &Path) -> Result<(), String> {
    let mut content = selected_dir.to_string_lossy().into_owned();
    content.push('\n');
    fs::write(path, content)
        .map_err(|e| format!("Failed writing shell cd target {}: {e}", path.display()))
}

fn ensure_dependencies() -> Result<(), String> {
    let required = ["fd", "fzf"];
    let missing: Vec<&str> = required
        .into_iter()
        .filter(|binary| {
            !matches!(
                Command::new(binary)
                    .arg("--version")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status(),
                Ok(status) if status.success()
            )
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

fn exec_command(selected_dir: &Path, mut parts: Vec<String>, cd_target_path: Option<&Path>) -> ! {
    let program = parts.remove(0);
    let err = Command::new(&program)
        .args(parts)
        .current_dir(selected_dir)
        .exec();

    if let Some(path) = cd_target_path {
        let _ = fs::remove_file(path);
    }

    eprintln!("Failed to execute '{program}': {err}");
    process::exit(1);
}
