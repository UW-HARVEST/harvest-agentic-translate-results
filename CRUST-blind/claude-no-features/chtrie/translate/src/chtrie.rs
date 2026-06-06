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
        // Overflow check: MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1) / 3
        let min_max = std::cmp::min(i32::MAX as usize, SZ_MAX);
        if min_max - (n - 1) < (n - 1) / 3 {
            return None;
        }
        let ecap = (n - 1) + (n - 1) / 3;
        // ecap may be 0 when n == 1; we still want a vec we can index, but
        // hashing will fail if ecap == 0. Allocate at least 1 slot to mirror
        // calloc returning a non-null pointer. The C code would also crash on
        // mod by zero in this case, so leave space accordingly.
        let etab_len = if ecap == 0 { 0 } else { ecap };
        let mut etab: Vec<Option<Box<ChtrieEdge>>> = Vec::with_capacity(etab_len);
        for _ in 0..etab_len {
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
        let h = ((from as u64) * (self.alphsz as u64) + (sym as u64))
            % (self.ecap as u64);
        let h = h as usize;

        // Traverse the linked list looking for a match.
        let mut cur = self.etab[h].as_ref();
        while let Some(p) = cur {
            if p.from == from && p.sym == sym {
                return p.to;
            }
            cur = p.next.as_ref();
        }

        if creat != 0 {
            // If no free indexes in pool and we've used up maxn, fail.
            if self.idxpool.is_empty() && self.idxmax >= self.maxn {
                // ENOMEM
                return -1;
            }
            let to = if let Some(idx) = self.idxpool.pop_back() {
                self.idxptr -= 1;
                idx
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
        let h = ((from as u64) * (self.alphsz as u64) + (sym as u64))
            % (self.ecap as u64);
        let h = h as usize;

        // First, check if any edge in this bucket matches (from, sym).
        let mut found = false;
        let mut head_match = false;
        {
            let mut cur = self.etab[h].as_ref();
            let mut is_head = true;
            while let Some(p) = cur {
                if p.from == from && p.sym == sym {
                    found = true;
                    head_match = is_head;
                    break;
                }
                cur = p.next.as_ref();
                is_head = false;
            }
        }
        if !found {
            return;
        }

        if head_match {
            // Replicate C behavior: when p is the head (q == NULL),
            // C sets `tr->etab[h] = NULL`, dropping the entire bucket.
            // Capture the matching edge's `to` so we can return its index
            // to the pool, matching `*tr->idxptr++ = p->to`.
            let head = self.etab[h].take().unwrap();
            let to_val = head.to;
            // The rest of the chain is dropped along with `head` here,
            // matching the C bug where subsequent edges become unreachable.
            self.idxpool.push_back(to_val);
            self.idxptr += 1;
        } else {
            // Walk the list with mutable references and remove the matching
            // node by splicing around it: q->next = p->next.
            let mut cur = self.etab[h].as_mut();
            while let Some(p) = cur {
                let take_next = match &p.next {
                    Some(next) => next.from == from && next.sym == sym,
                    None => false,
                };
                if take_next {
                    let mut removed = p.next.take().expect("matched next exists");
                    p.next = removed.next.take();
                    self.idxpool.push_back(removed.to);
                    self.idxptr += 1;
                    return;
                }
                cur = p.next.as_mut();
            }
        }
    }

    pub fn free(&mut self) {
        // In Rust, the buckets and edges are freed automatically when the
        // Chtrie is dropped. To mirror the C `chtrie_free`, we explicitly
        // clear the owned collections here.
        self.etab.clear();
        self.idxpool.clear();
        self.idxptr = 0;
        self.idxmax = 0;
        self.ecap = 0;
        self.maxn = 0;
        self.alphsz = 0;
    }
}
pub fn chtrie_walk(trie: &mut Chtrie, from: i32, sym: i32, creat: i32) -> i32 {
    trie.walk(from, sym, creat)
}
pub fn chtrie_del(trie: &mut Chtrie, from: i32, sym: i32) {
    trie.del(from, sym)
}
