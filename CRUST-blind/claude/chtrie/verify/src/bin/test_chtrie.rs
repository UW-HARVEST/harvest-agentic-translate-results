#[allow(unused_imports)]
use chtrie::chtrie::Chtrie;

#[test]
fn test_alloc_basic() {
    // alloc(10, 5)
    let tr = Chtrie::new(10, 5).unwrap();
    assert_eq!(tr.maxn, 10);
    assert_eq!(tr.alphsz, 5);
    assert_eq!(tr.ecap, 12);
    assert_eq!(tr.idxmax, 1);
    assert_eq!(tr.idxptr, 0);
    assert!(tr.idxpool.is_empty());
    assert_eq!(tr.etab.len(), 12);
}

#[test]
fn test_alloc_zero_zero_regulated() {
    // alloc(0, 0) -> regulated to (1, 1), ecap = 0
    let tr = Chtrie::new(0, 0).unwrap();
    assert_eq!(tr.maxn, 1);
    assert_eq!(tr.alphsz, 1);
    assert_eq!(tr.ecap, 0);
    assert_eq!(tr.idxmax, 1);
}

#[test]
fn test_alloc_zero_five_regulated() {
    // alloc(0, 5) -> (1, 5), ecap = 0
    let tr = Chtrie::new(0, 5).unwrap();
    assert_eq!(tr.maxn, 1);
    assert_eq!(tr.alphsz, 5);
    assert_eq!(tr.ecap, 0);
}

#[test]
fn test_alloc_one_five() {
    // alloc(1, 5): ecap = 0
    let tr = Chtrie::new(1, 5).unwrap();
    assert_eq!(tr.maxn, 1);
    assert_eq!(tr.alphsz, 5);
    assert_eq!(tr.ecap, 0);
}

#[test]
fn test_alloc_various_ecap() {
    // From running the C program:
    // alloc(2, 5): ecap=1
    // alloc(3, 5): ecap=2
    // alloc(4, 5): ecap=4
    // alloc(5, 5): ecap=5
    // alloc(100, 5): ecap=132
    // alloc(1000, 5): ecap=1332
    let cases = [(2usize, 1i32), (3, 2), (4, 4), (5, 5), (100, 132), (1000, 1332)];
    for &(n, expected_ecap) in &cases {
        let tr = Chtrie::new(n, 5).unwrap();
        assert_eq!(tr.maxn, n as i32);
        assert_eq!(tr.alphsz, 5);
        assert_eq!(tr.ecap, expected_ecap, "n={}", n);
    }
}

#[test]
fn test_alloc_too_large_returns_none() {
    // n > i32::MAX must yield None
    let tr = Chtrie::new((i32::MAX as usize) + 1, 5);
    assert!(tr.is_none());
    let tr = Chtrie::new(5, (i32::MAX as usize) + 1);
    assert!(tr.is_none());
}

#[test]
fn test_walk_no_creat_returns_minus_one() {
    let mut tr = Chtrie::new(10, 5).unwrap();
    assert_eq!(tr.walk(0, 1, 0), -1);
}

#[test]
fn test_walk_creates_node_returns_index() {
    let mut tr = Chtrie::new(10, 5).unwrap();
    let r = tr.walk(0, 1, 1);
    assert_eq!(r, 1);
    assert_eq!(tr.idxmax, 2);
    let r2 = tr.walk(0, 2, 1);
    assert_eq!(r2, 2);
    assert_eq!(tr.idxmax, 3);
    let r3 = tr.walk(1, 0, 1);
    assert_eq!(r3, 3);
    assert_eq!(tr.idxmax, 4);
}

#[test]
fn test_walk_finds_existing_node_no_create() {
    let mut tr = Chtrie::new(10, 5).unwrap();
    let r = tr.walk(0, 1, 1);
    assert_eq!(r, 1);
    let r2 = tr.walk(0, 1, 0);
    assert_eq!(r2, 1);
    let r3 = tr.walk(0, 1, 1);
    assert_eq!(r3, 1);
    // No new node created.
    assert_eq!(tr.idxmax, 2);
}

#[test]
fn test_walk_full_capacity() {
    // alloc(4, 3): ecap=4
    let mut tr = Chtrie::new(4, 3).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    assert_eq!(tr.walk(0, 1, 1), 2);
    assert_eq!(tr.walk(0, 2, 1), 3);
    // Now full (idxmax=4=maxn): should fail to create.
    assert_eq!(tr.walk(1, 0, 1), -1);
    // Delete one, should be able to create again.
    tr.del(0, 1);
    assert_eq!(tr.walk(1, 0, 1), 2);
}

#[test]
fn test_walk_capacity_two_two() {
    // alloc(2, 2): ecap=1
    let mut tr = Chtrie::new(2, 2).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    // Now idxmax=2=maxn, no more space.
    assert_eq!(tr.walk(0, 1, 1), -1);
}

#[test]
fn test_walk_capacity_two_one_loop_returns_existing() {
    // alloc(2, 1): ecap=1.
    // First walk(0,0,1) creates node 1; subsequent walk(0,0,1) finds existing.
    let mut tr = Chtrie::new(2, 1).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    assert_eq!(tr.walk(0, 0, 1), 1);
}

#[test]
fn test_del_then_walk_returns_minus_one() {
    let mut tr = Chtrie::new(10, 5).unwrap();
    assert_eq!(tr.walk(0, 1, 1), 1);
    assert_eq!(tr.walk(1, 0, 1), 2);
    assert_eq!(tr.walk(1, 0, 0), 2);
    tr.del(1, 0);
    assert_eq!(tr.walk(1, 0, 0), -1);
}

#[test]
fn test_del_re_add_reuses_index() {
    let mut tr = Chtrie::new(10, 5).unwrap();
    assert_eq!(tr.walk(0, 1, 1), 1);
    assert_eq!(tr.walk(1, 0, 1), 2);
    tr.del(1, 0);
    // Re-create: should reuse index 2 from the pool.
    assert_eq!(tr.walk(1, 0, 1), 2);
}

#[test]
fn test_del_lifo_reuse() {
    // From running C: delete two, reuse should be LIFO.
    let mut tr = Chtrie::new(10, 5).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    assert_eq!(tr.walk(0, 1, 1), 2);
    assert_eq!(tr.walk(0, 2, 1), 3);
    assert_eq!(tr.walk(0, 3, 1), 4);
    tr.del(0, 0); // releases 1
    tr.del(0, 2); // releases 3
    // Next allocation should reuse the most recently freed (3).
    let e = tr.walk(5, 0, 1);
    assert_eq!(e, 3, "expected reuse of node 3");
    let f = tr.walk(6, 0, 1);
    assert_eq!(f, 1, "expected reuse of node 1");
    // Pool drained, idxmax bumps next.
    let g = tr.walk(7, 0, 1);
    assert_eq!(g, 5, "expected new node 5 from idxmax");
}

#[test]
fn test_del_nonexistent_is_noop() {
    let mut tr = Chtrie::new(10, 5).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    // Deleting something that does not exist must not change state.
    tr.del(5, 5);
    assert_eq!(tr.walk(0, 0, 0), 1);
    assert_eq!(tr.idxmax, 2);
    assert!(tr.idxpool.is_empty());
}

#[test]
fn test_del_head_drops_chain_tail() {
    // Reproduce the C behaviour: deleting the head of a non-empty chain
    // sets the bucket to None (the C code is `etab[h] = NULL`). The other
    // entries become unreachable.
    // alphsz=12, n=10 -> ecap=12, hash = (from*12 + sym) % 12 = sym%12.
    // So (0,0) and (1,0) collide on bucket 0.
    let mut tr = Chtrie::new(10, 12).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    assert_eq!(tr.walk(1, 0, 1), 2); // becomes head of bucket 0
    tr.del(1, 0);
    // Head removed; tail (the (0,0,1) entry) is also dropped per C semantics.
    assert_eq!(tr.walk(0, 0, 0), -1);
    assert_eq!(tr.walk(1, 0, 0), -1);
}

#[test]
fn test_del_middle_of_chain() {
    // ecap=12. Build chain (3,0,4) -> (2,0,3) -> (1,0,2) -> (0,0,1).
    let mut tr = Chtrie::new(20, 12).unwrap();
    assert_eq!(tr.walk(0, 0, 1), 1);
    assert_eq!(tr.walk(1, 0, 1), 2);
    assert_eq!(tr.walk(2, 0, 1), 3);
    assert_eq!(tr.walk(3, 0, 1), 4);
    tr.del(1, 0); // delete a non-head entry
    // The remaining entries must still be reachable.
    assert_eq!(tr.walk(0, 0, 0), 1);
    assert_eq!(tr.walk(1, 0, 0), -1);
    assert_eq!(tr.walk(2, 0, 0), 3);
    assert_eq!(tr.walk(3, 0, 0), 4);
    // The pool should have the freed index (2).
    let reused = tr.walk(4, 0, 1);
    assert_eq!(reused, 2);
}

#[test]
fn test_dictionary_workflow_matches_c() {
    // Full replay of the C example from c_src/tests/test.c.
    let mut tr = Chtrie::new(65536, 256).unwrap();
    assert_eq!(tr.maxn, 65536);
    assert_eq!(tr.alphsz, 256);
    assert_eq!(tr.ecap, 87380);

    // Termination flags and child counts, exactly like the C harness.
    let n = 65536usize;
    let mut term = vec![0i32; n];
    let mut nchild = vec![0i32; n];

    // Helper closures emulating the C `add` and `del` functions inline so we
    // call walk/del directly.
    let add = |tr: &mut Chtrie, term: &mut [i32], nchild: &mut [i32], s: &str| {
        let mut it = 0i32;
        for byte in s.bytes() {
            let sym = byte as i32;
            if tr.walk(it, sym, 0) == -1 {
                nchild[it as usize] += 1;
            }
            it = tr.walk(it, sym, 1);
            assert!(it != -1);
        }
        term[it as usize] = 1;
    };

    let del_word = |tr: &mut Chtrie, term: &mut [i32], nchild: &mut [i32], s: &str| {
        let bytes: Vec<u8> = s.bytes().collect();
        let mut nodes = vec![0i32; bytes.len()];
        let mut symbs = vec![0i32; bytes.len()];
        let mut it = 0i32;
        let mut count: usize = 0;
        for (i, &b) in bytes.iter().enumerate() {
            if it < 0 {
                break;
            }
            nodes[i] = it;
            symbs[i] = b as i32;
            count += 1;
            it = tr.walk(it, b as i32, 0);
        }
        if it < 0 || term[it as usize] == 0 {
            return;
        }
        term[it as usize] = 0;
        while it > 0 && term[it as usize] == 0 && nchild[it as usize] == 0 {
            count -= 1;
            tr.del(nodes[count], symbs[count]);
            it = nodes[count];
            nchild[it as usize] -= 1;
        }
    };

    let query = |tr: &mut Chtrie, term: &[i32], s: &str| -> i32 {
        let mut it = 0i32;
        for b in s.bytes() {
            if it < 0 {
                break;
            }
            it = tr.walk(it, b as i32, 0);
        }
        if it >= 0 && term[it as usize] != 0 {
            1
        } else {
            0
        }
    };

    // Apply the dictionary additions/deletions exactly as the C test does.
    for w in &["", "the", "a", "an"] {
        add(&mut tr, &mut term, &mut nchild, w);
    }
    for w in &["he", "she", "his", "hers"] {
        add(&mut tr, &mut term, &mut nchild, w);
    }
    for w in &["the", "an", "a"] {
        del_word(&mut tr, &mut term, &mut nchild, w);
    }
    for w in &["this", "that"] {
        add(&mut tr, &mut term, &mut nchild, w);
    }

    let cases: &[(&str, i32)] = &[
        ("hello", 0),
        ("the", 0),
        ("his", 1),
        ("he", 1),
        ("his", 1),
        ("go", 0),
        ("he", 1),
        ("a", 0),
        ("an", 0),
        ("this", 1),
        ("that", 1),
        ("hey", 0),
        ("she", 1),
        ("hers", 1),
    ];
    for (word, expected) in cases {
        assert_eq!(query(&mut tr, &term, word), *expected, "query({})", word);
    }
}

#[test]
fn test_walk_multi_step_dictionary_node_indices() {
    // From running C: with maxn=65536, alphsz=256, building "the", "a", "an"
    // assigns nodes 1,2,3 to t,h,e and 4,5 to a,n.
    let mut tr = Chtrie::new(65536, 256).unwrap();
    let n1 = tr.walk(0, b't' as i32, 1);
    let n2 = tr.walk(n1, b'h' as i32, 1);
    let n3 = tr.walk(n2, b'e' as i32, 1);
    assert_eq!((n1, n2, n3), (1, 2, 3));

    let n4 = tr.walk(0, b'a' as i32, 1);
    assert_eq!(n4, 4);
    // Re-walk should NOT create a new node.
    let n4_again = tr.walk(0, b'a' as i32, 1);
    assert_eq!(n4_again, 4);
    let n5 = tr.walk(n4_again, b'n' as i32, 1);
    assert_eq!(n5, 5);

    // No-creat lookups:
    assert_eq!(tr.walk(0, b't' as i32, 0), 1);
    assert_eq!(tr.walk(1, b'h' as i32, 0), 2);
    assert_eq!(tr.walk(2, b'e' as i32, 0), 3);
    assert_eq!(tr.walk(0, b'a' as i32, 0), 4);
    assert_eq!(tr.walk(4, b'n' as i32, 0), 5);
}

#[test]
fn test_module_level_helpers() {
    // The module also exports free `chtrie_walk`/`chtrie_del` wrappers; make
    // sure they delegate to the methods.
    let mut tr = Chtrie::new(10, 5).unwrap();
    assert_eq!(chtrie::chtrie::chtrie_walk(&mut tr, 0, 0, 1), 1);
    assert_eq!(chtrie::chtrie::chtrie_walk(&mut tr, 0, 0, 0), 1);
    chtrie::chtrie::chtrie_del(&mut tr, 0, 0);
    assert_eq!(chtrie::chtrie::chtrie_walk(&mut tr, 0, 0, 0), -1);
}

#[test]
fn test_free_clears_state() {
    let mut tr = Chtrie::new(10, 5).unwrap();
    tr.walk(0, 0, 1);
    tr.free();
    assert_eq!(tr.maxn, 0);
    assert_eq!(tr.ecap, 0);
    assert_eq!(tr.idxmax, 0);
    assert_eq!(tr.idxptr, 0);
    assert!(tr.etab.is_empty());
    assert!(tr.idxpool.is_empty());
}

#[test]
fn test_sz_max_constant() {
    assert_eq!(chtrie::chtrie::SZ_MAX, usize::MAX);
}

fn main() {}
