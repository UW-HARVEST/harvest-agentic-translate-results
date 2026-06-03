// dag_lib.rs - Rust translation of dag_lib.c

use std::ptr;

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Edge {
    pub destination: *mut Node,
    pub distance: i32,
}

#[repr(C)]
pub struct Node {
    pub city_name: [u8; MAX_CITY_NAME],
    pub ref_count: i32,
    pub edges: [Edge; MAX_EDGES],
    pub edge_count: i32,
}

#[repr(C)]
pub struct Graph {
    pub nodes: [*mut Node; MAX_NODES],
    pub node_count: i32,
}

/// Helper: read a C-style nul-terminated string from a byte array as a Rust &str
pub fn city_name_to_str(name: &[u8; MAX_CITY_NAME]) -> &str {
    let len = name.iter().position(|&b| b == 0).unwrap_or(MAX_CITY_NAME);
    std::str::from_utf8(&name[..len]).unwrap_or("")
}

/// Helper: copy a Rust string into a fixed-size byte array, nul-terminated.
fn copy_to_city_name(dest: &mut [u8; MAX_CITY_NAME], src: &str) {
    let bytes = src.as_bytes();
    let copy_len = std::cmp::min(bytes.len(), MAX_CITY_NAME - 1);
    dest[..copy_len].copy_from_slice(&bytes[..copy_len]);
    for byte in dest.iter_mut().skip(copy_len) {
        *byte = 0;
    }
}

/// Compare a city_name byte array to a Rust string
fn city_name_eq(name: &[u8; MAX_CITY_NAME], s: &str) -> bool {
    city_name_to_str(name) == s
}

// Create a new empty graph
pub fn create_graph() -> *mut Graph {
    let graph = Box::new(Graph {
        nodes: [ptr::null_mut(); MAX_NODES],
        node_count: 0,
    });
    Box::into_raw(graph)
}

// Add a node to the graph
pub fn add_node(graph: *mut Graph, city_name: &str) -> *mut Node {
    if graph.is_null() {
        eprintln!("Error: NULL parameter in add_node");
        return ptr::null_mut();
    }

    unsafe {
        let g = &mut *graph;

        if g.node_count as usize >= MAX_NODES {
            eprintln!("Error: Graph is full (max {} nodes)", MAX_NODES);
            return ptr::null_mut();
        }

        // Check if node already exists
        for i in 0..g.node_count as usize {
            let existing = &*g.nodes[i];
            if city_name_eq(&existing.city_name, city_name) {
                eprintln!("Error: Node '{}' already exists", city_name);
                return ptr::null_mut();
            }
        }

        // Allocate new node
        let mut node = Box::new(Node {
            city_name: [0u8; MAX_CITY_NAME],
            ref_count: 1,
            edges: [Edge {
                destination: ptr::null_mut(),
                distance: 0,
            }; MAX_EDGES],
            edge_count: 0,
        });

        copy_to_city_name(&mut node.city_name, city_name);

        let node_ptr = Box::into_raw(node);
        g.nodes[g.node_count as usize] = node_ptr;
        g.node_count += 1;

        node_ptr
    }
}

// Add an edge between two nodes
pub fn add_edge(from: *mut Node, to: *mut Node, distance: i32) -> i32 {
    if from.is_null() || to.is_null() {
        eprintln!("Error: NULL node in add_edge");
        return -1;
    }

    unsafe {
        let f = &mut *from;

        if f.edge_count as usize >= MAX_EDGES {
            eprintln!(
                "Error: Node '{}' has maximum edges",
                city_name_to_str(&f.city_name)
            );
            return -1;
        }

        if distance < 0 {
            eprintln!("Error: Negative distance not allowed");
            return -1;
        }

        // Check for duplicate edge
        for i in 0..f.edge_count as usize {
            if f.edges[i].destination == to {
                eprintln!("Error: Edge already exists");
                return -1;
            }
        }

        let idx = f.edge_count as usize;
        f.edges[idx].destination = to;
        f.edges[idx].distance = distance;
        f.edge_count += 1;

        0
    }
}

// Delete a node (decrement ref count, free if 0)
pub fn delete_node(node: *mut Node) {
    if node.is_null() {
        return;
    }

    unsafe {
        (*node).ref_count -= 1;
        if (*node).ref_count == 0 {
            // Free the node by reconstructing the Box
            drop(Box::from_raw(node));
        }
    }
}

// Helper function to increment ref count recursively
unsafe fn increment_refs_recursive(
    node: *mut Node,
    visited: &mut [*mut Node; MAX_NODES],
    visited_count: &mut usize,
) {
    if node.is_null() {
        return;
    }

    // Check if already visited
    for i in 0..*visited_count {
        if visited[i] == node {
            return;
        }
    }

    // Mark as visited
    if *visited_count < MAX_NODES {
        visited[*visited_count] = node;
        *visited_count += 1;
    }

    // Increment ref count
    unsafe {
        (*node).ref_count += 1;

        // Recursively process all connected nodes
        let edge_count = (*node).edge_count as usize;
        for i in 0..edge_count {
            let dest = (*node).edges[i].destination;
            increment_refs_recursive(dest, visited, visited_count);
        }
    }
}

// Create shallow copy of subsection (increments ref counts)
pub fn shallow_copy(start: *mut Node) -> *mut Node {
    if start.is_null() {
        eprintln!("Error: NULL node in shallow_copy");
        return ptr::null_mut();
    }

    let mut visited: [*mut Node; MAX_NODES] = [ptr::null_mut(); MAX_NODES];
    let mut visited_count: usize = 0;

    unsafe {
        increment_refs_recursive(start, &mut visited, &mut visited_count);
    }

    start
}

// Helper structure for shortest path algorithm
#[derive(Copy, Clone)]
struct DijkstraNode {
    node: *mut Node,
    distance: i32,
    previous: *mut Node,
    visited: i32,
}

// Find shortest path using Dijkstra's algorithm
// Returns Some(Vec<*mut Node>) if path exists, None otherwise
pub fn find_shortest_path(
    start: *mut Node,
    end: *mut Node,
) -> Option<Vec<*mut Node>> {
    if start.is_null() || end.is_null() {
        eprintln!("Error: NULL parameter in find_shortest_path");
        return None;
    }

    let mut state: Vec<DijkstraNode> = Vec::with_capacity(MAX_NODES);

    // Add start node
    state.push(DijkstraNode {
        node: start,
        distance: 0,
        previous: ptr::null_mut(),
        visited: 0,
    });

    let mut current: *mut Node = start;

    unsafe {
        while !current.is_null() {
            // Find current node in state
            let mut current_idx: i32 = -1;
            for (i, s) in state.iter().enumerate() {
                if s.node == current {
                    current_idx = i as i32;
                    break;
                }
            }

            if current_idx == -1 {
                break;
            }

            let cur_idx = current_idx as usize;
            state[cur_idx].visited = 1;

            // Check if we reached the end
            if current == end {
                break;
            }

            let edge_count = (*current).edge_count as usize;
            let cur_distance = state[cur_idx].distance;

            for i in 0..edge_count {
                let neighbor = (*current).edges[i].destination;
                let edge_distance = (*current).edges[i].distance;
                let new_distance = cur_distance + edge_distance;

                // Find or add neighbor in state
                let mut neighbor_idx: i32 = -1;
                for (j, s) in state.iter().enumerate() {
                    if s.node == neighbor {
                        neighbor_idx = j as i32;
                        break;
                    }
                }

                if neighbor_idx == -1 && state.len() < MAX_NODES {
                    neighbor_idx = state.len() as i32;
                    state.push(DijkstraNode {
                        node: neighbor,
                        distance: i32::MAX,
                        previous: ptr::null_mut(),
                        visited: 0,
                    });
                }

                if neighbor_idx != -1 {
                    let n_idx = neighbor_idx as usize;
                    if new_distance < state[n_idx].distance {
                        state[n_idx].distance = new_distance;
                        state[n_idx].previous = current;
                    }
                }
            }

            // Find next unvisited node with minimum distance
            let mut min_distance = i32::MAX;
            current = ptr::null_mut();
            for s in state.iter() {
                if s.visited == 0 && s.distance < min_distance {
                    min_distance = s.distance;
                    current = s.node;
                }
            }
        }

        // Find end node in state
        let mut end_idx: i32 = -1;
        for (i, s) in state.iter().enumerate() {
            if s.node == end {
                end_idx = i as i32;
                break;
            }
        }

        if end_idx == -1 || state[end_idx as usize].distance == i32::MAX {
            eprintln!("No path found");
            return None;
        }

        // Reconstruct path
        let mut path: Vec<*mut Node> = Vec::new();
        let mut current_node: *mut Node = end;

        while !current_node.is_null() {
            path.push(current_node);

            let mut current_state_idx: i32 = -1;
            for (i, s) in state.iter().enumerate() {
                if s.node == current_node {
                    current_state_idx = i as i32;
                    break;
                }
            }

            if current_state_idx == -1 {
                break;
            }

            current_node = state[current_state_idx as usize].previous;
        }

        // Reverse path
        path.reverse();
        Some(path)
    }
}

// Get node by city name
pub fn get_node_by_name(graph: *mut Graph, city_name: &str) -> *mut Node {
    if graph.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let g = &*graph;
        for i in 0..g.node_count as usize {
            let node = &*g.nodes[i];
            if city_name_eq(&node.city_name, city_name) {
                return g.nodes[i];
            }
        }
    }

    ptr::null_mut()
}

// Print node information
pub fn print_node(node: *mut Node) {
    if node.is_null() {
        println!("NULL node");
        return;
    }

    unsafe {
        let n = &*node;
        println!(
            "City: {} (ref_count: {})",
            city_name_to_str(&n.city_name),
            n.ref_count
        );
        println!("  Edges:");
        for i in 0..n.edge_count as usize {
            let dest = &*n.edges[i].destination;
            println!(
                "    -> {} (distance: {})",
                city_name_to_str(&dest.city_name),
                n.edges[i].distance
            );
        }
    }
}

// Print entire graph
pub fn print_graph(graph: *mut Graph) {
    if graph.is_null() {
        println!("NULL graph");
        return;
    }

    unsafe {
        let g = &*graph;
        println!("Graph with {} nodes:", g.node_count);
        for i in 0..g.node_count as usize {
            print_node(g.nodes[i]);
        }
    }
}

// Free the entire graph
pub fn free_graph(graph: *mut Graph) {
    if graph.is_null() {
        return;
    }

    unsafe {
        let g = &mut *graph;
        for i in 0..g.node_count as usize {
            delete_node(g.nodes[i]);
        }
        // Reconstruct the Box to deallocate the Graph
        drop(Box::from_raw(graph));
    }
}
