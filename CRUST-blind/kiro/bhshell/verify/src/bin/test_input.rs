use bhshell::input;

#[test]
fn test_new_command() {
    let cmd = input::new_command();
    assert!(cmd.args.is_empty());
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_empty_string() {
    let cmd = input::bhshell_parse("");
    assert!(cmd.args.is_empty());
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_simple_command() {
    let cmd = input::bhshell_parse("ls");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "ls");
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_spaces_only() {
    let cmd = input::bhshell_parse("  ");
    assert!(cmd.args.is_empty());
}

#[test]
fn test_parse_multiple_args() {
    let cmd = input::bhshell_parse("ls -abc --aad --xx wow");
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-abc");
    assert_eq!(cmd.args[2], "--aad");
    assert_eq!(cmd.args[3], "--xx");
    assert_eq!(cmd.args[4], "wow");
    assert_eq!(cmd.args.len(), 5);
}

#[test]
fn test_parse_redirect() {
    let cmd = input::bhshell_parse("ls -abc   > wow.txt");
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-abc");
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.redirect_file_name, Some("wow.txt".to_string()));
}

#[test]
fn test_parse_pipe() {
    let cmd = input::bhshell_parse("ls | wow");
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.pipe_args[0], "wow");
    assert_eq!(cmd.pipe_args.len(), 1);
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_invalid_redirect_trailing() {
    let cmd = input::bhshell_parse("ls  >   ");
    assert!(cmd.args.is_empty());
}

#[test]
fn test_parse_invalid_pipe_trailing() {
    let cmd = input::bhshell_parse("ls  |   ");
    assert!(cmd.args.is_empty());
}

#[test]
fn test_parse_newline() {
    let cmd = input::bhshell_parse("\n");
    assert!(cmd.args.is_empty());
}

#[test]
fn test_parse_pipe_and_redirect() {
    let cmd = input::bhshell_parse("ls -l | grep idk > x.txt");
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-l");
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.pipe_args[0], "grep");
    assert_eq!(cmd.pipe_args[1], "idk");
    assert_eq!(cmd.pipe_args.len(), 2);
    assert_eq!(cmd.redirect_file_name, Some("x.txt".to_string()));
}

#[test]
fn test_parse_trailing_space() {
    let cmd = input::bhshell_parse("ls ");
    assert!(cmd.args.is_empty());
}

#[test]
fn test_parse_leading_space() {
    let cmd = input::bhshell_parse(" ls");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "ls");
}

#[test]
fn test_parse_tab_separator() {
    let cmd = input::bhshell_parse("ls\t-la");
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-la");
    assert_eq!(cmd.args.len(), 2);
}

#[test]
fn test_parse_just_pipe() {
    let cmd = input::bhshell_parse("|");
    assert!(cmd.args.is_empty());
}

#[test]
fn test_parse_just_redirect() {
    let cmd = input::bhshell_parse(">");
    assert!(cmd.args.is_empty());
}

#[test]
fn test_parse_nospace_pipe() {
    let cmd = input::bhshell_parse("ls|wc");
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.pipe_args[0], "wc");
    assert_eq!(cmd.pipe_args.len(), 1);
}

#[test]
fn test_parse_nospace_redirect() {
    let cmd = input::bhshell_parse("ls>out.txt");
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.redirect_file_name, Some("out.txt".to_string()));
    assert!(cmd.pipe_args.is_empty());
}

#[test]
fn test_parse_double_pipe() {
    let cmd = input::bhshell_parse("a|b|c");
    assert!(cmd.args.is_empty());
}

#[test]
fn test_parse_double_redirect() {
    let cmd = input::bhshell_parse("a>b>c");
    assert_eq!(cmd.args[0], "a");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.redirect_file_name, Some("c".to_string()));
}

#[test]
fn test_parse_pipe_redirect_combined() {
    let cmd = input::bhshell_parse("cat file | sort > out.txt");
    assert_eq!(cmd.args[0], "cat");
    assert_eq!(cmd.args[1], "file");
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.pipe_args[0], "sort");
    assert_eq!(cmd.pipe_args.len(), 1);
    assert_eq!(cmd.redirect_file_name, Some("out.txt".to_string()));
}

#[test]
fn test_parse_multiple_spaces_between_args() {
    let cmd = input::bhshell_parse("ls   -la   -h");
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-la");
    assert_eq!(cmd.args[2], "-h");
    assert_eq!(cmd.args.len(), 3);
}

#[test]
fn test_destroy_command_does_not_panic() {
    let cmd = input::new_command();
    input::destroy_command(cmd);
}

fn main() {}
