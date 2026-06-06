use std::io::{Read, Write};

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

fn find_command_mut<'a>(c: &'a mut Command, key: char) -> Option<&'a mut Command> {
    let mut current = c.children.as_deref_mut();
    while let Some(node) = current {
        if node.key == key {
            return Some(node);
        }
        current = node.next.as_deref_mut();
    }
    None
}

pub fn read_field(file: &mut &[u8], field: &str) -> String {
    let bytes: &[u8] = *file;
    let mut idx = 0usize;
    while idx < bytes.len() && bytes[idx] != b',' && bytes[idx] != b'\n' && bytes[idx] != 0 {
        idx += 1;
    }

    if idx >= bytes.len() || bytes[idx] != b',' {
        let found = if idx < bytes.len() {
            bytes[idx] as char
        } else {
            '\0'
        };
        eprint!("Found incorrect end after {}, found: {}", field, found);
        std::process::exit(1);
    }

    let value = String::from_utf8_lossy(&bytes[..idx]).into_owned();
    *file = &bytes[idx + 1..];
    value
}

pub fn load_file(c: &mut Command, file: &str) {
    let content = read_file(file);
    let bytes = content.into_bytes();
    let mut cursor: &[u8] = &bytes;
    while !cursor.is_empty() && cursor[0] != 0 {
        read_line(c, &mut cursor);
    }
}

pub fn start(c: &Command) {
    let mut current: Option<&Command> = Some(c);
    while let Some(cmd) = current {
        if cmd.children.is_none() {
            break;
        }
        let last_printed = print_command(cmd);
        let key = getch();
        let next = find_command(cmd, key);
        clear_lines(last_printed);
        match next {
            Some(found) => {
                if command_run(found) > 0 {
                    return;
                }
                current = Some(found);
            }
            None => {
                current = None;
            }
        }
    }
}

pub fn tree_add_command(tree: &mut Command, keys: &str, name: &str, command: &str) {
    let mut chars = keys.chars();
    let first_key = match chars.next() {
        Some(k) => k,
        None => return,
    };
    let rest = chars.as_str();

    if rest.is_empty() {
        // last key - either insert new or update existing
        if let Some(existing) = find_command_mut(tree, first_key) {
            existing.name = name.to_string();
            existing.command = command.to_string();
        } else {
            command_add_child(
                tree,
                Command::new(first_key, name.to_string(), command.to_string()),
            );
        }
        return;
    }

    if find_command_mut(tree, first_key).is_none() {
        command_add_child(
            tree,
            Command::new(first_key, DEFAULT_NAME.to_string(), String::new()),
        );
    }

    let child = find_command_mut(tree, first_key).expect("child must exist after insertion");
    tree_add_command(child, rest, name, command);
}

pub fn read_line(c: &mut Command, file: &mut &[u8]) {
    let key = read_field(file, "key");
    let name = read_field(file, "name");
    let command = read_until_eol(file);

    tree_add_command(c, &key, &name, &command);
}

pub fn command_add_child(c: &mut Command, child: Command) {
    let mut new_child = Box::new(child);

    // Empty list -> place at head
    if c.children.is_none() {
        c.children = Some(new_child);
        return;
    }

    // First child has greater key -> insert at head
    let first_key = c.children.as_ref().unwrap().key;
    if first_key > new_child.key {
        new_child.next = c.children.take();
        c.children = Some(new_child);
        return;
    }

    // Walk to insertion point: insert after the last node whose
    // next has key <= new_child.key
    let mut current: &mut Box<Command> = c.children.as_mut().unwrap();
    while current
        .next
        .as_ref()
        .map(|n| n.key <= new_child.key)
        .unwrap_or(false)
    {
        current = current.next.as_mut().unwrap();
    }

    new_child.next = current.next.take();
    current.next = Some(new_child);
}

pub fn getch() -> char {
    let mut buf = [0u8; 1];
    if std::io::stdin().read_exact(&mut buf).is_err() {
        return '\0';
    }
    buf[0] as char
}

pub fn print_command(c: &Command) -> i32 {
    // Default terminal width since we don't have ioctl access without unsafe/libc.
    let width: usize = 80;
    let mut lines: i32 = 0;

    if !c.name.is_empty() {
        eprintln!("{}{}:{}", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    // Find the longest child name length
    let mut max_line_width: usize = 0;
    let mut child = c.children.as_deref();
    while let Some(node) = child {
        let line_width = node.name.len();
        if line_width > max_line_width {
            max_line_width = line_width;
        }
        child = node.next.as_deref();
    }

    max_line_width += RIGHT_MARGIN;
    if max_line_width > width {
        max_line_width = width;
    }

    // 5 is extra characters printed before each item
    let denom = max_line_width + 5;
    let items_per_row = if denom == 0 { 1 } else { width / denom };
    let items_per_row = if items_per_row == 0 { 1 } else { items_per_row };

    let mut child = c.children.as_deref();
    let mut current_item: usize = 0;
    while let Some(node) = child {
        current_item += 1;

        if node.children.is_some() {
            eprint!(
                "{}{}{} {}\u{2794}{} {}+{:<width$}{}",
                YELLOW,
                node.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                BLUE,
                node.name,
                COLOR_OFF,
                width = max_line_width
            );
        } else {
            eprint!(
                "{}{}{} {}\u{2794}{}  {:<width$}",
                YELLOW,
                node.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                node.name,
                width = max_line_width
            );
        }

        if current_item % items_per_row == 0 {
            eprintln!();
            lines += 1;
        }

        child = node.next.as_deref();
    }

    eprintln!();
    lines += 1;

    lines
}

pub fn clear_lines(count: i32) {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.flush();
    for _ in 0..count {
        eprint!("\x1b[A\r\x1b[2K");
    }
    let _ = std::io::stderr().flush();
}

pub fn find_command(c: &Command, key: char) -> Option<&Command> {
    let mut child = c.children.as_deref();
    while let Some(node) = child {
        if node.key == key {
            return Some(node);
        }
        child = node.next.as_deref();
    }
    None
}

pub fn read_until_eol(file: &mut &[u8]) -> String {
    let bytes: &[u8] = *file;
    let mut idx = 0usize;
    while idx < bytes.len() && bytes[idx] != b'\n' && bytes[idx] != 0 {
        idx += 1;
    }

    let s = String::from_utf8_lossy(&bytes[..idx]).into_owned();
    if idx < bytes.len() && bytes[idx] == b'\n' {
        *file = &bytes[idx + 1..];
    } else {
        *file = &bytes[idx..];
    }
    s
}

pub fn read_file(file: &str) -> String {
    match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open file: {}", e);
            std::process::exit(1);
        }
    }
}

pub fn command_run(c: &Command) -> i32 {
    if !c.command.is_empty() {
        print!("{}", c.command);
        let _ = std::io::stdout().flush();
        c.command.len() as i32
    } else {
        0
    }
}
