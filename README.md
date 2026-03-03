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

```

When `include_root = true`, the picker includes `search_root` itself as a selectable entry.


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
