//! Level 3: `perform_computation_with_history` — the out-parameter heavy
//! function. Compared on return value, mutated `*history_count`, and the raw
//! bytes of every history slot.
mod common;

use common::{both, raw_bytes, Api, ComputationResult};
use std::ffi::{c_int, c_void};

extern "C" {
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

/// Drives `perform_computation_with_history` starting from a NULL history and
/// returns (return values, final count, raw bytes of the 10 slots).
unsafe fn run_from_null(api: &Api, calls: &[(c_int, c_int, c_int)]) -> (Vec<c_int>, c_int, Vec<u8>) {
    let mut history: *mut ComputationResult = std::ptr::null_mut();
    let mut count: c_int = 12345; // deliberately garbage: the NULL branch resets it
    let mut rets = Vec::new();
    for &(a, b, op) in calls {
        rets.push((api.perform_computation_with_history)(
            a,
            b,
            op,
            &mut history,
            &mut count,
        ));
    }
    assert!(!history.is_null(), "history was never allocated");
    let bytes = raw_bytes(history, 10);
    libc_free(history as *mut c_void);
    (rets, count, bytes)
}

fn call_scripts() -> Vec<Vec<(c_int, c_int, c_int)>> {
    vec![
        vec![(1, 2, 1)],
        vec![(7, 3, 4)],
        vec![(7, 3, 5)],
        vec![(7, 0, 4)],
        vec![(7, 0, 5)],
        vec![(-7, 3, 3)],
        vec![(6, 7, 2)],
        vec![(1, 1, 0)],   // default: -> add
        vec![(1, 1, 6)],   // default: -> add
        vec![(1, 1, -3)],  // default: -> add
        vec![(1, 1, i32::MIN)],
        // Fill exactly to the cap, then keep going past it.
        (0..10).map(|i| (i, i + 1, (i % 5) + 1)).collect(),
        (0..15).map(|i| (i * 3, i + 2, (i % 7) + 1)).collect(),
        (0..25).map(|i| (100 - i, i, i % 6)).collect(),
    ]
}

#[test]
fn perform_computation_from_null_history() {
    let b = both();
    for (i, script) in call_scripts().iter().enumerate() {
        let (cr, cc, cb) = unsafe { run_from_null(&b.c, script) };
        let (rr, rc, rb) = unsafe { run_from_null(&b.rust, script) };
        assert_eq!(cr, rr, "script #{i}: return values differ");
        assert_eq!(cc, rc, "script #{i}: final history_count differs");
        assert_eq!(cb, rb, "script #{i}: history bytes differ");
    }
}

#[test]
fn perform_computation_null_branch_resets_count() {
    let b = both();
    // A garbage incoming count must be reset to 0 by the NULL-history branch,
    // so the first entry always lands in slot 0 and the count becomes 1.
    for garbage in [-500_i32, -1, 0, 7, 10, 99, i32::MAX] {
        for api in [&b.c, &b.rust] {
            unsafe {
                let mut history: *mut ComputationResult = std::ptr::null_mut();
                let mut count: c_int = garbage;
                let ret = (api.perform_computation_with_history)(
                    41,
                    1,
                    1,
                    &mut history,
                    &mut count,
                );
                assert_eq!(ret, 42);
                assert_eq!(count, 1, "count not reset (incoming {garbage})");
                assert_eq!((*history).value, 42);
                libc_free(history as *mut c_void);
            }
        }
    }
}

#[test]
fn perform_computation_respects_the_cap() {
    let b = both();
    // With a pre-allocated (non-NULL) history, counts >= 10 must neither write
    // nor increment.
    for start in [10_i32, 11, 50, i32::MAX - 1] {
        let mut c_out = Vec::new();
        let mut r_out = Vec::new();
        for (api, out) in [(&b.c, &mut c_out), (&b.rust, &mut r_out)] {
            unsafe {
                let mut history = (api.allocate_results)(10);
                assert!(!history.is_null());
                let mut count: c_int = start;
                let ret = (api.perform_computation_with_history)(
                    9,
                    4,
                    3,
                    &mut history,
                    &mut count,
                );
                out.push((ret, count, raw_bytes(history, 10)));
                libc_free(history as *mut c_void);
            }
        }
        assert_eq!(c_out, r_out, "cap behaviour differs for start count {start}");
        assert_eq!(c_out[0].1, start, "count must not move at or above the cap");
        assert!(
            c_out[0].2.iter().all(|&x| x == 0),
            "history must stay untouched at or above the cap"
        );
    }
}

#[test]
fn perform_computation_at_cap_boundary() {
    let b = both();
    // start = 9 is the last accepted slot.
    let mut results = Vec::new();
    for api in [&b.c, &b.rust] {
        unsafe {
            let mut history = (api.allocate_results)(10);
            let mut count: c_int = 9;
            let r1 = (api.perform_computation_with_history)(20, 5, 3, &mut history, &mut count);
            let c1 = count;
            let r2 = (api.perform_computation_with_history)(20, 5, 3, &mut history, &mut count);
            let c2 = count;
            results.push((r1, c1, r2, c2, raw_bytes(history, 10)));
            libc_free(history as *mut c_void);
        }
    }
    assert_eq!(results[0], results[1], "boundary behaviour differs");
    assert_eq!(results[0].1, 10);
    assert_eq!(results[0].3, 10, "count must saturate at 10");
}

#[test]
fn history_struct_layout_is_interchangeable() {
    let b = both();
    // Fill a buffer with the C implementation and re-read/extend it with the
    // Rust implementation (and vice versa). Identical bytes prove the
    // ComputationResult layout and stride agree across the ABI.
    let mut dumps = Vec::new();
    for (writer, extender) in [(&b.c, &b.rust), (&b.rust, &b.c)] {
        unsafe {
            let mut history: *mut ComputationResult = std::ptr::null_mut();
            let mut count: c_int = 0;
            for i in 0..5 {
                (writer.perform_computation_with_history)(
                    i * 11,
                    3,
                    (i % 5) + 1,
                    &mut history,
                    &mut count,
                );
            }
            for i in 0..5 {
                (extender.perform_computation_with_history)(
                    1000 - i,
                    7,
                    (i % 5) + 1,
                    &mut history,
                    &mut count,
                );
            }
            assert_eq!(count, 10);
            dumps.push(raw_bytes(history, 10));
            libc_free(history as *mut c_void);
        }
    }
    assert_eq!(
        dumps[0], dumps[1],
        "C-then-Rust and Rust-then-C produce different history bytes"
    );
    // Every slot must carry a status of STATUS_SUCCESS (0) and a shifted stamp.
    unsafe {
        let slots: &[ComputationResult] =
            std::slice::from_raw_parts(dumps[0].as_ptr() as *const ComputationResult, 10);
        for (i, s) in slots.iter().enumerate() {
            assert_eq!(s.status, 0, "slot {i} status");
            assert!(s.timestamp > 0 && s.timestamp < 1_000_000, "slot {i} stamp");
        }
    }
}
