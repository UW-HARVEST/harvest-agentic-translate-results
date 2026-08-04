// Translated from dag_lib.c / dag_lib.h
use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

pub type NodeRef = Rc<RefCell<Node>>;

pub struct Edge {
    pub destination: NodeRef,
    pub distance: i32,
}

pub struct Node {
    /// Stored without trailing null. Truncated to MAX_CITY_NAME - 1 bytes.
    pub city_name: Vec<u8>,
    pub ref_count: i32,
    pub edges: Vec<Edge>,
}

pub struct Graph {
    pub nodes: Vec<NodeRef>,
}

/// Create a new empty graph.
pub fn create_graph() -> Graph {
    Graph { nodes: Vec::new() }
}

/// Add a node to the graph (returns reference to node).
pub fn add_node(graph: &mut Graph, city_name: &[u8]) -> Option<NodeRef> {
    if graph.nodes.len() >= MAX_NODES {
        eprintln!("Error: Graph is full (max {} nodes)", MAX_NODES);
        return None;
    }

    for n in &graph.nodes {
        if n.borrow().city_name == city_name {
            // fprintf(stderr, "Error: Node '%s' already exists\n", city_name);
            let mut stderr = io::stderr();
            stderr.write_all(b"Error: Node '").ok();
            stderr.write_all(city_name).ok();
            stderr.write_all(b"' already exists\n").ok();
            return None;
        }
    }

    // strncpy(node->city_name, city_name, MAX_CITY_NAME - 1);
    // node->city_name[MAX_CITY_NAME - 1] = '\0';
    let take = MAX_CITY_NAME - 1;
    let truncated: Vec<u8> = city_name.iter().take(take).copied().collect();

    let node = Rc::new(RefCell::new(Node {
        city_name: truncated,
        ref_count: 1,
        edges: Vec::new(),
    }));

    graph.nodes.push(node.clone());
    Some(node)
}

/// Add an edge between two nodes with a given distance.
/// Returns 0 on success, -1 on failure.
pub fn add_edge(from: &NodeRef, to: &NodeRef, distance: i32) -> i32 {
    {
        let from_borrow = from.borrow();
        if from_borrow.edges.len() >= MAX_EDGES {
            // fprintf(stderr, "Error: Node '%s' has maximum edges\n", from->city_name);
            let mut stderr = io::stderr();
            stderr.write_all(b"Error: Node '").ok();
            stderr.write_all(&from_borrow.city_name).ok();
            stderr.write_all(b"' has maximum edges\n").ok();
            return -1;
        }
    }

    if distance < 0 {
        eprintln!("Error: Negative distance not allowed");
        return -1;
    }

    {
        let from_borrow = from.borrow();
        for edge in &from_borrow.edges {
            if Rc::ptr_eq(&edge.destination, to) {
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

/// Delete a node (decrements ref count). The actual deallocation in C happens
/// when ref_count reaches 0. In Rust we don't actually free, since other
/// references may still hold it; the visible ref_count is tracked exactly as
/// the C original.
pub fn delete_node(node: &NodeRef) {
    let mut n = node.borrow_mut();
    n.ref_count -= 1;
    // Note: The C version frees memory when ref_count == 0, but the graph
    // still holds a pointer, leading to undefined behavior on subsequent
    // access. In Rust we keep the node alive (Rc ownership). Output that
    // depends only on print operations matches as long as inputs stay sane.
}

/// Helper function to increment ref count recursively, avoiding cycles.
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

    let children: Vec<NodeRef> = node
        .borrow()
        .edges
        .iter()
        .map(|e| e.destination.clone())
        .collect();
    for child in &children {
        increment_refs_recursive(child, visited);
    }
}

/// Create shallow copy of subsection (increments ref counts).
pub fn shallow_copy(start: &NodeRef) -> Option<NodeRef> {
    let mut visited: Vec<NodeRef> = Vec::new();
    increment_refs_recursive(start, &mut visited);
    Some(start.clone())
}

struct DijkstraNode {
    node: NodeRef,
    distance: i64, // i64 to allow INT_MAX comparisons safely
    previous: Option<NodeRef>,
    visited: bool,
}

const INT_MAX: i64 = i32::MAX as i64;

/// Find shortest path between two nodes.
/// Returns Some(path) on success (also sets path_length implicitly via Vec).
/// On failure, prints to stderr matching the C implementation.
pub fn find_shortest_path(start: &NodeRef, end: &NodeRef) -> Option<Vec<NodeRef>> {
    let mut state: Vec<DijkstraNode> = Vec::new();

    state.push(DijkstraNode {
        node: start.clone(),
        distance: 0,
        previous: None,
        visited: false,
    });

    let mut current: Option<NodeRef> = Some(start.clone());

    while let Some(cur) = current.clone() {
        let mut current_idx: i32 = -1;
        for (i, s) in state.iter().enumerate() {
            if Rc::ptr_eq(&s.node, &cur) {
                current_idx = i as i32;
                break;
            }
        }

        if current_idx == -1 {
            break;
        }

        let cur_idx = current_idx as usize;
        state[cur_idx].visited = true;

        if Rc::ptr_eq(&cur, end) {
            break;
        }

        // Update distances for neighbors
        let edges: Vec<(NodeRef, i32)> = cur
            .borrow()
            .edges
            .iter()
            .map(|e| (e.destination.clone(), e.distance))
            .collect();

        for (neighbor, edge_distance) in edges {
            let new_distance = state[cur_idx].distance + edge_distance as i64;

            // Find or add neighbor
            let mut neighbor_idx: i32 = -1;
            for (j, s) in state.iter().enumerate() {
                if Rc::ptr_eq(&s.node, &neighbor) {
                    neighbor_idx = j as i32;
                    break;
                }
            }

            if neighbor_idx == -1 && state.len() < MAX_NODES {
                neighbor_idx = state.len() as i32;
                state.push(DijkstraNode {
                    node: neighbor.clone(),
                    distance: INT_MAX,
                    previous: None,
                    visited: false,
                });
            }

            if neighbor_idx != -1 {
                let nidx = neighbor_idx as usize;
                if new_distance < state[nidx].distance {
                    state[nidx].distance = new_distance;
                    state[nidx].previous = Some(cur.clone());
                }
            }
        }

        // Find next unvisited node with minimum distance
        let mut min_distance = INT_MAX;
        let mut next: Option<NodeRef> = None;
        for s in &state {
            if !s.visited && s.distance < min_distance {
                min_distance = s.distance;
                next = Some(s.node.clone());
            }
        }
        current = next;
    }

    // Find end node in state
    let mut end_idx: i32 = -1;
    for (i, s) in state.iter().enumerate() {
        if Rc::ptr_eq(&s.node, end) {
            end_idx = i as i32;
            break;
        }
    }

    if end_idx == -1 || state[end_idx as usize].distance == INT_MAX {
        eprintln!("No path found");
        return None;
    }

    // Reconstruct path
    let mut path: Vec<NodeRef> = Vec::new();
    let mut current_node: Option<NodeRef> = Some(end.clone());

    while let Some(cn) = current_node.clone() {
        path.push(cn.clone());

        let mut current_state_idx: i32 = -1;
        for (i, s) in state.iter().enumerate() {
            if Rc::ptr_eq(&s.node, &cn) {
                current_state_idx = i as i32;
                break;
            }
        }

        if current_state_idx == -1 {
            break;
        }

        current_node = state[current_state_idx as usize].previous.clone();
    }

    // Reverse path
    let count = path.len();
    let mut result: Vec<NodeRef> = Vec::with_capacity(count);
    for i in 0..count {
        result.push(path[count - 1 - i].clone());
    }

    Some(result)
}

/// Get node by city name.
pub fn get_node_by_name(graph: &Graph, city_name: &[u8]) -> Option<NodeRef> {
    for n in &graph.nodes {
        if n.borrow().city_name == city_name {
            return Some(n.clone());
        }
    }
    None
}

/// Print node information.
pub fn print_node(node: &NodeRef) {
    let n = node.borrow();
    let mut stdout = io::stdout();
    stdout.write_all(b"City: ").ok();
    stdout.write_all(&n.city_name).ok();
    write!(stdout, " (ref_count: {})\n", n.ref_count).ok();
    stdout.write_all(b"  Edges:\n").ok();
    for e in &n.edges {
        stdout.write_all(b"    -> ").ok();
        stdout.write_all(&e.destination.borrow().city_name).ok();
        write!(stdout, " (distance: {})\n", e.distance).ok();
    }
}

/// Print entire graph.
pub fn print_graph(graph: &Graph) {
    println!("Graph with {} nodes:", graph.nodes.len());
    for n in &graph.nodes {
        print_node(n);
    }
}

/// Free the entire graph (decrement ref count for all nodes).
pub fn free_graph(graph: &mut Graph) {
    for n in &graph.nodes {
        let mut nb = n.borrow_mut();
        nb.ref_count -= 1;
    }
    graph.nodes.clear();
}
