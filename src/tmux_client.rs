use std::{
    fmt::Display,
    process::{Command, Stdio},
};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Id(String);

impl Id {
    fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self(session_id.into())
    }

    pub fn to_id(&self) -> Id {
        Id::new(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowID(SessionId, String);

impl WindowID {
    pub fn new(session_id: &SessionId, window_id: impl Into<String>) -> Self {
        Self(session_id.to_owned(), window_id.into())
    }

    pub fn to_id(&self) -> Id {
        Id::new(format!("{}:{}", self.0.to_id(), self.1))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaneID(WindowID, String);

impl PaneID {
    pub fn new(window_id: &WindowID, pane_id: impl Into<String>) -> Self {
        Self(window_id.to_owned(), pane_id.into())
    }

    pub fn to_id(&self) -> Id {
        Id::new(format!("{}:{}", self.0.to_id(), self.1))
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
    #[error("option `{0}` not found")]
    OptionNotFound(String),
}

#[allow(dead_code)]
pub trait Client {
    fn get_option(&mut self, option_name: OptionName) -> Result<OptionValue, Error>;
    fn set_option(&mut self, option_name: OptionName, option_value: OptionValue);

    fn new_session(&mut self, session_id: &SessionId, directory: &str);
    fn switch_to_session(&mut self, session_id: &SessionId);
    fn has_session(&mut self, session_id: &SessionId) -> bool;

    fn new_window(&mut self, session_id: &SessionId, directory: &str);
    fn rename_window(&mut self, window_id: WindowID, window_name: WindowName);

    fn new_pane(&mut self, window_id: WindowID, directory: &str);
    fn select_pane(&mut self, pane_id: PaneID);

    fn send_keys(&mut self, pane_id: PaneID, keys: Keys);

    fn use_layout(&mut self, layout: Layout);
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TmuxClient;

impl Client for TmuxClient {
    fn get_option(&mut self, option_name: OptionName) -> Result<OptionValue, Error> {
        let output = Command::new("tmux")
            .args(["show-options", "-gv", option_name.value()])
            .stderr(Stdio::null())
            .output()
            .map_err(|e| Error::OptionNotFound(e.to_string()))?;

        let value = str::from_utf8(output.stdout.trim_ascii())
            .map_err(|e| Error::OptionNotFound(e.to_string()))?;

        Ok(OptionValue::new(value))
    }

    fn set_option(&mut self, option_name: OptionName, option_value: OptionValue) {
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
                &session_id.to_id().to_string(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn switch_to_session(&mut self, session_id: &SessionId) {
        let _ = Command::new("tmux")
            .args(["switch-client", "-t", &session_id.to_id().to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn has_session(&mut self, session_id: &SessionId) -> bool {
        let output = Command::new("tmux")
            .args(["has-session", "-t", &session_id.to_id().to_string()])
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
            .args([
                "new-window",
                "-c",
                directory,
                "-t",
                &session_id.to_id().to_string(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn rename_window(&mut self, window_id: WindowID, window_name: WindowName) {
        let _ = Command::new("tmux")
            .args([
                "rename-window",
                "-t",
                &window_id.to_id().to_string(),
                window_name.value(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn new_pane(&mut self, window_id: WindowID, directory: &str) {
        let _ = Command::new("tmux")
            .args([
                "split-window",
                "-c",
                directory,
                "-t",
                &window_id.to_id().to_string(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn select_pane(&mut self, pane_id: PaneID) {
        let window_id = pane_id.0.clone();
        let _ = Command::new("tmux")
            .args(["select-window", "-t", &window_id.to_id().to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        let _ = Command::new("tmux")
            .args(["select-pane", "-t", &pane_id.to_id().to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn send_keys(&mut self, pane_id: PaneID, keys: Keys) {
        let _ = Command::new("tmux")
            .args([
                "send-keys",
                "-t",
                &pane_id.to_id().to_string(),
                keys.value(),
                "C-m",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn use_layout(&mut self, layout: Layout) {
        let _ = layout;
        todo!()
    }
}
