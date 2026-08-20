//! Translation of `c_src/src/lib.c` / `c_src/include/dag_lib.h`.
//!
//! The C code identifies nodes by their heap address and keeps dangling
//! pointers in `graph->nodes[]` after `delete_node()` frees a node.  To model
//! that faithfully (including the fact that a subsequent `malloc()` of the very
//! same size class hands the freed chunk back out, so the "old" pointer and the
//! "new" pointer alias) nodes live in an arena and are referenced by index; the
//! free lists below reproduce the order in which glibc recycles the chunks.
//!
//! A node whose storage has been released keeps its payload (there is nothing
//! deterministic to put there instead) but it can no longer be found by name:
//! in the real program the first 16 bytes of `city_name` are overwritten by the
//! allocator's book-keeping, so `strcmp()` against it never matches.
//!
//! The `!graph` / `!city_name` / `!node` guards of the C functions are omitted
//! where the caller can never pass a null pointer; the remaining checks keep
//! their original order.

use crate::cio::{die_abort, die_signal, err, Out};
use std::collections::VecDeque;

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

/// `node_t *` equivalent: an index into `Graph::arena`.
pub type NodeRef = usize;

#[derive(Clone, Copy)]
pub struct Edge {
    pub destination: NodeRef,
    pub distance: i32,
}

impl Edge {
    const fn zeroed() -> Edge {
        Edge {
            destination: usize::MAX,
            distance: 0,
        }
    }
}

pub struct Node {
    /// Contents of `char city_name[MAX_CITY_NAME]` up to the terminating NUL.
    pub city_name: Vec<u8>,
    pub ref_count: i32,
    pub edges: [Edge; MAX_EDGES],
    pub edge_count: i32,
    /// True while the node's storage is on the allocator's free list.
    pub freed: bool,
}

pub struct Graph {
    /// `node_t *nodes[MAX_NODES]` (only the first `node_count` are meaningful).
    pub nodes: Vec<NodeRef>,
    pub node_count: i32,
    /// Backing storage for every node ever allocated.
    arena: Vec<Node>,
    /// Freed chunks held in the allocator's per-size-class cache (LIFO, at most
    /// `TCACHE_COUNT` entries, like glibc's tcache).
    tcache: Vec<NodeRef>,
    /// Freed chunks that did not fit in the cache; they are handed back out in
    /// the order they were released (glibc's unsorted bin behaviour).
    unsorted: VecDeque<NodeRef>,
}

/// glibc's default `mp_.tcache_count`.
const TCACHE_COUNT: usize = 7;

impl Graph {
    /// `create_graph()`
    pub fn create_graph() -> Graph {
        Graph {
            nodes: vec![usize::MAX; MAX_NODES],
            node_count: 0,
            arena: Vec::new(),
            tcache: Vec::new(),
            unsorted: VecDeque::new(),
        }
    }

    pub fn node(&self, r: NodeRef) -> &Node {
        &self.arena[r]
    }

    fn node_mut(&mut self, r: NodeRef) -> &mut Node {
        &mut self.arena[r]
    }

    /// `malloc(sizeof(node_t))`: hands back the most recently freed chunk if
    /// there is one, otherwise fresh storage.
    fn alloc_node(&mut self, city_name: &[u8]) -> NodeRef {
        // strncpy(node->city_name, city_name, MAX_CITY_NAME - 1);
        // node->city_name[MAX_CITY_NAME - 1] = '\0';
        let copied = city_name.len().min(MAX_CITY_NAME - 1);
        let name: Vec<u8> = city_name[..copied].to_vec();
        let recycled = self.tcache.pop().or_else(|| self.unsorted.pop_front());
        match recycled {
            Some(r) => {
                let node = &mut self.arena[r];
                node.city_name = name;
                node.ref_count = 1;
                node.edge_count = 0;
                node.freed = false;
                r
            }
            None => {
                self.arena.push(Node {
                    city_name: name,
                    ref_count: 1,
                    edges: [Edge::zeroed(); MAX_EDGES],
                    edge_count: 0,
                    freed: false,
                });
                self.arena.len() - 1
            }
        }
    }

    /// `add_node()`
    pub fn add_node(&mut self, city_name: &[u8]) -> Option<NodeRef> {
        if self.node_count >= MAX_NODES as i32 {
            err(format!("Error: Graph is full (max {} nodes)\n", MAX_NODES).as_bytes());
            return None;
        }

        // Check if node already exists
        for i in 0..self.node_count as usize {
            let r = self.nodes[i];
            let node = &self.arena[r];
            if !node.freed && node.city_name.as_slice() == city_name {
                err(b"Error: Node '");
                err(city_name);
                err(b"' already exists\n");
                return None;
            }
        }

        let r = self.alloc_node(city_name);

        // Add to graph
        let idx = self.node_count as usize;
        self.nodes[idx] = r;
        self.node_count += 1;

        Some(r)
    }

    /// `add_edge()`
    pub fn add_edge(&mut self, from: NodeRef, to: NodeRef, distance: i32) -> i32 {
        if self.arena[from].edge_count >= MAX_EDGES as i32 {
            err(b"Error: Node '");
            let name = self.arena[from].city_name.clone();
            err(&name);
            err(b"' has maximum edges\n");
            return -1;
        }

        if distance < 0 {
            err(b"Error: Negative distance not allowed\n");
            return -1;
        }

        // Check for duplicate edge
        for i in 0..self.arena[from].edge_count as usize {
            if self.arena[from].edges[i].destination == to {
                err(b"Error: Edge already exists\n");
                return -1;
            }
        }

        // Add edge
        let node = self.node_mut(from);
        let slot = node.edge_count as usize;
        node.edges[slot] = Edge {
            destination: to,
            distance,
        };
        node.edge_count += 1;

        0
    }

    /// `delete_node()`
    pub fn delete_node(&mut self, r: NodeRef) {
        let node = &mut self.arena[r];
        node.ref_count = node.ref_count.wrapping_sub(1);

        if node.ref_count == 0 {
            // glibc notices a chunk that is still sitting in the per-size-class
            // cache being handed to free() a second time and aborts.
            if self.tcache.contains(&r) {
                err(b"free(): double free detected in tcache 2\n");
                die_abort();
            }
            self.arena[r].freed = true;
            if self.tcache.len() < TCACHE_COUNT {
                self.tcache.push(r);
            } else {
                self.unsorted.push_back(r);
            }
        }
    }

    /// `increment_refs_recursive()`
    fn increment_refs_recursive(&mut self, r: NodeRef, visited: &mut Vec<NodeRef>) {
        // Check if already visited
        for i in 0..visited.len() {
            if visited[i] == r {
                return;
            }
        }

        // Mark as visited
        if visited.len() < MAX_NODES {
            visited.push(r);
        }

        // Increment ref count
        {
            let node = &mut self.arena[r];
            node.ref_count = node.ref_count.wrapping_add(1);
        }

        // Recursively process all connected nodes
        for i in 0..self.arena[r].edge_count as usize {
            let dest = self.arena[r].edges[i].destination;
            self.increment_refs_recursive(dest, visited);
        }
    }

    /// `shallow_copy()`
    pub fn shallow_copy(&mut self, start: NodeRef) -> Option<NodeRef> {
        let mut visited: Vec<NodeRef> = Vec::new();
        self.increment_refs_recursive(start, &mut visited);
        Some(start)
    }

    /// `find_shortest_path()`.  Returns the path and its length; the length is
    /// set to 0 when no path exists (mirroring `*path_length = 0`).
    pub fn find_shortest_path(&self, start: NodeRef, end: NodeRef) -> (Option<Vec<NodeRef>>, i32) {
        struct DijkstraNode {
            node: NodeRef,
            distance: i32,
            previous: Option<NodeRef>,
            visited: bool,
        }

        let mut state: Vec<DijkstraNode> = Vec::with_capacity(MAX_NODES);

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
            let mut current_idx: i32 = -1;
            for i in 0..state.len() {
                if state[i].node == cur {
                    current_idx = i as i32;
                    break;
                }
            }

            if current_idx == -1 {
                break;
            }
            let current_idx = current_idx as usize;

            state[current_idx].visited = true;

            // Check if we reached the end
            if cur == end {
                break;
            }

            // Update distances for neighbors
            for i in 0..self.arena[cur].edge_count as usize {
                let neighbor = self.arena[cur].edges[i].destination;
                let new_distance = state[current_idx]
                    .distance
                    .wrapping_add(self.arena[cur].edges[i].distance);

                // Find or add neighbor in state
                let mut neighbor_idx: i32 = -1;
                for j in 0..state.len() {
                    if state[j].node == neighbor {
                        neighbor_idx = j as i32;
                        break;
                    }
                }

                if neighbor_idx == -1 && state.len() < MAX_NODES {
                    // Add new neighbor
                    neighbor_idx = state.len() as i32;
                    state.push(DijkstraNode {
                        node: neighbor,
                        distance: i32::MAX,
                        previous: None,
                        visited: false,
                    });
                }

                if neighbor_idx != -1 && new_distance < state[neighbor_idx as usize].distance {
                    let ni = neighbor_idx as usize;
                    state[ni].distance = new_distance;
                    state[ni].previous = Some(cur);
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
        let mut end_idx: i32 = -1;
        for i in 0..state.len() {
            if state[i].node == end {
                end_idx = i as i32;
                break;
            }
        }

        if end_idx == -1 || state[end_idx as usize].distance == i32::MAX {
            err(b"No path found\n");
            return (None, 0);
        }

        // Reconstruct path.
        //
        // `node_t *path[MAX_NODES]` is a fixed size array on the stack and the
        // `previous` links are not guaranteed to be acyclic (a distance
        // computation that overflows can make a node its own predecessor), so
        // this loop can write past the end of the array.  In the compiled
        // program `path[MAX_NODES]` is exactly `state[0].node`, i.e. the writes
        // walk over the Dijkstra state array that follows it on the stack:
        //
        //   path[100 + 4*k + 0] -> state[k].node
        //   path[100 + 4*k + 1] -> state[k].distance (+ padding)
        //   path[100 + 4*k + 2] -> state[k].previous
        //   path[100 + 4*k + 3] -> state[k].visited  (+ padding)
        //
        // Only `node` and `previous` are read again, so the clobbering of the
        // other two fields is invisible.  Past `state` come a few unused slots
        // and finally `count` itself (`path[507]`), and once that holds a heap
        // address the next write lands far outside the stack and the process
        // dies from a fatal signal.
        const STATE_SLOTS: usize = 4 * MAX_NODES;
        const COUNT_SLOT: usize = 507;

        let mut path: Vec<NodeRef> = Vec::with_capacity(MAX_NODES);
        let mut current_node: Option<NodeRef> = Some(end);

        while let Some(cn) = current_node {
            let slot = path.len();
            path.push(cn);

            if slot >= MAX_NODES {
                let offset = slot - MAX_NODES;
                if offset < STATE_SLOTS {
                    let k = offset / 4;
                    let field = offset % 4;
                    if k < state.len() {
                        match field {
                            0 => state[k].node = cn,
                            2 => state[k].previous = Some(cn),
                            _ => {}
                        }
                    }
                } else if slot >= COUNT_SLOT {
                    die_signal();
                }
            }

            // Find previous node
            let mut current_state_idx: i32 = -1;
            for i in 0..state.len() {
                if state[i].node == cn {
                    current_state_idx = i as i32;
                    break;
                }
            }

            if current_state_idx == -1 {
                break;
            }

            current_node = state[current_state_idx as usize].previous;
        }

        // Reverse path
        let count = path.len();
        if count > MAX_NODES + STATE_SLOTS {
            // The entries stored beyond the state array read back as leftover
            // stack contents, which are not reproducible.
            die_signal();
        }
        let mut result: Vec<NodeRef> = Vec::with_capacity(count);
        for i in 0..count {
            result.push(path[count - 1 - i]);
        }

        (Some(result), count as i32)
    }

    /// `get_node_by_name()`
    pub fn get_node_by_name(&self, city_name: &[u8]) -> Option<NodeRef> {
        for i in 0..self.node_count as usize {
            let r = self.nodes[i];
            let node = &self.arena[r];
            if !node.freed && node.city_name.as_slice() == city_name {
                return Some(r);
            }
        }

        None
    }

    /// `print_node()`
    pub fn print_node(&self, out: &mut Out, r: NodeRef) {
        let node = &self.arena[r];
        out.s("City: ");
        out.write(&node.city_name);
        out.write(format!(" (ref_count: {})\n", node.ref_count).as_bytes());
        out.s("  Edges:\n");
        for i in 0..node.edge_count as usize {
            let edge = node.edges[i];
            out.s("    -> ");
            out.write(&self.arena[edge.destination].city_name);
            out.write(format!(" (distance: {})\n", edge.distance).as_bytes());
        }
    }

    /// `print_graph()`
    pub fn print_graph(&self, out: &mut Out) {
        out.write(format!("Graph with {} nodes:\n", self.node_count).as_bytes());
        for i in 0..self.node_count as usize {
            self.print_node(out, self.nodes[i]);
        }
    }

    /// `free_graph()`
    pub fn free_graph(&mut self) {
        // Decrement ref count for all nodes
        for i in 0..self.node_count as usize {
            let r = self.nodes[i];
            self.delete_node(r);
        }
    }
}
