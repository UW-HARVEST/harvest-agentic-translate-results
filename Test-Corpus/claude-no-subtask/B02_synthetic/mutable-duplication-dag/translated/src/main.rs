// Rust translation of C DAG city route manager. Produces byte-identical output to the C version.

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
    city_name: String,
    ref_count: i32,
    edges: Vec<Edge>,
}

struct Graph {
    nodes: Vec<NodeRef>,
}

fn create_graph() -> Graph {
    Graph { nodes: Vec::new() }
}

// Truncate a city name to MAX_CITY_NAME-1 bytes, matching strncpy behavior.
fn truncate_city_name(city_name: &str) -> String {
    let bytes = city_name.as_bytes();
    let take = bytes.len().min(MAX_CITY_NAME - 1);
    // For non-UTF-8 boundary cases, fall back to lossy conversion of byte slice.
    match std::str::from_utf8(&bytes[..take]) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(&bytes[..take]).into_owned(),
    }
}

fn add_node(graph: &mut Graph, city_name: &str) -> Option<NodeRef> {
    if graph.nodes.len() >= MAX_NODES {
        eprintln!("Error: Graph is full (max {} nodes)", MAX_NODES);
        return None;
    }

    for n in &graph.nodes {
        if n.borrow().city_name == city_name {
            eprintln!("Error: Node '{}' already exists", city_name);
            return None;
        }
    }

    let truncated = truncate_city_name(city_name);

    let node = Rc::new(RefCell::new(Node {
        city_name: truncated,
        ref_count: 1,
        edges: Vec::new(),
    }));

    graph.nodes.push(node.clone());
    Some(node)
}

fn add_edge(from: &NodeRef, to: &NodeRef, distance: i32) -> i32 {
    {
        let from_borrow = from.borrow();
        if from_borrow.edges.len() >= MAX_EDGES {
            eprintln!("Error: Node '{}' has maximum edges", from_borrow.city_name);
            return -1;
        }
    }

    if distance < 0 {
        eprintln!("Error: Negative distance not allowed");
        return -1;
    }

    {
        let from_borrow = from.borrow();
        for e in &from_borrow.edges {
            if Rc::ptr_eq(&e.destination, to) {
                eprintln!("Error: Edge already exists");
                return -1;
            }
        }
    }

    from.borrow_mut().edges.push(Edge {
        destination: to.clone(),
        distance,
    });

    0
}

fn delete_node(node: &NodeRef) {
    let mut n = node.borrow_mut();
    n.ref_count -= 1;
    // In Rust, Rc handles actual freeing; we only track the C ref_count value.
}

fn increment_refs_recursive(node: &NodeRef, visited: &mut Vec<NodeRef>) {
    for v in visited.iter() {
        if Rc::ptr_eq(v, node) {
            return;
        }
    }

    if visited.len() < MAX_NODES {
        visited.push(node.clone());
    }

    node.borrow_mut().ref_count += 1;

    // Collect destinations first to avoid borrow conflicts during recursion.
    let dests: Vec<NodeRef> = node
        .borrow()
        .edges
        .iter()
        .map(|e| e.destination.clone())
        .collect();

    for d in dests {
        increment_refs_recursive(&d, visited);
    }
}

fn shallow_copy(start: &NodeRef) -> Option<NodeRef> {
    let mut visited: Vec<NodeRef> = Vec::new();
    increment_refs_recursive(start, &mut visited);
    Some(start.clone())
}

// Dijkstra state entry
struct DijkstraState {
    node: NodeRef,
    distance: i32,
    previous: Option<NodeRef>,
    visited: bool,
}

fn find_shortest_path(start: &NodeRef, end: &NodeRef) -> (Option<Vec<NodeRef>>, i32) {
    let mut state: Vec<DijkstraState> = Vec::new();

    state.push(DijkstraState {
        node: start.clone(),
        distance: 0,
        previous: None,
        visited: false,
    });

    let mut current: Option<NodeRef> = Some(start.clone());

    while let Some(cur) = current.clone() {
        // find current index
        let mut current_idx: Option<usize> = None;
        for (i, s) in state.iter().enumerate() {
            if Rc::ptr_eq(&s.node, &cur) {
                current_idx = Some(i);
                break;
            }
        }

        let cur_idx = match current_idx {
            Some(i) => i,
            None => break,
        };

        state[cur_idx].visited = true;

        if Rc::ptr_eq(&cur, end) {
            break;
        }

        // Snapshot edges to avoid holding borrow while mutating state.
        let edges: Vec<(NodeRef, i32)> = cur
            .borrow()
            .edges
            .iter()
            .map(|e| (e.destination.clone(), e.distance))
            .collect();

        let cur_distance = state[cur_idx].distance;

        for (neighbor, edge_distance) in edges {
            let new_distance = cur_distance + edge_distance;

            // find or add neighbor
            let mut neighbor_idx: Option<usize> = None;
            for (j, s) in state.iter().enumerate() {
                if Rc::ptr_eq(&s.node, &neighbor) {
                    neighbor_idx = Some(j);
                    break;
                }
            }

            let nidx = match neighbor_idx {
                Some(i) => Some(i),
                None => {
                    if state.len() < MAX_NODES {
                        let idx = state.len();
                        state.push(DijkstraState {
                            node: neighbor.clone(),
                            distance: i32::MAX,
                            previous: None,
                            visited: false,
                        });
                        Some(idx)
                    } else {
                        None
                    }
                }
            };

            if let Some(ni) = nidx {
                if new_distance < state[ni].distance {
                    state[ni].distance = new_distance;
                    state[ni].previous = Some(cur.clone());
                }
            }
        }

        // find next unvisited node with minimum distance
        let mut min_distance = i32::MAX;
        let mut next: Option<NodeRef> = None;
        for s in state.iter() {
            if !s.visited && s.distance < min_distance {
                min_distance = s.distance;
                next = Some(s.node.clone());
            }
        }
        current = next;
    }

    // find end node
    let mut end_idx: Option<usize> = None;
    for (i, s) in state.iter().enumerate() {
        if Rc::ptr_eq(&s.node, end) {
            end_idx = Some(i);
            break;
        }
    }

    let end_idx = match end_idx {
        Some(i) => i,
        None => {
            eprintln!("No path found");
            return (None, 0);
        }
    };

    if state[end_idx].distance == i32::MAX {
        eprintln!("No path found");
        return (None, 0);
    }

    // reconstruct path
    let mut path: Vec<NodeRef> = Vec::new();
    let mut current_node: Option<NodeRef> = Some(end.clone());

    while let Some(cn) = current_node.clone() {
        path.push(cn.clone());

        let mut current_state_idx: Option<usize> = None;
        for (i, s) in state.iter().enumerate() {
            if Rc::ptr_eq(&s.node, &cn) {
                current_state_idx = Some(i);
                break;
            }
        }

        let csi = match current_state_idx {
            Some(i) => i,
            None => break,
        };

        current_node = state[csi].previous.clone();
    }

    let count = path.len();
    let mut result: Vec<NodeRef> = Vec::with_capacity(count);
    for i in 0..count {
        result.push(path[count - 1 - i].clone());
    }

    (Some(result), count as i32)
}

fn get_node_by_name(graph: &Graph, city_name: &str) -> Option<NodeRef> {
    for n in &graph.nodes {
        if n.borrow().city_name == city_name {
            return Some(n.clone());
        }
    }
    None
}

fn print_node(node: &NodeRef) {
    let n = node.borrow();
    print!("City: {} (ref_count: {})\n", n.city_name, n.ref_count);
    print!("  Edges:\n");
    for e in &n.edges {
        print!(
            "    -> {} (distance: {})\n",
            e.destination.borrow().city_name,
            e.distance
        );
    }
}

fn print_graph(graph: &Graph) {
    print!("Graph with {} nodes:\n", graph.nodes.len());
    for n in &graph.nodes {
        print_node(n);
    }
}

fn free_graph(graph: &mut Graph) {
    for n in &graph.nodes {
        delete_node(n);
    }
    graph.nodes.clear();
}

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
}

// fgets-like reader: reads up to MAX_INPUT-1 bytes or until newline (inclusive).
// Returns None on EOF when nothing was read.
struct StdinReader {
    inner: io::Stdin,
    buf: Vec<u8>,
    pos: usize,
}

impl StdinReader {
    fn new() -> Self {
        StdinReader {
            inner: io::stdin(),
            buf: Vec::new(),
            pos: 0,
        }
    }

    fn read_byte(&mut self) -> io::Result<Option<u8>> {
        if self.pos >= self.buf.len() {
            self.buf.clear();
            self.pos = 0;
            let mut tmp = [0u8; 4096];
            let n = self.inner.read(&mut tmp)?;
            if n == 0 {
                return Ok(None);
            }
            self.buf.extend_from_slice(&tmp[..n]);
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Ok(Some(b))
    }

    // Mimic fgets(buf, MAX_INPUT, stdin): read up to MAX_INPUT-1 bytes; stops
    // after reading a newline (newline is included). Returns None on EOF before
    // any byte is read.
    fn fgets(&mut self) -> Option<String> {
        let mut out: Vec<u8> = Vec::new();
        let max_chars = MAX_INPUT - 1; // leave room for null terminator (in C)
        loop {
            if out.len() >= max_chars {
                break;
            }
            let b = match self.read_byte() {
                Ok(Some(b)) => b,
                Ok(None) => {
                    if out.is_empty() {
                        return None;
                    } else {
                        break;
                    }
                }
                Err(_) => {
                    if out.is_empty() {
                        return None;
                    } else {
                        break;
                    }
                }
            };
            out.push(b);
            if b == b'\n' {
                break;
            }
        }
        // Mirror C string semantics — treat as a byte string (assume mostly ASCII;
        // fall back to lossy conversion otherwise).
        Some(String::from_utf8_lossy(&out).into_owned())
    }
}

// Mimic strcspn(s, "\n"): truncate at first '\n' (replacing it with terminator).
fn strip_newline(s: &str) -> String {
    match s.find('\n') {
        Some(i) => s[..i].to_string(),
        None => s.to_string(),
    }
}

// Mimic sscanf("%d", ...): skip leading whitespace, parse signed integer.
// Returns Some(value) if at least one digit was matched.
fn sscanf_int(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let mut sign: i64 = 1;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let start = i;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    if i == start {
        return None;
    }
    let digits = std::str::from_utf8(&bytes[start..i]).ok()?;
    let value: i64 = digits.parse().ok()?;
    let signed = sign.saturating_mul(value);
    // C %d on int: clamp to i32 range to match typical 32-bit int.
    let clamped = if signed > i32::MAX as i64 {
        i32::MAX
    } else if signed < i32::MIN as i64 {
        i32::MIN
    } else {
        signed as i32
    };
    Some(clamped)
}

fn flush_stdout() {
    let _ = io::stdout().flush();
}

fn main() {
    let mut graph = create_graph();

    let mut stdin = StdinReader::new();

    print!("City Route Management System\n");
    print!("Commands are read from stdin\n");

    loop {
        print_menu();
        flush_stdout();

        let input = match stdin.fgets() {
            Some(s) => s,
            None => break,
        };

        let choice = match sscanf_int(&input) {
            Some(v) => v,
            None => {
                print!("Invalid input\n");
                flush_stdout();
                continue;
            }
        };

        match choice {
            1 => {
                // Add city
                print!("Enter city name: ");
                flush_stdout();
                let raw = match stdin.fgets() {
                    Some(s) => s,
                    None => break,
                };
                let city = strip_newline(&raw);
                let node = add_node(&mut graph, &city);
                if node.is_some() {
                    print!("Added city: {}\n", city);
                } else {
                    print!("Failed to add city\n");
                }
                flush_stdout();
            }
            2 => {
                // Add route
                print!("Enter from city: ");
                flush_stdout();
                let from_raw = match stdin.fgets() {
                    Some(s) => s,
                    None => break,
                };
                let from_city = strip_newline(&from_raw);

                print!("Enter to city: ");
                flush_stdout();
                let to_raw = match stdin.fgets() {
                    Some(s) => s,
                    None => break,
                };
                let to_city = strip_newline(&to_raw);

                print!("Enter distance: ");
                flush_stdout();
                let dist_raw = match stdin.fgets() {
                    Some(s) => s,
                    None => break,
                };
                let distance = match sscanf_int(&dist_raw) {
                    Some(v) => v,
                    None => {
                        print!("Invalid distance\n");
                        flush_stdout();
                        continue;
                    }
                };

                let from = get_node_by_name(&graph, &from_city);
                let to = get_node_by_name(&graph, &to_city);

                if from.is_none() {
                    print!("City '{}' not found\n", from_city);
                    flush_stdout();
                    continue;
                }
                if to.is_none() {
                    print!("City '{}' not found\n", to_city);
                    flush_stdout();
                    continue;
                }

                let from = from.unwrap();
                let to = to.unwrap();

                if add_edge(&from, &to, distance) == 0 {
                    print!(
                        "Added route: {} -> {} (distance: {})\n",
                        from_city, to_city, distance
                    );
                } else {
                    print!("Failed to add route\n");
                }
                flush_stdout();
            }
            3 => {
                // Show all cities
                print_graph(&graph);
                flush_stdout();
            }
            4 => {
                // Show city details
                print!("Enter city name: ");
                flush_stdout();
                let raw = match stdin.fgets() {
                    Some(s) => s,
                    None => break,
                };
                let city = strip_newline(&raw);
                match get_node_by_name(&graph, &city) {
                    Some(n) => print_node(&n),
                    None => print!("City '{}' not found\n", city),
                }
                flush_stdout();
            }
            5 => {
                // Find shortest path
                print!("Enter start city: ");
                flush_stdout();
                let start_raw = match stdin.fgets() {
                    Some(s) => s,
                    None => break,
                };
                let start_city = strip_newline(&start_raw);

                print!("Enter end city: ");
                flush_stdout();
                let end_raw = match stdin.fgets() {
                    Some(s) => s,
                    None => break,
                };
                let end_city = strip_newline(&end_raw);

                let start = get_node_by_name(&graph, &start_city);
                let end = get_node_by_name(&graph, &end_city);

                if start.is_none() {
                    print!("City '{}' not found\n", start_city);
                    flush_stdout();
                    continue;
                }
                if end.is_none() {
                    print!("City '{}' not found\n", end_city);
                    flush_stdout();
                    continue;
                }

                let start = start.unwrap();
                let end = end.unwrap();

                let (path, path_length) = find_shortest_path(&start, &end);
                if let Some(p) = path {
                    print!("Shortest path from {} to {}:\n", start_city, end_city);
                    for i in 0..(path_length as usize) {
                        print!("  {}. {}\n", i + 1, p[i].borrow().city_name);
                    }
                } else {
                    print!("No path found\n");
                }
                flush_stdout();
            }
            6 => {
                // Make shallow copy
                print!("Enter start city for shallow copy: ");
                flush_stdout();
                let raw = match stdin.fgets() {
                    Some(s) => s,
                    None => break,
                };
                let city = strip_newline(&raw);

                let node = match get_node_by_name(&graph, &city) {
                    Some(n) => n,
                    None => {
                        print!("City '{}' not found\n", city);
                        flush_stdout();
                        continue;
                    }
                };

                let copy = shallow_copy(&node);
                if let Some(c) = copy {
                    print!("Created shallow copy starting from {}\n", city);
                    print!("Reference counts incremented for all reachable nodes\n");
                    print_node(&c);
                } else {
                    print!("Failed to create shallow copy\n");
                }
                flush_stdout();
            }
            7 => {
                // Delete node
                print!("Enter city name to delete: ");
                flush_stdout();
                let raw = match stdin.fgets() {
                    Some(s) => s,
                    None => break,
                };
                let city = strip_newline(&raw);

                let node = match get_node_by_name(&graph, &city) {
                    Some(n) => n,
                    None => {
                        print!("City '{}' not found\n", city);
                        flush_stdout();
                        continue;
                    }
                };

                let cur_ref = node.borrow().ref_count;
                print!("Current ref count: {}\n", cur_ref);
                delete_node(&node);
                print!("Decremented reference count for {}\n", city);
                print!("Note: Node will be freed when ref count reaches 0\n");
                flush_stdout();
            }
            8 => {
                // Exit
                print!("Freeing graph and exiting...\n");
                flush_stdout();
                free_graph(&mut graph);
                return;
            }
            _ => {
                print!("Invalid choice\n");
                flush_stdout();
            }
        }
    }

    free_graph(&mut graph);
}
