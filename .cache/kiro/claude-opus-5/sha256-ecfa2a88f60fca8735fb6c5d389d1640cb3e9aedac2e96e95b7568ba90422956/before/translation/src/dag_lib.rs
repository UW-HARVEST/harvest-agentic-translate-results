/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
//! Port of `dag_lib.h` / `dag_lib.c`.
//!
//! The C program deliberately keeps dangling `node_t*` pointers in the graph
//! after `delete_node()` frees a node. To reproduce the observable behaviour of
//! that use-after-free byte for byte, nodes live in an arena (`Heap`) that is
//! never actually reclaimed; a "pointer" is an index into that arena. `Heap`
//! also mimics glibc's LIFO chunk recycling and the bytes glibc's `free()`
//! scribbles over the head of a freed chunk. See `Heap::free` for details.

use crate::cio::Console;

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

/// Stand-in for a C `NULL` node pointer.
pub const NULL: usize = usize::MAX;

#[derive(Clone, Copy)]
pub struct Edge {
    pub destination: usize,
    pub distance: i32,
}

#[derive(Clone)]
pub struct Node {
    pub city_name: [u8; MAX_CITY_NAME],
    pub ref_count: i32,
    pub edges: [Edge; MAX_EDGES],
    pub edge_count: i32,
}

impl Node {
    /// Contents of a freshly `malloc`ed, still uninitialised `node_t`.
    fn uninit() -> Node {
        Node {
            city_name: [0u8; MAX_CITY_NAME],
            ref_count: 0,
            edges: [Edge {
                destination: NULL,
                distance: 0,
            }; MAX_EDGES],
            edge_count: 0,
        }
    }
}

/// Emulation of the C heap for `node_t` allocations.
///
/// Enough of glibc's allocator is modelled to reproduce what the program can
/// observe through its dangling pointers:
///
/// * chunk addresses (a bump pointer into the top chunk plus per-size LIFO
///   recycling bins, i.e. tcache/fastbin behaviour),
/// * the 8 bytes `free()` writes over the head of a freed chunk, which are
///   glibc's safe-linked `next` pointer `(&e->next >> 12) ^ next` and overlap
///   `city_name`,
/// * the tcache double-free check, which this program does trip.
///
/// The base address is the one the reference build uses with ASLR disabled;
/// with ASLR enabled the C program's own output for these reads is random and
/// therefore not reproducible by anything.
pub struct Heap {
    slots: Vec<Node>,
    /// Synthetic chunk address of each slot.
    addrs: Vec<u64>,
    /// Address of the next chunk carved out of the top chunk.
    top: u64,
    /// Per chunk-size recycling bins, most recently freed last.
    bins: std::collections::HashMap<u64, Vec<u64>>,
    /// Chunks currently sitting in a bin (glibc's `e->key == tcache` test).
    in_bin: std::collections::HashSet<u64>,
    /// Slot owning each address, so a recycled chunk keeps its stale contents.
    owner: std::collections::HashMap<u64, usize>,
}

/// Address of the first `node_t` chunk in the reference build (measured with
/// ASLR disabled); the graph and the stdout buffer are allocated before it.
const HEAP_NODE_BASE: u64 = 0x4085f0;

/// `sizeof(node_t)`, the only size this program allocates nodes with.
const NODE_SIZE: usize = 240;

impl Heap {
    pub fn new() -> Heap {
        Heap {
            slots: Vec::new(),
            addrs: Vec::new(),
            top: HEAP_NODE_BASE,
            bins: std::collections::HashMap::new(),
            in_bin: std::collections::HashSet::new(),
            owner: std::collections::HashMap::new(),
        }
    }

    /// glibc chunk size for a request: `max(align16(req + 8), MINSIZE)`.
    fn chunk_size(request: usize) -> u64 {
        let n = ((request + 8 + 15) / 16 * 16) as u64;
        if n < 32 {
            32
        } else {
            n
        }
    }

    /// Address handed out for a request of `request` bytes.
    fn alloc_addr(&mut self, request: usize) -> u64 {
        let size = Heap::chunk_size(request);
        if let Some(bin) = self.bins.get_mut(&size) {
            if let Some(addr) = bin.pop() {
                self.in_bin.remove(&addr);
                return addr;
            }
        }
        let addr = self.top;
        self.top += size;
        addr
    }

    /// `malloc(sizeof(node_t))`. A recycled chunk is returned with its previous
    /// contents intact, exactly like glibc.
    fn malloc(&mut self) -> usize {
        let addr = self.alloc_addr(NODE_SIZE);
        if let Some(&idx) = self.owner.get(&addr) {
            return idx;
        }
        self.slots.push(Node::uninit());
        self.addrs.push(addr);
        let idx = self.slots.len() - 1;
        self.owner.insert(addr, idx);
        idx
    }

    /// `malloc(size)` / `free(ptr)` for the path array in `find_shortest_path`.
    /// Only the effect on later chunk addresses matters.
    pub fn alloc_scratch(&mut self, request: usize) -> u64 {
        self.alloc_addr(request)
    }

    pub fn free_scratch(&mut self, addr: u64, request: usize) {
        let size = Heap::chunk_size(request);
        self.bins.entry(size).or_default().push(addr);
        self.in_bin.insert(addr);
    }

    /// `free(node)`. Pushes the chunk on its bin and leaves glibc's safe-linked
    /// `next` pointer in the first 8 bytes, which alias `city_name`. Freeing a
    /// chunk that is already binned aborts, as glibc's double-free check does.
    fn free(&mut self, idx: usize, c: &mut Console) {
        let addr = self.addrs[idx];
        let size = Heap::chunk_size(NODE_SIZE);

        if self.in_bin.contains(&addr) {
            crate::cio::glibc_abort(c, b"free(): double free detected in tcache 2\n");
        }

        let bin = self.bins.entry(size).or_default();
        let next = bin.last().copied().unwrap_or(0);
        let protected = (addr >> 12) ^ next;
        bin.push(addr);
        self.in_bin.insert(addr);

        // `e->key` (bytes 8..16) is also written, but the protected pointer
        // always contains a NUL within the first 8 bytes here, so `%s` never
        // reaches it.
        self.slots[idx].city_name[..8].copy_from_slice(&protected.to_le_bytes());
    }

    pub fn get(&self, idx: usize) -> &Node {
        &self.slots[idx]
    }

    pub fn get_mut(&mut self, idx: usize) -> &mut Node {
        &mut self.slots[idx]
    }

    /// `node->city_name` viewed as a C string (bytes up to the first NUL).
    pub fn name(&self, idx: usize) -> &[u8] {
        cstr(&self.slots[idx].city_name)
    }
}

/// Bytes of a NUL-terminated C string held in `buf`.
pub fn cstr(buf: &[u8]) -> &[u8] {
    match buf.iter().position(|&b| b == 0) {
        Some(i) => &buf[..i],
        None => buf,
    }
}

pub struct Graph {
    /// `node_t *nodes[MAX_NODES]` as arena indices. Entries are *not* removed
    /// when a node is freed, matching the C code's dangling pointers.
    pub nodes: Vec<usize>,
}

impl Graph {
    pub fn node_count(&self) -> i32 {
        self.nodes.len() as i32
    }
}

// Create a new empty graph
pub fn create_graph() -> Graph {
    Graph {
        nodes: Vec::with_capacity(MAX_NODES),
    }
}

// Add a node to the graph
pub fn add_node(heap: &mut Heap, graph: &mut Graph, city_name: &[u8], c: &mut Console) -> usize {
    // The `!graph || !city_name` guard cannot trigger: main always passes a
    // valid graph and a valid buffer.

    if graph.node_count() as usize >= MAX_NODES {
        c.err(format!("Error: Graph is full (max {} nodes)\n", MAX_NODES).as_bytes());
        return NULL;
    }

    // Check if node already exists
    for i in 0..graph.node_count() as usize {
        if heap.name(graph.nodes[i]) == city_name {
            let mut m = Vec::new();
            m.extend_from_slice(b"Error: Node '");
            m.extend_from_slice(city_name);
            m.extend_from_slice(b"' already exists\n");
            c.err(&m);
            return NULL;
        }
    }

    // Allocate new node
    let idx = heap.malloc();

    // Initialize node: strncpy(dst, src, MAX_CITY_NAME - 1) copies at most 63
    // bytes and NUL-pads the remainder of those 63; byte 63 is then cleared.
    {
        let node = heap.get_mut(idx);
        let n = MAX_CITY_NAME - 1;
        let copied = if city_name.len() < n {
            city_name.len()
        } else {
            n
        };
        node.city_name[..copied].copy_from_slice(&city_name[..copied]);
        for b in node.city_name[copied..n].iter_mut() {
            *b = 0;
        }
        node.city_name[MAX_CITY_NAME - 1] = 0;
        node.ref_count = 1;
        node.edge_count = 0;
    }

    // Add to graph
    graph.nodes.push(idx);

    idx
}

// Add an edge between two nodes
pub fn add_edge(heap: &mut Heap, from: usize, to: usize, distance: i32, c: &mut Console) -> i32 {
    // The `!from || !to` guard cannot trigger: main checks both lookups first.

    if heap.get(from).edge_count as usize >= MAX_EDGES {
        let mut m = Vec::new();
        m.extend_from_slice(b"Error: Node '");
        m.extend_from_slice(heap.name(from));
        m.extend_from_slice(b"' has maximum edges\n");
        c.err(&m);
        return -1;
    }

    if distance < 0 {
        c.err(b"Error: Negative distance not allowed\n");
        return -1;
    }

    // Check for duplicate edge
    for i in 0..heap.get(from).edge_count as usize {
        if heap.get(from).edges[i].destination == to {
            c.err(b"Error: Edge already exists\n");
            return -1;
        }
    }

    // Add edge
    let node = heap.get_mut(from);
    let slot = node.edge_count as usize;
    node.edges[slot].destination = to;
    node.edges[slot].distance = distance;
    node.edge_count += 1;

    0
}

// Delete a node (decrement ref count, free if 0)
pub fn delete_node(heap: &mut Heap, node: usize, c: &mut Console) {
    if node == NULL {
        return;
    }

    heap.get_mut(node).ref_count -= 1;

    if heap.get(node).ref_count == 0 {
        heap.free(node, c);
    }
}

// Helper function to increment ref count recursively
fn increment_refs_recursive(heap: &mut Heap, node: usize, visited: &mut Vec<usize>) {
    if node == NULL {
        return;
    }

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
    heap.get_mut(node).ref_count += 1;

    // Recursively process all connected nodes
    for i in 0..heap.get(node).edge_count as usize {
        let dest = heap.get(node).edges[i].destination;
        increment_refs_recursive(heap, dest, visited);
    }
}

// Create shallow copy of subsection (increments ref counts)
pub fn shallow_copy(heap: &mut Heap, start: usize, c: &mut Console) -> usize {
    if start == NULL {
        c.err(b"Error: NULL node in shallow_copy\n");
        return NULL;
    }

    // Track visited nodes to avoid cycles
    let mut visited: Vec<usize> = Vec::with_capacity(MAX_NODES);

    // Increment ref counts for all reachable nodes
    increment_refs_recursive(heap, start, &mut visited);

    start
}

// Helper structure for shortest path algorithm
struct DijkstraNode {
    node: usize,
    distance: i32,
    previous: usize,
    visited: i32,
}

// Find shortest path using Dijkstra's algorithm
pub fn find_shortest_path(
    heap: &mut Heap,
    start: usize,
    end: usize,
    path_length: &mut i32,
    c: &mut Console,
) -> Option<Vec<usize>> {
    // The `!start || !end || !path_length` guard cannot trigger from main.

    // Initialize Dijkstra state
    let mut state: Vec<DijkstraNode> = Vec::with_capacity(MAX_NODES);

    // Add start node
    state.push(DijkstraNode {
        node: start,
        distance: 0,
        previous: NULL,
        visited: 0,
    });

    let mut current = start;

    while current != NULL {
        // Find current node in state
        let mut current_idx: isize = -1;
        for i in 0..state.len() {
            if state[i].node == current {
                current_idx = i as isize;
                break;
            }
        }

        if current_idx == -1 {
            break;
        }
        let ci = current_idx as usize;

        state[ci].visited = 1;

        // Check if we reached the end
        if current == end {
            break;
        }

        // Update distances for neighbors
        for i in 0..heap.get(current).edge_count as usize {
            let neighbor = heap.get(current).edges[i].destination;
            // C signed overflow here is UB; gcc/clang emit a wrapping add.
            let new_distance = state[ci]
                .distance
                .wrapping_add(heap.get(current).edges[i].distance);

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
                    previous: NULL,
                    visited: 0,
                });
            }

            if neighbor_idx != -1 {
                let ni = neighbor_idx as usize;
                if new_distance < state[ni].distance {
                    state[ni].distance = new_distance;
                    state[ni].previous = current;
                }
            }
        }

        // Find next unvisited node with minimum distance
        let mut min_distance = i32::MAX;
        current = NULL;
        for i in 0..state.len() {
            if state[i].visited == 0 && state[i].distance < min_distance {
                min_distance = state[i].distance;
                current = state[i].node;
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
        c.err(b"No path found\n");
        *path_length = 0;
        return None;
    }

    // Reconstruct path
    let mut path: Vec<usize> = Vec::with_capacity(MAX_NODES);
    let mut current_node = end;

    while current_node != NULL {
        path.push(current_node);

        // Find previous node
        let mut current_state_idx: isize = -1;
        for i in 0..state.len() {
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

    // malloc(sizeof(node_t*) * count). main frees it right after printing and
    // nothing is allocated in between, so the pair is modelled here; only its
    // effect on later chunk addresses is observable.
    let scratch = heap.alloc_scratch(count * 8);
    heap.free_scratch(scratch, count * 8);

    // Reverse path
    let mut result: Vec<usize> = Vec::with_capacity(count);
    for i in 0..count {
        result.push(path[count - 1 - i]);
    }

    *path_length = count as i32;
    Some(result)
}

// Get node by city name
pub fn get_node_by_name(heap: &Heap, graph: &Graph, city_name: &[u8]) -> usize {
    for i in 0..graph.node_count() as usize {
        if heap.name(graph.nodes[i]) == city_name {
            return graph.nodes[i];
        }
    }

    NULL
}

// Print node information
pub fn print_node(heap: &Heap, node: usize, c: &mut Console) {
    if node == NULL {
        c.out(b"NULL node\n");
        return;
    }

    let mut m = Vec::new();
    m.extend_from_slice(b"City: ");
    m.extend_from_slice(heap.name(node));
    m.extend_from_slice(format!(" (ref_count: {})\n", heap.get(node).ref_count).as_bytes());
    m.extend_from_slice(b"  Edges:\n");
    for i in 0..heap.get(node).edge_count as usize {
        let edge = heap.get(node).edges[i];
        m.extend_from_slice(b"    -> ");
        m.extend_from_slice(heap.name(edge.destination));
        m.extend_from_slice(format!(" (distance: {})\n", edge.distance).as_bytes());
    }
    c.out(&m);
}

// Print entire graph
pub fn print_graph(heap: &Heap, graph: &Graph, c: &mut Console) {
    c.out(format!("Graph with {} nodes:\n", graph.node_count()).as_bytes());
    for i in 0..graph.node_count() as usize {
        print_node(heap, graph.nodes[i], c);
    }
}

// Free the entire graph
pub fn free_graph(heap: &mut Heap, graph: &Graph, c: &mut Console) {
    // Decrement ref count for all nodes
    for i in 0..graph.node_count() as usize {
        delete_node(heap, graph.nodes[i], c);
    }

    // free(graph) has no observable effect here: nothing is allocated after it.
}
