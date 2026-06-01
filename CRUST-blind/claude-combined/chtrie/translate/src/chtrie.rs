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
        // Match C: MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1)/3
        let limit = (i32::MAX as usize).min(SZ_MAX);
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
        chtrie_walk(self, from, sym, creat)
    }
    pub fn del(&mut self, from: i32, sym: i32) {
        chtrie_del(self, from, sym)
    }
    pub fn free(&mut self) {
        // In Rust, memory is freed automatically when the struct is dropped.
        // For an explicit free, we can iteratively drop linked lists to avoid
        // recursive Drop stack overflow on very long chains.
        for slot in self.etab.iter_mut() {
            let mut cur = slot.take();
            while let Some(mut node) = cur {
                cur = node.next.take();
            }
        }
        self.etab.clear();
        self.idxpool.clear();
        self.idxptr = 0;
        self.idxmax = 0;
        self.ecap = 0;
    }
}
pub fn chtrie_walk(trie: &mut Chtrie, from: i32, sym: i32, creat: i32) -> i32 {
    let h = ((from as u64).wrapping_mul(trie.alphsz as u64).wrapping_add(sym as u64))
        % (trie.ecap as u64);
    let h = h as usize;

    // Search
    {
        let mut cur = trie.etab[h].as_deref();
        while let Some(p) = cur {
            if p.from == from && p.sym == sym {
                return p.to;
            }
            cur = p.next.as_deref();
        }
    }

    if creat != 0 {
        if trie.idxptr == 0 && trie.idxmax >= trie.maxn {
            return -1;
        }
        let to = if trie.idxptr != 0 {
            trie.idxptr -= 1;
            // Pop from "stack" top (back end of VecDeque)
            trie.idxpool.pop_back().unwrap_or(-1)
        } else {
            let v = trie.idxmax;
            trie.idxmax += 1;
            v
        };
        let new_node = Box::new(ChtrieEdge {
            next: trie.etab[h].take(),
            from,
            sym,
            to,
        });
        trie.etab[h] = Some(new_node);
        return to;
    }
    -1
}
pub fn chtrie_del(trie: &mut Chtrie, from: i32, sym: i32) {
    let h = ((from as u64).wrapping_mul(trie.alphsz as u64).wrapping_add(sym as u64))
        % (trie.ecap as u64);
    let h = h as usize;

    // Check if there's anything in the bucket
    if trie.etab[h].is_none() {
        return;
    }

    // Check if head matches (q == NULL case in C)
    let head_matches = {
        let head = trie.etab[h].as_ref().unwrap();
        head.from == from && head.sym == sym
    };

    if head_matches {
        // Match C exactly: tr->etab[h] = NULL (drops the entire chain).
        // Take the head, push its `to` back into the pool, drop the rest.
        let mut removed = trie.etab[h].take().unwrap();
        let to = removed.to;
        // Iteratively drop the chain (the rest is "leaked" in C, but in Rust
        // we can drop them; this matches the practical effect that the bucket
        // becomes empty).
        let mut cur = removed.next.take();
        while let Some(mut node) = cur {
            cur = node.next.take();
        }
        trie.idxpool.push_back(to);
        trie.idxptr += 1;
        return;
    }

    // Walk the chain looking for a match in `next`.
    let mut current = trie.etab[h].as_mut().unwrap();
    loop {
        let next_matches = match current.next.as_ref() {
            None => return,
            Some(n) => n.from == from && n.sym == sym,
        };
        if next_matches {
            let mut removed = current.next.take().unwrap();
            current.next = removed.next.take();
            trie.idxpool.push_back(removed.to);
            trie.idxptr += 1;
            return;
        }
        // Advance
        current = current.next.as_mut().unwrap();
    }
}
