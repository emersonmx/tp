use crate::config::Session;
#[cfg(test)]
use mockall::automock;
use std::{
    env,
    fmt::Display,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Id(String);

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionId(Id);

impl SessionId {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self(Id(session_id.into()))
    }

    pub fn id(&self) -> &Id {
        &self.0
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowID(SessionId, Id);

impl WindowID {
    pub fn new(session_id: &SessionId, window_id: impl Into<String>) -> Self {
        let window_id = Id(format!("{}:{}", session_id, window_id.into()));
        Self(session_id.clone(), window_id)
    }

    pub fn session_id(&self) -> &Id {
        self.0.id()
    }

    pub fn id(&self) -> &Id {
        &self.1
    }
}

impl Display for WindowID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaneID(WindowID, Id);

impl PaneID {
    pub fn new(window_id: &WindowID, pane_id: impl Into<String>) -> Self {
        let pane_id = Id(format!("{}.{}", window_id.clone(), pane_id.into()));
        Self(window_id.clone(), pane_id)
    }

    pub fn session_id(&self) -> &Id {
        self.0.session_id()
    }

    pub fn window_id(&self) -> &Id {
        self.0.id()
    }

    pub fn id(&self) -> &Id {
        &self.1
    }
}

impl Display for PaneID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowName(String);

impl WindowName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionName(String);

impl OptionName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionValue(String);

impl OptionValue {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layout(String);

#[derive(Debug, Clone, PartialEq)]
pub struct Keys(String);

impl Keys {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Error, PartialEq, Debug)]
pub enum Error {
    #[error("unable to setup base ids: {0}")]
    BaseIdsError(String),
    #[error("option `{0}` not found")]
    OptionNotFound(String),
}

#[allow(dead_code)]
#[cfg_attr(test, automock)]
pub trait Client {
    fn get_option(&mut self, option_name: &OptionName) -> Result<OptionValue, Error>;
    fn set_option(&mut self, option_name: &OptionName, option_value: &OptionValue);

    fn new_session(&mut self, session_id: &SessionId, directory: &str);
    fn switch_to_session(&mut self, session_id: &SessionId);
    fn has_session(&mut self, session_id: &SessionId) -> bool;

    fn new_window(&mut self, session_id: &SessionId, directory: &str);
    fn rename_window(&mut self, window_id: &WindowID, window_name: &WindowName);

    fn new_pane(&mut self, window_id: &WindowID, directory: &str);
    fn select_pane(&mut self, pane_id: &PaneID);

    fn send_keys(&mut self, pane_id: &PaneID, keys: Keys);

    fn use_layout(&mut self, layout: &Layout);
}

pub struct Output {
    pub session_name: String,
    pub is_new_session: bool,
    pub windows: Vec<(usize, Vec<usize>)>,
}

pub struct Muxer<C: Client> {
    client: C,
    base_window_id: usize,
    base_pane_id: usize,
}

fn directory_to_string(directory: Option<PathBuf>) -> String {
    directory
        .map(expand_tilde)
        .and_then(|dir| dir.to_str().map(|s| s.to_owned()))
        .unwrap_or_else(|| ".".to_owned())
}

fn expand_tilde(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    path.strip_prefix("~/")
        .ok()
        .and_then(|suffix| {
            env::var("HOME")
                .ok()
                .map(|home_str| PathBuf::from(home_str).join(suffix))
        })
        .unwrap_or_else(|| path.to_owned())
}

fn resolve_directory(
    session_dir: &Option<PathBuf>,
    window_dir: &Option<PathBuf>,
    pane_dir: &Option<PathBuf>,
) -> Option<PathBuf> {
    pane_dir
        .clone()
        .or_else(|| window_dir.clone())
        .or_else(|| session_dir.clone())
}

impl<C: Client> Muxer<C> {
    pub fn new(client: C) -> Self {
        Self {
            client,
            base_window_id: 0,
            base_pane_id: 0,
        }
    }

    pub fn apply(&mut self, session: &Session) -> Result<Output, Error> {
        let session_id = SessionId::new(&session.name);
        let mut windows = vec![];
        if self.client.has_session(&session_id) {
            self.client.switch_to_session(&session_id);
            return Ok(Output {
                session_name: session.name.clone(),
                is_new_session: false,
                windows,
            });
        }

        self.setup_base_ids()?;

        let first_window = session.windows.first();
        let initial_dir = resolve_directory(
            &session.directory,
            &first_window.and_then(|window| window.directory.clone()),
            &first_window
                .and_then(|window| window.panes.first().and_then(|pane| pane.directory.clone())),
        );
        let initial_dir = directory_to_string(initial_dir);
        self.client.new_session(&session_id, &initial_dir);

        let session_dir = session.directory.clone();
        let mut focus_pane: Option<PaneID> = None;
        for (wid, window) in session.windows.iter().enumerate() {
            let window_dir = resolve_directory(&session_dir, &window.directory, &None);
            if wid > 0 {
                let initial_dir = resolve_directory(
                    &session_dir,
                    &window_dir,
                    &window.panes.first().and_then(|pane| pane.directory.clone()),
                );
                self.client
                    .new_window(&session_id, &directory_to_string(initial_dir));
            }

            let widx = self.base_window_id + wid;
            let window_id = WindowID::new(&session_id, widx.to_string());
            if let Some(window_name) = &window.name {
                self.client
                    .rename_window(&window_id, &WindowName::new(window_name));
            }

            let mut panes: Vec<usize> = vec![];
            for (pid, pane) in window.panes.iter().enumerate() {
                let pidx = self.base_pane_id + pid;
                let pane_id = PaneID::new(&window_id, pidx.to_string());
                if pane.focus {
                    focus_pane = Some(pane_id.clone());
                }

                let pane_dir = resolve_directory(&session_dir, &window_dir, &pane.directory);
                if pid > 0 {
                    self.client
                        .new_pane(&window_id, &directory_to_string(pane_dir));
                }

                if let Some(cmd) = &pane.command {
                    self.client.send_keys(&pane_id, Keys::new(cmd));
                }

                panes.push(pidx);
            }

            windows.push((widx, panes));
        }

        if let Some(pane) = focus_pane {
            self.client.select_pane(&pane);
        }

        self.client.switch_to_session(&session_id);

        Ok(Output {
            session_name: session.name.clone(),
            is_new_session: true,
            windows,
        })
    }

    fn setup_base_ids(&mut self) -> Result<(), Error> {
        self.base_window_id = self.get_index("base-index")?;
        self.base_pane_id = self.get_index("pane-base-index")?;
        Ok(())
    }

    fn get_index(&mut self, name: &str) -> Result<usize, Error> {
        let value = self
            .client
            .get_option(&OptionName::new(name))
            .map_err(|e| Error::BaseIdsError(e.to_string()))?
            .value()
            .parse()
            .map_err(|e| Error::BaseIdsError(format!("{}", e)))?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mock_client() -> MockClient {
        let mut mock_client = MockClient::new();
        mock_client.expect_has_session().return_const(false);
        mock_client.expect_new_session().return_const(());
        mock_client.expect_switch_to_session().return_const(());
        mock_client
            .expect_get_option()
            .returning(|_| Ok(OptionValue::new("0")));
        mock_client.expect_send_keys().return_const(());
        mock_client
    }

    #[test]
    fn switch_to_session_if_exists() {
        let session: Session = serde_yaml::from_str("name: test").unwrap();
        let mut mock_client = MockClient::new();
        mock_client.expect_has_session().return_const(true);
        mock_client.expect_switch_to_session().return_const(());
        let mut runner = Muxer::new(mock_client);

        let output = runner.apply(&session).unwrap();

        assert_eq!(output.session_name, "test".to_string());
        assert!(!output.is_new_session);
    }

    #[test]
    fn create_a_session_if_not_exists() {
        let session: Session = serde_yaml::from_str("name: test").unwrap();
        let mock_client = make_mock_client();
        let mut runner = Muxer::new(mock_client);

        let output = runner.apply(&session).unwrap();

        assert_eq!(output.session_name, "test".to_string());
        assert!(output.is_new_session);
    }

    #[test]
    fn base_ids_starts_at_zero() {
        let session: Session = serde_yaml::from_str("name: test").unwrap();
        let mock_client = make_mock_client();
        let mut runner = Muxer::new(mock_client);

        let output = runner.apply(&session).unwrap();

        assert_eq!(output.windows, vec![(0, vec![0])]);
    }
}
