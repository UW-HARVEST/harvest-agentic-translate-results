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

        let edge_count = n - 1;
        if (i32::MAX as usize) - edge_count < edge_count / 3 {
            return None;
        }

        let ecap = edge_count + edge_count / 3;
        let mut etab = Vec::new();
        if etab.try_reserve_exact(ecap).is_err() {
            return None;
        }
        etab.resize_with(ecap, || None);

        let mut idxpool = VecDeque::new();
        if idxpool.try_reserve(n).is_err() {
            return None;
        }

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
            return -1;
        }

        let h = (((from as u64).wrapping_mul(self.alphsz as u64)).wrapping_add(sym as u64)
            % (self.ecap as u64)) as usize;

        let mut edge = self.etab[h].as_deref();
        while let Some(current) = edge {
            if current.from == from && current.sym == sym {
                return current.to;
            }
            edge = current.next.as_deref();
        }

        if creat == 0 {
            return -1;
        }

        if self.idxptr == 0 && self.idxmax >= self.maxn {
            return -1;
        }

        let to = if self.idxptr != 0 {
            match self.idxpool.pop_back() {
                Some(recycled) => {
                    self.idxptr -= 1;
                    recycled
                }
                None => return -1,
            }
        } else {
            let next = self.idxmax;
            self.idxmax += 1;
            next
        };

        let next = self.etab[h].take();
        self.etab[h] = Some(Box::new(ChtrieEdge {
            next,
            from,
            sym,
            to,
        }));
        to
    }
    pub fn del(&mut self, from: i32, sym: i32) {
        if self.ecap <= 0 {
            return;
        }

        let h = (((from as u64).wrapping_mul(self.alphsz as u64)).wrapping_add(sym as u64)
            % (self.ecap as u64)) as usize;

        let mut pos = 0usize;
        let mut edge = self.etab[h].as_deref();
        while let Some(current) = edge {
            if current.from == from && current.sym == sym {
                break;
            }
            pos += 1;
            edge = current.next.as_deref();
        }

        let Some(found) = edge else {
            return;
        };
        let recycled = found.to;

        if pos == 0 {
            let _dropped = self.etab[h].take();
        } else {
            let mut link = &mut self.etab[h];
            for _ in 0..(pos - 1) {
                let Some(current) = link.as_mut() else {
                    return;
                };
                link = &mut current.next;
            }
            let Some(predecessor) = link.as_mut() else {
                return;
            };
            let Some(mut target) = predecessor.next.take() else {
                return;
            };
            predecessor.next = target.next.take();
        }

        self.idxpool.push_back(recycled);
        self.idxptr += 1;
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
