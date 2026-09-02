//! Node storage plus a model of the glibc allocator behaviour that the C
//! program's output depends on.
//!
//! `c_src/src/lib.c` hands raw `node_t *` pointers to its callers and
//! `delete_node` calls `free` once a node's reference count reaches zero
//! *without* removing the pointer from the graph.  Every later operation
//! therefore reads (and sometimes frees again) memory the allocator owns, and
//! what the program prints is decided by glibc's malloc rather than by the
//! program's own logic:
//!
//! * a freed chunk keeps the bytes the program wrote, except for the first 16,
//!   which the allocator overwrites with its own bookkeeping -- so a deleted
//!   city's `ref_count` and edges still print, but its name does not, and
//!   `strcmp` against it never matches;
//! * `malloc` hands chunks back out in a very specific order, so adding cities
//!   after deleting some makes the graph's stale pointers alias the *new*
//!   nodes;
//! * freeing an already free chunk trips a heap consistency check, which prints
//!   a diagnostic and `abort()`s.
//!
//! Every `node_t` is 240 bytes and so comes from a single 256 byte chunk size
//! class.  That makes the heap a sequence of equally sized units; a unit index
//! stands in for an address, and index order is address order.  The model
//! covers the parts of `_int_malloc`/`_int_free` that a program allocating a
//! single size class can observe: the per-thread cache, the unsorted bin, the
//! small and large bins, backward/forward consolidation, the last remainder,
//! and growth from the top chunk.

use crate::cio::malloc_printerr;
use std::collections::{BTreeMap, VecDeque};

pub const MAX_CITY_NAME: usize = 64;

/// Stands in for `node_t *`.
pub type NodeId = usize;

/// `sizeof(node_t)` rounded up to a chunk: 240 bytes of payload + 8 byte header,
/// rounded to the 16 byte malloc alignment.
const UNIT: usize = 256;
/// glibc's `MINSIZE`.
const MINSIZE: usize = 32;
/// glibc's `MIN_LARGE_SIZE`: below this a free chunk goes into a small bin.
const MIN_LARGE_SIZE: usize = 1024;
/// glibc's `mp_.tcache_count`.
const TCACHE_COUNT: usize = 7;

pub struct Edge {
    pub destination: NodeId,
    pub distance: i32,
}

pub struct Node {
    pub city_name: [u8; MAX_CITY_NAME],
    pub ref_count: i32,
    pub edges: Vec<Edge>,
}

impl Node {
    pub fn new(name: [u8; MAX_CITY_NAME]) -> Self {
        Node {
            city_name: name,
            ref_count: 1,
            edges: Vec::new(),
        }
    }

    /// `node->city_name` as seen by `%s`: the bytes before the NUL terminator.
    pub fn name(&self) -> &[u8] {
        let end = self
            .city_name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(MAX_CITY_NAME);
        &self.city_name[..end]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Unit {
    /// Handed out to the program.
    InUse,
    /// Parked in the per-thread cache.  tcache chunks stay marked "in use", so
    /// they never take part in consolidation.
    Tcache,
    /// Part of a free block sitting in the unsorted bin or in a small/large
    /// bin.
    Free,
    /// Absorbed into the top chunk (never allocated, or freed back into it).
    Top,
}

/// Which list a free block currently sits on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Loc {
    Unsorted,
    Bin(usize),
}

/// A free block spanning `len` consecutive units starting at its key.
struct Block {
    len: usize,
    loc: Loc,
}

pub struct Arena {
    nodes: Vec<Node>,
    unit: Vec<Unit>,
    /// True while the first 16 bytes of `city_name` hold allocator bookkeeping
    /// (`tcache_entry::next`/`key`, or a bin's `fd`/`bk`) instead of a city
    /// name.
    clobbered: Vec<bool>,

    /// First unit belonging to the top chunk.
    top: usize,
    /// Free block registry, keyed by starting unit.
    blocks: BTreeMap<usize, Block>,
    /// Which free block a unit belongs to.
    owner: Vec<usize>,
    /// The unsorted bin.  `front` is the list head (most recently freed),
    /// `back` is the tail, which is where `_int_malloc` starts scanning.
    unsorted: VecDeque<usize>,
    /// Small and large bins, keyed by block length in units.  `back` is the
    /// bin's tail, i.e. `last(bin)`, which is what `_int_malloc` takes.
    bins: BTreeMap<usize, VecDeque<usize>>,
    /// The per-thread cache for the 256 byte size class (LIFO).
    tcache: Vec<NodeId>,
    /// `av->last_remainder`.
    last_remainder: Option<usize>,
}

impl Arena {
    pub fn new() -> Self {
        Arena {
            nodes: Vec::new(),
            unit: Vec::new(),
            clobbered: Vec::new(),
            top: 0,
            blocks: BTreeMap::new(),
            owner: Vec::new(),
            unsorted: VecDeque::new(),
            bins: BTreeMap::new(),
            tcache: Vec::new(),
            last_remainder: None,
        }
    }

    pub fn get(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    pub fn get_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id]
    }

    /// `strcmp(node->city_name, other) == 0` for the node in chunk `id`.
    ///
    /// A chunk whose first bytes the allocator overwrote never compares equal to
    /// a real city name; that is why looking a deleted city up again reports
    /// "not found".
    pub fn name_matches(&self, id: NodeId, other: &[u8]) -> bool {
        if self.clobbered[id] {
            return false;
        }
        self.nodes[id].name() == other
    }

    fn ensure_unit(&mut self, id: usize) {
        while self.nodes.len() <= id {
            self.nodes.push(Node::new([0u8; MAX_CITY_NAME]));
            self.unit.push(Unit::Top);
            self.clobbered.push(false);
            self.owner.push(usize::MAX);
        }
    }

    /// `size < MIN_LARGE_SIZE`
    fn is_small(len: usize) -> bool {
        len * UNIT < MIN_LARGE_SIZE
    }

    fn register(&mut self, start: usize, len: usize, loc: Loc) {
        self.blocks.insert(start, Block { len, loc });
        for u in start..start + len {
            self.unit[u] = Unit::Free;
            self.owner[u] = start;
        }
    }

    /// The allocator's bookkeeping (`tcache_entry::next`/`key` for a cached
    /// chunk, `fd`/`bk` for a binned one) occupies the first 16 bytes of the
    /// user block, which is exactly the start of `city_name`.
    ///
    /// Only the *start* of a free block is written, so a chunk that was
    /// consolidated into a neighbour, or absorbed into the top chunk, keeps its
    /// city name readable -- and therefore still answers to `strcmp`.
    fn clobber(&mut self, id: usize) {
        self.clobbered[id] = true;
        for b in self.nodes[id].city_name[..16].iter_mut() {
            *b = 0;
        }
    }

    /// Unlink a free block from whatever list holds it and forget it.
    fn unlink(&mut self, start: usize) -> usize {
        let block = self.blocks.remove(&start).expect("free block");
        match block.loc {
            Loc::Unsorted => {
                if let Some(p) = self.unsorted.iter().position(|&x| x == start) {
                    self.unsorted.remove(p);
                }
            }
            Loc::Bin(len) => {
                if let Some(q) = self.bins.get_mut(&len) {
                    if let Some(p) = q.iter().position(|&x| x == start) {
                        q.remove(p);
                    }
                }
            }
        }
        for u in start..start + block.len {
            self.owner[u] = usize::MAX;
        }
        block.len
    }

    /// Insert a free block into its small or large bin.
    ///
    /// Small bins are pure FIFO: chunks go in at the head and come out of the
    /// tail.  Large bins are kept in descending size order and glibc inserts a
    /// chunk of an already present size "in the second position", which is what
    /// the `insert(1, ..)` reproduces.
    fn bin_insert(&mut self, start: usize, len: usize) {
        let q = self.bins.entry(len).or_default();
        if Self::is_small(len) || q.is_empty() {
            if Self::is_small(len) {
                q.push_front(start);
            } else {
                q.push_back(start);
            }
        } else {
            q.insert(1, start);
        }
        if let Some(b) = self.blocks.get_mut(&start) {
            b.loc = Loc::Bin(len);
        }
        self.clobber(start);
    }

    /// `last(bin)`: take the block at the bin's tail.
    fn bin_take_last(&mut self, len: usize) -> Option<usize> {
        let q = self.bins.get_mut(&len)?;
        let start = q.pop_back()?;
        // `unlink` would look for it in the bin again, so drop the record here.
        let block = self.blocks.remove(&start).expect("free block");
        for u in start..start + block.len {
            self.owner[u] = usize::MAX;
        }
        Some(start)
    }

    /// `malloc(sizeof(node_t))`
    fn malloc(&mut self, node: Node) -> NodeId {
        let id = self.malloc_chunk();
        self.ensure_unit(id);
        self.unit[id] = Unit::InUse;
        self.clobbered[id] = false;
        self.nodes[id] = node;
        id
    }

    fn malloc_chunk(&mut self) -> usize {
        // 1. `__libc_malloc` -> `tcache_get`, LIFO.
        if let Some(id) = self.tcache.pop() {
            return id;
        }

        // 2. `_int_malloc`: exact fit small bin, stashing spares in the tcache.
        if let Some(id) = self.bin_take_last(1) {
            while self.tcache.len() < TCACHE_COUNT {
                match self.bin_take_last(1) {
                    Some(spare) => {
                        self.ensure_unit(spare);
                        self.unit[spare] = Unit::Tcache;
                        self.clobber(spare);
                        self.tcache.push(spare);
                    }
                    None => break,
                }
            }
            return id;
        }

        // 3. Scan the unsorted bin from its tail.
        let mut return_cached = false;
        while let Some(victim) = self.unsorted.pop_back() {
            let len = self.blocks.get(&victim).expect("free block").len;
            let was_only = self.unsorted.is_empty();

            // Split the last remainder when it is the only chunk left.
            if was_only
                && self.last_remainder == Some(victim)
                && len * UNIT > UNIT + MINSIZE
            {
                self.blocks.remove(&victim);
                for u in victim..victim + len {
                    self.owner[u] = usize::MAX;
                }
                let rem = victim + 1;
                self.register(rem, len - 1, Loc::Unsorted);
                self.unsorted.push_front(rem);
                self.clobber(rem);
                self.last_remainder = Some(rem);
                return victim;
            }

            self.blocks.remove(&victim);
            for u in victim..victim + len {
                self.owner[u] = usize::MAX;
            }

            if len == 1 {
                // Exact fit: fill the cache first and only return directly once
                // the cache is full.
                if self.tcache.len() < TCACHE_COUNT {
                    self.ensure_unit(victim);
                    self.unit[victim] = Unit::Tcache;
                    self.clobber(victim);
                    self.tcache.push(victim);
                    return_cached = true;
                    continue;
                }
                return victim;
            }

            // Not an exact fit: file it in its bin and keep scanning.
            self.register(victim, len, Loc::Unsorted);
            self.bin_insert(victim, len);
        }

        if return_cached {
            return self.tcache.pop().expect("cached chunk");
        }

        // 4. Next larger non-empty bin, split off what we need.
        let lens: Vec<usize> = self.bins.keys().copied().filter(|&l| l >= 2).collect();
        for len in lens {
            if let Some(victim) = self.bin_take_last(len) {
                let rem = victim + 1;
                self.register(rem, len - 1, Loc::Unsorted);
                self.unsorted.push_front(rem);
                self.clobber(rem);
                self.last_remainder = Some(rem);
                return victim;
            }
        }

        // 5. Carve a fresh chunk off the top of the heap.
        let id = self.top;
        self.top += 1;
        self.ensure_unit(id);
        id
    }

    /// `free(node)`
    fn free(&mut self, id: NodeId) {
        // A chunk that was absorbed back into the top chunk is caught by one of
        // `_int_free`'s lightweight tests.  If it *became* the top chunk its
        // header was rewritten with the top chunk's size, which `p == av->top`
        // detects; if it ended up in the interior of the top chunk its header is
        // stale and the backward consolidation check trips instead.
        if id == self.top {
            malloc_printerr(b"double free or corruption (top)\n");
        }
        if id > self.top {
            malloc_printerr(b"corrupted size vs. prev_size while consolidating\n");
        }
        match self.unit[id] {
            // `e->key == tcache_key` and the chunk is on the list.
            Unit::Tcache => malloc_printerr(b"free(): double free detected in tcache 2\n"),
            // `!prev_inuse (nextchunk)`.
            Unit::Free => malloc_printerr(b"double free or corruption (!prev)\n"),
            Unit::Top => malloc_printerr(b"corrupted size vs. prev_size while consolidating\n"),
            Unit::InUse => {}
        }

        // The allocator writes over the first 16 bytes of the user block, but
        // only where it actually stores a list pointer.
        if self.tcache.len() < TCACHE_COUNT {
            self.clobber(id);
            self.unit[id] = Unit::Tcache;
            self.tcache.push(id);
            return;
        }

        // Consolidate backward, then look at the following chunk.
        let mut start = id;
        let mut len = 1;
        if id > 0 && self.unit[id - 1] == Unit::Free {
            let prev = self.owner[id - 1];
            let plen = self.unlink(prev);
            start = prev;
            len += plen;
        }

        let next = id + 1;
        if next >= self.top {
            // `nextchunk == av->top`: the chunk is absorbed into the top chunk.
            // Only the chunk header is rewritten, so the city name survives.
            for u in start..self.top {
                self.unit[u] = Unit::Top;
                self.owner[u] = usize::MAX;
            }
            self.top = start;
            return;
        }

        if self.unit[next] == Unit::Free {
            let nlen = self.unlink(next);
            len += nlen;
        }

        self.register(start, len, Loc::Unsorted);
        self.unsorted.push_front(start);
        self.clobber(start);
    }

    /// `malloc` + node initialisation, as `add_node` performs it.
    pub fn alloc_node(&mut self, name: [u8; MAX_CITY_NAME]) -> NodeId {
        self.malloc(Node::new(name))
    }

    /// `free(node)`
    pub fn free_node(&mut self, id: NodeId) {
        self.free(id)
    }
}
