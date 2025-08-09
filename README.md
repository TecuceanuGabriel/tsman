# Tsman

A lightweight Tmux session manager with fuzzy-finding (powered by
[fuzzy-matcher](https://github.com/skim-rs/fuzzy-matcher)).

## Installation

```bash
cargo install tsman
```

## Usage

### Save current session

```bash
tsman save <session_name> # uses the specified name
tsman save # uses the current name of the session
```

### Open a session

```bash
tsman open <session_name>
```

### Edit a session config file

The file is opened for editing in `$EDITOR`.

```bash
tsman edit <session_name> # edit the config file of the specified session
tsman edit # edit the config file of the current session
```

### Delete a session config file

```bash
tsman delete <session_name>
```

### Open a menu that contains all saved sessions

```bash
tsman menu
tsman menu --preview # open the menu with the preview pane on
tsman menu --ask-for-confirmation # Open a confirmation popup when deleting
```

#### Menu keybindings:

Navigation:

| command              | action               |
| -------------------- | -------------------- |
| `Esc` / `C-c`        | Exit menu            |
| `Up arrow` / `C-p`   | Select previous item |
| `Down arrow` / `C-n` | Select next item     |

UI Controls:

| command | action                                    |
| ------- | ----------------------------------------- |
| `C-t`   | Toggle the visibility of the preview pane |
| `C-w`   | Delete last word from input               |

Session actions:

| command | saved session action                   | unsaved session actoin |
| ------- | -------------------------------------- | ---------------------- |
| `C-e`   | Edit config file of selected session   | -                      |
| `C-d`   | Delete config file of selected session | Kill session           |
| `C-s`   | -                                      | Save session           |
| `Enter` | Open session                           | Open session           |

## Configuration

You can add keybindings/aliases to your tmux/shell config file for faster usage.

Example config:

`~/.tmux.conf`:

```bash
bind -r f run-shell "tmux neww 'tsman menu -p -a'" # with preview pane and
                                                   # confirmation popup on
bind -r C-s run-shell "tsman save"
```

`~/.zshrc`:

```bash
alias mux-fd="tsman menu -p -a"
```

If you want to set up a custom location to store session config files set the
`TSMAN_CONFIG_STORAGE_DIR` env variable. You can add the following line to
your shell config file to make it persistent:

```bash
export TSMAN_CONFIG_STORAGE_DIR="$HOME/mux-sessions"
```

## Notes

- `$EDITOR` must be set to use the edit command.
- the session config files are saved by defult in `~/.config/.tsessions/`.

## Acknowledgements

- [tmuxinator](https://github.com/tmuxinator/tmuxinator)
- [ThePrimeagen's tmux-sessionizer](https://github.com/ThePrimeagen/tmux-sessionizer)
