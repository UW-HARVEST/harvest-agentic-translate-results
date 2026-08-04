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
    let mut current = c.children.as_deref_mut();
    while let Some(cmd) = current {
        if cmd.key == key {
            return Some(cmd);
        }
        current = cmd.next.as_deref_mut();
    }
    None
}

pub fn read_field(file: &mut &[u8], field: &str) -> String {
    let mut i = 0;
    while i < file.len() && file[i] != b',' && file[i] != b'\n' && file[i] != 0 {
        i += 1;
    }

    if i >= file.len() || file[i] != b',' {
        let found = if i < file.len() { file[i] as char } else { '\0' };
        eprintln!("Found incorrect end after {}, found: {}", field, found);
        std::process::exit(1);
    }

    let result = String::from_utf8_lossy(&file[..i]).into_owned();
    *file = &file[i + 1..];
    result
}

pub fn load_file(c: &mut Command, file: &str) {
    let content = read_file(file);
    let mut bytes: &[u8] = content.as_bytes();
    while !bytes.is_empty() && bytes[0] != 0 {
        read_line(c, &mut bytes);
    }
}

pub fn start(c: &Command) {
    let mut current: Option<&Command> = Some(c);
    loop {
        match current {
            Some(cmd) if cmd.children.is_some() => {
                let last_printed_lines = print_command(cmd);
                let key = getch();
                let next = find_command(cmd, key);
                clear_lines(last_printed_lines);
                if let Some(n) = next {
                    if command_run(n) > 0 {
                        return;
                    }
                }
                current = next;
            }
            _ => break,
        }
    }
}

pub fn tree_add_command(tree: &mut Command, keys: &str, name: &str, command: &str) {
    let mut chars = keys.chars();
    let key = match chars.next() {
        Some(k) => k,
        None => return,
    };
    let rest_start = key.len_utf8();
    let rest = &keys[rest_start..];

    if rest.is_empty() {
        if let Some(c) = find_command_mut(tree, key) {
            c.name = name.to_string();
            c.command = command.to_string();
        } else {
            command_add_child(
                tree,
                Command::new(key, name.to_string(), command.to_string()),
            );
        }
        return;
    }

    if find_command_mut(tree, key).is_none() {
        command_add_child(
            tree,
            Command::new(key, DEFAULT_NAME.to_string(), String::new()),
        );
    }

    let child = find_command_mut(tree, key).unwrap();
    tree_add_command(child, rest, name, command);
}

pub fn read_line(c: &mut Command, file: &mut &[u8]) {
    let key = read_field(file, "key");
    let name = read_field(file, "name");
    let command = read_until_eol(file);
    tree_add_command(c, &key, &name, &command);
}

pub fn command_add_child(c: &mut Command, child: Command) {
    let mut child = Box::new(child);

    // If no children, set as first child
    if c.children.is_none() {
        c.children = Some(child);
        return;
    }

    // If first child's key > new child's key, prepend
    {
        let first = c.children.as_ref().unwrap();
        if first.key > child.key {
            child.next = c.children.take();
            c.children = Some(child);
            return;
        }
    }

    // Walk to find insertion point
    let mut current: &mut Command = c.children.as_deref_mut().unwrap();
    while current.next.is_some() && current.next.as_ref().unwrap().key <= child.key {
        current = current.next.as_deref_mut().unwrap();
    }

    child.next = current.next.take();
    current.next = Some(child);
}

pub fn getch() -> char {
    use std::io::Read;
    let mut buf = [0u8; 1];
    if std::io::stdin().read(&mut buf).is_err() {
        return '\0';
    }
    buf[0] as char
}

pub fn print_command(c: &Command) -> i32 {
    use std::io::Write;
    let stderr = std::io::stderr();
    let mut stderr = stderr.lock();

    let width: usize = std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(80);

    let mut lines = 0i32;

    if !c.name.is_empty() {
        let _ = write!(stderr, "{}{}:{}\n", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    // Find longest item
    let mut max_line_width = 0usize;
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
    let mut current_item = 0usize;
    while let Some(ch) = child {
        current_item += 1;

        if ch.children.is_some() {
            let _ = write!(
                stderr,
                "{}{}{} {}\u{2794}{} {}+{:<width$}{}",
                YELLOW,
                ch.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                BLUE,
                ch.name,
                COLOR_OFF,
                width = max_line_width
            );
        } else {
            let _ = write!(
                stderr,
                "{}{}{} {}\u{2794}{}  {:<width$}",
                YELLOW,
                ch.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                ch.name,
                width = max_line_width
            );
        }

        if current_item % items_per_row == 0 {
            let _ = writeln!(stderr);
            lines += 1;
        }

        child = ch.next.as_deref();
    }

    let _ = writeln!(stderr);
    lines += 1;

    lines
}

pub fn clear_lines(count: i32) {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    for _ in 0..count {
        eprint!("\x1b[A\r\x1b[2K");
    }
}

pub fn find_command(c: &Command, key: char) -> Option<&Command> {
    let mut current = c.children.as_deref();
    while let Some(cmd) = current {
        if cmd.key == key {
            return Some(cmd);
        }
        current = cmd.next.as_deref();
    }
    None
}

pub fn read_until_eol(file: &mut &[u8]) -> String {
    let mut i = 0;
    while i < file.len() && file[i] != b'\n' && file[i] != 0 {
        i += 1;
    }

    let result = String::from_utf8_lossy(&file[..i]).into_owned();
    if i < file.len() && file[i] == b'\n' {
        *file = &file[i + 1..];
    } else {
        *file = &file[i..];
    }
    result
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
        c.command.len() as i32
    } else {
        0
    }
}
