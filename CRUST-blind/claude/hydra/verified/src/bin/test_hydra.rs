#[allow(unused_imports)]
use hydra::hydra::{
    command_add_child, command_run, find_command, load_file, read_field, read_file, read_until_eol,
    tree_add_command, Command, BLUE, COLOR_OFF, CYAN, DEFAULT_NAME, GREEN, PURPLE, RED,
    RIGHT_MARGIN, WHITE, YELLOW,
};

// ---------- Constants ----------

#[test]
fn test_color_constants() {
    assert_eq!(COLOR_OFF, "\x1b[0m");
    assert_eq!(RED, "\x1b[0;31m");
    assert_eq!(GREEN, "\x1b[0;32m");
    assert_eq!(YELLOW, "\x1b[0;33m");
    assert_eq!(BLUE, "\x1b[0;34m");
    assert_eq!(PURPLE, "\x1b[0;35m");
    assert_eq!(CYAN, "\x1b[0;36m");
    assert_eq!(WHITE, "\x1b[0;37m");
}

#[test]
fn test_other_constants() {
    assert_eq!(RIGHT_MARGIN, 5);
    assert_eq!(DEFAULT_NAME, "unnamed");
}

// ---------- Command::new ----------

#[test]
fn test_command_new_basic() {
    let cmd = Command::new('k', "name".to_string(), "command".to_string());
    assert_eq!(cmd.key, 'k');
    assert_eq!(cmd.name, "name");
    assert_eq!(cmd.command, "command");
    assert!(cmd.children.is_none());
    assert!(cmd.next.is_none());
}

#[test]
fn test_command_new_empty_strings() {
    let cmd = Command::new('a', String::new(), String::new());
    assert_eq!(cmd.key, 'a');
    assert_eq!(cmd.name, "");
    assert_eq!(cmd.command, "");
    assert!(cmd.children.is_none());
    assert!(cmd.next.is_none());
}

// ---------- command_add_child / find_command ----------

#[test]
fn test_command_add_child_in_order() {
    // C verified: adding 'c', 'a', 'b' results in [a, b, c]
    let mut parent = Command::new('\0', String::new(), String::new());
    command_add_child(&mut parent, Command::new('c', "C".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('a', "A".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('b', "B".to_string(), String::new()));

    let mut keys = Vec::new();
    let mut names = Vec::new();
    let mut cur = parent.children.as_deref();
    while let Some(c) = cur {
        keys.push(c.key);
        names.push(c.name.clone());
        cur = c.next.as_deref();
    }
    assert_eq!(keys, vec!['a', 'b', 'c']);
    assert_eq!(names, vec!["A".to_string(), "B".to_string(), "C".to_string()]);
}

#[test]
fn test_command_add_child_at_head_when_smaller() {
    // C verified: smaller key inserted at head.
    let mut parent = Command::new('\0', String::new(), String::new());
    command_add_child(&mut parent, Command::new('m', "M".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('a', "A".to_string(), String::new()));

    let first = parent.children.as_ref().unwrap();
    assert_eq!(first.key, 'a');
    let second = first.next.as_ref().unwrap();
    assert_eq!(second.key, 'm');
    assert!(second.next.is_none());
}

#[test]
fn test_command_add_child_in_middle() {
    // C verified: 'a','z','m' -> a, m, z
    let mut parent = Command::new('\0', String::new(), String::new());
    command_add_child(&mut parent, Command::new('a', "A".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('z', "Z".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('m', "M".to_string(), String::new()));

    let mut keys = Vec::new();
    let mut cur = parent.children.as_deref();
    while let Some(c) = cur {
        keys.push(c.key);
        cur = c.next.as_deref();
    }
    assert_eq!(keys, vec!['a', 'm', 'z']);
}

#[test]
fn test_command_add_child_duplicate_keys_preserves_insertion_order() {
    // C verified: dup:a-A1 dup:a-A2 dup:a-A3 (newer goes after older equal-key)
    let mut parent = Command::new('\0', String::new(), String::new());
    command_add_child(&mut parent, Command::new('a', "A1".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('a', "A2".to_string(), String::new()));
    command_add_child(&mut parent, Command::new('a', "A3".to_string(), String::new()));

    let mut names = Vec::new();
    let mut cur = parent.children.as_deref();
    while let Some(c) = cur {
        names.push(c.name.clone());
        cur = c.next.as_deref();
    }
    assert_eq!(
        names,
        vec!["A1".to_string(), "A2".to_string(), "A3".to_string()]
    );
}

#[test]
fn test_command_add_child_first_child_when_empty() {
    let mut parent = Command::new('\0', String::new(), String::new());
    command_add_child(&mut parent, Command::new('x', "X".to_string(), String::new()));
    let first = parent.children.as_ref().unwrap();
    assert_eq!(first.key, 'x');
    assert_eq!(first.name, "X");
    assert!(first.next.is_none());
}

// ---------- find_command ----------

#[test]
fn test_find_command_existing() {
    let mut root = Command::new('r', "root".to_string(), String::new());
    command_add_child(&mut root, Command::new('a', "A".to_string(), "echo a".to_string()));
    command_add_child(&mut root, Command::new('b', "B".to_string(), "echo b".to_string()));

    let found = find_command(&root, 'b').unwrap();
    assert_eq!(found.key, 'b');
    assert_eq!(found.name, "B");
    assert_eq!(found.command, "echo b");
}

#[test]
fn test_find_command_first() {
    let mut root = Command::new('r', "root".to_string(), String::new());
    command_add_child(&mut root, Command::new('a', "A".to_string(), "echo a".to_string()));
    command_add_child(&mut root, Command::new('b', "B".to_string(), "echo b".to_string()));

    let found = find_command(&root, 'a').unwrap();
    assert_eq!(found.key, 'a');
    assert_eq!(found.name, "A");
    assert_eq!(found.command, "echo a");
}

#[test]
fn test_find_command_missing() {
    let mut root = Command::new('r', "root".to_string(), String::new());
    command_add_child(&mut root, Command::new('a', "A".to_string(), String::new()));
    command_add_child(&mut root, Command::new('b', "B".to_string(), String::new()));

    assert!(find_command(&root, 'z').is_none());
}

#[test]
fn test_find_command_no_children() {
    let root = Command::new('r', "root".to_string(), String::new());
    assert!(find_command(&root, 'a').is_none());
}

// ---------- command_run ----------

#[test]
fn test_command_run_returns_byte_length() {
    // C verified: CommandRun returns fprintf result; "echo a" = 6 bytes
    let cmd = Command::new('a', "A".to_string(), "echo a".to_string());
    assert_eq!(command_run(&cmd), 6);
}

#[test]
fn test_command_run_single_char() {
    // C verified: "x" -> 1
    let cmd = Command::new('a', "A".to_string(), "x".to_string());
    assert_eq!(command_run(&cmd), 1);
}

#[test]
fn test_command_run_empty_command() {
    // C verified: empty command -> 0
    let cmd = Command::new('a', "A".to_string(), String::new());
    assert_eq!(command_run(&cmd), 0);
}

// ---------- read_field ----------

#[test]
fn test_read_field_basic() {
    // C verified: "key1,name1,cmd1\n..." -> key1, name1
    let bytes = b"key1,name1,cmd1\nkey2,name2,cmd2\n".to_vec();
    let mut slice: &[u8] = &bytes;
    let f1 = read_field(&mut slice, "key");
    assert_eq!(f1, "key1");
    let f2 = read_field(&mut slice, "name");
    assert_eq!(f2, "name1");
    // Slice should now point to "cmd1\nkey2,name2,cmd2\n"
    assert_eq!(slice, b"cmd1\nkey2,name2,cmd2\n");
}

#[test]
fn test_read_field_empty() {
    // C verified: ",,\n" -> "", "", ""
    let bytes = b",,\n".to_vec();
    let mut slice: &[u8] = &bytes;
    let f1 = read_field(&mut slice, "a");
    assert_eq!(f1, "");
    let f2 = read_field(&mut slice, "b");
    assert_eq!(f2, "");
    // Remaining is the eol fragment
    assert_eq!(slice, b"\n");
}

// ---------- read_until_eol ----------

#[test]
fn test_read_until_eol_with_newline() {
    // C verified: "line1\nline2" -> r="line1", remaining="line2"
    let bytes = b"line1\nline2".to_vec();
    let mut slice: &[u8] = &bytes;
    let r = read_until_eol(&mut slice);
    assert_eq!(r, "line1");
    assert_eq!(slice, b"line2");
}

#[test]
fn test_read_until_eol_no_newline() {
    // C verified: "noeol" -> r="noeol", remaining=""
    let bytes = b"noeol".to_vec();
    let mut slice: &[u8] = &bytes;
    let r = read_until_eol(&mut slice);
    assert_eq!(r, "noeol");
    assert_eq!(slice, b"");
}

#[test]
fn test_read_until_eol_empty() {
    let bytes = b"\n".to_vec();
    let mut slice: &[u8] = &bytes;
    let r = read_until_eol(&mut slice);
    assert_eq!(r, "");
    assert_eq!(slice, b"");
}

// ---------- tree_add_command ----------

#[test]
fn test_tree_add_command_single_key() {
    let mut tree = Command::new('\0', String::new(), String::new());
    tree_add_command(&mut tree, "a", "A", "cmd_a");
    let c = find_command(&tree, 'a').unwrap();
    assert_eq!(c.key, 'a');
    assert_eq!(c.name, "A");
    assert_eq!(c.command, "cmd_a");
    assert!(c.children.is_none());
}

#[test]
fn test_tree_add_command_multi_level() {
    // C verified: "ab" -> intermediate 'a' has name="unnamed", cmd=NULL/empty
    // 'b' under 'a' has name="AB", cmd="cmd_ab"
    let mut tree = Command::new('\0', String::new(), String::new());
    tree_add_command(&mut tree, "ab", "AB", "cmd_ab");

    let ca = find_command(&tree, 'a').unwrap();
    assert_eq!(ca.key, 'a');
    assert_eq!(ca.name, DEFAULT_NAME);
    assert_eq!(ca.command, "");

    let cab = find_command(ca, 'b').unwrap();
    assert_eq!(cab.key, 'b');
    assert_eq!(cab.name, "AB");
    assert_eq!(cab.command, "cmd_ab");
    assert!(cab.children.is_none());
}

#[test]
fn test_tree_add_command_update_existing() {
    // C verified: adding "a" twice updates name and command
    let mut tree = Command::new('\0', String::new(), String::new());
    tree_add_command(&mut tree, "a", "First", "cmd_first");
    tree_add_command(&mut tree, "a", "Second", "cmd_second");

    let ca = find_command(&tree, 'a').unwrap();
    assert_eq!(ca.key, 'a');
    assert_eq!(ca.name, "Second");
    assert_eq!(ca.command, "cmd_second");
}

#[test]
fn test_tree_add_command_shared_prefix() {
    // C verified: ab/ac share 'a' parent (named "unnamed")
    let mut tree = Command::new('\0', String::new(), String::new());
    tree_add_command(&mut tree, "ab", "AB", "cmd_ab");
    tree_add_command(&mut tree, "ac", "AC", "cmd_ac");

    let ca = find_command(&tree, 'a').unwrap();
    assert_eq!(ca.name, DEFAULT_NAME);

    let cab = find_command(ca, 'b').unwrap();
    assert_eq!(cab.name, "AB");
    assert_eq!(cab.command, "cmd_ab");

    let cac = find_command(ca, 'c').unwrap();
    assert_eq!(cac.name, "AC");
    assert_eq!(cac.command, "cmd_ac");
}

#[test]
fn test_tree_add_command_three_levels() {
    let mut tree = Command::new('\0', String::new(), String::new());
    tree_add_command(&mut tree, "abc", "Deep", "cmd_deep");

    let ca = find_command(&tree, 'a').unwrap();
    assert_eq!(ca.name, DEFAULT_NAME);
    let cab = find_command(ca, 'b').unwrap();
    assert_eq!(cab.name, DEFAULT_NAME);
    let cabc = find_command(cab, 'c').unwrap();
    assert_eq!(cabc.name, "Deep");
    assert_eq!(cabc.command, "cmd_deep");
}

// ---------- read_file ----------

#[test]
fn test_read_file_known_content() {
    // C verified that the cargo hydra file starts with "c,Cargo,\ncn,New Proj..."
    let content = read_file("c_src/hydras/cargo");
    let first_line = content.lines().next().unwrap();
    assert_eq!(first_line, "c,Cargo,");
    let second_line = content.lines().nth(1).unwrap();
    assert!(second_line.starts_with("cn,New Project,"));
}

// ---------- load_file ----------

#[test]
fn test_load_file_git_hydras() {
    // C verified the structure of the git hydra file.
    let mut root = Command::new('\0', String::new(), String::new());
    load_file(&mut root, "c_src/hydras/git");

    let g = find_command(&root, 'g').unwrap();
    assert_eq!(g.key, 'g');
    assert_eq!(g.name, "Git");
    assert_eq!(g.command, "");

    let g_pull = find_command(g, 'F').unwrap();
    assert_eq!(g_pull.key, 'F');
    assert_eq!(g_pull.name, "Pull");
    assert_eq!(g_pull.command, "git pull");

    let gb = find_command(g, 'b').unwrap();
    assert_eq!(gb.key, 'b');
    assert_eq!(gb.name, "Branch");
    assert_eq!(gb.command, "");

    let gbc = find_command(gb, 'c').unwrap();
    assert_eq!(gbc.key, 'c');
    assert_eq!(gbc.name, "Create");
    assert_eq!(
        gbc.command,
        "read -p \"New branch name: \" branch; git branch $branch; git switch $branch"
    );

    let gbl = find_command(gb, 'l').unwrap();
    assert_eq!(gbl.key, 'l');
    assert_eq!(gbl.name, "List");
    assert_eq!(gbl.command, "git branch");
}

fn main() {}
