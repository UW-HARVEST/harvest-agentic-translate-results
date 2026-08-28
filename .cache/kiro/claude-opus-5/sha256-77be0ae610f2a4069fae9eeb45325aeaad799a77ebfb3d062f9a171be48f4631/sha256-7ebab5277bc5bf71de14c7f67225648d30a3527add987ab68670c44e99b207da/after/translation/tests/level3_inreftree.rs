//! Level 3: the public entry point `inreftree`, the only symbol declared in
//! `include/lib.h`.
//!
//! `inreftree` resets `node_count` but *not* `node_table`, builds a four-node
//! tree, sums it, and then indexes the literal `"+*-%"` with `tree_sum % 4`.
//! Because C's `%` truncates toward zero, a negative sum indexes before the
//! literal, so the negative branch is exercised deliberately below.

mod common;

use common::*;
use std::ffi::c_int;

fn assert_same(p: &Pair, a: c_int, b: c_int, c: c_int, d: c_int) {
    let cv = p.c.inreftree(a, b, c, d);
    let rv = p.rs.inreftree(a, b, c, d);
    assert_eq!(
        cv, rv,
        "inreftree({a}, {b}, {c}, {d}): C={cv} Rust={rv}"
    );
    assert_eq!(
        p.c.node_count(),
        p.rs.node_count(),
        "inreftree({a}, {b}, {c}, {d}): node_count diverged"
    );
    let cb = p.c.node_table_bytes();
    let rb = p.rs.node_table_bytes();
    if cb != rb {
        let ct = p.c.node_table();
        let rt = p.rs.node_table();
        for i in 0..MAX_NODES {
            assert_eq!(
                ct[i], rt[i],
                "inreftree({a}, {b}, {c}, {d}): node_table[{i}] diverged"
            );
        }
        panic!("inreftree({a}, {b}, {c}, {d}): table bytes differ");
    }
}

/// Both libraries start from a zeroed table so the untouched slots 4..50 are
/// comparable.
fn fresh(p: &Pair) {
    p.c.reset();
    p.rs.reset();
}

#[test]
fn inreftree_small_exhaustive_grid() {
    let p = load();
    fresh(&p);
    let vals: [c_int; 9] = [-4, -3, -2, -1, 0, 1, 2, 3, 4];
    let mut n = 0usize;
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    assert_same(&p, a, b, c, d);
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 9 * 9 * 9 * 9);
}

#[test]
fn inreftree_covers_every_residue_positive_and_negative() {
    let p = load();
    fresh(&p);
    // tree_sum is param1 + param2 + param3 + param4. Drive the sum across a
    // contiguous range so every value of `tree_sum % 4` (including the
    // negative residues -1, -2, -3 that read out of bounds in C) is hit, with
    // param2 both zero and non-zero so target_id takes both values.
    for sum in -60i32..=60 {
        for &b in &[0i32, 1, -1, 7] {
            let a = sum - b;
            assert_same(&p, a, b, 0, 0);
            assert_same(&p, 0, b, a, 0);
            assert_same(&p, 0, b, 0, a);
        }
    }
}

#[test]
fn inreftree_param2_zero_switches_target() {
    let p = load();
    fresh(&p);
    // target_id starts as node 2 ("left" contains 'l'); when that node's value
    // is 0 it falls back to 1. Sweep across that boundary for every residue.
    for sum_base in -12i32..=12 {
        assert_same(&p, sum_base, 0, 0, 0);
        assert_same(&p, sum_base - 1, 1, 0, 0);
        assert_same(&p, sum_base + 1, -1, 0, 0);
    }
}

#[test]
fn inreftree_extreme_values() {
    let p = load();
    fresh(&p);
    let vals: [c_int; 14] = [
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
        c_int::MAX / 2,
        c_int::MIN / 2,
        0x4000_0000,
        -0x4000_0000,
        0x3FFF_FFFF,
        -0x3FFF_FFFF,
        1_000_000_000,
        -1_000_000_000,
        0,
        1,
    ];
    for &a in &vals {
        for &b in &vals {
            assert_same(&p, a, b, 0, 0);
            assert_same(&p, a, b, 1, -1);
            assert_same(&p, a, 0, b, 0);
            assert_same(&p, 0, a, 0, b);
        }
    }
}

#[test]
fn inreftree_wide_pseudorandom_sweep() {
    let p = load();
    fresh(&p);
    // Deterministic xorshift sweep over the full int range.
    let mut s: u32 = 0x1234_5678;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as i32
    };
    for _ in 0..4000 {
        let (a, b, c, d) = (next(), next(), next(), next());
        assert_same(&p, a, b, c, d);
    }
    // And a sweep biased toward small magnitudes, where residues change fast.
    for _ in 0..4000 {
        let (a, b, c, d) = (
            next() % 64,
            next() % 64,
            next() % 64,
            next() % 64,
        );
        assert_same(&p, a, b, c, d);
    }
}

#[test]
fn inreftree_does_not_clear_stale_table_entries() {
    let p = load();
    // inreftree only resets node_count, so slots 4..50 keep whatever was
    // there. Pre-fill both tables identically and confirm the leftover bytes
    // agree afterwards.
    for fill in [0x00u8, 0xFF, 0xAB, 0x2D] {
        for lib in [p.c, p.rs] {
            let base = lib.node_table_ptr() as *mut u8;
            let len = MAX_NODES * std::mem::size_of::<TreeNode>();
            unsafe { std::ptr::write_bytes(base, fill, len) };
            lib.set_node_count(0);
        }
        for &(a, b, c, d) in &[
            (1i32, 2i32, 3i32, 4i32),
            (0, 0, 0, 0),
            (-1, -2, -3, -4),
            (7, 0, 0, 0),
        ] {
            assert_same(&p, a, b, c, d);
        }
    }
}

#[test]
fn inreftree_is_idempotent_across_repeated_calls() {
    let p = load();
    fresh(&p);
    // Repeated invocations must not accumulate state differently in the two
    // implementations.
    let seq = [
        (5i32, 6i32, 7i32, 8i32),
        (0, 0, 0, 0),
        (-9, -9, -9, -9),
        (c_int::MAX, 1, 1, 1),
        (2, 0, 0, 0),
        (5, 6, 7, 8),
    ];
    for round in 0..5 {
        for &(a, b, c, d) in &seq {
            assert_same(&p, a, b, c, d);
            let _ = round;
        }
    }
    // Calling with a table left full from a previous scenario.
    for i in 0..MAX_NODES {
        for lib in [p.c, p.rs] {
            let base = lib.node_table_ptr();
            unsafe {
                std::ptr::write(
                    base.add(i),
                    TreeNode {
                        id: i as c_int,
                        value: i as c_int * 11,
                        parent_id: 0,
                        left_child_id: 1,
                        right_child_id: 2,
                        label: [b'z' as _; 32],
                    },
                )
            };
        }
    }
    p.c.set_node_count(MAX_NODES as c_int);
    p.rs.set_node_count(MAX_NODES as c_int);
    assert_same(&p, 3, 4, 5, 6);
}

#[test]
fn inreftree_matches_header_signature_expectations() {
    let p = load();
    fresh(&p);
    // The documented entry point from include/lib.h, spot-checked against
    // hand-derived expectations to confirm the shared behaviour is the C one
    // and not two identical bugs.
    //
    // tree_sum = a + b + c + d; target_id = 2 unless b == 0, then 1.
    // op = "+*-%"[tree_sum % 4] -> ADD / MULTIPLY / SUBTRACT / MODULO.
    let expected = |a: i32, b: i32, c: i32, d: i32| -> i32 {
        let sum = a
            .wrapping_add(b)
            .wrapping_add(c)
            .wrapping_add(d);
        let target = if b == 0 { 1 } else { 2 };
        match sum % 4 {
            0 => sum.wrapping_add(target),
            1 => sum.wrapping_mul(target),
            2 => sum.wrapping_sub(target),
            3 => sum.wrapping_rem(target),
            // Negative residues read out of bounds in C; that byte is not an
            // operator character, so parse_operation falls back to OP_ADD.
            _ => sum.wrapping_add(target),
        }
    };

    for a in -20i32..=20 {
        for b in -3i32..=3 {
            let got = p.c.inreftree(a, b, 0, 0);
            assert_eq!(
                got,
                expected(a, b, 0, 0),
                "hand-derived model disagrees with C for ({a}, {b}, 0, 0)"
            );
            assert_eq!(p.rs.inreftree(a, b, 0, 0), got);
        }
    }
}
