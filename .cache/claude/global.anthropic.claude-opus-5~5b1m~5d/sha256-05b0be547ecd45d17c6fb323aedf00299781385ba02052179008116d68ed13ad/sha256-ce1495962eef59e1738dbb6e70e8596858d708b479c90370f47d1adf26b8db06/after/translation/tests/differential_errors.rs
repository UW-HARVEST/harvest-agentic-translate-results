//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. The whole rejection surface of
//! `c_src/src/lib.c` is a single `if (...) abort();`, so "returning the same
//! error" means "dying from the same signal at the same point". Each case is
//! therefore run in a `fork()`ed child (core dumps disabled) and the *exact*
//! termination status of the C `.so` is compared with that of the Rust `.so` —
//! `SIGABRT` vs `SIGSEGV` vs normal return are all distinguished.

mod common;

use common::*;
use std::ffi::c_char;

/// `18446744073709551615UL / 2`, exactly as spelled in the C source.
const SIZE_MAX_HALF: usize = 18446744073709551615u64 as usize / 2;

fn expect_abort(ctx: &str, hex: *mut c_char, hex_maxlen: usize, bin: *const u8, bin_len: usize) {
    let out = diff_outcome(ctx, hex, hex_maxlen, bin, bin_len);
    assert_eq!(
        out,
        Outcome::Signaled(sys::SIGABRT),
        "{ctx}: expected abort()/SIGABRT from both, got {}",
        out.describe()
    );
}

fn expect_segv(ctx: &str, hex: *mut c_char, hex_maxlen: usize, bin: *const u8, bin_len: usize) {
    let out = diff_outcome(ctx, hex, hex_maxlen, bin, bin_len);
    assert_eq!(
        out,
        Outcome::Signaled(sys::SIGSEGV),
        "{ctx}: expected SIGSEGV from both, got {}",
        out.describe()
    );
}

// --------------------------------------------------------------------------
// Row 1 — cond A boundary: bin_len == SIZE_MAX/2
// --------------------------------------------------------------------------
#[test]
fn err01_bin_len_eq_size_max_half() {
    let mut buf = [0u8; 64];
    assert_eq!(SIZE_MAX_HALF, 0x7FFF_FFFF_FFFF_FFFF, "SIZE_MAX/2 constant");
    expect_abort(
        "err01",
        buf.as_mut_ptr() as *mut c_char,
        usize::MAX,
        buf.as_ptr(),
        SIZE_MAX_HALF,
    );
}

// --------------------------------------------------------------------------
// Row 2 — cond A: bin_len == SIZE_MAX/2 + 1
// --------------------------------------------------------------------------
#[test]
fn err02_bin_len_gt_size_max_half() {
    let mut buf = [0u8; 64];
    expect_abort(
        "err02",
        buf.as_mut_ptr() as *mut c_char,
        usize::MAX,
        buf.as_ptr(),
        SIZE_MAX_HALF + 1,
    );
}

// --------------------------------------------------------------------------
// Row 3 — cond A: bin_len == SIZE_MAX
// --------------------------------------------------------------------------
#[test]
fn err03_bin_len_size_max() {
    let mut buf = [0u8; 64];
    expect_abort(
        "err03",
        buf.as_mut_ptr() as *mut c_char,
        usize::MAX,
        buf.as_ptr(),
        usize::MAX,
    );
}

// --------------------------------------------------------------------------
// Row 4 — cond A fires independently of cond B
// --------------------------------------------------------------------------
#[test]
fn err04_cond_a_independent_of_cond_b() {
    let mut buf = [0u8; 64];
    // For each of these, `bin_len * 2` wraps modulo 2^64, so cond B alone
    // would be FALSE; only cond A rejects them.
    for &n in &[
        SIZE_MAX_HALF,
        SIZE_MAX_HALF + 1,
        0x8000_0000_0000_0000usize,
        0xFFFF_FFFF_FFFF_FFFEusize,
        usize::MAX,
    ] {
        let wrapped = n.wrapping_mul(2);
        expect_abort(
            &format!("err04/bin_len=0x{n:x} (bin_len*2 wraps to 0x{wrapped:x})"),
            buf.as_mut_ptr() as *mut c_char,
            usize::MAX,
            buf.as_ptr(),
            n,
        );
    }
}

// --------------------------------------------------------------------------
// Row 5 — cond A short-circuits before any dereference (NULL pointers)
// --------------------------------------------------------------------------
#[test]
fn err05_cond_a_precedes_deref() {
    for &n in &[SIZE_MAX_HALF, usize::MAX] {
        expect_abort(
            &format!("err05/bin_len=0x{n:x}"),
            std::ptr::null_mut(),
            usize::MAX,
            std::ptr::null(),
            n,
        );
    }
}

// --------------------------------------------------------------------------
// Row 6 — cond B with empty input: bin_len == 0, hex_maxlen == 0
// --------------------------------------------------------------------------
#[test]
fn err06_zero_len_zero_maxlen() {
    let mut buf = [0u8; 64];
    expect_abort("err06", buf.as_mut_ptr() as *mut c_char, 0, buf.as_ptr(), 0);
    // ... and with a NULL hex, proving the check precedes the `hex[0] = 0` store
    expect_abort("err06/null-hex", std::ptr::null_mut(), 0, buf.as_ptr(), 0);
}

// --------------------------------------------------------------------------
// Row 7 — cond B exact boundary: hex_maxlen == bin_len * 2
// --------------------------------------------------------------------------
#[test]
fn err07_maxlen_exactly_twice() {
    let mut buf = [0u8; 4096];
    for n in [1usize, 2, 3, 8, 17, 64, 255, 1024] {
        expect_abort(
            &format!("err07/n={n}"),
            buf.as_mut_ptr() as *mut c_char,
            2 * n,
            buf.as_ptr(),
            n,
        );
    }
}

// --------------------------------------------------------------------------
// Row 8 — cond B: hex_maxlen < bin_len * 2
// --------------------------------------------------------------------------
#[test]
fn err08_maxlen_less_than_twice() {
    let mut buf = [0u8; 4096];
    for (n, maxlen) in [(8usize, 3usize), (8, 15), (1, 1), (2, 1), (100, 99), (1024, 2047)] {
        expect_abort(
            &format!("err08/n={n} maxlen={maxlen}"),
            buf.as_mut_ptr() as *mut c_char,
            maxlen,
            buf.as_ptr(),
            n,
        );
    }
}

// --------------------------------------------------------------------------
// Row 9 — cond B: hex_maxlen == 0 with bin_len > 0
// --------------------------------------------------------------------------
#[test]
fn err09_zero_maxlen_nonzero_len() {
    let mut buf = [0u8; 64];
    for n in [1usize, 2, 7, 64] {
        expect_abort(
            &format!("err09/n={n}"),
            buf.as_mut_ptr() as *mut c_char,
            0,
            buf.as_ptr(),
            n,
        );
    }
}

// --------------------------------------------------------------------------
// Row 10 — cond B at large scale
// --------------------------------------------------------------------------
#[test]
fn err10_large_maxlen_exactly_twice() {
    let mut buf = [0u8; 64];
    for shift in [20u32, 32, 40, 62] {
        let n = 1usize << shift;
        expect_abort(
            &format!("err10/n=1<<{shift}"),
            buf.as_mut_ptr() as *mut c_char,
            n * 2,
            buf.as_ptr(),
            n,
        );
    }
}

// --------------------------------------------------------------------------
// Row 11 — cond B sweep: every rejecting hex_maxlen for small bin_len
// --------------------------------------------------------------------------
#[test]
fn err11_cond_b_exhaustive_sweep() {
    let f = impls();
    let mut buf = [0u8; 4096];
    let hex = buf.as_mut_ptr() as *mut c_char;
    let bin = buf.as_ptr();

    // Exhaustive for the small lengths: every hex_maxlen in 0..=2n must abort,
    // and 2n+1 must NOT.
    for n in 0usize..=96 {
        for maxlen in 0..=2 * n {
            let ctx = format!("err11/n={n} maxlen={maxlen}");
            let c = call_in_child(f.c, hex, maxlen, bin, n);
            let r = call_in_child(f.rust, hex, maxlen, bin, n);
            assert_eq!(c, r, "{ctx}: C {} vs Rust {}", c.describe(), r.describe());
            assert_eq!(
                c,
                Outcome::Signaled(sys::SIGABRT),
                "{ctx}: expected SIGABRT, got {}",
                c.describe()
            );
        }
        // one step inside the range: accepted by both
        let ctx = format!("err11/n={n} maxlen={} (valid)", 2 * n + 1);
        let c = call_in_child(f.c, hex, 2 * n + 1, bin, n);
        let r = call_in_child(f.rust, hex, 2 * n + 1, bin, n);
        assert_eq!(c, r, "{ctx}: C {} vs Rust {}", c.describe(), r.describe());
        assert_eq!(c, Outcome::Exited(0), "{ctx}: expected clean return, got {}", c.describe());
    }

    // Spot checks at larger lengths (full sweep would be pointlessly slow).
    for n in [31usize, 32, 63, 64, 255, 256, 1023, 1024] {
        for maxlen in [0usize, 1, 2, n, 2 * n - 1, 2 * n] {
            let ctx = format!("err11/large n={n} maxlen={maxlen}");
            let c = call_in_child(f.c, hex, maxlen, bin, n);
            let r = call_in_child(f.rust, hex, maxlen, bin, n);
            assert_eq!(c, r, "{ctx}: C {} vs Rust {}", c.describe(), r.describe());
            assert_eq!(
                c,
                Outcome::Signaled(sys::SIGABRT),
                "{ctx}: expected SIGABRT, got {}",
                c.describe()
            );
        }
    }
}

// --------------------------------------------------------------------------
// Row 12 — NULL hex passes the checks, then the `hex[0] = 0` store faults
// --------------------------------------------------------------------------
#[test]
fn err12_null_hex_writes_nul() {
    // bin_len == 0 so `bin` is never read; hex_maxlen == 1 > 0 so no abort.
    expect_segv("err12/n=0", std::ptr::null_mut(), 1, std::ptr::null(), 0);

    // Same with a valid `bin` and a positive length: still the NULL `hex` store.
    let buf = [0xABu8; 16];
    for n in [0usize, 1, 4] {
        expect_segv(
            &format!("err12/n={n}"),
            std::ptr::null_mut(),
            2 * n + 1,
            buf.as_ptr(),
            n,
        );
    }
}

// --------------------------------------------------------------------------
// Row 13 — NULL bin with bin_len > 0: the `bin[i]` load faults
// --------------------------------------------------------------------------
#[test]
fn err13_null_bin_read() {
    let mut buf = [0u8; 4096];
    for n in [1usize, 2, 64, 1024] {
        expect_segv(
            &format!("err13/n={n}"),
            buf.as_mut_ptr() as *mut c_char,
            2 * n + 1,
            std::ptr::null(),
            n,
        );
    }
}

// --------------------------------------------------------------------------
// Row 14 — one step INSIDE the range must not be rejected
// --------------------------------------------------------------------------
#[test]
fn err14_min_valid_maxlen_not_rejected() {
    let f = impls();
    let mut buf = [0u8; 8192];
    let hex = buf.as_mut_ptr() as *mut c_char;
    let bin = buf.as_ptr();
    for n in [0usize, 1, 2, 3, 8, 64, 255, 1024, 2048] {
        for maxlen in [2 * n + 1, 2 * n + 2, usize::MAX] {
            let ctx = format!("err14/n={n} maxlen={maxlen}");
            let c = call_in_child(f.c, hex, maxlen, bin, n);
            let r = call_in_child(f.rust, hex, maxlen, bin, n);
            assert_eq!(c, r, "{ctx}: C {} vs Rust {}", c.describe(), r.describe());
            assert_eq!(
                c,
                Outcome::Exited(0),
                "{ctx}: expected a clean return of `hex`, got {}",
                c.describe()
            );
        }
    }
    // largest bin_len that escapes cond A must NOT be rejected by cond A
    // (it is rejected by cond B unless hex_maxlen is astronomically large).
    let n = SIZE_MAX_HALF - 1;
    let c = call_in_child(f.c, hex, n.wrapping_mul(2), bin, n);
    let r = call_in_child(f.rust, hex, n.wrapping_mul(2), bin, n);
    assert_eq!(c, r, "err14/cond-a-boundary-1: C {} vs Rust {}", c.describe(), r.describe());
    assert_eq!(
        c,
        Outcome::Signaled(sys::SIGABRT),
        "err14/cond-a-boundary-1: expected cond B to reject, got {}",
        c.describe()
    );
}

// --------------------------------------------------------------------------
// Row 15 — bin_len = SIZE_MAX/2 - 1 escapes cond A and cond B, then the writes
//          run off the end of a guard-page-terminated `hex`
// --------------------------------------------------------------------------
#[test]
fn err15_oversized_len_runs_off_guard_page() {
    let f = impls();
    let ps = sys::page_size();
    let mut rng = Rng::new(SEED ^ 115);

    // The largest bin_len that does not trip cond A. With hex_maxlen ==
    // usize::MAX, cond B is also false (bin_len*2 == 0xFFFF...FC < MAX), so the
    // loop runs and walks off the end of `hex`.
    let bin_len = SIZE_MAX_HALF - 1;
    assert!(bin_len.wrapping_mul(2) < usize::MAX, "cond B must be false");

    // `bin` supplies real readable data for as long as `hex` lasts.
    let mut src = vec![0u8; ps];
    rng.fill(&mut src);

    let mut outs = Vec::new();
    let mut snaps = Vec::new();
    for (name, func) in [("C", f.c), ("Rust", f.rust)] {
        // MAP_SHARED so the parent can inspect what the child wrote before it died.
        let g = Guarded::new(1, true);
        g.fill(CANARY);
        let hex = g.ptr() as *mut c_char;
        let binp = src.as_ptr();
        let out = run_in_child(move || {
            unsafe { func(hex, usize::MAX, binp, bin_len) };
            0
        });
        assert_eq!(
            out,
            Outcome::Signaled(sys::SIGSEGV),
            "err15/{name}: expected SIGSEGV past the guard page, got {}",
            out.describe()
        );
        outs.push(out);
        snaps.push(g.snapshot());
    }
    assert_eq!(outs[0], outs[1], "err15: termination status mismatch");
    assert_buffers_eq("err15/bytes-written-before-fault", &snaps[0], &snaps[1]);
    // Sanity: the whole writable page really was filled before faulting.
    assert!(
        snaps[0].iter().all(|&b| b != CANARY),
        "err15: expected the full page to be written before the fault"
    );
}

// --------------------------------------------------------------------------
// Row 16 — `bin` runs into a PROT_NONE guard page
// --------------------------------------------------------------------------
#[test]
fn err16_bin_read_faults_at_guard_page() {
    let f = impls();
    let ps = sys::page_size();
    let mut rng = Rng::new(SEED ^ 116);

    // `bin` has exactly one readable page; we claim twice that length.
    let bin_len = 2 * ps;
    let hex_maxlen = 2 * bin_len + 1; // > bin_len*2, so cond B is false

    let mut outs = Vec::new();
    let mut snaps = Vec::new();
    let pattern: Vec<u8> = (0..ps).map(|_| rng.next_u8()).collect();

    for (name, func) in [("C", f.c), ("Rust", f.rust)] {
        let bin_g = Guarded::new(1, false);
        bin_g.copy_in(0, &pattern);
        // needs 2*ps writable bytes of output before the read fault
        let hex_g = Guarded::new(3, true);
        hex_g.fill(CANARY);
        let hex = hex_g.ptr() as *mut c_char;
        let binp = bin_g.ptr() as *const u8;
        let out = run_in_child(move || {
            unsafe { func(hex, hex_maxlen, binp, bin_len) };
            0
        });
        assert_eq!(
            out,
            Outcome::Signaled(sys::SIGSEGV),
            "err16/{name}: expected SIGSEGV reading past `bin`, got {}",
            out.describe()
        );
        outs.push(out);
        snaps.push(hex_g.snapshot()[..2 * ps].to_vec());
    }
    assert_eq!(outs[0], outs[1], "err16: termination status mismatch");
    assert_buffers_eq("err16/bytes-written-before-fault", &snaps[0], &snaps[1]);
    assert!(
        snaps[0].iter().all(|&b| b != CANARY),
        "err16: expected 2*page_size output bytes before the fault"
    );
}

// --------------------------------------------------------------------------
// Generic FFI-boundary boundaries beyond the table
// --------------------------------------------------------------------------

/// The API has no enum parameters (see `ERRORS.md`); the equivalent
/// "value with no valid variant" for this signature is an out-of-range length.
/// This sweeps the powers of two and their neighbours around both abort
/// conditions, in both `hex_maxlen` and `bin_len`.
#[test]
fn err_generic_length_boundary_sweep() {
    let f = impls();
    // Guard-page-terminated buffers: a combination that escapes both abort
    // conditions but is nonetheless nonsensical (e.g. bin_len = 2^40) must fault
    // deterministically in the child instead of silently corrupting memory, so
    // that C and Rust are compared on a well-defined observable.
    let hex_g = Guarded::new(4, false);
    let bin_g = Guarded::new(4, false);
    bin_g.fill(0x5A);
    let hex = hex_g.ptr() as *mut c_char;
    let bin = bin_g.ptr() as *const u8;

    let mut lens: Vec<usize> = Vec::new();
    for shift in 0..64u32 {
        let v = 1usize << shift;
        lens.push(v.wrapping_sub(1));
        lens.push(v);
        lens.push(v.wrapping_add(1));
    }
    lens.push(usize::MAX);
    lens.push(SIZE_MAX_HALF - 1);
    lens.push(SIZE_MAX_HALF);
    lens.push(SIZE_MAX_HALF + 1);
    lens.sort_unstable();
    lens.dedup();

    for &bin_len in &lens {
        let twice = bin_len.wrapping_mul(2);
        for &hex_maxlen in &[
            0usize,
            1,
            2,
            twice.wrapping_sub(1),
            twice,
            twice.wrapping_add(1),
            twice.wrapping_add(2),
            usize::MAX,
        ] {
            let ctx = format!("err_generic/bin_len=0x{bin_len:x} hex_maxlen={hex_maxlen}");
            let c = call_in_child(f.c, hex, hex_maxlen, bin, bin_len);
            let r = call_in_child(f.rust, hex, hex_maxlen, bin, bin_len);
            assert_eq!(c, r, "{ctx}: C {} vs Rust {}", c.describe(), r.describe());
            assert_ne!(c, Outcome::TimedOut, "{ctx}: both timed out");
        }
    }
}

/// All four NULL / non-NULL pointer combinations at the boundary lengths.
#[test]
fn err_generic_null_pointer_matrix() {
    let f = impls();
    let mut buf = [0u8; 64];
    let real_hex = buf.as_mut_ptr() as *mut c_char;
    let real_bin = buf.as_ptr();

    for &(hexp, hname) in &[(real_hex, "hex"), (std::ptr::null_mut(), "NULL")] {
        for &(binp, bname) in &[(real_bin, "bin"), (std::ptr::null(), "NULL")] {
            for &bin_len in &[0usize, 1, 2, SIZE_MAX_HALF, usize::MAX] {
                let twice = bin_len.wrapping_mul(2);
                for &hex_maxlen in &[0usize, 1, twice, twice.wrapping_add(1), usize::MAX] {
                    let ctx = format!(
                        "err_generic_null/hex={hname} bin={bname} n=0x{bin_len:x} maxlen={hex_maxlen}"
                    );
                    let c = call_in_child(f.c, hexp, hex_maxlen, binp, bin_len);
                    let r = call_in_child(f.rust, hexp, hex_maxlen, binp, bin_len);
                    assert_eq!(c, r, "{ctx}: C {} vs Rust {}", c.describe(), r.describe());
                    assert_ne!(c, Outcome::TimedOut, "{ctx}: both timed out");
                }
            }
        }
    }
}
