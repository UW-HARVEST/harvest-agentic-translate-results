//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! The only failure signal in this API is the `NULL` sentinel return value, so
//! "same error" means "both returned exactly `NULL`" (asserted as `NULL`-ness
//! equality, not merely "both failed somehow"), and for the *non*-rejecting
//! neighbours it means "both returned non-`NULL` with identical contents".

mod common;

use common::{Differ, Libs, Rng, SEED, n_bytes};
use std::ffi::c_char;

/// Assert both `.so`s return exactly `NULL` — the same rejection sentinel, not
/// merely "both failed somehow".
fn assert_both_null(d: &Differ<'_>, ctx: &str, size: i32, buf: Option<&[u8]>) {
    // First the ordinary differential comparison...
    match buf {
        None => d.assert_same_null(ctx, size),
        Some(b) => d.assert_same(ctx, size, b),
    }
    // ...then prove the shared result really is the NULL sentinel.
    let p = buf.map_or(std::ptr::null(), |b| b.as_ptr() as *const c_char);
    let cp = unsafe { d.call_c(size, p) };
    let rp = unsafe { d.call_rust(size, p) };
    assert!(cp.is_null(), "{ctx}: C must return NULL for size={size}");
    assert!(rp.is_null(), "{ctx}: Rust must return NULL for size={size}");
}

/* ================================================================== */
/* E1..E4 — `if (!src) return NULL;`  (lib.c:33)                       */
/* ================================================================== */

#[test]
fn e1_null_src_size_zero() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    assert_both_null(&d, "E1 NULL src, size=0", 0, None);
}

#[test]
fn e2_null_src_positive_size() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    for size in [1, 2, 3, 4, 5, 6, 7, 100, 4096, 1 << 20, i32::MAX] {
        assert_both_null(&d, "E2 NULL src, size>0", size, None);
    }
}

#[test]
fn e3_null_src_negative_size() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    for size in [-1, -2, -3, -4, -5, -100, -4096, -(1 << 30), i32::MIN] {
        assert_both_null(&d, "E3 NULL src, size<0", size, None);
    }
}

#[test]
fn e4_null_src_overflow_size() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    // Sizes that would make calloc fail (E5/E6) *if* the pointer were valid:
    // the NULL check at lib.c:33 must short-circuit first, so the result is
    // still NULL and no allocation is attempted.
    for size in [
        -4,
        -1000,
        536_870_912,
        1_073_741_820,
        -1_500_000_000,
        i32::MIN,
        i32::MAX,
    ] {
        assert_both_null(&d, "E4 NULL src + calloc-failing size", size, None);
    }
}

/* ================================================================== */
/* E5 — `if (!out) return NULL;` reached via negative size (lib.c:42)  */
/* ================================================================== */

#[test]
fn e5_calloc_fails_negative_n_from_negative_size() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 5);

    // Documented triggers, each with its computed n.
    let cases: [(i32, i32); 8] = [
        (-4, -1),
        (-5, -2),
        (-6, -4),
        (-7, -5),
        (-100, -129),
        (-1000, -1329),
        (-536_870_912, -715_827_878),
        (-1_500_000_000, -568_344_230),
    ];
    for (size, want_n) in cases {
        assert_eq!(n_bytes(size), want_n, "n model for size={size}");
        assert!(want_n < 0, "size={size} must produce a negative n");
        let buf = rng.bytes(16);
        assert_both_null(&d, "E5 calloc fails (negative n)", size, Some(&buf));
    }

    // randomized: every size <= -4 down to -2^29 has n < 0
    for _ in 0..2000 {
        let size = -(rng.range(4, 536_870_912) as i32);
        if n_bytes(size) >= 0 {
            continue;
        }
        let nlen = rng.range(1, 32) as usize;
        let buf = rng.bytes(nlen);
        assert_both_null(&d, "E5 calloc fails (random negative)", size, Some(&buf));
    }
    println!("E5: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* E6 — `if (!out) return NULL;` reached via positive int overflow      */
/* ================================================================== */

#[test]
fn e6_calloc_fails_negative_n_from_int_overflow() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 6);

    // size in [2^29, 1073741820] -> size*4 overflows int negative -> n < 0.
    let cases: [(i32, i32); 5] = [
        (536_870_912, -715_827_878),
        (600_000_000, -631_655_761),
        (700_000_000, -498_322_428),
        (1_000_000_000, -98_322_428),
        (1_073_741_820, -1),
    ];
    for (size, want_n) in cases {
        assert_eq!(n_bytes(size), want_n, "n model for size={size}");
        assert!(want_n < 0);
        // Only a 1-byte buffer is supplied on purpose: the C code must bail out
        // at lib.c:42 before ever dereferencing src.
        assert_both_null(&d, "E6 calloc fails (int overflow)", size, Some(&[0xAB]));
    }

    for _ in 0..2000 {
        let size = rng.range(536_870_912, 1_073_741_820) as i32;
        if n_bytes(size) > 0 {
            continue; // would enter the read loop -> C UB, not an error path
        }
        assert_both_null(&d, "E6 calloc fails (random overflow)", size, Some(&[0xAB]));
    }
    println!("E6: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* G1 — null pointer x every interesting size                          */
/* ================================================================== */

#[test]
fn g1_null_pointer_matrix() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let sizes = [
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        4,
        -4,
        5,
        -5,
        63,
        64,
        65,
        1 << 29,
        1 << 30,
        -(1 << 29),
        -(1 << 30),
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
    ];
    for size in sizes {
        assert_both_null(&d, "G1 null pointer matrix", size, None);
    }
    assert!(d.calls.get() as usize >= sizes.len());
}

/* ================================================================== */
/* G2 / G3 — zero length                                               */
/* ================================================================== */

#[test]
fn g2_zero_length() {
    let libs = Libs::load();
    let d = Differ::new(&libs);

    // size == 0 and strlen("") == 0 -> n = 4, nothing emitted
    assert_eq!(n_bytes(0), 4);
    d.assert_same("G2 empty string", 0, &[0u8]);

    let out = d.c_output(0, &[0u8]).expect("C must not return NULL");
    assert_eq!(out.len(), 4, "n must be 4");
    assert_eq!(out, vec![0u8; 4], "no bytes may be emitted");
    assert_eq!(d.rust_output(0, &[0u8]).unwrap(), out);
}

#[test]
fn g3_zero_length_leading_nul() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 33);

    for _ in 0..500 {
        let tail = rng.range(1, 64) as usize;
        let mut buf = vec![0u8]; // leading NUL: strlen stops immediately
        buf.extend(rng.bytes_in(tail, 0x01, 0xFF));
        d.assert_same("G3 leading NUL", 0, &buf);
        let out = d.c_output(0, &buf).unwrap();
        assert_eq!(out, vec![0u8; 4], "trailing data must not leak in");
    }
    println!("G3: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* G4 — oversized / overflowing lengths                                */
/* ================================================================== */

#[test]
fn g4_oversized_lengths() {
    let libs = Libs::load();
    let d = Differ::new(&libs);

    // Well-defined oversized values: n <= 0, so calloc fails and the C code
    // returns NULL *before* touching the (too small) source buffer.
    let mut tested = 0;
    for size in [
        1 << 29,
        (1 << 29) + 1,
        700_000_000,
        900_000_000,
        1_073_741_819,
        1_073_741_820,
    ] {
        assert!(n_bytes(size) <= 0, "size={size} should make calloc fail");
        assert_both_null(&d, "G4 oversized (calloc fails)", size, Some(&[0x41]));
        tested += 1;
    }
    assert_eq!(tested, 6);

    // Documented as untestable-by-construction: these compute a small positive
    // n and then enter the read loop, overrunning both buffers. That is C
    // undefined behaviour (a segfault would kill the test process), not a
    // reported error path. We assert the *model* here so the expectation is
    // recorded; the Rust side computes n with the same wrapping int arithmetic,
    // which is pinned down differentially by E5/E6 and C21-C24.
    for (size, want_n) in [
        (1_073_741_821i32, 0i32),
        (1_073_741_822, 2),
        (1_073_741_823, 3),
        (1 << 30, 4),
        (i32::MAX, 3),
        (1_500_000_000, 568_344_238),
    ] {
        assert_eq!(
            n_bytes(size),
            want_n,
            "n model for oversized size={size} (UB in C, not invoked)"
        );
        assert!(want_n >= 0, "these are exactly the dangerous ones");
    }
    println!("G4: {} differential calls (plus 6 UB values asserted by model)", d.calls.get());
}

/* ================================================================== */
/* G5 — one step past the valid range on the negative side             */
/* ================================================================== */

#[test]
fn g5_negative_size_boundary() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let buf = [0x11u8, 0x22, 0x33, 0x44];

    // -1, -2, -3: n = 3, 2, 0 -> calloc SUCCEEDS (glibc calloc(1,0) is non-NULL)
    for (size, want_n) in [(-1i32, 3i32), (-2, 2), (-3, 0)] {
        assert_eq!(n_bytes(size), want_n);
        d.assert_same("G5 negative boundary (calloc ok)", size, &buf);
        let out = d.c_output(size, &buf).expect("C must not return NULL");
        assert_eq!(out.len(), want_n as usize);
        assert!(out.iter().all(|&b| b == 0));
        assert_eq!(d.rust_output(size, &buf).unwrap(), out, "size={size}");
    }

    // one step further: -4 flips calloc to failure
    assert_eq!(n_bytes(-4), -1);
    assert_both_null(&d, "G5 negative boundary (calloc fails)", -4, Some(&buf));

    // and the flip is exactly at -4, not -5
    assert!(n_bytes(-3) >= 0 && n_bytes(-4) < 0);
    println!("G5: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* G6 — INT_MIN and -2^30 wrap size*4 to exactly zero                  */
/* ================================================================== */

#[test]
fn g6_int_min_and_neg_2_30() {
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let mut rng = Rng::new(SEED ^ 66);

    for size in [i32::MIN, -(1 << 30)] {
        assert_eq!(n_bytes(size), 4, "size*4 must wrap to 0 for size={size}");
        for _ in 0..50 {
            let nlen = rng.range(1, 40) as usize;
            let buf = rng.bytes(nlen);
            d.assert_same("G6 wrap-to-zero", size, &buf);
        }
        let buf = rng.bytes(8);
        let out = d.c_output(size, &buf).expect("C must not return NULL");
        assert_eq!(out, vec![0u8; 4], "loop must never run for size={size}");
        assert_eq!(d.rust_output(size, &buf).unwrap(), out);
    }
    println!("G6: {} differential calls", d.calls.get());
}

/* ================================================================== */
/* G7 — out-of-range enum values                                       */
/* ================================================================== */

#[test]
fn g7_no_enum_parameters_full_int_domain_instead() {
    // This API takes no enum (nor any flag/mode) parameter -- the only scalar is
    // `int size`, whose entire int domain is a legal input. As the substitute
    // for "out-of-range enum variant", sweep the *structural* classes of the
    // int domain and assert C/Rust agree on which ones are rejected.
    let libs = Libs::load();
    let d = Differ::new(&libs);
    let buf = [0x5Au8; 8];

    let classes: [(&str, i32); 10] = [
        ("zero (strlen mode)", 0),
        ("min positive", 1),
        ("in-range positive", 8),
        ("negative, calloc ok", -1),
        ("negative, calloc ok (n==0)", -3),
        ("negative, calloc fails", -4),
        ("negative extreme", i32::MIN),
        ("negative wrap to n==4", -(1 << 30)),
        ("positive overflow, calloc fails", 1 << 29),
        ("positive overflow, calloc fails (top)", 1_073_741_820),
    ];

    for (name, size) in classes {
        let ctx = format!("G7 {name}");
        if size == 0 {
            // strlen mode needs a NUL-terminated buffer
            d.assert_same(&ctx, 0, b"abcdefg\0");
            continue;
        }
        d.assert_same(&ctx, size, &buf);
    }
    println!("G7: {} differential calls", d.calls.get());
}
