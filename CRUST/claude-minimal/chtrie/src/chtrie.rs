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
        let n = if n < 1 { 1 } else { n };
        let m = if m < 1 { 1 } else { m };
        let int_max = i32::MAX as usize;
        if n > int_max || m > int_max {
            return None;
        }
        let min_max = std::cmp::min(int_max, SZ_MAX);
        // Check overflow: MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1)/3
        let lhs = match min_max.checked_sub(n - 1) {
            Some(v) => v,
            None => return None,
        };
        if lhs < (n - 1) / 3 {
            return None;
        }
        let ecap = (n - 1) + (n - 1) / 3;
        let mut etab: Vec<Option<Box<ChtrieEdge>>> = Vec::with_capacity(ecap);
        for _ in 0..ecap {
            etab.push(None);
        }
        let idxpool: VecDeque<i32> = VecDeque::with_capacity(n);
        Some(Self {
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
        if self.ecap <= 0 {
            // No buckets available: emulate ENOMEM/no-match
            return -1;
        }
        let h = ((from as u64)
            .wrapping_mul(self.alphsz as u64)
            .wrapping_add(sym as u64))
            % (self.ecap as u64);
        let h = h as usize;

        // Search the bucket for an existing edge.
        let mut cur = &self.etab[h];
        while let Some(node) = cur {
            if node.from == from && node.sym == sym {
                return node.to;
            }
            cur = &node.next;
        }

        if creat != 0 {
            // If pool is empty and we've assigned all maxn indexes, fail.
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

            // Insert new edge at the head of the bucket.
            let new_node = Box::new(ChtrieEdge {
                next: self.etab[h].take(),
                from,
                sym,
                to,
            });
            self.etab[h] = Some(new_node);
            return to;
        }
        -1
    }
    pub fn del(&mut self, from: i32, sym: i32) {
        if self.ecap <= 0 {
            return;
        }
        let h = ((from as u64)
            .wrapping_mul(self.alphsz as u64)
            .wrapping_add(sym as u64))
            % (self.ecap as u64);
        let h = h as usize;

        // Check if the head of the bucket matches.
        let head_matches = match &self.etab[h] {
            Some(n) => n.from == from && n.sym == sym,
            None => false,
        };

        if head_matches {
            // Match the C source's head-removal behavior:
            //     tr->etab[h] = NULL;
            // This drops the head edge (and any following edges in the same
            // bucket). Replicating this matches chtrie.c exactly.
            let head = self.etab[h].take().unwrap();
            self.idxpool.push_back(head.to);
            self.idxptr = self.idxpool.len() as i32;
            return;
        }

        // Otherwise traverse the chain looking for a non-head match.
        let mut cur = &mut self.etab[h];
        while let Some(boxed) = cur {
            let next_matches = match &boxed.next {
                Some(n) => n.from == from && n.sym == sym,
                None => false,
            };
            if next_matches {
                // Standard linked-list unlink: q->next = p->next
                let mut removed = boxed.next.take().unwrap();
                boxed.next = removed.next.take();
                self.idxpool.push_back(removed.to);
                self.idxptr = self.idxpool.len() as i32;
                return;
            }
            cur = &mut boxed.next;
        }
    }
    pub fn free(&mut self) {
        // Drop trait would also clean up automatically, but mirror the C API
        // by releasing all storage explicitly.
        self.etab.clear();
        self.idxpool.clear();
        self.idxptr = 0;
        self.idxmax = 1;
        self.ecap = 0;
    }
}
pub fn chtrie_walk(trie: &mut Chtrie, from: i32, sym: i32, creat: i32) -> i32 {
    trie.walk(from, sym, creat)
}
pub fn chtrie_del(trie: &mut Chtrie, from: i32, sym: i32) {
    trie.del(from, sym)
}
