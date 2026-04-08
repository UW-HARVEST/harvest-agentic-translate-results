use hydra::hydra::*;

#[test]
fn test_constants() {
    assert_eq!(RED, "\x1b[0;31m");
    assert_eq!(WHITE, "\x1b[0;37m");
    assert_eq!(COLOR_OFF, "\x1b[0m");
    assert_eq!(PURPLE, "\x1b[0;35m");
    assert_eq!(GREEN, "\x1b[0;32m");
    assert_eq!(YELLOW, "\x1b[0;33m");
    assert_eq!(BLUE, "\x1b[0;34m");
    assert_eq!(CYAN, "\x1b[0;36m");
    assert_eq!(RIGHT_MARGIN, 5);
    assert_eq!(DEFAULT_NAME, "unnamed");
}

#[test]
fn test_new_command() {
    let cmd = Command::new('k', "name".to_string(), "command".to_string());
    assert_eq!(cmd.key, 'k');
    assert_eq!(cmd.name, "name");
    assert_eq!(cmd.command, "command");
    assert!(cmd.children.is_none());
    assert!(cmd.next.is_none());
}

#[test]
fn test_command_add_child_sorted() {
    let mut parent = Command::new('\0', "root".to_string(), String::new());
    command_add_child(&mut parent, Command::new('c', "charlie".to_string(), "cmd_c".to_string()));
    command_add_child(&mut parent, Command::new('a', "alpha".to_string(), "cmd_a".to_string()));
    command_add_child(&mut parent, Command::new('b', "bravo".to_string(), "cmd_b".to_string()));

    let first = parent.children.as_ref().unwrap();
    assert_eq!(first.key, 'a');
    assert_eq!(first.name, "alpha");
    let second = first.next.as_ref().unwrap();
    assert_eq!(second.key, 'b');
    assert_eq!(second.name, "bravo");
    let third = second.next.as_ref().unwrap();
    assert_eq!(third.key, 'c');
    assert_eq!(third.name, "charlie");
    assert!(third.next.is_none());
}

#[test]
fn test_command_add_child_duplicate_keys() {
    let mut parent = Command::new('\0', "root".to_string(), String::new());
    command_add_child(&mut parent, Command::new('a', "first_a".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('a', "second_a".to_string(), String::new()));

    let first = parent.children.as_ref().unwrap();
    assert_eq!(first.key, 'a');
    assert_eq!(first.name, "first_a");
    let second = first.next.as_ref().unwrap();
    assert_eq!(second.key, 'a');
    assert_eq!(second.name, "second_a");
    assert!(second.next.is_none());
}

#[test]
fn test_find_command() {
    let mut parent = Command::new('\0', "root".to_string(), String::new());
    command_add_child(&mut parent, Command::new('a', "alpha".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('b', "bravo".to_string(), String::new()));

    let found = find_command(&parent, 'b');
    assert!(found.is_some());
    assert_eq!(found.unwrap().key, 'b');
    assert_eq!(found.unwrap().name, "bravo");

    let not_found = find_command(&parent, 'z');
    assert!(not_found.is_none());
}

#[test]
fn test_command_run_nonempty() {
    let cmd = Command::new('x', "test".to_string(), "hello world".to_string());
    let ret = command_run(&cmd);
    assert_eq!(ret, 11);
}

#[test]
fn test_command_run_empty() {
    let cmd = Command::new('y', "empty".to_string(), String::new());
    let ret = command_run(&cmd);
    assert_eq!(ret, 0);
}

#[test]
fn test_command_run_long() {
    let cmd = Command::new('x', "test".to_string(), "abcdefghij".to_string());
    let ret = command_run(&cmd);
    assert_eq!(ret, 10);
}

#[test]
fn test_read_field() {
    let data = b"keyval,rest of line";
    let mut slice: &[u8] = data;
    let field = read_field(&mut slice, "test_field");
    assert_eq!(field, "keyval");
    assert_eq!(slice, b"rest of line");
}

#[test]
fn test_read_until_eol_with_newline() {
    let data = b"some command here\nnext line";
    let mut slice: &[u8] = data;
    let result = read_until_eol(&mut slice);
    assert_eq!(result, "some command here");
    assert_eq!(slice, b"next line");
}

#[test]
fn test_read_until_eol_no_newline() {
    let data = b"last line no newline";
    let mut slice: &[u8] = data;
    let result = read_until_eol(&mut slice);
    assert_eq!(result, "last line no newline");
    assert_eq!(slice.len(), 0);
}

#[test]
fn test_tree_add_command_single_key() {
    let mut tree = Command::new('\0', "root".to_string(), String::new());
    tree_add_command(&mut tree, "a", "alpha", "cmd_alpha");
    tree_add_command(&mut tree, "b", "bravo", "cmd_bravo");

    let a = find_command(&tree, 'a').unwrap();
    assert_eq!(a.key, 'a');
    assert_eq!(a.name, "alpha");
    assert_eq!(a.command, "cmd_alpha");

    let b = find_command(&tree, 'b').unwrap();
    assert_eq!(b.key, 'b');
    assert_eq!(b.name, "bravo");
    assert_eq!(b.command, "cmd_bravo");
}

#[test]
fn test_tree_add_command_nested() {
    let mut tree = Command::new('\0', "root".to_string(), String::new());
    tree_add_command(&mut tree, "ab", "nested_b", "cmd_nested_b");

    let a = find_command(&tree, 'a').unwrap();
    assert_eq!(a.key, 'a');
    assert_eq!(a.name, "unnamed");
    assert_eq!(a.command, "");

    let b = find_command(a, 'b').unwrap();
    assert_eq!(b.key, 'b');
    assert_eq!(b.name, "nested_b");
    assert_eq!(b.command, "cmd_nested_b");
}

#[test]
fn test_tree_add_command_update_existing() {
    let mut tree = Command::new('\0', "root".to_string(), String::new());
    tree_add_command(&mut tree, "a", "alpha", "cmd_alpha");
    tree_add_command(&mut tree, "a", "alpha_updated", "cmd_alpha_updated");

    let a = find_command(&tree, 'a').unwrap();
    assert_eq!(a.key, 'a');
    assert_eq!(a.name, "alpha_updated");
    assert_eq!(a.command, "cmd_alpha_updated");
}

#[test]
fn test_read_line() {
    let mut tree = Command::new('\0', "root".to_string(), String::new());
    let data = b"x,xray,cmd_xray\ny,yankee,cmd_yankee\n";
    let mut slice: &[u8] = data;
    read_line(&mut tree, &mut slice);
    read_line(&mut tree, &mut slice);

    let x = find_command(&tree, 'x').unwrap();
    assert_eq!(x.key, 'x');
    assert_eq!(x.name, "xray");
    assert_eq!(x.command, "cmd_xray");

    let y = find_command(&tree, 'y').unwrap();
    assert_eq!(y.key, 'y');
    assert_eq!(y.name, "yankee");
    assert_eq!(y.command, "cmd_yankee");
}

#[test]
fn test_command_add_child_single() {
    let mut parent = Command::new('\0', "root".to_string(), String::new());
    command_add_child(&mut parent, Command::new('m', "mike".to_string(), String::new()));

    let child = parent.children.as_ref().unwrap();
    assert_eq!(child.key, 'm');
    assert_eq!(child.name, "mike");
    assert!(child.next.is_none());
}

#[test]
fn test_command_add_child_insert_before_head() {
    let mut parent = Command::new('\0', "root".to_string(), String::new());
    command_add_child(&mut parent, Command::new('z', "zulu".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('a', "alpha".to_string(), String::new()));

    let first = parent.children.as_ref().unwrap();
    assert_eq!(first.key, 'a');
    let second = first.next.as_ref().unwrap();
    assert_eq!(second.key, 'z');
    assert!(second.next.is_none());
}

#[test]
fn test_find_command_no_children() {
    let parent = Command::new('\0', "root".to_string(), String::new());
    assert!(find_command(&parent, 'a').is_none());
}

#[test]
fn test_tree_add_command_deep_nesting() {
    let mut tree = Command::new('\0', "root".to_string(), String::new());
    tree_add_command(&mut tree, "abc", "deep", "cmd_deep");

    let a = find_command(&tree, 'a').unwrap();
    assert_eq!(a.name, "unnamed");
    let b = find_command(a, 'b').unwrap();
    assert_eq!(b.name, "unnamed");
    let c = find_command(b, 'c').unwrap();
    assert_eq!(c.name, "deep");
    assert_eq!(c.command, "cmd_deep");
}

fn main() {}
