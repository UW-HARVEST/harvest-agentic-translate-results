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
        let n = n.max(1);
        let m = m.max(1);
        if n > i32::MAX as usize || m > i32::MAX as usize {
            return None;
        }
        let int_max = i32::MAX as usize;
        let sz_max = usize::MAX;
        let limit = int_max.min(sz_max);
        if limit - (n - 1) < (n - 1) / 3 {
            return None;
        }
        let ecap = (n - 1) + (n - 1) / 3;
        let mut etab = Vec::with_capacity(ecap);
        for _ in 0..ecap {
            etab.push(None);
        }
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
        let h = ((from as u64) * (self.alphsz as u64) + (sym as u64)) % (self.ecap as u64);
        let h = h as usize;
        // Search existing edges
        let mut cur = &self.etab[h];
        while let Some(edge) = cur {
            if edge.from == from && edge.sym == sym {
                return edge.to;
            }
            cur = &edge.next;
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
        let h = ((from as u64) * (self.alphsz as u64) + (sym as u64)) % (self.ecap as u64);
        let h = h as usize;
        // Check if head matches
        let head_matches = if let Some(ref edge) = self.etab[h] {
            edge.from == from && edge.sym == sym
        } else {
            false
        };
        if head_matches {
            // C code: etab[h] = NULL when removing head (drops rest of chain)
            let removed = self.etab[h].take().unwrap();
            self.idxpool.push_back(removed.to);
            return;
        }
        // Search deeper in the chain
        let mut cur = &mut self.etab[h];
        loop {
            let should_remove_next = if let Some(ref edge) = cur {
                if let Some(ref next) = edge.next {
                    next.from == from && next.sym == sym
                } else {
                    return;
                }
            } else {
                return;
            };
            if should_remove_next {
                let edge = cur.as_mut().unwrap();
                let removed = edge.next.take().unwrap();
                edge.next = removed.next;
                self.idxpool.push_back(removed.to);
                return;
            }
            cur = &mut cur.as_mut().unwrap().next;
        }
    }
    pub fn free(&mut self) {
        // Rust handles deallocation via Drop; clear data structures
        self.etab.clear();
        self.idxpool.clear();
    }
}
pub fn chtrie_walk(trie: &mut Chtrie, from: i32, sym: i32, creat: i32) -> i32 {
    trie.walk(from, sym, creat)
}
pub fn chtrie_del(trie: &mut Chtrie, from: i32, sym: i32) {
    trie.del(from, sym)
}
