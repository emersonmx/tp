use crate::cli::Cli;
use clap::CommandFactory;
use clap_complete::Shell;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid utf8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("replacement `{0}` not found")]
    ReplacementNotFound(String),
}

// Adapted from `just`
pub fn generate(shell: Shell) -> Result<(), Error> {
    let mut cmd = Cli::command();
    let cmd_name = cmd.get_name().to_string();
    let mut buf: Vec<u8> = Vec::new();
    clap_complete::generate(shell, &mut cmd, cmd_name, &mut buf);

    let mut content = String::from_utf8(buf)?;
    if shell == Shell::Zsh {
        for (needle, replacement) in ZSH_COMPLETION_REPLACEMENTS {
            replace(&mut content, needle, replacement)?;
        }
    };

    println!("{content}");
    Ok(())
}

fn replace(haystack: &mut String, needle: &str, replacement: &str) -> Result<(), Error> {
    if let Some(index) = haystack.find(needle) {
        haystack.replace_range(index..index + needle.len(), replacement);
        Ok(())
    } else {
        Err(Error::ReplacementNotFound(needle.to_string()))
    }
}

const ZSH_COMPLETION_REPLACEMENTS: &[(&str, &str)] = &[(
    r#"(load)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':session:_default' \"#,
    r#"(load)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':session:($(tp list))' \"#,
)];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_generate_zsh_completions() {
        generate(Shell::Zsh).unwrap();
    }
}
