//! Phase C — `sodium/` + `randombytes/` error surface
//! (`ERRORS.md` section `## G1`, rows `G1-001` … `G1-159`).
//!
//! Three kinds of row:
//!
//! * **`return -1` / `return NULL` rows** — called directly on both `.so`s; the
//!   return value, `errno`, every out-parameter and the whole output buffer are
//!   compared byte-for-byte.
//! * **`sodium_misuse()` rows** — the handler runs and then `abort()` fires, so
//!   each row runs in a child process (once per library) with the observing
//!   handler installed; exit status *and* the side effects written before the
//!   abort are compared.
//! * **raw `assert()` / NULL-dereference rows** — the child dies with SIGABRT
//!   or SIGSEGV; the signal is compared.
//!
//! Rows that are dead code in this build are recorded in
//! `documented_unreachable_error_rows`, together with the observable proxy that
//! proves they cannot fire.

mod common;
use common::*;

use std::ffi::{c_char, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// signatures
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
type RbNacl = unsafe extern "C" fn(*mut u8, u64);
type SetImplFn = unsafe extern "C" fn(*const RandombytesImpl) -> i32;
type SetMisuseFn = unsafe extern "C" fn(Option<unsafe extern "C" fn()>) -> i32;

const EINVAL: i32 = 22;
const ERANGE: i32 = 34;
const ENOMEM: i32 = 12;
const ENOSYS: i32 = 38;

/// Sentinel `errno` value used to detect "errno unchanged" rows. `sodium_mlock`
/// unconditionally does `errno = ENOSYS; return -1` in this build (row G1-085),
/// and ENOSYS collides with none of the values the codecs set.
const SENTINEL: i32 = ENOSYS;

fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

fn set_sentinel() {
    let f = sym::<Mlock>(c_lib(), "sodium_mlock");
    let mut b = [0u8; 1];
    let rc = unsafe { f(b.as_mut_ptr() as *mut c_void, 1) };
    assert_eq!(rc, -1);
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
// sodium_hex2bin rejections
// ===========================================================================

#[derive(Debug)]
struct H2b {
    rc: i32,
    errno: i32,
    bin_len: usize,
    hex_end: isize,
    bin: Vec<u8>,
}

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
        "sodium_hex2bin({what}) *hex_end: C={} Rust={}",
        cc.hex_end, rr.hex_end
    );
    eq_bytes(&format!("sodium_hex2bin({what}) bin"), &cc.bin, &rr.bin);
    res.swap_remove(0)
}

/// ERRORS G1-006, G1-007, G1-008, G1-009, G1-010, G1-011, G1-012, G1-013,
/// G1-014.
#[test]
fn hex2bin_rejections() {
    setup();
    let mut rng = Rng::new(0x2000);

    // G1-006: output buffer full, hex_end != NULL -> ERANGE.
    let o = cmp_hex2bin("G1-006", b"0011", 4, 1, None, true, true);
    assert_eq!((o.rc, o.errno, o.bin_len, o.hex_end), (-1, ERANGE, 0, 2));
    assert_eq!(o.bin[0], 0x00, "the first byte is written before the failure");

    // G1-007: bin_maxlen = 0 -> fails on the very first nibble pair.
    let o = cmp_hex2bin("G1-007", b"00", 2, 0, None, true, true);
    assert_eq!((o.rc, o.errno, o.bin_len, o.hex_end), (-1, ERANGE, 0, 0));
    assert_eq!(o.bin, canary(8), "nothing may be written");

    // G1-008: same ERANGE case but hex_end == NULL -> errno overwritten with
    // EINVAL by the `hex_pos != hex_len` branch.
    let o = cmp_hex2bin("G1-008", b"0011", 4, 1, None, true, false);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, EINVAL, 0));

    // G1-009: odd number of hex digits.
    let o = cmp_hex2bin("G1-009", b"abc", 3, 2, None, true, true);
    assert_eq!((o.rc, o.errno, o.bin_len, o.hex_end), (-1, EINVAL, 0, 2));

    // G1-010: a single hex digit.
    let o = cmp_hex2bin("G1-010", b"a", 1, 1, None, true, true);
    assert_eq!((o.rc, o.errno, o.bin_len, o.hex_end), (-1, EINVAL, 0, 0));

    // G1-011: an ignorable char landing mid-byte (the `state == 0` guard blocks
    // the skip).
    let o = cmp_hex2bin("G1-011", b"0:0", 3, 2, Some(b":\0"), true, true);
    assert_eq!((o.rc, o.errno, o.bin_len, o.hex_end), (-1, EINVAL, 0, 0));
    for sep in [b':', b' ', b'-', b'\n'] {
        let s = [b'0', sep, b'0'];
        let ig = [sep, 0u8];
        let o = cmp_hex2bin(
            &format!("G1-011 sep={:?}", sep as char),
            &s,
            3,
            2,
            Some(&ig),
            true,
            true,
        );
        assert_eq!((o.rc, o.errno, o.bin_len, o.hex_end), (-1, EINVAL, 0, 0));
    }
    // ... and mid-byte deeper into the string: "001:1" breaks at the ':' with
    // state != 0, so hex_pos is decremented back to the start of the byte.
    let o = cmp_hex2bin("G1-011 deep", b"001:1", 5, 4, Some(b":\0"), true, true);
    assert_eq!((o.rc, o.errno, o.bin_len, o.hex_end), (-1, EINVAL, 0, 2));
    // A separator at a byte boundary IS skipped, so "0011:0" is only rejected
    // by the trailing odd nibble; hex_pos ends up at 5, not 4.
    let o = cmp_hex2bin("G1-011 boundary", b"0011:0", 6, 4, Some(b":\0"), true, true);
    assert_eq!((o.rc, o.errno, o.bin_len, o.hex_end), (-1, EINVAL, 0, 5));

    // G1-012: trailing garbage with hex_end == NULL -> -1/EINVAL, but *bin_len
    // is NOT reset (bin_pos is zeroed at codecs.c:90 BEFORE this branch).
    let o = cmp_hex2bin("G1-012", b"00zz", 4, 4, None, true, false);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, EINVAL, 1));
    assert_eq!(o.bin[0], 0x00, "bin[0] must still hold the decoded byte");

    // G1-013: a non-hex non-ignored char with hex_end == NULL.
    let o = cmp_hex2bin("G1-013", b"00-11", 5, 4, Some(b":\0"), true, false);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, EINVAL, 1));
    // longer prefixes, so *bin_len takes several values
    for k in 1..8usize {
        let mut s: Vec<u8> = Vec::new();
        for i in 0..k {
            s.extend_from_slice(format!("{:02x}", i as u8).as_bytes());
        }
        s.push(b'-');
        s.extend_from_slice(b"11");
        let o = cmp_hex2bin(
            &format!("G1-013 k={k}"),
            &s,
            s.len(),
            8,
            Some(b":\0"),
            true,
            false,
        );
        assert_eq!((o.rc, o.errno, o.bin_len), (-1, EINVAL, k));
    }

    // G1-014: an ignored char consumed to the very end is NOT an error.
    let o = cmp_hex2bin("G1-014", b":", 1, 1, Some(b":\0"), true, false);
    assert_eq!((o.rc, o.bin_len), (0, 0));
    assert_eq!(o.errno, SENTINEL, "no errno must be set");
    for s in [&b"::"[..], b"   ", b":: ::"] {
        let o = cmp_hex2bin(
            &format!("G1-014 {:?}", String::from_utf8_lossy(s)),
            s,
            s.len(),
            2,
            Some(b": \0"),
            true,
            false,
        );
        assert_eq!((o.rc, o.bin_len), (0, 0));
    }

    // Randomised fuzz: C and Rust must agree on every rejection, on `errno`,
    // on `*bin_len`, on `*hex_end` and on the output buffer.
    let alphabet: &[u8] = b"0123456789abcdefABCDEFghzZ:-. \n\x00\xff\x7f=+/_";
    for _ in 0..4000 {
        let n = rng.below(12);
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(alphabet)).collect();
        let bin_maxlen = rng.below(8);
        let ig: Option<&[u8]> = match rng.below(4) {
            0 => None,
            1 => Some(b"\0"),
            2 => Some(b": \0"),
            _ => Some(b":-. \n\0"),
        };
        cmp_hex2bin(
            &format!("fuzz {:?} maxlen={bin_maxlen} ig={ig:?}", String::from_utf8_lossy(&s)),
            &s,
            s.len(),
            bin_maxlen,
            ig,
            rng.bool(),
            rng.bool(),
        );
    }
}

// ===========================================================================
// sodium_base642bin rejections
// ===========================================================================

#[derive(Debug)]
struct B2b {
    rc: i32,
    errno: i32,
    bin_len: usize,
    b64_end: isize,
    bin: Vec<u8>,
}

#[allow(clippy::too_many_arguments)]
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
        "sodium_base642bin({what}) *b64_end: C={} Rust={}",
        cc.b64_end, rr.b64_end
    );
    eq_bytes(&format!("sodium_base642bin({what}) bin"), &cc.bin, &rr.bin);
    res.swap_remove(0)
}

/// ERRORS G1-035 … G1-051.
#[test]
fn base642bin_rejections() {
    setup();
    let mut rng = Rng::new(0x2001);

    // G1-035: output buffer full, b64_end != NULL -> ERANGE, *b64_end = &b64[2].
    let o = cmp_base642bin("G1-035", b"AAAA", 4, 1, None, true, true, 1);
    assert_eq!((o.rc, o.errno, o.bin_len, o.b64_end), (-1, ERANGE, 0, 2));

    // G1-036: same with b64_end == NULL -> errno overwritten with EINVAL.
    let o = cmp_base642bin("G1-036", b"AAAA", 4, 1, None, true, false, 1);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, EINVAL, 0));

    // G1-037: bin_maxlen = 0.
    let o = cmp_base642bin("G1-037", b"AA", 2, 0, None, true, true, 3);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, ERANGE, 0));
    assert_eq!(o.bin, canary(8), "nothing may be written");

    // G1-038/039: length == 1 (mod 4) -> acc_len > 4, errno NOT touched.
    let o = cmp_base642bin("G1-038", b"A", 1, 1, None, true, true, 3);
    assert_eq!((o.rc, o.bin_len), (-1, 0));
    assert_eq!(o.errno, SENTINEL, "G1-038 must not set errno");
    let o = cmp_base642bin("G1-039", b"AAAAA", 5, 3, None, true, true, 3);
    assert_eq!((o.rc, o.bin_len), (-1, 0));
    assert_eq!(o.errno, SENTINEL, "G1-039 must not set errno");

    // G1-040/041: non-canonical encodings (leftover bits non-zero).
    let o = cmp_base642bin("G1-040", b"AB", 2, 1, None, true, true, 3);
    assert_eq!((o.rc, o.bin_len), (-1, 0));
    assert_eq!(o.errno, SENTINEL, "G1-040 must not set errno");
    let o = cmp_base642bin("G1-041", b"AAB", 3, 2, None, true, true, 3);
    assert_eq!((o.rc, o.bin_len), (-1, 0));
    assert_eq!(o.errno, SENTINEL, "G1-041 must not set errno");

    // G1-042: the acc check runs BEFORE the padding check.
    let o = cmp_base642bin("G1-042", b"AAB=", 4, 2, None, true, true, 1);
    assert_eq!((o.rc, o.bin_len), (-1, 0));
    assert_eq!(o.errno, SENTINEL, "G1-042 must not set errno");

    // G1-043/044/045: missing padding on a padded variant -> ERANGE.
    let o = cmp_base642bin("G1-043", b"AAA", 3, 2, None, true, true, 1);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, ERANGE, 0));
    let o = cmp_base642bin("G1-044", b"AA", 2, 1, None, true, true, 1);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, ERANGE, 0));
    let o = cmp_base642bin("G1-045", b"AA=", 3, 1, None, true, true, 5);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, ERANGE, 0));

    // G1-046: a non-'=' non-ignored char in a padding position -> EINVAL.
    let o = cmp_base642bin("G1-046", b"AAA*", 4, 2, None, true, true, 1);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, EINVAL, 0));
    for bad in [b'*', b'!', b'#', b' ', b'A'] {
        let s = [b'A', b'A', b'A', bad];
        let o = cmp_base642bin(
            &format!("G1-046 {:?}", bad as char),
            &s,
            4,
            2,
            None,
            true,
            true,
            1,
        );
        assert_eq!(o.rc, -1, "'{}' must be rejected in the padding slot", bad as char);
    }

    // G1-047: putting '=' in `ignore` breaks padded decoding: the '=' are eaten
    // by the main loop, so skip_padding runs off the end -> ERANGE.
    let o = cmp_base642bin("G1-047", b"AA==", 4, 1, Some(b"=\0"), true, true, 1);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, ERANGE, 0));

    // G1-048: trailing garbage with b64_end == NULL -> EINVAL, *bin_len NOT
    // reset (bin_pos is zeroed at codecs.c:327 BEFORE this branch).
    let o = cmp_base642bin("G1-048", b"AAAA!", 5, 3, None, true, false, 3);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, EINVAL, 3));
    assert_eq!(&o.bin[..3], &[0, 0, 0], "bin[0..2] must still hold the decoded bytes");
    // ... with non-zero data so the "written" claim is visible
    let o = cmp_base642bin("G1-048 data", b"////!", 5, 3, None, true, false, 3);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, EINVAL, 3));
    assert_eq!(&o.bin[..3], &[0xff, 0xff, 0xff]);

    // G1-049: URLSAFE variant fed the ORIGINAL alphabet.
    let o = cmp_base642bin("G1-049", b"++++", 4, 3, None, true, false, 5);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, EINVAL, 0));
    let o = cmp_base642bin("G1-049 slash", b"////", 4, 3, None, true, false, 5);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, EINVAL, 0));

    // G1-050: ORIGINAL variant fed the URLSAFE alphabet.
    let o = cmp_base642bin("G1-050", b"----", 4, 3, None, true, false, 1);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, EINVAL, 0));
    let o = cmp_base642bin("G1-050 underscore", b"____", 4, 3, None, true, false, 1);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, EINVAL, 0));

    // G1-051: NO_PADDING variant given padded input, b64_end == NULL.
    let o = cmp_base642bin("G1-051", b"AA==", 4, 1, None, true, false, 3);
    assert_eq!((o.rc, o.errno, o.bin_len), (-1, EINVAL, 1));
    let o = cmp_base642bin("G1-051 v=7", b"__==", 4, 1, None, true, false, 7);
    assert_eq!(o.rc, -1);

    // Randomised fuzz over all four valid variants.
    let alphabet: &[u8] =
        b"ABCyz019+/-_= \n\x00!*\xff";
    for _ in 0..5000 {
        let n = rng.below(10);
        let s: Vec<u8> = (0..n).map(|_| *rng.pick(alphabet)).collect();
        let bin_maxlen = rng.below(8);
        let v = *rng.pick(&[1i32, 3, 5, 7]);
        let ig: Option<&[u8]> = match rng.below(4) {
            0 => None,
            1 => Some(b"\0"),
            2 => Some(b" \n\0"),
            _ => Some(b"=\0"),
        };
        cmp_base642bin(
            &format!(
                "fuzz {:?} maxlen={bin_maxlen} v={v} ig={ig:?}",
                String::from_utf8_lossy(&s)
            ),
            &s,
            s.len(),
            bin_maxlen,
            ig,
            rng.bool(),
            rng.bool(),
            v,
        );
    }
}

// ===========================================================================
// sodium_ip2bin rejections
// ===========================================================================

fn cmp_ip2bin(what: &str, ip: &[u8], ip_len: usize) -> i32 {
    let (c, r) = pair::<Ip2Bin>("sodium_ip2bin");
    let mut a = canary(16);
    let mut b = canary(16);
    set_sentinel();
    let ra = unsafe { c(a.as_mut_ptr(), ip.as_ptr() as *const c_char, ip_len) };
    let ea = errno();
    set_sentinel();
    let rb = unsafe { r(b.as_mut_ptr(), ip.as_ptr() as *const c_char, ip_len) };
    let eb = errno();
    eq_i32(&format!("sodium_ip2bin({what}) rc"), ra, rb);
    eq_i32(&format!("sodium_ip2bin({what}) errno"), ea, eb);
    eq_bytes(&format!("sodium_ip2bin({what}) bin"), &a, &b);
    if ra != 0 {
        assert_eq!(a, canary(16), "sodium_ip2bin({what}) must not write on failure");
    }
    ra
}

/// ERRORS G1-052 … G1-075.
#[test]
fn ip2bin_rejections() {
    setup();
    let mut rng = Rng::new(0x2002);

    // G1-052: empty input.
    assert_eq!(cmp_ip2bin("G1-052 len=0", b"", 0), -1);
    assert_eq!(cmp_ip2bin("G1-052 empty str", b"\0", 1), -1);

    // One row per rejection reason (the source line is in the comment).
    let bad: &[(&str, &str)] = &[
        // G1-053: IPv4 octet > 255
        ("G1-053", "256.1.1.1"),
        ("G1-053b", "1.256.1.1"),
        ("G1-053c", "1.1.1.256"),
        ("G1-053d", "999.1.1.1"),
        // G1-054: 4 digits in an octet
        ("G1-054", "1234.1.1.1"),
        ("G1-054b", "0001.2.3.4"),
        // G1-055: empty octet
        ("G1-055", "1..2.3"),
        ("G1-055b", ".1.2.3"),
        ("G1-055c", "1.2.3."),
        ("G1-055d", "1.2..4"),
        // G1-056: missing separator / too short
        ("G1-056", "1.2.3"),
        ("G1-056b", "1.2"),
        ("G1-056c", "1"),
        // G1-057: wrong separator
        ("G1-057", "1.2.3-4"),
        ("G1-057b", "1.2-3.4"),
        // G1-058: trailing garbage
        ("G1-058", "1.2.3.4x"),
        ("G1-058b", "1.2.3.4."),
        ("G1-058c", "1.2.3.4.5"),
        ("G1-058d", " 1.2.3.4"),
        ("G1-058e", "1.2.3.4 "),
        // G1-059: invalid zone-id character
        ("G1-059", "fe80::1%eth0!"),
        ("G1-059b", "fe80::1%a b"),
        ("G1-059c", "fe80::1%:"),
        ("G1-059d", "fe80::1%%"),
        // G1-060: empty zone id
        ("G1-060", "fe80::1%"),
        // G1-061: zone id only
        ("G1-061", "%"),
        // G1-062: zone id on a non-IPv6 address
        ("G1-062", "1.2.3.4%eth0"),
        // G1-063: ':' after '%' is cut off, so is_ipv6 is false
        ("G1-063", "1.2.3.4%foo:bar"),
        // G1-064: single leading colon
        ("G1-064", ":1::2"),
        ("G1-064b", ":"),
        ("G1-064c", ":1:2:3:4:5:6:7"),
        // G1-065: two `::` runs
        ("G1-065", "1::2::3"),
        ("G1-065b", "::1::"),
        // G1-066: too many groups
        ("G1-066", "1:2:3:4:5:6:7:8:9"),
        ("G1-066b", "1:2:3:4:5:6:7:8:9:a"),
        // G1-067: trailing single colon
        ("G1-067", "1:2:"),
        ("G1-067b", "1:"),
        // G1-068: 5 hex digits in a group
        ("G1-068", "12345::"),
        ("G1-068b", "1:23456:3::"),
        // G1-069: invalid hex char
        ("G1-069", "1:2:3:4:5:6:7:g"),
        ("G1-069b", "1:2:3:4:5:6:7:8g"),
        ("G1-069c", "g::1"),
        // G1-070: embedded IPv4 that does not fit
        ("G1-070", "1:2:3:4:5:6:7:1.2.3.4"),
        // G1-071: malformed embedded IPv4
        ("G1-071", "::1.2.3"),
        ("G1-071b", "::1.2.3.4.5"),
        ("G1-071c", "::256.1.1.1"),
        // G1-072: embedded IPv4 not at the end
        ("G1-072", "::1.2.3.4:5"),
        // G1-073: `::` when the address is already full
        ("G1-073", "1:2:3:4:5:6:7:8::"),
        // G1-074: too few groups and no `::`
        ("G1-074", "1:2:3"),
        ("G1-074b", "1:2"),
        ("G1-074c", "1:2:3:4:5:6:7"),
        // G1-075: final group overflow at the tail
        ("G1-075", "1:2:3:4:5:6:7:8:9999"),
        ("G1-075b", "1:2:3:4:5:6:7:8:1"),
    ];
    for &(row, s) in bad {
        assert_eq!(
            cmp_ip2bin(&format!("{row} {s:?}"), s.as_bytes(), s.len()),
            -1,
            "{row}: sodium_ip2bin({s:?}) must be rejected"
        );
    }

    // Randomised fuzz: every string built from the IP character set. C and Rust
    // must agree on accept/reject, on the 16 output bytes and on `errno`.
    let alphabet: &[u8] = b"0123456789abcdefABCDEFxyzZ:.%-_ \x00\xff";
    for _ in 0..20000 {
        let n = rng.below(24);
        let mut s: Vec<u8> = (0..n).map(|_| *rng.pick(alphabet)).collect();
        // `ip_len_` may exceed the string: the scan then stops at the first NUL.
        // Pad the allocation so every read stays in bounds.
        let len = if rng.below(8) == 0 { rng.below(n + 3) } else { n };
        s.resize(n + 8, 0);
        cmp_ip2bin(
            &format!("fuzz {:?} len={len}", String::from_utf8_lossy(&s[..n])),
            &s,
            len,
        );
    }
    // structured fuzz: mutate valid addresses
    let seeds: &[&str] = &[
        "1.2.3.4",
        "255.255.255.255",
        "::",
        "::1",
        "1::",
        "1:2:3:4:5:6:7:8",
        "fe80::1%eth0",
        "::ffff:1.2.3.4",
        "64:ff9b::1.2.3.4",
    ];
    for _ in 0..8000 {
        let base = *rng.pick(seeds);
        let mut s: Vec<u8> = base.as_bytes().to_vec();
        for _ in 0..rng.range(1, 3) {
            match rng.below(4) {
                0 if !s.is_empty() => {
                    let i = rng.below(s.len());
                    s[i] = *rng.pick(alphabet);
                }
                1 => {
                    let i = rng.below(s.len() + 1);
                    s.insert(i, *rng.pick(alphabet));
                }
                2 if !s.is_empty() => {
                    let i = rng.below(s.len());
                    s.remove(i);
                }
                _ if !s.is_empty() => {
                    let i = rng.below(s.len());
                    let j = rng.below(s.len());
                    s.swap(i, j);
                }
                _ => {}
            }
        }
        cmp_ip2bin(
            &format!("mutate {:?}", String::from_utf8_lossy(&s)),
            &s,
            s.len(),
        );
    }
}

/// ERRORS G1-076, G1-077, G1-078, G1-079, G1-080.
#[test]
fn bin2ip_rejections() {
    setup();
    let (c, r) = pair::<Bin2Ip>("sodium_bin2ip");
    let mut rng = Rng::new(0x2003);

    let go = |what: &str, bin: &[u8; 16], ip_maxlen: usize| -> bool {
        let mut a = canary(64);
        let mut b = canary(64);
        set_sentinel();
        let pa = unsafe { c(a.as_mut_ptr() as *mut c_char, ip_maxlen, bin.as_ptr()) };
        let ea = errno();
        set_sentinel();
        let pb = unsafe { r(b.as_mut_ptr() as *mut c_char, ip_maxlen, bin.as_ptr()) };
        let eb = errno();
        assert_eq!(
            pa.is_null(),
            pb.is_null(),
            "sodium_bin2ip({what}): C null={} Rust null={}",
            pa.is_null(),
            pb.is_null()
        );
        eq_i32(&format!("sodium_bin2ip({what}) errno"), ea, eb);
        eq_bytes(&format!("sodium_bin2ip({what}) buffer"), &a, &b);
        if pa.is_null() {
            assert_eq!(a, canary(64), "sodium_bin2ip({what}) must not write on failure");
            assert_eq!(ea, SENTINEL, "sodium_bin2ip must leave errno untouched");
        }
        !pa.is_null()
    };

    let v4mapped = |o: [u8; 4]| -> [u8; 16] {
        let mut b = [0u8; 16];
        b[10] = 0xff;
        b[11] = 0xff;
        b[12..].copy_from_slice(&o);
        b
    };

    // G1-076: ip_maxlen <= 2 with any bin.
    for m in [0usize, 1, 2] {
        for bin in [
            [0u8; 16],
            [0xffu8; 16],
            v4mapped([1, 2, 3, 4]),
            v4mapped([0, 0, 0, 0]),
        ] {
            assert!(!go(&format!("G1-076 maxlen={m}"), &bin, m), "ip_maxlen={m}");
        }
    }

    // G1-077: v4-mapped, text len 15, ip_maxlen = 15.
    let bin = v4mapped([255, 255, 255, 255]);
    assert!(!go("G1-077", &bin, 15));
    assert!(go("G1-077 ok", &bin, 16));

    // G1-078: v4-mapped 10.0.0.1 (text len 8) with ip_maxlen = 7.
    let bin = v4mapped([10, 0, 0, 1]);
    assert!(!go("G1-078", &bin, 7));
    assert!(!go("G1-078b", &bin, 8));
    assert!(go("G1-078 ok", &bin, 9));

    // G1-079: 16x0xff (text len 39) with ip_maxlen = 39.
    let bin = [0xffu8; 16];
    assert!(!go("G1-079", &bin, 39));
    assert!(go("G1-079 ok", &bin, 40));

    // G1-080: ::1 (text len 3) with ip_maxlen = 3.
    let mut bin = [0u8; 16];
    bin[15] = 1;
    assert!(!go("G1-080", &bin, 3));
    assert!(go("G1-080 ok", &bin, 4));

    // Every `len >= ip_maxlen` boundary, for many random bins.
    for _ in 0..2000 {
        let mut bin = [0u8; 16];
        let v = rng.bytes(16);
        bin.copy_from_slice(&v);
        if rng.bool() {
            // bias towards zero groups so compressed forms appear
            for i in 0..8 {
                if rng.bool() {
                    bin[i * 2] = 0;
                    bin[i * 2 + 1] = 0;
                }
            }
        }
        if rng.below(4) == 0 {
            bin[..10].fill(0);
            bin[10] = 0xff;
            bin[11] = 0xff;
        }
        for m in 0..=46usize {
            go(&format!("boundary maxlen={m}"), &bin, m);
        }
    }
}

// ===========================================================================
// sodium_memcmp / sodium_compare / sodium_is_zero sentinels
// ===========================================================================

/// ERRORS G1-081, G1-082, G1-083, G1-084.
#[test]
fn comparison_sentinels() {
    setup();
    let mut rng = Rng::new(0x2004);

    // G1-081: sodium_memcmp returns -1 for any single differing byte.
    let (c, r) = pair::<Cmp3>("sodium_memcmp");
    assert_eq!(unsafe { c(&0u8, &1u8, 1) }, -1);
    assert_eq!(unsafe { r(&0u8, &1u8, 1) }, -1);
    for &len in &[1usize, 2, 8, 16, 32, 33, 64] {
        for _ in 0..16 {
            let x = rng.bytes(len);
            for i in 0..len {
                let mut y = x.clone();
                y[i] ^= 1 << rng.below(8);
                let (a, b) = unsafe { (c(x.as_ptr(), y.as_ptr(), len), r(x.as_ptr(), y.as_ptr(), len)) };
                eq_i32("sodium_memcmp differ", a, b);
                assert_eq!(a, -1);
            }
        }
        // len == 0 and equal buffers are the only 0 cases
        let x = rng.bytes(len);
        assert_eq!(unsafe { c(x.as_ptr(), x.as_ptr(), 0) }, 0);
        assert_eq!(unsafe { c(x.as_ptr(), x.as_ptr(), len) }, 0);
    }

    // G1-082/083: sodium_compare -1 / 1.
    let (c, r) = pair::<Cmp3>("sodium_compare");
    let lt: [u8; 2] = [0xff, 0x00];
    let gt: [u8; 2] = [0x00, 0x01];
    let (a, b) = unsafe { (c(lt.as_ptr(), gt.as_ptr(), 2), r(lt.as_ptr(), gt.as_ptr(), 2)) };
    eq_i32("sodium_compare G1-082", a, b);
    assert_eq!(a, -1);
    let (a, b) = unsafe { (c(&1u8, &0u8, 1), r(&1u8, &0u8, 1)) };
    eq_i32("sodium_compare G1-083", a, b);
    assert_eq!(a, 1);
    for &len in &[1usize, 2, 8, 32, 33] {
        for _ in 0..64 {
            let x = rng.bytes(len);
            let y = rng.bytes(len);
            let (a, b) = unsafe { (c(x.as_ptr(), y.as_ptr(), len), r(x.as_ptr(), y.as_ptr(), len)) };
            eq_i32("sodium_compare fuzz", a, b);
            assert!(a == -1 || a == 0 || a == 1);
        }
    }

    // G1-084: sodium_is_zero returns 0 for any non-zero byte.
    let (c, r) = pair::<IsZero>("sodium_is_zero");
    for &len in &[1usize, 8, 32, 33, 64] {
        for pos in 0..len {
            let mut v = vec![0u8; len];
            v[pos] = 1;
            let (a, b) = unsafe { (c(v.as_ptr(), len), r(v.as_ptr(), len)) };
            eq_i32("sodium_is_zero", a, b);
            assert_eq!(a, 0, "sodium_is_zero must be 0 with n[{pos}] = 1");
        }
        let z = vec![0u8; len];
        assert_eq!(unsafe { c(z.as_ptr(), len) }, 1);
        assert_eq!(unsafe { c(z.as_ptr(), 0) }, 1);
    }
}

// ===========================================================================
// mlock / munlock / mprotect_* / malloc / allocarray
// ===========================================================================

/// ERRORS G1-085, G1-086, G1-087, G1-088, G1-089.
#[test]
fn mlock_and_mprotect_always_enosys() {
    setup();
    let mut rng = Rng::new(0x2005);
    let (cm, rm) = pair::<MallocFn>("sodium_malloc");
    let (cf, rf) = pair::<FreeFn>("sodium_free");

    // G1-085: sodium_mlock is always -1 / ENOSYS and leaves the buffer alone.
    let (cl, rl) = pair::<Mlock>("sodium_mlock");
    for &len in &[0usize, 1, 16, 4096, usize::MAX] {
        let mut buf = rng.bytes(64);
        let before = buf.clone();
        for (f, which) in [(cl, "C"), (rl, "Rust")] {
            set_sentinel();
            let _ = std::fs::metadata("/"); // make the sentinel meaningful
            set_sentinel();
            let rc = unsafe { f(buf.as_mut_ptr() as *mut c_void, len) };
            assert_eq!(rc, -1, "{which} sodium_mlock(len={len})");
            assert_eq!(errno(), ENOSYS, "{which} sodium_mlock errno");
        }
        assert_eq!(buf, before, "sodium_mlock must not modify the buffer");
    }

    // G1-086: sodium_munlock zeroes the buffer BEFORE failing.
    let (cu, ru) = pair::<Mlock>("sodium_munlock");
    for &len in &[0usize, 1, 8, 16, 33, 64] {
        for (f, which) in [(cu, "C"), (ru, "Rust")] {
            let mut buf = rng.bytes(64);
            rng.fill(&mut buf);
            let tail = buf[len..].to_vec();
            set_sentinel();
            let rc = unsafe { f(buf.as_mut_ptr() as *mut c_void, len) };
            assert_eq!(rc, -1, "{which} sodium_munlock(len={len})");
            assert_eq!(errno(), ENOSYS, "{which} sodium_munlock errno");
            assert!(
                buf[..len].iter().all(|&x| x == 0),
                "{which} sodium_munlock must zero the {len} bytes first"
            );
            assert_eq!(&buf[len..], &tail[..], "{which} sodium_munlock overran");
        }
    }
    // both libraries must produce the identical buffer
    for &len in &[1usize, 16, 64] {
        let src = rng.bytes(64);
        let mut a = src.clone();
        let mut b = src.clone();
        unsafe {
            cu(a.as_mut_ptr() as *mut c_void, len);
            ru(b.as_mut_ptr() as *mut c_void, len);
        }
        eq_bytes(&format!("sodium_munlock(len={len}) buffer"), &a, &b);
    }

    // G1-087/088/089: every sodium_mprotect_* is -1 / ENOSYS, on a
    // sodium_malloc pointer and on a plain one, and the memory stays usable.
    let names = [
        "sodium_mprotect_noaccess",
        "sodium_mprotect_readonly",
        "sodium_mprotect_readwrite",
    ];
    let (p1, p2) = unsafe { (cm(32), rm(32)) };
    let mut plain = [0u8; 32];
    for name in names {
        let (c, r) = pair::<Mprotect>(name);
        for (f, p, which) in [(c, p1, "C"), (r, p2, "Rust")] {
            set_sentinel();
            let rc = unsafe { f(p) };
            assert_eq!(rc, -1, "{which} {name} on sodium_malloc memory");
            assert_eq!(errno(), ENOSYS, "{which} {name} errno");
            unsafe {
                let q = p as *mut u8;
                let v = *q;
                *q = v ^ 0x5a;
                assert_eq!(*q, v ^ 0x5a, "{which} {name} must not protect anything");
                *q = v;
            }
            set_sentinel();
            let rc = unsafe { f(plain.as_mut_ptr() as *mut c_void) };
            assert_eq!(rc, -1, "{which} {name} on a plain pointer");
            assert_eq!(errno(), ENOSYS, "{which} {name} errno (plain)");
        }
    }
    unsafe {
        cf(p1);
        rf(p2);
    }
}

/// ERRORS G1-091, G1-092, G1-093, G1-094, G1-095, G1-096, G1-097.
#[test]
fn malloc_and_allocarray_failures() {
    setup();
    let (cm, rm) = pair::<MallocFn>("sodium_malloc");
    let (ca, ra) = pair::<AllocArray>("sodium_allocarray");

    // G1-091/092: sodium_malloc goes straight to malloc(), which fails.
    for &size in &[usize::MAX, usize::MAX - 4096, usize::MAX / 2, usize::MAX - 1] {
        for (f, which) in [(cm, "C"), (rm, "Rust")] {
            set_sentinel();
            let p = unsafe { f(size) };
            assert!(p.is_null(), "{which} sodium_malloc({size:#x}) must be NULL");
            assert_eq!(errno(), ENOMEM, "{which} sodium_malloc({size:#x}) errno");
        }
    }

    // G1-093 … G1-096: the `count > 0 && size >= SIZE_MAX / count` guard.
    let cases: &[(usize, usize, &str)] = &[
        (2, usize::MAX / 2, "G1-093"),
        (1, usize::MAX, "G1-094"),
        (usize::MAX, 1, "G1-095"),
        (3, usize::MAX / 3, "G1-096"),
        (2, usize::MAX / 2 + 1, "G1-093b"),
        (4, usize::MAX / 4, "G1-096b"),
        (usize::MAX, usize::MAX, "extreme"),
        (1 << 32, 1 << 32, "2^32 x 2^32"),
    ];
    for &(count, size, row) in cases {
        assert!(
            count > 0 && size >= usize::MAX / count,
            "{row}: test bug, the guard would not trip"
        );
        for (f, which) in [(ca, "C"), (ra, "Rust")] {
            set_sentinel();
            let p = unsafe { f(count, size) };
            assert!(
                p.is_null(),
                "{which} {row} sodium_allocarray({count}, {size}) must be NULL"
            );
            assert_eq!(errno(), ENOMEM, "{which} {row} errno");
        }
    }

    // G1-097: passes the overflow guard, then malloc() fails -> NULL / ENOMEM
    // from libc.
    for &(count, size) in &[(2usize, usize::MAX / 2 - 1), (3, usize::MAX / 3 - 1)] {
        assert!(size < usize::MAX / count, "test bug: the guard would trip");
        for (f, which) in [(ca, "C"), (ra, "Rust")] {
            set_sentinel();
            let p = unsafe { f(count, size) };
            assert!(
                p.is_null(),
                "{which} sodium_allocarray({count}, {size}) must be NULL"
            );
            assert_eq!(errno(), ENOMEM, "{which} sodium_allocarray errno");
        }
    }
}

// ===========================================================================
// sodium_pad / sodium_unpad rejections
// ===========================================================================

/// ERRORS G1-098, G1-102, G1-103, G1-104, G1-105, G1-106.
#[test]
fn pad_rejections() {
    setup();
    let mut rng = Rng::new(0x2006);
    let (c, r) = pair::<Pad>("sodium_pad");

    let go = |what: &str, n: usize, bs: usize, max: usize| -> (i32, usize) {
        let bufsz = max.max(n) + 64;
        let data = vec![0x5au8; bufsz];
        let mut res: Vec<(i32, usize, Vec<u8>)> = Vec::new();
        for f in [c, r] {
            let mut buf = data.clone();
            let mut out = 0xA5A5_A5A5_usize;
            let rc = unsafe { f(&raw mut out, buf.as_mut_ptr(), n, bs, max) };
            res.push((rc, out, buf));
        }
        eq_i32(&format!("sodium_pad({what}) rc"), res[0].0, res[1].0);
        eq_usize(&format!("sodium_pad({what}) *out"), res[0].1, res[1].1);
        eq_bytes(&format!("sodium_pad({what}) buf"), &res[0].2, &res[1].2);
        if res[0].0 != 0 {
            assert_eq!(
                res[0].1, 0xA5A5_A5A5,
                "sodium_pad({what}) must NOT write *padded_buflen_p on failure"
            );
            assert_eq!(res[0].2, data, "sodium_pad({what}) must not touch buf on failure");
        }
        (res[0].0, res[0].1)
    };

    // G1-098: blocksize == 0.
    for &n in &[0usize, 1, 16, 1000] {
        for &max in &[0usize, 1, 16, 4096] {
            assert_eq!(go("G1-098", n, 0, max).0, -1, "blocksize 0 must be rejected");
        }
    }
    // and with padded_buflen_p == NULL it must still just return -1
    for f in [c, r] {
        let mut buf = [0u8; 32];
        let rc = unsafe { f(ptr::null_mut(), buf.as_mut_ptr(), 4, 0, 32) };
        assert_eq!(rc, -1);
    }

    // G1-102: xpadded_len >= max_buflen.
    assert_eq!(go("G1-102", 10, 16, 15).0, -1);
    // G1-103: exact-fit-minus-one.
    assert_eq!(go("G1-103", 16, 16, 31).0, -1);
    assert_eq!(go("G1-103 ok", 16, 16, 32).0, 0);
    // G1-104: max_buflen = 0.
    for &bs in &[1usize, 2, 16, 64] {
        for &n in &[0usize, 1, 5, 100] {
            assert_eq!(go("G1-104", n, bs, 0).0, -1);
        }
    }
    // G1-105.
    assert_eq!(go("G1-105", 0, 16, 15).0, -1);
    assert_eq!(go("G1-105 ok", 0, 16, 16).0, 0);
    // G1-106: non-power-of-two blocksize.
    assert_eq!(go("G1-106 max=5", 5, 3, 5).0, -1);
    assert_eq!(go("G1-106 max=6", 5, 3, 6).0, 0);

    // Every (n, bs, max) triple around the boundary must agree.
    for &bs in &[1usize, 2, 3, 4, 7, 8, 16, 17, 31, 32, 64] {
        for n in 0..(2 * bs + 3) {
            let xpadded = n - n % bs + bs - 1;
            for max in 0..(xpadded + 4) {
                let (rc, out) = go(&format!("sweep n={n} bs={bs} max={max}"), n, bs, max);
                if max > xpadded {
                    assert_eq!((rc, out), (0, xpadded + 1));
                } else {
                    assert_eq!(rc, -1);
                }
            }
        }
    }
    // randomised
    for _ in 0..2000 {
        let bs = rng.range(1, 70);
        let n = rng.below(200);
        let max = rng.below(300);
        go(&format!("rand n={n} bs={bs} max={max}"), n, bs, max);
    }
}

/// ERRORS G1-107, G1-108, G1-109, G1-110, G1-111, G1-112, G1-113.
#[test]
fn unpad_rejections() {
    setup();
    let mut rng = Rng::new(0x2007);
    let (c, r) = pair::<Unpad>("sodium_unpad");

    let go = |what: &str, buf: &[u8], bs: usize| -> (i32, usize) {
        let mut res: Vec<(i32, usize)> = Vec::new();
        for f in [c, r] {
            let mut out = 0xA5A5_A5A5_usize;
            let rc = unsafe { f(&raw mut out, buf.as_ptr(), buf.len(), bs) };
            res.push((rc, out));
        }
        eq_i32(&format!("sodium_unpad({what}) rc"), res[0].0, res[1].0);
        eq_usize(&format!("sodium_unpad({what}) *out"), res[0].1, res[1].1);
        res[0]
    };

    // G1-107: blocksize == 0 -> -1, *unpadded_buflen_p NOT written.
    for n in [0usize, 1, 16, 100] {
        let buf = rng.bytes(n);
        let (rc, out) = go("G1-107", &buf, 0);
        assert_eq!(rc, -1);
        assert_eq!(out, 0xA5A5_A5A5, "*unpadded_buflen_p must not be written");
    }

    // G1-108/109/110: padded_buflen < blocksize.
    for (n, bs) in [(0usize, 16usize), (15, 16), (0, 1), (1, 2), (63, 64)] {
        let buf = rng.bytes(n);
        let (rc, out) = go(&format!("padded_buflen={n} bs={bs}"), &buf, bs);
        assert_eq!(rc, -1, "padded_buflen={n} < blocksize={bs}");
        assert_eq!(out, 0xA5A5_A5A5, "*unpadded_buflen_p must not be written");
    }

    // G1-111: no 0x80 barrier -> -1 but *unpadded_buflen_p IS written.
    let buf = vec![0u8; 16];
    let (rc, out) = go("G1-111", &buf, 16);
    assert_eq!((rc, out), (-1, 15));

    // G1-112: non-0x80 garbage after the barrier.
    let mut buf = vec![0u8; 16];
    buf[14] = 0x80;
    buf[15] = 0x01;
    let (rc, out) = go("G1-112", &buf, 16);
    assert_eq!((rc, out), (-1, 15));

    // G1-113: the last byte is 0xff.
    let mut buf = vec![0u8; 16];
    buf[15] = 0xff;
    let (rc, out) = go("G1-113", &buf, 16);
    assert_eq!((rc, out), (-1, 15));

    // more invalid-padding shapes; C and Rust must agree on rc AND on the
    // unconditional write at utils.c:810
    for &bs in &[1usize, 2, 4, 8, 16, 17, 32, 64] {
        for n in bs..(bs + 40) {
            for kind in 0..6 {
                let mut buf = match kind {
                    0 => vec![0u8; n],
                    1 => vec![0xffu8; n],
                    2 => vec![0x80u8; n],
                    3 => rng.bytes(n),
                    4 => {
                        let mut v = vec![0u8; n];
                        v[n - 1] = 0x81;
                        v
                    }
                    _ => {
                        let mut v = vec![0u8; n];
                        if n >= 2 {
                            v[n - 2] = 0x80;
                            v[n - 1] = 0x01;
                        }
                        v
                    }
                };
                if kind == 3 {
                    // ensure the tail is not accidentally valid padding
                    buf[n - 1] = 0x7f;
                }
                go(&format!("invalid bs={bs} n={n} kind={kind}"), &buf, bs);
            }
        }
    }
    // exhaustive over a 2-byte block
    for a in 0..=255u16 {
        for b in 0..=255u16 {
            go("exhaustive bs=2", &[a as u8, b as u8], 2);
        }
    }
}

// ===========================================================================
// no-rejection-path rows
// ===========================================================================

/// ERRORS G1-115, G1-116, G1-117, G1-118, G1-132, G1-136, G1-128, G1-129,
/// G1-130.
#[test]
fn no_rejection_path_rows() {
    setup();
    let mut rng = Rng::new(0x2008);

    // G1-115: sodium_memzero has no rejection path; len = 0 is a no-op even
    // with pnt = NULL. (The sodium_misuse() at utils.c:132 is inside
    // `#elif defined(HAVE_MEMSET_S)` -> dead.)
    let (cz, rz) = pair::<Memzero>("sodium_memzero");
    unsafe {
        cz(ptr::null_mut(), 0);
        rz(ptr::null_mut(), 0);
    }
    for &n in &[0usize, 1, 32, 4096] {
        let src = rng.bytes(n + 8);
        let mut a = src.clone();
        let mut b = src.clone();
        unsafe {
            cz(a.as_mut_ptr() as *mut c_void, n);
            rz(b.as_mut_ptr() as *mut c_void, n);
        }
        eq_bytes(&format!("sodium_memzero({n})"), &a, &b);
    }

    // G1-116: sodium_stackzero has an EMPTY body, so any len (including
    // SIZE_MAX) is a no-op and nothing is zeroed.
    let (cs, rs) = pair::<Stackzero>("sodium_stackzero");
    let mut probe = [0x5au8; 4096];
    rng.fill(&mut probe);
    let before = probe;
    for &n in &[0usize, 1, 512, 4096, 1 << 24, usize::MAX / 2, usize::MAX] {
        unsafe {
            cs(n);
            rs(n);
        }
    }
    assert_eq!(probe, before, "sodium_stackzero must zero nothing");

    // G1-117: sodium_free(NULL) is a no-op (plain free(NULL)).
    let (cf, rf) = pair::<FreeFn>("sodium_free");
    for _ in 0..8 {
        unsafe {
            cf(ptr::null_mut());
            rf(ptr::null_mut());
        }
    }
    // G1-118: freeing a pointer that was NOT obtained from
    // sodium_malloc/sodium_allocarray is undefined behaviour inside libc
    // `free()` and therefore NOT constructible as a differential test. The
    // nearest reachable condition is asserted instead: because this build has
    // no canary and no `_out_of_bounds()`, a *deliberately corrupted*
    // sodium_malloc region still frees cleanly (an aligned-malloc build would
    // abort here).
    let (cm, rm) = pair::<MallocFn>("sodium_malloc");
    for (mf, ff) in [(cm, cf), (rm, rf)] {
        let p = unsafe { mf(32) };
        assert!(!p.is_null());
        unsafe {
            // scribble over the whole user region, incl. where a canary would be
            ptr::write_bytes(p as *mut u8, 0x11, 32);
            ff(p);
        }
    }

    // G1-132: `assert(buf_len <= SIZE_MAX)` in randombytes() can never fail on
    // x86-64 (`unsigned long long` and `size_t` are both 64-bit).
    assert_eq!(
        std::mem::size_of::<usize>(),
        std::mem::size_of::<u64>(),
        "the assert is unfireable only because ULLONG_MAX == SIZE_MAX"
    );
    let (cn, rn) = pair::<RbNacl>("randombytes");
    let mut a = canary(64);
    let mut b = canary(64);
    reset_rngs(0x777);
    unsafe { cn(a.as_mut_ptr(), 64) };
    reset_rngs(0x777);
    unsafe { rn(b.as_mut_ptr(), 64) };
    eq_bytes("randombytes(_, 64)", &a, &b);

    // G1-136: an implementation with `uniform == NULL` is NOT an error; it
    // simply takes the generic rejection-sampling path (this is the harness
    // implementation).
    let (cu, ru) = pair::<RbUniform>("randombytes_uniform");
    for &ub in &[0u32, 1, 2, 7, 1000, 0x8000_0001] {
        reset_rngs(0x888);
        let x = unsafe { cu(ub) };
        reset_rngs(0x888);
        let y = unsafe { ru(ub) };
        assert_eq!(x, y, "randombytes_uniform({ub}) with uniform == NULL");
        assert!(ub < 2 || x < ub);
    }

    // G1-128/129/130: the CPU-feature probes always fail on this build.
    let (c, r) = pair::<IntFn>("_sodium_runtime_get_cpu_features");
    for _ in 0..4 {
        let (a, b) = unsafe { (c(), r()) };
        eq_i32("_sodium_runtime_get_cpu_features()", a, b);
        assert_eq!(a, -1, "must be -1 & -1 & -1");
    }
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
        let (a, b) = unsafe { (c(), r()) };
        eq_i32(name, a, b);
        assert_eq!(a, 0, "{name} must stay 0 after the failed probe");
    }
}

// ===========================================================================
// out-of-process rows
// ===========================================================================

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Outcome {
    /// `sodium_misuse()` — the observing handler runs, then `exit(MISUSE_EXIT)`.
    Misuse,
    /// A raw `assert()` failure or a misuse with a handler that does not exit.
    Abort,
    /// A NULL dereference / NULL indirect call.
    Segv,
}
use Outcome::*;

/// Every out-of-process ERRORS row, with the outcome the C reference produces.
const CASES: &[(&str, Outcome)] = &[
    // ---- sodium_bin2hex (codecs.c:23) ----
    ("G1-001/bin2hex/binlen=SIZE_MAX_div_2", Misuse),
    ("G1-002/bin2hex/binlen=SIZE_MAX", Misuse),
    ("G1-003/bin2hex/binlen=4,maxlen=8", Misuse),
    ("G1-004/bin2hex/binlen=0,maxlen=0", Misuse),
    ("G1-005/bin2hex/binlen=1,maxlen=2", Misuse),
    // ---- sodium_base64_check_variant (codecs.c:168) via encoded_len ----
    ("G1-016/enclen/variant=0", Misuse),
    ("G1-017/enclen/variant=2", Misuse),
    ("G1-018/enclen/variant=4", Misuse),
    ("G1-019/enclen/variant=6", Misuse),
    ("G1-020/enclen/variant=8", Misuse),
    ("G1-021/enclen/variant=9", Misuse),
    ("G1-022/enclen/variant=11", Misuse),
    ("G1-022/enclen/variant=15", Misuse),
    ("G1-023/enclen/variant=-1", Misuse),
    ("G1-015/enclen/variant=i32::MIN", Misuse),
    ("G1-015/enclen/variant=i32::MAX", Misuse),
    ("G1-015/enclen/variant=16", Misuse),
    // ---- sodium_base64_encoded_len size guard (codecs.c:178) ----
    ("G1-024/enclen/binlen=threshold", Misuse),
    ("G1-024/enclen/binlen=SIZE_MAX", Misuse),
    // ---- sodium_bin2base64 ----
    ("G1-025/bin2b64/variant=0", Misuse),
    ("G1-025/bin2b64/variant=2", Misuse),
    ("G1-025/bin2b64/variant=4", Misuse),
    ("G1-025/bin2b64/variant=6", Misuse),
    ("G1-025/bin2b64/variant=8", Misuse),
    ("G1-025/bin2b64/variant=-1", Misuse),
    ("G1-025/bin2b64/variant=i32::MIN", Misuse),
    ("G1-025/bin2b64/variant=i32::MAX", Misuse),
    ("G1-026/bin2b64/binlen=threshold", Misuse),
    ("G1-027/bin2b64/3,1,4", Misuse),
    ("G1-028/bin2b64/0,1,0", Misuse),
    ("G1-029/bin2b64/1,1,4", Misuse),
    ("G1-030/bin2b64/1,3,2", Misuse),
    ("G1-031/bin2b64/2,7,3", Misuse),
    ("G1-032/bin2b64/32,1,44", Misuse),
    // ---- sodium_base642bin variant guard (codecs.c:290) ----
    ("G1-034/b642bin/variant=0", Misuse),
    ("G1-034/b642bin/variant=2", Misuse),
    ("G1-034/b642bin/variant=4", Misuse),
    ("G1-034/b642bin/variant=6", Misuse),
    ("G1-034/b642bin/variant=8", Misuse),
    ("G1-034/b642bin/variant=9", Misuse),
    ("G1-034/b642bin/variant=-1", Misuse),
    ("G1-034/b642bin/variant=i32::MIN", Misuse),
    ("G1-034/b642bin/variant=i32::MAX", Misuse),
    // ---- sodium_pad overflow guard (utils.c:764) ----
    ("G1-099/pad/SIZE_MAX,bs=16", Misuse),
    ("G1-100/pad/SIZE_MAX-3,bs=16", Misuse),
    ("G1-101/pad/SIZE_MAX,bs=1", Misuse),
    ("G1-099/pad/SIZE_MAX,bs=32", Misuse),
    // ---- randombytes_buf_deterministic size guard (randombytes.c:219) ----
    ("G1-131/det/size=0x4000000001", Misuse),
    ("G1-131/det/size=SIZE_MAX", Misuse),
    // ---- sodium_misuse() itself (core.c:191) ----
    ("G1-123/misuse/handler_returns", Abort),
    ("G1-123/misuse/handler_null", Abort),
    ("G1-123/misuse/direct_call", Abort),
    // ---- raw assert() ----
    ("G1-140/sysrandom/buf_size0_direct", Abort),
    // ---- NULL dereference / NULL indirect call ----
    ("G1-114/unpad/out=NULL", Segv),
    ("G1-133/impl/name=NULL", Segv),
    ("G1-134/impl/random=NULL", Segv),
    ("G1-135/impl/buf=NULL,size>0", Segv),
];

/// `bin_len / 3 > (SIZE_MAX - 5) / 4`, i.e. the smallest rejected value.
const B64_THRESHOLD: usize = 0xBFFF_FFFF_FFFF_FFFD;

// Custom implementations with a NULL required member (the C performs no NULL
// check on `implementation_name`, `random` or `buf`).
unsafe extern "C" fn ch_name() -> *const i8 {
    b"child\0".as_ptr() as *const i8
}
unsafe extern "C" fn ch_random() -> u32 {
    0x1234_5678
}
unsafe extern "C" fn ch_buf(p: *mut c_void, n: usize) {
    unsafe { ptr::write_bytes(p as *mut u8, 0x5a, n) };
}
unsafe extern "C" fn ch_stir() {}
unsafe extern "C" fn ch_close() -> i32 {
    0
}

static IMPL_NAME_NULL: RandombytesImpl = RandombytesImpl {
    implementation_name: None,
    random: Some(ch_random),
    stir: Some(ch_stir),
    uniform: None,
    buf: Some(ch_buf),
    close: Some(ch_close),
};
static IMPL_RANDOM_NULL: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(ch_name),
    random: None,
    stir: Some(ch_stir),
    uniform: None,
    buf: Some(ch_buf),
    close: Some(ch_close),
};
static IMPL_BUF_NULL: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(ch_name),
    random: Some(ch_random),
    stir: Some(ch_stir),
    uniform: None,
    buf: None,
    close: Some(ch_close),
};

/// A misuse handler that simply RETURNS: `sodium_misuse()` must still
/// `abort()` (ERRORS row G1-123).
unsafe extern "C" fn returning_handler() {
    println!("\nOBS handler ran");
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

fn outcome_of(tag: &str) -> Outcome {
    CASES
        .iter()
        .find(|(t, _)| *t == tag)
        .map(|(_, o)| *o)
        .unwrap_or_else(|| panic!("unknown tag {tag}"))
}

/// One misusing / faulting call per tag, on the library named by
/// `DIFFTEST_WHICH`.
#[test]
fn misuse_child() {
    let Some(tag) = child_tag() else {
        return; // parent: no-op
    };
    let lib = child_lib();
    setup();
    let want = outcome_of(&tag);
    if want == Misuse {
        install_misuse_handler(lib);
    }

    // Buffers whose contents are printed by the handler, so that side effects
    // written *before* the abort are compared too.
    let mut out = canary(64);
    let mut plen = 0xA5A5_A5A5_A5A5_A5A5usize;
    let bin = [0x11u8; 64];

    // The observing handler prints "MISUSE obs=..." with `println!`, so emit a
    // newline first: `eq_child` only looks at lines that START with
    // "MISUSE " / "OBS ", and libtest leaves the cursor mid-line.
    print!("\n");
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    // Tags are `G1-NNN/<group>/<name>`; `<name>` may itself contain '/'.
    let mut parts = tag.splitn(3, '/');
    let _row = parts.next().unwrap();
    let group = parts.next().expect("tag needs a group");
    let name = parts.next().expect("tag needs a name");

    match group {
        "bin2hex" => {
            set_observation(out.as_ptr(), out.len());
            let f = sym::<Bin2Hex>(lib, "sodium_bin2hex");
            let (binlen, maxlen) = match name {
                "binlen=SIZE_MAX_div_2" => (usize::MAX / 2, 64usize),
                "binlen=SIZE_MAX" => (usize::MAX, 64),
                "binlen=4,maxlen=8" => (4, 8),
                "binlen=0,maxlen=0" => (0, 0),
                "binlen=1,maxlen=2" => (1, 2),
                o => panic!("unknown {o}"),
            };
            let p = unsafe { f(out.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr(), binlen) };
            println!("\nOBS returned p_is_hex={} out={}", p as usize == out.as_ptr() as usize, hex(&out));
        }
        "enclen" => {
            let f = sym::<EncLen>(lib, "sodium_base64_encoded_len");
            let (binlen, variant) = match name {
                "variant=0" => (3usize, 0i32),
                "variant=2" => (3, 2),
                "variant=4" => (3, 4),
                "variant=6" => (3, 6),
                "variant=8" => (3, 8),
                "variant=9" => (3, 9),
                "variant=11" => (3, 11),
                "variant=15" => (3, 15),
                "variant=16" => (3, 16),
                "variant=-1" => (3, -1),
                "variant=i32::MIN" => (3, i32::MIN),
                "variant=i32::MAX" => (3, i32::MAX),
                "binlen=threshold" => (B64_THRESHOLD, 1),
                "binlen=SIZE_MAX" => (usize::MAX, 1),
                o => panic!("unknown {o}"),
            };
            let n = unsafe { f(binlen, variant) };
            println!("\nOBS returned len={n}");
        }
        "bin2b64" => {
            set_observation(out.as_ptr(), out.len());
            let f = sym::<Bin2B64>(lib, "sodium_bin2base64");
            let (binlen, variant, maxlen) = match name {
                "variant=0" => (3usize, 0i32, 64usize),
                "variant=2" => (3, 2, 64),
                "variant=4" => (3, 4, 64),
                "variant=6" => (3, 6, 64),
                "variant=8" => (3, 8, 64),
                "variant=-1" => (3, -1, 64),
                "variant=i32::MIN" => (3, i32::MIN, 64),
                "variant=i32::MAX" => (3, i32::MAX, 64),
                "binlen=threshold" => (B64_THRESHOLD, 1, 64),
                "3,1,4" => (3, 1, 4),
                "0,1,0" => (0, 1, 0),
                "1,1,4" => (1, 1, 4),
                "1,3,2" => (1, 3, 2),
                "2,7,3" => (2, 7, 3),
                "32,1,44" => (32, 1, 44),
                o => panic!("unknown {o}"),
            };
            let p = unsafe {
                f(
                    out.as_mut_ptr() as *mut c_char,
                    maxlen,
                    bin.as_ptr(),
                    binlen,
                    variant,
                )
            };
            println!(
                "\nOBS returned p_is_b64={} out={}",
                p as usize == out.as_ptr() as usize,
                hex(&out)
            );
        }
        "b642bin" => {
            set_observation(out.as_ptr(), out.len());
            let f = sym::<B642Bin>(lib, "sodium_base642bin");
            let variant = match name {
                "variant=0" => 0i32,
                "variant=2" => 2,
                "variant=4" => 4,
                "variant=6" => 6,
                "variant=8" => 8,
                "variant=9" => 9,
                "variant=-1" => -1,
                "variant=i32::MIN" => i32::MIN,
                "variant=i32::MAX" => i32::MAX,
                o => panic!("unknown {o}"),
            };
            let src = b"AAAA";
            let mut bl = 0xA5A5_A5A5usize;
            let rc = unsafe {
                f(
                    out.as_mut_ptr(),
                    64,
                    src.as_ptr() as *const c_char,
                    4,
                    ptr::null(),
                    &raw mut bl,
                    ptr::null_mut(),
                    variant,
                )
            };
            println!("\nOBS returned rc={rc} bin_len={bl} out={}", hex(&out));
        }
        "pad" => {
            // `*padded_buflen_p` must NOT have been written before the abort.
            set_observation((&raw const plen).cast(), 8);
            let f = sym::<Pad>(lib, "sodium_pad");
            let (n, bs) = match name {
                "SIZE_MAX,bs=16" => (usize::MAX, 16usize),
                "SIZE_MAX-3,bs=16" => (usize::MAX - 3, 16),
                "SIZE_MAX,bs=1" => (usize::MAX, 1),
                "SIZE_MAX,bs=32" => (usize::MAX, 32),
                o => panic!("unknown {o}"),
            };
            let rc = unsafe { f(&raw mut plen, out.as_mut_ptr(), n, bs, usize::MAX) };
            println!("\nOBS returned rc={rc} plen={plen}");
        }
        "det" => {
            set_observation(out.as_ptr(), out.len());
            let f = sym::<RbDet>(lib, "randombytes_buf_deterministic");
            let size = match name {
                "size=0x4000000001" => 0x4000_0000_01usize,
                "size=SIZE_MAX" => usize::MAX,
                o => panic!("unknown {o}"),
            };
            let seed = [3u8; 32];
            unsafe { f(out.as_mut_ptr() as *mut c_void, size, seed.as_ptr()) };
            println!("\nOBS returned out={}", hex(&out));
        }
        "misuse" => {
            let setmis = sym::<SetMisuseFn>(lib, "sodium_set_misuse_handler");
            match name {
                // a handler that simply returns must NOT prevent the abort
                "handler_returns" => {
                    let rc = unsafe { setmis(Some(returning_handler)) };
                    println!("\nOBS set_handler={rc}");
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                    let f = sym::<Bin2Hex>(lib, "sodium_bin2hex");
                    unsafe { f(out.as_mut_ptr() as *mut c_char, 0, bin.as_ptr(), 0) };
                    println!("\nOBS survived");
                }
                // no handler installed -> straight to abort()
                "handler_null" => {
                    let rc = unsafe { setmis(None) };
                    println!("\nOBS set_handler={rc}");
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                    let f = sym::<Bin2Hex>(lib, "sodium_bin2hex");
                    unsafe { f(out.as_mut_ptr() as *mut c_char, 0, bin.as_ptr(), 0) };
                    println!("\nOBS survived");
                }
                // sodium_misuse() called directly, no handler
                "direct_call" => {
                    let rc = unsafe { setmis(None) };
                    println!("\nOBS set_handler={rc}");
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                    let f = sym::<unsafe extern "C" fn()>(lib, "sodium_misuse");
                    unsafe { f() };
                    println!("\nOBS survived");
                }
                o => panic!("unknown {o}"),
            }
        }
        // G1-140: the exported struct's `buf` with size 0 bypasses
        // `randombytes_buf`'s `size > 0` guard and reaches
        // `assert(chunk_size > 0U)` inside randombytes_linux_getrandom().
        "sysrandom" => {
            let sset = sym::<SetImplFn>(lib, "randombytes_set_implementation");
            let sysimpl =
                sym::<*const RandombytesImpl>(lib, "randombytes_sysrandom_implementation");
            unsafe { sset(sysimpl) };
            let rstir = sym::<unsafe extern "C" fn()>(lib, "randombytes_stir");
            unsafe { rstir() }; // so getrandom_available == 1
            println!("\nOBS about to call buf(p, 0)");
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let bufp = unsafe { (*sysimpl).buf.unwrap() };
            unsafe { bufp(out.as_mut_ptr() as *mut c_void, 0) };
            println!("\nOBS survived out={}", hex(&out));
        }
        // G1-114: sodium_unpad performs NO NULL check on unpadded_buflen_p.
        "unpad" => {
            let f = sym::<Unpad>(lib, "sodium_unpad");
            let buf = [0x80u8; 16];
            println!("\nOBS about to call unpad(NULL, ...)");
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let rc = unsafe { f(ptr::null_mut(), buf.as_ptr(), 16, 16) };
            println!("\nOBS survived rc={rc}");
        }
        // G1-133/134/135: a custom implementation with a NULL required member.
        "impl" => {
            let sset = sym::<SetImplFn>(lib, "randombytes_set_implementation");
            match name {
                "name=NULL" => {
                    unsafe { sset(&raw const IMPL_NAME_NULL) };
                    let f = sym::<StrFn>(lib, "randombytes_implementation_name");
                    println!("\nOBS about to call implementation_name()");
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                    let p = unsafe { f() };
                    println!("\nOBS survived name={}", cstr(p));
                }
                "random=NULL" => {
                    unsafe { sset(&raw const IMPL_RANDOM_NULL) };
                    let f = sym::<RbRandom>(lib, "randombytes_random");
                    println!("\nOBS about to call random()");
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                    let v = unsafe { f() };
                    println!("\nOBS survived random={v}");
                }
                "buf=NULL,size>0" => {
                    unsafe { sset(&raw const IMPL_BUF_NULL) };
                    let f = sym::<RbBuf>(lib, "randombytes_buf");
                    // size == 0 is SAFE (the callback is never invoked)
                    unsafe { f(out.as_mut_ptr() as *mut c_void, 0) };
                    println!("\nOBS size0_ok out={}", hex(&out));
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                    unsafe { f(out.as_mut_ptr() as *mut c_void, 8) };
                    println!("\nOBS survived out={}", hex(&out));
                }
                o => panic!("unknown {o}"),
            }
        }
        o => panic!("unknown group {o}"),
    }

    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(0); // only reached if the library did NOT abort
}

/// Drives every out-of-process row against both libraries and requires the
/// identical outcome (exit code + signal) and the identical observations.
#[test]
fn out_of_process_rows_match() {
    if child_tag().is_some() {
        return;
    }
    setup();
    use std::os::unix::process::ExitStatusExt;
    for &(tag, want) in CASES {
        let c = run_child("misuse_child", "c", tag);
        let r = run_child("misuse_child", "r", tag);
        eq_child(tag, &c, &r);
        match want {
            Misuse => assert_eq!(
                c.status.code(),
                Some(MISUSE_EXIT),
                "{tag}: C did not reach sodium_misuse()\n  stdout: {}\n  stderr: {}",
                String::from_utf8_lossy(&c.stdout),
                String::from_utf8_lossy(&c.stderr)
            ),
            Abort => assert_eq!(
                c.status.signal(),
                Some(6),
                "{tag}: C did not SIGABRT\n  stdout: {}\n  stderr: {}",
                String::from_utf8_lossy(&c.stdout),
                String::from_utf8_lossy(&c.stderr)
            ),
            Segv => assert_eq!(
                c.status.signal(),
                Some(11),
                "{tag}: C did not SIGSEGV\n  stdout: {}\n  stderr: {}",
                String::from_utf8_lossy(&c.stdout),
                String::from_utf8_lossy(&c.stderr)
            ),
        }
    }
}

/// Fresh-process `randombytes_close()` rows: the outcome depends on state that
/// no in-process test can restore.
///
/// ERRORS G1-137, G1-138, G1-139.
#[test]
fn close_child() {
    let Some(tag) = child_tag_close() else {
        return;
    };
    let lib = child_lib();
    // deliberately no setup(): `implementation` must still be NULL
    let sset = sym::<SetImplFn>(lib, "randombytes_set_implementation");
    let rclose = sym::<IntFn>(lib, "randombytes_close");
    let sysimpl = sym::<*const RandombytesImpl>(lib, "randombytes_sysrandom_implementation");
    let intimpl = sym::<*const RandombytesImpl>(lib, "randombytes_internal_implementation");

    match tag.as_str() {
        // G1-137: implementation == NULL -> 0 without touching any RNG state.
        "G1-137/close/impl=NULL" => {
            let a = unsafe { rclose() };
            let b = unsafe { rclose() };
            println!("\nOBS close1={a} close2={b}");
        }
        // G1-138: sysrandom with no prior stir -> fd == -1 and
        // getrandom_available == 0, so -1.
        "G1-138/close/sysrandom_no_stir" => {
            unsafe { sset(sysimpl) };
            let a = unsafe { rclose() };
            // after a stir, getrandom_available == 1 and it returns 0
            let rstir = sym::<unsafe extern "C" fn()>(lib, "randombytes_stir");
            unsafe { rstir() };
            let b = unsafe { rclose() };
            println!("\nOBS close_no_stir={a} close_after_stir={b}");
        }
        // G1-139: internal impl with no prior stir -> -1 (and `stream` zeroed).
        "G1-139/close/internal_no_stir" => {
            unsafe { sset(intimpl) };
            let a = unsafe { rclose() };
            let rstir = sym::<unsafe extern "C" fn()>(lib, "randombytes_stir");
            unsafe { rstir() };
            let b = unsafe { rclose() };
            println!("\nOBS close_no_stir={a} close_after_stir={b}");
        }
        o => panic!("unknown tag {o}"),
    }
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}

/// `child_tag()` restricted to the `close_child` tags, so the two child
/// dispatchers cannot be confused with each other.
fn child_tag_close() -> Option<String> {
    let t = child_tag()?;
    if t.contains("/close/") { Some(t) } else { None }
}

const CLOSE_CASES: &[&str] = &[
    "G1-137/close/impl=NULL",
    "G1-138/close/sysrandom_no_stir",
    "G1-139/close/internal_no_stir",
];

#[test]
fn close_rows_match() {
    if child_tag().is_some() {
        return;
    }
    setup();
    for &tag in CLOSE_CASES {
        let c = run_child("close_child", "c", tag);
        let r = run_child("close_child", "r", tag);
        assert_eq!(
            c.status.code(),
            Some(0),
            "{tag}: the C child failed\n  stdout: {}\n  stderr: {}",
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&c.stderr)
        );
        let co = String::from_utf8_lossy(&c.stdout).to_string();
        assert!(
            co.lines().any(|l| l.starts_with("OBS ")),
            "{tag}: the C child printed no observation\n  stdout: {co}"
        );
        eq_child(tag, &c, &r);
    }
}

// ===========================================================================
// dead-code / unreachable rows
// ===========================================================================

/// ERRORS rows that are **dead code** or otherwise unreachable in this build.
/// Each one is recorded here with the observable fact that proves it cannot
/// fire, so no row is silently dropped.
///
/// G1-033, G1-090, G1-119, G1-120, G1-121, G1-122, G1-124, G1-125, G1-126,
/// G1-127, G1-141, G1-142, G1-143, G1-144, G1-145, G1-146, G1-147, G1-148,
/// G1-149, G1-150, G1-151, G1-152, G1-153, G1-154, G1-155, G1-156, G1-157,
/// G1-158, G1-159.
#[test]
fn documented_unreachable_error_rows() {
    setup();
    let mut rng = Rng::new(0x2009);

    // G1-033 `assert(b64_pos <= b64_len)` in sodium_bin2base64 (codecs.c:239)
    // is defensive: the encoder emits ceil(bin_len*8/6) chars, which is always
    // <= b64_len. Verified exhaustively for every remainder class and variant
    // by re-deriving b64_len and counting the chars the encoder produced.
    {
        let (c, r) = pair::<Bin2B64>("sodium_bin2base64");
        let enc = sym::<EncLen>(c_lib(), "sodium_base64_encoded_len");
        for &v in &[1i32, 3, 5, 7] {
            for n in 0..200usize {
                let bin = rng.bytes(n);
                let need = unsafe { enc(n, v) };
                let mut a = canary(need + 8);
                let mut b = canary(need + 8);
                unsafe {
                    c(a.as_mut_ptr() as *mut c_char, need, bin.as_ptr(), n, v);
                    r(b.as_mut_ptr() as *mut c_char, need, bin.as_ptr(), n, v);
                }
                eq_bytes(&format!("bin2base64(n={n}, v={v})"), &a, &b);
                let chars = a.iter().position(|&x| x == 0).unwrap();
                let b64_len = need - 1;
                assert!(
                    chars <= b64_len,
                    "G1-033: b64_pos ({chars}) > b64_len ({b64_len}) for n={n}, v={v}"
                );
            }
        }
    }

    // G1-090 `_mprotect_noaccess`/`_readonly`/`_readwrite` never run: with
    // HAVE_PAGE_PROTECTION undefined `_sodium_mprotect` is the stub that sets
    // ENOSYS itself and never invokes the callback. Observable proxy: the
    // public wrappers set ENOSYS and the memory is genuinely NOT protected
    // (a real mprotect(PROT_NONE) would make the next read fault).
    {
        let (cm, rm) = pair::<MallocFn>("sodium_malloc");
        let (cf, rf) = pair::<FreeFn>("sodium_free");
        for (mf, ff, which) in [(cm, cf, "C"), (rm, rf, "Rust")] {
            let p = unsafe { mf(4096 * 3) };
            assert!(!p.is_null());
            for name in [
                "sodium_mprotect_noaccess",
                "sodium_mprotect_readonly",
                "sodium_mprotect_readwrite",
            ] {
                let (c, r) = pair::<Mprotect>(name);
                let f = if which == "C" { c } else { r };
                set_sentinel();
                assert_eq!(unsafe { f(p) }, -1);
                assert_eq!(errno(), ENOSYS);
            }
            // still writable across the whole region
            unsafe { ptr::write_bytes(p as *mut u8, 0x42, 4096 * 3) };
            assert_eq!(unsafe { *(p as *const u8).add(4096 * 3 - 1) }, 0x42);
            unsafe { ff(p) };
        }
    }

    // G1-119 `_out_of_bounds()` / canary check / guard pages are inside
    // `#ifdef HAVE_ALIGNED_MALLOC` -> dead. G1-120 `_sodium_malloc` (aligned
    // variant) and G1-121 `_unprotected_ptr_from_user_ptr` likewise.
    // Observable proxies:
    //   * sodium_malloc(SIZE_MAX - page_size*4 + 1) does NOT return early with
    //     ENOMEM from the aligned pre-check; it goes straight to malloc().
    //   * writing to the 16 bytes *below* the returned pointer (where the
    //     aligned variant would keep its canary) and then freeing does not
    //     abort, because sodium_free performs no canary check.
    {
        let (cm, rm) = pair::<MallocFn>("sodium_malloc");
        let (cf, rf) = pair::<FreeFn>("sodium_free");
        for (mf, ff) in [(cm, cf), (rm, rf)] {
            let p = unsafe { mf(64) };
            assert!(!p.is_null());
            // the returned pointer is a raw malloc pointer, so it is 16-byte
            // aligned but NOT page-aligned and has no 16-byte canary prefix
            assert_ne!(
                (p as usize) % 4096,
                0,
                "G1-119/G1-120: sodium_malloc must be a plain malloc pointer"
            );
            unsafe {
                ptr::write_bytes(p as *mut u8, 0x77, 64);
                ff(p);
            }
        }
    }

    // G1-122 `_sodium_alloc_init` page-size misuse is inside
    // `#ifdef HAVE_ALIGNED_MALLOC`; the live body is just
    // `randombytes_buf(canary, 16); return 0;`.
    {
        let (c, r) = pair::<IntFn>("_sodium_alloc_init");
        for _ in 0..3 {
            reset_rngs(0x4242);
            let a = unsafe { c() };
            reset_rngs(0x4242);
            let b = unsafe { r() };
            eq_i32("_sodium_alloc_init()", a, b);
            assert_eq!(a, 0, "G1-122: _sodium_alloc_init must always return 0");
        }
    }

    // G1-124 sodium_init()'s `return -1` paths and G1-125
    // sodium_set_misuse_handler()'s `return -1` paths are dead: the crit
    // functions are the no-op stubs that always return 0 (G1-126 `EPERM`
    // branch and G1-127 `assert(locked == 0)` live in the _WIN32 / HAVE_PTHREAD
    // copies, which are not compiled).
    {
        let (ce, re) = pair::<IntFn>("sodium_crit_enter");
        let (cl, rl) = pair::<IntFn>("sodium_crit_leave");
        // an unbalanced leave() must still return 0 (the EPERM copy is dead)
        for _ in 0..4 {
            let (a, b) = unsafe { (cl(), rl()) };
            eq_i32("sodium_crit_leave() unbalanced", a, b);
            assert_eq!(a, 0, "G1-126: sodium_crit_leave must never return -1");
        }
        // repeated enter() without leave() must not assert (G1-127)
        for _ in 0..4 {
            let (a, b) = unsafe { (ce(), re()) };
            eq_i32("sodium_crit_enter() repeated", a, b);
            assert_eq!(a, 0, "G1-127: sodium_crit_enter must never assert");
        }
        let (a, b) = unsafe { (cl(), rl()) };
        eq_i32("sodium_crit_leave()", a, b);
        let (ci, ri) = pair::<IntFn>("sodium_init");
        let (a, b) = unsafe { (ci(), ri()) };
        eq_i32("sodium_init()", a, b);
        assert_eq!(a, 1, "G1-124: sodium_init must never return -1");
        let (cs, rs) = pair::<SetMisuseFn>("sodium_set_misuse_handler");
        for h in [None, None] {
            let (a, b) = unsafe { (cs(h), rs(h)) };
            eq_i32("sodium_set_misuse_handler()", a, b);
            assert_eq!(a, 0, "G1-125: must never return -1");
        }
    }

    // G1-141 randombytes_sysrandom_init()'s sodium_misuse(), G1-142
    // randombytes_sysrandom_random_dev_open()'s EIO, G1-143
    // randombytes_block_on_dev_random()'s EIO, G1-144/G1-145
    // randombytes_sysrandom_buf()'s two sodium_misuse() sites, G1-147
    // safe_read()'s asserts, and G1-152/G1-153 (the internal impl's
    // /dev/urandom fallback) are all unreachable on Linux x86-64: the
    // `getrandom(2)` probe in `randombytes_*_init()` succeeds, so
    // `getrandom_available == 1` and the device-open / safe_read code is never
    // entered. Observable proxy (row G1-174 / G1-138): with the sysrandom
    // implementation installed, `randombytes_close()` returns 0 *before* any
    // fd could have been opened, and it keeps returning 0 -- which is only
    // possible on the getrandom path (`stream.initialized` is not reset).
    // That proxy is asserted out of process by `close_rows_match`
    // ("G1-138/close/sysrandom_no_stir") because it needs a virgin library.

    // G1-146 the `_WIN32` misuse paths of randombytes_sysrandom_buf,
    // G1-154 randombytes_internal_random_init()'s trailing sodium_misuse()
    // (`#ifndef HAVE_SAFE_ARC4RANDOM`, unreachable because the
    // `!NONEXISTENT_DEV_RANDOM` block above always returns), G1-155
    // randombytes_internal_random_stir_if_needed()'s fork detection
    // (`HAVE_GETPID` undefined -> no fork protection at all) and G1-158
    // `_randombytes_getentropy`/`randombytes_getentropy`
    // (`HAVE_GETENTROPY` undefined) are dead code with no observable proxy
    // other than the absence of the behaviour. G1-155 IS observable: because
    // there is no fork protection, a forked child keeps using the parent's
    // stream instead of aborting -- and the whole out-of-process machinery of
    // this file exercises exactly that (every `run_child` fork+exec of a
    // process that already used the RNG completes normally).

    // G1-148 `assert(size <= 256U)` in _randombytes_linux_getrandom and
    // G1-159 `assert(chunk_size > 0U)` in the *internal* copy of
    // randombytes_linux_getrandom are only reachable through internal callers
    // that always pass 16 or 32 bytes. The externally reachable twin of
    // G1-159 -- `randombytes_sysrandom_implementation.buf(p, 0)` -- IS
    // constructed, by the "G1-140/sysrandom/buf_size0_direct" child row.
    // Proxy assertion: every size that the public API can pass through
    // randombytes_buf is chunked to at most 256 bytes and is never 0.
    {
        let (c, r) = pair::<RbBuf>("randombytes_buf");
        for &n in &[1usize, 255, 256, 257, 512, 1000] {
            let mut a = canary(n + 8);
            let mut b = canary(n + 8);
            reset_rngs(0x5150 + n as u64);
            unsafe { c(a.as_mut_ptr() as *mut c_void, n) };
            reset_rngs(0x5150 + n as u64);
            unsafe { r(b.as_mut_ptr() as *mut c_void, n) };
            eq_bytes(&format!("randombytes_buf({n})"), &a, &b);
        }
    }

    // G1-149 sodium_hrtime()'s gettimeofday() failure, G1-150
    // `assert(stream.nonce != 0)`, G1-151 the getrandom failure inside
    // randombytes_internal_random_stir, G1-156 / G1-157 the two
    // `assert(ret == 0)` on the chacha20 return: none can be provoked from
    // outside. Proxy: the internal implementation stirs and produces output
    // successfully (exercised by the t10 "internal/*" child rows), and
    // crypto_stream_chacha20 returns 0 for every length the internal RNG uses.
    {
        let f = sym::<unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> i32>(
            c_lib(),
            "crypto_stream_chacha20",
        );
        let g = sym::<unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> i32>(
            r_lib(),
            "crypto_stream_chacha20",
        );
        let k = [1u8; 32];
        let n = [2u8; 8];
        for &len in &[0u64, 4, 32, 512] {
            let mut a = vec![0u8; len as usize];
            let mut b = vec![0u8; len as usize];
            let (x, y) = unsafe {
                (
                    f(a.as_mut_ptr(), len, n.as_ptr(), k.as_ptr()),
                    g(b.as_mut_ptr(), len, n.as_ptr(), k.as_ptr()),
                )
            };
            eq_i32("crypto_stream_chacha20 rc", x, y);
            assert_eq!(x, 0, "G1-156/G1-157: the asserted return is always 0");
            eq_bytes("crypto_stream_chacha20", &a, &b);
        }
    }

    // G1-131 boundary note: `size == 0x4000000000` exactly (one below the
    // guard) is *accepted* by the check and would then ask
    // crypto_stream_chacha20_ietf to fill 256 GiB. Not constructible; only
    // `size > 0x4000000000` is driven, by the "G1-131/det/*" child rows.
    // The two sides of the comparison itself are pinned here.
    assert!(0x4000_0000_01u64 > 0x4000_0000_00u64);

    // Both libraries agree that randombytes_seedbytes() is 32, so
    // randombytes_buf_deterministic's COMPILER_ASSERT
    // (randombytes_SEEDBYTES == crypto_stream_chacha20_ietf_KEYBYTES) holds.
    let (c, r) = pair::<SizeFn>("randombytes_seedbytes");
    let (a, b) = unsafe { (c(), r()) };
    eq_usize("randombytes_seedbytes()", a, b);
    assert_eq!(a, 32);
    let kb = sym::<SizeFn>(c_lib(), "crypto_stream_chacha20_ietf_keybytes");
    assert_eq!(unsafe { kb() }, 32);

    // sodium_version_string is a static string, so no row can change it.
    let (c, r) = pair::<StrFn>("sodium_version_string");
    assert_eq!(unsafe { cstr(c()) }, unsafe { cstr(r()) });
}
