use std::process::{Command, Stdio};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct SessionName(String);

impl SessionName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn value(&self) -> String {
        self.0.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowID(SessionName, String);

impl WindowID {
    pub fn new(session: &SessionName, name: impl Into<String>) -> Self {
        Self(session.clone(), name.into())
    }

    pub fn value(&self) -> String {
        format!("{}:{}", self.0.value(), self.1)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowName(String);

impl WindowName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn value(&self) -> String {
        self.0.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaneID(WindowID, String);

#[derive(Debug, Clone, PartialEq)]
pub struct OptionName(String);

impl OptionName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn value(&self) -> String {
        self.0.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionValue(String);

impl OptionValue {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn value(&self) -> String {
        self.0.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layout(String);

#[derive(Debug, Clone, PartialEq)]
pub struct Keys(String);

#[derive(Error, PartialEq, Debug)]
pub enum Error {
    #[error("option `{0}` not found")]
    OptionNotFound(String),
}

pub trait Client {
    fn get_option(&mut self, name: OptionName) -> Result<OptionValue, Error>;
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TmuxClient;

impl Client for TmuxClient {
    fn get_option(&mut self, name: OptionName) -> Result<OptionValue, Error> {
        let output = Command::new("tmux")
            .args(["show-options", "-gv", &name.value()])
            .stderr(Stdio::null())
            .output()
            .map_err(|e| Error::OptionNotFound(e.to_string()))?;

        let value = str::from_utf8(output.stdout.trim_ascii())
            .map_err(|e| Error::OptionNotFound(e.to_string()))?;

        Ok(OptionValue::new(value))
    }

    fn set_option(&mut self, name: OptionName, value: OptionValue) {
        let (_, _) = (name, value);
        todo!()
    }

    fn new_session(&mut self, name: &SessionName) {
        let _ = Command::new("tmux")
            .args(["new-session", "-d", "-s", &name.value()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn switch_to_session(&mut self, name: &SessionName) {
        let _ = Command::new("tmux")
            .args(["switch-client", "-t", &name.value()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn has_session(&mut self, name: &SessionName) -> bool {
        let output = Command::new("tmux")
            .args(["has-session", "-t", &name.value()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match output {
            Ok(status) => status.success(),
            _ => false,
        }
    }

    fn new_window(&mut self, name: &SessionName) {
        let _ = Command::new("tmux")
            .args(["new-window", "-t", &name.value()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn rename_window(&mut self, id: WindowID, name: WindowName) {
        let _ = Command::new("tmux")
            .args(["rename-window", "-t", &id.value(), &name.value()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn new_pane(&mut self) {
        todo!()
    }

    fn select_pane(&mut self, id: PaneID) {
        let _ = id;
        todo!()
    }

    fn send_keys(&mut self, keys: Keys) {
        let _ = keys;
        todo!()
    }

    fn use_layout(&mut self, layout: Layout) {
        let _ = layout;
        todo!()
    }
}
