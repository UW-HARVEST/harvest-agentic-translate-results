use std::ffi::CStr;
use std::io::{self, Write};
use std::os::raw::{c_char, c_int, c_void};

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

// ============================================================
// Rust-native types and implementation
// ============================================================

pub struct Edge {
    pub destination: usize,
    pub distance: i32,
}

pub struct Node {
    pub city_name: String,
    pub ref_count: i32,
    pub edges: Vec<Edge>,
}

pub struct Graph {
    pub nodes: Vec<Node>,
}

pub fn rs_create_graph() -> Option<Graph> {
    Some(Graph { nodes: Vec::new() })
}

pub fn rs_add_node(graph: &mut Graph, city_name: &str) -> Option<usize> {
    if city_name.is_empty() {
        eprint!("Error: NULL parameter in add_node\n");
        return None;
    }
    if graph.nodes.len() >= MAX_NODES {
        eprint!("Error: Graph is full (max {} nodes)\n", MAX_NODES);
        return None;
    }
    for node in &graph.nodes {
        if node.city_name == city_name {
            eprint!("Error: Node '{}' already exists\n", city_name);
            return None;
        }
    }
    let mut name = city_name.to_string();
    name.truncate(MAX_CITY_NAME - 1);
    let idx = graph.nodes.len();
    graph.nodes.push(Node {
        city_name: name,
        ref_count: 1,
        edges: Vec::new(),
    });
    Some(idx)
}

pub fn rs_add_edge(graph: &mut Graph, from: usize, to: usize, distance: i32) -> i32 {
    if graph.nodes[from].edges.len() >= MAX_EDGES {
        eprint!(
            "Error: Node '{}' has maximum edges\n",
            graph.nodes[from].city_name
        );
        return -1;
    }
    if distance < 0 {
        eprint!("Error: Negative distance not allowed\n");
        return -1;
    }
    for e in &graph.nodes[from].edges {
        if e.destination == to {
            eprint!("Error: Edge already exists\n");
            return -1;
        }
    }
    graph.nodes[from].edges.push(Edge {
        destination: to,
        distance,
    });
    0
}

pub fn rs_delete_node(graph: &mut Graph, idx: usize) {
    graph.nodes[idx].ref_count -= 1;
}

pub fn rs_increment_refs_recursive(
    graph: &mut Graph,
    node_idx: usize,
    visited: &mut Vec<usize>,
) {
    if visited.contains(&node_idx) {
        return;
    }
    if visited.len() < MAX_NODES {
        visited.push(node_idx);
    }
    graph.nodes[node_idx].ref_count += 1;
    let destinations: Vec<usize> = graph.nodes[node_idx]
        .edges
        .iter()
        .map(|e| e.destination)
        .collect();
    for dest in destinations {
        rs_increment_refs_recursive(graph, dest, visited);
    }
}

pub fn rs_shallow_copy(graph: &mut Graph, start: usize) -> Option<usize> {
    let mut visited = Vec::new();
    rs_increment_refs_recursive(graph, start, &mut visited);
    Some(start)
}

struct DijkstraNode {
    node: usize,
    distance: i32,
    previous: Option<usize>,
    visited: bool,
}

pub fn rs_find_shortest_path(graph: &Graph, start: usize, end: usize) -> Option<Vec<usize>> {
    let mut state: Vec<DijkstraNode> = Vec::new();
    state.push(DijkstraNode {
        node: start,
        distance: 0,
        previous: None,
        visited: false,
    });

    let mut current = Some(start);

    while let Some(cur) = current {
        let current_idx = match state.iter().position(|s| s.node == cur) {
            Some(i) => i,
            None => break,
        };

        state[current_idx].visited = true;

        if cur == end {
            break;
        }

        let cur_dist = state[current_idx].distance;
        let edges: Vec<(usize, i32)> = graph.nodes[cur]
            .edges
            .iter()
            .map(|e| (e.destination, e.distance))
            .collect();

        for (neighbor, edge_dist) in edges {
            let new_distance = cur_dist + edge_dist;
            let neighbor_idx = match state.iter().position(|s| s.node == neighbor) {
                Some(i) => i,
                None => {
                    if state.len() < MAX_NODES {
                        let idx = state.len();
                        state.push(DijkstraNode {
                            node: neighbor,
                            distance: i32::MAX,
                            previous: None,
                            visited: false,
                        });
                        idx
                    } else {
                        continue;
                    }
                }
            };

            if new_distance < state[neighbor_idx].distance {
                state[neighbor_idx].distance = new_distance;
                state[neighbor_idx].previous = Some(current_idx);
            }
        }

        let mut min_distance = i32::MAX;
        current = None;
        for s in &state {
            if !s.visited && s.distance < min_distance {
                min_distance = s.distance;
                current = Some(s.node);
            }
        }
    }

    let end_idx = match state.iter().position(|s| s.node == end) {
        Some(i) => i,
        None => {
            eprint!("No path found\n");
            return None;
        }
    };

    if state[end_idx].distance == i32::MAX {
        eprint!("No path found\n");
        return None;
    }

    let mut path = Vec::new();
    let mut cur_state_idx = Some(end_idx);
    while let Some(idx) = cur_state_idx {
        path.push(state[idx].node);
        cur_state_idx = state[idx].previous;
    }
    path.reverse();
    Some(path)
}

pub fn rs_get_node_by_name(graph: &Graph, city_name: &str) -> Option<usize> {
    for (i, node) in graph.nodes.iter().enumerate() {
        if node.city_name == city_name {
            return Some(i);
        }
    }
    None
}

pub fn rs_print_node_to<W: Write>(graph: &Graph, idx: usize, w: &mut W) {
    let node = &graph.nodes[idx];
    write!(w, "City: {} (ref_count: {})\n", node.city_name, node.ref_count).unwrap();
    write!(w, "  Edges:\n").unwrap();
    for e in &node.edges {
        write!(
            w,
            "    -> {} (distance: {})\n",
            graph.nodes[e.destination].city_name, e.distance
        )
        .unwrap();
    }
}

pub fn rs_print_node(graph: &Graph, idx: usize) {
    rs_print_node_to(graph, idx, &mut io::stdout());
}

pub fn rs_print_graph_to<W: Write>(graph: &Graph, w: &mut W) {
    write!(w, "Graph with {} nodes:\n", graph.nodes.len()).unwrap();
    for i in 0..graph.nodes.len() {
        rs_print_node_to(graph, i, w);
    }
}

pub fn rs_print_graph(graph: &Graph) {
    rs_print_graph_to(graph, &mut io::stdout());
}

pub fn rs_free_graph(graph: &mut Graph) {
    for i in 0..graph.nodes.len() {
        graph.nodes[i].ref_count -= 1;
    }
}

// ============================================================
// C-compatible FFI exports
// ============================================================

#[repr(C)]
pub struct CEdge {
    pub destination: *mut CNode,
    pub distance: c_int,
}

#[repr(C)]
pub struct CNode {
    pub city_name: [c_char; 64],
    pub ref_count: c_int,
    pub edges: [CEdge; 10],
    pub edge_count: c_int,
}

#[repr(C)]
pub struct CGraph {
    pub nodes: [*mut CNode; 100],
    pub node_count: c_int,
}

unsafe fn cnode_set_city_name(node: *mut CNode, name: &str) {
    let bytes = name.as_bytes();
    let len = bytes.len().min(MAX_CITY_NAME - 1);
    for i in 0..len {
        (*node).city_name[i] = bytes[i] as c_char;
    }
    (*node).city_name[len] = 0;
}

unsafe fn cnode_city_name(node: *const CNode) -> String {
    CStr::from_ptr((*node).city_name.as_ptr())
        .to_string_lossy()
        .into_owned()
}

#[no_mangle]
pub unsafe extern "C" fn create_graph() -> *mut CGraph {
    let g = libc::malloc(std::mem::size_of::<CGraph>()) as *mut CGraph;
    if g.is_null() {
        eprint!("Error: Failed to allocate graph\n");
        return std::ptr::null_mut();
    }
    (*g).node_count = 0;
    for i in 0..MAX_NODES {
        (*g).nodes[i] = std::ptr::null_mut();
    }
    g
}

#[no_mangle]
pub unsafe extern "C" fn add_node(graph: *mut CGraph, city_name: *const c_char) -> *mut CNode {
    if graph.is_null() || city_name.is_null() {
        eprint!("Error: NULL parameter in add_node\n");
        return std::ptr::null_mut();
    }
    let name = CStr::from_ptr(city_name).to_string_lossy();
    if (*graph).node_count >= MAX_NODES as c_int {
        eprint!("Error: Graph is full (max {} nodes)\n", MAX_NODES);
        return std::ptr::null_mut();
    }
    for i in 0..(*graph).node_count as usize {
        if cnode_city_name((*graph).nodes[i]) == name.as_ref() {
            eprint!("Error: Node '{}' already exists\n", name);
            return std::ptr::null_mut();
        }
    }
    let node = libc::malloc(std::mem::size_of::<CNode>()) as *mut CNode;
    if node.is_null() {
        eprint!("Error: Failed to allocate node\n");
        return std::ptr::null_mut();
    }
    std::ptr::write_bytes(node, 0, 1);
    cnode_set_city_name(node, &name);
    (*node).ref_count = 1;
    (*node).edge_count = 0;
    (*graph).nodes[(*graph).node_count as usize] = node;
    (*graph).node_count += 1;
    node
}

#[no_mangle]
pub unsafe extern "C" fn add_edge(from: *mut CNode, to: *mut CNode, distance: c_int) -> c_int {
    if from.is_null() || to.is_null() {
        eprint!("Error: NULL node in add_edge\n");
        return -1;
    }
    if (*from).edge_count >= MAX_EDGES as c_int {
        eprint!(
            "Error: Node '{}' has maximum edges\n",
            cnode_city_name(from)
        );
        return -1;
    }
    if distance < 0 {
        eprint!("Error: Negative distance not allowed\n");
        return -1;
    }
    for i in 0..(*from).edge_count as usize {
        if (*from).edges[i].destination == to {
            eprint!("Error: Edge already exists\n");
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
pub unsafe extern "C" fn delete_node(node: *mut CNode) {
    if node.is_null() {
        return;
    }
    (*node).ref_count -= 1;
    if (*node).ref_count == 0 {
        libc::free(node as *mut c_void);
    }
}

unsafe fn ffi_increment_refs_recursive(
    node: *mut CNode,
    visited: &mut [*mut CNode; 100],
    visited_count: &mut usize,
) {
    if node.is_null() {
        return;
    }
    for i in 0..*visited_count {
        if visited[i] == node {
            return;
        }
    }
    if *visited_count < MAX_NODES {
        visited[*visited_count] = node;
        *visited_count += 1;
    }
    (*node).ref_count += 1;
    for i in 0..(*node).edge_count as usize {
        ffi_increment_refs_recursive((*node).edges[i].destination, visited, visited_count);
    }
}

#[no_mangle]
pub unsafe extern "C" fn shallow_copy(start: *mut CNode) -> *mut CNode {
    if start.is_null() {
        eprint!("Error: NULL node in shallow_copy\n");
        return std::ptr::null_mut();
    }
    let mut visited: [*mut CNode; 100] = [std::ptr::null_mut(); 100];
    let mut visited_count: usize = 0;
    ffi_increment_refs_recursive(start, &mut visited, &mut visited_count);
    start
}

#[repr(C)]
struct FfiDijkstraNode {
    node: *mut CNode,
    distance: c_int,
    previous: *mut CNode,
    visited: c_int,
}

#[no_mangle]
pub unsafe extern "C" fn find_shortest_path(
    start: *mut CNode,
    end: *mut CNode,
    path_length: *mut c_int,
) -> *mut *mut CNode {
    if start.is_null() || end.is_null() || path_length.is_null() {
        eprint!("Error: NULL parameter in find_shortest_path\n");
        return std::ptr::null_mut();
    }

    let mut state: Vec<FfiDijkstraNode> = Vec::new();
    state.push(FfiDijkstraNode {
        node: start,
        distance: 0,
        previous: std::ptr::null_mut(),
        visited: 0,
    });

    let mut current = start;

    loop {
        let current_idx = match state.iter().position(|s| s.node == current) {
            Some(i) => i,
            None => break,
        };
        state[current_idx].visited = 1;

        if current == end {
            break;
        }

        let cur_dist = state[current_idx].distance;
        for i in 0..(*current).edge_count as usize {
            let neighbor = (*current).edges[i].destination;
            let new_distance = cur_dist + (*current).edges[i].distance;

            let neighbor_idx = match state.iter().position(|s| s.node == neighbor) {
                Some(i) => i,
                None => {
                    if state.len() < MAX_NODES {
                        let idx = state.len();
                        state.push(FfiDijkstraNode {
                            node: neighbor,
                            distance: c_int::MAX,
                            previous: std::ptr::null_mut(),
                            visited: 0,
                        });
                        idx
                    } else {
                        continue;
                    }
                }
            };

            if new_distance < state[neighbor_idx].distance {
                state[neighbor_idx].distance = new_distance;
                state[neighbor_idx].previous = current;
            }
        }

        let mut min_distance = c_int::MAX;
        current = std::ptr::null_mut();
        for s in &state {
            if s.visited == 0 && s.distance < min_distance {
                min_distance = s.distance;
                current = s.node;
            }
        }
        if current.is_null() {
            break;
        }
    }

    let end_idx = match state.iter().position(|s| s.node == end) {
        Some(i) => i,
        None => {
            eprint!("No path found\n");
            *path_length = 0;
            return std::ptr::null_mut();
        }
    };

    if state[end_idx].distance == c_int::MAX {
        eprint!("No path found\n");
        *path_length = 0;
        return std::ptr::null_mut();
    }

    let mut path: Vec<*mut CNode> = Vec::new();
    let mut current_node = end;
    while !current_node.is_null() {
        path.push(current_node);
        match state.iter().position(|s| s.node == current_node) {
            Some(i) => current_node = state[i].previous,
            None => break,
        }
    }

    let count = path.len();
    let result = libc::malloc(std::mem::size_of::<*mut CNode>() * count) as *mut *mut CNode;
    if result.is_null() {
        eprint!("Error: Failed to allocate path\n");
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
    graph: *mut CGraph,
    city_name: *const c_char,
) -> *mut CNode {
    if graph.is_null() || city_name.is_null() {
        return std::ptr::null_mut();
    }
    let name = CStr::from_ptr(city_name).to_string_lossy();
    for i in 0..(*graph).node_count as usize {
        if cnode_city_name((*graph).nodes[i]) == name.as_ref() {
            return (*graph).nodes[i];
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn print_node(node: *mut CNode) {
    if node.is_null() {
        print!("NULL node\n");
        return;
    }
    print!(
        "City: {} (ref_count: {})\n",
        cnode_city_name(node),
        (*node).ref_count
    );
    print!("  Edges:\n");
    for i in 0..(*node).edge_count as usize {
        print!(
            "    -> {} (distance: {})\n",
            cnode_city_name((*node).edges[i].destination),
            (*node).edges[i].distance
        );
    }
}

#[no_mangle]
pub unsafe extern "C" fn print_graph(graph: *mut CGraph) {
    if graph.is_null() {
        print!("NULL graph\n");
        return;
    }
    print!("Graph with {} nodes:\n", (*graph).node_count);
    for i in 0..(*graph).node_count as usize {
        print_node((*graph).nodes[i]);
    }
}

#[no_mangle]
pub unsafe extern "C" fn free_graph(graph: *mut CGraph) {
    if graph.is_null() {
        return;
    }
    for i in 0..(*graph).node_count as usize {
        delete_node((*graph).nodes[i]);
    }
    libc::free(graph as *mut c_void);
}
