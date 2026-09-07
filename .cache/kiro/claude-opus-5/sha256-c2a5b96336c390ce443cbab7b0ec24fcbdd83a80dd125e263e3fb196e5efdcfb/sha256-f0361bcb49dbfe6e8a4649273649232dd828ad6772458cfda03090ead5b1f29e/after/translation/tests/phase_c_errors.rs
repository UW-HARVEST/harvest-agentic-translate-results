//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Because the C rejects by calling `abort()`
//! (process termination), each case is executed in a forked child and the
//! comparison is on the exact termination signal / exit status — not merely
//! "both failed somehow".

mod common;

use common::*;

// --- E1..E4: first guard, `bin_len >= LIMIT` -----------------------------
// These abort before any dereference, so null pointers are safe here; we still
// pass real buffers where the row says so.

#[test]
fn e1_bin_len_exactly_limit() {
    assert_same_term(
        "E1",
        Ptr::Buf(64),
        usize::MAX,
        Ptr::Buf(64),
        LIMIT,
        Term::Signaled(SIGABRT),
    );
}

#[test]
fn e2_bin_len_limit_plus_one() {
    assert_same_term(
        "E2",
        Ptr::Buf(64),
        usize::MAX,
        Ptr::Buf(64),
        LIMIT + 1,
        Term::Signaled(SIGABRT),
    );
}

#[test]
fn e3_bin_len_size_max() {
    assert_same_term(
        "E3",
        Ptr::Buf(64),
        usize::MAX,
        Ptr::Buf(64),
        usize::MAX,
        Term::Signaled(SIGABRT),
    );
}

#[test]
fn e4_first_guard_dominates() {
    assert_same_term(
        "E4",
        Ptr::Buf(64),
        0,
        Ptr::Buf(64),
        LIMIT,
        Term::Signaled(SIGABRT),
    );
}

// --- E5..E11: second guard, `hex_maxlen <= bin_len * 2` -----------------

#[test]
fn e5_hex_maxlen_equals_twice_bin_len() {
    assert_same_term("E5", Ptr::Buf(64), 8, Ptr::Buf(64), 4, Term::Signaled(SIGABRT));
}

#[test]
fn e6_hex_maxlen_one_below() {
    assert_same_term("E6", Ptr::Buf(64), 7, Ptr::Buf(64), 4, Term::Signaled(SIGABRT));
}

#[test]
fn e7_zero_len_zero_maxlen() {
    assert_same_term("E7", Ptr::Buf(64), 0, Ptr::Buf(64), 0, Term::Signaled(SIGABRT));
}

#[test]
fn e8_zero_maxlen_one_byte() {
    assert_same_term("E8", Ptr::Buf(64), 0, Ptr::Buf(64), 1, Term::Signaled(SIGABRT));
}

#[test]
fn e9_maxlen_one_one_byte() {
    assert_same_term("E9", Ptr::Buf(64), 1, Ptr::Buf(64), 1, Term::Signaled(SIGABRT));
}

#[test]
fn e10_maxlen_two_one_byte() {
    assert_same_term("E10", Ptr::Buf(64), 2, Ptr::Buf(64), 1, Term::Signaled(SIGABRT));
}

#[test]
fn e11_second_guard_full_sweep() {
    for bin_len in 0usize..=32 {
        for hex_maxlen in 0..=bin_len * 2 {
            assert_same_term(
                &format!("E11[bin_len={bin_len},hex_maxlen={hex_maxlen}]"),
                Ptr::Buf(128),
                hex_maxlen,
                Ptr::Buf(128),
                bin_len,
                Term::Signaled(SIGABRT),
            );
        }
    }
}

// --- E12: negative control, one step into the valid range --------------

#[test]
fn e12_one_step_past_guard_is_accepted() {
    for bin_len in 0usize..=32 {
        assert_same_term(
            &format!("E12[bin_len={bin_len}]"),
            Ptr::Buf(bin_len * 2 + 1),
            bin_len * 2 + 1,
            Ptr::Buf(bin_len.max(1)),
            bin_len,
            Term::Exited(0),
        );
    }
}

// --- E13: first-guard boundary observed from the other side ------------
// At `LIMIT - 1` the first guard does NOT fire, so the loop is entered and
// dereferences the null `bin`/`hex` -> memory fault, not SIGABRT. Together with
// E1 this pins the constant exactly.

#[test]
fn e13_limit_minus_one_enters_loop_and_faults() {
    let l = libs();
    let c = term_of(
        l.c_bin2hex,
        std::ptr::null_mut(),
        usize::MAX,
        std::ptr::null(),
        LIMIT - 1,
    );
    let r = term_of(
        l.rust_bin2hex,
        std::ptr::null_mut(),
        usize::MAX,
        std::ptr::null(),
        LIMIT - 1,
    );
    assert_eq!(c, r, "[E13] termination differs: C={c:?} Rust={r:?}");
    assert_ne!(
        c,
        Term::Signaled(SIGABRT),
        "[E13] guard must NOT fire at LIMIT-1 (got SIGABRT)"
    );
    assert!(
        matches!(c, Term::Signaled(SIGSEGV) | Term::Signaled(SIGBUS)),
        "[E13] expected a memory fault, got {c:?}"
    );
}

// --- E14..E18: null-pointer boundaries --------------------------------

#[test]
fn e14_null_hex_zero_len() {
    assert_same_term("E14", Ptr::Null, 1, Ptr::Buf(64), 0, Term::Signaled(SIGSEGV));
}

#[test]
fn e15_null_hex_nonzero_len() {
    assert_same_term("E15", Ptr::Null, 17, Ptr::Buf(64), 8, Term::Signaled(SIGSEGV));
}

#[test]
fn e16_null_bin_nonzero_len() {
    assert_same_term("E16", Ptr::Buf(64), 17, Ptr::Null, 8, Term::Signaled(SIGSEGV));
}

#[test]
fn e17_null_bin_zero_len_is_legal() {
    // The loop never runs, so `bin` is never dereferenced: both sides must
    // return normally. Also check the produced bytes are identical.
    assert_same_term("E17", Ptr::Buf(1), 1, Ptr::Null, 0, Term::Exited(0));
    assert_same("E17-bytes", 1, 0, 1, &[], 0, 0);
}

#[test]
fn e18_both_pointers_null_zero_len() {
    assert_same_term("E18", Ptr::Null, 1, Ptr::Null, 0, Term::Signaled(SIGSEGV));
}

// --- E19: hex_maxlen lies about the buffer size -----------------------

#[test]
fn e19_oversized_hex_maxlen_writes_only_needed_bytes() {
    let mut rng = Rng::new(0xE019);
    for bin_len in 0usize..=32 {
        let mut bin = vec![0u8; bin_len];
        rng.fill(&mut bin);
        // Buffer is exactly bin_len*2+1 bytes, but we claim SIZE_MAX. The whole
        // buffer is compared, so any extra write would either differ or fault.
        assert_same(
            &format!("E19[bin_len={bin_len}]"),
            bin_len * 2 + 1,
            0,
            usize::MAX,
            &bin,
            0,
            bin_len,
        );
        assert_same_term(
            &format!("E19-term[bin_len={bin_len}]"),
            Ptr::Buf(bin_len * 2 + 1),
            usize::MAX,
            Ptr::Buf(bin_len.max(1)),
            bin_len,
            Term::Exited(0),
        );
    }
}

// --- E20: out-of-range enum across FFI --------------------------------
// The C API declares no `enum` parameter; the four parameters are two pointers
// and two `size_t`s, whose full value ranges are covered above. This test
// documents that mechanically by asserting the extreme `size_t` values behave
// identically, including the ones with no "valid" meaning.

#[test]
fn e20_no_enum_params_extreme_size_t_values() {
    let extremes = [
        0usize,
        1,
        2,
        LIMIT - 1,
        LIMIT,
        LIMIT + 1,
        usize::MAX - 1,
        usize::MAX,
    ];
    for &bin_len in &extremes {
        for &hex_maxlen in &extremes {
            // Only the cases that abort before dereferencing are safe to run
            // with small buffers; the rest are covered by E12..E19.
            if bin_len >= LIMIT || hex_maxlen <= bin_len.saturating_mul(2) {
                assert_same_term(
                    &format!("E20[bin_len={bin_len},hex_maxlen={hex_maxlen}]"),
                    Ptr::Buf(128),
                    hex_maxlen,
                    Ptr::Buf(128),
                    bin_len,
                    Term::Signaled(SIGABRT),
                );
            }
        }
    }
}
