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
        // ecap = (n-1) + (n-1)/3, with overflow check matching the C version:
        // if MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1)/3, then overflow.
        let nm1 = n - 1;
        let third = nm1 / 3;
        let limit = std::cmp::min(i32::MAX as usize, usize::MAX);
        if limit - nm1 < third {
            return None;
        }
        let ecap = nm1 + third;
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
            // No capacity for edges; only the root (node 0) exists.
            if creat != 0 {
                // Cannot create any new node.
                return -1;
            }
            return -1;
        }
        let h = ((from as u64).wrapping_mul(self.alphsz as u64)
            + sym as u64) as usize
            % (self.ecap as usize);

        // Search the bucket for an existing edge.
        {
            let mut cur = &self.etab[h];
            while let Some(node) = cur {
                if node.from == from && node.sym == sym {
                    return node.to;
                }
                cur = &node.next;
            }
        }

        if creat != 0 {
            // No edge found; allocate one if there's space.
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
            let next = self.etab[h].take();
            self.etab[h] = Some(Box::new(ChtrieEdge {
                next,
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
        let h = ((from as u64).wrapping_mul(self.alphsz as u64)
            + sym as u64) as usize
            % (self.ecap as usize);

        // Take the chain, decompose it into individual nodes preserving order.
        let mut chain = self.etab[h].take();
        let mut nodes: Vec<Box<ChtrieEdge>> = Vec::new();
        while let Some(mut node) = chain {
            chain = node.next.take();
            nodes.push(node);
        }

        // Locate the first matching edge.
        let mut match_idx: Option<usize> = None;
        for (i, node) in nodes.iter().enumerate() {
            if node.from == from && node.sym == sym {
                match_idx = Some(i);
                break;
            }
        }

        match match_idx {
            None => {
                // Rebuild the chain unchanged.
                let mut head: Option<Box<ChtrieEdge>> = None;
                for mut node in nodes.into_iter().rev() {
                    node.next = head;
                    head = Some(node);
                }
                self.etab[h] = head;
            }
            Some(idx) => {
                // Replicate the C semantics:
                //   if (q) q->next = p->next; else tr->etab[h] = NULL;
                // i.e., if the match is the head, the entire chain is dropped;
                // otherwise, the matching node is unlinked.
                let to = nodes[idx].to;
                if idx == 0 {
                    // Head match: set etab[h] = NULL (drop the whole chain).
                    self.etab[h] = None;
                } else {
                    // Unlink node at idx from the rebuilt chain.
                    let mut kept: Vec<Box<ChtrieEdge>> =
                        Vec::with_capacity(nodes.len() - 1);
                    for (i, node) in nodes.into_iter().enumerate() {
                        if i != idx {
                            kept.push(node);
                        }
                    }
                    let mut head: Option<Box<ChtrieEdge>> = None;
                    for mut node in kept.into_iter().rev() {
                        node.next = head;
                        head = Some(node);
                    }
                    self.etab[h] = head;
                }
                // Push the freed node index onto the idxpool stack.
                self.idxpool.push_back(to);
            }
        }
    }

    pub fn free(&mut self) {
        // Rust handles deallocation via Drop, but we mimic the explicit
        // free by clearing the structures.
        self.etab.clear();
        self.idxpool.clear();
        self.idxmax = 0;
        self.maxn = 0;
        self.alphsz = 0;
        self.ecap = 0;
        self.idxptr = 0;
    }
}
pub fn chtrie_walk(trie: &mut Chtrie, from: i32, sym: i32, creat: i32) -> i32 {
    trie.walk(from, sym, creat)
}
pub fn chtrie_del(trie: &mut Chtrie, from: i32, sym: i32) {
    trie.del(from, sym)
}
