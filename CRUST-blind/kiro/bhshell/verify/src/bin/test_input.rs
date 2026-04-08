use bhshell::input::{bhshell_parse, new_command, destroy_command, Command};

#[test]
fn test_parse_empty_string() {
    let cmd = bhshell_parse("");
    assert!(cmd.args.is_empty());
}

#[test]
fn test_parse_simple_command() {
    let cmd = bhshell_parse("ls");
    assert_eq!(cmd.args, vec!["ls"]);
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_spaces_only() {
    let cmd = bhshell_parse("  ");
    assert!(cmd.args.is_empty());
}

#[test]
fn test_parse_multiple_args() {
    let cmd = bhshell_parse("ls -abc --aad --xx wow");
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-abc");
    assert_eq!(cmd.args[2], "--aad");
    assert_eq!(cmd.args[3], "--xx");
    assert_eq!(cmd.args[4], "wow");
}

#[test]
fn test_parse_redirect() {
    let cmd = bhshell_parse("ls -abc   > wow.txt");
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-abc");
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("wow.txt"));
}

#[test]
fn test_parse_pipe() {
    let cmd = bhshell_parse("ls | wow");
    assert_eq!(cmd.args, vec!["ls"]);
    assert_eq!(cmd.pipe_args, vec!["wow"]);
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_invalid_redirect_no_file() {
    let cmd = bhshell_parse("ls  >   ");
    assert!(cmd.args.is_empty());
}

#[test]
fn test_parse_invalid_pipe_no_command() {
    let cmd = bhshell_parse("ls  |   ");
    assert!(cmd.args.is_empty());
}

#[test]
fn test_parse_newline() {
    let cmd = bhshell_parse("\n");
    assert!(cmd.args.is_empty());
}

#[test]
fn test_parse_pipe_and_redirect() {
    let cmd = bhshell_parse("ls -l | grep idk > x.txt");
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-l");
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.pipe_args[0], "grep");
    assert_eq!(cmd.pipe_args[1], "idk");
    assert_eq!(cmd.pipe_args.len(), 2);
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("x.txt"));
}

#[test]
fn test_parse_tabs() {
    let cmd = bhshell_parse("ls\t-l");
    assert_eq!(cmd.args, vec!["ls", "-l"]);
}

#[test]
fn test_new_command() {
    let cmd = new_command();
    assert!(cmd.args.is_empty());
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_destroy_command_no_panic() {
    let cmd = Command {
        args: vec!["ls".to_string()],
        pipe_args: Vec::new(),
        redirect_file_name: None,
    };
    destroy_command(cmd);
}

#[test]
fn test_parse_single_char() {
    let cmd = bhshell_parse("a");
    assert_eq!(cmd.args, vec!["a"]);
}

fn main() {}
