//! Phase B — `sodium/` + `randombytes/` valid-input configuration rows
//! (`CONFIGS.md` section `## G1`, rows `G1-001` … `G1-174`).
//!
//! Everything is driven through `dlsym` on both `.so`s. Randomised inputs with
//! fixed seeds; many values per row.
//!
//! Rows whose observable behaviour depends on **process-global RNG state**
//! (`randombytes_set_implementation`, the first `sodium_init()`, the default
//! `sysrandom` / `internal` implementations) are executed in a **child
//! process** via `run_child`, because `cargo test` runs the `#[test]`s of one
//! binary in parallel threads and those rows would otherwise disturb each
//! other and the harness RNG.

mod common;
use common::*;

use std::ffi::{c_char, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// signatures (from include/sodium/{utils,core,randombytes,runtime,version}.h)
// ---------------------------------------------------------------------------

type Bin2Hex = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;
type Hex2Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
) -> i32;
type EncLen = unsafe extern "C" fn(usize, i32) -> usize;
type Bin2B64 = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, i32) -> *mut c_char;
type B642Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
    i32,
) -> i32;
type Ip2Bin = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> i32;
type Bin2Ip = unsafe extern "C" fn(*mut c_char, usize, *const u8) -> *mut c_char;
type Cmp3 = unsafe extern "C" fn(*const u8, *const u8, usize) -> i32;
type IsZero = unsafe extern "C" fn(*const u8, usize) -> i32;
type Incr = unsafe extern "C" fn(*mut u8, usize);
type AddSub = unsafe extern "C" fn(*mut u8, *const u8, usize);
type Pad = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> i32;
type Unpad = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> i32;
type Memzero = unsafe extern "C" fn(*mut c_void, usize);
type Stackzero = unsafe extern "C" fn(usize);
type MallocFn = unsafe extern "C" fn(usize) -> *mut c_void;
type AllocArray = unsafe extern "C" fn(usize, usize) -> *mut c_void;
type FreeFn = unsafe extern "C" fn(*mut c_void);
type Mlock = unsafe extern "C" fn(*mut c_void, usize) -> i32;
type Mprotect = unsafe extern "C" fn(*mut c_void) -> i32;
type IntFn = unsafe extern "C" fn() -> i32;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type SizeFn = unsafe extern "C" fn() -> usize;
type RbBuf = unsafe extern "C" fn(*mut c_void, usize);
type RbDet = unsafe extern "C" fn(*mut c_void, usize, *const u8);
type RbRandom = unsafe extern "C" fn() -> u32;
type RbUniform = unsafe extern "C" fn(u32) -> u32;
type RbVoid = unsafe extern "C" fn();
type RbNacl = unsafe extern "C" fn(*mut u8, u64);
type SetImplFn = unsafe extern "C" fn(*const RandombytesImpl) -> i32;
type SetMisuseFn = unsafe extern "C" fn(Option<unsafe extern "C" fn()>) -> i32;

/// The four accepted base64 variants (`(v & ~6) == 1`).
const VARIANTS: &[i32] = &[1, 3, 5, 7];

// ---------------------------------------------------------------------------
// errno helpers
//
// There is no portable way to *write* `errno` from safe Rust, so a sentinel is
// installed by calling `sodium_mlock`, which in this build unconditionally does
// `errno = ENOSYS; return -1` (ERRORS row G1-085). ENOSYS (38) collides with
// none of the values the codecs set (ERANGE 34 / EINVAL 22 / ENOMEM 12).
// ---------------------------------------------------------------------------

const SENTINEL: i32 = 38; // ENOSYS

fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

fn set_sentinel() {
    let f = sym::<Mlock>(c_lib(), "sodium_mlock");
    let mut b = [0u8; 1];
    let rc = unsafe { f(b.as_mut_ptr() as *mut c_void, 1) };
    assert_eq!(rc, -1, "sodium_mlock must fail in this build");
    assert_eq!(errno(), SENTINEL, "errno sentinel could not be installed");
}

fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<NULL>".into();
    }
    let mut v = Vec::new();
    let mut i = 0isize;
    loop {
        let b = unsafe { *p.offset(i) } as u8;
        if b == 0 {
            break;
        }
        v.push(b);
        i += 1;
    }
    String::from_utf8_lossy(&v).into_owned()
}

// ===========================================================================
// sodium_bin2hex
// ===========================================================================

/// Runs `sodium_bin2hex` on both libraries and compares the returned pointer
/// and the **entire** output buffer (so untouched regions are part of the
/// comparison).
fn cmp_bin2hex(what: &str, bin: &[u8], hex_maxlen: usize, bufsz: usize) -> Vec<u8> {
    let (c, r) = pair::<Bin2Hex>("sodium_bin2hex");
    assert!(bufsz >= hex_maxlen);
    let mut a = canary(bufsz);
    let mut b = canary(bufsz);
    let pa = unsafe { c(a.as_mut_ptr() as *mut c_char, hex_maxlen, bin.as_ptr(), bin.len()) };
    let pb = unsafe { r(b.as_mut_ptr() as *mut c_char, hex_maxlen, bin.as_ptr(), bin.len()) };
    assert_eq!(pa as usize, a.as_ptr() as usize, "{what}: C must return `hex`");
    assert_eq!(pb as usize, b.as_ptr() as usize, "{what}: Rust must return `hex`");
    eq_bytes(&format!("sodium_bin2hex({what})"), &a, &b);
    a
}

/// CONFIGS G1-001, G1-002, G1-003, G1-004, G1-005, G1-006.
#[test]
fn bin2hex_valid() {
    setup();
    let mut rng = Rng::new(0x1000);

    // G1-001: bin_len = 0, hex_maxlen = 1 -> only the terminator.
    let out = cmp_bin2hex("len=0,max=1", &[], 1, 8);
    assert_eq!(out[0], 0);
    assert_eq!(&out[1..], &canary(8)[1..], "nothing past the NUL may be written");

    // G1-002/003/004: every single-byte value, both nibble branches, lowercase.
    for v in 0u16..=255 {
        let b = [v as u8];
        let out = cmp_bin2hex(&format!("byte={v:#04x}"), &b, 3, 8);
        let want = format!("{:02x}", v as u8);
        assert_eq!(&out[..2], want.as_bytes(), "sodium_bin2hex must be lowercase");
        assert_eq!(out[2], 0);
        assert_eq!(&out[3..], &canary(8)[3..]);
    }

    // G1-005: exact fit vs oversized buffer; oversized leaves the tail UNTOUCHED
    // (contrast with sodium_bin2base64, which zero-fills).
    for _ in 0..32 {
        let bin = rng.bytes(2);
        let exact = cmp_bin2hex("len=2,exact", &bin, 5, 8);
        assert_eq!(exact[4], 0);
        assert_eq!(&exact[5..], &canary(8)[5..]);
        let big = cmp_bin2hex("len=2,max=64", &bin, 64, 64);
        assert_eq!(big[4], 0);
        assert_eq!(
            &big[5..],
            &canary(64)[5..],
            "sodium_bin2hex must not touch bytes past bin_len*2+1"
        );
    }

    // G1-006: typical and large sizes, many random inputs.
    for &n in &[1usize, 2, 3, 7, 16, 31, 32, 33, 64, 100, 255, 256, 1024] {
        for _ in 0..6 {
            let bin = rng.bytes(n);
            let out = cmp_bin2hex(&format!("len={n}"), &bin, n * 2 + 1, n * 2 + 9);
            assert_eq!(out[n * 2], 0);
            assert_eq!(String::from_utf8_lossy(&out[..n * 2]), hex(&bin));
        }
        // all-zero / all-ff edges
        for fill in [0u8, 0xff] {
            let bin = vec![fill; n];
            cmp_bin2hex(&format!("len={n},fill={fill:#x}"), &bin, n * 2 + 1, n * 2 + 9);
        }
    }
}

// ===========================================================================
// sodium_hex2bin
// ===========================================================================

#[derive(Debug, PartialEq, Eq)]
struct H2b {
    rc: i32,
    errno: i32,
    bin_len: usize,
    hex_end: isize,
    bin: Vec<u8>,
}

/// Runs `sodium_hex2bin` on both libraries, comparing return value, `errno`,
/// `*bin_len`, `*hex_end` (as an offset) and the whole output buffer.
fn cmp_hex2bin(
    what: &str,
    hex_in: &[u8],
    hex_len: usize,
    bin_maxlen: usize,
    ignore: Option<&[u8]>,
    use_bin_len: bool,
    use_hex_end: bool,
) -> H2b {
    let (c, r) = pair::<Hex2Bin>("sodium_hex2bin");
    let ig = ignore.map_or(ptr::null(), |s| {
        assert_eq!(*s.last().unwrap(), 0, "`ignore` must be NUL-terminated");
        s.as_ptr() as *const c_char
    });
    let bufsz = bin_maxlen + 8;
    let mut res: Vec<H2b> = Vec::new();
    for f in [c, r] {
        let mut bin = canary(bufsz);
        let mut bl = 0xA5A5_A5A5_usize;
        let mut he: *const c_char = ptr::null();
        let blp = if use_bin_len { &raw mut bl } else { ptr::null_mut() };
        let hep = if use_hex_end { &raw mut he } else { ptr::null_mut() };
        set_sentinel();
        let rc = unsafe {
            f(
                bin.as_mut_ptr(),
                bin_maxlen,
                hex_in.as_ptr() as *const c_char,
                hex_len,
                ig,
                blp,
                hep,
            )
        };
        let e = errno();
        let off = if use_hex_end {
            (he as isize) - (hex_in.as_ptr() as isize)
        } else {
            -1
        };
        res.push(H2b { rc, errno: e, bin_len: bl, hex_end: off, bin });
    }
    let (cc, rr) = (&res[0], &res[1]);
    eq_i32(&format!("sodium_hex2bin({what}) rc"), cc.rc, rr.rc);
    eq_i32(&format!("sodium_hex2bin({what}) errno"), cc.errno, rr.errno);
    eq_usize(&format!("sodium_hex2bin({what}) *bin_len"), cc.bin_len, rr.bin_len);
    assert_eq!(
        cc.hex_end, rr.hex_end,
        "sodium_hex2bin({what}) *hex_end offset: C={} Rust={}",
        cc.hex_end, rr.hex_end
    );
    eq_bytes(&format!("sodium_hex2bin({what}) bin"), &cc.bin, &rr.bin);
    res.swap_remove(0)
}

/// CONFIGS G1-007, G1-008, G1-009, G1-010, G1-011, G1-012, G1-013, G1-014, G1-015, G1-016, G1-017, G1-018, G1-019, G1-020, G1-021, G1-022.
#[test]
fn hex2bin_valid() {
    setup();
    let mut rng = Rng::new(0x1001);

    // G1-007: empty input.
    let o = cmp_hex2bin("empty", b"", 0, 0, None, true, false);
    assert_eq!((o.rc, o.bin_len), (0, 0));

    // G1-008/009/010: lowercase / UPPERCASE / MiXeD, all with hex_end = NULL.
    for (h, want) in [
        (&b"00ff"[..], vec![0x00u8, 0xff]),
        (&b"00FF"[..], vec![0x00, 0xff]),
        (&b"AbCd"[..], vec![0xab, 0xcd]),
        (&b"aBcD"[..], vec![0xab, 0xcd]),
        (&b"0123456789abcdefABCDEF"[..], vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xab, 0xcd, 0xef]),
    ] {
        let o = cmp_hex2bin(
            &format!("{}", String::from_utf8_lossy(h)),
            h,
            h.len(),
            h.len() / 2,
            None,
            true,
            false,
        );
        assert_eq!(o.rc, 0);
        assert_eq!(o.bin_len, want.len());
        assert_eq!(&o.bin[..want.len()], &want[..]);
    }

    // Randomised: every byte string round-trips through its own hex form, in
    // lower, upper and randomly-mixed case.
    for n in 0..40usize {
        for kind in 0..3 {
            let bin = rng.bytes(n);
            let h = hex(&bin);
            let bytes: Vec<u8> = h
                .bytes()
                .map(|b| match kind {
                    0 => b,
                    1 => b.to_ascii_uppercase(),
                    _ => {
                        if rng.bool() {
                            b.to_ascii_uppercase()
                        } else {
                            b
                        }
                    }
                })
                .collect();
            let o = cmp_hex2bin(
                &format!("rand n={n} kind={kind}"),
                &bytes,
                bytes.len(),
                n,
                None,
                true,
                true,
            );
            assert_eq!((o.rc, o.bin_len), (0, n));
            assert_eq!(o.hex_end, bytes.len() as isize);
            assert_eq!(&o.bin[..n], &bin[..]);
        }
    }

    // G1-011/012: `ignore` separators consumed at byte boundaries.
    let o = cmp_hex2bin("ignore=: 00:11:22", b"00:11:22", 8, 3, Some(b":\0"), true, false);
    assert_eq!((o.rc, o.bin_len), (0, 3));
    assert_eq!(&o.bin[..3], &[0x00, 0x11, 0x22]);
    let o = cmp_hex2bin(
        "ignore=': \\n' 00 11\\n22",
        b"00 11\n22",
        8,
        3,
        Some(b": \n\0"),
        true,
        false,
    );
    assert_eq!((o.rc, o.bin_len), (0, 3));
    // leading / interior / trailing separators, many random layouts
    for _ in 0..64 {
        let n = rng.range(0, 8);
        let bin = rng.bytes(n);
        let mut s: Vec<u8> = Vec::new();
        let seps: &[u8] = b": \n-";
        for (i, b) in bin.iter().enumerate() {
            if i == 0 || rng.bool() {
                for _ in 0..rng.range(0, 2) {
                    s.push(*rng.pick(seps));
                }
            }
            s.extend_from_slice(format!("{b:02x}").as_bytes());
        }
        for _ in 0..rng.range(0, 3) {
            s.push(*rng.pick(seps));
        }
        let o = cmp_hex2bin(
            &format!("ignore layout {}", String::from_utf8_lossy(&s)),
            &s,
            s.len(),
            n,
            Some(b": \n-\0"),
            true,
            true,
        );
        assert_eq!((o.rc, o.bin_len), (0, n), "input {:?}", String::from_utf8_lossy(&s));
        assert_eq!(&o.bin[..n], &bin[..]);
    }

    // G1-013: an `ignore` char that IS a hex digit is never skipped.
    let o = cmp_hex2bin("ignore=a hex=aa", b"aa", 2, 1, Some(b"a\0"), true, true);
    assert_eq!((o.rc, o.bin_len, o.bin[0]), (0, 1, 0xaa));
    let o = cmp_hex2bin("ignore=0123456789abcdef", b"deadbeef", 8, 4, Some(b"0123456789abcdef\0"), true, true);
    assert_eq!((o.rc, o.bin_len), (0, 4));
    assert_eq!(&o.bin[..4], &[0xde, 0xad, 0xbe, 0xef]);

    // G1-014: ignore = "" (non-NULL empty string) behaves like NULL for every
    // non-NUL char.
    let o = cmp_hex2bin("ignore=empty", b"0011", 4, 2, Some(b"\0"), true, true);
    assert_eq!((o.rc, o.bin_len), (0, 2));

    // G1-015: strchr(ignore, '\0') matches the terminator, so an embedded NUL
    // is skippable whenever `ignore` is non-NULL.
    let o = cmp_hex2bin("embedded NUL, ignore=:", b"00\x0011", 5, 2, Some(b":\0"), true, true);
    assert_eq!((o.rc, o.bin_len), (0, 2));
    assert_eq!(&o.bin[..2], &[0x00, 0x11]);
    assert_eq!(o.hex_end, 5);
    // ... and with ignore = "" as well
    let o = cmp_hex2bin("embedded NUL, ignore=empty", b"00\x0011", 5, 2, Some(b"\0"), true, true);
    assert_eq!((o.rc, o.bin_len), (0, 2));

    // G1-016: same input with ignore = NULL -> the loop breaks at the NUL.
    let o = cmp_hex2bin("embedded NUL, ignore=NULL", b"00\x0011", 5, 2, None, true, true);
    assert_eq!((o.rc, o.bin_len, o.hex_end), (0, 1, 2));

    // G1-017: trailing garbage tolerated when hex_end != NULL.
    let o = cmp_hex2bin("00zz", b"00zz", 4, 2, None, true, true);
    assert_eq!((o.rc, o.bin_len, o.hex_end), (0, 1, 2));
    assert_eq!(o.bin[0], 0x00);
    for tail in [&b"!"[..], b"zz", b"-", b" ", b"\x7f", b"\xff\xff", b"G", b"g"] {
        let mut s = b"deadbe".to_vec();
        s.extend_from_slice(tail);
        let o = cmp_hex2bin(
            &format!("garbage tail {:?}", tail),
            &s,
            s.len(),
            4,
            None,
            true,
            true,
        );
        assert_eq!((o.rc, o.bin_len, o.hex_end), (0, 3, 6));
    }

    // G1-018: hex_end on full consumption.
    let o = cmp_hex2bin("0011 full", b"0011", 4, 2, None, true, true);
    assert_eq!((o.rc, o.bin_len, o.hex_end), (0, 2, 4));

    // G1-019: bin_len = NULL.
    let o = cmp_hex2bin("bin_len=NULL", b"0011", 4, 2, None, false, false);
    assert_eq!(o.rc, 0);
    // and both out-params NULL
    let o = cmp_hex2bin("both out NULL", b"0011", 4, 2, None, false, false);
    assert_eq!(o.rc, 0);

    // G1-020: bin_maxlen much larger than needed -> the rest of `bin` untouched.
    let o = cmp_hex2bin("bin_maxlen=64", b"0011", 4, 64, None, true, true);
    assert_eq!((o.rc, o.bin_len), (0, 2));
    assert_eq!(&o.bin[2..], &canary(72)[2..], "bin tail must be untouched");

    // G1-021: hex_len bounds the scan, not a NUL.
    let o = cmp_hex2bin("hex_len=4 of 8", b"00112233", 4, 8, None, true, true);
    assert_eq!((o.rc, o.bin_len, o.hex_end), (0, 2, 4));
    for cut in 0..=8usize {
        if cut % 2 != 0 {
            continue;
        }
        let o = cmp_hex2bin(
            &format!("hex_len={cut}"),
            b"00112233",
            cut,
            8,
            None,
            true,
            true,
        );
        assert_eq!((o.rc, o.bin_len), (0, cut / 2));
    }

    // G1-022: large input.
    for n in [512usize, 1024] {
        let bin = rng.bytes(n);
        let h = hex(&bin);
        let o = cmp_hex2bin(&format!("large n={n}"), h.as_bytes(), h.len(), n, None, true, true);
        assert_eq!((o.rc, o.bin_len), (0, n));
        assert_eq!(&o.bin[..n], &bin[..]);
    }
}

// ===========================================================================
// sodium_base64_encoded_len
// ===========================================================================

/// CONFIGS G1-023, G1-024, G1-025, G1-026, G1-027, G1-028.
#[test]
fn base64_encoded_len_valid() {
    setup();
    let (c, r) = pair::<EncLen>("sodium_base64_encoded_len");
    let mut rng = Rng::new(0x1002);

    // G1-023/024/025/026: the documented tables for every variant.
    let expect: &[(i32, [usize; 7])] = &[
        (1, [1, 5, 5, 5, 9, 9, 9]),
        (3, [1, 3, 4, 5, 7, 8, 9]),
        (5, [1, 5, 5, 5, 9, 9, 9]),
        (7, [1, 3, 4, 5, 7, 8, 9]),
    ];
    for &(v, ref table) in expect {
        for (n, &want) in table.iter().enumerate() {
            let (a, b) = unsafe { (c(n, v), r(n, v)) };
            eq_usize(&format!("sodium_base64_encoded_len({n}, {v})"), a, b);
            assert_eq!(a, want, "sodium_base64_encoded_len({n}, {v})");
        }
    }

    // Exhaustive small range plus randomised large values, all 4 variants.
    for &v in VARIANTS {
        for n in 0..512usize {
            let (a, b) = unsafe { (c(n, v), r(n, v)) };
            eq_usize(&format!("sodium_base64_encoded_len({n}, {v})"), a, b);
        }
        // G1-027: large valid values.
        for n in [1_000_000usize, 999_999, 1_000_001, 3_000_000] {
            let (a, b) = unsafe { (c(n, v), r(n, v)) };
            eq_usize(&format!("sodium_base64_encoded_len({n}, {v})"), a, b);
        }
        for _ in 0..64 {
            let n = rng.next_u64() as usize >> 8; // large but far from the threshold
            let (a, b) = unsafe { (c(n, v), r(n, v)) };
            eq_usize(&format!("sodium_base64_encoded_len({n}, {v})"), a, b);
        }
        // G1-028: exactly one below the misuse threshold.
        //   threshold: bin_len / 3 > (SIZE_MAX - 5) / 4
        let thresh = 0xBFFF_FFFF_FFFF_FFFDusize;
        for n in [thresh - 1, thresh - 2, thresh - 3] {
            assert!(
                n / 3 <= (usize::MAX - 5) / 4,
                "test bug: {n:#x} is above the misuse threshold"
            );
            let (a, b) = unsafe { (c(n, v), r(n, v)) };
            eq_usize(&format!("sodium_base64_encoded_len({n:#x}, {v})"), a, b);
        }
    }
}

// ===========================================================================
// sodium_bin2base64
// ===========================================================================

fn cmp_bin2base64(what: &str, bin: &[u8], variant: i32, b64_maxlen: usize, bufsz: usize) -> Vec<u8> {
    let (c, r) = pair::<Bin2B64>("sodium_bin2base64");
    assert!(bufsz >= b64_maxlen);
    let mut a = canary(bufsz);
    let mut b = canary(bufsz);
    let pa = unsafe {
        c(
            a.as_mut_ptr() as *mut c_char,
            b64_maxlen,
            bin.as_ptr(),
            bin.len(),
            variant,
        )
    };
    let pb = unsafe {
        r(
            b.as_mut_ptr() as *mut c_char,
            b64_maxlen,
            bin.as_ptr(),
            bin.len(),
            variant,
        )
    };
    assert_eq!(pa as usize, a.as_ptr() as usize, "{what}: C must return `b64`");
    assert_eq!(pb as usize, b.as_ptr() as usize, "{what}: Rust must return `b64`");
    eq_bytes(&format!("sodium_bin2base64({what})"), &a, &b);
    a
}

/// CONFIGS G1-029, G1-030, G1-031, G1-032, G1-033, G1-034, G1-035, G1-036, G1-037, G1-038, G1-039, G1-040, G1-041.
#[test]
fn bin2base64_valid() {
    setup();
    let mut rng = Rng::new(0x1003);
    let enc_len = sym::<EncLen>(c_lib(), "sodium_base64_encoded_len");

    // G1-029: bin_len = 0, b64_maxlen = 1.
    for &v in VARIANTS {
        let out = cmp_bin2base64(&format!("empty v={v}"), &[], v, 1, 8);
        assert_eq!(out[0], 0);
        assert_eq!(&out[1..], &canary(8)[1..]);
    }

    // G1-030/031/032/033/034/037: the remainder cases per variant.
    let cases: &[(i32, &[u8], &str)] = &[
        (1, &[0x00], "AA=="),
        (1, &[0x00, 0x00], "AAA="),
        (1, &[0x00, 0x00, 0x00], "AAAA"),
        (3, &[0x00], "AA"),
        (3, &[0x00, 0x00], "AAA"),
        (5, &[0xff, 0xff, 0xff], "____"),
        (1, &[0xff, 0xff, 0xff], "////"),
        (5, &[0xfb, 0xef, 0xbe], "----"),
        (1, &[0xfb, 0xef, 0xbe], "++++"),
        (7, &[0xff], "_w"),
        (7, &[0xff, 0xff], "__8"),
    ];
    for &(v, bin, want) in cases {
        let need = unsafe { enc_len(bin.len(), v) };
        let out = cmp_bin2base64(&format!("v={v} bin={}", hex(bin)), bin, v, need, need + 8);
        assert_eq!(
            &out[..want.len()],
            want.as_bytes(),
            "sodium_bin2base64(v={v}, {}) should be {want}",
            hex(bin)
        );
        assert_eq!(out[want.len()], 0);
    }
    // G1-033: bin_len 4 and 5 both give b64_len = 8 for the padded variants.
    for &v in &[1, 5] {
        for n in [4usize, 5] {
            let bin = rng.bytes(n);
            let out = cmp_bin2base64(&format!("v={v} n={n}"), &bin, v, 9, 16);
            assert_eq!(out[8], 0, "b64_len must be 8");
            assert_ne!(out[7], 0);
        }
    }

    // G1-038: all 4 variants x every bin_len % 3 at small and large sizes.
    for &v in VARIANTS {
        for &n in &[
            0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 30, 31, 32, 33, 47, 48, 49, 63, 64, 65,
            1024, 1025, 1026,
        ] {
            for kind in 0..3 {
                let bin = match kind {
                    0 => rng.bytes(n),
                    1 => vec![0u8; n],
                    _ => vec![0xffu8; n],
                };
                let need = unsafe { enc_len(n, v) };
                // G1-039: exact fit.
                let out = cmp_bin2base64(
                    &format!("v={v} n={n} kind={kind} exact"),
                    &bin,
                    v,
                    need,
                    need + 8,
                );
                assert_eq!(out[need - 1], 0, "the terminator must be the last byte");
                assert_eq!(&out[need..], &canary(need + 8)[need..]);
            }
        }
    }

    // G1-040: b64_maxlen MUCH larger than needed -> the whole remaining buffer
    // is zero-filled (unlike sodium_bin2hex).
    for &v in VARIANTS {
        for &n in &[0usize, 1, 2, 3, 6, 9] {
            let bin = rng.bytes(n);
            let need = unsafe { enc_len(n, v) };
            let out = cmp_bin2base64(&format!("v={v} n={n} max=64"), &bin, v, 64, 64);
            assert!(
                out[need - 1..].iter().all(|&x| x == 0),
                "sodium_bin2base64 must zero-fill up to b64_maxlen: {}",
                hex(&out)
            );
            assert_eq!(out.len(), 64);
        }
    }

    // G1-039 + G1-040 combined sweep: every `b64_maxlen` from the exact fit up
    // to +40, so the boundary of the `do { b64[b64_pos++] = 0; } while (...)`
    // zero-fill loop is exercised at every offset.
    for &v in VARIANTS {
        for n in 0..12usize {
            let bin = rng.bytes(n);
            let need = unsafe { enc_len(n, v) };
            let base = cmp_bin2base64(
                &format!("sweep v={v} n={n} exact"),
                &bin,
                v,
                need,
                need + 48,
            );
            for extra in 0..40usize {
                let m = need + extra;
                let out = cmp_bin2base64(
                    &format!("sweep v={v} n={n} max={m}"),
                    &bin,
                    v,
                    m,
                    need + 48,
                );
                assert_eq!(
                    &out[..need - 1],
                    &base[..need - 1],
                    "b64_maxlen must not change the encoded text"
                );
                assert!(
                    out[need - 1..m].iter().all(|&x| x == 0),
                    "bytes {}..{m} must be zero-filled: {}",
                    need - 1,
                    hex(&out)
                );
                assert!(
                    out[m..].iter().all(|&x| x == 0xA5),
                    "sodium_bin2base64 must not write past b64_maxlen"
                );
            }
        }
    }

    // G1-041: every one of the 64 alphabet symbols, per variant.
    // 6-bit groups 0,1,2,...,63 packed big-endian = 48 bytes.
    let mut alpha = vec![0u8; 48];
    for g in 0..64usize {
        let bitpos = g * 6;
        for k in 0..6 {
            if (g >> (5 - k)) & 1 == 1 {
                let bit = bitpos + k;
                alpha[bit / 8] |= 0x80 >> (bit % 8);
            }
        }
    }
    for &v in VARIANTS {
        let need = unsafe { enc_len(48, v) };
        let out = cmp_bin2base64(&format!("alphabet v={v}"), &alpha, v, need, need + 8);
        let s = &out[..64];
        let mut seen: Vec<u8> = s.to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 64, "all 64 symbols must appear: {}", String::from_utf8_lossy(s));
        let last = *s.last().unwrap();
        if v & 4 != 0 {
            assert_eq!(last, b'_');
            assert_eq!(s[62], b'-');
        } else {
            assert_eq!(last, b'/');
            assert_eq!(s[62], b'+');
        }
    }
}

// ===========================================================================
// sodium_base642bin
// ===========================================================================

#[derive(Debug)]
struct B2b {
    rc: i32,
    errno: i32,
    bin_len: usize,
    b64_end: isize,
    bin: Vec<u8>,
}

fn cmp_base642bin(
    what: &str,
    b64: &[u8],
    b64_len: usize,
    bin_maxlen: usize,
    ignore: Option<&[u8]>,
    use_bin_len: bool,
    use_b64_end: bool,
    variant: i32,
) -> B2b {
    let (c, r) = pair::<B642Bin>("sodium_base642bin");
    let ig = ignore.map_or(ptr::null(), |s| {
        assert_eq!(*s.last().unwrap(), 0, "`ignore` must be NUL-terminated");
        s.as_ptr() as *const c_char
    });
    let bufsz = bin_maxlen + 8;
    let mut res: Vec<B2b> = Vec::new();
    for f in [c, r] {
        let mut bin = canary(bufsz);
        let mut bl = 0xA5A5_A5A5_usize;
        let mut be: *const c_char = ptr::null();
        let blp = if use_bin_len { &raw mut bl } else { ptr::null_mut() };
        let bep = if use_b64_end { &raw mut be } else { ptr::null_mut() };
        set_sentinel();
        let rc = unsafe {
            f(
                bin.as_mut_ptr(),
                bin_maxlen,
                b64.as_ptr() as *const c_char,
                b64_len,
                ig,
                blp,
                bep,
                variant,
            )
        };
        let e = errno();
        let off = if use_b64_end {
            (be as isize) - (b64.as_ptr() as isize)
        } else {
            -1
        };
        res.push(B2b { rc, errno: e, bin_len: bl, b64_end: off, bin });
    }
    let (cc, rr) = (&res[0], &res[1]);
    eq_i32(&format!("sodium_base642bin({what}) rc"), cc.rc, rr.rc);
    eq_i32(&format!("sodium_base642bin({what}) errno"), cc.errno, rr.errno);
    eq_usize(&format!("sodium_base642bin({what}) *bin_len"), cc.bin_len, rr.bin_len);
    assert_eq!(
        cc.b64_end, rr.b64_end,
        "sodium_base642bin({what}) *b64_end offset: C={} Rust={}",
        cc.b64_end, rr.b64_end
    );
    eq_bytes(&format!("sodium_base642bin({what}) bin"), &cc.bin, &rr.bin);
    res.swap_remove(0)
}

/// CONFIGS G1-042, G1-043, G1-044, G1-045, G1-046, G1-047, G1-048, G1-049, G1-050, G1-051, G1-052, G1-053, G1-054, G1-055, G1-056.
#[test]
fn base642bin_valid() {
    setup();
    let mut rng = Rng::new(0x1004);

    // G1-042/043/044: padded ORIGINAL variant.
    let o = cmp_base642bin("AAAA v=1", b"AAAA", 4, 3, None, true, false, 1);
    assert_eq!((o.rc, o.bin_len), (0, 3));
    let o = cmp_base642bin("AA== v=1", b"AA==", 4, 1, None, true, false, 1);
    assert_eq!((o.rc, o.bin_len), (0, 1));
    let o = cmp_base642bin("AAA= v=1", b"AAA=", 4, 2, None, true, false, 1);
    assert_eq!((o.rc, o.bin_len), (0, 2));

    // G1-045: NO_PADDING variant skips _sodium_base642bin_skip_padding.
    for (s, n) in [(&b"AA"[..], 1usize), (b"AAA", 2), (b"AAAA", 3)] {
        let o = cmp_base642bin(
            &format!("v=3 {}", String::from_utf8_lossy(s)),
            s,
            s.len(),
            n,
            None,
            true,
            false,
            3,
        );
        assert_eq!((o.rc, o.bin_len), (0, n));
    }

    // G1-046: v=3 with padded input and b64_end != NULL stops at the first '='.
    let o = cmp_base642bin("v=3 AA== b64_end", b"AA==", 4, 1, None, true, true, 3);
    assert_eq!((o.rc, o.bin_len, o.b64_end), (0, 1, 2));

    // G1-047/048: URLSAFE alphabet.
    let o = cmp_base642bin("v=5 ____", b"____", 4, 3, None, true, false, 5);
    assert_eq!((o.rc, o.bin_len), (0, 3));
    assert_eq!(&o.bin[..3], &[0xff, 0xff, 0xff]);
    // NOTE: CONFIGS G1-047 lists `variant=7, b64="__"` as a *valid* input, but
    // the C rejects it: two `_` give acc = 0xFFF / acc_len = 4, so the
    // leftover-bits test `(acc & ((1 << acc_len) - 1)) != 0` at codecs.c:319
    // fires. The nearest reachable valid input is `"_w"`. Both behaviours are
    // asserted here.
    let o = cmp_base642bin("v=7 __ (non-canonical)", b"__", 2, 1, None, true, false, 7);
    assert_eq!((o.rc, o.bin_len), (-1, 0));
    assert_eq!(o.bin[0], 0xff, "the byte is written before the rejection");
    let o = cmp_base642bin("v=7 _w", b"_w", 2, 1, None, true, false, 7);
    assert_eq!((o.rc, o.bin_len, o.bin[0]), (0, 1, 0xff));
    let o = cmp_base642bin("v=5 ----", b"----", 4, 3, None, true, false, 5);
    assert_eq!((o.rc, o.bin_len), (0, 3));
    assert_eq!(&o.bin[..3], &[0xfb, 0xef, 0xbe]);

    // G1-049/050/051: `ignore` in the data, in the padding run and after it.
    let o = cmp_base642bin("ignore ' \\n'", b"AAAA AAAA\n", 10, 6, Some(b" \n\0"), true, false, 1);
    assert_eq!((o.rc, o.bin_len), (0, 6));
    let o = cmp_base642bin("ignore ' ' AA == ", b"AA == ", 6, 1, Some(b" \0"), true, false, 1);
    assert_eq!((o.rc, o.bin_len), (0, 1));
    let o = cmp_base642bin("ignore ' ' AA== ", b"AA== ", 5, 1, Some(b" \0"), true, false, 1);
    assert_eq!((o.rc, o.bin_len), (0, 1));
    let o = cmp_base642bin("ignore ' ' AA==   ", b"AA==   ", 7, 1, Some(b" \0"), true, true, 1);
    assert_eq!((o.rc, o.bin_len, o.b64_end), (0, 1, 7));

    // G1-052: empty input, every variant.
    for &v in VARIANTS {
        let o = cmp_base642bin(&format!("empty v={v}"), b"", 0, 0, None, true, false, v);
        assert_eq!((o.rc, o.bin_len), (0, 0));
        let o = cmp_base642bin(&format!("empty v={v} ign"), b"", 0, 0, Some(b" \0"), true, true, v);
        assert_eq!((o.rc, o.bin_len, o.b64_end), (0, 0, 0));
    }

    // G1-053: both out-params independently optional.
    for (bl, be) in [(true, true), (true, false), (false, true), (false, false)] {
        let o = cmp_base642bin(
            &format!("optional out bl={bl} be={be}"),
            b"AAAA",
            4,
            3,
            None,
            bl,
            be,
            1,
        );
        assert_eq!(o.rc, 0);
    }

    // G1-054: bin_maxlen exact vs much larger.
    let o = cmp_base642bin("exact", b"AAAA", 4, 3, None, true, false, 1);
    assert_eq!(o.rc, 0);
    let o = cmp_base642bin("bin_maxlen=64", b"AAAA", 4, 64, None, true, false, 1);
    assert_eq!((o.rc, o.bin_len), (0, 3));
    assert_eq!(&o.bin[3..], &canary(72)[3..], "bin tail must be untouched");

    // G1-055: embedded NUL skipped when ignore != NULL.
    let o = cmp_base642bin(
        "embedded NUL v=3",
        b"AAAA\x00AAAA",
        9,
        6,
        Some(b" \0"),
        true,
        true,
        3,
    );
    assert_eq!((o.rc, o.bin_len, o.b64_end), (0, 6, 9));

    // G1-056: large input.
    let big: Vec<u8> = (0..4096).map(|i| b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"[i % 64]).collect();
    let o = cmp_base642bin("large v=3", &big, 4096, 3072, None, true, false, 3);
    assert_eq!((o.rc, o.bin_len), (0, 3072));

    // Randomised valid inputs for every variant, produced by sodium_bin2base64.
    let b2b = sym::<Bin2B64>(c_lib(), "sodium_bin2base64");
    let enc_len = sym::<EncLen>(c_lib(), "sodium_base64_encoded_len");
    for &v in VARIANTS {
        for n in 0..48usize {
            let bin = rng.bytes(n);
            let need = unsafe { enc_len(n, v) };
            let mut enc = vec![0u8; need];
            unsafe { b2b(enc.as_mut_ptr() as *mut c_char, need, bin.as_ptr(), n, v) };
            let txt = &enc[..need - 1];
            let o = cmp_base642bin(
                &format!("roundtrip v={v} n={n}"),
                txt,
                txt.len(),
                n,
                None,
                true,
                true,
                v,
            );
            assert_eq!((o.rc, o.bin_len), (0, n));
            assert_eq!(&o.bin[..n], &bin[..]);
        }
    }
}

/// CONFIGS G1-057 — `sodium_bin2base64` -> `sodium_base642bin` round-trip for
/// all 4 variants and `bin_len` 0..8 (plus randomised larger sizes).
#[test]
fn base64_roundtrip() {
    setup();
    let mut rng = Rng::new(0x1005);
    let (cb, rb) = pair::<Bin2B64>("sodium_bin2base64");
    let (cd, rd) = pair::<B642Bin>("sodium_base642bin");
    let enc_len = sym::<EncLen>(c_lib(), "sodium_base64_encoded_len");

    for &v in VARIANTS {
        let mut lens: Vec<usize> = (0..9).collect();
        lens.extend([16, 17, 31, 32, 33, 63, 64, 100, 255, 256]);
        for &n in &lens {
            for _ in 0..3 {
                let bin = rng.bytes(n);
                let need = unsafe { enc_len(n, v) };
                let mut e1 = canary(need);
                let mut e2 = canary(need);
                unsafe {
                    cb(e1.as_mut_ptr() as *mut c_char, need, bin.as_ptr(), n, v);
                    rb(e2.as_mut_ptr() as *mut c_char, need, bin.as_ptr(), n, v);
                }
                eq_bytes(&format!("bin2base64 v={v} n={n}"), &e1, &e2);

                let txt = &e1[..need - 1];
                let mut d1 = canary(n + 8);
                let mut d2 = canary(n + 8);
                let mut l1 = 0usize;
                let mut l2 = 0usize;
                let (ra, rr) = unsafe {
                    (
                        cd(
                            d1.as_mut_ptr(),
                            n,
                            txt.as_ptr() as *const c_char,
                            txt.len(),
                            ptr::null(),
                            &raw mut l1,
                            ptr::null_mut(),
                            v,
                        ),
                        rd(
                            d2.as_mut_ptr(),
                            n,
                            txt.as_ptr() as *const c_char,
                            txt.len(),
                            ptr::null(),
                            &raw mut l2,
                            ptr::null_mut(),
                            v,
                        ),
                    )
                };
                eq_i32(&format!("base642bin v={v} n={n} rc"), ra, rr);
                eq_usize(&format!("base642bin v={v} n={n} len"), l1, l2);
                eq_bytes(&format!("base642bin v={v} n={n} bin"), &d1, &d2);
                assert_eq!(ra, 0, "round-trip must succeed (v={v}, n={n})");
                assert_eq!(l1, n);
                assert_eq!(&d1[..n], &bin[..], "round-trip must recover the input");
            }
        }
    }
}

// ===========================================================================
// sodium_ip2bin / sodium_bin2ip
// ===========================================================================

fn cmp_ip2bin(what: &str, ip: &[u8], ip_len: usize) -> (i32, Vec<u8>) {
    let (c, r) = pair::<Ip2Bin>("sodium_ip2bin");
    let mut a = canary(16);
    let mut b = canary(16);
    let ra = unsafe { c(a.as_mut_ptr(), ip.as_ptr() as *const c_char, ip_len) };
    let rb = unsafe { r(b.as_mut_ptr(), ip.as_ptr() as *const c_char, ip_len) };
    eq_i32(&format!("sodium_ip2bin({what}) rc"), ra, rb);
    eq_bytes(&format!("sodium_ip2bin({what}) bin"), &a, &b);
    (ra, a)
}

fn cmp_bin2ip(what: &str, bin: &[u8; 16], ip_maxlen: usize, bufsz: usize) -> (bool, Vec<u8>) {
    let (c, r) = pair::<Bin2Ip>("sodium_bin2ip");
    assert!(bufsz >= ip_maxlen);
    let mut a = canary(bufsz);
    let mut b = canary(bufsz);
    let pa = unsafe { c(a.as_mut_ptr() as *mut c_char, ip_maxlen, bin.as_ptr()) };
    let pb = unsafe { r(b.as_mut_ptr() as *mut c_char, ip_maxlen, bin.as_ptr()) };
    assert_eq!(
        pa.is_null(),
        pb.is_null(),
        "sodium_bin2ip({what}): NULL-ness differs (C null={}, Rust null={})",
        pa.is_null(),
        pb.is_null()
    );
    if !pa.is_null() {
        assert_eq!(pa as usize, a.as_ptr() as usize, "{what}: C must return `ip`");
        assert_eq!(pb as usize, b.as_ptr() as usize, "{what}: Rust must return `ip`");
    }
    eq_bytes(&format!("sodium_bin2ip({what}) buffer"), &a, &b);
    (!pa.is_null(), a)
}

fn v4mapped(o: [u8; 4]) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[10] = 0xff;
    b[11] = 0xff;
    b[12..].copy_from_slice(&o);
    b
}

fn groups(g: [u16; 8]) -> [u8; 16] {
    let mut b = [0u8; 16];
    for i in 0..8 {
        b[i * 2] = (g[i] >> 8) as u8;
        b[i * 2 + 1] = (g[i] & 0xff) as u8;
    }
    b
}

fn text(buf: &[u8]) -> String {
    let n = buf.iter().position(|&x| x == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..n]).into_owned()
}

/// CONFIGS G1-058, G1-059, G1-060, G1-061, G1-062, G1-063, G1-064, G1-065, G1-066, G1-067, G1-068, G1-069, G1-070, G1-071, G1-072, G1-073.
#[test]
fn ip2bin_valid() {
    setup();

    // G1-058/059/060: IPv4 dotted-quad, leading zeros, boundary octets.
    let v4: &[(&str, [u8; 4])] = &[
        ("0.0.0.0", [0, 0, 0, 0]),
        ("1.2.3.4", [1, 2, 3, 4]),
        ("255.255.255.255", [255, 255, 255, 255]),
        ("01.02.03.04", [1, 2, 3, 4]),
        ("001.002.003.004", [1, 2, 3, 4]),
        ("0.0.0.255", [0, 0, 0, 255]),
        ("255.0.0.0", [255, 0, 0, 0]),
        ("10.0.0.1", [10, 0, 0, 1]),
        ("192.168.100.200", [192, 168, 100, 200]),
        ("000.000.000.000", [0, 0, 0, 0]),
        ("099.100.101.102", [99, 100, 101, 102]),
    ];
    for &(s, oct) in v4 {
        let (rc, bin) = cmp_ip2bin(s, s.as_bytes(), s.len());
        assert_eq!(rc, 0, "sodium_ip2bin({s}) must succeed");
        assert_eq!(&bin[..], &v4mapped(oct)[..], "sodium_ip2bin({s})");
    }
    // exhaustive-ish: every octet value in each position
    for pos in 0..4usize {
        for v in 0..=255u16 {
            let mut oct = [1u8, 2, 3, 4];
            oct[pos] = v as u8;
            let s = format!("{}.{}.{}.{}", oct[0], oct[1], oct[2], oct[3]);
            let (rc, bin) = cmp_ip2bin(&s, s.as_bytes(), s.len());
            assert_eq!(rc, 0);
            assert_eq!(&bin[..], &v4mapped(oct)[..]);
        }
    }

    // G1-061 … G1-070: IPv6 forms.
    let v6: &[(&str, [u16; 8])] = &[
        ("0001:0002:0003:0004:0005:0006:0007:0008", [1, 2, 3, 4, 5, 6, 7, 8]),
        ("1:2:3:4:5:6:7:8", [1, 2, 3, 4, 5, 6, 7, 8]),
        ("::", [0, 0, 0, 0, 0, 0, 0, 0]),
        ("::1", [0, 0, 0, 0, 0, 0, 0, 1]),
        ("1::", [1, 0, 0, 0, 0, 0, 0, 0]),
        ("1::8", [1, 0, 0, 0, 0, 0, 0, 8]),
        ("1:2::7:8", [1, 2, 0, 0, 0, 0, 7, 8]),
        ("FE80::1", [0xfe80, 0, 0, 0, 0, 0, 0, 1]),
        ("Fe80::AbCd", [0xfe80, 0, 0, 0, 0, 0, 0, 0xabcd]),
        ("fe80::abcd", [0xfe80, 0, 0, 0, 0, 0, 0, 0xabcd]),
        (
            "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
            [0xffff; 8],
        ),
        ("2001:db8::1", [0x2001, 0x0db8, 0, 0, 0, 0, 0, 1]),
        ("2001:0DB8:0000:0000:0000:0000:0000:0001", [0x2001, 0x0db8, 0, 0, 0, 0, 0, 1]),
        ("::ffff:1.2.3.4", [0, 0, 0, 0, 0, 0xffff, 0x0102, 0x0304]),
        ("::1.2.3.4", [0, 0, 0, 0, 0, 0, 0x0102, 0x0304]),
        ("64:ff9b::1.2.3.4", [0x64, 0xff9b, 0, 0, 0, 0, 0x0102, 0x0304]),
        ("1:2:3:4:5:6:1.2.3.4", [1, 2, 3, 4, 5, 6, 0x0102, 0x0304]),
        ("0:0:0:0:0:0:0:0", [0; 8]),
        ("1::2:3:4:5:6:7", [1, 0, 2, 3, 4, 5, 6, 7]),
    ];
    for &(s, g) in v6 {
        let (rc, bin) = cmp_ip2bin(s, s.as_bytes(), s.len());
        assert_eq!(rc, 0, "sodium_ip2bin({s}) must succeed");
        assert_eq!(&bin[..], &groups(g)[..], "sodium_ip2bin({s})");
    }
    // G1-070: "::ffff:1.2.3.4" == ip2bin("1.2.3.4")
    let (_, m) = cmp_ip2bin("::ffff:1.2.3.4", b"::ffff:1.2.3.4", 14);
    let (_, d) = cmp_ip2bin("1.2.3.4", b"1.2.3.4", 7);
    assert_eq!(m, d, "the v4-mapped form must equal the dotted-quad form");

    // G1-069: zone / scope ids are validated then discarded.
    let (_, plain) = cmp_ip2bin("fe80::1", b"fe80::1", 7);
    for z in ["fe80::1%eth0", "fe80::1%1", "fe80::1%en0.5", "fe80::1%A_b-c", "fe80::1%0", "fe80::1%Z", "fe80::1%a-b_c.d9"] {
        let (rc, bin) = cmp_ip2bin(z, z.as_bytes(), z.len());
        assert_eq!(rc, 0, "sodium_ip2bin({z}) must succeed");
        assert_eq!(bin, plain, "the zone id must be discarded ({z})");
    }

    // G1-071: ip_len_ shorter than the string.
    let (rc, bin) = cmp_ip2bin("1.2.3.4extra / len=7", b"1.2.3.4extra", 7);
    assert_eq!(rc, 0);
    assert_eq!(&bin[..], &v4mapped([1, 2, 3, 4])[..]);

    // G1-072: ip_len_ larger than the NUL-terminated string.
    let mut padded = b"1.2.3.4".to_vec();
    padded.resize(64, 0);
    let (rc, bin) = cmp_ip2bin("1.2.3.4 / len=64", &padded, 64);
    assert_eq!(rc, 0);
    assert_eq!(&bin[..], &v4mapped([1, 2, 3, 4])[..]);
    let mut padded6 = b"1:2:3:4:5:6:7:8".to_vec();
    padded6.resize(64, 0);
    let (rc, bin) = cmp_ip2bin("v6 / len=64", &padded6, 64);
    assert_eq!(rc, 0);
    assert_eq!(&bin[..], &groups([1, 2, 3, 4, 5, 6, 7, 8])[..]);

    // G1-073: ip_len_ authoritative -> a truncated address is rejected.
    let (rc, _) = cmp_ip2bin("1:2:3:4:5:6:7:8 / len=3", b"1:2:3:4:5:6:7:8", 3);
    assert_eq!(rc, -1, "a truncated address must be rejected");
    // ... but every prefix length must agree between C and Rust
    for cut in 0..=15usize {
        cmp_ip2bin(&format!("v6 prefix len={cut}"), b"1:2:3:4:5:6:7:8", cut);
    }
    for cut in 0..=7usize {
        cmp_ip2bin(&format!("v4 prefix len={cut}"), b"1.2.3.4", cut);
    }
}

/// CONFIGS G1-074, G1-075, G1-076, G1-077, G1-078, G1-079, G1-080, G1-081, G1-082, G1-083, G1-084, G1-085, G1-086.
#[test]
fn bin2ip_valid() {
    setup();

    // G1-074/075/076: the IPv4-mapped branch.
    for (oct, want) in [
        ([1u8, 2, 3, 4], "1.2.3.4"),
        ([0, 0, 0, 0], "0.0.0.0"),
        ([255, 255, 255, 255], "255.255.255.255"),
        ([10, 0, 0, 1], "10.0.0.1"),
        ([0, 0, 0, 255], "0.0.0.255"),
        ([100, 200, 30, 4], "100.200.30.4"),
    ] {
        let bin = v4mapped(oct);
        let (ok, buf) = cmp_bin2ip(&format!("v4 {want}"), &bin, 16, 48);
        assert!(ok);
        assert_eq!(text(&buf), want, "sodium_bin2ip must use base 10, no leading zeros");
        // G1-086: only len + 1 bytes are written.
        let len = want.len();
        assert_eq!(&buf[len + 1..], &canary(48)[len + 1..]);
        let (ok, buf2) = cmp_bin2ip(&format!("v4 {want} maxlen=46"), &bin, 46, 48);
        assert!(ok);
        assert_eq!(text(&buf2), want);
        // exact-fit boundary: ip_maxlen == len + 1
        let (ok, buf3) = cmp_bin2ip(&format!("v4 {want} exact"), &bin, len + 1, 48);
        assert!(ok);
        assert_eq!(text(&buf3), want);
    }
    // every octet value in every position
    for pos in 0..4usize {
        for v in 0..=255u16 {
            let mut oct = [1u8, 22, 133, 4];
            oct[pos] = v as u8;
            let bin = v4mapped(oct);
            let (ok, buf) = cmp_bin2ip(&format!("v4 pos={pos} v={v}"), &bin, 46, 48);
            assert!(ok);
            assert_eq!(
                text(&buf),
                format!("{}.{}.{}.{}", oct[0], oct[1], oct[2], oct[3])
            );
        }
    }

    // G1-077 … G1-085: the IPv6 branch.
    let v6: &[([u16; 8], &str)] = &[
        ([0, 0, 0, 0, 0, 0, 0, 0], "::"),
        ([0, 0, 0, 0, 0, 0, 0, 1], "::1"),
        ([1, 0, 2, 3, 4, 5, 6, 7], "1:0:2:3:4:5:6:7"),
        ([1, 0, 0, 2, 0, 0, 0, 3], "1:0:0:2::3"),
        ([1, 0, 0, 2, 0, 0, 3, 4], "1::2:0:0:3:4"),
        ([1, 2, 3, 4, 5, 6, 0, 0], "1:2:3:4:5:6::"),
        ([0, 0, 1, 2, 3, 4, 5, 6], "::1:2:3:4:5:6"),
        ([0xffff; 8], "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff"),
        ([0x0001, 0x0010, 0x0100, 0x1000, 1, 2, 3, 4], "1:10:100:1000:1:2:3:4"),
        ([1, 2, 3, 4, 5, 6, 7, 8], "1:2:3:4:5:6:7:8"),
        ([0xfe80, 0, 0, 0, 0, 0, 0, 1], "fe80::1"),
        ([0, 1, 0, 1, 0, 1, 0, 1], "0:1:0:1:0:1:0:1"),
        ([0, 0, 1, 0, 0, 1, 0, 0], "::1:0:0:1:0:0"),
        ([1, 0, 0, 0, 0, 0, 0, 0], "1::"),
        ([0, 0, 0, 0, 0, 0, 1, 0], "::1:0"),
        ([0xabcd, 0xef01, 0x2345, 0x6789, 0xa, 0xb, 0xc, 0xd], "abcd:ef01:2345:6789:a:b:c:d"),
    ];
    for &(g, want) in v6 {
        let bin = groups(g);
        let (ok, buf) = cmp_bin2ip(want, &bin, 46, 48);
        assert!(ok, "sodium_bin2ip({want}) must succeed");
        assert_eq!(text(&buf), want, "sodium_bin2ip -> {want}");
        // G1-086: v6 branch writes exactly len + 1 bytes.
        assert_eq!(&buf[want.len() + 1..], &canary(48)[want.len() + 1..]);
        // G1-077/078: minimum accepted ip_maxlen.
        let (ok, buf) = cmp_bin2ip(&format!("{want} exact"), &bin, want.len() + 1, 48);
        assert!(ok);
        assert_eq!(text(&buf), want);
    }
    // G1-084: all-0xff needs ip_maxlen >= 40.
    let all_ff = [0xffu8; 16];
    let (ok, _) = cmp_bin2ip("ffff... maxlen=40", &all_ff, 40, 48);
    assert!(ok);

    // Randomised: all 2^8 single-group patterns and random bins.
    let mut rng = Rng::new(0x1006);
    for _ in 0..2000 {
        let mut g = [0u16; 8];
        for i in 0..8 {
            // bias towards zeros so that compression runs are common
            g[i] = if rng.below(3) == 0 {
                0
            } else {
                rng.next_u32() as u16
            };
        }
        let bin = groups(g);
        cmp_bin2ip(&format!("rand {g:?}"), &bin, 46, 48);
        // and at a tight ip_maxlen so the `len >= ip_maxlen` boundary is hit
        for m in 3..=8usize {
            cmp_bin2ip(&format!("rand {g:?} maxlen={m}"), &bin, m, 48);
        }
    }
    for _ in 0..500 {
        let v = rng.bytes(16);
        let mut bin = [0u8; 16];
        bin.copy_from_slice(&v);
        cmp_bin2ip("fully random", &bin, 46, 48);
    }
}

/// CONFIGS G1-087 — `sodium_bin2ip(sodium_ip2bin(x))` is the canonical form.
#[test]
fn ip_roundtrip() {
    setup();
    let mut rng = Rng::new(0x1007);
    let (ci, ri) = pair::<Ip2Bin>("sodium_ip2bin");
    let (cb, rb) = pair::<Bin2Ip>("sodium_bin2ip");

    let inputs: &[&str] = &[
        "0.0.0.0",
        "1.2.3.4",
        "255.255.255.255",
        "01.02.03.04",
        "::",
        "::1",
        "1::",
        "1::8",
        "1:2::7:8",
        "0001:0002:0003:0004:0005:0006:0007:0008",
        "FE80::1",
        "fe80::1%eth0",
        "::ffff:1.2.3.4",
        "::1.2.3.4",
        "64:ff9b::1.2.3.4",
        "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
        "1:0:0:2:0:0:0:3",
        "1:0:0:2:0:0:3:4",
        "1:2:3:4:5:6:0:0",
        "0:0:1:2:3:4:5:6",
    ];
    // hand-written plus a large randomised set generated from random bins
    let mut all: Vec<String> = inputs.iter().map(|s| s.to_string()).collect();
    for _ in 0..400 {
        let mut g = [0u16; 8];
        for i in 0..8 {
            g[i] = if rng.below(3) == 0 { 0 } else { rng.next_u32() as u16 };
        }
        let bin = groups(g);
        let mut buf = canary(48);
        let p = unsafe { cb(buf.as_mut_ptr() as *mut c_char, 46, bin.as_ptr()) };
        assert!(!p.is_null());
        all.push(text(&buf));
    }
    for _ in 0..200 {
        let o = rng.bytes(4);
        all.push(format!("{}.{}.{}.{}", o[0], o[1], o[2], o[3]));
    }

    for s in &all {
        let mut b1 = canary(16);
        let mut b2 = canary(16);
        let (ra, rr) = unsafe {
            (
                ci(b1.as_mut_ptr(), s.as_ptr() as *const c_char, s.len()),
                ri(b2.as_mut_ptr(), s.as_ptr() as *const c_char, s.len()),
            )
        };
        eq_i32(&format!("ip2bin({s}) rc"), ra, rr);
        eq_bytes(&format!("ip2bin({s})"), &b1, &b2);
        if ra != 0 {
            continue;
        }
        let mut t1 = canary(48);
        let mut t2 = canary(48);
        let (pa, pb) = unsafe {
            (
                cb(t1.as_mut_ptr() as *mut c_char, 46, b1.as_ptr()),
                rb(t2.as_mut_ptr() as *mut c_char, 46, b2.as_ptr()),
            )
        };
        assert_eq!(pa.is_null(), pb.is_null(), "bin2ip({s}) NULL-ness");
        eq_bytes(&format!("bin2ip(ip2bin({s}))"), &t1, &t2);
        // the canonical form must itself round-trip to the same bytes
        let canon = text(&t1);
        let mut b3 = canary(16);
        let mut b4 = canary(16);
        let (ra2, rr2) = unsafe {
            (
                ci(b3.as_mut_ptr(), canon.as_ptr() as *const c_char, canon.len()),
                ri(b4.as_mut_ptr(), canon.as_ptr() as *const c_char, canon.len()),
            )
        };
        eq_i32(&format!("ip2bin(canon {canon}) rc"), ra2, rr2);
        eq_bytes(&format!("ip2bin(canon {canon})"), &b3, &b4);
        assert_eq!(ra2, 0, "the canonical form {canon} must re-parse");
        assert_eq!(b3, b1, "canonical form {canon} must map to the same bytes");
    }
}

// ===========================================================================
// sodium_pad / sodium_unpad
// ===========================================================================

fn cmp_pad(
    what: &str,
    data: &[u8],
    blocksize: usize,
    max_buflen: usize,
    use_out: bool,
) -> (i32, usize, Vec<u8>) {
    let (c, r) = pair::<Pad>("sodium_pad");
    let bufsz = max_buflen.max(data.len()) + 8;
    let mut res: Vec<(i32, usize, Vec<u8>)> = Vec::new();
    for f in [c, r] {
        let mut buf = canary(bufsz);
        buf[..data.len()].copy_from_slice(data);
        let mut out = 0xA5A5_A5A5_usize;
        let op = if use_out { &raw mut out } else { ptr::null_mut() };
        let rc = unsafe { f(op, buf.as_mut_ptr(), data.len(), blocksize, max_buflen) };
        res.push((rc, out, buf));
    }
    eq_i32(&format!("sodium_pad({what}) rc"), res[0].0, res[1].0);
    eq_usize(&format!("sodium_pad({what}) *padded_buflen_p"), res[0].1, res[1].1);
    eq_bytes(&format!("sodium_pad({what}) buf"), &res[0].2, &res[1].2);
    res.swap_remove(0)
}

fn cmp_unpad(what: &str, buf: &[u8], blocksize: usize) -> (i32, usize) {
    let (c, r) = pair::<Unpad>("sodium_unpad");
    let mut res: Vec<(i32, usize)> = Vec::new();
    for f in [c, r] {
        let mut out = 0xA5A5_A5A5_usize;
        let rc = unsafe { f(&raw mut out, buf.as_ptr(), buf.len(), blocksize) };
        res.push((rc, out));
    }
    eq_i32(&format!("sodium_unpad({what}) rc"), res[0].0, res[1].0);
    eq_usize(&format!("sodium_unpad({what}) *unpadded_buflen_p"), res[0].1, res[1].1);
    res[0]
}

/// CONFIGS G1-088, G1-089, G1-090, G1-091, G1-092, G1-093, G1-094, G1-095, G1-096.
#[test]
fn pad_valid() {
    setup();
    let mut rng = Rng::new(0x1008);

    // G1-088: blocksize = 1.
    for &n in &[0usize, 1, 5, 64] {
        let d = rng.bytes(n);
        let (rc, out, buf) = cmp_pad(&format!("bs=1 n={n}"), &d, 1, n + 1, true);
        assert_eq!((rc, out), (0, n + 1));
        assert_eq!(&buf[..n], &d[..], "the data must be preserved");
        assert_eq!(buf[n], 0x80);
    }

    // G1-089/090/091/092/093: power-of-two mask path and `%` path.
    let table: &[(usize, &[(usize, usize)])] = &[
        (2, &[(0, 2), (1, 2), (2, 4), (3, 4)]),
        (16, &[(0, 16), (1, 16), (15, 16), (16, 32), (17, 32), (32, 48)]),
        (64, &[(0, 64), (1, 64), (63, 64), (64, 128), (65, 128)]),
        (3, &[(0, 3), (1, 3), (2, 3), (3, 6), (4, 6)]),
        (17, &[(0, 17), (16, 17), (17, 34), (18, 34)]),
    ];
    for &(bs, rows) in table {
        for &(n, want) in rows {
            let d = rng.bytes(n);
            let (rc, out, buf) = cmp_pad(&format!("bs={bs} n={n}"), &d, bs, want, true);
            assert_eq!((rc, out), (0, want), "sodium_pad(n={n}, bs={bs})");
            assert_eq!(&buf[..n], &d[..], "the data must be preserved");
            assert_eq!(buf[n], 0x80, "the barrier byte");
            assert!(
                buf[n + 1..want].iter().all(|&x| x == 0),
                "everything after the barrier must be zero"
            );
            assert_eq!(&buf[want..], &canary(want + 8)[want..], "nothing past the padding");
        }
    }

    // G1-094: padded_buflen_p = NULL.
    for &bs in &[1usize, 2, 3, 16, 17, 64] {
        for n in 0..(bs * 2 + 2) {
            let d = rng.bytes(n);
            let want = n - n % bs + bs;
            let (rc, _, buf) = cmp_pad(&format!("bs={bs} n={n} out=NULL"), &d, bs, want, false);
            assert_eq!(rc, 0);
            assert_eq!(&buf[..n], &d[..]);
            assert_eq!(buf[n], 0x80);
        }
    }

    // G1-095: max_buflen exactly xpadded_len + 1 vs much larger; only
    // buf[xpadded_len - blocksize + 1 .. xpadded_len] is touched.
    for &bs in &[1usize, 2, 3, 16, 17, 64] {
        for n in 0..40usize {
            let d = rng.bytes(n);
            let xpadded = n - n % bs + bs - 1;
            let (rc, out, tight) = cmp_pad(
                &format!("bs={bs} n={n} tight"),
                &d,
                bs,
                xpadded + 1,
                true,
            );
            assert_eq!((rc, out), (0, xpadded + 1));
            let (rc, out, loose) = cmp_pad(
                &format!("bs={bs} n={n} loose"),
                &d,
                bs,
                xpadded + 1024,
                true,
            );
            assert_eq!((rc, out), (0, xpadded + 1));
            assert_eq!(
                &tight[..xpadded + 1],
                &loose[..xpadded + 1],
                "max_buflen must not change what is written"
            );
            assert!(
                loose[xpadded + 1..].iter().all(|&x| x == 0xA5),
                "sodium_pad must not touch bytes past xpadded_len"
            );
            // the low end: nothing below xpadded_len - blocksize + 1 is written
            let lo = xpadded + 1 - bs;
            assert_eq!(&tight[..lo.min(n)], &d[..lo.min(n)]);
        }
    }

    // G1-096: unpadded_buflen = 0, blocksize = 16, max_buflen = 16 -> the loop
    // rewrites buf[0..15].
    let (rc, out, buf) = cmp_pad("n=0 bs=16 max=16", &[], 16, 16, true);
    assert_eq!((rc, out), (0, 16));
    assert_eq!(buf[0], 0x80);
    assert!(buf[1..16].iter().all(|&x| x == 0));
    assert_eq!(&buf[16..], &canary(24)[16..]);

    // randomised sweep
    for _ in 0..400 {
        let bs = *rng.pick(&[1usize, 2, 3, 4, 5, 7, 8, 15, 16, 17, 31, 32, 64, 100, 128]);
        let n = rng.below(200);
        let d = rng.bytes(n);
        let want = n - n % bs + bs;
        let extra = rng.below(64);
        cmp_pad(&format!("rand bs={bs} n={n}"), &d, bs, want + extra, rng.bool());
    }
}

/// CONFIGS G1-097, G1-098, G1-099, G1-100, G1-101, G1-102.
#[test]
fn unpad_valid() {
    setup();
    let mut rng = Rng::new(0x1009);

    // G1-097: blocksize = 1.
    for &n in &[1usize, 2, 64] {
        let mut buf = rng.bytes(n);
        buf[n - 1] = 0x80;
        let (rc, out) = cmp_unpad(&format!("bs=1 n={n}"), &buf, 1);
        assert_eq!((rc, out), (0, n - 1));
    }

    // G1-098: pad_len = 0.
    let mut buf = rng.bytes(16);
    buf[15] = 0x80;
    let (rc, out) = cmp_unpad("bs=16 pad_len=0", &buf, 16);
    assert_eq!((rc, out), (0, 15));

    // G1-099: pad_len = 15 (maximum).
    let mut buf = vec![0u8; 16];
    buf[0] = 0x80;
    let (rc, out) = cmp_unpad("bs=16 pad_len=15", &buf, 16);
    assert_eq!((rc, out), (0, 0));

    // G1-100: padded_buflen == blocksize exactly.
    for &bs in &[1usize, 2, 3, 16, 17, 64] {
        let mut buf = vec![0u8; bs];
        buf[bs - 1] = 0x80;
        let (rc, out) = cmp_unpad(&format!("padded_buflen == bs = {bs}"), &buf, bs);
        assert_eq!((rc, out), (0, bs - 1));
    }

    // G1-101: padded_buflen much larger than blocksize.
    let mut buf = rng.bytes(1024);
    buf[1023] = 0x80;
    let (rc, out) = cmp_unpad("n=1024 bs=16", &buf, 16);
    assert_eq!((rc, out), (0, 1023));
    for pad in 0..16usize {
        let mut buf = rng.bytes(1024);
        buf[1023 - pad] = 0x80;
        for i in (1024 - pad)..1024 {
            buf[i] = 0;
        }
        let (rc, out) = cmp_unpad(&format!("n=1024 bs=16 pad={pad}"), &buf, 16);
        assert_eq!((rc, out), (0, 1023 - pad));
    }

    // G1-102: unpad with a blocksize different from the one used to pad.
    let cpad = sym::<Pad>(c_lib(), "sodium_pad");
    for (pad_bs, unpad_bs, n) in [(16usize, 64usize, 32usize), (16, 32, 32), (4, 16, 32), (64, 16, 128)] {
        let d = rng.bytes(n);
        let mut buf = canary(n + pad_bs + 8);
        buf[..n].copy_from_slice(&d);
        let mut plen = 0usize;
        let rc = unsafe {
            cpad(
                &raw mut plen,
                buf.as_mut_ptr(),
                n,
                pad_bs,
                n + pad_bs + 8,
            )
        };
        assert_eq!(rc, 0);
        buf.truncate(plen);
        let (rc, out) = cmp_unpad(
            &format!("pad bs={pad_bs} then unpad bs={unpad_bs}"),
            &buf,
            unpad_bs,
        );
        // whether it succeeds depends on where the barrier sits; C and Rust
        // must simply agree (checked inside cmp_unpad).
        let _ = (rc, out);
    }

    // randomised sweep over well-formed and near-miss buffers
    for _ in 0..2000 {
        let bs = *rng.pick(&[1usize, 2, 3, 4, 8, 15, 16, 17, 32, 64]);
        let n = rng.range(bs, bs + 200);
        let mut buf = rng.bytes(n);
        match rng.below(4) {
            0 => {
                // well-formed padding
                let pad = rng.below(bs);
                if pad < n {
                    buf[n - 1 - pad] = 0x80;
                    for i in (n - pad)..n {
                        buf[i] = 0;
                    }
                }
            }
            1 => {
                for i in (n - bs)..n {
                    buf[i] = 0;
                }
            }
            2 => {
                for i in (n - bs)..n {
                    buf[i] = 0x80;
                }
            }
            _ => {}
        }
        cmp_unpad(&format!("rand bs={bs} n={n}"), &buf, bs);
    }
}

/// CONFIGS G1-103 — `sodium_pad` + `sodium_unpad` round-trip matrix.
#[test]
fn pad_unpad_roundtrip() {
    setup();
    let mut rng = Rng::new(0x100a);
    let (cp, rp) = pair::<Pad>("sodium_pad");
    let (cu, ru) = pair::<Unpad>("sodium_unpad");

    for &bs in &[1usize, 2, 3, 16, 17, 64] {
        let mut lens = vec![0usize, 1, 1000];
        if bs >= 1 {
            lens.push(bs - 1);
        }
        lens.push(bs);
        lens.push(bs + 1);
        for &n in &lens {
            for _ in 0..4 {
                let d = rng.bytes(n);
                let cap = n + bs + 16;
                let mut b1 = canary(cap);
                let mut b2 = canary(cap);
                b1[..n].copy_from_slice(&d);
                b2[..n].copy_from_slice(&d);
                let mut p1 = 0usize;
                let mut p2 = 0usize;
                let (ra, rr) = unsafe {
                    (
                        cp(&raw mut p1, b1.as_mut_ptr(), n, bs, cap),
                        rp(&raw mut p2, b2.as_mut_ptr(), n, bs, cap),
                    )
                };
                eq_i32(&format!("pad(bs={bs}, n={n}) rc"), ra, rr);
                eq_usize(&format!("pad(bs={bs}, n={n}) len"), p1, p2);
                eq_bytes(&format!("pad(bs={bs}, n={n})"), &b1, &b2);
                assert_eq!(ra, 0);

                let mut u1 = 0usize;
                let mut u2 = 0usize;
                let (ra, rr) = unsafe {
                    (
                        cu(&raw mut u1, b1.as_ptr(), p1, bs),
                        ru(&raw mut u2, b2.as_ptr(), p2, bs),
                    )
                };
                eq_i32(&format!("unpad(bs={bs}, n={n}) rc"), ra, rr);
                eq_usize(&format!("unpad(bs={bs}, n={n}) len"), u1, u2);
                assert_eq!((ra, u1), (0, n), "round-trip must recover the length");
                assert_eq!(&b1[..n], &d[..], "round-trip must preserve the data");
            }
        }
    }
}

// ===========================================================================
// sodium_memcmp / sodium_compare / sodium_is_zero
// ===========================================================================

/// CONFIGS G1-104, G1-105.
#[test]
fn memcmp_valid() {
    setup();
    let mut rng = Rng::new(0x100b);
    let (c, r) = pair::<Cmp3>("sodium_memcmp");
    let go = |a: &[u8], b: &[u8], len: usize, what: &str| -> i32 {
        let (x, y) = unsafe { (c(a.as_ptr(), b.as_ptr(), len), r(a.as_ptr(), b.as_ptr(), len)) };
        eq_i32(&format!("sodium_memcmp({what})"), x, y);
        x
    };

    // G1-104: len = 0 -> 0 regardless of contents.
    let a = rng.bytes(8);
    let b = rng.bytes(8);
    assert_eq!(go(&a, &b, 0, "len=0"), 0);

    for &len in &[1usize, 8, 16, 32, 33] {
        for _ in 0..8 {
            let x = rng.bytes(len);
            assert_eq!(go(&x, &x, len, "equal"), 0);
            // differ at each single byte position
            for i in 0..len {
                let mut y = x.clone();
                y[i] ^= 1 << rng.below(8);
                assert_eq!(go(&x, &y, len, &format!("differ at {i}")), -1);
            }
            // differ in every byte
            let y: Vec<u8> = x.iter().map(|v| !v).collect();
            assert_eq!(go(&x, &y, len, "all differ"), -1);
            // fully random pairs
            let y = rng.bytes(len);
            let want = if x == y { 0 } else { -1 };
            assert_eq!(go(&x, &y, len, "random"), want);
        }
        // G1-105
        assert_eq!(go(&vec![0u8; len], &vec![0u8; len], len, "zero"), 0);
        assert_eq!(go(&vec![0xffu8; len], &vec![0xffu8; len], len, "ff"), 0);
        assert_eq!(go(&vec![0u8; len], &vec![0xffu8; len], len, "zero vs ff"), -1);
    }
}

/// CONFIGS G1-106, G1-107, G1-108, G1-109, G1-110.
#[test]
fn compare_valid() {
    setup();
    let mut rng = Rng::new(0x100c);
    let (c, r) = pair::<Cmp3>("sodium_compare");
    let go = |a: &[u8], b: &[u8], len: usize, what: &str| -> i32 {
        let (x, y) = unsafe { (c(a.as_ptr(), b.as_ptr(), len), r(a.as_ptr(), b.as_ptr(), len)) };
        eq_i32(&format!("sodium_compare({what})"), x, y);
        x
    };

    // G1-106: len = 0.
    let a = rng.bytes(8);
    let b = rng.bytes(8);
    assert_eq!(go(&a, &b, 0, "len=0"), 0);

    // G1-107.
    assert_eq!(go(&[0], &[1], 1, "0 vs 1"), -1);
    assert_eq!(go(&[1], &[0], 1, "1 vs 0"), 1);
    assert_eq!(go(&[7], &[7], 1, "7 vs 7"), 0);
    for x in 0..=255u16 {
        for y in 0..=255u16 {
            let want = (x as i32).cmp(&(y as i32)) as i32;
            let want = match want {
                _ if x < y => -1,
                _ if x > y => 1,
                _ => 0,
            };
            assert_eq!(go(&[x as u8], &[y as u8], 1, "exhaustive len=1"), want);
        }
    }

    // G1-108: little-endian ordering — index len-1 is most significant.
    assert_eq!(go(&[0xff, 0x00], &[0x00, 0x01], 2, "LE"), -1);
    assert_eq!(go(&[0x00, 0x01], &[0xff, 0x00], 2, "LE rev"), 1);

    // G1-109/110.
    for &len in &[8usize, 16, 32, 33] {
        assert_eq!(go(&vec![0u8; len], &vec![0xffu8; len], len, "zero<ff"), -1);
        assert_eq!(go(&vec![0xffu8; len], &vec![0u8; len], len, "ff>zero"), 1);
        assert_eq!(go(&vec![0x5au8; len], &vec![0x5au8; len], len, "eq"), 0);
        // difference in the least-significant byte only (index 0)
        for _ in 0..8 {
            let x = rng.bytes(len);
            let mut y = x.clone();
            y[0] = y[0].wrapping_add(1);
            let want = if y[0] > x[0] { -1 } else { 1 };
            assert_eq!(go(&x, &y, len, "lsb only"), want);
        }
        // fully random, compared against a little-endian reference
        for _ in 0..200 {
            let x = rng.bytes(len);
            let y = rng.bytes(len);
            let mut want = 0i32;
            for i in (0..len).rev() {
                if x[i] != y[i] {
                    want = if x[i] < y[i] { -1 } else { 1 };
                    break;
                }
            }
            assert_eq!(go(&x, &y, len, "random LE"), want);
        }
    }
}

/// CONFIGS G1-111.
#[test]
fn is_zero_valid() {
    setup();
    let mut rng = Rng::new(0x100d);
    let (c, r) = pair::<IsZero>("sodium_is_zero");
    let go = |n: &[u8], len: usize, what: &str| -> i32 {
        let (x, y) = unsafe { (c(n.as_ptr(), len), r(n.as_ptr(), len)) };
        eq_i32(&format!("sodium_is_zero({what})"), x, y);
        x
    };

    let junk = rng.bytes(64);
    assert_eq!(go(&junk, 0, "nlen=0"), 1);
    for &n in &[1usize, 8, 16, 32, 33, 64, 100] {
        assert_eq!(go(&vec![0u8; n], n, "all zero"), 1);
        for pos in [0usize, n / 2, n - 1] {
            let mut v = vec![0u8; n];
            v[pos] = 1;
            assert_eq!(go(&v, n, &format!("one at {pos}")), 0);
            v[pos] = 0x80;
            assert_eq!(go(&v, n, &format!("0x80 at {pos}")), 0);
            v[pos] = 0xff;
            assert_eq!(go(&v, n, &format!("0xff at {pos}")), 0);
        }
        for _ in 0..32 {
            let v = rng.bytes(n);
            let want = if v.iter().all(|&x| x == 0) { 1 } else { 0 };
            assert_eq!(go(&v, n, "random"), want);
        }
    }
}

// ===========================================================================
// sodium_increment / sodium_add / sodium_sub
// ===========================================================================

/// CONFIGS G1-112, G1-113, G1-114, G1-115, G1-116.
#[test]
fn increment_valid() {
    setup();
    let mut rng = Rng::new(0x100e);
    let (c, r) = pair::<Incr>("sodium_increment");
    let go = |v: &[u8], len: usize, what: &str| -> Vec<u8> {
        let mut a = v.to_vec();
        let mut b = v.to_vec();
        unsafe {
            c(a.as_mut_ptr(), len);
            r(b.as_mut_ptr(), len);
        }
        eq_bytes(&format!("sodium_increment({what})"), &a, &b);
        a
    };

    // G1-112: nlen = 0 -> no-op.
    let v = rng.bytes(8);
    assert_eq!(go(&v, 0, "nlen=0"), v);

    // G1-113.
    assert_eq!(go(&[0x00], 1, "00"), vec![0x01]);
    assert_eq!(go(&[0xfe], 1, "fe"), vec![0xff]);
    assert_eq!(go(&[0xff], 1, "ff wrap"), vec![0x00]);
    for x in 0..=255u16 {
        assert_eq!(go(&[x as u8], 1, "exhaustive"), vec![(x as u8).wrapping_add(1)]);
    }

    // G1-114/115/116: the lengths that would take asm paths in an asm build.
    for &n in &[1usize, 2, 4, 7, 8, 12, 16, 24, 32, 33, 64] {
        assert_eq!(go(&vec![0xffu8; n], n, "all ff"), vec![0u8; n]);
        let mut v = vec![0u8; n];
        v[0] = 0xff;
        let mut want = vec![0u8; n];
        if n > 1 {
            want[1] = 1;
        }
        assert_eq!(go(&v, n, "ff,00,..."), want);
        // partial carry stops at the first non-0xff byte
        let mut v = vec![0u8; n];
        v[0] = 0xff;
        if n > 1 {
            v[1] = 0xff;
        }
        let got = go(&v, n, "ff,ff,00,...");
        assert_eq!(got[0], 0);
        if n > 1 {
            assert_eq!(got[1], 0);
        }
        if n > 2 {
            assert_eq!(got[2], 1);
        }
        // randomised, checked against a little-endian bignum reference
        for _ in 0..64 {
            let v = rng.bytes(n);
            let got = go(&v, n, "random");
            let mut want = v.clone();
            let mut carry = 1u16;
            for i in 0..n {
                carry += want[i] as u16;
                want[i] = carry as u8;
                carry >>= 8;
            }
            assert_eq!(got, want, "sodium_increment reference mismatch");
        }
    }
}

/// CONFIGS G1-117, G1-118, G1-119, G1-120, G1-121.
#[test]
fn add_valid() {
    setup();
    let mut rng = Rng::new(0x100f);
    let (c, r) = pair::<AddSub>("sodium_add");
    let go = |a0: &[u8], b0: &[u8], len: usize, what: &str| -> Vec<u8> {
        let mut a = a0.to_vec();
        let mut b = a0.to_vec();
        unsafe {
            c(a.as_mut_ptr(), b0.as_ptr(), len);
            r(b.as_mut_ptr(), b0.as_ptr(), len);
        }
        eq_bytes(&format!("sodium_add({what})"), &a, &b);
        a
    };

    // G1-117.
    let x = rng.bytes(8);
    let y = rng.bytes(8);
    assert_eq!(go(&x, &y, 0, "len=0"), x);
    assert_eq!(go(&[0xff], &[0x01], 1, "ff+01"), vec![0x00]);
    for p in 0..=255u16 {
        for q in 0..=255u16 {
            assert_eq!(
                go(&[p as u8], &[q as u8], 1, "exhaustive len=1"),
                vec![(p as u8).wrapping_add(q as u8)]
            );
        }
    }

    // G1-118/119/120.
    for &n in &[1usize, 2, 8, 12, 16, 24, 32, 33, 64] {
        let mut one = vec![0u8; n];
        one[0] = 1;
        assert_eq!(go(&vec![0xffu8; n], &one, n, "ff + 1"), vec![0u8; n]);
        let mut want = vec![0xffu8; n];
        want[0] = 0xfe;
        assert_eq!(go(&vec![0xffu8; n], &vec![0xffu8; n], n, "ff + ff"), want);
        // randomised against a little-endian bignum reference
        for _ in 0..64 {
            let p = rng.bytes(n);
            let q = rng.bytes(n);
            let got = go(&p, &q, n, "random");
            let mut want = vec![0u8; n];
            let mut carry = 0u16;
            for i in 0..n {
                carry += p[i] as u16 + q[i] as u16;
                want[i] = carry as u8;
                carry >>= 8;
            }
            assert_eq!(got, want, "sodium_add reference mismatch");
        }
    }

    // G1-121: aliasing a == b (doubling in place).
    for &n in &[1usize, 8, 16, 32] {
        for fill in [0x80u8, 0xff, 0x01, 0x7f] {
            let mut a = vec![fill; n];
            let mut b = vec![fill; n];
            unsafe {
                c(a.as_mut_ptr(), a.as_ptr(), n);
                r(b.as_mut_ptr(), b.as_ptr(), n);
            }
            eq_bytes(&format!("sodium_add aliased n={n} fill={fill:#x}"), &a, &b);
        }
        for _ in 0..32 {
            let v = rng.bytes(n);
            let mut a = v.clone();
            let mut b = v.clone();
            unsafe {
                c(a.as_mut_ptr(), a.as_ptr(), n);
                r(b.as_mut_ptr(), b.as_ptr(), n);
            }
            eq_bytes(&format!("sodium_add aliased random n={n}"), &a, &b);
        }
    }
}

/// CONFIGS G1-122, G1-123, G1-124, G1-125.
#[test]
fn sub_valid() {
    setup();
    let mut rng = Rng::new(0x1010);
    let (c, r) = pair::<AddSub>("sodium_sub");
    let go = |a0: &[u8], b0: &[u8], len: usize, what: &str| -> Vec<u8> {
        let mut a = a0.to_vec();
        let mut b = a0.to_vec();
        unsafe {
            c(a.as_mut_ptr(), b0.as_ptr(), len);
            r(b.as_mut_ptr(), b0.as_ptr(), len);
        }
        eq_bytes(&format!("sodium_sub({what})"), &a, &b);
        a
    };

    // G1-122.
    let x = rng.bytes(8);
    let y = rng.bytes(8);
    assert_eq!(go(&x, &y, 0, "len=0"), x);
    assert_eq!(go(&[0x00], &[0x01], 1, "00-01"), vec![0xff]);
    for p in 0..=255u16 {
        for q in 0..=255u16 {
            assert_eq!(
                go(&[p as u8], &[q as u8], 1, "exhaustive len=1"),
                vec![(p as u8).wrapping_sub(q as u8)]
            );
        }
    }

    // G1-123/124/125 — including len = 64 (the would-be asm size).
    for &n in &[1usize, 2, 8, 12, 16, 24, 32, 33, 64] {
        let mut one = vec![0u8; n];
        one[0] = 1;
        assert_eq!(go(&vec![0u8; n], &one, n, "0 - 1"), vec![0xffu8; n]);
        let v = rng.bytes(n);
        assert_eq!(go(&v, &v, n, "a - a"), vec![0u8; n]);
        for _ in 0..64 {
            let p = rng.bytes(n);
            let q = rng.bytes(n);
            let got = go(&p, &q, n, "random");
            let mut want = vec![0u8; n];
            let mut borrow = 0i32;
            for i in 0..n {
                let d = p[i] as i32 - q[i] as i32 - borrow;
                want[i] = d as u8;
                borrow = if d < 0 { 1 } else { 0 };
            }
            assert_eq!(got, want, "sodium_sub reference mismatch");
        }
    }
    // aliasing a == b
    for &n in &[1usize, 8, 32, 64] {
        let v = rng.bytes(n);
        let mut a = v.clone();
        let mut b = v.clone();
        unsafe {
            c(a.as_mut_ptr(), a.as_ptr(), n);
            r(b.as_mut_ptr(), b.as_ptr(), n);
        }
        eq_bytes(&format!("sodium_sub aliased n={n}"), &a, &b);
        assert_eq!(a, vec![0u8; n]);
    }
}

// ===========================================================================
// sodium_memzero / sodium_stackzero
// ===========================================================================

/// CONFIGS G1-126, G1-127.
#[test]
fn memzero_and_stackzero() {
    setup();
    let mut rng = Rng::new(0x1011);
    let (cz, rz) = pair::<Memzero>("sodium_memzero");

    // G1-126: len = 0 with a NULL pointer must be a no-op.
    unsafe {
        cz(ptr::null_mut(), 0);
        rz(ptr::null_mut(), 0);
    }
    for &n in &[0usize, 1, 2, 7, 8, 15, 16, 32, 63, 64, 100, 4096] {
        for _ in 0..4 {
            let src = rng.bytes(n + 8);
            let mut a = src.clone();
            let mut b = src.clone();
            unsafe {
                cz(a.as_mut_ptr() as *mut c_void, n);
                rz(b.as_mut_ptr() as *mut c_void, n);
            }
            eq_bytes(&format!("sodium_memzero(len={n})"), &a, &b);
            assert!(a[..n].iter().all(|&x| x == 0), "the first {n} bytes must be zero");
            assert_eq!(&a[n..], &src[n..], "sodium_memzero must not overrun");
        }
        // sub-range in the middle of a buffer
        let src = rng.bytes(n + 16);
        let mut a = src.clone();
        let mut b = src.clone();
        unsafe {
            cz(a.as_mut_ptr().add(8) as *mut c_void, n);
            rz(b.as_mut_ptr().add(8) as *mut c_void, n);
        }
        eq_bytes(&format!("sodium_memzero(offset 8, len={n})"), &a, &b);
        assert_eq!(&a[..8], &src[..8]);
    }

    // G1-127: sodium_stackzero has an empty body — every length is a no-op.
    let (cs, rs) = pair::<Stackzero>("sodium_stackzero");
    for &n in &[0usize, 1, 512, 4096, 1 << 20, usize::MAX] {
        unsafe {
            cs(n);
            rs(n);
        }
    }
}

// ===========================================================================
// sodium_malloc / sodium_allocarray / sodium_free
// ===========================================================================

/// CONFIGS G1-128, G1-129, G1-130, G1-131, G1-132, G1-133, G1-134.
#[test]
fn malloc_allocarray_free() {
    setup();
    let (cm, rm) = pair::<MallocFn>("sodium_malloc");
    let (ca, ra) = pair::<AllocArray>("sodium_allocarray");
    let (cf, rf) = pair::<FreeFn>("sodium_free");

    // G1-128: size = 0 -> non-NULL (malloc(1)), nothing written.
    for f in [cm, rm] {
        let p = unsafe { f(0) };
        assert!(!p.is_null(), "sodium_malloc(0) must be non-NULL");
    }
    let (p1, p2) = unsafe { (cm(0), rm(0)) };
    assert!(!p1.is_null() && !p2.is_null());
    unsafe {
        cf(p1);
        rf(p2);
    }

    // G1-129: every byte is pre-filled with GARBAGE_VALUE = 0xdb.
    for &n in &[1usize, 2, 16, 17, 32, 100, 4096, 65536] {
        let (p1, p2) = unsafe { (cm(n), rm(n)) };
        assert!(!p1.is_null() && !p2.is_null(), "sodium_malloc({n})");
        let a = unsafe { std::slice::from_raw_parts(p1 as *const u8, n) };
        let b = unsafe { std::slice::from_raw_parts(p2 as *const u8, n) };
        eq_bytes(&format!("sodium_malloc({n}) contents"), a, b);
        assert!(
            a.iter().all(|&x| x == 0xdb),
            "sodium_malloc({n}) must fill with 0xdb, got {}",
            hex(&a[..a.len().min(32)])
        );
        unsafe {
            cf(p1);
            rf(p2);
        }
    }

    // G1-130: no guard page / canary / page rounding in this build — the
    // returned pointer is a plain malloc pointer, so *reading* one byte past
    // the requested region does not fault and sodium_free performs no canary
    // check. (Only a read probe: writing past the region would corrupt the
    // heap for real, since there is no redzone.)
    for &n in &[1usize, 16, 32] {
        let (p1, p2) = unsafe { (cm(n), rm(n)) };
        let x = unsafe { *(p1 as *const u8).add(n) };
        let y = unsafe { *(p2 as *const u8).add(n) };
        let _ = (x, y); // whatever the allocator left there; must not fault
        // the returned pointer is not page-aligned nor canary-prefixed
        assert!(
            (p1 as usize) % 4096 != 0 || n >= 4096,
            "sodium_malloc must be a plain malloc pointer"
        );
        unsafe {
            cf(p1);
            rf(p2);
        }
    }

    // G1-131: count = 0 skips the overflow check.
    for (count, size) in [(0usize, 0usize), (0, 1000), (0, usize::MAX)] {
        let (p1, p2) = unsafe { (ca(count, size), ra(count, size)) };
        assert!(
            !p1.is_null() && !p2.is_null(),
            "sodium_allocarray({count}, {size}) must be non-NULL"
        );
        unsafe {
            cf(p1);
            rf(p2);
        }
    }

    // G1-132.
    for (count, size) in [(4usize, 8usize), (1, 1), (1000, 32), (7, 13), (2, 3)] {
        let (p1, p2) = unsafe { (ca(count, size), ra(count, size)) };
        assert!(!p1.is_null() && !p2.is_null());
        let n = count * size;
        let a = unsafe { std::slice::from_raw_parts(p1 as *const u8, n) };
        let b = unsafe { std::slice::from_raw_parts(p2 as *const u8, n) };
        eq_bytes(&format!("sodium_allocarray({count},{size})"), a, b);
        assert!(a.iter().all(|&x| x == 0xdb));
        unsafe {
            cf(p1);
            rf(p2);
        }
    }

    // G1-133: passes the overflow check, then malloc fails -> NULL.
    let count = 2usize;
    let size = usize::MAX / 2 - 1;
    assert!(size < usize::MAX / count, "test bug: overflow check would trip");
    let (p1, p2) = unsafe { (ca(count, size), ra(count, size)) };
    assert!(p1.is_null(), "C sodium_allocarray(2, SIZE_MAX/2-1) must be NULL");
    assert!(p2.is_null(), "Rust sodium_allocarray(2, SIZE_MAX/2-1) must be NULL");

    // G1-134: sodium_free is a plain free; NULL is a no-op.
    unsafe {
        cf(ptr::null_mut());
        rf(ptr::null_mut());
    }
    for n in [0usize, 1, 64] {
        let (p1, p2) = unsafe { (cm(n), rm(n)) };
        unsafe {
            cf(p1);
            rf(p2);
        }
    }
    let (p1, p2) = unsafe { (ca(4, 8), ra(4, 8)) };
    unsafe {
        cf(p1);
        rf(p2);
    }
}

// ===========================================================================
// sodium_mlock / munlock / mprotect_*
// ===========================================================================

/// CONFIGS G1-135, G1-136.
#[test]
fn mlock_munlock_mprotect() {
    setup();
    let (cl, rl) = pair::<Mlock>("sodium_mlock");
    let (cu, ru) = pair::<Mlock>("sodium_munlock");
    let (cm, rm) = pair::<MallocFn>("sodium_malloc");
    let (cf, rf) = pair::<FreeFn>("sodium_free");
    let mut rng = Rng::new(0x1012);

    // G1-135: always -1 / ENOSYS; munlock zeroes first.
    let mut stack = [0x5au8; 64];
    let mut heap = rng.bytes(64);
    for &len in &[0usize, 1, 16, 64] {
        for (f, what) in [(cl, "C mlock"), (rl, "Rust mlock")] {
            let before = stack;
            set_sentinel();
            let rc = unsafe { f(stack.as_mut_ptr() as *mut c_void, len) };
            assert_eq!(rc, -1, "{what} must return -1");
            assert_eq!(errno(), 38, "{what} must set errno = ENOSYS");
            assert_eq!(stack, before, "{what} must not modify the buffer");
        }
        for (f, what) in [(cu, "C munlock"), (ru, "Rust munlock")] {
            rng.fill(&mut heap);
            let tail = heap[len..].to_vec();
            set_sentinel();
            let rc = unsafe { f(heap.as_mut_ptr() as *mut c_void, len) };
            assert_eq!(rc, -1, "{what} must return -1");
            assert_eq!(errno(), 38, "{what} must set errno = ENOSYS");
            assert!(
                heap[..len].iter().all(|&x| x == 0),
                "{what} must zero the buffer BEFORE failing"
            );
            assert_eq!(&heap[len..], &tail[..], "{what} must not overrun");
        }
    }

    // G1-136: full mprotect lifecycle on a sodium_malloc pointer; the memory
    // stays readable and writable throughout.
    for (mfn, ffn, tag) in [(cm, cf, "c"), (rm, rf, "r")] {
        let p = unsafe { mfn(32) };
        assert!(!p.is_null());
        for name in [
            "sodium_mprotect_noaccess",
            "sodium_mprotect_readonly",
            "sodium_mprotect_readwrite",
        ] {
            let (c, r) = pair::<Mprotect>(name);
            let f = if tag == "c" { c } else { r };
            set_sentinel();
            let rc = unsafe { f(p) };
            assert_eq!(rc, -1, "{name} must return -1");
            assert_eq!(errno(), 38, "{name} must set errno = ENOSYS");
            // still readable AND writable
            unsafe {
                let q = p as *mut u8;
                let v = *q;
                *q = v ^ 0xff;
                assert_eq!(*q, v ^ 0xff, "{name} must not actually protect anything");
                *q = v;
            }
        }
        unsafe { ffn(p) };
    }
    // ... and on a non-sodium pointer
    let mut plain = [0u8; 32];
    for name in [
        "sodium_mprotect_noaccess",
        "sodium_mprotect_readonly",
        "sodium_mprotect_readwrite",
    ] {
        let (c, r) = pair::<Mprotect>(name);
        for (f, which) in [(c, "C"), (r, "Rust")] {
            set_sentinel();
            let rc = unsafe { f(plain.as_mut_ptr() as *mut c_void) };
            assert_eq!(rc, -1, "{which} {name} must return -1");
            assert_eq!(errno(), 38, "{which} {name} must set errno = ENOSYS");
        }
    }
}

// ===========================================================================
// sodium_init / sodium_set_misuse_handler / runtime / version
// ===========================================================================

/// CONFIGS G1-138 (second and subsequent `sodium_init()` calls), G1-140,
/// G1-141 (`sodium_set_misuse_handler` return value, install and clear).
/// G1-137 (the FIRST call in a process) and G1-139 (using the API without
/// `sodium_init()`) need a virgin process and live in `fresh_process_rows`.
#[test]
fn init_and_misuse_handler() {
    setup();
    let (ci, ri) = pair::<IntFn>("sodium_init");
    for _ in 0..4 {
        let (a, b) = unsafe { (ci(), ri()) };
        eq_i32("sodium_init() repeat", a, b);
        assert_eq!(a, 1, "sodium_init() must be idempotent and return 1");
    }
    // the crit-section stubs always succeed, so -1 is unreachable
    let (ce, re) = pair::<IntFn>("sodium_crit_enter");
    let (cx, rx) = pair::<IntFn>("sodium_crit_leave");
    for _ in 0..4 {
        let (a, b) = unsafe { (ce(), re()) };
        eq_i32("sodium_crit_enter()", a, b);
        assert_eq!(a, 0);
        let (a, b) = unsafe { (cx(), rx()) };
        eq_i32("sodium_crit_leave()", a, b);
        assert_eq!(a, 0);
    }

    // G1-140/141: installing and clearing a handler always returns 0. (That
    // the handler actually runs before `abort()` is verified out of process by
    // t11_sodium_errors.)
    let (cs, rs) = pair::<SetMisuseFn>("sodium_set_misuse_handler");
    unsafe extern "C" fn dummy_handler() {}
    for h in [
        Some(dummy_handler as unsafe extern "C" fn()),
        None,
        Some(dummy_handler as unsafe extern "C" fn()),
        None,
    ] {
        let (a, b) = unsafe { (cs(h), rs(h)) };
        eq_i32("sodium_set_misuse_handler()", a, b);
        assert_eq!(a, 0);
    }
}

/// CONFIGS G1-142 (after `sodium_init()`), G1-143.
#[test]
fn runtime_features_all_zero() {
    setup();
    for name in [
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
        let (c, r) = pair::<IntFn>(name);
        for _ in 0..3 {
            let (a, b) = unsafe { (c(), r()) };
            eq_i32(name, a, b);
            assert_eq!(a, 0, "{name} must return 0 in this build");
        }
    }
    // G1-143: _sodium_runtime_get_cpu_features() returns -1 every time.
    let (c, r) = pair::<IntFn>("_sodium_runtime_get_cpu_features");
    for _ in 0..5 {
        let (a, b) = unsafe { (c(), r()) };
        eq_i32("_sodium_runtime_get_cpu_features()", a, b);
        assert_eq!(a, -1);
    }
    // ... and the feature flags are still all zero afterwards
    for name in ["sodium_runtime_has_sse2", "sodium_runtime_has_rdrand"] {
        let (c, r) = pair::<IntFn>(name);
        let (a, b) = unsafe { (c(), r()) };
        eq_i32(name, a, b);
        assert_eq!(a, 0);
    }
}

/// CONFIGS G1-144, G1-145, G1-146, G1-147.
#[test]
fn version_and_seedbytes() {
    setup();
    let (c, r) = pair::<StrFn>("sodium_version_string");
    let (a, b) = unsafe { (cstr(c()), cstr(r())) };
    assert_eq!(a, "1.0.23", "sodium_version_string()");
    assert_eq!(a, b, "sodium_version_string(): C {a} vs Rust {b}");
    // stable across calls
    for _ in 0..3 {
        assert_eq!(unsafe { cstr(c()) }, unsafe { cstr(r()) });
    }
    for (name, want) in [
        ("sodium_library_version_major", 30),
        ("sodium_library_version_minor", 0),
        ("sodium_library_minimal", 0),
    ] {
        let (c, r) = pair::<IntFn>(name);
        let (a, b) = unsafe { (c(), r()) };
        eq_i32(name, a, b);
        assert_eq!(a, want, "{name}");
    }
    // G1-147
    let (c, r) = pair::<SizeFn>("randombytes_seedbytes");
    let (a, b) = unsafe { (c(), r()) };
    eq_usize("randombytes_seedbytes()", a, b);
    assert_eq!(a, 32);
}

// ===========================================================================
// randombytes rows that only need the harness (deterministic) implementation
//
// These all consume the process-global RNG stream, so they live in ONE #[test]
// (the #[test]s of a binary run in parallel threads).
// ===========================================================================

/// CONFIGS G1-154 (via the harness impl, whose `uniform` is NULL), G1-156,
/// G1-157, G1-158, G1-159, G1-160, G1-161, G1-162 (chunking is exercised by
/// `randombytes_buf` sizes), G1-167.
#[test]
fn randombytes_rng_dependent() {
    setup();
    let (cu, ru) = pair::<RbUniform>("randombytes_uniform");
    let (cr, rr) = pair::<RbRandom>("randombytes_random");
    let (cb, rb) = pair::<RbBuf>("randombytes_buf");
    let (cn, rn) = pair::<RbNacl>("randombytes");

    // G1-156: upper_bound 0 and 1 -> 0 without consuming randomness. Prove
    // "without consuming" by checking that the next `randombytes_random()` is
    // the same as if uniform had not been called at all.
    for ub in [0u32, 1] {
        reset_rngs(0x5001);
        let base = unsafe { cr() };
        reset_rngs(0x5001);
        let u = unsafe { cu(ub) };
        let after = unsafe { cr() };
        assert_eq!(u, 0, "randombytes_uniform({ub}) must be 0");
        assert_eq!(after, base, "randombytes_uniform({ub}) must not draw");
        reset_rngs(0x5001);
        let u2 = unsafe { ru(ub) };
        let after2 = unsafe { rr() };
        assert_eq!((u2, after2), (u, after), "Rust randombytes_uniform({ub})");
    }

    // G1-157/158/159: the generic rejection-sampling path.
    let bounds: &[u32] = &[
        2, 3, 10, 256, 1000, 0x7fff_ffff, 0x8000_0001, 0x4000_0000, 0xffff_ffff, 0x8000_0000, 5, 7,
        100, 12345,
    ];
    for &ub in bounds {
        for seed in 0..24u64 {
            reset_rngs(0x6000 + seed);
            let a = unsafe { cu(ub) };
            reset_rngs(0x6000 + seed);
            let b = unsafe { ru(ub) };
            assert_eq!(a, b, "randombytes_uniform({ub}) seed={seed}: C {a} vs Rust {b}");
            assert!(a < ub, "randombytes_uniform({ub}) returned {a}");
        }
    }
    // powers of two never reject: exactly one draw is consumed
    for &ub in &[2u32, 4, 0x4000_0000, 0x8000_0000] {
        reset_rngs(0x7001);
        let raw = unsafe { cr() };
        reset_rngs(0x7001);
        let u = unsafe { cu(ub) };
        assert_eq!(u, raw % ub, "power-of-two bound must not reject");
        reset_rngs(0x7001);
        assert_eq!(unsafe { ru(ub) }, u);
    }

    // G1-160: randombytes_random returns the impl value unchanged.
    for seed in 0..16u64 {
        reset_rngs(0x8000 + seed);
        let a = unsafe { cr() };
        reset_rngs(0x8000 + seed);
        let b = unsafe { rr() };
        assert_eq!(a, b, "randombytes_random() seed={seed}");
    }

    // G1-161: size = 0 must not invoke impl->buf at all (again proved by the
    // RNG stream being untouched), even with a NULL buffer.
    reset_rngs(0x9001);
    unsafe { cb(ptr::null_mut(), 0) };
    let a = unsafe { cr() };
    reset_rngs(0x9001);
    unsafe { rb(ptr::null_mut(), 0) };
    let b = unsafe { rr() };
    reset_rngs(0x9001);
    let base = unsafe { cr() };
    assert_eq!(a, base, "randombytes_buf(_, 0) must not draw");
    assert_eq!(a, b);

    // G1-162: many sizes, including the 256-byte chunk boundary of the
    // sysrandom getrandom path.
    for &n in &[1usize, 2, 31, 32, 63, 64, 100, 255, 256, 257, 511, 512, 513, 1000, 4096] {
        for seed in 0..3u64 {
            let mut x = canary(n + 8);
            let mut y = canary(n + 8);
            reset_rngs(0xA000 + seed + n as u64);
            unsafe { cb(x.as_mut_ptr() as *mut c_void, n) };
            reset_rngs(0xA000 + seed + n as u64);
            unsafe { rb(y.as_mut_ptr() as *mut c_void, n) };
            eq_bytes(&format!("randombytes_buf({n})"), &x, &y);
            assert_eq!(&x[n..], &canary(n + 8)[n..], "randombytes_buf must not overrun");
        }
    }

    // G1-167: the NaCl-compatibility wrapper.
    reset_rngs(0xB001);
    let mut x = canary(8);
    unsafe { cn(x.as_mut_ptr(), 0) };
    assert_eq!(x, canary(8), "randombytes(_, 0) must write nothing");
    reset_rngs(0xB001);
    let mut y = canary(8);
    unsafe { rn(y.as_mut_ptr(), 0) };
    assert_eq!(y, canary(8));
    for &n in &[1u64, 8, 32, 33, 256, 1000] {
        let mut x = canary(n as usize + 8);
        let mut y = canary(n as usize + 8);
        reset_rngs(0xC000 + n);
        unsafe { cn(x.as_mut_ptr(), n) };
        reset_rngs(0xC000 + n);
        unsafe { rn(y.as_mut_ptr(), n) };
        eq_bytes(&format!("randombytes(_, {n})"), &x, &y);
        // and identical to randombytes_buf
        let mut z = canary(n as usize + 8);
        reset_rngs(0xC000 + n);
        unsafe { cb(z.as_mut_ptr() as *mut c_void, n as usize) };
        eq_bytes(&format!("randombytes vs randombytes_buf({n})"), &x, &z);
    }
}

/// CONFIGS G1-163, G1-164, G1-165, G1-166 — `randombytes_buf_deterministic`
/// never touches `implementation`, so it needs no RNG-state coordination.
#[test]
fn randombytes_buf_deterministic_valid() {
    setup();
    let (c, r) = pair::<RbDet>("randombytes_buf_deterministic");
    let mut rng = Rng::new(0x1013);

    let seeds: Vec<Vec<u8>> = {
        let mut v = vec![vec![0u8; 32], vec![0xffu8; 32]];
        v.push((0..32u8).collect());
        let mut s = vec![0x5au8; 32];
        v.push(s.clone());
        s[0] ^= 1; // differing in one bit
        v.push(s.clone());
        s[31] ^= 0x80;
        v.push(s);
        for _ in 0..8 {
            v.push(rng.bytes(32));
        }
        v
    };

    // G1-163/164/165: sizes and seeds.
    let sizes: &[usize] = &[0, 1, 2, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 255, 256, 1000];
    for seed in &seeds {
        let mut longest: Vec<u8> = Vec::new();
        for &n in sizes {
            let mut a = canary(n + 8);
            let mut b = canary(n + 8);
            unsafe {
                c(a.as_mut_ptr() as *mut c_void, n, seed.as_ptr());
                r(b.as_mut_ptr() as *mut c_void, n, seed.as_ptr());
            }
            eq_bytes(
                &format!("randombytes_buf_deterministic(n={n}, seed={})", hex(seed)),
                &a,
                &b,
            );
            assert_eq!(&a[n..], &canary(n + 8)[n..], "must not overrun");
            if n == 0 {
                assert_eq!(a, canary(8), "size = 0 must write nothing");
            }
            // reproducible byte-for-byte on a repeat call
            let mut a2 = canary(n + 8);
            unsafe { c(a2.as_mut_ptr() as *mut c_void, n, seed.as_ptr()) };
            assert_eq!(a, a2, "randombytes_buf_deterministic must be deterministic");
            if n > longest.len() {
                longest = a[..n].to_vec();
            }
        }
        // G1-163: the output for a smaller size is a strict PREFIX of the
        // output for a larger size.
        for &n in sizes {
            let mut a = canary(n);
            unsafe { c(a.as_mut_ptr() as *mut c_void, n, seed.as_ptr()) };
            assert_eq!(&a[..], &longest[..n], "shorter output must be a prefix");
        }
    }
    // distinct seeds give distinct keystreams
    let mut outs = Vec::new();
    for seed in &seeds {
        let mut a = vec![0u8; 64];
        unsafe { c(a.as_mut_ptr() as *mut c_void, 64, seed.as_ptr()) };
        outs.push(a);
    }
    for i in 0..outs.len() {
        for j in (i + 1)..outs.len() {
            assert_ne!(outs[i], outs[j], "distinct seeds must give distinct output");
        }
    }
    // G1-166: independent of the installed implementation and of sodium_init.
    // (The `set_implementation` half is covered by the `det/*` child rows.)
    let (ci, ri) = pair::<IntFn>("sodium_init");
    let seed = seeds[0].clone();
    let mut before = vec![0u8; 64];
    unsafe { c(before.as_mut_ptr() as *mut c_void, 64, seed.as_ptr()) };
    unsafe {
        ci();
        ri();
    }
    let mut after = vec![0u8; 64];
    unsafe { c(after.as_mut_ptr() as *mut c_void, 64, seed.as_ptr()) };
    assert_eq!(before, after, "sodium_init must not affect the DRG");
}

// ===========================================================================
// child-process rows: process-global RNG / init state
// ===========================================================================

// A deterministic RNG for the *custom* implementations installed by the child
// processes. Each child runs the identical sequence of operations, so the C
// child and the Rust child observe the identical stream.
static mut CH_STATE: u64 = 0;

fn ch_next() -> u64 {
    unsafe {
        let s = &mut *(&raw mut CH_STATE);
        *s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

unsafe extern "C" fn ch_name() -> *const i8 {
    b"difftest-child\0".as_ptr() as *const i8
}
unsafe extern "C" fn ch_random() -> u32 {
    (ch_next() >> 32) as u32
}
unsafe extern "C" fn ch_buf(p: *mut c_void, n: usize) {
    let mut i = 0usize;
    while i < n {
        let w = ch_next().to_le_bytes();
        let take = 8.min(n - i);
        unsafe { ptr::copy_nonoverlapping(w.as_ptr(), (p as *mut u8).add(i), take) };
        i += take;
    }
}
unsafe extern "C" fn ch_stir() {
    unsafe { *(&raw mut CH_STATE) = 0x1234_5678 };
}
unsafe extern "C" fn ch_close() -> i32 {
    4242
}
/// Distinctive `uniform` so that delegation (rather than the generic path) is
/// observable, including for `upper_bound` 0 and 1.
unsafe extern "C" fn ch_uniform(ub: u32) -> u32 {
    ub ^ 0xABCD_1234
}

static CH_FULL: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(ch_name),
    random: Some(ch_random),
    stir: Some(ch_stir),
    uniform: None,
    buf: Some(ch_buf),
    close: Some(ch_close),
};
static CH_STIR_NULL: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(ch_name),
    random: Some(ch_random),
    stir: None,
    uniform: None,
    buf: Some(ch_buf),
    close: Some(ch_close),
};
static CH_CLOSE_NULL: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(ch_name),
    random: Some(ch_random),
    stir: Some(ch_stir),
    uniform: None,
    buf: Some(ch_buf),
    close: None,
};
static CH_UNIFORM_SET: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(ch_name),
    random: Some(ch_random),
    stir: Some(ch_stir),
    uniform: Some(ch_uniform),
    buf: Some(ch_buf),
    close: Some(ch_close),
};
static CH_BUF_NULL: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(ch_name),
    random: Some(ch_random),
    stir: Some(ch_stir),
    uniform: None,
    buf: None,
    close: Some(ch_close),
};

const HAS_FNS: &[&str] = &[
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
];

/// Every configuration row that must observe a **virgin** library (nothing
/// initialised, no implementation installed) or that has to replace the global
/// `randombytes` implementation.
const CHILD_CASES: &[&str] = &[
    "fresh/init_sequence",      // G1-137, G1-138, G1-142 (before/after init)
    "fresh/no_init_apis",       // G1-139
    "fresh/impl_name_default",  // G1-148
    "impl/internal_name",       // G1-149
    "impl/sysrandom_name",      // G1-150
    "impl/null_reinstalls",     // G1-151
    "impl/stir_null",           // G1-152
    "impl/close_null",          // G1-153
    "impl/uniform_null",        // G1-154
    "impl/uniform_set",         // G1-155
    "impl/random_custom",       // G1-160
    "impl/buf_null_size0",      // G1-161
    "sys/buf_sizes",            // G1-162
    "sys/stir_close",           // G1-168, G1-169, G1-170, G1-174
    "det/independent_of_impl",  // G1-166
    "internal/close_after_use", // G1-171
    "internal/buf_zero_direct", // G1-172
    "internal/pool_refill",     // G1-173
];

#[test]
fn config_child() {
    let Some(tag) = child_tag() else {
        return; // parent: no-op
    };
    let lib = child_lib();
    // NOTE: deliberately no setup() — these rows are about the virgin state.
    let sset = sym::<SetImplFn>(lib, "randombytes_set_implementation");
    let name = sym::<StrFn>(lib, "randombytes_implementation_name");
    let rbuf = sym::<RbBuf>(lib, "randombytes_buf");
    let rrandom = sym::<RbRandom>(lib, "randombytes_random");
    let runiform = sym::<RbUniform>(lib, "randombytes_uniform");
    let rstir = sym::<RbVoid>(lib, "randombytes_stir");
    let rclose = sym::<IntFn>(lib, "randombytes_close");
    let sysimpl = sym::<*const RandombytesImpl>(lib, "randombytes_sysrandom_implementation");
    let intimpl = sym::<*const RandombytesImpl>(lib, "randombytes_internal_implementation");

    match tag.as_str() {
        // G1-137 / G1-138 / G1-142: the very first sodium_init() returns 0, all
        // later ones return 1, and every runtime_has_* is 0 both before and
        // after.
        "fresh/init_sequence" => {
            let init = sym::<IntFn>(lib, "sodium_init");
            let before: Vec<i32> = HAS_FNS
                .iter()
                .map(|n| unsafe { sym::<IntFn>(lib, n)() })
                .collect();
            let first = unsafe { init() };
            let after: Vec<i32> = HAS_FNS
                .iter()
                .map(|n| unsafe { sym::<IntFn>(lib, n)() })
                .collect();
            let second = unsafe { init() };
            let third = unsafe { init() };
            println!("\nOBS before={before:?} first={first} after={after:?} second={second} third={third}");
        }
        // G1-139: the G1 API works without sodium_init().
        "fresh/no_init_apis" => {
            let b2h = sym::<Bin2Hex>(lib, "sodium_bin2hex");
            let mut h = canary(16);
            let bin = [0xdeu8, 0xad, 0xbe, 0xef];
            unsafe { b2h(h.as_mut_ptr() as *mut c_char, 16, bin.as_ptr(), 4) };
            let mut buf = vec![0u8; 32];
            unsafe { rbuf(buf.as_mut_ptr() as *mut c_void, 32) };
            let all_zero = buf.iter().all(|&x| x == 0);
            let nm = cstr(unsafe { name() });
            println!("\nOBS hex={} buf_all_zero={all_zero} impl={nm}", text(&h));
        }
        // G1-148
        "fresh/impl_name_default" => {
            let nm = cstr(unsafe { name() });
            println!("\nOBS default_impl={nm}");
        }
        // G1-149
        "impl/internal_name" => {
            let rc = unsafe { sset(intimpl) };
            let nm = cstr(unsafe { name() });
            println!("\nOBS rc={rc} name={nm}");
        }
        // G1-150
        "impl/sysrandom_name" => {
            let rc = unsafe { sset(sysimpl) };
            let nm = cstr(unsafe { name() });
            println!("\nOBS rc={rc} name={nm}");
        }
        // G1-151: set_implementation(NULL) leaves implementation == NULL, so
        // randombytes_init_if_needed() silently re-installs the default.
        "impl/null_reinstalls" => {
            let rc = unsafe { sset(ptr::null()) };
            let nm = cstr(unsafe { name() });
            let rc2 = unsafe { sset(ptr::null()) };
            let mut b = vec![0u8; 16];
            unsafe { rbuf(b.as_mut_ptr() as *mut c_void, 16) };
            let nonzero = b.iter().any(|&x| x != 0);
            println!("\nOBS rc={rc} rc2={rc2} name={nm} buf_nonzero={nonzero}");
        }
        // G1-152: stir == NULL -> randombytes_stir() is a no-op.
        "impl/stir_null" => {
            let rc = unsafe { sset(&raw const CH_STIR_NULL) };
            unsafe { *(&raw mut CH_STATE) = 0 };
            unsafe { rstir() };
            unsafe { rstir() };
            let v1 = unsafe { rrandom() };
            let v2 = unsafe { rrandom() };
            println!("\nOBS rc={rc} v1={v1:#010x} v2={v2:#010x}");
        }
        // G1-153: close == NULL -> randombytes_close() returns 0.
        "impl/close_null" => {
            let rc = unsafe { sset(&raw const CH_CLOSE_NULL) };
            let a = unsafe { rclose() };
            let b = unsafe { rclose() };
            println!("\nOBS rc={rc} close1={a} close2={b}");
        }
        // G1-154: uniform == NULL -> the generic modulo-rejection algorithm
        // over implementation->random().
        "impl/uniform_null" => {
            let rc = unsafe { sset(&raw const CH_FULL) };
            let mut out = Vec::new();
            for &ub in &[
                0u32, 1, 2, 3, 10, 256, 1000, 0x7fff_ffff, 0x8000_0001, 0x4000_0000, 0xffff_ffff,
            ] {
                unsafe { *(&raw mut CH_STATE) = 0xDEAD_BEEF };
                let v = unsafe { runiform(ub) };
                assert!(ub < 2 || v < ub, "uniform({ub}) = {v}");
                out.push(format!("{ub}=>{v}"));
            }
            println!("\nOBS rc={rc} uniform={}", out.join(","));
        }
        // G1-155: uniform != NULL -> delegated unconditionally, so the `< 2`
        // shortcut is BYPASSED.
        "impl/uniform_set" => {
            let rc = unsafe { sset(&raw const CH_UNIFORM_SET) };
            let mut out = Vec::new();
            for &ub in &[0u32, 1, 2, 3, 1000, 0xffff_ffff] {
                let v = unsafe { runiform(ub) };
                out.push(format!("{ub}=>{v:#010x}"));
            }
            println!("\nOBS rc={rc} uniform={}", out.join(","));
        }
        // G1-160: randombytes_random passes the impl value through unchanged.
        "impl/random_custom" => {
            let rc = unsafe { sset(&raw const CH_FULL) };
            unsafe { *(&raw mut CH_STATE) = 0 };
            unsafe { rstir() }; // resets CH_STATE via ch_stir
            let vals: Vec<String> = (0..8).map(|_| format!("{:#010x}", unsafe { rrandom() })).collect();
            let mut b = vec![0u8; 40];
            unsafe { rbuf(b.as_mut_ptr() as *mut c_void, 40) };
            println!("\nOBS rc={rc} random={} buf={}", vals.join(","), hex(&b));
        }
        // G1-161: buf == NULL in the impl is safe as long as size == 0.
        "impl/buf_null_size0" => {
            let rc = unsafe { sset(&raw const CH_BUF_NULL) };
            unsafe { rbuf(ptr::null_mut(), 0) };
            let mut junk = [0x5au8; 4];
            unsafe { rbuf(junk.as_mut_ptr() as *mut c_void, 0) };
            println!("\nOBS rc={rc} junk={} ok=1", hex(&junk));
        }
        // G1-162: the real sysrandom impl, several sizes straddling the
        // 256-byte getrandom chunk. The bytes are real entropy, so only the
        // structure is compared.
        "sys/buf_sizes" => {
            unsafe { sset(sysimpl) };
            let mut sizes = Vec::new();
            for &n in &[1usize, 32, 255, 256, 257, 511, 512, 513, 4096] {
                let mut b = canary(n + 8);
                unsafe { rbuf(b.as_mut_ptr() as *mut c_void, n) };
                assert_eq!(&b[n..], &canary(n + 8)[n..], "overrun at n={n}");
                assert!(b[..n].iter().any(|&x| x != 0xA5), "nothing written at n={n}");
                sizes.push(n);
            }
            println!("\nOBS sysrandom_buf sizes={sizes:?} ok=1");
        }
        // G1-168 / G1-169 / G1-170 / G1-174: on the getrandom path
        // stream.initialized is NOT reset by close(), so *every* close returns
        // 0 and the RNG stays usable. (On the /dev/urandom fallback the second
        // close would return -1.)
        "sys/stir_close" => {
            unsafe { sset(sysimpl) };
            unsafe { rstir() };
            unsafe { rstir() };
            let c1 = unsafe { rclose() };
            let c2 = unsafe { rclose() };
            let mut b = vec![0u8; 32];
            unsafe { rbuf(b.as_mut_ptr() as *mut c_void, 32) };
            let usable = b.iter().any(|&x| x != 0);
            let c3 = unsafe { rclose() };
            println!("\nOBS stir_close c1={c1} c2={c2} c3={c3} usable={usable}");
        }
        // G1-166: randombytes_buf_deterministic never consults `implementation`.
        "det/independent_of_impl" => {
            let det = sym::<RbDet>(lib, "randombytes_buf_deterministic");
            let seed = [7u8; 32];
            let mut a = vec![0u8; 64];
            unsafe { det(a.as_mut_ptr() as *mut c_void, 64, seed.as_ptr()) };
            unsafe { sset(intimpl) };
            let mut b = vec![0u8; 64];
            unsafe { det(b.as_mut_ptr() as *mut c_void, 64, seed.as_ptr()) };
            unsafe { sset(ptr::null()) };
            let mut c = vec![0u8; 64];
            unsafe { det(c.as_mut_ptr() as *mut c_void, 64, seed.as_ptr()) };
            let init = sym::<IntFn>(lib, "sodium_init");
            unsafe { init() };
            let mut d = vec![0u8; 64];
            unsafe { det(d.as_mut_ptr() as *mut c_void, 64, seed.as_ptr()) };
            println!("\nOBS det={} same={}", hex(&a), (a == b && b == c && c == d));
        }
        // G1-171: the internal impl's close() zeroes the TLS stream, so the
        // next call re-stirs; global.initialized stays 1 in this build.
        "internal/close_after_use" => {
            unsafe { sset(intimpl) };
            let mut b = vec![0u8; 32];
            unsafe { rbuf(b.as_mut_ptr() as *mut c_void, 32) };
            let used = b.iter().any(|&x| x != 0);
            let c1 = unsafe { rclose() };
            let c2 = unsafe { rclose() };
            let mut b2 = vec![0u8; 32];
            unsafe { rbuf(b2.as_mut_ptr() as *mut c_void, 32) };
            let again = b2.iter().any(|&x| x != 0) && b2 != b;
            println!("\nOBS used={used} c1={c1} c2={c2} again={again}");
        }
        // G1-172: `.buf(p, 0)` reached through randombytes_buf (not invoked)
        // and through the exported struct directly (chacha20 with length 0 —
        // no assert on the internal implementation).
        "internal/buf_zero_direct" => {
            unsafe { sset(intimpl) };
            let mut junk = [0x5au8; 8];
            unsafe { rbuf(junk.as_mut_ptr() as *mut c_void, 0) };
            let via_wrapper = hex(&junk);
            // prime the generator, then call .buf directly with size = 0
            let mut b = vec![0u8; 16];
            unsafe { rbuf(b.as_mut_ptr() as *mut c_void, 16) };
            let bufp = unsafe { (*intimpl).buf.unwrap() };
            unsafe { bufp(junk.as_mut_ptr() as *mut c_void, 0) };
            println!("\nOBS wrapper={via_wrapper} direct={} ok=1", hex(&junk));
        }
        // G1-173: exhaust the 16-block pool ((16*32 - 32)/4 = 120 draws) many
        // times over so the refill path (re-key via xorkey + nonce++) runs.
        "internal/pool_refill" => {
            unsafe { sset(intimpl) };
            let n = 1000usize;
            let mut vals = Vec::with_capacity(n);
            for _ in 0..n {
                vals.push(unsafe { rrandom() });
            }
            let mut sorted = vals.clone();
            sorted.sort_unstable();
            sorted.dedup();
            let zeros = vals.iter().filter(|&&v| v == 0).count();
            println!(
                "\nOBS draws={n} distinct={} zeros={zeros} refills={}",
                sorted.len(),
                n / 120
            );
        }
        other => panic!("unknown tag {other}"),
    }
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}

/// Drives every `CHILD_CASES` row once against the C `.so` and once against
/// the Rust `.so` in a fresh process, and requires identical exit status and
/// identical observations.
#[test]
fn config_child_rows_match() {
    if child_tag().is_some() {
        return;
    }
    setup();
    for &tag in CHILD_CASES {
        let c = run_child("config_child", "c", tag);
        let r = run_child("config_child", "r", tag);
        assert_eq!(
            c.status.code(),
            Some(0),
            "{tag}: the C child failed\n  stdout: {}\n  stderr: {}",
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&c.stderr)
        );
        // guard against a vacuous comparison: `eq_child` only looks at lines
        // that *start* with "OBS " / "MISUSE ", so make sure there is one.
        let co = String::from_utf8_lossy(&c.stdout).to_string();
        assert!(
            co.lines().any(|l| l.starts_with("OBS ")),
            "{tag}: the C child printed no observation line\n  stdout: {co}"
        );
        eq_child(tag, &c, &r);
    }
}
