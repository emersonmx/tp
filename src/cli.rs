use crate::config;
use clap::Parser;
use clap_complete::Shell;

#[derive(Parser, Debug)]
#[command(about = "A simple tmux session loader")]
pub enum Cli {
    /// Create a new session file
    New { name: String },
    /// Load a session
    Load {
        #[arg(value_parser = parser_session_config)]
        session: config::Session,
    },
    /// List sessions
    List,
    /// Generate shel completions
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn parser_session_config(value: &str) -> Result<config::Session, config::Error> {
    config::load_session(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
