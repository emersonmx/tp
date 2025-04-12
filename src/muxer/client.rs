pub struct SessionName(String);
pub struct WindowID(SessionName, String);
pub struct WindowName(String);
pub struct PaneID(WindowID, String);
pub struct OptionName(String);
pub struct OptionValue(String);
pub struct Layout(String);

pub trait TmuxClient {
    fn get_option(name: OptionName) -> OptionValue;
    fn set_option(name: OptionName, value: OptionValue);

    fn new_session(name: SessionName);
    fn switch_to_session(name: SessionName);

    fn new_window();
    fn rename_window(id: WindowID, name: WindowName);

    fn new_pane();
    fn select_pane(id: PaneID);

    fn send_keys(text: impl Into<String>);

    fn use_layout(layout: Layout);
}
