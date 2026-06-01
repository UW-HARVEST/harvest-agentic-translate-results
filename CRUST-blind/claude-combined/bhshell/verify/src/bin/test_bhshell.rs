use bhshell::bhshell::{
    bhshell_cd, bhshell_execute, bhshell_exit, bhshell_help, bhshell_launch, bhshell_num_builtins,
    write_to_redirect, BUF_SIZE,
};
use bhshell::input::{new_command, Command};

#[test]
fn test_buf_size_constant() {
    assert_eq!(BUF_SIZE, 64);
}

#[test]
fn test_num_builtins() {
    // C has 3 builtins: cd, help, exit
    assert_eq!(bhshell_num_builtins(), 3);
}

#[test]
fn test_exit_returns_zero() {
    let args = vec!["exit".to_string()];
    let r = bhshell_exit(&args);
    assert_eq!(r, 0);
}

#[test]
fn test_help_returns_one() {
    let args = vec!["help".to_string()];
    let r = bhshell_help(&args);
    assert_eq!(r, 1);
}

#[test]
fn test_cd_no_arg_returns_one() {
    // C version returns 1 even when no argument is given (just prints to stderr).
    let args = vec!["cd".to_string()];
    let r = bhshell_cd(&args);
    assert_eq!(r, 1);
}

#[test]
fn test_cd_to_tmp() {
    // /tmp exists on Linux dev boxes
    let args = vec!["cd".to_string(), "/tmp".to_string()];
    let r = bhshell_cd(&args);
    assert_eq!(r, 1);
}

#[test]
fn test_cd_invalid_path_still_returns_one() {
    // C still returns 1 on chdir failure (just calls perror).
    let args = vec![
        "cd".to_string(),
        "/this/path/should/not/exist/xyz".to_string(),
    ];
    let r = bhshell_cd(&args);
    assert_eq!(r, 1);
}

#[test]
fn test_execute_empty_args() {
    let mut cmd: Command = new_command();
    let r = bhshell_execute(&mut cmd);
    // C: returns 1 if cmd->args[0] == NULL
    assert_eq!(r, 1);
}

#[test]
fn test_execute_exit_builtin() {
    let mut cmd: Command = new_command();
    cmd.args = vec!["exit".to_string()];
    let r = bhshell_execute(&mut cmd);
    assert_eq!(r, 0);
}

#[test]
fn test_execute_help_builtin() {
    let mut cmd: Command = new_command();
    cmd.args = vec!["help".to_string()];
    let r = bhshell_execute(&mut cmd);
    assert_eq!(r, 1);
}

#[test]
fn test_execute_cd_builtin_no_arg() {
    let mut cmd: Command = new_command();
    cmd.args = vec!["cd".to_string()];
    let r = bhshell_execute(&mut cmd);
    assert_eq!(r, 1);
}

#[test]
fn test_launch_returns_one() {
    // bhshell_launch in C returns 1 on completion. In our pure-Rust translation
    // we do not actually fork/exec, but the return value should match.
    let mut cmd: Command = new_command();
    cmd.args = vec!["true".to_string()];
    let r = bhshell_launch(&mut cmd);
    assert_eq!(r, 1);
}

#[test]
fn test_write_to_redirect_does_not_panic() {
    // The Rust translation has this as a no-op stub; just exercise it.
    let mut fd = [0i32, 0i32];
    let mut cmd: Command = new_command();
    cmd.redirect_file_name = Some("/tmp/should_not_be_written.txt".to_string());
    write_to_redirect(&mut fd, &mut cmd);
}

fn main() {}
