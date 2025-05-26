use clap::{Parser, ValueHint};
use clap_complete::Shell;
use tp::config::{Error, Session};

#[derive(Parser, Debug)]
#[command(about = "A simple tmux session loader")]
pub enum Cli {
    /// Create a new session file
    New { session_name: String },
    /// Load a session
    Load {
        #[arg(value_parser = parser_session_config, value_hint = ValueHint::Other)]
        session: Session,
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

fn parser_session_config(value: &str) -> Result<Session, Error> {
    Session::load_from_name(value)
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
