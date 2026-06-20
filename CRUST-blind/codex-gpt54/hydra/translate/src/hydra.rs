use crossterm::event::{read, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size};
use std::fs;
use std::io::{self, Write};

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

fn split_first_char(s: &str) -> Option<(char, &str)> {
    let mut chars = s.char_indices();
    let (_, first) = chars.next()?;
    let next_index = chars.next().map_or(s.len(), |(idx, _)| idx);
    Some((first, &s[next_index..]))
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

fn insert_child(link: &mut Option<Box<Command>>, mut child: Box<Command>) {
    if link.as_ref().map_or(true, |node| node.key > child.key) {
        child.next = link.take();
        *link = Some(child);
        return;
    }

    if let Some(node) = link.as_mut() {
        insert_child(&mut node.next, child);
    }
}

pub fn read_field(file: &mut &[u8], field: &str) -> String {
    let mut end = 0;
    while end < file.len() && file[end] != b',' && file[end] != b'\n' && file[end] != 0 {
        end += 1;
    }

    if end >= file.len() || file[end] != b',' {
        let found = file.get(end).copied().unwrap_or(0) as char;
        eprint!("Found incorrect end after {}, found: {}", field, found);
        std::process::exit(1);
    }

    let value = String::from_utf8_lossy(&file[..end]).into_owned();
    *file = &file[end + 1..];
    value
}
pub fn load_file(c: &mut Command, file: &str) {
    let content = read_file(file);
    let mut cursor = content.as_bytes();
    while !cursor.is_empty() && cursor[0] != 0 {
        read_line(c, &mut cursor);
    }
}
pub fn start(c: &Command) {
    let mut current = Some(c);

    while let Some(node) = current {
        if node.children.is_none() {
            break;
        }

        let last_printed_lines = print_command(node);
        let selected = find_command(node, getch());
        clear_lines(last_printed_lines);

        if let Some(command) = selected {
            if command_run(command) > 0 {
                return;
            }
        }

        current = selected;
    }
}
pub fn tree_add_command(tree: &mut Command, keys: &str, name: &str, command: &str) {
    let Some((key, rest)) = split_first_char(keys) else {
        return;
    };

    if rest.is_empty() {
        if let Some(existing) = find_command_mut(tree, key) {
            existing.name = name.to_string();
            existing.command = command.to_string();
        } else {
            command_add_child(
                tree,
                Command::new(key, name.to_string(), command.to_string()),
            );
        }
        return;
    }

    if find_command(tree, key).is_none() {
        command_add_child(
            tree,
            Command::new(key, DEFAULT_NAME.to_string(), String::new()),
        );
    }

    if let Some(child) = find_command_mut(tree, key) {
        tree_add_command(child, rest, name, command);
    }
}
pub fn read_line(c: &mut Command, file: &mut &[u8]) {
    let key = read_field(file, "key");
    let name = read_field(file, "name");
    let command = read_until_eol(file);
    tree_add_command(c, &key, &name, &command);
}
pub fn command_add_child(c: &mut Command, child: Command) {
    insert_child(&mut c.children, Box::new(child));
}
pub fn getch() -> char {
    if enable_raw_mode().is_err() {
        return '\0';
    }

    loop {
        match read() {
            Ok(Event::Key(event)) => {
                let key = match event.code {
                    KeyCode::Char(c) => c,
                    KeyCode::Enter => '\n',
                    KeyCode::Tab => '\t',
                    KeyCode::Backspace => '\u{8}',
                    KeyCode::Esc => '\u{1b}',
                    _ => continue,
                };
                let _ = disable_raw_mode();
                return key;
            }
            Ok(_) => continue,
            Err(_) => {
                let _ = disable_raw_mode();
                return '\0';
            }
        }
    }
}
pub fn print_command(c: &Command) -> i32 {
    let width = size().map(|(width, _)| width as usize).unwrap_or(80);
    let mut lines = 0;

    if !c.name.is_empty() {
        eprint!("{}{}:{}\n", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    let mut max_line_width = 0usize;
    let mut child = c.children.as_deref();
    while let Some(node) = child {
        max_line_width = max_line_width.max(node.name.len());
        child = node.next.as_deref();
    }

    max_line_width = (max_line_width + RIGHT_MARGIN).min(width);
    let items_per_row = (width / (max_line_width + 5)).max(1);

    child = c.children.as_deref();
    let mut current_item = 0usize;
    while let Some(node) = child {
        current_item += 1;

        if node.children.is_some() {
            eprint!(
                "{}{}{} {}➔{} {}+{:<width$}{}",
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
                "{}{}{} {}➔{}  {:<width$}",
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
    let mut stderr = io::stderr().lock();
    for _ in 0..count.max(0) {
        let _ = stderr.write_all(b"\x1b[A\r\x1b[2K");
    }
    let _ = stderr.flush();
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
    let mut end = 0;
    while end < file.len() && file[end] != b'\n' && file[end] != 0 {
        end += 1;
    }

    let value = String::from_utf8_lossy(&file[..end]).into_owned();
    *file = if end < file.len() && file[end] == b'\n' {
        &file[end + 1..]
    } else {
        &file[end..]
    };
    value
}
pub fn read_file(file: &str) -> String {
    match fs::read(file) {
        Ok(content) => String::from_utf8_lossy(&content).into_owned(),
        Err(err) => {
            eprintln!("Failed to open file: {}", err);
            std::process::exit(1);
        }
    }
}
pub fn command_run(c: &Command) -> i32 {
    if c.command.is_empty() {
        return 0;
    }

    let mut stdout = io::stdout().lock();
    if stdout.write_all(c.command.as_bytes()).is_ok() && stdout.flush().is_ok() {
        c.command.len() as i32
    } else {
        0
    }
}
