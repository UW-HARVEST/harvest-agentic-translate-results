use chtrie::chtrie::{Chtrie, chtrie_walk, chtrie_del};

// === new() tests ===

#[test]
fn new_basic() {
    let tr = Chtrie::new(10, 4).unwrap();
    assert_eq!(tr.maxn, 10);
    assert_eq!(tr.alphsz, 4);
    // ecap = (10-1) + (10-1)/3 = 9 + 3 = 12
    assert_eq!(tr.ecap, 12);
    assert_eq!(tr.idxmax, 1);
    assert_eq!(tr.etab.len(), 12);
}

#[test]
fn new_regulates_small_values() {
    // n=0,m=0 regulated to 1,1; ecap=(1-1)+(1-1)/3=0
    let tr = Chtrie::new(0, 0).unwrap();
    assert_eq!(tr.maxn, 1);
    assert_eq!(tr.alphsz, 1);
    assert_eq!(tr.ecap, 0);
}

#[test]
fn new_n2_m2() {
    let tr = Chtrie::new(2, 2).unwrap();
    assert_eq!(tr.maxn, 2);
    assert_eq!(tr.alphsz, 2);
    assert_eq!(tr.ecap, 1); // (2-1)+(2-1)/3 = 1+0 = 1
}

#[test]
fn new_n2_m1() {
    let tr = Chtrie::new(2, 1).unwrap();
    assert_eq!(tr.maxn, 2);
    assert_eq!(tr.alphsz, 1);
    assert_eq!(tr.ecap, 1);
}

#[test]
fn new_large() {
    let tr = Chtrie::new(65536, 256).unwrap();
    assert_eq!(tr.maxn, 65536);
    assert_eq!(tr.alphsz, 256);
    let ecap = 65535 + 65535 / 3;
    assert_eq!(tr.ecap, ecap as i32);
}

#[test]
fn new_overflow_returns_none() {
    // Very large n that would overflow ecap calculation
    let result = Chtrie::new(usize::MAX, 1);
    assert!(result.is_none());
}

#[test]
fn new_n_exceeds_i32_max() {
    let result = Chtrie::new(i32::MAX as usize + 1, 1);
    assert!(result.is_none());
}

#[test]
fn new_m_exceeds_i32_max() {
    let result = Chtrie::new(10, i32::MAX as usize + 1);
    assert!(result.is_none());
}

// === walk() tests ===

#[test]
fn walk_lookup_empty_returns_neg1() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    assert_eq!(tr.walk(0, 0, 0), -1);
    assert_eq!(tr.walk(0, 1, 0), -1);
    assert_eq!(tr.walk(0, 3, 0), -1);
}

#[test]
fn walk_create_assigns_sequential_ids() {
    let mut tr = Chtrie::new(100, 4).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    assert_eq!(tr.walk(0, 1, 1), 2);
    assert_eq!(tr.walk(0, 2, 1), 3);
}

#[test]
fn walk_lookup_after_create() {
    let mut tr = Chtrie::new(100, 4).unwrap();
    tr.walk(0, 0, 1);
    tr.walk(0, 1, 1);
    assert_eq!(tr.walk(0, 0, 0), 1);
    assert_eq!(tr.walk(0, 1, 0), 2);
    assert_eq!(tr.walk(0, 2, 0), -1);
}

#[test]
fn walk_create_idempotent() {
    let mut tr = Chtrie::new(100, 4).unwrap();
    let a = tr.walk(0, 0, 1);
    let b = tr.walk(0, 0, 1); // same edge, should return same node
    assert_eq!(a, b);
    assert_eq!(a, 1);
}

#[test]
fn walk_chain_nodes() {
    let mut tr = Chtrie::new(100, 256).unwrap();
    // Build a path: 0 -'h'-> 1 -'e'-> 2
    let n1 = tr.walk(0, b'h' as i32, 1);
    let n2 = tr.walk(n1, b'e' as i32, 1);
    assert_eq!(n1, 1);
    assert_eq!(n2, 2);
    // Lookup
    assert_eq!(tr.walk(0, b'h' as i32, 0), 1);
    assert_eq!(tr.walk(1, b'e' as i32, 0), 2);
}

#[test]
fn walk_capacity_full_returns_neg1() {
    // n=2: only node 0 (root) and node 1 possible
    let mut tr = Chtrie::new(2, 2).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    assert_eq!(tr.walk(0, 1, 1), -1); // full
}

// === del() tests ===

#[test]
fn del_nonexistent_is_noop() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    tr.del(0, 0); // no-op on empty
    // Still works fine
    assert_eq!(tr.walk(0, 0, 1), 1);
}

#[test]
fn del_removes_edge() {
    let mut tr = Chtrie::new(100, 4).unwrap();
    tr.walk(0, 0, 1); // node 1
    assert_eq!(tr.walk(0, 0, 0), 1);
    tr.del(0, 0);
    assert_eq!(tr.walk(0, 0, 0), -1);
}

#[test]
fn del_head_drops_chain() {
    // Create two edges that hash to the same bucket
    // h = (from * alphsz + sym) % ecap
    // With n=10, m=4: ecap = 9+3 = 12
    // (0,0) -> h=0, (3,0) -> h=(12)%12=0 — same bucket!
    let mut tr = Chtrie::new(10, 4).unwrap();
    let a = tr.walk(0, 0, 1); // h=0, node 1
    let b = tr.walk(3, 0, 1); // h=0, node 2 (head of chain)
    assert_eq!(a, 1);
    assert_eq!(b, 2);

    // Both should be findable
    assert_eq!(tr.walk(0, 0, 0), 1);
    assert_eq!(tr.walk(3, 0, 0), 2);

    // Delete head (3,0) — C bug: etab[h]=NULL drops entire chain
    tr.del(3, 0);
    assert_eq!(tr.walk(3, 0, 0), -1);
    assert_eq!(tr.walk(0, 0, 0), -1); // collateral damage from bug
}

#[test]
fn del_nonhead_preserves_chain() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    tr.walk(0, 0, 1); // h=0, node 1 (tail)
    tr.walk(3, 0, 1); // h=0, node 2 (head)

    // Delete non-head (0,0) — correctly unlinks just this node
    tr.del(0, 0);
    assert_eq!(tr.walk(0, 0, 0), -1);
    assert_eq!(tr.walk(3, 0, 0), 2); // preserved
}

// === Index reuse (LIFO) tests ===

#[test]
fn del_then_create_reuses_index_lifo() {
    let mut tr = Chtrie::new(100, 4).unwrap();
    let a = tr.walk(0, 0, 1); // node 1
    let b = tr.walk(0, 1, 1); // node 2
    let c = tr.walk(0, 2, 1); // node 3
    assert_eq!((a, b, c), (1, 2, 3));

    tr.del(0, 1); // pushes 2
    tr.del(0, 2); // pushes 3

    // LIFO: pop 3 first, then 2
    let d = tr.walk(0, 3, 1);
    let e = tr.walk(1, 0, 1);
    assert_eq!(d, 3);
    assert_eq!(e, 2);
}

// === free() test ===

#[test]
fn free_clears_data() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    tr.walk(0, 0, 1);
    tr.free();
    assert!(tr.etab.is_empty());
    assert!(tr.idxpool.is_empty());
}

// === Module-level function wrappers ===

#[test]
fn chtrie_walk_wrapper() {
    let mut tr = Chtrie::new(100, 4).unwrap();
    assert_eq!(chtrie_walk(&mut tr, 0, 0, 1), 1);
    assert_eq!(chtrie_walk(&mut tr, 0, 0, 0), 1);
}

#[test]
fn chtrie_del_wrapper() {
    let mut tr = Chtrie::new(100, 4).unwrap();
    chtrie_walk(&mut tr, 0, 0, 1);
    chtrie_del(&mut tr, 0, 0);
    assert_eq!(chtrie_walk(&mut tr, 0, 0, 0), -1);
}

// === Full trie integration test (matches C test.c) ===

#[test]
fn full_trie_dict_test() {
    let n: usize = 65536;
    let m: usize = 256;
    let mut tr = Chtrie::new(n, m).unwrap();
    let mut term = vec![false; n];
    let mut nchild = vec![0i32; n];

    let dict1 = ["", "the", "a", "an"];
    let dict2 = ["he", "she", "his", "hers"];
    let dict3 = ["this", "that"];
    let stop = ["the", "an", "a"];

    let add = |tr: &mut Chtrie, term: &mut Vec<bool>, nchild: &mut Vec<i32>, s: &str| {
        let mut it: i32 = 0;
        for &ch in s.as_bytes() {
            if tr.walk(it, ch as i32, 0) == -1 {
                nchild[it as usize] += 1;
            }
            it = tr.walk(it, ch as i32, 1);
            assert!(it >= 0);
        }
        term[it as usize] = true;
    };

    let del = |tr: &mut Chtrie, term: &mut Vec<bool>, nchild: &mut Vec<i32>, s: &str| {
        let mut nodes = Vec::new();
        let mut symbs = Vec::new();
        let mut it: i32 = 0;
        for &ch in s.as_bytes() {
            nodes.push(it);
            symbs.push(ch as i32);
            it = tr.walk(it, ch as i32, 0);
            if it < 0 { break; }
        }
        if it < 0 || !term[it as usize] { return; }
        term[it as usize] = false;
        while it > 0 && !term[it as usize] && nchild[it as usize] == 0 {
            let n = nodes.len() - 1;
            let node = nodes[n];
            let sym = symbs[n];
            nodes.pop();
            symbs.pop();
            tr.del(node, sym);
            it = node;
            nchild[it as usize] -= 1;
        }
    };

    let query = |tr: &mut Chtrie, term: &Vec<bool>, s: &str| -> bool {
        let mut it: i32 = 0;
        for &ch in s.as_bytes() {
            it = tr.walk(it, ch as i32, 0);
            if it < 0 { return false; }
        }
        it >= 0 && term[it as usize]
    };

    for s in &dict1 { add(&mut tr, &mut term, &mut nchild, s); }
    for s in &dict2 { add(&mut tr, &mut term, &mut nchild, s); }
    for s in &stop  { del(&mut tr, &mut term, &mut nchild, s); }
    for s in &dict3 { add(&mut tr, &mut term, &mut nchild, s); }

    let cases = [
        ("hello", false), ("the", false), ("his", true), ("he", true),
        ("his", true), ("go", false), ("he", true), ("a", false),
        ("an", false), ("this", true), ("that", true), ("hey", false),
        ("she", true), ("hers", true),
    ];
    for (word, expected) in &cases {
        assert_eq!(query(&mut tr, &term, word), *expected, "query({}) failed", word);
    }
}

fn main() {}
