use std::process::{Command, Stdio};
use tp::muxer::{
    Client, Error, Keys, Layout, OptionName, OptionValue, PaneID, SessionId, WindowID, WindowName,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TmuxClient;

impl Client for TmuxClient {
    fn get_option(&mut self, option_name: &OptionName) -> Result<OptionValue, Error> {
        let output = Command::new("tmux")
            .args(["show-options", "-gv", option_name.value()])
            .stderr(Stdio::null())
            .output()
            .map_err(|e| Error::OptionNotFound(e.to_string()))?;

        let value = str::from_utf8(output.stdout.trim_ascii())
            .map_err(|e| Error::OptionNotFound(e.to_string()))?;

        Ok(OptionValue::new(value))
    }

    fn set_option(&mut self, option_name: &OptionName, option_value: &OptionValue) {
        let (_, _) = (option_name, option_value);
        todo!()
    }

    fn new_session(&mut self, session_id: &SessionId, directory: &str) {
        let _ = Command::new("tmux")
            .args([
                "new-session",
                "-d",
                "-c",
                directory,
                "-s",
                &session_id.to_string(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn switch_to_session(&mut self, session_id: &SessionId) {
        let _ = Command::new("tmux")
            .args(["switch-client", "-t", &session_id.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn has_session(&mut self, session_id: &SessionId) -> bool {
        let output = Command::new("tmux")
            .args(["has-session", "-t", &session_id.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match output {
            Ok(status) => status.success(),
            _ => false,
        }
    }

    fn new_window(&mut self, session_id: &SessionId, directory: &str) {
        let _ = Command::new("tmux")
            .args(["new-window", "-c", directory, "-t", &session_id.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn rename_window(&mut self, window_id: &WindowID, window_name: &WindowName) {
        let _ = Command::new("tmux")
            .args([
                "rename-window",
                "-t",
                &window_id.to_string(),
                window_name.value(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn new_pane(&mut self, window_id: &WindowID, directory: &str) {
        let _ = Command::new("tmux")
            .args([
                "split-window",
                "-c",
                directory,
                "-t",
                &window_id.to_string(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn select_pane(&mut self, pane_id: &PaneID) {
        let window_id = pane_id.window_id();
        let _ = Command::new("tmux")
            .args(["select-window", "-t", &window_id.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        let _ = Command::new("tmux")
            .args(["select-pane", "-t", &pane_id.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn send_keys(&mut self, pane_id: &PaneID, keys: Keys) {
        let _ = Command::new("tmux")
            .args(["send-keys", "-t", &pane_id.to_string(), keys.value(), "C-m"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn use_layout(&mut self, layout: &Layout) {
        let _ = layout;
        todo!()
    }
}
