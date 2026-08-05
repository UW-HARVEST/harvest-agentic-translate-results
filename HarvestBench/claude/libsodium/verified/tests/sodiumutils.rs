//! Differential tests for the SODIUM UTILS + CODECS + RUNTIME family.
//!
//! Every call goes through the exported symbol loaded from BOTH the C `.so`
//! and the Rust cdylib; results are compared byte-for-byte.
//!
//! NOTE ON ABORTING PATHS: several C entry points call `sodium_misuse()` ->
//! `abort()` on misuse (e.g. `sodium_bin2hex` with too-small buffer,
//! `sodium_bin2base64`/`sodium_base64_encoded_len` with an invalid variant or
//! too-small buffer). We deliberately do NOT exercise those in-process because
//! aborting would kill the whole test binary; they are documented in
//! docs/sodiumutils_ERRORS.md instead. Only the graceful (`-1` / sentinel)
//! rejection paths are exercised here.

#[macro_use]
mod common;
use common::{libs, Rng};

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

type Hex2BinFn = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
) -> c_int;

type Base642BinFn = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
    c_int,
) -> c_int;

// Base64 variant constants (from utils.h).
const VARIANT_ORIGINAL: c_int = 1;
const VARIANT_ORIGINAL_NO_PADDING: c_int = 3;
const VARIANT_URLSAFE: c_int = 5;
const VARIANT_URLSAFE_NO_PADDING: c_int = 7;
const ALL_VARIANTS: [c_int; 4] = [
    VARIANT_ORIGINAL,
    VARIANT_ORIGINAL_NO_PADDING,
    VARIANT_URLSAFE,
    VARIANT_URLSAFE_NO_PADDING,
];

// ---------------------------------------------------------------------------
// sodium_memcmp
// ---------------------------------------------------------------------------
#[test]
fn memcmp_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(
            l,
            b"sodium_memcmp",
            unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int
        );
        let mut rng = Rng::new(0x11);
        for _ in 0..2000 {
            let len = rng.range(64);
            let a = rng.vec(len);
            let mut b = a.clone();
            // 50% of the time flip a random byte so we exercise both equal & differing.
            if len > 0 && rng.next_u64() & 1 == 0 {
                let idx = rng.range(len);
                b[idx] ^= 1 + (rng.next_u64() & 0xff) as u8;
            }
            let cc = c(a.as_ptr(), b.as_ptr(), len);
            let rr = r(a.as_ptr(), b.as_ptr(), len);
            assert_eq!(cc, rr, "memcmp len={} a={:?} b={:?}", len, a, b);
        }
        // len == 0 always equal.
        assert_eq!(c(a_null(), a_null(), 0), r(a_null(), a_null(), 0));
    }
}

fn a_null() -> *const u8 {
    // safe: len==0 means the pointer is never dereferenced.
    std::ptr::NonNull::<u8>::dangling().as_ptr()
}

// ---------------------------------------------------------------------------
// sodium_compare
// ---------------------------------------------------------------------------
#[test]
fn compare_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(
            l,
            b"sodium_compare",
            unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int
        );
        let mut rng = Rng::new(0x22);
        for _ in 0..3000 {
            let len = rng.range(40);
            let a = rng.vec(len);
            let mut b = rng.vec(len);
            // occasionally make them equal, or off by one in the top limb.
            match rng.next_u64() % 4 {
                0 => b = a.clone(),
                1 if len > 0 => {
                    b = a.clone();
                    let i = rng.range(len);
                    b[i] = b[i].wrapping_add(1);
                }
                _ => {}
            }
            let cc = c(a.as_ptr(), b.as_ptr(), len);
            let rr = r(a.as_ptr(), b.as_ptr(), len);
            assert_eq!(cc, rr, "compare len={} a={:?} b={:?}", len, a, b);
            assert!((-1..=1).contains(&cc));
        }
        // fixed edge cases across several lengths (little-endian ordering).
        for &len in &[1usize, 2, 8, 12, 16, 24, 32] {
            let hi = vec![0xffu8; len];
            let lo = vec![0x00u8; len];
            assert_eq!(c(lo.as_ptr(), hi.as_ptr(), len), r(lo.as_ptr(), hi.as_ptr(), len));
            assert_eq!(c(hi.as_ptr(), lo.as_ptr(), len), r(hi.as_ptr(), lo.as_ptr(), len));
            assert_eq!(c(hi.as_ptr(), hi.as_ptr(), len), r(hi.as_ptr(), hi.as_ptr(), len));
        }
    }
}

// ---------------------------------------------------------------------------
// sodium_is_zero
// ---------------------------------------------------------------------------
#[test]
fn is_zero_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(
            l,
            b"sodium_is_zero",
            unsafe extern "C" fn(*const u8, usize) -> c_int
        );
        let mut rng = Rng::new(0x33);
        for _ in 0..2000 {
            let len = rng.range(48);
            let mut v = vec![0u8; len];
            // mostly-zero with occasional nonzero byte.
            if len > 0 && rng.next_u64() % 3 == 0 {
                v[rng.range(len)] = 1 + (rng.next_u64() & 0xff) as u8;
            }
            assert_eq!(c(v.as_ptr(), len), r(v.as_ptr(), len), "is_zero {:?}", v);
        }
        // empty -> both return 1.
        assert_eq!(c(a_null(), 0), r(a_null(), 0));
    }
}

// ---------------------------------------------------------------------------
// sodium_increment
// ---------------------------------------------------------------------------
#[test]
fn increment_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(
            l,
            b"sodium_increment",
            unsafe extern "C" fn(*mut u8, usize)
        );
        let mut rng = Rng::new(0x44);
        for _ in 0..3000 {
            let len = rng.range(40);
            let base = rng.vec(len);
            let mut a = base.clone();
            let mut b = base.clone();
            c(a.as_mut_ptr(), len);
            r(b.as_mut_ptr(), len);
            assert_eq!(a, b, "increment len={} base={:?}", len, base);
        }
        // carry-chain edge cases at the 8/12/24 asm-relevant lengths.
        for &len in &[0usize, 1, 8, 12, 16, 24, 32] {
            for pat in [0x00u8, 0xff] {
                let mut a = vec![pat; len];
                let mut b = vec![pat; len];
                c(a.as_mut_ptr(), len);
                r(b.as_mut_ptr(), len);
                assert_eq!(a, b, "increment edge len={} pat={:#x}", len, pat);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// sodium_add
// ---------------------------------------------------------------------------
#[test]
fn add_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(
            l,
            b"sodium_add",
            unsafe extern "C" fn(*mut u8, *const u8, usize)
        );
        let mut rng = Rng::new(0x55);
        for _ in 0..3000 {
            let len = rng.range(40);
            let a0 = rng.vec(len);
            let b = rng.vec(len);
            let mut a1 = a0.clone();
            let mut a2 = a0.clone();
            c(a1.as_mut_ptr(), b.as_ptr(), len);
            r(a2.as_mut_ptr(), b.as_ptr(), len);
            assert_eq!(a1, a2, "add len={} a={:?} b={:?}", len, a0, b);
        }
        for &len in &[0usize, 1, 8, 12, 24, 32, 64] {
            let a0 = vec![0xffu8; len];
            let b = vec![0xffu8; len];
            let mut a1 = a0.clone();
            let mut a2 = a0.clone();
            c(a1.as_mut_ptr(), b.as_ptr(), len);
            r(a2.as_mut_ptr(), b.as_ptr(), len);
            assert_eq!(a1, a2, "add carry edge len={}", len);
        }
    }
}

// ---------------------------------------------------------------------------
// sodium_sub
// ---------------------------------------------------------------------------
#[test]
fn sub_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(
            l,
            b"sodium_sub",
            unsafe extern "C" fn(*mut u8, *const u8, usize)
        );
        let mut rng = Rng::new(0x66);
        for _ in 0..3000 {
            let len = rng.range(80);
            let a0 = rng.vec(len);
            let b = rng.vec(len);
            let mut a1 = a0.clone();
            let mut a2 = a0.clone();
            c(a1.as_mut_ptr(), b.as_ptr(), len);
            r(a2.as_mut_ptr(), b.as_ptr(), len);
            assert_eq!(a1, a2, "sub len={} a={:?} b={:?}", len, a0, b);
        }
        // borrow edge cases incl the 64-byte asm-relevant length.
        for &len in &[0usize, 1, 8, 12, 24, 32, 64] {
            let a0 = vec![0x00u8; len];
            let b = vec![0x01u8; len];
            let mut a1 = a0.clone();
            let mut a2 = a0.clone();
            c(a1.as_mut_ptr(), b.as_ptr(), len);
            r(a2.as_mut_ptr(), b.as_ptr(), len);
            assert_eq!(a1, a2, "sub borrow edge len={}", len);
        }
    }
}

// ---------------------------------------------------------------------------
// sodium_memzero
// ---------------------------------------------------------------------------
#[test]
fn memzero_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(l, b"sodium_memzero", unsafe extern "C" fn(*mut u8, usize));
        let mut rng = Rng::new(0x77);
        for _ in 0..500 {
            let len = rng.range(128);
            let mut a = rng.vec(len);
            let mut b = a.clone();
            c(a.as_mut_ptr(), len);
            r(b.as_mut_ptr(), len);
            assert_eq!(a, b);
            assert!(a.iter().all(|&x| x == 0));
        }
        // len 0 no-op on a valid buffer.
        let mut a = vec![1u8; 4];
        let mut b = vec![1u8; 4];
        c(a.as_mut_ptr(), 0);
        r(b.as_mut_ptr(), 0);
        assert_eq!(a, b);
    }
}

// ---------------------------------------------------------------------------
// sodium_stackzero (no-op in this build config; just make sure both callable)
// ---------------------------------------------------------------------------
#[test]
fn stackzero_callable() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(l, b"sodium_stackzero", unsafe extern "C" fn(usize));
        for len in [0usize, 16, 64, 256] {
            c(len);
            r(len);
        }
    }
}

// ---------------------------------------------------------------------------
// sodium_bin2hex
// ---------------------------------------------------------------------------
#[test]
fn bin2hex_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(
            l,
            b"sodium_bin2hex",
            unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char
        );
        let mut rng = Rng::new(0x88);
        for _ in 0..2000 {
            let len = rng.range(64);
            let bin = rng.vec(len);
            let maxlen = len * 2 + 1; // exact minimum
            let mut co = vec![0u8; maxlen + 4];
            let mut ro = vec![0u8; maxlen + 4];
            let cp = c(co.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), len);
            let rp = r(ro.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), len);
            // both return the passed-in buffer pointer.
            assert_eq!(cp, co.as_mut_ptr() as *mut c_char);
            assert_eq!(rp, ro.as_mut_ptr() as *mut c_char);
            assert_eq!(co, ro, "bin2hex len={} bin={:?}", len, bin);
        }
    }
}

// ---------------------------------------------------------------------------
// sodium_hex2bin
// ---------------------------------------------------------------------------
fn cstr(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

#[allow(clippy::too_many_arguments)]
unsafe fn run_hex2bin(
    f: &libloading::Symbol<Hex2BinFn>,
    bin_maxlen: usize,
    hex: &[c_char],
    hex_len: usize,
    ignore: Option<&[c_char]>,
    with_bin_len: bool,
    with_hex_end: bool,
) -> (c_int, Vec<u8>, usize, isize) {
    let mut bin = vec![0u8; bin_maxlen];
    let mut bin_len: usize = 0xdead_beef;
    let mut hex_end: *const c_char = std::ptr::null();
    let ign_ptr = ignore.map(|s| s.as_ptr()).unwrap_or(std::ptr::null());
    let ret = f(
        bin.as_mut_ptr(),
        bin_maxlen,
        hex.as_ptr(),
        hex_len,
        ign_ptr,
        if with_bin_len { &mut bin_len } else { std::ptr::null_mut() },
        if with_hex_end { &mut hex_end } else { std::ptr::null_mut() },
    );
    // offset of hex_end from start (or -1 if not requested / null).
    let end_off = if with_hex_end && !hex_end.is_null() {
        hex_end as isize - hex.as_ptr() as isize
    } else {
        -1
    };
    (ret, bin, bin_len, end_off)
}

#[test]
fn hex2bin_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(l, b"sodium_hex2bin", Hex2BinFn);

        let ignore_colon = cstr(": ");
        let mut rng = Rng::new(0x99);

        // ---- Phase B: valid pure-hex strings, many random. ----
        for _ in 0..2000 {
            let nbytes = rng.range(48);
            let bin = rng.vec(nbytes);
            // build hex string
            let mut hs = String::new();
            for b in &bin {
                hs.push_str(&format!("{:02x}", b));
            }
            let hex = cstr(&hs);
            let hlen = hs.len();
            for &(bl, he) in &[(true, true), (false, false), (true, false), (false, true)] {
                let cc = run_hex2bin(&c, nbytes, &hex, hlen, None, bl, he);
                let rr = run_hex2bin(&r, nbytes, &hex, hlen, None, bl, he);
                assert_eq!(cc, rr, "hex2bin valid hs={} bl={} he={}", hs, bl, he);
            }
        }

        // ---- with ignore chars interspersed (colons/spaces). ----
        for _ in 0..2000 {
            let nbytes = rng.range(24);
            let bin = rng.vec(nbytes);
            let mut hs = String::new();
            for (i, b) in bin.iter().enumerate() {
                if i > 0 && rng.next_u64() & 1 == 0 {
                    hs.push(if rng.next_u64() & 1 == 0 { ':' } else { ' ' });
                }
                hs.push_str(&format!("{:02x}", b));
            }
            let hex = cstr(&hs);
            let hlen = hs.len();
            let cc = run_hex2bin(&c, nbytes, &hex, hlen, Some(&ignore_colon), true, true);
            let rr = run_hex2bin(&r, nbytes, &hex, hlen, Some(&ignore_colon), true, true);
            assert_eq!(cc, rr, "hex2bin ignore hs={:?}", hs);
        }

        // ---- uppercase / mixed case ----
        for hs in ["DEADBEEF", "AbCdEf01", "0A0b0C"] {
            let hex = cstr(hs);
            let cc = run_hex2bin(&c, 16, &hex, hs.len(), None, true, true);
            let rr = run_hex2bin(&r, 16, &hex, hs.len(), None, true, true);
            assert_eq!(cc, rr, "hex2bin case {}", hs);
        }

        // ---- Phase C: error / edge cases ----
        // Trailing non-hex char, hex_end requested -> ret 0, hex_end at that char.
        for hs in ["ab:cd", "abZZ", "ab ", "gg", "a", "abc", "", "12345"] {
            let hex = cstr(hs);
            // with hex_end (stops gracefully at non-hex)
            let cc = run_hex2bin(&c, 32, &hex, hs.len(), None, true, true);
            let rr = run_hex2bin(&r, 32, &hex, hs.len(), None, true, true);
            assert_eq!(cc, rr, "hex2bin trailing hs={:?} (hex_end)", hs);
            // without hex_end (any leftover -> -1)
            let cc = run_hex2bin(&c, 32, &hex, hs.len(), None, true, false);
            let rr = run_hex2bin(&r, 32, &hex, hs.len(), None, true, false);
            assert_eq!(cc, rr, "hex2bin trailing hs={:?} (no hex_end)", hs);
        }

        // ---- odd number of hex digits (dangling nibble) -> -1, EINVAL ----
        for hs in ["abc", "a", "abcde"] {
            let hex = cstr(hs);
            let cc = run_hex2bin(&c, 32, &hex, hs.len(), None, true, true);
            let rr = run_hex2bin(&r, 32, &hex, hs.len(), None, true, true);
            assert_eq!(cc, rr, "hex2bin odd hs={:?}", hs);
        }

        // ---- bin_maxlen too small -> -1, ERANGE, bin_pos reset to 0 ----
        for &maxlen in &[0usize, 1, 2, 3] {
            let hs = "aabbccdd";
            let hex = cstr(hs);
            let cc = run_hex2bin(&c, maxlen, &hex, hs.len(), None, true, true);
            let rr = run_hex2bin(&r, maxlen, &hex, hs.len(), None, true, true);
            assert_eq!(cc, rr, "hex2bin small buf maxlen={}", maxlen);
        }

        // ---- hex_len shorter than the string ----
        let hs = "aabbccdd";
        let hex = cstr(hs);
        for hlen in 0..=hs.len() {
            let cc = run_hex2bin(&c, 8, &hex, hlen, None, true, true);
            let rr = run_hex2bin(&r, 8, &hex, hlen, None, true, true);
            assert_eq!(cc, rr, "hex2bin hlen={}", hlen);
        }
    }
}

// ---------------------------------------------------------------------------
// sodium_base64_encoded_len
// ---------------------------------------------------------------------------
#[test]
fn base64_encoded_len_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(
            l,
            b"sodium_base64_encoded_len",
            unsafe extern "C" fn(usize, c_int) -> usize
        );
        for &v in &ALL_VARIANTS {
            for bin_len in 0usize..300 {
                assert_eq!(
                    c(bin_len, v),
                    r(bin_len, v),
                    "encoded_len bin_len={} variant={}",
                    bin_len,
                    v
                );
            }
            // a few large values
            for &bin_len in &[1000usize, 4096, 65535, 1_000_000] {
                assert_eq!(c(bin_len, v), r(bin_len, v), "encoded_len big {} v={}", bin_len, v);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// sodium_bin2base64 + sodium_base642bin round-trip and cross-check
// ---------------------------------------------------------------------------
#[test]
fn bin2base64_matches() {
    let l = libs();
    unsafe {
        let (enc_c, enc_r) = sympair!(
            l,
            b"sodium_bin2base64",
            unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char
        );
        let (len_c, _len_r) = sympair!(
            l,
            b"sodium_base64_encoded_len",
            unsafe extern "C" fn(usize, c_int) -> usize
        );
        let mut rng = Rng::new(0xAA);
        for &v in &ALL_VARIANTS {
            for _ in 0..1500 {
                let nbytes = rng.range(64);
                let bin = rng.vec(nbytes);
                let maxlen = len_c(nbytes, v);
                let mut co = vec![0u8; maxlen + 4];
                let mut ro = vec![0u8; maxlen + 4];
                let cp = enc_c(co.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), nbytes, v);
                let rp = enc_r(ro.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), nbytes, v);
                assert_eq!(cp, co.as_mut_ptr() as *mut c_char);
                assert_eq!(rp, ro.as_mut_ptr() as *mut c_char);
                assert_eq!(co, ro, "bin2base64 v={} nbytes={} bin={:?}", v, nbytes, bin);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn run_base642bin(
    f: &libloading::Symbol<Base642BinFn>,
    bin_maxlen: usize,
    b64: &[c_char],
    b64_len: usize,
    ignore: Option<&[c_char]>,
    with_bin_len: bool,
    with_b64_end: bool,
    variant: c_int,
) -> (c_int, Vec<u8>, usize, isize) {
    let mut bin = vec![0u8; bin_maxlen];
    let mut bin_len: usize = 0xdead_beef;
    let mut b64_end: *const c_char = std::ptr::null();
    let ign_ptr = ignore.map(|s| s.as_ptr()).unwrap_or(std::ptr::null());
    let ret = f(
        bin.as_mut_ptr(),
        bin_maxlen,
        b64.as_ptr(),
        b64_len,
        ign_ptr,
        if with_bin_len { &mut bin_len } else { std::ptr::null_mut() },
        if with_b64_end { &mut b64_end } else { std::ptr::null_mut() },
        variant,
    );
    let end_off = if with_b64_end && !b64_end.is_null() {
        b64_end as isize - b64.as_ptr() as isize
    } else {
        -1
    };
    (ret, bin, bin_len, end_off)
}

#[test]
fn base642bin_matches() {
    let l = libs();
    unsafe {
        let (dec_c, dec_r) = sympair!(l, b"sodium_base642bin", Base642BinFn);
        let (enc_c, _enc_r) = sympair!(
            l,
            b"sodium_bin2base64",
            unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char
        );
        let (len_c, _) = sympair!(
            l,
            b"sodium_base64_encoded_len",
            unsafe extern "C" fn(usize, c_int) -> usize
        );

        let ignore_ws = cstr(" \n\r\t");
        let mut rng = Rng::new(0xBB);

        // ---- Phase B: encode with C then decode with both, all variants. ----
        for &v in &ALL_VARIANTS {
            for _ in 0..1500 {
                let nbytes = rng.range(48);
                let bin = rng.vec(nbytes);
                let maxlen = len_c(nbytes, v);
                let mut b64buf = vec![0u8; maxlen + 1];
                enc_c(b64buf.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), nbytes, v);
                // convert to c_char slice up to (but not incl) the NUL, plus NUL.
                let s_len = maxlen - 1; // encoded_len includes trailing NUL
                let b64: Vec<c_char> = b64buf[..=s_len].iter().map(|&x| x as c_char).collect();

                for &(bl, be) in &[(true, true), (false, false), (true, false)] {
                    let cc = run_base642bin(&dec_c, nbytes + 4, &b64, s_len, None, bl, be, v);
                    let rr = run_base642bin(&dec_r, nbytes + 4, &b64, s_len, None, bl, be, v);
                    assert_eq!(cc, rr, "base642bin roundtrip v={} nbytes={}", v, nbytes);
                    // decoded bytes should equal original (sanity of the C side too).
                    assert_eq!(&cc.1[..nbytes], &bin[..], "roundtrip content v={}", v);
                }
            }
        }

        // ---- with ignore whitespace interspersed ----
        for &v in &ALL_VARIANTS {
            for _ in 0..1000 {
                let nbytes = rng.range(24);
                let bin = rng.vec(nbytes);
                let maxlen = len_c(nbytes, v);
                let mut b64buf = vec![0u8; maxlen + 1];
                enc_c(b64buf.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), nbytes, v);
                let s_len = maxlen - 1;
                // inject whitespace at random positions
                let mut chars: Vec<u8> = b64buf[..s_len].to_vec();
                let mut i = 0;
                while i < chars.len() {
                    if rng.next_u64() % 3 == 0 {
                        let ws = [b' ', b'\n', b'\r', b'\t'][(rng.next_u64() % 4) as usize];
                        chars.insert(i, ws);
                        i += 1;
                    }
                    i += 1;
                }
                let new_len = chars.len();
                chars.push(0);
                let b64: Vec<c_char> = chars.iter().map(|&x| x as c_char).collect();
                let cc = run_base642bin(&dec_c, nbytes + 4, &b64, new_len, Some(&ignore_ws), true, true, v);
                let rr = run_base642bin(&dec_r, nbytes + 4, &b64, new_len, Some(&ignore_ws), true, true, v);
                assert_eq!(cc, rr, "base642bin ignore v={} nbytes={}", v, nbytes);
            }
        }

        // ---- Phase C: error / edge cases (graceful -1) ----
        // Invalid characters mid-string, with & without hex_end/b64_end, with & without ignore.
        let bad_inputs: &[&str] = &[
            "####",       // all invalid
            "AB!CD",      // invalid mid
            "A",          // 6 bits only -> acc_len=6 >4 -> -1
            "AB",         // 12 bits -> acc_len=4, leftover bits maybe nonzero
            "ABC",        // 18 bits -> 2 bytes + acc_len 2
            "====",       // only padding
            "AB==",       // valid single byte w/ padding
            "AB=",        // truncated padding
            "QUJD",       // "ABC"
            "QUJDRA==",   // "ABCD"
            "/w==",       // 0xff original
            "_w==",       // urlsafe-ish
        ];
        for &v in &ALL_VARIANTS {
            for &s in bad_inputs {
                let b64 = cstr(s);
                for &(bl, be, ign) in &[
                    (true, true, false),
                    (true, false, false),
                    (false, true, false),
                    (true, true, true),
                ] {
                    let ig = if ign { Some(&ignore_ws[..]) } else { None };
                    let cc = run_base642bin(&dec_c, 32, &b64, s.len(), ig, bl, be, v);
                    let rr = run_base642bin(&dec_r, 32, &b64, s.len(), ig, bl, be, v);
                    assert_eq!(cc, rr, "base642bin bad v={} s={:?} bl={} be={} ign={}", v, s, bl, be, ign);
                }
            }
        }

        // ---- bin_maxlen too small -> ERANGE, -1 ----
        for &v in &ALL_VARIANTS {
            let s = "QUJDRA=="; // decodes to 4 bytes
            let b64 = cstr(s);
            for &maxlen in &[0usize, 1, 2, 3] {
                let cc = run_base642bin(&dec_c, maxlen, &b64, s.len(), None, true, true, v);
                let rr = run_base642bin(&dec_r, maxlen, &b64, s.len(), None, true, true, v);
                assert_eq!(cc, rr, "base642bin small buf v={} maxlen={}", v, maxlen);
            }
        }

        // ---- b64_len shorter than string ----
        let s = "QUJDRA==";
        let b64 = cstr(s);
        for &v in &ALL_VARIANTS {
            for blen in 0..=s.len() {
                let cc = run_base642bin(&dec_c, 16, &b64, blen, None, true, true, v);
                let rr = run_base642bin(&dec_r, 16, &b64, blen, None, true, true, v);
                assert_eq!(cc, rr, "base642bin blen={} v={}", blen, v);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// sodium_pad
// ---------------------------------------------------------------------------
#[test]
fn pad_matches() {
    let l = libs();
    unsafe {
        let (c, r) = sympair!(
            l,
            b"sodium_pad",
            unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> c_int
        );
        let mut rng = Rng::new(0xCC);
        for _ in 0..4000 {
            // blocksize both power-of-two and not.
            let blocksize = 1 + rng.range(32);
            let unpadded = rng.range(128);
            // pick a generous max_buflen; occasionally too small to force -1.
            let max_buflen = if rng.next_u64() % 4 == 0 {
                rng.range(unpadded + 4)
            } else {
                unpadded + blocksize + 8
            };
            let bufcap = unpadded + blocksize + 16;
            let seed = rng.vec(bufcap);

            let mut cbuf = seed.clone();
            let mut rbuf = seed.clone();
            let mut cpl: usize = 0xdead;
            let mut rpl: usize = 0xdead;
            let cc = c(&mut cpl, cbuf.as_mut_ptr(), unpadded, blocksize, max_buflen);
            let rr = r(&mut rpl, rbuf.as_mut_ptr(), unpadded, blocksize, max_buflen);
            assert_eq!(cc, rr, "pad ret bs={} un={} max={}", blocksize, unpadded, max_buflen);
            if cc == 0 {
                assert_eq!(cpl, rpl, "pad len bs={} un={} max={}", blocksize, unpadded, max_buflen);
                assert_eq!(cbuf, rbuf, "pad buf bs={} un={} max={}", blocksize, unpadded, max_buflen);
            }
        }
        // blocksize == 0 -> -1 (also test with null padded_buflen_p).
        {
            let mut buf = vec![0u8; 16];
            let cc = c(std::ptr::null_mut(), buf.as_mut_ptr(), 4, 0, 32);
            let rr = r(std::ptr::null_mut(), buf.as_mut_ptr(), 4, 0, 32);
            assert_eq!(cc, rr);
            assert_eq!(cc, -1);
        }
        // null padded_buflen_p on a success path.
        {
            let mut cbuf = vec![7u8; 32];
            let mut rbuf = vec![7u8; 32];
            let cc = c(std::ptr::null_mut(), cbuf.as_mut_ptr(), 5, 8, 32);
            let rr = r(std::ptr::null_mut(), rbuf.as_mut_ptr(), 5, 8, 32);
            assert_eq!(cc, rr);
            assert_eq!(cbuf, rbuf);
        }
    }
}

// ---------------------------------------------------------------------------
// sodium_unpad
// ---------------------------------------------------------------------------
#[test]
fn unpad_matches() {
    let l = libs();
    unsafe {
        let (pad_c, _pad_r) = sympair!(
            l,
            b"sodium_pad",
            unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> c_int
        );
        let (c, r) = sympair!(
            l,
            b"sodium_unpad",
            unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> c_int
        );
        let mut rng = Rng::new(0xDD);

        // ---- Phase B: pad then unpad -> valid, recovers length. ----
        for _ in 0..4000 {
            let blocksize = 1 + rng.range(32);
            let unpadded = rng.range(128);
            let bufcap = unpadded + blocksize + 16;
            let mut buf = rng.vec(bufcap);
            let mut padded_len: usize = 0;
            let pr = pad_c(&mut padded_len, buf.as_mut_ptr(), unpadded, blocksize, bufcap);
            if pr != 0 {
                continue;
            }
            let mut cul: usize = 0xdead;
            let mut rul: usize = 0xdead;
            let cc = c(&mut cul, buf.as_ptr(), padded_len, blocksize);
            let rr = r(&mut rul, buf.as_ptr(), padded_len, blocksize);
            assert_eq!(cc, rr, "unpad ret bs={} un={}", blocksize, unpadded);
            assert_eq!(cul, rul, "unpad len bs={} un={}", blocksize, unpadded);
            assert_eq!(cc, 0);
            assert_eq!(cul, unpadded);
        }

        // ---- Phase C: invalid padding & edge cases (graceful -1) ----
        // random buffers unpadded directly (mostly invalid padding).
        for _ in 0..4000 {
            let blocksize = 1 + rng.range(32);
            let padded = blocksize + rng.range(96);
            let buf = rng.vec(padded);
            let mut cul: usize = 0xdead;
            let mut rul: usize = 0xdead;
            let cc = c(&mut cul, buf.as_ptr(), padded, blocksize);
            let rr = r(&mut rul, buf.as_ptr(), padded, blocksize);
            assert_eq!(cc, rr, "unpad rand ret bs={} pad={} buf={:?}", blocksize, padded, buf);
            // *unpadded_buflen_p is written before the return in both; compare.
            assert_eq!(cul, rul, "unpad rand len bs={} pad={}", blocksize, padded);
        }

        // padded_buflen < blocksize -> -1.
        {
            let buf = vec![0x80u8; 8];
            let mut cul = 0usize;
            let mut rul = 0usize;
            let cc = c(&mut cul, buf.as_ptr(), 4, 8);
            let rr = r(&mut rul, buf.as_ptr(), 4, 8);
            assert_eq!(cc, rr);
            assert_eq!(cc, -1);
        }
        // blocksize == 0 -> -1.
        {
            let buf = vec![0x80u8; 8];
            let mut cul = 0usize;
            let mut rul = 0usize;
            let cc = c(&mut cul, buf.as_ptr(), 8, 0);
            let rr = r(&mut rul, buf.as_ptr(), 8, 0);
            assert_eq!(cc, rr);
            assert_eq!(cc, -1);
        }
        // Known-good explicit padding: block of size N with 0x80 then zeros.
        for &bs in &[8usize, 16, 17] {
            let mut buf = vec![0u8; bs];
            buf[0] = 0x80; // one full block of pure padding
            let mut cul = 0usize;
            let mut rul = 0usize;
            let cc = c(&mut cul, buf.as_ptr(), bs, bs);
            let rr = r(&mut rul, buf.as_ptr(), bs, bs);
            assert_eq!(cc, rr);
            assert_eq!(cul, rul);
            assert_eq!(cc, 0);
            assert_eq!(cul, 0);
        }
    }
}

// ---------------------------------------------------------------------------
// sodium_runtime_has_*
// ---------------------------------------------------------------------------
#[test]
fn runtime_features_match() {
    let l = libs();
    let names: &[&[u8]] = &[
        b"sodium_runtime_has_sse2",
        b"sodium_runtime_has_sse3",
        b"sodium_runtime_has_ssse3",
        b"sodium_runtime_has_sse41",
        b"sodium_runtime_has_avx",
        b"sodium_runtime_has_avx2",
        b"sodium_runtime_has_avx512f",
        b"sodium_runtime_has_neon",
        b"sodium_runtime_has_aesni",
        b"sodium_runtime_has_pclmul",
        b"sodium_runtime_has_rdrand",
        b"sodium_runtime_has_armcrypto",
    ];
    unsafe {
        for name in names {
            let (c, r) = sympair!(l, *name, unsafe extern "C" fn() -> c_int);
            let cv = c();
            let rv = r();
            assert_eq!(
                cv,
                rv,
                "runtime feature {} c={} rust={}",
                std::str::from_utf8(name).unwrap(),
                cv,
                rv
            );
        }
    }
}

// ---------------------------------------------------------------------------
// version functions
// ---------------------------------------------------------------------------
#[test]
fn version_functions_match() {
    let l = libs();
    unsafe {
        let (cs, rs) = sympair!(l, b"sodium_version_string", unsafe extern "C" fn() -> *const c_char);
        let c_str = CStr::from_ptr(cs());
        let r_str = CStr::from_ptr(rs());
        assert_eq!(c_str, r_str, "version string");

        let (cmaj, rmaj) = sympair!(l, b"sodium_library_version_major", unsafe extern "C" fn() -> c_int);
        assert_eq!(cmaj(), rmaj(), "version major");

        let (cmin, rmin) = sympair!(l, b"sodium_library_version_minor", unsafe extern "C" fn() -> c_int);
        assert_eq!(cmin(), rmin(), "version minor");

        let (cml, rml) = sympair!(l, b"sodium_library_minimal", unsafe extern "C" fn() -> c_int);
        assert_eq!(cml(), rml(), "library minimal");
    }
}
