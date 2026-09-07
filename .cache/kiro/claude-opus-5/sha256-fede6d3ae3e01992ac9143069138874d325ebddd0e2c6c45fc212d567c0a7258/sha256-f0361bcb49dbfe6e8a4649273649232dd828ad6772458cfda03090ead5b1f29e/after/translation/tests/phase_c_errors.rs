//! Phase C — error-path / boundary differential tests, one test per `ERRORS.md`
//! row. Every test asserts C and Rust agree on the *specific* result (exact
//! wrapped value, exact returned-pointer class, exact termination signal), never
//! merely that "both failed".

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// Rows 1–3, 18 — invalid pointers. Compared via forked children so the exact
// termination signal can be observed on both sides.
// ---------------------------------------------------------------------------

/// Row 1 — `outer == NULL`: `*outer` dereferences the null page.
///
/// The C artifact dies with SIGSEGV. The Rust release cdylib must do the same;
/// a debug build may instead trip `core`'s optional UB precondition assertion,
/// which is proven from the child's stderr rather than assumed (see
/// `assert_same_fault`).
#[test]
fn err_row01_null_pointer_segv() {
    let _g = gate();
    let p = Pair::fresh();
    let c = run_in_child_capturing_stderr(|| {
        let _ = unsafe { (p.c.sa)(std::ptr::null_mut()) };
    });
    let r = run_in_child_capturing_stderr(|| {
        let _ = unsafe { (p.r.sa)(std::ptr::null_mut()) };
    });
    assert_eq!(
        c.0,
        Outcome::Signal(libc::SIGSEGV),
        "expected SIGSEGV from the C null dereference, got {:?}",
        c.0
    );
    assert_same_fault("row01 null pointer", c, r);
}

/// Row 2 — non-null but unmapped address.
#[test]
fn err_row02_wild_pointer_segv() {
    let _g = gate();
    let p = Pair::fresh();
    for wild in [1usize, 3, 0x10, 0xdead_beef, usize::MAX & !3] {
        let c = run_in_child_capturing_stderr(|| {
            let _ = unsafe { (p.c.sa)(wild as *mut c_int) };
        });
        let r = run_in_child_capturing_stderr(|| {
            let _ = unsafe { (p.r.sa)(wild as *mut c_int) };
        });
        assert!(
            matches!(
                c.0,
                Outcome::Signal(libc::SIGSEGV) | Outcome::Signal(libc::SIGBUS)
            ),
            "wild pointer {wild:#x}: expected SIGSEGV/SIGBUS from C, got {:?}",
            c.0
        );
        assert_same_fault(&format!("row02 wild pointer {wild:#x}"), c, r);
    }
}

/// Map one page, write `value`, then make it read-only. Leaked deliberately: the
/// mapping must outlive the forked child.
fn readonly_page_with(value: c_int) -> *mut c_int {
    unsafe {
        let len = 4096;
        let p = libc::mmap(
            std::ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(p, libc::MAP_FAILED, "mmap failed");
        *(p as *mut c_int) = value;
        assert_eq!(libc::mprotect(p, len, libc::PROT_READ), 0, "mprotect failed");
        p as *mut c_int
    }
}

/// Row 3 — `outer` points into a read-only mapping and the **else**-branch runs,
/// so the `*outer += inner` store faults. (The read in `*outer >= inner`
/// succeeds, so this specifically exercises the store, not the load.)
#[test]
fn err_row03_readonly_store_segv() {
    let _g = gate();
    let p = Pair::fresh();
    // inner == 1 on a fresh library, so a value of 0 takes the else-branch.
    let ro = readonly_page_with(0);
    let c = run_in_child_capturing_stderr(|| {
        let _ = unsafe { (p.c.sa)(ro) };
    });
    let r = run_in_child_capturing_stderr(|| {
        let _ = unsafe { (p.r.sa)(ro) };
    });
    assert_eq!(
        c.0,
        Outcome::Signal(libc::SIGSEGV),
        "expected SIGSEGV writing a PROT_READ page, got {:?}",
        c.0
    );
    assert_same_fault("row03 read-only store", c, r);

    // Control: with a value that takes the *then*-branch, nothing is stored
    // through `outer`, so both must survive and agree.
    let ro2 = readonly_page_with(5);
    let oc2 = run_in_child(|| {
        let _ = unsafe { (p.c.sa)(ro2) };
    });
    let or2 = run_in_child(|| {
        let _ = unsafe { (p.r.sa)(ro2) };
    });
    assert_eq!(oc2, or2, "read-only then-branch outcome differs");
    assert_eq!(oc2, Outcome::Exit(0), "then-branch should not fault: {oc2:?}");
    // And the value/pointer behaviour must match in-process too.
    let a = p.c.call_raw(ro2);
    let b = p.r.call_raw(ro2);
    assert_eq!(a, b, "read-only then-branch observation differs");
}

/// Row 18 — misaligned `int*` (UB per C; x86-64 tolerates it). The harness reads
/// through the pointer with `read_unaligned`, so only the *library's* behaviour is
/// under test.
#[test]
fn err_row18_misaligned_pointer() {
    let _g = gate();
    for off in [1usize, 2, 3, 5, 6, 7] {
        for v in [0i32, 1, -1, 7, i32::MAX, i32::MIN] {
            let p = Pair::fresh();
            let mut buf_c = [0u8; 16];
            let mut buf_r = [0u8; 16];
            buf_c[off..off + 4].copy_from_slice(&v.to_ne_bytes());
            buf_r[off..off + 4].copy_from_slice(&v.to_ne_bytes());
            let a = p.c.call_raw(unsafe { buf_c.as_mut_ptr().add(off) } as *mut c_int);
            let b = p.r.call_raw(unsafe { buf_r.as_mut_ptr().add(off) } as *mut c_int);
            assert_eq!(a, b, "misaligned off={off} v={v}: C={a:?} Rust={b:?}");
            assert_eq!(
                buf_c, buf_r,
                "misaligned off={off} v={v}: caller buffers differ"
            );
            let (pc, pr) = (p.c.probe_inner(), p.r.probe_inner());
            assert_eq!(pc, pr, "misaligned off={off} v={v}: `inner` differs");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 4–11 — integer-overflow and predicate-boundary conditions.
// ---------------------------------------------------------------------------

/// Row 4 — then-branch overflow: `inner == 1`, `*outer == INT_MAX`.
#[test]
fn err_row04_then_overflow_intmax() {
    let _g = gate();
    let p = Pair::fresh();
    let o = p.both("row04 inner=1 + INT_MAX", |l| vec![l.call_local(i32::MAX)]);
    // Pin the exact expected C behaviour: wrap to INT_MIN, return `&inner`.
    assert!(
        !o[0].ret_is_arg && o[0].ret_is_internal,
        "row04 must return &inner: {:?}",
        o[0]
    );
    assert_eq!(o[0].ret_val, i32::MIN, "1 + INT_MAX should wrap to INT_MIN");
    assert_eq!(Lib::inner_from_probe(p.c.probe_inner()), i32::MIN);
    assert_eq!(Lib::inner_from_probe(p.r.probe_inner()), i32::MIN);
}

/// Row 5 — then-branch overflow with `outer` **aliasing** `inner`, both `INT_MAX`.
#[test]
fn err_row05_then_overflow_aliased() {
    let _g = gate();
    let p = Pair::fresh();
    // inner: 1 -> 1 + (INT_MAX-1) = INT_MAX, and `&inner` is now known.
    let w = p.both("row05 raise inner to INT_MAX", |l| {
        vec![l.call_local(i32::MAX - 1)]
    });
    assert_eq!(w[0].ret_val, i32::MAX, "inner should be INT_MAX");
    let o = p.both("row05 aliased INT_MAX + INT_MAX", |l| vec![l.call_aliased()]);
    assert_eq!(o[0].ret_val, -2, "INT_MAX + INT_MAX should wrap to -2");
    assert!(o[0].ret_is_internal, "row05 must return &inner: {:?}", o[0]);
    assert_eq!(Lib::inner_from_probe(p.c.probe_inner()), -2);
    assert_eq!(Lib::inner_from_probe(p.r.probe_inner()), -2);
}

/// Row 6 — then-branch underflow: `inner == INT_MIN`, `*outer == INT_MIN`.
#[test]
fn err_row06_then_underflow_intmin() {
    let _g = gate();
    let p = Pair::fresh();
    // 1 + INT_MAX wraps inner to INT_MIN (row 4).
    let w = p.both("row06 drive inner to INT_MIN", |l| {
        vec![l.call_local(i32::MAX)]
    });
    assert_eq!(w[0].ret_val, i32::MIN);
    // Now `INT_MIN >= INT_MIN` takes the then-branch: INT_MIN + INT_MIN -> 0.
    let o = p.both("row06 INT_MIN + INT_MIN", |l| vec![l.call_local(i32::MIN)]);
    assert!(!o[0].ret_is_arg, "row06 then-branch expected: {:?}", o[0]);
    assert_eq!(o[0].ret_val, 0, "INT_MIN + INT_MIN should wrap to 0");
    assert_eq!(Lib::inner_from_probe(p.c.probe_inner()), 0);
    assert_eq!(Lib::inner_from_probe(p.r.probe_inner()), 0);
}

/// Row 7 — else-branch overflow: `inner == INT_MAX`, `*outer == 1`.
#[test]
fn err_row07_else_overflow() {
    let _g = gate();
    let p = Pair::fresh();
    let w = p.both("row07 raise inner to INT_MAX", |l| {
        vec![l.call_local(i32::MAX - 1)]
    });
    assert_eq!(w[0].ret_val, i32::MAX);
    let o = p.both("row07 else 1 + INT_MAX", |l| vec![l.call_local(1)]);
    assert!(o[0].ret_is_arg, "else-branch must return `outer`: {:?}", o[0]);
    assert_eq!(o[0].ret_val, i32::MIN, "1 + INT_MAX should wrap to INT_MIN");
    assert_eq!(o[0].arg_val, i32::MIN, "the caller's object must be updated");
    // `inner` must be untouched by the else-branch.
    assert_eq!(Lib::inner_from_probe(p.c.probe_inner()), i32::MAX);
    assert_eq!(Lib::inner_from_probe(p.r.probe_inner()), i32::MAX);
}

/// Row 8 — else-branch underflow: negative `inner`, more-negative `*outer`.
#[test]
fn err_row08_else_underflow() {
    let _g = gate();
    let p = Pair::fresh();
    // Step 1: `1 + INT_MAX` wraps `inner` to INT_MIN (then-branch).
    assert_eq!(
        p.both("row08 inner -> INT_MIN", |l| vec![l.call_local(i32::MAX)])[0].ret_val,
        i32::MIN
    );
    assert_eq!(Lib::inner_from_probe(p.c.probe_inner()), i32::MIN);
    // Step 2: `1 >= INT_MIN`, so the then-branch makes `inner == INT_MIN + 1`
    // (negative, but not INT_MIN, so something can still be `< inner`).
    let w = p.both("row08 inner -> INT_MIN+1", |l| vec![l.call_local(1)]);
    assert_eq!(w[0].ret_val, i32::MIN + 1);
    assert_eq!(Lib::inner_from_probe(p.c.probe_inner()), i32::MIN + 1);
    assert_eq!(Lib::inner_from_probe(p.r.probe_inner()), i32::MIN + 1);
    // Step 3: `INT_MIN < INT_MIN + 1`, so the else-branch runs and
    // `*outer += inner` underflows: INT_MIN + (INT_MIN+1) wraps to 1.
    let o = p.both("row08 else INT_MIN + inner", |l| {
        vec![l.call_local(i32::MIN)]
    });
    assert!(o[0].ret_is_arg, "else-branch expected: {:?}", o[0]);
    assert_eq!(
        o[0].ret_val,
        i32::MIN.wrapping_add(i32::MIN + 1),
        "INT_MIN + (INT_MIN+1) should wrap"
    );
    assert_eq!(o[0].ret_val, 1, "…specifically to 1");
    assert_eq!(o[0].arg_val, 1, "the caller's object must be updated");
    // The else-branch must leave `inner` untouched.
    assert_eq!(Lib::inner_from_probe(p.c.probe_inner()), i32::MIN + 1);
    assert_eq!(Lib::inner_from_probe(p.r.probe_inner()), i32::MIN + 1);
}

/// Row 9 — predicate boundary, one step below: `*outer == inner - 1` ⇒ else-branch.
#[test]
fn err_row09_predicate_one_below() {
    let _g = gate();
    for warm in [1i32, 2, 100, 1 << 16, i32::MAX - 1] {
        let p = Pair::fresh();
        let oc = p.c.call_local(warm);
        let or = p.r.call_local(warm);
        assert_eq!(oc, or, "row09 warm={warm} warm-up diverged");
        let inner = oc.ret_val;
        let v = inner.wrapping_sub(1);
        let a = p.c.call_local(v);
        let b = p.r.call_local(v);
        assert_eq!(a, b, "row09 inner={inner} v=inner-1={v}");
        assert!(
            a.ret_is_arg,
            "row09: `*outer == inner-1` must take the else-branch, got {a:?}"
        );
        assert_eq!(a.ret_val, v.wrapping_add(inner));
    }
}

/// Row 10 — predicate boundary, exactly equal: `*outer == inner` ⇒ then-branch.
#[test]
fn err_row10_predicate_equal() {
    let _g = gate();
    for warm in [1i32, 2, 100, 1 << 16, i32::MAX - 1] {
        let p = Pair::fresh();
        let oc = p.c.call_local(warm);
        let or = p.r.call_local(warm);
        assert_eq!(oc, or, "row10 warm={warm} warm-up diverged");
        let inner = oc.ret_val;
        let a = p.c.call_local(inner);
        let b = p.r.call_local(inner);
        assert_eq!(a, b, "row10 inner={inner} v==inner");
        assert!(
            !a.ret_is_arg,
            "row10: `*outer == inner` must take the then-branch, got {a:?}"
        );
        assert_eq!(a.ret_val, inner.wrapping_add(inner));
    }
}

/// Row 11 — `*outer == INT_MIN` with `inner == 1`: guaranteed else-branch.
#[test]
fn err_row11_intmin_else() {
    let _g = gate();
    let p = Pair::fresh();
    let a = p.c.call_local(i32::MIN);
    let b = p.r.call_local(i32::MIN);
    assert_eq!(a, b, "row11: C={a:?} Rust={b:?}");
    assert!(a.ret_is_arg, "row11 must return `outer`: {a:?}");
    assert_eq!(a.ret_val, i32::MIN + 1);
    assert_eq!(a.arg_val, i32::MIN + 1);
}

// ---------------------------------------------------------------------------
// Rows 12–17 — `driver` boundaries.
// ---------------------------------------------------------------------------

/// Row 12 — `iterations == 0`: returns immediately, zero output, `inner` untouched.
#[test]
fn err_row12_iterations_zero() {
    let _g = gate();
    let p = Pair::fresh();
    for v in [i32::MIN, -1, 0, 1, 42, i32::MAX] {
        let oc = capture_stdout(|| p.c.driver(v, 0));
        let or = capture_stdout(|| p.r.driver(v, 0));
        assert_eq!(oc, or, "row12 v={v}: stdout differs");
        assert!(oc.is_empty(), "row12 v={v}: expected zero bytes, got {oc:?}");
    }
    // `inner` must still be 1 on both.
    assert_eq!(Lib::inner_from_probe(p.c.probe_inner()), 1);
    assert_eq!(Lib::inner_from_probe(p.r.probe_inner()), 1);
}

/// Row 13 — `iterations < 0`: returns immediately, zero output.
#[test]
fn err_row13_iterations_negative() {
    let _g = gate();
    let p = Pair::fresh();
    for n in [-1i32, -2, -100, -(1 << 20), i32::MIN + 1] {
        for v in [i32::MIN, 0, 1, i32::MAX] {
            let oc = capture_stdout(|| p.c.driver(v, n));
            let or = capture_stdout(|| p.r.driver(v, n));
            assert_eq!(oc, or, "row13 v={v} n={n}: stdout differs");
            assert!(oc.is_empty(), "row13 v={v} n={n}: expected zero bytes");
        }
    }
    assert_eq!(Lib::inner_from_probe(p.c.probe_inner()), 1);
    assert_eq!(Lib::inner_from_probe(p.r.probe_inner()), 1);
}

/// Row 14 — `iterations == INT_MIN`, one step past every valid count.
#[test]
fn err_row14_iterations_intmin() {
    let _g = gate();
    let p = Pair::fresh();
    for v in [i32::MIN, -1, 0, 1, i32::MAX] {
        let oc = capture_stdout(|| p.c.driver(v, i32::MIN));
        let or = capture_stdout(|| p.r.driver(v, i32::MIN));
        assert_eq!(oc, or, "row14 v={v}: stdout differs");
        assert!(oc.is_empty(), "row14 v={v}: expected zero bytes, got {oc:?}");
    }
    assert_eq!(Lib::inner_from_probe(p.c.probe_inner()), 1);
    assert_eq!(Lib::inner_from_probe(p.r.probe_inner()), 1);
}

/// Row 15 — `iterations == INT_MAX` is not runnable in bounded time. Documented in
/// `ERRORS.md`; here we assert the *approach* to it stays identical for the
/// largest counts that do fit in the budget, so the loop-counter arithmetic
/// (`i < iterations`, `i++` / `wrapping_add(1)`) is covered.
#[test]
fn err_row15_large_iteration_counts() {
    let _g = gate();
    for n in [1i32, 2, 255, 256, 1000, 4096, 65536] {
        let p = Pair::fresh();
        p.both_driver(&format!("row15 n={n}"), 1, n);
    }
}

/// Row 16 — `initial_value == INT_MAX` driven through the wrapper (row-4 overflow).
#[test]
fn err_row16_driver_overflow_intmax() {
    let _g = gate();
    for n in [1i32, 2, 3, 8, 64] {
        let p = Pair::fresh();
        p.both_driver(&format!("row16 INT_MAX n={n}"), i32::MAX, n);
    }
    // Pin the first printed line: 1 + INT_MAX wraps to INT_MIN.
    let p = Pair::fresh();
    let out = capture_stdout(|| p.c.driver(i32::MAX, 1));
    assert_eq!(
        out,
        format!("{}\n", i32::MIN).into_bytes(),
        "row16: unexpected C output"
    );
    let out_r = capture_stdout(|| p.r.driver(i32::MAX, 1));
    assert_eq!(out, out_r);
}

/// Row 17 — `initial_value == INT_MIN` driven through the wrapper.
#[test]
fn err_row17_driver_intmin() {
    let _g = gate();
    for n in [1i32, 2, 3, 8, 64, 1024] {
        let p = Pair::fresh();
        p.both_driver(&format!("row17 INT_MIN n={n}"), i32::MIN, n);
    }
    // Pin the first two printed lines: INT_MIN < 1 ⇒ else-branch ⇒ INT_MIN+1;
    // then INT_MIN+1 < 1 ⇒ else again ⇒ INT_MIN+2.
    let p = Pair::fresh();
    let out = capture_stdout(|| p.c.driver(i32::MIN, 2));
    assert_eq!(
        out,
        format!("{}\n{}\n", i32::MIN + 1, i32::MIN + 2).into_bytes(),
        "row17: unexpected C output"
    );
    let out_r = capture_stdout(|| p.r.driver(i32::MIN, 2));
    assert_eq!(out, out_r);
}

// ---------------------------------------------------------------------------
// Generic C-API boundaries beyond the table
// ---------------------------------------------------------------------------

/// `static_alias` has no enum parameter, so the FFI analogue of "an out-of-range
/// enum value" is an arbitrary `int` bit pattern. Sweep every byte-pattern class
/// exhaustively over a wide set of values on a fresh library each time, and also
/// as a second call after a randomized first call.
#[test]
fn err_generic_full_int_domain_sweep() {
    let _g = gate();
    let mut vals: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        -65537,
        -65536,
        -65535,
        -257,
        -256,
        -255,
        -2,
        -1,
        0,
        1,
        2,
        255,
        256,
        257,
        65535,
        65536,
        65537,
        i32::MAX / 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    // Every single-bit and single-bit-complement pattern too.
    for b in 0..32 {
        vals.push(1i32.wrapping_shl(b));
        vals.push(!(1i32.wrapping_shl(b)));
    }
    for &v in &vals {
        let p = Pair::fresh();
        p.both(&format!("sweep v={v}"), |l| vec![l.call_local(v)]);
    }
    // Second-call variants: same sweep after a fixed first call, so `inner != 1`.
    for &v in &vals {
        let p = Pair::fresh();
        p.both(&format!("sweep after-warm v={v}"), |l| {
            vec![l.call_local(12345), l.call_local(v)]
        });
    }
}

/// `driver`'s `iterations` domain sweep (the "length" parameter of this API):
/// zero, negative, one, and oversized-but-runnable counts, crossed with extreme
/// `initial_value`s.
#[test]
fn err_generic_iterations_domain_sweep() {
    let _g = gate();
    for n in [i32::MIN, i32::MIN + 1, -1000, -2, -1, 0, 1, 2, 3, 100, 1000] {
        for v in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
            let p = Pair::fresh();
            p.both_driver(&format!("iter-sweep v={v} n={n}"), v, n);
        }
    }
}

/// `static_alias` never returns NULL for any valid input — assert the sentinel
/// really is absent from the C behaviour, and that Rust agrees. (`call_raw`
/// already asserts non-NULL; this makes the claim explicit and exhaustive over
/// the extremes and over both branches.)
#[test]
fn err_generic_return_is_never_null() {
    let _g = gate();
    for v in [i32::MIN, -1, 0, 1, i32::MAX] {
        let p = Pair::fresh();
        let mut lc: c_int = v;
        let mut lr: c_int = v;
        let rc = unsafe { (p.c.sa)(&mut lc) };
        let rr = unsafe { (p.r.sa)(&mut lr) };
        assert!(!rc.is_null(), "C returned NULL for v={v}");
        assert!(!rr.is_null(), "Rust returned NULL for v={v}");
        assert_eq!(rc == (&mut lc as *mut c_int), rr == (&mut lr as *mut c_int));
        assert_eq!(unsafe { *rc }, unsafe { *rr });
        assert_eq!(lc, lr);
    }
}

/// The returned pointer must be *usable* by the caller exactly as in C: writing
/// through it and calling again must behave identically. (This is what a real
/// consumer would do with `&inner`.)
#[test]
fn err_generic_write_through_returned_pointer() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 0xBEEF);
    for k in 0..50 {
        let p = Pair::fresh();
        let warm = rng.i32_in(1, i32::MAX);
        let oc = p.c.call_local(warm);
        let or = p.r.call_local(warm);
        assert_eq!(oc, or, "write-through[{k}] warm-up diverged");
        // Both libraries have revealed &inner; scribble the same value into it.
        let w = rng.i32_any();
        let (pc, pr) = (p.c.internal_ptr(), p.r.internal_ptr());
        assert!(!pc.is_null() && !pr.is_null());
        unsafe {
            *pc = w;
            *pr = w;
        }
        let v = rng.i32_any();
        p.both(&format!("write-through[{k}] inner:={w} then v={v}"), |l| {
            vec![l.call_local(v)]
        });
    }
}

/// Calling with a pointer to `mmap`ed (heap-external) memory rather than a stack
/// local — a different mapping class, same required behaviour.
#[test]
fn err_generic_mmapped_argument() {
    let _g = gate();
    let mut rng = Rng::new(SEED ^ 0xF00D);
    for k in 0..50 {
        let p = Pair::fresh();
        let v = rng.i32_any();
        let a = unsafe {
            let m = libc::mmap(
                std::ptr::null_mut(),
                4096,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            assert_ne!(m, libc::MAP_FAILED);
            *(m as *mut c_int) = v;
            let o = p.c.call_raw(m as *mut c_int);
            libc::munmap(m, 4096);
            o
        };
        let b = unsafe {
            let m = libc::mmap(
                std::ptr::null_mut(),
                4096,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            assert_ne!(m, libc::MAP_FAILED);
            *(m as *mut c_int) = v;
            let o = p.r.call_raw(m as *mut c_int);
            libc::munmap(m, 4096);
            o
        };
        assert_eq!(a, b, "mmapped-arg[{k}] v={v}: C={a:?} Rust={b:?}");
        let _ = std::ptr::null_mut::<c_void>();
    }
}
