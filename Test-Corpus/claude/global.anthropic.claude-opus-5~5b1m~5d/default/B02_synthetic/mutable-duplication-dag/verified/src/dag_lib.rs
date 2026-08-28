//! Translation of `c_src/src/lib.c` / `c_src/include/dag_lib.h`.
//!
//! The C code hands out raw `node_t *` pointers, keeps them inside the graph
//! and (in `delete_node`) `free()`s a node while the graph still holds the
//! pointer.  Here every node lives in an arena (`Arena`) and a `node_t *` is
//! modelled as an index into that arena (`NodeRef`), so node identity
//! comparisons (`from->edges[i].destination == to`) keep working.
//!
//! Because the C program keeps *using* freed nodes, the arena also has to model
//! what glibc's `malloc`/`free` do to a freed `node_t` - see `Arena` for the
//! details.  Those effects are observable in the program's output and are
//! deterministic, so they are reproduced here.

use crate::cio::{eput, malloc_printerr, COut};

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
    /// `true` while the chunk is on one of the allocator's free lists, i.e.
    /// between the `free()` in `delete_node` and the next `malloc()` that
    /// hands the very same chunk out again.
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

/// glibc's `TCACHE_COUNT`: a size class caches at most 7 freed chunks.
const TCACHE_COUNT: usize = 7;

/// What glibc's `free()` leaves in the first 16 bytes of the user data of a
/// chunk it puts on a free list: `tcache_entry { next, key }` (or `fd`/`bk` for
/// the unsorted bin).  Those 16 bytes overlap `node_t::city_name`, which is why
/// a freed node's name turns into garbage and `strcmp()` in
/// `get_node_by_name()`/`add_node()` stops matching it.  `ref_count` (offset
/// 64) and `edges`/`edge_count` (offset 72 onwards) are *not* touched by
/// `free()`, so they keep their values - the C program relies on that.
///
/// The real bytes are heap addresses (`next` is `PROTECT_PTR(NULL)`, i.e.
/// `&next >> 12`), so they differ on every run because of ASLR; the placeholder
/// below only has to have the same shape: non-empty, NUL-terminated inside the
/// first 8 bytes and never equal to a city name a user could type.
const FREE_LIST_METADATA: [u8; 16] = [
    0x0e, 0x8e, 0x38, 0x00, 0x00, 0x00, 0x00, 0x00, // next = PROTECT_PTR(NULL)
    0xa0, 0x92, 0x55, 0x55, 0x55, 0x55, 0x00, 0x00, // key  = &tcache
];

/// A maximal run of adjacent free chunks, `lo..=hi`, in address order - which
/// is arena index order, because chunks are carved out of the top chunk from
/// low to high.  glibc coalesces neighbouring free chunks, so its bins hold
/// runs like this rather than individual chunks.
#[derive(Clone, Copy)]
struct Run {
    lo: NodeRef,
    hi: NodeRef,
}

/// Backing store for every `malloc`ed `node_t`: one entry per distinct heap
/// chunk, so a `NodeRef` behaves like the `node_t *` it stands for - including
/// the fact that a chunk which is freed and allocated again yields the *same*
/// pointer, which the C program's output exposes.
///
/// Every `node_t` lands in the same glibc size class (`sizeof(node_t)` is 240,
/// so the chunk is 256 bytes), and this is what glibc 2.34 does with such a
/// chunk - each rule below was checked against the C program:
///
///   * `free()` puts the chunk on the tcache bin as it is, while that bin holds
///     fewer than `TCACHE_COUNT` chunks, so `malloc()` hands the most recently
///     freed chunk back first;
///   * with a full tcache bin the chunk is coalesced with the adjacent free
///     runs (a chunk in the tcache still looks allocated and never takes part)
///     and then either absorbed into the top chunk, if it borders it, or put at
///     the head of the unsorted bin;
///   * `malloc()` with an empty tcache bin empties the unsorted bin oldest
///     first, moving same-sized chunks into the tcache and sorting bigger runs
///     into their size bin, and then returns the *last* chunk it cached - so
///     the newest of those chunks comes back first;
///   * a run that is too big is taken from its bin (smallest first) and split,
///     the caller gets its lowest chunk and the rest goes back to the unsorted
///     bin;
///   * `free()` writes free list metadata (`tcache_entry`, or `fd`/`bk`) over
///     the first 16 bytes of the chunk's user data, i.e. over the start of
///     `city_name` - but only for the chunk that actually goes on a list: a
///     chunk absorbed into the top chunk, or one that is merged into the middle
///     or the end of a run, keeps its name and stays findable by
///     `get_node_by_name()`;
///   * freeing a chunk that is already free is a double free: glibc prints a
///     diagnostic and aborts instead of returning.
pub struct Arena {
    pub nodes: Vec<Node>,
    /// How many chunks are carved out of the top chunk.  Freeing the highest
    /// chunk absorbs it back into the top chunk, which moves this down again;
    /// `nodes[top..]` is the storage inside the top chunk, which the C program
    /// can still read through a stale pointer.
    top: usize,
    /// The tcache bin: LIFO, at most `TCACHE_COUNT` chunks.
    tcache: Vec<NodeRef>,
    /// The unsorted bin, oldest run first.
    unsorted: Vec<Run>,
    /// Runs that were sorted into their size bin, oldest first.
    binned: Vec<Run>,
}

impl Arena {
    pub fn new() -> Arena {
        Arena {
            nodes: Vec::new(),
            top: 0,
            tcache: Vec::new(),
            unsorted: Vec::new(),
            binned: Vec::new(),
        }
    }

    /// Overwrite the start of `city_name` with free list metadata.
    fn write_free_list_metadata(&mut self, node: NodeRef) {
        self.nodes[node].city_name[..FREE_LIST_METADATA.len()]
            .copy_from_slice(&FREE_LIST_METADATA);
    }

    /// Take the free run that ends just below `chunk` out of its bin.
    fn take_run_below(&mut self, chunk: NodeRef) -> Option<Run> {
        if chunk == 0 {
            return None;
        }
        let wanted = chunk - 1;
        if let Some(i) = self.unsorted.iter().position(|r| r.hi == wanted) {
            return Some(self.unsorted.remove(i));
        }
        if let Some(i) = self.binned.iter().position(|r| r.hi == wanted) {
            return Some(self.binned.remove(i));
        }
        None
    }

    /// Take the free run that starts just above `chunk` out of its bin.
    fn take_run_above(&mut self, chunk: NodeRef) -> Option<Run> {
        let wanted = chunk + 1;
        if let Some(i) = self.unsorted.iter().position(|r| r.lo == wanted) {
            return Some(self.unsorted.remove(i));
        }
        if let Some(i) = self.binned.iter().position(|r| r.lo == wanted) {
            return Some(self.binned.remove(i));
        }
        None
    }

    /// `malloc(sizeof(node_t))`: reuse a chunk from the bins if there is one,
    /// otherwise carve a fresh one out of the top chunk.
    fn malloc_node(&mut self) -> NodeRef {
        if let Some(node) = self.tcache.pop() {
            return node;
        }

        // Empty the unsorted bin, oldest run first.
        let unsorted = std::mem::take(&mut self.unsorted);
        let mut cached = false;
        let mut exact: Option<NodeRef> = None;
        for run in unsorted {
            if exact.is_some() {
                // The walk stopped early; the rest of the bin stays as it is.
                self.unsorted.push(run);
            } else if run.lo == run.hi {
                if self.tcache.len() < TCACHE_COUNT {
                    self.tcache.push(run.lo);
                    cached = true;
                } else {
                    // The tcache filled up, so this one goes to the caller.
                    exact = Some(run.lo);
                }
            } else {
                self.binned.push(run);
            }
        }
        if let Some(node) = exact {
            return node;
        }
        if cached {
            return self.tcache.pop().expect("just cached a chunk");
        }

        // Nothing of the right size: take the smallest run that is big enough
        // (oldest first among equals) and split it.
        if !self.binned.is_empty() {
            let mut best = 0;
            for i in 1..self.binned.len() {
                if self.binned[i].hi - self.binned[i].lo < self.binned[best].hi - self.binned[best].lo
                {
                    best = i;
                }
            }
            let run = self.binned.remove(best);
            if run.hi > run.lo {
                let remainder = Run {
                    lo: run.lo + 1,
                    hi: run.hi,
                };
                self.write_free_list_metadata(remainder.lo);
                self.unsorted.push(remainder);
            }
            return run.lo;
        }

        // Carve from the top chunk.
        if self.top < self.nodes.len() {
            let node = self.top;
            self.top += 1;
            return node;
        }
        self.nodes.push(Node {
            city_name: [0u8; MAX_CITY_NAME],
            ref_count: 0,
            edges: Vec::new(),
            edge_count: 0,
            freed: false,
        });
        self.top = self.nodes.len();
        self.nodes.len() - 1
    }

    /// `free(node)`.
    fn free_node(&mut self, node: NodeRef) {
        if self.nodes[node].freed {
            self.report_double_free(node);
        }
        self.nodes[node].freed = true;

        // The tcache takes the chunk on its own, without looking at neighbours.
        if self.tcache.len() < TCACHE_COUNT {
            self.write_free_list_metadata(node);
            self.tcache.push(node);
            return;
        }

        // Otherwise coalesce with the adjacent free runs.
        let mut lo = node;
        let mut hi = node;
        if let Some(below) = self.take_run_below(lo) {
            lo = below.lo;
        }
        if let Some(above) = self.take_run_above(hi) {
            hi = above.hi;
        }

        if hi + 1 == self.top {
            // Borders the top chunk, so it is absorbed into it.  Only the chunk
            // *header* is rewritten, so every name in the absorbed run survives.
            self.top = lo;
        } else {
            // Goes to the unsorted bin: `fd`/`bk` land in the run's lowest
            // chunk, the names of the others survive.
            self.write_free_list_metadata(lo);
            self.unsorted.push(Run { lo, hi });
        }
    }

    /// glibc's diagnostics for freeing a chunk that is already free.  Which one
    /// it is depends on where the chunk sits.
    fn report_double_free(&self, node: NodeRef) -> ! {
        if self.tcache.contains(&node) {
            malloc_printerr(b"free(): double free detected in tcache 2\n");
        }
        if node >= self.top {
            // Inside the top chunk: the size read from the header covers the
            // whole rest of the heap, so the "is the next chunk still inside
            // the arena" check is what fails.
            malloc_printerr(b"double free or corruption (out)\n");
        }
        // On a bin: the next chunk's PREV_INUSE bit is clear.
        malloc_printerr(b"double free or corruption (!prev)\n")
    }

    /// `malloc(sizeof(node_t))` + the initialisation performed by `add_node`.
    fn alloc_node(&mut self, city_name: &[u8]) -> NodeRef {
        let node = self.malloc_node();

        // `malloc()` does not clear the chunk, but `strncpy(dst, src, 63)`
        // writes all 63 bytes (NUL padded) and `city_name[63] = '\0'` the last
        // one, so the whole array is overwritten either way.
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

        let slot = &mut self.nodes[node];
        slot.city_name = buf;
        slot.ref_count = 1;
        // `edge_count = 0` hides the stale `edges` left over from the previous
        // life of the chunk, so dropping them changes nothing observable.
        slot.edges.clear();
        slot.edge_count = 0;
        slot.freed = false;

        node
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
        arena.free_node(node);
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

/// The value a `node_t *` in an untouched `state` slot stands for: such a slot
/// is never compared against a real node, because only `state[..state_count]`
/// is ever searched.
const NO_NODE: NodeRef = usize::MAX;

// Helper structure for shortest path algorithm
#[derive(Clone, Copy)]
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
    // dijkstra_node_t state[MAX_NODES];
    //
    // This has to be a fixed size array with a separate counter, exactly like
    // the C: the path reconstruction below writes past its own array and into
    // *these* slots, including slots beyond `state_count`.
    let mut state = [DijkstraNode {
        node: NO_NODE,
        distance: 0,
        previous: None,
        visited: false,
    }; MAX_NODES];
    let mut state_count: usize = 0;

    // Add start node
    state[state_count].node = start;
    state[state_count].distance = 0;
    state[state_count].previous = None;
    state[state_count].visited = false;
    state_count += 1;

    let mut current: Option<NodeRef> = Some(start);

    while let Some(cur) = current {
        // Find current node in state
        let mut current_idx: isize = -1;
        for i in 0..state_count {
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
            for j in 0..state_count {
                if state[j].node == neighbor {
                    neighbor_idx = j as isize;
                    break;
                }
            }

            if neighbor_idx == -1 && state_count < MAX_NODES {
                // Add new neighbor
                neighbor_idx = state_count as isize;
                state[state_count].node = neighbor;
                state[state_count].distance = i32::MAX;
                state[state_count].previous = None;
                state[state_count].visited = false;
                state_count += 1;
            }

            if neighbor_idx != -1 && new_distance < state[neighbor_idx as usize].distance {
                state[neighbor_idx as usize].distance = new_distance;
                state[neighbor_idx as usize].previous = Some(cur);
            }
        }

        // Find next unvisited node with minimum distance
        let mut min_distance = i32::MAX;
        current = None;
        for i in 0..state_count {
            if !state[i].visited && state[i].distance < min_distance {
                min_distance = state[i].distance;
                current = Some(state[i].node);
            }
        }
    }

    // Find end node in state
    let mut end_idx: isize = -1;
    for i in 0..state_count {
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
    //
    // `node_t *path[MAX_NODES]` is filled through `path[count++]` with no bound
    // check at all.  The `previous` chain *can* be longer than MAX_NODES: an
    // overflowed (negative) distance is able to relax a node that is already
    // visited, and that rewrites its `previous`, which can make the chain
    // cyclic.  gcc lays the frame out with `path` directly below `state`, so
    // `&path[MAX_NODES] == &state[0]` and the writes past the end of `path`
    // land in the state array, one 8-byte word at a time:
    //
    //     path[MAX_NODES + 4*s + 0]  ->  state[s].node
    //     path[MAX_NODES + 4*s + 1]  ->  state[s].distance (+ padding)
    //     path[MAX_NODES + 4*s + 2]  ->  state[s].previous
    //     path[MAX_NODES + 4*s + 3]  ->  state[s].visited  (+ padding)
    //
    // The loop below reads `state[i].node` and `state[i].previous` again, so
    // those two writes change where it goes next - usually it stops, because
    // `state[0].node` is overwritten with the current node and `state[0]` is
    // the start node whose `previous` is NULL.  `distance` and `visited` are
    // never read again, so clobbering them is invisible.
    //
    // `path` keeps every value that was written, because that is what the
    // reversal below reads back out of those same words.
    let mut path: Vec<NodeRef> = Vec::new();
    let mut current_node: Option<NodeRef> = Some(end);

    while let Some(cn) = current_node {
        // path[count++] = current_node;
        let count = path.len();
        if count >= MAX_NODES {
            let word = count - MAX_NODES;
            let slot = word / 4;
            if slot >= MAX_NODES {
                // Past `state` as well: from here the writes eat the rest of
                // the frame (the loop's own variables, the return address) and
                // the process dies without printing anything more.
                crate::cio::stack_smash();
            }
            match word % 4 {
                0 => state[slot].node = cn,
                2 => state[slot].previous = Some(cn),
                _ => {}
            }
        }
        path.push(cn);

        // Find previous node
        let mut current_state_idx: isize = -1;
        for i in 0..state_count {
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
