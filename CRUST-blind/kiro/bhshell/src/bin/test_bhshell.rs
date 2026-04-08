use bhshell::bhshell::{bhshell_num_builtins, bhshell_exit, bhshell_cd, bhshell_help};

#[test]
fn test_num_builtins() {
    assert_eq!(bhshell_num_builtins(), 3);
}

#[test]
fn test_exit_returns_zero() {
    let args = vec!["exit".to_string()];
    assert_eq!(bhshell_exit(&args), 0);
}

#[test]
fn test_cd_returns_one() {
    let args = vec!["cd".to_string(), "/tmp".to_string()];
    assert_eq!(bhshell_cd(&args), 1);
}

#[test]
fn test_cd_no_arg_returns_one() {
    let args = vec!["cd".to_string()];
    assert_eq!(bhshell_cd(&args), 1);
}

#[test]
fn test_help_returns_one() {
    let args = vec!["help".to_string()];
    assert_eq!(bhshell_help(&args), 1);
}

#[test]
fn test_cd_invalid_dir_returns_one() {
    let args = vec!["cd".to_string(), "/nonexistent_dir_xyz_12345".to_string()];
    assert_eq!(bhshell_cd(&args), 1);
}

fn main() {}
