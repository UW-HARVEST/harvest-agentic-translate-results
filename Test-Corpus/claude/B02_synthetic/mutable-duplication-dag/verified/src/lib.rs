// C-ABI-compatible Rust translation of dag_lib.
//
// Layout, signatures, and behavior must match the C original exactly so that
// tests loading the C .so and the Rust .so see byte-identical results.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

#[repr(C)]
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
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn fprintf(stream: *mut u8, fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    static stderr: *mut u8;
}

// Replicate strncpy semantics: copy up to n bytes from src to dst, padding with
// zero if src shorter, and not zero-terminating if src is exactly n bytes.
unsafe fn c_strncpy(dst: *mut c_char, src: *const c_char, n: usize) {
    let mut i = 0usize;
    while i < n {
        let c = *src.add(i);
        *dst.add(i) = c;
        if c == 0 {
            break;
        }
        i += 1;
    }
    // Pad remaining with zeros (matches strncpy)
    while i < n {
        *dst.add(i) = 0;
        i += 1;
    }
}

unsafe fn c_strcmp(a: *const c_char, b: *const c_char) -> c_int {
    let mut i = 0usize;
    loop {
        let ca = *a.add(i) as u8;
        let cb = *b.add(i) as u8;
        if ca != cb {
            return ca as c_int - cb as c_int;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn create_graph() -> *mut graph_t {
    let g = malloc(std::mem::size_of::<graph_t>()) as *mut graph_t;
    if g.is_null() {
        fprintf(
            stderr,
            b"Error: Failed to allocate graph\n\0".as_ptr() as *const c_char,
        );
        return ptr::null_mut();
    }
    (*g).node_count = 0;
    for i in 0..MAX_NODES {
        (*g).nodes[i] = ptr::null_mut();
    }
    g
}

#[no_mangle]
pub unsafe extern "C" fn add_node(graph: *mut graph_t, city_name: *const c_char) -> *mut node_t {
    if graph.is_null() || city_name.is_null() {
        fprintf(
            stderr,
            b"Error: NULL parameter in add_node\n\0".as_ptr() as *const c_char,
        );
        return ptr::null_mut();
    }

    if (*graph).node_count >= MAX_NODES as c_int {
        fprintf(
            stderr,
            b"Error: Graph is full (max %d nodes)\n\0".as_ptr() as *const c_char,
            MAX_NODES as c_int,
        );
        return ptr::null_mut();
    }

    for i in 0..(*graph).node_count {
        let n = (*graph).nodes[i as usize];
        if c_strcmp((*n).city_name.as_ptr(), city_name) == 0 {
            fprintf(
                stderr,
                b"Error: Node '%s' already exists\n\0".as_ptr() as *const c_char,
                city_name,
            );
            return ptr::null_mut();
        }
    }

    let node = malloc(std::mem::size_of::<node_t>()) as *mut node_t;
    if node.is_null() {
        fprintf(
            stderr,
            b"Error: Failed to allocate node\n\0".as_ptr() as *const c_char,
        );
        return ptr::null_mut();
    }

    c_strncpy((*node).city_name.as_mut_ptr(), city_name, MAX_CITY_NAME - 1);
    (*node).city_name[MAX_CITY_NAME - 1] = 0;
    (*node).ref_count = 1;
    (*node).edge_count = 0;
    // Note: edges array is left uninitialized just like in C (malloc returns
    // uninitialized memory).

    let idx = (*graph).node_count as usize;
    (*graph).nodes[idx] = node;
    (*graph).node_count += 1;

    node
}

#[no_mangle]
pub unsafe extern "C" fn add_edge(from: *mut node_t, to: *mut node_t, distance: c_int) -> c_int {
    if from.is_null() || to.is_null() {
        fprintf(
            stderr,
            b"Error: NULL node in add_edge\n\0".as_ptr() as *const c_char,
        );
        return -1;
    }

    if (*from).edge_count >= MAX_EDGES as c_int {
        fprintf(
            stderr,
            b"Error: Node '%s' has maximum edges\n\0".as_ptr() as *const c_char,
            (*from).city_name.as_ptr(),
        );
        return -1;
    }

    if distance < 0 {
        fprintf(
            stderr,
            b"Error: Negative distance not allowed\n\0".as_ptr() as *const c_char,
        );
        return -1;
    }

    for i in 0..(*from).edge_count {
        if (*from).edges[i as usize].destination == to {
            fprintf(
                stderr,
                b"Error: Edge already exists\n\0".as_ptr() as *const c_char,
            );
            return -1;
        }
    }

    let idx = (*from).edge_count as usize;
    (*from).edges[idx].destination = to;
    (*from).edges[idx].distance = distance;
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
        free(node as *mut u8);
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
    for i in 0..*visited_count {
        if *visited.offset(i as isize) == node {
            return;
        }
    }
    if (*visited_count as usize) < MAX_NODES {
        *visited.offset(*visited_count as isize) = node;
        *visited_count += 1;
    }
    (*node).ref_count += 1;
    for i in 0..(*node).edge_count {
        increment_refs_recursive((*node).edges[i as usize].destination, visited, visited_count);
    }
}

#[no_mangle]
pub unsafe extern "C" fn shallow_copy(start: *mut node_t) -> *mut node_t {
    if start.is_null() {
        fprintf(
            stderr,
            b"Error: NULL node in shallow_copy\n\0".as_ptr() as *const c_char,
        );
        return ptr::null_mut();
    }
    let mut visited: [*mut node_t; MAX_NODES] = [ptr::null_mut(); MAX_NODES];
    let mut visited_count: c_int = 0;
    increment_refs_recursive(start, visited.as_mut_ptr(), &mut visited_count);
    start
}

#[repr(C)]
struct dijkstra_node_t {
    node: *mut node_t,
    distance: c_int,
    previous: *mut node_t,
    visited: c_int,
}

const INT_MAX: c_int = c_int::MAX;

#[no_mangle]
pub unsafe extern "C" fn find_shortest_path(
    start: *mut node_t,
    end: *mut node_t,
    path_length: *mut c_int,
) -> *mut *mut node_t {
    if start.is_null() || end.is_null() || path_length.is_null() {
        fprintf(
            stderr,
            b"Error: NULL parameter in find_shortest_path\n\0".as_ptr() as *const c_char,
        );
        return ptr::null_mut();
    }

    // Initialize Dijkstra state. Use uninitialized memory to mirror C's stack
    // semantics; we only ever read fields after writing them.
    let mut state: [dijkstra_node_t; MAX_NODES] = std::array::from_fn(|_| dijkstra_node_t {
        node: ptr::null_mut(),
        distance: 0,
        previous: ptr::null_mut(),
        visited: 0,
    });
    let mut state_count: c_int = 0;

    state[0].node = start;
    state[0].distance = 0;
    state[0].previous = ptr::null_mut();
    state[0].visited = 0;
    state_count += 1;

    let mut current: *mut node_t = start;

    while !current.is_null() {
        let mut current_idx: c_int = -1;
        for i in 0..state_count {
            if state[i as usize].node == current {
                current_idx = i;
                break;
            }
        }
        if current_idx == -1 {
            break;
        }
        state[current_idx as usize].visited = 1;
        if current == end {
            break;
        }
        let edge_count = (*current).edge_count;
        for i in 0..edge_count {
            let neighbor = (*current).edges[i as usize].destination;
            // C addition; can overflow.
            let new_distance =
                state[current_idx as usize].distance.wrapping_add((*current).edges[i as usize].distance);

            let mut neighbor_idx: c_int = -1;
            for j in 0..state_count {
                if state[j as usize].node == neighbor {
                    neighbor_idx = j;
                    break;
                }
            }
            if neighbor_idx == -1 && (state_count as usize) < MAX_NODES {
                neighbor_idx = state_count;
                state[state_count as usize].node = neighbor;
                state[state_count as usize].distance = INT_MAX;
                state[state_count as usize].previous = ptr::null_mut();
                state[state_count as usize].visited = 0;
                state_count += 1;
            }

            if neighbor_idx != -1 {
                let nidx = neighbor_idx as usize;
                if new_distance < state[nidx].distance {
                    state[nidx].distance = new_distance;
                    state[nidx].previous = current;
                }
            }
        }

        let mut min_distance: c_int = INT_MAX;
        current = ptr::null_mut();
        for i in 0..state_count {
            if state[i as usize].visited == 0 && state[i as usize].distance < min_distance {
                min_distance = state[i as usize].distance;
                current = state[i as usize].node;
            }
        }
    }

    let mut end_idx: c_int = -1;
    for i in 0..state_count {
        if state[i as usize].node == end {
            end_idx = i;
            break;
        }
    }
    if end_idx == -1 || state[end_idx as usize].distance == INT_MAX {
        fprintf(stderr, b"No path found\n\0".as_ptr() as *const c_char);
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
        let mut current_state_idx: c_int = -1;
        for i in 0..state_count {
            if state[i as usize].node == current_node {
                current_state_idx = i;
                break;
            }
        }
        if current_state_idx == -1 {
            break;
        }
        current_node = state[current_state_idx as usize].previous;
    }

    let result = malloc(std::mem::size_of::<*mut node_t>() * count as usize) as *mut *mut node_t;
    if result.is_null() {
        fprintf(
            stderr,
            b"Error: Failed to allocate path\n\0".as_ptr() as *const c_char,
        );
        *path_length = 0;
        return ptr::null_mut();
    }
    for i in 0..count {
        *result.offset(i as isize) = path[(count - 1 - i) as usize];
    }
    *path_length = count;
    result
}

#[no_mangle]
pub unsafe extern "C" fn get_node_by_name(
    graph: *mut graph_t,
    city_name: *const c_char,
) -> *mut node_t {
    if graph.is_null() || city_name.is_null() {
        return ptr::null_mut();
    }
    for i in 0..(*graph).node_count {
        let n = (*graph).nodes[i as usize];
        if c_strcmp((*n).city_name.as_ptr(), city_name) == 0 {
            return n;
        }
    }
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn print_node(node: *mut node_t) {
    if node.is_null() {
        printf(b"NULL node\n\0".as_ptr() as *const c_char);
        return;
    }
    printf(
        b"City: %s (ref_count: %d)\n\0".as_ptr() as *const c_char,
        (*node).city_name.as_ptr(),
        (*node).ref_count,
    );
    printf(b"  Edges:\n\0".as_ptr() as *const c_char);
    for i in 0..(*node).edge_count {
        let e = &(*node).edges[i as usize];
        printf(
            b"    -> %s (distance: %d)\n\0".as_ptr() as *const c_char,
            (*e.destination).city_name.as_ptr(),
            e.distance,
        );
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_graph(graph: *mut graph_t) {
    if graph.is_null() {
        printf(b"NULL graph\n\0".as_ptr() as *const c_char);
        return;
    }
    printf(
        b"Graph with %d nodes:\n\0".as_ptr() as *const c_char,
        (*graph).node_count,
    );
    for i in 0..(*graph).node_count {
        print_node((*graph).nodes[i as usize]);
    }
}

#[no_mangle]
pub unsafe extern "C" fn free_graph(graph: *mut graph_t) {
    if graph.is_null() {
        return;
    }
    for i in 0..(*graph).node_count {
        delete_node((*graph).nodes[i as usize]);
    }
    free(graph as *mut u8);
}

// CStr is referenced only to keep import non-warning when used by callers/tests.
#[allow(dead_code)]
fn _cstr_ref(_s: &CStr) {}
