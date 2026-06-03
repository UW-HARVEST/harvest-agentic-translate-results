// Generated Rust Code
use std::collections::VecDeque;
pub const SZ_MAX: usize = usize::MAX;
// Struct Definitions
pub struct ChtrieEdge {
    pub next: Option<Box<ChtrieEdge>>,
    pub from: i32,
    pub sym: i32,
    pub to: i32,
}
pub struct Chtrie {
    pub etab: Vec<Option<Box<ChtrieEdge>>>,
    pub idxpool: VecDeque<i32>,
    pub idxptr: i32,
    pub idxmax: i32,
    pub maxn: i32,
    pub alphsz: i32,
    pub ecap: i32,
}
impl Chtrie {
    pub fn new(n: usize, m: usize) -> Option<Self> {
        let mut n = n;
        let mut m = m;
        if n < 1 {
            n = 1;
        }
        if m < 1 {
            m = 1;
        }
        if n > i32::MAX as usize || m > i32::MAX as usize {
            return None;
        }
        // Check for overflow in (n-1) + (n-1)/3
        let lim = std::cmp::min(i32::MAX as usize, SZ_MAX);
        if lim - (n - 1) < (n - 1) / 3 {
            return None;
        }
        let ecap = (n - 1) + (n - 1) / 3;
        let mut etab: Vec<Option<Box<ChtrieEdge>>> = Vec::with_capacity(ecap);
        for _ in 0..ecap {
            etab.push(None);
        }
        let idxpool = VecDeque::with_capacity(n);
        Some(Chtrie {
            etab,
            idxpool,
            idxptr: 0,
            idxmax: 1,
            maxn: n as i32,
            alphsz: m as i32,
            ecap: ecap as i32,
        })
    }

    pub fn walk(&mut self, from: i32, sym: i32, creat: i32) -> i32 {
        let h = ((from as u64).wrapping_mul(self.alphsz as u64))
            .wrapping_add(sym as u64)
            % (self.ecap as u64);
        let h = h as usize;

        // Search for an existing edge.
        let mut current = self.etab[h].as_ref();
        while let Some(node) = current {
            if node.from == from && node.sym == sym {
                return node.to;
            }
            current = node.next.as_ref();
        }

        if creat != 0 {
            // Check that we have capacity for a new node.
            if self.idxpool.is_empty() && self.idxmax >= self.maxn {
                return -1;
            }
            let to = if let Some(idx) = self.idxpool.pop_back() {
                idx
            } else {
                let idx = self.idxmax;
                self.idxmax += 1;
                idx
            };
            self.idxptr = self.idxpool.len() as i32;
            let next = self.etab[h].take();
            let new_edge = Box::new(ChtrieEdge {
                next,
                from,
                sym,
                to,
            });
            self.etab[h] = Some(new_edge);
            return to;
        }
        -1
    }

    pub fn del(&mut self, from: i32, sym: i32) {
        let h = ((from as u64).wrapping_mul(self.alphsz as u64))
            .wrapping_add(sym as u64)
            % (self.ecap as u64);
        let h = h as usize;

        // Walk down the singly linked list using a "pointer-to-pointer" style.
        let mut current: &mut Option<Box<ChtrieEdge>> = &mut self.etab[h];
        loop {
            let matches = match current {
                Some(node) => node.from == from && node.sym == sym,
                None => return,
            };
            if matches {
                let mut taken = current.take().expect("must be Some");
                *current = taken.next.take();
                self.idxpool.push_back(taken.to);
                self.idxptr = self.idxpool.len() as i32;
                return;
            }
            current = &mut current.as_mut().expect("must be Some").next;
        }
    }

    pub fn free(&mut self) {
        // Iteratively drop all linked-list edges to avoid potential
        // recursion, then clear the supporting buffers.
        for slot in self.etab.iter_mut() {
            let mut head = slot.take();
            while let Some(mut node) = head {
                head = node.next.take();
            }
        }
        self.etab.clear();
        self.idxpool.clear();
        self.idxptr = 0;
        self.idxmax = 0;
        self.maxn = 0;
        self.alphsz = 0;
        self.ecap = 0;
    }
}
pub fn chtrie_walk(trie: &mut Chtrie, from: i32, sym: i32, creat: i32) -> i32 {
    trie.walk(from, sym, creat)
}
pub fn chtrie_del(trie: &mut Chtrie, from: i32, sym: i32) {
    trie.del(from, sym);
}
