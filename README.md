 # runin


Pick a directory under a configured root using `fd + fzf` and run a command inside it.


`runin` is designed for quickly running commands (e.g. `nvim`, `code`, `tmux`) from any project directory.


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


---


## How it works


- Uses `fd` to list directories under `search_root`

- Pipes results into `fzf` for interactive selection

- Executes the selected command inside the chosen directory


---


## Dependencies


`runin` requires:


- `fd`

- `fzf`


Both are mandatory and must be available in your `PATH`.

If missing, `runin` prints a clear message to install `fd` and `fzf` and ensure both are in `PATH`.

When `include_root = true`, the picker includes the configured `search_root` itself as a selectable entry.


---


## Running (no pun intended)


From source:


```bash

git clone https://github.com/MiguelRegueiro/runin

cd runin

cargo run --release

```

---


## Philosophy


`runin` is intentionally simple.


Select directory → run command → done. 
