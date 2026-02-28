//! CLI argument parser
use crate::util::validate_session_name;
use clap::{Parser, Subcommand};

/// Command-line argument parser for `tsman`.
#[derive(Debug, Parser)]
#[command(name = "tsman")]
#[command(
    about = "A session manager for tmux",
    long_about = "tsman - A lightweight session manager for tmux.

Key Features:
 - Quickly save/restore/delete sessions. 
 - Manage sessions from the interactive TUI menu.
 - Manually edit saved session for better control.

Examples:
 tsman save my-session # save the current session as `my-session`
 tsman edit my-session # edit `my-session` for your liking
 tsman open my-session # restore `my-session`
 tsman menu -p -a      # open the TUI menu with the preview panel and 
                       # delete confirmation prompting on

Use `tsman <COMMAND> --help` for more details."
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

/// CLI subcommands for `tsman`.
///
/// Each variant corresponds to an action that can be performed on `tmux`
/// sessions.
#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(
        about = "Save the current session",
        long_about = "Save the currently attached tmux session. Stores the 
<session_name>.yaml config file in $TSMAN_CONFIG_STORAGE_DIR if set, or 
~/.config/.tsessions.",
        alias = "s"
    )]
    Save {
        /// Name of the session (default: name of current session)
        #[arg(value_parser = validate_session_name)]
        session_name: Option<String>,
    },

    #[command(
        about = "Open the specified session",
        long_about = "Restore the selected session and then attach to it.",
        arg_required_else_help = true,
        alias = "o"
    )]
    Open {
        /// Name of the session
        #[arg(value_parser = validate_session_name)]
        session_name: String,
    },

    #[command(
        about = "Edit the specified session",
        long_about = "Open the config file of the specified session in $EDITOR
for manual editing.",
        alias = "e"
    )]
    Edit {
        /// Name of the session (default: name of current session)
        #[arg(value_parser = validate_session_name)]
        session_name: Option<String>,
    },

    #[command(
        about = "Delete specified session",
        long_about = "Remove the config file of the specified session from the
config storage directory.",
        arg_required_else_help = true,
        alias = "d"
    )]
    Delete {
        /// Name of the session
        #[arg(value_parser = validate_session_name)]
        session_name: String,
    },

    #[command(
        about = "Open up a menu containing all sessions",
        long_about = "Open up an interactive menu containing all saved or 
currently active sessions.",
        alias = "m"
    )]
    Menu {
        #[clap(long, short, help = "Show preview pane on start up")]
        preview: bool,
        #[clap(
            long,
            short,
            help = "Prompt for confirmation before deleting a session"
        )]
        ask_for_confirmation: bool,
    },

    #[command(
        about = "Manage layout templates",
        long_about = "Manage layout templates. Layouts capture window/pane structure
without working directories, allowing reuse across projects.",
        alias = "l"
    )]
    Layout {
        #[command(subcommand)]
        command: LayoutCommands,
    },
}

/// Subcommands for managing layout templates.
#[derive(Debug, Subcommand)]
pub enum LayoutCommands {
    #[command(
        about = "Save current session as a layout",
        long_about = "Capture the current tmux session's window/pane structure as a
reusable layout template. Stores the <layout_name>.yaml config file in
$TSMAN_LAYOUT_STORAGE_DIR if set, or ~/.config/.tlayouts.",
        alias = "s"
    )]
    Save {
        /// Name of the layout (default: name of current session)
        #[arg(value_parser = validate_session_name)]
        layout_name: Option<String>,
    },

    #[command(
        about = "Create a new session from a layout",
        long_about = "Create a new tmux session using a saved layout template.
All panes will start in the specified working directory.",
        arg_required_else_help = true,
        alias = "c"
    )]
    Create {
        /// Name of the layout to use
        #[arg(value_parser = validate_session_name)]
        layout_name: String,

        /// Working directory for the new session
        work_dir: String,

        /// Name for the new session (default: layout name)
        #[arg(value_parser = validate_session_name)]
        session_name: Option<String>,
    },

    #[command(
        about = "List all saved layouts",
        alias = "ls"
    )]
    List,

    #[command(
        about = "Delete a saved layout",
        arg_required_else_help = true,
        alias = "d"
    )]
    Delete {
        /// Name of the layout
        #[arg(value_parser = validate_session_name)]
        layout_name: String,
    },

    #[command(
        about = "Edit a layout config file",
        long_about = "Open the config file of the specified layout in $EDITOR
for manual editing.",
        arg_required_else_help = true,
        alias = "e"
    )]
    Edit {
        /// Name of the layout
        #[arg(value_parser = validate_session_name)]
        layout_name: String,
    },
}
