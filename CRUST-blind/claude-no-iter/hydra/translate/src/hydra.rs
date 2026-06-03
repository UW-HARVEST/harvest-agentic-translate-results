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
    fn walk(opt: &mut Option<Box<Command>>, key: char) -> Option<&mut Command> {
        match opt {
            None => None,
            Some(boxed) => {
                if boxed.key == key {
                    Some(boxed.as_mut())
                } else {
                    walk(&mut boxed.next, key)
                }
            }
        }
    }
    walk(&mut c.children, key)
}

fn get_terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80)
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
    let mut current: &Command = c;
    while current.children.is_some() {
        let last_printed = print_command(current);
        let key = getch();
        let next = find_command(current, key);
        clear_lines(last_printed);
        match next {
            Some(n) => {
                if command_run(n) > 0 {
                    return;
                }
                current = n;
            }
            None => return,
        }
    }
}

pub fn tree_add_command(tree: &mut Command, keys: &str, name: &str, command: &str) {
    if keys.is_empty() {
        return;
    }

    let key = keys.chars().next().unwrap();
    let rest = &keys[key.len_utf8()..];

    if rest.is_empty() {
        // Last key in chain - either create or update
        match find_command_mut(tree, key) {
            Some(c) => {
                c.name = name.to_string();
                c.command = command.to_string();
            }
            None => {
                command_add_child(
                    tree,
                    Command::new(key, name.to_string(), command.to_string()),
                );
            }
        }
        return;
    }

    // Need to recurse - ensure intermediate node exists
    if find_command_mut(tree, key).is_none() {
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

    let mut current = c.children.as_mut().unwrap();
    while current.next.is_some() && current.next.as_ref().unwrap().key <= child.key {
        current = current.next.as_mut().unwrap();
    }

    child.next = current.next.take();
    current.next = Some(child);
}

pub fn getch() -> char {
    use std::io::Read;
    let mut buf = [0u8; 1];
    let _ = std::io::stdin().read(&mut buf);
    buf[0] as char
}

pub fn print_command(c: &Command) -> i32 {
    let width: usize = get_terminal_width();

    let mut lines: i32 = 0;

    if !c.name.is_empty() {
        eprintln!("{}{}:{}", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    // Find longest item
    let mut max_line_width: usize = 0;
    let mut child_opt = c.children.as_deref();
    while let Some(child) = child_opt {
        let lw = child.name.len();
        if lw > max_line_width {
            max_line_width = lw;
        }
        child_opt = child.next.as_deref();
    }

    max_line_width += RIGHT_MARGIN;
    if max_line_width > width {
        max_line_width = width;
    }

    let items_per_row = (width / (max_line_width + 5)).max(1);

    let mut current_item: usize = 0;
    let mut child_opt = c.children.as_deref();
    while let Some(child) = child_opt {
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

        child_opt = child.next.as_deref();
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
    let mut node = c.children.as_deref();
    while let Some(cmd) = node {
        if cmd.key == key {
            return Some(cmd);
        }
        node = cmd.next.as_deref();
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
