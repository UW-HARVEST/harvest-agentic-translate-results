// Translated from main.c
mod dag_lib;

use std::io::{self, Read, Write};

use dag_lib::{
    add_edge, add_node, create_graph, delete_node, find_shortest_path, free_graph,
    get_node_by_name, print_graph, print_node, shallow_copy, Graph,
};

const MAX_INPUT: usize = 256;

fn print_menu() {
    print!("\n=== DAG City Route Manager ===\n");
    print!("1. Add city (node)\n");
    print!("2. Add route (edge)\n");
    print!("3. Show all cities\n");
    print!("4. Show city details\n");
    print!("5. Find shortest path\n");
    print!("6. Make shallow copy of subsection\n");
    print!("7. Delete node\n");
    print!("8. Exit\n");
    print!("Choice: ");
    io::stdout().flush().ok();
}

/// Stdin reader that mimics C's fgets behavior. fgets reads up to n-1 bytes,
/// stops at newline (and includes the newline), or EOF, then stores '\0'.
struct StdinReader {
    stdin: io::Stdin,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl StdinReader {
    fn new() -> Self {
        StdinReader {
            stdin: io::stdin(),
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    /// Read one byte from stdin. Returns Some(byte) or None on EOF.
    fn read_byte(&mut self) -> Option<u8> {
        if self.eof {
            return None;
        }
        if self.pos >= self.buf.len() {
            self.buf.clear();
            self.pos = 0;
            let mut chunk = [0u8; 4096];
            match self.stdin.read(&mut chunk) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.buf.extend_from_slice(&chunk[..n]);
                }
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Some(b)
    }

    /// fgets-equivalent: reads up to max_len-1 bytes or until newline (inclusive)
    /// or EOF. Returns Some(Vec<u8>) on success, None on EOF with no bytes read.
    /// The returned Vec does NOT contain a trailing null byte (so it represents
    /// the C string content); but it DOES contain the trailing '\n' if read.
    fn fgets(&mut self, max_len: usize) -> Option<Vec<u8>> {
        if max_len <= 1 {
            return Some(Vec::new());
        }
        let cap = max_len - 1;
        let mut out: Vec<u8> = Vec::new();
        let mut got_any = false;

        while out.len() < cap {
            match self.read_byte() {
                Some(b) => {
                    got_any = true;
                    out.push(b);
                    if b == b'\n' {
                        break;
                    }
                }
                None => {
                    // EOF
                    if !got_any {
                        return None;
                    }
                    break;
                }
            }
        }
        Some(out)
    }
}

/// Parse leading signed integer from a byte slice, skipping leading whitespace.
/// Returns Some(value) if at least one digit (after optional sign) was consumed.
/// Mimics `sscanf(s, "%d", &x)`.
fn parse_int_scanf(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    while i < s.len()
        && (s[i] == b' '
            || s[i] == b'\t'
            || s[i] == b'\n'
            || s[i] == b'\r'
            || s[i] == 0x0B
            || s[i] == 0x0C)
    {
        i += 1;
    }

    let mut sign: i64 = 1;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        if s[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }

    let start = i;
    let mut value: i64 = 0;
    let mut has_digit = false;
    while i < s.len() && (b'0'..=b'9').contains(&s[i]) {
        has_digit = true;
        // Saturate-like accumulation; C uses overflow behavior we don't try
        // to replicate beyond i32 range.
        value = value.saturating_mul(10).saturating_add((s[i] - b'0') as i64);
        i += 1;
    }
    if !has_digit && start == i {
        return None;
    }
    if !has_digit {
        return None;
    }

    let signed = sign.saturating_mul(value);
    // Clamp to i32 like C int.
    let clamped = if signed > i32::MAX as i64 {
        i32::MAX
    } else if signed < i32::MIN as i64 {
        i32::MIN
    } else {
        signed as i32
    };
    Some(clamped)
}

/// Strip first '\n' (and everything after if any) — equivalent to:
///   input[strcspn(input, "\n")] = 0;
/// in C. This finds the first newline and "terminates" there.
fn strip_newline(buf: &[u8]) -> Vec<u8> {
    if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        buf[..pos].to_vec()
    } else {
        buf.to_vec()
    }
}

fn main() {
    let mut graph: Graph = create_graph();
    let mut reader = StdinReader::new();

    print!("City Route Management System\n");
    print!("Commands are read from stdin\n");
    io::stdout().flush().ok();

    loop {
        print_menu();

        let line = match reader.fgets(MAX_INPUT) {
            Some(l) => l,
            None => break,
        };

        let choice = match parse_int_scanf(&line) {
            Some(c) => c,
            None => {
                print!("Invalid input\n");
                io::stdout().flush().ok();
                continue;
            }
        };

        match choice {
            1 => {
                // Add city
                print!("Enter city name: ");
                io::stdout().flush().ok();
                let buf = match reader.fgets(MAX_INPUT) {
                    Some(b) => b,
                    None => break,
                };
                let name = strip_newline(&buf);

                let node = add_node(&mut graph, &name);
                if node.is_some() {
                    let mut stdout = io::stdout();
                    stdout.write_all(b"Added city: ").ok();
                    stdout.write_all(&name).ok();
                    stdout.write_all(b"\n").ok();
                    stdout.flush().ok();
                } else {
                    print!("Failed to add city\n");
                    io::stdout().flush().ok();
                }
            }
            2 => {
                // Add route
                print!("Enter from city: ");
                io::stdout().flush().ok();
                let from_buf = match reader.fgets(MAX_INPUT) {
                    Some(b) => b,
                    None => break,
                };
                let from_city = strip_newline(&from_buf);

                print!("Enter to city: ");
                io::stdout().flush().ok();
                let to_buf = match reader.fgets(MAX_INPUT) {
                    Some(b) => b,
                    None => break,
                };
                let to_city = strip_newline(&to_buf);

                print!("Enter distance: ");
                io::stdout().flush().ok();
                let dist_buf = match reader.fgets(MAX_INPUT) {
                    Some(b) => b,
                    None => break,
                };
                let distance = match parse_int_scanf(&dist_buf) {
                    Some(d) => d,
                    None => {
                        print!("Invalid distance\n");
                        io::stdout().flush().ok();
                        continue;
                    }
                };

                let from = get_node_by_name(&graph, &from_city);
                let to = get_node_by_name(&graph, &to_city);

                if from.is_none() {
                    let mut stdout = io::stdout();
                    stdout.write_all(b"City '").ok();
                    stdout.write_all(&from_city).ok();
                    stdout.write_all(b"' not found\n").ok();
                    stdout.flush().ok();
                    continue;
                }
                if to.is_none() {
                    let mut stdout = io::stdout();
                    stdout.write_all(b"City '").ok();
                    stdout.write_all(&to_city).ok();
                    stdout.write_all(b"' not found\n").ok();
                    stdout.flush().ok();
                    continue;
                }

                let from = from.unwrap();
                let to = to.unwrap();

                if add_edge(&from, &to, distance) == 0 {
                    let mut stdout = io::stdout();
                    stdout.write_all(b"Added route: ").ok();
                    stdout.write_all(&from_city).ok();
                    stdout.write_all(b" -> ").ok();
                    stdout.write_all(&to_city).ok();
                    write!(stdout, " (distance: {})\n", distance).ok();
                    stdout.flush().ok();
                } else {
                    print!("Failed to add route\n");
                    io::stdout().flush().ok();
                }
            }
            3 => {
                // Show all cities
                print_graph(&graph);
                io::stdout().flush().ok();
            }
            4 => {
                // Show city details
                print!("Enter city name: ");
                io::stdout().flush().ok();
                let buf = match reader.fgets(MAX_INPUT) {
                    Some(b) => b,
                    None => break,
                };
                let name = strip_newline(&buf);

                match get_node_by_name(&graph, &name) {
                    Some(node) => {
                        print_node(&node);
                        io::stdout().flush().ok();
                    }
                    None => {
                        let mut stdout = io::stdout();
                        stdout.write_all(b"City '").ok();
                        stdout.write_all(&name).ok();
                        stdout.write_all(b"' not found\n").ok();
                        stdout.flush().ok();
                    }
                }
            }
            5 => {
                // Find shortest path
                print!("Enter start city: ");
                io::stdout().flush().ok();
                let start_buf = match reader.fgets(MAX_INPUT) {
                    Some(b) => b,
                    None => break,
                };
                let start_city = strip_newline(&start_buf);

                print!("Enter end city: ");
                io::stdout().flush().ok();
                let end_buf = match reader.fgets(MAX_INPUT) {
                    Some(b) => b,
                    None => break,
                };
                let end_city = strip_newline(&end_buf);

                let start = get_node_by_name(&graph, &start_city);
                let end = get_node_by_name(&graph, &end_city);

                if start.is_none() {
                    let mut stdout = io::stdout();
                    stdout.write_all(b"City '").ok();
                    stdout.write_all(&start_city).ok();
                    stdout.write_all(b"' not found\n").ok();
                    stdout.flush().ok();
                    continue;
                }
                if end.is_none() {
                    let mut stdout = io::stdout();
                    stdout.write_all(b"City '").ok();
                    stdout.write_all(&end_city).ok();
                    stdout.write_all(b"' not found\n").ok();
                    stdout.flush().ok();
                    continue;
                }

                let start = start.unwrap();
                let end = end.unwrap();

                match find_shortest_path(&start, &end) {
                    Some(path) => {
                        let mut stdout = io::stdout();
                        stdout.write_all(b"Shortest path from ").ok();
                        stdout.write_all(&start_city).ok();
                        stdout.write_all(b" to ").ok();
                        stdout.write_all(&end_city).ok();
                        stdout.write_all(b":\n").ok();
                        for (i, n) in path.iter().enumerate() {
                            write!(stdout, "  {}. ", i + 1).ok();
                            stdout.write_all(&n.borrow().city_name).ok();
                            stdout.write_all(b"\n").ok();
                        }
                        stdout.flush().ok();
                    }
                    None => {
                        print!("No path found\n");
                        io::stdout().flush().ok();
                    }
                }
            }
            6 => {
                // Make shallow copy
                print!("Enter start city for shallow copy: ");
                io::stdout().flush().ok();
                let buf = match reader.fgets(MAX_INPUT) {
                    Some(b) => b,
                    None => break,
                };
                let name = strip_newline(&buf);

                let node = get_node_by_name(&graph, &name);
                if node.is_none() {
                    let mut stdout = io::stdout();
                    stdout.write_all(b"City '").ok();
                    stdout.write_all(&name).ok();
                    stdout.write_all(b"' not found\n").ok();
                    stdout.flush().ok();
                    continue;
                }

                let node = node.unwrap();
                match shallow_copy(&node) {
                    Some(copy) => {
                        let mut stdout = io::stdout();
                        stdout.write_all(b"Created shallow copy starting from ").ok();
                        stdout.write_all(&name).ok();
                        stdout.write_all(b"\n").ok();
                        stdout
                            .write_all(b"Reference counts incremented for all reachable nodes\n")
                            .ok();
                        stdout.flush().ok();
                        print_node(&copy);
                        io::stdout().flush().ok();
                    }
                    None => {
                        print!("Failed to create shallow copy\n");
                        io::stdout().flush().ok();
                    }
                }
            }
            7 => {
                // Delete node
                print!("Enter city name to delete: ");
                io::stdout().flush().ok();
                let buf = match reader.fgets(MAX_INPUT) {
                    Some(b) => b,
                    None => break,
                };
                let name = strip_newline(&buf);

                let node = get_node_by_name(&graph, &name);
                if node.is_none() {
                    let mut stdout = io::stdout();
                    stdout.write_all(b"City '").ok();
                    stdout.write_all(&name).ok();
                    stdout.write_all(b"' not found\n").ok();
                    stdout.flush().ok();
                    continue;
                }

                let node = node.unwrap();
                let rc = node.borrow().ref_count;
                print!("Current ref count: {}\n", rc);
                io::stdout().flush().ok();

                delete_node(&node);

                {
                    let mut stdout = io::stdout();
                    stdout.write_all(b"Decremented reference count for ").ok();
                    stdout.write_all(&name).ok();
                    stdout.write_all(b"\n").ok();
                    stdout
                        .write_all(b"Note: Node will be freed when ref count reaches 0\n")
                        .ok();
                    stdout.flush().ok();
                }
            }
            8 => {
                // Exit
                print!("Freeing graph and exiting...\n");
                io::stdout().flush().ok();
                free_graph(&mut graph);
                return;
            }
            _ => {
                print!("Invalid choice\n");
                io::stdout().flush().ok();
            }
        }
    }

    free_graph(&mut graph);
}
