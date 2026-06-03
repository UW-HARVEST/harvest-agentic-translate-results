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

fn terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&w| w > 0)
        .unwrap_or(80)
}

fn find_command_mut<'a>(c: &'a mut Command, key: char) -> Option<&'a mut Command> {
    // Two-pass approach: first locate index immutably, then traverse mutably.
    let mut idx: usize = 0;
    let mut found = false;
    {
        let mut cur = c.children.as_deref();
        while let Some(node) = cur {
            if node.key == key {
                found = true;
                break;
            }
            idx += 1;
            cur = node.next.as_deref();
        }
    }
    if !found {
        return None;
    }
    let mut cur_mut: &mut Command = c.children.as_deref_mut().expect("found above");
    for _ in 0..idx {
        cur_mut = cur_mut.next.as_deref_mut().expect("indexed above");
    }
    Some(cur_mut)
}

pub fn read_field(file: &mut &[u8], field: &str) -> String {
    let mut end = 0;
    while end < file.len() && file[end] != b',' && file[end] != b'\n' && file[end] != 0 {
        end += 1;
    }

    if end >= file.len() || file[end] != b',' {
        let found = if end < file.len() {
            file[end] as char
        } else {
            '\0'
        };
        eprint!("Found incorrect end after {}, found: {}", field, found);
        std::process::exit(1);
    }

    let result = String::from_utf8_lossy(&file[..end]).into_owned();
    *file = &file[end + 1..];
    result
}

pub fn load_file(c: &mut Command, file: &str) {
    let content = read_file(file);
    let bytes = content.into_bytes();
    let mut slice: &[u8] = &bytes;
    while !slice.is_empty() && slice[0] != 0 {
        read_line(c, &mut slice);
    }
}

pub fn start(c: &Command) {
    let mut current: Option<&Command> = Some(c);
    while let Some(cur) = current {
        if cur.children.is_none() {
            break;
        }
        let last_printed_lines = print_command(cur);
        let key = getch();
        let next = find_command(cur, key);
        clear_lines(last_printed_lines);
        match next {
            Some(n) => {
                if command_run(n) > 0 {
                    return;
                }
                current = Some(n);
            }
            None => {
                // command_run(NULL) returns 0, then loop ends because c is NULL
                return;
            }
        }
    }
}

pub fn tree_add_command(tree: &mut Command, keys: &str, name: &str, command: &str) {
    let mut iter = keys.chars();
    let key = match iter.next() {
        Some(k) => k,
        None => return,
    };
    let rest: String = iter.collect();
    let is_last = rest.is_empty();

    let exists = find_command(tree, key).is_some();

    if is_last {
        if !exists {
            command_add_child(
                tree,
                Command::new(key, name.to_string(), command.to_string()),
            );
        } else if let Some(c) = find_command_mut(tree, key) {
            c.name = name.to_string();
            c.command = command.to_string();
        }
        return;
    }

    if !exists {
        command_add_child(
            tree,
            Command::new(key, DEFAULT_NAME.to_string(), String::new()),
        );
    }

    if let Some(c) = find_command_mut(tree, key) {
        tree_add_command(c, &rest, name, command);
    }
}

pub fn read_line(c: &mut Command, file: &mut &[u8]) {
    let key = read_field(file, "key");
    let name = read_field(file, "name");
    let command = read_until_eol(file);
    tree_add_command(c, &key, &name, &command);
}

pub fn command_add_child(c: &mut Command, child: Command) {
    let mut child = Box::new(child);

    // No existing children: insert as first.
    if c.children.is_none() {
        c.children = Some(child);
        return;
    }

    // First child key greater than new child's key: insert at head.
    if c.children.as_ref().unwrap().key > child.key {
        child.next = c.children.take();
        c.children = Some(child);
        return;
    }

    // Walk to find insertion point.
    let mut cur: &mut Command = c.children.as_deref_mut().unwrap();
    while cur.next.is_some() && cur.next.as_ref().unwrap().key <= child.key {
        cur = cur.next.as_deref_mut().unwrap();
    }

    child.next = cur.next.take();
    cur.next = Some(child);
}

pub fn getch() -> char {
    use std::io::Read;
    let mut buf = [0u8; 1];
    let _ = std::io::stdin().read(&mut buf);
    buf[0] as char
}

pub fn print_command(c: &Command) -> i32 {
    let width = terminal_width();

    let mut lines: i32 = 0;

    if !c.name.is_empty() {
        eprintln!("{}{}:{}", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    // Find longest item.
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

    let denom = max_line_width + 5;
    let items_per_row = if denom == 0 { 1 } else { width / denom };
    let items_per_row = items_per_row.max(1);

    let mut child = c.children.as_deref();
    let mut current_item: usize = 0;
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
    // Make sure stdout is flushed/unbuffered semantics-wise.
    let _ = std::io::stdout().flush();

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
    let mut end = 0;
    while end < file.len() && file[end] != b'\n' && file[end] != 0 {
        end += 1;
    }
    let result = String::from_utf8_lossy(&file[..end]).into_owned();
    if end < file.len() && file[end] == b'\n' {
        *file = &file[end + 1..];
    } else {
        *file = &file[end..];
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
        let _ = std::io::stdout().flush();
        c.command.len() as i32
    } else {
        0
    }
}
