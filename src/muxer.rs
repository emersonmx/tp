use std::{
    env,
    path::{Path, PathBuf},
};

use crate::{
    config::Session,
    tmux_client::{Client, Keys, OptionName, PaneID, SessionId, TmuxClient, WindowID, WindowName},
};
use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum Error {
    #[error("unable to setup base ids: {0}")]
    BaseIdsError(String),
}

pub struct Output {
    pub session_name: String,
    pub is_new_session: bool,
    pub windows: Vec<(usize, Vec<usize>)>,
}

struct Muxer<C: Client> {
    client: C,
    base_window_id: usize,
    base_pane_id: usize,
}

fn directory_to_string(directory: Option<PathBuf>) -> String {
    directory
        .map(|dir| expand_tilde(&dir))
        .as_ref()
        .and_then(|dir| dir.to_str())
        .unwrap_or(".")
        .to_owned()
}

fn expand_tilde(path: &Path) -> PathBuf {
    path.strip_prefix("~/")
        .ok()
        .and_then(|suffix| {
            env::var("HOME")
                .ok()
                .map(|home_str| PathBuf::from(home_str).join(suffix))
        })
        .unwrap_or(path.to_owned())
}

impl<C: Client> Muxer<C> {
    fn new(client: C) -> Self {
        Self {
            client,
            base_window_id: 0,
            base_pane_id: 0,
        }
    }

    fn apply(&mut self, session: &Session) -> Result<Output, Error> {
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

        let mut initial_directory = session.directory.clone();
        if let Some(window) = session.windows.first() {
            initial_directory = window
                .directory
                .clone()
                .or_else(|| initial_directory.clone());
            if let Some(pane) = window.panes.first() {
                initial_directory = pane.directory.clone().or_else(|| initial_directory.clone())
            }
        }
        self.client
            .new_session(&session_id, &directory_to_string(initial_directory));

        let mut focus_pane: Option<PaneID> = None;
        let base_dir = session.directory.clone();
        for (wid, window) in session.windows.iter().enumerate() {
            let widx = self.base_window_id + wid;
            let window_id = WindowID::new(&session_id, widx.to_string());
            let base_dir = window.directory.clone().or_else(|| base_dir.clone());
            if wid > 0 {
                let mut base_dir = base_dir.clone();
                if let Some(pane) = window.panes.first() {
                    base_dir = pane.directory.clone().or_else(|| base_dir.clone());
                }
                self.client
                    .new_window(&session_id, &directory_to_string(base_dir.clone()));
            }

            if let Some(window_name) = &window.name {
                self.client
                    .rename_window(&window_id, &WindowName::new(window_name));
            }

            let mut panes: Vec<usize> = vec![];
            for (pid, pane) in window.panes.iter().enumerate() {
                let pidx = self.base_pane_id + pid;
                let pane_id = PaneID::new(&window_id, pidx.to_string());
                let base_dir = pane.directory.clone().or_else(|| base_dir.clone());
                if pane.focus {
                    focus_pane = Some(pane_id.clone());
                }

                if pid > 0 {
                    self.client
                        .new_pane(&window_id, &directory_to_string(base_dir.clone()));
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

pub fn apply(session: &Session) -> Result<Output, Error> {
    let client: TmuxClient = Default::default();
    let mut runner = Muxer::new(client);
    runner.apply(session)
}

#[cfg(test)]
mod tests {
    use mockall::mock;

    use super::*;
    use crate::tmux_client::*;

    mock! {
        Client {}
        impl Client for Client {
            fn get_option(&mut self, option_name: &OptionName) -> Result<OptionValue, crate::tmux_client::Error>;
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
    }

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
