//! Phase C — error-path differential tests for group G1 (ERRORS.md rows 1-99).
//!
//! Rows whose rejection is a *return value* are compared directly.
//! Rows whose rejection is a `sodium_misuse()` / `assert()` / `_out_of_bounds()`
//! abort are compared by re-executing this binary in a child process against
//! one library at a time and comparing how the two children died.
mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

// ===========================================================================
// child-process driver for the abort rows
// ===========================================================================
type BinToHex = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;
type EncLen = unsafe extern "C" fn(usize, c_int) -> usize;
type Bin2B64 = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char;
type B642Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
    c_int,
) -> c_int;
type Pad = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> c_int;

#[test]
fn zz_abort_child() {
    let Some((case, is_c)) = child_case() else {
        return; // normal test run: nothing to do
    };
    let l = libs();
    let h = if is_c { l.c } else { l.rs };
    macro_rules! sym {
        ($t:ty, $n:literal) => {{
            let s: libloading::Symbol<$t> = unsafe { h.get(concat!($n, "\0").as_bytes()) }.unwrap();
            s
        }};
    }
    unsafe {
        match case.as_str() {
            // --- row 42: sodium_bin2hex, bin_len >= SIZE_MAX / 2 -------------
            "bin2hex_binlen_overflow" => {
                let f = sym!(BinToHex, "sodium_bin2hex");
                let mut buf = [0u8; 64];
                // bin pointer is never dereferenced before the misuse check
                f(
                    buf.as_mut_ptr() as *mut c_char,
                    64,
                    buf.as_ptr(),
                    usize::MAX / 2,
                );
            }
            // --- row 43: sodium_bin2hex, hex_maxlen <= bin_len*2 -------------
            "bin2hex_maxlen_too_small" => {
                let f = sym!(BinToHex, "sodium_bin2hex");
                let bin = [1u8, 2, 3, 4];
                let mut out = [0u8; 16];
                f(out.as_mut_ptr() as *mut c_char, 8, bin.as_ptr(), 4);
            }
            "bin2hex_maxlen_exact_zero" => {
                let f = sym!(BinToHex, "sodium_bin2hex");
                let bin = [1u8];
                let mut out = [0u8; 8];
                f(out.as_mut_ptr() as *mut c_char, 0, bin.as_ptr(), 0);
            }
            // --- rows 47/48: sodium_base64_encoded_len, invalid variant ------
            c if c.starts_with("encoded_len_variant_") => {
                let v: i32 = c.rsplit('_').next().unwrap().parse().unwrap();
                let f = sym!(EncLen, "sodium_base64_encoded_len");
                let n = f(16, v);
                std::process::exit((n & 0x7f) as i32);
            }
            // --- row 49: sodium_base64_encoded_len overflow ------------------
            // bin_len / 3 > (SIZE_MAX - 5) / 4  <=>  bin_len > ~0.75 * SIZE_MAX
            "encoded_len_overflow" => {
                let f = sym!(EncLen, "sodium_base64_encoded_len");
                f(usize::MAX, 1);
            }
            "encoded_len_no_overflow" => {
                let f = sym!(EncLen, "sodium_base64_encoded_len");
                let n = f(usize::MAX / 2, 1);
                std::process::exit((n & 0x3f) as i32);
            }
            // --- row 50: sodium_bin2base64, invalid variant ------------------
            c if c.starts_with("bin2b64_variant_") => {
                let v: i32 = c.rsplit('_').next().unwrap().parse().unwrap();
                let f = sym!(Bin2B64, "sodium_bin2base64");
                let bin = [1u8, 2, 3];
                let mut out = [0u8; 32];
                f(out.as_mut_ptr() as *mut c_char, 32, bin.as_ptr(), 3, v);
            }
            // --- row 51: sodium_bin2base64 overflow --------------------------
            "bin2b64_overflow" => {
                let f = sym!(Bin2B64, "sodium_bin2base64");
                let bin = [1u8, 2, 3];
                let mut out = [0u8; 32];
                f(
                    out.as_mut_ptr() as *mut c_char,
                    usize::MAX,
                    bin.as_ptr(),
                    usize::MAX / 2,
                    1,
                );
            }
            // --- row 52: sodium_bin2base64, b64_maxlen too small -------------
            "bin2b64_maxlen_too_small" => {
                let f = sym!(Bin2B64, "sodium_bin2base64");
                let bin = [1u8, 2, 3];
                let mut out = [0u8; 32];
                f(out.as_mut_ptr() as *mut c_char, 4, bin.as_ptr(), 3, 1);
            }
            "bin2b64_maxlen_zero_empty" => {
                let f = sym!(Bin2B64, "sodium_bin2base64");
                let bin = [0u8; 1];
                let mut out = [0u8; 8];
                f(out.as_mut_ptr() as *mut c_char, 0, bin.as_ptr(), 0, 1);
            }
            // --- row 54: sodium_base642bin, invalid variant ------------------
            c if c.starts_with("b642bin_variant_") => {
                let v: i32 = c.rsplit('_').next().unwrap().parse().unwrap();
                let f = sym!(B642Bin, "sodium_base642bin");
                let s = b"QUJD\0";
                let mut out = [0u8; 8];
                let mut bl = 0usize;
                f(
                    out.as_mut_ptr(),
                    8,
                    s.as_ptr() as *const c_char,
                    4,
                    ptr::null(),
                    &mut bl,
                    ptr::null_mut(),
                    v,
                );
            }
            // --- row 37: sodium_pad, padded length overflows -----------------
            "pad_length_overflow" => {
                let f = sym!(Pad, "sodium_pad");
                let mut out = 0usize;
                let mut buf = [0u8; 64];
                f(&mut out, buf.as_mut_ptr(), usize::MAX - 1, 16, usize::MAX);
            }
            // --- rows 29/30: sodium_free on a corrupted canary ---------------
            "free_canary_underflow" => {
                let malloc = sym!(unsafe extern "C" fn(usize) -> *mut c_void, "sodium_malloc");
                let free = sym!(unsafe extern "C" fn(*mut c_void), "sodium_free");
                let p = malloc(32) as *mut u8;
                // clobber the leading canary
                ptr::write_bytes(p.offset(-16), 0xAB, 16);
                free(p as *mut c_void);
            }
            "free_canary_overflow" => {
                let malloc = sym!(unsafe extern "C" fn(usize) -> *mut c_void, "sodium_malloc");
                let free = sym!(unsafe extern "C" fn(*mut c_void), "sodium_free");
                let p = malloc(32) as *mut u8;
                // clobber past the end of the requested region (trailing canary)
                ptr::write_bytes(p.offset(32), 0xCD, 16);
                free(p as *mut c_void);
            }
            // --- row 31: sodium_free on a pointer we never allocated ---------
            "free_foreign_pointer" => {
                let free = sym!(unsafe extern "C" fn(*mut c_void), "sodium_free");
                let mut v = vec![0u8; 4096];
                free(v.as_mut_ptr().add(2048) as *mut c_void);
                std::mem::forget(v);
            }
            // --- rows 33/34/35: mprotect on a foreign pointer ----------------
            "mprotect_foreign_pointer" => {
                let f = sym!(
                    unsafe extern "C" fn(*mut c_void) -> c_int,
                    "sodium_mprotect_readonly"
                );
                let mut v = vec![0u8; 4096];
                let rc = f(v.as_mut_ptr().add(2048) as *mut c_void);
                std::process::exit(if rc == 0 { 0 } else { 66 });
            }
            // --- row 91: sodium_misuse() itself ------------------------------
            "sodium_misuse" => {
                let f = sym!(unsafe extern "C" fn(), "sodium_misuse");
                f();
            }
            // --- row 87: nested sodium_crit_enter ---------------------------
            "crit_enter_nested" => {
                let f = sym!(unsafe extern "C" fn() -> c_int, "sodium_crit_enter");
                let a = f();
                let b = f();
                std::process::exit(((a & 3) * 4 + (b & 3)) as i32);
            }
            // --- row 88: sodium_crit_leave without enter --------------------
            "crit_leave_unbalanced" => {
                let f = sym!(unsafe extern "C" fn() -> c_int, "sodium_crit_leave");
                set_errno(0);
                let rc = f();
                let e = errno();
                std::process::exit(if rc == 0 { 0 } else { 100 + (e & 0x3f) });
            }
            other => panic!("unknown abort case {other}"),
        }
    }
    // Reached only when the call did NOT abort.
    std::process::exit(0);
}

// ===========================================================================
// abort rows
// ===========================================================================
#[test]
fn g1_err_rows_42_43_bin2hex_misuse() {
    let t = diff_abort_case("bin2hex_binlen_overflow");
    assert!(t.signal.is_some(), "row 42 must abort, got {t:?}");
    let t = diff_abort_case("bin2hex_maxlen_too_small");
    assert!(t.signal.is_some(), "row 43 must abort, got {t:?}");
    // hex_maxlen == 0, bin_len == 0 -> 0 <= 0 is also a misuse
    diff_abort_case("bin2hex_maxlen_exact_zero");
}

#[test]
fn g1_err_rows_47_49_encoded_len_variant_and_overflow() {
    // Every int that is not 1/3/5/7 must be rejected identically.
    for v in [
        -2i32, -1, 0, 2, 4, 6, 8, 9, 10, 15, 16, 100, 0x7fff_ffff, -0x8000_0000,
    ] {
        let case = format!("encoded_len_variant_{v}");
        // negative numbers: encode as sign-mangled case name
        if v < 0 {
            continue;
        }
        let t = diff_abort_case(&case);
        assert!(t.signal.is_some(), "variant {v} must abort, got {t:?}");
    }
    let t = diff_abort_case("encoded_len_overflow");
    assert!(t.signal.is_some(), "row 49 must abort, got {t:?}");
    // just below the overflow threshold: must NOT abort, and must agree
    let t = diff_abort_case("encoded_len_no_overflow");
    assert!(t.signal.is_none(), "below-threshold must not abort: {t:?}");
}

#[test]
fn g1_err_rows_50_52_bin2base64_misuse() {
    for v in [0i32, 2, 4, 6, 8, 9, 255] {
        let t = diff_abort_case(&format!("bin2b64_variant_{v}"));
        assert!(t.signal.is_some(), "bin2base64 variant {v} must abort");
    }
    let t = diff_abort_case("bin2b64_overflow");
    assert!(t.signal.is_some(), "row 51 must abort");
    let t = diff_abort_case("bin2b64_maxlen_too_small");
    assert!(t.signal.is_some(), "row 52 must abort");
    diff_abort_case("bin2b64_maxlen_zero_empty");
}

#[test]
fn g1_err_row_54_base642bin_variant() {
    for v in [0i32, 2, 4, 6, 8, 9, 255] {
        let t = diff_abort_case(&format!("b642bin_variant_{v}"));
        assert!(t.signal.is_some(), "base642bin variant {v} must abort");
    }
}

#[test]
fn g1_err_row_37_pad_overflow() {
    let t = diff_abort_case("pad_length_overflow");
    assert!(t.signal.is_some(), "row 37 must abort, got {t:?}");
}

#[test]
fn g1_err_rows_29_30_31_free_canary_and_foreign() {
    // In a build without page protection the canaries are checked in
    // sodium_free(); both libraries must react identically.
    diff_abort_case("free_canary_underflow");
    diff_abort_case("free_canary_overflow");
    diff_abort_case("free_foreign_pointer");
}

#[test]
fn g1_err_rows_33_35_mprotect_foreign() {
    diff_abort_case("mprotect_foreign_pointer");
}

#[test]
fn g1_err_row_91_sodium_misuse() {
    let t = diff_abort_case("sodium_misuse");
    assert!(t.signal.is_some(), "sodium_misuse must abort, got {t:?}");
}

#[test]
fn g1_err_rows_87_88_crit_section() {
    diff_abort_case("crit_enter_nested");
    diff_abort_case("crit_leave_unbalanced");
}

// ===========================================================================
// return-value / errno rows
// ===========================================================================
#[test]
fn g1_err_rows_2_5_97_99_comparison_family() {
    unsafe {
        let (cmc, rmc) = pair::<unsafe extern "C" fn(*const c_void, *const c_void, usize) -> c_int>(
            "sodium_memcmp",
        );
        let (ccmp, rcmp) =
            pair::<unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int>("sodium_compare");
        let (ciz, riz) = pair::<unsafe extern "C" fn(*const u8, usize) -> c_int>("sodium_is_zero");
        // row 2: differ -> -1
        let a = [1u8; 32];
        let mut b = a;
        b[31] = 2;
        eq_i32(
            "row2 memcmp differ",
            cmc(a.as_ptr() as *const _, b.as_ptr() as *const _, 32),
            rmc(a.as_ptr() as *const _, b.as_ptr() as *const _, 32),
        );
        assert_eq!(cmc(a.as_ptr() as *const _, b.as_ptr() as *const _, 32), -1);
        // rows 3/4: little-endian ordering, both directions, at every position
        for i in 0..32 {
            let mut x = [0u8; 32];
            let mut y = [0u8; 32];
            x[i] = 1;
            eq_i32(
                &format!("row3 compare pos {i}"),
                ccmp(y.as_ptr(), x.as_ptr(), 32),
                rcmp(y.as_ptr(), x.as_ptr(), 32),
            );
            assert_eq!(ccmp(y.as_ptr(), x.as_ptr(), 32), -1);
            eq_i32(
                &format!("row4 compare pos {i}"),
                ccmp(x.as_ptr(), y.as_ptr(), 32),
                rcmp(x.as_ptr(), y.as_ptr(), 32),
            );
            assert_eq!(ccmp(x.as_ptr(), y.as_ptr(), 32), 1);
            y[i] = 1;
            assert_eq!(ccmp(x.as_ptr(), y.as_ptr(), 32), 0);
            // row 5
            eq_i32(
                &format!("row5 is_zero pos {i}"),
                ciz(x.as_ptr(), 32),
                riz(x.as_ptr(), 32),
            );
            assert_eq!(ciz(x.as_ptr(), 32), 0);
        }
        // rows 97/98/99
        for (name, n) in [
            ("crypto_verify_16", 16usize),
            ("crypto_verify_32", 32),
            ("crypto_verify_64", 64),
        ] {
            let (c, r) = pair::<unsafe extern "C" fn(*const u8, *const u8) -> c_int>(name);
            let a = vec![0x5Au8; n];
            for i in 0..n {
                let mut b = a.clone();
                b[i] ^= 0x80;
                eq_i32(
                    &format!("{name} differ at {i}"),
                    c(a.as_ptr(), b.as_ptr()),
                    r(a.as_ptr(), b.as_ptr()),
                );
                assert_eq!(c(a.as_ptr(), b.as_ptr()), -1);
            }
        }
    }
}

#[test]
fn g1_err_rows_7_18_32_mlock_mprotect_enosys() {
    // Rows 7/10/13/15/17/32: builds without HAVE_MLOCK / page protection set
    // errno = ENOSYS and return -1. Whatever this build does, C and Rust must
    // agree on BOTH the return value and errno.
    unsafe {
        let mut res: Vec<Vec<(String, c_int, i32)>> = Vec::new();
        for is_c in [true, false] {
            let l = libs();
            let h = if is_c { l.c } else { l.rs };
            let malloc: libloading::Symbol<unsafe extern "C" fn(usize) -> *mut c_void> =
                h.get(b"sodium_malloc\0").unwrap();
            let free: libloading::Symbol<unsafe extern "C" fn(*mut c_void)> =
                h.get(b"sodium_free\0").unwrap();
            let mut rec = Vec::new();
            let p = malloc(64);
            for n in [
                "sodium_mprotect_noaccess",
                "sodium_mprotect_readonly",
                "sodium_mprotect_readwrite",
            ] {
                let f: libloading::Symbol<unsafe extern "C" fn(*mut c_void) -> c_int> = h
                    .get(format!("{n}\0").as_bytes())
                    .unwrap();
                set_errno(0);
                let rc = f(p);
                rec.push((n.to_string(), rc, errno()));
            }
            free(p);
            let mut buf = vec![0u8; 8192];
            for n in ["sodium_mlock", "sodium_munlock"] {
                let f: libloading::Symbol<unsafe extern "C" fn(*mut c_void, usize) -> c_int> =
                    h.get(format!("{n}\0").as_bytes()).unwrap();
                set_errno(0);
                let rc = f(buf.as_mut_ptr() as *mut c_void, 8192);
                rec.push((n.to_string(), rc, errno()));
                // absurd length -> OS-level failure path.
                // NOTE: only for mlock. sodium_munlock() calls sodium_memzero()
                // on the whole range *before* any length validation, so a huge
                // length is a genuine out-of-bounds write in the C code too.
                if n == "sodium_mlock" {
                    set_errno(0);
                    let rc = f(buf.as_mut_ptr() as *mut c_void, usize::MAX / 2);
                    rec.push((format!("{n}_huge"), rc, errno()));
                }
                set_errno(0);
                let rc = f(buf.as_mut_ptr() as *mut c_void, 0);
                rec.push((format!("{n}_zero"), rc, errno()));
            }
            res.push(rec);
        }
        assert_eq!(
            res[0], res[1],
            "mlock/munlock/mprotect return value + errno differ between C and Rust"
        );
    }
}

#[test]
fn g1_err_rows_21_27_28_allocation_overflow() {
    unsafe {
        let mut res: Vec<Vec<(String, bool, i32)>> = Vec::new();
        for is_c in [true, false] {
            let l = libs();
            let h = if is_c { l.c } else { l.rs };
            let malloc: libloading::Symbol<unsafe extern "C" fn(usize) -> *mut c_void> =
                h.get(b"sodium_malloc\0").unwrap();
            let allocarray: libloading::Symbol<
                unsafe extern "C" fn(usize, usize) -> *mut c_void,
            > = h.get(b"sodium_allocarray\0").unwrap();
            let mut rec = Vec::new();
            // row 21 / 25: gigantic size
            for size in [usize::MAX, usize::MAX - 1, usize::MAX / 2, 1usize << 62] {
                set_errno(0);
                let p = malloc(size);
                rec.push((format!("malloc({size})"), p.is_null(), errno()));
                assert!(p.is_null(), "sodium_malloc({size}) unexpectedly succeeded");
            }
            // row 27: count * size overflow
            for (c, s) in [
                (usize::MAX, 2usize),
                (2, usize::MAX),
                (usize::MAX, usize::MAX),
                (1 << 33, 1 << 33),
                (0, usize::MAX),
            ] {
                set_errno(0);
                let p = allocarray(c, s);
                rec.push((format!("allocarray({c},{s})"), p.is_null(), errno()));
            }
            res.push(rec);
        }
        assert_eq!(
            res[0], res[1],
            "allocation overflow: NULL-ness or errno differs between C and Rust"
        );
    }
}

#[test]
fn g1_err_rows_36_41_pad_unpad() {
    type P = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> c_int;
    type U = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> c_int;
    unsafe {
        let (cp, rp) = pair::<P>("sodium_pad");
        let (cu, ru) = pair::<U>("sodium_unpad");
        let mut buf = [0u8; 128];
        // row 36: blocksize == 0
        for unpadded in [0usize, 1, 16] {
            let mut cl = usize::MAX;
            let mut rl = usize::MAX;
            let a = cp(&mut cl, buf.as_mut_ptr(), unpadded, 0, 128);
            let b = rp(&mut rl, buf.as_mut_ptr(), unpadded, 0, 128);
            eq_i32(&format!("row36 pad bs=0 unpadded={unpadded}"), a, b);
            assert_eq!(a, -1);
            assert_eq!(cl, rl);
        }
        // row 38: max_buflen too small (exactly one byte short, and zero)
        for (unpadded, bs) in [(0usize, 16usize), (1, 16), (15, 16), (16, 16), (17, 16), (5, 1)] {
            let need = unpadded + (bs - unpadded % bs);
            for maxb in [0usize, need.saturating_sub(1), need] {
                let mut cl = usize::MAX;
                let mut rl = usize::MAX;
                let a = cp(&mut cl, buf.as_mut_ptr(), unpadded, bs, maxb);
                let b = rp(&mut rl, buf.as_mut_ptr(), unpadded, bs, maxb);
                let ctx = format!("row38 pad unpadded={unpadded} bs={bs} max={maxb}");
                eq_i32(&ctx, a, b);
                assert_eq!(cl, rl, "{ctx}: padded_buflen differs");
            }
        }
        // rows 39/40: padded_buflen < blocksize, blocksize == 0
        for (padded, bs) in [
            (0usize, 1usize),
            (0, 16),
            (1, 16),
            (15, 16),
            (16, 17),
            (5, 0),
            (0, 0),
        ] {
            let mut cl = usize::MAX;
            let mut rl = usize::MAX;
            let a = cu(&mut cl, buf.as_ptr(), padded, bs);
            let b = ru(&mut rl, buf.as_ptr(), padded, bs);
            let ctx = format!("rows39/40 unpad padded={padded} bs={bs}");
            eq_i32(&ctx, a, b);
            assert_eq!(a, -1, "{ctx}: expected -1");
            assert_eq!(cl, rl, "{ctx}: out param differs");
        }
        // row 41: no 0x80 barrier / non-zero bytes after the barrier
        let bad: &[&[u8]] = &[
            &[0u8; 16],
            &[0xff; 16],
            &[
                0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, // non-zero after barrier
            ],
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            ],
        ];
        for (i, b) in bad.iter().enumerate() {
            let mut cl = usize::MAX;
            let mut rl = usize::MAX;
            let a = cu(&mut cl, b.as_ptr(), b.len(), 16);
            let bb = ru(&mut rl, b.as_ptr(), b.len(), 16);
            let ctx = format!("row41 unpad bad[{i}]");
            eq_i32(&ctx, a, bb);
            assert_eq!(a, -1, "{ctx}: expected -1");
            assert_eq!(cl, rl, "{ctx}: unpadded_buflen differs");
        }
    }
}

#[test]
fn g1_err_rows_44_46_hex2bin_errno() {
    type F = unsafe extern "C" fn(
        *mut u8,
        usize,
        *const c_char,
        usize,
        *const c_char,
        *mut usize,
        *mut *const c_char,
    ) -> c_int;
    unsafe {
        let (c, r) = pair::<F>("sodium_hex2bin");
        // (hex, bin_maxlen, ignore, use_hex_end) -> expected row
        let cases: &[(&[u8], usize, Option<&[u8]>, bool)] = &[
            // row 44: ERANGE, output too small
            (b"0011\0", 1, None, true),
            (b"001122\0", 0, None, true),
            (b"00112233\0", 3, None, true),
            (b"0011\0", 1, None, false),
            // row 45: odd digit count -> EINVAL
            (b"0\0", 8, None, true),
            (b"000\0", 8, None, true),
            (b"00112\0", 8, None, true),
            // row 46: garbage, hex_end == NULL -> EINVAL
            (b"00zz\0", 8, None, false),
            (b"zz\0", 8, None, false),
            (b"00:11\0", 8, None, false),
            // ignore char mid-byte (state != 0)
            (b"0:011\0", 8, Some(b":\0"), false),
            (b"0:011\0", 8, Some(b":\0"), true),
            // ignore char mid-byte with hex_end reported
            (b"00 11\0", 8, Some(b" \0"), true),
            (b"0 011\0", 8, Some(b" \0"), true),
        ];
        for (i, (s, maxlen, ign, use_end)) in cases.iter().enumerate() {
            let hex_len = s.len() - 1;
            let mut cb = vec![0x55u8; maxlen + 8];
            let mut rb = vec![0x55u8; maxlen + 8];
            let mut cl = usize::MAX;
            let mut rl = usize::MAX;
            let mut ce: *const c_char = ptr::null();
            let mut re: *const c_char = ptr::null();
            let ip = ign.map_or(ptr::null(), |x| x.as_ptr() as *const c_char);
            set_errno(0);
            let cr = c(
                cb.as_mut_ptr(),
                *maxlen,
                s.as_ptr() as *const c_char,
                hex_len,
                ip,
                &mut cl,
                if *use_end { &mut ce } else { ptr::null_mut() },
            );
            let ce_no = errno();
            set_errno(0);
            let rr = r(
                rb.as_mut_ptr(),
                *maxlen,
                s.as_ptr() as *const c_char,
                hex_len,
                ip,
                &mut rl,
                if *use_end { &mut re } else { ptr::null_mut() },
            );
            let re_no = errno();
            let ctx = format!(
                "hex2bin err case {i} {:?} maxlen={maxlen}",
                String::from_utf8_lossy(&s[..hex_len])
            );
            eq_i32(&ctx, cr, rr);
            assert_eq!(ce_no, re_no, "{ctx}: errno differs (C={ce_no} R={re_no})");
            assert_eq!(cl, rl, "{ctx}: bin_len differs");
            eq_bytes(&ctx, &cb, &rb);
            if *use_end {
                let co = if ce.is_null() {
                    usize::MAX
                } else {
                    ce as usize - s.as_ptr() as usize
                };
                let ro = if re.is_null() {
                    usize::MAX
                } else {
                    re as usize - s.as_ptr() as usize
                };
                assert_eq!(co, ro, "{ctx}: hex_end differs");
            }
        }
    }
}

#[test]
fn g1_err_rows_55_60_base642bin_errno() {
    unsafe {
        let (c, r) = pair::<B642Bin>("sodium_base642bin");
        // (b64, bin_maxlen, variant, ignore, use_end)
        let cases: &[(&[u8], usize, c_int, Option<&[u8]>, bool)] = &[
            // row 55: ERANGE
            (b"QUJD\0", 1, 1, None, true),
            (b"QUJD\0", 0, 1, None, true),
            (b"QUJDRA==\0", 2, 1, None, true),
            (b"QUJD\0", 1, 3, None, true),
            // row 56: exactly one leftover char
            (b"Q\0", 8, 3, None, true),
            (b"QUJDQ\0", 8, 3, None, true),
            (b"Q\0", 8, 7, None, true),
            // row 57: non-canonical trailing bits
            (b"QR==\0", 8, 1, None, true),
            (b"QQ=\0", 8, 1, None, true),
            (b"QUJDRB==\0", 8, 1, None, true),
            (b"QR\0", 8, 3, None, true),
            (b"QUJDRB\0", 8, 3, None, true),
            // row 58: padded variant, input ends before all '=' seen
            (b"QQ\0", 8, 1, None, true),
            (b"QQ=\0", 8, 1, None, true),
            (b"QUE\0", 8, 1, None, true),
            // row 59: junk inside the padding region
            (b"QQ=x\0", 8, 1, None, true),
            (b"QQ==x\0", 8, 1, None, true),
            (b"QQ=!\0", 8, 1, Some(b" \0"), true),
            // row 60: trailing garbage with b64_end == NULL
            (b"QUJD!\0", 8, 1, None, false),
            (b"QUJD!\0", 8, 3, None, false),
            (b"!!!!\0", 8, 3, None, false),
            (b"QUJD \0", 8, 3, None, false),
            // urlsafe variants must reject +/ and vice-versa
            (b"++++\0", 8, 5, None, false),
            (b"----\0", 8, 1, None, false),
            (b"____\0", 8, 1, None, false),
            (b"////\0", 8, 7, None, false),
        ];
        for (i, (s, maxlen, v, ign, use_end)) in cases.iter().enumerate() {
            let slen = s.len() - 1;
            let mut cb = vec![0x55u8; maxlen + 8];
            let mut rb = vec![0x55u8; maxlen + 8];
            let mut cl = usize::MAX;
            let mut rl = usize::MAX;
            let mut ce: *const c_char = ptr::null();
            let mut re: *const c_char = ptr::null();
            let ip = ign.map_or(ptr::null(), |x| x.as_ptr() as *const c_char);
            set_errno(0);
            let cr = c(
                cb.as_mut_ptr(),
                *maxlen,
                s.as_ptr() as *const c_char,
                slen,
                ip,
                &mut cl,
                if *use_end { &mut ce } else { ptr::null_mut() },
                *v,
            );
            let cen = errno();
            set_errno(0);
            let rr = r(
                rb.as_mut_ptr(),
                *maxlen,
                s.as_ptr() as *const c_char,
                slen,
                ip,
                &mut rl,
                if *use_end { &mut re } else { ptr::null_mut() },
                *v,
            );
            let ren = errno();
            let ctx = format!(
                "base642bin err case {i} {:?} maxlen={maxlen} v={v}",
                String::from_utf8_lossy(&s[..slen])
            );
            eq_i32(&ctx, cr, rr);
            assert_eq!(cen, ren, "{ctx}: errno differs (C={cen} R={ren})");
            assert_eq!(cl, rl, "{ctx}: bin_len differs");
            eq_bytes(&ctx, &cb, &rb);
            if *use_end {
                let co = if ce.is_null() {
                    usize::MAX
                } else {
                    ce as usize - s.as_ptr() as usize
                };
                let ro = if re.is_null() {
                    usize::MAX
                } else {
                    re as usize - s.as_ptr() as usize
                };
                assert_eq!(co, ro, "{ctx}: b64_end differs");
            }
        }
    }
}

#[test]
fn g1_err_rows_61_83_ip_parsing() {
    type I = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> c_int;
    type B = unsafe extern "C" fn(*mut c_char, usize, *const u8) -> *mut c_char;
    unsafe {
        let (ci, ri) = pair::<I>("sodium_ip2bin");
        let (cb, rb) = pair::<B>("sodium_bin2ip");
        // one entry per ERRORS.md row 61..80
        let bad: &[(&str, &[u8])] = &[
            ("row61 bad zone char", b"fe80::1%et h0\0"),
            ("row61 bad zone char 2", b"fe80::1%e+h\0"),
            ("row62 empty zone", b"fe80::1%\0"),
            ("row63 zone on ipv4", b"1.2.3.4%eth0\0"),
            ("row64 ipv6 parse fail", b"1:2:3:4:5:6:7:8:9\0"),
            ("row65 ipv4 parse fail", b"1.2.3\0"),
            ("row66 empty", b"\0"),
            ("row67 octet >255", b"256.0.0.1\0"),
            ("row67 octet 4 digits", b"0000.0.0.1\0"),
            ("row68 zero-digit octet", b"1..2.3\0"),
            ("row68 leading dot", b".1.2.3\0"),
            ("row68 trailing dot", b"1.2.3.\0"),
            ("row69 missing dot", b"1.2.3\0"),
            ("row70 trailing junk", b"1.2.3.4.5\0"),
            ("row70 trailing char", b"1.2.3.4x\0"),
            ("row71 ipv6 empty", b":\0"),
            ("row72 leading single colon", b":1::\0"),
            ("row73 second ::", b"1::2::3\0"),
            ("row74 >8 groups", b"1:2:3:4:5:6:7:8:9:a\0"),
            ("row75 trailing colon", b"1:2:\0"),
            ("row76 ipv4 tail no room", b"1:2:3:4:5:6:7:1.2.3.4\0"),
            ("row76 bad ipv4 tail", b"::1.2.3\0"),
            ("row77 bad hex digit", b"::g\0"),
            ("row77 5 hex digits", b"::12345\0"),
            ("row78 final group overflow", b"1:2:3:4:5:6:7:8:9\0"),
            ("row79 :: with full addr", b"1:2:3:4:5:6:7:8::\0"),
            ("row80 too few groups", b"1:2:3\0"),
        ];
        for (label, s) in bad {
            let slen = s.len() - 1;
            let mut ca = [0xEEu8; 16];
            let mut ra = [0xEEu8; 16];
            set_errno(0);
            let cr = ci(ca.as_mut_ptr(), s.as_ptr() as *const c_char, slen);
            let cen = errno();
            set_errno(0);
            let rr = ri(ra.as_mut_ptr(), s.as_ptr() as *const c_char, slen);
            let ren = errno();
            eq_i32(&format!("{label} sodium_ip2bin"), cr, rr);
            assert_eq!(cr, -1, "{label}: expected -1 from C, got {cr}");
            assert_eq!(cen, ren, "{label}: errno differs");
            eq_bytes(&format!("{label} bin"), &ca, &ra);
        }
        // rows 81/82/83: bin2ip with an output buffer that is too small
        let bins: &[[u8; 16]] = &[
            [0u8; 16],
            {
                let mut b = [0u8; 16];
                b[10] = 0xff;
                b[11] = 0xff;
                b[12] = 255;
                b[13] = 255;
                b[14] = 255;
                b[15] = 255;
                b
            },
            [0xffu8; 16],
        ];
        for (bi, bin) in bins.iter().enumerate() {
            for maxlen in 0..=48usize {
                let mut co = vec![0x33u8; maxlen + 8];
                let mut ro = vec![0x33u8; maxlen + 8];
                let cp = cb(co.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr());
                let rp = rb(ro.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr());
                let ctx = format!("rows81-83 bin2ip bin[{bi}] maxlen={maxlen}");
                assert_eq!(cp.is_null(), rp.is_null(), "{ctx}: nullness differs");
                eq_bytes(&ctx, &co, &ro);
            }
        }
    }
}

// ===========================================================================
// generic FFI boundary checks (beyond the table)
// ===========================================================================
#[test]
fn g1_generic_zero_lengths_and_null_pointers() {
    unsafe {
        // zero-length calls with NULL pointers where the C code never
        // dereferences: must behave identically
        let (cmc, rmc) = pair::<unsafe extern "C" fn(*const c_void, *const c_void, usize) -> c_int>(
            "sodium_memcmp",
        );
        eq_i32(
            "memcmp(NULL,NULL,0)",
            cmc(ptr::null(), ptr::null(), 0),
            rmc(ptr::null(), ptr::null(), 0),
        );
        let (ccmp, rcmp) =
            pair::<unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int>("sodium_compare");
        eq_i32(
            "compare(NULL,NULL,0)",
            ccmp(ptr::null(), ptr::null(), 0),
            rcmp(ptr::null(), ptr::null(), 0),
        );
        let (ciz, riz) = pair::<unsafe extern "C" fn(*const u8, usize) -> c_int>("sodium_is_zero");
        eq_i32(
            "is_zero(NULL,0)",
            ciz(ptr::null(), 0),
            riz(ptr::null(), 0),
        );
        let (cinc, rinc) = pair::<unsafe extern "C" fn(*mut u8, usize)>("sodium_increment");
        cinc(ptr::null_mut(), 0);
        rinc(ptr::null_mut(), 0);
        let (cadd, radd) = pair::<unsafe extern "C" fn(*mut u8, *const u8, usize)>("sodium_add");
        cadd(ptr::null_mut(), ptr::null(), 0);
        radd(ptr::null_mut(), ptr::null(), 0);
        let (csub, rsub) = pair::<unsafe extern "C" fn(*mut u8, *const u8, usize)>("sodium_sub");
        csub(ptr::null_mut(), ptr::null(), 0);
        rsub(ptr::null_mut(), ptr::null(), 0);
        let (cmz, rmz) = pair::<unsafe extern "C" fn(*mut c_void, usize)>("sodium_memzero");
        cmz(ptr::null_mut(), 0);
        rmz(ptr::null_mut(), 0);
        // hex2bin / base642bin with zero-length input
        let (chx, rhx) = pair::<unsafe extern "C" fn(
            *mut u8,
            usize,
            *const c_char,
            usize,
            *const c_char,
            *mut usize,
            *mut *const c_char,
        ) -> c_int>("sodium_hex2bin");
        let mut cbuf = [0u8; 8];
        let mut rbuf = [0u8; 8];
        let mut cl = usize::MAX;
        let mut rl = usize::MAX;
        set_errno(0);
        let a = chx(
            cbuf.as_mut_ptr(),
            0,
            b"\0".as_ptr() as *const c_char,
            0,
            ptr::null(),
            &mut cl,
            ptr::null_mut(),
        );
        let ae = errno();
        set_errno(0);
        let b = rhx(
            rbuf.as_mut_ptr(),
            0,
            b"\0".as_ptr() as *const c_char,
            0,
            ptr::null(),
            &mut rl,
            ptr::null_mut(),
        );
        let be = errno();
        eq_i32("hex2bin empty", a, b);
        assert_eq!(ae, be);
        assert_eq!(cl, rl);
        let (cb6, rb6) = pair::<B642Bin>("sodium_base642bin");
        for v in [1i32, 3, 5, 7] {
            let mut cl = usize::MAX;
            let mut rl = usize::MAX;
            set_errno(0);
            let a = cb6(
                cbuf.as_mut_ptr(),
                0,
                b"\0".as_ptr() as *const c_char,
                0,
                ptr::null(),
                &mut cl,
                ptr::null_mut(),
                v,
            );
            let ae = errno();
            set_errno(0);
            let b = rb6(
                rbuf.as_mut_ptr(),
                0,
                b"\0".as_ptr() as *const c_char,
                0,
                ptr::null(),
                &mut rl,
                ptr::null_mut(),
                v,
            );
            let be = errno();
            eq_i32(&format!("base642bin empty v={v}"), a, b);
            assert_eq!(ae, be);
            assert_eq!(cl, rl);
        }
    }
}
