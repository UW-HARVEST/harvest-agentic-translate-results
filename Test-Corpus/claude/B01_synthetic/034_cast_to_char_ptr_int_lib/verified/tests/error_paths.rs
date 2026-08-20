//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1 … E8) plus the generic C-API boundary
//! rows (G1 … G5). Both libraries are exercised through their `.so` exports and
//! must reject / fail / survive *identically* — same observable state, not just
//! "both failed somehow".
//!
//! Recall the mechanical finding recorded in `ERRORS.md`: the C has **no**
//! validation and **no** error channel (`void driver(int)`, `printf` return
//! value ignored at both call sites). The observable outcomes available for
//! comparison are therefore: the emitted bytes, the `stdout` `FILE` error flag,
//! `errno`, and the fact that the call returns instead of crashing.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// E1 + E2 — print_hex's `len` guard (`i < len`): len == 0 and len < 0.
//
// `driver` always passes the compile-time constant `sizeof(int) == 4`, so the
// zero/negative-length branches are unreachable from the public API. What must
// be verified is exactly that: the output shape is *always* 4 bytes' worth of
// hex plus one newline — never the 1-byte `"\n"` that a len <= 0 pass would
// produce, and never a run-away loop from a negative length.
// ---------------------------------------------------------------------------
#[test]
fn err_e1_len_zero_and_negative_are_unreachable_but_shape_is_fixed() {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();

    let mut rng = Rng::new(SEED ^ 0xE1);
    let mut inputs = vec![0, -1, i32::MIN, i32::MAX, 1, -2];
    inputs.extend(rng.sample(2_000));

    let (c_out, rust_out) = with_stdout(|env| {
        let c_out = env.capture_file(|| {
            for &x in &inputs {
                unsafe { c(x) }
            }
        });
        let rust_out = env.capture_file(|| {
            for &x in &inputs {
                unsafe { r(x) }
            }
        });
        (c_out, rust_out)
    });

    assert_streams_match("E1/E2 fixed record shape", &inputs, &c_out, &rust_out);

    // len == 0 or len < 0 would change the record length; assert it never does.
    assert_eq!(
        c_out.len(),
        inputs.len() * RECORD_LEN,
        "C record length changed: len<=0 branch was somehow taken"
    );
    assert_eq!(
        rust_out.len(),
        inputs.len() * RECORD_LEN,
        "Rust record length changed: len<=0 branch was somehow taken"
    );
    for (i, rec) in rust_out.chunks(RECORD_LEN).enumerate() {
        assert_eq!(rec.len(), RECORD_LEN, "short record #{i}");
        assert_ne!(rec, b"\n", "record #{i} is a bare newline (len<=0 path)");
        assert_eq!(rec[RECORD_LEN - 1], b'\n', "record #{i} lacks its newline");
        assert!(
            rec[..RECORD_LEN - 1].iter().all(|b| b.is_ascii_hexdigit()),
            "record #{i} contains a non-hex byte"
        );
    }
}

// ---------------------------------------------------------------------------
// E3 — no input value is ever rejected: the whole 32-bit domain is accepted and
// always yields one complete record; there is no error return to compare
// because the function is `void`.
// ---------------------------------------------------------------------------
#[test]
fn err_e3_no_value_is_ever_rejected() {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();

    // extremes, one-step-past-boundary values, and randomized coverage
    let mut inputs: Vec<i32> = vec![
        0,
        1,
        -1,
        2,
        -2,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        i16::MIN as i32 - 1,
        i16::MIN as i32,
        i16::MAX as i32,
        i16::MAX as i32 + 1,
        u16::MAX as i32,
        u16::MAX as i32 + 1,
        i8::MIN as i32 - 1,
        i8::MIN as i32,
        i8::MAX as i32,
        i8::MAX as i32 + 1,
        u8::MAX as i32,
        u8::MAX as i32 + 1,
    ];
    let mut rng = Rng::new(SEED ^ 0xE3);
    inputs.extend(rng.sample(5_000));

    let (c_out, rust_out, returned) = with_stdout(|env| {
        let c_out = env.capture_file(|| {
            for &x in &inputs {
                unsafe { c(x) }
            }
        });
        let mut returned = 0usize;
        let rust_out = env.capture_file(|| {
            for &x in &inputs {
                unsafe { r(x) };
                // reaching this line proves the call returned normally
                returned += 1;
            }
        });
        (c_out, rust_out, returned)
    });

    assert_eq!(returned, inputs.len(), "a Rust call did not return");
    assert_streams_match("E3 nothing is rejected", &inputs, &c_out, &rust_out);
}

// ---------------------------------------------------------------------------
// E4 — stdout write fails with EBADF (fd 1 is open read-only).
// ---------------------------------------------------------------------------
#[test]
fn err_e4_stdout_ebadf() {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();
    let x = 0xDEAD_BEEFu32 as i32;

    let (c_state, rust_state) = with_stdout(|env| {
        // unbuffered so the failing write(2) happens inside the call
        assert_eq!(env.set_mode(IONBF, 0), 0, "setvbuf(_IONBF) failed");
        // /dev/null opened read-only: every write(2) returns EBADF
        let c_state = env.run_with_stdout_on("/dev/null", O_RDONLY, || unsafe { c(x) });
        let rust_state = env.run_with_stdout_on("/dev/null", O_RDONLY, || unsafe { r(x) });
        env.set_mode(IOFBF, 4096);
        (c_state, rust_state)
    });

    assert_eq!(
        c_state, rust_state,
        "EBADF handling differs: C (ferror, errno) = {c_state:?}, Rust = {rust_state:?}"
    );
    assert_ne!(c_state.0, 0, "expected the C write to fail (ferror set)");
    assert_eq!(c_state.1, EBADF, "expected errno == EBADF, got {}", c_state.1);
}

// ---------------------------------------------------------------------------
// E5 — stdout write fails with ENOSPC (/dev/full).
// ---------------------------------------------------------------------------
#[test]
fn err_e5_stdout_enospc_dev_full() {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();
    let x = -12345;

    let (c_state, rust_state) = with_stdout(|env| {
        assert_eq!(env.set_mode(IONBF, 0), 0, "setvbuf(_IONBF) failed");
        let c_state = env.run_with_stdout_on("/dev/full", O_WRONLY, || unsafe { c(x) });
        let rust_state = env.run_with_stdout_on("/dev/full", O_WRONLY, || unsafe { r(x) });
        env.set_mode(IOFBF, 4096);
        (c_state, rust_state)
    });

    assert_eq!(
        c_state, rust_state,
        "ENOSPC handling differs: C (ferror, errno) = {c_state:?}, Rust = {rust_state:?}"
    );
    assert_ne!(c_state.0, 0, "expected the C write to fail (ferror set)");
    assert_eq!(
        c_state.1, ENOSPC,
        "expected errno == ENOSPC, got {}",
        c_state.1
    );
}

// ---------------------------------------------------------------------------
// E6 — stdout write fails with EPIPE (pipe with a closed read end).
// ---------------------------------------------------------------------------
#[test]
fn err_e6_stdout_epipe() {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();
    let x = i32::MIN;

    let (c_state, rust_state) = with_stdout(|env| {
        assert_eq!(env.set_mode(IONBF, 0), 0, "setvbuf(_IONBF) failed");
        let c_state = env.run_with_stdout_on_broken_pipe(|| unsafe { c(x) });
        let rust_state = env.run_with_stdout_on_broken_pipe(|| unsafe { r(x) });
        env.set_mode(IOFBF, 4096);
        (c_state, rust_state)
    });

    assert_eq!(
        c_state, rust_state,
        "EPIPE handling differs: C (ferror, errno) = {c_state:?}, Rust = {rust_state:?}"
    );
    assert_ne!(c_state.0, 0, "expected the C write to fail (ferror set)");
    assert_eq!(c_state.1, EPIPE, "expected errno == EPIPE, got {}", c_state.1);
}

// ---------------------------------------------------------------------------
// E7 — stdout already carries a sticky FILE error flag.
// ---------------------------------------------------------------------------
#[test]
fn err_e7_sticky_stream_error_state() {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();
    let x = 0x0BAD_F00Du32 as i32;

    let (c_res, rust_res) = with_stdout(|env| {
        let c_res = env.sticky_error_then_capture(|| unsafe { c(x) }, || unsafe { c(x) });
        let rust_res = env.sticky_error_then_capture(|| unsafe { r(x) }, || unsafe { r(x) });
        (c_res, rust_res)
    });

    assert_eq!(
        c_res.0, rust_res.0,
        "ferror(stdout) after writing on an errored stream differs: C={}, Rust={}",
        c_res.0, rust_res.0
    );
    assert_eq!(
        c_res.1,
        rust_res.1,
        "bytes emitted on an errored stream differ:\n  C   = {:?}\n  Rust= {:?}",
        String::from_utf8_lossy(&c_res.1),
        String::from_utf8_lossy(&rust_res.1)
    );
}

// ---------------------------------------------------------------------------
// E8 — `print_hex` is `static` in C, so it must not be reachable via dlsym in
// either library.
// ---------------------------------------------------------------------------
#[test]
fn err_e8_internal_symbol_not_exported() {
    let l = libs();
    for name in [
        &b"print_hex\0"[..],
        &b"driver_print_hex\0"[..],
        &b"_print_hex\0"[..],
    ] {
        let c_found = unsafe { l.c_lib.get::<DriverFn>(name) }.is_ok();
        let rust_found = unsafe { l.rust_lib.get::<DriverFn>(name) }.is_ok();
        assert_eq!(
            c_found,
            rust_found,
            "dlsym({:?}) parity broken: C={c_found}, Rust={rust_found}",
            String::from_utf8_lossy(name)
        );
        assert!(
            !c_found,
            "{:?} must stay internal",
            String::from_utf8_lossy(name)
        );
    }

    // A symbol that exists in neither must fail in both.
    let bogus = &b"definitely_not_a_symbol\0"[..];
    assert!(unsafe { l.c_lib.get::<DriverFn>(bogus) }.is_err());
    assert!(unsafe { l.rust_lib.get::<DriverFn>(bogus) }.is_err());

    // …and the one real symbol must resolve in both.
    assert!(unsafe { l.c_lib.get::<DriverFn>(b"driver\0") }.is_ok());
    assert!(unsafe { l.rust_lib.get::<DriverFn>(b"driver\0") }.is_ok());
}

// ---------------------------------------------------------------------------
// G1 — there is no pointer parameter in the public API, so there is no null
// pointer boundary to hit. Verified mechanically against the header.
// ---------------------------------------------------------------------------
#[test]
fn err_g1_no_pointer_parameters_in_public_api() {
    let header = std::fs::read_to_string(manifest_dir().join("c_src/include/driver.h"))
        .expect("read driver.h");
    let decls: Vec<&str> = header
        .lines()
        .map(str::trim)
        .filter(|l| l.ends_with(");"))
        .collect();
    assert_eq!(
        decls,
        vec!["void driver(int x);"],
        "the public API changed; re-derive ERRORS.md/CONFIGS.md"
    );
    assert!(
        !decls[0].contains('*'),
        "a pointer parameter appeared: a null-pointer row is now required"
    );
    // The single exported symbol takes one `int` and returns nothing, so the
    // only marshallable inputs are the 2^32 bit patterns exercised by E3/G3.
}

// ---------------------------------------------------------------------------
// G3 — values one step past the `int` range, as they wrap when marshalled.
// ---------------------------------------------------------------------------
#[test]
fn err_g3_one_past_int_range_wraps() {
    let l = libs();
    let c64 = l.c_driver64();
    let r64 = l.rust_driver64();

    let raw: Vec<i64> = vec![
        i32::MAX as i64 + 1,
        i32::MIN as i64 - 1,
        u32::MAX as i64,
        u32::MAX as i64 + 1,
        0x1_0000_0000,
        -1,
        i64::MIN,
        i64::MAX,
        0x7FFF_FFFF_FFFF_FFFF,
        -0x8000_0000_0000_0000,
    ];
    let inputs: Vec<i32> = raw.iter().map(|&v| v as u64 as u32 as i32).collect();

    let (c_out, rust_out) = with_stdout(|env| {
        let c_out = env.capture_file(|| {
            for &v in &raw {
                unsafe { c64(v) }
            }
        });
        let rust_out = env.capture_file(|| {
            for &v in &raw {
                unsafe { r64(v) }
            }
        });
        (c_out, rust_out)
    });
    assert_streams_match("G3 one past int range", &inputs, &c_out, &rust_out);
}

// ---------------------------------------------------------------------------
// G4 — "out-of-range enum" analogue: an argument register whose upper 32 bits
// carry a pattern that has no meaning for the callee.
// ---------------------------------------------------------------------------
#[test]
fn err_g4_upper_register_bits_ignored() {
    let l = libs();
    let c64 = l.c_driver64();
    let r64 = l.rust_driver64();

    let mut raw: Vec<i64> = Vec::new();
    for hi in [
        0x0000_0000u64,
        0xFFFF_FFFF,
        0xDEAD_BEEF,
        0x8000_0000,
        0x7FFF_FFFF,
    ] {
        for lo in [0x0000_0000u64, 0x0000_0001, 0xFFFF_FFFF, 0x8000_0000, 0x0BAD_C0DE] {
            raw.push(((hi << 32) | lo) as i64);
        }
    }
    let inputs: Vec<i32> = raw.iter().map(|&v| v as u64 as u32 as i32).collect();

    let (c_out, rust_out) = with_stdout(|env| {
        let c_out = env.capture_file(|| {
            for &v in &raw {
                unsafe { c64(v) }
            }
        });
        let rust_out = env.capture_file(|| {
            for &v in &raw {
                unsafe { r64(v) }
            }
        });
        (c_out, rust_out)
    });
    assert_streams_match("G4 upper bits ignored", &inputs, &c_out, &rust_out);
}

// ---------------------------------------------------------------------------
// G5 — after a stream error is cleared, both libraries resume identically.
// ---------------------------------------------------------------------------
#[test]
fn err_g5_call_after_error_recovers_identically() {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();
    let mut rng = Rng::new(SEED ^ 0xE5);
    let inputs = rng.sample(200);

    let (c_state, rust_state, c_out, rust_out) = with_stdout(|env| {
        assert_eq!(env.set_mode(IONBF, 0), 0, "setvbuf(_IONBF) failed");
        // provoke failures on both sides (each helper clears the flag on exit)
        let c_state = env.run_with_stdout_on("/dev/full", O_WRONLY, || unsafe { c(1) });
        let rust_state = env.run_with_stdout_on("/dev/full", O_WRONLY, || unsafe { r(1) });
        env.set_mode(IOFBF, 4096);
        // now both must work exactly as before
        let c_out = env.capture_file(|| {
            for &x in &inputs {
                unsafe { c(x) }
            }
        });
        let rust_out = env.capture_file(|| {
            for &x in &inputs {
                unsafe { r(x) }
            }
        });
        (c_state, rust_state, c_out, rust_out)
    });

    assert_eq!(c_state, rust_state, "failure states differ before recovery");
    assert_streams_match("G5 recovery", &inputs, &c_out, &rust_out);
    assert_eq!(c_out.len(), inputs.len() * RECORD_LEN);
}

// ---------------------------------------------------------------------------
// Extra: hammer the error paths with randomized inputs so the comparison is not
// based on a single hand-picked value.
// ---------------------------------------------------------------------------
#[test]
fn err_randomized_failure_states_match() {
    let l = libs();
    let c = l.c_driver();
    let r = l.rust_driver();
    let mut rng = Rng::new(SEED ^ 0xBEEF);

    let cases: Vec<(&str, &str, c_int)> = vec![
        ("EBADF", "/dev/null", O_RDONLY),
        ("ENOSPC", "/dev/full", O_WRONLY),
    ];

    with_stdout(|env| {
        assert_eq!(env.set_mode(IONBF, 0), 0, "setvbuf(_IONBF) failed");
        for _ in 0..64 {
            let x = rng.next_interesting_i32();
            for (label, path, flags) in &cases {
                let cs = env.run_with_stdout_on(path, *flags, || unsafe { c(x) });
                let rs = env.run_with_stdout_on(path, *flags, || unsafe { r(x) });
                assert_eq!(
                    cs, rs,
                    "[{label}] state differs for x=0x{:08x}: C={cs:?} Rust={rs:?}",
                    x as u32
                );
            }
        }
        env.set_mode(IOFBF, 4096);
    });
}
