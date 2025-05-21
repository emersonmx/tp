use crate::{
    config::Session,
    tmux_client::{Client, OptionName, SessionName, WindowID, WindowName},
};
use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum Error {
    #[error("unable to setup base ids: {0}")]
    BaseIdsError(String),
}

pub struct Muxer<C: Client> {
    client: C,
    base_window_id: usize,
    base_pane_id: usize,
}

pub struct Output {
    pub session_name: String,
    pub is_new_session: bool,
    pub windows: Vec<(usize, Vec<usize>)>,
}

impl<C: Client> Muxer<C> {
    pub fn new(client: C) -> Self {
        Self {
            client,
            base_window_id: 0,
            base_pane_id: 0,
        }
    }

    pub fn apply(&mut self, session: Session) -> Result<Output, Error> {
        let session_name = SessionName::new(session.name.to_owned());
        let mut windows = vec![];
        if self.client.has_session(&session_name) {
            self.client.switch_to_session(&session_name);
            return Ok(Output {
                session_name: session.name.to_owned(),
                is_new_session: false,
                windows,
            });
        }

        self.setup_base_ids()?;
        self.client.new_session(&session_name);

        for (widx, window) in session.windows.iter().enumerate() {
            println!("{widx} - {window:?}");
            if widx > 0 {
                self.client.new_window(&session_name);
            }

            let window_name = window.name.clone().unwrap_or("".to_string());
            if !window_name.is_empty() {
                self.client.rename_window(
                    WindowID::new(&session_name, (self.base_window_id + widx).to_string()),
                    WindowName::new(window_name),
                );
            }
        }
        windows.push((self.base_window_id, vec![self.base_pane_id]));

        self.client.switch_to_session(&session_name);

        Ok(Output {
            session_name: session.name.to_owned(),
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
            .get_option(OptionName::new(name))
            .map_err(|e| Error::BaseIdsError(e.to_string()))?
            .value()
            .parse()
            .map_err(|e| Error::BaseIdsError(format!("{}", e)))?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use mockall::mock;

    use super::*;
    use crate::tmux_client::*;

    mock! {
        Client {}
        impl Client for Client {
            fn get_option(&mut self, name: OptionName) -> Result<OptionValue, crate::tmux_client::Error>;
            fn set_option(&mut self, name: OptionName, value: OptionValue);

            fn new_session(&mut self, name: &SessionName);
            fn switch_to_session(&mut self, name: &SessionName);
            fn has_session(&mut self, name: &SessionName) -> bool;

            fn new_window(&mut self, name: &SessionName);
            fn rename_window(&mut self, id: WindowID, name: WindowName);

            fn new_pane(&mut self);
            fn select_pane(&mut self, id: PaneID);

            fn send_keys(&mut self, keys: Keys);

            fn use_layout(&mut self, layout: Layout);
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
        mock_client
    }

    #[test]
    fn switch_to_session_if_exists() {
        let session: Session = serde_yaml::from_str("name: test").unwrap();
        let mut mock_client = MockClient::new();
        mock_client.expect_has_session().return_const(true);
        mock_client.expect_switch_to_session().return_const(());
        let mut runner = Muxer::new(mock_client);

        let output = runner.apply(session).unwrap();

        assert_eq!(output.session_name, "test".to_string());
        assert!(!output.is_new_session);
    }

    #[test]
    fn create_a_session_if_not_exists() {
        let session: Session = serde_yaml::from_str("name: test").unwrap();
        let mock_client = make_mock_client();
        let mut runner = Muxer::new(mock_client);

        let output = runner.apply(session).unwrap();

        assert_eq!(output.session_name, "test".to_string());
        assert!(output.is_new_session);
    }

    #[test]
    fn base_ids_starts_at_zero() {
        let session: Session = serde_yaml::from_str("name: test").unwrap();
        let mock_client = make_mock_client();
        let mut runner = Muxer::new(mock_client);

        let output = runner.apply(session).unwrap();

        assert_eq!(output.windows, vec![(0, vec![0])]);
    }
}
