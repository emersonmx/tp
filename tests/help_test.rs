use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn show_help() {
    let mut cmd = Command::cargo_bin("tp")
        .unwrap()
        .arg("--help")
        .assert()
        .success();

    for c in ["load", "list"] {
        cmd = cmd.stdout(contains(c));
    }
}

#[test]
fn default_to_help() {
    let mut cmd = Command::cargo_bin("tp").unwrap();
    cmd.assert()
        .failure()
        .stderr(contains("A simple tmux project loader"));
}
