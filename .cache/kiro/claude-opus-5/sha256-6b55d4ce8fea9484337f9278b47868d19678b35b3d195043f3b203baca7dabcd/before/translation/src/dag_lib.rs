//! Translation of `c_src/src/lib.c` / `c_src/include/dag_lib.h`.
//!
//! The C original stores raw `node_t *` pointers in the graph and hands them
//! out to callers.  `delete_node` frees a node once its reference count hits
//! zero *without* removing it from the graph, so the graph keeps dangling
//! pointers afterwards.  To reproduce the program's behaviour without unsafe
//! code, nodes live in an arena (`Arena`) and are addressed by index
//! (`NodeId`), which plays the role of the pointer: index comparison replaces
//! pointer comparison, and a "freed" node keeps its slot so that later reads
//! still see its contents.

use crate::cio::{err, Out};

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

/// Stands in for `node_t *`.
pub type NodeId = usize;

pub struct Edge {
    pub destination: NodeId,
    pub distance: i32,
}

pub struct Node {
    pub city_name: [u8; MAX_CITY_NAME],
    pub ref_count: i32,
    pub edges: Vec<Edge>,
    /// True once `delete_node` dropped the reference count to zero and the C
    /// original called `free`.
    pub freed: bool,
}

impl Node {
    /// `node->city_name` as seen by `%s`: bytes up to the NUL terminator.
    pub fn name(&self) -> &[u8] {
        let end = self
            .city_name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(MAX_CITY_NAME);
        &self.city_name[..end]
    }

    /// `strcmp(node->city_name, other) == 0`.
    ///
    /// A freed node never compares equal: glibc overwrites the first 16 bytes of
    /// the chunk (the tcache `next` and `key` fields, which land exactly on
    /// `city_name[0..16]`) with allocator metadata derived from a heap address,
    /// so the dangling pointers still held by the graph no longer match any city
    /// name.  That is why deleting a city twice reports "not found" the second
    /// time.
    pub fn name_matches(&self, other: &[u8]) -> bool {
        if self.freed {
            return false;
        }
        self.name() == other
    }
}

/// Backing store for every node ever allocated.
///
/// `free_list` models glibc's tcache: freeing a node returns its chunk to a
/// per-size LIFO list, and the next allocation of the same size hands the very
/// same address back out.  Reusing the slot reproduces the aliasing the C
/// program exhibits when a deleted city is added again -- the stale pointer in
/// `graph->nodes` and in other nodes' edges then refers to the new node.
pub struct Arena {
    nodes: Vec<Node>,
    free_list: Vec<NodeId>,
}

impl Arena {
    pub fn new() -> Self {
        Arena {
            nodes: Vec::new(),
            free_list: Vec::new(),
        }
    }

    pub fn get(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    pub fn get_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id]
    }

    /// `malloc(sizeof(node_t))`
    fn alloc(&mut self, node: Node) -> NodeId {
        match self.free_list.pop() {
            Some(id) => {
                self.nodes[id] = node;
                id
            }
            None => {
                self.nodes.push(node);
                self.nodes.len() - 1
            }
        }
    }

    /// `free(node)`
    fn dealloc(&mut self, id: NodeId) {
        let n = &mut self.nodes[id];
        n.freed = true;
        // The allocator writes its `next`/`key` bookkeeping over the first 16
        // bytes of the user block, destroying the start of `city_name`.
        for b in n.city_name[..16].iter_mut() {
            *b = 0;
        }
        self.free_list.push(id);
    }
}

pub struct Graph {
    pub nodes: Vec<NodeId>,
}

impl Graph {
    /// `graph->node_count`
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

/// `create_graph`
pub fn create_graph() -> Option<Graph> {
    // malloc never fails here, so the error branch is unreachable.
    Some(Graph {
        nodes: Vec::with_capacity(MAX_NODES),
    })
}

/// `add_node`
pub fn add_node(
    arena: &mut Arena,
    graph: Option<&mut Graph>,
    city_name: Option<&[u8]>,
) -> Option<NodeId> {
    let (graph, city_name) = match (graph, city_name) {
        (Some(g), Some(c)) => (g, c),
        _ => {
            err(b"Error: NULL parameter in add_node\n");
            return None;
        }
    };

    if graph.node_count() >= MAX_NODES {
        err(format!("Error: Graph is full (max {} nodes)\n", MAX_NODES).as_bytes());
        return None;
    }

    // Check if node already exists
    for i in 0..graph.node_count() {
        if arena.get(graph.nodes[i]).name_matches(city_name) {
            let mut msg = Vec::new();
            msg.extend_from_slice(b"Error: Node '");
            msg.extend_from_slice(city_name);
            msg.extend_from_slice(b"' already exists\n");
            err(&msg);
            return None;
        }
    }

    // Initialize node: strncpy(node->city_name, city_name, MAX_CITY_NAME - 1)
    // copies at most 63 bytes and zero pads the remainder; the last byte is
    // then explicitly cleared.
    let mut name = [0u8; MAX_CITY_NAME];
    let n = city_name.len().min(MAX_CITY_NAME - 1);
    name[..n].copy_from_slice(&city_name[..n]);

    let node = arena.alloc(Node {
        city_name: name,
        ref_count: 1,
        edges: Vec::new(),
        freed: false,
    });

    // Add to graph
    graph.nodes.push(node);

    Some(node)
}

/// `add_edge`
pub fn add_edge(
    arena: &mut Arena,
    from: Option<NodeId>,
    to: Option<NodeId>,
    distance: i32,
) -> i32 {
    let (from, to) = match (from, to) {
        (Some(f), Some(t)) => (f, t),
        _ => {
            err(b"Error: NULL node in add_edge\n");
            return -1;
        }
    };

    if arena.get(from).edges.len() >= MAX_EDGES {
        let mut msg = Vec::new();
        msg.extend_from_slice(b"Error: Node '");
        msg.extend_from_slice(arena.get(from).name());
        msg.extend_from_slice(b"' has maximum edges\n");
        err(&msg);
        return -1;
    }

    if distance < 0 {
        err(b"Error: Negative distance not allowed\n");
        return -1;
    }

    // Check for duplicate edge
    for edge in &arena.get(from).edges {
        if edge.destination == to {
            err(b"Error: Edge already exists\n");
            return -1;
        }
    }

    // Add edge
    arena.get_mut(from).edges.push(Edge {
        destination: to,
        distance,
    });

    0
}

/// `delete_node`
pub fn delete_node(arena: &mut Arena, node: Option<NodeId>) {
    let node = match node {
        Some(n) => n,
        None => return,
    };

    let n = arena.get_mut(node);
    n.ref_count -= 1;

    if n.ref_count == 0 {
        arena.dealloc(node);
    }
}

/// `increment_refs_recursive`
fn increment_refs_recursive(arena: &mut Arena, node: NodeId, visited: &mut Vec<NodeId>) {
    // Check if already visited
    if visited.contains(&node) {
        return;
    }

    // Mark as visited
    if visited.len() < MAX_NODES {
        visited.push(node);
    }

    // Increment ref count
    arena.get_mut(node).ref_count += 1;

    // Recursively process all connected nodes
    let edge_count = arena.get(node).edges.len();
    for i in 0..edge_count {
        let dest = arena.get(node).edges[i].destination;
        increment_refs_recursive(arena, dest, visited);
    }
}

/// `shallow_copy`
pub fn shallow_copy(arena: &mut Arena, start: Option<NodeId>) -> Option<NodeId> {
    let start = match start {
        Some(s) => s,
        None => {
            err(b"Error: NULL node in shallow_copy\n");
            return None;
        }
    };

    let mut visited: Vec<NodeId> = Vec::new();
    increment_refs_recursive(arena, start, &mut visited);

    Some(start)
}

/// `dijkstra_node_t`
struct DijkstraNode {
    node: NodeId,
    distance: i32,
    previous: Option<NodeId>,
    visited: bool,
}

/// `find_shortest_path`
pub fn find_shortest_path(
    arena: &Arena,
    start: Option<NodeId>,
    end: Option<NodeId>,
    path_length: &mut i32,
) -> Option<Vec<NodeId>> {
    let (start, end) = match (start, end) {
        (Some(s), Some(e)) => (s, e),
        _ => {
            err(b"Error: NULL parameter in find_shortest_path\n");
            return None;
        }
    };

    // Initialize Dijkstra state
    let mut state: Vec<DijkstraNode> = Vec::new();

    // Add start node
    state.push(DijkstraNode {
        node: start,
        distance: 0,
        previous: None,
        visited: false,
    });

    let mut current: Option<NodeId> = Some(start);

    while let Some(cur) = current {
        // Find current node in state
        let mut current_idx: Option<usize> = None;
        for i in 0..state.len() {
            if state[i].node == cur {
                current_idx = Some(i);
                break;
            }
        }

        let current_idx = match current_idx {
            Some(i) => i,
            None => break,
        };

        state[current_idx].visited = true;

        // Check if we reached the end
        if cur == end {
            break;
        }

        // Update distances for neighbors
        let edge_count = arena.get(cur).edges.len();
        for i in 0..edge_count {
            let neighbor = arena.get(cur).edges[i].destination;
            let new_distance = state[current_idx]
                .distance
                .wrapping_add(arena.get(cur).edges[i].distance);

            // Find or add neighbor in state
            let mut neighbor_idx: Option<usize> = None;
            for j in 0..state.len() {
                if state[j].node == neighbor {
                    neighbor_idx = Some(j);
                    break;
                }
            }

            if neighbor_idx.is_none() && state.len() < MAX_NODES {
                // Add new neighbor
                neighbor_idx = Some(state.len());
                state.push(DijkstraNode {
                    node: neighbor,
                    distance: i32::MAX,
                    previous: None,
                    visited: false,
                });
            }

            if let Some(ni) = neighbor_idx {
                if new_distance < state[ni].distance {
                    state[ni].distance = new_distance;
                    state[ni].previous = Some(cur);
                }
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
    let mut end_idx: Option<usize> = None;
    for i in 0..state.len() {
        if state[i].node == end {
            end_idx = Some(i);
            break;
        }
    }

    let no_path = match end_idx {
        None => true,
        Some(i) => state[i].distance == i32::MAX,
    };
    if no_path {
        err(b"No path found\n");
        *path_length = 0;
        return None;
    }

    // Reconstruct path
    let mut path: Vec<NodeId> = Vec::new();
    let mut current_node: Option<NodeId> = Some(end);

    while let Some(cn) = current_node {
        path.push(cn);

        // Find previous node
        let mut current_state_idx: Option<usize> = None;
        for i in 0..state.len() {
            if state[i].node == cn {
                current_state_idx = Some(i);
                break;
            }
        }

        let csi = match current_state_idx {
            Some(i) => i,
            None => break,
        };

        current_node = state[csi].previous;
    }

    // Reverse path
    let count = path.len();
    let mut result: Vec<NodeId> = Vec::with_capacity(count);
    for i in 0..count {
        result.push(path[count - 1 - i]);
    }

    *path_length = count as i32;
    Some(result)
}

/// `get_node_by_name`
pub fn get_node_by_name(
    arena: &Arena,
    graph: Option<&Graph>,
    city_name: Option<&[u8]>,
) -> Option<NodeId> {
    let (graph, city_name) = match (graph, city_name) {
        (Some(g), Some(c)) => (g, c),
        _ => return None,
    };

    for i in 0..graph.node_count() {
        if arena.get(graph.nodes[i]).name_matches(city_name) {
            return Some(graph.nodes[i]);
        }
    }

    None
}

/// `print_node`
pub fn print_node(out: &mut Out, arena: &Arena, node: Option<NodeId>) {
    let node = match node {
        Some(n) => n,
        None => {
            out.write(b"NULL node\n");
            return;
        }
    };

    let n = arena.get(node);

    let mut line = Vec::new();
    line.extend_from_slice(b"City: ");
    line.extend_from_slice(n.name());
    line.extend_from_slice(format!(" (ref_count: {})\n", n.ref_count).as_bytes());
    out.write(&line);
    out.write(b"  Edges:\n");
    for edge in &n.edges {
        let mut line = Vec::new();
        line.extend_from_slice(b"    -> ");
        line.extend_from_slice(arena.get(edge.destination).name());
        line.extend_from_slice(format!(" (distance: {})\n", edge.distance).as_bytes());
        out.write(&line);
    }
}

/// `print_graph`
pub fn print_graph(out: &mut Out, arena: &Arena, graph: Option<&Graph>) {
    let graph = match graph {
        Some(g) => g,
        None => {
            out.write(b"NULL graph\n");
            return;
        }
    };

    out.write(format!("Graph with {} nodes:\n", graph.node_count()).as_bytes());
    for i in 0..graph.node_count() {
        print_node(out, arena, Some(graph.nodes[i]));
    }
}

/// `free_graph`
pub fn free_graph(arena: &mut Arena, graph: Option<Graph>) {
    let graph = match graph {
        Some(g) => g,
        None => return,
    };

    // Decrement ref count for all nodes
    for i in 0..graph.node_count() {
        delete_node(arena, Some(graph.nodes[i]));
    }
}
