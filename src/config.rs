use std::{
    env::{self},
    fs, io,
    path::PathBuf,
    vec,
};
use thiserror::Error;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Error, Debug)]
pub enum Error {
    #[error("unable to load: {0}")]
    UnableToLoad(#[from] io::Error),
    #[error("parser error: {0}")]
    UnableToParseConfig(#[from] serde_yaml::Error),
    #[error("invalid session directory")]
    InvalidSessionDirectory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Window {
    pub name: Option<String>,
    #[serde(default = "default_panes")]
    pub panes: Vec<Pane>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pane {
    #[serde(default)]
    pub focus: bool,
    #[serde(default)]
    pub command: String,
}

fn default_directory() -> PathBuf {
    ".".into()
}

fn default_windows() -> Vec<Window> {
    vec![Window {
        name: None,
        panes: default_panes(),
    }]
}

fn default_panes() -> Vec<Pane> {
    vec![Pane::default()]
}

fn deserialize_directory<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let expanded = expand_tilde(&value);
    Ok(PathBuf::from(expanded))
}

fn expand_tilde(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = env::var("HOME") {
            return format!("{}/{}", home.trim_end_matches('/'), stripped);
        }
    }
    path.to_string()
}

pub fn load_session(name: impl AsRef<str>) -> Result<Session, Error> {
    let dir = sessions_dir().ok_or(Error::InvalidSessionDirectory)?;
    let path = dir.join(format!("{}.yaml", name.as_ref())).canonicalize()?;
    let content = fs::read_to_string(path)?;
    let session = serde_yaml::from_str(&content)?;
    Ok(session)
}

fn sessions_dir() -> Option<PathBuf> {
    env::var("TP_SESSIONS_DIR")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join(".config/tp"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_simple_session_file() {
        let content = "name: simple-test";
        let session: Session = serde_yaml::from_str(content).unwrap();

        assert_eq!(session.name, "simple-test");
        assert_eq!(session.directory, PathBuf::from("."));
    }

    #[test]
    fn read_not_found_session_file() {
        let session = load_session("not-found/path");

        assert!(matches!(session, Err(Error::UnableToLoad(_))));
    }

    #[test]
    fn read_incorrect_session_file() {
        let content = "parser error";
        let session: Result<Session, Error> = serde_yaml::from_str(content).map_err(Error::from);

        assert!(matches!(session, Err(Error::UnableToParseConfig(_))));
    }

    #[test]
    fn load_session_invalid_dir() {
        temp_env::with_vars_unset(["HOME", "TP_SESSIONS_DIR"], || {
            let session = load_session("a-session-path");

            assert!(matches!(session, Err(Error::InvalidSessionDirectory)));
        });
    }

    #[test]
    fn session_must_have_one_window_with_one_pane() {
        let content = "name: simple-test";
        let session: Session = serde_yaml::from_str(content).unwrap();

        assert_eq!(session.windows.len(), 1);
        assert_eq!(session.windows[0].panes.len(), 1);
        assert_eq!(session.windows[0].name, None);
        assert!(!session.windows[0].panes[0].focus);
        assert_eq!(session.windows[0].panes[0].command, String::new());
    }

    #[test]
    fn window_must_have_one_pane() {
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
