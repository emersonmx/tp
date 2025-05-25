use serde::{Deserialize, Serialize};
use std::{env, fs, io, path::PathBuf};
use thiserror::Error;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<PathBuf>,
    #[serde(default = "default_windows")]
    pub windows: Vec<Window>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Window {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<PathBuf>,
    #[serde(default = "default_panes")]
    pub panes: Vec<Pane>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pane {
    #[serde(default)]
    pub focus: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

fn default_windows() -> Vec<Window> {
    vec![Window {
        name: None,
        directory: None,
        panes: default_panes(),
    }]
}

fn default_panes() -> Vec<Pane> {
    vec![Pane::default()]
}

impl Session {
    const DEFAULT_DIR_ENV: &str = "TP_SESSIONS_DIR";
    const HOME_ENV: &str = "HOME";
    const DEFAULT_DIR: &str = ".config/tp";
    const DEFAULT_FILE_EXT: &str = "yaml";

    pub fn load_from_name(name: impl AsRef<str>) -> Result<Self, Error> {
        let dir = Self::default_directory().ok_or(Error::InvalidSessionDirectory)?;
        let path = dir
            .join(format!("{}.{}", name.as_ref(), Self::DEFAULT_FILE_EXT))
            .canonicalize()?;
        let content = fs::read_to_string(path)?;
        let session = Self::load_from_string(&content)?;
        Ok(session)
    }

    fn default_directory() -> Option<PathBuf> {
        env::var(Self::DEFAULT_DIR_ENV)
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                env::var(Self::HOME_ENV)
                    .ok()
                    .map(|home| PathBuf::from(home).join(Self::DEFAULT_DIR))
            })
    }

    pub fn load_from_string(content: impl AsRef<str>) -> Result<Self, Error> {
        let session: Self = serde_yaml::from_str(content.as_ref())?;
        Ok(session)
    }

    pub fn create(name: impl Into<String>) -> Result<PathBuf, Error> {
        let session = Self {
            name: name.into(),
            directory: Some(".".into()),
            windows: vec![Window {
                name: Some("shell".to_string()),
                panes: vec![Pane {
                    focus: true,
                    command: Some("echo 'Hello :)'".to_string()),
                    ..Default::default()
                }],
                ..Default::default()
            }],
        };

        let dir = Self::default_directory().ok_or(Error::InvalidSessionDirectory)?;
        let path = dir.join(format!("{}.{}", session.name, Self::DEFAULT_FILE_EXT));
        let content = serde_yaml::to_string(&session)?;

        fs::write(&path, content)?;

        Ok(path)
    }

    pub fn list() -> Vec<String> {
        let mut sessions: Vec<String> = Self::default_directory()
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
            .collect();
        sessions.sort();
        sessions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn read_simple_session_file() {
        let content = "name: simple-test";
        let session: Session = Session::load_from_string(content).unwrap();

        assert_eq!(session.name, "simple-test");
        assert_eq!(session.directory, None);
    }

    #[test]
    fn read_not_found_session_file() {
        let session = Session::load_from_name("not-found/path");

        assert!(matches!(session, Err(Error::UnableToLoad(_))));
    }

    #[test]
    fn read_incorrect_session_file() {
        let content = "parser error";
        let session: Result<Session, Error> = Session::load_from_string(content);

        assert!(matches!(session, Err(Error::UnableToParseConfig(_))));
    }

    #[test]
    fn load_session_invalid_dir() {
        temp_env::with_vars_unset([Session::HOME_ENV, Session::DEFAULT_DIR_ENV], || {
            let session = Session::load_from_name("a-session-path");

            assert!(matches!(session, Err(Error::InvalidSessionDirectory)));
        });
    }

    #[test]
    fn session_must_have_one_window_with_one_pane() {
        let content = "name: simple-test";
        let session: Session = Session::load_from_string(content).unwrap();

        assert_eq!(session.windows.len(), 1);
        assert_eq!(session.windows[0].panes.len(), 1);
        assert_eq!(session.windows[0].name, None);
        assert!(!session.windows[0].panes[0].focus);
        assert_eq!(session.windows[0].panes[0].command, None);
    }

    #[test]
    fn window_must_have_one_pane() {
        let content = "
        name: simple-test
        windows:
          -
        ";
        let session: Session = Session::load_from_string(content).unwrap();

        assert_eq!(session.windows.len(), 1);
        assert_eq!(session.windows[0].panes.len(), 1);
        assert_eq!(session.windows[0].name, None);
        assert!(!session.windows[0].panes[0].focus);
        assert_eq!(session.windows[0].panes[0].command, None);
    }

    #[test]
    fn list_all_sessions() {
        let temp_test_dir = tempdir().expect("Failed to create temporary directory");
        let tmp_dir = temp_test_dir.path();
        temp_env::with_var(
            Session::DEFAULT_DIR_ENV,
            Some(tmp_dir.to_str().unwrap()),
            || {
                fs::write(
                    tmp_dir.join(format!("session1.{}", Session::DEFAULT_FILE_EXT)),
                    "name: session1",
                )
                .unwrap();
                fs::write(
                    tmp_dir.join(format!("session2.{}", Session::DEFAULT_FILE_EXT)),
                    "name: session2",
                )
                .unwrap();
                fs::write(tmp_dir.join("other_file.txt"), "content").unwrap();
                fs::create_dir(tmp_dir.join("subdir")).unwrap();

                let mut sessions = Session::list();
                sessions.sort();

                assert_eq!(
                    sessions,
                    vec!["session1".to_string(), "session2".to_string()]
                );
            },
        );
    }

    #[test]
    fn list_sessions_when_empty_dir() {
        let temp_test_dir = tempdir().expect("Failed to create temporary directory");
        let tmp_dir = temp_test_dir.path();
        temp_env::with_var(
            Session::DEFAULT_DIR_ENV,
            Some(tmp_dir.to_str().unwrap()),
            || {
                let sessions = Session::list();
                assert!(sessions.is_empty());
            },
        );
    }

    #[test]
    fn list_sessions_when_sessions_dir_not_exists() {
        temp_env::with_vars_unset([Session::HOME_ENV, Session::DEFAULT_DIR_ENV], || {
            let sessions = Session::list();
            assert!(sessions.is_empty());
        });
    }

    #[test]
    fn list_sessions_with_read_error() {
        let temp_file =
            tempfile::NamedTempFile::new().expect("Failed to create temporary directory");
        let tmp_dir = temp_file.path();
        temp_env::with_var(
            Session::DEFAULT_DIR_ENV,
            Some(tmp_dir.to_str().unwrap()),
            || {
                fs::write(tmp_dir, "this is a file").unwrap();

                let sessions = Session::list();
                assert!(sessions.is_empty());
            },
        );
    }

    #[test]
    fn when_new_session_success() {
        let session_name = "new-test-session";
        let temp_test_dir = tempdir().expect("Failed to create temporary directory");
        let tmp_dir = temp_test_dir.path();

        temp_env::with_var(
            Session::DEFAULT_DIR_ENV,
            Some(tmp_dir.to_str().unwrap()),
            || {
                let result = Session::create(session_name);
                assert!(result.is_ok());

                let created_path = result.unwrap();
                let expected_path =
                    tmp_dir.join(format!("{}.{}", session_name, Session::DEFAULT_FILE_EXT));
                assert_eq!(created_path, expected_path);
                assert!(created_path.exists());

                let content =
                    fs::read_to_string(&created_path).expect("Failed to read created file");
                let session: Session = Session::load_from_string(&content)
                    .expect("Failed to deserialize created session");

                assert_eq!(session.name, session_name);
                assert_eq!(session.directory, Some(".".into()));
                assert_eq!(session.windows.len(), 1);
                assert_eq!(session.windows[0].name, Some("shell".to_string()));
                assert_eq!(session.windows[0].panes.len(), 1);
                assert!(session.windows[0].panes[0].focus);
                assert_eq!(
                    session.windows[0].panes[0].command,
                    Some("echo 'Hello :)'".to_string())
                );
            },
        );
    }

    #[test]
    fn when_new_session_invalid_dir() {
        temp_env::with_vars_unset([Session::HOME_ENV, Session::DEFAULT_DIR_ENV], || {
            let result = Session::create("some-session");
            assert!(matches!(result, Err(Error::InvalidSessionDirectory)));
        });
    }
}
