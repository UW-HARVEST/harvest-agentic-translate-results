// Phase B -- valid-path differential tests.
//
// One test per row of CONFIGS.md. Every row drives BOTH shared objects through
// their exported symbols and compares results byte-for-byte, using many
// randomized inputs per row (fixed seed, reproducible).

mod common;
use common::*;
use std::ffi::c_int;

/// Re-implementation of `matrixsum`'s body using ONLY the library's low-level
/// exports, so the composed pipeline is exercised the way a real consumer would
/// compose it (CONFIGS.md row 22).
fn external_pipeline(a: &Api, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> c_int {
    const FLAG_READ: c_int = 0b0001;
    const FLAG_WRITE: c_int = 0b0010;
    const FLAG_EXECUTE: c_int = 0b0100;
    const FLAG_DELETE: c_int = 0b1000;
    let hex_base: c_int = 0xFF;
    let hex_multiplier: c_int = 0x10;

    let mut permissions: c_int = 0;
    if p1 != 0 {
        permissions |= FLAG_READ;
    }
    if p2 != 0 {
        permissions |= FLAG_WRITE;
    }
    if p3 != 0 {
        permissions |= FLAG_EXECUTE;
    }
    if p4 != 0 {
        permissions |= FLAG_DELETE;
    }

    unsafe {
        let arr = (a.init_array)(2);
        if arr.is_null() {
            return -1;
        }
        (a.add_element)(arr, p1);
        (a.add_element)(arr, p2);
        (a.add_element)(arr, p3);
        (a.add_element)(arr, p4);

        let mut sum: c_int = 0;
        for v in a.read_elems(arr) {
            sum = sum.wrapping_add(v);
        }

        let flag_count = (a.process_flags)(permissions);
        let matrix_sum = (a.calculate_matrix_checksum)();

        let result = sum
            .wrapping_mul(hex_multiplier)
            .wrapping_add(flag_count.wrapping_mul(hex_base))
            .wrapping_add(matrix_sum & 0xFFF);

        (a.free_array)(arr);
        result
    }
}

// ===========================================================================
// Rows 1-3: process_flags (lowest-level leaf, no allocation, no globals)
// ===========================================================================

#[test]
fn cfg01_process_flags_exhaustive_low_nibble() {
    let p = load_pair();
    for flags in 0..16 {
        assert_same!(
            format!("process_flags({flags:#06b})"),
            unsafe { (p.c.process_flags)(flags) },
            unsafe { (p.r.process_flags)(flags) }
        );
    }
}

#[test]
fn cfg02_process_flags_reserved_high_bits_ignored() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..4000 {
        let low = (rng.below(16)) as c_int;
        // Randomize everything above the 4 documented flag bits.
        let high = (rng.next_i32() as u32 & 0xFFFF_FFF0) as c_int;
        let flags = low | high;
        assert_same!(
            format!("process_flags({flags:#010x}) low={low:#b}"),
            unsafe { (p.c.process_flags)(flags) },
            unsafe { (p.r.process_flags)(flags) }
        );
    }
}

#[test]
fn cfg03_process_flags_full_int_range_and_boundaries() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 3);
    let mut cases: Vec<c_int> = vec![
        0,
        -1,
        1,
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
        0x0F,
        0x10,
        !0x0F,
        0x7FFF_FFF0,
        -16,
    ];
    for _ in 0..5000 {
        cases.push(rng.next_i32());
    }
    for flags in cases {
        assert_same!(
            format!("process_flags({flags})"),
            unsafe { (p.c.process_flags)(flags) },
            unsafe { (p.r.process_flags)(flags) }
        );
    }
}

// ===========================================================================
// Row 4: calculate_matrix_checksum on default state
// ===========================================================================

#[test]
fn cfg04_checksum_default_matrix_is_stable() {
    let p = load_pair();
    with_matrix_lock(&p, || {
        for i in 0..50 {
            assert_same!(
                format!("calculate_matrix_checksum() call #{i} (default matrix)"),
                unsafe { (p.c.calculate_matrix_checksum)() },
                unsafe { (p.r.calculate_matrix_checksum)() }
            );
        }
        // Also pin the absolute expected value from the C source constants.
        let expect: c_int = MATRIX_DEFAULT.iter().sum();
        assert_eq!(unsafe { (p.c.calculate_matrix_checksum)() }, expect);
        assert_eq!(unsafe { (p.r.calculate_matrix_checksum)() }, expect);
    });
}

// ===========================================================================
// Rows 5-7: init_array
// ===========================================================================

fn check_init_free(p: &Pair, cap: usize) {
    unsafe {
        let ch = (p.c.init_array)(cap);
        let rh = (p.r.init_array)(cap);
        assert_same!(
            format!("init_array({cap}) null-ness"),
            ch.is_null(),
            rh.is_null()
        );
        if ch.is_null() {
            return;
        }
        assert_same!(
            format!("init_array({cap}) handle state"),
            p.c.snapshot(ch),
            p.r.snapshot(rh)
        );
        let cs = p.c.read_handle(ch);
        assert_eq!(cs.size, 0, "C init_array must start empty");
        assert_eq!(cs.capacity, cap, "C init_array capacity");
        (p.c.free_array)(ch);
        (p.r.free_array)(rh);
    }
}

#[test]
fn cfg05_init_array_capacity_one() {
    let p = load_pair();
    check_init_free(&p, 1);
}

#[test]
fn cfg06_init_array_capacity_two_as_used_by_matrixsum() {
    let p = load_pair();
    check_init_free(&p, 2);
}

#[test]
fn cfg07_init_array_randomized_small_and_medium_capacities() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 7);
    for cap in [1usize, 2, 3, 4, 5, 7, 8, 16, 17, 63, 64, 255, 256, 1024, 4096] {
        check_init_free(&p, cap);
    }
    for _ in 0..800 {
        let cap = 1 + rng.below(4096) as usize;
        check_init_free(&p, cap);
    }
}

// ===========================================================================
// Rows 8-9: expand_array
// ===========================================================================

#[test]
fn cfg08_expand_array_from_capacity_one_preserves_contents() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..300 {
        let v = rng.next_i32();
        unsafe {
            let ch = (p.c.init_array)(1);
            let rh = (p.r.init_array)(1);
            assert!(!ch.is_null() && !rh.is_null());

            assert_same!("add_element pre-expand", (p.c.add_element)(ch, v), (p.r.add_element)(rh, v));
            assert_same!("state pre-expand", p.c.snapshot(ch), p.r.snapshot(rh));

            assert_same!("expand_array rc", (p.c.expand_array)(ch), (p.r.expand_array)(rh));
            assert_same!("state post-expand", p.c.snapshot(ch), p.r.snapshot(rh));

            let cs = p.c.read_handle(ch);
            assert_eq!(cs.capacity, 2, "capacity must double 1 -> 2");
            assert_eq!(p.c.read_elems(ch), vec![v], "contents must survive realloc");

            (p.c.free_array)(ch);
            (p.r.free_array)(rh);
        }
    }
}

#[test]
fn cfg09_expand_array_repeated_doublings() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..300 {
        let cap = 1 + rng.below(32) as usize;
        let nfill = rng.below(cap as u64 + 1) as usize;
        let vals: Vec<c_int> = (0..nfill).map(|_| rng.next_i32()).collect();
        unsafe {
            let ch = (p.c.init_array)(cap);
            let rh = (p.r.init_array)(cap);
            assert!(!ch.is_null() && !rh.is_null());
            for &v in &vals {
                assert_same!("fill", (p.c.add_element)(ch, v), (p.r.add_element)(rh, v));
            }
            for round in 0..4 {
                assert_same!(
                    format!("expand_array round {round} (cap={cap})"),
                    (p.c.expand_array)(ch),
                    (p.r.expand_array)(rh)
                );
                assert_same!(
                    format!("state after expand round {round} (cap={cap})"),
                    p.c.snapshot(ch),
                    p.r.snapshot(rh)
                );
                let cs = p.c.read_handle(ch);
                assert_eq!(cs.capacity, cap << (round + 1), "doubling {round}");
                assert_eq!(p.c.read_elems(ch), vals, "contents preserved");
            }
            (p.c.free_array)(ch);
            (p.r.free_array)(rh);
        }
    }
}

// ===========================================================================
// Rows 10-13: add_element (fast path, boundary, one expansion, many)
// ===========================================================================

/// Append `vals` to a fresh array of `cap`, comparing rc + full state each step.
fn check_append_sequence(p: &Pair, cap: usize, vals: &[c_int]) {
    unsafe {
        let ch = (p.c.init_array)(cap);
        let rh = (p.r.init_array)(cap);
        assert_same!(
            format!("init_array({cap}) null-ness"),
            ch.is_null(),
            rh.is_null()
        );
        if ch.is_null() {
            return;
        }
        for (i, &v) in vals.iter().enumerate() {
            assert_same!(
                format!("add_element #{i} value={v} (cap={cap})"),
                (p.c.add_element)(ch, v),
                (p.r.add_element)(rh, v)
            );
            assert_same!(
                format!("state after add #{i} (cap={cap}, n={})", vals.len()),
                p.c.snapshot(ch),
                p.r.snapshot(rh)
            );
        }
        assert_eq!(p.c.read_elems(ch), vals, "C stored contents");
        assert_eq!(p.r.read_elems(rh), vals, "Rust stored contents");
        (p.c.free_array)(ch);
        (p.r.free_array)(rh);
    }
}

#[test]
fn cfg10_add_element_below_capacity_fast_path() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..400 {
        let cap = 2 + rng.below(30) as usize;
        let n = rng.below(cap as u64) as usize; // strictly fewer than cap
        let vals: Vec<c_int> = (0..n).map(|_| rng.interesting_i32()).collect();
        check_append_sequence(&p, cap, &vals);
    }
}

#[test]
fn cfg11_add_element_exactly_capacity_no_expansion() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 11);
    for cap in [1usize, 2, 3, 4, 8, 16, 33] {
        for _ in 0..40 {
            let vals: Vec<c_int> = (0..cap).map(|_| rng.interesting_i32()).collect();
            check_append_sequence(&p, cap, &vals);
        }
    }
}

#[test]
fn cfg12_add_element_capacity_plus_one_triggers_one_expansion() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 12);
    for cap in [1usize, 2, 3, 4, 8, 16, 33] {
        for _ in 0..40 {
            let vals: Vec<c_int> = (0..cap + 1).map(|_| rng.interesting_i32()).collect();
            check_append_sequence(&p, cap, &vals);
        }
    }
}

#[test]
fn cfg13_add_element_many_repeated_doublings() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..60 {
        let vals: Vec<c_int> = (0..200).map(|_| rng.interesting_i32()).collect();
        check_append_sequence(&p, 1, &vals);
    }
    // Also from capacity 2 (matrixsum's shape) and 3 (odd, so doublings are 3,6,12...)
    for cap in [2usize, 3, 5] {
        for _ in 0..30 {
            let vals: Vec<c_int> = (0..150).map(|_| rng.next_i32()).collect();
            check_append_sequence(&p, cap, &vals);
        }
    }
}

// ===========================================================================
// Rows 14-16: mutated global `matrix`
// ===========================================================================

#[test]
fn cfg14_checksum_with_randomized_mutated_matrix() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 14);
    with_matrix_lock(&p, || {
        for _ in 0..2000 {
            let mut m = [0 as c_int; MATRIX_LEN];
            for e in m.iter_mut() {
                *e = rng.next_small();
            }
            p.set_matrices(&m);
            assert_same!(
                format!("calculate_matrix_checksum() matrix={m:?}"),
                unsafe { (p.c.calculate_matrix_checksum)() },
                unsafe { (p.r.calculate_matrix_checksum)() }
            );
        }
    });
}

#[test]
fn cfg15_checksum_with_overflowing_matrix_extremes() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 15);
    with_matrix_lock(&p, || {
        // Deliberate wrap-around cases: the C accumulates into a plain `int`.
        let mut cases: Vec<[c_int; MATRIX_LEN]> = vec![
            [c_int::MAX; MATRIX_LEN],
            [c_int::MIN; MATRIX_LEN],
            [
                c_int::MAX,
                c_int::MAX,
                c_int::MIN,
                c_int::MIN,
                1,
                -1,
                0,
                0,
                c_int::MAX,
                c_int::MIN,
                7,
                -7,
            ],
            [-1; MATRIX_LEN],
            [0; MATRIX_LEN],
        ];
        for _ in 0..1500 {
            let mut m = [0 as c_int; MATRIX_LEN];
            for e in m.iter_mut() {
                *e = rng.interesting_i32();
            }
            cases.push(m);
        }
        for m in cases {
            p.set_matrices(&m);
            assert_same!(
                format!("overflowing checksum matrix={m:?}"),
                unsafe { (p.c.calculate_matrix_checksum)() },
                unsafe { (p.r.calculate_matrix_checksum)() }
            );
        }
    });
}

#[test]
fn cfg16_matrixsum_with_mutated_matrix() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 16);
    with_matrix_lock(&p, || {
        for _ in 0..1500 {
            let mut m = [0 as c_int; MATRIX_LEN];
            for e in m.iter_mut() {
                *e = rng.interesting_i32();
            }
            p.set_matrices(&m);
            let (a, b, c, d) = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            assert_same!(
                format!("matrixsum({a},{b},{c},{d}) with matrix={m:?}"),
                unsafe { (p.c.matrixsum)(a, b, c, d) },
                unsafe { (p.r.matrixsum)(a, b, c, d) }
            );
        }
    });
}

// ===========================================================================
// Rows 17-21: matrixsum
// ===========================================================================

#[test]
fn cfg17_matrixsum_all_16_permission_patterns() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 17);
    with_matrix_lock(&p, || {
        for pattern in 0..16u32 {
            for _ in 0..200 {
                let mk = |bit: u32, rng: &mut Rng| -> c_int {
                    if pattern & (1 << bit) != 0 {
                        rng.next_nonzero_i32()
                    } else {
                        0
                    }
                };
                let a = mk(0, &mut rng);
                let b = mk(1, &mut rng);
                let c = mk(2, &mut rng);
                let d = mk(3, &mut rng);
                assert_same!(
                    format!("matrixsum({a},{b},{c},{d}) pattern={pattern:#06b}"),
                    unsafe { (p.c.matrixsum)(a, b, c, d) },
                    unsafe { (p.r.matrixsum)(a, b, c, d) }
                );
            }
        }
    });
}

#[test]
fn cfg18_matrixsum_all_zero_params() {
    let p = load_pair();
    with_matrix_lock(&p, || {
        assert_same!("matrixsum(0,0,0,0)", unsafe { (p.c.matrixsum)(0, 0, 0, 0) }, unsafe {
            (p.r.matrixsum)(0, 0, 0, 0)
        });
        // flag_count == 0 and sum == 0, so the result is the matrix term alone.
        let expect = unsafe { (p.c.calculate_matrix_checksum)() } & 0xFFF;
        assert_eq!(unsafe { (p.c.matrixsum)(0, 0, 0, 0) }, expect);
        assert_eq!(unsafe { (p.r.matrixsum)(0, 0, 0, 0) }, expect);
    });
}

#[test]
fn cfg19_matrixsum_randomized_full_int_range() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 19);
    with_matrix_lock(&p, || {
        for _ in 0..20_000 {
            let (a, b, c, d) = (
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            );
            assert_same!(
                format!("matrixsum({a},{b},{c},{d})"),
                unsafe { (p.c.matrixsum)(a, b, c, d) },
                unsafe { (p.r.matrixsum)(a, b, c, d) }
            );
        }
    });
}

#[test]
fn cfg20_matrixsum_boundary_scalars_in_every_position() {
    let p = load_pair();
    const B: [c_int; 8] = [
        0,
        1,
        -1,
        c_int::MAX,
        c_int::MIN,
        0x0800_0000,
        0x7FFF_FFF0,
        -0x0800_0000,
    ];
    with_matrix_lock(&p, || {
        // Full cross product over the boundary set: 8^4 = 4096 combinations.
        for &a in &B {
            for &b in &B {
                for &c in &B {
                    for &d in &B {
                        assert_same!(
                            format!("matrixsum({a},{b},{c},{d})"),
                            unsafe { (p.c.matrixsum)(a, b, c, d) },
                            unsafe { (p.r.matrixsum)(a, b, c, d) }
                        );
                    }
                }
            }
        }
    });
}

#[test]
fn cfg21_matrixsum_repeated_calls_no_state_leak() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 21);
    with_matrix_lock(&p, || {
        // Each call allocates, expands and frees; results must not drift.
        let fixed = unsafe { (p.c.matrixsum)(7, 8, 9, 10) };
        for i in 0..5000 {
            let (a, b, c, d) = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            assert_same!(
                format!("churn call #{i}: matrixsum({a},{b},{c},{d})"),
                unsafe { (p.c.matrixsum)(a, b, c, d) },
                unsafe { (p.r.matrixsum)(a, b, c, d) }
            );
            assert_eq!(
                unsafe { (p.c.matrixsum)(7, 8, 9, 10) },
                fixed,
                "C result drifted after churn"
            );
            assert_eq!(
                unsafe { (p.r.matrixsum)(7, 8, 9, 10) },
                fixed,
                "Rust result drifted after churn"
            );
        }
    });
}

// ===========================================================================
// Row 22: composed pipeline via the low-level exports
// ===========================================================================

#[test]
fn cfg22_composed_pipeline_matches_one_shot_wrapper() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 22);
    with_matrix_lock(&p, || {
        for _ in 0..4000 {
            // Vary the global too, so the pipeline's matrix term varies.
            if rng.below(4) == 0 {
                let mut m = [0 as c_int; MATRIX_LEN];
                for e in m.iter_mut() {
                    *e = rng.next_small();
                }
                p.set_matrices(&m);
            }
            let (a, b, c, d) = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            let c_one = unsafe { (p.c.matrixsum)(a, b, c, d) };
            let r_one = unsafe { (p.r.matrixsum)(a, b, c, d) };
            let c_ext = external_pipeline(&p.c, a, b, c, d);
            let r_ext = external_pipeline(&p.r, a, b, c, d);

            let ctx = format!("pipeline({a},{b},{c},{d})");
            assert_same!(format!("{ctx} one-shot"), c_one, r_one);
            assert_same!(format!("{ctx} composed"), c_ext, r_ext);
            assert_eq!(c_one, c_ext, "C one-shot vs composed: {ctx}");
            assert_eq!(r_one, r_ext, "Rust one-shot vs composed: {ctx}");
        }
    });
}

// ===========================================================================
// Row 23: cross-.so struct ABI layout
// ===========================================================================

#[test]
fn cfg23_dynamic_array_struct_abi_layout() {
    let p = load_pair();
    assert_eq!(std::mem::size_of::<DynamicArray>(), 24, "sizeof(DynamicArray)");
    unsafe {
        let ch = (p.c.init_array)(4);
        let rh = (p.r.init_array)(4);
        assert!(!ch.is_null() && !rh.is_null());
        for v in [11, 22, 33] {
            (p.c.add_element)(ch, v);
            (p.r.add_element)(rh, v);
        }
        // Read the raw fields at their C offsets: data@0, size@8, capacity@16.
        for (name, h) in [("C", ch), ("RUST", rh)] {
            let base = h as *const u8;
            let data = *(base.add(0) as *const *mut c_int);
            let size = *(base.add(8) as *const usize);
            let capacity = *(base.add(16) as *const usize);
            assert!(!data.is_null(), "[{name}] data@0");
            assert_eq!(size, 3, "[{name}] size@8 (size_t)");
            assert_eq!(capacity, 4, "[{name}] capacity@16 (size_t)");
            assert_eq!(
                std::slice::from_raw_parts(data, 3),
                &[11, 22, 33],
                "[{name}] buffer contents"
            );
        }
        assert_same!("struct-ABI snapshot", p.c.snapshot(ch), p.r.snapshot(rh));
        (p.c.free_array)(ch);
        (p.r.free_array)(rh);
    }
}

// ===========================================================================
// Row 24: several live handles, interleaved operations
// ===========================================================================

#[test]
fn cfg24_interleaved_multiple_live_handles() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..60 {
        const N: usize = 6;
        let caps: Vec<usize> = (0..N).map(|_| 1 + rng.below(4) as usize).collect();
        unsafe {
            let ch: Vec<_> = caps.iter().map(|&c| (p.c.init_array)(c)).collect();
            let rh: Vec<_> = caps.iter().map(|&c| (p.r.init_array)(c)).collect();
            assert!(ch.iter().all(|h| !h.is_null()) && rh.iter().all(|h| !h.is_null()));

            for step in 0..120 {
                let k = rng.below(N as u64) as usize;
                if rng.below(8) == 0 {
                    assert_same!(
                        format!("step {step}: expand_array handle {k}"),
                        (p.c.expand_array)(ch[k]),
                        (p.r.expand_array)(rh[k])
                    );
                } else {
                    let v = rng.interesting_i32();
                    assert_same!(
                        format!("step {step}: add_element handle {k} value {v}"),
                        (p.c.add_element)(ch[k], v),
                        (p.r.add_element)(rh[k], v)
                    );
                }
                // Every handle must stay consistent, not just the one touched.
                for j in 0..N {
                    assert_same!(
                        format!("step {step}: handle {j} state (caps={caps:?})"),
                        p.c.snapshot(ch[j]),
                        p.r.snapshot(rh[j])
                    );
                }
            }
            for j in 0..N {
                (p.c.free_array)(ch[j]);
                (p.r.free_array)(rh[j]);
            }
        }
    }
}
