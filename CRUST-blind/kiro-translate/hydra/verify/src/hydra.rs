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

fn find_command_mut(c: &mut Command, key: char) -> Option<&mut Command> {
    let mut child = c.children.as_deref_mut();
    while let Some(node) = child {
        if node.key == key {
            return Some(node);
        }
        child = node.next.as_deref_mut();
    }
    None
}

pub fn read_field(file: &mut &[u8], field: &str) -> String {
    let start = *file;
    let mut i = 0;
    while i < file.len() && file[i] != b',' && file[i] != b'\n' && file[i] != 0 {
        i += 1;
    }
    if i >= file.len() || file[i] != b',' {
        eprintln!("Found incorrect end after {}, found: {}", field,
            if i < file.len() { file[i] as char } else { '\0' });
        std::process::exit(1);
    }
    let key = String::from_utf8_lossy(&start[..i]).to_string();
    *file = &file[i + 1..];
    key
}

pub fn load_file(c: &mut Command, file: &str) {
    let content = read_file(file);
    let bytes = content.into_bytes();
    let mut slice: &[u8] = &bytes;
    while !slice.is_empty() {
        read_line(c, &mut slice);
    }
}

pub fn start(c: &Command) {
    let mut current = Some(c);
    while let Some(cmd) = current {
        if cmd.children.is_none() {
            break;
        }
        let last_printed_lines = print_command(cmd);
        let ch = getch();
        let found = find_command(cmd, ch);
        clear_lines(last_printed_lines);
        if let Some(found_cmd) = found {
            if command_run(found_cmd) > 0 {
                return;
            }
            // Navigate into the found command's subtree
            current = Some(found_cmd);
        } else {
            current = None;
        }
    }
}

pub fn tree_add_command(tree: &mut Command, keys: &str, name: &str, command: &str) {
    let key_bytes = keys.as_bytes();
    let first_key = key_bytes[0] as char;

    if key_bytes.len() == 1 {
        // Check if command already exists
        if let Some(existing) = find_command_mut(tree, first_key) {
            existing.name = name.to_string();
            existing.command = command.to_string();
        } else {
            command_add_child(tree, Command::new(first_key, name.to_string(), command.to_string()));
        }
        return;
    }

    // Need to find or create intermediate node
    let has_child = find_command_mut(tree, first_key).is_some();
    if !has_child {
        command_add_child(tree, Command::new(first_key, DEFAULT_NAME.to_string(), String::new()));
    }
    let c = find_command_mut(tree, first_key).unwrap();
    tree_add_command(c, &keys[1..], name, command);
}

pub fn read_line(c: &mut Command, file: &mut &[u8]) {
    let key = read_field(file, "key");
    let name = read_field(file, "name");
    let command = read_until_eol(file);
    tree_add_command(c, &key, &name, &command);
}

pub fn command_add_child(c: &mut Command, mut child: Command) {
    if c.children.is_none() {
        c.children = Some(Box::new(child));
        return;
    }

    if c.children.as_ref().unwrap().key > child.key {
        child.next = c.children.take();
        c.children = Some(Box::new(child));
        return;
    }

    let mut last = c.children.as_deref_mut().unwrap();
    while last.next.is_some() && last.next.as_ref().unwrap().key <= child.key {
        last = last.next.as_deref_mut().unwrap();
    }
    child.next = last.next.take();
    last.next = Some(Box::new(child));
}

pub fn getch() -> char {
    use termion::raw::IntoRawMode;
    let _raw = io::stdout().into_raw_mode().unwrap();
    let mut buf = [0u8; 1];
    io::stdin().read_exact(&mut buf).unwrap_or(());
    buf[0] as char
}

pub fn print_command(c: &Command) -> i32 {
    let width = terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(80);

    let mut lines: i32 = 0;

    if !c.name.is_empty() {
        eprint!("{}{}:{}\n", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    // Find longest item name
    let mut max_line_width: usize = 0;
    let mut child = c.children.as_deref();
    while let Some(node) = child {
        let lw = node.name.len();
        if lw > max_line_width {
            max_line_width = lw;
        }
        child = node.next.as_deref();
    }

    max_line_width += RIGHT_MARGIN;
    if max_line_width > width {
        max_line_width = width;
    }

    let items_per_row = width / (max_line_width + 5);

    child = c.children.as_deref();
    let mut current_item = 0;
    while let Some(node) = child {
        current_item += 1;

        if node.children.is_some() {
            eprint!("{}{}{} {}➔{} {}+{:<width$}{}",
                YELLOW, node.key, COLOR_OFF,
                PURPLE, COLOR_OFF,
                BLUE, node.name, COLOR_OFF,
                width = max_line_width);
        } else {
            eprint!("{}{}{} {}➔{}  {:<width$}",
                YELLOW, node.key, COLOR_OFF,
                PURPLE, COLOR_OFF,
                node.name,
                width = max_line_width);
        }

        if items_per_row > 0 && current_item % items_per_row == 0 {
            eprint!("\n");
            lines += 1;
        }

        child = node.next.as_deref();
    }

    eprint!("\n");
    lines += 1;

    lines
}

pub fn clear_lines(count: i32) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    for _ in 0..count {
        let _ = handle.write_all(b"\x1b[A\r\x1b[2K");
    }
    let _ = handle.flush();
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
    let mut i = 0;
    while i < file.len() && file[i] != b'\n' && file[i] != 0 {
        i += 1;
    }
    let s = String::from_utf8_lossy(&file[..i]).to_string();
    if i < file.len() && file[i] == b'\n' {
        *file = &file[i + 1..];
    } else {
        *file = &file[i..];
    }
    s
}

pub fn read_file(file: &str) -> String {
    match std::fs::read_to_string(file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Failed to open file: {}", e);
            std::process::exit(1);
        }
    }
}

pub fn command_run(c: &Command) -> i32 {
    if !c.command.is_empty() {
        print!("{}", c.command);
        let _ = io::stdout().flush();
        c.command.len() as i32
    } else {
        0
    }
}
