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

fn find_child_mut(c: &mut Command, key: char) -> Option<&mut Command> {
    let mut current = &mut c.children;
    while current.is_some() {
        if current.as_ref().unwrap().key == key {
            return current.as_deref_mut();
        }
        current = &mut current.as_mut().unwrap().next;
    }
    None
}

pub fn read_field(file: &mut &[u8], field: &str) -> String {
    let mut i = 0;
    while i < file.len() && file[i] != b',' && file[i] != b'\n' {
        i += 1;
    }

    if i >= file.len() || file[i] != b',' {
        let found = if i < file.len() {
            file[i] as char
        } else {
            '\0'
        };
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
    while !bytes.is_empty() {
        read_line(c, &mut bytes);
    }
}

pub fn start(c: &Command) {
    let mut current: Option<&Command> = Some(c);
    while let Some(cmd) = current {
        if cmd.children.is_none() {
            return;
        }
        let last_printed_lines = print_command(cmd);
        let key = getch();
        let next = find_command(cmd, key);
        clear_lines(last_printed_lines);
        match next {
            Some(n) => {
                if command_run(n) > 0 {
                    return;
                }
                current = Some(n);
            }
            None => return,
        }
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
        // Last key in path
        if let Some(c) = find_child_mut(tree, key) {
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

    // Find or create child for current key, then recurse
    if find_child_mut(tree, key).is_none() {
        command_add_child(
            tree,
            Command::new(key, DEFAULT_NAME.to_string(), String::new()),
        );
    }

    let child = find_child_mut(tree, key).expect("child must exist after add");
    tree_add_command(child, &rest, name, command);
}

pub fn read_line(c: &mut Command, file: &mut &[u8]) {
    let key = read_field(file, "key");
    let name = read_field(file, "name");
    let command = read_until_eol(file);

    tree_add_command(c, &key, &name, &command);
}

pub fn command_add_child(c: &mut Command, child: Command) {
    let mut new_child = Box::new(child);

    // Empty children list
    if c.children.is_none() {
        c.children = Some(new_child);
        return;
    }

    // Insert at the head if the new child's key is smaller than the current head
    let head_key = c.children.as_ref().unwrap().key;
    if head_key > new_child.key {
        new_child.next = c.children.take();
        c.children = Some(new_child);
        return;
    }

    // Walk the list to find the correct insertion position (sorted ascending by key)
    let mut current: &mut Box<Command> = c.children.as_mut().unwrap();
    while current.next.is_some() && current.next.as_ref().unwrap().key <= new_child.key {
        current = current.next.as_mut().unwrap();
    }

    new_child.next = current.next.take();
    current.next = Some(new_child);
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
    // Default to 80 cols when terminal width cannot be detected
    let width: usize = 80;

    let mut lines: i32 = 0;

    if !c.name.is_empty() {
        eprintln!("{}{}:{}", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    // Find longest item
    let mut max_line_width: usize = 0;
    let mut child = c.children.as_deref();
    while let Some(curr) = child {
        let line_width = curr.name.chars().count();
        if line_width > max_line_width {
            max_line_width = line_width;
        }
        child = curr.next.as_deref();
    }

    max_line_width += RIGHT_MARGIN;
    if max_line_width > width {
        max_line_width = width;
    }

    let items_per_row = width / (max_line_width + 5);
    let items_per_row = if items_per_row == 0 { 1 } else { items_per_row };

    let mut child = c.children.as_deref();
    let mut current_item: usize = 0;
    while let Some(curr) = child {
        current_item += 1;

        if curr.children.is_some() {
            eprint!(
                "{}{}{} {}➔{} {}+{:<width$}{}",
                YELLOW,
                curr.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                BLUE,
                curr.name,
                COLOR_OFF,
                width = max_line_width
            );
        } else {
            eprint!(
                "{}{}{} {}➔{}  {:<width$}",
                YELLOW,
                curr.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                curr.name,
                width = max_line_width
            );
        }

        if current_item % items_per_row == 0 {
            eprintln!();
            lines += 1;
        }

        child = curr.next.as_deref();
    }

    eprintln!();
    lines += 1;

    lines
}

pub fn clear_lines(count: i32) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.flush();

    let stderr = std::io::stderr();
    let mut err = stderr.lock();
    for _ in 0..count {
        let _ = write!(err, "\x1b[A\r\x1b[2K");
    }
    let _ = err.flush();
}

pub fn find_command(c: &Command, key: char) -> Option<&Command> {
    let mut child = c.children.as_deref();
    while let Some(curr) = child {
        if curr.key == key {
            return Some(curr);
        }
        child = curr.next.as_deref();
    }
    None
}

pub fn read_until_eol(file: &mut &[u8]) -> String {
    let mut i = 0;
    while i < file.len() && file[i] != b'\n' {
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
        return c.command.len() as i32;
    }
    0
}
