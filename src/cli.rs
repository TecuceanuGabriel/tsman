//! CLI argument parser
use std::fmt;

use clap::{Parser, Subcommand};
use regex::Regex;

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
}

/// Error type returned when a session name is invalid.
#[derive(Debug)]
struct SessionNameError(String);

impl std::error::Error for SessionNameError {}

impl fmt::Display for SessionNameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validates a session name according to the rules:
///
/// - Must be between 1 and 30 characters long.
/// - Can only contain alphanumeric characters, underscores (`_`),
/// and hyphens (`-`).
///
/// # Errors
///
/// Returns a [`SessionNameError`] if the name is invalid.
///
/// # Examples
/// ```
/// # use tsman::validate_session_name;
/// assert!(validate_session_name("valid_name-123").is_ok());
/// assert!(validate_session_name("invalid name").is_err());
/// ```
fn validate_session_name(name: &str) -> Result<String, SessionNameError> {
    let re = Regex::new(r"^[a-zA-Z0-9_-]{1,30}$").unwrap();
    if !re.is_match(name) {
        Err(SessionNameError(
            "Session name must be 1-30 characters long and only contain [a-zA-Z0-9_-]"
                .into(),
        ))
    } else {
        Ok(name.to_string())
    }
}
