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

#[derive(Debug, Clone, PartialEq)]
pub struct OptionValue(String);

#[derive(Debug, Clone, PartialEq)]
pub struct Layout(String);

#[derive(Debug, Clone, PartialEq)]
pub struct Keys(String);

#[derive(Error, PartialEq, Debug)]
pub enum Error {}

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
