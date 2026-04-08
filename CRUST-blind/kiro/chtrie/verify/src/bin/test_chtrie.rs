use chtrie::chtrie::{Chtrie, chtrie_walk, chtrie_del};

#[test]
fn test_new_valid() {
    let tr = Chtrie::new(10, 4);
    assert!(tr.is_some());
    let tr = tr.unwrap();
    assert_eq!(tr.maxn, 10);
    assert_eq!(tr.alphsz, 4);
    assert_eq!(tr.ecap, 12); // (10-1)+(10-1)/3 = 9+3
    assert_eq!(tr.idxmax, 1);
    assert_eq!(tr.idxptr, 0);
    assert_eq!(tr.etab.len(), 12);
}

#[test]
fn test_new_regulates_zero() {
    // n=0,m=0 regulated to n=1,m=1; ecap=(1-1)+(1-1)/3=0
    let tr = Chtrie::new(0, 0);
    assert!(tr.is_some());
    let tr = tr.unwrap();
    assert_eq!(tr.maxn, 1);
    assert_eq!(tr.alphsz, 1);
    assert_eq!(tr.ecap, 0);
}

#[test]
fn test_walk_no_create_returns_neg1() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    assert_eq!(tr.walk(0, 0, 0), -1);
}

#[test]
fn test_walk_create_returns_1() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
}

#[test]
fn test_walk_lookup_after_create() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    assert_eq!(tr.walk(0, 0, 0), 1);
}

#[test]
fn test_walk_multiple_children() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    assert_eq!(tr.walk(0, 1, 1), 2);
    assert_eq!(tr.walk(1, 2, 1), 3);
    assert_eq!(tr.walk(1, 3, 1), 4);
}

#[test]
fn test_del_and_lookup() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    tr.del(0, 0);
    assert_eq!(tr.walk(0, 0, 0), -1);
}

#[test]
fn test_del_reuse_index_lifo() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1); // n1=1
    assert_eq!(tr.walk(0, 1, 1), 2); // n2=2
    tr.del(0, 0); // recycle 1
    assert_eq!(tr.walk(0, 2, 1), 1); // reuses 1 (LIFO)
}

#[test]
fn test_del_multiple_reuse_lifo() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1); // a=1
    assert_eq!(tr.walk(0, 1, 1), 2); // b=2
    assert_eq!(tr.walk(0, 2, 1), 3); // c=3
    tr.del(0, 1); // recycle 2
    tr.del(0, 2); // recycle 3
    // LIFO: last deleted (3) popped first
    assert_eq!(tr.walk(0, 3, 1), 3);
    assert_eq!(tr.walk(0, 1, 1), 2);
}

#[test]
fn test_del_nonexistent() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    tr.del(0, 0); // no-op, should not panic
    assert_eq!(tr.walk(0, 0, 0), -1);
}

#[test]
fn test_capacity_limit() {
    // n=2: root(0) + 1 node max
    let mut tr = Chtrie::new(2, 1).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    assert_eq!(tr.walk(1, 0, 1), -1); // full
}

#[test]
fn test_recreate_after_del() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    assert_eq!(tr.walk(1, 2, 1), 3 - 1); // idxmax goes 1,2 so this is 2
    // Let me redo: sequential creation
    let mut tr = Chtrie::new(10, 4).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    assert_eq!(tr.walk(0, 1, 1), 2);
    assert_eq!(tr.walk(1, 2, 1), 3);
    assert_eq!(tr.walk(1, 3, 1), 4);
    tr.del(0, 0);
    assert_eq!(tr.walk(0, 0, 0), -1);
    assert_eq!(tr.walk(0, 0, 1), 1); // reuses recycled index 1
    // edges from old node 1 still exist
    assert_eq!(tr.walk(1, 2, 0), 3);
}

#[test]
fn test_large_alphabet() {
    let mut tr = Chtrie::new(100, 256).unwrap();
    assert_eq!(tr.walk(0, 255, 1), 1);
    assert_eq!(tr.walk(0, 255, 0), 1);
}

#[test]
fn test_ecap_zero_walk_returns_neg1() {
    // n=1 => ecap=0, walk should return -1
    let mut tr = Chtrie::new(1, 1).unwrap();
    assert_eq!(tr.ecap, 0);
    assert_eq!(tr.walk(0, 0, 0), -1);
    assert_eq!(tr.walk(0, 0, 1), -1);
}

#[test]
fn test_free_clears() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    tr.walk(0, 0, 1);
    tr.free();
    assert!(tr.etab.is_empty());
    assert!(tr.idxpool.is_empty());
}

#[test]
fn test_chtrie_walk_free_fn() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    assert_eq!(chtrie_walk(&mut tr, 0, 0, 1), 1);
    assert_eq!(chtrie_walk(&mut tr, 0, 0, 0), 1);
}

#[test]
fn test_chtrie_del_free_fn() {
    let mut tr = Chtrie::new(10, 4).unwrap();
    assert_eq!(chtrie_walk(&mut tr, 0, 0, 1), 1);
    chtrie_del(&mut tr, 0, 0);
    assert_eq!(chtrie_walk(&mut tr, 0, 0, 0), -1);
}

#[test]
fn test_trie_dictionary() {
    // Replicate the C test: add dict1, dict2, delete stop words, add dict3, query
    let mut tr = Chtrie::new(65536, 256).unwrap();
    let mut term = vec![false; 65536];
    let mut nchild = vec![0i32; 65536];

    let dict1 = ["", "the", "a", "an"];
    let dict2 = ["he", "she", "his", "hers"];
    let dict3 = ["this", "that"];
    let stop = ["the", "an", "a"];

    let add = |tr: &mut Chtrie, term: &mut Vec<bool>, nchild: &mut Vec<i32>, s: &str| {
        let mut it = 0i32;
        for &b in s.as_bytes() {
            if tr.walk(it, b as i32, 0) == -1 {
                nchild[it as usize] += 1;
            }
            it = tr.walk(it, b as i32, 1);
            assert!(it >= 0);
        }
        term[it as usize] = true;
    };

    let del = |tr: &mut Chtrie, term: &mut Vec<bool>, nchild: &mut Vec<i32>, s: &str| {
        let mut nodes = Vec::new();
        let mut symbs = Vec::new();
        let mut it = 0i32;
        for &b in s.as_bytes() {
            nodes.push(it);
            symbs.push(b as i32);
            it = tr.walk(it, b as i32, 0);
            if it < 0 { return; }
        }
        if it < 0 || !term[it as usize] { return; }
        term[it as usize] = false;
        while it > 0 && !term[it as usize] && nchild[it as usize] == 0 {
            let n = nodes.len() - 1;
            let node = nodes[n];
            let sym = symbs[n];
            tr.del(node, sym);
            it = node;
            nchild[it as usize] -= 1;
            nodes.pop();
            symbs.pop();
        }
    };

    let query = |tr: &mut Chtrie, term: &Vec<bool>, s: &str| -> bool {
        let mut it = 0i32;
        for &b in s.as_bytes() {
            it = tr.walk(it, b as i32, 0);
            if it < 0 { return false; }
        }
        term[it as usize]
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
