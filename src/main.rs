mod cli;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use tp::{config, muxer::Muxer, tmux_client::TmuxClient};

fn main() -> Result<()> {
    match Cli::parse() {
        Cli::List => list_sessions(),
        Cli::New { name } => new_session(name),
        Cli::Load { session } => load_session(session),
    }
}

fn list_sessions() -> Result<()> {
    Ok(())
}

fn new_session(name: impl Into<String>) -> Result<()> {
    println!("{:?}", name.into());
    Ok(())
}

fn load_session(session: config::Session) -> Result<()> {
    let client: TmuxClient = Default::default();
    let mut runner = Muxer::new(client);

    let _ = runner.apply(session).unwrap();
    Ok(())
}
