//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Both libraries are driven exclusively
//! through their `.so` exports (`libloading`), never by calling Rust directly.

mod common;

use common::*;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// C1 — process_flags, exhaustive flag subsets 0..=15
// ---------------------------------------------------------------------------
#[test]
fn c1_process_flags_all_subsets() {
    let _g = lock();
    let p = libs();
    for flags in 0..=15i32 {
        let c = unsafe { (p.c.process_flags)(flags) };
        let r = unsafe { (p.rs.process_flags)(flags) };
        assert_eq!(c, r, "process_flags({flags:#06b}) C={c} RUST={r}");
        assert_eq!(c, flags.count_ones() as i32, "sanity: popcount of nibble");
    }
}

// ---------------------------------------------------------------------------
// C2 — process_flags with bits outside the FLAG_* nibble
// ---------------------------------------------------------------------------
#[test]
fn c2_process_flags_extra_bits() {
    let _g = lock();
    let p = libs();
    let fixed: &[c_int] = &[
        0x10, 0x20, 0x40, 0x7F, 0x80, 0xFF, 0x100, 0xFFFF, 0x7FFF_FFFF, 0x1234_5678,
    ];
    for &flags in fixed {
        let c = unsafe { (p.c.process_flags)(flags) };
        let r = unsafe { (p.rs.process_flags)(flags) };
        assert_eq!(c, r, "process_flags({flags:#x})");
    }
    let mut rng = Rng::new(SEED ^ 0xC2);
    for _ in 0..20_000 {
        let flags = rng.spicy_i32();
        let c = unsafe { (p.c.process_flags)(flags) };
        let r = unsafe { (p.rs.process_flags)(flags) };
        assert_eq!(c, r, "process_flags({flags:#x}) random");
    }
}

// ---------------------------------------------------------------------------
// C3 — process_flags with negative inputs (sign bit set)
// ---------------------------------------------------------------------------
#[test]
fn c3_process_flags_negative() {
    let _g = lock();
    let p = libs();
    let fixed: &[c_int] = &[-1, -2, -3, -8, -16, i32::MIN, i32::MIN + 1, -0x7FFF_FFFF];
    for &flags in fixed {
        let c = unsafe { (p.c.process_flags)(flags) };
        let r = unsafe { (p.rs.process_flags)(flags) };
        assert_eq!(c, r, "process_flags({flags})");
    }
    let mut rng = Rng::new(SEED ^ 0xC3);
    for _ in 0..10_000 {
        let flags = -(rng.next_i32().wrapping_abs());
        let c = unsafe { (p.c.process_flags)(flags) };
        let r = unsafe { (p.rs.process_flags)(flags) };
        assert_eq!(c, r, "process_flags({flags}) random negative");
    }
}

// ---------------------------------------------------------------------------
// C4 — calculate_matrix_checksum on the untouched factory matrix
// ---------------------------------------------------------------------------
#[test]
fn c4_checksum_factory_matrix() {
    let _g = lock();
    let p = libs();
    reset_matrix(p);
    assert_eq!(
        p.c.read_matrix(),
        p.rs.read_matrix(),
        "factory `matrix` contents differ between libraries"
    );
    let c = unsafe { (p.c.calculate_matrix_checksum)() };
    let r = unsafe { (p.rs.calculate_matrix_checksum)() };
    assert_eq!(c, r, "checksum C={c} RUST={r}");
    assert_eq!(c, 916, "sanity: documented factory checksum (10 + 160 + 746)");
}

// ---------------------------------------------------------------------------
// C5 — checksum over randomized small matrix contents
// ---------------------------------------------------------------------------
#[test]
fn c5_checksum_random_matrix() {
    let _g = lock();
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xC5);
    for _ in 0..5_000 {
        let mut m = [0i32; 12];
        for v in m.iter_mut() {
            *v = (rng.below(20_001) as i64 - 10_000) as i32;
        }
        p.c.write_matrix(&m);
        p.rs.write_matrix(&m);
        let c = unsafe { (p.c.calculate_matrix_checksum)() };
        let r = unsafe { (p.rs.calculate_matrix_checksum)() };
        assert_eq!(c, r, "checksum for {m:?}");
    }
    reset_matrix(p);
}

// ---------------------------------------------------------------------------
// C6 — checksum with overflowing / extreme matrix contents
// ---------------------------------------------------------------------------
#[test]
fn c6_checksum_overflow_matrix() {
    let _g = lock();
    let p = libs();
    let cases: Vec<[c_int; 12]> = vec![
        [i32::MAX; 12],
        [i32::MIN; 12],
        [0; 12],
        [
            i32::MAX,
            i32::MIN,
            i32::MAX,
            i32::MIN,
            i32::MAX,
            i32::MIN,
            i32::MAX,
            i32::MIN,
            i32::MAX,
            i32::MIN,
            i32::MAX,
            i32::MIN,
        ],
        [
            i32::MAX,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ],
        [i32::MIN, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0x4000_0000; 12],
        [-0x4000_0000; 12],
    ];
    for m in &cases {
        p.c.write_matrix(m);
        p.rs.write_matrix(m);
        let c = unsafe { (p.c.calculate_matrix_checksum)() };
        let r = unsafe { (p.rs.calculate_matrix_checksum)() };
        assert_eq!(c, r, "overflow checksum for {m:?}");
    }
    let mut rng = Rng::new(SEED ^ 0xC6);
    for _ in 0..5_000 {
        let mut m = [0i32; 12];
        for v in m.iter_mut() {
            *v = rng.spicy_i32();
        }
        p.c.write_matrix(&m);
        p.rs.write_matrix(&m);
        let c = unsafe { (p.c.calculate_matrix_checksum)() };
        let r = unsafe { (p.rs.calculate_matrix_checksum)() };
        assert_eq!(c, r, "spicy checksum for {m:?}");
    }
    reset_matrix(p);
}

// ---------------------------------------------------------------------------
// C7 — init_array(1) + free_array, no elements
// ---------------------------------------------------------------------------
#[test]
fn c7_init_free_capacity_one() {
    let _g = lock();
    let p = libs();
    for _ in 0..1_000 {
        let ca = unsafe { (p.c.init_array)(1) };
        let ra = unsafe { (p.rs.init_array)(1) };
        assert_eq!(p.c.view(ca), p.rs.view(ra), "init_array(1) field mismatch");
        let v = p.c.view(ca).expect("C init_array(1) should succeed");
        assert_eq!((v.size, v.capacity, v.data_null), (0, 1, false));
        unsafe { (p.c.free_array)(ca) };
        unsafe { (p.rs.free_array)(ra) };
    }
}

// ---------------------------------------------------------------------------
// C8 — init_array(0), the degenerate capacity
// ---------------------------------------------------------------------------
#[test]
fn c8_init_zero_capacity() {
    let _g = lock();
    let p = libs();
    let ca = unsafe { (p.c.init_array)(0) };
    let ra = unsafe { (p.rs.init_array)(0) };
    assert_eq!(p.c.view(ca), p.rs.view(ra), "init_array(0) field mismatch");
    let v = p.c.view(ca).expect("glibc malloc(0) returns non-NULL");
    assert_eq!((v.size, v.capacity), (0, 0));
    unsafe { (p.c.free_array)(ca) };
    unsafe { (p.rs.free_array)(ra) };
}

// ---------------------------------------------------------------------------
// C9 — fill exactly to capacity (no growth)
// ---------------------------------------------------------------------------
#[test]
fn c9_fill_exact_capacity() {
    let _g = lock();
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xC9);
    for cap in 1..=8usize {
        for _ in 0..200 {
            let vals: Vec<c_int> = (0..cap).map(|_| rng.spicy_i32()).collect();
            let ca = unsafe { (p.c.init_array)(cap) };
            let ra = unsafe { (p.rs.init_array)(cap) };
            assert!(!ca.is_null() && !ra.is_null());
            for &v in &vals {
                let rc = unsafe { (p.c.add_element)(ca, v) };
                let rr = unsafe { (p.rs.add_element)(ra, v) };
                assert_eq!(rc, rr, "add_element({v}) rc mismatch cap={cap}");
                assert_eq!(rc, 1, "should succeed below capacity");
                assert_eq!(p.c.view(ca), p.rs.view(ra), "state after add cap={cap}");
            }
            assert_eq!(p.c.elements(ca, cap), p.rs.elements(ra, cap), "buffer bytes");
            assert_eq!(p.c.elements(ca, cap), vals);
            unsafe { (p.c.free_array)(ca) };
            unsafe { (p.rs.free_array)(ra) };
        }
    }
}

// ---------------------------------------------------------------------------
// C10 — one element past capacity (exactly one growth)
// ---------------------------------------------------------------------------
#[test]
fn c10_one_growth() {
    let _g = lock();
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xCA);
    for cap in 1..=8usize {
        for _ in 0..200 {
            let n = cap + 1;
            let vals: Vec<c_int> = (0..n).map(|_| rng.spicy_i32()).collect();
            let ca = unsafe { (p.c.init_array)(cap) };
            let ra = unsafe { (p.rs.init_array)(cap) };
            for &v in &vals {
                let rc = unsafe { (p.c.add_element)(ca, v) };
                let rr = unsafe { (p.rs.add_element)(ra, v) };
                assert_eq!(rc, rr, "rc mismatch cap={cap} v={v}");
                assert_eq!(p.c.view(ca), p.rs.view(ra), "state cap={cap}");
            }
            let v = p.c.view(ca).unwrap();
            assert_eq!((v.size, v.capacity), (n, cap * 2), "growth cap={cap}");
            assert_eq!(p.c.elements(ca, n), p.rs.elements(ra, n));
            assert_eq!(p.c.elements(ca, n), vals);
            unsafe { (p.c.free_array)(ca) };
            unsafe { (p.rs.free_array)(ra) };
        }
    }
}

// ---------------------------------------------------------------------------
// C11 — repeated doubling from capacity 1 up to 64 elements
// ---------------------------------------------------------------------------
#[test]
fn c11_repeated_doubling() {
    let _g = lock();
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xCB);
    for n in 1..=64usize {
        let vals: Vec<c_int> = (0..n).map(|_| rng.spicy_i32()).collect();
        let ca = unsafe { (p.c.init_array)(1) };
        let ra = unsafe { (p.rs.init_array)(1) };
        for (i, &v) in vals.iter().enumerate() {
            let rc = unsafe { (p.c.add_element)(ca, v) };
            let rr = unsafe { (p.rs.add_element)(ra, v) };
            assert_eq!(rc, rr, "rc mismatch at i={i}");
            assert_eq!(p.c.view(ca), p.rs.view(ra), "state at i={i} n={n}");
            assert_eq!(
                p.c.elements(ca, i + 1),
                p.rs.elements(ra, i + 1),
                "buffer at i={i}"
            );
        }
        unsafe { (p.c.free_array)(ca) };
        unsafe { (p.rs.free_array)(ra) };
    }
}

// ---------------------------------------------------------------------------
// C12 — capacity 2 (the shape matrixsum uses) with 0..=5 elements
// ---------------------------------------------------------------------------
#[test]
fn c12_capacity_two_like_matrixsum() {
    let _g = lock();
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xCC);
    for n in 0..=5usize {
        for _ in 0..500 {
            let vals: Vec<c_int> = (0..n).map(|_| rng.spicy_i32()).collect();
            let ca = unsafe { (p.c.init_array)(2) };
            let ra = unsafe { (p.rs.init_array)(2) };
            for &v in &vals {
                let rc = unsafe { (p.c.add_element)(ca, v) };
                let rr = unsafe { (p.rs.add_element)(ra, v) };
                assert_eq!(rc, rr);
                assert_eq!(p.c.view(ca), p.rs.view(ra));
            }
            assert_eq!(p.c.elements(ca, n), p.rs.elements(ra, n));
            // Sum the buffer the way matrixsum does (wrapping).
            let sum_c = p.c.elements(ca, n).iter().fold(0i32, |a, &b| a.wrapping_add(b));
            let sum_r = p.rs.elements(ra, n).iter().fold(0i32, |a, &b| a.wrapping_add(b));
            assert_eq!(sum_c, sum_r);
            unsafe { (p.c.free_array)(ca) };
            unsafe { (p.rs.free_array)(ra) };
        }
    }
}

// ---------------------------------------------------------------------------
// C13 — expand_array called directly, repeatedly, on an empty array
// ---------------------------------------------------------------------------
#[test]
fn c13_expand_array_direct() {
    let _g = lock();
    let p = libs();
    for cap in 1..=8usize {
        let ca = unsafe { (p.c.init_array)(cap) };
        let ra = unsafe { (p.rs.init_array)(cap) };
        for round in 0..3 {
            let rc = unsafe { (p.c.expand_array)(ca) };
            let rr = unsafe { (p.rs.expand_array)(ra) };
            assert_eq!(rc, rr, "expand rc cap={cap} round={round}");
            assert_eq!(rc, 1);
            assert_eq!(p.c.view(ca), p.rs.view(ra), "cap={cap} round={round}");
            let v = p.c.view(ca).unwrap();
            assert_eq!(v.capacity, cap << (round + 1));
            assert_eq!(v.size, 0, "expand must not touch size");
        }
        unsafe { (p.c.free_array)(ca) };
        unsafe { (p.rs.free_array)(ra) };
    }
}

// ---------------------------------------------------------------------------
// C14 — expand_array after a partial fill: contents must survive realloc
// ---------------------------------------------------------------------------
#[test]
fn c14_expand_preserves_contents() {
    let _g = lock();
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xCE);
    for cap in 1..=8usize {
        for k in 0..=cap {
            let vals: Vec<c_int> = (0..k).map(|_| rng.spicy_i32()).collect();
            let ca = unsafe { (p.c.init_array)(cap) };
            let ra = unsafe { (p.rs.init_array)(cap) };
            for &v in &vals {
                assert_eq!(
                    unsafe { (p.c.add_element)(ca, v) },
                    unsafe { (p.rs.add_element)(ra, v) }
                );
            }
            assert_eq!(
                unsafe { (p.c.expand_array)(ca) },
                unsafe { (p.rs.expand_array)(ra) }
            );
            assert_eq!(p.c.view(ca), p.rs.view(ra), "after expand cap={cap} k={k}");
            assert_eq!(p.c.elements(ca, k), p.rs.elements(ra, k), "preserved");
            assert_eq!(p.c.elements(ca, k), vals);
            // Keep adding into the expanded buffer.
            for _ in 0..cap {
                let v = rng.spicy_i32();
                assert_eq!(
                    unsafe { (p.c.add_element)(ca, v) },
                    unsafe { (p.rs.add_element)(ra, v) }
                );
                assert_eq!(p.c.view(ca), p.rs.view(ra));
            }
            let n = p.c.view(ca).unwrap().size;
            assert_eq!(p.c.elements(ca, n), p.rs.elements(ra, n));
            unsafe { (p.c.free_array)(ca) };
            unsafe { (p.rs.free_array)(ra) };
        }
    }
}

// ---------------------------------------------------------------------------
// C15 — randomized low-level pipeline scripts, whole trace compared
// ---------------------------------------------------------------------------
#[test]
fn c15_randomized_pipeline() {
    let _g = lock();
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xCF);
    for script in 0..300 {
        let cap = (rng.below(8) + 1) as usize;
        let ca = unsafe { (p.c.init_array)(cap) };
        let ra = unsafe { (p.rs.init_array)(cap) };
        assert_eq!(p.c.view(ca), p.rs.view(ra), "init cap={cap}");
        let steps = rng.below(40) + 1;
        for step in 0..steps {
            match rng.below(4) {
                0 => {
                    let rc = unsafe { (p.c.expand_array)(ca) };
                    let rr = unsafe { (p.rs.expand_array)(ra) };
                    assert_eq!(rc, rr, "script={script} step={step} expand");
                }
                _ => {
                    let v = rng.spicy_i32();
                    let rc = unsafe { (p.c.add_element)(ca, v) };
                    let rr = unsafe { (p.rs.add_element)(ra, v) };
                    assert_eq!(rc, rr, "script={script} step={step} add({v})");
                }
            }
            assert_eq!(
                p.c.view(ca),
                p.rs.view(ra),
                "script={script} step={step} state"
            );
            let n = p.c.view(ca).unwrap().size;
            assert_eq!(
                p.c.elements(ca, n),
                p.rs.elements(ra, n),
                "script={script} step={step} buffer"
            );
        }
        unsafe { (p.c.free_array)(ca) };
        unsafe { (p.rs.free_array)(ra) };
    }
}

// ---------------------------------------------------------------------------
// C16 — matrixsum: all 16 zero/non-zero param patterns, randomized magnitudes
// ---------------------------------------------------------------------------
#[test]
fn c16_matrixsum_flag_patterns() {
    let _g = lock();
    let p = libs();
    reset_matrix(p);
    let mut rng = Rng::new(SEED ^ 0x10);
    for pattern in 0..16u32 {
        for _ in 0..200 {
            let mut ps = [0i32; 4];
            for (i, slot) in ps.iter_mut().enumerate() {
                if pattern & (1 << i) != 0 {
                    // non-zero
                    let mut v = rng.spicy_i32();
                    if v == 0 {
                        v = 1;
                    }
                    *slot = v;
                } else {
                    *slot = 0;
                }
            }
            let c = unsafe { (p.c.matrixsum)(ps[0], ps[1], ps[2], ps[3]) };
            let r = unsafe { (p.rs.matrixsum)(ps[0], ps[1], ps[2], ps[3]) };
            assert_eq!(c, r, "matrixsum{ps:?} pattern={pattern:#06b} C={c} RUST={r}");
        }
    }
}

// ---------------------------------------------------------------------------
// C17 — matrixsum: fully random params
// ---------------------------------------------------------------------------
#[test]
fn c17_matrixsum_random() {
    let _g = lock();
    let p = libs();
    reset_matrix(p);
    let mut rng = Rng::new(SEED ^ 0x11);
    for _ in 0..50_000 {
        let a = rng.spicy_i32();
        let b = rng.spicy_i32();
        let c2 = rng.spicy_i32();
        let d = rng.spicy_i32();
        let c = unsafe { (p.c.matrixsum)(a, b, c2, d) };
        let r = unsafe { (p.rs.matrixsum)(a, b, c2, d) };
        assert_eq!(c, r, "matrixsum({a},{b},{c2},{d}) C={c} RUST={r}");
    }
}

// ---------------------------------------------------------------------------
// C18 — matrixsum: full 8^4 cross product of boundary values
// ---------------------------------------------------------------------------
#[test]
fn c18_matrixsum_boundary_cross_product() {
    let _g = lock();
    let p = libs();
    reset_matrix(p);
    let vals: [c_int; 8] = [0, 1, -1, i32::MAX, i32::MIN, 0xFF, 0x10, 0xFFF];
    for &a in &vals {
        for &b in &vals {
            for &c2 in &vals {
                for &d in &vals {
                    let c = unsafe { (p.c.matrixsum)(a, b, c2, d) };
                    let r = unsafe { (p.rs.matrixsum)(a, b, c2, d) };
                    assert_eq!(c, r, "matrixsum({a},{b},{c2},{d})");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C19 — matrixsum x mutated matrix (the & 0xFFF masking interaction)
// ---------------------------------------------------------------------------
#[test]
fn c19_matrixsum_with_mutated_matrix() {
    let _g = lock();
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0x13);

    let mut matrices: Vec<[c_int; 12]> = vec![
        FACTORY_MATRIX,
        [0; 12],
        [1000; 12],                    // checksum 12000 > 0xFFF
        [-1000; 12],                   // negative checksum
        [i32::MAX; 12],
        [i32::MIN; 12],
        [0xFFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0x1000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [-1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0x7FF, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ];
    for _ in 0..40 {
        let mut m = [0i32; 12];
        for v in m.iter_mut() {
            *v = rng.spicy_i32();
        }
        matrices.push(m);
    }

    for m in &matrices {
        p.c.write_matrix(m);
        p.rs.write_matrix(m);
        assert_eq!(
            unsafe { (p.c.calculate_matrix_checksum)() },
            unsafe { (p.rs.calculate_matrix_checksum)() },
            "checksum for {m:?}"
        );
        for _ in 0..400 {
            let a = rng.spicy_i32();
            let b = rng.spicy_i32();
            let c2 = rng.spicy_i32();
            let d = rng.spicy_i32();
            let c = unsafe { (p.c.matrixsum)(a, b, c2, d) };
            let r = unsafe { (p.rs.matrixsum)(a, b, c2, d) };
            assert_eq!(c, r, "matrixsum({a},{b},{c2},{d}) with matrix {m:?}");
        }
    }
    reset_matrix(p);
}

// ---------------------------------------------------------------------------
// C20 — randomized whole-library session across all 7 entry points
// ---------------------------------------------------------------------------
#[test]
fn c20_whole_library_session() {
    let _g = lock();
    let p = libs();
    reset_matrix(p);
    let mut rng = Rng::new(SEED ^ 0x14);
    let mut live: Vec<(*mut DynamicArray, *mut DynamicArray)> = Vec::new();

    for step in 0..4_000 {
        match rng.below(7) {
            0 => {
                let cap = (rng.below(6) + 1) as usize;
                let ca = unsafe { (p.c.init_array)(cap) };
                let ra = unsafe { (p.rs.init_array)(cap) };
                assert_eq!(p.c.view(ca), p.rs.view(ra), "step={step} init");
                live.push((ca, ra));
            }
            1 if !live.is_empty() => {
                let i = rng.below(live.len() as u64) as usize;
                let (ca, ra) = live[i];
                let v = rng.spicy_i32();
                assert_eq!(
                    unsafe { (p.c.add_element)(ca, v) },
                    unsafe { (p.rs.add_element)(ra, v) },
                    "step={step} add"
                );
                assert_eq!(p.c.view(ca), p.rs.view(ra), "step={step} add state");
                let n = p.c.view(ca).unwrap().size;
                assert_eq!(p.c.elements(ca, n), p.rs.elements(ra, n));
            }
            2 if !live.is_empty() => {
                let i = rng.below(live.len() as u64) as usize;
                let (ca, ra) = live[i];
                assert_eq!(
                    unsafe { (p.c.expand_array)(ca) },
                    unsafe { (p.rs.expand_array)(ra) },
                    "step={step} expand"
                );
                assert_eq!(p.c.view(ca), p.rs.view(ra), "step={step} expand state");
            }
            3 if !live.is_empty() => {
                let i = rng.below(live.len() as u64) as usize;
                let (ca, ra) = live.remove(i);
                unsafe { (p.c.free_array)(ca) };
                unsafe { (p.rs.free_array)(ra) };
            }
            4 => {
                let f = rng.spicy_i32();
                assert_eq!(
                    unsafe { (p.c.process_flags)(f) },
                    unsafe { (p.rs.process_flags)(f) },
                    "step={step} process_flags({f})"
                );
            }
            5 => {
                let mut m = [0i32; 12];
                for v in m.iter_mut() {
                    *v = rng.spicy_i32();
                }
                p.c.write_matrix(&m);
                p.rs.write_matrix(&m);
                assert_eq!(p.c.read_matrix(), p.rs.read_matrix(), "step={step} matrix");
                assert_eq!(
                    unsafe { (p.c.calculate_matrix_checksum)() },
                    unsafe { (p.rs.calculate_matrix_checksum)() },
                    "step={step} checksum"
                );
            }
            _ => {
                let a = rng.spicy_i32();
                let b = rng.spicy_i32();
                let c2 = rng.spicy_i32();
                let d = rng.spicy_i32();
                assert_eq!(
                    unsafe { (p.c.matrixsum)(a, b, c2, d) },
                    unsafe { (p.rs.matrixsum)(a, b, c2, d) },
                    "step={step} matrixsum({a},{b},{c2},{d})"
                );
            }
        }
    }
    for (ca, ra) in live {
        unsafe { (p.c.free_array)(ca) };
        unsafe { (p.rs.free_array)(ra) };
    }
    reset_matrix(p);
}
