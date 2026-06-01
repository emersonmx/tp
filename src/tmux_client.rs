use crate::muxer::{
    Client, Error, Keys, Layout, OptionName, OptionValue, PaneID, SessionId, WindowID, WindowName,
};
use std::process::Command;

pub trait CommandExecutor {
    fn execute(&self, args: &[&str]) -> std::io::Result<std::process::Output>;
}

pub struct StdCommandExecutor;

impl CommandExecutor for StdCommandExecutor {
    fn execute(&self, args: &[&str]) -> std::io::Result<std::process::Output> {
        Command::new("tmux").args(args).output()
    }
}

pub struct TmuxClient<E: CommandExecutor>(E);

impl TmuxClient<StdCommandExecutor> {
    pub fn new() -> Self {
        TmuxClient(StdCommandExecutor)
    }
}

impl<E: CommandExecutor> Client for TmuxClient<E> {
    fn is_running_inside_tmux(&mut self) -> bool {
        std::env::var("TMUX").is_ok()
    }

    fn get_option(&mut self, option_name: &OptionName) -> Result<OptionValue, Error> {
        let output = self
            .0
            .execute(&["show-options", "-gv", option_name.value()])
            .map_err(system_error)?;

        if !output.status.success() {
            return Err(Error::OptionNotFound(option_name.value().to_string()));
        }

        let value = String::from_utf8_lossy(output.stdout.trim_ascii());

        Ok(OptionValue::new(value))
    }

    fn set_option(
        &mut self,
        option_name: &OptionName,
        option_value: &OptionValue,
    ) -> Result<(), Error> {
        let (_, _) = (option_name, option_value);
        todo!()
    }

    fn new_session(&mut self, session_id: &SessionId, directory: &str) -> Result<(), Error> {
        self.0
            .execute(&[
                "new-session",
                "-d",
                "-c",
                directory,
                "-s",
                &session_id.to_string(),
            ])
            .map_err(system_error)?;
        Ok(())
    }

    fn switch_to_session(&mut self, session_id: &SessionId) -> Result<(), Error> {
        self.0
            .execute(&["switch-client", "-t", &session_id.to_string()])
            .map_err(system_error)?;
        Ok(())
    }

    fn has_session(&mut self, session_id: &SessionId) -> Result<bool, Error> {
        let output = self
            .0
            .execute(&["has-session", "-t", &session_id.to_string()])
            .map_err(system_error)?;

        Ok(output.status.success())
    }

    fn new_window(&mut self, session_id: &SessionId, directory: &str) -> Result<(), Error> {
        self.0
            .execute(&["new-window", "-c", directory, "-t", &session_id.to_string()])
            .map_err(system_error)?;
        Ok(())
    }

    fn rename_window(
        &mut self,
        window_id: &WindowID,
        window_name: &WindowName,
    ) -> Result<(), Error> {
        self.0
            .execute(&[
                "rename-window",
                "-t",
                &window_id.to_string(),
                window_name.value(),
            ])
            .map_err(system_error)?;
        Ok(())
    }

    fn new_pane(&mut self, window_id: &WindowID, directory: &str) -> Result<(), Error> {
        self.0
            .execute(&[
                "split-window",
                "-c",
                directory,
                "-t",
                &window_id.to_string(),
            ])
            .map_err(system_error)?;
        Ok(())
    }

    fn select_pane(&mut self, pane_id: &PaneID) -> Result<(), Error> {
        let window_id = pane_id.window_id();
        self.0
            .execute(&["select-window", "-t", &window_id.to_string()])
            .map_err(system_error)?;

        self.0
            .execute(&["select-pane", "-t", &pane_id.to_string()])
            .map_err(system_error)?;
        Ok(())
    }

    fn send_keys(&mut self, pane_id: &PaneID, keys: Keys) -> Result<(), Error> {
        self.0
            .execute(&["send-keys", "-t", &pane_id.to_string(), keys.value(), "C-m"])
            .map_err(system_error)?;
        Ok(())
    }

    fn use_layout(&mut self, layout: &Layout) -> Result<(), Error> {
        let _ = layout;
        todo!()
    }
}

fn system_error(source: std::io::Error) -> Error {
    Error::System(source)
}
