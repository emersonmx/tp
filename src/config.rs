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

pub fn list_sessions() -> Vec<String> {
    sessions_dir()
        .and_then(|dir| fs::read_dir(dir).ok())
        .into_iter()
        .flatten()
        .filter_map(|entry_result| entry_result.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
        .filter_map(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(|s| s.to_string())
        })
        .collect()
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

    #[test]
    fn list_all_sessions() {
        temp_env::with_var(
            "TP_SESSIONS_DIR",
            Some("/tmp/test_list_all_sessions".to_string()),
            || {
                let tmp_dir = PathBuf::from("/tmp/test_list_all_sessions");
                fs::create_dir_all(&tmp_dir).expect("Failed to create test directory");

                fs::write(tmp_dir.join("session1.yaml"), "name: session1").unwrap();
                fs::write(tmp_dir.join("session2.yaml"), "name: session2").unwrap();
                fs::write(tmp_dir.join("other_file.txt"), "content").unwrap();
                fs::create_dir(tmp_dir.join("subdir")).unwrap();

                let mut sessions = list_sessions();
                sessions.sort();

                assert_eq!(
                    sessions,
                    vec!["session1".to_string(), "session2".to_string()]
                );

                fs::remove_dir_all(&tmp_dir).expect("Failed to clean up test directory");
            },
        );
    }

    #[test]
    fn list_sessions_when_empty_dir() {
        temp_env::with_var(
            "TP_SESSIONS_DIR",
            Some("/tmp/list_sessions_when_empty_dir".to_string()),
            || {
                let tmp_dir = PathBuf::from("/tmp/list_sessions_when_empty_dir");
                fs::create_dir_all(&tmp_dir).expect("Failed to create test directory");

                let sessions = list_sessions();
                assert!(sessions.is_empty());

                fs::remove_dir_all(&tmp_dir).expect("Failed to clean up test directory");
            },
        );
    }

    #[test]
    fn list_sessions_when_sessions_dir_not_exists() {
        temp_env::with_vars_unset(["HOME", "TP_SESSIONS_DIR"], || {
            let sessions = list_sessions();
            assert!(sessions.is_empty());
        });
    }

    #[test]
    fn list_sessions_with_read_error() {
        temp_env::with_var(
            "TP_SESSIONS_DIR",
            Some("/tmp/not_a_dir_file".to_string()),
            || {
                fs::write("/tmp/not_a_dir_file", "this is a file").unwrap();

                let sessions = list_sessions();
                assert!(sessions.is_empty());

                fs::remove_file("/tmp/not_a_dir_file").unwrap();
            },
        );
    }
}
