// Phase B (continued) -- CONFIGS.md rows 41..45.
//
// The configurations the first pass left out: large POSITIVE allocation sizes
// (where malloc/realloc succeed, the mirror of the failure rows in ERRORS.md),
// several buffers alive at once, and cross-library heap handoff.

mod common;

use common::*;
use std::ffi::c_int;

// ===========================================================================
// CONFIGS.md row 41 -- large positive capacities.
// ===========================================================================
#[test]
fn row41_create_buffer_large_positive_capacities() {
    let (cl, rl) = pair();
    unsafe {
        for cap in [1 << 24, 1 << 28, 1 << 30, i32::MAX - 1, i32::MAX] {
            // Allocate and release one at a time so the two libraries face the
            // same allocator state and cannot disagree for want of memory.
            let cb = (cl.create_buffer)(cap);
            let c_ok = !cb.is_null();
            let (c_cap, c_len, c_first) = if c_ok {
                ((*cb).capacity, (*cb).length, read_n((*cb).data, 1))
            } else {
                (0, 0, vec![])
            };
            (cl.destroy_buffer)(cb);

            let rb = (rl.create_buffer)(cap);
            let r_ok = !rb.is_null();
            let (r_cap, r_len, r_first) = if r_ok {
                ((*rb).capacity, (*rb).length, read_n((*rb).data, 1))
            } else {
                (0, 0, vec![])
            };
            (rl.destroy_buffer)(rb);

            assert_eq!(c_ok, r_ok, "create_buffer({cap}): success differs");
            assert_eq!((c_cap, c_len), (r_cap, r_len), "create_buffer({cap}) fields");
            assert_eq!(c_first, r_first, "create_buffer({cap}) data[0]");
            if c_ok {
                assert_eq!(c_cap, cap, "capacity must be stored verbatim");
                assert_eq!(c_len, 0);
                assert_eq!(c_first, vec![0u8], "data[0] must be NUL");
            }
        }
    }
}

// ===========================================================================
// CONFIGS.md row 42 -- grow branch where new_capacity stays POSITIVE, so
// realloc SUCCEEDS. Mirror of ERRORS.md #6/#7.
// ===========================================================================
#[test]
fn row42_append_large_but_successful_realloc() {
    let (cl, rl) = pair();
    let s = cstring(b"tail-marker");
    let slen = 11 as c_int;

    for length in [1_000_000i32, 50_000_000, 100_000_000, 1_000_000_000] {
        let required = length + slen + 1;
        let new_capacity = required.wrapping_mul(2);
        if new_capacity <= 0 {
            continue; // that is the failure case, covered by ERRORS.md #6/#7
        }
        unsafe {
            // Give the buffer a real allocation big enough that `length` is a
            // legal offset, then force a grow.
            let cb = (cl.create_buffer)(length + 1);
            let rb = (rl.create_buffer)(length + 1);
            if cb.is_null() || rb.is_null() {
                assert_eq!(cb.is_null(), rb.is_null(), "setup nullness differs");
                (cl.destroy_buffer)(cb);
                (rl.destroy_buffer)(rb);
                continue;
            }
            (*cb).length = length;
            (*rb).length = length;
            // Make the byte at `length` a NUL so the buffer is a valid string.
            *(*cb).data.add(length as usize) = 0;
            *(*rb).data.add(length as usize) = 0;

            let cr = (cl.append_to_buffer)(cb, s.as_ptr());
            let rr = (rl.append_to_buffer)(rb, s.as_ptr());
            assert_eq!(cr, rr, "length={length}: return differs");
            assert_eq!(cr, 0, "length={length}: realloc should have succeeded");
            assert_eq!(
                (*cb).capacity,
                (*rb).capacity,
                "length={length}: capacity differs"
            );
            assert_eq!(
                (*cb).capacity, new_capacity,
                "length={length}: capacity must be required*2"
            );
            assert_eq!((*cb).length, (*rb).length, "length={length}: length differs");
            assert_eq!((*cb).length, length + slen);
            // The appended bytes landed at the right offset in both.
            let cbytes = read_n((*cb).data.add(length as usize), slen as usize + 1);
            let rbytes = read_n((*rb).data.add(length as usize), slen as usize + 1);
            assert_eq!(cbytes, rbytes, "length={length}: appended bytes differ");
            assert_eq!(cbytes, b"tail-marker\0".to_vec());

            (*cb).length = 0;
            (*rb).length = 0;
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }
    }
}

// ===========================================================================
// CONFIGS.md row 43 -- many buffers alive at once, interleaved operations.
// ===========================================================================
#[test]
fn row43_many_simultaneous_buffers_interleaved() {
    let (cl, rl) = pair();
    let mut rng = Rng::new();
    const N: usize = 64;
    unsafe {
        let mut cbufs = Vec::with_capacity(N);
        let mut rbufs = Vec::with_capacity(N);
        let mut hwm = vec![1usize; N];

        for i in 0..N {
            let cap = rng.range_i32(1, 96);
            let cb = (cl.create_buffer)(cap);
            let rb = (rl.create_buffer)(cap);
            assert!(!cb.is_null() && !rb.is_null());
            assert_eq!(snapshot(cb), snapshot(rb), "buffer {i} initial state");
            cbufs.push(cb);
            rbufs.push(rb);
        }

        // Round-robin random appends across all live buffers.
        for step in 0..1500 {
            let i = rng.below(N as u64) as usize;
            let s = rng.ascii_below(150);
            let cs = cstring(&s);
            let cr = (cl.append_to_buffer)(cbufs[i], cs.as_ptr());
            let rr = (rl.append_to_buffer)(rbufs[i], cs.as_ptr());
            assert_eq!(cr, rr, "step={step} buf={i}: return differs");
            hwm[i] = hwm[i].max((*cbufs[i]).length as usize + 1);
            assert_eq!(
                snapshot_hwm(cbufs[i], hwm[i]),
                snapshot_hwm(rbufs[i], hwm[i]),
                "step={step} buf={i}: state differs"
            );
            // Every other live buffer must be untouched.
            if step % 100 == 0 {
                for j in 0..N {
                    assert_eq!(
                        snapshot_hwm(cbufs[j], hwm[j]),
                        snapshot_hwm(rbufs[j], hwm[j]),
                        "step={step}: buffer {j} diverged"
                    );
                }
            }
        }

        // Destroy in a shuffled order.
        let mut order: Vec<usize> = (0..N).collect();
        for i in (1..N).rev() {
            let j = rng.below((i + 1) as u64) as usize;
            order.swap(i, j);
        }
        for i in order {
            (cl.destroy_buffer)(cbufs[i]);
            (rl.destroy_buffer)(rbufs[i]);
        }
    }
}

// ===========================================================================
// CONFIGS.md row 44 -- cross-library handoff. A buffer created by one library
// is grown and freed by the other. This only works if both really use the same
// libc allocator, which is exactly what the translation claims.
// ===========================================================================
#[test]
fn row44_cross_library_buffer_handoff() {
    let (cl, rl) = pair();
    let mut rng = Rng::new();
    unsafe {
        for trial in 0..200 {
            // C creates, Rust grows and frees.
            {
                let b = (cl.create_buffer)(rng.range_i32(1, 32));
                assert!(!b.is_null());
                let mut hwm = 1usize;
                for _ in 0..12 {
                    let s = rng.ascii_below(120);
                    let cs = cstring(&s);
                    assert_eq!(
                        (rl.append_to_buffer)(b, cs.as_ptr()),
                        0,
                        "trial={trial}: Rust append on a C-created buffer failed"
                    );
                    hwm = hwm.max((*b).length as usize + 1);
                    assert!((*b).length < (*b).capacity);
                }
                (rl.destroy_buffer)(b);
            }
            // Rust creates, C grows and frees.
            {
                let b = (rl.create_buffer)(rng.range_i32(1, 32));
                assert!(!b.is_null());
                for _ in 0..12 {
                    let s = rng.ascii_below(120);
                    let cs = cstring(&s);
                    assert_eq!(
                        (cl.append_to_buffer)(b, cs.as_ptr()),
                        0,
                        "trial={trial}: C append on a Rust-created buffer failed"
                    );
                    assert!((*b).length < (*b).capacity);
                }
                (cl.destroy_buffer)(b);
            }
        }

        // And the states must agree when the same script is replayed with the
        // roles swapped: C-create/Rust-append vs Rust-create/C-append.
        for trial in 0..200 {
            let cap = Rng::with_seed(SEED ^ trial).range_i32(1, 64);
            let mut r1 = Rng::with_seed(SEED ^ (trial << 8));
            let mut r2 = Rng::with_seed(SEED ^ (trial << 8));

            let a = (cl.create_buffer)(cap); // created by C, appended by Rust
            let b = (rl.create_buffer)(cap); // created by Rust, appended by C
            assert!(!a.is_null() && !b.is_null());
            let mut hwm = 1usize;
            for step in 0..20 {
                let sa = r1.ascii_below(100);
                let sb = r2.ascii_below(100);
                assert_eq!(sa, sb, "PRNG desync");
                let cs = cstring(&sa);
                let ra = (rl.append_to_buffer)(a, cs.as_ptr());
                let rb = (cl.append_to_buffer)(b, cs.as_ptr());
                assert_eq!(ra, rb, "trial={trial} step={step}: return differs");
                hwm = hwm.max((*a).length as usize + 1);
                assert_eq!(
                    snapshot_hwm(a, hwm),
                    snapshot_hwm(b, hwm),
                    "trial={trial} step={step}: cross-wired state differs"
                );
            }
            (rl.destroy_buffer)(a);
            (cl.destroy_buffer)(b);
        }
    }
}

// ===========================================================================
// CONFIGS.md row 45 -- repeated buffapp calls leave no residual state.
// ===========================================================================
#[test]
fn row45_buffapp_repeated_calls_are_stateless() {
    let (cl, rl) = pair();
    let p = (13, 7, 22, 5);
    unsafe {
        // The same input must give the same answer and the same bytes every time.
        let (cv0, cout0) = capture_stdout(|| (cl.buffapp)(p.0, p.1, p.2, p.3));
        let (rv0, rout0) = capture_stdout(|| (rl.buffapp)(p.0, p.1, p.2, p.3));
        assert_eq!(cv0, rv0);
        assert_eq!(cout0, rout0);

        for i in 0..200 {
            let (cv, cout) = capture_stdout(|| (cl.buffapp)(p.0, p.1, p.2, p.3));
            let (rv, rout) = capture_stdout(|| (rl.buffapp)(p.0, p.1, p.2, p.3));
            assert_eq!(cv, cv0, "C buffapp not stateless at iteration {i}");
            assert_eq!(rv, rv0, "Rust buffapp not stateless at iteration {i}");
            assert_eq!(cout, cout0, "C stdout changed at iteration {i}");
            assert_eq!(rout, rout0, "Rust stdout changed at iteration {i}");
            assert_eq!(cv, rv);
            assert_eq!(cout, rout);
        }

        // Interleave with other entry points to be sure nothing bleeds through.
        let mut rng = Rng::new();
        for _ in 0..300 {
            let q = (
                rng.spicy_i32(),
                rng.spicy_i32(),
                rng.spicy_i32(),
                rng.spicy_i32(),
            );
            // Skip the one trapping class (ERRORS.md #24).
            let i1 = {
                let op = (cl.get_operation_name)(q.0.wrapping_rem(4));
                (cl.perform_operation)(q.0, q.1, op)
            };
            let i2 = {
                let op = (cl.get_operation_name)(q.2.wrapping_rem(4));
                (cl.perform_operation)(q.2, q.3, op)
            };
            if i1.wrapping_mul(i2) == -1 && i1.wrapping_add(i2) == i32::MIN {
                continue;
            }
            let cb = (cl.create_buffer)(8);
            let rb = (rl.create_buffer)(8);
            let s = cstring(b"noise");
            (cl.append_to_buffer)(cb, s.as_ptr());
            (rl.append_to_buffer)(rb, s.as_ptr());

            let (cv, cout) = capture_stdout(|| (cl.buffapp)(q.0, q.1, q.2, q.3));
            let (rv, rout) = capture_stdout(|| (rl.buffapp)(q.0, q.1, q.2, q.3));
            assert_eq!(cv, rv, "buffapp{q:?} after interleaved work");
            assert_eq!(cout, rout, "buffapp{q:?} stdout after interleaved work");

            assert_eq!(snapshot(cb), snapshot(rb), "side buffer disturbed");
            (cl.destroy_buffer)(cb);
            (rl.destroy_buffer)(rb);
        }

        // And the repeated fixed input still matches after all that.
        let (cv, cout) = capture_stdout(|| (cl.buffapp)(p.0, p.1, p.2, p.3));
        assert_eq!(cv, cv0);
        assert_eq!(cout, cout0);
    }
}
