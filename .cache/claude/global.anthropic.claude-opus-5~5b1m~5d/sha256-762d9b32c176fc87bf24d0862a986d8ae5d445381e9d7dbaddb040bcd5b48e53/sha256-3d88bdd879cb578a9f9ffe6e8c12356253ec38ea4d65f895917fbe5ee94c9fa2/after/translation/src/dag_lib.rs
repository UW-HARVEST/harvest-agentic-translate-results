//! Translation of `c_src/src/lib.c` / `c_src/include/dag_lib.h`.
//!
//! The C code hands out raw `node_t *` pointers, keeps them inside the graph
//! and (in `delete_node`) `free()`s a node while the graph still holds the
//! pointer.  Here every node lives in an arena (`Arena`) and a `node_t *` is
//! modelled as an index into that arena (`NodeRef`), so node identity
//! comparisons (`from->edges[i].destination == to`) keep working.  `free()` is
//! modelled as "the storage stays valid and unchanged", which keeps the
//! translation memory safe while preserving all observable output.

use crate::cio::{eput, COut};

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

/// Stand-in for `node_t *`.
pub type NodeRef = usize;

/// `struct edge_t`
pub struct Edge {
    pub destination: NodeRef,
    pub distance: i32,
}

/// `struct node_t`
pub struct Node {
    /// `char city_name[MAX_CITY_NAME]`
    pub city_name: [u8; MAX_CITY_NAME],
    pub ref_count: i32,
    pub edges: Vec<Edge>,
    /// Kept explicitly so the `edge_count >= MAX_EDGES` check mirrors the C.
    pub edge_count: i32,
    /// Set once `free()` would have been called on the node.
    #[allow(dead_code)]
    pub freed: bool,
}

impl Node {
    /// The value of the `city_name` C string (bytes up to the first NUL).
    pub fn name(&self) -> &[u8] {
        match self.city_name.iter().position(|&b| b == 0) {
            Some(i) => &self.city_name[..i],
            None => &self.city_name[..],
        }
    }
}

/// Backing store for every `malloc`ed `node_t`.
pub struct Arena {
    pub nodes: Vec<Node>,
}

impl Arena {
    pub fn new() -> Arena {
        Arena { nodes: Vec::new() }
    }

    /// `malloc(sizeof(node_t))` + the initialisation performed by `add_node`.
    fn alloc_node(&mut self, city_name: &[u8]) -> NodeRef {
        let mut buf = [0u8; MAX_CITY_NAME];
        // strncpy(node->city_name, city_name, MAX_CITY_NAME - 1);
        // node->city_name[MAX_CITY_NAME - 1] = '\0';
        let n = if city_name.len() < MAX_CITY_NAME - 1 {
            city_name.len()
        } else {
            MAX_CITY_NAME - 1
        };
        buf[..n].copy_from_slice(&city_name[..n]);
        buf[MAX_CITY_NAME - 1] = 0;
        self.nodes.push(Node {
            city_name: buf,
            ref_count: 1,
            edges: Vec::new(),
            edge_count: 0,
            freed: false,
        });
        self.nodes.len() - 1
    }
}

/// `struct graph_t`
pub struct Graph {
    pub nodes: Vec<NodeRef>,
    pub node_count: i32,
}

// Create a new empty graph
pub fn create_graph() -> Option<Graph> {
    Some(Graph {
        nodes: Vec::new(),
        node_count: 0,
    })
}

// Add a node to the graph
pub fn add_node(arena: &mut Arena, graph: &mut Graph, city_name: &[u8]) -> Option<NodeRef> {
    if graph.node_count as usize >= MAX_NODES {
        eput(format!("Error: Graph is full (max {} nodes)\n", MAX_NODES).as_bytes());
        return None;
    }

    // Check if node already exists
    for i in 0..graph.node_count as usize {
        if arena.nodes[graph.nodes[i]].name() == city_name {
            let mut msg: Vec<u8> = Vec::new();
            msg.extend_from_slice(b"Error: Node '");
            msg.extend_from_slice(city_name);
            msg.extend_from_slice(b"' already exists\n");
            eput(&msg);
            return None;
        }
    }

    let node = arena.alloc_node(city_name);

    // Add to graph
    graph.nodes.push(node);
    graph.node_count += 1;

    Some(node)
}

// Add an edge between two nodes
pub fn add_edge(arena: &mut Arena, from: NodeRef, to: NodeRef, distance: i32) -> i32 {
    if arena.nodes[from].edge_count as usize >= MAX_EDGES {
        let mut msg: Vec<u8> = Vec::new();
        msg.extend_from_slice(b"Error: Node '");
        msg.extend_from_slice(arena.nodes[from].name());
        msg.extend_from_slice(b"' has maximum edges\n");
        eput(&msg);
        return -1;
    }

    if distance < 0 {
        eput(b"Error: Negative distance not allowed\n");
        return -1;
    }

    // Check for duplicate edge
    for i in 0..arena.nodes[from].edge_count as usize {
        if arena.nodes[from].edges[i].destination == to {
            eput(b"Error: Edge already exists\n");
            return -1;
        }
    }

    // Add edge
    arena.nodes[from].edges.push(Edge {
        destination: to,
        distance,
    });
    arena.nodes[from].edge_count += 1;

    0
}

// Delete a node (decrement ref count, free if 0)
pub fn delete_node(arena: &mut Arena, node: NodeRef) {
    arena.nodes[node].ref_count -= 1;

    if arena.nodes[node].ref_count == 0 {
        // free(node): the storage is left untouched here, mirroring the
        // original program which keeps using the (dangling) pointer.
        arena.nodes[node].freed = true;
    }
}

// Helper function to increment ref count recursively
fn increment_refs_recursive(arena: &mut Arena, node: NodeRef, visited: &mut Vec<NodeRef>) {
    // Check if already visited
    for i in 0..visited.len() {
        if visited[i] == node {
            return;
        }
    }

    // Mark as visited
    if visited.len() < MAX_NODES {
        visited.push(node);
    }

    // Increment ref count
    arena.nodes[node].ref_count += 1;

    // Recursively process all connected nodes
    for i in 0..arena.nodes[node].edge_count as usize {
        let destination = arena.nodes[node].edges[i].destination;
        increment_refs_recursive(arena, destination, visited);
    }
}

// Create shallow copy of subsection (increments ref counts)
pub fn shallow_copy(arena: &mut Arena, start: NodeRef) -> Option<NodeRef> {
    // Track visited nodes to avoid cycles
    let mut visited: Vec<NodeRef> = Vec::new();

    // Increment ref counts for all reachable nodes
    increment_refs_recursive(arena, start, &mut visited);

    Some(start)
}

// Helper structure for shortest path algorithm
struct DijkstraNode {
    node: NodeRef,
    distance: i32,
    previous: Option<NodeRef>,
    visited: bool,
}

// Find shortest path using Dijkstra's algorithm
pub fn find_shortest_path(
    arena: &Arena,
    start: NodeRef,
    end: NodeRef,
    path_length: &mut i32,
) -> Option<Vec<NodeRef>> {
    // Initialize Dijkstra state
    let mut state: Vec<DijkstraNode> = Vec::new();

    // Add start node
    state.push(DijkstraNode {
        node: start,
        distance: 0,
        previous: None,
        visited: false,
    });

    let mut current: Option<NodeRef> = Some(start);

    while let Some(cur) = current {
        // Find current node in state
        let mut current_idx: isize = -1;
        for i in 0..state.len() {
            if state[i].node == cur {
                current_idx = i as isize;
                break;
            }
        }

        if current_idx == -1 {
            break;
        }
        let ci = current_idx as usize;

        state[ci].visited = true;

        // Check if we reached the end
        if cur == end {
            break;
        }

        // Update distances for neighbors
        for i in 0..arena.nodes[cur].edge_count as usize {
            let neighbor = arena.nodes[cur].edges[i].destination;
            let new_distance = state[ci]
                .distance
                .wrapping_add(arena.nodes[cur].edges[i].distance);

            // Find or add neighbor in state
            let mut neighbor_idx: isize = -1;
            for j in 0..state.len() {
                if state[j].node == neighbor {
                    neighbor_idx = j as isize;
                    break;
                }
            }

            if neighbor_idx == -1 && state.len() < MAX_NODES {
                // Add new neighbor
                neighbor_idx = state.len() as isize;
                state.push(DijkstraNode {
                    node: neighbor,
                    distance: i32::MAX,
                    previous: None,
                    visited: false,
                });
            }

            if neighbor_idx != -1 && new_distance < state[neighbor_idx as usize].distance {
                state[neighbor_idx as usize].distance = new_distance;
                state[neighbor_idx as usize].previous = Some(cur);
            }
        }

        // Find next unvisited node with minimum distance
        let mut min_distance = i32::MAX;
        current = None;
        for i in 0..state.len() {
            if !state[i].visited && state[i].distance < min_distance {
                min_distance = state[i].distance;
                current = Some(state[i].node);
            }
        }
    }

    // Find end node in state
    let mut end_idx: isize = -1;
    for i in 0..state.len() {
        if state[i].node == end {
            end_idx = i as isize;
            break;
        }
    }

    if end_idx == -1 || state[end_idx as usize].distance == i32::MAX {
        eput(b"No path found\n");
        *path_length = 0;
        return None;
    }

    // Reconstruct path
    let mut path: Vec<NodeRef> = Vec::new();
    let mut current_node: Option<NodeRef> = Some(end);

    while let Some(cn) = current_node {
        path.push(cn);

        // Find previous node
        let mut current_state_idx: isize = -1;
        for i in 0..state.len() {
            if state[i].node == cn {
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

    // Reverse path
    let mut result: Vec<NodeRef> = Vec::with_capacity(count);
    for i in 0..count {
        result.push(path[count - 1 - i]);
    }

    *path_length = count as i32;
    Some(result)
}

// Get node by city name
pub fn get_node_by_name(arena: &Arena, graph: &Graph, city_name: &[u8]) -> Option<NodeRef> {
    for i in 0..graph.node_count as usize {
        if arena.nodes[graph.nodes[i]].name() == city_name {
            return Some(graph.nodes[i]);
        }
    }

    None
}

// Print node information
pub fn print_node(out: &mut COut, arena: &Arena, node: NodeRef) {
    let n = &arena.nodes[node];
    out.put(b"City: ");
    out.put(n.name());
    out.put(format!(" (ref_count: {})\n", n.ref_count).as_bytes());
    out.put(b"  Edges:\n");
    for i in 0..n.edge_count as usize {
        out.put(b"    -> ");
        out.put(arena.nodes[n.edges[i].destination].name());
        out.put(format!(" (distance: {})\n", n.edges[i].distance).as_bytes());
    }
}

// Print entire graph
pub fn print_graph(out: &mut COut, arena: &Arena, graph: &Graph) {
    out.put(format!("Graph with {} nodes:\n", graph.node_count).as_bytes());
    for i in 0..graph.node_count as usize {
        print_node(out, arena, graph.nodes[i]);
    }
}

// Free the entire graph
pub fn free_graph(arena: &mut Arena, graph: &Graph) {
    // Decrement ref count for all nodes
    for i in 0..graph.node_count as usize {
        delete_node(arena, graph.nodes[i]);
    }

    // free(graph)
}
