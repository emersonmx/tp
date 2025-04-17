use super::tmux_client::{Client, SessionName};
use crate::config::Session;
use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum Error {}

pub struct Muxer<C: Client> {
    client: C,
    base_window_id: usize,
    base_pane_id: usize,
}

impl<C: Client> Muxer<C> {
    pub fn new(client: C) -> Self {
        Self {
            client,
            base_window_id: 0,
            base_pane_id: 0,
        }
    }

    pub fn apply(&mut self, session: Session) -> Result<(), Error> {
        self.setup_base_ids()?;

        let session_name = SessionName::new(session.name);
        if self.client.has_session(&session_name) {
            self.client.switch_to_session(&session_name);
            return Ok(());
        }

        self.client.new_session(&session_name);

        Ok(())
    }

    fn setup_base_ids(&mut self) -> Result<(), Error> {
        self.base_window_id = 0;
        self.base_pane_id = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mockall::{
        mock,
        predicate::{self, *},
    };

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

            fn new_window(&mut self);
            fn rename_window(&mut self, id: WindowID, name: WindowName);

            fn new_pane(&mut self);
            fn select_pane(&mut self, id: PaneID);

            fn send_keys(&mut self, keys: Keys);

            fn use_layout(&mut self, layout: Layout);
        }
    }

    #[test]
    fn switch_to_session_if_exists() {
        let session: Session = serde_yaml::from_str("name: test").unwrap();

        let mut mock_client = MockClient::new();
        mock_client.expect_has_session().returning(|_| true);
        mock_client
            .expect_switch_to_session()
            .with(predicate::eq(SessionName::new("test")))
            .times(1)
            .returning(|_| {});
        let mut runner = Muxer::new(mock_client);

        runner.apply(session).unwrap();
    }

    #[test]
    fn create_a_session_if_not_exists() {
        let session: Session = serde_yaml::from_str("name: test").unwrap();

        let mut mock_client = MockClient::new();
        mock_client.expect_has_session().returning(|_| false);
        mock_client
            .expect_new_session()
            .with(predicate::eq(SessionName::new("test")))
            .times(1)
            .returning(|_| {});
        let mut runner = Muxer::new(mock_client);

        runner.apply(session).unwrap();
    }
}
