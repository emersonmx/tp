use std::{
    env, fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;

use serde::{Deserialize, Serialize};

#[derive(Error, PartialEq, Debug)]
pub enum Error {
    #[error("unable to load: {0}")]
    UnableToLoad(String),
    #[error("parser error: {0}")]
    UnableToParseConfig(String),
    #[error("invalid sessino directory")]
    InvalidSessionDirectory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    #[serde(default = "default_directory")]
    pub directory: PathBuf,
    #[serde(default)]
    pub windows: Vec<Window>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Window {
    pub name: Option<String>,
    #[serde(default)]
    pub panes: Vec<Pane>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pane {
    #[serde(default)]
    pub focus: bool,
    #[serde(default)]
    pub command: Vec<String>,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::UnableToLoad(err.to_string())
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(err: serde_yaml::Error) -> Self {
        Error::UnableToParseConfig(err.to_string())
    }
}

fn default_directory() -> PathBuf {
    Path::new(".").to_path_buf()
}

pub fn load_session(name: impl Into<String>) -> Result<Session, Error> {
    let name = name.into();
    let dir = sessions_dir().ok_or(Error::InvalidSessionDirectory)?;
    let session_path = dir.join(format!("{}.yaml", name)).canonicalize()?;
    let content = fs::read_to_string(session_path)?;
    let session = from_content(content)?;
    Ok(session)
}

fn from_content(content: impl Into<String>) -> Result<Session, Error> {
    let session = serde_yaml::from_str(&content.into())?;
    Ok(session)
}

fn sessions_dir() -> Option<PathBuf> {
    match env::var("TP_SESSIONS_DIR") {
        Ok(d) => Some(PathBuf::from(d)),
        Err(_) => match env::var("HOME") {
            Ok(e) => Some(PathBuf::from(e).join(".config/tp")),
            Err(_) => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_simple_session_file() {
        let content = r#"name: simple-test"#;

        let session = from_content(content).unwrap();

        assert_eq!(session.name, "simple-test");
        assert_eq!(session.directory, Path::new("."));
    }

    #[test]
    fn read_incorrect_session_file() {
        let content = "parser error";

        let result = from_content(content);

        assert_eq!(
            result,
            Err(Error::UnableToParseConfig(
                "invalid type: string \"parser error\", expected struct Session".to_string()
            ))
        );
    }
}
