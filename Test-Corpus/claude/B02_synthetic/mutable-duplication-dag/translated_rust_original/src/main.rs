use std::io::{self, Read, Write};

const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;
const MAX_INPUT: usize = 256;

#[derive(Clone)]
struct Edge {
    destination: usize, // index into Graph::nodes
    distance: i32,
}

#[derive(Clone)]
struct Node {
    city_name: String, // up to MAX_CITY_NAME-1 bytes (null-terminator implied)
    ref_count: i32,
    edges: Vec<Edge>,
    edge_count: i32,
}

struct Graph {
    nodes: Vec<Option<Node>>, // Option to mimic potential freed slots; we never remove
    node_count: i32,
}

// ------- stdin/stdout helpers (line-buffered) -------

struct Stdin {
    buffer: Vec<u8>,
    pos: usize,
}

impl Stdin {
    fn new() -> Self {
        // Read all of stdin once into buffer; this is acceptable since
        // operations are line-based via fgets which reads up to a newline or EOF.
        let mut bytes = Vec::new();
        io::stdin().read_to_end(&mut bytes).ok();
        Self {
            buffer: bytes,
            pos: 0,
        }
    }

    /// Mimic C's fgets: read up to size-1 bytes, or until newline (kept), or EOF.
    /// Returns None on immediate EOF (no bytes read).
    fn fgets(&mut self, size: usize) -> Option<String> {
        if self.pos >= self.buffer.len() {
            return None;
        }
        let max_chars = size.saturating_sub(1);
        let start = self.pos;
        let mut end = start;
        let mut count = 0;
        while end < self.buffer.len() && count < max_chars {
            let b = self.buffer[end];
            end += 1;
            count += 1;
            if b == b'\n' {
                break;
            }
        }
        let bytes = &self.buffer[start..end];
        self.pos = end;
        // Convert bytes to a String using lossy UTF-8 conversion to keep behavior safe.
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}

fn print(s: &str) {
    let stdout = io::stdout();
    let mut h = stdout.lock();
    let _ = h.write_all(s.as_bytes());
    let _ = h.flush();
}

fn eprint(s: &str) {
    let stderr = io::stderr();
    let mut h = stderr.lock();
    let _ = h.write_all(s.as_bytes());
    let _ = h.flush();
}

// strcspn(input, "\n") equivalent: index of first '\n' or string length
fn strip_newline(s: &str) -> String {
    if let Some(idx) = s.find('\n') {
        s[..idx].to_string()
    } else {
        s.to_string()
    }
}

// sscanf("%d", &x) equivalent. Returns Some(i32) if parseable, otherwise None.
// Mimics C's behavior: skip whitespace, optional sign, then digits.
fn sscanf_int(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r' || bytes[i] == 0x0B || bytes[i] == 0x0C) {
        i += 1;
    }
    let mut sign: i64 = 1;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let digit_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == digit_start {
        return None;
    }
    let digits = std::str::from_utf8(&bytes[digit_start..i]).ok()?;
    let val: i64 = digits.parse().ok()?;
    let result = sign * val;
    // Saturate/truncate like C int overflow undefined; just use as i32.
    Some(result as i32)
}

// Truncate string to MAX_CITY_NAME-1 bytes (mirrors strncpy + null term).
fn truncate_city(s: &str) -> String {
    let max = MAX_CITY_NAME - 1;
    let bytes = s.as_bytes();
    if bytes.len() <= max {
        s.to_string()
    } else {
        // Truncate by bytes; may split a UTF-8 char, but matches C behavior.
        // Use lossy decode to keep it valid UTF-8 in our String.
        String::from_utf8_lossy(&bytes[..max]).into_owned()
    }
}

// ------- DAG library functions -------

fn create_graph() -> Option<Graph> {
    Some(Graph {
        nodes: Vec::with_capacity(MAX_NODES),
        node_count: 0,
    })
}

fn add_node(graph: &mut Graph, city_name: &str) -> Option<usize> {
    if graph.node_count as usize >= MAX_NODES {
        eprint(&format!("Error: Graph is full (max {} nodes)\n", MAX_NODES));
        return None;
    }

    // Check duplicate
    for i in 0..graph.node_count as usize {
        if let Some(n) = &graph.nodes[i] {
            if n.city_name == city_name {
                eprint(&format!("Error: Node '{}' already exists\n", city_name));
                return None;
            }
        }
    }

    let new_node = Node {
        city_name: truncate_city(city_name),
        ref_count: 1,
        edges: Vec::with_capacity(MAX_EDGES),
        edge_count: 0,
    };

    let idx = graph.node_count as usize;
    if graph.nodes.len() <= idx {
        graph.nodes.push(Some(new_node));
    } else {
        graph.nodes[idx] = Some(new_node);
    }
    graph.node_count += 1;
    Some(idx)
}

fn add_edge(graph: &mut Graph, from_idx: usize, to_idx: usize, distance: i32) -> i32 {
    // Check if from has too many edges
    let from_edge_count = graph.nodes[from_idx].as_ref().unwrap().edge_count;
    if from_edge_count as usize >= MAX_EDGES {
        let from_name = graph.nodes[from_idx].as_ref().unwrap().city_name.clone();
        eprint(&format!("Error: Node '{}' has maximum edges\n", from_name));
        return -1;
    }

    if distance < 0 {
        eprint("Error: Negative distance not allowed\n");
        return -1;
    }

    // Duplicate edge check
    let from = graph.nodes[from_idx].as_ref().unwrap();
    for i in 0..from.edge_count as usize {
        if from.edges[i].destination == to_idx {
            eprint("Error: Edge already exists\n");
            return -1;
        }
    }

    let from_mut = graph.nodes[from_idx].as_mut().unwrap();
    from_mut.edges.push(Edge {
        destination: to_idx,
        distance,
    });
    from_mut.edge_count += 1;
    0
}

fn delete_node(graph: &mut Graph, idx: usize) {
    if let Some(n) = graph.nodes[idx].as_mut() {
        n.ref_count -= 1;
        // We don't actually free the node to keep references valid;
        // the printed ref_count still mirrors C's behavior.
    }
}

fn increment_refs_recursive(
    graph: &mut Graph,
    idx: usize,
    visited: &mut Vec<usize>,
) {
    for &v in visited.iter() {
        if v == idx {
            return;
        }
    }
    if visited.len() < MAX_NODES {
        visited.push(idx);
    }

    if let Some(n) = graph.nodes[idx].as_mut() {
        n.ref_count += 1;
    }

    let edges: Vec<usize> = graph.nodes[idx]
        .as_ref()
        .map(|n| n.edges[..n.edge_count as usize].iter().map(|e| e.destination).collect())
        .unwrap_or_default();

    for dest in edges {
        increment_refs_recursive(graph, dest, visited);
    }
}

fn shallow_copy(graph: &mut Graph, start: usize) -> Option<usize> {
    let mut visited: Vec<usize> = Vec::with_capacity(MAX_NODES);
    increment_refs_recursive(graph, start, &mut visited);
    Some(start)
}

#[derive(Clone)]
struct DijkstraNode {
    node_idx: usize,
    distance: i32,
    previous: Option<usize>,
    visited: bool,
}

fn find_shortest_path(graph: &Graph, start: usize, end: usize) -> Option<Vec<usize>> {
    let mut state: Vec<DijkstraNode> = Vec::with_capacity(MAX_NODES);
    state.push(DijkstraNode {
        node_idx: start,
        distance: 0,
        previous: None,
        visited: false,
    });

    let mut current: Option<usize> = Some(start);

    while let Some(curr_idx) = current {
        let mut current_state_idx: Option<usize> = None;
        for (i, st) in state.iter().enumerate() {
            if st.node_idx == curr_idx {
                current_state_idx = Some(i);
                break;
            }
        }
        let cs_idx = match current_state_idx {
            Some(v) => v,
            None => break,
        };

        state[cs_idx].visited = true;

        if curr_idx == end {
            break;
        }

        // Process neighbors
        let cur_dist = state[cs_idx].distance;
        let edges: Vec<(usize, i32)> = {
            let n = graph.nodes[curr_idx].as_ref().unwrap();
            (0..n.edge_count as usize)
                .map(|i| (n.edges[i].destination, n.edges[i].distance))
                .collect()
        };

        for (neighbor, edge_dist) in edges {
            let new_distance = cur_dist.wrapping_add(edge_dist);

            let mut neighbor_idx: Option<usize> = None;
            for (j, st) in state.iter().enumerate() {
                if st.node_idx == neighbor {
                    neighbor_idx = Some(j);
                    break;
                }
            }

            if neighbor_idx.is_none() && state.len() < MAX_NODES {
                state.push(DijkstraNode {
                    node_idx: neighbor,
                    distance: i32::MAX,
                    previous: None,
                    visited: false,
                });
                neighbor_idx = Some(state.len() - 1);
            }

            if let Some(ni) = neighbor_idx {
                if new_distance < state[ni].distance {
                    state[ni].distance = new_distance;
                    state[ni].previous = Some(curr_idx);
                }
            }
        }

        // Find next unvisited with min distance
        let mut min_dist = i32::MAX;
        current = None;
        for st in state.iter() {
            if !st.visited && st.distance < min_dist {
                min_dist = st.distance;
                current = Some(st.node_idx);
            }
        }
    }

    let mut end_state: Option<usize> = None;
    for (i, st) in state.iter().enumerate() {
        if st.node_idx == end {
            end_state = Some(i);
            break;
        }
    }

    let end_idx = match end_state {
        Some(i) if state[i].distance != i32::MAX => i,
        _ => {
            eprint("No path found\n");
            return None;
        }
    };
    let _ = end_idx;

    // Reconstruct path
    let mut path: Vec<usize> = Vec::new();
    let mut curr_node: Option<usize> = Some(end);
    while let Some(cn) = curr_node {
        path.push(cn);
        let mut found_idx: Option<usize> = None;
        for (i, st) in state.iter().enumerate() {
            if st.node_idx == cn {
                found_idx = Some(i);
                break;
            }
        }
        match found_idx {
            Some(i) => curr_node = state[i].previous,
            None => break,
        }
    }

    path.reverse();
    Some(path)
}

fn get_node_by_name(graph: &Graph, city_name: &str) -> Option<usize> {
    for i in 0..graph.node_count as usize {
        if let Some(n) = &graph.nodes[i] {
            if n.city_name == city_name {
                return Some(i);
            }
        }
    }
    None
}

fn print_node(graph: &Graph, idx: usize) {
    let n = match graph.nodes[idx].as_ref() {
        Some(v) => v,
        None => {
            print("NULL node\n");
            return;
        }
    };
    print(&format!(
        "City: {} (ref_count: {})\n",
        n.city_name, n.ref_count
    ));
    print("  Edges:\n");
    for i in 0..n.edge_count as usize {
        let dest = &graph.nodes[n.edges[i].destination].as_ref().unwrap().city_name;
        print(&format!(
            "    -> {} (distance: {})\n",
            dest, n.edges[i].distance
        ));
    }
}

fn print_graph(graph: &Graph) {
    print(&format!("Graph with {} nodes:\n", graph.node_count));
    for i in 0..graph.node_count as usize {
        print_node(graph, i);
    }
}

fn free_graph(graph: &mut Graph) {
    for i in 0..graph.node_count as usize {
        delete_node(graph, i);
    }
}

fn print_menu() {
    print("\n=== DAG City Route Manager ===\n");
    print("1. Add city (node)\n");
    print("2. Add route (edge)\n");
    print("3. Show all cities\n");
    print("4. Show city details\n");
    print("5. Find shortest path\n");
    print("6. Make shallow copy of subsection\n");
    print("7. Delete node\n");
    print("8. Exit\n");
    print("Choice: ");
}

fn main() {
    let mut graph = match create_graph() {
        Some(g) => g,
        None => {
            eprint("Failed to create graph\n");
            std::process::exit(1);
        }
    };

    let mut stdin = Stdin::new();

    print("City Route Management System\n");
    print("Commands are read from stdin\n");

    loop {
        print_menu();

        let input = match stdin.fgets(MAX_INPUT) {
            Some(s) => s,
            None => break,
        };

        let choice = match sscanf_int(&input) {
            Some(c) => c,
            None => {
                print("Invalid input\n");
                continue;
            }
        };

        match choice {
            1 => {
                print("Enter city name: ");
                let line = match stdin.fgets(MAX_INPUT) {
                    Some(s) => s,
                    None => break,
                };
                let name = strip_newline(&line);

                match add_node(&mut graph, &name) {
                    Some(_idx) => {
                        print(&format!("Added city: {}\n", name));
                    }
                    None => {
                        print("Failed to add city\n");
                    }
                }
            }
            2 => {
                print("Enter from city: ");
                let from_line = match stdin.fgets(MAX_INPUT) {
                    Some(s) => s,
                    None => break,
                };
                let from_city = strip_newline(&from_line);

                print("Enter to city: ");
                let to_line = match stdin.fgets(MAX_INPUT) {
                    Some(s) => s,
                    None => break,
                };
                let to_city = strip_newline(&to_line);

                print("Enter distance: ");
                let dist_line = match stdin.fgets(MAX_INPUT) {
                    Some(s) => s,
                    None => break,
                };
                let distance = match sscanf_int(&dist_line) {
                    Some(v) => v,
                    None => {
                        print("Invalid distance\n");
                        continue;
                    }
                };

                let from = get_node_by_name(&graph, &from_city);
                let to = get_node_by_name(&graph, &to_city);

                if from.is_none() {
                    print(&format!("City '{}' not found\n", from_city));
                    continue;
                }
                if to.is_none() {
                    print(&format!("City '{}' not found\n", to_city));
                    continue;
                }

                let from_idx = from.unwrap();
                let to_idx = to.unwrap();

                if add_edge(&mut graph, from_idx, to_idx, distance) == 0 {
                    print(&format!(
                        "Added route: {} -> {} (distance: {})\n",
                        from_city, to_city, distance
                    ));
                } else {
                    print("Failed to add route\n");
                }
            }
            3 => {
                print_graph(&graph);
            }
            4 => {
                print("Enter city name: ");
                let line = match stdin.fgets(MAX_INPUT) {
                    Some(s) => s,
                    None => break,
                };
                let name = strip_newline(&line);

                match get_node_by_name(&graph, &name) {
                    Some(idx) => print_node(&graph, idx),
                    None => print(&format!("City '{}' not found\n", name)),
                }
            }
            5 => {
                print("Enter start city: ");
                let s_line = match stdin.fgets(MAX_INPUT) {
                    Some(s) => s,
                    None => break,
                };
                let start_city = strip_newline(&s_line);

                print("Enter end city: ");
                let e_line = match stdin.fgets(MAX_INPUT) {
                    Some(s) => s,
                    None => break,
                };
                let end_city = strip_newline(&e_line);

                let start = get_node_by_name(&graph, &start_city);
                let end = get_node_by_name(&graph, &end_city);

                if start.is_none() {
                    print(&format!("City '{}' not found\n", start_city));
                    continue;
                }
                if end.is_none() {
                    print(&format!("City '{}' not found\n", end_city));
                    continue;
                }

                let start_idx = start.unwrap();
                let end_idx = end.unwrap();

                match find_shortest_path(&graph, start_idx, end_idx) {
                    Some(path) => {
                        print(&format!(
                            "Shortest path from {} to {}:\n",
                            start_city, end_city
                        ));
                        for (i, &p_idx) in path.iter().enumerate() {
                            let n = graph.nodes[p_idx].as_ref().unwrap();
                            print(&format!("  {}. {}\n", i + 1, n.city_name));
                        }
                    }
                    None => {
                        print("No path found\n");
                    }
                }
            }
            6 => {
                print("Enter start city for shallow copy: ");
                let line = match stdin.fgets(MAX_INPUT) {
                    Some(s) => s,
                    None => break,
                };
                let name = strip_newline(&line);

                let node = get_node_by_name(&graph, &name);
                if node.is_none() {
                    print(&format!("City '{}' not found\n", name));
                    continue;
                }
                let node_idx = node.unwrap();

                let copy = shallow_copy(&mut graph, node_idx);
                if let Some(c_idx) = copy {
                    print(&format!("Created shallow copy starting from {}\n", name));
                    print("Reference counts incremented for all reachable nodes\n");
                    print_node(&graph, c_idx);
                } else {
                    print("Failed to create shallow copy\n");
                }
            }
            7 => {
                print("Enter city name to delete: ");
                let line = match stdin.fgets(MAX_INPUT) {
                    Some(s) => s,
                    None => break,
                };
                let name = strip_newline(&line);

                let node = get_node_by_name(&graph, &name);
                if node.is_none() {
                    print(&format!("City '{}' not found\n", name));
                    continue;
                }
                let idx = node.unwrap();

                let rc = graph.nodes[idx].as_ref().unwrap().ref_count;
                print(&format!("Current ref count: {}\n", rc));
                delete_node(&mut graph, idx);
                print(&format!("Decremented reference count for {}\n", name));
                print("Note: Node will be freed when ref count reaches 0\n");
            }
            8 => {
                print("Freeing graph and exiting...\n");
                free_graph(&mut graph);
                return;
            }
            _ => {
                print("Invalid choice\n");
            }
        }
    }

    free_graph(&mut graph);
}
