mod cli;

use std::io;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use cli::Cli;
use tp::{config, muxer};

fn main() -> Result<()> {
    match Cli::parse() {
        Cli::List => {
            for session in config::list_sessions() {
                println!("{session}");
            }
        }
        Cli::New { session_name } => {
            let session_path = config::new_session(session_name)?;
            println!(
                "Created new session configuration at: {}",
                session_path.display()
            );
        }
        Cli::Load { session } => {
            let output = muxer::apply(session)?;
            if output.is_new_session {
                println!("Session {} was created!", output.session_name);
            } else {
                println!(
                    "Session {} already exists! Switching...",
                    output.session_name
                );
            }
        }
        Cli::Completions { shell } => {
            let mut cmd = Cli::command();
            let cmd_name = cmd.get_name().to_string();
            clap_complete::generate(shell, &mut cmd, cmd_name, &mut io::stdout());
        }
    }

    Ok(())
}
