//! Phase C — ERROR-PATH differential tests, ERRORS.md rows 1–61.
//!
//! For every invalid-input condition we construct that exact condition, drive
//! BOTH the C `.so` and the Rust `.so`, and assert they agree on the *observable
//! error surface*: identical integer return / NULL sentinel / errno, or —
//! for the `sodium_misuse()`→`abort()` paths — the identical process fate in a
//! forked child (`Fate::Signaled(6)` for `SIGABRT`).
//!
//! Build configuration note (drives which rows are reachable): the C `.so` in
//! `c_src/build` is compiled with `C_DEFINES = -Dsodium_EXPORTS` and NO `HAVE_*`
//! feature macros (verified in `build/CMakeFiles/sodium.dir/flags.make`).
//! Consequently:
//!   * `HAVE_ALIGNED_MALLOC` is undefined  → `_sodium_malloc` is plain `malloc`;
//!     the guarded ENOMEM/canary/misuse paths (rows 45/46/47/48) never compile.
//!   * `HAVE_MLOCK`/`HAVE_MPROTECT` undefined → mlock/munlock/mprotect set ENOSYS.
//!   * `HAVE_PTHREAD`/`HAVE_ATOMIC_OPS`/`_WIN32` all undefined → `sodium_crit_enter`
//!     / `sodium_crit_leave` are the trivial `return 0;` variants, so the
//!     `EPERM` "leave while unlocked" path (row 33) never compiles.
//! These facts are re-stated on the individual tests / unreachable-row notes.

mod common;
use common::*;

// ---- errno numbers on Linux (per task spec) ----
const EINVAL: i32 = 22;
const ERANGE: i32 = 34;
const ENOMEM: i32 = 12;
const ENOSYS: i32 = 38;
// EPERM=1, EFBIG=27 are referenced in notes only.

// ---- base64 variant constants (from sodium/utils.h) ----
const B64_ORIGINAL: i32 = 1;
const B64_ORIGINAL_NO_PADDING: i32 = 3;
const B64_URLSAFE: i32 = 5;
const B64_URLSAFE_NO_PADDING: i32 = 7;

// ---- exact C signatures (from include/sodium/utils.h) ----
type Bin2Hex = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> *mut u8;
type Hex2Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    usize,
    *const u8,
    *mut usize,
    *mut *const u8,
) -> i32;
type Bin2B64 = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, i32) -> *mut u8;
type B642Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    usize,
    *const u8,
    *mut usize,
    *mut *const u8,
    i32,
) -> i32;
type Ip2Bin = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
type Bin2Ip = unsafe extern "C" fn(*mut u8, usize, *const u8) -> *mut u8;
type Mlock = unsafe extern "C" fn(*mut core::ffi::c_void, usize) -> i32;
type Malloc = unsafe extern "C" fn(usize) -> *mut core::ffi::c_void;
type Allocarray = unsafe extern "C" fn(usize, usize) -> *mut core::ffi::c_void;
type FreeFn = unsafe extern "C" fn(*mut core::ffi::c_void);
type Pad = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> i32;
type Unpad = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> i32;
type Verify = unsafe extern "C" fn(*const u8, *const u8) -> i32;
type Memcmp = unsafe extern "C" fn(*const core::ffi::c_void, *const core::ffi::c_void, usize) -> i32;
type Compare = unsafe extern "C" fn(*const u8, *const u8, usize) -> i32;
type IsZero = unsafe extern "C" fn(*const u8, usize) -> i32;
type CritLeave = unsafe extern "C" fn() -> i32;

// ===========================================================================
// sodium/codecs.c — hex
// ===========================================================================

/// ERRORS.md rows 1, 2: `sodium_bin2hex`.
///  * Row 1: `hex_maxlen <= bin_len*2` → `sodium_misuse()` → abort (observed as
///    identical child fate).
///  * Row 2: `bin_len >= SIZE_MAX/2` → also `misuse`, but a real allocation is
///    impossible on a 64-bit host; marked `unreachable (64-bit)` in ERRORS.md.
///    We still exercise the `bin_len >= SIZE_MAX/2` guard directly (it aborts
///    before touching memory), confirming C and Rust abort identically.
#[test]
fn bin2hex_output_too_small_aborts() {
    let d = duo();
    let (cf, rf) = d.pair::<Bin2Hex>("sodium_bin2hex");
    let cf = *cf;
    let rf = *rf;

    // Row 1: buffer exactly one byte too small (need bin_len*2 + 1).
    for bin_len in [1usize, 2, 8, 32] {
        let too_small = bin_len * 2; // needs > bin_len*2, so this misuses
        same_fate(
            &format!("bin2hex row1 too-small (bin_len={bin_len})"),
            || {
                let bin = vec![0xABu8; bin_len];
                let mut out = vec![0u8; too_small + 8];
                unsafe { cf(out.as_mut_ptr(), too_small, bin.as_ptr(), bin_len) };
            },
            || {
                let bin = vec![0xABu8; bin_len];
                let mut out = vec![0u8; too_small + 8];
                unsafe { rf(out.as_mut_ptr(), too_small, bin.as_ptr(), bin_len) };
            },
        );
    }

    // Row 2 (bin_len >= SIZE_MAX/2): the `||` short-circuits into misuse before
    // any memory access. Confirms identical abort on the overflow guard too.
    same_fate(
        "bin2hex row2 bin_len>=SIZE_MAX/2 (abort before access)",
        || {
            let mut out = [0u8; 4];
            unsafe { cf(out.as_mut_ptr(), 4, out.as_ptr(), usize::MAX / 2) };
        },
        || {
            let mut out = [0u8; 4];
            unsafe { rf(out.as_mut_ptr(), 4, out.as_ptr(), usize::MAX / 2) };
        },
    );
}

/// ERRORS.md rows 3, 4, 5: `sodium_hex2bin`.
///  * Row 3: a non-hex char that is not in `ignore` → returns -1, errno EINVAL
///    (via the `hex_end == NULL && hex_pos != hex_len` trailing check).
///  * Row 4: more hex pairs than `bin_maxlen` → -1, errno ERANGE.
///  * Row 5: odd number of hex digits (dangling nibble) → -1, errno EINVAL.
#[test]
fn hex2bin_invalid_char_range_and_odd() {
    let d = duo();
    let (cf, rf) = d.pair::<Hex2Bin>("sodium_hex2bin");
    let cf = *cf;
    let rf = *rf;

    // `use_end`: when true, pass a non-NULL `hex_end`, which suppresses the
    // trailing `hex_pos != hex_len` EINVAL override so the ERANGE set at the
    // break point survives (matching ERRORS.md's stated errno for row 4).
    let run = |f: Hex2Bin, maxlen: usize, h: &[u8], use_end: bool| -> (i32, i32) {
        with_errno(|| {
            let mut bin = [0u8; 64];
            let mut blen = 0usize;
            let mut hend: *const u8 = core::ptr::null();
            let hend_p = if use_end {
                &mut hend as *mut *const u8
            } else {
                core::ptr::null_mut()
            };
            unsafe {
                f(
                    bin.as_mut_ptr(),
                    maxlen,
                    h.as_ptr(),
                    h.len(),
                    core::ptr::null(),
                    &mut blen,
                    hend_p,
                )
            }
        })
    };

    // Row 3: invalid char, no ignore set, hex_end NULL → EINVAL.
    for hex in ["zz", "12x4", "gg", "de!ad"] {
        let (rc, ce) = run(cf, 64, hex.as_bytes(), false);
        let (rr, re) = run(rf, 64, hex.as_bytes(), false);
        eq_i32(&format!("hex2bin row3 {hex:?} ret"), rc, rr);
        assert_eq!(ce, re, "hex2bin row3 {hex:?}: errno {ce} != {re}");
        assert_eq!(rc, -1, "hex2bin row3 {hex:?} should be -1");
        assert_eq!(ce, EINVAL, "hex2bin row3 {hex:?} errno should be EINVAL, got {ce}");
    }

    // Row 4: too many pairs for bin_maxlen → ERANGE. Pass non-NULL hex_end so
    // the ERANGE (set when `bin_pos >= bin_maxlen`) is the observed errno.
    for (hex, maxlen) in [("deadbeef", 1usize), ("0011223344", 2), ("aabbcc", 0)] {
        let (rc, ce) = run(cf, maxlen, hex.as_bytes(), true);
        let (rr, re) = run(rf, maxlen, hex.as_bytes(), true);
        eq_i32(&format!("hex2bin row4 {hex:?} max={maxlen} ret"), rc, rr);
        assert_eq!(ce, re, "hex2bin row4 {hex:?}: errno {ce} != {re}");
        assert_eq!(rc, -1, "hex2bin row4 {hex:?} should be -1");
        assert_eq!(ce, ERANGE, "hex2bin row4 {hex:?} errno should be ERANGE, got {ce}");
    }

    // Row 5: odd digit count (dangling nibble) → EINVAL.
    for hex in ["abc", "1", "12345", "deadb"] {
        let (rc, ce) = run(cf, 64, hex.as_bytes(), false);
        let (rr, re) = run(rf, 64, hex.as_bytes(), false);
        eq_i32(&format!("hex2bin row5 {hex:?} ret"), rc, rr);
        assert_eq!(ce, re, "hex2bin row5 {hex:?}: errno {ce} != {re}");
        assert_eq!(rc, -1, "hex2bin row5 {hex:?} should be -1");
        assert_eq!(ce, EINVAL, "hex2bin row5 {hex:?} errno should be EINVAL, got {ce}");
    }
}

// ===========================================================================
// sodium/codecs.c — base64 invalid variant (misuse) reached via EVERY b64 fn
// ===========================================================================

/// ERRORS.md rows 6, 296: `sodium_base64_check_variant` reached through EVERY
/// base64 entry point with an INVALID variant (any int whose
/// `(v as u32) & !0x6 != 0x1`, i.e. not in {1,3,5,7}). Each must
/// `sodium_misuse()` → abort. Verified for `sodium_base64_encoded_len`,
/// `sodium_bin2base64`, and `sodium_base642bin` in a forked child.
#[test]
fn base64_invalid_variant_aborts_everywhere() {
    let d = duo();
    let (enc_c, enc_r) =
        d.pair::<unsafe extern "C" fn(usize, i32) -> usize>("sodium_base64_encoded_len");
    let enc_c = *enc_c;
    let enc_r = *enc_r;
    let (b2b_c, b2b_r) = d.pair::<Bin2B64>("sodium_bin2base64");
    let b2b_c = *b2b_c;
    let b2b_r = *b2b_r;
    let (d2b_c, d2b_r) = d.pair::<B642Bin>("sodium_base642bin");
    let d2b_c = *d2b_c;
    let d2b_r = *d2b_r;

    let bad_variants: [i32; 7] = [0, 2, 4, 6, 8, -1, i32::MAX];
    for v in bad_variants {
        same_fate(
            &format!("base64_encoded_len bad variant {v}"),
            || {
                unsafe { enc_c(16, v) };
            },
            || {
                unsafe { enc_r(16, v) };
            },
        );
        same_fate(
            &format!("bin2base64 bad variant {v}"),
            || {
                let bin = [0u8; 16];
                let mut out = [0u8; 64];
                unsafe { b2b_c(out.as_mut_ptr(), out.len(), bin.as_ptr(), bin.len(), v) };
            },
            || {
                let bin = [0u8; 16];
                let mut out = [0u8; 64];
                unsafe { b2b_r(out.as_mut_ptr(), out.len(), bin.as_ptr(), bin.len(), v) };
            },
        );
        same_fate(
            &format!("base642bin bad variant {v}"),
            || {
                let b64 = b"AAAA";
                let mut bin = [0u8; 16];
                let mut blen = 0usize;
                unsafe {
                    d2b_c(
                        bin.as_mut_ptr(),
                        bin.len(),
                        b64.as_ptr(),
                        b64.len(),
                        core::ptr::null(),
                        &mut blen,
                        core::ptr::null_mut(),
                        v,
                    )
                };
            },
            || {
                let b64 = b"AAAA";
                let mut bin = [0u8; 16];
                let mut blen = 0usize;
                unsafe {
                    d2b_r(
                        bin.as_mut_ptr(),
                        bin.len(),
                        b64.as_ptr(),
                        b64.len(),
                        core::ptr::null(),
                        &mut blen,
                        core::ptr::null_mut(),
                        v,
                    )
                };
            },
        );
    }
}

/// ERRORS.md rows 7, 8, 10: 64-bit-UNREACHABLE base64 sizing overflows.
///  * Row 7: `sodium_base64_encoded_len` `bin_len/3 > (SIZE_MAX-5)/4`.
///  * Row 8: `sodium_bin2base64` `nibbles > (SIZE_MAX-5)/4`.
///  * Row 10: internal `assert(b64_pos <= b64_len)`.
/// All require a `bin_len` near `SIZE_MAX` (≈ 3·(SIZE_MAX-5)/4 ≈ 1.4e19 bytes),
/// which cannot be allocated or even addressed on a 64-bit host, so the trigger
/// condition is not constructible through the public API without first
/// dereferencing an impossible `bin` buffer. Marked `unreachable (64-bit)` /
/// `unreachable` in ERRORS.md — NOT differential-tested (no faked test). This is
/// a documentation-only note.
#[test]
fn base64_sizing_overflow_unreachable_note() {
    // Row 7/8: 3 * ((SIZE_MAX-5)/4) already overflows usize; a bin_len large
    // enough to trip the guard implies a >1.4e19-byte input buffer, impossible
    // on 64-bit. Row 10's assert can only fire if that sizing math is wrong,
    // which is likewise unreachable. Left untested by construction.
    assert!((usize::MAX - 5) / 4 > 0);
}

/// ERRORS.md row 9: `sodium_bin2base64` output buffer too small
/// (`b64_maxlen <= b64_len`) → `sodium_misuse()` → abort. Exercised across all
/// four variants and several bin lengths / remainder classes.
#[test]
fn bin2base64_output_too_small_aborts() {
    let d = duo();
    let (cf, rf) = d.pair::<Bin2B64>("sodium_bin2base64");
    let cf = *cf;
    let rf = *rf;

    for variant in [
        B64_ORIGINAL,
        B64_ORIGINAL_NO_PADDING,
        B64_URLSAFE,
        B64_URLSAFE_NO_PADDING,
    ] {
        for bin_len in [1usize, 2, 3, 4, 5, 16] {
            // Compute the encoded len the way C does, then pass a buffer
            // exactly equal to it — the C guard requires b64_maxlen > b64_len,
            // so `== b64_len` misuses.
            let nibbles = bin_len / 3;
            let remainder = bin_len - 3 * nibbles;
            let mut b64_len = nibbles * 4;
            if remainder != 0 {
                if (variant as u32) & 0x2 == 0 {
                    b64_len += 4;
                } else {
                    b64_len += 2 + (remainder >> 1);
                }
            }
            let maxlen = b64_len; // one short of the required b64_len + 1
            same_fate(
                &format!("bin2base64 row9 too-small variant={variant} bin_len={bin_len}"),
                || {
                    let bin = vec![0x55u8; bin_len];
                    let mut out = vec![0u8; maxlen + 8];
                    unsafe { cf(out.as_mut_ptr(), maxlen, bin.as_ptr(), bin_len, variant) };
                },
                || {
                    let bin = vec![0x55u8; bin_len];
                    let mut out = vec![0u8; maxlen + 8];
                    unsafe { rf(out.as_mut_ptr(), maxlen, bin.as_ptr(), bin_len, variant) };
                },
            );
        }
    }
}

// ===========================================================================
// sodium/codecs.c — base642bin decode errors
// ===========================================================================

/// ERRORS.md rows 11, 12, 13: `sodium_base642bin`.
///  * Row 11: invalid base64 char not in `ignore` → -1, errno EINVAL (via the
///    `b64_end == NULL && b64_pos != b64_len` trailing check).
///  * Row 12: decoded output longer than `bin_maxlen` → -1, errno ERANGE.
///  * Row 13: non-zero trailing bits in the final partial group → -1, EINVAL.
#[test]
fn base642bin_char_range_and_trailing_bits() {
    let d = duo();
    let (cf, rf) = d.pair::<B642Bin>("sodium_base642bin");
    let cf = *cf;
    let rf = *rf;

    let call = |f: B642Bin, bin_max: usize, b64: &[u8], variant: i32, use_end: bool| -> (i32, i32) {
        with_errno(|| {
            let mut bin = [0u8; 64];
            let mut blen = 0usize;
            let mut bend: *const u8 = core::ptr::null();
            let bend_p = if use_end {
                &mut bend as *mut *const u8
            } else {
                core::ptr::null_mut()
            };
            unsafe {
                f(
                    bin.as_mut_ptr(),
                    bin_max,
                    b64.as_ptr(),
                    b64.len(),
                    core::ptr::null(),
                    &mut blen,
                    bend_p,
                    variant,
                )
            }
        })
    };

    // Row 11: invalid char, ORIGINAL, no ignore, b64_end NULL → EINVAL.
    for b64 in [&b"AA*A"[..], b"::::", b"AAAA!", b"  AA"] {
        let (rc, ce) = call(cf, 64, b64, B64_ORIGINAL, false);
        let (rr, re) = call(rf, 64, b64, B64_ORIGINAL, false);
        eq_i32(&format!("base642bin row11 {b64:?} ret"), rc, rr);
        assert_eq!(ce, re, "base642bin row11 {b64:?}: errno {ce} != {re}");
        assert_eq!(rc, -1, "base642bin row11 {b64:?} should be -1");
        assert_eq!(ce, EINVAL, "base642bin row11 {b64:?} errno should be EINVAL");
    }

    // Row 12: valid b64 decoding to more bytes than bin_maxlen → ERANGE.
    // Pass non-NULL b64_end so the ERANGE (set at `bin_pos >= bin_maxlen`) is
    // not overwritten by the trailing EINVAL check.
    for (b64, maxlen) in [(&b"AAAAAAAA"[..], 2usize), (b"////////", 1), (b"QUJD", 1)] {
        let (rc, ce) = call(cf, maxlen, b64, B64_ORIGINAL, true);
        let (rr, re) = call(rf, maxlen, b64, B64_ORIGINAL, true);
        eq_i32(&format!("base642bin row12 {b64:?} ret"), rc, rr);
        assert_eq!(ce, re, "base642bin row12 {b64:?}: errno {ce} != {re}");
        assert_eq!(rc, -1, "base642bin row12 {b64:?} should be -1");
        assert_eq!(ce, ERANGE, "base642bin row12 {b64:?} errno should be ERANGE");
    }

    // Row 13: non-zero trailing bits in final partial group.
    // Two base64 chars encode 12 bits → 1 byte + 4 leftover bits that must be
    // zero. NO_PADDING variant so the trailing-bits check (not padding) fails.
    for b64 in [&b"AB"[..], b"AC", b"/B", b"AAB"] {
        let (rc, ce) = call(cf, 64, b64, B64_ORIGINAL_NO_PADDING, false);
        let (rr, re) = call(rf, 64, b64, B64_ORIGINAL_NO_PADDING, false);
        eq_i32(&format!("base642bin row13 {b64:?} ret"), rc, rr);
        assert_eq!(ce, re, "base642bin row13 {b64:?}: errno {ce} != {re}");
        assert_eq!(rc, -1, "base642bin row13 {b64:?} should be -1");
    }
}

/// ERRORS.md rows 14, 15: `_sodium_base642bin_skip_padding` (padded variants).
///  * Row 14: padded variant, input truncated before the required `=` →
///    -1, errno ERANGE.
///  * Row 15: padded variant, a non-`=` / non-ignored char in the padding
///    region → -1, errno EINVAL.
#[test]
fn base642bin_padding_truncated_and_bad_char() {
    let d = duo();
    let (cf, rf) = d.pair::<B642Bin>("sodium_base642bin");
    let cf = *cf;
    let rf = *rf;

    let call = |f: B642Bin, b64: &[u8], variant: i32| -> (i32, i32) {
        with_errno(|| {
            let mut bin = [0u8; 64];
            let mut blen = 0usize;
            unsafe {
                f(
                    bin.as_mut_ptr(),
                    64,
                    b64.as_ptr(),
                    b64.len(),
                    core::ptr::null(),
                    &mut blen,
                    core::ptr::null_mut(),
                    variant,
                )
            }
        })
    };

    // Row 14: "QQ" decodes 1 byte with acc_len=4 → needs a '=' padding char,
    // but input ends → ERANGE. "QQ=" gives one '=' then runs out → ERANGE.
    // (Inputs like "QUJ" fail the earlier trailing-bits check instead and do
    // not exercise the padding path, so they are intentionally excluded.)
    for b64 in [&b"QQ"[..], b"QQ="] {
        let (rc, ce) = call(cf, b64, B64_ORIGINAL);
        let (rr, re) = call(rf, b64, B64_ORIGINAL);
        eq_i32(&format!("base642bin row14 {b64:?} ret"), rc, rr);
        assert_eq!(ce, re, "base642bin row14 {b64:?}: errno {ce} != {re}");
        assert_eq!(rc, -1, "base642bin row14 {b64:?} should be -1");
        assert_eq!(ce, ERANGE, "base642bin row14 {b64:?} errno should be ERANGE");
    }

    // Row 15: padding region contains a non-'=' char (and no ignore set) →
    // EINVAL. "QQ*=" — after 1 decoded byte we need padding; '*' is invalid.
    for b64 in [&b"QQ*="[..], b"QQ.=", b"QUJ*"] {
        let (rc, ce) = call(cf, b64, B64_ORIGINAL);
        let (rr, re) = call(rf, b64, B64_ORIGINAL);
        eq_i32(&format!("base642bin row15 {b64:?} ret"), rc, rr);
        assert_eq!(ce, re, "base642bin row15 {b64:?}: errno {ce} != {re}");
        assert_eq!(rc, -1, "base642bin row15 {b64:?} should be -1");
        assert_eq!(ce, EINVAL, "base642bin row15 {b64:?} errno should be EINVAL");
    }
}

/// ERRORS.md rows 16, 17, 18: base64 alphabet / padding mismatches.
///  * Row 16: URLSAFE variant fed `+` or `/` → invalid char → -1, EINVAL.
///  * Row 17: ORIGINAL variant fed `-` or `_` → invalid char → -1, EINVAL.
///  * Row 18: NO_PADDING variant fed `=` → `=` is not a valid symbol and not
///    consumed as padding → trailing check fails → -1, EINVAL.
#[test]
fn base642bin_variant_alphabet_mismatch() {
    let d = duo();
    let (cf, rf) = d.pair::<B642Bin>("sodium_base642bin");
    let cf = *cf;
    let rf = *rf;

    let call = |f: B642Bin, b64: &[u8], variant: i32| -> (i32, i32) {
        with_errno(|| {
            let mut bin = [0u8; 64];
            let mut blen = 0usize;
            unsafe {
                f(
                    bin.as_mut_ptr(),
                    64,
                    b64.as_ptr(),
                    b64.len(),
                    core::ptr::null(),
                    &mut blen,
                    core::ptr::null_mut(),
                    variant,
                )
            }
        })
    };

    // Row 16: URLSAFE fed '+' or '/'.
    for b64 in [&b"AB+D"[..], b"AB/D", b"++++", b"////"] {
        let (rc, ce) = call(cf, b64, B64_URLSAFE);
        let (rr, re) = call(rf, b64, B64_URLSAFE);
        eq_i32(&format!("base642bin row16 {b64:?} ret"), rc, rr);
        assert_eq!(ce, re, "base642bin row16 {b64:?}: errno {ce} != {re}");
        assert_eq!(rc, -1, "base642bin row16 {b64:?} should be -1");
        assert_eq!(ce, EINVAL, "base642bin row16 {b64:?} errno should be EINVAL");
    }
    // also NO_PADDING urlsafe fed '+'/'/'
    for b64 in [&b"AB+D"[..], b"AB/D"] {
        let (rc, ce) = call(cf, b64, B64_URLSAFE_NO_PADDING);
        let (rr, re) = call(rf, b64, B64_URLSAFE_NO_PADDING);
        eq_i32(&format!("base642bin row16b {b64:?} ret"), rc, rr);
        assert_eq!(ce, re, "base642bin row16b {b64:?}: errno {ce} != {re}");
    }

    // Row 17: ORIGINAL fed '-' or '_'.
    for b64 in [&b"AB-D"[..], b"AB_D", b"----", b"____"] {
        let (rc, ce) = call(cf, b64, B64_ORIGINAL);
        let (rr, re) = call(rf, b64, B64_ORIGINAL);
        eq_i32(&format!("base642bin row17 {b64:?} ret"), rc, rr);
        assert_eq!(ce, re, "base642bin row17 {b64:?}: errno {ce} != {re}");
        assert_eq!(rc, -1, "base642bin row17 {b64:?} should be -1");
        assert_eq!(ce, EINVAL, "base642bin row17 {b64:?} errno should be EINVAL");
    }

    // Row 18: NO_PADDING fed '='.
    for b64 in [&b"QQ=="[..], b"QUJ=", b"AAAA="] {
        let (rc, ce) = call(cf, b64, B64_ORIGINAL_NO_PADDING);
        let (rr, re) = call(rf, b64, B64_ORIGINAL_NO_PADDING);
        eq_i32(&format!("base642bin row18 {b64:?} ret"), rc, rr);
        assert_eq!(ce, re, "base642bin row18 {b64:?}: errno {ce} != {re}");
        assert_eq!(rc, -1, "base642bin row18 {b64:?} should be -1");
        assert_eq!(ce, EINVAL, "base642bin row18 {b64:?} errno should be EINVAL");
    }
}

// ===========================================================================
// sodium/codecs.c — IP codecs
// ===========================================================================

/// ERRORS.md rows 19, 20, 21, 22, 23, 24: `sodium_ip2bin` error returns (-1).
///  * Row 19: hex digit outside `0-9a-fA-F` inside an IPv6 group (`ip_hex_digit`
///    returns -1 → `parse_ipv6` fails).
///  * Row 20: zone char not in `[0-9a-zA-Z._-]`.
///  * Row 21: `%` present but zone empty (`zone+1 >= end`).
///  * Row 22: `%zone` attached to an IPv4 (non-IPv6) address.
///  * Row 23: malformed IPv6.
///  * Row 24: malformed IPv4.
#[test]
fn ip2bin_all_error_paths() {
    let d = duo();
    let (cf, rf) = d.pair::<Ip2Bin>("sodium_ip2bin");
    let cf = *cf;
    let rf = *rf;

    let call = |f: Ip2Bin, ip: &[u8]| -> (i32, [u8; 16]) {
        let mut bin = [0u8; 16];
        let r = unsafe { f(bin.as_mut_ptr(), ip.as_ptr(), ip.len()) };
        (r, bin)
    };

    let cases: &[(&str, &[u8])] = &[
        // Row 19: bad hex digit inside an IPv6 group.
        ("row19 bad hexdigit", b"2001:db8:g::1"),
        ("row19 bad hexdigit2", b"::z"),
        // Row 20: illegal zone char.
        ("row20 zone bad char", b"fe80::1%e th0"),
        ("row20 zone bad char2", b"fe80::1%a/b"),
        // Row 21: empty zone.
        ("row21 empty zone", b"fe80::1%"),
        // Row 22: zone on IPv4.
        ("row22 zone on ipv4", b"192.168.0.1%eth0"),
        // Row 23: malformed IPv6.
        ("row23 malformed ipv6", b"1:2:3:4:5:6:7:8:9"),
        ("row23 malformed ipv6b", b":::"),
        ("row23 malformed ipv6c", b"12345::1"),
        // Row 24: malformed IPv4.
        ("row24 malformed ipv4", b"256.1.1.1"),
        ("row24 malformed ipv4b", b"1.2.3"),
        ("row24 malformed ipv4c", b"1.2.3.4.5"),
        ("row24 malformed ipv4d", b"1..2.3"),
    ];
    for (name, ip) in cases {
        let (rc, cb) = call(cf, ip);
        let (rr, rb) = call(rf, ip);
        eq_i32(&format!("ip2bin {name} ret"), rc, rr);
        assert_eq!(rc, -1, "ip2bin {name}: expected -1 from C, got {rc}");
        // On error C and Rust should still agree byte-for-byte on the output.
        eq_bytes(&format!("ip2bin {name} bin"), &cb, &rb);
    }
}

/// ERRORS.md rows 25, 26, 27: `sodium_bin2ip` returns NULL.
///  * Row 25: `ip_maxlen <= 2` → NULL.
///  * Row 26: rendered IPv4-mapped address length `>= ip_maxlen` → NULL.
///  * Row 27: rendered IPv6 address length `>= ip_maxlen` → NULL.
#[test]
fn bin2ip_buffer_too_small_null() {
    let d = duo();
    let (cf, rf) = d.pair::<Bin2Ip>("sodium_bin2ip");
    let cf = *cf;
    let rf = *rf;

    let call = |f: Bin2Ip, maxlen: usize, bin: &[u8; 16]| -> bool {
        let mut out = vec![0u8; maxlen.max(1) + 64];
        let p = unsafe { f(out.as_mut_ptr(), maxlen, bin.as_ptr()) };
        p.is_null()
    };

    let ipv4_mapped: [u8; 16] = {
        let mut b = [0u8; 16];
        b[10] = 0xff;
        b[11] = 0xff;
        b[12] = 192;
        b[13] = 168;
        b[14] = 0;
        b[15] = 1;
        b
    };
    let ipv6: [u8; 16] = [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

    // Row 25: ip_maxlen <= 2 always NULL, regardless of address.
    for maxlen in [0usize, 1, 2] {
        let cn = call(cf, maxlen, &ipv4_mapped);
        let rn = call(rf, maxlen, &ipv4_mapped);
        assert_eq!(cn, rn, "bin2ip row25 maxlen={maxlen}: C null={cn} Rust null={rn}");
        assert!(cn, "bin2ip row25 maxlen={maxlen}: expected NULL");
    }

    // Row 26: IPv4-mapped "192.168.0.1" renders 11 chars, needs maxlen>=12.
    for maxlen in [3usize, 5, 11] {
        let cn = call(cf, maxlen, &ipv4_mapped);
        let rn = call(rf, maxlen, &ipv4_mapped);
        assert_eq!(cn, rn, "bin2ip row26 maxlen={maxlen}: C null={cn} Rust null={rn}");
        assert!(cn, "bin2ip row26 maxlen={maxlen}: expected NULL (rendered len >= maxlen)");
    }

    // Row 27: IPv6 "2001:db8::1" renders 11 chars; maxlen just under → NULL.
    for maxlen in [3usize, 6, 11] {
        let cn = call(cf, maxlen, &ipv6);
        let rn = call(rf, maxlen, &ipv6);
        assert_eq!(cn, rn, "bin2ip row27 maxlen={maxlen}: C null={cn} Rust null={rn}");
        assert!(cn, "bin2ip row27 maxlen={maxlen}: expected NULL (rendered len >= maxlen)");
    }
}

// ===========================================================================
// sodium/core.c + runtime.c — rows 28–37
// ===========================================================================

/// ERRORS.md rows 28–33, 35–37: `sodium/core.c` / `sodium/runtime.c` error
/// surfaces — UNREACHABILITY ANALYSIS for this build (`-Dsodium_EXPORTS`, no
/// `HAVE_*`):
///  * Rows 28, 29, 30 (`sodium_init` crit enter/leave failure): both
///    `sodium_crit_enter`/`_leave` compile to the trivial `return 0;` variant
///    (no `_WIN32`/`HAVE_PTHREAD`/`HAVE_ATOMIC_OPS`), so they can never fail.
///    UNREACHABLE. We assert `sodium_init` returns identically (both return 1 on
///    the already-initialised path) as a sanity check of reachable behaviour.
///  * Row 31 (`_sodium_crit_init` Windows `default:` arm): Windows-only,
///    not compiled on Linux. UNREACHABLE (non-Win).
///  * Row 32 (`sodium_crit_enter` `pthread_mutex_lock` fails): the pthread
///    branch is `#elif defined(HAVE_PTHREAD)`, not compiled. UNREACHABLE.
///  * Row 33 (`sodium_crit_leave` while `locked == 0` → -1 EPERM): the EPERM
///    return lives inside the `_WIN32` and `HAVE_PTHREAD` branches only. The
///    compiled variant is the trailing `#else { return 0; }`, so a "leave while
///    unlocked" returns 0, NOT -1/EPERM. UNREACHABLE in this build. We DO
///    exercise the reachable behaviour: `sodium_crit_leave` returns 0 in BOTH
///    libs (differentially verified below).
///  * Row 35 (`sodium_set_misuse_handler` crit failure): crit ops can't fail.
///    UNREACHABLE. Not driven (would also install a handler, which is forbidden).
///  * Row 36 (`_sodium_runtime_arm_cpu_features` non-ARM → -1 internal): static
///    internal function on x86-64; not an exported symbol → not reachable
///    through any public export → not differential-testable.
///  * Row 37 (`_sodium_runtime_intel_cpu_features` `cpuid(0)` reports 0 leaves):
///    impossible on any real x86-64 host running these tests. UNREACHABLE.
#[test]
fn core_crit_leave_returns_zero_in_this_build() {
    let d = duo();
    // Row 33 reachable behaviour: crit_leave returns 0 in the no-HAVE_* build.
    let (cf, rf) = d.pair::<CritLeave>("sodium_crit_leave");
    let (rc, ce) = with_errno(|| unsafe { (*cf)() });
    let (rr, re) = with_errno(|| unsafe { (*rf)() });
    eq_i32("sodium_crit_leave ret (row33 reachable path)", rc, rr);
    assert_eq!(rc, 0, "expected crit_leave==0 in no-HAVE_* build, got {rc}");
    assert_eq!(ce, re, "sodium_crit_leave errno {ce} != {re}");

    // Rows 28-30 sanity: sodium_init already-initialised path returns 1 in both.
    let (ic, ir) = d.pair::<unsafe extern "C" fn() -> i32>("sodium_init");
    let rc = unsafe { (*ic)() };
    let rr = unsafe { (*ir)() };
    eq_i32("sodium_init already-init returns identically", rc, rr);
    assert_eq!(rc, 1, "expected sodium_init==1 (already initialised), got {rc}");
}

/// ERRORS.md row 34: `sodium_misuse()` always runs the (absent) handler then
/// `abort()`s. With NO handler installed (task forbids installing one), both
/// libraries must terminate the forked child with `SIGABRT` identically.
#[test]
fn misuse_always_aborts() {
    let d = duo();
    let (cf, rf) = d.pair::<unsafe extern "C" fn()>("sodium_misuse");
    let cf = *cf;
    let rf = *rf;
    let fate_c = in_child(|| unsafe { cf() });
    let fate_r = in_child(|| unsafe { rf() });
    assert_eq!(fate_c, fate_r, "sodium_misuse fate: C {fate_c:?} != Rust {fate_r:?}");
    assert_eq!(fate_c, Fate::Signaled(6), "sodium_misuse should raise SIGABRT (6), got {fate_c:?}");
}

// ===========================================================================
// sodium/utils.c — rows 38–57
// ===========================================================================

/// ERRORS.md rows 38, 39, 42, 43, 44, 46, 47, 48, 51: `sodium/utils.c`
/// UNREACHABLE / non-API rows in this build.
///  * Row 38 (`sodium_memzero` `memset_s` failure): no `HAVE_MEMSET_S`; the
///    volatile byte-loop variant is compiled and cannot fail. UNREACHABLE.
///  * Row 39 (`_sodium_alloc_init` `page_size < CANARY_SIZE`): guarded by
///    `#ifdef HAVE_ALIGNED_MALLOC`, undefined here → not compiled. UNREACHABLE.
///  * Rows 42, 51 (`_mprotect_*` / `_sodium_mprotect` no mprotect): reachable
///    behaviour (ENOSYS) tested in `mprotect_returns_enosys`.
///  * Row 43 (`_out_of_bounds` canary/guard violation → SIGSEGV/abort): only
///    reachable through the guarded allocator, not compiled. Not API-catchable.
///    UNREACHABLE.
///  * Row 44 (`_unprotected_ptr_from_user_ptr` misuse): inside
///    `HAVE_ALIGNED_MALLOC` only. UNREACHABLE.
///  * Rows 46, 47, 48 (`_sodium_malloc` guarded ENOMEM / OOM / assert): the
///    guarded allocator body is inside `#else HAVE_ALIGNED_MALLOC`; the
///    compiled `_sodium_malloc` is plain `malloc(size?size:1)` with none of
///    these checks. UNREACHABLE as written. The plain-malloc OOM behaviour is
///    covered by `malloc_huge_returns_null` / `allocarray_overflow`.
#[test]
fn utils_unreachable_rows_note() {
    // Documentation-only: see the doc comment above. The listed rows are
    // compiled out or unreachable via the public API in this build config.
    assert!(true);
}

/// ERRORS.md rows 40, 41: `sodium_mlock` / `sodium_munlock` return values.
/// This build defines no `HAVE_MLOCK` / `WINAPI_DESKTOP`, so both fall through
/// to `errno = ENOSYS; return -1;`. `sodium_munlock` first zeroes the buffer.
/// Verified: identical return (-1) and errno (ENOSYS) in C and Rust, and that
/// munlock zeroed the buffer identically.
#[test]
fn mlock_munlock_enosys() {
    let d = duo();
    let (mlc, mlr) = d.pair::<Mlock>("sodium_mlock");
    let mlc = *mlc;
    let mlr = *mlr;
    let (muc, mur) = d.pair::<Mlock>("sodium_munlock");
    let muc = *muc;
    let mur = *mur;

    for len in [0usize, 1, 16, 64, 4096] {
        // mlock
        let mut cbuf = vec![0xAAu8; len.max(1)];
        let mut rbuf = vec![0xAAu8; len.max(1)];
        let (rc, ce) = with_errno(|| unsafe { mlc(cbuf.as_mut_ptr() as *mut _, len) });
        let (rr, re) = with_errno(|| unsafe { mlr(rbuf.as_mut_ptr() as *mut _, len) });
        eq_i32(&format!("sodium_mlock len={len} ret"), rc, rr);
        assert_eq!(rc, -1, "sodium_mlock should return -1 (ENOSYS build)");
        assert_eq!(ce, re, "sodium_mlock errno {ce} != {re}");
        assert_eq!(ce, ENOSYS, "sodium_mlock errno should be ENOSYS, got {ce}");

        // munlock — must zero the buffer AND return -1/ENOSYS.
        let mut cbuf = vec![0xAAu8; len.max(1)];
        let mut rbuf = vec![0xAAu8; len.max(1)];
        let (rc, ce) = with_errno(|| unsafe { muc(cbuf.as_mut_ptr() as *mut _, len) });
        let (rr, re) = with_errno(|| unsafe { mur(rbuf.as_mut_ptr() as *mut _, len) });
        eq_i32(&format!("sodium_munlock len={len} ret"), rc, rr);
        assert_eq!(rc, -1, "sodium_munlock should return -1 (ENOSYS build)");
        assert_eq!(ce, re, "sodium_munlock errno {ce} != {re}");
        assert_eq!(ce, ENOSYS, "sodium_munlock errno should be ENOSYS, got {ce}");
        eq_bytes(&format!("sodium_munlock zeroed len={len}"), &cbuf, &rbuf);
        if len > 0 {
            assert!(cbuf[..len].iter().all(|&b| b == 0), "munlock did not zero buffer");
        }
    }
}

/// ERRORS.md rows 42, 51: `sodium_mprotect_{noaccess,readonly,readwrite}`.
/// No `HAVE_PAGE_PROTECTION` in this build → `_sodium_mprotect` sets ENOSYS and
/// returns -1. Verified identical return + errno across C and Rust.
#[test]
fn mprotect_returns_enosys() {
    let d = duo();
    for name in [
        "sodium_mprotect_noaccess",
        "sodium_mprotect_readonly",
        "sodium_mprotect_readwrite",
    ] {
        let (cf, rf) = d.pair::<unsafe extern "C" fn(*mut core::ffi::c_void) -> i32>(name);
        let mut dummy = [0u8; 16];
        let (rc, ce) = with_errno(|| unsafe { (*cf)(dummy.as_mut_ptr() as *mut _) });
        let (rr, re) = with_errno(|| unsafe { (*rf)(dummy.as_mut_ptr() as *mut _) });
        eq_i32(&format!("{name} ret"), rc, rr);
        assert_eq!(rc, -1, "{name} should return -1 (no HAVE_PAGE_PROTECTION)");
        assert_eq!(ce, re, "{name} errno {ce} != {re}");
        assert_eq!(ce, ENOSYS, "{name} errno should be ENOSYS, got {ce}");
    }
}

/// ERRORS.md rows 45, 49: `_sodium_malloc` / `sodium_malloc` with
/// `size >= SIZE_MAX - page_size*4`.
///
/// In this build (`HAVE_ALIGNED_MALLOC` undefined) `_sodium_malloc` is plain
/// `malloc(size ? size : 1)` — the sodium ENOMEM guard (row 45) and the canary
/// machinery are NOT compiled. So the observable behaviour is glibc's:
/// `malloc(huge)` returns NULL and sets errno ENOMEM. We assert C and Rust
/// agree on NULL-ness (row 49) AND on errno. The dedicated sodium ENOMEM guard
/// of row 45 is only reachable in a `HAVE_ALIGNED_MALLOC` build; here we test
/// the reachable plain-malloc equivalent.
#[test]
fn malloc_huge_returns_null() {
    let d = duo();
    let (cf, rf) = d.pair::<Malloc>("sodium_malloc");
    let cf = *cf;
    let rf = *rf;

    for size in [usize::MAX, usize::MAX - 4096, usize::MAX / 2] {
        let (pc, ce) = with_errno(|| unsafe { cf(size) });
        let (pr, re) = with_errno(|| unsafe { rf(size) });
        assert_eq!(
            pc.is_null(),
            pr.is_null(),
            "sodium_malloc({size}): C null={} Rust null={}",
            pc.is_null(),
            pr.is_null()
        );
        assert!(pc.is_null(), "sodium_malloc({size}) should fail (huge)");
        assert_eq!(ce, re, "sodium_malloc({size}) errno {ce} != {re}");
    }
}

/// ERRORS.md row 50: `sodium_allocarray` with `count > 0 && size >= SIZE_MAX/count`
/// → NULL, errno ENOMEM. This check IS compiled (in the wrapper, not the guarded
/// allocator), so both C and Rust must return NULL and set ENOMEM.
#[test]
fn allocarray_overflow() {
    let d = duo();
    let (cf, rf) = d.pair::<Allocarray>("sodium_allocarray");
    let cf = *cf;
    let rf = *rf;
    let (frc, frr) = d.pair::<FreeFn>("sodium_free");
    let frc = *frc;
    let frr = *frr;

    // Overflow cases: count>0 && size >= SIZE_MAX/count.
    for (count, size) in [
        (2usize, usize::MAX / 2 + 1),
        (16, usize::MAX / 16 + 1),
        (usize::MAX, 2),
        (3, usize::MAX / 3 + 1),
    ] {
        let (pc, ce) = with_errno(|| unsafe { cf(count, size) });
        let (pr, re) = with_errno(|| unsafe { rf(count, size) });
        assert_eq!(
            pc.is_null(),
            pr.is_null(),
            "sodium_allocarray({count},{size}): C null={} Rust null={}",
            pc.is_null(),
            pr.is_null()
        );
        assert!(pc.is_null(), "sodium_allocarray({count},{size}) should fail");
        assert_eq!(ce, re, "sodium_allocarray({count},{size}) errno {ce} != {re}");
        assert_eq!(ce, ENOMEM, "expected ENOMEM, got {ce}");
    }

    // Sanity (non-overflow): both succeed; free through the matching lib.
    for (count, size) in [(4usize, 8usize), (0, 100), (10, 0)] {
        let pc = unsafe { cf(count, size) };
        let pr = unsafe { rf(count, size) };
        assert_eq!(
            pc.is_null(),
            pr.is_null(),
            "sodium_allocarray({count},{size}) success null-parity"
        );
        if !pc.is_null() {
            unsafe { frc(pc) };
        }
        if !pr.is_null() {
            unsafe { frr(pr) };
        }
    }
}

/// ERRORS.md rows 52, 53, 54: `sodium_pad`.
///  * Row 52: `blocksize == 0` → -1.
///  * Row 53: `SIZE_MAX - unpadded_buflen <= xpadlen` → `sodium_misuse()`
///    (abort). This needs `unpadded_buflen` within `blocksize` of SIZE_MAX,
///    which cannot be backed by a real `buf` on 64-bit → marked `unreachable`
///    in ERRORS.md. NOT differential-tested (see note below).
///  * Row 54: `xpadded_len >= max_buflen` (output buffer too small) → -1.
#[test]
fn pad_blocksize_zero_and_buffer_too_small() {
    let d = duo();
    let (cf, rf) = d.pair::<Pad>("sodium_pad");
    let cf = *cf;
    let rf = *rf;

    // Row 52: blocksize == 0 → -1 (before any buffer touch).
    for unpadded in [0usize, 1, 10, 100] {
        let (rc, _) = with_errno(|| {
            let mut buf = vec![0u8; 256];
            let mut plen = 0usize;
            unsafe { cf(&mut plen, buf.as_mut_ptr(), unpadded, 0, buf.len()) }
        });
        let (rr, _) = with_errno(|| {
            let mut buf = vec![0u8; 256];
            let mut plen = 0usize;
            unsafe { rf(&mut plen, buf.as_mut_ptr(), unpadded, 0, buf.len()) }
        });
        eq_i32(&format!("pad row52 blocksize=0 unpadded={unpadded}"), rc, rr);
        assert_eq!(rc, -1, "pad blocksize=0 should be -1");
    }

    // Row 54: xpadded_len >= max_buflen (buffer too small).
    let cases: &[(usize, usize, usize)] = &[
        // (unpadded, blocksize, max_buflen) with xpadded_len >= max_buflen
        (10, 16, 10), // xpadded_len=15 >= 10
        (16, 16, 16), // xpadded_len=31 >= 16
        (0, 16, 0),   // xpadded_len=15 >= 0
        (5, 8, 5),    // xpadded_len=7 >= 5
        (7, 8, 7),    // xpadded_len=7 >= 7
        (10, 10, 10), // non-power-of-two blocksize; xpadded_len=19 >= 10
    ];
    for &(unpadded, blocksize, max_buflen) in cases {
        let (rc, _) = with_errno(|| {
            let mut buf = vec![0u8; 512];
            let mut plen = 0usize;
            unsafe { cf(&mut plen, buf.as_mut_ptr(), unpadded, blocksize, max_buflen) }
        });
        let (rr, _) = with_errno(|| {
            let mut buf = vec![0u8; 512];
            let mut plen = 0usize;
            unsafe { rf(&mut plen, buf.as_mut_ptr(), unpadded, blocksize, max_buflen) }
        });
        eq_i32(
            &format!("pad row54 too-small u={unpadded} b={blocksize} m={max_buflen}"),
            rc,
            rr,
        );
        assert_eq!(rc, -1, "pad row54 should be -1");
    }
    // Row 53 note: `SIZE_MAX - unpadded_buflen <= xpadlen` requires
    // unpadded_buflen ≈ SIZE_MAX with a real backing buffer — impossible on
    // 64-bit. Marked `unreachable` in ERRORS.md; not faked here.
}

/// ERRORS.md rows 55, 56, 57 (+ 300): `sodium_unpad`.
///  * Row 55: `padded_buflen < blocksize` → -1.
///  * Row 56: `blocksize == 0` → -1.
///  * Row 57: no `0x80` barrier byte in the last block → -1.
///  * Row 300 boundary: blocksize==0 and short-buffer edges are exercised here.
#[test]
fn unpad_short_buffer_zero_blocksize_and_no_barrier() {
    let d = duo();
    let (cf, rf) = d.pair::<Unpad>("sodium_unpad");
    let cf = *cf;
    let rf = *rf;

    let call = |f: Unpad, buf: &[u8], padded_buflen: usize, blocksize: usize| -> (i32, usize) {
        let mut ulen = 0xDEAD_BEEFusize;
        let r = unsafe { f(&mut ulen, buf.as_ptr(), padded_buflen, blocksize) };
        (r, ulen)
    };

    // Row 55: padded_buflen < blocksize → -1.
    for (padded, blocksize) in [(8usize, 16usize), (0, 1), (15, 16), (3, 100)] {
        let buf = vec![0u8; padded.max(1)];
        let (rc, cu) = call(cf, &buf, padded, blocksize);
        let (rr, ru) = call(rf, &buf, padded, blocksize);
        eq_i32(&format!("unpad row55 padded={padded} bs={blocksize} ret"), rc, rr);
        assert_eq!(rc, -1, "unpad row55 should be -1");
        assert_eq!(cu, ru, "unpad row55 out-len C {cu} != Rust {ru}");
    }

    // Row 56: blocksize == 0 → -1 (the `blocksize <= 0` guard).
    for padded in [0usize, 1, 16, 64] {
        let buf = vec![0x80u8; padded.max(1)];
        let (rc, cu) = call(cf, &buf, padded, 0);
        let (rr, ru) = call(rf, &buf, padded, 0);
        eq_i32(&format!("unpad row56 blocksize=0 padded={padded} ret"), rc, rr);
        assert_eq!(rc, -1, "unpad row56 should be -1");
        assert_eq!(cu, ru, "unpad row56 out-len C {cu} != Rust {ru}");
    }

    // Row 57: last block has no 0x80 barrier byte → -1.
    for (padded, blocksize) in [(16usize, 16usize), (32, 16), (8, 8), (10, 5)] {
        // Fill the whole buffer with 0x00 (no 0x80 anywhere in last block).
        let buf = vec![0x00u8; padded];
        let (rc, cu) = call(cf, &buf, padded, blocksize);
        let (rr, ru) = call(rf, &buf, padded, blocksize);
        eq_i32(&format!("unpad row57 no-barrier padded={padded} bs={blocksize} ret"), rc, rr);
        assert_eq!(rc, -1, "unpad row57 should be -1 (no barrier)");
        assert_eq!(cu, ru, "unpad row57 out-len C {cu} != Rust {ru}");

        // Also a block full of 0xFF (still no clean 0x80 barrier semantics).
        let buf = vec![0xFFu8; padded];
        let (rc, cu) = call(cf, &buf, padded, blocksize);
        let (rr, ru) = call(rf, &buf, padded, blocksize);
        eq_i32(&format!("unpad row57 ff padded={padded} bs={blocksize} ret"), rc, rr);
        assert_eq!(cu, ru, "unpad row57 ff out-len C {cu} != Rust {ru}");
    }
}

// ===========================================================================
// crypto_verify / memcmp / compare / is_zero — rows 58–61
// ===========================================================================

/// ERRORS.md row 58: `crypto_verify_16/32/64` return -1 on ANY differing byte.
/// Exercised with a single-bit flip in every byte position for each width, plus
/// the equal case (must return 0) as a control.
#[test]
fn crypto_verify_differing() {
    let d = duo();
    let mut rng = Rng::new(0xC0FFEE);
    for (name, n) in [
        ("crypto_verify_16", 16usize),
        ("crypto_verify_32", 32),
        ("crypto_verify_64", 64),
    ] {
        let (cf, rf) = d.pair::<Verify>(name);
        let cf = *cf;
        let rf = *rf;
        let base = rng.bytes(n);

        // Equal control: 0 in both.
        let rc = unsafe { cf(base.as_ptr(), base.as_ptr()) };
        let rr = unsafe { rf(base.as_ptr(), base.as_ptr()) };
        eq_i32(&format!("{name} equal"), rc, rr);
        assert_eq!(rc, 0, "{name} equal should be 0");

        // One-bit flip in every byte / bit position.
        for byte in 0..n {
            for bit in 0..8 {
                let mut other = base.clone();
                other[byte] ^= 1 << bit;
                let rc = unsafe { cf(base.as_ptr(), other.as_ptr()) };
                let rr = unsafe { rf(base.as_ptr(), other.as_ptr()) };
                eq_i32(&format!("{name} diff byte={byte} bit={bit}"), rc, rr);
                assert_eq!(rc, -1, "{name} differing should be -1");
            }
        }
    }
}

/// ERRORS.md row 59 (+ 295): `sodium_memcmp` returns -1 on ANY difference and 0
/// when equal, across many lengths incl. len==0; one-bit flip in every byte.
#[test]
fn memcmp_differing() {
    let d = duo();
    let (cf, rf) = d.pair::<Memcmp>("sodium_memcmp");
    let cf = *cf;
    let rf = *rf;
    let mut rng = Rng::new(0x1234_5678);

    for &n in LENS {
        let a = rng.bytes(n);
        // equal
        let rc = unsafe { cf(a.as_ptr() as *const _, a.as_ptr() as *const _, n) };
        let rr = unsafe { rf(a.as_ptr() as *const _, a.as_ptr() as *const _, n) };
        eq_i32(&format!("memcmp equal n={n}"), rc, rr);
        assert_eq!(rc, 0, "memcmp equal n={n} should be 0");

        // differing (n==0 has no byte to flip and is always equal)
        for byte in 0..n {
            let mut b = a.clone();
            b[byte] ^= 0x80;
            let rc = unsafe { cf(a.as_ptr() as *const _, b.as_ptr() as *const _, n) };
            let rr = unsafe { rf(a.as_ptr() as *const _, b.as_ptr() as *const _, n) };
            eq_i32(&format!("memcmp diff n={n} byte={byte}"), rc, rr);
            assert_eq!(rc, -1, "memcmp differing should be -1");
        }
    }
}

/// ERRORS.md row 60 (+ 301): `sodium_compare`.
///  * b1 < b2 (little-endian) → -1; b1 > b2 → 1; equal → 0.
///  * len == 0 → 0.
/// We drive all three orderings plus the empty case, byte-for-byte, and fuzz.
#[test]
fn compare_orderings() {
    let d = duo();
    let (cf, rf) = d.pair::<Compare>("sodium_compare");
    let cf = *cf;
    let rf = *rf;

    let call = |f: Compare, a: &[u8], b: &[u8]| -> i32 {
        assert_eq!(a.len(), b.len());
        unsafe { f(a.as_ptr(), b.as_ptr(), a.len()) }
    };

    // len == 0 → 0 (row 301).
    let rc = call(cf, &[], &[]);
    let rr = call(rf, &[], &[]);
    eq_i32("compare len=0", rc, rr);
    assert_eq!(rc, 0, "compare len=0 should be 0");

    // Little-endian: the MOST significant byte is the LAST element.
    let cases: &[(&str, Vec<u8>, Vec<u8>, i32)] = &[
        ("b1<b2 high byte", vec![0, 0, 0, 1], vec![0, 0, 0, 2], -1),
        ("b1>b2 high byte", vec![0, 0, 0, 2], vec![0, 0, 0, 1], 1),
        ("equal", vec![9, 9, 9, 9], vec![9, 9, 9, 9], 0),
        ("b1<b2 low byte only", vec![1, 0, 0, 0], vec![2, 0, 0, 0], -1),
        ("b1>b2 low byte only", vec![2, 0, 0, 0], vec![1, 0, 0, 0], 1),
        ("high dominates low", vec![0xff, 0, 0, 1], vec![0x00, 0, 0, 2], -1),
    ];
    for (name, a, b, expect) in cases {
        let rc = call(cf, a, b);
        let rr = call(rf, a, b);
        eq_i32(&format!("compare {name}"), rc, rr);
        assert_eq!(rc, *expect, "compare {name}: C returned {rc}, expected {expect}");
    }

    // Fuzz: random pairs of several lengths, C and Rust must always agree.
    let mut rng = Rng::new(0xABCD_1234);
    for &n in &[1usize, 2, 8, 16, 24, 32, 64] {
        for _ in 0..64 {
            let a = rng.bytes(n);
            let b = rng.bytes(n);
            let rc = call(cf, &a, &b);
            let rr = call(rf, &a, &b);
            eq_i32(&format!("compare fuzz n={n}"), rc, rr);
        }
    }
}

/// ERRORS.md row 61 (+ 301): `sodium_is_zero`.
///  * Any non-zero byte → 0.
///  * All-zero buffer → 1.
///  * len == 0 → 1 (empty buffer is "all zero").
#[test]
fn is_zero_cases() {
    let d = duo();
    let (cf, rf) = d.pair::<IsZero>("sodium_is_zero");
    let cf = *cf;
    let rf = *rf;

    // len == 0 → 1 (row 301).
    let rc = unsafe { cf(core::ptr::null(), 0) };
    let rr = unsafe { rf(core::ptr::null(), 0) };
    eq_i32("is_zero len=0", rc, rr);
    assert_eq!(rc, 1, "is_zero len=0 should be 1");

    for &n in &[1usize, 2, 8, 16, 32, 64, 100] {
        // all zero → 1
        let z = vec![0u8; n];
        let rc = unsafe { cf(z.as_ptr(), n) };
        let rr = unsafe { rf(z.as_ptr(), n) };
        eq_i32(&format!("is_zero all-zero n={n}"), rc, rr);
        assert_eq!(rc, 1, "is_zero all-zero should be 1");

        // one non-zero byte in each position → 0
        for byte in 0..n {
            let mut b = vec![0u8; n];
            b[byte] = 0x01;
            let rc = unsafe { cf(b.as_ptr(), n) };
            let rr = unsafe { rf(b.as_ptr(), n) };
            eq_i32(&format!("is_zero nonzero n={n} byte={byte}"), rc, rr);
            assert_eq!(rc, 0, "is_zero with nonzero byte should be 0");

            // also 0xff variant
            let mut b = vec![0u8; n];
            b[byte] = 0xff;
            let rc = unsafe { cf(b.as_ptr(), n) };
            let rr = unsafe { rf(b.as_ptr(), n) };
            eq_i32(&format!("is_zero 0xff n={n} byte={byte}"), rc, rr);
            assert_eq!(rc, 0, "is_zero with 0xff byte should be 0");
        }
    }
}
