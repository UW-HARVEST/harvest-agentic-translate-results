use bhshell::input::{bhshell_parse, destroy_command, new_command, Command};

/// In the C code, `bhshell_parse` returns NULL on invalid input.
/// In the Rust translation, this is represented by a Command with empty `args`.
fn is_invalid(cmd: &Command) -> bool {
    cmd.args.is_empty()
}

#[test]
fn test_new_command_returns_empty() {
    let c = new_command();
    assert!(c.args.is_empty());
    assert!(c.pipe_args.is_empty());
    assert!(c.redirect_file_name.is_none());
}

#[test]
fn test_command_default() {
    let c = Command::default();
    assert!(c.args.is_empty());
    assert!(c.pipe_args.is_empty());
    assert!(c.redirect_file_name.is_none());
}

#[test]
fn test_destroy_command_does_not_panic() {
    let c = new_command();
    destroy_command(c);
    let mut c2 = Command::default();
    c2.args = vec!["x".to_string()];
    c2.pipe_args = vec!["y".to_string()];
    c2.redirect_file_name = Some("z".to_string());
    destroy_command(c2);
}

#[test]
fn test_parse_empty() {
    // C: bhshell_parse("") returns NULL.
    let cmd = bhshell_parse("");
    assert!(is_invalid(&cmd));
    assert!(cmd.args.is_empty());
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_only_space() {
    // C: " " returns NULL.
    let cmd = bhshell_parse(" ");
    assert!(is_invalid(&cmd));
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_only_spaces() {
    // C: "  " returns NULL.
    let cmd = bhshell_parse("  ");
    assert!(is_invalid(&cmd));
}

#[test]
fn test_parse_only_newline() {
    // C: "\n" returns NULL.
    let cmd = bhshell_parse("\n");
    assert!(is_invalid(&cmd));
}

#[test]
fn test_parse_only_tab() {
    // C: "\t" returns NULL.
    let cmd = bhshell_parse("\t");
    assert!(is_invalid(&cmd));
}

#[test]
fn test_parse_simple() {
    // C: "ls" -> args=["ls"], pipe_args=NULL, redirect=NULL.
    let cmd = bhshell_parse("ls");
    assert!(!is_invalid(&cmd));
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "ls");
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_two_args() {
    // C: "ls -l" -> args=["ls","-l"]
    let cmd = bhshell_parse("ls -l");
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-l");
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_multiple() {
    // C: "ls -abc --aad --xx wow" -> 5 args
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
    // C: "ls -abc   > wow.txt" -> args=["ls","-abc"], redirect="wow.txt"
    let cmd = bhshell_parse("ls -abc   > wow.txt");
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-abc");
    assert!(cmd.pipe_args.is_empty());
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("wow.txt"));
}

#[test]
fn test_parse_redirect_simple() {
    // C: "ls > wow.txt" -> args=["ls"], redirect="wow.txt"
    let cmd = bhshell_parse("ls > wow.txt");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "ls");
    assert!(cmd.pipe_args.is_empty());
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("wow.txt"));
}

#[test]
fn test_parse_pipe() {
    // C: "ls | wow" -> args=["ls"], pipe_args=["wow"]
    let cmd = bhshell_parse("ls | wow");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.pipe_args.len(), 1);
    assert_eq!(cmd.pipe_args[0], "wow");
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_pipe_redirect_full() {
    // C: "ls -l | grep idk > x.txt" -> args=["ls","-l"], pipe_args=["grep","idk"], redirect="x.txt"
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
fn test_parse_invalid_redirect_no_target() {
    // C: "ls  >   " -> NULL (trailing whitespace, no target gathered)
    let cmd = bhshell_parse("ls  >   ");
    assert!(is_invalid(&cmd));
}

#[test]
fn test_parse_invalid_pipe_no_target() {
    // C: "ls  |   " -> NULL
    let cmd = bhshell_parse("ls  |   ");
    assert!(is_invalid(&cmd));
}

#[test]
fn test_parse_trailing_space_invalid() {
    // C: "ls -l " -> NULL because final s.position == 0
    let cmd = bhshell_parse("ls -l ");
    assert!(is_invalid(&cmd));
}

#[test]
fn test_parse_trailing_space_simple() {
    // C: "ls " -> NULL because final s.position == 0
    let cmd = bhshell_parse("ls ");
    assert!(is_invalid(&cmd));
}

#[test]
fn test_parse_leading_space() {
    // C: " ls" -> args=["ls"]
    let cmd = bhshell_parse(" ls");
    assert!(!is_invalid(&cmd));
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "ls");
}

#[test]
fn test_parse_leading_space_two_args() {
    // C: " ls -l" -> args=["ls","-l"]
    let cmd = bhshell_parse(" ls -l");
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-l");
}

#[test]
fn test_parse_trailing_newline_invalid() {
    // C: "ls -l\n" -> NULL, because '\n' is treated like space, leading to s.position==0 at end.
    let cmd = bhshell_parse("ls -l\n");
    assert!(is_invalid(&cmd));
}

#[test]
fn test_parse_double_space_separator() {
    // C: "ls  -l" -> args=["ls","-l"]
    let cmd = bhshell_parse("ls  -l");
    assert_eq!(cmd.args.len(), 2);
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.args[1], "-l");
}

#[test]
fn test_parse_pipe_no_spaces() {
    // C: "a|b" -> args=["a"], pipe_args=["b"]
    let cmd = bhshell_parse("a|b");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "a");
    assert_eq!(cmd.pipe_args.len(), 1);
    assert_eq!(cmd.pipe_args[0], "b");
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_pipe_just_trailing_space_invalid() {
    // C: "a |" -> NULL because s.position == 0 at the end
    let cmd = bhshell_parse("a |");
    assert!(is_invalid(&cmd));
}

#[test]
fn test_parse_redirect_no_space() {
    // C: "a >b" -> args=["a"], redirect="b"
    let cmd = bhshell_parse("a >b");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "a");
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("b"));
    assert!(cmd.pipe_args.is_empty());
}

#[test]
fn test_parse_redirect_only_invalid() {
    // C: ">file" -> args==NULL (no args were collected), so NULL
    let cmd = bhshell_parse(">file");
    assert!(is_invalid(&cmd));
}

#[test]
fn test_parse_pipe_only_invalid() {
    // C: "|cmd" -> args==NULL → NULL
    let cmd = bhshell_parse("|cmd");
    assert!(is_invalid(&cmd));
}

#[test]
fn test_parse_redirect_no_space_to_target() {
    // C: "cmd >file" -> args=["cmd"], redirect="file"
    let cmd = bhshell_parse("cmd >file");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "cmd");
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("file"));
}

#[test]
fn test_parse_pipe_redirect_no_space() {
    // C: "cmd|wow >file" -> args=["cmd"], pipe_args=["wow"], redirect="file"
    let cmd = bhshell_parse("cmd|wow >file");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "cmd");
    assert_eq!(cmd.pipe_args.len(), 1);
    assert_eq!(cmd.pipe_args[0], "wow");
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("file"));
}

#[test]
fn test_parse_redirect_takes_last_word() {
    // C: "cmd > a b" -> the parser keeps overwriting `redirect` when more
    // tokens follow the '>' marker, so final redirect="b".
    let cmd = bhshell_parse("cmd > a b");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "cmd");
    assert!(cmd.pipe_args.is_empty());
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("b"));
}

#[test]
fn test_parse_double_redirect() {
    // C: "ls > wow > foo" -> args=["ls"], redirect="foo"
    // After first '>', `current = REDIRECT`. The token "wow" assigns redirect="wow".
    // Then second '>': because `current == REDIRECT`, the loop body branch for '>' does nothing
    // before resetting `current = REDIRECT`. Then "foo" sets redirect="foo".
    let cmd = bhshell_parse("ls > wow > foo");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.redirect_file_name.as_deref(), Some("foo"));
}

#[test]
fn test_parse_double_pipe_invalid_via_two_pipes() {
    // C: "ls  |  | wow" -> args=["ls"], pipe_args=["wow"]
    // First '|' transitions ARG->PIPE_ARG; second '|' has s.position==0 (just spaces between),
    // so it doesn't try to flush a string and therefore stays in PIPE_ARG, producing pipe_args=["wow"].
    let cmd = bhshell_parse("ls  |  | wow");
    assert_eq!(cmd.args.len(), 1);
    assert_eq!(cmd.args[0], "ls");
    assert_eq!(cmd.pipe_args.len(), 1);
    assert_eq!(cmd.pipe_args[0], "wow");
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_only_tabs_in_args_invalid() {
    // C: "\tls\t-l\t" -> NULL because final s.position==0.
    let cmd = bhshell_parse("\tls\t-l\t");
    assert!(is_invalid(&cmd));
}

#[test]
fn test_parse_alphabet_args() {
    // C: "a b c d e" -> 5 args
    let cmd = bhshell_parse("a b c d e");
    assert_eq!(cmd.args.len(), 5);
    assert_eq!(cmd.args[0], "a");
    assert_eq!(cmd.args[1], "b");
    assert_eq!(cmd.args[2], "c");
    assert_eq!(cmd.args[3], "d");
    assert_eq!(cmd.args[4], "e");
    assert!(cmd.pipe_args.is_empty());
    assert!(cmd.redirect_file_name.is_none());
}

#[test]
fn test_parse_two_pipes_invalid() {
    // C: "ls|grep|wow" -> NULL.
    // First '|' moves to PIPE_ARG. Then "grep" appended. Then second '|':
    // current==PIPE_ARG, s.position>0, branch returns NULL.
    let cmd = bhshell_parse("ls|grep|wow");
    assert!(is_invalid(&cmd));
}

#[test]
fn test_parse_redirect_alone_with_space_invalid() {
    // C: "> " -> NULL
    let cmd = bhshell_parse("> ");
    assert!(is_invalid(&cmd));
}

fn main() {}
