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
    let mut end = 0;
    while end < file.len() && file[end] != b',' && file[end] != b'\n' {
        end += 1;
    }

    if end >= file.len() || file[end] != b',' {
        let found = if end < file.len() {
            file[end] as char
        } else {
            '\0'
        };
        eprintln!("Found incorrect end after {}, found: {}", field, found);
        std::process::exit(1);
    }

    let key = std::str::from_utf8(&file[..end]).unwrap().to_string();
    *file = &file[end + 1..];
    key
}

pub fn load_file(c: &mut Command, file: &str) {
    let content = read_file(file);
    let bytes = content.as_bytes();
    let mut slice: &[u8] = bytes;
    while !slice.is_empty() {
        read_line(c, &mut slice);
    }
}

pub fn start(c: &Command) {
    let mut current: Option<&Command> = Some(c);
    while let Some(node) = current {
        if node.children.is_none() {
            break;
        }
        let last_printed_lines = print_command(node);
        let ch = getch();
        let next = find_command(node, ch);
        clear_lines(last_printed_lines);
        let run_count = match next {
            Some(n) => command_run(n),
            None => 0,
        };
        if run_count > 0 {
            return;
        }
        current = next;
    }
}

pub fn tree_add_command(tree: &mut Command, keys: &str, name: &str, command: &str) {
    let mut chars = keys.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return,
    };
    let rest: String = chars.collect();

    if rest.is_empty() {
        // Last key, add or update at this level
        if let Some(existing) = find_command_mut(tree, first) {
            existing.name = name.to_string();
            existing.command = command.to_string();
        } else {
            command_add_child(
                tree,
                Command::new(first, name.to_string(), command.to_string()),
            );
        }
        return;
    }

    // Need to recurse
    if find_command(tree, first).is_none() {
        command_add_child(
            tree,
            Command::new(first, DEFAULT_NAME.to_string(), String::new()),
        );
    }

    let child = find_command_mut(tree, first).unwrap();
    tree_add_command(child, &rest, name, command);
}

pub fn read_line(c: &mut Command, file: &mut &[u8]) {
    let key = read_field(file, "key");
    let name = read_field(file, "name");
    let command = read_until_eol(file);
    tree_add_command(c, &key, &name, &command);
}

pub fn command_add_child(c: &mut Command, mut child: Command) {
    // If no children yet, become first child
    if c.children.is_none() {
        c.children = Some(Box::new(child));
        return;
    }

    // If new child should come before head
    let head = c.children.as_ref().unwrap();
    if head.key > child.key {
        let old_head = c.children.take().unwrap();
        child.next = Some(old_head);
        c.children = Some(Box::new(child));
        return;
    }

    // Walk to find insertion point
    let mut current = c.children.as_mut().unwrap();
    while let Some(ref next) = current.next {
        if next.key > child.key {
            break;
        }
        current = current.next.as_mut().unwrap();
    }

    let after = current.next.take();
    child.next = after;
    current.next = Some(Box::new(child));
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
    let width: usize = std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(80);

    let mut lines = 0;

    if !c.name.is_empty() {
        eprintln!("{}{}:{}", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    // Find longest item
    let mut max_line_width: usize = 0;
    let mut child_ref = c.children.as_deref();
    while let Some(child) = child_ref {
        let line_width = child.name.chars().count();
        if line_width > max_line_width {
            max_line_width = line_width;
        }
        child_ref = child.next.as_deref();
    }

    max_line_width += RIGHT_MARGIN;
    if max_line_width > width {
        max_line_width = width;
    }

    let items_per_row = if max_line_width + 5 == 0 {
        1
    } else {
        let denom = max_line_width + 5;
        let v = width / denom;
        if v == 0 {
            1
        } else {
            v
        }
    };

    let mut child_ref = c.children.as_deref();
    let mut current_item = 0;
    while let Some(child) = child_ref {
        current_item += 1;

        if child.children.is_some() {
            eprint!(
                "{}{}{} {}\u{2794}{} {}+{:<width$}{}",
                YELLOW,
                child.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                BLUE,
                child.name,
                COLOR_OFF,
                width = max_line_width
            );
        } else {
            eprint!(
                "{}{}{} {}\u{2794}{}  {:<width$}",
                YELLOW,
                child.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                child.name,
                width = max_line_width
            );
        }

        if current_item % items_per_row == 0 {
            eprintln!();
            lines += 1;
        }

        child_ref = child.next.as_deref();
    }

    eprintln!();
    lines += 1;

    lines
}

pub fn clear_lines(count: i32) {
    for _ in 0..count {
        eprint!("\x1b[A\r\x1b[2K");
    }
}

pub fn find_command(c: &Command, key: char) -> Option<&Command> {
    let mut child_ref = c.children.as_deref();
    while let Some(child) = child_ref {
        if child.key == key {
            return Some(child);
        }
        child_ref = child.next.as_deref();
    }
    None
}

fn find_command_mut(c: &mut Command, key: char) -> Option<&mut Command> {
    let mut current = c.children.as_deref_mut();
    while let Some(child) = current {
        if child.key == key {
            return Some(child);
        }
        current = child.next.as_deref_mut();
    }
    None
}

pub fn read_until_eol(file: &mut &[u8]) -> String {
    let mut end = 0;
    while end < file.len() && file[end] != b'\n' {
        end += 1;
    }

    let s = std::str::from_utf8(&file[..end]).unwrap().to_string();

    if end < file.len() && file[end] == b'\n' {
        *file = &file[end + 1..];
    } else {
        *file = &file[end..];
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
        return c.command.len() as i32;
    }
    0
}
