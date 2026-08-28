//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both implementations are loaded from their `.so` and driven through `dlsym`.

mod common;

use common::{load, DEFAULT_MATRIX, SEED};
use std::ffi::c_int;

// ===========================================================================
// C1 / C2 / C3 — process_flags
// ===========================================================================

#[test]
fn c1_process_flags_exhaustive_0_255() {
    let p = load();
    for f in 0..=255i32 {
        assert_eq!(
            p.c.process_flags(f),
            p.rs.process_flags(f),
            "process_flags({f}) diverged"
        );
    }
}

#[test]
fn c2_process_flags_random_full_range() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0xC2);
    for _ in 0..4096 {
        let f = rng.spicy_i32();
        assert_eq!(
            p.c.process_flags(f),
            p.rs.process_flags(f),
            "process_flags({f}) diverged"
        );
    }
    // also plain uniform draws
    for _ in 0..4096 {
        let f = rng.next_i32();
        assert_eq!(p.c.process_flags(f), p.rs.process_flags(f), "process_flags({f})");
    }
}

#[test]
fn c3_process_flags_boundaries() {
    let p = load();
    let cases: [c_int; 20] = [
        0,
        1,
        2,
        3,
        4,
        7,
        8,
        15,
        16,
        -1,
        c_int::MIN,
        c_int::MAX,
        c_int::MIN + 1,
        c_int::MAX - 1,
        0xF0,
        !0xF,
        0x0000_0010,
        0x7FFF_FFF0,
        -16,
        -8,
    ];
    for f in cases {
        assert_eq!(
            p.c.process_flags(f),
            p.rs.process_flags(f),
            "process_flags({f:#x}) diverged"
        );
    }
}

// ===========================================================================
// C4 / C5 / C6 / C20 — calculate_matrix_checksum and the exported `matrix`
// ===========================================================================

#[test]
fn c4_matrix_checksum_pristine() {
    let p = load();
    let c = p.c.calculate_matrix_checksum();
    let r = p.rs.calculate_matrix_checksum();
    assert_eq!(c, r, "pristine checksum diverged");
    // sanity: 0x01+..+0xD4
    assert_eq!(c, 916, "unexpected pristine checksum from C");
}

#[test]
fn c5_matrix_checksum_random_matrices() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0xC5);
    for i in 0..512 {
        let mut m = [0i32; 12];
        for slot in m.iter_mut() {
            *slot = rng.spicy_i32();
        }
        p.c.matrix_write(&m);
        p.rs.matrix_write(&m);
        assert_eq!(p.c.matrix_read(), m, "C matrix write-back failed");
        assert_eq!(p.rs.matrix_read(), m, "Rust matrix write-back failed");
        assert_eq!(
            p.c.calculate_matrix_checksum(),
            p.rs.calculate_matrix_checksum(),
            "checksum diverged for matrix #{i}: {m:?}"
        );
    }
    p.c.matrix_reset();
    p.rs.matrix_reset();
}

#[test]
fn c6_matrix_checksum_overflow_extremes() {
    let p = load();
    let patterns: Vec<[i32; 12]> = vec![
        [i32::MAX; 12],
        [i32::MIN; 12],
        [
            i32::MAX,
            i32::MAX,
            i32::MIN,
            i32::MIN,
            i32::MAX,
            i32::MIN,
            0,
            1,
            -1,
            i32::MAX,
            i32::MIN,
            0,
        ],
        [0; 12],
        [1; 12],
        [-1; 12],
        [i32::MAX / 2; 12],
        [i32::MIN / 2; 12],
        [
            0x4000_0000,
            0x4000_0000,
            0x4000_0000,
            0x4000_0000,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ],
        [
            -0x4000_0000,
            -0x4000_0000,
            -0x4000_0000,
            -0x4000_0000,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ],
    ];
    for (i, m) in patterns.iter().enumerate() {
        p.c.matrix_write(m);
        p.rs.matrix_write(m);
        assert_eq!(
            p.c.calculate_matrix_checksum(),
            p.rs.calculate_matrix_checksum(),
            "checksum diverged for overflow pattern #{i}"
        );
    }
    p.c.matrix_reset();
    p.rs.matrix_reset();
}

#[test]
fn c20_matrix_symbol_identity() {
    let p = load();
    // Same default contents, byte for byte.
    assert_eq!(
        p.c.matrix_bytes(),
        p.rs.matrix_bytes(),
        "default `matrix` bytes differ"
    );
    assert_eq!(p.c.matrix_read(), DEFAULT_MATRIX);
    assert_eq!(p.rs.matrix_read(), DEFAULT_MATRIX);
    // Writable in both, and observable byte-for-byte.
    let m: [i32; 12] = [
        -1,
        0,
        1,
        i32::MIN,
        i32::MAX,
        2,
        3,
        4,
        5,
        6,
        7,
        0x1234_5678,
    ];
    p.c.matrix_write(&m);
    p.rs.matrix_write(&m);
    assert_eq!(p.c.matrix_bytes(), p.rs.matrix_bytes(), "mutated bytes differ");
    p.c.matrix_reset();
    p.rs.matrix_reset();
    assert_eq!(p.c.matrix_bytes(), p.rs.matrix_bytes());
}

// ===========================================================================
// C7 / C8 — init_array / free_array round trip
// ===========================================================================

const CAPS: [usize; 13] = [0, 1, 2, 3, 4, 7, 8, 16, 63, 64, 1000, 65536, 1 << 20];

#[test]
fn c7_init_array_layout() {
    let p = load();
    for cap in CAPS {
        unsafe {
            let a = p.c.init_array(cap);
            let b = p.rs.init_array(cap);
            assert_eq!(a.is_null(), b.is_null(), "init_array({cap}) NULL-ness differs");
            if a.is_null() {
                continue;
            }
            let ha = p.c.header(a);
            let hb = p.rs.header(b);
            assert_eq!(
                ha.data.is_null(),
                hb.data.is_null(),
                "init_array({cap}) data NULL-ness differs"
            );
            assert_eq!(ha.size, hb.size, "init_array({cap}) size differs");
            assert_eq!(ha.capacity, hb.capacity, "init_array({cap}) capacity differs");
            assert_eq!(ha.size, 0, "init_array({cap}) size should be 0");
            assert_eq!(ha.capacity, cap, "init_array({cap}) capacity should echo");
            p.c.free_array(a);
            p.rs.free_array(b);
        }
    }
}

#[test]
fn c8_init_free_lifecycle() {
    let p = load();
    for cap in CAPS {
        for _ in 0..8 {
            unsafe {
                let a = p.c.init_array(cap);
                let b = p.rs.init_array(cap);
                assert_eq!(a.is_null(), b.is_null());
                p.c.free_array(a);
                p.rs.free_array(b);
            }
        }
    }
}

// ===========================================================================
// C9 / C10 / C11 / C13 — add_element growth behaviour
// ===========================================================================

/// Push `values` into a fresh array of capacity `cap` in both libraries and
/// assert every observable (return codes, size, capacity, buffer) matches.
fn push_and_compare(p: &common::Pair, cap: usize, values: &[c_int], ctx: &str) {
    unsafe {
        let a = p.c.init_array(cap);
        let b = p.rs.init_array(cap);
        assert_eq!(a.is_null(), b.is_null(), "{ctx}: init NULL-ness");
        if a.is_null() {
            return;
        }
        for (i, &v) in values.iter().enumerate() {
            let ra = p.c.add_element(a, v);
            let rb = p.rs.add_element(b, v);
            assert_eq!(ra, rb, "{ctx}: add_element #{i} ({v}) return code");
            let ha = p.c.header(a);
            let hb = p.rs.header(b);
            assert_eq!(ha.size, hb.size, "{ctx}: size after #{i}");
            assert_eq!(ha.capacity, hb.capacity, "{ctx}: capacity after #{i}");
        }
        let ha = p.c.header(a);
        let hb = p.rs.header(b);
        // Only the initialised prefix (0..size) is meaningful.
        let ea = p.c.elements(a, ha.size);
        let eb = p.rs.elements(b, hb.size);
        assert_eq!(ea, eb, "{ctx}: buffer contents");
        p.c.free_array(a);
        p.rs.free_array(b);
    }
}

#[test]
fn c9_add_element_no_growth() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0xC9);
    for cap in [1usize, 2, 3, 4, 8, 16, 64] {
        for _ in 0..32 {
            let k = rng.below(cap as u64) as usize; // k < cap
            let vals: Vec<c_int> = (0..k).map(|_| rng.spicy_i32()).collect();
            push_and_compare(&p, cap, &vals, &format!("cap={cap} k={k}"));
        }
    }
}

#[test]
fn c10_add_element_exactly_one_growth() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0xC10);
    for cap in [1usize, 2, 3, 4, 5, 8, 16, 63, 64] {
        for _ in 0..8 {
            let vals: Vec<c_int> = (0..cap + 1).map(|_| rng.spicy_i32()).collect();
            unsafe {
                let a = p.c.init_array(cap);
                let b = p.rs.init_array(cap);
                for (i, &v) in vals.iter().enumerate() {
                    assert_eq!(
                        p.c.add_element(a, v),
                        p.rs.add_element(b, v),
                        "cap={cap}: add #{i}"
                    );
                }
                let ha = p.c.header(a);
                let hb = p.rs.header(b);
                assert_eq!(ha.size, hb.size);
                assert_eq!(ha.capacity, hb.capacity);
                assert_eq!(ha.capacity, cap * 2, "cap={cap}: expected one doubling");
                assert_eq!(ha.size, cap + 1);
                assert_eq!(p.c.elements(a, ha.size), p.rs.elements(b, hb.size));
                assert_eq!(p.c.elements(a, ha.size), vals);
                p.c.free_array(a);
                p.rs.free_array(b);
            }
        }
    }
}

#[test]
fn c11_add_element_many_growths() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0xC11);
    for cap in [1usize, 2, 3] {
        for n in [0usize, 1, 2, 3, 5, 9, 17, 33, 65, 129, 257] {
            let vals: Vec<c_int> = (0..n).map(|_| rng.spicy_i32()).collect();
            push_and_compare(&p, cap, &vals, &format!("many cap={cap} n={n}"));
        }
    }
}

#[test]
fn c13_add_element_extreme_values() {
    let p = load();
    let extremes: [c_int; 7] = [0, 1, -1, c_int::MIN, c_int::MAX, c_int::MIN + 1, c_int::MAX - 1];
    // every ordered pair/triple of extremes into a capacity-2 array (forces growth)
    for &x in &extremes {
        for &y in &extremes {
            for &z in &extremes {
                push_and_compare(&p, 2, &[x, y, z], &format!("extremes {x},{y},{z}"));
            }
        }
    }
}

// ===========================================================================
// C12 — expand_array driven directly
// ===========================================================================

#[test]
fn c12_expand_array_direct() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0xC12);
    for cap in [1usize, 2, 3, 4, 7, 16] {
        for expands in 0..=8usize {
            unsafe {
                let a = p.c.init_array(cap);
                let b = p.rs.init_array(cap);
                assert_eq!(a.is_null(), b.is_null());
                for e in 0..expands {
                    let ra = p.c.expand_array(a);
                    let rb = p.rs.expand_array(b);
                    assert_eq!(ra, rb, "cap={cap} expand #{e} return code");
                    let ha = p.c.header(a);
                    let hb = p.rs.header(b);
                    assert_eq!(ha.capacity, hb.capacity, "cap={cap} expand #{e} capacity");
                    assert_eq!(ha.size, hb.size, "cap={cap} expand #{e} size");
                    assert_eq!(
                        ha.capacity,
                        cap << (e + 1),
                        "cap={cap} expand #{e}: expected doubling"
                    );
                }
                // then push a few elements and re-compare
                let n = 1 + rng.below(5) as usize;
                for i in 0..n {
                    let v = rng.spicy_i32();
                    assert_eq!(p.c.add_element(a, v), p.rs.add_element(b, v), "post-expand add #{i}");
                }
                let ha = p.c.header(a);
                let hb = p.rs.header(b);
                assert_eq!(ha.size, hb.size);
                assert_eq!(ha.capacity, hb.capacity);
                assert_eq!(p.c.elements(a, ha.size), p.rs.elements(b, hb.size));
                p.c.free_array(a);
                p.rs.free_array(b);
            }
        }
    }
}

// ===========================================================================
// C14 / C15 / C16 / C17 / C18 — matrixsum
// ===========================================================================

#[test]
fn c14_matrixsum_all_permission_combos() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0xC14);
    for mask in 0..16u32 {
        for _ in 0..64 {
            let mut v = [0i32; 4];
            for (i, slot) in v.iter_mut().enumerate() {
                *slot = if mask & (1 << i) != 0 {
                    // non-zero
                    loop {
                        let x = rng.spicy_i32();
                        if x != 0 {
                            break x;
                        }
                    }
                } else {
                    0
                };
            }
            let c = p.c.matrixsum(v[0], v[1], v[2], v[3]);
            let r = p.rs.matrixsum(v[0], v[1], v[2], v[3]);
            assert_eq!(c, r, "matrixsum{v:?} (mask={mask:#04b}) diverged");
        }
    }
}

#[test]
fn c15_matrixsum_random() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0xC15);
    for _ in 0..8192 {
        let (a, b, c, d) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
        assert_eq!(
            p.c.matrixsum(a, b, c, d),
            p.rs.matrixsum(a, b, c, d),
            "matrixsum({a},{b},{c},{d}) diverged"
        );
    }
    for _ in 0..8192 {
        let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_eq!(
            p.c.matrixsum(a, b, c, d),
            p.rs.matrixsum(a, b, c, d),
            "matrixsum({a},{b},{c},{d}) diverged"
        );
    }
}

#[test]
fn c16_matrixsum_extreme_cross_product() {
    let p = load();
    let ext: [c_int; 8] = [
        0,
        1,
        -1,
        c_int::MIN,
        c_int::MAX,
        c_int::MIN + 1,
        c_int::MAX - 1,
        0x0800_0000,
    ];
    for &a in &ext {
        for &b in &ext {
            for &c in &ext {
                for &d in &ext {
                    assert_eq!(
                        p.c.matrixsum(a, b, c, d),
                        p.rs.matrixsum(a, b, c, d),
                        "matrixsum({a},{b},{c},{d}) diverged"
                    );
                }
            }
        }
    }
}

#[test]
fn c17_matrixsum_with_mutated_matrix() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0xC17);
    for i in 0..256 {
        let mut m = [0i32; 12];
        for slot in m.iter_mut() {
            *slot = rng.spicy_i32();
        }
        p.c.matrix_write(&m);
        p.rs.matrix_write(&m);
        for _ in 0..8 {
            let (a, b, c, d) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
            assert_eq!(
                p.c.matrixsum(a, b, c, d),
                p.rs.matrixsum(a, b, c, d),
                "matrixsum({a},{b},{c},{d}) with matrix #{i} {m:?} diverged"
            );
        }
    }
    p.c.matrix_reset();
    p.rs.matrix_reset();
}

#[test]
fn c18_matrixsum_repeatable_no_residual_state() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0xC18);
    let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
    let c0 = p.c.matrixsum(a, b, c, d);
    let r0 = p.rs.matrixsum(a, b, c, d);
    assert_eq!(c0, r0);
    for i in 0..1000 {
        assert_eq!(p.c.matrixsum(a, b, c, d), c0, "C drifted at iteration {i}");
        assert_eq!(p.rs.matrixsum(a, b, c, d), r0, "Rust drifted at iteration {i}");
    }
    // interleave other work, then re-check
    for _ in 0..100 {
        unsafe {
            let x = p.c.init_array(2);
            let y = p.rs.init_array(2);
            p.c.add_element(x, 7);
            p.rs.add_element(y, 7);
            p.c.free_array(x);
            p.rs.free_array(y);
        }
        assert_eq!(p.c.matrixsum(a, b, c, d), c0);
        assert_eq!(p.rs.matrixsum(a, b, c, d), r0);
    }
}

// ===========================================================================
// C19 — full low-level pipeline, randomized scripts
// ===========================================================================

#[test]
fn c19_random_pipeline_scripts() {
    let p = load();
    let mut rng = common::Rng::new(SEED ^ 0xC19);
    for script in 0..512 {
        let cap = 1 + rng.below(8) as usize;
        let steps = rng.below(40) as usize;
        unsafe {
            let a = p.c.init_array(cap);
            let b = p.rs.init_array(cap);
            assert_eq!(a.is_null(), b.is_null(), "script {script}: init");
            if a.is_null() {
                continue;
            }
            let mut expands = 0;
            for step in 0..steps {
                // 25% expand_array, 75% add_element (bounded expands so the
                // doubling chain stays allocatable)
                if rng.below(4) == 0 && expands < 6 {
                    expands += 1;
                    let ra = p.c.expand_array(a);
                    let rb = p.rs.expand_array(b);
                    assert_eq!(ra, rb, "script {script} step {step}: expand rc");
                } else {
                    let v = rng.spicy_i32();
                    let ra = p.c.add_element(a, v);
                    let rb = p.rs.add_element(b, v);
                    assert_eq!(ra, rb, "script {script} step {step}: add rc");
                }
                let ha = p.c.header(a);
                let hb = p.rs.header(b);
                assert_eq!(ha.size, hb.size, "script {script} step {step}: size");
                assert_eq!(ha.capacity, hb.capacity, "script {script} step {step}: capacity");
            }
            let ha = p.c.header(a);
            let hb = p.rs.header(b);
            assert_eq!(
                p.c.elements(a, ha.size),
                p.rs.elements(b, hb.size),
                "script {script}: final buffer"
            );
            p.c.free_array(a);
            p.rs.free_array(b);
        }
    }
}
