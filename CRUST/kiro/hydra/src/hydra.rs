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

pub fn command_run(c: &Command) -> i32 {
    if !c.command.is_empty() {
        print!("{}", c.command);
        io::stdout().flush().ok();
        c.command.len() as i32
    } else {
        0
    }
}

pub fn command_add_child(c: &mut Command, child: Command) {
    let mut child = Box::new(child);

    if c.children.is_none() {
        c.children = Some(child);
        return;
    }

    if c.children.as_ref().unwrap().key > child.key {
        child.next = c.children.take();
        c.children = Some(child);
        return;
    }

    let mut last = c.children.as_mut().unwrap();
    while last.next.is_some() && last.next.as_ref().unwrap().key <= child.key {
        last = last.next.as_mut().unwrap();
    }
    child.next = last.next.take();
    last.next = Some(child);
}

pub fn find_command(c: &Command, key: char) -> Option<&Command> {
    let mut child = c.children.as_deref();
    while let Some(ch) = child {
        if ch.key == key {
            return Some(ch);
        }
        child = ch.next.as_deref();
    }
    None
}

fn find_command_mut(c: &mut Command, key: char) -> Option<&mut Command> {
    let mut child = c.children.as_deref_mut();
    while let Some(ch) = child {
        if ch.key == key {
            return Some(ch);
        }
        child = ch.next.as_deref_mut();
    }
    None
}

pub fn tree_add_command(tree: &mut Command, keys: &str, name: &str, command: &str) {
    let key = keys.chars().next().unwrap();
    let rest = &keys[key.len_utf8()..];

    if rest.is_empty() {
        if let Some(existing) = find_command_mut(tree, key) {
            existing.name = name.to_string();
            existing.command = command.to_string();
        } else {
            command_add_child(tree, Command::new(key, name.to_string(), command.to_string()));
        }
        return;
    }

    if find_command_mut(tree, key).is_none() {
        command_add_child(tree, Command::new(key, DEFAULT_NAME.to_string(), String::new()));
    }

    let c = find_command_mut(tree, key).unwrap();
    tree_add_command(c, rest, name, command);
}

pub fn print_command(c: &Command) -> i32 {
    let width = terminal_width();
    let mut lines = 0;

    if !c.name.is_empty() {
        eprint!("{}{}:{}\n", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    let mut max_line_width = 0;
    let mut child = c.children.as_deref();
    while let Some(ch) = child {
        let lw = ch.name.len();
        if lw > max_line_width {
            max_line_width = lw;
        }
        child = ch.next.as_deref();
    }

    max_line_width += RIGHT_MARGIN;
    if max_line_width > width {
        max_line_width = width;
    }

    let items_per_row = width / (max_line_width + 5);

    child = c.children.as_deref();
    let mut current_item = 0;
    while let Some(ch) = child {
        current_item += 1;

        if ch.children.is_some() {
            eprint!(
                "{}{}{} {}➔{} {}+{:<width$}{}",
                YELLOW, ch.key, COLOR_OFF, PURPLE, COLOR_OFF, BLUE, ch.name, COLOR_OFF,
                width = max_line_width
            );
        } else {
            eprint!(
                "{}{}{} {}➔{}  {:<width$}",
                YELLOW, ch.key, COLOR_OFF, PURPLE, COLOR_OFF, ch.name,
                width = max_line_width
            );
        }

        if items_per_row > 0 && current_item % items_per_row == 0 {
            eprint!("\n");
            lines += 1;
        }

        child = ch.next.as_deref();
    }

    eprint!("\n");
    lines += 1;

    lines
}

fn terminal_width() -> usize {
    // Try to get terminal width, default to 80
    if let Some((w, _)) = term_size() {
        w
    } else {
        80
    }
}

fn term_size() -> Option<(usize, usize)> {
    // Use ioctl TIOCGWINSZ via libc-free approach
    // We'll just use a simple default or environment variable
    if let Ok(cols) = std::env::var("COLUMNS") {
        if let Ok(c) = cols.parse::<usize>() {
            return Some((c, 24));
        }
    }
    // Try ioctl manually using nix-style raw syscall - but we want to stay safe
    // Default fallback
    Some((80, 24))
}

pub fn getch() -> char {
    // Read a single character from stdin without echo
    let mut buf = [0u8; 1];
    io::stdin().read_exact(&mut buf).unwrap_or(());
    buf[0] as char
}

pub fn read_file(file: &str) -> String {
    std::fs::read_to_string(file).unwrap_or_else(|e| {
        eprintln!("Failed to open file: {}", e);
        std::process::exit(1);
    })
}

pub fn read_field(file: &mut &[u8], field: &str) -> String {
    let start = *file;
    let mut i = 0;
    while i < file.len() && file[i] != b',' && file[i] != b'\n' && file[i] != 0 {
        i += 1;
    }

    if i >= file.len() || file[i] != b',' {
        let found = if i < file.len() { file[i] as char } else { '?' };
        eprintln!("Found incorrect end after {}, found: {}", field, found);
        std::process::exit(1);
    }

    let key = std::str::from_utf8(&start[..i]).unwrap().to_string();
    *file = &file[i + 1..];
    key
}

pub fn read_until_eol(file: &mut &[u8]) -> String {
    let mut i = 0;
    while i < file.len() && file[i] != b'\n' && file[i] != 0 {
        i += 1;
    }

    let s = std::str::from_utf8(&file[..i]).unwrap().to_string();

    if i < file.len() && file[i] == b'\n' {
        *file = &file[i + 1..];
    } else {
        *file = &file[i..];
    }

    s
}

pub fn read_line(c: &mut Command, file: &mut &[u8]) {
    let key = read_field(file, "key");
    let name = read_field(file, "name");
    let command = read_until_eol(file);
    tree_add_command(c, &key, &name, &command);
}

pub fn clear_lines(count: i32) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    for _ in 0..count {
        write!(handle, "\x1b[A\r\x1b[2K").ok();
    }
    handle.flush().ok();
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
        clear_lines(last_printed_lines);
        if let Some(found) = find_command(cmd, ch) {
            if command_run(found) > 0 {
                return;
            }
            current = Some(found);
        } else {
            break;
        }
    }
}
