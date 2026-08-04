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
        // Mirror C: regulate small values
        let n = if n < 1 { 1 } else { n };
        let m = if m < 1 { 1 } else { m };

        // Mirror C: enforce INT_MAX upper bound
        if n > i32::MAX as usize || m > i32::MAX as usize {
            return None;
        }

        // Overflow check from C: MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1)/3
        let limit = (i32::MAX as usize).min(SZ_MAX);
        if limit < (n - 1) || limit - (n - 1) < (n - 1) / 3 {
            return None;
        }

        let ecap = (n - 1) + (n - 1) / 3;

        let mut etab: Vec<Option<Box<ChtrieEdge>>> = Vec::with_capacity(ecap);
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
        // Guard against zero capacity (n=1 case): no children possible.
        if self.ecap <= 0 {
            return -1;
        }

        let h = ((from as u64)
            .wrapping_mul(self.alphsz as u64)
            .wrapping_add(sym as u64))
            % (self.ecap as u64);
        let h = h as usize;

        // Search the chain for an existing matching edge.
        let mut cur = self.etab[h].as_ref();
        while let Some(p) = cur {
            if p.from == from && p.sym == sym {
                return p.to;
            }
            cur = p.next.as_ref();
        }

        if creat != 0 {
            // If the index pool is empty and we've already used every node,
            // we've exhausted capacity.
            if self.idxptr == 0 && self.idxmax >= self.maxn {
                return -1;
            }

            // Allocate a new node index: prefer reused indices from the pool.
            let to = if self.idxptr != 0 {
                self.idxptr -= 1;
                // C uses LIFO (--idxptr then deref), so pop from the back.
                self.idxpool.pop_back().unwrap()
            } else {
                let v = self.idxmax;
                self.idxmax += 1;
                v
            };

            // Insert at the head of the bucket.
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

        let h = ((from as u64)
            .wrapping_mul(self.alphsz as u64)
            .wrapping_add(sym as u64))
            % (self.ecap as u64);
        let h = h as usize;

        // Find the index of the matching edge in the bucket's chain.
        let mut match_idx: Option<usize> = None;
        let mut cur = self.etab[h].as_ref();
        let mut idx: usize = 0;
        while let Some(p) = cur {
            if p.from == from && p.sym == sym {
                match_idx = Some(idx);
                break;
            }
            cur = p.next.as_ref();
            idx += 1;
        }

        let match_idx = match match_idx {
            Some(i) => i,
            None => return, // not found, leave trie unchanged
        };

        if match_idx == 0 {
            // Match at head. The C reference implementation sets
            // `etab[h] = NULL` here (rather than `p->next`), which is
            // preserved here for behavioral parity with the C source.
            let head = self.etab[h].take().unwrap();
            self.idxpool.push_back(head.to);
            self.idxptr += 1;
            // self.etab[h] is already None.
        } else {
            // Walk to the predecessor of the matching node and splice it out.
            let mut prev = self.etab[h].as_mut().unwrap();
            for _ in 0..(match_idx - 1) {
                prev = prev.next.as_mut().unwrap();
            }
            let mut removed = prev.next.take().unwrap();
            prev.next = removed.next.take();
            self.idxpool.push_back(removed.to);
            self.idxptr += 1;
        }
    }

    pub fn free(&mut self) {
        // Rust's Drop semantics handle deallocation automatically when the
        // Chtrie is dropped. This explicit free clears all heap-owned state
        // so the trie no longer references any edges or pooled indices.
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
    trie.del(from, sym)
}
