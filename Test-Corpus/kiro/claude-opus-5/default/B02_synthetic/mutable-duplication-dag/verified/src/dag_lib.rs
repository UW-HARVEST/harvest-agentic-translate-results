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

/// Result of reading `node->edges[i]` out of the arena.
pub enum EdgeRead {
    /// The destination is the address of a node in the arena.
    Node { dest: usize, distance: i32 },
    /// The destination bytes are all zero, i.e. a NULL `node_t*`.
    Null { distance: i32 },
    /// The destination bytes are not a node address, because `i` is past the
    /// end of the chunk or the chunk was overwritten with something that is not
    /// a node pointer. Dereferencing it kills the C program.
    Wild,
}

/// What the C program does the moment it follows a pointer it read out of a
/// chunk that no longer holds a `node_t`: the value is a chunk header or an
/// address past the end of the heap, so the process dies from `SIGSEGV` and
/// everything still sitting in the stdout buffer is discarded.
pub fn segfault() -> ! {
    // The same dereference the C program performs. `read_volatile` is not
    // optimised away, so this really does raise SIGSEGV.
    unsafe {
        std::ptr::read_volatile(257usize as *const u8);
    }
    // Unreachable in practice; keep the death observable if it ever is.
    std::process::abort()
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
/// * the tcache double-free check,
/// * a freed node chunk being handed to the `malloc` in `find_shortest_path`
///   and overwritten with the path array, which leaves a `node_t` whose fields
///   are pieces of node addresses (see `corrupt`).
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
    /// Raw chunk bytes for slots whose chunk was reused for something that is
    /// not a `node_t`. While an entry is present it, not `slots[i]`, is what
    /// the program reads through its dangling pointer.
    corrupt: std::collections::HashMap<usize, Vec<u8>>,
}

/// Address of the first `node_t` chunk in the reference build (measured with
/// ASLR disabled); the graph and the stdout buffer are allocated before it.
const HEAP_NODE_BASE: u64 = 0x4085f0;

/// `sizeof(node_t)`, the only size this program allocates nodes with.
const NODE_SIZE: usize = 240;

/// Bytes a `node_t` chunk can be read through before it runs into the next
/// chunk's header: `chunk_size(240) - 8`.
const NODE_CHUNK_USABLE: usize = 248;

// Byte offsets of the fields of `node_t` in its chunk, as laid out by the ABI:
// `char city_name[64]; int ref_count; /* 4 bytes padding */ edge_t edges[10];
// int edge_count; /* 4 bytes padding */`, with `edge_t` being an 8-byte pointer,
// a 4-byte int and 4 bytes of padding.
const OFF_REF_COUNT: usize = 64;
const OFF_EDGES: usize = 72;
const EDGE_STRIDE: usize = 16;
const OFF_EDGE_COUNT: usize = 232;

impl Heap {
    pub fn new() -> Heap {
        Heap {
            slots: Vec::new(),
            addrs: Vec::new(),
            top: HEAP_NODE_BASE,
            bins: std::collections::HashMap::new(),
            in_bin: std::collections::HashSet::new(),
            owner: std::collections::HashMap::new(),
            corrupt: std::collections::HashMap::new(),
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
            // add_node overwrites the name, the reference count and the edge
            // count straight away, so a chunk that had been reused for a path
            // array is a plain node again; the stale edges are never read
            // because edge_count is 0.
            self.corrupt.remove(&idx);
            return idx;
        }
        self.slots.push(Node::uninit());
        self.addrs.push(addr);
        let idx = self.slots.len() - 1;
        self.owner.insert(addr, idx);
        idx
    }

    /// `malloc(sizeof(node_t*) * count)` for the path array in
    /// `find_shortest_path`.
    pub fn alloc_scratch(&mut self, request: usize) -> u64 {
        self.alloc_addr(request)
    }

    /// The stores `result[i] = ...` make into the path array. When the array was
    /// handed a recycled node chunk, this is what wrecks that node.
    pub fn write_scratch(&mut self, addr: u64, contents: &[u64]) {
        let Some(&idx) = self.owner.get(&addr) else {
            return;
        };
        let mut image = self.chunk_image(idx);
        for (i, value) in contents.iter().enumerate() {
            let at = i * 8;
            if at + 8 <= NODE_CHUNK_USABLE {
                image[at..at + 8].copy_from_slice(&value.to_le_bytes());
            }
        }
        self.corrupt.insert(idx, image);
    }

    /// `free(path)`. Like [`Heap::free`] this leaves the safe-linked `next`
    /// pointer at the head of the chunk.
    pub fn free_scratch(&mut self, addr: u64, request: usize) {
        let size = Heap::chunk_size(request);
        let bin = self.bins.entry(size).or_default();
        let next = bin.last().copied().unwrap_or(0);
        let protected = (addr >> 12) ^ next;
        bin.push(addr);
        self.in_bin.insert(addr);
        if let Some(&idx) = self.owner.get(&addr) {
            self.write_head(idx, protected);
        }
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

        self.write_head(idx, protected);
    }

    /// The 8 bytes glibc's `free()` writes at the head of the chunk, which alias
    /// `city_name`. (`e->key`, bytes 8..16, is written too, but the protected
    /// pointer always holds a NUL inside the first 8 bytes here, so `%s` never
    /// reaches it.)
    fn write_head(&mut self, idx: usize, protected: u64) {
        let bytes = protected.to_le_bytes();
        match self.corrupt.get_mut(&idx) {
            Some(image) => image[..8].copy_from_slice(&bytes),
            None => self.slots[idx].city_name[..8].copy_from_slice(&bytes),
        }
    }

    /// The chunk of `idx` as raw bytes: either the image kept for a wrecked
    /// chunk, or the bytes the fields of `slots[idx]` occupy. The 8 bytes past
    /// the end of `node_t` are part of the chunk but never written by the
    /// program, and are zero in a chunk that came from fresh heap.
    fn chunk_image(&self, idx: usize) -> Vec<u8> {
        if let Some(image) = self.corrupt.get(&idx) {
            return image.clone();
        }
        let node = &self.slots[idx];
        let mut image = vec![0u8; NODE_CHUNK_USABLE];
        image[..MAX_CITY_NAME].copy_from_slice(&node.city_name);
        image[OFF_REF_COUNT..OFF_REF_COUNT + 4].copy_from_slice(&node.ref_count.to_le_bytes());
        for (i, edge) in node.edges.iter().enumerate() {
            let at = OFF_EDGES + i * EDGE_STRIDE;
            let dest = if edge.destination == NULL {
                0
            } else {
                self.addrs[edge.destination]
            };
            image[at..at + 8].copy_from_slice(&dest.to_le_bytes());
            image[at + 8..at + 12].copy_from_slice(&edge.distance.to_le_bytes());
        }
        image[OFF_EDGE_COUNT..OFF_EDGE_COUNT + 4].copy_from_slice(&node.edge_count.to_le_bytes());
        image
    }

    /// Address of the chunk backing `idx`, i.e. the value of the `node_t*` the
    /// C program holds.
    pub fn addr_of(&self, idx: usize) -> u64 {
        self.addrs[idx]
    }

    fn read_i32(image: &[u8], at: usize) -> i32 {
        i32::from_le_bytes([image[at], image[at + 1], image[at + 2], image[at + 3]])
    }

    fn read_u64(image: &[u8], at: usize) -> u64 {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&image[at..at + 8]);
        u64::from_le_bytes(bytes)
    }

    /// `node->ref_count`.
    pub fn ref_count(&self, idx: usize) -> i32 {
        match self.corrupt.get(&idx) {
            Some(image) => Heap::read_i32(image, OFF_REF_COUNT),
            None => self.slots[idx].ref_count,
        }
    }

    pub fn set_ref_count(&mut self, idx: usize, value: i32) {
        match self.corrupt.get_mut(&idx) {
            Some(image) => {
                image[OFF_REF_COUNT..OFF_REF_COUNT + 4].copy_from_slice(&value.to_le_bytes())
            }
            None => self.slots[idx].ref_count = value,
        }
    }

    /// `node->edge_count`.
    pub fn edge_count(&self, idx: usize) -> i32 {
        match self.corrupt.get(&idx) {
            Some(image) => Heap::read_i32(image, OFF_EDGE_COUNT),
            None => self.slots[idx].edge_count,
        }
    }

    /// `node->edges[i]`. For an intact node this is just the array entry; for a
    /// wrecked chunk the bytes at that offset are pieces of whatever overwrote
    /// it, and past `NODE_CHUNK_USABLE` they belong to the next chunk's header.
    pub fn read_edge(&self, idx: usize, i: usize) -> EdgeRead {
        let Some(image) = self.corrupt.get(&idx) else {
            let edge = self.slots[idx].edges[i];
            return if edge.destination == NULL {
                EdgeRead::Null {
                    distance: edge.distance,
                }
            } else {
                EdgeRead::Node {
                    dest: edge.destination,
                    distance: edge.distance,
                }
            };
        };

        let at = OFF_EDGES + i * EDGE_STRIDE;
        if at + 12 > NODE_CHUNK_USABLE {
            // Past the chunk: the C program reads the next chunk's size field,
            // which is never a node address.
            return EdgeRead::Wild;
        }
        let dest = Heap::read_u64(image, at);
        let distance = Heap::read_i32(image, at + 8);
        if dest == 0 {
            return EdgeRead::Null { distance };
        }
        match self.owner.get(&dest) {
            Some(&slot) => EdgeRead::Node {
                dest: slot,
                distance,
            },
            None => EdgeRead::Wild,
        }
    }

    /// `node->edges[i] = {to, distance}` followed by `node->edge_count++`.
    pub fn push_edge(&mut self, idx: usize, to: usize, distance: i32) {
        let count = self.edge_count(idx);
        match self.corrupt.get_mut(&idx) {
            Some(image) => {
                let at = OFF_EDGES + count as usize * EDGE_STRIDE;
                let dest = self.addrs[to];
                image[at..at + 8].copy_from_slice(&dest.to_le_bytes());
                image[at + 8..at + 12].copy_from_slice(&distance.to_le_bytes());
                image[OFF_EDGE_COUNT..OFF_EDGE_COUNT + 4]
                    .copy_from_slice(&(count + 1).to_le_bytes());
            }
            None => {
                let node = &mut self.slots[idx];
                let slot = count as usize;
                node.edges[slot].destination = to;
                node.edges[slot].distance = distance;
                node.edge_count = count + 1;
            }
        }
    }

    pub fn get_mut(&mut self, idx: usize) -> &mut Node {
        &mut self.slots[idx]
    }

    /// `node->city_name` viewed as a C string (bytes up to the first NUL).
    pub fn name(&self, idx: usize) -> &[u8] {
        match self.corrupt.get(&idx) {
            Some(image) => cstr(&image[..MAX_CITY_NAME]),
            None => cstr(&self.slots[idx].city_name),
        }
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

    if heap.edge_count(from) as usize >= MAX_EDGES {
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
    for i in 0..heap.edge_count(from) as usize {
        let same = match heap.read_edge(from, i) {
            EdgeRead::Node { dest, .. } => dest == to,
            // A NULL or non-node destination cannot be the node main looked up.
            EdgeRead::Null { .. } | EdgeRead::Wild => false,
        };
        if same {
            c.err(b"Error: Edge already exists\n");
            return -1;
        }
    }

    // Add edge
    heap.push_edge(from, to, distance);

    0
}

// Delete a node (decrement ref count, free if 0)
pub fn delete_node(heap: &mut Heap, node: usize, c: &mut Console) {
    if node == NULL {
        return;
    }

    heap.set_ref_count(node, heap.ref_count(node).wrapping_sub(1));

    if heap.ref_count(node) == 0 {
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
    heap.set_ref_count(node, heap.ref_count(node).wrapping_add(1));

    // Recursively process all connected nodes
    for i in 0..heap.edge_count(node) as usize {
        match heap.read_edge(node, i) {
            EdgeRead::Node { dest, .. } => increment_refs_recursive(heap, dest, visited),
            // `if (!node) return;` at the top of the recursion.
            EdgeRead::Null { .. } => {}
            // The C code increments `ref_count` through this pointer.
            EdgeRead::Wild => segfault(),
        }
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
) -> Option<PathArray> {
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
        for i in 0..heap.edge_count(current) as usize {
            let (neighbor, edge_distance) = match heap.read_edge(current, i) {
                EdgeRead::Node { dest, distance } => (dest, distance),
                // A NULL destination is stored in the state array like any
                // other pointer; it ends the search if it is ever selected.
                EdgeRead::Null { distance } => (NULL, distance),
                // The C code walks off the end of the chunk and keeps reading
                // until it leaves the heap.
                EdgeRead::Wild => segfault(),
            };
            // C signed overflow here is UB; gcc/clang emit a wrapping add.
            let new_distance = state[ci].distance.wrapping_add(edge_distance);

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

    // malloc(sizeof(node_t*) * count), then `result[i] = ...` fills it in. If
    // the chunk handed over was recycled from a freed node, those stores land on
    // that node's fields; main frees the array after printing, exactly as the C
    // code does, so the order of the two effects is preserved.
    let scratch = heap.alloc_scratch(count * 8);

    // Reverse path
    let mut result: Vec<usize> = Vec::with_capacity(count);
    for i in 0..count {
        result.push(path[count - 1 - i]);
    }

    let stored: Vec<u64> = result.iter().map(|&idx| heap.addr_of(idx)).collect();
    heap.write_scratch(scratch, &stored);

    *path_length = count as i32;
    Some(PathArray {
        nodes: result,
        addr: scratch,
        request: count * 8,
    })
}

/// The `node_t**` main receives, plus the chunk it lives in so that main can
/// `free()` it where the C code does.
pub struct PathArray {
    pub nodes: Vec<usize>,
    addr: u64,
    request: usize,
}

/// `free(path)` in main, after the path has been printed.
pub fn free_path(heap: &mut Heap, path: PathArray) {
    heap.free_scratch(path.addr, path.request);
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
    m.extend_from_slice(format!(" (ref_count: {})\n", heap.ref_count(node)).as_bytes());
    m.extend_from_slice(b"  Edges:\n");
    for i in 0..heap.edge_count(node) as usize {
        match heap.read_edge(node, i) {
            EdgeRead::Node { dest, distance } => {
                m.extend_from_slice(b"    -> ");
                m.extend_from_slice(heap.name(dest));
                m.extend_from_slice(format!(" (distance: {})\n", distance).as_bytes());
            }
            // glibc's printf prints "(null)" for a NULL %s argument.
            EdgeRead::Null { distance } => {
                m.extend_from_slice(b"    -> (null)");
                m.extend_from_slice(format!(" (distance: {})\n", distance).as_bytes());
            }
            EdgeRead::Wild => {
                // printf copies the literal part of the format into the stream
                // buffer and only then reads the string it cannot reach.
                m.extend_from_slice(b"    -> ");
                c.out(&m);
                segfault();
            }
        }
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
