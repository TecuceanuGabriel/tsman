use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "tsman")]
#[command(about = "A session manager for tmux", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// create new session file in <SESSIONS> directory and open it up in
    /// $EDITOR, attach a client to the session when done editing.
    #[command(arg_required_else_help = true)]
    New { session_name: String },

    /// save the current session
    #[command(arg_required_else_help = true)]
    Save { session_name: String },

    /// open a session
    #[command(arg_required_else_help = true)]
    Open { session_name: String },

    /// open the config file of <SESSION_NAME> in $EDITOR
    #[command(arg_required_else_help = true)]
    Edit { session_name: String },

    /// display menu containing all sessions
    Menu,
}
