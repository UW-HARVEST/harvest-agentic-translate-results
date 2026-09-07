//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid
//! input/condition, calls BOTH `.so`s, and asserts they return the SAME
//! sentinel (`NULL` / `0` / `-1`), not merely "both failed".

mod common;

use common::*;
use std::ffi::c_int;

// ===========================================================================
// Row 2 — init_array with an oversized capacity => malloc fails => NULL
// ===========================================================================
#[test]
fn err_init_array_oversized_capacity_returns_null() {
    let p = load_pair();
    // Each of these makes `initial_capacity * sizeof(int)` a byte count that
    // genuinely cannot be satisfied: it neither wraps to something small nor
    // lands in the allocatable range. Note (1<<62)+1 is deliberately NOT here:
    // its byte count wraps to 4, so the C succeeds (see row 4's test).
    let caps: [usize; 6] = [
        usize::MAX,
        usize::MAX - 1,
        usize::MAX / 2,
        usize::MAX / 4,
        1usize << 61,
        1usize << 60,
    ];
    for cap in caps {
        let bytes = cap.wrapping_mul(4);
        assert!(
            bytes >= (1usize << 48),
            "precondition: {:#x} must be an unallocatable byte count for cap {:#x}",
            bytes,
            cap
        );
        let (cs, rs) = probe_init_sequential(&p, cap);
        diff_eq!(
            cs.is_none(),
            rs.is_none(),
            "init_array({:#x}) must both return NULL",
            cap
        );
        assert!(cs.is_none(), "expected C init_array({:#x}) == NULL", cap);
    }
}

// ===========================================================================
// Row 3 — init_array(0): malloc(0) succeeds, so this is NOT an error
// ===========================================================================
#[test]
fn err_init_array_zero_capacity_is_not_an_error() {
    let p = load_pair();
    unsafe {
        let ca = p.c.init_array(0);
        let ra = p.rust.init_array(0);
        diff_eq!(ca.is_null(), ra.is_null(), "init_array(0) nullness");
        assert!(!ca.is_null(), "glibc malloc(0) is non-NULL; C should succeed");
        diff_eq!(
            p.c.snapshot(ca, 0),
            p.rust.snapshot(ra, 0),
            "init_array(0) struct state"
        );
        assert_eq!((*ca).size, 0);
        assert_eq!((*ca).capacity, 0);
        p.c.free_array(ca);
        p.rust.free_array(ra);
    }
}

// ===========================================================================
// Row 4 — capacity whose byte count wraps to 0 => malloc(0) => NOT an error
// ===========================================================================
#[test]
fn err_init_array_capacity_multiply_wraps_to_zero() {
    let p = load_pair();
    // On LP64, sizeof(int)==4, so (1<<62)*4 == 2^64 == 0 (mod 2^64), and
    // ((1<<62)+1)*4 == 4. The C does no overflow check, so neither may be
    // rejected — both become tiny mallocs that succeed while `capacity` keeps
    // the absurd unchecked value.
    assert_eq!(core::mem::size_of::<c_int>(), 4);
    for cap in [1usize << 62, (1usize << 62) + 1, (1usize << 62) + 2, 1usize << 63] {
        let bytes = cap.wrapping_mul(4);
        assert!(bytes < 16, "precondition: byte count {:#x} must wrap small", bytes);

        let (cs, rs) = probe_init_sequential(&p, cap);
        diff_eq!(
            cs.is_none(),
            rs.is_none(),
            "init_array({:#x}) (wraps to malloc({})) nullness",
            cap,
            bytes
        );
        diff_eq!(cs.clone(), rs, "init_array({:#x}) struct state", cap);
        if let Some(s) = cs {
            assert_eq!(s.capacity, cap, "C must record the unchecked capacity");
            assert_eq!(s.size, 0);
        }
    }
}

// ===========================================================================
// Row 5 — expand_array(NULL) => 0
// ===========================================================================
#[test]
fn err_expand_array_null_returns_zero() {
    let p = load_pair();
    unsafe {
        let cv = p.c.expand_array(core::ptr::null_mut());
        let rv = p.rust.expand_array(core::ptr::null_mut());
        diff_eq!(cv, rv, "expand_array(NULL)");
        assert_eq!(cv, 0, "C expand_array(NULL) must be 0");
    }
}

// ===========================================================================
// Row 6 — expand_array whose realloc cannot be satisfied => 0, state intact
// ===========================================================================
#[test]
fn err_expand_array_realloc_failure_returns_zero() {
    let p = load_pair();
    unsafe {
        // Forge a huge capacity on a real (small) buffer so that
        // capacity*2*sizeof(int) is unallocatable but does not wrap to 0.
        // (A capacity of exactly 1<<61 would make the byte count wrap to 0,
        // turning the call into realloc(p, 0), which glibc treats as free(p) —
        // that is row 7's case, not an allocation failure, so it is excluded
        // here.)
        for forged in [usize::MAX / 8, usize::MAX / 4, 1usize << 59, 1usize << 50] {
            assert_ne!(
                forged.wrapping_mul(2).wrapping_mul(4),
                0,
                "precondition: byte count must not wrap to 0 for {:#x}",
                forged
            );
            let ca = p.c.init_array(4);
            let ra = p.rust.init_array(4);
            assert!(!ca.is_null() && !ra.is_null());
            (*ca).capacity = forged;
            (*ra).capacity = forged;
            let c_data = (*ca).data;
            let r_data = (*ra).data;

            let cv = p.c.expand_array(ca);
            let rv = p.rust.expand_array(ra);
            diff_eq!(cv, rv, "expand_array with forged capacity {:#x}", forged);
            assert_eq!(cv, 0, "C expand_array should fail for capacity {:#x}", forged);

            // On failure the C leaves data/capacity untouched; Rust must too.
            diff_eq!(
                ((*ca).data == c_data, (*ca).capacity, (*ca).size),
                ((*ra).data == r_data, (*ra).capacity, (*ra).size),
                "state preserved after failed expand ({:#x})",
                forged
            );
            assert_eq!((*ca).capacity, forged, "C must not update capacity on failure");

            // restore a sane capacity before freeing
            (*ca).capacity = 4;
            (*ra).capacity = 4;
            p.c.free_array(ca);
            p.rust.free_array(ra);
        }
    }
}

// ===========================================================================
// Row 7 — expand_array on capacity 0 => realloc(p, 0) => 0 (glibc frees p)
// ===========================================================================
#[test]
fn err_expand_array_zero_capacity_realloc_zero() {
    let p = load_pair();
    unsafe {
        let ca = p.c.init_array(0);
        let ra = p.rust.init_array(0);
        assert!(!ca.is_null() && !ra.is_null());
        assert_eq!((*ca).capacity, 0);

        let cv = p.c.expand_array(ca);
        let rv = p.rust.expand_array(ra);
        diff_eq!(cv, rv, "expand_array on capacity-0 array");

        // Whatever glibc's realloc(p, 0) does, both must agree on the
        // observable struct state afterwards.
        diff_eq!(
            ((*ca).capacity, (*ca).size, (*ca).data.is_null()),
            ((*ra).capacity, (*ra).size, (*ra).data.is_null()),
            "struct state after realloc(p, 0)"
        );

        // NOTE: under glibc, realloc(p, 0) frees `p` and returns NULL while the
        // C leaves `arr->data` dangling, so calling free_array here would be a
        // double free *in the C too*. The header allocation is deliberately
        // leaked rather than replicating that UB inside the test process.
        let _ = (ca, ra);
    }
}

// ===========================================================================
// Row 8 — add_element(NULL, v) => 0
// ===========================================================================
#[test]
fn err_add_element_null_returns_zero() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_00C8_5EED);
    unsafe {
        let mut vals: Vec<c_int> = I32_BOUNDARIES.to_vec();
        for _ in 0..200 {
            vals.push(rng.i32_any());
        }
        for v in vals {
            let cv = p.c.add_element(core::ptr::null_mut(), v);
            let rv = p.rust.add_element(core::ptr::null_mut(), v);
            diff_eq!(cv, rv, "add_element(NULL, {})", v);
            assert_eq!(cv, 0, "C add_element(NULL, {}) must be 0", v);
        }
    }
}

// ===========================================================================
// Row 9 — add_element on a full array whose growth fails => 0, size unchanged
// ===========================================================================
#[test]
fn err_add_element_growth_failure_returns_zero() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_00C9_5EED);
    unsafe {
        for forged in [usize::MAX / 8, usize::MAX / 4, 1usize << 59] {
            assert_ne!(forged.wrapping_mul(2).wrapping_mul(4), 0);
            let ca = p.c.init_array(4);
            let ra = p.rust.init_array(4);
            assert!(!ca.is_null() && !ra.is_null());
            // size >= capacity AND capacity*2*4 unallocatable
            (*ca).capacity = forged;
            (*ra).capacity = forged;
            (*ca).size = forged;
            (*ra).size = forged;

            let v = rng.i32_any();
            let cv = p.c.add_element(ca, v);
            let rv = p.rust.add_element(ra, v);
            diff_eq!(cv, rv, "add_element on ungrowable array ({:#x})", forged);
            assert_eq!(cv, 0, "C add_element must fail for capacity {:#x}", forged);
            diff_eq!(
                ((*ca).size, (*ca).capacity),
                ((*ra).size, (*ra).capacity),
                "size/capacity unchanged after failed add ({:#x})",
                forged
            );
            assert_eq!((*ca).size, forged, "C must not bump size on failure");

            (*ca).size = 0;
            (*ra).size = 0;
            (*ca).capacity = 4;
            (*ra).capacity = 4;
            p.c.free_array(ca);
            p.rust.free_array(ra);
        }
    }
}

// ===========================================================================
// Row 10 — add_element on a capacity-0 array => 0 (growth to 0 fails)
// ===========================================================================
#[test]
fn err_add_element_zero_capacity_returns_zero() {
    let p = load_pair();
    unsafe {
        let ca = p.c.init_array(0);
        let ra = p.rust.init_array(0);
        assert!(!ca.is_null() && !ra.is_null());

        let cv = p.c.add_element(ca, 42);
        let rv = p.rust.add_element(ra, 42);
        diff_eq!(cv, rv, "add_element on capacity-0 array");
        diff_eq!(
            ((*ca).size, (*ca).capacity, (*ca).data.is_null()),
            ((*ra).size, (*ra).capacity, (*ra).data.is_null()),
            "struct state after add_element on capacity-0 array"
        );

        // Same glibc realloc(p, 0) situation as row 7: deliberately leaked.
        let _ = (ca, ra);
    }
}

// ===========================================================================
// Row 11 — free_array(NULL) is a no-op in both
// ===========================================================================
#[test]
fn err_free_array_null_is_noop() {
    let p = load_pair();
    unsafe {
        for _ in 0..100 {
            p.c.free_array(core::ptr::null_mut());
            p.rust.free_array(core::ptr::null_mut());
        }
        // still functional afterwards => neither crashed nor corrupted state
        diff_eq!(
            p.c.calculate_matrix_checksum(),
            p.rust.calculate_matrix_checksum(),
            "library usable after free_array(NULL)"
        );
    }
}

// ===========================================================================
// Row 13 — process_flags with out-of-range "enum"/flag ints (never rejects)
// ===========================================================================
#[test]
fn err_process_flags_out_of_range_values() {
    let p = load_pair();
    unsafe {
        // Values one step past the documented FLAG_READ..FLAG_DELETE range, and
        // ints with no valid flag variant at all. C enums accept any int.
        let mut vals: Vec<c_int> = vec![
            0x10,       // one past FLAG_DELETE
            0x0F + 1,   // one past the full valid nibble
            0x20,
            0x80,
            0xFF,
            0x100,
            0xFFFF,
            -1,
            -2,
            -16,
            i32::MIN,
            i32::MAX,
            i32::MAX - 1,
            0x7FFF_FFF0,
            i32::MIN + 1,
        ];
        // every single-bit value, including bits with no flag variant
        for bit in 0..32u32 {
            vals.push(1i32.wrapping_shl(bit));
            vals.push(!(1i32.wrapping_shl(bit)));
        }
        for v in vals {
            let cv = p.c.process_flags(v);
            let rv = p.rust.process_flags(v);
            diff_eq!(cv, rv, "process_flags({:#010x})", v);
            assert!(
                (0..=4).contains(&cv),
                "C process_flags({:#010x}) returned {} (never rejects, 0..4)",
                v,
                cv
            );
        }
    }
}

// ===========================================================================
// Row 14 — matrixsum with extreme / one-past-range int arguments
// ===========================================================================
#[test]
fn err_matrixsum_extreme_int_arguments() {
    let p = load_pair();
    let mut rng = Rng::new(0x0000_00CE_5EED);
    unsafe {
        let extremes: [c_int; 8] = [
            i32::MIN,
            i32::MIN + 1,
            i32::MIN / 2,
            -1,
            1,
            i32::MAX / 2,
            i32::MAX - 1,
            i32::MAX,
        ];
        for &a in extremes.iter() {
            for &b in extremes.iter() {
                for &c in extremes.iter() {
                    for &d in extremes.iter() {
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
        // randomized extremes mixed with normal values
        for _ in 0..5_000 {
            let pick = |r: &mut Rng| {
                if r.below(2) == 0 {
                    extremes[r.below(8) as usize]
                } else {
                    r.i32_any()
                }
            };
            let a = pick(&mut rng);
            let b = pick(&mut rng);
            let c = pick(&mut rng);
            let d = pick(&mut rng);
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
// Row 15 — calculate_matrix_checksum with an overflowing global
// ===========================================================================
#[test]
fn err_matrix_checksum_overflowing_global() {
    let p = load_pair();
    unsafe {
        let saved_c = p.c.read_matrix();
        let saved_r = p.rust.read_matrix();

        for vals in [[i32::MAX; 12], [i32::MIN; 12], [i32::MAX / 3; 12], [i32::MIN / 3; 12]] {
            p.c.write_matrix(&vals);
            p.rust.write_matrix(&vals);
            diff_eq!(
                p.c.calculate_matrix_checksum(),
                p.rust.calculate_matrix_checksum(),
                "overflowing checksum for {:?}",
                vals
            );
            // and through matrixsum, which masks with 0xFFF
            diff_eq!(
                p.c.matrixsum(1, 1, 1, 1),
                p.rust.matrixsum(1, 1, 1, 1),
                "matrixsum with overflowing matrix {:?}",
                vals
            );
        }

        p.c.write_matrix(&saved_c);
        p.rust.write_matrix(&saved_r);
    }
}

// ===========================================================================
// Generic FFI boundaries not tied to a single ERRORS.md row
// ===========================================================================

/// Every pointer-taking entry point must tolerate NULL identically.
#[test]
fn err_generic_all_null_pointer_entry_points() {
    let p = load_pair();
    unsafe {
        let n = core::ptr::null_mut();
        diff_eq!(p.c.expand_array(n), p.rust.expand_array(n), "expand_array(NULL)");
        diff_eq!(p.c.add_element(n, 0), p.rust.add_element(n, 0), "add_element(NULL, 0)");
        diff_eq!(
            p.c.add_element(n, i32::MIN),
            p.rust.add_element(n, i32::MIN),
            "add_element(NULL, INT_MIN)"
        );
        p.c.free_array(n);
        p.rust.free_array(n);

        // repeated NULL abuse must not destabilize either library
        for _ in 0..1000 {
            diff_eq!(p.c.expand_array(n), p.rust.expand_array(n), "expand_array(NULL) rep");
            diff_eq!(p.c.add_element(n, 1), p.rust.add_element(n, 1), "add_element(NULL) rep");
            p.c.free_array(n);
            p.rust.free_array(n);
        }
        diff_eq!(
            p.c.matrixsum(1, 2, 3, 4),
            p.rust.matrixsum(1, 2, 3, 4),
            "matrixsum still correct after NULL abuse"
        );
    }
}

/// Zero and oversized `size_t` lengths at the `init_array` boundary.
#[test]
fn err_generic_zero_and_oversized_lengths() {
    let p = load_pair();
    // A sweep spanning zero, tiny, and every power of two up to usize::MAX,
    // which crosses the malloc-succeeds / byte-count-wraps / malloc-fails
    // regimes. Both implementations must agree on each. Probed sequentially so
    // the multi-gigabyte sizes do not contend for the process memory ceiling.
    let mut caps: Vec<usize> = vec![0, 1, 2, 3];
    for shift in 0..64u32 {
        caps.push(1usize << shift);
        caps.push((1usize << shift) + 1);
    }
    caps.push(usize::MAX);
    caps.push(usize::MAX - 1);

    for cap in caps {
        let (cs, rs) = probe_init_sequential(&p, cap);
        diff_eq!(cs.is_none(), rs.is_none(), "init_array({:#x}) nullness", cap);
        diff_eq!(cs, rs, "init_array({:#x}) struct state", cap);
    }
}

/// Values one step past a "documented valid range" on every scalar entry point.
#[test]
fn err_generic_one_past_range_scalars() {
    let p = load_pair();
    unsafe {
        // process_flags: valid documented mask is 0b0000..0b1111
        for v in [-1, 0, 0b1111, 0b1_0000, 0b1_0001] {
            diff_eq!(
                p.c.process_flags(v),
                p.rust.process_flags(v),
                "process_flags one-past {:#x}",
                v
            );
        }
        // matrixsum has no documented range; step past the int extremes by
        // wrapping into them from both ends.
        for v in [i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX] {
            diff_eq!(
                p.c.matrixsum(v, v, v, v),
                p.rust.matrixsum(v, v, v, v),
                "matrixsum({v}, {v}, {v}, {v})"
            );
            diff_eq!(
                p.c.matrixsum(v, 0, 0, 0),
                p.rust.matrixsum(v, 0, 0, 0),
                "matrixsum({v}, 0, 0, 0)"
            );
            diff_eq!(
                p.c.matrixsum(0, 0, 0, v),
                p.rust.matrixsum(0, 0, 0, v),
                "matrixsum(0, 0, 0, {v})"
            );
        }
    }
}
