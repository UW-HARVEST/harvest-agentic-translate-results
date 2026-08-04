// Translated from main.c - matches C output byte-for-byte.
//
// Uses the C-ABI driver library from src/lib.rs so that the binary's behavior
// is identical to the C implementation (printing via C's printf, etc.).

use std::io::{self, Read};
use std::os::raw::c_char;

use driver::{
    add_edge, add_node, create_graph, delete_node, find_shortest_path, free_graph,
    get_node_by_name, graph_t, node_t, print_graph, print_node, shallow_copy,
};

const MAX_INPUT: usize = 256;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> i32;
    fn free(ptr: *mut u8);
    fn fflush(stream: *mut u8) -> i32;
    static stdout: *mut u8;
}

fn print_menu() {
    unsafe {
        printf(b"\n=== DAG City Route Manager ===\n\0".as_ptr() as *const c_char);
        printf(b"1. Add city (node)\n\0".as_ptr() as *const c_char);
        printf(b"2. Add route (edge)\n\0".as_ptr() as *const c_char);
        printf(b"3. Show all cities\n\0".as_ptr() as *const c_char);
        printf(b"4. Show city details\n\0".as_ptr() as *const c_char);
        printf(b"5. Find shortest path\n\0".as_ptr() as *const c_char);
        printf(b"6. Make shallow copy of subsection\n\0".as_ptr() as *const c_char);
        printf(b"7. Delete node\n\0".as_ptr() as *const c_char);
        printf(b"8. Exit\n\0".as_ptr() as *const c_char);
        printf(b"Choice: \0".as_ptr() as *const c_char);
        fflush(stdout);
    }
}

/// Stdin reader matching C's fgets semantics (reads up to n-1 bytes, including
/// trailing newline; '\0'-terminates the buffer).
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
                Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
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

    /// Read up to `max_len-1` bytes or to a newline, then NUL-terminate.
    /// Returns Some(buf with trailing NUL) or None on EOF with no data.
    fn fgets(&mut self, max_len: usize) -> Option<Vec<u8>> {
        if max_len == 0 {
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
                    if !got_any {
                        return None;
                    }
                    break;
                }
            }
        }
        out.push(0);
        Some(out)
    }
}

/// Parse a leading decimal integer, returns None if no digit.
fn parse_int(s: &[u8]) -> Option<i32> {
    let mut i = 0usize;
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t' || s[i] == b'\n' || s[i] == b'\r') {
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
    while i < s.len() && (b'0'..=b'9').contains(&s[i]) {
        value = value.saturating_mul(10).saturating_add((s[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        return None;
    }
    let signed = sign.saturating_mul(value);
    Some(signed.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

/// Replace the first '\n' in a NUL-terminated buffer with '\0'.
fn strip_newline(buf: &mut [u8]) {
    if let Some(pos) = buf.iter().position(|&b| b == b'\n' || b == 0) {
        buf[pos] = 0;
        for b in buf.iter_mut().skip(pos + 1) {
            *b = 0;
        }
    }
}

fn flush_stdout() {
    unsafe {
        fflush(stdout);
    }
}

fn main() {
    unsafe {
        let graph: *mut graph_t = create_graph();
        if graph.is_null() {
            eprintln!("Failed to create graph");
            std::process::exit(1);
        }
        let mut reader = StdinReader::new();

        printf(b"City Route Management System\n\0".as_ptr() as *const c_char);
        printf(b"Commands are read from stdin\n\0".as_ptr() as *const c_char);

        loop {
            print_menu();

            let mut input = match reader.fgets(MAX_INPUT) {
                Some(b) => b,
                None => break,
            };

            let choice = match parse_int(&input) {
                Some(c) => c,
                None => {
                    printf(b"Invalid input\n\0".as_ptr() as *const c_char);
                    flush_stdout();
                    continue;
                }
            };

            match choice {
                1 => {
                    printf(b"Enter city name: \0".as_ptr() as *const c_char);
                    flush_stdout();
                    let mut buf = match reader.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break,
                    };
                    strip_newline(&mut buf);

                    let node = add_node(graph, buf.as_ptr() as *const c_char);
                    if !node.is_null() {
                        printf(
                            b"Added city: %s\n\0".as_ptr() as *const c_char,
                            buf.as_ptr() as *const c_char,
                        );
                    } else {
                        printf(b"Failed to add city\n\0".as_ptr() as *const c_char);
                    }
                    let _ = input;
                }
                2 => {
                    printf(b"Enter from city: \0".as_ptr() as *const c_char);
                    flush_stdout();
                    let mut from_city = match reader.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break,
                    };
                    strip_newline(&mut from_city);

                    printf(b"Enter to city: \0".as_ptr() as *const c_char);
                    flush_stdout();
                    let mut to_city = match reader.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break,
                    };
                    strip_newline(&mut to_city);

                    printf(b"Enter distance: \0".as_ptr() as *const c_char);
                    flush_stdout();
                    input = match reader.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break,
                    };
                    let distance = match parse_int(&input) {
                        Some(d) => d,
                        None => {
                            printf(b"Invalid distance\n\0".as_ptr() as *const c_char);
                            flush_stdout();
                            continue;
                        }
                    };

                    let from = get_node_by_name(graph, from_city.as_ptr() as *const c_char);
                    let to = get_node_by_name(graph, to_city.as_ptr() as *const c_char);

                    if from.is_null() {
                        printf(
                            b"City '%s' not found\n\0".as_ptr() as *const c_char,
                            from_city.as_ptr() as *const c_char,
                        );
                        flush_stdout();
                        continue;
                    }
                    if to.is_null() {
                        printf(
                            b"City '%s' not found\n\0".as_ptr() as *const c_char,
                            to_city.as_ptr() as *const c_char,
                        );
                        flush_stdout();
                        continue;
                    }

                    if add_edge(from, to, distance) == 0 {
                        printf(
                            b"Added route: %s -> %s (distance: %d)\n\0".as_ptr() as *const c_char,
                            from_city.as_ptr() as *const c_char,
                            to_city.as_ptr() as *const c_char,
                            distance,
                        );
                    } else {
                        printf(b"Failed to add route\n\0".as_ptr() as *const c_char);
                    }
                }
                3 => {
                    print_graph(graph);
                    flush_stdout();
                }
                4 => {
                    printf(b"Enter city name: \0".as_ptr() as *const c_char);
                    flush_stdout();
                    let mut buf = match reader.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break,
                    };
                    strip_newline(&mut buf);
                    let node = get_node_by_name(graph, buf.as_ptr() as *const c_char);
                    if !node.is_null() {
                        print_node(node);
                    } else {
                        printf(
                            b"City '%s' not found\n\0".as_ptr() as *const c_char,
                            buf.as_ptr() as *const c_char,
                        );
                    }
                    flush_stdout();
                }
                5 => {
                    printf(b"Enter start city: \0".as_ptr() as *const c_char);
                    flush_stdout();
                    let mut start_city = match reader.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break,
                    };
                    strip_newline(&mut start_city);

                    printf(b"Enter end city: \0".as_ptr() as *const c_char);
                    flush_stdout();
                    let mut end_city = match reader.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break,
                    };
                    strip_newline(&mut end_city);

                    let start = get_node_by_name(graph, start_city.as_ptr() as *const c_char);
                    let end = get_node_by_name(graph, end_city.as_ptr() as *const c_char);

                    if start.is_null() {
                        printf(
                            b"City '%s' not found\n\0".as_ptr() as *const c_char,
                            start_city.as_ptr() as *const c_char,
                        );
                        flush_stdout();
                        continue;
                    }
                    if end.is_null() {
                        printf(
                            b"City '%s' not found\n\0".as_ptr() as *const c_char,
                            end_city.as_ptr() as *const c_char,
                        );
                        flush_stdout();
                        continue;
                    }

                    let mut path_length: i32 = 0;
                    let path: *mut *mut node_t = find_shortest_path(start, end, &mut path_length);
                    if !path.is_null() {
                        printf(
                            b"Shortest path from %s to %s:\n\0".as_ptr() as *const c_char,
                            start_city.as_ptr() as *const c_char,
                            end_city.as_ptr() as *const c_char,
                        );
                        for i in 0..path_length {
                            let n = *path.offset(i as isize);
                            printf(
                                b"  %d. %s\n\0".as_ptr() as *const c_char,
                                i + 1,
                                (*n).city_name.as_ptr(),
                            );
                        }
                        free(path as *mut u8);
                    } else {
                        printf(b"No path found\n\0".as_ptr() as *const c_char);
                    }
                    flush_stdout();
                }
                6 => {
                    printf(
                        b"Enter start city for shallow copy: \0".as_ptr() as *const c_char,
                    );
                    flush_stdout();
                    let mut buf = match reader.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break,
                    };
                    strip_newline(&mut buf);

                    let node = get_node_by_name(graph, buf.as_ptr() as *const c_char);
                    if node.is_null() {
                        printf(
                            b"City '%s' not found\n\0".as_ptr() as *const c_char,
                            buf.as_ptr() as *const c_char,
                        );
                        flush_stdout();
                        continue;
                    }
                    let copy = shallow_copy(node);
                    if !copy.is_null() {
                        printf(
                            b"Created shallow copy starting from %s\n\0".as_ptr() as *const c_char,
                            buf.as_ptr() as *const c_char,
                        );
                        printf(
                            b"Reference counts incremented for all reachable nodes\n\0".as_ptr()
                                as *const c_char,
                        );
                        print_node(copy);
                    } else {
                        printf(b"Failed to create shallow copy\n\0".as_ptr() as *const c_char);
                    }
                    flush_stdout();
                }
                7 => {
                    printf(b"Enter city name to delete: \0".as_ptr() as *const c_char);
                    flush_stdout();
                    let mut buf = match reader.fgets(MAX_INPUT) {
                        Some(b) => b,
                        None => break,
                    };
                    strip_newline(&mut buf);

                    let node = get_node_by_name(graph, buf.as_ptr() as *const c_char);
                    if node.is_null() {
                        printf(
                            b"City '%s' not found\n\0".as_ptr() as *const c_char,
                            buf.as_ptr() as *const c_char,
                        );
                        flush_stdout();
                        continue;
                    }
                    printf(
                        b"Current ref count: %d\n\0".as_ptr() as *const c_char,
                        (*node).ref_count,
                    );
                    delete_node(node);
                    printf(
                        b"Decremented reference count for %s\n\0".as_ptr() as *const c_char,
                        buf.as_ptr() as *const c_char,
                    );
                    printf(
                        b"Note: Node will be freed when ref count reaches 0\n\0".as_ptr()
                            as *const c_char,
                    );
                    flush_stdout();
                }
                8 => {
                    printf(b"Freeing graph and exiting...\n\0".as_ptr() as *const c_char);
                    flush_stdout();
                    free_graph(graph);
                    return;
                }
                _ => {
                    printf(b"Invalid choice\n\0".as_ptr() as *const c_char);
                    flush_stdout();
                }
            }
        }

        free_graph(graph);
    }
}
