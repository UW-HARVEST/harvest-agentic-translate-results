use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;

const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;

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

#[no_mangle]
pub unsafe extern "C" fn create_graph() -> *mut graph_t {
    let graph = libc_malloc::<graph_t>();
    if graph.is_null() {
        eprintln!("Error: Failed to allocate graph");
        return ptr::null_mut();
    }
    (*graph).node_count = 0;
    for i in 0..MAX_NODES {
        (*graph).nodes[i] = ptr::null_mut();
    }
    graph
}

#[no_mangle]
pub unsafe extern "C" fn add_node(graph: *mut graph_t, city_name: *const c_char) -> *mut node_t {
    if graph.is_null() || city_name.is_null() {
        eprintln!("Error: NULL parameter in add_node");
        return ptr::null_mut();
    }
    let g = &mut *graph;
    if g.node_count >= MAX_NODES as c_int {
        eprintln!("Error: Graph is full (max {} nodes)", MAX_NODES);
        return ptr::null_mut();
    }
    // Check if node already exists
    for i in 0..g.node_count as usize {
        if libc_strcmp((*g.nodes[i]).city_name.as_ptr(), city_name) == 0 {
            let name = CStr::from_ptr(city_name);
            eprintln!("Error: Node '{}' already exists", name.to_string_lossy());
            return ptr::null_mut();
        }
    }
    let node = libc_malloc::<node_t>();
    if node.is_null() {
        eprintln!("Error: Failed to allocate node");
        return ptr::null_mut();
    }
    libc_strncpy((*node).city_name.as_mut_ptr(), city_name, MAX_CITY_NAME - 1);
    (*node).city_name[MAX_CITY_NAME - 1] = 0;
    (*node).ref_count = 1;
    (*node).edge_count = 0;
    g.nodes[g.node_count as usize] = node;
    g.node_count += 1;
    node
}

#[no_mangle]
pub unsafe extern "C" fn add_edge(from: *mut node_t, to: *mut node_t, distance: c_int) -> c_int {
    if from.is_null() || to.is_null() {
        eprintln!("Error: NULL node in add_edge");
        return -1;
    }
    let f = &mut *from;
    if f.edge_count >= MAX_EDGES as c_int {
        let name = CStr::from_ptr(f.city_name.as_ptr());
        eprintln!("Error: Node '{}' has maximum edges", name.to_string_lossy());
        return -1;
    }
    if distance < 0 {
        eprintln!("Error: Negative distance not allowed");
        return -1;
    }
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

#[no_mangle]
pub unsafe extern "C" fn delete_node(node: *mut node_t) {
    if node.is_null() {
        return;
    }
    (*node).ref_count -= 1;
    if (*node).ref_count == 0 {
        libc_free(node as *mut u8);
    }
}

unsafe fn increment_refs_recursive(
    node: *mut node_t,
    visited: &mut [*mut node_t; MAX_NODES],
    visited_count: &mut i32,
) {
    if node.is_null() {
        return;
    }
    for i in 0..*visited_count as usize {
        if visited[i] == node {
            return;
        }
    }
    if (*visited_count as usize) < MAX_NODES {
        visited[*visited_count as usize] = node;
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
        eprintln!("Error: NULL node in shallow_copy");
        return ptr::null_mut();
    }
    let mut visited = [ptr::null_mut::<node_t>(); MAX_NODES];
    let mut visited_count: i32 = 0;
    increment_refs_recursive(start, &mut visited, &mut visited_count);
    start
}

#[repr(C)]
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
        eprintln!("Error: NULL parameter in find_shortest_path");
        return ptr::null_mut();
    }

    let mut state: [DijkstraNode; MAX_NODES] = std::mem::zeroed();
    let mut state_count: usize = 0;

    state[0].node = start;
    state[0].distance = 0;
    state[0].previous = ptr::null_mut();
    state[0].visited = 0;
    state_count = 1;

    let mut current: *mut node_t = start;

    while !current.is_null() {
        let mut current_idx: Option<usize> = None;
        for i in 0..state_count {
            if state[i].node == current {
                current_idx = Some(i);
                break;
            }
        }
        let ci = match current_idx {
            Some(i) => i,
            None => break,
        };

        state[ci].visited = 1;

        if current == end {
            break;
        }

        let cur_dist = state[ci].distance;
        for i in 0..(*current).edge_count as usize {
            let neighbor = (*current).edges[i].destination;
            let new_distance = cur_dist + (*current).edges[i].distance;

            let mut neighbor_idx: Option<usize> = None;
            for j in 0..state_count {
                if state[j].node == neighbor {
                    neighbor_idx = Some(j);
                    break;
                }
            }

            if neighbor_idx.is_none() && state_count < MAX_NODES {
                let idx = state_count;
                state[idx].node = neighbor;
                state[idx].distance = c_int::MAX;
                state[idx].previous = ptr::null_mut();
                state[idx].visited = 0;
                state_count += 1;
                neighbor_idx = Some(idx);
            }

            if let Some(ni) = neighbor_idx {
                if new_distance < state[ni].distance {
                    state[ni].distance = new_distance;
                    state[ni].previous = current;
                }
            }
        }

        let mut min_distance = c_int::MAX;
        current = ptr::null_mut();
        for i in 0..state_count {
            if state[i].visited == 0 && state[i].distance < min_distance {
                min_distance = state[i].distance;
                current = state[i].node;
            }
        }
    }

    // Find end node in state
    let mut end_idx: Option<usize> = None;
    for i in 0..state_count {
        if state[i].node == end {
            end_idx = Some(i);
            break;
        }
    }

    match end_idx {
        None => {
            eprintln!("No path found");
            *path_length = 0;
            return ptr::null_mut();
        }
        Some(ei) => {
            if state[ei].distance == c_int::MAX {
                eprintln!("No path found");
                *path_length = 0;
                return ptr::null_mut();
            }
        }
    }

    // Reconstruct path
    let mut path: [*mut node_t; MAX_NODES] = [ptr::null_mut(); MAX_NODES];
    let mut count: usize = 0;
    let mut current_node: *mut node_t = end;

    while !current_node.is_null() {
        path[count] = current_node;
        count += 1;

        let mut found = false;
        for i in 0..state_count {
            if state[i].node == current_node {
                current_node = state[i].previous;
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }

    // Allocate result and reverse
    let result = libc_malloc_array::<*mut node_t>(count);
    if result.is_null() {
        eprintln!("Error: Failed to allocate path");
        *path_length = 0;
        return ptr::null_mut();
    }

    for i in 0..count {
        *result.add(i) = path[count - 1 - i];
    }

    *path_length = count as c_int;
    result
}

#[no_mangle]
pub unsafe extern "C" fn free_graph(graph: *mut graph_t) {
    if graph.is_null() {
        return;
    }
    for i in 0..(*graph).node_count as usize {
        delete_node((*graph).nodes[i]);
    }
    libc_free(graph as *mut u8);
}

#[no_mangle]
pub unsafe extern "C" fn get_node_by_name(
    graph: *mut graph_t,
    city_name: *const c_char,
) -> *mut node_t {
    if graph.is_null() || city_name.is_null() {
        return ptr::null_mut();
    }
    let g = &*graph;
    for i in 0..g.node_count as usize {
        if libc_strcmp((*g.nodes[i]).city_name.as_ptr(), city_name) == 0 {
            return g.nodes[i];
        }
    }
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn print_node(node: *mut node_t) {
    if node.is_null() {
        println!("NULL node");
        return;
    }
    let n = &*node;
    let name = CStr::from_ptr(n.city_name.as_ptr());
    println!(
        "City: {} (ref_count: {})",
        name.to_string_lossy(),
        n.ref_count
    );
    println!("  Edges:");
    for i in 0..n.edge_count as usize {
        let dest_name = CStr::from_ptr((*n.edges[i].destination).city_name.as_ptr());
        println!(
            "    -> {} (distance: {})",
            dest_name.to_string_lossy(),
            n.edges[i].distance
        );
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_graph(graph: *mut graph_t) {
    if graph.is_null() {
        println!("NULL graph");
        return;
    }
    let g = &*graph;
    println!("Graph with {} nodes:", g.node_count);
    for i in 0..g.node_count as usize {
        print_node(g.nodes[i]);
    }
}

// Minimal libc helpers
unsafe fn libc_malloc<T>() -> *mut T {
    extern "C" {
        fn malloc(size: usize) -> *mut c_void;
    }
    malloc(std::mem::size_of::<T>()) as *mut T
}

unsafe fn libc_malloc_array<T>(count: usize) -> *mut T {
    extern "C" {
        fn malloc(size: usize) -> *mut c_void;
    }
    malloc(std::mem::size_of::<T>() * count) as *mut T
}

unsafe fn libc_free(ptr: *mut u8) {
    extern "C" {
        fn free(ptr: *mut c_void);
    }
    free(ptr as *mut c_void);
}

unsafe fn libc_strcmp(a: *const c_char, b: *const c_char) -> c_int {
    extern "C" {
        fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    }
    strcmp(a, b)
}

unsafe fn libc_strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char {
    extern "C" {
        fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    }
    strncpy(dst, src, n)
}
