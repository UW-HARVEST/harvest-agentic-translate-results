//! Phase C — error/rejection-path differential tests, one per row of `ERRORS.md`.
//!
//! The C library has no return codes (both entry points are `void`), so the
//! observable "error result" is (a) the exact bytes written to stdout and
//! (b) how the call terminates. Both are compared.

mod common;

use common::{Exit, Impl, Libs, Rng, assert_same, capture_stdout, capture_stdout_status, fork_and_run};
use std::ffi::c_char;

// ------------------------------------------------------------ ERRORS.md row 1
// printLine(NULL): the `if (line != NULL)` guard rejects the input.

#[test]
fn err_01_printline_null() {
    let libs = Libs::load();
    let (c, c_exit) = capture_stdout_status(|| {
        let f = libs.print_line(Impl::C);
        unsafe { f(std::ptr::null()) }
    });
    let (r, r_exit) = capture_stdout_status(|| {
        let f = libs.print_line(Impl::Rust);
        unsafe { f(std::ptr::null()) }
    });
    assert_eq!(c_exit, Exit::Code(0), "C printLine(NULL) must return normally");
    assert_eq!(r_exit, c_exit, "termination differs for printLine(NULL)");
    assert_same("printLine(NULL)", &c, &r);
    assert!(c.is_empty(), "the rejection must emit zero bytes, got {c:?}");

    // Repeated nulls, and nulls mixed with valid calls, must stay a no-op.
    let script = |which: Impl| {
        let p = libs.print_line(which);
        capture_stdout(|| unsafe {
            p(std::ptr::null());
            p(b"x\0".as_ptr() as *const c_char);
            p(std::ptr::null());
            p(std::ptr::null());
            p(b"\0".as_ptr() as *const c_char);
        })
    };
    let c = script(Impl::C);
    let r = script(Impl::Rust);
    assert_same("printLine null/valid mix", &c, &r);
    assert_eq!(c, b"x\n\n".to_vec());
}

// ------------------------------------------------------------ ERRORS.md row 2
// printLine(""): zero-length input, the boundary of "valid".

#[test]
fn err_02_printline_empty() {
    let libs = Libs::load();
    let call = |which: Impl| {
        let p = libs.print_line(which);
        capture_stdout(|| unsafe { p(b"\0".as_ptr() as *const c_char) })
    };
    let c = call(Impl::C);
    let r = call(Impl::Rust);
    assert_same("printLine(\"\")", &c, &r);
    assert_eq!(c, b"\n".to_vec(), "empty string must still emit exactly one newline");
}

// ------------------------------------------------------------ ERRORS.md row 3
// A non-NUL-terminated buffer is undefined behaviour in the C and is therefore
// not exercised; this test documents that the *terminated* version of the same
// buffer (the only defined case) agrees.

#[test]
fn err_03_printline_unterminated_is_ub_documented() {
    let libs = Libs::load();
    let mut buf = vec![b'A'; 99];
    buf.push(0);
    let call = |which: Impl| {
        let p = libs.print_line(which);
        capture_stdout(|| unsafe { p(buf.as_ptr() as *const c_char) })
    };
    assert_same("printLine(99 'A' + NUL)", &call(Impl::C), &call(Impl::Rust));
}

// ------------------------------------------------------------ ERRORS.md row 4
// driver(100): the exact first value rejected by `if (data < 100)`.

#[test]
fn err_04_driver_at_boundary() {
    let libs = Libs::load();
    let c = libs.call_driver(Impl::C, 100);
    let r = libs.call_driver(Impl::Rust, 100);
    assert_same("driver(100)", &c, &r);
    assert_eq!(c, b"\n".to_vec(), "rejected input must leave dest empty");

    // One step either side of the boundary must also agree.
    assert_same("driver(99)", &libs.call_driver(Impl::C, 99), &libs.call_driver(Impl::Rust, 99));
    assert_same("driver(101)", &libs.call_driver(Impl::C, 101), &libs.call_driver(Impl::Rust, 101));
    assert_ne!(
        libs.call_driver(Impl::C, 99),
        libs.call_driver(Impl::C, 100),
        "the boundary must actually change C's behaviour (guards the test itself)"
    );
}

// ------------------------------------------------------------ ERRORS.md row 5
// driver(data) for data > 100, including oversized lengths up to INT_MAX.

#[test]
fn err_05_driver_above_boundary() {
    let libs = Libs::load();
    let mut cases = vec![101, 102, 127, 128, 255, 256, 1000, 65535, 65536, 1 << 20, i32::MAX - 1, i32::MAX];
    let mut rng = Rng::new(0x5EED_0005);
    for _ in 0..300 {
        cases.push(rng.range_i32(101, i32::MAX));
    }
    for data in cases {
        let c = libs.call_driver(Impl::C, data);
        let r = libs.call_driver(Impl::Rust, data);
        assert_same(&format!("driver({data})"), &c, &r);
        assert_eq!(c, b"\n".to_vec(), "driver({data}) must be rejected by the range check");
    }
}

// ------------------------------------------------------------ ERRORS.md row 6
// driver(99): largest accepted value; writes dest[99], the last in-bounds byte.

#[test]
fn err_06_driver_max_inrange() {
    let libs = Libs::load();
    let (c, c_exit) = capture_stdout_status(|| {
        let f = libs.driver(Impl::C);
        unsafe { f(99) }
    });
    let (r, r_exit) = capture_stdout_status(|| {
        let f = libs.driver(Impl::Rust);
        unsafe { f(99) }
    });
    assert_eq!(c_exit, Exit::Code(0));
    assert_eq!(r_exit, c_exit, "termination differs for driver(99)");
    assert_same("driver(99)", &c, &r);
    let mut expected = vec![b'A'; 99];
    expected.push(b'\n');
    assert_eq!(c, expected);
}

// ------------------------------------------------------------ ERRORS.md row 7
// driver(data) for data < 0: passes `data < 100`, then sign-extends `data` into
// strncpy's size_t and writes dest[data] out of bounds. Deliberate UB in the C;
// both libraries must die the same way. Run in a forked child so the crash does
// not take the test runner with it.

#[test]
fn err_07_driver_negative_crashes_identically() {
    let libs = Libs::load();
    for data in [-1i32, -2, -50, -99, -100, -1000, i32::MIN + 1, i32::MIN] {
        let c_exit = fork_and_run(|| {
            let f = libs.driver(Impl::C);
            unsafe { f(data) }
        });
        let r_exit = fork_and_run(|| {
            let f = libs.driver(Impl::Rust);
            unsafe { f(data) }
        });
        assert_eq!(
            r_exit, c_exit,
            "driver({data}) terminates differently: C {c_exit:?} vs Rust {r_exit:?}"
        );
        // Guard the test itself: the C really must be crashing here.
        assert!(
            matches!(c_exit, Exit::Signal(_)),
            "expected the C to crash on driver({data}), got {c_exit:?}"
        );
    }
}

// ------------------------------------------------------------ ERRORS.md row 8
// driver(0): degenerate valid length.

#[test]
fn err_08_driver_zero() {
    let libs = Libs::load();
    let c = libs.call_driver(Impl::C, 0);
    let r = libs.call_driver(Impl::Rust, 0);
    assert_same("driver(0)", &c, &r);
    assert_eq!(c, b"\n".to_vec());
}

// ------------------------------------------- generic FFI-boundary sweep
// `driver`'s parameter is a bare `int`, so every 32-bit value is a legal FFI
// input (this stands in for the "out-of-range enum value" case: the API has no
// enum, and the full integer domain is what an external caller can pass).
// Values that fall in the UB region (data < 0) are compared by termination
// status; the rest by output bytes.

#[test]
fn err_09_full_int_domain_random_probe() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x5EED_0009);

    // Defined region: any data >= 0.
    for _ in 0..1500 {
        let data = (rng.next_u64() as u32 & 0x7fff_ffff) as i32;
        let c = libs.call_driver(Impl::C, data);
        let r = libs.call_driver(Impl::Rust, data);
        assert_same(&format!("driver({data})"), &c, &r);
    }

    // Powers of two and their neighbours, plus the exact type extremes.
    let mut edges = vec![0i32, 1, 2, 98, 99, 100, 101, i32::MAX, i32::MAX - 1];
    for shift in 0..31 {
        let v = 1i32 << shift;
        edges.extend_from_slice(&[v - 1, v, v.saturating_add(1)]);
    }
    for data in edges {
        if data < 0 {
            continue;
        }
        let c = libs.call_driver(Impl::C, data);
        let r = libs.call_driver(Impl::Rust, data);
        assert_same(&format!("driver({data})"), &c, &r);
    }

    // UB region: compare how the process dies, not the bytes.
    for _ in 0..12 {
        let data = -(rng.range_i32(1, i32::MAX));
        let c_exit = fork_and_run(|| {
            let f = libs.driver(Impl::C);
            unsafe { f(data) }
        });
        let r_exit = fork_and_run(|| {
            let f = libs.driver(Impl::Rust);
            unsafe { f(data) }
        });
        assert_eq!(r_exit, c_exit, "driver({data}): C {c_exit:?} vs Rust {r_exit:?}");
    }
}

// ------------------------------------------------------- symbol-parity guard
// Also asserted in phase_d_parity.rs; repeated here so that an error-path run
// alone still notices a vanished export.

#[test]
fn err_10_both_libraries_export_the_same_entry_points() {
    let libs = Libs::load();
    for which in [Impl::C, Impl::Rust] {
        let _ = libs.driver(which);
        let _ = libs.print_line(which);
    }
}
