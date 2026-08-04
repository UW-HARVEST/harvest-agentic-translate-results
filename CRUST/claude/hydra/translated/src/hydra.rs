use std::io::{self, Read, Write};

pub const RED: &str = "\x1b[0;31m";
pub const WHITE: &str = "\x1b[0;37m";
pub const COLOR_OFF: &str = "\x1b[0m";
pub const PURPLE: &str = "\x1b[0;35m";
pub const GREEN: &str = "\x1b[0;32m";
pub const YELLOW: &str = "\x1b[0;33m";
pub const BLUE: &str = "\x1b[0;34m";
pub const RIGHT_MARGIN: usize = 5;
pub const CYAN: &str = "\x1b[0;36m";
pub const DEFAULT_NAME: &str = "unnamed";

pub struct Command {
    pub key: char,
    pub name: String,
    pub command: String,
    pub children: Option<Box<Command>>,
    pub next: Option<Box<Command>>,
}

impl Command {
    pub fn new(key: char, name: String, command: String) -> Self {
        Command {
            key,
            name,
            command,
            children: None,
            next: None,
        }
    }
}

/// Reads characters from `file` (a slice of bytes) until a comma is encountered.
/// Returns the bytes prior to the comma as a String. Advances the slice past the
/// comma. If end-of-file or end-of-line is reached before a comma, prints an
/// error and exits.
pub fn read_field(file: &mut &[u8], field: &str) -> String {
    let mut idx = 0;
    while idx < file.len() && file[idx] != b',' && file[idx] != b'\n' && file[idx] != 0 {
        idx += 1;
    }

    if idx >= file.len() || file[idx] != b',' {
        let found = if idx < file.len() {
            file[idx] as char
        } else {
            '\0'
        };
        eprint!("Found incorrect end after {}, found: {}", field, found);
        std::process::exit(1);
    }

    let key = String::from_utf8_lossy(&file[..idx]).into_owned();
    *file = &file[idx + 1..];
    key
}

/// Loads a CSV-like file from disk and adds each line as a command to the tree.
pub fn load_file(c: &mut Command, file: &str) {
    let content = read_file(file);
    let bytes = content.into_bytes();
    let mut slice: &[u8] = &bytes;
    while !slice.is_empty() && slice[0] != 0 {
        read_line(c, &mut slice);
    }
}

/// Interactive entry point: prints command tree, reads a key, and runs the
/// appropriate child command. Not meant to be invoked from tests.
pub fn start(c: &Command) {
    let mut current: Option<&Command> = Some(c);
    while let Some(cmd) = current {
        if cmd.children.is_none() {
            break;
        }
        let last_printed_lines = print_command(cmd);
        let key = getch();
        let next = find_command(cmd, key);
        clear_lines(last_printed_lines);
        if let Some(next_cmd) = next {
            if command_run(next_cmd) > 0 {
                return;
            }
            current = Some(next_cmd);
        } else {
            return;
        }
    }
}

/// Adds a chain of commands described by `keys` into the tree. The deepest node
/// gets the supplied `name` and `command`. Intermediate nodes are created with
/// the default name and an empty command.
pub fn tree_add_command(tree: &mut Command, keys: &str, name: &str, command: &str) {
    let mut chars = keys.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return,
    };
    let rest: String = chars.collect();

    if rest.is_empty() {
        // Look up an existing child with the same key.
        let exists = find_child_index(tree, first).is_some();
        if !exists {
            command_add_child(
                tree,
                Command::new(first, name.to_string(), command.to_string()),
            );
        } else if let Some(child) = find_child_mut(tree, first) {
            child.name = name.to_string();
            child.command = command.to_string();
        }
        return;
    }

    if find_child_index(tree, first).is_none() {
        command_add_child(
            tree,
            Command::new(first, DEFAULT_NAME.to_string(), String::new()),
        );
    }

    if let Some(child) = find_child_mut(tree, first) {
        tree_add_command(child, &rest, name, command);
    }
}

fn find_child_index(tree: &Command, key: char) -> Option<()> {
    let mut cur = tree.children.as_deref();
    while let Some(c) = cur {
        if c.key == key {
            return Some(());
        }
        cur = c.next.as_deref();
    }
    None
}

fn find_child_mut(tree: &mut Command, key: char) -> Option<&mut Command> {
    let mut cur = tree.children.as_deref_mut();
    while let Some(c) = cur {
        if c.key == key {
            return Some(c);
        }
        cur = c.next.as_deref_mut();
    }
    None
}

/// Reads a single CSV line: key,name,command\n and inserts it into the tree.
pub fn read_line(c: &mut Command, file: &mut &[u8]) {
    let key = read_field(file, "key");
    let name = read_field(file, "name");
    let command = read_until_eol(file);
    tree_add_command(c, &key, &name, &command);
}

/// Inserts `child` into the children list of `c`, keeping the list sorted in
/// ascending order by key. Mirrors the C `CommandAddChild` semantics: when keys
/// are equal, the new child is inserted *after* existing equal-key children.
pub fn command_add_child(c: &mut Command, child: Command) {
    let mut child = Box::new(child);

    // Empty list: insert at the head.
    if c.children.is_none() {
        c.children = Some(child);
        return;
    }

    // Replace head if the existing head's key is greater than the child's key.
    let head_key = c.children.as_ref().unwrap().key;
    if head_key > child.key {
        let old_head = c.children.take();
        child.next = old_head;
        c.children = Some(child);
        return;
    }

    // Walk to the last node whose `next` is either None or has a key greater
    // than `child.key`.
    let mut cursor = c.children.as_mut().unwrap().as_mut();
    loop {
        let advance = match cursor.next.as_ref() {
            Some(n) => n.key <= child.key,
            None => false,
        };
        if advance {
            cursor = cursor.next.as_mut().unwrap().as_mut();
        } else {
            break;
        }
    }

    let after = cursor.next.take();
    child.next = after;
    cursor.next = Some(child);
}

/// Read a single character from stdin. Best-effort port of the C version
/// without termios manipulation.
pub fn getch() -> char {
    let mut buf = [0u8; 1];
    match io::stdin().read(&mut buf) {
        Ok(n) if n > 0 => buf[0] as char,
        _ => '\0',
    }
}

/// Prints the given command and its immediate children to stderr. Returns the
/// number of newline-terminated lines emitted so they can be cleared later.
pub fn print_command(c: &Command) -> i32 {
    let width = terminal_width();

    let mut lines: i32 = 0;

    if !c.name.is_empty() {
        eprintln!("{}{}:{}", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    let mut max_line_width: usize = 0;
    let mut child = c.children.as_deref();
    while let Some(ch) = child {
        let line_width = ch.name.len();
        if line_width > max_line_width {
            max_line_width = line_width;
        }
        child = ch.next.as_deref();
    }

    max_line_width += RIGHT_MARGIN;
    if max_line_width > width {
        max_line_width = width;
    }

    let denom = max_line_width + 5;
    let items_per_row = if denom == 0 { 1 } else { width / denom };
    let items_per_row = if items_per_row == 0 { 1 } else { items_per_row };

    let mut child = c.children.as_deref();
    let mut current_item: usize = 0;
    while let Some(ch) = child {
        current_item += 1;

        let padded = pad_right(&ch.name, max_line_width);
        if ch.children.is_some() {
            eprint!(
                "{}{}{} {}\u{2794}{} {}+{}{}",
                YELLOW, ch.key, COLOR_OFF, PURPLE, COLOR_OFF, BLUE, padded, COLOR_OFF
            );
        } else {
            eprint!(
                "{}{}{} {}\u{2794}{}  {}",
                YELLOW, ch.key, COLOR_OFF, PURPLE, COLOR_OFF, padded
            );
        }

        if current_item % items_per_row == 0 {
            eprintln!();
            lines += 1;
        }

        child = ch.next.as_deref();
    }

    eprintln!();
    lines += 1;

    lines
}

fn pad_right(s: &str, width: usize) -> String {
    if s.len() >= width {
        s.to_string()
    } else {
        let mut out = String::with_capacity(width);
        out.push_str(s);
        for _ in 0..(width - s.len()) {
            out.push(' ');
        }
        out
    }
}

fn terminal_width() -> usize {
    // Reasonable default in non-tty environments.
    80
}

/// Emits `count` ANSI escape sequences that move up and clear the line.
pub fn clear_lines(count: i32) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.flush();
    for _ in 0..count {
        eprint!("\x1b[A\r\x1b[2K");
    }
}

/// Returns the immediate child of `c` whose key matches `key`, if any.
pub fn find_command(c: &Command, key: char) -> Option<&Command> {
    let mut child = c.children.as_deref();
    while let Some(cur) = child {
        if cur.key == key {
            return Some(cur);
        }
        child = cur.next.as_deref();
    }
    None
}

/// Reads bytes from `file` until a newline or NUL is found. Returns the bytes
/// preceding it as a String and advances the slice past the newline.
pub fn read_until_eol(file: &mut &[u8]) -> String {
    let mut idx = 0;
    while idx < file.len() && file[idx] != b'\n' && file[idx] != 0 {
        idx += 1;
    }
    let s = String::from_utf8_lossy(&file[..idx]).into_owned();
    if idx < file.len() && file[idx] == b'\n' {
        *file = &file[idx + 1..];
    } else {
        *file = &file[idx..];
    }
    s
}

/// Reads a file into a string. On failure, mimics the C version by exiting.
pub fn read_file(file: &str) -> String {
    match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open file: {}", e);
            std::process::exit(1);
        }
    }
}

/// Writes the command's `command` string to stdout. Returns the number of
/// bytes written, mirroring `fprintf`'s return value.
pub fn command_run(c: &Command) -> i32 {
    if !c.command.is_empty() {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(c.command.as_bytes());
        let _ = handle.flush();
        return c.command.len() as i32;
    }
    0
}
