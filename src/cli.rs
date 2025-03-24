use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "A simple tmux project loader")]
pub enum Cli {
    /// Load a project
    Load { project: String },
    /// List projects
    List,
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
