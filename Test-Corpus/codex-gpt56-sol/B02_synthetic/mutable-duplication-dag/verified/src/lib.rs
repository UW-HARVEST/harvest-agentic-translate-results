use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;
use std::sync::atomic::{AtomicIsize, Ordering};

const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Edge {
    pub destination: *mut Node,
    pub distance: c_int,
}

#[repr(C)]
pub struct Node {
    pub city_name: [c_char; MAX_CITY_NAME],
    pub ref_count: c_int,
    pub edges: [Edge; MAX_EDGES],
    pub edge_count: c_int,
}

#[repr(C)]
pub struct Graph {
    pub nodes: [*mut Node; MAX_NODES],
    pub node_count: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DijkstraNode {
    node: *mut Node,
    distance: c_int,
    previous: *mut Node,
    visited: c_int,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn strncpy(destination: *mut c_char, source: *const c_char, count: usize) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    static mut stderr: *mut c_void;
}

static FAIL_ALLOC_AFTER: AtomicIsize = AtomicIsize::new(-1);

unsafe fn dag_malloc(size: usize) -> *mut c_void {
    let remaining = FAIL_ALLOC_AFTER.load(Ordering::Relaxed);
    if remaining == 0 {
        return ptr::null_mut();
    }
    if remaining > 0 {
        FAIL_ALLOC_AFTER.fetch_sub(1, Ordering::Relaxed);
    }
    unsafe { malloc(size) }
}

#[no_mangle]
pub extern "C" fn dag_test_fail_alloc_after(successful_allocations: isize) {
    FAIL_ALLOC_AFTER.store(successful_allocations, Ordering::Relaxed);
}

unsafe fn write_stderr(message: &'static [u8]) {
    unsafe {
        fprintf(
            stderr,
            b"%s\0".as_ptr().cast(),
            message.as_ptr().cast::<c_char>(),
        );
    }
}

#[no_mangle]
pub unsafe extern "C" fn create_graph() -> *mut Graph {
    let graph = unsafe { dag_malloc(size_of::<Graph>()).cast::<Graph>() };
    if graph.is_null() {
        unsafe { write_stderr(b"Error: Failed to allocate graph\n\0") };
        return ptr::null_mut();
    }

    unsafe {
        (*graph).node_count = 0;
        for index in 0..MAX_NODES {
            (*graph).nodes[index] = ptr::null_mut();
        }
    }
    graph
}

#[no_mangle]
pub unsafe extern "C" fn add_node(graph: *mut Graph, city_name: *const c_char) -> *mut Node {
    if graph.is_null() || city_name.is_null() {
        unsafe { write_stderr(b"Error: NULL parameter in add_node\n\0") };
        return ptr::null_mut();
    }

    if unsafe { (*graph).node_count } >= MAX_NODES as c_int {
        unsafe {
            fprintf(
                stderr,
                b"Error: Graph is full (max %d nodes)\n\0".as_ptr().cast(),
                MAX_NODES as c_int,
            );
        }
        return ptr::null_mut();
    }

    for index in 0..unsafe { (*graph).node_count } as usize {
        let existing = unsafe { (*graph).nodes[index] };
        if unsafe { strcmp((*existing).city_name.as_ptr(), city_name) } == 0 {
            unsafe {
                fprintf(
                    stderr,
                    b"Error: Node '%s' already exists\n\0".as_ptr().cast(),
                    city_name,
                );
            }
            return ptr::null_mut();
        }
    }

    let node = unsafe { dag_malloc(size_of::<Node>()).cast::<Node>() };
    if node.is_null() {
        unsafe { write_stderr(b"Error: Failed to allocate node\n\0") };
        return ptr::null_mut();
    }

    unsafe {
        strncpy((*node).city_name.as_mut_ptr(), city_name, MAX_CITY_NAME - 1);
        (*node).city_name[MAX_CITY_NAME - 1] = 0;
        (*node).ref_count = 1;
        (*node).edge_count = 0;

        let index = (*graph).node_count as usize;
        (*graph).nodes[index] = node;
        (*graph).node_count += 1;
    }
    node
}

#[no_mangle]
pub unsafe extern "C" fn add_edge(from: *mut Node, to: *mut Node, distance: c_int) -> c_int {
    if from.is_null() || to.is_null() {
        unsafe { write_stderr(b"Error: NULL node in add_edge\n\0") };
        return -1;
    }

    if unsafe { (*from).edge_count } >= MAX_EDGES as c_int {
        unsafe {
            fprintf(
                stderr,
                b"Error: Node '%s' has maximum edges\n\0".as_ptr().cast(),
                (*from).city_name.as_ptr(),
            );
        }
        return -1;
    }

    if distance < 0 {
        unsafe { write_stderr(b"Error: Negative distance not allowed\n\0") };
        return -1;
    }

    for index in 0..unsafe { (*from).edge_count } as usize {
        if unsafe { (*from).edges[index].destination } == to {
            unsafe { write_stderr(b"Error: Edge already exists\n\0") };
            return -1;
        }
    }

    unsafe {
        let index = (*from).edge_count as usize;
        (*from).edges[index].destination = to;
        (*from).edges[index].distance = distance;
        (*from).edge_count += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn delete_node(node: *mut Node) {
    if node.is_null() {
        return;
    }

    unsafe {
        (*node).ref_count = (*node).ref_count.wrapping_sub(1);
        if (*node).ref_count == 0 {
            free(node.cast());
        }
    }
}

unsafe fn increment_refs_recursive(
    node: *mut Node,
    visited: &mut [*mut Node; MAX_NODES],
    visited_count: &mut usize,
) {
    if node.is_null() {
        return;
    }

    for candidate in &visited[..*visited_count] {
        if *candidate == node {
            return;
        }
    }

    if *visited_count < MAX_NODES {
        visited[*visited_count] = node;
        *visited_count += 1;
    }

    unsafe {
        (*node).ref_count = (*node).ref_count.wrapping_add(1);
        for index in 0..(*node).edge_count as usize {
            increment_refs_recursive((*node).edges[index].destination, visited, visited_count);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn shallow_copy(start: *mut Node) -> *mut Node {
    if start.is_null() {
        unsafe { write_stderr(b"Error: NULL node in shallow_copy\n\0") };
        return ptr::null_mut();
    }

    let mut visited = [ptr::null_mut(); MAX_NODES];
    let mut visited_count = 0;
    unsafe { increment_refs_recursive(start, &mut visited, &mut visited_count) };
    start
}

#[no_mangle]
pub unsafe extern "C" fn find_shortest_path(
    start: *mut Node,
    end: *mut Node,
    path_length: *mut c_int,
) -> *mut *mut Node {
    if start.is_null() || end.is_null() || path_length.is_null() {
        unsafe { write_stderr(b"Error: NULL parameter in find_shortest_path\n\0") };
        return ptr::null_mut();
    }

    let empty_state = DijkstraNode {
        node: ptr::null_mut(),
        distance: 0,
        previous: ptr::null_mut(),
        visited: 0,
    };
    let mut state = [empty_state; MAX_NODES];
    let mut state_count = 1_usize;
    state[0] = DijkstraNode {
        node: start,
        distance: 0,
        previous: ptr::null_mut(),
        visited: 0,
    };

    let mut current = start;
    while !current.is_null() {
        let mut current_index = None;
        for (index, item) in state[..state_count].iter().enumerate() {
            if item.node == current {
                current_index = Some(index);
                break;
            }
        }
        let Some(current_index) = current_index else {
            break;
        };

        state[current_index].visited = 1;
        if current == end {
            break;
        }

        for edge_index in 0..unsafe { (*current).edge_count } as usize {
            let edge = unsafe { (*current).edges[edge_index] };
            let new_distance = state[current_index].distance.wrapping_add(edge.distance);
            let mut neighbor_index = None;
            for (index, item) in state[..state_count].iter().enumerate() {
                if item.node == edge.destination {
                    neighbor_index = Some(index);
                    break;
                }
            }

            if neighbor_index.is_none() && state_count < MAX_NODES {
                neighbor_index = Some(state_count);
                state[state_count] = DijkstraNode {
                    node: edge.destination,
                    distance: c_int::MAX,
                    previous: ptr::null_mut(),
                    visited: 0,
                };
                state_count += 1;
            }

            if let Some(index) = neighbor_index {
                if new_distance < state[index].distance {
                    state[index].distance = new_distance;
                    state[index].previous = current;
                }
            }
        }

        let mut minimum_distance = c_int::MAX;
        current = ptr::null_mut();
        for item in &state[..state_count] {
            if item.visited == 0 && item.distance < minimum_distance {
                minimum_distance = item.distance;
                current = item.node;
            }
        }
    }

    let mut end_index = None;
    for (index, item) in state[..state_count].iter().enumerate() {
        if item.node == end {
            end_index = Some(index);
            break;
        }
    }

    if end_index.is_none() || state[end_index.unwrap()].distance == c_int::MAX {
        unsafe {
            write_stderr(b"No path found\n\0");
            *path_length = 0;
        }
        return ptr::null_mut();
    }

    let mut path = [ptr::null_mut(); MAX_NODES];
    let mut count = 0_usize;
    let mut current_node = end;
    while !current_node.is_null() {
        path[count] = current_node;
        count += 1;

        let mut current_state_index = None;
        for (index, item) in state[..state_count].iter().enumerate() {
            if item.node == current_node {
                current_state_index = Some(index);
                break;
            }
        }
        let Some(current_state_index) = current_state_index else {
            break;
        };
        current_node = state[current_state_index].previous;
    }

    let result = unsafe { dag_malloc(size_of::<*mut Node>() * count).cast::<*mut Node>() };
    if result.is_null() {
        unsafe {
            write_stderr(b"Error: Failed to allocate path\n\0");
            *path_length = 0;
        }
        return ptr::null_mut();
    }

    for index in 0..count {
        unsafe { *result.add(index) = path[count - 1 - index] };
    }
    unsafe { *path_length = count as c_int };
    result
}

#[no_mangle]
pub unsafe extern "C" fn get_node_by_name(
    graph: *mut Graph,
    city_name: *const c_char,
) -> *mut Node {
    if graph.is_null() || city_name.is_null() {
        return ptr::null_mut();
    }

    for index in 0..unsafe { (*graph).node_count } as usize {
        let node = unsafe { (*graph).nodes[index] };
        if unsafe { strcmp((*node).city_name.as_ptr(), city_name) } == 0 {
            return node;
        }
    }
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn print_node(node: *mut Node) {
    if node.is_null() {
        unsafe { printf(b"NULL node\n\0".as_ptr().cast()) };
        return;
    }

    unsafe {
        printf(
            b"City: %s (ref_count: %d)\n\0".as_ptr().cast(),
            (*node).city_name.as_ptr(),
            (*node).ref_count,
        );
        printf(b"  Edges:\n\0".as_ptr().cast());
        for index in 0..(*node).edge_count as usize {
            let edge = (*node).edges[index];
            printf(
                b"    -> %s (distance: %d)\n\0".as_ptr().cast(),
                (*edge.destination).city_name.as_ptr(),
                edge.distance,
            );
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_graph(graph: *mut Graph) {
    if graph.is_null() {
        unsafe { printf(b"NULL graph\n\0".as_ptr().cast()) };
        return;
    }

    unsafe {
        printf(
            b"Graph with %d nodes:\n\0".as_ptr().cast(),
            (*graph).node_count,
        );
        for index in 0..(*graph).node_count as usize {
            print_node((*graph).nodes[index]);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn free_graph(graph: *mut Graph) {
    if graph.is_null() {
        return;
    }

    unsafe {
        for index in 0..(*graph).node_count as usize {
            delete_node((*graph).nodes[index]);
        }
        free(graph.cast());
    }
}
