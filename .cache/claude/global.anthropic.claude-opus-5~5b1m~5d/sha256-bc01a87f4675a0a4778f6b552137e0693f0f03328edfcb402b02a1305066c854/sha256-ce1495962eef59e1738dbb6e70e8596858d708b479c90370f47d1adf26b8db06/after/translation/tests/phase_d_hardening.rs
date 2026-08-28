// Phase D — additional ABI-level hardening checks.
//
// These probe observable properties that the CONFIGS.md / ERRORS.md rows do not
// force on their own, but that a real consumer can depend on:
//
//   H1  `get_operation(k)` must return the ADDRESS OF THE EXPORTED SYMBOL, not a
//       thunk. In C, `ops[0] = multiply_with_static` makes
//       `get_operation(0) == &multiply_with_static` an exact pointer equality
//       that a caller can test.
//   H2  Caller-supplied function pointers (not one of the library's own four)
//       must be invoked exactly once, with the right arguments, via the C ABI.
//   H3  Unaligned `int*` passed to `compute_checksum` -- the C reaches it only
//       through `memcpy`, so it is well defined and must not be mis-handled.
//   H4  Unaligned `ComputeState*` passed to `init_state` / `apply_operation`.
//   H5  The library must be re-entrant enough to be called from a callback.

mod common;
use common::*;

use std::ffi::{c_int, CString};
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

// ===========================================================================
// H1 — get_operation returns the exported symbol address
// ===========================================================================

#[test]
fn h1_get_operation_returns_exported_symbol_addresses() {
    let (c, r) = libs();

    for (lib, who) in [(c, "C"), (r, "Rust")] {
        for k in 0..4i32 {
            let got = unsafe { (lib.get_operation)(k) };
            let want = lib.kernel(k as usize);
            assert_eq!(
                got.map(|f| f as usize),
                Some(want as usize),
                "H1 [{who}]: get_operation({k}) must be the address of the exported \
                 kernel symbol (got {:?}, want {:?})",
                got.map(|f| f as usize),
                want as usize
            );
        }
        // Distinct opcodes must map to distinct functions.
        let addrs: Vec<usize> = (0..4)
            .map(|k| unsafe { (lib.get_operation)(k) }.unwrap() as usize)
            .collect();
        let mut sorted = addrs.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 4, "H1 [{who}]: opcodes must map to 4 distinct functions");
    }
}

// ===========================================================================
// H2 — caller-supplied callbacks
// ===========================================================================

static PROBE_CALLS: AtomicUsize = AtomicUsize::new(0);
static PROBE_LAST_A: AtomicI32 = AtomicI32::new(0);
static PROBE_LAST_B: AtomicI32 = AtomicI32::new(0);

/// A function pointer that belongs to NEITHER library.
unsafe extern "C" fn probe(a: c_int, b: c_int) -> c_int {
    PROBE_CALLS.fetch_add(1, Ordering::SeqCst);
    PROBE_LAST_A.store(a, Ordering::SeqCst);
    PROBE_LAST_B.store(b, Ordering::SeqCst);
    a.wrapping_mul(7).wrapping_sub(b) ^ 0x5A5A
}

#[test]
fn h2_execute_operation_with_caller_supplied_callback() {
    let (c, r) = libs();
    let name = CString::new("PROBE").unwrap();
    let mut rng = Rng::new(0xDEAD_0002);

    for i in 0..200 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();

        PROBE_CALLS.store(0, Ordering::SeqCst);
        let (cv, co) = capture(|| unsafe { (c.execute_operation)(Some(probe), a, b, name.as_ptr()) });
        let c_calls = PROBE_CALLS.load(Ordering::SeqCst);
        let (c_a, c_b) = (
            PROBE_LAST_A.load(Ordering::SeqCst),
            PROBE_LAST_B.load(Ordering::SeqCst),
        );

        PROBE_CALLS.store(0, Ordering::SeqCst);
        let (rv, ro) = capture(|| unsafe { (r.execute_operation)(Some(probe), a, b, name.as_ptr()) });
        let r_calls = PROBE_CALLS.load(Ordering::SeqCst);
        let (r_a, r_b) = (
            PROBE_LAST_A.load(Ordering::SeqCst),
            PROBE_LAST_B.load(Ordering::SeqCst),
        );

        assert_eq!(c_calls, 1, "H2 iter {i}: C must invoke the callback exactly once");
        assert_eq!(r_calls, 1, "H2 iter {i}: Rust must invoke the callback exactly once");
        assert_eq!((c_a, c_b), (a, b), "H2 iter {i}: C passed the wrong arguments");
        assert_eq!((r_a, r_b), (a, b), "H2 iter {i}: Rust passed the wrong arguments");
        assert_eq!(cv, rv, "H2 iter {i}: return value (a={a}, b={b})");
        assert_stdout_eq(&format!("H2 iter {i} a={a} b={b}"), &co, &ro);
    }
}

#[test]
fn h2b_apply_operation_with_caller_supplied_callback() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xDEAD_0003);

    for i in 0..200 {
        let seed = rng.interesting_i32();
        let value = rng.interesting_i32();

        let mut cb = StateBuf::new();
        let mut rb = StateBuf::new();
        let _ = capture(|| unsafe { (c.init_state)(cb.as_ptr(), seed) });
        let _ = capture(|| unsafe { (r.init_state)(rb.as_ptr(), seed) });

        PROBE_CALLS.store(0, Ordering::SeqCst);
        let (_, co) = capture(|| unsafe { (c.apply_operation)(cb.as_ptr(), value, Some(probe)) });
        let c_calls = PROBE_CALLS.load(Ordering::SeqCst);

        PROBE_CALLS.store(0, Ordering::SeqCst);
        let (_, ro) = capture(|| unsafe { (r.apply_operation)(rb.as_ptr(), value, Some(probe)) });
        let r_calls = PROBE_CALLS.load(Ordering::SeqCst);

        assert_eq!(c_calls, 1, "H2b iter {i}: C callback invocations");
        assert_eq!(r_calls, 1, "H2b iter {i}: Rust callback invocations");
        assert_stdout_eq(&format!("H2b iter {i}"), &co, &ro);
        assert_eq!(
            cb.bytes(),
            rb.bytes(),
            "H2b iter {i}: state after callback (seed={seed}, value={value})"
        );
        assert_eq!(cb.state().operation_count, 1, "H2b iter {i}: operation_count");
    }
}

// ===========================================================================
// H3 — unaligned `int*` for compute_checksum (reached only via memcpy in C)
// ===========================================================================

#[test]
fn h3_compute_checksum_unaligned_values_pointer() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xDEAD_0004);

    for i in 0..200 {
        // 16 payload bytes plus slack so every offset stays in bounds.
        let mut bytes = vec![0u8; 16 + 8];
        for b in bytes.iter_mut() {
            *b = (rng.next_u32() & 0xFF) as u8;
        }

        for offset in 0..8usize {
            for count in 1..=4i32 {
                let mut cbuf = bytes.clone();
                let mut rbuf = bytes.clone();
                let cp = unsafe { cbuf.as_mut_ptr().add(offset) } as *mut c_int;
                let rp = unsafe { rbuf.as_mut_ptr().add(offset) } as *mut c_int;

                let cv = unsafe { (c.compute_checksum)(cp, count) };
                let rv = unsafe { (r.compute_checksum)(rp, count) };
                assert_eq!(
                    cv, rv,
                    "H3 iter {i} offset={offset} count={count}: unaligned int* diverged"
                );
                assert_eq!(cbuf, bytes, "H3: C modified the buffer");
                assert_eq!(rbuf, bytes, "H3: Rust modified the buffer");
            }
        }
    }
}

// ===========================================================================
// H4 — unaligned `ComputeState*`
// ===========================================================================

#[test]
fn h4_unaligned_state_pointer() {
    let (c, r) = libs();
    let mut rng = Rng::new(0xDEAD_0005);

    for i in 0..100 {
        let seed = rng.interesting_i32();
        let value = rng.interesting_i32();

        for offset in 1..4usize {
            // 12 payload bytes + offset slack, poisoned so stray writes show up.
            let mut cbuf = vec![GUARD_BYTE; STATE_SIZE + 8];
            let mut rbuf = vec![GUARD_BYTE; STATE_SIZE + 8];
            let cp = unsafe { cbuf.as_mut_ptr().add(offset) } as *mut ComputeState;
            let rp = unsafe { rbuf.as_mut_ptr().add(offset) } as *mut ComputeState;

            let (_, co) = capture(|| unsafe { (c.init_state)(cp, seed) });
            let (_, ro) = capture(|| unsafe { (r.init_state)(rp, seed) });
            assert_stdout_eq(
                &format!("H4 iter {i} offset={offset} init_state"),
                &co,
                &ro,
            );
            assert_eq!(
                cbuf, rbuf,
                "H4 iter {i} offset={offset}: init_state bytes diverged (seed={seed})"
            );

            let cf = unsafe { (c.get_operation)(0) };
            let rf = unsafe { (r.get_operation)(0) };
            let (_, co) = capture(|| unsafe { (c.apply_operation)(cp, value, cf) });
            let (_, ro) = capture(|| unsafe { (r.apply_operation)(rp, value, rf) });
            assert_stdout_eq(
                &format!("H4 iter {i} offset={offset} apply_operation"),
                &co,
                &ro,
            );
            assert_eq!(
                cbuf, rbuf,
                "H4 iter {i} offset={offset}: apply_operation bytes diverged \
                 (seed={seed}, value={value})"
            );
        }
    }
}

// ===========================================================================
// H5 — re-entrancy: call the library from inside a callback it invokes
// ===========================================================================

static REENTRANT_RESULT: AtomicI32 = AtomicI32::new(0);
static REENTRANT_LIB: AtomicUsize = AtomicUsize::new(0);

/// Calls `compute_checksum` on whichever library is selected, from inside a
/// callback that the same library is currently executing.
unsafe extern "C" fn reentrant(a: c_int, b: c_int) -> c_int {
    let which = REENTRANT_LIB.load(Ordering::SeqCst);
    let lib = if which == 0 { c_lib() } else { rust_lib() };
    let mut vals = [a, b, a ^ b, a.wrapping_add(b)];
    let sum = unsafe { (lib.compute_checksum)(vals.as_mut_ptr(), 4) };
    REENTRANT_RESULT.store(sum as c_int, Ordering::SeqCst);
    a.wrapping_add(b) ^ sum as c_int
}

#[test]
fn h5_reentrant_call_from_callback() {
    let (c, r) = libs();
    let name = CString::new("REENTRANT").unwrap();
    let mut rng = Rng::new(0xDEAD_0006);

    for i in 0..100 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();

        REENTRANT_LIB.store(0, Ordering::SeqCst);
        let (cv, co) =
            capture(|| unsafe { (c.execute_operation)(Some(reentrant), a, b, name.as_ptr()) });
        let c_inner = REENTRANT_RESULT.load(Ordering::SeqCst);

        REENTRANT_LIB.store(1, Ordering::SeqCst);
        let (rv, ro) =
            capture(|| unsafe { (r.execute_operation)(Some(reentrant), a, b, name.as_ptr()) });
        let r_inner = REENTRANT_RESULT.load(Ordering::SeqCst);

        assert_eq!(c_inner, r_inner, "H5 iter {i}: nested compute_checksum diverged");
        assert_eq!(cv, rv, "H5 iter {i}: outer return value (a={a}, b={b})");
        assert_stdout_eq(&format!("H5 iter {i} a={a} b={b}"), &co, &ro);
    }
}
