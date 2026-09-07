//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`, each driven with many randomized inputs
//! from a fixed seed. Both implementations are reached only through their
//! `.so` exports.

mod common;

use common::*;
use std::ffi::c_int;

// ===========================================================================
// Row 1 — process_flags, exhaustive 4-bit option space
// ===========================================================================
#[test]
fn cfg01_process_flags_all_16_nibbles() {
    let p = load_pair();
    unsafe {
        for flags in 0..16i32 {
            diff_eq!(
                p.c.process_flags(flags),
                p.rust.process_flags(flags),
                "process_flags(0b{:04b})",
                flags
            );
        }
    }
}

// ===========================================================================
// Row 2 — process_flags, nibble x randomized high-bit noise
// ===========================================================================
#[test]
fn cfg02_process_flags_nibble_times_high_bit_noise() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_0002_5EED);
    unsafe {
        for nibble in 0..16i32 {
            for _ in 0..500 {
                // any bits at or above 0x10, including the sign bit
                let noise = (rng.i32_any() as u32 & 0xFFFF_FFF0) as i32;
                let flags = nibble | noise;
                diff_eq!(
                    p.c.process_flags(flags),
                    p.rust.process_flags(flags),
                    "process_flags({:#010x}) nibble=0b{:04b}",
                    flags,
                    nibble
                );
            }
            // explicit high-bit boundaries
            for extra in [0x10, 0xF0, 0xFF00, 0x7FFF_FFF0u32 as i32, i32::MIN] {
                let flags = nibble | extra;
                diff_eq!(
                    p.c.process_flags(flags),
                    p.rust.process_flags(flags),
                    "process_flags({:#010x})",
                    flags
                );
            }
        }
    }
}

// ===========================================================================
// Row 3 — process_flags, randomized full-range i32 + boundaries
// ===========================================================================
#[test]
fn cfg03_process_flags_full_range_random() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_0003_5EED);
    unsafe {
        for b in I32_BOUNDARIES {
            diff_eq!(
                p.c.process_flags(b),
                p.rust.process_flags(b),
                "process_flags(boundary {})",
                b
            );
        }
        for _ in 0..20_000 {
            let f = rng.i32_any();
            diff_eq!(
                p.c.process_flags(f),
                p.rust.process_flags(f),
                "process_flags({})",
                f
            );
        }
    }
}

// ===========================================================================
// Row 4 — calculate_matrix_checksum on the pristine global, repeatedly
// ===========================================================================
#[test]
fn cfg04_matrix_checksum_pristine_and_stable() {
    let p = load_pair();
    unsafe {
        // The initializer data itself must be identical, row-major.
        diff_eq!(p.c.read_matrix(), p.rust.read_matrix(), "pristine matrix global");
        let expect = p.c.calculate_matrix_checksum();
        for i in 0..100 {
            diff_eq!(
                p.c.calculate_matrix_checksum(),
                p.rust.calculate_matrix_checksum(),
                "calculate_matrix_checksum() call #{}",
                i
            );
        }
        // and it is stable / pure
        assert_eq!(expect, p.c.calculate_matrix_checksum());
    }
}

// ===========================================================================
// Row 5 — calculate_matrix_checksum over a randomized global
// ===========================================================================
#[test]
fn cfg05_matrix_checksum_randomized_global() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_0005_5EED);
    unsafe {
        let saved_c = p.c.read_matrix();
        let saved_r = p.rust.read_matrix();

        for iter in 0..5_000 {
            let mut vals = [0i32; 12];
            for v in vals.iter_mut() {
                *v = rng.i32_any();
            }
            p.c.write_matrix(&vals);
            p.rust.write_matrix(&vals);
            // the write must be observable identically through the data symbol
            diff_eq!(p.c.read_matrix(), p.rust.read_matrix(), "matrix readback iter {}", iter);
            diff_eq!(
                p.c.calculate_matrix_checksum(),
                p.rust.calculate_matrix_checksum(),
                "checksum over randomized matrix {:?}",
                vals
            );
        }

        p.c.write_matrix(&saved_c);
        p.rust.write_matrix(&saved_r);
    }
}

// ===========================================================================
// Row 6 — calculate_matrix_checksum at boundary matrix shapes
// ===========================================================================
#[test]
fn cfg06_matrix_checksum_boundary_shapes() {
    let p = load_pair();
    unsafe {
        let saved_c = p.c.read_matrix();
        let saved_r = p.rust.read_matrix();

        let mut shapes: Vec<[c_int; 12]> = vec![
            [0; 12],
            [i32::MAX; 12],
            [i32::MIN; 12],
            [1; 12],
            [-1; 12],
            [0xFFF; 12],
            [0x1000; 12],
        ];
        // alternating +-
        let mut alt = [0i32; 12];
        for (i, v) in alt.iter_mut().enumerate() {
            *v = if i % 2 == 0 { i32::MAX } else { i32::MIN };
        }
        shapes.push(alt);
        // single non-zero cell in each of the 12 positions, at each boundary
        for pos in 0..12 {
            for b in [1, -1, i32::MAX, i32::MIN, 0xFFF, 0x1000] {
                let mut m = [0i32; 12];
                m[pos] = b;
                shapes.push(m);
            }
        }

        for vals in &shapes {
            p.c.write_matrix(vals);
            p.rust.write_matrix(vals);
            diff_eq!(
                p.c.calculate_matrix_checksum(),
                p.rust.calculate_matrix_checksum(),
                "checksum over boundary matrix {:?}",
                vals
            );
        }

        p.c.write_matrix(&saved_c);
        p.rust.write_matrix(&saved_r);
    }
}

// ===========================================================================
// Row 7 — init_array/free_array with the smallest non-zero capacity
// ===========================================================================
#[test]
fn cfg07_init_array_capacity_one() {
    let p = load_pair();
    unsafe {
        let ca = p.c.init_array(1);
        let ra = p.rust.init_array(1);
        assert!(!ca.is_null(), "C init_array(1) returned NULL");
        assert!(!ra.is_null(), "Rust init_array(1) returned NULL");
        diff_eq!(
            p.c.snapshot(ca, 0),
            p.rust.snapshot(ra, 0),
            "init_array(1) struct state"
        );
        p.c.free_array(ca);
        p.rust.free_array(ra);
    }
}

// ===========================================================================
// Row 8 — init_array/free_array across capacity shapes + randomized
// ===========================================================================
#[test]
fn cfg08_init_array_capacity_shapes() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_0008_5EED);
    unsafe {
        let mut caps: Vec<usize> = vec![0, 1, 2, 3, 4, 8, 64, 4096];
        for _ in 0..2_000 {
            caps.push(rng.range(1, 65_536) as usize);
        }
        for cap in caps {
            let ca = p.c.init_array(cap);
            let ra = p.rust.init_array(cap);
            diff_eq!(ca.is_null(), ra.is_null(), "init_array({}) nullness", cap);
            diff_eq!(
                p.c.snapshot(ca, 0),
                p.rust.snapshot(ra, 0),
                "init_array({}) struct state",
                cap
            );
            p.c.free_array(ca);
            p.rust.free_array(ra);
        }
    }
}

// ===========================================================================
// Row 9 — add_element below capacity (no growth path)
// ===========================================================================
#[test]
fn cfg09_add_element_below_capacity() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_0009_5EED);
    unsafe {
        for _ in 0..1_000 {
            let cap = rng.range(2, 64) as usize;
            let n = rng.below(cap as u64) as usize; // n < cap
            let ca = p.c.init_array(cap);
            let ra = p.rust.init_array(cap);
            assert!(!ca.is_null() && !ra.is_null());
            for _ in 0..n {
                let v = rng.i32_any();
                diff_eq!(
                    p.c.add_element(ca, v),
                    p.rust.add_element(ra, v),
                    "add_element(cap={}, v={}) return",
                    cap,
                    v
                );
            }
            diff_eq!(
                p.c.snapshot(ca, n),
                p.rust.snapshot(ra, n),
                "after {} adds into cap {}",
                n,
                cap
            );
            p.c.free_array(ca);
            p.rust.free_array(ra);
        }
    }
}

// ===========================================================================
// Row 10 — add_element exactly filling capacity
// ===========================================================================
#[test]
fn cfg10_add_element_exactly_at_capacity() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_000A_5EED);
    unsafe {
        for cap in [1usize, 2, 3, 4, 7, 8, 16, 33, 64] {
            for _ in 0..50 {
                let ca = p.c.init_array(cap);
                let ra = p.rust.init_array(cap);
                assert!(!ca.is_null() && !ra.is_null());
                for i in 0..cap {
                    let v = rng.i32_any();
                    diff_eq!(
                        p.c.add_element(ca, v),
                        p.rust.add_element(ra, v),
                        "add #{} filling cap {}",
                        i,
                        cap
                    );
                }
                diff_eq!(
                    p.c.snapshot(ca, cap),
                    p.rust.snapshot(ra, cap),
                    "array exactly full at cap {}",
                    cap
                );
                p.c.free_array(ca);
                p.rust.free_array(ra);
            }
        }
    }
}

// ===========================================================================
// Row 11 — add_element triggering exactly one growth
// ===========================================================================
#[test]
fn cfg11_add_element_single_growth() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_000B_5EED);
    unsafe {
        for cap in [1usize, 2, 3, 4, 5, 8, 16, 31, 32, 100] {
            for _ in 0..30 {
                let ca = p.c.init_array(cap);
                let ra = p.rust.init_array(cap);
                assert!(!ca.is_null() && !ra.is_null());
                let n = cap + 1;
                for i in 0..n {
                    let v = rng.i32_any();
                    diff_eq!(
                        p.c.add_element(ca, v),
                        p.rust.add_element(ra, v),
                        "add #{} of {} (cap {})",
                        i,
                        n,
                        cap
                    );
                }
                let cs = p.c.snapshot(ca, n).unwrap();
                let rs = p.rust.snapshot(ra, n).unwrap();
                assert_eq!(cs, rs, "single-growth state at cap {}", cap);
                assert_eq!(cs.capacity, cap * 2, "C capacity should have doubled");
                assert_eq!(cs.size, n);
                p.c.free_array(ca);
                p.rust.free_array(ra);
            }
        }
    }
}

// ===========================================================================
// Row 12 — repeated growth (many doublings)
// ===========================================================================
#[test]
fn cfg12_add_element_repeated_growth() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_000C_5EED);
    unsafe {
        for start_cap in [1usize, 2] {
            for _ in 0..40 {
                let n = rng.range(100, 500) as usize;
                let ca = p.c.init_array(start_cap);
                let ra = p.rust.init_array(start_cap);
                assert!(!ca.is_null() && !ra.is_null());
                for i in 0..n {
                    let v = rng.i32_any();
                    diff_eq!(
                        p.c.add_element(ca, v),
                        p.rust.add_element(ra, v),
                        "add #{} of {} (start cap {})",
                        i,
                        n,
                        start_cap
                    );
                    // compare capacity growth after every insert, not just at the end
                    diff_eq!(
                        (*ca).capacity,
                        (*ra).capacity,
                        "capacity after add #{} (start cap {})",
                        i,
                        start_cap
                    );
                }
                diff_eq!(
                    p.c.snapshot(ca, n),
                    p.rust.snapshot(ra, n),
                    "after {} adds from start cap {}",
                    n,
                    start_cap
                );
                p.c.free_array(ca);
                p.rust.free_array(ra);
            }
        }
    }
}

// ===========================================================================
// Row 13 — expand_array driven directly (low-level entry point)
// ===========================================================================
#[test]
fn cfg13_expand_array_direct_doubling_sequence() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_000D_5EED);
    unsafe {
        for _ in 0..300 {
            let cap = rng.range(1, 16) as usize;
            let k = rng.range(1, 12) as usize;
            let ca = p.c.init_array(cap);
            let ra = p.rust.init_array(cap);
            assert!(!ca.is_null() && !ra.is_null());
            for step in 0..k {
                diff_eq!(
                    p.c.expand_array(ca),
                    p.rust.expand_array(ra),
                    "expand_array step {} (start cap {})",
                    step,
                    cap
                );
                diff_eq!(
                    ((*ca).size, (*ca).capacity, (*ca).data.is_null()),
                    ((*ra).size, (*ra).capacity, (*ra).data.is_null()),
                    "state after expand step {} (start cap {})",
                    step,
                    cap
                );
            }
            p.c.free_array(ca);
            p.rust.free_array(ra);
        }
    }
}

// ===========================================================================
// Row 14 — interleaved add_element / expand_array (composed pipeline)
// ===========================================================================
#[test]
fn cfg14_interleaved_add_and_expand() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_000E_5EED);
    unsafe {
        for prog in 0..400 {
            let cap = rng.range(1, 8) as usize;
            let ca = p.c.init_array(cap);
            let ra = p.rust.init_array(cap);
            assert!(!ca.is_null() && !ra.is_null());
            let mut pushed = 0usize;
            let steps = rng.range(5, 60) as usize;
            for s in 0..steps {
                match rng.below(4) {
                    0 => {
                        diff_eq!(
                            p.c.expand_array(ca),
                            p.rust.expand_array(ra),
                            "prog {} step {}: expand_array",
                            prog,
                            s
                        );
                    }
                    _ => {
                        let v = rng.i32_any();
                        diff_eq!(
                            p.c.add_element(ca, v),
                            p.rust.add_element(ra, v),
                            "prog {} step {}: add_element({})",
                            prog,
                            s,
                            v
                        );
                        pushed += 1;
                    }
                }
                diff_eq!(
                    ((*ca).size, (*ca).capacity),
                    ((*ra).size, (*ra).capacity),
                    "prog {} step {}: size/capacity",
                    prog,
                    s
                );
            }
            diff_eq!(
                p.c.snapshot(ca, pushed),
                p.rust.snapshot(ra, pushed),
                "prog {} final state",
                prog
            );
            p.c.free_array(ca);
            p.rust.free_array(ra);
        }
    }
}

// ===========================================================================
// Row 15 — matrixsum, exhaustive 16 zero/non-zero patterns
// ===========================================================================
#[test]
fn cfg15_matrixsum_all_16_flag_patterns() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_000F_5EED);
    unsafe {
        for pattern in 0..16u32 {
            for _ in 0..1_000 {
                let mk = |bit: u32, r: &mut Rng| {
                    if pattern & (1 << bit) != 0 {
                        r.i32_small_nonzero()
                    } else {
                        0
                    }
                };
                let a = mk(0, &mut rng);
                let b = mk(1, &mut rng);
                let c = mk(2, &mut rng);
                let d = mk(3, &mut rng);
                diff_eq!(
                    p.c.matrixsum(a, b, c, d),
                    p.rust.matrixsum(a, b, c, d),
                    "matrixsum({}, {}, {}, {}) pattern 0b{:04b}",
                    a,
                    b,
                    c,
                    d,
                    pattern
                );
            }
        }
    }
}

// ===========================================================================
// Row 16 — matrixsum, randomized full-range i32 (overflowing arithmetic)
// ===========================================================================
#[test]
fn cfg16_matrixsum_full_range_random() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_0010_5EED);
    unsafe {
        for _ in 0..20_000 {
            let a = rng.i32_any();
            let b = rng.i32_any();
            let c = rng.i32_any();
            let d = rng.i32_any();
            diff_eq!(
                p.c.matrixsum(a, b, c, d),
                p.rust.matrixsum(a, b, c, d),
                "matrixsum({}, {}, {}, {})",
                a,
                b,
                c,
                d
            );
        }
    }
}

// ===========================================================================
// Row 17 — matrixsum degenerate all-zero shape
// ===========================================================================
#[test]
fn cfg17_matrixsum_all_zero() {
    let p = load_pair();
    unsafe {
        diff_eq!(
            p.c.matrixsum(0, 0, 0, 0),
            p.rust.matrixsum(0, 0, 0, 0),
            "matrixsum(0,0,0,0)"
        );
    }
}

// ===========================================================================
// Row 18 — matrixsum boundary cross-product over the 4 parameter positions
// ===========================================================================
#[test]
fn cfg18_matrixsum_boundary_cross_product() {
    let p = load_pair();
    unsafe {
        // full 9^4 = 6561 cross-product of the boundary set
        for &a in I32_BOUNDARIES.iter() {
            for &b in I32_BOUNDARIES.iter() {
                for &c in I32_BOUNDARIES.iter() {
                    for &d in I32_BOUNDARIES.iter() {
                        diff_eq!(
                            p.c.matrixsum(a, b, c, d),
                            p.rust.matrixsum(a, b, c, d),
                            "matrixsum({}, {}, {}, {})",
                            a,
                            b,
                            c,
                            d
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 19 — matrixsum x mutated matrix global (option/state interaction)
// ===========================================================================
#[test]
fn cfg19_matrixsum_with_mutated_matrix_global() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_0013_5EED);
    unsafe {
        let saved_c = p.c.read_matrix();
        let saved_r = p.rust.read_matrix();

        // deterministic `& 0xFFF`-boundary matrices: make the checksum land on
        // 0xFFF, 0x1000, 0x1001, negatives, and overflow.
        let mut shapes: Vec<[c_int; 12]> = Vec::new();
        for total in [
            0, 1, 0xFFE, 0xFFF, 0x1000, 0x1001, 0x1FFF, 0x2000, -1, -0xFFF, -0x1000, i32::MAX,
            i32::MIN,
        ] {
            let mut m = [0i32; 12];
            m[0] = total;
            shapes.push(m);
            // same total spread across cells
            let mut m2 = [0i32; 12];
            m2[3] = total / 2;
            m2[7] = total - total / 2;
            shapes.push(m2);
        }
        for _ in 0..300 {
            let mut m = [0i32; 12];
            for v in m.iter_mut() {
                *v = rng.i32_any();
            }
            shapes.push(m);
        }

        for vals in &shapes {
            p.c.write_matrix(vals);
            p.rust.write_matrix(vals);
            for _ in 0..20 {
                let a = rng.i32_any();
                let b = rng.i32_small();
                let c = rng.i32_any();
                let d = rng.i32_small();
                diff_eq!(
                    p.c.matrixsum(a, b, c, d),
                    p.rust.matrixsum(a, b, c, d),
                    "matrixsum({}, {}, {}, {}) with matrix {:?}",
                    a,
                    b,
                    c,
                    d,
                    vals
                );
            }
            // and the low-level checksum in the same state
            diff_eq!(
                p.c.calculate_matrix_checksum(),
                p.rust.calculate_matrix_checksum(),
                "checksum with matrix {:?}",
                vals
            );
        }

        p.c.write_matrix(&saved_c);
        p.rust.write_matrix(&saved_r);
    }
}

// ===========================================================================
// Row 20 — randomized whole-library program (order/hidden-state divergence)
// ===========================================================================
#[test]
fn cfg20_randomized_full_stack_program() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_0014_5EED);
    unsafe {
        let saved_c = p.c.read_matrix();
        let saved_r = p.rust.read_matrix();

        // live arrays, kept in lockstep between the two implementations
        let mut arrays: Vec<(*mut DynamicArray, *mut DynamicArray, usize)> = Vec::new();

        for step in 0..20_000 {
            match rng.below(7) {
                0 => {
                    let cap = rng.range(1, 32) as usize;
                    let ca = p.c.init_array(cap);
                    let ra = p.rust.init_array(cap);
                    diff_eq!(ca.is_null(), ra.is_null(), "step {}: init_array({})", step, cap);
                    if !ca.is_null() {
                        arrays.push((ca, ra, 0));
                    }
                }
                1 if !arrays.is_empty() => {
                    let i = rng.below(arrays.len() as u64) as usize;
                    let (ca, ra, n) = arrays[i];
                    let v = rng.i32_any();
                    diff_eq!(
                        p.c.add_element(ca, v),
                        p.rust.add_element(ra, v),
                        "step {}: add_element({})",
                        step,
                        v
                    );
                    arrays[i].2 = n + 1;
                    diff_eq!(
                        ((*ca).size, (*ca).capacity),
                        ((*ra).size, (*ra).capacity),
                        "step {}: size/capacity after add",
                        step
                    );
                }
                2 if !arrays.is_empty() => {
                    let i = rng.below(arrays.len() as u64) as usize;
                    let (ca, ra, _) = arrays[i];
                    diff_eq!(
                        p.c.expand_array(ca),
                        p.rust.expand_array(ra),
                        "step {}: expand_array",
                        step
                    );
                    diff_eq!(
                        ((*ca).size, (*ca).capacity),
                        ((*ra).size, (*ra).capacity),
                        "step {}: size/capacity after expand",
                        step
                    );
                }
                3 if !arrays.is_empty() => {
                    let i = rng.below(arrays.len() as u64) as usize;
                    let (ca, ra, n) = arrays.remove(i);
                    diff_eq!(
                        p.c.snapshot(ca, n),
                        p.rust.snapshot(ra, n),
                        "step {}: snapshot before free",
                        step
                    );
                    p.c.free_array(ca);
                    p.rust.free_array(ra);
                }
                4 => {
                    let mut m = [0i32; 12];
                    for v in m.iter_mut() {
                        *v = rng.i32_any();
                    }
                    p.c.write_matrix(&m);
                    p.rust.write_matrix(&m);
                    diff_eq!(
                        p.c.calculate_matrix_checksum(),
                        p.rust.calculate_matrix_checksum(),
                        "step {}: checksum after matrix write",
                        step
                    );
                }
                5 => {
                    let (a, b, c, d) = (rng.i32_any(), rng.i32_any(), rng.i32_any(), rng.i32_any());
                    diff_eq!(
                        p.c.matrixsum(a, b, c, d),
                        p.rust.matrixsum(a, b, c, d),
                        "step {}: matrixsum({}, {}, {}, {})",
                        step,
                        a,
                        b,
                        c,
                        d
                    );
                }
                _ => {
                    let f = rng.i32_any();
                    diff_eq!(
                        p.c.process_flags(f),
                        p.rust.process_flags(f),
                        "step {}: process_flags({})",
                        step,
                        f
                    );
                }
            }
        }

        for (ca, ra, n) in arrays.drain(..) {
            diff_eq!(p.c.snapshot(ca, n), p.rust.snapshot(ra, n), "final snapshot");
            p.c.free_array(ca);
            p.rust.free_array(ra);
        }

        p.c.write_matrix(&saved_c);
        p.rust.write_matrix(&saved_r);
    }
}
