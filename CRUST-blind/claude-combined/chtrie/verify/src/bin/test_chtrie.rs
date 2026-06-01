#![allow(dead_code, unused_imports)]
use chtrie::chtrie::{chtrie_del, chtrie_walk, Chtrie, SZ_MAX};

#[test]
fn test_sz_max_constant() {
    assert_eq!(SZ_MAX, usize::MAX);
}

#[test]
fn test_alloc_basic() {
    let tr = Chtrie::new(10, 26).expect("alloc failed");
    assert_eq!(tr.maxn, 10);
    assert_eq!(tr.alphsz, 26);
    assert_eq!(tr.ecap, 12);
    assert_eq!(tr.idxmax, 1);
    assert_eq!(tr.idxptr, 0);
    assert_eq!(tr.etab.len(), 12);
    assert!(tr.idxpool.is_empty());
}

#[test]
fn test_alloc_zero_args() {
    // Must regulate to (1, 1), ecap=0, idxmax=1
    let tr = Chtrie::new(0, 0).expect("alloc failed");
    assert_eq!(tr.maxn, 1);
    assert_eq!(tr.alphsz, 1);
    assert_eq!(tr.ecap, 0);
    assert_eq!(tr.idxmax, 1);
    assert_eq!(tr.idxptr, 0);
    assert_eq!(tr.etab.len(), 0);
}

#[test]
fn test_alloc_one_one() {
    let tr = Chtrie::new(1, 1).expect("alloc failed");
    assert_eq!(tr.maxn, 1);
    assert_eq!(tr.alphsz, 1);
    assert_eq!(tr.ecap, 0);
    assert_eq!(tr.idxmax, 1);
}

#[test]
fn test_alloc_two_two() {
    // ecap = (2-1) + (2-1)/3 = 1
    let tr = Chtrie::new(2, 2).expect("alloc failed");
    assert_eq!(tr.maxn, 2);
    assert_eq!(tr.alphsz, 2);
    assert_eq!(tr.ecap, 1);
}

#[test]
fn test_alloc_too_large() {
    // n > INT_MAX
    let n = (i32::MAX as usize) + 1;
    assert!(Chtrie::new(n, 1).is_none());
    // m > INT_MAX
    assert!(Chtrie::new(1, n).is_none());
}

#[test]
fn test_walk_create_basic_indexes() {
    let mut tr = Chtrie::new(10, 26).expect("alloc failed");
    let r1 = chtrie_walk(&mut tr, 0, 0, 1);
    let r2 = chtrie_walk(&mut tr, 0, 1, 1);
    let r3 = chtrie_walk(&mut tr, r1, 0, 1);
    let r4 = chtrie_walk(&mut tr, r1, 1, 1);
    let r5 = chtrie_walk(&mut tr, r2, 2, 1);
    assert_eq!(r1, 1);
    assert_eq!(r2, 2);
    assert_eq!(r3, 3);
    assert_eq!(r4, 4);
    assert_eq!(r5, 5);
    assert_eq!(tr.idxmax, 6);
}

#[test]
fn test_walk_lookup_existing_and_missing() {
    let mut tr = Chtrie::new(10, 26).expect("alloc failed");
    let r1 = chtrie_walk(&mut tr, 0, 0, 1);
    assert_eq!(r1, 1);

    // Existing edge: should return same idx, no creation
    let q1 = chtrie_walk(&mut tr, 0, 0, 0);
    assert_eq!(q1, 1);
    assert_eq!(tr.idxmax, 2);

    // Missing edge, no create
    let q2 = chtrie_walk(&mut tr, 0, 99, 0);
    assert_eq!(q2, -1);
    assert_eq!(tr.idxmax, 2);
}

#[test]
fn test_walk_create_then_existing_returns_same_index() {
    let mut tr = Chtrie::new(10, 26).expect("alloc failed");
    let r1 = chtrie_walk(&mut tr, 0, 5, 1);
    let r2 = chtrie_walk(&mut tr, 0, 5, 1); // exists, should return same
    assert_eq!(r1, r2);
    assert_eq!(r1, 1);
    assert_eq!(tr.idxmax, 2);
}

#[test]
fn test_walk_max_capacity() {
    // maxn=3 means indexes 0,1,2 — only 2 child slots after root
    let mut tr = Chtrie::new(3, 4).expect("alloc failed");
    assert_eq!(tr.ecap, 2);
    let a = chtrie_walk(&mut tr, 0, 0, 1);
    let b = chtrie_walk(&mut tr, 0, 1, 1);
    let c = chtrie_walk(&mut tr, 0, 2, 1);
    assert_eq!(a, 1);
    assert_eq!(b, 2);
    assert_eq!(c, -1);
    assert_eq!(tr.idxmax, 3);
}

#[test]
fn test_walk_at_capacity_two_two() {
    let mut tr = Chtrie::new(2, 2).expect("alloc failed");
    let r0 = chtrie_walk(&mut tr, 0, 0, 1);
    assert_eq!(r0, 1);
    let r1 = chtrie_walk(&mut tr, 0, 1, 1);
    assert_eq!(r1, -1);
}

#[test]
fn test_del_reuses_index() {
    let mut tr = Chtrie::new(10, 26).expect("alloc failed");
    let r1 = chtrie_walk(&mut tr, 0, 0, 1);
    let r2 = chtrie_walk(&mut tr, 0, 1, 1);
    assert_eq!(r1, 1);
    assert_eq!(r2, 2);

    // Delete edge (0, 1) -> r2 (=2) is reclaimed
    chtrie_del(&mut tr, 0, 1);
    assert_eq!(tr.idxptr, 1);

    // Lookup of (0,1) should fail now
    let q = chtrie_walk(&mut tr, 0, 1, 0);
    assert_eq!(q, -1);

    // Add new edge (0, 5) — should reuse pool slot (=2)
    let r3 = chtrie_walk(&mut tr, 0, 5, 1);
    assert_eq!(r3, 2);
    assert_eq!(tr.idxmax, 3); // not incremented
    assert_eq!(tr.idxptr, 0); // pool consumed
}

#[test]
fn test_del_nonexistent_is_noop() {
    let mut tr = Chtrie::new(10, 26).expect("alloc failed");
    chtrie_walk(&mut tr, 0, 0, 1);
    let before_idxmax = tr.idxmax;
    let before_idxptr = tr.idxptr;
    // empty bucket: deletion does nothing
    chtrie_del(&mut tr, 5, 5);
    // existing bucket but different (from, sym): does nothing
    chtrie_del(&mut tr, 0, 7);
    assert_eq!(tr.idxmax, before_idxmax);
    assert_eq!(tr.idxptr, before_idxptr);

    let q = chtrie_walk(&mut tr, 0, 0, 0);
    assert_eq!(q, 1); // still there
}

#[test]
fn test_del_tail_of_chain() {
    // ecap=9, alphsz=3, hash collision: (0,0) and (3,0) both hash to 0
    let mut tr = Chtrie::new(8, 3).expect("alloc failed");
    assert_eq!(tr.ecap, 9);
    let n00 = chtrie_walk(&mut tr, 0, 0, 1); // idx=1
    let _n01 = chtrie_walk(&mut tr, 0, 1, 1); // idx=2
    let n02 = chtrie_walk(&mut tr, 0, 2, 1); // idx=3
    let n30 = chtrie_walk(&mut tr, n02, 0, 1); // idx=4, collides at h=0
    assert_eq!(n00, 1);
    assert_eq!(n02, 3);
    assert_eq!(n30, 4);

    // Bucket 0: HEAD=(3,0) -> (0,0). Delete tail (0,0).
    chtrie_del(&mut tr, 0, 0);
    // (0,0) gone, (3,0) still there
    let l00 = chtrie_walk(&mut tr, 0, 0, 0);
    let l30 = chtrie_walk(&mut tr, n02, 0, 0);
    assert_eq!(l00, -1);
    assert_eq!(l30, 4);

    // Re-add (0,0) -> reuse n00 (=1)
    let re = chtrie_walk(&mut tr, 0, 0, 1);
    assert_eq!(re, 1);
}

#[test]
fn test_del_head_of_chain_destroys_chain() {
    // C semantics: when head matches, etab[h] is set to NULL
    // (the rest of the chain is leaked in C). We test the observable
    // effect: lookups for the rest of the chain also fail.
    let mut tr = Chtrie::new(8, 3).expect("alloc failed");
    assert_eq!(tr.ecap, 9);
    let n00 = chtrie_walk(&mut tr, 0, 0, 1); // idx=1, h=0
    let _n01 = chtrie_walk(&mut tr, 0, 1, 1); // idx=2
    let n02 = chtrie_walk(&mut tr, 0, 2, 1); // idx=3
    let n30 = chtrie_walk(&mut tr, n02, 0, 1); // idx=4, h=0
    assert_eq!(n00, 1);
    assert_eq!(n02, 3);
    assert_eq!(n30, 4);

    // Head of bucket 0 is (3,0). Delete it.
    chtrie_del(&mut tr, n02, 0);

    // Both (0,0) and (3,0) are now unreachable
    let l00 = chtrie_walk(&mut tr, 0, 0, 0);
    let l30 = chtrie_walk(&mut tr, n02, 0, 0);
    assert_eq!(l00, -1);
    assert_eq!(l30, -1);

    // Pool received n30 (=4). Next create reuses 4.
    let re = chtrie_walk(&mut tr, 0, 0, 1);
    assert_eq!(re, 4);
}

#[test]
fn test_walk_no_create_returns_minus_one() {
    let mut tr = Chtrie::new(10, 5).expect("alloc failed");
    let w = chtrie_walk(&mut tr, 0, 3, 0);
    assert_eq!(w, -1);
    assert_eq!(tr.idxmax, 1); // no creation
}

#[test]
fn test_method_walk_and_del() {
    // Verify the impl methods delegate the same behavior
    let mut tr = Chtrie::new(10, 26).expect("alloc failed");
    let r1 = tr.walk(0, 0, 1);
    let r2 = tr.walk(0, 1, 1);
    assert_eq!(r1, 1);
    assert_eq!(r2, 2);

    tr.del(0, 0);
    let q = tr.walk(0, 0, 0);
    assert_eq!(q, -1);

    let r3 = tr.walk(0, 5, 1);
    assert_eq!(r3, 1); // reused
}

// Trie-as-set test, mirroring the C test program. Add words, delete some,
// add more, then run a list of queries and assert exact expected outputs.
fn add(tr: &mut Chtrie, term: &mut [i32], nchild: &mut [i32], s: &str) {
    let mut it: i32 = 0;
    for &b in s.as_bytes() {
        let sym = b as i32;
        if chtrie_walk(tr, it, sym, 0) == -1 {
            nchild[it as usize] += 1;
        }
        it = chtrie_walk(tr, it, sym, 1);
        assert!(it != -1, "chtrie_walk failed");
    }
    term[it as usize] = 1;
}

fn del_word(tr: &mut Chtrie, term: &mut [i32], nchild: &mut [i32], s: &str) {
    let mut nodes: Vec<i32> = Vec::new();
    let mut symbs: Vec<i32> = Vec::new();
    let mut it: i32 = 0;
    let bytes = s.as_bytes();
    let mut idx = 0;
    while it >= 0 && idx < bytes.len() {
        nodes.push(it);
        symbs.push(bytes[idx] as i32);
        it = chtrie_walk(tr, it, bytes[idx] as i32, 0);
        idx += 1;
    }
    if it < 0 || term[it as usize] == 0 {
        return;
    }
    term[it as usize] = 0;
    while it > 0 && term[it as usize] == 0 && nchild[it as usize] == 0 {
        let n = nodes.len() - 1;
        chtrie_del(tr, nodes[n], symbs[n]);
        nodes.pop();
        symbs.pop();
        it = if n > 0 { *nodes.last().unwrap() } else { 0 };
        // Re-derive: the previous loop's nodes[n] was the parent; we need to
        // decrement that parent's nchild. But we already popped, so use it.
        nchild[it as usize] -= 1;
    }
}

fn query(tr: &mut Chtrie, term: &[i32], s: &str) -> i32 {
    let mut it: i32 = 0;
    for &b in s.as_bytes() {
        if it < 0 {
            break;
        }
        it = chtrie_walk(tr, it, b as i32, 0);
    }
    if it >= 0 && term[it as usize] != 0 {
        1
    } else {
        0
    }
}

#[test]
fn test_trie_dictionary_matches_c() {
    const N: usize = 65536;
    const M: usize = 256;
    let mut tr = Chtrie::new(N, M).expect("alloc failed");
    let mut term = vec![0i32; N];
    let mut nchild = vec![0i32; N];

    let dict1 = ["", "the", "a", "an"];
    let dict2 = ["he", "she", "his", "hers"];
    let dict3 = ["this", "that"];
    let stop = ["the", "an", "a"];

    for w in dict1.iter() {
        add(&mut tr, &mut term, &mut nchild, w);
    }
    for w in dict2.iter() {
        add(&mut tr, &mut term, &mut nchild, w);
    }
    for w in stop.iter() {
        del_word(&mut tr, &mut term, &mut nchild, w);
    }
    for w in dict3.iter() {
        add(&mut tr, &mut term, &mut nchild, w);
    }

    let cases = [
        "hello", "the", "his", "he", "his", "go", "he", "a", "an", "this", "that",
        "hey", "she", "hers",
    ];
    let expected = [0, 0, 1, 1, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1];

    for (c, e) in cases.iter().zip(expected.iter()) {
        let r = query(&mut tr, &term, c);
        assert_eq!(r, *e, "query({}): got {} expected {}", c, r, e);
    }
}

#[test]
fn test_free_clears_state() {
    let mut tr = Chtrie::new(10, 26).expect("alloc failed");
    chtrie_walk(&mut tr, 0, 0, 1);
    chtrie_walk(&mut tr, 0, 1, 1);
    tr.free();
    assert_eq!(tr.etab.len(), 0);
    assert_eq!(tr.ecap, 0);
    assert_eq!(tr.idxptr, 0);
    assert!(tr.idxpool.is_empty());
}

fn main() {}
