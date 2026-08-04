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
        // Mirror the C overflow check:
        //   MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1)/3
        let lim = (i32::MAX as usize).min(usize::MAX);
        if lim.saturating_sub(n_minus_1) < n_minus_1 / 3 {
            return None;
        }
        let ecap = n_minus_1 + n_minus_1 / 3;
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
            .wrapping_add(sym as u64))
            % (self.ecap as u64);
        let h = h as usize;

        // Lookup in the bucket's linked list.
        let mut cur = self.etab[h].as_deref();
        while let Some(node) = cur {
            if node.from == from && node.sym == sym {
                return node.to;
            }
            cur = node.next.as_deref();
        }

        if creat != 0 {
            if self.idxpool.is_empty() && self.idxmax >= self.maxn {
                return -1;
            }
            let to = if let Some(v) = self.idxpool.pop_back() {
                v
            } else {
                let v = self.idxmax;
                self.idxmax += 1;
                v
            };
            self.idxptr = self.idxpool.len() as i32;

            let old_head = self.etab[h].take();
            let new_node = Box::new(ChtrieEdge {
                next: old_head,
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

        enum Action {
            Stop,
            Remove,
            Advance,
        }

        let mut cur: &mut Option<Box<ChtrieEdge>> = &mut self.etab[h];
        loop {
            let action = match cur.as_ref() {
                None => Action::Stop,
                Some(node) if node.from == from && node.sym == sym => Action::Remove,
                Some(_) => Action::Advance,
            };
            match action {
                Action::Stop => return,
                Action::Remove => {
                    let mut taken = cur.take().unwrap();
                    *cur = taken.next.take();
                    self.idxpool.push_back(taken.to);
                    self.idxptr = self.idxpool.len() as i32;
                    return;
                }
                Action::Advance => {
                    cur = &mut cur.as_mut().unwrap().next;
                }
            }
        }
    }
    pub fn free(&mut self) {
        // Iteratively drop linked-list buckets to avoid potential
        // recursive-Drop stack overflow on long chains.
        for bucket in self.etab.iter_mut() {
            let mut cur = bucket.take();
            while let Some(mut node) = cur {
                cur = node.next.take();
            }
        }
        self.etab.clear();
        self.idxpool.clear();
        self.idxptr = 0;
        self.idxmax = 1;
    }
}
pub fn chtrie_walk(trie: &mut Chtrie, from: i32, sym: i32, creat: i32) -> i32 {
    trie.walk(from, sym, creat)
}
pub fn chtrie_del(trie: &mut Chtrie, from: i32, sym: i32) {
    trie.del(from, sym);
}
