use clap::Parser;
use cli::Cli;

mod cli;

fn main() {
    Cli::parse();
}
