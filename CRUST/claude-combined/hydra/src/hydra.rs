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
    let mut i = 0;
    while i < file.len() && file[i] != b',' && file[i] != b'\n' && file[i] != 0 {
        i += 1;
    }

    if i >= file.len() || file[i] != b',' {
        let found = if i < file.len() {
            file[i] as char
        } else {
            '\0'
        };
        eprint!("Found incorrect end after {}, found: {}", field, found);
        std::process::exit(1);
    }

    let result = String::from_utf8_lossy(&file[..i]).into_owned();
    *file = &file[i + 1..];
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
    while let Some(cmd) = current {
        if cmd.children.is_none() {
            return;
        }
        let last_printed_lines = print_command(cmd);
        let key = getch();
        let next = find_command(cmd, key);
        clear_lines(last_printed_lines);
        match next {
            Some(next_cmd) => {
                if command_run(next_cmd) > 0 {
                    return;
                }
                current = Some(next_cmd);
            }
            None => return,
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

pub fn tree_add_command(tree: &mut Command, keys: &str, name: &str, command: &str) {
    let key = match keys.chars().next() {
        Some(c) => c,
        None => return,
    };
    let rest = &keys[key.len_utf8()..];

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

    if find_command(tree, key).is_none() {
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

fn insert_sorted(list: &mut Option<Box<Command>>, mut new_child: Box<Command>) {
    match list {
        None => {
            *list = Some(new_child);
        }
        Some(node) => {
            if node.key > new_child.key {
                new_child.next = list.take();
                *list = Some(new_child);
            } else {
                insert_sorted(&mut node.next, new_child);
            }
        }
    }
}

pub fn command_add_child(c: &mut Command, child: Command) {
    let new_child = Box::new(child);
    insert_sorted(&mut c.children, new_child);
}

pub fn getch() -> char {
    use std::io::Read;
    let mut buf = [0u8; 1];
    let _ = std::io::stdin().read(&mut buf);
    buf[0] as char
}

pub fn print_command(c: &Command) -> i32 {
    let width: usize = 80;
    let mut lines: i32 = 0;

    if !c.name.is_empty() {
        eprintln!("{}{}:{}", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

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
    let items_per_row = if denom == 0 { 0 } else { width / denom };

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

        if items_per_row > 0 && current_item % items_per_row == 0 {
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
    use std::io::Write;
    let _ = std::io::stdout().flush();
    for _ in 0..count {
        eprint!("\x1b[A\r\x1b[2K");
    }
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
        Err(_) => {
            eprintln!("Failed to open file");
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
