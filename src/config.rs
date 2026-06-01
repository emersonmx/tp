#[cfg(test)]
use mockall::automock;
use serde::{Deserialize, Serialize};
use std::{env, fs, io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid session directory")]
    InvalidSessionDirectory,
    #[error("file '{file}' not found")]
    FileNotFound {
        file: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("unable to read file '{file}'")]
    FileUnreadable {
        file: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("unable to write session file '{file}'")]
    WriteFailed {
        file: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("parser error")]
    UnableToParseConfig(#[source] serde_yaml::Error),
    #[error("unable to serialize session")]
    SerializationFailed {
        file: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
}

#[cfg_attr(test, automock)]
trait HomeDirProvider: Send + Sync {
    fn home_dir(&self) -> Option<PathBuf>;
}

struct EnvHomeDirProvider;

impl HomeDirProvider for EnvHomeDirProvider {
    fn home_dir(&self) -> Option<PathBuf> {
        env::home_dir()
    }
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
    const DEFAULT_DIR: &str = ".config/tp";
    const DEFAULT_FILE_EXT: &str = "yaml";
    const DEFAULT_HOME_DIR_PROVIDER: EnvHomeDirProvider = EnvHomeDirProvider;

    pub fn load_from_name(name: impl AsRef<str>) -> Result<Self, Error> {
        let session_path = Self::get_session_path(name.as_ref())?;
        let content = fs::read_to_string(&session_path).map_err(|e| Error::FileUnreadable {
            file: session_path,
            source: e,
        })?;
        let session = Self::load_from_string(&content)?;
        Ok(session)
    }

    fn get_session_path(name: &str) -> Result<PathBuf, Error> {
        let dir = Self::default_directory_with_provider(Self::DEFAULT_HOME_DIR_PROVIDER)
            .ok_or(Error::InvalidSessionDirectory)?;
        let path = dir
            .join(format!("{}.{}", name, Self::DEFAULT_FILE_EXT))
            .canonicalize()
            .map_err(|e| Error::FileNotFound {
                file: dir.join(format!("{}.{}", name, Self::DEFAULT_FILE_EXT)),
                source: e,
            })?;
        Ok(path)
    }

    fn default_directory_with_provider(provider: impl HomeDirProvider) -> Option<PathBuf> {
        env::var(Self::DEFAULT_DIR_ENV)
            .ok()
            .map(PathBuf::from)
            .or_else(|| provider.home_dir().map(|home| home.join(Self::DEFAULT_DIR)))
    }

    pub fn load_from_string(content: impl AsRef<str>) -> Result<Self, Error> {
        let session: Self =
            serde_yaml::from_str(content.as_ref()).map_err(Error::UnableToParseConfig)?;
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

        let dir = Self::default_directory_with_provider(Self::DEFAULT_HOME_DIR_PROVIDER)
            .ok_or(Error::InvalidSessionDirectory)?;
        let session_path = dir.join(format!("{}.{}", session.name, Self::DEFAULT_FILE_EXT));

        let content = serde_yaml::to_string(&session).map_err(|e| Error::SerializationFailed {
            file: session_path.clone(),
            source: e,
        })?;

        fs::write(&session_path, content).map_err(|e| Error::WriteFailed {
            file: session_path.clone(),
            source: e,
        })?;

        Ok(session_path)
    }

    pub fn list() -> Vec<String> {
        let mut sessions: Vec<String> =
            Self::default_directory_with_provider(Session::DEFAULT_HOME_DIR_PROVIDER)
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
    use rstest::rstest;
    use std::{
        fs::{File, Permissions},
        os::unix::fs::PermissionsExt,
    };
    use tempfile::tempdir;

    #[rstest]
    fn load_from_name_file_unreadable() {
        let session_name = "unreadable-session";
        let temp_test_dir = tempdir().expect("Failed to create temporary directory");
        let tmp_dir = temp_test_dir.path();

        temp_env::with_var(
            Session::DEFAULT_DIR_ENV,
            Some(tmp_dir.to_str().unwrap()),
            || {
                let file_path = tmp_dir.join(format!("{}.yaml", session_name));
                File::create(&file_path).unwrap();
                fs::set_permissions(&file_path, Permissions::from_mode(0o000)).unwrap();

                let result = Session::load_from_name(session_name).unwrap_err();

                assert!(matches!(result, Error::FileUnreadable { .. }));
            },
        );
    }

    #[rstest]
    fn should_load_from_name() {
        let session_name = "test-session";
        let temp_test_dir = tempdir().expect("Failed to create temporary directory");
        let tmp_dir = temp_test_dir.path();

        temp_env::with_var(
            Session::DEFAULT_DIR_ENV,
            Some(tmp_dir.to_str().unwrap()),
            || {
                let file_path = tmp_dir.join(format!("{}.yaml", session_name));
                fs::write(&file_path, "name: test-session").unwrap();

                let result = Session::load_from_name(session_name).unwrap();
                assert_eq!(result.name, session_name);
            },
        );
    }

    #[rstest]
    fn get_session_path_file_not_found() {
        temp_env::with_var_unset("HOME", || {
            let session = Session::get_session_path("a-session-path");

            assert!(matches!(session, Err(Error::FileNotFound { .. })));
        });
    }

    #[rstest]
    fn default_directory_when_no_envs() {
        let mut mock = MockHomeDirProvider::new();
        mock.expect_home_dir().return_const(None);

        temp_env::with_var_unset(Session::DEFAULT_DIR_ENV, || {
            let dir = Session::default_directory_with_provider(mock);
            assert!(dir.is_none());
        });
    }

    #[rstest]
    fn default_directory_from_env() {
        temp_env::with_var(Session::DEFAULT_DIR_ENV, Some("/custom/dir"), || {
            let dir = Session::default_directory_with_provider(Session::DEFAULT_HOME_DIR_PROVIDER);
            assert_eq!(dir, Some(PathBuf::from("/custom/dir")));
        });
    }

    #[rstest]
    fn load_from_string_parser_error() {
        let content = "parser error";
        let session = Session::load_from_string(content).unwrap_err();

        assert!(matches!(session, Error::UnableToParseConfig(_)));
    }

    #[rstest]
    fn should_load_from_string() {
        let content = "name: simple-test";
        let session: Session = Session::load_from_string(content).unwrap();

        assert_eq!(session.name, "simple-test");
        assert_eq!(session.directory, None);
    }

    #[rstest]
    fn session_must_have_one_window_with_one_pane() {
        let content = "name: simple-test";
        let session: Session = Session::load_from_string(content).unwrap();

        assert_eq!(session.windows.len(), 1);
        assert_eq!(session.windows[0].panes.len(), 1);
        assert_eq!(session.windows[0].name, None);
        assert!(!session.windows[0].panes[0].focus);
        assert_eq!(session.windows[0].panes[0].command, None);
    }

    #[rstest]
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

    #[rstest]
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

    #[rstest]
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

    #[rstest]
    fn list_sessions_when_sessions_dir_not_exists() {
        temp_env::with_var(Session::DEFAULT_DIR_ENV, Some("invalid-dir"), || {
            let sessions = Session::list();
            assert!(sessions.is_empty());
        });
    }

    #[rstest]
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

    #[rstest]
    fn when_new_session_success() {
        let session_name = "new-test-session";
        let temp_test_dir = tempdir().expect("Failed to create temporary directory");
        let tmp_dir = temp_test_dir.path();

        temp_env::with_var(
            Session::DEFAULT_DIR_ENV,
            Some(tmp_dir.to_str().unwrap()),
            || {
                let result = Session::create(session_name);
                dbg!(&result);
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
}
