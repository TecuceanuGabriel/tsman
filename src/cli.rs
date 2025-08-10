use clap::{Parser, Subcommand};

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

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(
        about = "Save the current session",
        long_about = "Save the currently attached tmux session. Stores the 
<session_name>.yaml config file in $TSMAN_CONFIG_STORAGE_DIR if set, or 
~/.config/.tsessions."
    )]
    Save {
        /// Name of the session (default: name of current session)
        session_name: Option<String>,
    },

    #[command(
        about = "Open the specified session",
        long_about = "Restore the selected session and then attach to it.",
        arg_required_else_help = true
    )]
    Open {
        /// Name of the session
        session_name: String,
    },

    #[command(
        about = "Edit the specified session",
        long_about = "Open the config file of the specified session in $EDITOR
for manual editing."
    )]
    Edit {
        /// Name of the session (default: name of current session)
        session_name: Option<String>,
    },

    #[command(
        about = "Delete specified session",
        long_about = "Remove the config file of the specified session from the
config storage directory.",
        arg_required_else_help = true
    )]
    Delete {
        /// Name of the session
        session_name: String,
    },

    #[command(
        about = "Open up a menu containing all sessions",
        long_about = "Open up an interactive menu containing all saved or 
currently active sessions."
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
}
