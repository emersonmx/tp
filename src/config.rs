use std::{env, path::PathBuf};

pub fn project_dir() -> Option<PathBuf> {
    match env::var("HOME") {
        Ok(e) => Some(PathBuf::from(e).join(".config/tp")),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_project_dir() {
        let tp_home = "/home/tp";
        unsafe { env::set_var("HOME", tp_home) };

        assert_eq!(
            project_dir(),
            Some(PathBuf::from(format!("{tp_home}/.config/tp")))
        );
    }

    #[test]
    fn error_when_invalid_home_dir() {
        unsafe { env::remove_var("HOME") };

        assert_eq!(project_dir(), None);
    }
}
