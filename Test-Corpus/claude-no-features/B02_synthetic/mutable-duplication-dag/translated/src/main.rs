// Translated from c_src/src/main.c and c_src/src/lib.c
// Reproduces the original C behavior byte-for-byte.

use std::io::{self, Read, Write};

const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;
const MAX_INPUT: usize = 256;

type NodeId = usize;

#[derive(Clone)]
struct Edge {
    destination: NodeId,
    distance: i32,
}

struct Node {
    city_name: Vec<u8>, // bytes without trailing NUL, max length MAX_CITY_NAME-1
    ref_count: i32,
    edges: Vec<Edge>,
    edge_count: i32,
}

struct Graph {
    nodes: Vec<Option<Node>>, // indexed by NodeId; None means slot was never filled
    node_ids: Vec<NodeId>,    // ids in insertion order (parallels C graph->nodes[0..node_count])
    node_count: i32,
}

impl Graph {
    fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            node_ids: Vec::new(),
            node_count: 0,
        }
    }
}

// ============== Helpers ==============

// Mimic C's strncpy(dst, src, MAX_CITY_NAME-1) followed by '\0' terminator.
// Result is the bytes (no NUL) up to MAX_CITY_NAME-1 in length.
fn truncate_city_name(src: &[u8]) -> Vec<u8> {
    let max = MAX_CITY_NAME - 1;
    let len = src.len().min(max);
    src[..len].to_vec()
}

// Mimic C fgets: read up to max-1 bytes or until '\n' (which is included).
// Returns None on EOF if nothing was read.
fn fgets_like<R: Read>(reader: &mut R, max: usize) -> Option<Vec<u8>> {
    if max < 2 {
        return Some(Vec::new());
    }
    let limit = max - 1;
    let mut buf: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while buf.len() < limit {
        match reader.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

// Equivalent to C's `input[strcspn(input, "\n")] = 0;`:
// truncate at the first '\n' if present, else leave as is.
fn strip_newline(buf: &[u8]) -> Vec<u8> {
    if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        buf[..pos].to_vec()
    } else {
        buf.to_vec()
    }
}

// Mimic sscanf(buf, "%d", &n): skip whitespace, then parse a signed decimal int.
// Returns Some(n) if a valid int was parsed, None otherwise.
fn sscanf_int(buf: &[u8]) -> Option<i32> {
    // skip C whitespace
    let mut i = 0;
    while i < buf.len() {
        match buf[i] {
            b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c => i += 1,
            _ => break,
        }
    }
    let start = i;
    if i < buf.len() && (buf[i] == b'-' || buf[i] == b'+') {
        i += 1;
    }
    let digit_start = i;
    while i < buf.len() && buf[i].is_ascii_digit() {
        i += 1;
    }
    if i == digit_start {
        return None;
    }
    // %d in C parses long-form into int; emulate wrapping behavior via i64 -> i32 cast.
    let s = std::str::from_utf8(&buf[start..i]).ok()?;
    match s.parse::<i64>() {
        Ok(v) => Some(v as i32),
        Err(_) => {
            // Out of i64 range. Match C undefined-ish behavior by clamping.
            if s.starts_with('-') {
                Some(i32::MIN)
            } else {
                Some(i32::MAX)
            }
        }
    }
}

// Helpers for printing output via locked stdout writer (avoids partial-line interleaving issues).
fn out_write(out: &mut io::StdoutLock<'_>, bytes: &[u8]) {
    let _ = out.write_all(bytes);
}

fn err_write(bytes: &[u8]) {
    let stderr = io::stderr();
    let mut h = stderr.lock();
    let _ = h.write_all(bytes);
}

// ============== Library functions (from lib.c) ==============

fn create_graph() -> Option<Graph> {
    Some(Graph::new())
}

// Add a node to the graph. Mirrors add_node in lib.c.
// On success returns NodeId. Errors go to stderr.
fn add_node(graph: &mut Graph, city_name: &[u8]) -> Option<NodeId> {
    // The C code checks: if (!graph || !city_name). graph is always non-null in caller,
    // and city_name comes from a buffer (never NULL) — but the empty-string case is allowed
    // and is what "Enter city name: " followed by Enter would produce.

    if graph.node_count as usize >= MAX_NODES {
        err_write(format!("Error: Graph is full (max {} nodes)\n", MAX_NODES).as_bytes());
        return None;
    }

    // Check if a node already exists with this name (compare strncpy-truncated form
    // because that's how C stored it).
    let trimmed = truncate_city_name(city_name);
    for &id in &graph.node_ids {
        if let Some(node) = graph.nodes[id].as_ref() {
            if node.city_name == trimmed {
                let mut msg = b"Error: Node '".to_vec();
                // C prints the *raw* incoming city_name (before strncpy), so do the same.
                msg.extend_from_slice(city_name);
                msg.extend_from_slice(b"' already exists\n");
                err_write(&msg);
                return None;
            }
        }
    }

    let node = Node {
        city_name: trimmed,
        ref_count: 1,
        edges: Vec::new(),
        edge_count: 0,
    };

    let new_id = graph.nodes.len();
    graph.nodes.push(Some(node));
    graph.node_ids.push(new_id);
    graph.node_count += 1;

    Some(new_id)
}

// Add an edge between two nodes. Mirrors add_edge in lib.c.
// Returns 0 on success, -1 on failure (errors printed to stderr).
fn add_edge(graph: &mut Graph, from: NodeId, to: NodeId, distance: i32) -> i32 {
    // Validate from-node exists and check max edges.
    {
        let from_node = match graph.nodes.get(from).and_then(|n| n.as_ref()) {
            Some(n) => n,
            None => {
                err_write(b"Error: NULL node in add_edge\n");
                return -1;
            }
        };
        if from_node.edge_count as usize >= MAX_EDGES {
            let mut msg = b"Error: Node '".to_vec();
            msg.extend_from_slice(&from_node.city_name);
            msg.extend_from_slice(b"' has maximum edges\n");
            err_write(&msg);
            return -1;
        }
    }
    // Validate to-node exists.
    if graph.nodes.get(to).and_then(|n| n.as_ref()).is_none() {
        err_write(b"Error: NULL node in add_edge\n");
        return -1;
    }

    if distance < 0 {
        err_write(b"Error: Negative distance not allowed\n");
        return -1;
    }

    // Check for duplicate edge.
    {
        let from_node = graph.nodes[from].as_ref().unwrap();
        for e in &from_node.edges {
            if e.destination == to {
                err_write(b"Error: Edge already exists\n");
                return -1;
            }
        }
    }

    // Add edge.
    let from_node = graph.nodes[from].as_mut().unwrap();
    from_node.edges.push(Edge {
        destination: to,
        distance,
    });
    from_node.edge_count += 1;

    0
}

// Decrement ref count; in safe Rust we keep nodes alive (the original frees memory
// at zero, but reproducing use-after-free is undefined behavior).
fn delete_node(graph: &mut Graph, id: NodeId) {
    if let Some(node) = graph.nodes.get_mut(id).and_then(|n| n.as_mut()) {
        node.ref_count -= 1;
        // In C: free(node) when ref_count == 0. We keep memory alive.
    }
}

// Recursively increment ref counts, tracking visited node ids.
fn increment_refs_recursive(graph: &mut Graph, id: NodeId, visited: &mut Vec<NodeId>) {
    // Check if already visited
    for &v in visited.iter() {
        if v == id {
            return;
        }
    }
    if visited.len() < MAX_NODES {
        visited.push(id);
    }

    // Increment ref count and snapshot edge destinations to recurse.
    let edge_dests: Vec<NodeId> = match graph.nodes.get_mut(id).and_then(|n| n.as_mut()) {
        Some(node) => {
            node.ref_count += 1;
            node.edges.iter().map(|e| e.destination).collect()
        }
        None => return,
    };

    for d in edge_dests {
        increment_refs_recursive(graph, d, visited);
    }
}

fn shallow_copy(graph: &mut Graph, start: NodeId) -> Option<NodeId> {
    if graph.nodes.get(start).and_then(|n| n.as_ref()).is_none() {
        err_write(b"Error: NULL node in shallow_copy\n");
        return None;
    }
    let mut visited: Vec<NodeId> = Vec::new();
    increment_refs_recursive(graph, start, &mut visited);
    Some(start)
}

// Dijkstra-like helper (mirrors find_shortest_path in lib.c).
struct DijkstraNode {
    node: NodeId,
    distance: i64, // use i64 so INT_MAX comparison is safe
    previous: Option<NodeId>,
    visited: bool,
}

const INT_MAX_AS_I64: i64 = i32::MAX as i64;

fn find_shortest_path(graph: &Graph, start: NodeId, end: NodeId) -> Option<Vec<NodeId>> {
    // Initialize state with start node.
    let mut state: Vec<DijkstraNode> = Vec::new();
    state.push(DijkstraNode {
        node: start,
        distance: 0,
        previous: None,
        visited: false,
    });

    let mut current: Option<NodeId> = Some(start);

    while let Some(cur) = current {
        // Find current in state.
        let current_idx = state.iter().position(|s| s.node == cur);
        let current_idx = match current_idx {
            Some(i) => i,
            None => break,
        };

        state[current_idx].visited = true;

        // Reached the end?
        if cur == end {
            break;
        }

        // For each edge, update distances.
        let cur_distance = state[current_idx].distance;
        let edges: Vec<(NodeId, i32)> = match graph.nodes.get(cur).and_then(|n| n.as_ref()) {
            Some(n) => n.edges.iter().map(|e| (e.destination, e.distance)).collect(),
            None => Vec::new(),
        };

        for (neighbor, edge_dist) in edges {
            let new_distance = cur_distance.saturating_add(edge_dist as i64);

            let mut neighbor_idx = state.iter().position(|s| s.node == neighbor);
            if neighbor_idx.is_none() && state.len() < MAX_NODES {
                state.push(DijkstraNode {
                    node: neighbor,
                    distance: INT_MAX_AS_I64,
                    previous: None,
                    visited: false,
                });
                neighbor_idx = Some(state.len() - 1);
            }

            if let Some(j) = neighbor_idx {
                if new_distance < state[j].distance {
                    state[j].distance = new_distance;
                    state[j].previous = Some(cur);
                }
            }
        }

        // Find next unvisited node with minimum distance.
        let mut min_distance = INT_MAX_AS_I64;
        let mut next: Option<NodeId> = None;
        for s in &state {
            if !s.visited && s.distance < min_distance {
                min_distance = s.distance;
                next = Some(s.node);
            }
        }
        current = next;
    }

    // Find end node in state.
    let end_idx = state.iter().position(|s| s.node == end);
    let end_idx = match end_idx {
        Some(i) => i,
        None => {
            err_write(b"No path found\n");
            return None;
        }
    };
    if state[end_idx].distance == INT_MAX_AS_I64 {
        err_write(b"No path found\n");
        return None;
    }

    // Reconstruct path.
    let mut path: Vec<NodeId> = Vec::new();
    let mut current_node: Option<NodeId> = Some(end);
    while let Some(cn) = current_node {
        path.push(cn);
        let csi = state.iter().position(|s| s.node == cn);
        match csi {
            Some(i) => current_node = state[i].previous,
            None => break,
        }
    }

    // Reverse.
    path.reverse();
    Some(path)
}

fn get_node_by_name(graph: &Graph, city_name: &[u8]) -> Option<NodeId> {
    // City names are stored truncated to MAX_CITY_NAME-1.
    let trimmed = truncate_city_name(city_name);
    for &id in &graph.node_ids {
        if let Some(node) = graph.nodes[id].as_ref() {
            if node.city_name == trimmed {
                return Some(id);
            }
        }
    }
    None
}

fn print_node(out: &mut io::StdoutLock<'_>, graph: &Graph, id: NodeId) {
    let node = match graph.nodes.get(id).and_then(|n| n.as_ref()) {
        Some(n) => n,
        None => {
            out_write(out, b"NULL node\n");
            return;
        }
    };
    let mut header = b"City: ".to_vec();
    header.extend_from_slice(&node.city_name);
    header.extend_from_slice(format!(" (ref_count: {})\n", node.ref_count).as_bytes());
    out_write(out, &header);
    out_write(out, b"  Edges:\n");
    for e in &node.edges {
        let dest_name = match graph.nodes.get(e.destination).and_then(|n| n.as_ref()) {
            Some(d) => d.city_name.clone(),
            None => Vec::new(),
        };
        let mut line = b"    -> ".to_vec();
        line.extend_from_slice(&dest_name);
        line.extend_from_slice(format!(" (distance: {})\n", e.distance).as_bytes());
        out_write(out, &line);
    }
}

fn print_graph(out: &mut io::StdoutLock<'_>, graph: &Graph) {
    out_write(
        out,
        format!("Graph with {} nodes:\n", graph.node_count).as_bytes(),
    );
    for &id in &graph.node_ids.clone() {
        print_node(out, graph, id);
    }
}

fn free_graph(graph: &mut Graph) {
    let ids: Vec<NodeId> = graph.node_ids.clone();
    for id in ids {
        delete_node(graph, id);
    }
}

// ============== main ==============

fn print_menu(out: &mut io::StdoutLock<'_>) {
    out_write(out, b"\n=== DAG City Route Manager ===\n");
    out_write(out, b"1. Add city (node)\n");
    out_write(out, b"2. Add route (edge)\n");
    out_write(out, b"3. Show all cities\n");
    out_write(out, b"4. Show city details\n");
    out_write(out, b"5. Find shortest path\n");
    out_write(out, b"6. Make shallow copy of subsection\n");
    out_write(out, b"7. Delete node\n");
    out_write(out, b"8. Exit\n");
    out_write(out, b"Choice: ");
}

fn main() {
    let mut graph = match create_graph() {
        Some(g) => g,
        None => {
            err_write(b"Failed to create graph\n");
            std::process::exit(1);
        }
    };

    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    out_write(&mut out, b"City Route Management System\n");
    out_write(&mut out, b"Commands are read from stdin\n");

    loop {
        print_menu(&mut out);
        let _ = out.flush();

        let line = match fgets_like(&mut stdin_lock, MAX_INPUT) {
            Some(l) => l,
            None => break,
        };

        let choice = match sscanf_int(&line) {
            Some(c) => c,
            None => {
                out_write(&mut out, b"Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => {
                // Add city
                out_write(&mut out, b"Enter city name: ");
                let _ = out.flush();
                let raw = match fgets_like(&mut stdin_lock, MAX_INPUT) {
                    Some(l) => l,
                    None => continue,
                };
                let stripped = strip_newline(&raw);

                match add_node(&mut graph, &stripped) {
                    Some(_) => {
                        let mut msg = b"Added city: ".to_vec();
                        msg.extend_from_slice(&stripped);
                        msg.push(b'\n');
                        out_write(&mut out, &msg);
                    }
                    None => {
                        out_write(&mut out, b"Failed to add city\n");
                    }
                }
            }
            2 => {
                // Add route
                out_write(&mut out, b"Enter from city: ");
                let _ = out.flush();
                let from_raw = match fgets_like(&mut stdin_lock, MAX_INPUT) {
                    Some(l) => l,
                    None => continue,
                };
                let from_city = strip_newline(&from_raw);

                out_write(&mut out, b"Enter to city: ");
                let _ = out.flush();
                let to_raw = match fgets_like(&mut stdin_lock, MAX_INPUT) {
                    Some(l) => l,
                    None => continue,
                };
                let to_city = strip_newline(&to_raw);

                out_write(&mut out, b"Enter distance: ");
                let _ = out.flush();
                let dist_raw = match fgets_like(&mut stdin_lock, MAX_INPUT) {
                    Some(l) => l,
                    None => continue,
                };
                let distance = match sscanf_int(&dist_raw) {
                    Some(d) => d,
                    None => {
                        out_write(&mut out, b"Invalid distance\n");
                        continue;
                    }
                };

                let from_id = get_node_by_name(&graph, &from_city);
                let to_id = get_node_by_name(&graph, &to_city);

                if from_id.is_none() {
                    let mut msg = b"City '".to_vec();
                    msg.extend_from_slice(&from_city);
                    msg.extend_from_slice(b"' not found\n");
                    out_write(&mut out, &msg);
                    continue;
                }
                if to_id.is_none() {
                    let mut msg = b"City '".to_vec();
                    msg.extend_from_slice(&to_city);
                    msg.extend_from_slice(b"' not found\n");
                    out_write(&mut out, &msg);
                    continue;
                }

                let from_id = from_id.unwrap();
                let to_id = to_id.unwrap();

                if add_edge(&mut graph, from_id, to_id, distance) == 0 {
                    let mut msg = b"Added route: ".to_vec();
                    msg.extend_from_slice(&from_city);
                    msg.extend_from_slice(b" -> ");
                    msg.extend_from_slice(&to_city);
                    msg.extend_from_slice(format!(" (distance: {})\n", distance).as_bytes());
                    out_write(&mut out, &msg);
                } else {
                    out_write(&mut out, b"Failed to add route\n");
                }
            }
            3 => {
                // Show all cities
                print_graph(&mut out, &graph);
            }
            4 => {
                // Show city details
                out_write(&mut out, b"Enter city name: ");
                let _ = out.flush();
                let raw = match fgets_like(&mut stdin_lock, MAX_INPUT) {
                    Some(l) => l,
                    None => continue,
                };
                let name = strip_newline(&raw);

                match get_node_by_name(&graph, &name) {
                    Some(id) => print_node(&mut out, &graph, id),
                    None => {
                        let mut msg = b"City '".to_vec();
                        msg.extend_from_slice(&name);
                        msg.extend_from_slice(b"' not found\n");
                        out_write(&mut out, &msg);
                    }
                }
            }
            5 => {
                // Find shortest path
                out_write(&mut out, b"Enter start city: ");
                let _ = out.flush();
                let start_raw = match fgets_like(&mut stdin_lock, MAX_INPUT) {
                    Some(l) => l,
                    None => continue,
                };
                let start_city = strip_newline(&start_raw);

                out_write(&mut out, b"Enter end city: ");
                let _ = out.flush();
                let end_raw = match fgets_like(&mut stdin_lock, MAX_INPUT) {
                    Some(l) => l,
                    None => continue,
                };
                let end_city = strip_newline(&end_raw);

                let start_id = get_node_by_name(&graph, &start_city);
                let end_id = get_node_by_name(&graph, &end_city);

                if start_id.is_none() {
                    let mut msg = b"City '".to_vec();
                    msg.extend_from_slice(&start_city);
                    msg.extend_from_slice(b"' not found\n");
                    out_write(&mut out, &msg);
                    continue;
                }
                if end_id.is_none() {
                    let mut msg = b"City '".to_vec();
                    msg.extend_from_slice(&end_city);
                    msg.extend_from_slice(b"' not found\n");
                    out_write(&mut out, &msg);
                    continue;
                }

                let start_id = start_id.unwrap();
                let end_id = end_id.unwrap();

                match find_shortest_path(&graph, start_id, end_id) {
                    Some(path) => {
                        let mut header = b"Shortest path from ".to_vec();
                        header.extend_from_slice(&start_city);
                        header.extend_from_slice(b" to ");
                        header.extend_from_slice(&end_city);
                        header.extend_from_slice(b":\n");
                        out_write(&mut out, &header);
                        for (i, id) in path.iter().enumerate() {
                            let name = graph.nodes[*id]
                                .as_ref()
                                .map(|n| n.city_name.clone())
                                .unwrap_or_default();
                            let mut line = format!("  {}. ", i + 1).into_bytes();
                            line.extend_from_slice(&name);
                            line.push(b'\n');
                            out_write(&mut out, &line);
                        }
                    }
                    None => {
                        out_write(&mut out, b"No path found\n");
                    }
                }
            }
            6 => {
                // Shallow copy
                out_write(&mut out, b"Enter start city for shallow copy: ");
                let _ = out.flush();
                let raw = match fgets_like(&mut stdin_lock, MAX_INPUT) {
                    Some(l) => l,
                    None => continue,
                };
                let name = strip_newline(&raw);

                let id = match get_node_by_name(&graph, &name) {
                    Some(i) => i,
                    None => {
                        let mut msg = b"City '".to_vec();
                        msg.extend_from_slice(&name);
                        msg.extend_from_slice(b"' not found\n");
                        out_write(&mut out, &msg);
                        continue;
                    }
                };

                match shallow_copy(&mut graph, id) {
                    Some(copy_id) => {
                        let mut msg = b"Created shallow copy starting from ".to_vec();
                        msg.extend_from_slice(&name);
                        msg.push(b'\n');
                        out_write(&mut out, &msg);
                        out_write(
                            &mut out,
                            b"Reference counts incremented for all reachable nodes\n",
                        );
                        print_node(&mut out, &graph, copy_id);
                    }
                    None => {
                        out_write(&mut out, b"Failed to create shallow copy\n");
                    }
                }
            }
            7 => {
                // Delete node
                out_write(&mut out, b"Enter city name to delete: ");
                let _ = out.flush();
                let raw = match fgets_like(&mut stdin_lock, MAX_INPUT) {
                    Some(l) => l,
                    None => continue,
                };
                let name = strip_newline(&raw);

                let id = match get_node_by_name(&graph, &name) {
                    Some(i) => i,
                    None => {
                        let mut msg = b"City '".to_vec();
                        msg.extend_from_slice(&name);
                        msg.extend_from_slice(b"' not found\n");
                        out_write(&mut out, &msg);
                        continue;
                    }
                };

                let rc = graph.nodes[id].as_ref().unwrap().ref_count;
                out_write(
                    &mut out,
                    format!("Current ref count: {}\n", rc).as_bytes(),
                );
                delete_node(&mut graph, id);
                let mut msg = b"Decremented reference count for ".to_vec();
                msg.extend_from_slice(&name);
                msg.push(b'\n');
                out_write(&mut out, &msg);
                out_write(
                    &mut out,
                    b"Note: Node will be freed when ref count reaches 0\n",
                );
            }
            8 => {
                // Exit
                out_write(&mut out, b"Freeing graph and exiting...\n");
                free_graph(&mut graph);
                let _ = out.flush();
                return;
            }
            _ => {
                out_write(&mut out, b"Invalid choice\n");
            }
        }
    }

    free_graph(&mut graph);
    let _ = out.flush();
}
