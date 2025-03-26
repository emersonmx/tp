use anyhow::Result;
use clap::Parser;
use cli::Cli;

mod cli;
mod config;

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
    println!("{:?}", session);
    Ok(())
}
