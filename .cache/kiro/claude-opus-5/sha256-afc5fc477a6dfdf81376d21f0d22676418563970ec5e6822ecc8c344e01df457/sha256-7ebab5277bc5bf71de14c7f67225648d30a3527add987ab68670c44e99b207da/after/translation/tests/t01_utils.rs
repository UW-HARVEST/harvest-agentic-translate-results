//! Lowest layer: crypto_verify_*, sodium_* utility primitives, hex/base64
//! codecs, IP codecs, padding, and the runtime/version accessors.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_uchar, c_void};

// ---------------------------------------------------------------------------
// crypto_verify_N
// ---------------------------------------------------------------------------

fn verify_case(name: &str, n: usize) {
    cmp_size(&format!("{name}_bytes"));
    unsafe {
        let (c, r): (FnVerify, FnVerify) = pair(name);
        let mut rng = Rng::new(0x11 + n as u64);
        // equal buffers
        for _ in 0..8 {
            let a = rng.vec(n);
            assert_eq!(c(a.as_ptr(), a.as_ptr()), r(a.as_ptr(), a.as_ptr()), "{name} equal");
            let b = a.clone();
            assert_eq!(c(a.as_ptr(), b.as_ptr()), r(a.as_ptr(), b.as_ptr()), "{name} clone");
        }
        // differ in exactly one bit at every position
        for pos in 0..n {
            for bit in 0..8 {
                let a = rng.vec(n);
                let mut b = a.clone();
                b[pos] ^= 1 << bit;
                assert_eq!(
                    c(a.as_ptr(), b.as_ptr()),
                    r(a.as_ptr(), b.as_ptr()),
                    "{name} diff at byte {pos} bit {bit}"
                );
            }
        }
        // all-zero / all-ones edges
        let z = vec![0u8; n];
        let o = vec![0xffu8; n];
        for (x, y) in [(&z, &z), (&z, &o), (&o, &z), (&o, &o)] {
            assert_eq!(c(x.as_ptr(), y.as_ptr()), r(x.as_ptr(), y.as_ptr()), "{name} edge");
        }
    }
}

#[test]
fn crypto_verify_16_32_64() {
    verify_case("crypto_verify_16", 16);
    verify_case("crypto_verify_32", 32);
    verify_case("crypto_verify_64", 64);
}

// ---------------------------------------------------------------------------
// sodium_memcmp / compare / is_zero / increment / add / sub
// ---------------------------------------------------------------------------

type FnMemcmp = unsafe extern "C" fn(*const c_void, *const c_void, usize) -> c_int;
type FnCompare = unsafe extern "C" fn(*const c_uchar, *const c_uchar, usize) -> c_int;
type FnIsZero = unsafe extern "C" fn(*const c_uchar, usize) -> c_int;
type FnIncrement = unsafe extern "C" fn(*mut c_uchar, usize);
type FnAddSub = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, usize);

#[test]
fn sodium_memcmp_matches() {
    unsafe {
        let (c, r): (FnMemcmp, FnMemcmp) = pair("sodium_memcmp");
        let mut rng = Rng::new(1);
        for len in 0..40usize {
            let a = rng.vec(len);
            let b = a.clone();
            assert_eq!(
                c(a.as_ptr() as _, b.as_ptr() as _, len),
                r(a.as_ptr() as _, b.as_ptr() as _, len),
                "memcmp equal len {len}"
            );
            if len > 0 {
                for pos in 0..len {
                    let mut d = a.clone();
                    d[pos] ^= 0x80;
                    assert_eq!(
                        c(a.as_ptr() as _, d.as_ptr() as _, len),
                        r(a.as_ptr() as _, d.as_ptr() as _, len),
                        "memcmp diff len {len} pos {pos}"
                    );
                }
            }
        }
    }
}

#[test]
fn sodium_compare_matches() {
    unsafe {
        let (c, r): (FnCompare, FnCompare) = pair("sodium_compare");
        let mut rng = Rng::new(2);
        for len in 0..34usize {
            for _ in 0..20 {
                let a = rng.vec(len);
                let b = rng.vec(len);
                assert_eq!(
                    c(a.as_ptr(), b.as_ptr(), len),
                    r(a.as_ptr(), b.as_ptr(), len),
                    "compare random len {len}"
                );
            }
            let a = rng.vec(len);
            assert_eq!(
                c(a.as_ptr(), a.as_ptr(), len),
                r(a.as_ptr(), a.as_ptr(), len),
                "compare self len {len}"
            );
            // little-endian ordering: perturb each byte up and down
            for pos in 0..len {
                let mut lo = a.clone();
                let mut hi = a.clone();
                lo[pos] = lo[pos].wrapping_sub(1);
                hi[pos] = hi[pos].wrapping_add(1);
                assert_eq!(
                    c(a.as_ptr(), lo.as_ptr(), len),
                    r(a.as_ptr(), lo.as_ptr(), len),
                    "compare lo len {len} pos {pos}"
                );
                assert_eq!(
                    c(a.as_ptr(), hi.as_ptr(), len),
                    r(a.as_ptr(), hi.as_ptr(), len),
                    "compare hi len {len} pos {pos}"
                );
            }
        }
    }
}

#[test]
fn sodium_is_zero_matches() {
    unsafe {
        let (c, r): (FnIsZero, FnIsZero) = pair("sodium_is_zero");
        for len in 0..40usize {
            let z = vec![0u8; len];
            assert_eq!(c(z.as_ptr(), len), r(z.as_ptr(), len), "is_zero zeros {len}");
            for pos in 0..len {
                let mut v = z.clone();
                v[pos] = 1;
                assert_eq!(c(v.as_ptr(), len), r(v.as_ptr(), len), "is_zero {len} pos {pos}");
                v[pos] = 0xff;
                assert_eq!(c(v.as_ptr(), len), r(v.as_ptr(), len), "is_zero ff {len} pos {pos}");
            }
        }
    }
}

#[test]
fn sodium_increment_matches() {
    unsafe {
        let (c, r): (FnIncrement, FnIncrement) = pair("sodium_increment");
        for len in 0..20usize {
            // carry propagation edges
            let seeds: Vec<Vec<u8>> = vec![
                vec![0u8; len],
                vec![0xffu8; len],
                {
                    let mut v = vec![0xffu8; len];
                    if len > 0 {
                        v[len - 1] = 0;
                    }
                    v
                },
                {
                    let mut v = vec![0u8; len];
                    if len > 0 {
                        v[0] = 0xff;
                    }
                    v
                },
            ];
            for s in seeds {
                let mut a = s.clone();
                let mut b = s.clone();
                for _ in 0..5 {
                    c(a.as_mut_ptr(), len);
                    r(b.as_mut_ptr(), len);
                    assert_bytes_eq(&format!("sodium_increment len {len}"), &a, &b);
                }
            }
            let mut rng = Rng::new(3 + len as u64);
            let s = rng.vec(len);
            let mut a = s.clone();
            let mut b = s.clone();
            for _ in 0..300 {
                c(a.as_mut_ptr(), len);
                r(b.as_mut_ptr(), len);
                assert_bytes_eq(&format!("sodium_increment rand len {len}"), &a, &b);
            }
        }
    }
}

#[test]
fn sodium_add_sub_matches() {
    unsafe {
        let (cadd, radd): (FnAddSub, FnAddSub) = pair("sodium_add");
        let (csub, rsub): (FnAddSub, FnAddSub) = pair("sodium_sub");
        let mut rng = Rng::new(4);
        for len in 0..24usize {
            for _ in 0..30 {
                let x = rng.vec(len);
                let y = rng.vec(len);
                let mut a = x.clone();
                let mut b = x.clone();
                cadd(a.as_mut_ptr(), y.as_ptr(), len);
                radd(b.as_mut_ptr(), y.as_ptr(), len);
                assert_bytes_eq(&format!("sodium_add len {len}"), &a, &b);

                let mut a = x.clone();
                let mut b = x.clone();
                csub(a.as_mut_ptr(), y.as_ptr(), len);
                rsub(b.as_mut_ptr(), y.as_ptr(), len);
                assert_bytes_eq(&format!("sodium_sub len {len}"), &a, &b);
            }
            // all-ones + all-ones, zero - one: full carry / borrow chains
            let ones = vec![0xffu8; len];
            let zeros = vec![0u8; len];
            let mut one = vec![0u8; len];
            if len > 0 {
                one[0] = 1;
            }
            for (base, arg) in [
                (&ones, &ones),
                (&zeros, &one),
                (&ones, &one),
                (&zeros, &ones),
            ] {
                let mut a = base.clone();
                let mut b = base.clone();
                cadd(a.as_mut_ptr(), arg.as_ptr(), len);
                radd(b.as_mut_ptr(), arg.as_ptr(), len);
                assert_bytes_eq(&format!("sodium_add edge len {len}"), &a, &b);
                let mut a = base.clone();
                let mut b = base.clone();
                csub(a.as_mut_ptr(), arg.as_ptr(), len);
                rsub(b.as_mut_ptr(), arg.as_ptr(), len);
                assert_bytes_eq(&format!("sodium_sub edge len {len}"), &a, &b);
            }
        }
    }
}

#[test]
fn sodium_memzero_matches() {
    type F = unsafe extern "C" fn(*mut c_void, usize);
    unsafe {
        let (c, r): (F, F) = pair("sodium_memzero");
        let mut rng = Rng::new(5);
        for len in 0..40usize {
            let s = rng.vec(len.max(1));
            let mut a = s.clone();
            let mut b = s.clone();
            c(a.as_mut_ptr() as _, len);
            r(b.as_mut_ptr() as _, len);
            assert_bytes_eq(&format!("sodium_memzero len {len}"), &a, &b);
        }
    }
}

// ---------------------------------------------------------------------------
// hex codec
// ---------------------------------------------------------------------------

type FnBin2Hex =
    unsafe extern "C" fn(*mut c_char, usize, *const c_uchar, usize) -> *mut c_char;
type FnHex2Bin = unsafe extern "C" fn(
    *mut c_uchar,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
) -> c_int;

#[test]
fn sodium_bin2hex_matches() {
    unsafe {
        let (c, r): (FnBin2Hex, FnBin2Hex) = pair("sodium_bin2hex");
        let mut rng = Rng::new(6);
        for len in 0..40usize {
            let bin = rng.vec(len);
            let maxlen = len * 2 + 1;
            let mut ca = vec![0xAAu8; maxlen + 8];
            let mut ra = vec![0xAAu8; maxlen + 8];
            let cp = c(ca.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), len);
            let rp = r(ra.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), len);
            assert_eq!(
                cp as usize - ca.as_ptr() as usize,
                rp as usize - ra.as_ptr() as usize,
                "bin2hex return offset len {len}"
            );
            assert_bytes_eq(&format!("sodium_bin2hex len {len}"), &ca, &ra);
        }
    }
}

#[test]
fn sodium_hex2bin_matches() {
    unsafe {
        let (c, r): (FnHex2Bin, FnHex2Bin) = pair("sodium_hex2bin");
        let cases: Vec<(&str, Option<&str>, usize)> = vec![
            ("", None, 8),
            ("00", None, 8),
            ("deadbeef", None, 8),
            ("DEADBEEF", None, 8),
            ("DeAdBeEf", None, 8),
            ("dead beef", Some(" "), 8),
            ("de:ad:be:ef", Some(":"), 8),
            ("de:ad:be:ef", None, 8),
            ("dea", None, 8),
            ("deadbeefzz", None, 8),
            ("zz", None, 8),
            ("deadbeef", None, 2),   // bin_maxlen too small
            ("deadbeef", None, 4),   // exactly fits
            ("deadbeef", None, 0),
            ("  deadbeef  ", Some(" "), 8),
            ("g", None, 8),
            ("0g", None, 8),
            ("ffffffffffffffff", None, 8),
            (":::", Some(":"), 8),
            ("00112233445566778899aabbccddeeff", None, 16),
            ("00112233445566778899aabbccddeeff", None, 15),
        ];
        for (hexs, ignore, maxlen) in cases {
            let hexb = hexs.as_bytes();
            let ign_c: Option<std::ffi::CString> =
                ignore.map(|s| std::ffi::CString::new(s).unwrap());
            let ign_ptr = ign_c
                .as_ref()
                .map(|s| s.as_ptr())
                .unwrap_or(std::ptr::null());

            let mut cb = vec![0xAAu8; maxlen + 8];
            let mut rb = vec![0xAAu8; maxlen + 8];
            let mut cl: usize = 0xdead;
            let mut rl: usize = 0xdead;
            let mut ce: *const c_char = std::ptr::null();
            let mut re: *const c_char = std::ptr::null();

            let cr = c(
                cb.as_mut_ptr(),
                maxlen,
                hexb.as_ptr() as *const c_char,
                hexb.len(),
                ign_ptr,
                &mut cl,
                &mut ce,
            );
            let rr = r(
                rb.as_mut_ptr(),
                maxlen,
                hexb.as_ptr() as *const c_char,
                hexb.len(),
                ign_ptr,
                &mut rl,
                &mut re,
            );
            let tag = format!("hex2bin({hexs:?}, ignore={ignore:?}, maxlen={maxlen})");
            assert_eq!(cr, rr, "{tag} return");
            assert_eq!(cl, rl, "{tag} bin_len");
            let coff = if ce.is_null() {
                usize::MAX
            } else {
                ce as usize - hexb.as_ptr() as usize
            };
            let roff = if re.is_null() {
                usize::MAX
            } else {
                re as usize - hexb.as_ptr() as usize
            };
            assert_eq!(coff, roff, "{tag} hex_end offset");
            assert_bytes_eq(&tag, &cb, &rb);
        }
        // NULL bin_len / hex_end pointers
        let hexb = b"deadbeef";
        let mut cb = [0xAAu8; 8];
        let mut rb = [0xAAu8; 8];
        let cr = c(
            cb.as_mut_ptr(),
            8,
            hexb.as_ptr() as *const c_char,
            8,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        let rr = r(
            rb.as_mut_ptr(),
            8,
            hexb.as_ptr() as *const c_char,
            8,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert_eq!(cr, rr, "hex2bin null-out return");
        assert_bytes_eq("hex2bin null-out", &cb, &rb);
    }
}

#[test]
fn hex_roundtrip_random() {
    unsafe {
        let (cb2h, rb2h): (FnBin2Hex, FnBin2Hex) = pair("sodium_bin2hex");
        let (ch2b, rh2b): (FnHex2Bin, FnHex2Bin) = pair("sodium_hex2bin");
        let mut rng = Rng::new(7);
        for len in 0..32usize {
            let bin = rng.vec(len);
            let mut hexbuf = vec![0u8; len * 2 + 1];
            cb2h(hexbuf.as_mut_ptr() as *mut c_char, hexbuf.len(), bin.as_ptr(), len);
            let mut hexbuf2 = vec![0u8; len * 2 + 1];
            rb2h(hexbuf2.as_mut_ptr() as *mut c_char, hexbuf2.len(), bin.as_ptr(), len);
            assert_bytes_eq("bin2hex roundtrip stage", &hexbuf, &hexbuf2);

            let mut cb = vec![0xAAu8; len + 4];
            let mut rb = vec![0xAAu8; len + 4];
            let mut cl = 0usize;
            let mut rl = 0usize;
            let cr = ch2b(
                cb.as_mut_ptr(),
                len + 4,
                hexbuf.as_ptr() as *const c_char,
                len * 2,
                std::ptr::null(),
                &mut cl,
                std::ptr::null_mut(),
            );
            let rr = rh2b(
                rb.as_mut_ptr(),
                len + 4,
                hexbuf.as_ptr() as *const c_char,
                len * 2,
                std::ptr::null(),
                &mut rl,
                std::ptr::null_mut(),
            );
            assert_eq!((cr, cl), (rr, rl), "hex roundtrip len {len}");
            assert_bytes_eq("hex roundtrip", &cb, &rb);
            assert_eq!(&cb[..len], &bin[..], "hex roundtrip value len {len}");
        }
    }
}

// ---------------------------------------------------------------------------
// base64 codec
// ---------------------------------------------------------------------------

type FnB64Len = unsafe extern "C" fn(usize, c_int) -> usize;
type FnBin2B64 =
    unsafe extern "C" fn(*mut c_char, usize, *const c_uchar, usize, c_int) -> *mut c_char;
type FnB642Bin = unsafe extern "C" fn(
    *mut c_uchar,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
    c_int,
) -> c_int;

const VARIANTS: [c_int; 4] = [1, 3, 5, 7];

#[test]
fn sodium_base64_encoded_len_matches() {
    unsafe {
        let (c, r): (FnB64Len, FnB64Len) = pair("sodium_base64_encoded_len");
        for v in VARIANTS {
            for len in 0..200usize {
                assert_eq!(c(len, v), r(len, v), "base64_encoded_len({len}, {v})");
            }
            for len in [1000usize, 100_000, 1 << 20] {
                assert_eq!(c(len, v), r(len, v), "base64_encoded_len({len}, {v})");
            }
        }
    }
}

#[test]
fn sodium_bin2base64_matches() {
    unsafe {
        let (clen, _): (FnB64Len, FnB64Len) = pair("sodium_base64_encoded_len");
        let (c, r): (FnBin2B64, FnBin2B64) = pair("sodium_bin2base64");
        let mut rng = Rng::new(8);
        for v in VARIANTS {
            for len in 0..40usize {
                let bin = rng.vec(len);
                let maxlen = clen(len, v);
                let mut ca = vec![0xAAu8; maxlen + 8];
                let mut ra = vec![0xAAu8; maxlen + 8];
                let cp = c(ca.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), len, v);
                let rp = r(ra.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), len, v);
                assert_eq!(
                    cp as usize - ca.as_ptr() as usize,
                    rp as usize - ra.as_ptr() as usize,
                    "bin2base64 ret variant {v} len {len}"
                );
                assert_bytes_eq(&format!("bin2base64 variant {v} len {len}"), &ca, &ra);
            }
            // all byte values, to cover every alphabet slot
            let all: Vec<u8> = (0..=255u8).collect();
            let maxlen = clen(all.len(), v);
            let mut ca = vec![0xAAu8; maxlen + 8];
            let mut ra = vec![0xAAu8; maxlen + 8];
            c(ca.as_mut_ptr() as *mut c_char, maxlen, all.as_ptr(), all.len(), v);
            r(ra.as_mut_ptr() as *mut c_char, maxlen, all.as_ptr(), all.len(), v);
            assert_bytes_eq(&format!("bin2base64 all-bytes variant {v}"), &ca, &ra);
        }
    }
}

#[test]
fn sodium_base642bin_matches() {
    unsafe {
        let (c, r): (FnB642Bin, FnB642Bin) = pair("sodium_base642bin");
        let cases: Vec<(&str, Option<&str>)> = vec![
            ("", None),
            ("AA==", None),
            ("AA", None),
            ("AAA=", None),
            ("AAA", None),
            ("AAAA", None),
            ("/+8=", None),
            ("_-8=", None),
            ("/+8", None),
            ("_-8", None),
            ("AA=A", None),
            ("A", None),
            ("A===", None),
            ("====", None),
            ("=", None),
            ("AAAA====", None),
            ("QUJD", None),
            ("QUJDRA==", None),
            ("QU JD", Some(" ")),
            ("QU\nJD", Some("\n")),
            ("QUJD\n", Some("\n")),
            ("QUJD\n", None),
            ("QUJD=", None),
            ("AB==", None),
            ("AQ==", None),
            ("Ag==", None),
            ("A/==", None),
            ("ABC=", None),
            ("ABD=", None),
            ("ABE=", None),
            ("!!!!", None),
            ("QUJD!", Some("!")),
            ("QUJD!", None),
            ("//////8=", None),
            ("________", None),
            ("aGVsbG8gd29ybGQ=", None),
            ("aGVsbG8gd29ybGQ", None),
        ];
        for v in VARIANTS {
            for (s, ignore) in &cases {
                let b = s.as_bytes();
                let ign_c: Option<std::ffi::CString> =
                    ignore.map(|x| std::ffi::CString::new(x).unwrap());
                let ign_ptr = ign_c.as_ref().map(|x| x.as_ptr()).unwrap_or(std::ptr::null());
                for maxlen in [0usize, 1, 2, 3, 4, 32] {
                    let mut cb = vec![0xAAu8; maxlen + 8];
                    let mut rb = vec![0xAAu8; maxlen + 8];
                    let mut cl = 0xdeadusize;
                    let mut rl = 0xdeadusize;
                    let mut ce: *const c_char = std::ptr::null();
                    let mut re: *const c_char = std::ptr::null();
                    let cr = c(
                        cb.as_mut_ptr(),
                        maxlen,
                        b.as_ptr() as *const c_char,
                        b.len(),
                        ign_ptr,
                        &mut cl,
                        &mut ce,
                        v,
                    );
                    let rr = r(
                        rb.as_mut_ptr(),
                        maxlen,
                        b.as_ptr() as *const c_char,
                        b.len(),
                        ign_ptr,
                        &mut rl,
                        &mut re,
                        v,
                    );
                    let tag =
                        format!("base642bin({s:?}, ign={ignore:?}, maxlen={maxlen}, variant={v})");
                    assert_eq!(cr, rr, "{tag} return");
                    assert_eq!(cl, rl, "{tag} bin_len");
                    let coff = if ce.is_null() {
                        usize::MAX
                    } else {
                        ce as usize - b.as_ptr() as usize
                    };
                    let roff = if re.is_null() {
                        usize::MAX
                    } else {
                        re as usize - b.as_ptr() as usize
                    };
                    assert_eq!(coff, roff, "{tag} b64_end");
                    assert_bytes_eq(&tag, &cb, &rb);
                }
            }
        }
    }
}

#[test]
fn base64_roundtrip_random() {
    unsafe {
        let (clen, _): (FnB64Len, FnB64Len) = pair("sodium_base64_encoded_len");
        let (cenc, renc): (FnBin2B64, FnBin2B64) = pair("sodium_bin2base64");
        let (cdec, rdec): (FnB642Bin, FnB642Bin) = pair("sodium_base642bin");
        let mut rng = Rng::new(9);
        for v in VARIANTS {
            for len in 0..48usize {
                let bin = rng.vec(len);
                let maxlen = clen(len, v);
                let mut enc = vec![0u8; maxlen + 1];
                cenc(enc.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), len, v);
                let mut enc2 = vec![0u8; maxlen + 1];
                renc(enc2.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), len, v);
                assert_bytes_eq("b64 roundtrip encode", &enc, &enc2);

                let slen = enc.iter().position(|&x| x == 0).unwrap();
                let mut cb = vec![0xAAu8; len + 8];
                let mut rb = vec![0xAAu8; len + 8];
                let mut cl = 0usize;
                let mut rl = 0usize;
                let cr = cdec(
                    cb.as_mut_ptr(),
                    len + 8,
                    enc.as_ptr() as *const c_char,
                    slen,
                    std::ptr::null(),
                    &mut cl,
                    std::ptr::null_mut(),
                    v,
                );
                let rr = rdec(
                    rb.as_mut_ptr(),
                    len + 8,
                    enc.as_ptr() as *const c_char,
                    slen,
                    std::ptr::null(),
                    &mut rl,
                    std::ptr::null_mut(),
                    v,
                );
                assert_eq!((cr, cl), (rr, rl), "b64 roundtrip decode variant {v} len {len}");
                assert_bytes_eq("b64 roundtrip decode", &cb, &rb);
                assert_eq!(cr, 0, "b64 roundtrip should decode");
                assert_eq!(&cb[..len], &bin[..], "b64 roundtrip value variant {v} len {len}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// IP codecs
// ---------------------------------------------------------------------------

type FnIp2Bin = unsafe extern "C" fn(*mut c_uchar, *const c_char, usize) -> c_int;
type FnBin2Ip = unsafe extern "C" fn(*mut c_char, usize, *const c_uchar) -> *mut c_char;

#[test]
fn sodium_ip2bin_matches() {
    unsafe {
        let (c, r): (FnIp2Bin, FnIp2Bin) = pair("sodium_ip2bin");
        let cases = [
            "0.0.0.0",
            "127.0.0.1",
            "255.255.255.255",
            "192.168.1.1",
            "1.2.3.4",
            "01.2.3.4",
            "1.2.3",
            "1.2.3.4.5",
            "256.1.1.1",
            "1.1.1.256",
            "",
            ".",
            "...",
            "1.2.3.",
            "::",
            "::1",
            "2001:db8::1",
            "2001:0db8:0000:0000:0000:0000:0000:0001",
            "fe80::1%eth0",
            "::ffff:1.2.3.4",
            "::ffff:192.168.0.1",
            "1:2:3:4:5:6:7:8",
            "1:2:3:4:5:6:7:8:9",
            "1:2:3:4:5:6:7",
            "1::2::3",
            "abcd:ef01:2345:6789:abcd:ef01:2345:6789",
            "ABCD:EF01::",
            "12345::",
            "g::1",
            "0:0:0:0:0:0:0:0",
            "::0.0.0.0",
            "1.2.3.4 ",
            " 1.2.3.4",
            "1.2.3.4\0extra",
            "999.999.999.999",
            "0000000001.2.3.4",
            "1:2:3:4:5:6:1.2.3.4",
            "::1:2:3:4:5:6:7:8",
            "fe80::",
            ":",
            ":::",
            "0:0:0:0:0:ffff:127.0.0.1",
        ];
        for s in cases {
            let b = s.as_bytes();
            for ip_len in [b.len(), b.len() + 1] {
                let mut cb = [0xAAu8; 24];
                let mut rb = [0xAAu8; 24];
                // pass a NUL-terminated buffer so reads inside the string are safe
                let mut owned = b.to_vec();
                owned.push(0);
                owned.push(0);
                let cr = c(cb.as_mut_ptr(), owned.as_ptr() as *const c_char, ip_len);
                let rr = r(rb.as_mut_ptr(), owned.as_ptr() as *const c_char, ip_len);
                let tag = format!("ip2bin({s:?}, len={ip_len})");
                assert_eq!(cr, rr, "{tag} return");
                assert_bytes_eq(&tag, &cb, &rb);
            }
        }
    }
}

#[test]
fn sodium_bin2ip_matches() {
    unsafe {
        let (c, r): (FnBin2Ip, FnBin2Ip) = pair("sodium_bin2ip");
        let mut cases: Vec<[u8; 16]> = vec![
            [0; 16],
            [0xff; 16],
            {
                let mut v = [0u8; 16];
                v[15] = 1;
                v
            },
            {
                // IPv4-mapped ::ffff:1.2.3.4
                let mut v = [0u8; 16];
                v[10] = 0xff;
                v[11] = 0xff;
                v[12] = 1;
                v[13] = 2;
                v[14] = 3;
                v[15] = 4;
                v
            },
            {
                let mut v = [0u8; 16];
                v[0] = 0x20;
                v[1] = 0x01;
                v[2] = 0x0d;
                v[3] = 0xb8;
                v[15] = 1;
                v
            },
        ];
        let mut rng = Rng::new(10);
        for _ in 0..64 {
            let mut v = [0u8; 16];
            rng.fill(&mut v);
            cases.push(v);
        }
        // sparse patterns with runs of zeros (exercise :: compression)
        for zstart in 0..8usize {
            for zlen in 1..=(8 - zstart) {
                let mut v = [0u8; 16];
                for i in 0..8 {
                    if i < zstart || i >= zstart + zlen {
                        v[i * 2] = 0x12;
                        v[i * 2 + 1] = 0x34;
                    }
                }
                cases.push(v);
            }
        }
        for bin in &cases {
            for maxlen in [0usize, 1, 4, 8, 16, 40, 46, 64] {
                let mut ca = vec![0xAAu8; maxlen + 8];
                let mut ra = vec![0xAAu8; maxlen + 8];
                let cp = c(ca.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr());
                let rp = r(ra.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr());
                let tag = format!("bin2ip({}, maxlen={maxlen})", hex(bin));
                assert_eq!(cp.is_null(), rp.is_null(), "{tag} null-ness");
                if !cp.is_null() {
                    assert_eq!(
                        cp as usize - ca.as_ptr() as usize,
                        rp as usize - ra.as_ptr() as usize,
                        "{tag} ret offset"
                    );
                }
                assert_bytes_eq(&tag, &ca, &ra);
            }
        }
    }
}

#[test]
fn ip_roundtrip() {
    unsafe {
        let (cb2i, _): (FnBin2Ip, FnBin2Ip) = pair("sodium_bin2ip");
        let (ci2b, ri2b): (FnIp2Bin, FnIp2Bin) = pair("sodium_ip2bin");
        let mut rng = Rng::new(11);
        for _ in 0..200 {
            let mut bin = [0u8; 16];
            rng.fill(&mut bin);
            // zero out a random run to produce :: forms
            let s = (rng.byte() % 8) as usize;
            let l = (rng.byte() % 8) as usize;
            for i in s..(s + l).min(8) {
                bin[i * 2] = 0;
                bin[i * 2 + 1] = 0;
            }
            let mut txt = [0u8; 64];
            let p = cb2i(txt.as_mut_ptr() as *mut c_char, txt.len(), bin.as_ptr());
            if p.is_null() {
                continue;
            }
            let slen = txt.iter().position(|&x| x == 0).unwrap();
            let mut cb = [0xAAu8; 20];
            let mut rb = [0xAAu8; 20];
            let cr = ci2b(cb.as_mut_ptr(), txt.as_ptr() as *const c_char, slen);
            let rr = ri2b(rb.as_mut_ptr(), txt.as_ptr() as *const c_char, slen);
            let tag = format!("ip roundtrip {:?}", String::from_utf8_lossy(&txt[..slen]));
            assert_eq!(cr, rr, "{tag} return");
            assert_bytes_eq(&tag, &cb, &rb);
        }
    }
}

// ---------------------------------------------------------------------------
// padding
// ---------------------------------------------------------------------------

type FnPad =
    unsafe extern "C" fn(*mut usize, *mut c_uchar, usize, usize, usize) -> c_int;
type FnUnpad = unsafe extern "C" fn(*mut usize, *const c_uchar, usize, usize) -> c_int;

#[test]
fn sodium_pad_matches() {
    unsafe {
        let (c, r): (FnPad, FnPad) = pair("sodium_pad");
        let mut rng = Rng::new(12);
        for blocksize in [0usize, 1, 2, 3, 8, 16, 17, 64] {
            for unpadded in 0..40usize {
                for max in [0usize, unpadded, unpadded + 1, unpadded + 16, 128] {
                    let cap = max.max(unpadded) + 80;
                    let src = rng.vec(cap);
                    let mut ca = src.clone();
                    let mut ra = src.clone();
                    let mut cl = 0xdeadusize;
                    let mut rl = 0xdeadusize;
                    let cr = c(&mut cl, ca.as_mut_ptr(), unpadded, blocksize, max);
                    let rr = r(&mut rl, ra.as_mut_ptr(), unpadded, blocksize, max);
                    let tag =
                        format!("sodium_pad(unpadded={unpadded}, bs={blocksize}, max={max})");
                    assert_eq!(cr, rr, "{tag} return");
                    assert_eq!(cl, rl, "{tag} padded_buflen");
                    assert_bytes_eq(&tag, &ca, &ra);
                }
            }
        }
        // NULL padded_buflen_p
        let mut a = [0u8; 64];
        let mut b = [0u8; 64];
        let cr = c(std::ptr::null_mut(), a.as_mut_ptr(), 5, 16, 64);
        let rr = r(std::ptr::null_mut(), b.as_mut_ptr(), 5, 16, 64);
        assert_eq!(cr, rr, "sodium_pad null out return");
        assert_bytes_eq("sodium_pad null out", &a, &b);
    }
}

#[test]
fn sodium_unpad_matches() {
    unsafe {
        let (c, r): (FnUnpad, FnUnpad) = pair("sodium_unpad");
        let mut rng = Rng::new(13);
        for blocksize in [0usize, 1, 2, 3, 8, 16, 17, 64] {
            for padded in 0..48usize {
                for _ in 0..3 {
                    let buf = rng.vec(padded.max(1));
                    let mut cl = 0xdeadusize;
                    let mut rl = 0xdeadusize;
                    let cr = c(&mut cl, buf.as_ptr(), padded, blocksize);
                    let rr = r(&mut rl, buf.as_ptr(), padded, blocksize);
                    let tag = format!("sodium_unpad(padded={padded}, bs={blocksize})");
                    assert_eq!(cr, rr, "{tag} return");
                    assert_eq!(cl, rl, "{tag} unpadded_buflen");
                }
                // properly-terminated padding
                if padded > 0 {
                    let mut buf = vec![0u8; padded];
                    buf[padded - 1] = 0x80;
                    let mut cl = 0usize;
                    let mut rl = 0usize;
                    let cr = c(&mut cl, buf.as_ptr(), padded, blocksize);
                    let rr = r(&mut rl, buf.as_ptr(), padded, blocksize);
                    assert_eq!((cr, cl), (rr, rl), "unpad 0x80-last padded={padded} bs={blocksize}");
                }
            }
        }
    }
}

#[test]
fn pad_unpad_roundtrip() {
    unsafe {
        let (cpad, rpad): (FnPad, FnPad) = pair("sodium_pad");
        let (cunpad, runpad): (FnUnpad, FnUnpad) = pair("sodium_unpad");
        let mut rng = Rng::new(14);
        for blocksize in [1usize, 2, 8, 16, 17, 64] {
            for unpadded in 0..40usize {
                let mut buf = rng.vec(unpadded + blocksize + 8);
                let mut rbuf = buf.clone();
                let mut cl = 0usize;
                let mut rl = 0usize;
                let cr = cpad(&mut cl, buf.as_mut_ptr(), unpadded, blocksize, buf.len());
                let rr = rpad(&mut rl, rbuf.as_mut_ptr(), unpadded, blocksize, rbuf.len());
                assert_eq!((cr, cl), (rr, rl), "pad rt bs={blocksize} n={unpadded}");
                assert_bytes_eq("pad rt buf", &buf, &rbuf);
                if cr == 0 {
                    let mut cu = 0usize;
                    let mut ru = 0usize;
                    let c2 = cunpad(&mut cu, buf.as_ptr(), cl, blocksize);
                    let r2 = runpad(&mut ru, rbuf.as_ptr(), rl, blocksize);
                    assert_eq!((c2, cu), (r2, ru), "unpad rt bs={blocksize} n={unpadded}");
                    assert_eq!(cu, unpadded, "unpad rt value bs={blocksize} n={unpadded}");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// runtime feature detection / version / misc
// ---------------------------------------------------------------------------

#[test]
fn sodium_runtime_flags_match() {
    for f in [
        "sodium_runtime_has_neon",
        "sodium_runtime_has_armcrypto",
        "sodium_runtime_has_sse2",
        "sodium_runtime_has_sse3",
        "sodium_runtime_has_ssse3",
        "sodium_runtime_has_sse41",
        "sodium_runtime_has_avx",
        "sodium_runtime_has_avx2",
        "sodium_runtime_has_avx512f",
        "sodium_runtime_has_pclmul",
        "sodium_runtime_has_aesni",
        "sodium_runtime_has_rdrand",
    ] {
        cmp_int(f);
    }
}

#[test]
fn version_accessors_match() {
    cmp_cstr("sodium_version_string");
    cmp_int("sodium_library_version_major");
    cmp_int("sodium_library_version_minor");
    cmp_int("sodium_library_minimal");
}

#[test]
fn randombytes_deterministic_matches() {
    // randombytes_buf_deterministic must be identical; it does not use the
    // installed implementation.
    type F = unsafe extern "C" fn(*mut c_void, usize, *const c_uchar);
    unsafe {
        let (c, r): (F, F) = pair("randombytes_buf_deterministic");
        let mut rng = Rng::new(15);
        for size in [0usize, 1, 31, 32, 33, 63, 64, 65, 100, 256, 1000] {
            let seed = rng.vec(32);
            let mut ca = vec![0xAAu8; size + 8];
            let mut ra = vec![0xAAu8; size + 8];
            c(ca.as_mut_ptr() as _, size, seed.as_ptr());
            r(ra.as_mut_ptr() as _, size, seed.as_ptr());
            assert_bytes_eq(&format!("randombytes_buf_deterministic size {size}"), &ca, &ra);
        }
        cmp_size("randombytes_seedbytes");
    }
}

#[test]
fn randombytes_uniform_matches() {
    // uniform() is NULL in our installed impl, so each library falls back to
    // its own rejection-sampling code driven by the shared deterministic
    // random(). Reset the stream before each call so both see the same input.
    type F = unsafe extern "C" fn(u32) -> u32;
    unsafe {
        let (c, r): (F, F) = pair("randombytes_uniform");
        for ub in [
            0u32, 1, 2, 3, 5, 7, 10, 16, 17, 100, 255, 256, 1000, 0x7fff_ffff, 0x8000_0000,
            0xffff_ffff, 0xffff_fffe,
        ] {
            det_reset();
            let cv = c(ub);
            det_reset();
            let rv = r(ub);
            assert_eq!(cv, rv, "randombytes_uniform({ub})");
        }
    }
}

#[test]
fn randombytes_random_and_buf_match() {
    type FRand = unsafe extern "C" fn() -> u32;
    type FBuf = unsafe extern "C" fn(*mut c_void, usize);
    type FNacl = unsafe extern "C" fn(*mut c_uchar, u64);
    unsafe {
        let (cr, rr): (FRand, FRand) = pair("randombytes_random");
        for _ in 0..16 {
            det_reset();
            let a = cr();
            det_reset();
            let b = rr();
            assert_eq!(a, b, "randombytes_random");
        }
        let (cb, rb): (FBuf, FBuf) = pair("randombytes_buf");
        for size in [0usize, 1, 7, 32, 100] {
            let mut x = vec![0u8; size + 4];
            let mut y = vec![0u8; size + 4];
            det_reset();
            cb(x.as_mut_ptr() as _, size);
            det_reset();
            rb(y.as_mut_ptr() as _, size);
            assert_bytes_eq(&format!("randombytes_buf {size}"), &x, &y);
        }
        let (cn, rn): (FNacl, FNacl) = pair("randombytes");
        for size in [0u64, 1, 33, 128] {
            let mut x = vec![0u8; size as usize + 4];
            let mut y = vec![0u8; size as usize + 4];
            det_reset();
            cn(x.as_mut_ptr(), size);
            det_reset();
            rn(y.as_mut_ptr(), size);
            assert_bytes_eq(&format!("randombytes {size}"), &x, &y);
        }
        cmp_cstr("randombytes_implementation_name");
    }
}
