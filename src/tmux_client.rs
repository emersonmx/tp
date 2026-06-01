use crate::muxer::{
    Client, Error, Keys, Layout, OptionName, OptionValue, PaneID, SessionId, WindowID, WindowName,
};
use std::process::Command;

#[cfg_attr(test, mockall::automock)]
pub trait CommandExecutor {
    fn execute(&self, args: Vec<String>) -> std::io::Result<std::process::Output>;
}

pub struct StdCommandExecutor;

impl CommandExecutor for StdCommandExecutor {
    fn execute(&self, args: Vec<String>) -> std::io::Result<std::process::Output> {
        Command::new("tmux").args(args).output()
    }
}

pub struct TmuxClient<E: CommandExecutor>(E);

impl<E: CommandExecutor> TmuxClient<E> {
    fn execute(&self, args: &[&str]) -> std::io::Result<std::process::Output> {
        self.0.execute(args.iter().map(|s| s.to_string()).collect())
    }
}

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

    fn switch_to_session(&mut self, session_id: &SessionId) -> Result<(), Error> {
        self.execute(&["switch-client", "-t", &session_id.to_string()])
            .map_err(system_error)?;
        Ok(())
    }

    fn has_session(&mut self, session_id: &SessionId) -> Result<bool, Error> {
        let output = self
            .execute(&["has-session", "-t", &session_id.to_string()])
            .map_err(system_error)?;

        Ok(output.status.success())
    }

    fn new_window(&mut self, session_id: &SessionId, directory: &str) -> Result<(), Error> {
        self.execute(&["new-window", "-c", directory, "-t", &session_id.to_string()])
            .map_err(system_error)?;
        Ok(())
    }

    fn rename_window(
        &mut self,
        window_id: &WindowID,
        window_name: &WindowName,
    ) -> Result<(), Error> {
        self.execute(&[
            "rename-window",
            "-t",
            &window_id.to_string(),
            window_name.value(),
        ])
        .map_err(system_error)?;
        Ok(())
    }

    fn new_pane(&mut self, window_id: &WindowID, directory: &str) -> Result<(), Error> {
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

    fn select_pane(&mut self, pane_id: &PaneID) -> Result<(), Error> {
        let window_id = pane_id.window_id();
        self.execute(&["select-window", "-t", &window_id.to_string()])
            .map_err(system_error)?;

        self.execute(&["select-pane", "-t", &pane_id.to_string()])
            .map_err(system_error)?;
        Ok(())
    }

    fn send_keys(&mut self, pane_id: &PaneID, keys: Keys) -> Result<(), Error> {
        self.execute(&["send-keys", "-t", &pane_id.to_string(), keys.value(), "C-m"])
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use std::os::unix::process::ExitStatusExt;

    #[fixture]
    fn executor() -> MockCommandExecutor {
        MockCommandExecutor::new()
    }

    #[rstest]
    fn should_detect_tmux_environment(executor: MockCommandExecutor) {
        let mut client = TmuxClient(executor);

        // Not running inside tmux
        temp_env::with_var_unset("TMUX", || {
            assert!(!client.is_running_inside_tmux());
        });

        // Running inside tmux
        temp_env::with_var("TMUX", Some("test"), || {
            assert!(client.is_running_inside_tmux());
        });
    }

    #[rstest]
    fn should_get_option(mut executor: MockCommandExecutor) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["show-options", "-gv", "test-option"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| {
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: b"test-value\n".to_vec(),
                    stderr: Vec::new(),
                })
            });

        let mut client = TmuxClient(executor);

        let option_name = OptionName::new("test-option");
        let expected_value = "test-value";

        let result = client.get_option(&option_name).unwrap();

        assert_eq!(result.value(), expected_value);
    }

    #[rstest]
    fn get_option_should_return_option_not_found_for_nonexistent_option(
        mut executor: MockCommandExecutor,
    ) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["show-options", "-gv", "nonexistent-option"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| {
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(1),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            });

        let mut client = TmuxClient(executor);

        let option_name = OptionName::new("nonexistent-option");

        let result = client.get_option(&option_name);

        assert!(matches!(
            result,
            Err(Error::OptionNotFound(name)) if name == "nonexistent-option"
        ));
    }

    #[rstest]
    fn get_option_should_return_system_error_on_command_failure(mut executor: MockCommandExecutor) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["show-options", "-gv", "test-option"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| Err(std::io::Error::other("system error")));

        let mut client = TmuxClient(executor);

        let option_name = OptionName::new("test-option");

        let result = client.get_option(&option_name);

        assert!(matches!(result, Err(Error::System(_))));
    }

    // TODO: Add tests for set_option when implemented

    #[rstest]
    fn new_session_should_create_session(mut executor: MockCommandExecutor) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &[
                    "new-session",
                    "-d",
                    "-c",
                    "/test/directory",
                    "-s",
                    "test-session",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
            })
            .returning(|_| {
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            });

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");
        let directory = "/test/directory";

        let result = client.new_session(&session_id, directory);

        assert!(result.is_ok());
    }

    #[rstest]
    fn new_session_should_return_system_error_on_command_failure(
        mut executor: MockCommandExecutor,
    ) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &[
                    "new-session",
                    "-d",
                    "-c",
                    "/test/directory",
                    "-s",
                    "test-session",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
            })
            .returning(|_| Err(std::io::Error::other("system error")));

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");
        let directory = "/test/directory";

        let result = client.new_session(&session_id, directory);

        assert!(matches!(result, Err(Error::System(_))));
    }

    #[rstest]
    fn switch_to_session_should_switch_session(mut executor: MockCommandExecutor) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["switch-client", "-t", "test-session"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| {
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            });

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");

        let result = client.switch_to_session(&session_id);

        assert!(result.is_ok());
    }

    #[rstest]
    fn switch_to_session_should_return_system_error_on_command_failure(
        mut executor: MockCommandExecutor,
    ) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["switch-client", "-t", "test-session"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| Err(std::io::Error::other("system error")));

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");

        let result = client.switch_to_session(&session_id);

        assert!(matches!(result, Err(Error::System(_))));
    }

    #[rstest]
    fn has_session_should_return_true_if_session_exists(mut executor: MockCommandExecutor) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["has-session", "-t", "test-session"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| {
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            });

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");

        let result = client.has_session(&session_id).unwrap();

        assert!(result);
    }

    #[rstest]
    fn has_session_should_return_false_if_session_does_not_exist(
        mut executor: MockCommandExecutor,
    ) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["has-session", "-t", "test-session"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| {
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(1),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            });

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");

        let result = client.has_session(&session_id).unwrap();

        assert!(!result);
    }

    #[rstest]
    fn has_session_should_return_system_error_on_command_failure(
        mut executor: MockCommandExecutor,
    ) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["has-session", "-t", "test-session"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| Err(std::io::Error::other("system error")));

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");

        let result = client.has_session(&session_id);

        assert!(matches!(result, Err(Error::System(_))));
    }

    #[rstest]
    fn new_window_should_create_window(mut executor: MockCommandExecutor) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["new-window", "-c", "/test/directory", "-t", "test-session"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| {
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            });

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");
        let directory = "/test/directory";

        let result = client.new_window(&session_id, directory);

        assert!(result.is_ok());
    }

    #[rstest]
    fn new_window_should_return_system_error_on_command_failure(mut executor: MockCommandExecutor) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["new-window", "-c", "/test/directory", "-t", "test-session"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| Err(std::io::Error::other("system error")));

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");
        let directory = "/test/directory";

        let result = client.new_window(&session_id, directory);

        assert!(matches!(result, Err(Error::System(_))));
    }

    #[rstest]
    fn rename_window_should_rename_window(mut executor: MockCommandExecutor) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["rename-window", "-t", "test-session:0", "new-window-name"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| {
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            });

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");
        let window_id = WindowID::new(&session_id, "0");
        let window_name = WindowName::new("new-window-name");

        let result = client.rename_window(&window_id, &window_name);

        assert!(result.is_ok());
    }

    #[rstest]
    fn rename_window_should_return_system_error_on_command_failure(
        mut executor: MockCommandExecutor,
    ) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["rename-window", "-t", "test-session:0", "new-window-name"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| Err(std::io::Error::other("system error")));

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");
        let window_id = WindowID::new(&session_id, "0");
        let window_name = WindowName::new("new-window-name");

        let result = client.rename_window(&window_id, &window_name);

        assert!(matches!(result, Err(Error::System(_))));
    }

    #[rstest]
    fn new_pane_should_create_pane(mut executor: MockCommandExecutor) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &[
                    "split-window",
                    "-c",
                    "/test/directory",
                    "-t",
                    "test-session:0",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
            })
            .returning(|_| {
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            });

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");
        let window_id = WindowID::new(&session_id, "0");
        let directory = "/test/directory";

        let result = client.new_pane(&window_id, directory);

        assert!(result.is_ok());
    }

    #[rstest]
    fn new_pane_should_return_system_error_on_command_failure(mut executor: MockCommandExecutor) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &[
                    "split-window",
                    "-c",
                    "/test/directory",
                    "-t",
                    "test-session:0",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
            })
            .returning(|_| Err(std::io::Error::other("system error")));

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");
        let window_id = WindowID::new(&session_id, "0");
        let directory = "/test/directory";

        let result = client.new_pane(&window_id, directory);

        assert!(matches!(result, Err(Error::System(_))));
    }

    #[rstest]
    fn select_pane_should_select_pane(mut executor: MockCommandExecutor) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["select-window", "-t", "test-session:0"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| {
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            });

        executor
            .expect_execute()
            .withf(|args| {
                args == &["select-pane", "-t", "test-session:0.1"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| {
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            });

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");
        let window_id = WindowID::new(&session_id, "0");
        let pane_id = PaneID::new(&window_id, "1");

        let result = client.select_pane(&pane_id);

        assert!(result.is_ok());
    }

    #[rstest]
    fn select_pane_should_return_system_error_on_command_failure(
        mut executor: MockCommandExecutor,
    ) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["select-window", "-t", "test-session:0"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| Err(std::io::Error::other("system error")));

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");
        let window_id = WindowID::new(&session_id, "0");
        let pane_id = PaneID::new(&window_id, "1");

        let result = client.select_pane(&pane_id);

        assert!(matches!(result, Err(Error::System(_))));
    }

    #[rstest]
    fn send_keys_should_send_keys_to_pane(mut executor: MockCommandExecutor) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["send-keys", "-t", "test-session:0.1", "test-keys", "C-m"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| {
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            });

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");
        let window_id = WindowID::new(&session_id, "0");
        let pane_id = PaneID::new(&window_id, "1");
        let keys = Keys::new("test-keys");

        let result = client.send_keys(&pane_id, keys);

        assert!(result.is_ok());
    }

    #[rstest]
    fn send_keys_should_return_system_error_on_command_failure(mut executor: MockCommandExecutor) {
        executor
            .expect_execute()
            .withf(|args| {
                args == &["send-keys", "-t", "test-session:0.1", "test-keys", "C-m"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .returning(|_| Err(std::io::Error::other("system error")));

        let mut client = TmuxClient(executor);

        let session_id = SessionId::new("test-session");
        let window_id = WindowID::new(&session_id, "0");
        let pane_id = PaneID::new(&window_id, "1");
        let keys = Keys::new("test-keys");

        let result = client.send_keys(&pane_id, keys);

        assert!(matches!(result, Err(Error::System(_))));
    }

    // TODO: Add tests for use_layout when implemented
}
