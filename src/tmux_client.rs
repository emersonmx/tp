use std::process::{Command, Stdio};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct SessionName(String);

impl SessionName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn value(&self) -> &String {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowID(SessionName, String);

#[derive(Debug, Clone, PartialEq)]
pub struct WindowName(String);

#[derive(Debug, Clone, PartialEq)]
pub struct PaneID(WindowID, String);

#[derive(Debug, Clone, PartialEq)]
pub struct OptionName(String);

impl OptionName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn value(&self) -> &String {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionValue(String);

impl OptionValue {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn value(&self) -> &String {
        &self.0
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

    fn new_window(&mut self);
    fn rename_window(&mut self, id: WindowID, name: WindowName);

    fn new_pane(&mut self);
    fn select_pane(&mut self, id: PaneID);

    fn send_keys(&mut self, keys: Keys);

    fn use_layout(&mut self, layout: Layout);
}

pub struct TmuxClient;

impl TmuxClient {
    pub fn new() -> Self {
        Self {}
    }
}

impl Client for TmuxClient {
    fn get_option(&mut self, name: OptionName) -> Result<OptionValue, Error> {
        let _ = name;
        todo!()
    }

    fn set_option(&mut self, name: OptionName, value: OptionValue) {
        let (_, _) = (name, value);
        todo!()
    }

    fn new_session(&mut self, name: &SessionName) {
        let _ = name;
        todo!()
    }

    fn switch_to_session(&mut self, name: &SessionName) {
        let _ = Command::new("tmux")
            .args(["switch-client", "-t", name.value()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
    }

    fn has_session(&mut self, name: &SessionName) -> bool {
        Command::new("tmux")
            .args(["has-session", "-t", name.value()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    fn new_window(&mut self) {
        todo!()
    }

    fn rename_window(&mut self, id: WindowID, name: WindowName) {
        let (_, _) = (id, name);
        todo!()
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
