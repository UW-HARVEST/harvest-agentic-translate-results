use bhshell::bhshell::{
    bhshell_cd, bhshell_execute, bhshell_exit, bhshell_help, bhshell_launch,
    bhshell_num_builtins, write_to_redirect, BUF_SIZE,
};
use bhshell::input::Command;

#[test]
fn test_buf_size_constant() {
    // C: #define BUF_SIZE 64
    assert_eq!(BUF_SIZE, 64);
}

#[test]
fn test_num_builtins() {
    // C: bhshell_num_builtins() returns sizeof(bhshell_builtin_str)/sizeof(char*) == 3
    assert_eq!(bhshell_num_builtins(), 3);
}

#[test]
fn test_exit_returns_zero() {
    // C: bhshell_exit() always returns 0.
    let args: Vec<String> = vec!["exit".to_string()];
    assert_eq!(bhshell_exit(&args), 0);
}

#[test]
fn test_exit_returns_zero_with_no_args() {
    // Even with empty args, C returns 0 (it does not even read args[0]).
    let args: Vec<String> = Vec::new();
    assert_eq!(bhshell_exit(&args), 0);
}

#[test]
fn test_exit_returns_zero_with_extra_args() {
    let args: Vec<String> = vec![
        "exit".to_string(),
        "ignored".to_string(),
        "also ignored".to_string(),
    ];
    assert_eq!(bhshell_exit(&args), 0);
}

#[test]
fn test_help_returns_one() {
    // C: bhshell_help always returns 1.
    let args: Vec<String> = vec!["help".to_string()];
    assert_eq!(bhshell_help(&args), 1);
}

#[test]
fn test_help_returns_one_with_no_args() {
    let args: Vec<String> = Vec::new();
    assert_eq!(bhshell_help(&args), 1);
}

#[test]
fn test_cd_no_argument_returns_one() {
    // C: bhshell_cd with args[1] == NULL prints an error but still returns 1.
    let args: Vec<String> = vec!["cd".to_string()];
    assert_eq!(bhshell_cd(&args), 1);
}

#[test]
fn test_cd_to_temp_returns_one() {
    // C: bhshell_cd("/tmp") returns 1 (valid chdir).
    let args: Vec<String> = vec!["cd".to_string(), "/tmp".to_string()];
    let original = std::env::current_dir().expect("get cwd");
    assert_eq!(bhshell_cd(&args), 1);
    // Restore the original directory so other tests aren't affected.
    let _ = std::env::set_current_dir(&original);
}

#[test]
fn test_cd_invalid_dir_still_returns_one() {
    // C: bhshell_cd to nonexistent dir prints error via perror but still returns 1.
    let args: Vec<String> = vec![
        "cd".to_string(),
        "/this/path/should/never/exist/xyzzy_qwerty_42".to_string(),
    ];
    assert_eq!(bhshell_cd(&args), 1);
}

#[test]
fn test_execute_empty_args_returns_one() {
    // C: bhshell_execute(cmd) with cmd->args[0] == NULL returns 1.
    let mut cmd = Command::default();
    let rc = bhshell_execute(&mut cmd);
    assert_eq!(rc, 1);
}

#[test]
fn test_execute_dispatches_exit() {
    // C: "exit" -> bhshell_exit -> 0.
    let mut cmd = Command::default();
    cmd.args = vec!["exit".to_string()];
    let rc = bhshell_execute(&mut cmd);
    assert_eq!(rc, 0);
}

#[test]
fn test_execute_dispatches_help() {
    // C: "help" -> bhshell_help -> 1.
    let mut cmd = Command::default();
    cmd.args = vec!["help".to_string()];
    let rc = bhshell_execute(&mut cmd);
    assert_eq!(rc, 1);
}

#[test]
fn test_execute_dispatches_cd_no_arg() {
    // C: "cd" -> bhshell_cd -> 1.
    let mut cmd = Command::default();
    cmd.args = vec!["cd".to_string()];
    let rc = bhshell_execute(&mut cmd);
    assert_eq!(rc, 1);
}

#[test]
fn test_execute_dispatches_cd_to_tmp() {
    // C: "cd /tmp" -> bhshell_cd -> 1, working dir changes.
    let original = std::env::current_dir().expect("get cwd");
    let mut cmd = Command::default();
    cmd.args = vec!["cd".to_string(), "/tmp".to_string()];
    let rc = bhshell_execute(&mut cmd);
    assert_eq!(rc, 1);
    let _ = std::env::set_current_dir(&original);
}

#[test]
fn test_execute_external_command_returns_one() {
    // C: bhshell_launch on a real binary returns 1 after wait.
    let mut cmd = Command::default();
    cmd.args = vec!["true".to_string()];
    let rc = bhshell_execute(&mut cmd);
    assert_eq!(rc, 1);
}

#[test]
fn test_execute_external_command_with_arg_returns_one() {
    // C: bhshell_launch on /bin/echo with an argument returns 1.
    let mut cmd = Command::default();
    cmd.args = vec!["echo".to_string(), "hi".to_string()];
    let rc = bhshell_execute(&mut cmd);
    assert_eq!(rc, 1);
}

#[test]
fn test_launch_empty_args_returns_one() {
    // bhshell_launch on empty args should not crash; should return 1.
    let mut cmd = Command::default();
    let rc = bhshell_launch(&mut cmd);
    assert_eq!(rc, 1);
}

#[test]
fn test_launch_external_command_returns_one() {
    let mut cmd = Command::default();
    cmd.args = vec!["true".to_string()];
    let rc = bhshell_launch(&mut cmd);
    assert_eq!(rc, 1);
}

#[test]
fn test_launch_with_redirect_creates_file() {
    // The Rust `write_to_redirect_from_child` writes the captured stdout to
    // the redirect file. C uses a pipe; the Rust port uses Stdio::piped()
    // and reads to a buffer, which produces the same observable file.
    let path = "/tmp/bhshell_redir_launch_test.txt";
    let _ = std::fs::remove_file(path);

    let mut cmd = Command::default();
    cmd.args = vec!["echo".to_string(), "hello".to_string()];
    cmd.redirect_file_name = Some(path.to_string());
    let rc = bhshell_launch(&mut cmd);
    assert_eq!(rc, 1);

    let content = std::fs::read_to_string(path).expect("redirected file exists");
    assert_eq!(content, "hello\n");
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_launch_with_pipe_returns_one() {
    // "echo abc | cat" in C returns 1.
    let mut cmd = Command::default();
    cmd.args = vec!["echo".to_string(), "abc".to_string()];
    cmd.pipe_args = vec!["cat".to_string()];
    let rc = bhshell_launch(&mut cmd);
    assert_eq!(rc, 1);
}

#[test]
fn test_launch_with_pipe_and_redirect() {
    // "echo abc | cat > file" should produce "abc\n" in the file and return 1.
    let path = "/tmp/bhshell_redir_pipe_test.txt";
    let _ = std::fs::remove_file(path);

    let mut cmd = Command::default();
    cmd.args = vec!["echo".to_string(), "abc".to_string()];
    cmd.pipe_args = vec!["cat".to_string()];
    cmd.redirect_file_name = Some(path.to_string());
    let rc = bhshell_launch(&mut cmd);
    assert_eq!(rc, 1);

    let content = std::fs::read_to_string(path).expect("redirected file exists");
    assert_eq!(content, "abc\n");
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_write_to_redirect_creates_file() {
    // write_to_redirect should create the file at the redirect path.
    let path = "/tmp/bhshell_write_to_redirect_test.txt";
    let _ = std::fs::remove_file(path);

    let mut cmd = Command::default();
    cmd.redirect_file_name = Some(path.to_string());
    let mut redirect_fd = [0i32; 2];
    write_to_redirect(&mut redirect_fd, &mut cmd);

    assert!(std::path::Path::new(path).exists());
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_write_to_redirect_no_path_does_nothing() {
    // With no redirect_file_name, write_to_redirect should do nothing and not panic.
    let mut cmd = Command::default();
    let mut redirect_fd = [0i32; 2];
    write_to_redirect(&mut redirect_fd, &mut cmd);
}

fn main() {}
