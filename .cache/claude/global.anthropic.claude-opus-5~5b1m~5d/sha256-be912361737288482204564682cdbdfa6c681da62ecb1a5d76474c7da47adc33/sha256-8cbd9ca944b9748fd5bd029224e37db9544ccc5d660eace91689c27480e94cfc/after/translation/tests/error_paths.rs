// Phase C — error-path differential tests. One test per row of ERRORS.md
// (E1..E9) plus the generic FFI boundary cases (null, misalignment, partial
// out-of-bounds reads, out-of-range "enum" ints).
//
// The library has no error return channel (every function is `void`), so an
// "error" manifests either as a fatal signal or as silently-accepted output.
// Fatal-signal rows are run in a forked child so the exact WTERMSIG can be
// compared between C and Rust.

mod common;

use common::*;

fn c_print() -> FnPtrArg {
    c_fn(b"printIntPtrLine\0")
}
fn rs_print() -> FnPtrArg {
    rs_fn(b"printIntPtrLine\0")
}
fn c_driver() -> FnIntArg {
    c_fn(b"driver\0")
}
fn rs_driver() -> FnIntArg {
    rs_fn(b"driver\0")
}

/// Assert C and Rust die (or survive) identically when handed `p`.
fn cmp_fatal(p: *const libc::c_int, ctx: &str) -> (ChildOutcome, ChildOutcome) {
    let (cf, rf) = (c_print(), rs_print());
    let addr = p as usize;
    let c = run_in_child(|| unsafe { cf(addr as *const libc::c_int) });
    let r = run_in_child(|| unsafe { rf(addr as *const libc::c_int) });
    assert_eq!(
        (c.signal, c.exit),
        (r.signal, r.exit),
        "{ctx}: termination status differs: C={c:?} RUST={r:?}"
    );
    assert_eq!(
        c.stdout, r.stdout,
        "{ctx}: bytes written before dying differ: C={:?} RUST={:?}",
        show(&c.stdout),
        show(&r.stdout)
    );
    (c, r)
}

// ------------------------------------------------------------------ E1
// printIntPtrLine(NULL) — no null check in the C, so it faults.
#[test]
fn err_e1_null_ptr_segv() {
    let (c, _r) = cmp_fatal(std::ptr::null(), "E1 NULL");
    assert_eq!(
        c.signal,
        Some(libc::SIGSEGV),
        "E1 expected SIGSEGV from C, got {c:?}"
    );
    assert!(c.stdout.is_empty(), "E1 nothing should be printed");
}

// ------------------------------------------------------------------ E2
// Non-null wild / unmapped addresses.
#[test]
fn err_e2_wild_ptr_segv() {
    for addr in [
        0x1usize,
        0x4,
        0xdead_beef_000,
        0x7fff_ffff_f000,
        usize::MAX & !0x7,
    ] {
        let (c, _r) = cmp_fatal(addr as *const libc::c_int, &format!("E2 addr={addr:#x}"));
        assert_eq!(
            c.signal,
            Some(libc::SIGSEGV),
            "E2 {addr:#x}: expected SIGSEGV, got {c:?}"
        );
    }
}

// ------------------------------------------------------------------ E3
// Misaligned but fully mapped: x86-64 tolerates it; both must print the same
// little-endian int and must NOT crash.
#[test]
fn err_e3_misaligned_ptr() {
    unsafe {
        let len = 4096;
        let base = libc::mmap(
            std::ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(base, libc::MAP_FAILED);
        let bytes = base as *mut u8;
        let mut rng = Rng::new(0xE3_5EED);
        for i in 0..len {
            *bytes.add(i) = rng.next_u8();
        }
        let (cf, rf) = (c_print(), rs_print());
        for off in [1usize, 2, 3, 9, 17, 4090] {
            let p = bytes.add(off) as *const libc::c_int;
            let _g = stdout_lock();
            let c_out = capture(|| cf(p));
            let r_out = capture(|| rf(p));
            assert_eq!(
                c_out,
                r_out,
                "E3 off={off}: C={:?} RUST={:?}",
                show(&c_out),
                show(&r_out)
            );
            let expected = std::ptr::read_unaligned(p);
            assert_eq!(c_out, format!("{expected}\n").into_bytes());
        }
        libc::munmap(base, len);
    }
}

// ------------------------------------------------------------------ E4
// Partial out-of-bounds: the last 1..3 bytes of a mapped page whose successor
// page is unmapped, so the 4-byte read straddles into nothing.
#[test]
fn err_e4_page_edge_segv() {
    unsafe {
        let page = 4096usize;
        // Reserve two pages, then drop the second so the first is a hard edge.
        let base = libc::mmap(
            std::ptr::null_mut(),
            page * 2,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(base, libc::MAP_FAILED);
        libc::munmap((base as *mut u8).add(page) as *mut libc::c_void, page);
        let bytes = base as *mut u8;
        for i in 0..page {
            *bytes.add(i) = 0xAB;
        }
        for tail in [1usize, 2, 3] {
            let p = bytes.add(page - tail) as *const libc::c_int;
            let (c, _r) = cmp_fatal(p, &format!("E4 tail={tail}"));
            assert_eq!(
                c.signal,
                Some(libc::SIGSEGV),
                "E4 tail={tail}: expected SIGSEGV, got {c:?}"
            );
        }
        // Control: fully inside the page => no fault, identical output.
        let p = bytes.add(page - 4) as *const libc::c_int;
        let (cf, rf) = (c_print(), rs_print());
        let _g = stdout_lock();
        let c_out = capture(|| cf(p));
        let r_out = capture(|| rf(p));
        assert_eq!(c_out, r_out, "E4 control differs");
        assert!(!c_out.is_empty());
        libc::munmap(base, page);
    }
}

// ------------------------------------------------------------------ E5
// Boundary `int` values are accepted with no range check.
#[test]
fn err_e5_int_boundaries() {
    let (cf, rf) = (c_print(), rs_print());
    for &v in INT_BOUNDARIES {
        let slot: libc::c_int = v;
        let p = &slot as *const libc::c_int;
        let _g = stdout_lock();
        let c_out = capture(|| unsafe { cf(p) });
        let r_out = capture(|| unsafe { rf(p) });
        assert_eq!(c_out, r_out, "E5 v={v} differs");
        assert_eq!(c_out, format!("{v}\n").into_bytes(), "E5 v={v} shape");
    }
    // Explicitly pin the asymmetric INT_MIN rendering.
    let slot: libc::c_int = i32::MIN;
    let p = &slot as *const libc::c_int;
    let _g = stdout_lock();
    assert_eq!(
        capture(|| unsafe { cf(p) }),
        b"-2147483648\n".to_vec(),
        "E5 INT_MIN"
    );
    assert_eq!(
        capture(|| unsafe { rf(p) }),
        b"-2147483648\n".to_vec(),
        "E5 INT_MIN (rust)"
    );
}

// ------------------------------------------------------------------ E6
// bad() is CWE-457. There is no error code to compare, so compare the
// observable outcome class + termination status across many independent
// children (stability of the outcome is itself the property under test).
#[test]
fn err_e6_bad_is_ub_no_error_code() {
    let cb: FnVoid = c_fn(b"bad\0");
    let rb: FnVoid = rs_fn(b"bad\0");
    let mut c_outcomes = Vec::new();
    let mut r_outcomes = Vec::new();
    for _ in 0..8 {
        c_outcomes.push(run_in_child(|| unsafe { cb() }));
        r_outcomes.push(run_in_child(|| unsafe { rb() }));
    }
    let c_crash = c_outcomes.iter().filter(|o| o.crashed()).count();
    let r_crash = r_outcomes.iter().filter(|o| o.crashed()).count();
    eprintln!(
        "E6: C crashed {c_crash}/8 {:?}; RUST crashed {r_crash}/8 {:?}",
        c_outcomes[0], r_outcomes[0]
    );
    assert_eq!(
        c_crash, r_crash,
        "E6: crash-class parity: C={c_outcomes:?} RUST={r_outcomes:?}"
    );
    if c_crash > 0 {
        assert_eq!(
            c_outcomes[0].signal, r_outcomes[0].signal,
            "E6: fatal signal differs"
        );
    }
    // Whatever happens, no function in this library can report an error to its
    // caller: every symbol is `void`-returning. Sanity-check that by confirming
    // a surviving child exits 0 (never a non-zero error status of its own).
    for o in c_outcomes.iter().chain(r_outcomes.iter()) {
        if let Some(code) = o.exit {
            assert_eq!(code, 0, "E6: no error code channel exists, got {o:?}");
        }
    }
    // Non-vacuity: when the garbage pointer happens to be dereferenceable, the
    // call must still have gone all the way through printIntPtrLine, i.e. the
    // child emitted EXACTLY ONE `%d\n` line. The *value* is indeterminate (the
    // C standard leaves the uninitialised stack slot unspecified) so it is not
    // compared, but the output SHAPE is, in both implementations.
    for (tag, o) in c_outcomes
        .iter()
        .map(|o| ("C", o))
        .chain(r_outcomes.iter().map(|o| ("RUST", o)))
    {
        if !o.crashed() {
            assert_single_decimal_line(&o.stdout, &format!("E6 {tag}"));
        }
    }
}

// ------------------------------------------------------------------ E7
// Out-of-range "enum-like" ints across the FFI boundary: `if (useGood)` is a
// truthiness test, so EVERY non-zero value must take good().
#[test]
fn err_e7_out_of_range_enum_values() {
    let (cf, rf) = (c_driver(), rs_driver());
    let mut vals: Vec<libc::c_int> = vec![
        2,
        3,
        -1,
        -2,
        0x100,
        0x1_0000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x7fff_fffe,
        -0x8000_0000i64 as i32,
    ];
    let mut rng = Rng::new(0xE7_5EED);
    while vals.len() < 200 {
        let v = rng.next_i32();
        if v != 0 && v != 1 {
            vals.push(v);
        }
    }
    for v in vals {
        let _g = stdout_lock();
        let c_out = capture(|| unsafe { cf(v) });
        let r_out = capture(|| unsafe { rf(v) });
        assert_eq!(
            c_out,
            r_out,
            "E7 driver({v}): C={:?} RUST={:?}",
            show(&c_out),
            show(&r_out)
        );
        assert_eq!(
            c_out,
            b"5\n".to_vec(),
            "E7 driver({v}) must be truthy => good()"
        );
    }
}

// ------------------------------------------------------------------ E8
// Exactly 0 (and only 0) routes to the CWE-457 path; both must agree.
#[test]
fn err_e8_driver_zero_routes_to_bad() {
    let (cd, rd) = (c_driver(), rs_driver());
    let cb: FnVoid = c_fn(b"bad\0");
    let rb: FnVoid = rs_fn(b"bad\0");

    let c0 = run_in_child(|| unsafe { cd(0) });
    let r0 = run_in_child(|| unsafe { rd(0) });
    let cbad = run_in_child(|| unsafe { cb() });
    let rbad = run_in_child(|| unsafe { rb() });

    eprintln!("E8: C driver(0)={c0:?} bad()={cbad:?}");
    eprintln!("E8: R driver(0)={r0:?} bad()={rbad:?}");

    assert_eq!(
        c0.crashed(),
        r0.crashed(),
        "E8: driver(0) outcome class differs: C={c0:?} RUST={r0:?}"
    );
    if c0.crashed() {
        assert_eq!(c0.signal, r0.signal, "E8: driver(0) signal differs");
        assert_eq!(c0.signal, cbad.signal, "E8: C driver(0) != C bad()");
        assert_eq!(r0.signal, rbad.signal, "E8: RUST driver(0) != RUST bad()");
    }
    // And 0 is the ONLY value that does this: +-1 around it are the good path.
    for v in [1, -1] {
        let _g = stdout_lock();
        let c_out = capture(|| unsafe { cd(v) });
        let r_out = capture(|| unsafe { rd(v) });
        assert_eq!(c_out, b"5\n".to_vec(), "E8 neighbour {v} (C)");
        assert_eq!(r_out, b"5\n".to_vec(), "E8 neighbour {v} (RUST)");
    }
}

// ------------------------------------------------------------------ E9
// No failure mode: no state, no allocation, no error path on repetition or
// nesting of the safe entry points.
#[test]
fn err_e9_no_failure_mode_on_repeat() {
    let (cd, rd) = (c_driver(), rs_driver());
    let cg: FnVoid = c_fn(b"good\0");
    let rg: FnVoid = rs_fn(b"good\0");
    let _g = stdout_lock();
    let c_out = capture(|| unsafe {
        for i in 0..250 {
            cd(if i % 3 == 0 { 1 } else { i + 1 });
            cg();
        }
    });
    let r_out = capture(|| unsafe {
        for i in 0..250 {
            rd(if i % 3 == 0 { 1 } else { i + 1 });
            rg();
        }
    });
    assert_eq!(c_out, r_out, "E9 repeated-call streams differ");
    assert_eq!(c_out, b"5\n".repeat(500), "E9 expected 500 lines of 5");
}

// ------------------------------------------------------------------ generic
// Zero/oversized "lengths" do not exist in this API (no length parameters), so
// the generic boundary sweep is over the whole 64-bit pointer space near
// interesting addresses, checked for identical termination status.
#[test]
fn generic_pointer_boundary_sweep() {
    let (cf, rf) = (c_print(), rs_print());
    // Mapped, valid: must both print identically and survive.
    let v: libc::c_int = 0x1234_5678;
    let p = &v as *const libc::c_int;
    let _g = stdout_lock();
    assert_eq!(capture(|| unsafe { cf(p) }), capture(|| unsafe { rf(p) }));
    drop(_g);

    // Unmapped/degenerate: must both fault the same way.
    for addr in [0usize, 1, 2, 3, 7, 0xfff, 0x1000] {
        let (c, r) = cmp_fatal(addr as *const libc::c_int, &format!("sweep {addr:#x}"));
        assert_eq!(c.signal, r.signal);
        assert!(
            c.signal.is_some(),
            "sweep {addr:#x} unexpectedly survived: {c:?}"
        );
    }
}
