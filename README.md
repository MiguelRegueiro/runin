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

---


## Configuration


Open interactive configuration:


```bash

runin config

```

If `~/.config/runin/config.toml` does not exist, running `runin` launches the same interactive setup flow automatically.

Enable persistent directory changes in your shell:

```bash

eval "$(runin init bash)"

```

Use `zsh` or `fish` instead of `bash` for those shells. Add the same line to your shell startup file to keep it enabled.

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

When `cd_after_run = true`, `runin` changes the current shell to the selected directory after the command exits. This requires the shell integration from `runin init`.


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
