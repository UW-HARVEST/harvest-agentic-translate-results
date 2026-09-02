//! Exhaustive brute-force differential sweep.
//!
//! Over a small alphabet the input space is finite and can be enumerated
//! completely, which is stronger than sampling: every reachable combination of
//! (numElem, physical dst size, dst contents, src contents) in the space below
//! is compared. `0` is included in the dst alphabet so terminated, unterminated
//! and terminator-at-every-position destinations all occur.

mod common;

use common::*;

/// Every sequence of length `len` over `alphabet`.
fn words(alphabet: &[i32], len: usize) -> Vec<Vec<i32>> {
    let mut out = vec![Vec::new()];
    for _ in 0..len {
        let mut next = Vec::with_capacity(out.len() * alphabet.len());
        for w in &out {
            for &c in alphabet {
                let mut v = w.clone();
                v.push(c);
                next.push(v);
            }
        }
        out = next;
    }
    out
}

#[test]
fn exhaustive_small_alphabet() {
    // dst alphabet includes 0 (terminator), a positive and a negative value.
    let dst_alpha = [0i32, 7, -3];
    // src is always NUL-terminated; its body avoids 0 so `len` is exact, and a
    // separate loop covers bodies that contain an internal 0.
    let src_body_alpha = [7i32, -3];

    let mut total = 0usize;
    for phys in 1usize..=6 {
        let dsts = words(&dst_alpha, phys);
        for n in 1..=phys {
            for dst in &dsts {
                for src_len in 0..=4usize {
                    for body in words(&src_body_alpha, src_len) {
                        let mut src = body;
                        src.push(0);
                        assert_same(
                            &Case::new(dst.clone(), n, Src::Own(src)),
                            &format!("exhaustive phys={phys} n={n} dst={dst:?}"),
                        );
                        total += 1;
                    }
                }
            }
        }
    }
    eprintln!("exhaustive_small_alphabet: {total} distinct input tuples compared");
    assert!(total > 150_000, "sweep unexpectedly small: {total}");
}

/// Same sweep with `numElem` deliberately larger than the physical allocation,
/// but only for destinations that terminate early enough that the C stays inside
/// the memory we own.
#[test]
fn exhaustive_numelem_exceeds_allocation() {
    let dst_alpha = [0i32, 7, -3];
    let src_body_alpha = [7i32, -3];
    let mut total = 0usize;

    // Physical allocation is padded to 32 elements; the logical prefix is what
    // varies, and every prefix here contains a 0 within the first 4 elements.
    for prefix_len in 1usize..=4 {
        for prefix in words(&dst_alpha, prefix_len) {
            if !prefix.contains(&0) {
                continue; // would run past the padding
            }
            let mut dst = prefix.clone();
            dst.resize(32, 7); // padding, deliberately non-zero
            for n in [prefix_len + 1, 8, 16, 32, 64, 1 << 12] {
                for src_len in 0..=3usize {
                    for body in words(&src_body_alpha, src_len) {
                        let mut src = body;
                        src.push(0);
                        assert_same(
                            &Case::new(dst.clone(), n, Src::Own(src)),
                            &format!("exhaustive-oversized n={n} prefix={prefix:?}"),
                        );
                        total += 1;
                    }
                }
            }
        }
    }
    eprintln!("exhaustive_numelem_exceeds_allocation: {total} tuples compared");
    assert!(total > 1_000, "sweep unexpectedly small: {total}");
}

/// Exhaustive sweep of the NULL/zero-length rejection space.
#[test]
fn exhaustive_rejection_space() {
    let dst_alpha = [0i32, 7, -3];
    let mut total = 0usize;
    for phys in 1usize..=4 {
        for dst in words(&dst_alpha, phys) {
            for n in [0usize, 1, phys, phys + 1, usize::MAX] {
                // src == NULL
                assert_same(
                    &Case::new(dst.clone(), n, Src::Null),
                    &format!("reject src=NULL n={n} dst={dst:?}"),
                );
                total += 1;
                // dst == NULL
                assert_same(
                    &Case::null_dst(n, Src::Null),
                    &format!("reject dst=NULL src=NULL n={n}"),
                );
                assert_same(
                    &Case::null_dst(n, Src::Own(vec![7, 0])),
                    &format!("reject dst=NULL src=ok n={n}"),
                );
                total += 2;
            }
        }
    }
    eprintln!("exhaustive_rejection_space: {total} tuples compared");
    assert!(total > 100);
}
