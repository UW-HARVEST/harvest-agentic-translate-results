use hydra::hydra::*;

// --- Command::new ---

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
fn test_new_command_empty_strings() {
    let cmd = Command::new('\0', String::new(), String::new());
    assert_eq!(cmd.key, '\0');
    assert_eq!(cmd.name, "");
    assert_eq!(cmd.command, "");
}

// --- command_run ---

#[test]
fn test_command_run_with_string() {
    let cmd = Command::new('a', "test".to_string(), "hello".to_string());
    assert_eq!(command_run(&cmd), 5);
}

#[test]
fn test_command_run_longer_string() {
    let cmd = Command::new('a', "test".to_string(), "abcdefghij".to_string());
    assert_eq!(command_run(&cmd), 10);
}

#[test]
fn test_command_run_empty_command() {
    let cmd = Command::new('a', "test".to_string(), String::new());
    assert_eq!(command_run(&cmd), 0);
}

// --- command_add_child / find_command ---

#[test]
fn test_add_child_single() {
    let mut parent = Command::new('\0', String::new(), String::new());
    let child = Command::new('a', "alpha".to_string(), String::new());
    command_add_child(&mut parent, child);
    assert!(parent.children.is_some());
    assert_eq!(parent.children.as_ref().unwrap().key, 'a');
}

#[test]
fn test_add_child_sorted_order() {
    let mut parent = Command::new('\0', String::new(), String::new());
    command_add_child(&mut parent, Command::new('c', "charlie".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('a', "alpha".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('b', "bravo".to_string(), String::new()));

    let first = parent.children.as_ref().unwrap();
    assert_eq!(first.key, 'a');
    let second = first.next.as_ref().unwrap();
    assert_eq!(second.key, 'b');
    let third = second.next.as_ref().unwrap();
    assert_eq!(third.key, 'c');
    assert!(third.next.is_none());
}

#[test]
fn test_add_child_duplicate_keys() {
    let mut parent = Command::new('\0', String::new(), String::new());
    command_add_child(&mut parent, Command::new('a', "first".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('a', "second".to_string(), String::new()));

    let first = parent.children.as_ref().unwrap();
    assert_eq!(first.key, 'a');
    assert_eq!(first.name, "first");
    let second = first.next.as_ref().unwrap();
    assert_eq!(second.key, 'a');
    assert_eq!(second.name, "second");
}

#[test]
fn test_find_command_exists() {
    let mut parent = Command::new('\0', String::new(), String::new());
    command_add_child(&mut parent, Command::new('a', "alpha".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('b', "beta".to_string(), String::new()));

    let found = find_command(&parent, 'a');
    assert!(found.is_some());
    assert_eq!(found.unwrap().key, 'a');
    assert_eq!(found.unwrap().name, "alpha");
}

#[test]
fn test_find_command_not_found() {
    let mut parent = Command::new('\0', String::new(), String::new());
    command_add_child(&mut parent, Command::new('a', "alpha".to_string(), String::new()));
    assert!(find_command(&parent, 'z').is_none());
}

#[test]
fn test_find_command_no_children() {
    let parent = Command::new('\0', String::new(), String::new());
    assert!(find_command(&parent, 'a').is_none());
}

// --- tree_add_command ---

#[test]
fn test_tree_add_single_key() {
    let mut tree = Command::new('\0', "root".to_string(), String::new());
    tree_add_command(&mut tree, "a", "alpha", "cmd_a");

    let a = find_command(&tree, 'a').unwrap();
    assert_eq!(a.name, "alpha");
    assert_eq!(a.command, "cmd_a");
}

#[test]
fn test_tree_add_multiple_keys() {
    let mut tree = Command::new('\0', "root".to_string(), String::new());
    tree_add_command(&mut tree, "a", "alpha", "cmd_a");
    tree_add_command(&mut tree, "b", "beta", "cmd_b");

    assert_eq!(find_command(&tree, 'a').unwrap().name, "alpha");
    assert_eq!(find_command(&tree, 'b').unwrap().name, "beta");
}

#[test]
fn test_tree_add_nested_keys() {
    let mut tree = Command::new('\0', "root".to_string(), String::new());
    tree_add_command(&mut tree, "ab", "nested", "cmd_nested");

    let a = find_command(&tree, 'a').unwrap();
    assert_eq!(a.name, DEFAULT_NAME); // intermediate node gets "unnamed"
    let b = find_command(a, 'b').unwrap();
    assert_eq!(b.name, "nested");
    assert_eq!(b.command, "cmd_nested");
}

#[test]
fn test_tree_add_deep_nesting() {
    let mut tree = Command::new('\0', "root".to_string(), String::new());
    tree_add_command(&mut tree, "abc", "deep", "cmd_deep");

    let a = find_command(&tree, 'a').unwrap();
    assert_eq!(a.name, DEFAULT_NAME);
    let b = find_command(a, 'b').unwrap();
    assert_eq!(b.name, DEFAULT_NAME);
    let c = find_command(b, 'c').unwrap();
    assert_eq!(c.name, "deep");
    assert_eq!(c.command, "cmd_deep");
}

#[test]
fn test_tree_add_update_existing() {
    let mut tree = Command::new('\0', "root".to_string(), String::new());
    tree_add_command(&mut tree, "a", "old_name", "old_cmd");
    tree_add_command(&mut tree, "a", "new_name", "new_cmd");

    let a = find_command(&tree, 'a').unwrap();
    assert_eq!(a.name, "new_name");
    assert_eq!(a.command, "new_cmd");
}

#[test]
fn test_tree_add_existing_intermediate_then_leaf() {
    let mut tree = Command::new('\0', "root".to_string(), String::new());
    tree_add_command(&mut tree, "ab", "alpha-beta", "cmd_ab");
    // Now 'a' exists as unnamed intermediate; add leaf at 'a'
    tree_add_command(&mut tree, "a", "alpha", "cmd_a");

    let a = find_command(&tree, 'a').unwrap();
    assert_eq!(a.name, "alpha");
    assert_eq!(a.command, "cmd_a");
    // child 'b' should still exist
    let b = find_command(a, 'b').unwrap();
    assert_eq!(b.name, "alpha-beta");
}

// --- read_field ---

#[test]
fn test_read_field_basic() {
    let data = b"key,name,command\n";
    let mut slice: &[u8] = data;
    let key = read_field(&mut slice, "key");
    assert_eq!(key, "key");
    let name = read_field(&mut slice, "name");
    assert_eq!(name, "name");
}

#[test]
fn test_read_field_single_char() {
    let data = b"a,b,c\n";
    let mut slice: &[u8] = data;
    assert_eq!(read_field(&mut slice, "f1"), "a");
    assert_eq!(read_field(&mut slice, "f2"), "b");
}

// --- read_until_eol ---

#[test]
fn test_read_until_eol_with_newline() {
    let data = b"command\n";
    let mut slice: &[u8] = data;
    let result = read_until_eol(&mut slice);
    assert_eq!(result, "command");
    assert!(slice.is_empty());
}

#[test]
fn test_read_until_eol_no_newline() {
    let data = b"no_newline";
    let mut slice: &[u8] = data;
    let result = read_until_eol(&mut slice);
    assert_eq!(result, "no_newline");
    assert!(slice.is_empty());
}

#[test]
fn test_read_until_eol_empty() {
    let data = b"\n";
    let mut slice: &[u8] = data;
    let result = read_until_eol(&mut slice);
    assert_eq!(result, "");
    assert!(slice.is_empty());
}

// --- read_line ---

#[test]
fn test_read_line_basic() {
    let data = b"a,alpha,cmd_alpha\n";
    let mut slice: &[u8] = data;
    let mut root = Command::new('\0', "root".to_string(), String::new());
    read_line(&mut root, &mut slice);

    let a = find_command(&root, 'a').unwrap();
    assert_eq!(a.name, "alpha");
    assert_eq!(a.command, "cmd_alpha");
}

#[test]
fn test_read_line_multi_key() {
    let data = b"ab,nested,cmd_nested\n";
    let mut slice: &[u8] = data;
    let mut root = Command::new('\0', "root".to_string(), String::new());
    read_line(&mut root, &mut slice);

    let a = find_command(&root, 'a').unwrap();
    assert_eq!(a.name, DEFAULT_NAME);
    let b = find_command(a, 'b').unwrap();
    assert_eq!(b.name, "nested");
    assert_eq!(b.command, "cmd_nested");
}

#[test]
fn test_read_line_multiple_lines() {
    let data = b"a,alpha,cmd_a\nb,beta,cmd_b\n";
    let mut slice: &[u8] = data;
    let mut root = Command::new('\0', "root".to_string(), String::new());
    read_line(&mut root, &mut slice);
    read_line(&mut root, &mut slice);

    assert_eq!(find_command(&root, 'a').unwrap().name, "alpha");
    assert_eq!(find_command(&root, 'b').unwrap().name, "beta");
}

// --- load_file ---

#[test]
fn test_load_file() {
    let path = "/tmp/test_hydra_load.txt";
    std::fs::write(path, "a,alpha,cmd_a\nb,beta,cmd_b\n").unwrap();

    let mut root = Command::new('\0', "root".to_string(), String::new());
    load_file(&mut root, path);

    let a = find_command(&root, 'a').unwrap();
    assert_eq!(a.name, "alpha");
    assert_eq!(a.command, "cmd_a");
    let b = find_command(&root, 'b').unwrap();
    assert_eq!(b.name, "beta");
    assert_eq!(b.command, "cmd_b");

    std::fs::remove_file(path).ok();
}

// --- read_file ---

#[test]
fn test_read_file() {
    let path = "/tmp/test_hydra_readfile.txt";
    std::fs::write(path, "hello world").unwrap();
    let content = read_file(path);
    assert_eq!(content, "hello world");
    std::fs::remove_file(path).ok();
}

fn main() {}
