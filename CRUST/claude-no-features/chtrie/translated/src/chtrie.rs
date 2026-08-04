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
        // Replicate the overflow check:
        //   MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1) / 3
        // i.e. (n-1) + (n-1)/3 must fit in i32.
        let n1 = n - 1;
        if (i32::MAX as usize) - n1 < n1 / 3 {
            return None;
        }
        let ecap = n1 + n1 / 3;
        let mut etab: Vec<Option<Box<ChtrieEdge>>> = Vec::with_capacity(ecap);
        for _ in 0..ecap {
            etab.push(None);
        }
        Some(Chtrie {
            etab,
            idxpool: VecDeque::with_capacity(n),
            idxptr: 0,
            idxmax: 1,
            maxn: n as i32,
            alphsz: m as i32,
            ecap: ecap as i32,
        })
    }

    pub fn walk(&mut self, from: i32, sym: i32, creat: i32) -> i32 {
        if self.ecap <= 0 {
            // No edge slots; only possible when maxn == 1 (only root).
            if creat != 0 {
                // Cannot create new nodes; root is index 0 and idxmax already == maxn.
                // Match C: would set errno=ENOMEM and return -1.
                return -1;
            }
            return -1;
        }
        let ecap = self.ecap as u64;
        let h = (((from as u64).wrapping_mul(self.alphsz as u64))
            .wrapping_add(sym as u64))
            % ecap;
        let h = h as usize;

        // Search for an existing edge in this bucket.
        let mut cur = self.etab[h].as_ref();
        while let Some(edge) = cur {
            if edge.from == from && edge.sym == sym {
                return edge.to;
            }
            cur = edge.next.as_ref();
        }

        if creat != 0 {
            // Out of nodes: no recycled indexes and idxmax already at maxn.
            if self.idxpool.is_empty() && self.idxmax >= self.maxn {
                return -1;
            }
            // Pop a recycled index if available, otherwise allocate a new one.
            let to = if let Some(idx) = self.idxpool.pop_back() {
                self.idxptr -= 1;
                idx
            } else {
                let i = self.idxmax;
                self.idxmax += 1;
                i
            };
            // Insert the new edge at the head of the bucket chain.
            let old_head = self.etab[h].take();
            self.etab[h] = Some(Box::new(ChtrieEdge {
                next: old_head,
                from,
                sym,
                to,
            }));
            return to;
        }
        -1
    }

    pub fn del(&mut self, from: i32, sym: i32) {
        if self.ecap <= 0 {
            return;
        }
        let ecap = self.ecap as u64;
        let h = (((from as u64).wrapping_mul(self.alphsz as u64))
            .wrapping_add(sym as u64))
            % ecap;
        let h = h as usize;

        // Replicate C behavior: if the matching edge is the head of the chain,
        // the chain is set to NULL (the rest of the chain is dropped). This is
        // a quirk of the original C code that we preserve.
        let head_matches = matches!(&self.etab[h], Some(e) if e.from == from && e.sym == sym);
        if head_matches {
            if let Some(e) = self.etab[h].take() {
                self.idxpool.push_back(e.to);
                self.idxptr += 1;
            }
            return;
        }

        // Otherwise, walk the chain looking for a match. Take the chain out,
        // rebuild it without the matching node.
        let mut chain = self.etab[h].take();
        let mut new_chain: Option<Box<ChtrieEdge>> = None;
        let mut found = false;
        while let Some(mut node) = chain {
            chain = node.next.take();
            if !found && node.from == from && node.sym == sym {
                found = true;
                self.idxpool.push_back(node.to);
                self.idxptr += 1;
            } else {
                node.next = new_chain;
                new_chain = Some(node);
            }
        }
        self.etab[h] = new_chain;
    }

    pub fn free(&mut self) {
        // Drop all edge chains and reset bookkeeping.
        for slot in self.etab.iter_mut() {
            // Iteratively drop the linked list to avoid deep recursion.
            let mut cur = slot.take();
            while let Some(mut node) = cur {
                cur = node.next.take();
            }
        }
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
