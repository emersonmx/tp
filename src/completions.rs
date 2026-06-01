use crate::cli::Cli;
use clap::CommandFactory;
use clap_complete::Shell;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid utf8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("replacement `{0}` not found")]
    ReplacementNotFound(String),
}

// Adapted from `just`
pub fn generate(shell: Shell) -> Result<String, Error> {
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

    Ok(content)
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
    use insta::assert_snapshot;
    use rstest::rstest;

    #[rstest]
    fn should_generate_bash_completions() {
        let result = generate(Shell::Bash);
        assert_snapshot!(result.unwrap());
    }

    #[rstest]
    fn should_generate_zsh_completions() {
        let result = generate(Shell::Zsh);
        assert_snapshot!(result.unwrap());
    }

    #[rstest]
    fn should_replace_needle_in_haystack() {
        let mut haystack = "foo bar baz".to_string();
        replace(&mut haystack, "bar", "qux").unwrap();
        assert_eq!(haystack, "foo qux baz");
    }

    #[rstest]
    fn raise_error_if_replacement_not_found() {
        let mut haystack = "foo bar baz".to_string();
        let result = replace(&mut haystack, "qux", "quux");
        assert!(matches!(result, Err(Error::ReplacementNotFound(_))));
    }
}
