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
        let found = if i < file.len() { file[i] as char } else { '\0' };
        eprint!("Found incorrect end after {}, found: {}", field, found);
        std::process::exit(1);
    }

    let result = std::str::from_utf8(&file[..i]).unwrap_or("").to_string();
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
        // Look for existing child
        {
            let mut current = tree.children.as_deref_mut();
            while let Some(c) = current {
                if c.key == key {
                    c.name = name.to_string();
                    c.command = command.to_string();
                    return;
                }
                current = c.next.as_deref_mut();
            }
        }
        command_add_child(
            tree,
            Command::new(key, name.to_string(), command.to_string()),
        );
        return;
    }

    // Check if a child with this key exists
    let exists = {
        let mut current = tree.children.as_deref();
        let mut found = false;
        while let Some(c) = current {
            if c.key == key {
                found = true;
                break;
            }
            current = c.next.as_deref();
        }
        found
    };

    if !exists {
        command_add_child(
            tree,
            Command::new(key, DEFAULT_NAME.to_string(), String::new()),
        );
    }

    // Find the matching child mutably and recurse
    let mut current = tree.children.as_deref_mut();
    while let Some(c) = current {
        if c.key == key {
            tree_add_command(c, &rest, name, command);
            return;
        }
        current = c.next.as_deref_mut();
    }
}
pub fn read_line(c: &mut Command, file: &mut &[u8]) {
    let key = read_field(file, "key");
    let name = read_field(file, "name");
    let command = read_until_eol(file);
    tree_add_command(c, &key, &name, &command);
}
pub fn command_add_child(c: &mut Command, child: Command) {
    let mut child_box = Box::new(child);

    if c.children.is_none() {
        c.children = Some(child_box);
        return;
    }

    if c.children.as_ref().unwrap().key > child_box.key {
        child_box.next = c.children.take();
        c.children = Some(child_box);
        return;
    }

    let mut last_child = c.children.as_mut().unwrap();
    while last_child.next.is_some()
        && last_child.next.as_ref().unwrap().key <= child_box.key
    {
        last_child = last_child.next.as_mut().unwrap();
    }

    child_box.next = last_child.next.take();
    last_child.next = Some(child_box);
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
    // Default to a typical terminal width.
    let width: usize = 80;

    let mut lines: i32 = 0;

    if !c.name.is_empty() {
        eprintln!("{}{}:{}", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    // Find the longest item name
    let mut max_line_width: usize = 0;
    let mut child = c.children.as_deref();
    while let Some(cc) = child {
        let lw = cc.name.len();
        if lw > max_line_width {
            max_line_width = lw;
        }
        child = cc.next.as_deref();
    }

    max_line_width += RIGHT_MARGIN;
    if max_line_width > width {
        max_line_width = width;
    }

    let mut items_per_row = width / (max_line_width + 5);
    if items_per_row == 0 {
        items_per_row = 1;
    }

    let mut child = c.children.as_deref();
    let mut current_item: usize = 0;
    while let Some(cc) = child {
        current_item += 1;

        if cc.children.is_some() {
            eprint!(
                "{}{}{} {}\u{2794}{} {}+{:<w$}{}",
                YELLOW,
                cc.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                BLUE,
                cc.name,
                COLOR_OFF,
                w = max_line_width
            );
        } else {
            eprint!(
                "{}{}{} {}\u{2794}{}  {:<w$}",
                YELLOW,
                cc.key,
                COLOR_OFF,
                PURPLE,
                COLOR_OFF,
                cc.name,
                w = max_line_width
            );
        }

        if current_item % items_per_row == 0 {
            eprintln!();
            lines += 1;
        }

        child = cc.next.as_deref();
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
    while let Some(cc) = child {
        if cc.key == key {
            return Some(cc);
        }
        child = cc.next.as_deref();
    }
    None
}
pub fn read_until_eol(file: &mut &[u8]) -> String {
    let mut i = 0;
    while i < file.len() && file[i] != b'\n' && file[i] != 0 {
        i += 1;
    }

    let result = std::str::from_utf8(&file[..i]).unwrap_or("").to_string();

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
        return c.command.len() as i32;
    }
    0
}
