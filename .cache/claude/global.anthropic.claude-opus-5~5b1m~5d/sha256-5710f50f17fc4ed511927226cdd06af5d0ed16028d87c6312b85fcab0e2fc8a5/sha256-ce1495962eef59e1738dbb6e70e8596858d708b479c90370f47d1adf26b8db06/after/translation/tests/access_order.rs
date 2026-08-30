//! Argument-ACCESS-ORDER regression tests.
//!
//! `strcspn`'s *result* is order-independent, but which pointer it dereferences
//! first — and how far — is observable through faults, and it is where the
//! original translation diverged from the C. This file pins the order and the
//! read extents down empirically, comparing C against Rust for each case.
//!
//! Established ground truth (see ERRORS.md "Divergence found and fixed"):
//!   1. `s2` (the reject set) is consumed IN FULL before `s1` is examined.
//!   2. `s2` is read up to and including its NUL, never past it.
//!   3. `s1` is read only up to the byte that stops the scan, never past it.

mod common;

use common::*;
use std::ffi::c_char;

const NULLP: *const c_char = std::ptr::null();

/// Asserts C and Rust agree, and returns the (shared) outcome.
fn both(s1: *const c_char, s2: *const c_char, what: &str) -> ForkResult {
    assert_same_fork(s1, s2, what)
}

/// (1) The reject set is dereferenced even when `s1` is empty and the answer is
///     already determined.
#[test]
fn order_s2_is_consumed_before_s1_is_examined() {
    let empty = CBuf::new(b"");

    // A NULL reject set kills the call despite the empty s1.
    let r = both(empty.ptr(), NULLP, "s1=\"\" s2=NULL");
    assert_eq!(
        r.outcome,
        Outcome::Signaled(libc::SIGSEGV),
        "s2 must be read first"
    );

    // So does an unterminated one, at every length: this rules out an
    // implementation that peeks at only the first byte or two of s2.
    for &n in &[1usize, 2, 3, 4, 8, 64, 4096, 5000] {
        let u = UnterminatedString::new(&vec![b'x'; n]);
        let r = both(empty.ptr(), u.ptr(), "s1=\"\" s2=unterminated");
        assert_eq!(
            r.outcome,
            Outcome::Signaled(libc::SIGSEGV),
            "the whole reject set must be scanned (len {n})"
        );
    }
}

/// (2) ... but not one byte further than its NUL.
#[test]
fn order_s2_read_extent_is_exactly_up_to_its_nul() {
    let empty = CBuf::new(b"");
    let abc = CBuf::new(b"abc");
    for n in 0..16usize {
        let g = GuardedString::new(&vec![b'x'; n]);
        // with an empty s1
        let r = both(empty.ptr(), g.ptr(), "guarded s2, empty s1");
        assert_eq!(
            r.outcome,
            Outcome::Exited(0),
            "over-read past s2's NUL (reject len {n}, empty s1)"
        );
        assert_eq!(r.stdout, b"0\n".to_vec());

        // and with a non-empty s1 that shares no byte with s2
        let r = both(abc.ptr(), g.ptr(), "guarded s2, s1=abc");
        assert_eq!(
            r.outcome,
            Outcome::Exited(0),
            "over-read past s2's NUL (reject len {n}, s1=abc)"
        );
        assert_eq!(r.stdout, b"3\n".to_vec());
    }
}

/// (3) `s1` is read only as far as the byte that stops the scan.
#[test]
fn order_s1_read_extent_stops_at_nul_or_match() {
    // NUL flush against a guard page: no over-read allowed.
    for n in 0..16usize {
        let g = GuardedString::new(&vec![b'a'; n]);
        let s2 = CBuf::new(b"Z");
        let r = both(g.ptr(), s2.ptr(), "guarded s1, no match");
        assert_eq!(r.outcome, Outcome::Exited(0), "over-read past s1's NUL (len {n})");
        assert_eq!(r.stdout, format!("{n}\n").into_bytes());
    }

    // A *match* must also stop the scan before the unmapped page: build an
    // unterminated s1 whose LAST byte is in the reject set, so a correct
    // implementation stops exactly there and an over-reading one dies.
    for &n in &[1usize, 2, 8, 64, 4096] {
        let mut body = vec![b'a'; n];
        body[n - 1] = b'Z';
        let u = UnterminatedString::new(&body);
        let s2 = CBuf::new(b"Z");
        let r = both(u.ptr(), s2.ptr(), "unterminated s1 with match at last byte");
        assert_eq!(
            r.outcome,
            Outcome::Exited(0),
            "scan must stop at the match without reading further (len {n})"
        );
        assert_eq!(r.stdout, format!("{}\n", n - 1).into_bytes());
    }
}

/// An unterminated `s1` still dies once the reject set is valid, for reject
/// lengths on both sides of glibc's internal algorithm switch (0/1 vs >= 2).
#[test]
fn order_unterminated_s1_faults_for_every_reject_length() {
    for m in 0..6usize {
        let s2 = CBuf::new(&vec![b'Z'; m]);
        let u = UnterminatedString::new(&vec![b'a'; 64]);
        let r = both(u.ptr(), s2.ptr(), "unterminated s1");
        assert_eq!(
            r.outcome,
            Outcome::Signaled(libc::SIGSEGV),
            "unterminated s1 should fault (reject len {m})"
        );
    }
}

/// A NULL `s1` faults for every reject length, once `s2` has been consumed.
#[test]
fn order_null_s1_faults_for_every_reject_length() {
    for m in 0..6usize {
        let s2 = CBuf::new(&vec![b'Z'; m]);
        let r = both(NULLP, s2.ptr(), "s1=NULL");
        assert_eq!(
            r.outcome,
            Outcome::Signaled(libc::SIGSEGV),
            "NULL s1 should fault (reject len {m})"
        );
    }
    let r = both(NULLP, NULLP, "both NULL");
    assert_eq!(r.outcome, Outcome::Signaled(libc::SIGSEGV));
}
