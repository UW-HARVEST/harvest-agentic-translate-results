//! Additional FFI-boundary edge inputs that a real external caller can produce
//! but which neither the happy path nor the CONFIGS/ERRORS rows reach directly.

mod common;

use common::*;
use std::ffi::CString;

/// `struct ConfigFlags` has 4-byte alignment, but nothing stops a foreign caller
/// from handing over a pointer into the middle of a buffer. The C reads/writes
/// the bitfields through the storage unit; the Rust translation must agree at
/// every misalignment.
#[test]
fn misaligned_config_flags_pointer() {
    let _g = lock();
    env_clear_all();
    let (c, r) = libs();

    // Some patterns set the debug bit, so the calls print; the streams are
    // compared elsewhere, so suppress them here.
    silenced(|| {
    for offset in 0usize..4 {
        for pattern in [0x00u8, 0xFF, 0xA5, 0x5A] {
            for value in [i32::MIN, -1, 0, 1, 0x4000_0000, i32::MAX] {
                // 8 bytes of backing store so a 4-byte access at any offset in
                // 0..4 stays inside the allocation.
                let mut buf_c = [pattern; 8];
                let mut buf_r = [pattern; 8];

                let rc = unsafe {
                    (c.apply_bit_operations)(value, buf_c.as_mut_ptr().add(offset) as *mut u32)
                };
                let rr = unsafe {
                    (r.apply_bit_operations)(value, buf_r.as_mut_ptr().add(offset) as *mut u32)
                };
                assert_eq!(
                    rr, rc,
                    "apply_bit_operations diverged at offset {offset}, pattern {pattern:#x}, value {value}"
                );
                assert_eq!(buf_r, buf_c, "buffer diverged (offset {offset})");

                let rc = unsafe {
                    (c.perform_operation)(value, value, buf_c.as_mut_ptr().add(offset) as *mut u32)
                };
                let rr = unsafe {
                    (r.perform_operation)(value, value, buf_r.as_mut_ptr().add(offset) as *mut u32)
                };
                assert_eq!(
                    rr, rc,
                    "perform_operation diverged at offset {offset}, pattern {pattern:#x}"
                );
                assert_eq!(buf_r, buf_c, "buffer diverged (offset {offset})");
            }

            // And the writing entry point.
            let mut buf_c = [pattern; 8];
            let mut buf_r = [pattern; 8];
            unsafe { (c.init_config_from_env)(buf_c.as_mut_ptr().add(offset) as *mut u32) };
            unsafe { (r.init_config_from_env)(buf_r.as_mut_ptr().add(offset) as *mut u32) };
            assert_eq!(
                buf_r, buf_c,
                "init_config_from_env wrote different bytes at offset {offset}, pattern {pattern:#x}"
            );
        }
    }
    });
}

/// `init_config_from_env` must not write past the 4-byte storage unit.
#[test]
fn init_config_does_not_write_out_of_bounds() {
    let _g = lock();
    env_clear_all();
    env_set("PROG_VERBOSE", "1");
    env_set("PROG_DEBUG", "1");
    env_set("PROG_OPTIMIZE", "1");
    let (c, r) = libs();

    let mut buf_c = [0xCDu8; 32];
    let mut buf_r = [0xCDu8; 32];
    unsafe { (c.init_config_from_env)(buf_c.as_mut_ptr().add(12) as *mut u32) };
    unsafe { (r.init_config_from_env)(buf_r.as_mut_ptr().add(12) as *mut u32) };
    assert_eq!(buf_r, buf_c, "guard bytes or payload differ");
    assert!(
        buf_c[..12].iter().all(|&b| b == 0xCD) && buf_c[16..].iter().all(|&b| b == 0xCD),
        "the C itself wrote outside the 4-byte storage unit: {buf_c:?}"
    );
    env_clear_all();
}

/// Odd `env_name` shapes handed to `parse_env_numeric`. `getenv` has its own
/// rules for these (a name containing `=` can never match), and both libraries
/// must inherit them identically because both call the same libc `getenv`.
#[test]
fn odd_env_name_shapes() {
    let _g = lock();
    env_clear_all();
    env_set("PROG_BASE_OFFSET", "1234");

    let names = [
        "PROG_BASE_OFFSET",
        "PROG_BASE_OFFSET=",
        "PROG_BASE_OFFSET=5",
        "=PROG_BASE_OFFSET",
        "PROG_BASE_OFFSE",
        "PROG_BASE_OFFSETX",
        "prog_base_offset",
        "",
        "=",
        " PROG_BASE_OFFSET",
        "PROG_BASE_OFFSET ",
    ];

    diff("edge: odd env_name shapes", move |lib| {
        let mut out = Vec::new();
        for n in names {
            let cn = CString::new(n).unwrap();
            for d in [0i32, -1, 64, i32::MIN, i32::MAX] {
                out.push(unsafe { (lib.parse_env_numeric)(cn.as_ptr(), d) } as i64);
            }
        }
        out
    });
    env_clear_all();
}

/// Env *values* containing high bytes / non-UTF-8 / embedded whitespace. The C
/// treats them as raw bytes via `strchr`/`atoi`.
#[test]
fn high_byte_and_whitespace_env_values() {
    let _g = lock();
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for b in [0x80u8, 0xFF, 0x01, 0x7F, 0x2C, 0x3B, 0x20, 0x09, 0x0A, 0x0D] {
        cases.push(vec![b]);
        cases.push(vec![b'1', b]);
        cases.push(vec![b, b'1']);
        cases.push(vec![b'-', b, b'5']);
        cases.push(vec![b'4', b, b'2']);
    }
    cases.push("１２３".as_bytes().to_vec()); // full-width digits
    cases.push("−5".as_bytes().to_vec()); // U+2212 minus sign
    cases.push(vec![0xC3, 0xA9, b'7']);

    diff("edge: high-byte env values", move |lib| {
        let mut out = Vec::new();
        let name = CString::new("PROG_MULTIPLIER").unwrap();
        for bytes in &cases {
            env_clear_all();
            // Values with a NUL cannot exist in `environ`, and none is generated
            // above; anything else goes through verbatim.
            let v = CString::new(bytes.clone()).unwrap();
            unsafe { set_env_raw(c"PROG_MULTIPLIER".as_ptr(), v.as_ptr()) };
            for d in [0i32, -1, 10, i32::MIN, i32::MAX] {
                out.push(unsafe { (lib.parse_env_numeric)(name.as_ptr(), d) } as i64);
            }
            // and end to end
            out.push(unsafe { (lib.envy)(3, 5, 7, 9) } as i64);
        }
        env_clear_all();
        out
    });
}

/// Repeated calls with an unchanged environment must be idempotent in both
/// libraries (neither caches anything).
#[test]
fn repeated_calls_are_idempotent() {
    let _g = lock();
    env_clear_all();
    env_set("PROG_OPTIMIZE", "1");
    env_set("PROG_BASE_OFFSET", "-7");
    env_set("PROG_MULTIPLIER", "3");
    let (c, r) = libs();
    let first_c = unsafe { (c.envy)(11, 22, 33, 44) };
    let first_r = unsafe { (r.envy)(11, 22, 33, 44) };
    assert_eq!(first_r, first_c);
    for _ in 0..1000 {
        assert_eq!(unsafe { (c.envy)(11, 22, 33, 44) }, first_c);
        assert_eq!(unsafe { (r.envy)(11, 22, 33, 44) }, first_c);
    }
    env_clear_all();
}

/// stdout is block-buffered when redirected to a file while stderr is
/// unbuffered, so the *interleaving* of the two streams is itself an observable.
/// Comparing them separately (as the Phase B/C rows do) would not catch a
/// difference in where the flush points fall.
#[test]
fn merged_stdout_and_stderr_interleave_identically() {
    let _g = lock();
    let (c, r) = libs();

    // Every combination that produces output on both streams: verbose/debug on,
    // and both numeric variables rejected so warnings go to stderr.
    for verbose in [false, true] {
        for debug in [false, true] {
            for (off, mult) in [
                (",", ";"),
                (";", ","),
                (",", "5"),
                ("5", ";"),
                (",", ","),
                (";", ";"),
            ] {
                let setup = || {
                    env_clear_all();
                    if verbose {
                        env_set("PROG_VERBOSE", "1");
                    }
                    if debug {
                        env_set("PROG_DEBUG", "1");
                    }
                    env_set("PROG_BASE_OFFSET", off);
                    env_set("PROG_MULTIPLIER", mult);
                };

                let body = |lib: &Lib| {
                    let mut v = Vec::new();
                    for (a, b, cc, d) in [
                        (1i32, 2i32, 3i32, 4i32),
                        (-1, -2, -3, -4),
                        (i32::MIN, i32::MAX, 0, 0),
                        (0, 0, 0, 0),
                        (i32::MAX, i32::MIN, 7, -7),
                    ] {
                        v.push(unsafe { (lib.envy)(a, b, cc, d) });
                    }
                    v
                };

                setup();
                let (vc, mc) = capture_merged(|| body(c));
                setup();
                let (vr, mr) = capture_merged(|| body(r));

                assert_eq!(
                    vr, vc,
                    "return values diverged (verbose={verbose}, debug={debug}, {off:?}/{mult:?})"
                );
                assert!(
                    !mc.is_empty(),
                    "expected output for verbose={verbose} debug={debug} {off:?}/{mult:?}"
                );
                assert_streams_eq(
                    &format!("merged v={verbose} d={debug} {off}/{mult}"),
                    "merged stdout+stderr",
                    &mc,
                    &mr,
                );
            }
        }
    }
    env_clear_all();
}
