// Phase B — valid-path differential tests, one test per row of CONFIGS.md.
// Every call goes through `libloading` into either the C `.so` or the Rust
// `.so`; the Rust crate is never linked or called directly.

mod common;

use common::*;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// Row 1 — process_flags: all 16 exhaustive flag-bit combinations
// ---------------------------------------------------------------------------
#[test]
fn row01_process_flags_all_16_combinations() {
    let p = libs();
    for flags in 0..16i32 {
        unsafe {
            assert_eq!(
                (p.c.process_flags)(flags),
                (p.rust.process_flags)(flags),
                "process_flags({flags:#06b})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 2 — process_flags: flag bits combined with unmodelled high bits
// ---------------------------------------------------------------------------
#[test]
fn row02_process_flags_with_unmodelled_high_bits() {
    let p = libs();
    let extras: [i32; 8] =
        [0x10, 0x20, 0xF0, 0xFF00, 0x7FFF_FF00, -0x100, 0x5555_5550, i32::MIN];
    for flags in 0..16i32 {
        for e in extras {
            let v = flags | e;
            unsafe {
                assert_eq!(
                    (p.c.process_flags)(v),
                    (p.rust.process_flags)(v),
                    "process_flags({v:#x}) (flags={flags:#x} extra={e:#x})"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — process_flags: negatives, INT_MIN/MAX and 4096 random i32
// ---------------------------------------------------------------------------
#[test]
fn row03_process_flags_randomized_full_range() {
    let p = libs();
    let mut rng = Rng::new(0xF1A65_EED);
    let mut cases: Vec<i32> = vec![-1, i32::MIN, i32::MAX, 0, i32::MIN + 1, i32::MAX - 1];
    for _ in 0..4096 {
        cases.push(rng.spicy_i32());
    }
    for v in cases {
        unsafe {
            assert_eq!(
                (p.c.process_flags)(v),
                (p.rust.process_flags)(v),
                "process_flags({v})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — calculate_matrix_checksum on the default matrix, repeatedly
// ---------------------------------------------------------------------------
#[test]
fn row04_matrix_checksum_default() {
    let p = libs();
    // `matrix` is shared process-wide; serialise against other tests.
    let _guard = matrix_lock();
    unsafe {
        // The default global data must also be byte-identical.
        assert_eq!(p.c.get_matrix(), p.rust.get_matrix(), "default `matrix` contents");
        for _ in 0..8 {
            assert_eq!(
                (p.c.calculate_matrix_checksum)(),
                (p.rust.calculate_matrix_checksum)(),
                "calculate_matrix_checksum() on default matrix"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 — calculate_matrix_checksum with the exported `matrix` overwritten
// ---------------------------------------------------------------------------
#[test]
fn row05_matrix_checksum_mutated_global() {
    let p = libs();
    // `matrix` is shared process-wide; serialise against other tests.
    let _guard = matrix_lock();
    let mut rng = Rng::new(0x5EED_0005);
    unsafe {
        let orig_c = p.c.get_matrix();
        let orig_rust = p.rust.get_matrix();
        for iter in 0..512 {
            let mut m = [[0i32; 4]; 3];
            for row in m.iter_mut() {
                for v in row.iter_mut() {
                    *v = rng.spicy_i32();
                }
            }
            p.c.set_matrix(m);
            p.rust.set_matrix(m);
            assert_eq!(p.c.get_matrix(), p.rust.get_matrix(), "iter {iter}: matrix readback");
            assert_eq!(
                (p.c.calculate_matrix_checksum)(),
                (p.rust.calculate_matrix_checksum)(),
                "iter {iter}: checksum of {m:?}"
            );
        }
        p.c.set_matrix(orig_c);
        p.rust.set_matrix(orig_rust);
        assert_eq!(
            (p.c.calculate_matrix_checksum)(),
            (p.rust.calculate_matrix_checksum)(),
            "checksum after restoring the default matrix"
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 6-10 — init_array across capacity shapes, plus free_array round-trip
// ---------------------------------------------------------------------------
fn check_init(p: &Pair, cap: SizeT) {
    unsafe {
        let a = (p.c.init_array)(cap);
        let b = (p.rust.init_array)(cap);
        assert_eq!(
            p.c.snapshot(a),
            p.rust.snapshot(b),
            "init_array({cap}) produced differing arrays"
        );
        // Round-trip through free_array (row 10). `size` is 0 so no element
        // memory is inspected after the free.
        (p.c.free_array)(a);
        (p.rust.free_array)(b);
    }
}

#[test]
fn row06_init_array_zero_capacity() {
    let p = libs();
    check_init(&p, 0);
}

#[test]
fn row07_init_array_capacity_one_and_two() {
    let p = libs();
    check_init(&p, 1);
    check_init(&p, 2);
}

#[test]
fn row08_init_array_random_small_capacity() {
    let p = libs();
    let mut rng = Rng::new(0x1111_2222);
    for _ in 0..256 {
        let cap = 1 + rng.below(1024) as SizeT;
        check_init(&p, cap);
    }
}

#[test]
fn row09_init_array_large_satisfiable_capacity() {
    let p = libs();
    for cap in [1usize << 16, 1 << 20] {
        check_init(&p, cap);
    }
}

#[test]
fn row10_init_free_roundtrip_all_shapes() {
    let p = libs();
    for cap in [0usize, 1, 2, 3, 7, 8, 64, 1 << 16, 1 << 20] {
        check_init(&p, cap);
    }
}

// ---------------------------------------------------------------------------
// Row 11 — a single expand_array doubling from a fresh array
// ---------------------------------------------------------------------------
#[test]
fn row11_expand_array_single_doubling() {
    let p = libs();
    unsafe {
        for k in [1usize, 2, 3, 7, 64] {
            let a = (p.c.init_array)(k);
            let b = (p.rust.init_array)(k);
            assert!(!a.is_null() && !b.is_null(), "init_array({k}) failed");
            let ra = (p.c.expand_array)(a);
            let rb = (p.rust.expand_array)(b);
            assert_eq!(ra, rb, "expand_array return for capacity {k}");
            assert_eq!(p.c.snapshot(a), p.rust.snapshot(b), "state after expand from {k}");
            assert_eq!((*a).capacity, 2 * k, "C capacity doubling sanity for {k}");
            (p.c.free_array)(a);
            (p.rust.free_array)(b);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — repeated expand_array, contents preserved across every realloc
// ---------------------------------------------------------------------------
#[test]
fn row12_expand_array_repeated_doublings() {
    let p = libs();
    let mut rng = Rng::new(0x0C0C_0C0C);
    unsafe {
        for k in [1usize, 2, 3, 5] {
            let a = (p.c.init_array)(k);
            let b = (p.rust.init_array)(k);
            assert!(!a.is_null() && !b.is_null());
            // Fill it up first so the reallocs have payload to move.
            for _ in 0..k {
                let v = rng.spicy_i32();
                assert_eq!((p.c.add_element)(a, v), (p.rust.add_element)(b, v));
            }
            for n in 1..=6 {
                let ra = (p.c.expand_array)(a);
                let rb = (p.rust.expand_array)(b);
                assert_eq!(ra, rb, "expand #{n} return (k={k})");
                assert_eq!(
                    p.c.snapshot(a),
                    p.rust.snapshot(b),
                    "state after expand #{n} (k={k})"
                );
                assert_eq!((*a).capacity, k << n, "capacity after {n} doublings of {k}");
            }
            (p.c.free_array)(a);
            (p.rust.free_array)(b);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 — expand with n live elements; all n readable afterwards
// ---------------------------------------------------------------------------
#[test]
fn row13_expand_preserves_elements() {
    let p = libs();
    let mut rng = Rng::new(0xD13D_13D1);
    unsafe {
        for trial in 0..64 {
            let cap = 1 + rng.below(16) as SizeT;
            let n = rng.below(cap as u64 + 1) as SizeT;
            let a = (p.c.init_array)(cap);
            let b = (p.rust.init_array)(cap);
            assert!(!a.is_null() && !b.is_null());
            let vals: Vec<c_int> = (0..n).map(|_| rng.spicy_i32()).collect();
            for &v in &vals {
                assert_eq!((p.c.add_element)(a, v), (p.rust.add_element)(b, v));
            }
            assert_eq!((p.c.expand_array)(a), (p.rust.expand_array)(b), "trial {trial}");
            let sa = p.c.snapshot(a);
            let sb = p.rust.snapshot(b);
            assert_eq!(sa, sb, "trial {trial}: post-expand state");
            assert_eq!(sa.elements, vals, "trial {trial}: elements survived realloc");
            (p.c.free_array)(a);
            (p.rust.free_array)(b);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 14-18 — add_element growth boundaries and value shapes
// ---------------------------------------------------------------------------
/// Add `n` values to a fresh array of capacity `cap` in both libraries,
/// comparing the return value of every single `add_element` call and the full
/// array state after each one.
fn drive_adds(p: &Pair, cap: SizeT, values: &[c_int]) {
    unsafe {
        let a = (p.c.init_array)(cap);
        let b = (p.rust.init_array)(cap);
        assert_eq!(a.is_null(), b.is_null(), "init_array({cap}) null-ness");
        if a.is_null() {
            return;
        }
        for (i, &v) in values.iter().enumerate() {
            let ra = (p.c.add_element)(a, v);
            let rb = (p.rust.add_element)(b, v);
            assert_eq!(ra, rb, "add_element #{i} (value {v}, cap {cap}) return value");
            assert_eq!(
                p.c.snapshot(a),
                p.rust.snapshot(b),
                "state after add_element #{i} (value {v}, cap {cap})"
            );
        }
        // If the buffer got freed by a `realloc(p, 0)` (capacity 0 shape), the
        // pointer dangles in BOTH libraries; neutralise it identically so the
        // free below is safe.
        if (*a).capacity == 0 {
            (*a).data = std::ptr::null_mut();
            (*b).data = std::ptr::null_mut();
        }
        (p.c.free_array)(a);
        (p.rust.free_array)(b);
    }
}

#[test]
fn row14_add_exactly_capacity_no_growth() {
    let p = libs();
    let mut rng = Rng::new(0x1414_1414);
    for cap in [1usize, 2, 3, 8, 64] {
        let vals: Vec<c_int> = (0..cap).map(|_| rng.spicy_i32()).collect();
        drive_adds(&p, cap, &vals);
    }
}

#[test]
fn row15_add_one_past_capacity_triggers_growth() {
    let p = libs();
    let mut rng = Rng::new(0x1515_1515);
    for cap in [1usize, 2, 3, 8, 64] {
        let vals: Vec<c_int> = (0..cap + 1).map(|_| rng.spicy_i32()).collect();
        drive_adds(&p, cap, &vals);
    }
}

#[test]
fn row16_add_64_elements_from_capacity_one() {
    let p = libs();
    let mut rng = Rng::new(0x1616_1616);
    let vals: Vec<c_int> = (0..64).map(|_| rng.spicy_i32()).collect();
    drive_adds(&p, 1, &vals);
}

#[test]
fn row17_matrixsum_array_shape_capacity_two_four_adds() {
    let p = libs();
    let mut rng = Rng::new(0x1717_1717);
    for _ in 0..64 {
        let vals: Vec<c_int> = (0..4).map(|_| rng.spicy_i32()).collect();
        drive_adds(&p, 2, &vals);
    }
}

#[test]
fn row18_add_element_edge_values() {
    let p = libs();
    let edges: Vec<c_int> = vec![0, -1, 1, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 0x7F];
    for cap in [1usize, 2, 4, 8] {
        drive_adds(&p, cap, &edges);
    }
}

// ---------------------------------------------------------------------------
// Row 19 — 512 random low-level programs over the whole array API
// ---------------------------------------------------------------------------
#[test]
fn row19_random_low_level_pipelines() {
    let p = libs();
    let mut rng = Rng::new(0x1919_1919);
    unsafe {
        for prog in 0..512 {
            let cap = rng.below(9) as SizeT; // includes the degenerate 0
            let a = (p.c.init_array)(cap);
            let b = (p.rust.init_array)(cap);
            assert_eq!(a.is_null(), b.is_null(), "prog {prog}: init_array({cap})");
            if a.is_null() {
                continue;
            }
            assert_eq!(p.c.snapshot(a), p.rust.snapshot(b), "prog {prog}: fresh state");

            let steps = 1 + rng.below(24);
            for step in 0..steps {
                match rng.below(4) {
                    0 => {
                        let ra = (p.c.expand_array)(a);
                        let rb = (p.rust.expand_array)(b);
                        assert_eq!(ra, rb, "prog {prog} step {step}: expand_array return");
                    }
                    _ => {
                        let v = rng.spicy_i32();
                        let ra = (p.c.add_element)(a, v);
                        let rb = (p.rust.add_element)(b, v);
                        assert_eq!(
                            ra, rb,
                            "prog {prog} step {step}: add_element({v}) return"
                        );
                    }
                }
                assert_eq!(
                    p.c.snapshot(a),
                    p.rust.snapshot(b),
                    "prog {prog} step {step}: array state diverged"
                );
                if (*a).capacity == 0 {
                    // realloc(p,0) freed the buffer in both libs; stop touching it.
                    break;
                }
            }
            if (*a).capacity == 0 {
                (*a).data = std::ptr::null_mut();
                (*b).data = std::ptr::null_mut();
            }
            (p.c.free_array)(a);
            (p.rust.free_array)(b);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 20-23 — matrixsum
// ---------------------------------------------------------------------------
fn check_matrixsum(p: &Pair, q: [c_int; 4]) {
    unsafe {
        let ra = (p.c.matrixsum)(q[0], q[1], q[2], q[3]);
        let rb = (p.rust.matrixsum)(q[0], q[1], q[2], q[3]);
        assert_eq!(ra, rb, "matrixsum{q:?}");
    }
}

#[test]
fn row20_matrixsum_all_16_validity_patterns() {
    let p = libs();
    // `matrix` is shared process-wide; serialise against other tests.
    let _guard = matrix_lock();
    let mut rng = Rng::new(0x2020_2020);
    for pattern in 0..16u32 {
        for _ in 0..32 {
            let mut q = [0i32; 4];
            for (i, v) in q.iter_mut().enumerate() {
                *v = if pattern & (1 << i) != 0 {
                    // guaranteed non-zero
                    let mut x = rng.spicy_i32();
                    if x == 0 {
                        x = 1;
                    }
                    x
                } else {
                    0
                };
            }
            check_matrixsum(&p, q);
        }
    }
}

#[test]
fn row21_matrixsum_small_values_no_overflow() {
    let p = libs();
    // `matrix` is shared process-wide; serialise against other tests.
    let _guard = matrix_lock();
    let mut rng = Rng::new(0x2121_2121);
    for _ in 0..1024 {
        let q = [
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
        ];
        check_matrixsum(&p, q);
    }
}

#[test]
fn row22_matrixsum_overflow_shapes() {
    let p = libs();
    // `matrix` is shared process-wide; serialise against other tests.
    let _guard = matrix_lock();
    let edges: [i32; 9] =
        [i32::MAX, i32::MIN, i32::MAX / 2, i32::MIN / 2, i32::MAX - 1, i32::MIN + 1, 0, 1, -1];
    for &a in &edges {
        for &b in &edges {
            for &c in &edges {
                for &d in &edges {
                    check_matrixsum(&p, [a, b, c, d]);
                }
            }
        }
    }
}

#[test]
fn row23_matrixsum_randomized_full_range() {
    let p = libs();
    // `matrix` is shared process-wide; serialise against other tests.
    let _guard = matrix_lock();
    let mut rng = Rng::new(0x2323_2323);
    for _ in 0..4096 {
        let q = [rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32()];
        check_matrixsum(&p, q);
    }
}

// ---------------------------------------------------------------------------
// Row 24 — matrixsum with a mutated `matrix` (exercises the `& 0xFFF` mask)
// ---------------------------------------------------------------------------
#[test]
fn row24_matrixsum_with_mutated_matrix() {
    let p = libs();
    // `matrix` is shared process-wide; serialise against other tests.
    let _guard = matrix_lock();
    let mut rng = Rng::new(0x2424_2424);
    unsafe {
        let orig = p.c.get_matrix();
        // Hand-picked matrices that push the checksum above 0xFFF, exactly on
        // the mask boundary, and negative.
        let mut mats: Vec<[[c_int; 4]; 3]> = vec![
            [[0; 4]; 3],
            [[0xFFF, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
            [[0x1000, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
            [[0x1001, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
            [[-1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
            [[i32::MIN, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
            [[i32::MAX, 1, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
            [[0x1_0000; 4]; 3],
        ];
        for _ in 0..128 {
            let mut m = [[0i32; 4]; 3];
            for row in m.iter_mut() {
                for v in row.iter_mut() {
                    *v = rng.spicy_i32();
                }
            }
            mats.push(m);
        }
        for m in mats {
            p.c.set_matrix(m);
            p.rust.set_matrix(m);
            assert_eq!(
                (p.c.calculate_matrix_checksum)(),
                (p.rust.calculate_matrix_checksum)(),
                "checksum with matrix {m:?}"
            );
            for _ in 0..8 {
                let q = [rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32()];
                unsafe {
                    let ra = (p.c.matrixsum)(q[0], q[1], q[2], q[3]);
                    let rb = (p.rust.matrixsum)(q[0], q[1], q[2], q[3]);
                    assert_eq!(ra, rb, "matrixsum{q:?} with matrix {m:?}");
                }
            }
        }
        p.c.set_matrix(orig);
        p.rust.set_matrix(orig);
    }
}

// ---------------------------------------------------------------------------
// Row 25 — interleaved global mutation, matrixsum, and low-level array use
// ---------------------------------------------------------------------------
#[test]
fn row25_interleaved_state_no_cross_leaks() {
    let p = libs();
    // `matrix` is shared process-wide; serialise against other tests.
    let _guard = matrix_lock();
    let mut rng = Rng::new(0x2525_2525);
    unsafe {
        let orig = p.c.get_matrix();
        for round in 0..128 {
            // 1. mutate the global
            let mut m = [[0i32; 4]; 3];
            for row in m.iter_mut() {
                for v in row.iter_mut() {
                    *v = rng.range_i32(-4096, 4096);
                }
            }
            p.c.set_matrix(m);
            p.rust.set_matrix(m);

            // 2. one-shot wrapper
            let q = [rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32()];
            assert_eq!(
                (p.c.matrixsum)(q[0], q[1], q[2], q[3]),
                (p.rust.matrixsum)(q[0], q[1], q[2], q[3]),
                "round {round}: matrixsum{q:?}"
            );

            // 3. low-level pipeline, held live across the wrapper call
            let a = (p.c.init_array)(2);
            let b = (p.rust.init_array)(2);
            assert!(!a.is_null() && !b.is_null());
            for i in 0..6 {
                let v = rng.spicy_i32();
                assert_eq!(
                    (p.c.add_element)(a, v),
                    (p.rust.add_element)(b, v),
                    "round {round}: add #{i}"
                );
            }
            assert_eq!(
                (p.c.matrixsum)(q[3], q[2], q[1], q[0]),
                (p.rust.matrixsum)(q[3], q[2], q[1], q[0]),
                "round {round}: matrixsum reversed while an array is live"
            );
            assert_eq!(
                p.c.snapshot(a),
                p.rust.snapshot(b),
                "round {round}: live array perturbed by matrixsum"
            );
            assert_eq!(
                (p.c.process_flags)(q[0]),
                (p.rust.process_flags)(q[0]),
                "round {round}: process_flags after mixed traffic"
            );
            assert_eq!(
                (p.c.calculate_matrix_checksum)(),
                (p.rust.calculate_matrix_checksum)(),
                "round {round}: checksum unchanged by matrixsum"
            );
            (p.c.free_array)(a);
            (p.rust.free_array)(b);
        }
        p.c.set_matrix(orig);
        p.rust.set_matrix(orig);
    }
}
