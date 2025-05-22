mod cli;

use std::io;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use cli::Cli;
use tp::{config, muxer::Muxer, tmux_client::TmuxClient};

fn main() -> Result<()> {
    match Cli::parse() {
        Cli::List => list_sessions(),
        Cli::New { name } => new_session(name),
        Cli::Load { session } => load_session(session),
        Cli::Completions { shell } => print_completions(shell),
    }
}

fn list_sessions() -> Result<()> {
    for session in config::list_sessions() {
        println!("{session}");
    }
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

fn print_completions(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    let cmd_name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, cmd_name, &mut io::stdout());
    Ok(())
}
