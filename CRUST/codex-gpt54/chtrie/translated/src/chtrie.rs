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
        let int_max = i32::MAX as usize;

        if n > int_max || m > int_max {
            return None;
        }

        let n_minus_1 = n - 1;
        if int_max.saturating_sub(n_minus_1) < n_minus_1 / 3 {
            return None;
        }

        let ecap = n_minus_1 + n_minus_1 / 3;

        Some(Self {
            etab: std::iter::repeat_with(|| None).take(ecap).collect(),
            idxpool: VecDeque::with_capacity(n),
            idxptr: 0,
            idxmax: 1,
            maxn: n as i32,
            alphsz: m as i32,
            ecap: ecap as i32,
        })
    }
    pub fn walk(&mut self, from: i32, sym: i32, creat: i32) -> i32 {
        let h = (((from as i64) * (self.alphsz as i64)) + (sym as i64)).rem_euclid(self.ecap as i64)
            as usize;

        let mut edge = self.etab[h].as_deref();
        while let Some(node) = edge {
            if node.from == from && node.sym == sym {
                return node.to;
            }
            edge = node.next.as_deref();
        }

        if creat == 0 {
            return -1;
        }

        if self.idxpool.is_empty() && self.idxmax >= self.maxn {
            return -1;
        }

        let to = if let Some(reused) = self.idxpool.pop_back() {
            reused
        } else {
            let next = self.idxmax;
            self.idxmax += 1;
            next
        };

        let new_edge = Box::new(ChtrieEdge {
            next: self.etab[h].take(),
            from,
            sym,
            to,
        });
        self.etab[h] = Some(new_edge);
        self.idxptr = self.idxpool.len() as i32;
        to
    }
    pub fn del(&mut self, from: i32, sym: i32) {
        let h = (((from as i64) * (self.alphsz as i64)) + (sym as i64)).rem_euclid(self.ecap as i64)
            as usize;

        match self.etab[h].as_mut() {
            None => return,
            Some(head) if head.from == from && head.sym == sym => {
                let removed = self.etab[h].take().expect("head exists");
                self.idxpool.push_back(removed.to);
                self.idxptr = self.idxpool.len() as i32;
                return;
            }
            Some(_) => {}
        }

        let mut current = self.etab[h].as_mut();
        while let Some(node) = current {
            let remove_next = matches!(node.next.as_ref(), Some(next) if next.from == from && next.sym == sym);
            if remove_next {
                let mut removed = node.next.take().expect("next exists");
                node.next = removed.next.take();
                self.idxpool.push_back(removed.to);
                self.idxptr = self.idxpool.len() as i32;
                return;
            }
            current = node.next.as_mut();
        }
    }
    pub fn free(&mut self) {
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
