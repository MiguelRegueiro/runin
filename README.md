 # runin

[![CI](https://github.com/MiguelRegueiro/runin/actions/workflows/ci.yml/badge.svg)](https://github.com/MiguelRegueiro/runin/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/runin.svg)](https://crates.io/crates/runin)

Pick a project directory with `fd + fzf` and run a command inside it.


---


## Usage


Run the default configured command:


```bash

runin

```


Run a custom command instead of the default:


```bash

runin nvim .

```


```bash

runin tmux new-session

```

Run without changing your shell directory afterward:

```bash

runin --no-cd

```

---

## Installation

Install from crates.io:

```bash
cargo install runin
```

Enable persistent directory changes:

```bash
runin shell install
```

Restart your shell after installing, or run the source command printed by `runin shell install`.

---


## Configuration


Open interactive configuration:


```bash

runin config

```

If `~/.config/runin/config.toml` does not exist, running `runin` launches the same interactive setup flow automatically.

Enable persistent directory changes in your shell:

```bash

runin shell install

```

`runin shell install` detects your shell from `$SHELL`. You can also pass it explicitly:

```bash
runin shell install fish
runin shell install zsh
runin shell install bash
```

For fish, this writes `~/.config/fish/conf.d/runin.fish`. For bash and zsh, it writes `~/.config/runin/runin.bash` or `~/.config/runin/runin.zsh` and adds a managed source block to your shell startup file.

Check whether shell integration is installed and active:

```bash
runin shell status
runin doctor
```

Remove shell integration:

```bash
runin shell uninstall
```

For manual setups, `runin init bash`, `runin init zsh`, and `runin init fish` still print the shell integration to stdout.

Interactive flow:

```text
runin config
────────────
Search root [/home/user]:
>
Default command [nvim .]:
>
Include root [n]:
>
Include hidden paths [n]:
>
Change shell directory after run [y]:
>
saved
```

If no values change, status prints `unchanged`.


Config file location:


```

~/.config/runin/config.toml

```


Example configuration:


```toml

search_root = "/home/user"

default_command = "nvim ."

include_root = false

include_hidden = false

cd_after_run = true

```

When `include_root = true`, the picker includes `search_root` itself as a selectable entry.

When `cd_after_run = true`, `runin` changes the current shell to the selected directory after the command exits. This requires shell integration from `runin shell install`.


---


## How it works


- Uses `fd` to list directories under `search_root`

- Pipes results into `fzf` for interactive selection

- Executes the selected command inside the chosen directory


---


## Dependencies

Required tools:
- `fd`
- `fzf`

Both must be available in your `PATH`.
If missing, `runin` prints an install hint.


---


<details>
<summary>Running from source</summary>

```bash
git clone https://github.com/MiguelRegueiro/runin
cd runin
cargo run --release
```

</details>

---


## Philosophy


`runin` is intentionally simple.


Select directory → run command → done. 
