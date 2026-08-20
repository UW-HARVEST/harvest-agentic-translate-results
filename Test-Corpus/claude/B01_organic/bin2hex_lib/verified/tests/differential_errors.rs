//! Phase C — error-path differential tests (one test per `ERRORS.md` row).
//!
//! The only rejection in the library is `abort()`, and the only other failure
//! mode is a segfault from the UB the C code deliberately allows.  Both kill the
//! process, so each call runs in a `fork()`ed child and the *exact* termination
//! status (`WTERMSIG` / `WEXITSTATUS`) of the C child is compared with the Rust
//! child's — not merely "both failed".

mod common;

use common::*;
use std::ffi::c_char;

fn scratch() -> Vec<u8> {
    vec![0xAAu8; 4096]
}

fn expect_abort(label: &str, hex_maxlen: usize, bin_len: usize) {
    let mut buf = scratch();
    let mut bin = scratch();
    let out = diff_outcome(
        label,
        buf.as_mut_ptr().cast(),
        hex_maxlen,
        bin.as_mut_ptr(),
        bin_len,
    );
    assert_eq!(
        out,
        Outcome::Signaled(libc::SIGABRT),
        "[{label}] expected SIGABRT from both implementations, got {}",
        out.describe()
    );
}

/// E1 — `bin_len == SIZE_MAX / 2` (the exact `>=` boundary of term 1), with
/// term 2 false.
#[test]
fn e1_bin_len_at_limit() {
    assert_eq!(C_LIMIT, 9223372036854775807usize);
    expect_abort("E1", SIZE_MAX, C_LIMIT);
}

/// E2 — `bin_len == SIZE_MAX`.
#[test]
fn e2_bin_len_size_max() {
    expect_abort("E2", SIZE_MAX, SIZE_MAX);
}

/// E3 — further over-range `bin_len` values.
#[test]
fn e3_bin_len_over_range() {
    for bin_len in [
        C_LIMIT + 1,
        0x8000_0000_0000_0000usize,
        0x8000_0000_0000_0001usize,
        SIZE_MAX - 1,
    ] {
        expect_abort(&format!("E3/{bin_len:#x}"), SIZE_MAX, bin_len);
        // also with a small hex_maxlen (both terms true)
        expect_abort(&format!("E3/{bin_len:#x}/small"), 1, bin_len);
    }
}

/// E1' — one step *below* the term-1 boundary must NOT abort (proves the
/// boundary is `>=`, not `>`): `bin_len = SIZE_MAX/2 - 1` with `hex_maxlen`
/// smaller than `2*bin_len` falls through to term 2 (covered by E7), while a
/// small `bin_len` passes both checks (covered by Phase B).  Here we pin the
/// closest accepted `bin_len` values that still abort only via term 2.
#[test]
fn e1b_just_below_limit_uses_term2() {
    for bin_len in [C_LIMIT - 1, C_LIMIT - 2, C_LIMIT / 2] {
        // hex_maxlen <= 2*bin_len -> abort through the *second* term
        expect_abort(&format!("E1b/{bin_len:#x}"), 0, bin_len);
        expect_abort(&format!("E1b/{bin_len:#x}/1"), 1, bin_len);
    }
}

/// E4 — `hex_maxlen == bin_len * 2` exactly: room for the digits but not for the
/// NUL terminator.
#[test]
fn e4_hex_maxlen_equals_exactly_needed_digits() {
    for bin_len in [1usize, 2, 3, 4, 7, 8, 15, 16, 255, 256, 1024] {
        expect_abort(&format!("E4/{bin_len}"), bin_len * 2, bin_len);
    }
}

/// E5 — `hex_maxlen < bin_len * 2`.
#[test]
fn e5_hex_maxlen_too_small() {
    let mut rng = Rng::new(0x0500_0001);
    for bin_len in [1usize, 2, 3, 4, 8, 33, 256] {
        for hex_maxlen in [0usize, 1, bin_len, bin_len * 2 - 1] {
            expect_abort(&format!("E5/{bin_len}/{hex_maxlen}"), hex_maxlen, bin_len);
        }
        // randomized within the rejecting range
        for _ in 0..4 {
            let hex_maxlen = rng.below(bin_len * 2 + 1); // 0 ..= 2*bin_len
            expect_abort(&format!("E5r/{bin_len}/{hex_maxlen}"), hex_maxlen, bin_len);
        }
    }
}

/// E6 — degenerate empty input with `hex_maxlen == 0`: `0 <= 0` is true.
#[test]
fn e6_empty_zero_maxlen() {
    expect_abort("E6", 0, 0);
}

/// E7 — short-circuit ordering: term 1 false, term 2 true, with a `bin_len` so
/// large that any read of `bin` would segfault.  Getting SIGABRT (not SIGSEGV)
/// proves the checks run — in the same order — before the loop.
#[test]
fn e7_short_circuit_order() {
    let mut buf = scratch();
    for bin_len in [C_LIMIT - 1, C_LIMIT - 3] {
        for hex_maxlen in [0usize, 1, 2, 1024] {
            let out = diff_outcome(
                "E7",
                buf.as_mut_ptr().cast(),
                hex_maxlen,
                std::ptr::null(), // never dereferenced: validation aborts first
                bin_len,
            );
            assert_eq!(
                out,
                Outcome::Signaled(libc::SIGABRT),
                "[E7] expected SIGABRT (validation before loop), got {}",
                out.describe()
            );
        }
    }
}

/// E8 — term 1 false *and* term 2 false with an absurd `bin_len`: the C code
/// accepts it and runs off the end of both buffers.  The UB must reproduce
/// identically (both die with the same signal).
#[test]
fn e8_accepted_but_walks_off_buffers() {
    let mut buf = vec![0xAAu8; 1 << 16];
    let bin = vec![0x5Au8; 1 << 16];
    let bin_len = C_LIMIT - 1; // term 1 false
    // hex_maxlen = SIZE_MAX > 2*bin_len == 0xFFFF_FFFF_FFFF_FFFC -> term 2 false
    assert!(SIZE_MAX > bin_len.wrapping_mul(2));
    let out = diff_outcome(
        "E8",
        buf.as_mut_ptr().cast(),
        SIZE_MAX,
        bin.as_ptr(),
        bin_len,
    );
    assert_eq!(
        out,
        Outcome::Signaled(libc::SIGSEGV),
        "[E8] expected SIGSEGV from both implementations, got {}",
        out.describe()
    );
}

/// G1 — `hex == NULL` with validation passing: `hex[0] = 0` faults.
#[test]
fn g1_null_hex_passes_validation_then_faults() {
    let bin = scratch();
    for (hex_maxlen, bin_len) in [(1usize, 0usize), (3, 1), (9, 4)] {
        let out = diff_outcome(
            "G1",
            std::ptr::null_mut::<c_char>(),
            hex_maxlen,
            bin.as_ptr(),
            bin_len,
        );
        assert_eq!(
            out,
            Outcome::Signaled(libc::SIGSEGV),
            "[G1] expected SIGSEGV, got {}",
            out.describe()
        );
    }
}

/// G2 — `bin == NULL` with `bin_len == 0` is *valid* (never dereferenced): both
/// must return normally and write the NUL.
#[test]
fn g2_null_bin_empty_is_valid() {
    let f = impls();
    for hex_maxlen in [1usize, 2, 64, SIZE_MAX] {
        // survives in a forked child ...
        let mut buf = scratch();
        let out = diff_outcome(
            "G2",
            buf.as_mut_ptr().cast(),
            hex_maxlen,
            std::ptr::null(),
            0,
        );
        assert_eq!(
            out,
            Outcome::Exited(0),
            "[G2] expected clean exit, got {}",
            out.describe()
        );

        // ... and produces identical bytes in-process
        let mut c_buf = [0xAAu8; 8];
        let mut r_buf = [0xAAu8; 8];
        unsafe {
            (f.c)(c_buf.as_mut_ptr().cast(), hex_maxlen, std::ptr::null(), 0);
            (f.r)(r_buf.as_mut_ptr().cast(), hex_maxlen, std::ptr::null(), 0);
        }
        assert_eq!(c_buf, r_buf, "[G2] output mismatch");
        assert_eq!(c_buf[0], 0, "[G2] NUL terminator not written");
    }
}

/// G3 — `bin == NULL` with `bin_len > 0`: the first loop iteration faults.
#[test]
fn g3_null_bin_nonempty_faults() {
    let mut buf = scratch();
    for bin_len in [1usize, 2, 16] {
        let out = diff_outcome(
            "G3",
            buf.as_mut_ptr().cast(),
            bin_len * 2 + 1,
            std::ptr::null(),
            bin_len,
        );
        assert_eq!(
            out,
            Outcome::Signaled(libc::SIGSEGV),
            "[G3] expected SIGSEGV, got {}",
            out.describe()
        );
    }
}

/// G4 — both pointers NULL but `hex_maxlen == 0`: validation fires *first*, so
/// this must be SIGABRT, not SIGSEGV.
#[test]
fn g4_null_pointers_but_invalid_lengths_abort_first() {
    for (hex_maxlen, bin_len) in [(0usize, 0usize), (0, 4), (8, 4), (0, SIZE_MAX)] {
        let out = diff_outcome(
            "G4",
            std::ptr::null_mut::<c_char>(),
            hex_maxlen,
            std::ptr::null(),
            bin_len,
        );
        assert_eq!(
            out,
            Outcome::Signaled(libc::SIGABRT),
            "[G4] expected SIGABRT for hex_maxlen={hex_maxlen} bin_len={bin_len}, got {}",
            out.describe()
        );
    }
}

/// G5/G6 — the accepted side of the boundary: `hex_maxlen == 2*bin_len + 1`
/// (exact minimum) and `hex_maxlen == SIZE_MAX` must both succeed identically,
/// including for `bin_len == 0`.  This pins down the `<=` in term 2.
#[test]
fn g5_g6_accepted_boundary() {
    let f = impls();
    let mut rng = Rng::new(0x0600_0001);
    for bin_len in [0usize, 1, 2, 3, 8, 64, 255] {
        let mut bin = vec![0u8; bin_len.max(1)];
        rng.fill(&mut bin);
        for hex_maxlen in [bin_len * 2 + 1, bin_len * 2 + 2, SIZE_MAX] {
            let need = bin_len * 2 + 1;
            let mut c_buf = vec![0xAAu8; need + 4];
            let mut r_buf = vec![0xAAu8; need + 4];
            unsafe {
                let c_ret = (f.c)(c_buf.as_mut_ptr().cast(), hex_maxlen, bin.as_ptr(), bin_len);
                let r_ret = (f.r)(r_buf.as_mut_ptr().cast(), hex_maxlen, bin.as_ptr(), bin_len);
                assert_eq!(c_ret as *const u8, c_buf.as_ptr());
                assert_eq!(r_ret as *const u8, r_buf.as_ptr());
            }
            assert_eq!(
                c_buf, r_buf,
                "[G6] mismatch at bin_len={bin_len} hex_maxlen={hex_maxlen}"
            );
            assert_eq!(c_buf[need - 1], 0, "[G6] missing NUL");
            assert_eq!(c_buf[need], 0xAA, "[G6] wrote past hex[2*bin_len]");
        }
        // one step below the accepted minimum aborts
        expect_abort(&format!("G6/reject/{bin_len}"), bin_len * 2, bin_len);
    }
}

/// Sweep of the whole `hex_maxlen` decision boundary for small `bin_len`:
/// every value from 0 to `2*bin_len + 3` must be classified identically
/// (abort vs. clean exit) by C and Rust.
#[test]
fn boundary_sweep_hex_maxlen() {
    let bin = vec![0x5Au8; 16];
    for bin_len in 0usize..=8 {
        for hex_maxlen in 0usize..=(bin_len * 2 + 3) {
            let mut buf = scratch();
            let out = diff_outcome(
                &format!("sweep/{bin_len}/{hex_maxlen}"),
                buf.as_mut_ptr().cast(),
                hex_maxlen,
                bin.as_ptr(),
                bin_len,
            );
            let expected = if hex_maxlen <= bin_len * 2 {
                Outcome::Signaled(libc::SIGABRT)
            } else {
                Outcome::Exited(0)
            };
            assert_eq!(
                out, expected,
                "sweep: bin_len={bin_len} hex_maxlen={hex_maxlen} -> {}",
                out.describe()
            );
        }
    }
}

/// Randomized classification fuzz over the whole argument space (valid and
/// invalid mixed), comparing abort-vs-succeed decisions between C and Rust.
#[test]
fn randomized_validation_fuzz() {
    let mut rng = Rng::new(0xF022_0001);
    let bin = vec![0xC3u8; 1024];
    for _ in 0..300 {
        let bin_len = rng.below(64);
        // hex_maxlen drawn from a range that straddles the decision boundary
        let hex_maxlen = rng.below(2 * 64 + 4);
        let mut buf = scratch();
        diff_outcome(
            &format!("fuzz/{bin_len}/{hex_maxlen}"),
            buf.as_mut_ptr().cast(),
            hex_maxlen,
            bin.as_ptr(),
            bin_len,
        );
    }
    // and a few extreme size_t values
    for _ in 0..40 {
        let bin_len = match rng.below(4) {
            0 => rng.next_u64() as usize,
            1 => SIZE_MAX - rng.below(4),
            2 => C_LIMIT + rng.below(4) - 2,
            _ => rng.below(8),
        };
        let hex_maxlen = match rng.below(3) {
            0 => SIZE_MAX,
            1 => rng.next_u64() as usize,
            _ => rng.below(32),
        };
        // Skip the (rare) combination that is *accepted* with a huge bin_len,
        // which is E8's slow buffer walk rather than a validation decision.
        if bin_len < C_LIMIT && hex_maxlen > bin_len.wrapping_mul(2) && bin_len > 1024 {
            continue;
        }
        let mut buf = scratch();
        diff_outcome(
            &format!("fuzz2/{bin_len:#x}/{hex_maxlen:#x}"),
            buf.as_mut_ptr().cast(),
            hex_maxlen,
            bin.as_ptr(),
            bin_len,
        );
    }
}
