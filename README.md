# tp

`tp` is a simple command-line tool written in Rust designed to manage and load
your tmux sessions from configuration files. It allows you to define complex
tmux layouts, including multiple windows and panes, and quickly load them with a
single command.

## Features

*   **Create New Sessions**: Quickly scaffold new session configuration files.
*   **Load Sessions**: Load pre-defined tmux sessions, including windows and
    panes with specific directories and commands.
*   **List Sessions**: Easily view all available session configurations.
*   **Shell Completions**: Generate shell completion scripts for various shells
    (bash, zsh, fish, etc.).

## Installation

You can install `tp` using `cargo`:

```bash
cargo install --git https://github.com/emersonmx/tp
```

## Usage

### General Help

```bash
tp --help
```

### Create a New Session File

This command creates a new session configuration file with a default structure
in your sessions directory.

```bash
tp new my-new-session
```

Example Output:
```
Created new session configuration at: /home/user/.config/tp/my-new-session.yaml
```

The created file `my-new-session.yaml` will contain:

```yaml
name: my-new-session
directory: .
windows:
  - name: shell
    panes:
      - focus: true
        command: echo 'Hello :)'
```

### Load a Session

Load a session defined in a configuration file. If the session doesn't exist, it
will be created. If it already exists, `tp` will switch to it.

```bash
tp load my-project-session
```

### List Sessions

List all available tmux session configuration files.

```bash
tp list
```

Example Output:
```
my-project-session
another-session
my-new-session
```

### Generate Shell Completions

Generate shell completion scripts for your preferred shell. This can help with
autocompletion for `tp` commands and session names.

```bash
tp completions zsh > _tp
# Then source it in your zshrc, e.g., `source ~/.zsh_completions/_tp`
```

Replace `zsh` with `bash`, `fish`, `powershell`, or `elvish` as needed.

## Configuration

`tp` looks for session configuration files in the following locations, in order
of precedence:

1.  The directory specified by the `TP_SESSIONS_DIR` environment variable.
2.  `$HOME/.config/tp/`

Session files are YAML files with a `.yaml` extension.

### Session File Structure Example

```yaml
# my-project-session.yaml
name: my-project-session
directory: ~/Code/my-project
windows:
  - name: editor
    directory: ~/Code/my-project/src
    panes:
      - focus: true
        command: nvim .
  - name: server
    panes:
      - command: cargo run
  - name: tests
    panes:
      - command: cargo test
      - command: watch cargo test
```

*   **`name`**: (Required) The name of the tmux session.
*   **`directory`**: (Optional) The base directory for the session. If not
    specified, `tp` defaults to `.` (the current directory where `tp` is run). This
    can be overridden at the window or pane level. Tilde `~` expansion is supported.
*   **`windows`**: (Optional) A list of window configurations. If not specified,
    one default window with one default pane is created.
    *   **`name`**: (Optional) The name of the window.
    *   **`directory`**: (Optional) The directory for this window. Overrides the
        session directory.
    *   **`panes`**: (Optional) A list of pane configurations within the window.
        If not specified, one default pane is created.
        *   **`focus`**: (Optional, default: `false`) If `true`, this pane will
            be selected after the session is created.
        *   **`directory`**: (Optional) The directory for this pane. Overrides
            window and session directories.
        *   **`command`**: (Optional) A command to execute in this pane upon
            creation.

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file
for details.
