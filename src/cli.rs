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
    /// save the current session
    Save { session_name: Option<String> },

    /// open the selected session
    #[command(arg_required_else_help = true)]
    Open { session_name: String },

    /// open the config file of the selected session in $EDITOR
    Edit,

    /// display menu containing all sessions
    Menu,
}
