use std::{
    env::{self},
    fs, io,
    path::{Path, PathBuf},
    vec,
};
use thiserror::Error;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Error, PartialEq, Debug)]
pub enum Error {
    #[error("unable to load: {0}")]
    UnableToLoad(String),
    #[error("parser error: {0}")]
    UnableToParseConfig(String),
    #[error("invalid session directory")]
    InvalidSessionDirectory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    #[serde(
        default = "default_directory",
        deserialize_with = "deserialize_directory"
    )]
    pub directory: PathBuf,
    #[serde(default = "default_windows")]
    pub windows: Vec<Window>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Window {
    pub name: Option<String>,
    #[serde(default = "default_panes")]
    pub panes: Vec<Pane>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pane {
    #[serde(default)]
    pub focus: bool,
    #[serde(default)]
    pub command: String,
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

fn default_windows() -> Vec<Window> {
    vec![Window {
        name: None,
        panes: default_panes(),
    }]
}

fn default_panes() -> Vec<Pane> {
    vec![Pane {
        focus: false,
        command: String::new(),
    }]
}

fn deserialize_directory<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let value: serde_yaml::Value = Deserialize::deserialize(deserializer)?;
    let str_value = match serde_yaml::to_string(&value) {
        Ok(s) => match env::var("HOME") {
            Ok(e) => s.trim().replace("~", &e),
            _ => ".".to_string(),
        },
        _ => ".".to_string(),
    };
    let path = Path::new(&str_value).to_path_buf();
    Ok(path)
}

pub fn load_session(name: impl Into<String>) -> Result<Session, Error> {
    let name = name.into();
    let dir = sessions_dir().ok_or(Error::InvalidSessionDirectory)?;
    let session_path = dir.join(format!("{}.yaml", name)).canonicalize()?;
    let content = fs::read_to_string(session_path)?;
    let session = serde_yaml::from_str(&content)?;
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
        let content = "name: simple-test";
        let session: Session = serde_yaml::from_str(content).unwrap();

        assert_eq!(session.name, "simple-test");
        assert_eq!(session.directory, Path::new("."));
    }

    #[test]
    fn read_incorrect_session_file() {
        let content = "parser error";
        let session: Result<Session, Error> =
            serde_yaml::from_str(content).map_err(|e| Error::UnableToParseConfig(e.to_string()));

        assert_eq!(
            session,
            Err(Error::UnableToParseConfig(
                "invalid type: string \"parser error\", expected struct Session".to_string()
            ))
        );
    }

    #[test]
    fn session_must_have_at_least_one_window_with_one_pane() {
        let content = "name: simple-test";
        let session: Session = serde_yaml::from_str(content).unwrap();

        assert_eq!(session.windows.len(), 1);
        assert_eq!(session.windows[0].panes.len(), 1);
        assert_eq!(session.windows[0].name, None);
        assert!(!session.windows[0].panes[0].focus);
        assert_eq!(session.windows[0].panes[0].command, String::new());
    }

    #[test]
    fn window_must_have_at_least_one_pane() {
        let content = "
        name: simple-test
        windows:
          -
        ";
        let session: Session = serde_yaml::from_str(content).unwrap();

        assert_eq!(session.windows.len(), 1);
        assert_eq!(session.windows[0].panes.len(), 1);
        assert_eq!(session.windows[0].name, None);
        assert!(!session.windows[0].panes[0].focus);
        assert_eq!(session.windows[0].panes[0].command, String::new());
    }
}
