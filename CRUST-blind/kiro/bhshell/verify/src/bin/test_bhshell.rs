use ::bhshell::bhshell;
use ::bhshell::input;

#[test]
fn test_num_builtins() {
    assert_eq!(bhshell::bhshell_num_builtins(), 3);
}

#[test]
fn test_exit_returns_zero() {
    let args = vec!["exit".to_string()];
    assert_eq!(bhshell::bhshell_exit(&args), 0);
}

#[test]
fn test_help_returns_one() {
    let args = vec!["help".to_string()];
    assert_eq!(bhshell::bhshell_help(&args), 1);
}

#[test]
fn test_cd_no_arg_returns_one() {
    let args = vec!["cd".to_string()];
    assert_eq!(bhshell::bhshell_cd(&args), 1);
}

#[test]
fn test_execute_empty_args() {
    let mut cmd = input::Command {
        args: vec![],
        pipe_args: vec![],
        redirect_file_name: None,
    };
    assert_eq!(bhshell::bhshell_execute(&mut cmd), 1);
}

#[test]
fn test_execute_empty_first_arg() {
    let mut cmd = input::Command {
        args: vec!["".to_string()],
        pipe_args: vec![],
        redirect_file_name: None,
    };
    assert_eq!(bhshell::bhshell_execute(&mut cmd), 1);
}

#[test]
fn test_execute_exit() {
    let mut cmd = input::Command {
        args: vec!["exit".to_string()],
        pipe_args: vec![],
        redirect_file_name: None,
    };
    assert_eq!(bhshell::bhshell_execute(&mut cmd), 0);
}

#[test]
fn test_execute_help() {
    let mut cmd = input::Command {
        args: vec!["help".to_string()],
        pipe_args: vec![],
        redirect_file_name: None,
    };
    assert_eq!(bhshell::bhshell_execute(&mut cmd), 1);
}

#[test]
fn test_cd_with_valid_dir() {
    let args = vec!["cd".to_string(), "/tmp".to_string()];
    assert_eq!(bhshell::bhshell_cd(&args), 1);
}

#[test]
fn test_cd_with_invalid_dir() {
    let args = vec!["cd".to_string(), "/nonexistent_dir_xyz".to_string()];
    assert_eq!(bhshell::bhshell_cd(&args), 1);
}

fn main() {}
