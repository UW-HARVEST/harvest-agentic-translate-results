//! ERRORS.md G1 / G2 / G8: the generic FFI-boundary cases.
//!
//! * G1/G2 — `flags` is a C `int` bitmask, so *any* `int` is a valid input,
//!   including values with no documented bit set. Only `0x001`, `0x004` and
//!   `0x010` are ever tested by the C; every other bit must be inert. Verified
//!   over the full 5-bit cross-product plus randomized and extreme ints.
//! * G8 — NULL across the boundary. Every prototype is `nonnull` and the C
//!   dereferences unconditionally, so both implementations must fault
//!   identically. Checked out-of-process so the signal can be compared.

mod common;

use common::*;
use std::ffi::c_int;

const MAIL: c_int = 0x001;
const EXEC: c_int = 0x002;
const READ_ALL: c_int = 0x004;
const READ_FAILED: c_int = 0x008;
const FP_SET: c_int = 0x010;

fn shapes() -> Vec<Vec<u8>> {
    vec![
        b"".to_vec(),
        MINIMAL.as_bytes().to_vec(),
        b"** Alert 1.2: mail - syscheck,\n2016 Apr 19 20:29:00 h->/l\n\
          Rule: 550 (level 7) -> 'Integrity checksum changed.'\n\
          Integrity checksum changed for: '/etc/passwd'\n"
            .to_vec(),
        b"** Alert 9.9: notmail - g,\n2016 Apr 19 20:29:00 h->/l\n\
          Rule: 1 (level 2) -> 'x'\nSrc IP: 10.0.0.1\nSrc Port: 22\n\
          Dst IP: 10.0.0.2\nDst Port: 443\nUser: root\n"
            .to_vec(),
        b"garbage\n** Alert bad\n** Alert 1.2: mail - g\n2016 Apr 19 20:29:00 h->/l\n".to_vec(),
        {
            let mut v = MINIMAL.as_bytes().to_vec();
            v.extend_from_slice(MINIMAL.as_bytes());
            v
        },
    ]
}

/// G1 + G2 — `GetAlertData` under every documented flag combination and under
/// arbitrary / extreme `int` values.
#[test]
fn g1_arbitrary_flag_ints() {
    let shapes = shapes();

    // Full cross-product of the five documented bits.
    for flag in 0..32 {
        for (i, s) in shapes.iter().enumerate() {
            assert_gad_eq(flag, s, 0, &format!("G2 flag={flag:#x} shape#{i}"));
        }
    }

    // Values with NO valid variant: high bits, negatives, extremes.
    let extremes: Vec<c_int> = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -2,
        -1,
        0,
        1,
        0x20,
        0x40,
        0x100,
        0x1000,
        0x4000_0000,
        c_int::MAX,
        c_int::MAX - 1,
        !MAIL,
        !READ_ALL,
        !FP_SET,
        EXEC | READ_FAILED,
        0x7FFF_FFE0,
    ];
    for &flag in &extremes {
        for (i, s) in shapes.iter().enumerate() {
            assert_gad_eq(flag, s, 0, &format!("G1 extreme flag={flag} shape#{i}"));
        }
    }

    // Randomized ints, 400 of them, each against a random shape.
    let mut rng = Rng::new(0xF1A65);
    for n in 0..400 {
        let flag = rng.i32();
        let s = rng.pick(&shapes).clone();
        assert_gad_eq(flag, &s, 0, &format!("G1 random#{n} flag={flag}"));
    }

    // The undocumented bits must be *inert*: masking them off must not change
    // anything observable.
    let inert_mask = !(MAIL | READ_ALL | FP_SET);
    let (c, r) = libs();
    for n in 0..200 {
        let flag = rng.i32();
        let s = rng.pick(&shapes).clone();
        let masked = flag & !inert_mask;
        let a = gad_on_file(c, flag, &s, 0);
        let b = gad_on_file(c, masked, &s, 0);
        assert_eq!(
            a, b,
            "C: undocumented bits are not inert for flag={flag} (#{n})"
        );
        let a2 = gad_on_file(r, flag, &s, 0);
        let b2 = gad_on_file(r, masked, &s, 0);
        assert_eq!(
            a2, b2,
            "RUST: undocumented bits are not inert for flag={flag} (#{n})"
        );
        assert_eq!(a, a2, "C/RUST diverge for flag={flag} (#{n})");
    }
}

/// G1 for `Init_FileQueue`: arbitrary `int` flags, differentially compared.
#[test]
fn g1_init_arbitrary_flag_ints() {
    let g = world();
    write_alerts_log(MINIMAL.as_bytes());
    let (c, r) = libs();
    let t = tm::new(19, 3, 116);

    let mut cases: Vec<c_int> = (0..32).collect();
    cases.extend_from_slice(&[
        c_int::MIN,
        -1,
        0x20,
        0x100,
        0x4000_0000,
        c_int::MAX,
        !FP_SET,
        !READ_ALL,
    ]);
    let mut rng = Rng::new(0x1417);
    // Only flags WITHOUT FP_SET: with FP_SET and a NULL fp the queue takes the
    // "return 0 before fopen" path, which is E6 and tested there.
    for _ in 0..200 {
        cases.push(rng.i32() & !FP_SET);
    }

    for flag in cases {
        let mut fqc = file_queue::zeroed();
        let mut fqr = file_queue::zeroed();
        let (rc_c, ec) = unsafe {
            let rc = (c.init_file_queue)(&mut fqc, &t, flag);
            (rc, snap_queue(&fqc))
        };
        let (rc_r, er) = unsafe {
            let rc = (r.init_file_queue)(&mut fqr, &t, flag);
            (rc, snap_queue(&fqr))
        };
        unsafe {
            if !fqc.fp.is_null() {
                fclose(fqc.fp);
            }
            if !fqr.fp.is_null() {
                fclose(fqr.fp);
            }
        }
        assert_eq!(rc_c, rc_r, "Init_FileQueue rc differs for flags={flag:#x}");
        assert_eq!(ec, er, "Init_FileQueue state differs for flags={flag:#x}");
    }
    drop(g);
}

/// G8 — NULL pointers across the boundary must behave identically. A defensive
/// NULL check added to the Rust would show up as a clean exit where the C died.
#[test]
fn g8_no_defensive_null_checks() {
    // These unconditionally dereference the NULL in the C, so both must fault
    // with the SAME signal (SIGSEGV = 11).
    for case in [
        "gad_null_fp",
        "free_null",
        "init_null_fq",
        "init_null_tm",
        "readmon_null_fq",
    ] {
        let c = assert_helper_same(case);
        assert_eq!(
            c.signal,
            Some(11),
            "[{case}] expected both to die with SIGSEGV, got {c:?}"
        );
        assert!(
            !String::from_utf8_lossy(&c.stdout).contains("NO_FAULT"),
            "[{case}] the C did not fault — revisit the ERRORS.md G8 row"
        );
    }

    // `merror(NULL, ...)` does NOT fault: glibc's snprintf tolerates a NULL
    // format string here and writes nothing but the terminating NUL, so merror
    // emits just the "\n" of its own "%s\n". Checked both on a clean stack and
    // after a previous merror call has dirtied the (uninitialized in C,
    // zeroed in Rust) 256-byte buffer.
    let c = assert_helper_same("merror_null_template");
    assert_eq!(c.code, Some(0), "unexpected: {c:?}");
    assert_eq!(c.stderr, b"\n".to_vec());

    let c = assert_helper_same("merror_null_after_real");
    assert_eq!(c.code, Some(0), "unexpected: {c:?}");
    assert!(
        c.stderr.ends_with(b"\n\n"),
        "the NULL-template call must still emit only its own newline, \
         i.e. the stale buffer contents must not leak: {:?}",
        String::from_utf8_lossy(&c.stderr)
    );
}
