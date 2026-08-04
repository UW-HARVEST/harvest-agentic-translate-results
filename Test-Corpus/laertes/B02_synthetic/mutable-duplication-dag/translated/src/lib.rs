extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn strncpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
        __n: size_t,
    ) -> *mut libc::c_char;
    fn strcmp(
        __s1: *const libc::c_char,
        __s2: *const libc::c_char,
    ) -> libc::c_int;
}
pub type size_t = usize;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut libc::c_void,
    pub __pad2: *mut libc::c_void,
    pub __pad3: *mut libc::c_void,
    pub __pad4: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = _IO_FILE;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct node_t {
    pub city_name: [libc::c_char; 64],
    pub ref_count: libc::c_int,
    pub edges: [edge_t; 10],
    pub edge_count: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct edge_t {
    pub destination: *mut node_t,
    pub distance: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct graph_t {
    pub nodes: [*mut node_t; 100],
    pub node_count: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dijkstra_node_t {
    pub node: *mut node_t,
    pub distance: libc::c_int,
    pub previous: *mut node_t,
    pub visited: libc::c_int,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn create_graph() -> *mut graph_t {
    let mut graph: *mut graph_t =
        malloc(std::mem::size_of::<graph_t>() as size_t) as *mut graph_t;
    if graph.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: Failed to allocate graph\n\0" as *const u8 as *const libc::c_char,
        );
        return std::ptr::null_mut::<graph_t>();
    }
    (*graph).node_count = 0 as libc::c_int;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < MAX_NODES {
        (*graph).nodes[i as usize] = std::ptr::null_mut::<node_t>();
        i += 1;
    }
    return graph;
}
#[no_mangle]
pub unsafe extern "C" fn add_node(
    mut graph: *mut graph_t,
    mut city_name: *const libc::c_char,
) -> *mut node_t {
    if graph.is_null() || city_name.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL parameter in add_node\n\0" as *const u8 as *const libc::c_char,
        );
        return std::ptr::null_mut::<node_t>();
    }
    if (*graph).node_count >= MAX_NODES {
        fprintf(
            stderr as *mut FILE,
            b"Error: Graph is full (max %d nodes)\n\0" as *const u8 as *const libc::c_char,
            MAX_NODES,
        );
        return std::ptr::null_mut::<node_t>();
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < (*graph).node_count {
        if strcmp(
            &raw mut (**(&raw mut (*graph).nodes as *mut *mut node_t).offset(i as isize)).city_name
                as *mut libc::c_char,
            city_name,
        ) == 0 as libc::c_int
        {
            fprintf(
                stderr as *mut FILE,
                b"Error: Node '%s' already exists\n\0" as *const u8 as *const libc::c_char,
                city_name,
            );
            return std::ptr::null_mut::<node_t>();
        }
        i += 1;
    }
    let mut node: *mut node_t = malloc(std::mem::size_of::<node_t>() as size_t) as *mut node_t;
    if node.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: Failed to allocate node\n\0" as *const u8 as *const libc::c_char,
        );
        return std::ptr::null_mut::<node_t>();
    }
    strncpy(
        &raw mut (*node).city_name as *mut libc::c_char,
        city_name,
        (MAX_CITY_NAME - 1 as libc::c_int) as size_t,
    );
    (*node).city_name[(MAX_CITY_NAME - 1 as libc::c_int) as usize] =
        '\0' as i32 as libc::c_char;
    (*node).ref_count = 1 as libc::c_int;
    (*node).edge_count = 0 as libc::c_int;
    let fresh0 = (*graph).node_count;
    (*graph).node_count = (*graph).node_count + 1;
    (*graph).nodes[fresh0 as usize] = node;
    return node;
}
#[no_mangle]
pub unsafe extern "C" fn add_edge(
    mut from: *mut node_t,
    mut to: *mut node_t,
    mut distance: libc::c_int,
) -> libc::c_int {
    if from.is_null() || to.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL node in add_edge\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    if (*from).edge_count >= MAX_EDGES {
        fprintf(
            stderr as *mut FILE,
            b"Error: Node '%s' has maximum edges\n\0" as *const u8 as *const libc::c_char,
            &raw mut (*from).city_name as *mut libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    if distance < 0 as libc::c_int {
        fprintf(
            stderr as *mut FILE,
            b"Error: Negative distance not allowed\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < (*from).edge_count {
        if (*from).edges[i as usize].destination == to {
            fprintf(
                stderr as *mut FILE,
                b"Error: Edge already exists\n\0" as *const u8 as *const libc::c_char,
            );
            return -(1 as libc::c_int);
        }
        i += 1;
    }
    (*from).edges[(*from).edge_count as usize].destination = to;
    (*from).edges[(*from).edge_count as usize].distance = distance;
    (*from).edge_count += 1;
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn delete_node(mut node: *mut node_t) {
    if node.is_null() {
        return;
    }
    (*node).ref_count -= 1;
    if (*node).ref_count == 0 as libc::c_int {
        free(node as *mut libc::c_void);
    }
}
unsafe extern "C" fn increment_refs_recursive(
    mut node: *mut node_t,
    mut visited: *mut *mut node_t,
    mut visited_count: *mut libc::c_int,
) {
    if node.is_null() {
        return;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < *visited_count {
        if *visited.offset(i as isize) == node {
            return;
        }
        i += 1;
    }
    if *visited_count < MAX_NODES {
        let fresh1 = *visited_count;
        *visited_count = *visited_count + 1;
        let ref mut fresh2 = *visited.offset(fresh1 as isize);
        *fresh2 = node;
    }
    (*node).ref_count += 1;
    let mut i_0: libc::c_int = 0 as libc::c_int;
    while i_0 < (*node).edge_count {
        increment_refs_recursive(
            (*node).edges[i_0 as usize].destination,
            visited,
            visited_count,
        );
        i_0 += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn shallow_copy(mut start: *mut node_t) -> *mut node_t {
    if start.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL node in shallow_copy\n\0" as *const u8 as *const libc::c_char,
        );
        return std::ptr::null_mut::<node_t>();
    }
    let mut visited: [*mut node_t; 100] = [std::ptr::null_mut::<node_t>(); 100];
    let mut visited_count: libc::c_int = 0 as libc::c_int;
    increment_refs_recursive(
        start,
        &raw mut visited as *mut *mut node_t,
        &raw mut visited_count,
    );
    return start;
}
#[no_mangle]
pub unsafe extern "C" fn find_shortest_path(
    mut start: *mut node_t,
    mut end: *mut node_t,
    mut path_length: *mut libc::c_int,
) -> *mut *mut node_t {
    if start.is_null() || end.is_null() || path_length.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL parameter in find_shortest_path\n\0" as *const u8
                as *const libc::c_char,
        );
        return std::ptr::null_mut::<*mut node_t>();
    }
    let mut state: [dijkstra_node_t; 100] = [dijkstra_node_t {
        node: std::ptr::null_mut::<node_t>(),
        distance: 0,
        previous: std::ptr::null_mut::<node_t>(),
        visited: 0,
    }; 100];
    let mut state_count: libc::c_int = 0 as libc::c_int;
    state[state_count as usize].node = start;
    state[state_count as usize].distance = 0 as libc::c_int;
    state[state_count as usize].previous = std::ptr::null_mut::<node_t>();
    state[state_count as usize].visited = 0 as libc::c_int;
    state_count += 1;
    let mut current: *mut node_t = start;
    while !current.is_null() {
        let mut current_idx: libc::c_int = -(1 as libc::c_int);
        let mut i: libc::c_int = 0 as libc::c_int;
        while i < state_count {
            if state[i as usize].node == current {
                current_idx = i;
                break;
            } else {
                i += 1;
            }
        }
        if current_idx == -(1 as libc::c_int) {
            break;
        }
        state[current_idx as usize].visited = 1 as libc::c_int;
        if current == end {
            break;
        }
        let mut i_0: libc::c_int = 0 as libc::c_int;
        while i_0 < (*current).edge_count {
            let mut neighbor: *mut node_t = (*current).edges[i_0 as usize].destination;
            let mut new_distance: libc::c_int =
                state[current_idx as usize].distance + (*current).edges[i_0 as usize].distance;
            let mut neighbor_idx: libc::c_int = -(1 as libc::c_int);
            let mut j: libc::c_int = 0 as libc::c_int;
            while j < state_count {
                if state[j as usize].node == neighbor {
                    neighbor_idx = j;
                    break;
                } else {
                    j += 1;
                }
            }
            if neighbor_idx == -(1 as libc::c_int) && state_count < MAX_NODES {
                neighbor_idx = state_count;
                state[state_count as usize].node = neighbor;
                state[state_count as usize].distance = INT_MAX;
                state[state_count as usize].previous = std::ptr::null_mut::<node_t>();
                state[state_count as usize].visited = 0 as libc::c_int;
                state_count += 1;
            }
            if neighbor_idx != -(1 as libc::c_int)
                && new_distance < state[neighbor_idx as usize].distance
            {
                state[neighbor_idx as usize].distance = new_distance;
                state[neighbor_idx as usize].previous = current;
            }
            i_0 += 1;
        }
        let mut min_distance: libc::c_int = INT_MAX;
        current = std::ptr::null_mut::<node_t>();
        let mut i_1: libc::c_int = 0 as libc::c_int;
        while i_1 < state_count {
            if state[i_1 as usize].visited == 0 && state[i_1 as usize].distance < min_distance {
                min_distance = state[i_1 as usize].distance;
                current = state[i_1 as usize].node;
            }
            i_1 += 1;
        }
    }
    let mut end_idx: libc::c_int = -(1 as libc::c_int);
    let mut i_2: libc::c_int = 0 as libc::c_int;
    while i_2 < state_count {
        if state[i_2 as usize].node == end {
            end_idx = i_2;
            break;
        } else {
            i_2 += 1;
        }
    }
    if end_idx == -(1 as libc::c_int) || state[end_idx as usize].distance == INT_MAX {
        fprintf(
            stderr as *mut FILE,
            b"No path found\n\0" as *const u8 as *const libc::c_char,
        );
        *path_length = 0 as libc::c_int;
        return std::ptr::null_mut::<*mut node_t>();
    }
    let mut path: [*mut node_t; 100] = [std::ptr::null_mut::<node_t>(); 100];
    let mut count: libc::c_int = 0 as libc::c_int;
    let mut current_node: *mut node_t = end;
    while !current_node.is_null() {
        let fresh3 = count;
        count = count + 1;
        path[fresh3 as usize] = current_node;
        let mut current_state_idx: libc::c_int = -(1 as libc::c_int);
        let mut i_3: libc::c_int = 0 as libc::c_int;
        while i_3 < state_count {
            if state[i_3 as usize].node == current_node {
                current_state_idx = i_3;
                break;
            } else {
                i_3 += 1;
            }
        }
        if current_state_idx == -(1 as libc::c_int) {
            break;
        }
        current_node = state[current_state_idx as usize].previous;
    }
    let mut result: *mut *mut node_t =
        malloc((std::mem::size_of::<*mut node_t>() as size_t).wrapping_mul(count as size_t))
            as *mut *mut node_t;
    if result.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: Failed to allocate path\n\0" as *const u8 as *const libc::c_char,
        );
        *path_length = 0 as libc::c_int;
        return std::ptr::null_mut::<*mut node_t>();
    }
    let mut i_4: libc::c_int = 0 as libc::c_int;
    while i_4 < count {
        let ref mut fresh4 = *result.offset(i_4 as isize);
        *fresh4 = path[(count - 1 as libc::c_int - i_4) as usize];
        i_4 += 1;
    }
    *path_length = count;
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn get_node_by_name(
    mut graph: *mut graph_t,
    mut city_name: *const libc::c_char,
) -> *mut node_t {
    if graph.is_null() || city_name.is_null() {
        return std::ptr::null_mut::<node_t>();
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < (*graph).node_count {
        if strcmp(
            &raw mut (**(&raw mut (*graph).nodes as *mut *mut node_t).offset(i as isize)).city_name
                as *mut libc::c_char,
            city_name,
        ) == 0 as libc::c_int
        {
            return (*graph).nodes[i as usize];
        }
        i += 1;
    }
    return std::ptr::null_mut::<node_t>();
}
#[no_mangle]
pub unsafe extern "C" fn print_node(mut node: *mut node_t) {
    if node.is_null() {
        printf(b"NULL node\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    printf(
        b"City: %s (ref_count: %d)\n\0" as *const u8 as *const libc::c_char,
        &raw mut (*node).city_name as *mut libc::c_char,
        (*node).ref_count,
    );
    printf(b"  Edges:\n\0" as *const u8 as *const libc::c_char);
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < (*node).edge_count {
        printf(
            b"    -> %s (distance: %d)\n\0" as *const u8 as *const libc::c_char,
            &raw mut (*(*(&raw mut (*node).edges as *mut edge_t).offset(i as isize)).destination)
                .city_name as *mut libc::c_char,
            (*node).edges[i as usize].distance,
        );
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn print_graph(mut graph: *mut graph_t) {
    if graph.is_null() {
        printf(b"NULL graph\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    printf(
        b"Graph with %d nodes:\n\0" as *const u8 as *const libc::c_char,
        (*graph).node_count,
    );
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < (*graph).node_count {
        print_node((*graph).nodes[i as usize]);
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn free_graph(mut graph: *mut graph_t) {
    if graph.is_null() {
        return;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < (*graph).node_count {
        delete_node((*graph).nodes[i as usize]);
        i += 1;
    }
    free(graph as *mut libc::c_void);
}
pub const MAX_CITY_NAME: libc::c_int = 64 as libc::c_int;
pub const MAX_EDGES: libc::c_int = 10 as libc::c_int;
pub const MAX_NODES: libc::c_int = 100 as libc::c_int;
pub const __INT_MAX__: libc::c_int = 2147483647 as libc::c_int;
pub const INT_MAX: libc::c_int = __INT_MAX__;
