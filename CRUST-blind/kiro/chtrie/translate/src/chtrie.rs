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
        let n_minus_1 = n - 1;
        // overflow check: (n-1) + (n-1)/3 must not overflow
        let limit = std::cmp::min(i32::MAX as usize, SZ_MAX);
        if limit - n_minus_1 < n_minus_1 / 3 {
            return None;
        }
        let ecap = n_minus_1 + n_minus_1 / 3;
        let mut etab = Vec::with_capacity(ecap);
        etab.resize_with(ecap, || None);
        Some(Chtrie {
            etab,
            idxpool: VecDeque::new(),
            idxptr: 0,
            idxmax: 1,
            maxn: n as i32,
            alphsz: m as i32,
            ecap: ecap as i32,
        })
    }
    pub fn walk(&mut self, from: i32, sym: i32, creat: i32) -> i32 {
        if self.ecap == 0 {
            return -1;
        }
        let h = ((from as u64) * (self.alphsz as u64) + (sym as u64)) % (self.ecap as u64);
        let h = h as usize;
        // Search existing edges
        let mut p = &self.etab[h];
        loop {
            match p {
                Some(edge) => {
                    if edge.from == from && edge.sym == sym {
                        return edge.to;
                    }
                    p = &edge.next;
                }
                None => break,
            }
        }
        if creat != 0 {
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
        if self.ecap == 0 {
            return;
        }
        let h = ((from as u64) * (self.alphsz as u64) + (sym as u64)) % (self.ecap as u64);
        let h = h as usize;
        // Walk the linked list to find and remove the matching edge
        let mut cur = self.etab[h].take();
        let mut prev: Vec<Box<ChtrieEdge>> = Vec::new();
        let mut found_to: Option<i32> = None;
        while let Some(mut edge) = cur {
            cur = edge.next.take();
            if found_to.is_none() && edge.from == from && edge.sym == sym {
                found_to = Some(edge.to);
                // skip this edge (don't push to prev)
            } else {
                prev.push(edge);
            }
        }
        // Rebuild the chain
        let mut head: Option<Box<ChtrieEdge>> = None;
        for mut edge in prev.into_iter().rev() {
            edge.next = head;
            head = Some(edge);
        }
        self.etab[h] = head;
        if let Some(to) = found_to {
            self.idxpool.push_back(to);
        }
    }
    pub fn free(&mut self) {
        self.etab.clear();
        self.idxpool.clear();
    }
}
pub fn chtrie_walk(trie: &mut Chtrie, from: i32, sym: i32, creat: i32) -> i32 {
    trie.walk(from, sym, creat)
}
pub fn chtrie_del(trie: &mut Chtrie, from: i32, sym: i32) {
    trie.del(from, sym);
}
