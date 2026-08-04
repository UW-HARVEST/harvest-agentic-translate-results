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
        // Overflow check: MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1) / 3
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
        let h = ((from as u64)
            .wrapping_mul(self.alphsz as u64)
            .wrapping_add(sym as u64)
            % self.ecap as u64) as usize;

        // Search the linked list at bucket h
        let mut cursor = self.etab[h].as_ref();
        while let Some(node) = cursor {
            if node.from == from && node.sym == sym {
                return node.to;
            }
            cursor = node.next.as_ref();
        }

        if creat != 0 {
            // Out of capacity check: pool empty AND idxmax >= maxn
            if self.idxptr == 0 && self.idxmax >= self.maxn {
                return -1;
            }
            let to = if self.idxptr != 0 {
                // pop from idxpool
                let v = self.idxpool.pop_back().expect("idxpool nonempty");
                self.idxptr -= 1;
                v
            } else {
                let v = self.idxmax;
                self.idxmax += 1;
                v
            };
            // Insert new edge at head
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
            .wrapping_add(sym as u64)
            % self.ecap as u64) as usize;

        // Check if head matches
        let head_matches = match self.etab[h].as_ref() {
            Some(node) => node.from == from && node.sym == sym,
            None => return,
        };

        if head_matches {
            // Match C behavior: set entire bucket to None when removing the head
            let to = self.etab[h].as_ref().unwrap().to;
            self.etab[h] = None;
            self.idxpool.push_back(to);
            self.idxptr += 1;
            return;
        }

        // Walk through subsequent nodes to find a match
        let mut node: &mut Option<Box<ChtrieEdge>> =
            &mut self.etab[h].as_mut().unwrap().next;
        loop {
            let matches = match node {
                Some(n) => n.from == from && n.sym == sym,
                None => return,
            };
            if matches {
                let mut removed = node.take().unwrap();
                *node = removed.next.take();
                self.idxpool.push_back(removed.to);
                self.idxptr += 1;
                return;
            }
            // Advance to next slot
            node = &mut node.as_mut().unwrap().next;
        }
    }

    pub fn free(&mut self) {
        // Iteratively drop each bucket's chain to avoid recursive Drop stack
        for slot in self.etab.iter_mut() {
            let mut current = slot.take();
            while let Some(mut node) = current {
                current = node.next.take();
            }
        }
        self.etab.clear();
        self.idxpool.clear();
        self.idxptr = 0;
        self.idxmax = 1;
        self.ecap = 0;
    }
}

impl Drop for Chtrie {
    fn drop(&mut self) {
        // Iteratively drop chains to avoid recursive Drop blowing the stack
        for slot in self.etab.iter_mut() {
            let mut current = slot.take();
            while let Some(mut node) = current {
                current = node.next.take();
            }
        }
    }
}

pub fn chtrie_walk(trie: &mut Chtrie, from: i32, sym: i32, creat: i32) -> i32 {
    trie.walk(from, sym, creat)
}
pub fn chtrie_del(trie: &mut Chtrie, from: i32, sym: i32) {
    trie.del(from, sym)
}
