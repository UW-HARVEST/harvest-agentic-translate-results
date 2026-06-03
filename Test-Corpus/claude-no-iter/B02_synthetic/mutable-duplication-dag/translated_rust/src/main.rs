// Rust translation of the C DAG city route manager.
//
// The original program manages a directed graph of cities with reference-counted
// nodes. We mirror its observable behavior (output bytes, error messages, prompt
// order, and reference-count semantics) using Rc<RefCell<Node>> internally.

use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::rc::Rc;

const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;
const MAX_INPUT: usize = 256;

type NodeRef = Rc<RefCell<Node>>;

struct Edge {
    destination: NodeRef,
    distance: i32,
}

struct Node {
    /// Stored as raw bytes (no trailing NUL), truncated to MAX_CITY_NAME-1.
    city_name: Vec<u8>,
    ref_count: i32,
    edges: Vec<Edge>,
}

struct Graph {
    nodes: Vec<NodeRef>,
}

// ---------------------------------------------------------------------------
// stdout helpers — write raw bytes to mimic printf's exact byte output.
// ---------------------------------------------------------------------------

fn write_stdout(bytes: &[u8]) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(bytes);
}

fn write_stderr(bytes: &[u8]) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    let _ = handle.write_all(bytes);
}

fn print_str(s: &str) {
    write_stdout(s.as_bytes());
}

fn eprint_str(s: &str) {
    write_stderr(s.as_bytes());
}

// ---------------------------------------------------------------------------
// fgets-equivalent: read at most max-1 bytes from stdin, stopping at '\n' or
// EOF. Returns false if no bytes were read (matches fgets returning NULL).
// ---------------------------------------------------------------------------

fn fgets(buf: &mut Vec<u8>, max: usize, reader: &mut impl Read) -> bool {
    buf.clear();
    if max <= 1 {
        return false;
    }
    let mut byte = [0u8; 1];
    while buf.len() < max - 1 {
        match reader.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => return false,
        }
    }
    !buf.is_empty()
}

/// Mimics `input[strcspn(input, "\n")] = 0`: truncate at first '\n'.
fn strip_newline(buf: &mut Vec<u8>) {
    if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        buf.truncate(pos);
    }
}

/// Mimics `sscanf(input, "%d", &x)` returning 1 on success.
/// Skips leading whitespace, parses an optional sign and digits.
fn sscanf_int(s: &[u8]) -> Option<i32> {
    let mut i = 0;
    while i < s.len() && (s[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let start = i;
    let mut value: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((s[i] - b'0') as i64);
        i += 1;
    }
    if i == start {
        return None;
    }
    let value = if neg { value.wrapping_neg() } else { value };
    Some(value as i32)
}

// ---------------------------------------------------------------------------
// Graph operations (translations of dag_lib.c).
// ---------------------------------------------------------------------------

fn create_graph() -> Graph {
    Graph { nodes: Vec::new() }
}

fn add_node(graph: &mut Graph, city_name: &[u8]) -> Option<NodeRef> {
    if graph.nodes.len() >= MAX_NODES {
        eprint_str(&format!("Error: Graph is full (max {} nodes)\n", MAX_NODES));
        return None;
    }

    // Check if node already exists. In C, strcmp compares the stored
    // (already-truncated) name with the full input string, so we compare
    // stored bytes against the input verbatim.
    for n in &graph.nodes {
        if n.borrow().city_name.as_slice() == city_name {
            // %s prints the input string as-is
            write_stderr(b"Error: Node '");
            write_stderr(city_name);
            write_stderr(b"' already exists\n");
            return None;
        }
    }

    // Truncate city name to MAX_CITY_NAME - 1 bytes (mirrors strncpy behavior).
    let truncated_len = city_name.len().min(MAX_CITY_NAME - 1);
    let stored_name = city_name[..truncated_len].to_vec();

    let node = Rc::new(RefCell::new(Node {
        city_name: stored_name,
        ref_count: 1,
        edges: Vec::new(),
    }));

    graph.nodes.push(Rc::clone(&node));
    Some(node)
}

fn add_edge(from: &NodeRef, to: &NodeRef, distance: i32) -> i32 {
    {
        let from_borrow = from.borrow();
        if from_borrow.edges.len() >= MAX_EDGES {
            // Mirror "Error: Node '%s' has maximum edges\n"
            write_stderr(b"Error: Node '");
            write_stderr(&from_borrow.city_name);
            write_stderr(b"' has maximum edges\n");
            return -1;
        }
    }

    if distance < 0 {
        eprint_str("Error: Negative distance not allowed\n");
        return -1;
    }

    // Check duplicate edge (pointer equality).
    {
        let from_borrow = from.borrow();
        for e in &from_borrow.edges {
            if Rc::ptr_eq(&e.destination, to) {
                eprint_str("Error: Edge already exists\n");
                return -1;
            }
        }
    }

    from.borrow_mut().edges.push(Edge {
        destination: Rc::clone(to),
        distance,
    });
    0
}

fn delete_node(node: &NodeRef) {
    // Decrement ref_count; we don't actually drop here because Rc owns the
    // memory. The visible behavior (the printed ref_count) is preserved.
    let mut n = node.borrow_mut();
    n.ref_count -= 1;
    // In C, when ref_count == 0 the node is freed. We intentionally let Rc
    // keep the value alive to avoid use-after-free, since the original C
    // program leaves dangling pointers in the graph after deletion.
}

fn increment_refs_recursive(node: &NodeRef, visited: &mut Vec<NodeRef>) {
    // Already visited?
    for v in visited.iter() {
        if Rc::ptr_eq(v, node) {
            return;
        }
    }
    if visited.len() < MAX_NODES {
        visited.push(Rc::clone(node));
    }
    node.borrow_mut().ref_count += 1;
    // Snapshot the edge destinations to avoid holding a borrow during recursion.
    let destinations: Vec<NodeRef> = node
        .borrow()
        .edges
        .iter()
        .map(|e| Rc::clone(&e.destination))
        .collect();
    for d in &destinations {
        increment_refs_recursive(d, visited);
    }
}

fn shallow_copy(start: &NodeRef) -> NodeRef {
    let mut visited: Vec<NodeRef> = Vec::new();
    increment_refs_recursive(start, &mut visited);
    Rc::clone(start)
}

struct DijkstraEntry {
    node: NodeRef,
    distance: i32,
    previous: Option<NodeRef>,
    visited: bool,
}

/// Returns Some(path) on success. On failure prints "No path found\n" to stderr
/// (matching the original C behavior) and returns None.
fn find_shortest_path(start: &NodeRef, end: &NodeRef) -> Option<Vec<NodeRef>> {
    let mut state: Vec<DijkstraEntry> = Vec::new();
    state.push(DijkstraEntry {
        node: Rc::clone(start),
        distance: 0,
        previous: None,
        visited: false,
    });

    let mut current: Option<NodeRef> = Some(Rc::clone(start));

    while let Some(cur) = current.clone() {
        // Find current node in state.
        let mut current_idx: Option<usize> = None;
        for (i, e) in state.iter().enumerate() {
            if Rc::ptr_eq(&e.node, &cur) {
                current_idx = Some(i);
                break;
            }
        }
        let current_idx = match current_idx {
            Some(i) => i,
            None => break,
        };

        state[current_idx].visited = true;

        // If we reached the end, stop exploring.
        if Rc::ptr_eq(&cur, end) {
            break;
        }

        // Snapshot edge info to avoid borrow conflicts.
        let edges: Vec<(NodeRef, i32)> = cur
            .borrow()
            .edges
            .iter()
            .map(|e| (Rc::clone(&e.destination), e.distance))
            .collect();

        let cur_distance = state[current_idx].distance;
        for (neighbor, edge_distance) in edges {
            let new_distance = cur_distance.wrapping_add(edge_distance);

            let mut neighbor_idx: Option<usize> = None;
            for (j, e) in state.iter().enumerate() {
                if Rc::ptr_eq(&e.node, &neighbor) {
                    neighbor_idx = Some(j);
                    break;
                }
            }

            if neighbor_idx.is_none() && state.len() < MAX_NODES {
                neighbor_idx = Some(state.len());
                state.push(DijkstraEntry {
                    node: Rc::clone(&neighbor),
                    distance: i32::MAX,
                    previous: None,
                    visited: false,
                });
            }

            if let Some(idx) = neighbor_idx {
                if new_distance < state[idx].distance {
                    state[idx].distance = new_distance;
                    state[idx].previous = Some(Rc::clone(&cur));
                }
            }
        }

        // Pick the next unvisited node with the smallest distance.
        let mut min_distance = i32::MAX;
        let mut next: Option<NodeRef> = None;
        for e in state.iter() {
            if !e.visited && e.distance < min_distance {
                min_distance = e.distance;
                next = Some(Rc::clone(&e.node));
            }
        }
        current = next;
    }

    // Find end node in state.
    let mut end_idx: Option<usize> = None;
    for (i, e) in state.iter().enumerate() {
        if Rc::ptr_eq(&e.node, end) {
            end_idx = Some(i);
            break;
        }
    }

    let end_idx = match end_idx {
        Some(i) => i,
        None => {
            eprint_str("No path found\n");
            return None;
        }
    };
    if state[end_idx].distance == i32::MAX {
        eprint_str("No path found\n");
        return None;
    }

    // Reconstruct path (end -> start), then reverse.
    let mut path: Vec<NodeRef> = Vec::new();
    let mut current_node: Option<NodeRef> = Some(Rc::clone(end));
    while let Some(cn) = current_node.clone() {
        path.push(Rc::clone(&cn));

        let mut idx: Option<usize> = None;
        for (i, e) in state.iter().enumerate() {
            if Rc::ptr_eq(&e.node, &cn) {
                idx = Some(i);
                break;
            }
        }
        match idx {
            Some(i) => {
                current_node = state[i].previous.as_ref().map(Rc::clone);
            }
            None => break,
        }
    }

    path.reverse();
    Some(path)
}

fn get_node_by_name(graph: &Graph, city_name: &[u8]) -> Option<NodeRef> {
    for n in &graph.nodes {
        if n.borrow().city_name.as_slice() == city_name {
            return Some(Rc::clone(n));
        }
    }
    None
}

fn print_node(node: &NodeRef) {
    // "City: %s (ref_count: %d)\n"
    let n = node.borrow();
    write_stdout(b"City: ");
    write_stdout(&n.city_name);
    print_str(&format!(" (ref_count: {})\n", n.ref_count));
    print_str("  Edges:\n");
    for e in &n.edges {
        // "    -> %s (distance: %d)\n"
        write_stdout(b"    -> ");
        write_stdout(&e.destination.borrow().city_name);
        print_str(&format!(" (distance: {})\n", e.distance));
    }
}

fn print_graph(graph: &Graph) {
    print_str(&format!("Graph with {} nodes:\n", graph.nodes.len()));
    for n in &graph.nodes {
        print_node(n);
    }
}

fn free_graph(graph: &mut Graph) {
    // Mirror C behavior: decrement ref_count for each node.
    for n in &graph.nodes {
        delete_node(n);
    }
    graph.nodes.clear();
}

// ---------------------------------------------------------------------------
// Main loop (translation of main.c).
// ---------------------------------------------------------------------------

fn print_menu() {
    print_str("\n=== DAG City Route Manager ===\n");
    print_str("1. Add city (node)\n");
    print_str("2. Add route (edge)\n");
    print_str("3. Show all cities\n");
    print_str("4. Show city details\n");
    print_str("5. Find shortest path\n");
    print_str("6. Make shallow copy of subsection\n");
    print_str("7. Delete node\n");
    print_str("8. Exit\n");
    print_str("Choice: ");
}

fn main() {
    let mut graph = create_graph();
    let stdin = io::stdin();
    let mut stdin_handle = stdin.lock();

    let mut input: Vec<u8> = Vec::new();

    print_str("City Route Management System\n");
    print_str("Commands are read from stdin\n");

    loop {
        print_menu();

        if !fgets(&mut input, MAX_INPUT, &mut stdin_handle) {
            break;
        }

        let choice = match sscanf_int(&input) {
            Some(v) => v,
            None => {
                print_str("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => {
                // Add city
                print_str("Enter city name: ");
                if !fgets(&mut input, MAX_INPUT, &mut stdin_handle) {
                    continue;
                }
                strip_newline(&mut input);

                let node_opt = add_node(&mut graph, &input);
                if node_opt.is_some() {
                    write_stdout(b"Added city: ");
                    write_stdout(&input);
                    write_stdout(b"\n");
                } else {
                    print_str("Failed to add city\n");
                }
            }
            2 => {
                // Add route
                let mut from_city: Vec<u8> = Vec::new();
                let mut to_city: Vec<u8> = Vec::new();

                print_str("Enter from city: ");
                if !fgets(&mut from_city, MAX_INPUT, &mut stdin_handle) {
                    continue;
                }
                strip_newline(&mut from_city);

                print_str("Enter to city: ");
                if !fgets(&mut to_city, MAX_INPUT, &mut stdin_handle) {
                    continue;
                }
                strip_newline(&mut to_city);

                print_str("Enter distance: ");
                if !fgets(&mut input, MAX_INPUT, &mut stdin_handle) {
                    continue;
                }
                let distance = match sscanf_int(&input) {
                    Some(v) => v,
                    None => {
                        print_str("Invalid distance\n");
                        continue;
                    }
                };

                let from = get_node_by_name(&graph, &from_city);
                let to = get_node_by_name(&graph, &to_city);

                if from.is_none() {
                    write_stdout(b"City '");
                    write_stdout(&from_city);
                    write_stdout(b"' not found\n");
                    continue;
                }
                if to.is_none() {
                    write_stdout(b"City '");
                    write_stdout(&to_city);
                    write_stdout(b"' not found\n");
                    continue;
                }
                let from = from.unwrap();
                let to = to.unwrap();

                if add_edge(&from, &to, distance) == 0 {
                    write_stdout(b"Added route: ");
                    write_stdout(&from_city);
                    write_stdout(b" -> ");
                    write_stdout(&to_city);
                    print_str(&format!(" (distance: {})\n", distance));
                } else {
                    print_str("Failed to add route\n");
                }
            }
            3 => {
                print_graph(&graph);
            }
            4 => {
                print_str("Enter city name: ");
                if !fgets(&mut input, MAX_INPUT, &mut stdin_handle) {
                    continue;
                }
                strip_newline(&mut input);

                match get_node_by_name(&graph, &input) {
                    Some(n) => print_node(&n),
                    None => {
                        write_stdout(b"City '");
                        write_stdout(&input);
                        write_stdout(b"' not found\n");
                    }
                }
            }
            5 => {
                let mut start_city: Vec<u8> = Vec::new();
                let mut end_city: Vec<u8> = Vec::new();

                print_str("Enter start city: ");
                if !fgets(&mut start_city, MAX_INPUT, &mut stdin_handle) {
                    continue;
                }
                strip_newline(&mut start_city);

                print_str("Enter end city: ");
                if !fgets(&mut end_city, MAX_INPUT, &mut stdin_handle) {
                    continue;
                }
                strip_newline(&mut end_city);

                let start = get_node_by_name(&graph, &start_city);
                let end = get_node_by_name(&graph, &end_city);

                if start.is_none() {
                    write_stdout(b"City '");
                    write_stdout(&start_city);
                    write_stdout(b"' not found\n");
                    continue;
                }
                if end.is_none() {
                    write_stdout(b"City '");
                    write_stdout(&end_city);
                    write_stdout(b"' not found\n");
                    continue;
                }
                let start = start.unwrap();
                let end = end.unwrap();

                let path = find_shortest_path(&start, &end);
                match path {
                    Some(p) => {
                        write_stdout(b"Shortest path from ");
                        write_stdout(&start_city);
                        write_stdout(b" to ");
                        write_stdout(&end_city);
                        write_stdout(b":\n");
                        for (i, n) in p.iter().enumerate() {
                            print_str(&format!("  {}. ", i + 1));
                            write_stdout(&n.borrow().city_name);
                            write_stdout(b"\n");
                        }
                    }
                    None => {
                        print_str("No path found\n");
                    }
                }
            }
            6 => {
                print_str("Enter start city for shallow copy: ");
                if !fgets(&mut input, MAX_INPUT, &mut stdin_handle) {
                    continue;
                }
                strip_newline(&mut input);

                match get_node_by_name(&graph, &input) {
                    None => {
                        write_stdout(b"City '");
                        write_stdout(&input);
                        write_stdout(b"' not found\n");
                    }
                    Some(node) => {
                        let copy = shallow_copy(&node);
                        write_stdout(b"Created shallow copy starting from ");
                        write_stdout(&input);
                        write_stdout(b"\n");
                        print_str("Reference counts incremented for all reachable nodes\n");
                        print_node(&copy);
                    }
                }
            }
            7 => {
                print_str("Enter city name to delete: ");
                if !fgets(&mut input, MAX_INPUT, &mut stdin_handle) {
                    continue;
                }
                strip_newline(&mut input);

                match get_node_by_name(&graph, &input) {
                    None => {
                        write_stdout(b"City '");
                        write_stdout(&input);
                        write_stdout(b"' not found\n");
                    }
                    Some(node) => {
                        let rc_now = node.borrow().ref_count;
                        print_str(&format!("Current ref count: {}\n", rc_now));
                        delete_node(&node);
                        write_stdout(b"Decremented reference count for ");
                        write_stdout(&input);
                        write_stdout(b"\n");
                        print_str("Note: Node will be freed when ref count reaches 0\n");
                    }
                }
            }
            8 => {
                print_str("Freeing graph and exiting...\n");
                free_graph(&mut graph);
                let _ = io::stdout().flush();
                std::process::exit(0);
            }
            _ => {
                print_str("Invalid choice\n");
            }
        }
    }

    free_graph(&mut graph);
    let _ = io::stdout().flush();
}

// Suppress dead_code warning for `previous` field reads only via debug paths.
#[allow(dead_code)]
fn _silence_unused() {
    let _ = MAX_CITY_NAME;
}
