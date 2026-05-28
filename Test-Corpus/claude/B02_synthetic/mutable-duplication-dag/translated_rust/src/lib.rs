// Rust translation of dag_lib.c, exporting an identical C ABI.
//
// Layout, signatures, and behavior must match dag_lib.h / lib.c exactly so
// that callers loading either .so via libloading observe the same results.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

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
#[derive(Copy, Clone)]
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

// ---- helpers ----

unsafe fn cstr_eq_bytes(name: *const c_char, target: &CStr) -> bool {
    libc::strcmp(name, target.as_ptr()) == 0
}

unsafe fn print_stderr(msg: &str) {
    // Mirror fprintf(stderr, ...) by writing the bytes to stderr fd directly.
    let _ = libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
}

// ---- exported functions ----

#[no_mangle]
pub unsafe extern "C" fn create_graph() -> *mut graph_t {
    let layout = std::alloc::Layout::new::<graph_t>();
    let ptr = libc::malloc(layout.size()) as *mut graph_t;
    if ptr.is_null() {
        let msg = b"Error: Failed to allocate graph\n";
        libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
        return std::ptr::null_mut();
    }
    (*ptr).node_count = 0;
    for i in 0..MAX_NODES {
        (*ptr).nodes[i] = std::ptr::null_mut();
    }
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn add_node(graph: *mut graph_t, city_name: *const c_char) -> *mut node_t {
    if graph.is_null() || city_name.is_null() {
        let msg = b"Error: NULL parameter in add_node\n";
        libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
        return std::ptr::null_mut();
    }
    if (*graph).node_count as usize >= MAX_NODES {
        let msg = format!("Error: Graph is full (max {} nodes)\n", MAX_NODES);
        libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
        return std::ptr::null_mut();
    }
    // Check for duplicate
    for i in 0..(*graph).node_count as usize {
        let existing = (*graph).nodes[i];
        if !existing.is_null() && libc::strcmp((*existing).city_name.as_ptr(), city_name) == 0 {
            // "Error: Node '%s' already exists\n"
            let name_cstr = CStr::from_ptr(city_name);
            let msg = format!(
                "Error: Node '{}' already exists\n",
                name_cstr.to_string_lossy()
            );
            libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
            return std::ptr::null_mut();
        }
    }

    let layout = std::alloc::Layout::new::<node_t>();
    let node = libc::malloc(layout.size()) as *mut node_t;
    if node.is_null() {
        let msg = b"Error: Failed to allocate node\n";
        libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
        return std::ptr::null_mut();
    }

    // strncpy(node->city_name, city_name, MAX_CITY_NAME - 1);
    // node->city_name[MAX_CITY_NAME-1] = '\0';
    libc::strncpy(
        (*node).city_name.as_mut_ptr(),
        city_name,
        MAX_CITY_NAME - 1,
    );
    (*node).city_name[MAX_CITY_NAME - 1] = 0;

    (*node).ref_count = 1;
    (*node).edge_count = 0;

    let idx = (*graph).node_count as usize;
    (*graph).nodes[idx] = node;
    (*graph).node_count += 1;

    node
}

#[no_mangle]
pub unsafe extern "C" fn add_edge(from: *mut node_t, to: *mut node_t, distance: c_int) -> c_int {
    if from.is_null() || to.is_null() {
        let msg = b"Error: NULL node in add_edge\n";
        libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
        return -1;
    }
    if (*from).edge_count as usize >= MAX_EDGES {
        let name = CStr::from_ptr((*from).city_name.as_ptr()).to_string_lossy();
        let msg = format!("Error: Node '{}' has maximum edges\n", name);
        libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
        return -1;
    }
    if distance < 0 {
        let msg = b"Error: Negative distance not allowed\n";
        libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
        return -1;
    }
    for i in 0..(*from).edge_count as usize {
        if (*from).edges[i].destination == to {
            let msg = b"Error: Edge already exists\n";
            libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
            return -1;
        }
    }
    let ec = (*from).edge_count as usize;
    (*from).edges[ec].destination = to;
    (*from).edges[ec].distance = distance;
    (*from).edge_count += 1;
    0
}

#[no_mangle]
pub unsafe extern "C" fn delete_node(node: *mut node_t) {
    if node.is_null() {
        return;
    }
    (*node).ref_count -= 1;
    if (*node).ref_count == 0 {
        libc::free(node as *mut libc::c_void);
    }
}

unsafe fn increment_refs_recursive(
    node: *mut node_t,
    visited: *mut *mut node_t,
    visited_count: *mut c_int,
) {
    if node.is_null() {
        return;
    }
    for i in 0..*visited_count as isize {
        if *visited.offset(i) == node {
            return;
        }
    }
    if (*visited_count as usize) < MAX_NODES {
        *visited.offset(*visited_count as isize) = node;
        *visited_count += 1;
    }
    (*node).ref_count += 1;
    for i in 0..(*node).edge_count as usize {
        increment_refs_recursive((*node).edges[i].destination, visited, visited_count);
    }
}

#[no_mangle]
pub unsafe extern "C" fn shallow_copy(start: *mut node_t) -> *mut node_t {
    if start.is_null() {
        let msg = b"Error: NULL node in shallow_copy\n";
        libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
        return std::ptr::null_mut();
    }
    let mut visited: [*mut node_t; MAX_NODES] = [std::ptr::null_mut(); MAX_NODES];
    let mut visited_count: c_int = 0;
    increment_refs_recursive(start, visited.as_mut_ptr(), &mut visited_count);
    start
}

#[repr(C)]
#[derive(Copy, Clone)]
struct DijkstraNode {
    node: *mut node_t,
    distance: c_int,
    previous: *mut node_t,
    visited: c_int,
}

#[no_mangle]
pub unsafe extern "C" fn find_shortest_path(
    start: *mut node_t,
    end: *mut node_t,
    path_length: *mut c_int,
) -> *mut *mut node_t {
    if start.is_null() || end.is_null() || path_length.is_null() {
        let msg = b"Error: NULL parameter in find_shortest_path\n";
        libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
        return std::ptr::null_mut();
    }
    let mut state: [DijkstraNode; MAX_NODES] = [DijkstraNode {
        node: std::ptr::null_mut(),
        distance: 0,
        previous: std::ptr::null_mut(),
        visited: 0,
    }; MAX_NODES];
    let mut state_count: usize = 0;

    state[state_count].node = start;
    state[state_count].distance = 0;
    state[state_count].previous = std::ptr::null_mut();
    state[state_count].visited = 0;
    state_count += 1;

    let mut current: *mut node_t = start;

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
        let cur_idx_us = current_idx as usize;
        state[cur_idx_us].visited = 1;

        if current == end {
            break;
        }

        for i in 0..(*current).edge_count as usize {
            let neighbor = (*current).edges[i].destination;
            let new_distance = state[cur_idx_us]
                .distance
                .wrapping_add((*current).edges[i].distance);

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
                state[state_count].previous = std::ptr::null_mut();
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
        current = std::ptr::null_mut();
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
        let msg = b"No path found\n";
        libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
        *path_length = 0;
        return std::ptr::null_mut();
    }

    let mut path: [*mut node_t; MAX_NODES] = [std::ptr::null_mut(); MAX_NODES];
    let mut count: usize = 0;
    let mut current_node: *mut node_t = end;

    while !current_node.is_null() {
        if count >= MAX_NODES {
            break;
        }
        path[count] = current_node;
        count += 1;
        let mut cs_idx: i32 = -1;
        for i in 0..state_count {
            if state[i].node == current_node {
                cs_idx = i as i32;
                break;
            }
        }
        if cs_idx == -1 {
            break;
        }
        current_node = state[cs_idx as usize].previous;
    }

    let result = libc::malloc(std::mem::size_of::<*mut node_t>() * count) as *mut *mut node_t;
    if result.is_null() {
        let msg = b"Error: Failed to allocate path\n";
        libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
        *path_length = 0;
        return std::ptr::null_mut();
    }

    for i in 0..count {
        *result.add(i) = path[count - 1 - i];
    }

    *path_length = count as c_int;
    result
}

#[no_mangle]
pub unsafe extern "C" fn get_node_by_name(
    graph: *mut graph_t,
    city_name: *const c_char,
) -> *mut node_t {
    if graph.is_null() || city_name.is_null() {
        return std::ptr::null_mut();
    }
    for i in 0..(*graph).node_count as usize {
        let n = (*graph).nodes[i];
        if !n.is_null() && libc::strcmp((*n).city_name.as_ptr(), city_name) == 0 {
            return n;
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn print_node(node: *mut node_t) {
    if node.is_null() {
        // printf -> stdout
        let msg = "NULL node\n";
        libc::write(1, msg.as_ptr() as *const libc::c_void, msg.len());
        return;
    }
    let name = CStr::from_ptr((*node).city_name.as_ptr()).to_string_lossy();
    let s = format!("City: {} (ref_count: {})\n", name, (*node).ref_count);
    libc::write(1, s.as_ptr() as *const libc::c_void, s.len());
    let edges_hdr = "  Edges:\n";
    libc::write(1, edges_hdr.as_ptr() as *const libc::c_void, edges_hdr.len());
    for i in 0..(*node).edge_count as usize {
        let dest = (*node).edges[i].destination;
        let dest_name = CStr::from_ptr((*dest).city_name.as_ptr()).to_string_lossy();
        let s2 = format!(
            "    -> {} (distance: {})\n",
            dest_name,
            (*node).edges[i].distance
        );
        libc::write(1, s2.as_ptr() as *const libc::c_void, s2.len());
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_graph(graph: *mut graph_t) {
    if graph.is_null() {
        let msg = "NULL graph\n";
        libc::write(1, msg.as_ptr() as *const libc::c_void, msg.len());
        return;
    }
    let s = format!("Graph with {} nodes:\n", (*graph).node_count);
    libc::write(1, s.as_ptr() as *const libc::c_void, s.len());
    for i in 0..(*graph).node_count as usize {
        print_node((*graph).nodes[i]);
    }
}

#[no_mangle]
pub unsafe extern "C" fn free_graph(graph: *mut graph_t) {
    if graph.is_null() {
        return;
    }
    for i in 0..(*graph).node_count as usize {
        delete_node((*graph).nodes[i]);
    }
    libc::free(graph as *mut libc::c_void);
}

// Silence unused-warnings helpers when only some functions are referenced.
#[allow(dead_code)]
fn _unused_helpers() {
    let _ = cstr_eq_bytes;
    let _ = print_stderr;
}
