//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! The only failure signal the C API has is a `NULL` return, produced by the
//! five allocation sites (E1–E5). Those are reached by running the call in a
//! `fork()`ed child whose address space is capped with `setrlimit(RLIMIT_AS)`
//! *after* the (large) inputs have been built in the parent, so the failing
//! allocation is the one inside the library.
//!
//! E6–E8 are the NULL-pointer arguments the C never checks: the correct
//! translation must fault exactly like the C rather than "helpfully" returning
//! `NULL`, so the children's termination *signals* are compared.
//!
//! E9 is the zero-length-needle infinite loop.

mod common;

use common::*;
use std::ffi::CString;
use std::time::Duration;

/// Size of the allocation we force the library to attempt: comfortably larger
/// than the slack we leave in the child's address space.
const BIG: usize = 64 * 1024 * 1024;
/// Head-room granted to the child on top of its current virtual size. Small
/// enough that a `BIG` allocation must fail, large enough that the runtime
/// bookkeeping of the call itself cannot fail.
const SLACK: u64 = 8 * 1024 * 1024;

fn big_run(byte: u8) -> Vec<u8> {
    vec![byte; BIG]
}

// ---------------------------------------------------------------- E1 ---------
/// `malloc` @ lib.c:34 — the leading-context buffer of `inx_start + 1` bytes.
/// `orig` = 64 MiB of 'a' followed by "X"; `search` = "X" matches at index
/// 64 MiB, so the very first allocation is the huge one.
#[test]
fn err_e1_malloc_leading_fails() {
    let mut o = big_run(b'a');
    o.push(b'X');
    let o = CString::new(o).unwrap();
    let s = CString::new("X").unwrap();
    let v = CString::new("y").unwrap();
    let out = diff_under_rlimit("E1", SLACK, &o, &s, &v);
    assert_eq!(
        out,
        Outcome::Exited(CHILD_NULL),
        "E1: both implementations must return NULL when the leading malloc fails"
    );
}

// ---------------------------------------------------------------- E2 ---------
/// `realloc` @ lib.c:45 — the replacement-value append. `inx_start == 0` so the
/// leading `malloc` is skipped and `realloc(NULL, 1 + value_len)` is the first
/// (huge) allocation.
#[test]
fn err_e2_realloc_value_fails() {
    let o = CString::new("X").unwrap();
    let s = CString::new("X").unwrap();
    let v = CString::new(big_run(b'y')).unwrap();
    let out = diff_under_rlimit("E2", SLACK, &o, &s, &v);
    assert_eq!(
        out,
        Outcome::Exited(CHILD_NULL),
        "E2: both implementations must return NULL when the value realloc fails"
    );
}

// ---------------------------------------------------------------- E3 ---------
/// `realloc` @ lib.c:62 — the between-matches gap append. Two matches with a
/// 64 MiB gap: the first small allocations succeed, the gap `realloc` fails.
#[test]
fn err_e3_realloc_gap_fails() {
    let mut o = Vec::with_capacity(BIG + 4);
    o.push(b'X');
    o.extend_from_slice(&big_run(b'a'));
    o.push(b'X');
    let o = CString::new(o).unwrap();
    let s = CString::new("X").unwrap();
    let v = CString::new("y").unwrap();
    let out = diff_under_rlimit("E3", SLACK, &o, &s, &v);
    assert_eq!(
        out,
        Outcome::Exited(CHILD_NULL),
        "E3: both implementations must return NULL when the gap realloc fails"
    );
}

// ---------------------------------------------------------------- E4 ---------
/// `realloc` @ lib.c:80 — the trailing-context append. One match at index 0
/// followed by a 64 MiB tail.
#[test]
fn err_e4_realloc_tail_fails() {
    let mut o = Vec::with_capacity(BIG + 2);
    o.push(b'X');
    o.extend_from_slice(&big_run(b'a'));
    let o = CString::new(o).unwrap();
    let s = CString::new("X").unwrap();
    let v = CString::new("y").unwrap();
    let out = diff_under_rlimit("E4", SLACK, &o, &s, &v);
    assert_eq!(
        out,
        Outcome::Exited(CHILD_NULL),
        "E4: both implementations must return NULL when the tail realloc fails"
    );
}

// ---------------------------------------------------------------- E5 ---------
/// `strdup` @ lib.c:24 — the no-match fast path.
#[test]
fn err_e5_strdup_nomatch_fails() {
    let o = CString::new(big_run(b'a')).unwrap();
    let s = CString::new("X").unwrap(); // absent from `orig`
    let v = CString::new("y").unwrap();
    let out = diff_under_rlimit("E5", SLACK, &o, &s, &v);
    assert_eq!(
        out,
        Outcome::Exited(CHILD_NULL),
        "E5: both implementations must return NULL when strdup(orig) fails"
    );
}

// ------------------------------------------------------------- E6/E7/E8/E10 --
/// The C dereferences all three pointers unconditionally (`strlen` @ 11-13).
/// A faithful translation must fault the same way; in particular it must *not*
/// return `NULL` (that would be a behaviour change) and must not turn the fault
/// into a Rust panic/`abort` (`SIGABRT` != `SIGSEGV`).
fn null_arg_case(row: &str, orig_null: bool, search_null: bool, value_null: bool) {
    let o = CString::new("aXbXc").unwrap();
    let s = CString::new("X").unwrap();
    let v = CString::new("y").unwrap();
    let op = if orig_null {
        std::ptr::null()
    } else {
        o.as_ptr()
    };
    let sp = if search_null {
        std::ptr::null()
    } else {
        s.as_ptr()
    };
    let vp = if value_null {
        std::ptr::null()
    } else {
        v.as_ptr()
    };

    let l = libs();
    let run = |f: SarFn| {
        run_in_child(None, Duration::from_secs(20), move || unsafe {
            let p = f(op, sp, vp);
            if p.is_null() {
                CHILD_NULL
            } else {
                CHILD_NONNULL
            }
        })
    };

    let oc = run(l.c);
    let or_ = run(l.rust);
    assert_eq!(
        oc, or_,
        "[{row}] DIVERGENCE on NULL argument(s) \
         (orig_null={orig_null}, search_null={search_null}, value_null={value_null}): \
         C -> {oc:?}, Rust -> {or_:?}"
    );
    // Document (and pin) what the C actually does: it faults.
    assert_eq!(
        oc,
        Outcome::Signaled(libc::SIGSEGV),
        "[{row}] expected SIGSEGV from the unchecked dereference, got {oc:?}"
    );
}

#[test]
fn err_e6_null_orig_segv() {
    null_arg_case("E6", true, false, false);
}

#[test]
fn err_e7_null_search_segv() {
    null_arg_case("E7", false, true, false);
}

#[test]
fn err_e8_null_value_segv() {
    null_arg_case("E8", false, false, true);
}

#[test]
fn err_e10_all_null_segv() {
    null_arg_case("E10", true, true, true);
}

// ---------------------------------------------------------------- E9 ---------
/// Zero-length `search`: `strstr(h, "")` returns `h`, `inx_start` never
/// advances, and `while (p != NULL)` never terminates.
///
/// (a) With `value_len > 0` the loop `realloc`s unboundedly; under a capped
///     address space it terminates by returning `NULL`.
#[test]
fn err_e9a_empty_search_exhausts_memory() {
    let o = CString::new("hello world").unwrap();
    let s = CString::new("").unwrap();
    // 1 MiB per iteration, so ~8 iterations exhaust the slack: fast and
    // deterministic instead of 8 million tiny reallocs.
    let v = CString::new(vec![b'y'; 1024 * 1024]).unwrap();
    let out = diff_under_rlimit("E9a", SLACK, &o, &s, &v);
    assert_eq!(
        out,
        Outcome::Exited(CHILD_NULL),
        "E9a: the unbounded realloc loop must end in NULL for both"
    );
}

/// (b) With `value_len == 0` nothing grows and the loop spins forever. Both
///     implementations must hang identically (asserted with a bounded timeout).
#[test]
fn err_e9b_empty_search_and_empty_value_hangs() {
    let o = CString::new("hello world").unwrap();
    let s = CString::new("").unwrap();
    let v = CString::new("").unwrap();
    let l = libs();
    let (op, sp, vp) = (o.as_ptr(), s.as_ptr(), v.as_ptr());
    let run = |f: SarFn| {
        run_in_child(None, Duration::from_millis(1500), move || unsafe {
            let p = f(op, sp, vp);
            if p.is_null() {
                CHILD_NULL
            } else {
                CHILD_NONNULL
            }
        })
    };
    let oc = run(l.c);
    let or_ = run(l.rust);
    assert_eq!(
        oc, or_,
        "E9b DIVERGENCE for search==\"\" && value==\"\": C -> {oc:?}, Rust -> {or_:?}"
    );
    assert_eq!(
        oc,
        Outcome::Timeout,
        "E9b: expected non-termination (the C `while (p != NULL)` loop never exits), got {oc:?}"
    );
}

// ------------------------------------------- generic boundary sweeps ---------
/// Zero-length inputs are *valid*, not errors — but assert the pair agrees on
/// every combination of empty/one-byte arguments (excluding the empty `search`
/// cases, which are E9's non-termination).
#[test]
fn err_boundary_empty_and_one_byte_combinations() {
    let candidates: [&[u8]; 4] = [b"", b"a", b"X", b"aXa"];
    for o in candidates {
        for s in candidates {
            if s.is_empty() {
                continue; // E9: non-terminating in the C, covered above
            }
            for v in candidates {
                diff("boundary", o, s, v);
            }
        }
    }
}

/// Every single byte value 0x01..=0xFF as a one-byte needle against a haystack
/// containing it and one that does not: covers the whole `char` domain the API
/// accepts (there are no enums or integers in this API to push out of range;
/// the byte domain is its entire scalar input space).
#[test]
fn err_boundary_full_byte_domain() {
    for b in 1u8..=255u8 {
        let s = [b];
        // present exactly once, in the middle
        let o = [b'a', b, b'a'];
        diff("boundary-byte", &o, &s, b"QQ");
        // present at both ends
        let o2 = [b, b'a', b];
        diff("boundary-byte", &o2, &s, b"");
        // absent (0x01 is never 'a')
        let o3 = [b'a', b'a', b'a'];
        diff("boundary-byte", &o3, &s, b"Q");
        // haystack is a single copy of the byte
        diff("boundary-byte", &s, &s, b"Q");
    }
}
