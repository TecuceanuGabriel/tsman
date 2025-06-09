use crate::cli::{Args, Commands};

pub fn handle(args: Args) {
    match args.command {
        Commands::New { session_name } => todo!(),
        Commands::Save { session_name } => {
            println!("saving session {session_name}");
        }
        Commands::Open { session_name } => todo!(),
        Commands::Edit { session_name } => todo!(),
        Commands::Menu => todo!(),
    }
}
