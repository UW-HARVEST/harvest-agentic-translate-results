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
            // ERANGE
            return None;
        }
        // Check overflow: MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1) / 3
        let limit = std::cmp::min(i32::MAX as usize, SZ_MAX);
        if limit - (n - 1) < (n - 1) / 3 {
            return None;
        }
        let ecap = (n - 1) + (n - 1) / 3;
        let mut etab: Vec<Option<Box<ChtrieEdge>>> = Vec::with_capacity(ecap);
        for _ in 0..ecap {
            etab.push(None);
        }
        let idxpool: VecDeque<i32> = VecDeque::with_capacity(n);
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
        let h = ((from as u64).wrapping_mul(self.alphsz as u64) + sym as u64)
            % (self.ecap as u64);
        let h = h as usize;

        // Search for existing edge
        let mut p = self.etab[h].as_ref();
        while let Some(node) = p {
            if node.from == from && node.sym == sym {
                return node.to;
            }
            p = node.next.as_ref();
        }

        if creat != 0 {
            if self.idxptr == 0 && self.idxmax >= self.maxn {
                // ENOMEM
                return -1;
            }
            let to = if self.idxptr != 0 {
                self.idxptr -= 1;
                // Pop from end (stack-like behavior matching C: *--tr->idxptr)
                self.idxpool.pop_back().unwrap()
            } else {
                let v = self.idxmax;
                self.idxmax += 1;
                v
            };
            let new_edge = Box::new(ChtrieEdge {
                next: self.etab[h].take(),
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
        let h = ((from as u64).wrapping_mul(self.alphsz as u64) + sym as u64)
            % (self.ecap as u64);
        let h = h as usize;

        // Check if head matches (q == NULL case in C)
        let head_matches = match &self.etab[h] {
            Some(node) => node.from == from && node.sym == sym,
            None => return,
        };

        if head_matches {
            // Match C behavior: when q is NULL (head match), set bucket to NULL
            // This matches the original C code (which drops the rest of the chain)
            let head = self.etab[h].take().unwrap();
            let to = head.to;
            self.etab[h] = None;
            self.idxpool.push_back(to);
            self.idxptr += 1;
            return;
        }

        // Walk through chain to find a matching node
        let mut current = self.etab[h].as_mut().unwrap();
        loop {
            let next_matches = match &current.next {
                Some(next_node) => next_node.from == from && next_node.sym == sym,
                None => return, // not found
            };

            if next_matches {
                // Remove current.next from the chain
                let mut to_remove = current.next.take().unwrap();
                current.next = to_remove.next.take();
                self.idxpool.push_back(to_remove.to);
                self.idxptr += 1;
                return;
            }

            // Move to next node
            current = current.next.as_mut().unwrap();
        }
    }
    pub fn free(&mut self) {
        // Iteratively drop edge chains to avoid potential stack overflow
        // from recursive Box drops on long chains.
        for slot in self.etab.iter_mut() {
            let mut p = slot.take();
            while let Some(mut node) = p {
                p = node.next.take();
            }
        }
        self.etab.clear();
        self.idxpool.clear();
        self.idxptr = 0;
    }
}
pub fn chtrie_walk(trie: &mut Chtrie, from: i32, sym: i32, creat: i32) -> i32 {
    trie.walk(from, sym, creat)
}
pub fn chtrie_del(trie: &mut Chtrie, from: i32, sym: i32) {
    trie.del(from, sym)
}
