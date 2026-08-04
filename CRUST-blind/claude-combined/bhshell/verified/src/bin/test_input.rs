use bhshell::input::{self, bhshell_parse, destroy_command, new_command, Command};

#[test]
fn test_new_command_default() {
    let c: Command = new_command();
    assert!(c.args.is_empty());
    assert!(c.pipe_args.is_empty());
    assert!(c.redirect_file_name.is_none());
}

#[test]
fn test_destroy_command_does_not_panic() {
    let c = new_command();
    destroy_command(c);
}

#[test]
fn test_parse_empty() {
    let cmd = bhshell_parse("");
    // C returns NULL — Rust default Command (empty args) is the equivalent.
    assert!(cmd.args.is_empty());
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_only_spaces() {
    let cmd = bhshell_parse("  ");
    assert!(cmd.args.is_empty());
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_only_newline() {
    let cmd = bhshell_parse("\n");
    assert!(cmd.args.is_empty());
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_simple() {
    let cmd = bhshell_parse("ls");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "ls");
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_simple_with_arg() {
    let cmd = bhshell_parse("ls -l");
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-l");
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_multiple_args() {
    let cmd = bhshell_parse("ls -abc --aad --xx wow");
    assert_eq!(cmd.args.len(), 5);
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-abc");
    assert_eq!(cmd.args[2], "--aad");
    assert_eq!(cmd.args[3], "--xx");
    assert_eq!(cmd.args[4], "wow");
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_redirect() {
    let cmd = bhshell_parse("ls -abc   > wow.txt");
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-abc");
    assert!(cmd.pipe_args.is_empty());
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("wow.txt"));
}

#[test]
fn test_parse_pipe() {
    let cmd = bhshell_parse("ls | wow");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.pipe_args.len(), 1);
    assert_eq!(cmd.pipe_args[0], "wow");
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_invalid_redirect_no_target() {
    let cmd = bhshell_parse("ls  >   ");
    // C returns NULL for trailing redirect with no filename
    assert!(cmd.args.is_empty());
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_invalid_pipe_no_target() {
    let cmd = bhshell_parse("ls  |   ");
    // C returns NULL for trailing pipe with no command
    assert!(cmd.args.is_empty());
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_pipe_with_redirect() {
    let cmd = bhshell_parse("ls -l | grep idk > x.txt");
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-l");
    assert_eq!(cmd.pipe_args.len(), 2);
    assert_eq!(cmd.pipe_args[0], "grep");
    assert_eq!(cmd.pipe_args[1], "idk");
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("x.txt"));
}

#[test]
fn test_parse_pipe_with_args() {
    let cmd = bhshell_parse("cmd1 | cmd2 arg1 arg2");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "cmd1");
    assert_eq!(cmd.pipe_args.len(), 3);
    assert_eq!(cmd.pipe_args[0], "cmd2");
    assert_eq!(cmd.pipe_args[1], "arg1");
    assert_eq!(cmd.pipe_args[2], "arg2");
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_redirect_no_spaces() {
    // Equivalent C output: args=[x], redirect=y
    let cmd = bhshell_parse("x>y");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "x");
    assert!(cmd.pipe_args.is_empty());
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("y"));
}

#[test]
fn test_parse_pipe_no_spaces() {
    // Equivalent C output: args=[x], pipe_args=[y]
    let cmd = bhshell_parse("x|y");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "x");
    assert_eq!(cmd.pipe_args.len(), 1);
    assert_eq!(cmd.pipe_args[0], "y");
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_complex() {
    let cmd = bhshell_parse("a b c | d e f > out.txt");
    assert_eq!(cmd.args, vec!["a", "b", "c"]);
    assert_eq!(cmd.pipe_args, vec!["d", "e", "f"]);
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("out.txt"));
}

#[test]
fn test_parse_cd_command() {
    let cmd = bhshell_parse("cd /tmp");
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.args[0], "cd");
    assert_eq!(cmd.args[1], "/tmp");
}

#[test]
fn test_parse_help_command() {
    let cmd = bhshell_parse("help");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "help");
}

#[test]
fn test_parse_exit_command() {
    let cmd = bhshell_parse("exit");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "exit");
}

#[test]
fn test_parse_echo_redirect() {
    let cmd = bhshell_parse("echo > file");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "echo");
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("file"));
}

#[test]
fn test_module_function_exists() {
    let _: fn() -> String = input::bhshell_read_line;
}

fn main() {}
