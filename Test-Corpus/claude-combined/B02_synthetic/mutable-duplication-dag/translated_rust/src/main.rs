// Translation of c_src/src/main.c and c_src/src/lib.c to Rust.
// Uses raw pointers and libc malloc/free to reproduce the exact behavior
// of the original C program (including its undefined behavior like
// use-after-free, which depends on the system allocator's exact behavior).

use std::ffi::CStr;
use std::io::{self, BufRead, BufReader, Write};
use std::os::raw::{c_char, c_int};
use std::ptr;

const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;

#[repr(C)]
#[derive(Copy, Clone)]
struct Edge {
    destination: *mut Node,
    distance: c_int,
}

#[repr(C)]
struct Node {
    city_name: [c_char; MAX_CITY_NAME],
    ref_count: c_int,
    edges: [Edge; MAX_EDGES],
    edge_count: c_int,
}

#[repr(C)]
struct Graph {
    nodes: [*mut Node; MAX_NODES],
    node_count: c_int,
}

// ---------- Helpers ----------

unsafe fn c_str_from_ptr<'a>(p: *const c_char) -> &'a [u8] {
    if p.is_null() {
        return &[];
    }
    CStr::from_ptr(p).to_bytes()
}

// Print a C string (raw byte slice) to stdout / stderr using its raw bytes
// (avoids UTF-8 issues).
fn print_bytes(stdout: &mut io::StdoutLock<'_>, bytes: &[u8]) {
    let _ = stdout.write_all(bytes);
}

fn eprint_bytes(bytes: &[u8]) {
    let _ = io::stderr().write_all(bytes);
}

// Equivalent of strncpy(dst, src, n) followed by dst[MAX-1] = 0:
//   - copy up to (MAX-1) bytes from src
//   - pad with NULs the rest of dst
//   - guarantee final NUL terminator at MAX-1
unsafe fn strncpy_terminated(dst: *mut c_char, src: &[u8]) {
    // strncpy semantics: copy up to n bytes; if src has fewer than n bytes
    // (including a stop at any embedded NUL), the rest is NUL-padded.
    let n = MAX_CITY_NAME - 1;
    let src_len = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    let copy_len = src_len.min(n);
    for i in 0..copy_len {
        *dst.add(i) = src[i] as c_char;
    }
    for i in copy_len..n {
        *dst.add(i) = 0;
    }
    *dst.add(MAX_CITY_NAME - 1) = 0;
}

unsafe fn strcmp_bytes(a: *const c_char, b: &[u8]) -> bool {
    // Returns true if equal (mimics strcmp == 0).
    let a_bytes = c_str_from_ptr(a);
    a_bytes == b
}

// ---------- Library functions ----------

unsafe fn create_graph() -> *mut Graph {
    let g = libc::malloc(std::mem::size_of::<Graph>()) as *mut Graph;
    if g.is_null() {
        eprint_bytes(b"Error: Failed to allocate graph\n");
        return ptr::null_mut();
    }
    (*g).node_count = 0;
    for i in 0..MAX_NODES {
        (*g).nodes[i] = ptr::null_mut();
    }
    g
}

unsafe fn add_node(graph: *mut Graph, city_name: &[u8]) -> *mut Node {
    if graph.is_null() {
        eprint_bytes(b"Error: NULL parameter in add_node\n");
        return ptr::null_mut();
    }
    if (*graph).node_count >= MAX_NODES as c_int {
        let mut buf = Vec::new();
        let _ = write!(buf, "Error: Graph is full (max {} nodes)\n", MAX_NODES);
        eprint_bytes(&buf);
        return ptr::null_mut();
    }

    for i in 0..(*graph).node_count as usize {
        let node = (*graph).nodes[i];
        if strcmp_bytes((*node).city_name.as_ptr(), city_name) {
            let mut buf = Vec::new();
            let _ = buf.write_all(b"Error: Node '");
            let _ = buf.write_all(city_name);
            let _ = buf.write_all(b"' already exists\n");
            eprint_bytes(&buf);
            return ptr::null_mut();
        }
    }

    let node = libc::malloc(std::mem::size_of::<Node>()) as *mut Node;
    if node.is_null() {
        eprint_bytes(b"Error: Failed to allocate node\n");
        return ptr::null_mut();
    }
    strncpy_terminated((*node).city_name.as_mut_ptr(), city_name);
    (*node).ref_count = 1;
    (*node).edge_count = 0;
    // edges array is uninitialized (mirrors C's behavior: malloc doesn't
    // zero, and only the first edge_count edges are read).

    let idx = (*graph).node_count as usize;
    (*graph).nodes[idx] = node;
    (*graph).node_count += 1;

    node
}

unsafe fn add_edge(from: *mut Node, to: *mut Node, distance: c_int) -> c_int {
    if from.is_null() || to.is_null() {
        eprint_bytes(b"Error: NULL node in add_edge\n");
        return -1;
    }
    if (*from).edge_count >= MAX_EDGES as c_int {
        let name_bytes = c_str_from_ptr((*from).city_name.as_ptr());
        let mut buf = Vec::new();
        let _ = buf.write_all(b"Error: Node '");
        let _ = buf.write_all(name_bytes);
        let _ = buf.write_all(b"' has maximum edges\n");
        eprint_bytes(&buf);
        return -1;
    }
    if distance < 0 {
        eprint_bytes(b"Error: Negative distance not allowed\n");
        return -1;
    }
    for i in 0..(*from).edge_count as usize {
        if (*from).edges[i].destination == to {
            eprint_bytes(b"Error: Edge already exists\n");
            return -1;
        }
    }
    let idx = (*from).edge_count as usize;
    (*from).edges[idx].destination = to;
    (*from).edges[idx].distance = distance;
    (*from).edge_count += 1;
    0
}

unsafe fn delete_node(node: *mut Node) {
    if node.is_null() {
        return;
    }
    (*node).ref_count -= 1;
    if (*node).ref_count == 0 {
        libc::free(node as *mut libc::c_void);
    }
}

unsafe fn increment_refs_recursive(
    node: *mut Node,
    visited: *mut *mut Node,
    visited_count: *mut c_int,
) {
    if node.is_null() {
        return;
    }
    for i in 0..*visited_count as usize {
        if *visited.add(i) == node {
            return;
        }
    }
    if (*visited_count as usize) < MAX_NODES {
        *visited.add(*visited_count as usize) = node;
        *visited_count += 1;
    }
    (*node).ref_count += 1;
    for i in 0..(*node).edge_count as usize {
        increment_refs_recursive((*node).edges[i].destination, visited, visited_count);
    }
}

unsafe fn shallow_copy(start: *mut Node) -> *mut Node {
    if start.is_null() {
        eprint_bytes(b"Error: NULL node in shallow_copy\n");
        return ptr::null_mut();
    }
    let mut visited: [*mut Node; MAX_NODES] = [ptr::null_mut(); MAX_NODES];
    let mut visited_count: c_int = 0;
    increment_refs_recursive(start, visited.as_mut_ptr(), &mut visited_count);
    start
}

#[derive(Copy, Clone)]
struct DijkstraNode {
    node: *mut Node,
    distance: c_int,
    previous: *mut Node,
    visited: c_int,
}

unsafe fn find_shortest_path(
    start: *mut Node,
    end: *mut Node,
    path_length: *mut c_int,
) -> *mut *mut Node {
    if start.is_null() || end.is_null() || path_length.is_null() {
        eprint_bytes(b"Error: NULL parameter in find_shortest_path\n");
        return ptr::null_mut();
    }

    let mut state: [DijkstraNode; MAX_NODES] = [DijkstraNode {
        node: ptr::null_mut(),
        distance: 0,
        previous: ptr::null_mut(),
        visited: 0,
    }; MAX_NODES];
    let mut state_count: usize = 0;

    state[state_count].node = start;
    state[state_count].distance = 0;
    state[state_count].previous = ptr::null_mut();
    state[state_count].visited = 0;
    state_count += 1;

    let mut current: *mut Node = start;

    while !current.is_null() {
        let mut current_idx: i32 = -1;
        for i in 0..state_count {
            if state[i].node == current {
                current_idx = i as i32;
                break;
            }
        }
        if current_idx == -1 {
            break;
        }
        let cidx = current_idx as usize;
        state[cidx].visited = 1;

        if current == end {
            break;
        }

        let edge_count = (*current).edge_count as usize;
        let curr_distance = state[cidx].distance;
        for i in 0..edge_count {
            let neighbor = (*current).edges[i].destination;
            // C uses int arithmetic; signed overflow is UB but in practice
            // wraps. Use wrapping_add for parity with most C compilers.
            let new_distance = curr_distance.wrapping_add((*current).edges[i].distance);

            let mut neighbor_idx: i32 = -1;
            for j in 0..state_count {
                if state[j].node == neighbor {
                    neighbor_idx = j as i32;
                    break;
                }
            }

            if neighbor_idx == -1 && state_count < MAX_NODES {
                neighbor_idx = state_count as i32;
                state[state_count].node = neighbor;
                state[state_count].distance = c_int::MAX;
                state[state_count].previous = ptr::null_mut();
                state[state_count].visited = 0;
                state_count += 1;
            }

            if neighbor_idx != -1 {
                let ni = neighbor_idx as usize;
                if new_distance < state[ni].distance {
                    state[ni].distance = new_distance;
                    state[ni].previous = current;
                }
            }
        }

        let mut min_distance = c_int::MAX;
        current = ptr::null_mut();
        for i in 0..state_count {
            if state[i].visited == 0 && state[i].distance < min_distance {
                min_distance = state[i].distance;
                current = state[i].node;
            }
        }
    }

    let mut end_idx: i32 = -1;
    for i in 0..state_count {
        if state[i].node == end {
            end_idx = i as i32;
            break;
        }
    }

    if end_idx == -1 || state[end_idx as usize].distance == c_int::MAX {
        eprint_bytes(b"No path found\n");
        *path_length = 0;
        return ptr::null_mut();
    }

    let mut path: [*mut Node; MAX_NODES] = [ptr::null_mut(); MAX_NODES];
    let mut count: usize = 0;
    let mut current_node: *mut Node = end;

    while !current_node.is_null() {
        path[count] = current_node;
        count += 1;

        let mut current_state_idx: i32 = -1;
        for i in 0..state_count {
            if state[i].node == current_node {
                current_state_idx = i as i32;
                break;
            }
        }
        if current_state_idx == -1 {
            break;
        }
        current_node = state[current_state_idx as usize].previous;
    }

    let result =
        libc::malloc(std::mem::size_of::<*mut Node>() * count) as *mut *mut Node;
    if result.is_null() {
        eprint_bytes(b"Error: Failed to allocate path\n");
        *path_length = 0;
        return ptr::null_mut();
    }
    for i in 0..count {
        *result.add(i) = path[count - 1 - i];
    }
    *path_length = count as c_int;
    result
}

unsafe fn get_node_by_name(graph: *mut Graph, city_name: &[u8]) -> *mut Node {
    if graph.is_null() {
        return ptr::null_mut();
    }
    for i in 0..(*graph).node_count as usize {
        let n = (*graph).nodes[i];
        if strcmp_bytes((*n).city_name.as_ptr(), city_name) {
            return n;
        }
    }
    ptr::null_mut()
}

unsafe fn print_node(stdout: &mut io::StdoutLock<'_>, node: *mut Node) {
    if node.is_null() {
        print_bytes(stdout, b"NULL node\n");
        return;
    }
    let name_bytes = c_str_from_ptr((*node).city_name.as_ptr());
    let mut buf = Vec::new();
    let _ = buf.write_all(b"City: ");
    let _ = buf.write_all(name_bytes);
    let _ = write!(buf, " (ref_count: {})\n", (*node).ref_count);
    let _ = buf.write_all(b"  Edges:\n");
    print_bytes(stdout, &buf);

    for i in 0..(*node).edge_count as usize {
        let dest = (*node).edges[i].destination;
        let dest_name = c_str_from_ptr((*dest).city_name.as_ptr());
        let mut line = Vec::new();
        let _ = line.write_all(b"    -> ");
        let _ = line.write_all(dest_name);
        let _ = write!(line, " (distance: {})\n", (*node).edges[i].distance);
        print_bytes(stdout, &line);
    }
}

unsafe fn print_graph(stdout: &mut io::StdoutLock<'_>, graph: *mut Graph) {
    if graph.is_null() {
        print_bytes(stdout, b"NULL graph\n");
        return;
    }
    let mut buf = Vec::new();
    let _ = write!(buf, "Graph with {} nodes:\n", (*graph).node_count);
    print_bytes(stdout, &buf);
    for i in 0..(*graph).node_count as usize {
        print_node(stdout, (*graph).nodes[i]);
    }
}

unsafe fn free_graph(graph: *mut Graph) {
    if graph.is_null() {
        return;
    }
    for i in 0..(*graph).node_count as usize {
        delete_node((*graph).nodes[i]);
    }
    libc::free(graph as *mut libc::c_void);
}

// ---------- Main interactive driver ----------

fn print_menu(stdout: &mut io::StdoutLock<'_>) {
    print_bytes(stdout, b"\n=== DAG City Route Manager ===\n");
    print_bytes(stdout, b"1. Add city (node)\n");
    print_bytes(stdout, b"2. Add route (edge)\n");
    print_bytes(stdout, b"3. Show all cities\n");
    print_bytes(stdout, b"4. Show city details\n");
    print_bytes(stdout, b"5. Find shortest path\n");
    print_bytes(stdout, b"6. Make shallow copy of subsection\n");
    print_bytes(stdout, b"7. Delete node\n");
    print_bytes(stdout, b"8. Exit\n");
    print_bytes(stdout, b"Choice: ");
    let _ = stdout.flush();
}

// strcspn(input, "\n") - return bytes up to first '\n' (or full slice).
fn strip_newline(buf: &[u8]) -> &[u8] {
    match buf.iter().position(|&b| b == b'\n') {
        Some(i) => &buf[..i],
        None => buf,
    }
}

// Parse an integer like sscanf(input, "%d", &x): skip leading whitespace,
// optional sign, then digits. Returns Some(x) if at least one digit parsed.
fn parse_int(buf: &[u8]) -> Option<c_int> {
    let mut i = 0;
    while i < buf.len() && (buf[i] as char).is_whitespace() {
        i += 1;
    }
    let start = i;
    if i < buf.len() && (buf[i] == b'+' || buf[i] == b'-') {
        i += 1;
    }
    let digit_start = i;
    while i < buf.len() && buf[i].is_ascii_digit() {
        i += 1;
    }
    if digit_start == i {
        return None;
    }
    let s = std::str::from_utf8(&buf[start..i]).ok()?;
    // C's sscanf("%d") wraps via undefined behavior on overflow; we use
    // i64 and truncate.
    if let Ok(v) = s.parse::<i64>() {
        return Some(v as c_int);
    }
    // On overflow, fall back to None (matches sscanf returning 0 matches).
    None
}

// Read one line from `reader` into `buf` (overwrites). Returns true on
// success, false on EOF/error (matches fgets returning NULL).
fn read_line<R: BufRead>(reader: &mut R, buf: &mut Vec<u8>) -> bool {
    buf.clear();
    match reader.read_until(b'\n', buf) {
        Ok(0) => false,
        Ok(_) => {
            // Note: fgets reads up to MAX_INPUT - 1 bytes and stops at '\n'.
            // We don't enforce a length cap because realistic inputs are
            // far shorter, and matching that would only matter for very
            // long lines (> 255 chars).
            true
        }
        Err(_) => false,
    }
}

fn main() {
    unsafe {
        let graph = create_graph();
        if graph.is_null() {
            eprint_bytes(b"Failed to create graph\n");
            std::process::exit(1);
        }

        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        let mut line: Vec<u8> = Vec::new();

        print_bytes(&mut stdout, b"City Route Management System\n");
        print_bytes(&mut stdout, b"Commands are read from stdin\n");

        loop {
            print_menu(&mut stdout);

            if !read_line(&mut reader, &mut line) {
                break;
            }

            let choice = match parse_int(&line) {
                Some(c) => c,
                None => {
                    print_bytes(&mut stdout, b"Invalid input\n");
                    continue;
                }
            };

            match choice {
                1 => {
                    print_bytes(&mut stdout, b"Enter city name: ");
                    let _ = stdout.flush();
                    if !read_line(&mut reader, &mut line) {
                        break;
                    }
                    let name = strip_newline(&line).to_vec();
                    let node = add_node(graph, &name);
                    if !node.is_null() {
                        let mut buf = Vec::new();
                        let _ = buf.write_all(b"Added city: ");
                        let _ = buf.write_all(&name);
                        let _ = buf.write_all(b"\n");
                        print_bytes(&mut stdout, &buf);
                    } else {
                        print_bytes(&mut stdout, b"Failed to add city\n");
                    }
                }
                2 => {
                    let mut from_buf: Vec<u8> = Vec::new();
                    let mut to_buf: Vec<u8> = Vec::new();
                    let mut dist_buf: Vec<u8> = Vec::new();

                    print_bytes(&mut stdout, b"Enter from city: ");
                    let _ = stdout.flush();
                    if !read_line(&mut reader, &mut from_buf) {
                        break;
                    }
                    let from_city = strip_newline(&from_buf).to_vec();

                    print_bytes(&mut stdout, b"Enter to city: ");
                    let _ = stdout.flush();
                    if !read_line(&mut reader, &mut to_buf) {
                        break;
                    }
                    let to_city = strip_newline(&to_buf).to_vec();

                    print_bytes(&mut stdout, b"Enter distance: ");
                    let _ = stdout.flush();
                    if !read_line(&mut reader, &mut dist_buf) {
                        break;
                    }
                    let distance = match parse_int(&dist_buf) {
                        Some(d) => d,
                        None => {
                            print_bytes(&mut stdout, b"Invalid distance\n");
                            continue;
                        }
                    };

                    let from = get_node_by_name(graph, &from_city);
                    let to = get_node_by_name(graph, &to_city);

                    if from.is_null() {
                        let mut buf = Vec::new();
                        let _ = buf.write_all(b"City '");
                        let _ = buf.write_all(&from_city);
                        let _ = buf.write_all(b"' not found\n");
                        print_bytes(&mut stdout, &buf);
                        continue;
                    }
                    if to.is_null() {
                        let mut buf = Vec::new();
                        let _ = buf.write_all(b"City '");
                        let _ = buf.write_all(&to_city);
                        let _ = buf.write_all(b"' not found\n");
                        print_bytes(&mut stdout, &buf);
                        continue;
                    }

                    if add_edge(from, to, distance) == 0 {
                        let mut buf = Vec::new();
                        let _ = buf.write_all(b"Added route: ");
                        let _ = buf.write_all(&from_city);
                        let _ = buf.write_all(b" -> ");
                        let _ = buf.write_all(&to_city);
                        let _ = write!(buf, " (distance: {})\n", distance);
                        print_bytes(&mut stdout, &buf);
                    } else {
                        print_bytes(&mut stdout, b"Failed to add route\n");
                    }
                }
                3 => {
                    print_graph(&mut stdout, graph);
                }
                4 => {
                    print_bytes(&mut stdout, b"Enter city name: ");
                    let _ = stdout.flush();
                    if !read_line(&mut reader, &mut line) {
                        break;
                    }
                    let name = strip_newline(&line).to_vec();
                    let node = get_node_by_name(graph, &name);
                    if !node.is_null() {
                        print_node(&mut stdout, node);
                    } else {
                        let mut buf = Vec::new();
                        let _ = buf.write_all(b"City '");
                        let _ = buf.write_all(&name);
                        let _ = buf.write_all(b"' not found\n");
                        print_bytes(&mut stdout, &buf);
                    }
                }
                5 => {
                    let mut start_buf: Vec<u8> = Vec::new();
                    let mut end_buf: Vec<u8> = Vec::new();

                    print_bytes(&mut stdout, b"Enter start city: ");
                    let _ = stdout.flush();
                    if !read_line(&mut reader, &mut start_buf) {
                        break;
                    }
                    let start_city = strip_newline(&start_buf).to_vec();

                    print_bytes(&mut stdout, b"Enter end city: ");
                    let _ = stdout.flush();
                    if !read_line(&mut reader, &mut end_buf) {
                        break;
                    }
                    let end_city = strip_newline(&end_buf).to_vec();

                    let start = get_node_by_name(graph, &start_city);
                    let end = get_node_by_name(graph, &end_city);

                    if start.is_null() {
                        let mut buf = Vec::new();
                        let _ = buf.write_all(b"City '");
                        let _ = buf.write_all(&start_city);
                        let _ = buf.write_all(b"' not found\n");
                        print_bytes(&mut stdout, &buf);
                        continue;
                    }
                    if end.is_null() {
                        let mut buf = Vec::new();
                        let _ = buf.write_all(b"City '");
                        let _ = buf.write_all(&end_city);
                        let _ = buf.write_all(b"' not found\n");
                        print_bytes(&mut stdout, &buf);
                        continue;
                    }

                    let mut path_length: c_int = 0;
                    let path = find_shortest_path(start, end, &mut path_length);
                    if !path.is_null() {
                        let mut buf = Vec::new();
                        let _ = buf.write_all(b"Shortest path from ");
                        let _ = buf.write_all(&start_city);
                        let _ = buf.write_all(b" to ");
                        let _ = buf.write_all(&end_city);
                        let _ = buf.write_all(b":\n");
                        print_bytes(&mut stdout, &buf);
                        for i in 0..path_length as usize {
                            let n = *path.add(i);
                            let name_bytes = c_str_from_ptr((*n).city_name.as_ptr());
                            let mut line2 = Vec::new();
                            let _ = write!(line2, "  {}. ", i + 1);
                            let _ = line2.write_all(name_bytes);
                            let _ = line2.write_all(b"\n");
                            print_bytes(&mut stdout, &line2);
                        }
                        libc::free(path as *mut libc::c_void);
                    } else {
                        print_bytes(&mut stdout, b"No path found\n");
                    }
                }
                6 => {
                    print_bytes(&mut stdout, b"Enter start city for shallow copy: ");
                    let _ = stdout.flush();
                    if !read_line(&mut reader, &mut line) {
                        break;
                    }
                    let name = strip_newline(&line).to_vec();
                    let node = get_node_by_name(graph, &name);
                    if node.is_null() {
                        let mut buf = Vec::new();
                        let _ = buf.write_all(b"City '");
                        let _ = buf.write_all(&name);
                        let _ = buf.write_all(b"' not found\n");
                        print_bytes(&mut stdout, &buf);
                        continue;
                    }
                    let copy = shallow_copy(node);
                    if !copy.is_null() {
                        let mut buf = Vec::new();
                        let _ = buf.write_all(b"Created shallow copy starting from ");
                        let _ = buf.write_all(&name);
                        let _ = buf.write_all(b"\n");
                        print_bytes(&mut stdout, &buf);
                        print_bytes(
                            &mut stdout,
                            b"Reference counts incremented for all reachable nodes\n",
                        );
                        print_node(&mut stdout, copy);
                    } else {
                        print_bytes(&mut stdout, b"Failed to create shallow copy\n");
                    }
                }
                7 => {
                    print_bytes(&mut stdout, b"Enter city name to delete: ");
                    let _ = stdout.flush();
                    if !read_line(&mut reader, &mut line) {
                        break;
                    }
                    let name = strip_newline(&line).to_vec();
                    let node = get_node_by_name(graph, &name);
                    if node.is_null() {
                        let mut buf = Vec::new();
                        let _ = buf.write_all(b"City '");
                        let _ = buf.write_all(&name);
                        let _ = buf.write_all(b"' not found\n");
                        print_bytes(&mut stdout, &buf);
                        continue;
                    }
                    let mut buf = Vec::new();
                    let _ = write!(buf, "Current ref count: {}\n", (*node).ref_count);
                    print_bytes(&mut stdout, &buf);
                    delete_node(node);
                    let mut buf2 = Vec::new();
                    let _ = buf2.write_all(b"Decremented reference count for ");
                    let _ = buf2.write_all(&name);
                    let _ = buf2.write_all(b"\n");
                    print_bytes(&mut stdout, &buf2);
                    print_bytes(
                        &mut stdout,
                        b"Note: Node will be freed when ref count reaches 0\n",
                    );
                }
                8 => {
                    print_bytes(&mut stdout, b"Freeing graph and exiting...\n");
                    let _ = stdout.flush();
                    free_graph(graph);
                    return;
                }
                _ => {
                    print_bytes(&mut stdout, b"Invalid choice\n");
                }
            }
        }

        free_graph(graph);
    }
}
