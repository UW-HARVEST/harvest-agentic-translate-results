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
        if n > i32::MAX as usize || m > i32::MAX as usize {
            return None;
        }
        // Mirror the C overflow check:
        //   MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1) / 3
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
        if self.ecap <= 0 {
            return -1;
        }
        let h_full = (from as u64)
            .wrapping_mul(self.alphsz as u64)
            .wrapping_add(sym as u64);
        let h = (h_full % self.ecap as u64) as usize;

        // Search in the bucket's chain
        let mut p = self.etab[h].as_ref();
        while let Some(node) = p {
            if node.from == from && node.sym == sym {
                return node.to;
            }
            p = node.next.as_ref();
        }

        if creat != 0 {
            // If the pool is empty and we cannot allocate a new node, fail.
            if self.idxpool.is_empty() && self.idxmax >= self.maxn {
                return -1;
            }
            let to = if let Some(idx) = self.idxpool.pop_back() {
                self.idxptr = self.idxpool.len() as i32;
                idx
            } else {
                let v = self.idxmax;
                self.idxmax += 1;
                v
            };
            // Prepend the new edge to the chain.
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
        if self.ecap <= 0 {
            return;
        }
        let h_full = (from as u64)
            .wrapping_mul(self.alphsz as u64)
            .wrapping_add(sym as u64);
        let h = (h_full % self.ecap as u64) as usize;

        // Check the head of the chain.
        let head_matches = match self.etab[h].as_ref() {
            Some(node) => node.from == from && node.sym == sym,
            None => return,
        };
        if head_matches {
            // Mirror the C behavior: when removing the head, set the bucket
            // to None (the C code does `tr->etab[h] = NULL`, dropping the
            // tail of the chain). Only the head's `to` is returned to the
            // pool.
            let head_box = self.etab[h].take().unwrap();
            self.idxpool.push_back(head_box.to);
            self.idxptr = self.idxpool.len() as i32;
            return;
        }

        // Walk the chain looking at q.next.
        let mut q: &mut ChtrieEdge = match self.etab[h].as_mut() {
            Some(b) => b.as_mut(),
            None => return,
        };
        loop {
            let next_matches = match q.next.as_ref() {
                Some(p) => p.from == from && p.sym == sym,
                None => return,
            };
            if next_matches {
                let mut p_box = q.next.take().unwrap();
                q.next = p_box.next.take();
                self.idxpool.push_back(p_box.to);
                self.idxptr = self.idxpool.len() as i32;
                return;
            }
            q = q.next.as_mut().unwrap().as_mut();
        }
    }
    pub fn free(&mut self) {
        // In Rust, dropping the Chtrie automatically frees all owned
        // resources. This method clears the contents so the trie is in
        // an empty state after the call.
        self.etab.clear();
        self.idxpool.clear();
        self.idxptr = 0;
        self.idxmax = 0;
        self.maxn = 0;
        self.ecap = 0;
    }
}
pub fn chtrie_walk(trie: &mut Chtrie, from: i32, sym: i32, creat: i32) -> i32 {
    trie.walk(from, sym, creat)
}
pub fn chtrie_del(trie: &mut Chtrie, from: i32, sym: i32) {
    trie.del(from, sym)
}
