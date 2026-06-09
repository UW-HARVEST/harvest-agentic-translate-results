#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use libc::{c_char, c_int, fprintf, free, malloc, printf, strcmp, strncpy, FILE};
use std::mem;
use std::ptr;

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct edge_t {
    pub destination: *mut node_t,
    pub distance: c_int,
}

#[repr(C)]
pub struct node_t {
    pub city_name: [c_char; MAX_CITY_NAME],
    pub ref_count: c_int,
    pub edges: [edge_t; MAX_EDGES],
    pub edge_count: c_int,
}

#[repr(C)]
pub struct graph_t {
    pub nodes: [*mut node_t; MAX_NODES],
    pub node_count: c_int,
}

extern "C" {
    static mut stderr: *mut FILE;
}

#[inline]
fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// Create a new empty graph
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_graph() -> *mut graph_t {
    let graph = malloc(mem::size_of::<graph_t>()) as *mut graph_t;
    if graph.is_null() {
        fprintf(stderr, cstr(b"Error: Failed to allocate graph\n\0"));
        return ptr::null_mut();
    }

    (*graph).node_count = 0;
    let mut i = 0;
    while i < MAX_NODES {
        (*graph).nodes[i] = ptr::null_mut();
        i += 1;
    }

    graph
}

// Add a node to the graph
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_node(graph: *mut graph_t, city_name: *const c_char) -> *mut node_t {
    if graph.is_null() || city_name.is_null() {
        fprintf(stderr, cstr(b"Error: NULL parameter in add_node\n\0"));
        return ptr::null_mut();
    }

    if (*graph).node_count >= MAX_NODES as c_int {
        fprintf(
            stderr,
            cstr(b"Error: Graph is full (max %d nodes)\n\0"),
            MAX_NODES as c_int,
        );
        return ptr::null_mut();
    }

    // Check if node already exists
    let mut i: c_int = 0;
    while i < (*graph).node_count {
        let n = (*graph).nodes[i as usize];
        if strcmp((*n).city_name.as_ptr(), city_name) == 0 {
            fprintf(
                stderr,
                cstr(b"Error: Node '%s' already exists\n\0"),
                city_name,
            );
            return ptr::null_mut();
        }
        i += 1;
    }

    // Allocate new node
    let node = malloc(mem::size_of::<node_t>()) as *mut node_t;
    if node.is_null() {
        fprintf(stderr, cstr(b"Error: Failed to allocate node\n\0"));
        return ptr::null_mut();
    }

    // Initialize node
    strncpy(
        (*node).city_name.as_mut_ptr(),
        city_name,
        MAX_CITY_NAME - 1,
    );
    (*node).city_name[MAX_CITY_NAME - 1] = 0;
    (*node).ref_count = 1;
    (*node).edge_count = 0;

    // Add to graph
    let nc = (*graph).node_count as usize;
    (*graph).nodes[nc] = node;
    (*graph).node_count += 1;

    node
}

// Add an edge between two nodes
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_edge(from: *mut node_t, to: *mut node_t, distance: c_int) -> c_int {
    if from.is_null() || to.is_null() {
        fprintf(stderr, cstr(b"Error: NULL node in add_edge\n\0"));
        return -1;
    }

    if (*from).edge_count >= MAX_EDGES as c_int {
        fprintf(
            stderr,
            cstr(b"Error: Node '%s' has maximum edges\n\0"),
            (*from).city_name.as_ptr(),
        );
        return -1;
    }

    if distance < 0 {
        fprintf(stderr, cstr(b"Error: Negative distance not allowed\n\0"));
        return -1;
    }

    // Check for duplicate edge
    let mut i: c_int = 0;
    while i < (*from).edge_count {
        if (*from).edges[i as usize].destination == to {
            fprintf(stderr, cstr(b"Error: Edge already exists\n\0"));
            return -1;
        }
        i += 1;
    }

    // Add edge
    let ec = (*from).edge_count as usize;
    (*from).edges[ec].destination = to;
    (*from).edges[ec].distance = distance;
    (*from).edge_count += 1;

    0
}

// Delete a node (decrement ref count, free if 0)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn delete_node(node: *mut node_t) {
    if node.is_null() {
        return;
    }

    (*node).ref_count -= 1;

    if (*node).ref_count == 0 {
        free(node as *mut libc::c_void);
    }
}

// Helper function to increment ref count recursively
unsafe fn increment_refs_recursive(
    node: *mut node_t,
    visited: *mut *mut node_t,
    visited_count: *mut c_int,
) {
    if node.is_null() {
        return;
    }

    // Check if already visited
    let mut i: c_int = 0;
    while i < *visited_count {
        if *visited.offset(i as isize) == node {
            return;
        }
        i += 1;
    }

    // Mark as visited
    if (*visited_count as usize) < MAX_NODES {
        *visited.offset(*visited_count as isize) = node;
        *visited_count += 1;
    }

    // Increment ref count
    (*node).ref_count += 1;

    // Recursively process all connected nodes
    let mut j: c_int = 0;
    while j < (*node).edge_count {
        increment_refs_recursive(
            (*node).edges[j as usize].destination,
            visited,
            visited_count,
        );
        j += 1;
    }
}

// Create shallow copy of subsection (increments ref counts)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shallow_copy(start: *mut node_t) -> *mut node_t {
    if start.is_null() {
        fprintf(stderr, cstr(b"Error: NULL node in shallow_copy\n\0"));
        return ptr::null_mut();
    }

    // Track visited nodes to avoid cycles
    let mut visited: [*mut node_t; MAX_NODES] = [ptr::null_mut(); MAX_NODES];
    let mut visited_count: c_int = 0;

    // Increment ref counts for all reachable nodes
    increment_refs_recursive(start, visited.as_mut_ptr(), &mut visited_count);

    start
}

// Helper structure for shortest path algorithm
#[derive(Copy, Clone)]
struct DijkstraNode {
    node: *mut node_t,
    distance: c_int,
    previous: *mut node_t,
    visited: c_int,
}

// Find shortest path using Dijkstra's algorithm
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_shortest_path(
    start: *mut node_t,
    end: *mut node_t,
    path_length: *mut c_int,
) -> *mut *mut node_t {
    if start.is_null() || end.is_null() || path_length.is_null() {
        fprintf(
            stderr,
            cstr(b"Error: NULL parameter in find_shortest_path\n\0"),
        );
        return ptr::null_mut();
    }

    // Initialize Dijkstra state
    let mut state: [DijkstraNode; MAX_NODES] = [DijkstraNode {
        node: ptr::null_mut(),
        distance: 0,
        previous: ptr::null_mut(),
        visited: 0,
    }; MAX_NODES];
    let mut state_count: c_int = 0;

    // Add start node
    state[state_count as usize].node = start;
    state[state_count as usize].distance = 0;
    state[state_count as usize].previous = ptr::null_mut();
    state[state_count as usize].visited = 0;
    state_count += 1;

    let mut current: *mut node_t = start;

    while !current.is_null() {
        // Find current node in state
        let mut current_idx: c_int = -1;
        let mut i: c_int = 0;
        while i < state_count {
            if state[i as usize].node == current {
                current_idx = i;
                break;
            }
            i += 1;
        }

        if current_idx == -1 {
            break;
        }

        state[current_idx as usize].visited = 1;

        // Check if we reached the end
        if current == end {
            break;
        }

        // Update distances for neighbors
        let mut e: c_int = 0;
        while e < (*current).edge_count {
            let neighbor = (*current).edges[e as usize].destination;
            let new_distance =
                state[current_idx as usize].distance + (*current).edges[e as usize].distance;

            // Find or add neighbor in state
            let mut neighbor_idx: c_int = -1;
            let mut j: c_int = 0;
            while j < state_count {
                if state[j as usize].node == neighbor {
                    neighbor_idx = j;
                    break;
                }
                j += 1;
            }

            if neighbor_idx == -1 && (state_count as usize) < MAX_NODES {
                // Add new neighbor
                neighbor_idx = state_count;
                state[state_count as usize].node = neighbor;
                state[state_count as usize].distance = c_int::MAX;
                state[state_count as usize].previous = ptr::null_mut();
                state[state_count as usize].visited = 0;
                state_count += 1;
            }

            if neighbor_idx != -1 && new_distance < state[neighbor_idx as usize].distance {
                state[neighbor_idx as usize].distance = new_distance;
                state[neighbor_idx as usize].previous = current;
            }

            e += 1;
        }

        // Find next unvisited node with minimum distance
        let mut min_distance = c_int::MAX;
        current = ptr::null_mut();
        let mut k: c_int = 0;
        while k < state_count {
            if state[k as usize].visited == 0 && state[k as usize].distance < min_distance {
                min_distance = state[k as usize].distance;
                current = state[k as usize].node;
            }
            k += 1;
        }
    }

    // Find end node in state
    let mut end_idx: c_int = -1;
    let mut i: c_int = 0;
    while i < state_count {
        if state[i as usize].node == end {
            end_idx = i;
            break;
        }
        i += 1;
    }

    if end_idx == -1 || state[end_idx as usize].distance == c_int::MAX {
        fprintf(stderr, cstr(b"No path found\n\0"));
        *path_length = 0;
        return ptr::null_mut();
    }

    // Reconstruct path
    let mut path: [*mut node_t; MAX_NODES] = [ptr::null_mut(); MAX_NODES];
    let mut count: c_int = 0;
    let mut current_node: *mut node_t = end;

    while !current_node.is_null() {
        path[count as usize] = current_node;
        count += 1;

        // Find previous node
        let mut current_state_idx: c_int = -1;
        let mut i: c_int = 0;
        while i < state_count {
            if state[i as usize].node == current_node {
                current_state_idx = i;
                break;
            }
            i += 1;
        }

        if current_state_idx == -1 {
            break;
        }

        current_node = state[current_state_idx as usize].previous;
    }

    // Reverse path
    let result = malloc(mem::size_of::<*mut node_t>() * count as usize) as *mut *mut node_t;
    if result.is_null() {
        fprintf(stderr, cstr(b"Error: Failed to allocate path\n\0"));
        *path_length = 0;
        return ptr::null_mut();
    }

    let mut i: c_int = 0;
    while i < count {
        *result.offset(i as isize) = path[(count - 1 - i) as usize];
        i += 1;
    }

    *path_length = count;
    result
}

// Get node by city name
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_node_by_name(
    graph: *mut graph_t,
    city_name: *const c_char,
) -> *mut node_t {
    if graph.is_null() || city_name.is_null() {
        return ptr::null_mut();
    }

    let mut i: c_int = 0;
    while i < (*graph).node_count {
        let n = (*graph).nodes[i as usize];
        if strcmp((*n).city_name.as_ptr(), city_name) == 0 {
            return n;
        }
        i += 1;
    }

    ptr::null_mut()
}

// Print node information
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_node(node: *mut node_t) {
    if node.is_null() {
        printf(cstr(b"NULL node\n\0"));
        return;
    }

    printf(
        cstr(b"City: %s (ref_count: %d)\n\0"),
        (*node).city_name.as_ptr(),
        (*node).ref_count,
    );
    printf(cstr(b"  Edges:\n\0"));
    let mut i: c_int = 0;
    while i < (*node).edge_count {
        printf(
            cstr(b"    -> %s (distance: %d)\n\0"),
            (*(*node).edges[i as usize].destination).city_name.as_ptr(),
            (*node).edges[i as usize].distance,
        );
        i += 1;
    }
}

// Print entire graph
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_graph(graph: *mut graph_t) {
    if graph.is_null() {
        printf(cstr(b"NULL graph\n\0"));
        return;
    }

    printf(
        cstr(b"Graph with %d nodes:\n\0"),
        (*graph).node_count,
    );
    let mut i: c_int = 0;
    while i < (*graph).node_count {
        print_node((*graph).nodes[i as usize]);
        i += 1;
    }
}

// Free the entire graph
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_graph(graph: *mut graph_t) {
    if graph.is_null() {
        return;
    }

    // Decrement ref count for all nodes
    let mut i: c_int = 0;
    while i < (*graph).node_count {
        delete_node((*graph).nodes[i as usize]);
        i += 1;
    }

    free(graph as *mut libc::c_void);
}
