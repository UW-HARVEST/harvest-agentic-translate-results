//! Level 2: `select_operation`, `get_computation_timestamp`, `allocate_results`.
mod common;

use common::{both, raw_bytes, ComputationResult};
use std::ffi::c_int;

/// Operation values covering the enumerated set, the `default:` arm, and the
/// out-of-range / negative values `mathop` can actually produce.
fn op_values() -> Vec<c_int> {
    let mut v: Vec<c_int> = (-12..=12).collect();
    v.extend([100, -100, i32::MAX, i32::MIN, 1000, -1000]);
    v
}

#[test]
fn select_operation_resolves_to_the_same_function() {
    let b = both();
    for op in op_values() {
        let (cp, rp) = unsafe { ((b.c.select_operation)(op), (b.rust.select_operation)(op)) };
        let cn = b.c.name_of_op(cp);
        let rn = b.rust.name_of_op(rp);
        assert!(
            !cn.starts_with('<'),
            "C select_operation({op}) returned an unrecognised pointer: {cn}"
        );
        assert_eq!(
            cn, rn,
            "select_operation({op}) picked different functions (C={cn}, Rust={rn})"
        );
    }
}

#[test]
fn select_operation_returned_pointers_behave_identically() {
    let b = both();
    let probes: [(c_int, c_int); 12] = [
        (0, 0),
        (1, 0),
        (0, 1),
        (7, 3),
        (-7, 3),
        (7, -3),
        (-7, -3),
        (100, 7),
        (i32::MAX, 3),
        (i32::MIN, 3),
        (12345, 0),
        (-1, 1),
    ];
    for op in op_values() {
        let (cp, rp) = unsafe { ((b.c.select_operation)(op), (b.rust.select_operation)(op)) };
        for (a, x) in probes {
            let (cr, rr) = unsafe { (cp(a, x, 0), rp(a, x, 0)) };
            assert_eq!(cr, rr, "select_operation({op}) applied to ({a},{x})");
        }
    }
}

#[test]
fn get_computation_timestamp_matches() {
    let b = both();
    for _ in 0..64 {
        let (cr, rr) = unsafe {
            (
                (b.c.get_computation_timestamp)(),
                (b.rust.get_computation_timestamp)(),
            )
        };
        assert_eq!(
            cr, rr,
            "get_computation_timestamp() differs (time() >> 29 changes only every ~17 years)"
        );
        // Sanity: the shift really is applied (a raw epoch would be ~1.7e9).
        assert!(cr < 1_000_000, "value {cr} does not look shifted by 29");
    }
}

#[test]
fn allocate_results_zeroed_and_same_layout() {
    let b = both();
    assert_eq!(std::mem::size_of::<ComputationResult>(), 24);

    for count in [1_i32, 2, 3, 10, 11, 100, 1000] {
        unsafe {
            let cp = (b.c.allocate_results)(count);
            let rp = (b.rust.allocate_results)(count);
            assert!(!cp.is_null(), "C allocate_results({count}) returned NULL");
            assert!(!rp.is_null(), "Rust allocate_results({count}) returned NULL");

            let n = count as usize;
            let cb = raw_bytes(cp, n);
            let rb = raw_bytes(rp, n);
            assert_eq!(cb, rb, "allocate_results({count}) contents differ");
            assert!(
                cb.iter().all(|&x| x == 0),
                "allocate_results({count}) is not zeroed"
            );
            assert_eq!(cb.len(), n * 24, "unexpected element stride");

            libc_free(cp as *mut _);
            libc_free(rp as *mut _);
        }
    }
}

#[test]
fn allocate_results_zero_count() {
    let b = both();
    unsafe {
        // calloc(0, 24) is implementation-defined but consistent within a
        // process: both sides must agree on null-ness.
        let cp = (b.c.allocate_results)(0);
        let rp = (b.rust.allocate_results)(0);
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "allocate_results(0) disagrees on NULL"
        );
        if !cp.is_null() {
            libc_free(cp as *mut _);
        }
        if !rp.is_null() {
            libc_free(rp as *mut _);
        }
    }
}

#[test]
fn allocate_results_negative_count() {
    let b = both();
    // A negative count sign-extends to an enormous size_t; calloc must fail
    // the same way on both sides.
    for count in [-1_i32, -10, i32::MIN] {
        unsafe {
            let cp = (b.c.allocate_results)(count);
            let rp = (b.rust.allocate_results)(count);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "allocate_results({count}) disagrees on NULL (C={cp:?}, Rust={rp:?})"
            );
            if !cp.is_null() {
                libc_free(cp as *mut _);
            }
            if !rp.is_null() {
                libc_free(rp as *mut _);
            }
        }
    }
}

extern "C" {
    #[link_name = "free"]
    fn libc_free(p: *mut std::ffi::c_void);
}
