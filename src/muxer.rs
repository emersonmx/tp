#![allow(dead_code)]

use crate::config::Session;
use std::{
    env,
    fmt::Display,
    io::Result as IoResult,
    path::{Path, PathBuf},
    process::{Command, Output as StdOutput},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Id(String);

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionId(Id);

impl SessionId {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self(Id(session_id.into()))
    }

    pub fn id(&self) -> &Id {
        &self.0
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowId(SessionId, Id);

impl WindowId {
    pub fn new(session_id: &SessionId, window_id: impl Into<String>) -> Self {
        let window_id = Id(format!("{}:{}", session_id, window_id.into()));
        Self(session_id.clone(), window_id)
    }

    pub fn session_id(&self) -> &Id {
        self.0.id()
    }

    pub fn id(&self) -> &Id {
        &self.1
    }
}

impl Display for WindowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaneId(WindowId, Id);

impl PaneId {
    pub fn new(window_id: &WindowId, pane_id: impl Into<String>) -> Self {
        let pane_id = Id(format!("{}.{}", window_id.clone(), pane_id.into()));
        Self(window_id.clone(), pane_id)
    }

    pub fn session_id(&self) -> &Id {
        self.0.session_id()
    }

    pub fn window_id(&self) -> &Id {
        self.0.id()
    }

    pub fn id(&self) -> &Id {
        &self.1
    }
}

impl Display for PaneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id())
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

impl Layout {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unable to setup base ids: {0}")]
    BaseIds(String),
    #[error("not running inside a tmux session")]
    NotInsideTmuxSession,
    #[error("option `{0}` not found")]
    OptionNotFound(String),
    #[error("a system error occurred while executing tmux command")]
    System(#[source] std::io::Error),
    #[error("tmux command failed ({0})")]
    Tmux(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub session_name: String,
    pub is_new_session: bool,
    pub windows: Vec<(usize, Vec<usize>)>,
}

pub struct Muxer<E>
where
    E: Fn(&[&str]) -> IoResult<StdOutput>,
{
    base_window_id: usize,
    base_pane_id: usize,
    command_executor: E,
}

impl Muxer<fn(&[&str]) -> IoResult<StdOutput>> {
    pub fn new() -> Self {
        Self {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: |args| Command::new("tmux").args(args).output(),
        }
    }
}

impl<E> Muxer<E>
where
    E: Fn(&[&str]) -> IoResult<StdOutput>,
{
    pub fn apply(&mut self, session: &Session) -> Result<Output, Error> {
        if !self.is_running_inside_tmux() {
            return Err(Error::NotInsideTmuxSession);
        }

        let session_id = SessionId::new(&session.name);
        let mut windows = vec![];
        if self.has_session(&session_id)? {
            self.switch_to_session(&session_id)?;
            return Ok(Output {
                session_name: session.name.clone(),
                is_new_session: false,
                windows,
            });
        }

        self.setup_base_ids()?;

        let first_window = session.windows.first();
        let initial_dir = resolve_directory(
            &session.directory,
            &first_window.and_then(|window| window.directory.clone()),
            &first_window
                .and_then(|window| window.panes.first().and_then(|pane| pane.directory.clone())),
        );
        let initial_dir = directory_to_string(initial_dir);
        self.new_session(&session_id, &initial_dir)?;

        let session_dir = session.directory.clone();
        let mut focus_pane: Option<PaneId> = None;
        for (wid, window) in session.windows.iter().enumerate() {
            let window_dir = resolve_directory(&session_dir, &window.directory, &None);
            if wid > 0 {
                let initial_dir = resolve_directory(
                    &session_dir,
                    &window_dir,
                    &window.panes.first().and_then(|pane| pane.directory.clone()),
                );
                self.new_window(&session_id, &directory_to_string(initial_dir))?;
            }

            let widx = self.base_window_id + wid;
            let window_id = WindowId::new(&session_id, widx.to_string());
            if let Some(window_name) = &window.name {
                self.rename_window(&window_id, &WindowName::new(window_name))?;
            }

            let mut panes: Vec<usize> = vec![];
            for (pid, pane) in window.panes.iter().enumerate() {
                let pidx = self.base_pane_id + pid;
                let pane_id = PaneId::new(&window_id, pidx.to_string());
                if pane.focus {
                    focus_pane = Some(pane_id.clone());
                }

                let pane_dir = resolve_directory(&session_dir, &window_dir, &pane.directory);
                if pid > 0 {
                    self.new_pane(&window_id, &directory_to_string(pane_dir))?;
                }

                if let Some(cmd) = &pane.command {
                    self.send_keys(&pane_id, Keys::new(cmd))?;
                }

                panes.push(pidx);
            }

            windows.push((widx, panes));
        }

        if let Some(pane) = focus_pane {
            self.select_pane(&pane)?;
        }

        self.switch_to_session(&session_id)?;

        Ok(Output {
            session_name: session.name.clone(),
            is_new_session: true,
            windows,
        })
    }

    fn execute(&self, args: &[&str]) -> IoResult<StdOutput> {
        (self.command_executor)(args)
    }

    fn is_running_inside_tmux(&self) -> bool {
        std::env::var("TMUX").is_ok()
    }

    fn get_option(&self, option_name: &OptionName) -> Result<OptionValue, Error> {
        let output = self
            .execute(&["show-options", "-gv", option_name.value()])
            .map_err(system_error)?;

        if !output.status.success() {
            return Err(Error::OptionNotFound(option_name.value().to_string()));
        }

        let value = String::from_utf8_lossy(output.stdout.trim_ascii());

        Ok(OptionValue::new(value))
    }

    fn set_option(
        &self,
        option_name: &OptionName,
        option_value: &OptionValue,
    ) -> Result<(), Error> {
        let (_, _) = (option_name, option_value);
        todo!()
    }

    fn new_session(&self, session_id: &SessionId, directory: &str) -> Result<(), Error> {
        self.execute(&[
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

    fn switch_to_session(&self, session_id: &SessionId) -> Result<(), Error> {
        self.execute(&["switch-client", "-t", &session_id.to_string()])
            .map_err(system_error)?;
        Ok(())
    }

    fn has_session(&self, session_id: &SessionId) -> Result<bool, Error> {
        let output = self
            .execute(&["has-session", "-t", &session_id.to_string()])
            .map_err(system_error)?;

        Ok(output.status.success())
    }

    fn new_window(&self, session_id: &SessionId, directory: &str) -> Result<(), Error> {
        self.execute(&["new-window", "-c", directory, "-t", &session_id.to_string()])
            .map_err(system_error)?;
        Ok(())
    }

    fn rename_window(&self, window_id: &WindowId, window_name: &WindowName) -> Result<(), Error> {
        self.execute(&[
            "rename-window",
            "-t",
            &window_id.to_string(),
            window_name.value(),
        ])
        .map_err(system_error)?;
        Ok(())
    }

    fn new_pane(&self, window_id: &WindowId, directory: &str) -> Result<(), Error> {
        self.execute(&[
            "split-window",
            "-c",
            directory,
            "-t",
            &window_id.to_string(),
        ])
        .map_err(system_error)?;
        Ok(())
    }

    fn select_pane(&self, pane_id: &PaneId) -> Result<(), Error> {
        let window_id = pane_id.window_id();
        self.execute(&["select-window", "-t", &window_id.to_string()])
            .map_err(system_error)?;

        self.execute(&["select-pane", "-t", &pane_id.to_string()])
            .map_err(system_error)?;
        Ok(())
    }

    fn send_keys(&self, pane_id: &PaneId, keys: Keys) -> Result<(), Error> {
        self.execute(&["send-keys", "-t", &pane_id.to_string(), keys.value(), "C-m"])
            .map_err(system_error)?;
        Ok(())
    }

    fn use_layout(&self, layout: &Layout) -> Result<(), Error> {
        let _ = layout;
        todo!()
    }

    fn setup_base_ids(&mut self) -> Result<(), Error> {
        self.base_window_id = self.get_index("base-index")?;
        self.base_pane_id = self.get_index("pane-base-index")?;
        Ok(())
    }

    fn get_index(&mut self, name: &str) -> Result<usize, Error> {
        let value = self
            .get_option(&OptionName::new(name))
            .map_err(|e| Error::BaseIds(e.to_string()))?
            .value()
            .parse()
            .map_err(|e| Error::BaseIds(format!("{}", e)))?;
        Ok(value)
    }
}

fn directory_to_string(directory: Option<PathBuf>) -> String {
    directory
        .map(expand_tilde)
        .and_then(|dir| dir.to_str().map(|s| s.to_owned()))
        .unwrap_or_else(|| ".".to_owned())
}

fn expand_tilde(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    path.strip_prefix("~/")
        .ok()
        .and_then(|suffix| {
            env::var("HOME")
                .ok()
                .map(|home_str| PathBuf::from(home_str).join(suffix))
        })
        .unwrap_or_else(|| path.to_owned())
}

fn resolve_directory(
    session_dir: &Option<PathBuf>,
    window_dir: &Option<PathBuf>,
    pane_dir: &Option<PathBuf>,
) -> Option<PathBuf> {
    pane_dir
        .as_ref()
        .or(window_dir.as_ref())
        .or(session_dir.as_ref())
        .map(|p| p.to_owned())
}

fn system_error(source: std::io::Error) -> Error {
    Error::System(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Pane, Window};
    use rstest::{fixture, rstest};
    use std::cell::RefCell;
    use std::os::unix::process::ExitStatusExt;
    use std::rc::Rc;

    type IoResultOutput = IoResult<StdOutput>;
    type CalledArgs = Rc<RefCell<String>>;

    fn mock_muxer() -> (CalledArgs, Muxer<impl Fn(&[&str]) -> IoResultOutput>) {
        let called_args = Rc::new(RefCell::new(String::new()));
        let called_args_clone = called_args.clone();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                called_args_clone.borrow_mut().push_str(&args.join(" "));
                called_args_clone.borrow_mut().push('\n');
                Ok(StdOutput {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: vec![],
                    stderr: vec![],
                })
            },
        };
        (called_args, muxer)
    }

    fn mock_muxer_with_apply_behavior() -> (CalledArgs, Muxer<impl Fn(&[&str]) -> IoResultOutput>) {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                let output = muxer.execute(args)?;

                match args[0] {
                    "has-session" => Ok(StdOutput {
                        status: std::process::ExitStatus::from_raw(1),
                        ..output
                    }),
                    "show-options" => Ok(StdOutput {
                        stdout: b"1\n".to_vec(),
                        ..output
                    }),
                    _ => Ok(output),
                }
            },
        };
        (called_args, muxer)
    }

    #[fixture]
    fn session() -> Session {
        Session {
            name: "test".to_string(),
            directory: None,
            windows: vec![],
        }
    }

    #[fixture]
    fn window_id(session_id: SessionId) -> WindowId {
        WindowId::new(&session_id, "0")
    }

    #[fixture]
    fn pane_id(window_id: WindowId) -> PaneId {
        PaneId::new(&window_id, "0")
    }

    #[fixture]
    fn session_id() -> SessionId {
        SessionId::new("test")
    }

    #[rstest]
    fn new_muxer_allows_custom_command_executor() {
        let (called_args, muxer) = mock_muxer();

        let output = muxer.execute(&["test-command", "arg1", "arg2"]);

        assert!(output.is_ok());
        let expected = "test-command arg1 arg2\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn apply_not_inside_tmux_session(session: Session) {
        temp_env::with_var_unset("TMUX", || {
            let mut muxer = Muxer::new();

            let output = muxer.apply(&session);

            assert!(matches!(output, Err(Error::NotInsideTmuxSession)));
        });
    }

    #[rstest]
    fn apply_when_has_session(session: Session) {
        let (called_args, mut muxer) = mock_muxer();

        temp_env::with_var("TMUX", Some("test"), || {
            let output = muxer.apply(&session).unwrap();

            assert_eq!(output.session_name, "test");
            assert!(!output.is_new_session);
            assert!(output.windows.is_empty());
        });

        let expected = ["has-session -t test", "switch-client -t test\n"].join("\n");
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn apply_on_new_session_without_windows(session: Session) {
        let (called_args, mut muxer) = mock_muxer_with_apply_behavior();

        temp_env::with_var("TMUX", Some("test"), || {
            let output = muxer.apply(&session).unwrap();

            assert_eq!(output.session_name, "test");
            assert!(output.is_new_session);
            assert!(output.windows.is_empty());
        });

        let expected = [
            "has-session -t test",
            "show-options -gv base-index",
            "show-options -gv pane-base-index",
            "new-session -d -c . -s test",
            "switch-client -t test\n",
        ]
        .join("\n");
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn apply_on_new_session_with_two_panes(session: Session) {
        let session = Session {
            windows: vec![Window {
                panes: vec![Default::default(), Default::default()],
                ..Default::default()
            }],
            ..session
        };
        let (called_args, mut muxer) = mock_muxer_with_apply_behavior();

        temp_env::with_var("TMUX", Some("test"), || {
            let output = muxer.apply(&session).unwrap();

            assert_eq!(output.session_name, "test");
            assert!(output.is_new_session);
            assert!(!output.windows.is_empty());
        });

        let expected = [
            "has-session -t test",
            "show-options -gv base-index",
            "show-options -gv pane-base-index",
            "new-session -d -c . -s test",
            "split-window -c . -t test:1",
            "switch-client -t test\n",
        ]
        .join("\n");
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn apply_on_new_session_with_two_windows(session: Session) {
        let session = Session {
            windows: vec![Default::default(), Default::default()],
            ..session
        };
        let (called_args, mut muxer) = mock_muxer_with_apply_behavior();

        temp_env::with_var("TMUX", Some("test"), || {
            let output = muxer.apply(&session).unwrap();

            assert_eq!(output.session_name, "test");
            assert!(output.is_new_session);
            assert!(!output.windows.is_empty());
        });

        let expected = [
            "has-session -t test",
            "show-options -gv base-index",
            "show-options -gv pane-base-index",
            "new-session -d -c . -s test",
            "new-window -c . -t test",
            "switch-client -t test\n",
        ]
        .join("\n");
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn apply_on_new_session_when_name_a_window(session: Session) {
        let session = Session {
            windows: vec![Window {
                name: Some("window1".to_string()),
                ..Default::default()
            }],
            ..session
        };
        let (called_args, mut muxer) = mock_muxer_with_apply_behavior();

        temp_env::with_var("TMUX", Some("test"), || {
            let output = muxer.apply(&session).unwrap();

            assert_eq!(output.session_name, "test");
            assert!(output.is_new_session);
            assert!(!output.windows.is_empty());
        });

        let expected = [
            "has-session -t test",
            "show-options -gv base-index",
            "show-options -gv pane-base-index",
            "new-session -d -c . -s test",
            "rename-window -t test:1 window1",
            "switch-client -t test\n",
        ]
        .join("\n");
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn apply_on_new_session_when_focus_a_pane(session: Session) {
        let session = Session {
            windows: vec![Window {
                panes: vec![
                    Pane {
                        focus: true,
                        ..Default::default()
                    },
                    Default::default(),
                ],
                ..Default::default()
            }],
            ..session
        };
        let (called_args, mut muxer) = mock_muxer_with_apply_behavior();

        temp_env::with_var("TMUX", Some("test"), || {
            let output = muxer.apply(&session).unwrap();

            assert_eq!(output.session_name, "test");
            assert!(output.is_new_session);
            assert!(!output.windows.is_empty());
        });

        let expected = [
            "has-session -t test",
            "show-options -gv base-index",
            "show-options -gv pane-base-index",
            "new-session -d -c . -s test",
            "split-window -c . -t test:1",
            "select-window -t test:1",
            "select-pane -t test:1.1",
            "switch-client -t test\n",
        ]
        .join("\n");
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn apply_on_new_session_when_run_a_command(session: Session) {
        let session = Session {
            windows: vec![Window {
                panes: vec![Pane {
                    command: Some("echo hello".to_string()),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..session
        };
        let (called_args, mut muxer) = mock_muxer_with_apply_behavior();

        temp_env::with_var("TMUX", Some("test"), || {
            let output = muxer.apply(&session).unwrap();

            assert_eq!(output.session_name, "test");
            assert!(output.is_new_session);
            assert!(!output.windows.is_empty());
        });

        let expected = [
            "has-session -t test",
            "show-options -gv base-index",
            "show-options -gv pane-base-index",
            "new-session -d -c . -s test",
            "send-keys -t test:1.1 echo hello C-m",
            "switch-client -t test\n",
        ]
        .join("\n");
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn call_execute_with_args() {
        let (called_args, muxer) = mock_muxer();
        let args = ["new-session", "-d", "-s", "test"];

        let _ = muxer.execute(&args);

        let expected = "new-session -d -s test\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn call_execute_without_args() {
        let (called_args, muxer) = mock_muxer();
        let args = [];

        let _ = muxer.execute(&args);

        let expected = "\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn check_is_running_inside_tmux() {
        temp_env::with_var("TMUX", Some("test"), || {
            let muxer = Muxer::new();

            assert!(muxer.is_running_inside_tmux());
        });

        temp_env::with_var_unset("TMUX", || {
            let muxer = Muxer::new();

            assert!(!muxer.is_running_inside_tmux());
        });
    }

    #[rstest]
    fn get_option_returns_value() {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args, ["show-options", "-gv", "base-index"]);
                let output = muxer.execute(args)?;
                Ok(StdOutput {
                    stdout: b"1\n".to_vec(),
                    ..output
                })
            },
        };

        let option_value = muxer.get_option(&OptionName::new("base-index")).unwrap();

        let expected = "show-options -gv base-index\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert_eq!(option_value.value(), "1");
    }

    #[rstest]
    fn get_option_system_error() {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args, ["show-options", "-gv", "base-index"]);
                muxer.execute(args)?;
                Err(std::io::Error::other("command failed"))
            },
        };

        let output = muxer.get_option(&OptionName::new("base-index"));

        let expected = "show-options -gv base-index\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::System(_))));
    }

    #[rstest]
    fn get_option_not_found() {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args, ["show-options", "-gv", "base-index"]);
                let output = muxer.execute(args)?;
                Ok(StdOutput {
                    status: std::process::ExitStatus::from_raw(1),
                    ..output
                })
            },
        };

        let output = muxer.get_option(&OptionName::new("base-index"));

        let expected = "show-options -gv base-index\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(
            output,
            Err(Error::OptionNotFound(option)) if option == "base-index"
        ));
    }

    // TODO: add set_option tests when implemented

    #[rstest]
    fn new_session_calls_execute(session_id: SessionId) {
        let (called_args, muxer) = mock_muxer();
        let directory = "/home/user";

        let _ = muxer.new_session(&session_id, directory);

        let expected = "new-session -d -c /home/user -s test\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn new_session_system_error(session_id: SessionId) {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(
                    args,
                    ["new-session", "-d", "-c", "/home/user", "-s", "test"]
                );
                muxer.execute(args)?;
                Err(std::io::Error::other("command failed"))
            },
        };
        let directory = "/home/user";

        let output = muxer.new_session(&session_id, directory);

        let expected = "new-session -d -c /home/user -s test\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::System(_))));
    }

    #[rstest]
    fn switch_to_session_calls_execute(session_id: SessionId) {
        let (called_args, muxer) = mock_muxer();

        let _ = muxer.switch_to_session(&session_id);

        let expected = "switch-client -t test\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn switch_to_session_system_error(session_id: SessionId) {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args, ["switch-client", "-t", "test"]);
                muxer.execute(args)?;
                Err(std::io::Error::other("command failed"))
            },
        };

        let output = muxer.switch_to_session(&session_id);

        let expected = "switch-client -t test\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::System(_))));
    }

    #[rstest]
    fn has_session_returns_true(session_id: SessionId) {
        let (called_args, muxer) = mock_muxer();

        let output = muxer.has_session(&session_id).unwrap();

        assert!(output);

        let expected = "has-session -t test\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn has_session_returns_false(session_id: SessionId) {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args, ["has-session", "-t", "test"]);
                let output = muxer.execute(args)?;
                Ok(StdOutput {
                    status: std::process::ExitStatus::from_raw(1),
                    ..output
                })
            },
        };

        let output = muxer.has_session(&session_id).unwrap();

        let expected = "has-session -t test\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(!output);
    }

    #[rstest]
    fn has_session_system_error(session_id: SessionId) {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args, ["has-session", "-t", "test"]);
                muxer.execute(args)?;
                Err(std::io::Error::other("command failed"))
            },
        };

        let output = muxer.has_session(&session_id);

        let expected = "has-session -t test\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::System(_))));
    }

    #[rstest]
    fn new_window_calls_execute(session_id: SessionId) {
        let (called_args, muxer) = mock_muxer();
        let directory = "/home/user";

        let _ = muxer.new_window(&session_id, directory);

        let expected = "new-window -c /home/user -t test\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn new_window_system_error(session_id: SessionId) {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args, ["new-window", "-c", "/home/user", "-t", "test"]);
                muxer.execute(args)?;
                Err(std::io::Error::other("command failed"))
            },
        };
        let directory = "/home/user";

        let output = muxer.new_window(&session_id, directory);

        let expected = "new-window -c /home/user -t test\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::System(_))));
    }

    #[rstest]
    fn rename_window_calls_execute(window_id: WindowId) {
        let (called_args, muxer) = mock_muxer();
        let window_name = WindowName::new("my-window");

        let _ = muxer.rename_window(&window_id, &window_name);

        let expected = "rename-window -t test:0 my-window\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn rename_window_system_error(window_id: WindowId) {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args, ["rename-window", "-t", "test:0", "my-window"]);
                muxer.execute(args)?;
                Err(std::io::Error::other("command failed"))
            },
        };
        let window_name = WindowName::new("my-window");

        let output = muxer.rename_window(&window_id, &window_name);

        let expected = "rename-window -t test:0 my-window\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::System(_))));
    }

    #[rstest]
    fn new_pane_calls_execute(window_id: WindowId) {
        let (called_args, muxer) = mock_muxer();
        let directory = "/home/user";

        let _ = muxer.new_pane(&window_id, directory);

        let expected = "split-window -c /home/user -t test:0\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn new_pane_system_error(window_id: WindowId) {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args, ["split-window", "-c", "/home/user", "-t", "test:0"]);
                muxer.execute(args)?;
                Err(std::io::Error::other("command failed"))
            },
        };
        let directory = "/home/user";

        let output = muxer.new_pane(&window_id, directory);

        let expected = "split-window -c /home/user -t test:0\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::System(_))));
    }

    #[rstest]
    fn select_pane_calls_execute(pane_id: PaneId) {
        let (called_args, muxer) = mock_muxer();

        let _ = muxer.select_pane(&pane_id);

        let expected = ["select-window -t test:0", "select-pane -t test:0.0\n"].join("\n");
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn select_pane_system_error_on_select_window(pane_id: PaneId) {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                let output = muxer.execute(args)?;
                if args == ["select-window", "-t", "test:0"] {
                    Err(std::io::Error::other("command failed"))
                } else {
                    Ok(output)
                }
            },
        };

        let output = muxer.select_pane(&pane_id);

        let expected = "select-window -t test:0\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::System(_))));
    }

    #[rstest]
    fn select_pane_system_error_on_select_pane(pane_id: PaneId) {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                let output = muxer.execute(args)?;
                if args == ["select-pane", "-t", "test:0.0"] {
                    Err(std::io::Error::other("command failed"))
                } else {
                    Ok(output)
                }
            },
        };

        let output = muxer.select_pane(&pane_id);

        let expected = ["select-window -t test:0", "select-pane -t test:0.0\n"].join("\n");
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::System(_))));
    }

    #[rstest]
    fn send_keys_calls_execute(pane_id: PaneId) {
        let (called_args, muxer) = mock_muxer();
        let keys = Keys::new("ls -la");

        let _ = muxer.send_keys(&pane_id, keys);

        let expected = "send-keys -t test:0.0 ls -la C-m\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn send_keys_system_error(pane_id: PaneId) {
        let (called_args, muxer) = mock_muxer();
        let muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args, ["send-keys", "-t", "test:0.0", "ls -la", "C-m"]);
                muxer.execute(args)?;
                Err(std::io::Error::other("command failed"))
            },
        };
        let keys = Keys::new("ls -la");

        let output = muxer.send_keys(&pane_id, keys);

        let expected = "send-keys -t test:0.0 ls -la C-m\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::System(_))));
    }

    // TODO: add use_layout tests when implemented

    #[rstest]
    fn setup_base_ids_calls_get_index() {
        let (called_args, muxer) = mock_muxer();
        let mut muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args[..2], ["show-options", "-gv"]);
                let output = muxer.execute(args)?;
                match args[2] {
                    "base-index" | "pane-base-index" => Ok(StdOutput {
                        stdout: b"1\n".to_vec(),
                        ..output
                    }),
                    _ => Ok(output),
                }
            },
        };

        let _ = muxer.setup_base_ids();

        let expected = [
            "show-options -gv base-index",
            "show-options -gv pane-base-index\n",
        ]
        .join("\n");
        assert_eq!(*called_args.borrow(), expected.to_string());
    }

    #[rstest]
    fn setup_base_ids_system_error() {
        let (called_args, muxer) = mock_muxer();
        let mut muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args, ["show-options", "-gv", "base-index"]);
                muxer.execute(args)?;
                Err(std::io::Error::other("command failed"))
            },
        };

        let output = muxer.setup_base_ids();

        let expected = "show-options -gv base-index\n";

        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::BaseIds(_))));
    }

    #[rstest]
    fn setup_base_ids_parse_error() {
        let (called_args, muxer) = mock_muxer();
        let mut muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args, ["show-options", "-gv", "base-index"]);
                let output = muxer.execute(args)?;
                Ok(StdOutput {
                    stdout: b"not-a-number\n".to_vec(),
                    ..output
                })
            },
        };

        let output = muxer.setup_base_ids();

        let expected = "show-options -gv base-index\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::BaseIds(_))));
    }

    #[rstest]
    fn get_index_returns_value() {
        let (called_args, muxer) = mock_muxer();
        let mut muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args[..2], ["show-options", "-gv"]);
                let output = muxer.execute(args)?;
                match args[2] {
                    "base-index" => Ok(StdOutput {
                        stdout: b"1\n".to_vec(),
                        ..output
                    }),
                    "pane-base-index" => Ok(StdOutput {
                        stdout: b"2\n".to_vec(),
                        ..output
                    }),
                    _ => Ok(output),
                }
            },
        };

        let base_index = muxer.get_index("base-index").unwrap();
        let pane_base_index = muxer.get_index("pane-base-index").unwrap();

        let expected = [
            "show-options -gv base-index",
            "show-options -gv pane-base-index\n",
        ]
        .join("\n");
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert_eq!(base_index, 1);
        assert_eq!(pane_base_index, 2);
    }

    #[rstest]
    fn get_index_system_error() {
        let (called_args, muxer) = mock_muxer();
        let mut muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args[..2], ["show-options", "-gv"]);
                let output = muxer.execute(args)?;
                match args[2] {
                    "base-index" => Err(std::io::Error::other("command failed")),
                    "pane-base-index" => Ok(StdOutput {
                        stdout: b"2\n".to_vec(),
                        ..output
                    }),
                    _ => Ok(output),
                }
            },
        };

        let output = muxer.get_index("base-index");

        let expected = "show-options -gv base-index\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::BaseIds(_))));
    }

    #[rstest]
    fn get_index_parse_error() {
        let (called_args, muxer) = mock_muxer();
        let mut muxer = Muxer {
            base_window_id: 0,
            base_pane_id: 0,
            command_executor: move |args| {
                assert_eq!(args[..2], ["show-options", "-gv"]);
                let output = muxer.execute(args)?;
                match args[2] {
                    "base-index" => Ok(StdOutput {
                        stdout: b"not-a-number\n".to_vec(),
                        ..output
                    }),
                    "pane-base-index" => Ok(StdOutput {
                        stdout: b"2\n".to_vec(),
                        ..output
                    }),
                    _ => Ok(output),
                }
            },
        };

        let output = muxer.get_index("base-index");

        let expected = "show-options -gv base-index\n";
        assert_eq!(*called_args.borrow(), expected.to_string());
        assert!(matches!(output, Err(Error::BaseIds(_))));
    }

    #[rstest]
    fn directory_to_string_converts_path() {
        let path = Some(PathBuf::from("/home/user"));
        let result = directory_to_string(path);
        assert_eq!(result, "/home/user".to_string());
    }

    #[rstest]
    fn directory_to_string_expands_tilde() {
        temp_env::with_var("HOME", Some("/home/user"), || {
            let path = Some(PathBuf::from("~/project"));
            let result = directory_to_string(path);
            assert_eq!(result, "/home/user/project".to_string());
        });
    }

    #[rstest]
    fn directory_to_string_none_returns_dot() {
        let result = directory_to_string(None);
        assert_eq!(result, ".".to_string());
    }

    #[rstest]
    fn expand_tilde_expands_path() {
        temp_env::with_var("HOME", Some("/home/user"), || {
            let path = PathBuf::from("~/project");
            let result = expand_tilde(path);
            assert_eq!(result, PathBuf::from("/home/user/project"));
        });
    }

    #[rstest]
    fn expand_tilde_no_tilde_returns_same_path() {
        let path = PathBuf::from("/home/user/project");
        let result = expand_tilde(path.clone());
        assert_eq!(result, path);
    }

    #[rstest]
    fn expand_tilde_only_tilde_returns_home() {
        temp_env::with_var("HOME", Some("/home/user"), || {
            let path = PathBuf::from("~");
            let result = expand_tilde(path);
            assert_eq!(result, PathBuf::from("/home/user"));
        });
    }

    #[rstest]
    fn resolve_directory_returns_pane_dir() {
        let session_dir = Some(PathBuf::from("/session"));
        let window_dir = Some(PathBuf::from("/window"));
        let pane_dir = Some(PathBuf::from("/pane"));

        let result = resolve_directory(&session_dir, &window_dir, &pane_dir);

        assert_eq!(result, Some(PathBuf::from("/pane")));
    }

    #[rstest]
    fn resolve_directory_returns_window_dir() {
        let session_dir = Some(PathBuf::from("/session"));
        let window_dir = Some(PathBuf::from("/window"));
        let pane_dir = None;

        let result = resolve_directory(&session_dir, &window_dir, &pane_dir);

        assert_eq!(result, Some(PathBuf::from("/window")));
    }

    #[rstest]
    fn resolve_directory_returns_session_dir() {
        let session_dir = Some(PathBuf::from("/session"));
        let window_dir = None;
        let pane_dir = None;

        let result = resolve_directory(&session_dir, &window_dir, &pane_dir);

        assert_eq!(result, Some(PathBuf::from("/session")));
    }

    #[rstest]
    fn resolve_directory_returns_none() {
        let session_dir = None;
        let window_dir = None;
        let pane_dir = None;

        let result = resolve_directory(&session_dir, &window_dir, &pane_dir);

        assert_eq!(result, None);
    }

    #[rstest]
    fn system_error_wraps_io_error() {
        let io_error = std::io::Error::other("command failed");
        let error = system_error(io_error);

        assert!(matches!(error, Error::System(_)));
    }

    #[rstest]
    fn system_error_source() {
        let io_error = std::io::Error::other("command failed");
        let error = system_error(io_error);

        match error {
            Error::System(source) => {
                assert_eq!(source.to_string(), "command failed".to_string());
            }
            _ => panic!("Expected Error::System"),
        }
    }
}
