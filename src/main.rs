use crate::{config::Session, muxer::Muxer};
use clap::Parser;
use cli::Cli;
use completions::generate;

mod cli;
mod completions;
mod config;
mod muxer;

fn main() -> anyhow::Result<()> {
    match Cli::parse() {
        Cli::List => {
            for session in Session::list() {
                println!("{session}");
            }
        }
        Cli::New { session_name } => {
            let session_path = Session::create(session_name)?;
            println!(
                "Created new session configuration at: {}",
                session_path.display()
            );
        }
        Cli::Load { session } => {
            let mut runner = Muxer::new();

            let output = runner.apply(&session)?;
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
            let completion = generate(shell)?;
            println!("{}", completion);
        }
    }

    Ok(())
}
