use crate::{config::Session, muxer::Muxer};
use anyhow::Result;
use clap::Parser;
use cli::Cli;
use completions::generate;
use tmux_client::TmuxClient;

mod cli;
mod completions;
mod config;
mod muxer;
mod tmux_client;

fn main() -> Result<()> {
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
            let client: TmuxClient = Default::default();
            let mut runner = Muxer::new(client);

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
        Cli::Completions { shell } => generate(shell)?,
    }

    Ok(())
}
