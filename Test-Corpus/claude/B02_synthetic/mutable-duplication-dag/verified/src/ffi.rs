//! `extern "C"` surface of `c_src/src/lib.c` / `c_src/include/dag_lib.h`.
//!
//! `src/dag_lib.rs` translates the same C file for the in-process use of
//! `src/main.rs`; because the C API hands raw `node_t *` values back to its
//! callers (and `main.c` calls `free()` on the array returned by
//! `find_shortest_path()`), that arena/index based model cannot be exposed
//! across the FFI boundary.  This module therefore provides the exported
//! `create_graph` / `add_node` / ... symbols with exactly the C data layout and
//! the C allocator, so an external caller cannot tell the two libraries apart.
//!
//! Everything is a statement-for-statement translation of `lib.c`; the C
//! `printf` / `fprintf(stderr, ...)` calls go through glibc's own `stdout` /
//! `stderr` streams so buffering (fully buffered stdout, unbuffered stderr)
//! matches byte for byte.

#![allow(non_camel_case_types)]

use std::os::raw::{c_char, c_int, c_void};

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

/// Opaque `FILE`.
#[repr(C)]
pub struct FILE {
    _private: [u8; 0],
}

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn fwrite(ptr: *const c_void, size: usize, nmemb: usize, stream: *mut FILE) -> usize;
    #[link_name = "stdout"]
    static mut c_stdout: *mut FILE;
    #[link_name = "stderr"]
    static mut c_stderr: *mut FILE;
}

/// `printf("%s", ...)` equivalent (fully buffered stdout).
fn out(bytes: &[u8]) {
    unsafe {
        let stream = c_stdout;
        fwrite(bytes.as_ptr() as *const c_void, 1, bytes.len(), stream);
    }
}

/// `fprintf(stderr, "%s", ...)` equivalent (unbuffered stderr).
fn errout(bytes: &[u8]) {
    unsafe {
        let stream = c_stderr;
        fwrite(bytes.as_ptr() as *const c_void, 1, bytes.len(), stream);
    }
}

// ---------------------------------------------------------------------------
// C data layout
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
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

// ---------------------------------------------------------------------------
// C string helpers
// ---------------------------------------------------------------------------

unsafe fn cstr_len(p: *const c_char) -> usize {
    let mut n = 0usize;
    while *p.add(n) != 0 {
        n += 1;
    }
    n
}

/// `strcmp(a, b) == 0`
unsafe fn strcmp_eq(a: *const c_char, b: *const c_char) -> bool {
    let mut i = 0usize;
    loop {
        let ca = *a.add(i);
        let cb = *b.add(i);
        if ca != cb {
            return false;
        }
        if ca == 0 {
            return true;
        }
        i += 1;
    }
}

/// The bytes of a NUL terminated C string (without the terminator).
unsafe fn cstr_bytes<'a>(p: *const c_char) -> &'a [u8] {
    std::slice::from_raw_parts(p as *const u8, cstr_len(p))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// `create_graph()`
#[no_mangle]
pub unsafe extern "C" fn create_graph() -> *mut graph_t {
    let graph = malloc(std::mem::size_of::<graph_t>()) as *mut graph_t;
    if graph.is_null() {
        errout(b"Error: Failed to allocate graph\n");
        return std::ptr::null_mut();
    }

    (*graph).node_count = 0;
    for i in 0..MAX_NODES {
        (*graph).nodes[i] = std::ptr::null_mut();
    }

    graph
}

/// `add_node()`
#[no_mangle]
pub unsafe extern "C" fn add_node(graph: *mut graph_t, city_name: *const c_char) -> *mut node_t {
    if graph.is_null() || city_name.is_null() {
        errout(b"Error: NULL parameter in add_node\n");
        return std::ptr::null_mut();
    }

    if (*graph).node_count >= MAX_NODES as c_int {
        errout(format!("Error: Graph is full (max {} nodes)\n", MAX_NODES).as_bytes());
        return std::ptr::null_mut();
    }

    // Check if node already exists
    for i in 0..(*graph).node_count as usize {
        let existing = (*graph).nodes[i];
        if strcmp_eq((*existing).city_name.as_ptr(), city_name) {
            let mut msg = Vec::new();
            msg.extend_from_slice(b"Error: Node '");
            msg.extend_from_slice(cstr_bytes(city_name));
            msg.extend_from_slice(b"' already exists\n");
            errout(&msg);
            return std::ptr::null_mut();
        }
    }

    // Allocate new node
    let node = malloc(std::mem::size_of::<node_t>()) as *mut node_t;
    if node.is_null() {
        errout(b"Error: Failed to allocate node\n");
        return std::ptr::null_mut();
    }

    // Initialize node:
    //   strncpy(node->city_name, city_name, MAX_CITY_NAME - 1);
    //   node->city_name[MAX_CITY_NAME - 1] = '\0';
    // `strncpy` zero-pads the destination up to MAX_CITY_NAME - 1 bytes.
    let mut copied = 0usize;
    while copied < MAX_CITY_NAME - 1 {
        let c = *city_name.add(copied);
        if c == 0 {
            break;
        }
        (*node).city_name[copied] = c;
        copied += 1;
    }
    for i in copied..MAX_CITY_NAME {
        (*node).city_name[i] = 0;
    }
    (*node).ref_count = 1;
    (*node).edge_count = 0;

    // Add to graph
    let idx = (*graph).node_count as usize;
    (*graph).nodes[idx] = node;
    (*graph).node_count += 1;

    node
}

/// `add_edge()`
#[no_mangle]
pub unsafe extern "C" fn add_edge(from: *mut node_t, to: *mut node_t, distance: c_int) -> c_int {
    if from.is_null() || to.is_null() {
        errout(b"Error: NULL node in add_edge\n");
        return -1;
    }

    if (*from).edge_count >= MAX_EDGES as c_int {
        let mut msg = Vec::new();
        msg.extend_from_slice(b"Error: Node '");
        msg.extend_from_slice(cstr_bytes((*from).city_name.as_ptr()));
        msg.extend_from_slice(b"' has maximum edges\n");
        errout(&msg);
        return -1;
    }

    if distance < 0 {
        errout(b"Error: Negative distance not allowed\n");
        return -1;
    }

    // Check for duplicate edge
    for i in 0..(*from).edge_count as usize {
        if (*from).edges[i].destination == to {
            errout(b"Error: Edge already exists\n");
            return -1;
        }
    }

    // Add edge
    let slot = (*from).edge_count as usize;
    (*from).edges[slot].destination = to;
    (*from).edges[slot].distance = distance;
    (*from).edge_count += 1;

    0
}

/// `delete_node()`
#[no_mangle]
pub unsafe extern "C" fn delete_node(node: *mut node_t) {
    if node.is_null() {
        return;
    }

    (*node).ref_count = (*node).ref_count.wrapping_sub(1);

    if (*node).ref_count == 0 {
        free(node as *mut c_void);
    }
}

/// `increment_refs_recursive()` (file local in the C, not exported)
unsafe fn increment_refs_recursive(
    node: *mut node_t,
    visited: *mut *mut node_t,
    visited_count: *mut c_int,
) {
    if node.is_null() {
        return;
    }

    // Check if already visited
    for i in 0..*visited_count as usize {
        if *visited.add(i) == node {
            return;
        }
    }

    // Mark as visited
    if *visited_count < MAX_NODES as c_int {
        *visited.add(*visited_count as usize) = node;
        *visited_count += 1;
    }

    // Increment ref count
    (*node).ref_count = (*node).ref_count.wrapping_add(1);

    // Recursively process all connected nodes
    for i in 0..(*node).edge_count as usize {
        increment_refs_recursive((*node).edges[i].destination, visited, visited_count);
    }
}

/// `shallow_copy()`
#[no_mangle]
pub unsafe extern "C" fn shallow_copy(start: *mut node_t) -> *mut node_t {
    if start.is_null() {
        errout(b"Error: NULL node in shallow_copy\n");
        return std::ptr::null_mut();
    }

    // Track visited nodes to avoid cycles
    let mut visited: [*mut node_t; MAX_NODES] = [std::ptr::null_mut(); MAX_NODES];
    let mut visited_count: c_int = 0;

    // Increment ref counts for all reachable nodes
    increment_refs_recursive(start, visited.as_mut_ptr(), &mut visited_count);

    start
}

/// `dijkstra_node_t`
#[derive(Clone, Copy)]
struct dijkstra_node_t {
    node: *mut node_t,
    distance: c_int,
    previous: *mut node_t,
    visited: c_int,
}

/// `find_shortest_path()`
#[no_mangle]
pub unsafe extern "C" fn find_shortest_path(
    start: *mut node_t,
    end: *mut node_t,
    path_length: *mut c_int,
) -> *mut *mut node_t {
    if start.is_null() || end.is_null() || path_length.is_null() {
        errout(b"Error: NULL parameter in find_shortest_path\n");
        return std::ptr::null_mut();
    }

    // Initialize Dijkstra state
    let mut state: [dijkstra_node_t; MAX_NODES] = [dijkstra_node_t {
        node: std::ptr::null_mut(),
        distance: 0,
        previous: std::ptr::null_mut(),
        visited: 0,
    }; MAX_NODES];
    let mut state_count: usize = 0;

    // Add start node
    state[state_count].node = start;
    state[state_count].distance = 0;
    state[state_count].previous = std::ptr::null_mut();
    state[state_count].visited = 0;
    state_count += 1;

    let mut current: *mut node_t = start;

    while !current.is_null() {
        // Find current node in state
        let mut current_idx: isize = -1;
        for i in 0..state_count {
            if state[i].node == current {
                current_idx = i as isize;
                break;
            }
        }

        if current_idx == -1 {
            break;
        }
        let current_idx = current_idx as usize;

        state[current_idx].visited = 1;

        // Check if we reached the end
        if current == end {
            break;
        }

        // Update distances for neighbors
        for i in 0..(*current).edge_count as usize {
            let neighbor = (*current).edges[i].destination;
            let new_distance = state[current_idx]
                .distance
                .wrapping_add((*current).edges[i].distance);

            // Find or add neighbor in state
            let mut neighbor_idx: isize = -1;
            for j in 0..state_count {
                if state[j].node == neighbor {
                    neighbor_idx = j as isize;
                    break;
                }
            }

            if neighbor_idx == -1 && state_count < MAX_NODES {
                // Add new neighbor
                neighbor_idx = state_count as isize;
                state[state_count].node = neighbor;
                state[state_count].distance = c_int::MAX;
                state[state_count].previous = std::ptr::null_mut();
                state[state_count].visited = 0;
                state_count += 1;
            }

            if neighbor_idx != -1 && new_distance < state[neighbor_idx as usize].distance {
                let ni = neighbor_idx as usize;
                state[ni].distance = new_distance;
                state[ni].previous = current;
            }
        }

        // Find next unvisited node with minimum distance
        let mut min_distance = c_int::MAX;
        current = std::ptr::null_mut();
        for i in 0..state_count {
            if state[i].visited == 0 && state[i].distance < min_distance {
                min_distance = state[i].distance;
                current = state[i].node;
            }
        }
    }

    // Find end node in state
    let mut end_idx: isize = -1;
    for i in 0..state_count {
        if state[i].node == end {
            end_idx = i as isize;
            break;
        }
    }

    if end_idx == -1 || state[end_idx as usize].distance == c_int::MAX {
        errout(b"No path found\n");
        *path_length = 0;
        return std::ptr::null_mut();
    }

    // Reconstruct path.
    //
    // `node_t *path[MAX_NODES]` is a fixed size stack array and the `previous`
    // links are only guaranteed to be acyclic while no distance computation
    // overflows, so this loop can run past the end of the array.  See the note
    // in `dag_lib.rs`: the overrun lands on the Dijkstra state array, whose
    // `node` / `previous` fields are the only ones read back.
    const STATE_SLOTS: usize = 4 * MAX_NODES;
    const COUNT_SLOT: usize = 507;

    let mut path: Vec<*mut node_t> = Vec::with_capacity(MAX_NODES);
    let mut current_node: *mut node_t = end;

    while !current_node.is_null() {
        let slot = path.len();
        path.push(current_node);

        if slot >= MAX_NODES {
            let offset = slot - MAX_NODES;
            if offset < STATE_SLOTS {
                let k = offset / 4;
                match offset % 4 {
                    0 => state[k].node = current_node,
                    2 => state[k].previous = current_node,
                    _ => {}
                }
            } else if slot >= COUNT_SLOT {
                std::process::abort();
            }
        }

        // Find previous node
        let mut current_state_idx: isize = -1;
        for i in 0..state_count {
            if state[i].node == current_node {
                current_state_idx = i as isize;
                break;
            }
        }

        if current_state_idx == -1 {
            break;
        }

        current_node = state[current_state_idx as usize].previous;
    }

    let count = path.len();
    if count > MAX_NODES + STATE_SLOTS {
        // Beyond the state array the C reads back leftover stack contents,
        // which are not reproducible.
        std::process::abort();
    }

    // Reverse path
    let result = malloc(std::mem::size_of::<*mut node_t>() * count) as *mut *mut node_t;
    if result.is_null() {
        errout(b"Error: Failed to allocate path\n");
        *path_length = 0;
        return std::ptr::null_mut();
    }

    for i in 0..count {
        *result.add(i) = path[count - 1 - i];
    }

    *path_length = count as c_int;
    result
}

/// `get_node_by_name()`
#[no_mangle]
pub unsafe extern "C" fn get_node_by_name(
    graph: *mut graph_t,
    city_name: *const c_char,
) -> *mut node_t {
    if graph.is_null() || city_name.is_null() {
        return std::ptr::null_mut();
    }

    for i in 0..(*graph).node_count as usize {
        let node = (*graph).nodes[i];
        if strcmp_eq((*node).city_name.as_ptr(), city_name) {
            return node;
        }
    }

    std::ptr::null_mut()
}

/// `print_node()`
#[no_mangle]
pub unsafe extern "C" fn print_node(node: *mut node_t) {
    if node.is_null() {
        out(b"NULL node\n");
        return;
    }

    let mut msg = Vec::new();
    msg.extend_from_slice(b"City: ");
    msg.extend_from_slice(cstr_bytes((*node).city_name.as_ptr()));
    msg.extend_from_slice(format!(" (ref_count: {})\n", (*node).ref_count).as_bytes());
    out(&msg);
    out(b"  Edges:\n");
    for i in 0..(*node).edge_count as usize {
        let edge = (*node).edges[i];
        let mut msg = Vec::new();
        msg.extend_from_slice(b"    -> ");
        msg.extend_from_slice(cstr_bytes((*edge.destination).city_name.as_ptr()));
        msg.extend_from_slice(format!(" (distance: {})\n", edge.distance).as_bytes());
        out(&msg);
    }
}

/// `print_graph()`
#[no_mangle]
pub unsafe extern "C" fn print_graph(graph: *mut graph_t) {
    if graph.is_null() {
        out(b"NULL graph\n");
        return;
    }

    out(format!("Graph with {} nodes:\n", (*graph).node_count).as_bytes());
    for i in 0..(*graph).node_count as usize {
        print_node((*graph).nodes[i]);
    }
}

/// `free_graph()`
#[no_mangle]
pub unsafe extern "C" fn free_graph(graph: *mut graph_t) {
    if graph.is_null() {
        return;
    }

    // Decrement ref count for all nodes
    for i in 0..(*graph).node_count as usize {
        delete_node((*graph).nodes[i]);
    }

    free(graph as *mut c_void);
}
