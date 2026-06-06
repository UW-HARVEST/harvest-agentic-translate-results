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
        let nm1 = n - 1;
        // Overflow check: (n-1) + (n-1)/3 must fit in i32
        if (i32::MAX as usize).saturating_sub(nm1) < nm1 / 3 {
            return None;
        }
        let ecap = nm1 + nm1 / 3;
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
        chtrie_walk(self, from, sym, creat)
    }
    pub fn del(&mut self, from: i32, sym: i32) {
        chtrie_del(self, from, sym)
    }
    pub fn free(&mut self) {
        // Iteratively drop linked-list buckets to avoid recursion-induced
        // stack overflow when chains are long.
        for slot in self.etab.iter_mut() {
            let mut head = slot.take();
            while let Some(mut node) = head {
                head = node.next.take();
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
        // Same iterative cleanup as `free` to prevent deep recursion drops.
        for slot in self.etab.iter_mut() {
            let mut head = slot.take();
            while let Some(mut node) = head {
                head = node.next.take();
            }
        }
    }
}

pub fn chtrie_walk(trie: &mut Chtrie, from: i32, sym: i32, creat: i32) -> i32 {
    if trie.ecap <= 0 {
        if creat != 0 {
            // Cannot create edges without bucket capacity.
            return -1;
        }
        return -1;
    }
    let ecap = trie.ecap as u64;
    let h = (from as u64)
        .wrapping_mul(trie.alphsz as u64)
        .wrapping_add(sym as u64)
        % ecap;
    let h = h as usize;

    // Search the chain for a matching edge.
    let mut cur = trie.etab[h].as_deref();
    while let Some(edge) = cur {
        if edge.from == from && edge.sym == sym {
            return edge.to;
        }
        cur = edge.next.as_deref();
    }

    if creat != 0 {
        // Allocate a new node index.
        if trie.idxpool.is_empty() && trie.idxmax >= trie.maxn {
            return -1;
        }
        let to: i32 = if !trie.idxpool.is_empty() {
            trie.idxpool.pop_back().unwrap()
        } else {
            let v = trie.idxmax;
            trie.idxmax += 1;
            v
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
    if trie.ecap <= 0 {
        return;
    }
    let ecap = trie.ecap as u64;
    let h = (from as u64)
        .wrapping_mul(trie.alphsz as u64)
        .wrapping_add(sym as u64)
        % ecap;
    let h = h as usize;

    // Check the head of the chain first.
    let head_matches = match &trie.etab[h] {
        Some(node) => node.from == from && node.sym == sym,
        None => false,
    };
    if head_matches {
        // Match the original C behavior: when the head matches,
        // the bucket is cleared (and the rest of the chain is dropped).
        let mut head = trie.etab[h].take().unwrap();
        let to = head.to;
        // Clear chain iteratively.
        let mut next = head.next.take();
        while let Some(mut node) = next {
            next = node.next.take();
        }
        trie.idxpool.push_back(to);
        return;
    }

    // Walk the chain looking for an edge whose `next` matches.
    let mut cur: &mut Option<Box<ChtrieEdge>> = &mut trie.etab[h];
    loop {
        match cur {
            None => return,
            Some(boxed) => {
                let next_matches = match &boxed.next {
                    Some(n) => n.from == from && n.sym == sym,
                    None => false,
                };
                if next_matches {
                    let mut removed = boxed.next.take().unwrap();
                    boxed.next = removed.next.take();
                    trie.idxpool.push_back(removed.to);
                    return;
                }
                cur = &mut boxed.next;
            }
        }
    }
}
