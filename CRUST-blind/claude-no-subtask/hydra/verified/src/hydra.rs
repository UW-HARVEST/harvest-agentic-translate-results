use std::io::Read;
use std::io::Write;

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

// Helper: find a child with the given key, returning a mutable reference.
fn find_child_mut(c: &mut Command, key: char) -> Option<&mut Command> {
    fn walk(opt: &mut Option<Box<Command>>, key: char) -> Option<&mut Command> {
        match opt {
            None => None,
            Some(node) => {
                if node.key == key {
                    Some(node.as_mut())
                } else {
                    walk(&mut node.next, key)
                }
            }
        }
    }
    walk(&mut c.children, key)
}

pub fn read_field(file: &mut &[u8], field: &str) -> String {
    let bytes: &[u8] = *file;
    let mut idx = 0;
    while idx < bytes.len() && bytes[idx] != b',' && bytes[idx] != b'\n' {
        idx += 1;
    }

    if idx >= bytes.len() || bytes[idx] != b',' {
        let found = if idx < bytes.len() { bytes[idx] as char } else { '\0' };
        eprint!("Found incorrect end after {}, found: {}", field, found);
        std::process::exit(1);
    }

    let result = String::from_utf8_lossy(&bytes[..idx]).into_owned();
    *file = &bytes[idx + 1..];
    result
}

pub fn load_file(c: &mut Command, file: &str) {
    let content = read_file(file);
    let bytes = content.as_bytes();
    let mut cursor: &[u8] = bytes;
    while !cursor.is_empty() {
        read_line(c, &mut cursor);
    }
}

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
        let run_result = match next {
            Some(n) => command_run(n),
            None => 0,
        };
        if run_result > 0 {
            return;
        }
        current = next;
    }
}

pub fn tree_add_command(tree: &mut Command, keys: &str, name: &str, command: &str) {
    let mut chars = keys.chars();
    let key = match chars.next() {
        Some(k) => k,
        None => return,
    };
    let rest: String = chars.collect();

    if rest.is_empty() {
        if let Some(existing) = find_child_mut(tree, key) {
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

    if find_child_mut(tree, key).is_none() {
        command_add_child(
            tree,
            Command::new(key, DEFAULT_NAME.to_string(), String::new()),
        );
    }
    let child = find_child_mut(tree, key).expect("child just inserted");
    tree_add_command(child, &rest, name, command);
}

pub fn read_line(c: &mut Command, file: &mut &[u8]) {
    let key = read_field(file, "key");
    let name = read_field(file, "name");
    let command = read_until_eol(file);
    tree_add_command(c, &key, &name, &command);
}

pub fn command_add_child(c: &mut Command, child: Command) {
    let mut child_box = Box::new(child);

    // Case 1: no children yet.
    if c.children.is_none() {
        c.children = Some(child_box);
        return;
    }

    // Case 2: insert at front.
    if c.children.as_ref().unwrap().key > child_box.key {
        child_box.next = c.children.take();
        c.children = Some(child_box);
        return;
    }

    // Case 3: walk to find insertion point.
    let mut last_child: &mut Box<Command> = c.children.as_mut().unwrap();
    loop {
        let advance = match &last_child.next {
            Some(n) if n.key <= child_box.key => true,
            _ => false,
        };
        if !advance {
            break;
        }
        last_child = last_child.next.as_mut().unwrap();
    }

    child_box.next = last_child.next.take();
    last_child.next = Some(child_box);
}

pub fn getch() -> char {
    let mut buf = [0u8; 1];
    match std::io::stdin().read_exact(&mut buf) {
        Ok(()) => buf[0] as char,
        Err(_) => '\0',
    }
}

pub fn print_command(c: &Command) -> i32 {
    let width: i32 = std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80);

    let mut lines = 0i32;

    // Equivalent to C's `if (c->name)` — always true here since name is a String.
    if !c.name.is_empty() {
        eprintln!("{}{}:{}", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    // Find longest item.
    let mut max_line_width: i32 = 0;
    let mut child = c.children.as_deref();
    while let Some(node) = child {
        let line_width = node.name.len() as i32;
        if line_width > max_line_width {
            max_line_width = line_width;
        }
        child = node.next.as_deref();
    }

    max_line_width += RIGHT_MARGIN as i32;
    if max_line_width > width {
        max_line_width = width;
    }

    let mut items_per_row = width / (max_line_width + 5);
    if items_per_row <= 0 {
        items_per_row = 1;
    }

    let mut child = c.children.as_deref();
    let mut current_item: i32 = 0;
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    while let Some(node) = child {
        current_item += 1;

        if node.children.is_some() {
            let _ = write!(
                handle,
                "{}{}{} {}➔{} {}+{:<width$}{}",
                YELLOW,
                node.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                BLUE,
                node.name,
                COLOR_OFF,
                width = max_line_width as usize,
            );
        } else {
            let _ = write!(
                handle,
                "{}{}{} {}➔{}  {:<width$}",
                YELLOW,
                node.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                node.name,
                width = max_line_width as usize,
            );
        }

        if current_item % items_per_row == 0 {
            let _ = writeln!(handle);
            lines += 1;
        }

        child = node.next.as_deref();
    }

    let _ = writeln!(handle);
    lines += 1;

    lines
}

pub fn clear_lines(count: i32) {
    // Make sure stdout is unbuffered (similar to setbuf(stdout, NULL)).
    let _ = std::io::stdout().flush();
    for _ in 0..count {
        eprint!("\x1b[A\r\x1b[2K");
    }
    let _ = std::io::stderr().flush();
}

pub fn find_command(c: &Command, key: char) -> Option<&Command> {
    let mut cur = c.children.as_deref();
    while let Some(node) = cur {
        if node.key == key {
            return Some(node);
        }
        cur = node.next.as_deref();
    }
    None
}

pub fn read_until_eol(file: &mut &[u8]) -> String {
    let bytes: &[u8] = *file;
    let mut idx = 0;
    while idx < bytes.len() && bytes[idx] != b'\n' {
        idx += 1;
    }

    let result = String::from_utf8_lossy(&bytes[..idx]).into_owned();
    if idx < bytes.len() && bytes[idx] == b'\n' {
        *file = &bytes[idx + 1..];
    } else {
        *file = &bytes[idx..];
    }
    result
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
        c.command.len() as i32
    } else {
        0
    }
}
