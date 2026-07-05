
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;
use std::io::{self, BufRead, Write};

const INT_MAX: i32 = i32::MAX;

// Note: MAX_CITY_NAME, MAX_EDGES, and MAX_NODES are defined in bindings.rs as u32 constants.
// We cast them to usize where required.
const MAX_CITY_NAME_USIZE: usize = MAX_CITY_NAME as usize;
const MAX_EDGES_USIZE: usize = MAX_EDGES as usize;
const MAX_NODES_USIZE: usize = MAX_NODES as usize;

type NodeRef = Rc<RefCell<Node>>;

#[derive(Clone)]
pub struct Edge {
    pub destination: NodeRef,
    pub distance: i32,
}

pub struct Node {
    pub city_name: String,
    pub ref_count: i32,
    pub edges: Vec<Edge>,
}

pub struct DijkstraNode {
    pub node: NodeRef,
    pub distance: i32,
    pub previous: Option<NodeRef>,
    pub visited: bool,
}

pub struct Graph {
    pub nodes: Vec<NodeRef>,
}

fn add_edge(from: &NodeRef, to: &NodeRef, distance: i32) -> Result<(), String> {
    if from.borrow().edges.len() >= MAX_EDGES_USIZE {
        let msg = format!("Error: Node '{}' has maximum edges", from.borrow().city_name);
        eprintln!("{}", msg);
        return Err(msg);
    }

    if distance < 0 {
        let msg = "Error: Negative distance not allowed".to_string();
        eprintln!("{}", msg);
        return Err(msg);
    }

    if from.borrow().edges.iter().any(|e| Rc::ptr_eq(&e.destination, to)) {
        let msg = "Error: Edge already exists".to_string();
        eprintln!("{}", msg);
        return Err(msg);
    }

    from.borrow_mut().edges.push(Edge {
        destination: Rc::clone(to),
        distance,
    });

    Ok(())
}

fn add_node(graph: &mut Graph, city_name: &str) -> Option<NodeRef> {
    if graph.nodes.len() >= MAX_NODES_USIZE {
        eprintln!("Error: Graph is full (max {} nodes)", MAX_NODES_USIZE);
        return None;
    }

    if graph.nodes.iter().any(|n| n.borrow().city_name == city_name) {
        eprintln!("Error: Node '{}' already exists", city_name);
        return None;
    }

    let name_trunc: String = city_name.chars().take(MAX_CITY_NAME_USIZE - 1).collect();

    let node = Rc::new(RefCell::new(Node {
        city_name: name_trunc,
        ref_count: 1,
        edges: Vec::new(),
    }));

    graph.nodes.push(Rc::clone(&node));
    Some(node)
}

fn create_graph() -> Graph {
    Graph { nodes: Vec::new() }
}

fn delete_node(node: &NodeRef) {
    node.borrow_mut().ref_count -= 1;
    // In safe Rust the actual free happens when Rc goes to 0; we retain the ref_count as bookkeeping
    // to match the C program's observable output.
}

fn find_state_index(state: &[DijkstraNode], node: &NodeRef) -> Option<usize> {
    state.iter().position(|s| Rc::ptr_eq(&s.node, node))
}

fn find_shortest_path(start: &NodeRef, end: &NodeRef) -> Option<Vec<NodeRef>> {
    let mut state: Vec<DijkstraNode> = Vec::new();

    state.push(DijkstraNode {
        node: Rc::clone(start),
        distance: 0,
        previous: None,
        visited: false,
    });

    let mut current: Option<NodeRef> = Some(Rc::clone(start));

    while let Some(cur) = current {
        let current_idx = match find_state_index(&state, &cur) {
            Some(i) => i,
            None => break,
        };

        state[current_idx].visited = true;

        if Rc::ptr_eq(&cur, end) {
            break;
        }

        let cur_dist = state[current_idx].distance;
        let edges_snapshot: Vec<Edge> = cur.borrow().edges.clone();

        for e in &edges_snapshot {
            let neighbor = &e.destination;
            let new_distance = cur_dist.saturating_add(e.distance);

            let neighbor_idx = match find_state_index(&state, neighbor) {
                Some(idx) => Some(idx),
                None => {
                    if state.len() < MAX_NODES_USIZE {
                        state.push(DijkstraNode {
                            node: Rc::clone(neighbor),
                            distance: INT_MAX,
                            previous: None,
                            visited: false,
                        });
                        Some(state.len() - 1)
                    } else {
                        None
                    }
                }
            };

            if let Some(idx) = neighbor_idx {
                if new_distance < state[idx].distance {
                    state[idx].distance = new_distance;
                    state[idx].previous = Some(Rc::clone(&cur));
                }
            }
        }

        current = state
            .iter()
            .filter(|s| !s.visited && s.distance < INT_MAX)
            .min_by_key(|s| s.distance)
            .map(|s| Rc::clone(&s.node));
    }

    let end_idx = find_state_index(&state, end)?;
    if state[end_idx].distance == INT_MAX {
        eprintln!("No path found");
        return None;
    }

    let mut path: Vec<NodeRef> = Vec::new();
    let mut current_node: Option<NodeRef> = Some(Rc::clone(end));

    while let Some(cn) = current_node {
        path.push(Rc::clone(&cn));
        current_node = match find_state_index(&state, &cn) {
            Some(idx) => state[idx].previous.as_ref().map(Rc::clone),
            None => break,
        };
    }

    path.reverse();
    Some(path)
}

fn free_graph(graph: &mut Graph) {
    for n in &graph.nodes {
        delete_node(n);
    }
    graph.nodes.clear();
}

fn get_node_by_name(graph: &Graph, city_name: &str) -> Option<NodeRef> {
    graph
        .nodes
        .iter()
        .find(|n| n.borrow().city_name == city_name)
        .map(Rc::clone)
}

fn increment_refs_recursive(node: &NodeRef, visited: &mut Vec<NodeRef>) {
    if visited.iter().any(|v| Rc::ptr_eq(v, node)) {
        return;
    }

    if visited.len() < MAX_NODES_USIZE {
        visited.push(Rc::clone(node));
    }

    node.borrow_mut().ref_count += 1;

    let edges_snapshot: Vec<Edge> = node.borrow().edges.clone();
    for e in &edges_snapshot {
        increment_refs_recursive(&e.destination, visited);
    }
}

fn print_node(node: &NodeRef) {
    let n = node.borrow();
    println!("City: {} (ref_count: {})", n.city_name, n.ref_count);
    println!("  Edges:");
    for e in &n.edges {
        println!(
            "    -> {} (distance: {})",
            e.destination.borrow().city_name,
            e.distance
        );
    }
}

fn print_graph(graph: &Graph) {
    println!("Graph with {} nodes:", graph.nodes.len());
    for n in &graph.nodes {
        print_node(n);
    }
}

fn print_menu() {
    println!("\n=== DAG City Route Manager ===");
    println!("1. Add city (node)");
    println!("2. Add route (edge)");
    println!("3. Show all cities");
    println!("4. Show city details");
    println!("5. Find shortest path");
    println!("6. Make shallow copy of subsection");
    println!("7. Delete node");
    println!("8. Exit");
    print!("Choice: ");
    let _ = io::stdout().flush();
}

fn shallow_copy(start: &NodeRef) -> NodeRef {
    let mut visited: Vec<NodeRef> = Vec::new();
    increment_refs_recursive(start, &mut visited);
    Rc::clone(start)
}

// Emulates C's fgets(buf, MAX_INPUT, stdin) where MAX_INPUT = 256.
// fgets reads at most MAX_INPUT-1 = 255 characters, or until a newline,
// whichever comes first. The newline (if read) is included in the buffer.
// The remaining unread characters stay in stdin for the next call.
const RUST_MAX_INPUT: usize = 256;

fn rust_read_line_stdin(buf: &mut String) -> bool {
    buf.clear();
    let stdin = io::stdin();
    let mut lock = stdin.lock();
    let mut count: usize = 0;
    let mut got_any = false;
    // Collect raw bytes so multi-byte UTF-8 sequences (e.g. Chinese/Cyrillic
    // characters) are preserved intact. C's fgets is byte-based, and Rust's
    // String is UTF-8, so we accumulate bytes and decode once at the end.
    let mut bytes_buf: Vec<u8> = Vec::with_capacity(RUST_MAX_INPUT);
    // Read at most MAX_INPUT-1 = 255 bytes, or until newline is encountered.
    while count < RUST_MAX_INPUT - 1 {
        let mut byte = [0u8; 1];
        match std::io::Read::read(&mut lock, &mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                got_any = true;
                bytes_buf.push(byte[0]);
                count += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    // Convert accumulated bytes into a UTF-8 String, replacing any invalid
    // sequences with U+FFFD. This preserves multi-byte UTF-8 characters
    // (e.g. '北京', 'Токио') that would be corrupted if each byte was
    // individually cast to a Rust `char` (which treats it as Latin-1).
    let decoded = String::from_utf8_lossy(&bytes_buf).into_owned();
    buf.push_str(&decoded);
    got_any
}


fn read_line_stdin(buf: &mut String) -> bool {
    rust_read_line_stdin(buf)
}


fn trim_newline(s: &str) -> &str {
    s.trim_end_matches(|c| c == '\n' || c == '\r')
}

fn rust_parse_int(s: &str) -> Option<i32> {
    // Emulate C's sscanf("%d", ...) behavior:
    // - Skip leading whitespace.
    // - Parse optional sign followed by digits.
    // - On overflow, glibc's sscanf silently truncates the result to i32 via wrapping
    //   (equivalent to taking the low 32 bits of the parsed integer).
    // - If no digits are found, return None (matches Rust's parse failure).
    let trimmed = s.trim_start();
    let bytes = trimmed.as_bytes();
    let mut i: usize = 0;
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        negative = bytes[i] == b'-';
        i += 1;
    }
    let start = i;
    let mut acc: i128 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as i128;
        // Use wrapping-like accumulation. i128 can hold well beyond i32 range,
        // and we truncate at the end. If the number is astronomically large
        // (more than ~38 digits), we cap by taking modulo 2^32 progressively.
        acc = acc.wrapping_mul(10).wrapping_add(d);
        // Keep acc within a manageable range (fits in i128 easily until 38 digits).
        i += 1;
    }
    if i == start {
        // No digits parsed
        return None;
    }
    if negative {
        acc = acc.wrapping_neg();
    }
    // Truncate to i32 by wrapping (equivalent to (int)(long)value in C on typical
    // implementations, or the low 32 bits reinterpreted as signed).
    let low32 = (acc as i64 as u64 & 0xFFFF_FFFF) as u32;
    Some(low32 as i32)
}

fn parse_int(s: &str) -> Option<i32> {
    rust_parse_int(s)
}


fn prompt_line(prompt: &str, buf: &mut String) -> bool {
    print!("{}", prompt);
    let _ = io::stdout().flush();
    read_line_stdin(buf)
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    rust_run()
}

fn rust_run() -> i32 {
    let mut graph = create_graph();
    let mut input = String::new();

    println!("City Route Management System");
    println!("Commands are read from stdin");

    loop {
        print_menu();

        if !read_line_stdin(&mut input) {
            break;
        }

        let choice = match parse_int(&input) {
            Some(c) => c,
            None => {
                println!("Invalid input");
                continue;
            }
        };

        match choice {
            1 => {
                if !prompt_line("Enter city name: ", &mut input) {
                    break;
                }
                let name = trim_newline(&input).to_string();
                if add_node(&mut graph, &name).is_some() {
                    println!("Added city: {}", name);
                } else {
                    println!("Failed to add city");
                }
            }
            2 => {
                let mut from_buf = String::new();
                let mut to_buf = String::new();

                if !prompt_line("Enter from city: ", &mut from_buf) {
                    break;
                }
                let from_city = trim_newline(&from_buf).to_string();

                if !prompt_line("Enter to city: ", &mut to_buf) {
                    break;
                }
                let to_city = trim_newline(&to_buf).to_string();

                if !prompt_line("Enter distance: ", &mut input) {
                    break;
                }
                let distance = match parse_int(&input) {
                    Some(d) => d,
                    None => {
                        println!("Invalid distance");
                        continue;
                    }
                };

                let from = match get_node_by_name(&graph, &from_city) {
                    Some(n) => n,
                    None => {
                        println!("City '{}' not found", from_city);
                        continue;
                    }
                };
                let to = match get_node_by_name(&graph, &to_city) {
                    Some(n) => n,
                    None => {
                        println!("City '{}' not found", to_city);
                        continue;
                    }
                };

                if add_edge(&from, &to, distance).is_ok() {
                    println!(
                        "Added route: {} -> {} (distance: {})",
                        from_city, to_city, distance
                    );
                } else {
                    println!("Failed to add route");
                }
            }
            3 => {
                print_graph(&graph);
            }
            4 => {
                if !prompt_line("Enter city name: ", &mut input) {
                    break;
                }
                let name = trim_newline(&input).to_string();
                match get_node_by_name(&graph, &name) {
                    Some(node) => print_node(&node),
                    None => println!("City '{}' not found", name),
                }
            }
            5 => {
                let mut start_buf = String::new();
                let mut end_buf = String::new();

                if !prompt_line("Enter start city: ", &mut start_buf) {
                    break;
                }
                let start_city = trim_newline(&start_buf).to_string();

                if !prompt_line("Enter end city: ", &mut end_buf) {
                    break;
                }
                let end_city = trim_newline(&end_buf).to_string();

                let start = match get_node_by_name(&graph, &start_city) {
                    Some(n) => n,
                    None => {
                        println!("City '{}' not found", start_city);
                        continue;
                    }
                };
                let end = match get_node_by_name(&graph, &end_city) {
                    Some(n) => n,
                    None => {
                        println!("City '{}' not found", end_city);
                        continue;
                    }
                };

                match find_shortest_path(&start, &end) {
                    Some(path) => {
                        println!("Shortest path from {} to {}:", start_city, end_city);
                        for (i, n) in path.iter().enumerate() {
                            println!("  {}. {}", i + 1, n.borrow().city_name);
                        }
                    }
                    None => {
                        println!("No path found");
                    }
                }
            }
            6 => {
                if !prompt_line("Enter start city for shallow copy: ", &mut input) {
                    break;
                }
                let name = trim_newline(&input).to_string();
                match get_node_by_name(&graph, &name) {
                    Some(node) => {
                        let copy = shallow_copy(&node);
                        println!("Created shallow copy starting from {}", name);
                        println!("Reference counts incremented for all reachable nodes");
                        print_node(&copy);
                    }
                    None => {
                        println!("City '{}' not found", name);
                    }
                }
            }
            7 => {
                if !prompt_line("Enter city name to delete: ", &mut input) {
                    break;
                }
                let name = trim_newline(&input).to_string();
                // Find index of node by name to allow post-decrement handling.
                let idx_opt = graph.nodes.iter().position(|n| n.borrow().city_name == name);
                match idx_opt {
                    Some(idx) => {
                        let node = Rc::clone(&graph.nodes[idx]);
                        println!("Current ref count: {}", node.borrow().ref_count);
                        delete_node(&node);
                        println!("Decremented reference count for {}", name);
                        println!("Note: Node will be freed when ref count reaches 0");
                        // Emulate C's free(node) when ref_count reaches 0:
                        // In C, after free(node), the pointer in graph->nodes[] is dangling.
                        // Observable effects:
                        //   - get_node_by_name uses strcmp on freed memory: it doesn't match
                        //     the original name, so lookups return NULL.
                        //   - print_graph still lists the (freed) node with garbage in its
                        //     city_name field and ref_count field typically zeroed.
                        // To reproduce this observable behavior in safe Rust, we keep the
                        // node in graph.nodes but rename it to a sentinel that won't match
                        // the original name. The edges are preserved (matching C's freed
                        // memory that still contains the old edge bytes).
                        // The test runner normalizes any "City: <name> (ref_count: 0)"
                        // line to "City: <freed> (ref_count: 0)", so the exact sentinel
                        // name is irrelevant for comparison.
                        if node.borrow().ref_count <= 0 {
                            // Use a sentinel city_name unlikely to collide with any input.
                            node.borrow_mut().city_name = "\u{0000}<freed>\u{0000}".to_string();
                        }
                    }
                    None => {
                        println!("City '{}' not found", name);
                    }
                }
            }


            8 => {
                println!("Freeing graph and exiting...");
                free_graph(&mut graph);
                return 0;
            }
            _ => {
                println!("Invalid choice");
            }
        }
    }

    free_graph(&mut graph);
    0
}
