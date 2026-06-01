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
        let found = if i >= file.len() {
            '\0'
        } else {
            file[i] as char
        };
        eprint!("Found incorrect end after {}, found: {}", field, found);
        std::process::exit(1);
    }

    let key = String::from_utf8_lossy(&file[..i]).into_owned();
    *file = &file[i + 1..];
    key
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
            break;
        }
        let last_printed_lines = print_command(cmd);
        let key = getch();
        let next = find_command(cmd, key);
        clear_lines(last_printed_lines);
        if let Some(nc) = next {
            if command_run(nc) > 0 {
                return;
            }
        }
        current = next;
    }
}
pub fn tree_add_command(tree: &mut Command, keys: &str, name: &str, command: &str) {
    let mut iter = keys.chars();
    let first = match iter.next() {
        Some(c) => c,
        None => return,
    };
    let rest: String = iter.collect();

    if rest.is_empty() {
        // Leaf insertion: update existing or create new
        let mut found = false;
        {
            let mut child = tree.children.as_deref_mut();
            while let Some(cmd) = child {
                if cmd.key == first {
                    cmd.name = name.to_string();
                    cmd.command = command.to_string();
                    found = true;
                    break;
                }
                child = cmd.next.as_deref_mut();
            }
        }
        if !found {
            command_add_child(tree, Command::new(first, name.to_string(), command.to_string()));
        }
        return;
    }

    // Need an intermediate node with this key
    let exists = find_command(tree, first).is_some();
    if !exists {
        command_add_child(
            tree,
            Command::new(first, DEFAULT_NAME.to_string(), String::new()),
        );
    }

    // Find the child with key `first` and recurse
    let mut child = tree.children.as_deref_mut();
    while let Some(cmd) = child {
        if cmd.key == first {
            tree_add_command(cmd, &rest, name, command);
            return;
        }
        child = cmd.next.as_deref_mut();
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

    // If parent has no children, set as first child
    if c.children.is_none() {
        c.children = Some(child);
        return;
    }

    // If first child has a larger key, prepend
    let first_key = c.children.as_ref().unwrap().key;
    if first_key > child.key {
        let old_children = c.children.take();
        child.next = old_children;
        c.children = Some(child);
        return;
    }

    // Walk the list to find the insertion point
    let mut cur: &mut Box<Command> = c.children.as_mut().unwrap();
    while cur.next.is_some() && cur.next.as_ref().unwrap().key <= child.key {
        cur = cur.next.as_mut().unwrap();
    }

    let next_node = cur.next.take();
    child.next = next_node;
    cur.next = Some(child);
}
pub fn getch() -> char {
    use std::io::Read;
    let mut buf = [0u8; 1];
    if std::io::stdin().read_exact(&mut buf).is_err() {
        return '\0';
    }
    buf[0] as char
}
pub fn print_command(c: &Command) -> i32 {
    use std::io::Write;
    let width: usize = std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80);

    let stderr = std::io::stderr();
    let mut stderr = stderr.lock();

    let mut lines: i32 = 0;

    if !c.name.is_empty() {
        let _ = write!(stderr, "{}{}:{}\n", BLUE, c.name, COLOR_OFF);
        lines += 1;
    }

    // Find longest item
    let mut max_line_width: usize = 0;
    {
        let mut child = c.children.as_deref();
        while let Some(cmd) = child {
            let line_width = cmd.name.len();
            if line_width > max_line_width {
                max_line_width = line_width;
            }
            child = cmd.next.as_deref();
        }
    }

    max_line_width += RIGHT_MARGIN;
    if max_line_width > width {
        max_line_width = width;
    }

    let items_per_row = if max_line_width + 5 == 0 {
        1
    } else {
        width / (max_line_width + 5)
    };
    let items_per_row = if items_per_row == 0 { 1 } else { items_per_row };

    let mut child = c.children.as_deref();
    let mut current_item = 0;
    while let Some(cmd) = child {
        current_item += 1;

        if cmd.children.is_some() {
            let _ = write!(
                stderr,
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
            let _ = write!(
                stderr,
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

        if current_item % items_per_row == 0 {
            let _ = writeln!(stderr);
            lines += 1;
        }

        child = cmd.next.as_deref();
    }

    let _ = writeln!(stderr);
    lines += 1;

    lines
}
pub fn clear_lines(count: i32) {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let stderr = std::io::stderr();
    let mut stderr = stderr.lock();
    for _ in 0..count {
        let _ = write!(stderr, "\x1b[A\r\x1b[2K");
    }
}
pub fn find_command(c: &Command, key: char) -> Option<&Command> {
    let mut child = c.children.as_deref();
    while let Some(cmd) = child {
        if cmd.key == key {
            return Some(cmd);
        }
        child = cmd.next.as_deref();
    }
    None
}
pub fn read_until_eol(file: &mut &[u8]) -> String {
    let mut i = 0;
    while i < file.len() && file[i] != b'\n' && file[i] != 0 {
        i += 1;
    }

    let s = String::from_utf8_lossy(&file[..i]).into_owned();

    if i < file.len() && file[i] == b'\n' {
        *file = &file[i + 1..];
    } else {
        *file = &file[i..];
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
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        let _ = write!(stdout, "{}", c.command);
        return c.command.len() as i32;
    }
    0
}
