use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal,
};
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
pub fn read_field(file: &mut &[u8], field: &str) -> String {
    let end = file
        .iter()
        .position(|&byte| matches!(byte, b',' | b'\n' | 0))
        .unwrap_or(file.len());

    let value = String::from_utf8_lossy(&file[..end]).into_owned();
    *file = &file[end..];

    match file.first().copied() {
        Some(b',') => {
            *file = &file[1..];
            value
        }
        Some(found) => {
            eprintln!(
                "Found incorrect end after {}, found: {}",
                field, found as char
            );
            std::process::exit(1);
        }
        None => {
            eprintln!("Found incorrect end after {}, found: EOF", field);
            std::process::exit(1);
        }
    }
}
pub fn load_file(c: &mut Command, file: &str) {
    let content = read_file(file);
    let mut remaining = content.as_bytes();
    while !remaining.is_empty() {
        read_line(c, &mut remaining);
    }
}
pub fn start(c: &Command) {
    let mut current = Some(c);

    while let Some(command) = current {
        if command.children.is_none() {
            break;
        }

        let last_printed_lines = print_command(command);
        current = find_command(command, getch());
        clear_lines(last_printed_lines);

        if let Some(next) = current {
            if command_run(next) > 0 {
                return;
            }
        }
    }
}
pub fn tree_add_command(tree: &mut Command, keys: &str, name: &str, command: &str) {
    if keys.is_empty() {
        return;
    }

    tree_add_command_bytes(tree, keys.as_bytes(), name, command);
}
pub fn read_line(c: &mut Command, file: &mut &[u8]) {
    let key = read_field(file, "key");
    let name = read_field(file, "name");
    let command = read_until_eol(file);

    tree_add_command(c, &key, &name, &command);
}
pub fn command_add_child(c: &mut Command, child: Command) {
    let mut child = Box::new(child);

    match c.children.as_mut() {
        None => {
            c.children = Some(child);
            return;
        }
        Some(first_child) if first_child.key > child.key => {
            child.next = c.children.take();
            c.children = Some(child);
            return;
        }
        Some(_) => {}
    }

    let mut current = c.children.as_mut().expect("child list exists");
    while current
        .next
        .as_ref()
        .is_some_and(|next_child| next_child.key <= child.key)
    {
        current = current.next.as_mut().expect("next child exists");
    }

    child.next = current.next.take();
    current.next = Some(child);
}
pub fn getch() -> char {
    terminal::enable_raw_mode().ok();

    let key = loop {
        match event::read() {
            Ok(Event::Key(key_event))
                if matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
            {
                break match key_event.code {
                    KeyCode::Char(c) => c,
                    KeyCode::Enter => '\n',
                    KeyCode::Tab => '\t',
                    KeyCode::Backspace => '\u{0008}',
                    KeyCode::Esc => '\u{001b}',
                    _ => continue,
                };
            }
            Ok(_) => {}
            Err(_) => break '\0',
        }
    };

    terminal::disable_raw_mode().ok();
    key
}
pub fn print_command(c: &Command) -> i32 {
    let width = terminal::size().map(|(w, _)| usize::from(w)).unwrap_or(80);
    let mut lines = 0;

    if !c.name.is_empty() {
        eprintln!("{}{}:{}", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    let mut max_line_width = 0usize;
    let mut child = c.children.as_deref();
    while let Some(command) = child {
        max_line_width = max_line_width.max(command.name.len());
        child = command.next.as_deref();
    }

    max_line_width += RIGHT_MARGIN;
    if max_line_width > width {
        max_line_width = width;
    }

    let items_per_row = (width / (max_line_width + 5)).max(1);
    let mut current_item = 0usize;
    let mut child = c.children.as_deref();

    while let Some(command) = child {
        current_item += 1;

        if command.children.is_some() {
            eprint!(
                "{}{}{} {}➔{} {}+{:<width$}{}",
                YELLOW,
                command.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                BLUE,
                command.name,
                COLOR_OFF,
                width = max_line_width
            );
        } else {
            eprint!(
                "{}{}{} {}➔{}  {:<width$}",
                YELLOW,
                command.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                command.name,
                width = max_line_width
            );
        }

        if current_item % items_per_row == 0 {
            eprintln!();
            lines += 1;
        }

        child = command.next.as_deref();
    }

    eprintln!();
    let _ = io::stderr().flush();
    lines + 1
}
pub fn clear_lines(count: i32) {
    let _ = io::stdout().flush();
    for _ in 0..count.max(0) {
        eprint!("\x1b[A\r\x1b[2K");
    }
    let _ = io::stderr().flush();
}
pub fn find_command(c: &Command, key: char) -> Option<&Command> {
    let mut child = c.children.as_deref();
    while let Some(command) = child {
        if command.key == key {
            return Some(command);
        }
        child = command.next.as_deref();
    }
    None
}
pub fn read_until_eol(file: &mut &[u8]) -> String {
    let end = file
        .iter()
        .position(|&byte| matches!(byte, b'\n' | 0))
        .unwrap_or(file.len());
    let value = String::from_utf8_lossy(&file[..end]).into_owned();
    *file = &file[end..];

    if file.first() == Some(&b'\n') {
        *file = &file[1..];
    }

    value
}
pub fn read_file(file: &str) -> String {
    match fs::read_to_string(file) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("Failed to open file: {}", error);
            std::process::exit(1);
        }
    }
}
pub fn command_run(c: &Command) -> i32 {
    if c.command.is_empty() {
        return 0;
    }

    print!("{}", c.command);
    let _ = io::stdout().flush();
    c.command.len() as i32
}

fn tree_add_command_bytes(tree: &mut Command, keys: &[u8], name: &str, command: &str) {
    let key = keys[0] as char;

    if keys.len() == 1 {
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

    let child = get_or_insert_child(tree, key);
    tree_add_command_bytes(child, &keys[1..], name, command);
}

fn find_command_mut(c: &mut Command, key: char) -> Option<&mut Command> {
    let mut child = c.children.as_deref_mut();
    while let Some(command) = child {
        if command.key == key {
            return Some(command);
        }
        child = command.next.as_deref_mut();
    }
    None
}

fn get_or_insert_child(c: &mut Command, key: char) -> &mut Command {
    let mut link = &mut c.children;

    loop {
        if link.as_ref().is_some_and(|child| child.key == key) {
            return link.as_deref_mut().expect("child exists");
        }

        if link.as_ref().is_some_and(|child| child.key > key) {
            let mut child = Box::new(Command::new(key, DEFAULT_NAME.to_string(), String::new()));
            child.next = link.take();
            *link = Some(child);
            return link.as_deref_mut().expect("child inserted");
        }

        match link {
            Some(node) => {
                link = &mut node.next;
            }
            None => {
                *link = Some(Box::new(Command::new(
                    key,
                    DEFAULT_NAME.to_string(),
                    String::new(),
                )));
                return link.as_deref_mut().expect("child inserted");
            }
        }
    }
}
