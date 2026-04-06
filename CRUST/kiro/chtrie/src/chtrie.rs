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
        let ecap = match (n - 1).checked_add((n - 1) / 3) {
            Some(e) if e <= i32::MAX as usize => e,
            _ => return None,
        };
        Some(Chtrie {
            etab: (0..ecap).map(|_| None).collect(),
            idxpool: VecDeque::new(),
            idxptr: 0,
            idxmax: 1,
            maxn: n as i32,
            alphsz: m as i32,
            ecap: ecap as i32,
        })
    }
    pub fn walk(&mut self, from: i32, sym: i32, creat: i32) -> i32 {
        chtrie_walk(self, from, sym, creat)
    }
    pub fn del(&mut self, from: i32, sym: i32) {
        chtrie_del(self, from, sym)
    }
    pub fn free(&mut self) {
        self.etab.clear();
        self.idxpool.clear();
    }
}
pub fn chtrie_walk(trie: &mut Chtrie, from: i32, sym: i32, creat: i32) -> i32 {
    let h = ((from as u64) * (trie.alphsz as u64) + (sym as u64)) % (trie.ecap as u64);
    let h = h as usize;
    // Search existing edges
    {
        let mut p = &trie.etab[h];
        while let Some(edge) = p {
            if edge.from == from && edge.sym == sym {
                return edge.to;
            }
            p = &edge.next;
        }
    }
    if creat != 0 {
        if trie.idxpool.is_empty() && trie.idxmax >= trie.maxn {
            return -1;
        }
        let to = if let Some(idx) = trie.idxpool.pop_back() {
            idx
        } else {
            let idx = trie.idxmax;
            trie.idxmax += 1;
            idx
        };
        let old_head = trie.etab[h].take();
        trie.etab[h] = Some(Box::new(ChtrieEdge {
            next: old_head,
            from,
            sym,
            to,
        }));
        return to;
    }
    -1
}
pub fn chtrie_del(trie: &mut Chtrie, from: i32, sym: i32) {
    let h = ((from as u64) * (trie.alphsz as u64) + (sym as u64)) % (trie.ecap as u64);
    let h = h as usize;
    // Check if head matches
    if let Some(ref edge) = trie.etab[h] {
        if edge.from == from && edge.sym == sym {
            let removed = trie.etab[h].take().unwrap();
            // C code sets etab[h] = NULL when removing head (drops rest of chain)
            trie.idxpool.push_back(removed.to);
            return;
        }
    }
    // Search in chain
    let mut cur = &mut trie.etab[h];
    loop {
        match cur {
            Some(edge) if edge.next.is_some() => {
                let next = edge.next.as_ref().unwrap();
                if next.from == from && next.sym == sym {
                    let removed = edge.next.take().unwrap();
                    edge.next = removed.next;
                    trie.idxpool.push_back(removed.to);
                    return;
                }
                cur = &mut cur.as_mut().unwrap().next;
            }
            _ => return,
        }
    }
}
