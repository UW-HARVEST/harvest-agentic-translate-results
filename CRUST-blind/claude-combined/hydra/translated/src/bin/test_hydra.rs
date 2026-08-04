use hydra::hydra::{
    command_add_child, command_run, find_command, read_field, read_until_eol,
    tree_add_command, Command, BLUE, COLOR_OFF, CYAN, DEFAULT_NAME, GREEN, PURPLE, RED,
    RIGHT_MARGIN, WHITE, YELLOW,
};

#[test]
fn test_constants() {
    assert_eq!(RED, "\x1b[0;31m");
    assert_eq!(GREEN, "\x1b[0;32m");
    assert_eq!(YELLOW, "\x1b[0;33m");
    assert_eq!(BLUE, "\x1b[0;34m");
    assert_eq!(PURPLE, "\x1b[0;35m");
    assert_eq!(CYAN, "\x1b[0;36m");
    assert_eq!(WHITE, "\x1b[0;37m");
    assert_eq!(COLOR_OFF, "\x1b[0m");
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
fn test_command_add_child_first() {
    // Adding to empty parent
    let mut parent = Command::new('\0', String::new(), String::new());
    let child = Command::new('a', String::new(), String::new());
    command_add_child(&mut parent, child);
    assert!(parent.children.is_some());
    let c = parent.children.as_ref().unwrap();
    assert_eq!(c.key, 'a');
    assert!(c.next.is_none());
}

#[test]
fn test_command_add_child_sorted_order() {
    // Replicates the C test: add c, a, b -> result ordering a, b, c
    let mut parent = Command::new('\0', String::new(), String::new());
    let a = Command::new('a', String::new(), String::new());
    let b = Command::new('b', String::new(), String::new());
    let c = Command::new('c', String::new(), String::new());

    command_add_child(&mut parent, c);
    command_add_child(&mut parent, a);
    command_add_child(&mut parent, b);

    let first = parent.children.as_ref().expect("first child");
    assert_eq!(first.key, 'a');
    let second = first.next.as_ref().expect("second child");
    assert_eq!(second.key, 'b');
    let third = second.next.as_ref().expect("third child");
    assert_eq!(third.key, 'c');
    assert!(third.next.is_none());
}

#[test]
fn test_command_add_child_prepend() {
    // Add child whose key is smaller than first child
    let mut parent = Command::new('\0', String::new(), String::new());
    let m = Command::new('m', String::new(), String::new());
    let z = Command::new('z', String::new(), String::new());
    let a = Command::new('a', String::new(), String::new());

    command_add_child(&mut parent, m);
    command_add_child(&mut parent, z);
    command_add_child(&mut parent, a);

    let first = parent.children.as_ref().unwrap();
    assert_eq!(first.key, 'a');
    let second = first.next.as_ref().unwrap();
    assert_eq!(second.key, 'm');
    let third = second.next.as_ref().unwrap();
    assert_eq!(third.key, 'z');
}

#[test]
fn test_command_add_child_equal_keys() {
    // C code uses <= so equal-key entries go *after* existing
    let mut parent = Command::new('\0', String::new(), String::new());
    let a1 = Command::new('a', "first".to_string(), String::new());
    let a2 = Command::new('a', "second".to_string(), String::new());

    command_add_child(&mut parent, a1);
    command_add_child(&mut parent, a2);

    let first = parent.children.as_ref().unwrap();
    assert_eq!(first.key, 'a');
    assert_eq!(first.name, "first");
    let second = first.next.as_ref().unwrap();
    assert_eq!(second.key, 'a');
    assert_eq!(second.name, "second");
}

#[test]
fn test_find_command_present() {
    let mut root = Command::new('\0', String::new(), String::new());
    command_add_child(&mut root, Command::new('z', String::new(), String::new()));
    command_add_child(&mut root, Command::new('a', String::new(), String::new()));
    command_add_child(&mut root, Command::new('m', String::new(), String::new()));

    let f = find_command(&root, 'm');
    assert!(f.is_some());
    assert_eq!(f.unwrap().key, 'm');

    let fa = find_command(&root, 'a');
    assert!(fa.is_some());
    assert_eq!(fa.unwrap().key, 'a');

    let fz = find_command(&root, 'z');
    assert!(fz.is_some());
    assert_eq!(fz.unwrap().key, 'z');
}

#[test]
fn test_find_command_absent() {
    let mut root = Command::new('\0', String::new(), String::new());
    command_add_child(&mut root, Command::new('a', String::new(), String::new()));

    let res = find_command(&root, 'x');
    assert!(res.is_none());
}

#[test]
fn test_find_command_empty() {
    let root = Command::new('\0', String::new(), String::new());
    let res = find_command(&root, 'a');
    assert!(res.is_none());
}

#[test]
fn test_command_run_with_command() {
    let cmd = Command::new('k', "n".to_string(), "echo hello".to_string());
    let n = command_run(&cmd);
    assert_eq!(n, 10);
}

#[test]
fn test_command_run_empty_command() {
    let cmd = Command::new('k', "n".to_string(), String::new());
    let n = command_run(&cmd);
    assert_eq!(n, 0);
}

#[test]
fn test_read_field_basic() {
    let data = b"hello,world,more\n";
    let mut slice: &[u8] = data;
    let r = read_field(&mut slice, "key");
    assert_eq!(r, "hello");
    // The remaining slice should start with "world,more\n"
    assert_eq!(slice, b"world,more\n");
}

#[test]
fn test_read_field_two_fields() {
    let data = b"hello,world,more\nfoo,bar,baz\n";
    let mut slice: &[u8] = data;
    let r1 = read_field(&mut slice, "key");
    assert_eq!(r1, "hello");
    let r2 = read_field(&mut slice, "name");
    assert_eq!(r2, "world");
    assert_eq!(slice, b"more\nfoo,bar,baz\n");
}

#[test]
fn test_read_until_eol_basic() {
    let data = b"more\nfoo,bar,baz\n";
    let mut slice: &[u8] = data;
    let r = read_until_eol(&mut slice);
    assert_eq!(r, "more");
    assert_eq!(slice, b"foo,bar,baz\n");
}

#[test]
fn test_read_until_eol_no_newline() {
    let data = b"final";
    let mut slice: &[u8] = data;
    let r = read_until_eol(&mut slice);
    assert_eq!(r, "final");
    // After consuming, slice should be empty (or remain at the null terminator equivalent)
    assert_eq!(slice.len(), 0);
}

#[test]
fn test_read_until_eol_immediate_newline() {
    let data = b"\nrest";
    let mut slice: &[u8] = data;
    let r = read_until_eol(&mut slice);
    assert_eq!(r, "");
    assert_eq!(slice, b"rest");
}

#[test]
fn test_read_field_then_eol() {
    let data = b"hello,world,more\n";
    let mut slice: &[u8] = data;
    let r1 = read_field(&mut slice, "key");
    let r2 = read_field(&mut slice, "name");
    let r3 = read_until_eol(&mut slice);
    assert_eq!(r1, "hello");
    assert_eq!(r2, "world");
    assert_eq!(r3, "more");
    assert_eq!(slice.len(), 0);
}

#[test]
fn test_tree_add_single_key_new() {
    let mut t = Command::new('\0', String::new(), String::new());
    tree_add_command(&mut t, "a", "first", "cmd1");
    let fa = find_command(&t, 'a');
    assert!(fa.is_some());
    let fa = fa.unwrap();
    assert_eq!(fa.key, 'a');
    assert_eq!(fa.name, "first");
    assert_eq!(fa.command, "cmd1");
    assert!(fa.children.is_none());
    assert!(fa.next.is_none());
}

#[test]
fn test_tree_add_single_key_update_existing() {
    let mut t = Command::new('\0', String::new(), String::new());
    tree_add_command(&mut t, "a", "first", "cmd1");
    tree_add_command(&mut t, "a", "updated", "newcmd");
    let fa = find_command(&t, 'a');
    assert!(fa.is_some());
    let fa = fa.unwrap();
    assert_eq!(fa.key, 'a');
    assert_eq!(fa.name, "updated");
    assert_eq!(fa.command, "newcmd");
}

#[test]
fn test_tree_add_multi_key_creates_intermediate() {
    let mut t = Command::new('\0', String::new(), String::new());
    tree_add_command(&mut t, "xy", "xy-name", "cmd-xy");
    let fx = find_command(&t, 'x');
    assert!(fx.is_some());
    let fx = fx.unwrap();
    assert_eq!(fx.key, 'x');
    assert_eq!(fx.name, DEFAULT_NAME);
    assert_eq!(fx.command, "");
    let fy = find_command(fx, 'y');
    assert!(fy.is_some());
    let fy = fy.unwrap();
    assert_eq!(fy.key, 'y');
    assert_eq!(fy.name, "xy-name");
    assert_eq!(fy.command, "cmd-xy");
}

#[test]
fn test_tree_add_multi_key_reuses_intermediate() {
    let mut t = Command::new('\0', String::new(), String::new());
    tree_add_command(&mut t, "xy", "xy-name", "cmd-xy");
    tree_add_command(&mut t, "xz", "xz-name", "cmd-xz");

    let fx = find_command(&t, 'x').expect("x intermediate");
    assert_eq!(fx.key, 'x');
    assert_eq!(fx.name, DEFAULT_NAME);

    // x should have children y then z (sorted)
    let y = fx.children.as_deref().expect("y child");
    assert_eq!(y.key, 'y');
    assert_eq!(y.name, "xy-name");
    assert_eq!(y.command, "cmd-xy");
    let z = y.next.as_deref().expect("z child");
    assert_eq!(z.key, 'z');
    assert_eq!(z.name, "xz-name");
    assert_eq!(z.command, "cmd-xz");
    assert!(z.next.is_none());

    // Make sure x is the only top-level child
    assert!(fx.next.is_none());
}

#[test]
fn test_tree_add_then_subcommand_after_leaf() {
    // First add "a" as a leaf, then add "ab" - this turns 'a' into an
    // intermediate with a child 'b'. The C code does NOT update 'a' to default
    // name in this case; it only creates a missing intermediate.
    let mut t = Command::new('\0', String::new(), String::new());
    tree_add_command(&mut t, "a", "leaf", "leafcmd");
    tree_add_command(&mut t, "ab", "ab-name", "ab-cmd");

    let fa = find_command(&t, 'a').expect("a present");
    // Per the C code: when 'a' already exists, it keeps its existing name/command
    assert_eq!(fa.name, "leaf");
    assert_eq!(fa.command, "leafcmd");

    let fb = find_command(fa, 'b').expect("b under a");
    assert_eq!(fb.key, 'b');
    assert_eq!(fb.name, "ab-name");
    assert_eq!(fb.command, "ab-cmd");
}

#[test]
fn test_tree_add_deep_chain() {
    let mut t = Command::new('\0', String::new(), String::new());
    tree_add_command(&mut t, "abc", "deep", "deepcmd");

    let fa = find_command(&t, 'a').expect("a");
    assert_eq!(fa.name, DEFAULT_NAME);
    assert_eq!(fa.command, "");
    let fb = find_command(fa, 'b').expect("b");
    assert_eq!(fb.name, DEFAULT_NAME);
    assert_eq!(fb.command, "");
    let fc = find_command(fb, 'c').expect("c");
    assert_eq!(fc.name, "deep");
    assert_eq!(fc.command, "deepcmd");
}

#[test]
fn test_load_file_reads_full_tree() {
    // Replicate the structure of `c_src/hydras/git`
    use std::io::Write;
    let dir = std::env::temp_dir();
    let path = dir.join("test_hydra_load.txt");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "g,Git,").unwrap();
        writeln!(f, "gd,Diff,git diff").unwrap();
        writeln!(f, "gs,Status,git status").unwrap();
    }

    let mut t = Command::new('\0', String::new(), String::new());
    hydra::hydra::load_file(&mut t, path.to_str().unwrap());

    let fg = find_command(&t, 'g').expect("g present");
    assert_eq!(fg.key, 'g');
    assert_eq!(fg.name, "Git");
    assert_eq!(fg.command, "");

    let fd = find_command(fg, 'd').expect("d under g");
    assert_eq!(fd.name, "Diff");
    assert_eq!(fd.command, "git diff");

    let fs = find_command(fg, 's').expect("s under g");
    assert_eq!(fs.name, "Status");
    assert_eq!(fs.command, "git status");

    // Children ordering of g: d, s
    let first = fg.children.as_deref().expect("first child of g");
    assert_eq!(first.key, 'd');
    let second = first.next.as_deref().expect("second child of g");
    assert_eq!(second.key, 's');
    assert!(second.next.is_none());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_read_file_returns_contents() {
    use std::io::Write;
    let dir = std::env::temp_dir();
    let path = dir.join("test_hydra_readfile.txt");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        write!(f, "hello world").unwrap();
    }
    let s = hydra::hydra::read_file(path.to_str().unwrap());
    assert_eq!(s, "hello world");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_clear_lines_does_not_panic() {
    // Just verify it doesn't panic with various counts
    hydra::hydra::clear_lines(0);
    hydra::hydra::clear_lines(3);
}

#[test]
fn test_print_command_returns_lines_count() {
    // Build a small tree and verify print_command returns at least 1 line
    let mut root = Command::new('\0', "Root".to_string(), String::new());
    command_add_child(
        &mut root,
        Command::new('a', "Apple".to_string(), "cmd-a".to_string()),
    );
    command_add_child(
        &mut root,
        Command::new('b', "Banana".to_string(), "cmd-b".to_string()),
    );

    let n = hydra::hydra::print_command(&root);
    // Header line + final trailing newline -> at least 2 lines
    assert!(n >= 2);
}

fn main() {}
