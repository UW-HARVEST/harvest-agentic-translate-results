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
    fn walk(node: Option<&mut Command>, key: char) -> Option<&mut Command> {
        match node {
            None => None,
            Some(n) if n.key == key => Some(n),
            Some(n) => walk(n.next.as_deref_mut(), key),
        }
    }
    walk(c.children.as_deref_mut(), key)
}

fn insert_in_sorted_list(node: &mut Command, mut child: Box<Command>) {
    match node.next.as_ref() {
        Some(next) if next.key <= child.key => {
            insert_in_sorted_list(node.next.as_mut().unwrap(), child);
        }
        _ => {
            child.next = node.next.take();
            node.next = Some(child);
        }
    }
}

pub fn read_field(file: &mut &[u8], field: &str) -> String {
    let mut i = 0;
    while i < file.len() && file[i] != b',' && file[i] != b'\n' && file[i] != 0 {
        i += 1;
    }
    if i >= file.len() || file[i] != b',' {
        let c = if i < file.len() {
            file[i] as char
        } else {
            '\0'
        };
        eprint!("Found incorrect end after {}, found: {}", field, c);
        std::process::exit(1);
    }
    let result = String::from_utf8_lossy(&file[..i]).to_string();
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
    loop {
        match current {
            None => break,
            Some(cmd) if cmd.children.is_none() => break,
            Some(cmd) => {
                let lines = print_command(cmd);
                let key = getch();
                let next = find_command(cmd, key);
                clear_lines(lines);
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
    let c = find_command_mut(tree, key).unwrap();
    tree_add_command(c, &rest, name, command);
}

pub fn read_line(c: &mut Command, file: &mut &[u8]) {
    let key = read_field(file, "key");
    let name = read_field(file, "name");
    let command = read_until_eol(file);
    tree_add_command(c, &key, &name, &command);
}

pub fn command_add_child(c: &mut Command, child: Command) {
    let mut child = Box::new(child);

    match c.children.as_ref() {
        None => {
            c.children = Some(child);
        }
        Some(first) if first.key > child.key => {
            child.next = c.children.take();
            c.children = Some(child);
        }
        Some(_) => {
            insert_in_sorted_list(c.children.as_mut().unwrap(), child);
        }
    }
}

pub fn getch() -> char {
    use std::io::Read;
    let mut buf = [0u8; 1];
    match std::io::stdin().read(&mut buf) {
        Ok(n) if n > 0 => buf[0] as char,
        _ => '\0',
    }
}

pub fn print_command(c: &Command) -> i32 {
    let width: usize = std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|w: &usize| *w > 0)
        .unwrap_or(80);

    let mut lines: i32 = 0;

    if !c.name.is_empty() {
        eprintln!("{}{}:{}", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    // Find longest item
    let mut max_line_width: usize = 0;
    let mut child = c.children.as_deref();
    while let Some(cmd) = child {
        let line_width = cmd.name.chars().count();
        if line_width > max_line_width {
            max_line_width = line_width;
        }
        child = cmd.next.as_deref();
    }

    max_line_width += RIGHT_MARGIN;
    if max_line_width > width {
        max_line_width = width;
    }

    let denom = max_line_width + 5;
    let items_per_row = if denom == 0 { 0 } else { width / denom };

    let mut child = c.children.as_deref();
    let mut current_item: usize = 0;
    while let Some(cmd) = child {
        current_item += 1;

        if cmd.children.is_some() {
            eprint!(
                "{}{}{} {}\u{2794}{} {}+{:<width$}{}",
                YELLOW,
                cmd.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                BLUE,
                cmd.name,
                COLOR_OFF,
                width = max_line_width
            );
        } else {
            eprint!(
                "{}{}{} {}\u{2794}{}  {:<width$}",
                YELLOW,
                cmd.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                cmd.name,
                width = max_line_width
            );
        }

        if items_per_row > 0 && current_item % items_per_row == 0 {
            eprintln!();
            lines += 1;
        }

        child = cmd.next.as_deref();
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
    let result = String::from_utf8_lossy(&file[..i]).to_string();
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
        use std::io::Write;
        print!("{}", c.command);
        let _ = std::io::stdout().flush();
        c.command.len() as i32
    } else {
        0
    }
}
