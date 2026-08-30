//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. `driver` returns `void` and has no error
//! channel, so "the same rejection" means: the same *termination status* (normal
//! exit vs. the exact terminating signal number) and the same stdout bytes. Each
//! call therefore runs in a forked child whose status is compared, not merely
//! "both failed somehow".

mod common;

use common::*;
use std::ffi::c_char;
use std::ptr;

const NULLP: *const c_char = ptr::null();

// --- row 1: s1 == NULL, s2 valid -------------------------------------------
#[test]
fn err_01_s1_null() {
    for s2 in [&b"a"[..], b"abc", b"\xff\x01", &all_nonzero_bytes()] {
        let b = CBuf::new(s2);
        let res = assert_same_fork(NULLP, b.ptr(), "s1=NULL, s2 valid");
        assert_eq!(
            res.outcome,
            Outcome::Signaled(libc::SIGSEGV),
            "expected SIGSEGV dereferencing a NULL s1, got {:?}",
            res.outcome
        );
        assert!(res.stdout.is_empty(), "nothing should be printed");
    }
}

// --- row 2: s1 == NULL, s2 == "" -------------------------------------------
#[test]
fn err_02_s1_null_s2_empty() {
    let b = CBuf::new(b"");
    let res = assert_same_fork(NULLP, b.ptr(), "s1=NULL, s2=\"\"");
    assert_eq!(res.outcome, Outcome::Signaled(libc::SIGSEGV));
    assert!(res.stdout.is_empty());
}

// --- row 3: s1 valid non-empty, s2 == NULL ---------------------------------
#[test]
fn err_03_s2_null() {
    for s1 in [&b"a"[..], b"abc", b"\x80", b"the quick brown fox"] {
        let a = CBuf::new(s1);
        let res = assert_same_fork(a.ptr(), NULLP, "s1 valid non-empty, s2=NULL");
        assert_eq!(
            res.outcome,
            Outcome::Signaled(libc::SIGSEGV),
            "expected SIGSEGV reading the reject set from NULL, got {:?}",
            res.outcome
        );
        assert!(res.stdout.is_empty());
    }
}

// --- row 4: s1 == "", s2 == NULL  (NO short-circuit: still faults) ----------
#[test]
fn err_04_s2_null_s1_empty() {
    // Empirically determined C behaviour: the reject set is consumed BEFORE s1 is
    // examined, so an empty s1 does NOT save a NULL s2 from being dereferenced.
    // This is the case that caught the original translation bug.
    let a = CBuf::new(b"");
    let res = assert_same_fork(a.ptr(), NULLP, "s1=\"\", s2=NULL");
    assert_eq!(
        res.outcome,
        Outcome::Signaled(libc::SIGSEGV),
        "s2 must be dereferenced even for an empty s1, got {:?}",
        res.outcome
    );
    assert!(res.stdout.is_empty());
}

// --- row 5: both NULL -------------------------------------------------------
#[test]
fn err_05_both_null() {
    let res = assert_same_fork(NULLP, NULLP, "s1=NULL, s2=NULL");
    assert_eq!(res.outcome, Outcome::Signaled(libc::SIGSEGV));
    assert!(res.stdout.is_empty());
}

// --- row 6: s1 not NUL-terminated ------------------------------------------
#[test]
fn err_06_s1_unterminated() {
    for &len in &[1usize, 7, 64, 4096, 5000] {
        // s1 is all 'a', s2 shares no byte with it, so the scan runs off the end.
        let body: Vec<u8> = std::iter::repeat_n(b'a', len).collect();
        let u = UnterminatedString::new(&body);
        let b = CBuf::new(b"Z");
        let res = assert_same_fork(u.ptr(), b.ptr(), "s1 unterminated");
        assert_eq!(
            res.outcome,
            Outcome::Signaled(libc::SIGSEGV),
            "expected SIGSEGV running off an unterminated s1 (len {len}), got {:?}",
            res.outcome
        );
        assert!(res.stdout.is_empty());
    }
}

// --- row 7: s2 not NUL-terminated ------------------------------------------
#[test]
fn err_07_s2_unterminated() {
    for &len in &[1usize, 7, 64, 4096] {
        // s2 is all high bytes, s1 is ASCII, so the reject-set scan never finds a
        // match and runs off the end of the mapping.
        let body: Vec<u8> = std::iter::repeat_n(0xC3u8, len).collect();
        let u = UnterminatedString::new(&body);
        let a = CBuf::new(b"abc");
        let res = assert_same_fork(a.ptr(), u.ptr(), "s2 unterminated");
        assert_eq!(
            res.outcome,
            Outcome::Signaled(libc::SIGSEGV),
            "expected SIGSEGV running off an unterminated s2 (len {len}), got {:?}",
            res.outcome
        );
        assert!(res.stdout.is_empty());
    }
}

// --- row 7b: s2 unterminated with an EMPTY s1 (still faults) ---------------
#[test]
fn err_07b_s2_unterminated_empty_s1() {
    // The answer (0) is knowable without reading s2 at all, but the C library
    // still scans the reject set first and therefore still dies. Checked at
    // several lengths so a "reads only the first byte or two" implementation
    // cannot pass.
    for &len in &[1usize, 2, 3, 4, 64, 4096, 5000] {
        let body: Vec<u8> = std::iter::repeat_n(b'x', len).collect();
        let u = UnterminatedString::new(&body);
        let a = CBuf::new(b"");
        let res = assert_same_fork(a.ptr(), u.ptr(), "s2 unterminated, s1 empty");
        assert_eq!(
            res.outcome,
            Outcome::Signaled(libc::SIGSEGV),
            "reject set must be scanned unconditionally (len {len}), got {:?}",
            res.outcome
        );
        assert!(res.stdout.is_empty());
    }
}

/// The mirror of row 7b: a *properly terminated* s2 flush against a guard page
/// must NOT fault even at the shortest lengths — this pins the read EXTENT, so
/// the fix for row 7b cannot be "read s2[0] and s2[1] unconditionally".
#[test]
fn err_07c_s2_read_extent_exact() {
    for len in 0..8usize {
        let body: Vec<u8> = std::iter::repeat_n(b'x', len).collect();
        let g = GuardedString::new(&body);
        let a = CBuf::new(b"");
        let res = assert_same_fork(a.ptr(), g.ptr(), "guarded s2, empty s1");
        assert_eq!(
            res.outcome,
            Outcome::Exited(0),
            "over-read past s2's NUL at reject length {len}: {:?}",
            res.outcome
        );
        assert_eq!(res.stdout, b"0\n".to_vec());
    }
}

// --- row 8: s1 NUL flush against a guard page (must NOT fault) -------------
#[test]
fn err_08_page_boundary_s1() {
    let mut rng = Rng::new(0x0808);
    for _ in 0..100 {
        let len = rng.range(1, 300);
        // no match
        let body = rng.bytes_from(len, &(1u8..=127).collect::<Vec<u8>>());
        let g = GuardedString::new(&body);
        let b = CBuf::new(&(128u8..=200).collect::<Vec<u8>>());
        let res = assert_same_fork(g.ptr(), b.ptr(), "s1 at page boundary, no match");
        assert_eq!(
            res.outcome,
            Outcome::Exited(0),
            "over-read past the NUL of s1: {:?}",
            res.outcome
        );
        assert_eq!(res.stdout, format!("{len}\n").into_bytes());

        // match somewhere
        let idx = rng.below(len);
        let mut m = body.clone();
        m[idx] = 0xAA;
        let g2 = GuardedString::new(&m);
        let b2 = CBuf::new(b"\xaa");
        let res2 = assert_same_fork(g2.ptr(), b2.ptr(), "s1 at page boundary, with match");
        assert_eq!(res2.outcome, Outcome::Exited(0));
        assert_eq!(res2.stdout, format!("{idx}\n").into_bytes());
    }
}

// --- row 9: s2 NUL flush against a guard page (must NOT fault) -------------
#[test]
fn err_09_page_boundary_s2() {
    let mut rng = Rng::new(0x0909);
    for _ in 0..100 {
        let m = rng.range(1, 200);
        let s2 = rng.bytes_from(m, &(128u8..=255).collect::<Vec<u8>>());
        let g = GuardedString::new(&s2);
        let n = rng.range(1, 60);
        let s1 = rng.bytes_from(n, &(1u8..=127).collect::<Vec<u8>>());
        let a = CBuf::new(&s1);
        let res = assert_same_fork(a.ptr(), g.ptr(), "s2 at page boundary");
        assert_eq!(
            res.outcome,
            Outcome::Exited(0),
            "over-read past the NUL of s2: {:?}",
            res.outcome
        );
        assert_eq!(res.stdout, format!("{n}\n").into_bytes());
    }
}

// --- row 10: 1-byte empty s1 at the last byte of a page -------------------
#[test]
fn err_10_page_boundary_empty_s1() {
    // GuardedString::new(b"") puts a lone NUL at the last readable byte.
    let g = GuardedString::new(b"");
    for s2 in [&b""[..], b"a", &all_nonzero_bytes()] {
        let b = CBuf::new(s2);
        let res = assert_same_fork(g.ptr(), b.ptr(), "empty s1 at page boundary");
        assert_eq!(
            res.outcome,
            Outcome::Exited(0),
            "empty s1 at a page boundary must not fault: {:?}",
            res.outcome
        );
        assert_eq!(res.stdout, b"0\n".to_vec());
    }
    // ... and with s2 == NULL, which DOES fault (row 4: no short-circuit).
    let res = assert_same_fork(g.ptr(), NULLP, "empty guarded s1, s2=NULL");
    assert_eq!(res.outcome, Outcome::Signaled(libc::SIGSEGV));
    assert!(res.stdout.is_empty());
}

// --- row 11: huge result value --------------------------------------------
#[test]
fn err_11_huge_length() {
    for &len in &[1usize << 20, (1 << 20) + 7, 1 << 22] {
        let s1: Vec<u8> = std::iter::repeat_n(b'a', len).collect();
        let a = CBuf::new(&s1);
        let b = CBuf::new(b"Z");
        let c = c_out(a.ptr(), b.ptr());
        let r = rust_out(a.ptr(), b.ptr());
        assert_eq!(c, r, "divergence at len {len}");
        assert_eq!(
            c,
            format!("{len}\n").into_bytes(),
            "%zu truncated a large size_t at len {len}"
        );
    }
}

// --- row 12: s2 longer than s1, no match (O(n*m) worst case) ---------------
#[test]
fn err_12_no_match_long_s2() {
    let mut rng = Rng::new(0x1212);
    for _ in 0..200 {
        let n = rng.range(1, 64);
        let s1 = rng.bytes_from(n, &(1u8..=127).collect::<Vec<u8>>());
        let s2 = rng.bytes_from_range(200, 2000, &(128u8..=255).collect::<Vec<u8>>());
        let a = CBuf::new(&s1);
        let b = CBuf::new(&s2);
        let c = c_out(a.ptr(), b.ptr());
        let r = rust_out(a.ptr(), b.ptr());
        assert_eq!(c, r);
        assert_eq!(c, format!("{n}\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// Generic boundary conditions required regardless of the table.
// ---------------------------------------------------------------------------

/// There is no enum/int/flag parameter on this API, so the analogue of an
/// "out-of-range enum value crossing the FFI boundary" is an arbitrary
/// never-valid *pointer* value. Both implementations must react identically.
#[test]
fn generic_wild_pointer_values() {
    let valid = CBuf::new(b"abc");
    // A mix of classic bogus pointer values: NULL, small non-canonical addresses,
    // a misaligned odd address, and a non-canonical high address.
    let wild: [usize; 7] = [
        0,
        1,
        7,
        0xdead,
        0xffff_ffff,
        usize::MAX,
        usize::MAX & !0xfff,
    ];
    for &w in &wild {
        let p = w as *const c_char;
        // as s1
        let a = assert_same_fork(p, valid.ptr(), "wild pointer as s1");
        assert!(
            matches!(a.outcome, Outcome::Signaled(_)),
            "wild s1 {w:#x} unexpectedly survived: {:?}",
            a.outcome
        );
        // as s2 (with non-empty s1, so s2 is actually dereferenced)
        let b = assert_same_fork(valid.ptr(), p, "wild pointer as s2");
        assert!(
            matches!(b.outcome, Outcome::Signaled(_)),
            "wild s2 {w:#x} unexpectedly survived: {:?}",
            b.outcome
        );
        // as s2 with an EMPTY s1 -> still faults: the reject set is read first.
        let empty = CBuf::new(b"");
        let c = assert_same_fork(empty.ptr(), p, "wild pointer as s2 with empty s1");
        assert!(
            matches!(c.outcome, Outcome::Signaled(_)),
            "wild s2 {w:#x} with empty s1 unexpectedly survived: {:?}",
            c.outcome
        );
    }
}

/// Zero-length inputs are valid, not errors — assert both agree they are benign.
#[test]
fn generic_zero_length_inputs() {
    for (s1, s2) in [
        (&b""[..], &b""[..]),
        (b"", b"a"),
        (b"a", b""),
        (b"", &all_nonzero_bytes()[..]),
    ] {
        let a = CBuf::new(s1);
        let b = CBuf::new(s2);
        let res = assert_same_fork(a.ptr(), b.ptr(), "zero-length input");
        assert_eq!(res.outcome, Outcome::Exited(0));
        assert_eq!(
            res.stdout,
            format!("{}\n", strcspn_ref(s1, s2)).into_bytes()
        );
    }
}

/// One step past the only "range" this API has: the byte-value domain. 0x00
/// terminates, 0x01 is the first legal byte, 0xFF the last. Sweep the edges.
#[test]
fn generic_byte_value_range_edges() {
    for &edge in &[0x01u8, 0x02, 0x7e, 0x7f, 0x80, 0x81, 0xfe, 0xff] {
        // edge as the only byte of s1, swept against every possible s2 byte.
        for b in 1u8..=255 {
            assert_same_and_eq(&[edge], &[b], if edge == b { 0 } else { 1 });
        }
    }
    // An interior 0x00 is *not* representable in a C string: verify both
    // implementations stop at it identically (the byte after it is unreachable).
    let buf = b"ab\0cd\0";
    let p = buf.as_ptr() as *const c_char;
    let s2 = CBuf::new(b"cd");
    let c = c_out(p, s2.ptr());
    let r = rust_out(p, s2.ptr());
    assert_eq!(c, r);
    assert_eq!(c, b"2\n".to_vec(), "must stop at the interior NUL");
}
