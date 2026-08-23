//! t01_codecs.rs — C-vs-Rust differential verification of
//! `c_src/libsodium/sodium/codecs.c`.
//!
//! Specification: `CONFIGS.md` rows 1–29 (valid-input configuration surface) and
//! `ERRORS.md` rows 1–47 (rejection surface).
//!
//! Every call goes through `dlsym` on BOTH shared objects; no Rust function is
//! ever called directly, so the `#[no_mangle]` export wrappers are under test.
//!
//! IMPORTANT — how to run this:
//! ```text
//! cargo build && cargo test --test t01_codecs
//! ```
//! `cargo test` alone does NOT rebuild `target/debug/liblibsodium.so`: the crate
//! is `crate-type = ["cdylib"]`, and an integration test target does not depend
//! on the cdylib artifact, so `cargo test` happily runs against a stale `.so`.
//! Always `cargo build` first, or a change to `src/` will not be under test.
//!
//! Conventions used throughout:
//!   * every output buffer is prefilled with `SENTINEL` (0xAA) and the FULL
//!     buffer (including the slack past the declared max length) is compared,
//!     so any out-of-range write diverges;
//!   * `errno` is primed with `ERRNO_MARK` before each call and compared after,
//!     which simultaneously covers the "errno set" rows and the rows where the
//!     C deliberately leaves errno UNCHANGED (ERRORS 18/19);
//!   * `misuse` rows run the call in a forked child on each library and require
//!     an identical `SIGABRT`.

mod common;
use common::*;
use libc::{c_char, c_int};

// --------------------------------------------------------------------- config

const SENTINEL: u8 = 0xAA;
/// An errno value libsodium never sets, so "unchanged" is observable.
const ERRNO_MARK: c_int = 0x5EED;

const V_ORIG: c_int = 1; // sodium_base64_VARIANT_ORIGINAL
const V_ORIG_NP: c_int = 3; // sodium_base64_VARIANT_ORIGINAL_NO_PADDING
const V_URL: c_int = 5; // sodium_base64_VARIANT_URLSAFE
const V_URL_NP: c_int = 7; // sodium_base64_VARIANT_URLSAFE_NO_PADDING
const VARIANTS: [c_int; 4] = [V_ORIG, V_ORIG_NP, V_URL, V_URL_NP];
/// ERRORS row 11 / task requirement: out-of-range enum values crossing FFI.
const BAD_VARIANTS: [c_int; 8] = [0, 2, 4, 6, 8, 9, 99, 0xFFFF_FFFFu32 as c_int];

/// CONFIGS rows 1/2 `bin_len` set.
const HEX_LENS: [usize; 7] = [0, 1, 2, 3, 16, 64, 255];
/// CONFIGS rows 11–14 `bin_len` set.
const B64_LENS: [usize; 13] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 32, 33, 64, 255];

// ----------------------------------------------------------------- signatures

type FBin2Hex = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;
type FHex2Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
) -> c_int;
type FEncLen = unsafe extern "C" fn(usize, c_int) -> usize;
type FBin2B64 = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char;
type FB642Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut usize,
    *mut *const c_char,
    c_int,
) -> c_int;
type FIp2Bin = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> c_int;
type FBin2Ip = unsafe extern "C" fn(*mut c_char, usize, *const u8) -> *mut c_char;

// -------------------------------------------------------------------- helpers

fn set_errno(v: c_int) {
    unsafe { *libc::__errno_location() = v };
}
fn get_errno() -> c_int {
    unsafe { *libc::__errno_location() }
}

/// NUL-terminated byte vector for `ignore` / `ip` arguments.
fn cs(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b {
        if (0x20..0x7f).contains(&c) {
            s.push(c as char);
        } else {
            s.push_str(&format!("\\x{c:02x}"));
        }
    }
    s
}

/// Translate a fixed ORIGINAL-alphabet vector into the URLSAFE alphabet so the
/// same shape can be fed to both padded variants.
fn tr(s: &[u8], variant: c_int) -> Vec<u8> {
    if (variant as u32) & 4 != 0 {
        s.iter()
            .map(|&c| match c {
                b'/' => b'_',
                b'+' => b'-',
                x => x,
            })
            .collect()
    } else {
        s.to_vec()
    }
}

fn to_hex(bin: &[u8], upper: bool) -> Vec<u8> {
    let d: &[u8; 16] = if upper {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let mut v = Vec::with_capacity(bin.len() * 2);
    for &b in bin {
        v.push(d[(b >> 4) as usize]);
        v.push(d[(b & 0xf) as usize]);
    }
    v
}

fn to_hex_mixed(bin: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::with_capacity(bin.len() * 2);
    for &b in bin {
        for nib in [b >> 4, b & 0xf] {
            let up = rng.byte() & 1 == 1;
            v.push(if nib < 10 {
                b'0' + nib
            } else if up {
                b'A' + nib - 10
            } else {
                b'a' + nib - 10
            });
        }
    }
    v
}

// ============================================================== hex2bin driver

#[derive(Debug, Clone, PartialEq, Eq)]
struct H2b {
    ret: c_int,
    bin: Vec<u8>,
    bin_len: Option<usize>,
    hex_end: Option<isize>,
    errno: c_int,
}

#[derive(Clone)]
struct H2bCall<'a> {
    bin_cap: usize,
    bin_maxlen: usize,
    hex: &'a [u8],
    hex_len: usize,
    ignore: Option<&'a [u8]>,
    want_bin_len: bool,
    want_hex_end: bool,
    errno_pre: c_int,
}

impl<'a> H2bCall<'a> {
    fn new(hex: &'a [u8], bin_maxlen: usize) -> Self {
        H2bCall {
            bin_cap: bin_maxlen + 8,
            bin_maxlen,
            hex,
            hex_len: hex.len(),
            ignore: None,
            want_bin_len: true,
            want_hex_end: false,
            errno_pre: ERRNO_MARK,
        }
    }
    fn ig(mut self, i: &'a [u8]) -> Self {
        self.ignore = Some(i);
        self
    }
    fn hex_end(mut self, v: bool) -> Self {
        self.want_hex_end = v;
        self
    }
    fn bin_len_out(mut self, v: bool) -> Self {
        self.want_bin_len = v;
        self
    }
    fn hex_len(mut self, v: usize) -> Self {
        self.hex_len = v;
        self
    }

    fn run(&self, f: FHex2Bin) -> H2b {
        assert!(self.bin_cap >= self.bin_maxlen, "test bug: bin_cap too small");
        assert!(self.hex_len <= self.hex.len(), "test bug: hex_len past buffer");
        let mut bin = vec![SENTINEL; self.bin_cap];
        let mut bl: usize = 0xDEAD_BEEF;
        let mut he: *const c_char = 1usize as *const c_char;
        let ig = self
            .ignore
            .map_or(std::ptr::null(), |i| i.as_ptr() as *const c_char);
        set_errno(self.errno_pre);
        let ret = unsafe {
            f(
                bin.as_mut_ptr(),
                self.bin_maxlen,
                self.hex.as_ptr() as *const c_char,
                self.hex_len,
                ig,
                if self.want_bin_len {
                    &mut bl
                } else {
                    std::ptr::null_mut()
                },
                if self.want_hex_end {
                    &mut he
                } else {
                    std::ptr::null_mut()
                },
            )
        };
        let e = get_errno();
        H2b {
            ret,
            bin,
            bin_len: if self.want_bin_len { Some(bl) } else { None },
            hex_end: if self.want_hex_end {
                Some(he as isize - self.hex.as_ptr() as isize)
            } else {
                None
            },
            errno: e,
        }
    }

    fn ctx(&self, what: &str) -> String {
        format!(
            "{what} [sodium_hex2bin hex=\"{}\" hex_len={} bin_maxlen={} ignore={:?} bin_len_p={} hex_end_p={}]",
            show(self.hex),
            self.hex_len,
            self.bin_maxlen,
            self.ignore.map(show),
            self.want_bin_len,
            self.want_hex_end
        )
    }

    /// Run on both libraries and compare EVERYTHING; returns the C result.
    fn check(&self, what: &str, fc: FHex2Bin, fr: FHex2Bin) -> H2b {
        let c = self.run(fc);
        let r = self.run(fr);
        let ctx = self.ctx(what);
        assert_eq!(c.ret, r.ret, "{ctx}: RETURN C={} rust={}", c.ret, r.ret);
        assert_eq_bytes(&format!("{ctx} bin buffer"), &c.bin, &r.bin);
        assert_eq!(
            c.bin_len, r.bin_len,
            "{ctx}: *bin_len C={:?} rust={:?}",
            c.bin_len, r.bin_len
        );
        assert_eq!(
            c.hex_end, r.hex_end,
            "{ctx}: *hex_end offset C={:?} rust={:?}",
            c.hex_end, r.hex_end
        );
        assert_eq!(c.errno, r.errno, "{ctx}: errno C={} rust={}", c.errno, r.errno);
        c
    }
}

// =========================================================== base642bin driver

#[derive(Debug, Clone, PartialEq, Eq)]
struct B2b {
    ret: c_int,
    bin: Vec<u8>,
    bin_len: Option<usize>,
    b64_end: Option<isize>,
    errno: c_int,
}

#[derive(Clone)]
struct B2bCall<'a> {
    bin_cap: usize,
    bin_maxlen: usize,
    b64: &'a [u8],
    b64_len: usize,
    ignore: Option<&'a [u8]>,
    want_bin_len: bool,
    want_b64_end: bool,
    variant: c_int,
    errno_pre: c_int,
}

impl<'a> B2bCall<'a> {
    fn new(b64: &'a [u8], bin_maxlen: usize, variant: c_int) -> Self {
        B2bCall {
            bin_cap: bin_maxlen + 8,
            bin_maxlen,
            b64,
            b64_len: b64.len(),
            ignore: None,
            want_bin_len: true,
            want_b64_end: false,
            variant,
            errno_pre: ERRNO_MARK,
        }
    }
    fn ig(mut self, i: &'a [u8]) -> Self {
        self.ignore = Some(i);
        self
    }
    fn b64_end(mut self, v: bool) -> Self {
        self.want_b64_end = v;
        self
    }
    fn bin_len_out(mut self, v: bool) -> Self {
        self.want_bin_len = v;
        self
    }
    fn b64_len(mut self, v: usize) -> Self {
        self.b64_len = v;
        self
    }

    fn run(&self, f: FB642Bin) -> B2b {
        assert!(self.bin_cap >= self.bin_maxlen, "test bug: bin_cap too small");
        assert!(self.b64_len <= self.b64.len(), "test bug: b64_len past buffer");
        let mut bin = vec![SENTINEL; self.bin_cap];
        let mut bl: usize = 0xDEAD_BEEF;
        let mut be: *const c_char = 1usize as *const c_char;
        let ig = self
            .ignore
            .map_or(std::ptr::null(), |i| i.as_ptr() as *const c_char);
        set_errno(self.errno_pre);
        let ret = unsafe {
            f(
                bin.as_mut_ptr(),
                self.bin_maxlen,
                self.b64.as_ptr() as *const c_char,
                self.b64_len,
                ig,
                if self.want_bin_len {
                    &mut bl
                } else {
                    std::ptr::null_mut()
                },
                if self.want_b64_end {
                    &mut be
                } else {
                    std::ptr::null_mut()
                },
                self.variant,
            )
        };
        let e = get_errno();
        B2b {
            ret,
            bin,
            bin_len: if self.want_bin_len { Some(bl) } else { None },
            b64_end: if self.want_b64_end {
                Some(be as isize - self.b64.as_ptr() as isize)
            } else {
                None
            },
            errno: e,
        }
    }

    fn ctx(&self, what: &str) -> String {
        format!(
            "{what} [sodium_base642bin b64=\"{}\" b64_len={} bin_maxlen={} ignore={:?} variant={} bin_len_p={} b64_end_p={}]",
            show(self.b64),
            self.b64_len,
            self.bin_maxlen,
            self.ignore.map(show),
            self.variant,
            self.want_bin_len,
            self.want_b64_end
        )
    }

    fn check(&self, what: &str, fc: FB642Bin, fr: FB642Bin) -> B2b {
        let c = self.run(fc);
        let r = self.run(fr);
        let ctx = self.ctx(what);
        assert_eq!(c.ret, r.ret, "{ctx}: RETURN C={} rust={}", c.ret, r.ret);
        assert_eq_bytes(&format!("{ctx} bin buffer"), &c.bin, &r.bin);
        assert_eq!(
            c.bin_len, r.bin_len,
            "{ctx}: *bin_len C={:?} rust={:?}",
            c.bin_len, r.bin_len
        );
        assert_eq!(
            c.b64_end, r.b64_end,
            "{ctx}: *b64_end offset C={:?} rust={:?}",
            c.b64_end, r.b64_end
        );
        assert_eq!(c.errno, r.errno, "{ctx}: errno C={} rust={}", c.errno, r.errno);
        c
    }
}

// ================================================= bin2hex / bin2base64 driver

/// Returns the C output buffer (full `cap` bytes) after asserting parity.
fn bin2hex_check(
    what: &str,
    fc: FBin2Hex,
    fr: FBin2Hex,
    cap: usize,
    hex_maxlen: usize,
    bin: &[u8],
) -> Vec<u8> {
    assert!(cap >= hex_maxlen, "test bug: cap too small");
    let run = |f: FBin2Hex| -> (Vec<u8>, bool, c_int) {
        let mut buf = vec![SENTINEL; cap];
        let p = buf.as_mut_ptr() as *mut c_char;
        set_errno(ERRNO_MARK);
        let ret = unsafe { f(p, hex_maxlen, bin.as_ptr(), bin.len()) };
        (buf, ret == p, get_errno())
    };
    let (bc, okc, ec) = run(fc);
    let (br, okr, er) = run(fr);
    let ctx = format!(
        "{what} [sodium_bin2hex bin_len={} hex_maxlen={hex_maxlen}]",
        bin.len()
    );
    assert!(okc, "{ctx}: C did not return the `hex` argument");
    assert!(okr, "{ctx}: rust did not return the `hex` argument");
    assert_eq_bytes(&ctx, &bc, &br);
    assert_eq!(ec, er, "{ctx}: errno C={ec} rust={er}");
    bc
}

fn bin2b64_check(
    what: &str,
    fc: FBin2B64,
    fr: FBin2B64,
    cap: usize,
    b64_maxlen: usize,
    bin: &[u8],
    variant: c_int,
) -> Vec<u8> {
    assert!(cap >= b64_maxlen, "test bug: cap too small");
    let run = |f: FBin2B64| -> (Vec<u8>, bool, c_int) {
        let mut buf = vec![SENTINEL; cap];
        let p = buf.as_mut_ptr() as *mut c_char;
        set_errno(ERRNO_MARK);
        let ret = unsafe { f(p, b64_maxlen, bin.as_ptr(), bin.len(), variant) };
        (buf, ret == p, get_errno())
    };
    let (bc, okc, ec) = run(fc);
    let (br, okr, er) = run(fr);
    let ctx = format!(
        "{what} [sodium_bin2base64 bin_len={} b64_maxlen={b64_maxlen} variant={variant}]",
        bin.len()
    );
    assert!(okc, "{ctx}: C did not return the `b64` argument");
    assert!(okr, "{ctx}: rust did not return the `b64` argument");
    // Whole-buffer comparison: the C zero-fills b64[b64_len..b64_maxlen).
    assert_eq_bytes(&ctx, &bc, &br);
    assert_eq!(ec, er, "{ctx}: errno C={ec} rust={er}");
    bc
}

// =============================================================== IP drivers

fn ip2bin_check(what: &str, fc: FIp2Bin, fr: FIp2Bin, ip: &[u8], ip_len: usize) -> (c_int, Vec<u8>) {
    assert!(ip_len <= ip.len(), "test bug: ip_len past buffer");
    let run = |f: FIp2Bin| -> (c_int, Vec<u8>, c_int) {
        let mut buf = vec![SENTINEL; 24]; // 16 out + 8 guard
        set_errno(ERRNO_MARK);
        let ret = unsafe { f(buf.as_mut_ptr(), ip.as_ptr() as *const c_char, ip_len) };
        (ret, buf, get_errno())
    };
    let (rc, bc, ec) = run(fc);
    let (rr, br, er) = run(fr);
    let ctx = format!(
        "{what} [sodium_ip2bin ip=\"{}\" ip_len_={ip_len}]",
        show(&ip[..ip_len.min(ip.len())])
    );
    assert_eq!(rc, rr, "{ctx}: RETURN C={rc} rust={rr}");
    assert_eq_bytes(&format!("{ctx} bin[16]+guard"), &bc, &br);
    assert_eq!(ec, er, "{ctx}: errno C={ec} rust={er}");
    (rc, bc)
}

fn bin2ip_check(
    what: &str,
    fc: FBin2Ip,
    fr: FBin2Ip,
    bin: &[u8],
    ip_maxlen: usize,
    cap: usize,
) -> (bool, Vec<u8>) {
    assert!(bin.len() >= 16, "test bug: bin must be 16 bytes");
    assert!(cap >= ip_maxlen, "test bug: cap too small");
    let run = |f: FBin2Ip| -> (bool, Vec<u8>, c_int) {
        let mut buf = vec![SENTINEL; cap];
        let p = buf.as_mut_ptr() as *mut c_char;
        set_errno(ERRNO_MARK);
        let ret = unsafe { f(p, ip_maxlen, bin.as_ptr()) };
        assert!(
            ret.is_null() || ret == p,
            "sodium_bin2ip returned a pointer that is neither NULL nor `ip`"
        );
        (!ret.is_null(), buf, get_errno())
    };
    let (okc, bc, ec) = run(fc);
    let (okr, br, er) = run(fr);
    let ctx = format!(
        "{what} [sodium_bin2ip bin={} ip_maxlen={ip_maxlen}]",
        hexs(&bin[..16])
    );
    assert_eq!(okc, okr, "{ctx}: NULL-ness of return differs C={okc} rust={okr}");
    assert_eq_bytes(&format!("{ctx} ip buffer"), &bc, &br);
    assert_eq!(ec, er, "{ctx}: errno C={ec} rust={er}");
    (okc, bc)
}

/// C-side ground-truth text of a `bin2ip` result (up to the NUL).
fn cstr_prefix(b: &[u8]) -> Vec<u8> {
    let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    b[..n].to_vec()
}

// ############################################################################
// CONFIGS rows 1–2 — sodium_bin2hex
// ############################################################################

#[test]
fn configs_01_02_bin2hex_exact_and_oversized_hex_maxlen() {
    init_both();
    let (c, r) = fnpair!("sodium_bin2hex", FBin2Hex);
    let (fc, fr) = (*c, *r);
    let mut rng = Rng::new(SEED);

    for &n in HEX_LENS.iter() {
        for iter in 0..64 {
            let bin: Vec<u8> = match iter {
                0 => vec![0u8; n],
                1 => vec![0xffu8; n],
                2 => (0..n).map(|i| i as u8).collect(),
                _ => rng.bytes(n),
            };

            // -------- CONFIGS row 1: hex_maxlen == 2*n+1 (minimum legal)
            let exact = 2 * n + 1;
            let out = bin2hex_check(
                "CONFIGS-1 bin2hex exact hex_maxlen",
                fc,
                fr,
                exact + 8,
                exact,
                &bin,
            );
            // C is ground truth; independently verify the encoding + NUL.
            assert_eq_bytes(
                "CONFIGS-1 bin2hex encoding vs reference",
                &to_hex(&bin, false),
                &out[..2 * n],
            );
            assert_eq!(out[2 * n], 0, "CONFIGS-1: missing NUL terminator");
            for (i, &b) in out[exact..].iter().enumerate() {
                assert_eq!(
                    b, SENTINEL,
                    "CONFIGS-1: C wrote past hex_maxlen at +{i} (n={n})"
                );
            }

            // -------- CONFIGS row 2: hex_maxlen oversized
            let slack = 1 + rng.below(16);
            let over = exact + slack;
            let out = bin2hex_check(
                "CONFIGS-2 bin2hex oversized hex_maxlen",
                fc,
                fr,
                over + 8,
                over,
                &bin,
            );
            assert_eq_bytes(
                "CONFIGS-2 bin2hex encoding vs reference",
                &to_hex(&bin, false),
                &out[..2 * n],
            );
            assert_eq!(out[2 * n], 0, "CONFIGS-2: missing NUL terminator");
            for (i, &b) in out[2 * n + 1..].iter().enumerate() {
                assert_eq!(
                    b, SENTINEL,
                    "CONFIGS-2: bytes past 2*bin_len+1 must be untouched, +{i} (n={n})"
                );
            }
        }
    }
}

// ############################################################################
// CONFIGS rows 3–7 — sodium_hex2bin valid shapes, × hex_end NULL/non-NULL
// ############################################################################

struct HexCase {
    row: &'static str,
    hex: Vec<u8>,
    ignore: Option<Vec<u8>>,
    nbytes: usize,
}

fn hex_cases(rng: &mut Rng) -> Vec<HexCase> {
    let mut v = Vec::new();

    // ---- CONFIGS row 3: lower / UPPER / MiXeD, ignore = NULL
    for i in 0..96 {
        let n = rng.below(33);
        let bin = rng.bytes(n);
        let hex = match i % 3 {
            0 => to_hex(&bin, false),
            1 => to_hex(&bin, true),
            _ => to_hex_mixed(&bin, rng),
        };
        v.push(HexCase {
            row: "CONFIGS-3 hex2bin case",
            hex,
            ignore: None,
            nbytes: n,
        });
    }

    // ---- CONFIGS row 4: ignore=":" separators leading / trailing / between
    for _ in 0..96 {
        let n = rng.below(24);
        let bin = rng.bytes(n);
        let mut hex = Vec::new();
        if rng.byte() & 1 == 1 {
            // leading separators
            for _ in 0..1 + rng.below(3) {
                hex.push(b':');
            }
        }
        for (i, &b) in bin.iter().enumerate() {
            if i != 0 && rng.byte() & 1 == 1 {
                hex.push(b':');
            }
            let h = to_hex(&[b], rng.byte() & 1 == 1);
            hex.extend_from_slice(&h);
        }
        if rng.byte() & 1 == 1 {
            // trailing separators
            for _ in 0..1 + rng.below(3) {
                hex.push(b':');
            }
        }
        v.push(HexCase {
            row: "CONFIGS-4 hex2bin ignore=\":\"",
            hex,
            ignore: Some(cs(":")),
            nbytes: n,
        });
    }

    // ---- CONFIGS row 5: ignore=" \n", multi-line hex-dump shape
    for _ in 0..96 {
        let n = rng.below(40);
        let bin = rng.bytes(n);
        let per_line = 1 + rng.below(8);
        let mut hex = Vec::new();
        for (i, &b) in bin.iter().enumerate() {
            if i != 0 {
                if i % per_line == 0 {
                    hex.push(b'\n');
                } else {
                    hex.push(b' ');
                }
            }
            hex.extend_from_slice(&to_hex(&[b], false));
        }
        if rng.byte() & 1 == 1 {
            hex.push(b'\n');
        }
        v.push(HexCase {
            row: "CONFIGS-5 hex2bin ignore=\" \\n\"",
            hex,
            ignore: Some(cs(" \n")),
            nbytes: n,
        });
    }

    // ---- CONFIGS row 6: ignore="" — nothing ignorable except the NUL quirk
    for i in 0..96 {
        let n = rng.below(33);
        let bin = rng.bytes(n);
        let hex = if i % 2 == 0 {
            to_hex(&bin, false)
        } else {
            to_hex(&bin, true)
        };
        v.push(HexCase {
            row: "CONFIGS-6 hex2bin ignore=\"\"",
            hex,
            ignore: Some(cs("")),
            nbytes: n,
        });
    }

    v
}

#[test]
fn configs_03_to_07_hex2bin_valid_shapes_and_hex_end() {
    init_both();
    let (c, r) = fnpair!("sodium_hex2bin", FHex2Bin);
    let (fc, fr) = (*c, *r);
    let mut rng = Rng::new(SEED);
    let cases = hex_cases(&mut rng);
    assert!(cases.len() >= 64 * 4);

    for case in &cases {
        let ig = case.ignore.as_deref();
        // CONFIGS rows 3–6 use hex_end == NULL; row 7 repeats every one of them
        // with hex_end != NULL. Both bin_len variants are exercised too.
        for &want_hex_end in &[false, true] {
            for &want_bin_len in &[true, false] {
                let mut call = H2bCall::new(&case.hex, case.nbytes)
                    .hex_end(want_hex_end)
                    .bin_len_out(want_bin_len);
                if let Some(i) = ig {
                    call = call.ig(i);
                }
                let label = if want_hex_end {
                    "CONFIGS-7 (hex_end!=NULL) / "
                } else {
                    ""
                };
                let out = call.check(&format!("{label}{}", case.row), fc, fr);

                // The C is ground truth: all of these shapes are LEGAL, so it
                // must fully consume the input and decode every byte.
                assert_eq!(
                    out.ret,
                    0,
                    "{}: legal input rejected by the C: errno={}",
                    call.ctx(case.row),
                    out.errno
                );
                if let Some(bl) = out.bin_len {
                    assert_eq!(bl, case.nbytes, "{}: wrong *bin_len", call.ctx(case.row));
                }
                if let Some(he) = out.hex_end {
                    assert_eq!(
                        he as usize,
                        case.hex.len(),
                        "{}: *hex_end must be at the end",
                        call.ctx(case.row)
                    );
                }
                assert_eq!(
                    out.errno, ERRNO_MARK,
                    "{}: errno must be untouched on success",
                    call.ctx(case.row)
                );
            }
        }
    }
}

// ############################################################################
// CONFIGS row 8 — bin_maxlen oversized; hex_len in {0,1,2,3, even, odd}
// ############################################################################

#[test]
fn configs_08_hex2bin_oversized_bin_maxlen_and_hex_len_shapes() {
    init_both();
    let (c, r) = fnpair!("sodium_hex2bin", FHex2Bin);
    let (fc, fr) = (*c, *r);
    let mut rng = Rng::new(SEED);

    for _ in 0..96 {
        let n = 1 + rng.below(24);
        let bin = rng.bytes(n);
        let hex = to_hex_mixed(&bin, &mut rng);
        // hex_len sweep: 0,1,2,3 plus a random even and a random odd length.
        let even = 2 * rng.below(hex.len() / 2 + 1);
        let odd = (2 * rng.below(hex.len() / 2) + 1).min(hex.len());
        let mut lens = vec![0usize, 1, 2, 3, even, odd, hex.len()];
        lens.retain(|&l| l <= hex.len());
        for &hl in &lens {
            let slack = 1 + rng.below(16);
            for &want_hex_end in &[false, true] {
                for ig in &[None, Some(cs(":"))] {
                    let mut call = H2bCall::new(&hex, n + slack)
                        .hex_len(hl)
                        .hex_end(want_hex_end);
                    if let Some(i) = ig {
                        call = call.ig(i);
                    }
                    call.check("CONFIGS-8 hex2bin oversized bin_maxlen", fc, fr);
                }
            }
        }
    }
}

// ############################################################################
// CONFIGS row 9 — round trip bin2hex -> hex2bin, bin_len 0..64
// ############################################################################

#[test]
fn configs_09_hex_roundtrip() {
    init_both();
    let (bh_c, bh_r) = fnpair!("sodium_bin2hex", FBin2Hex);
    let (hb_c, hb_r) = fnpair!("sodium_hex2bin", FHex2Bin);
    let (bhc, bhr) = (*bh_c, *bh_r);
    let (hbc, hbr) = (*hb_c, *hb_r);
    let mut rng = Rng::new(SEED);

    for n in 0..=64usize {
        for _ in 0..2 {
            let bin = rng.bytes(n);
            let cap = 2 * n + 1 + 8;
            let hex_buf = bin2hex_check("CONFIGS-9 roundtrip encode", bhc, bhr, cap, 2 * n + 1, &bin);
            let hex = &hex_buf[..2 * n];

            for &want_hex_end in &[false, true] {
                let call = H2bCall::new(hex, n).hex_end(want_hex_end);
                let out = call.check("CONFIGS-9 roundtrip decode", hbc, hbr);
                assert_eq!(out.ret, 0, "CONFIGS-9: round-trip decode failed (n={n})");
                assert_eq!(out.bin_len, Some(n), "CONFIGS-9: wrong *bin_len (n={n})");
                assert_eq_bytes("CONFIGS-9 roundtrip payload", &bin, &out.bin[..n]);
            }
        }
    }
}

// ############################################################################
// CONFIGS row 10 — sodium_base64_encoded_len
// ############################################################################

fn expected_encoded_len(bin_len: usize, variant: c_int) -> usize {
    let rem = bin_len % 3;
    let base = (bin_len / 3) * 4;
    if rem == 0 {
        base + 1
    } else if (variant as u32) & 2 == 0 {
        base + 4 + 1
    } else {
        base + 2 + (rem >> 1) + 1
    }
}

#[test]
fn configs_10_base64_encoded_len_table() {
    init_both();
    let (c, r) = fnpair!("sodium_base64_encoded_len", FEncLen);
    let (fc, fr) = (*c, *r);
    let (bb_c, bb_r) = fnpair!("sodium_bin2base64", FBin2B64);
    let (bbc, bbr) = (*bb_c, *bb_r);
    let mut rng = Rng::new(SEED);

    let mut lens: Vec<usize> = (0..=40).collect();
    lens.extend_from_slice(&[41, 63, 64, 65, 255, 256, 1000, 4095, 4096, 65535]);
    for _ in 0..64 {
        lens.push(rng.below(100_000));
    }

    for &v in VARIANTS.iter() {
        for &n in &lens {
            set_errno(ERRNO_MARK);
            let cc = unsafe { fc(n, v) };
            let ec = get_errno();
            set_errno(ERRNO_MARK);
            let rr = unsafe { fr(n, v) };
            let er = get_errno();
            assert_eq!(cc, rr, "CONFIGS-10: encoded_len({n},{v}) C={cc} rust={rr}");
            assert_eq!(ec, er, "CONFIGS-10: encoded_len({n},{v}) errno C={ec} rust={er}");
            assert_eq!(
                cc,
                expected_encoded_len(n, v),
                "CONFIGS-10: encoded_len({n},{v}) disagrees with the documented table"
            );
        }
    }

    // Cross-check against the actual encoder for the small lengths: the encoded
    // length must be exactly strlen(b64) + 1.
    for &v in VARIANTS.iter() {
        for n in 0..=40usize {
            let bin = rng.bytes(n);
            let need = unsafe { fc(n, v) };
            let out = bin2b64_check("CONFIGS-10 encoded_len vs encoder", bbc, bbr, need + 8, need, &bin, v);
            let slen = out.iter().position(|&b| b == 0).unwrap();
            assert_eq!(
                slen + 1,
                need,
                "CONFIGS-10: encoded_len({n},{v})={need} but strlen(b64)={slen}"
            );
        }
    }
}

// ############################################################################
// CONFIGS rows 11–15 — sodium_bin2base64 per variant, exact + oversized
// ############################################################################

/// Reference base64 encoder, independent of libsodium.
fn ref_b64(bin: &[u8], variant: c_int) -> Vec<u8> {
    let alpha: &[u8; 64] = if (variant as u32) & 4 != 0 {
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
    } else {
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
    };
    let pad = (variant as u32) & 2 == 0;
    let mut out = Vec::new();
    for chunk in bin.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        let idx = [
            (n >> 18) & 0x3f,
            (n >> 12) & 0x3f,
            (n >> 6) & 0x3f,
            n & 0x3f,
        ];
        let take = chunk.len() + 1;
        for k in 0..take {
            out.push(alpha[idx[k] as usize]);
        }
        if pad {
            for _ in take..4 {
                out.push(b'=');
            }
        }
    }
    out
}

#[test]
fn configs_11_to_14_bin2base64_all_variants_exact_maxlen() {
    init_both();
    let (c, r) = fnpair!("sodium_bin2base64", FBin2B64);
    let (fc, fr) = (*c, *r);
    let (el_c, _el_r) = fnpair!("sodium_base64_encoded_len", FEncLen);
    let elc = *el_c;
    let mut rng = Rng::new(SEED);

    for (vi, &v) in VARIANTS.iter().enumerate() {
        let row = 11 + vi; // CONFIGS rows 11,12,13,14
        for &n in B64_LENS.iter() {
            for iter in 0..64 {
                let bin: Vec<u8> = match iter {
                    0 => vec![0u8; n],
                    1 => vec![0xffu8; n],
                    2 => (0..n).map(|i| i as u8).collect(),
                    3 => (0..n).map(|i| (255 - i) as u8).collect(),
                    _ => rng.bytes(n),
                };
                let need = unsafe { elc(n, v) };
                let out = bin2b64_check(
                    &format!("CONFIGS-{row} bin2base64 exact b64_maxlen"),
                    fc,
                    fr,
                    need + 8,
                    need,
                    &bin,
                    v,
                );
                // Independent reference check against the C output.
                let want = ref_b64(&bin, v);
                assert_eq_bytes(
                    &format!("CONFIGS-{row} bin2base64 vs reference encoder (n={n} v={v})"),
                    &want,
                    &out[..want.len()],
                );
                assert_eq!(out[need - 1], 0, "CONFIGS-{row}: missing NUL");
                for (i, &b) in out[need..].iter().enumerate() {
                    assert_eq!(
                        b, SENTINEL,
                        "CONFIGS-{row}: C wrote past b64_maxlen at +{i} (n={n} v={v})"
                    );
                }
            }
        }
    }
}

#[test]
fn configs_15_bin2base64_oversized_maxlen_zero_fill() {
    init_both();
    let (c, r) = fnpair!("sodium_bin2base64", FBin2B64);
    let (fc, fr) = (*c, *r);
    let (el_c, _) = fnpair!("sodium_base64_encoded_len", FEncLen);
    let elc = *el_c;
    let mut rng = Rng::new(SEED);

    for &v in VARIANTS.iter() {
        for &n in B64_LENS.iter() {
            for _ in 0..64 {
                let bin = rng.bytes(n);
                let need = unsafe { elc(n, v) };
                let b64_len = need - 1;
                let slack = 1 + rng.below(24);
                let maxlen = need + slack;
                let out = bin2b64_check(
                    "CONFIGS-15 bin2base64 oversized b64_maxlen",
                    fc,
                    fr,
                    maxlen + 8,
                    maxlen,
                    &bin,
                    v,
                );
                let want = ref_b64(&bin, v);
                assert_eq_bytes(
                    "CONFIGS-15 bin2base64 vs reference encoder",
                    &want,
                    &out[..want.len()],
                );
                // The C zero-fills b64[b64_len .. b64_maxlen).
                for i in b64_len..maxlen {
                    assert_eq!(
                        out[i], 0,
                        "CONFIGS-15: b64[{i}] must be zero-filled (n={n} v={v} b64_len={b64_len} maxlen={maxlen})"
                    );
                }
                for i in maxlen..out.len() {
                    assert_eq!(out[i], SENTINEL, "CONFIGS-15: wrote past b64_maxlen at {i}");
                }
            }
        }
    }
}

// ############################################################################
// CONFIGS rows 16–18 — sodium_base642bin valid decodes
// ############################################################################

#[test]
fn configs_16_17_18_base642bin_valid_decodes() {
    init_both();
    let (bb_c, bb_r) = fnpair!("sodium_bin2base64", FBin2B64);
    let (bbc, bbr) = (*bb_c, *bb_r);
    let (b2_c, b2_r) = fnpair!("sodium_base642bin", FB642Bin);
    let (b2c, b2r) = (*b2_c, *b2_r);
    let (el_c, _) = fnpair!("sodium_base64_encoded_len", FEncLen);
    let elc = *el_c;
    let mut rng = Rng::new(SEED);

    let mut seen_mod: [bool; 4] = [false; 4];

    for &v in VARIANTS.iter() {
        // bin_len 0..48 covers every bin_len%3 class => b64_len%4 in {0,2,3}.
        for n in 0..=48usize {
            for _ in 0..2 {
                let bin = rng.bytes(n);
                let need = unsafe { elc(n, v) };
                let enc = bin2b64_check("CONFIGS-16 encode for decode", bbc, bbr, need + 8, need, &bin, v);
                let b64 = &enc[..need - 1];
                seen_mod[b64.len() % 4] = true;

                // ---- CONFIGS row 16: bin_maxlen exact
                let out = B2bCall::new(b64, n, v).check("CONFIGS-16 base642bin exact bin_maxlen", b2c, b2r);
                assert_eq!(
                    out.ret, 0,
                    "CONFIGS-16: valid base64 rejected by the C (n={n} v={v} b64=\"{}\") errno={}",
                    show(b64),
                    out.errno
                );
                assert_eq!(out.bin_len, Some(n), "CONFIGS-16: wrong *bin_len");
                assert_eq_bytes("CONFIGS-16 decoded payload", &bin, &out.bin[..n]);

                // ---- CONFIGS row 17: bin_maxlen oversized, b64_end=NULL, bin_len=NULL
                let slack = 1 + rng.below(16);
                let out17 = B2bCall::new(b64, n + slack, v)
                    .bin_len_out(false)
                    .check("CONFIGS-17 base642bin oversized, no out-params", b2c, b2r);
                assert_eq!(out17.ret, 0, "CONFIGS-17: valid base64 rejected by the C");
                assert_eq_bytes("CONFIGS-17 decoded payload", &bin, &out17.bin[..n]);
                for i in n..out17.bin.len() {
                    assert_eq!(out17.bin[i], SENTINEL, "CONFIGS-17: wrote past decoded length");
                }

                // ---- CONFIGS row 18: b64_end != NULL, consumed offset
                let out18 = B2bCall::new(b64, n + slack, v)
                    .b64_end(true)
                    .check("CONFIGS-18 base642bin b64_end", b2c, b2r);
                assert_eq!(out18.ret, 0, "CONFIGS-18: valid base64 rejected by the C");
                assert_eq!(
                    out18.b64_end,
                    Some(b64.len() as isize),
                    "CONFIGS-18: *b64_end must be at the end of the input"
                );
            }
        }
    }
    assert!(seen_mod[0] && seen_mod[2] && seen_mod[3], "CONFIGS-16: b64_len%4 coverage gap");
    assert!(!seen_mod[1], "b64_len%4==1 is never a valid encoding");
}

// ############################################################################
// CONFIGS row 19 — sodium_base642bin with ignore=" \n" / " \n\r"
// ############################################################################

#[test]
fn configs_19_base642bin_ignore_whitespace() {
    init_both();
    let (bb_c, bb_r) = fnpair!("sodium_bin2base64", FBin2B64);
    let (bbc, bbr) = (*bb_c, *bb_r);
    let (b2_c, b2_r) = fnpair!("sodium_base642bin", FB642Bin);
    let (b2c, b2r) = (*b2_c, *b2_r);
    let (el_c, _) = fnpair!("sodium_base64_encoded_len", FEncLen);
    let elc = *el_c;
    let mut rng = Rng::new(SEED);

    let ig_sets = [cs(" \n"), cs(" \n\r")];

    for &v in VARIANTS.iter() {
        for _ in 0..64 {
            let n = rng.below(40);
            let bin = rng.bytes(n);
            let need = unsafe { elc(n, v) };
            let enc = bin2b64_check("CONFIGS-19 encode", bbc, bbr, need + 8, need, &bin, v);
            let b64 = &enc[..need - 1];

            for ig in ig_sets.iter() {
                let ws: Vec<u8> = ig[..ig.len() - 1].to_vec();
                let mut noisy = Vec::new();
                // leading whitespace
                for _ in 0..rng.below(4) {
                    noisy.push(*rng.pick(&ws));
                }
                // interior whitespace
                for (i, &ch) in b64.iter().enumerate() {
                    if i != 0 && rng.byte() & 3 == 0 {
                        noisy.push(*rng.pick(&ws));
                    }
                    noisy.push(ch);
                }
                // trailing whitespace
                for _ in 0..rng.below(4) {
                    noisy.push(*rng.pick(&ws));
                }

                for &want_end in &[false, true] {
                    let out = B2bCall::new(&noisy, n, v)
                        .ig(ig)
                        .b64_end(want_end)
                        .check("CONFIGS-19 base642bin ignore whitespace", b2c, b2r);
                    assert_eq!(
                        out.ret,
                        0,
                        "CONFIGS-19: whitespace-laced valid base64 rejected by the C (v={v} b64=\"{}\") errno={}",
                        show(&noisy),
                        out.errno
                    );
                    assert_eq!(out.bin_len, Some(n), "CONFIGS-19: wrong *bin_len");
                    assert_eq_bytes("CONFIGS-19 decoded payload", &bin, &out.bin[..n]);
                    if let Some(e) = out.b64_end {
                        assert_eq!(
                            e as usize,
                            noisy.len(),
                            "CONFIGS-19: trailing ignorables must be consumed"
                        );
                    }
                }
            }
        }
    }
}

// ############################################################################
// CONFIGS row 20 — "A" / "AA" / "AAAA" / "A===" per variant
// ############################################################################

#[test]
fn configs_20_base642bin_letter_a_shapes() {
    init_both();
    let (b2_c, b2_r) = fnpair!("sodium_base642bin", FB642Bin);
    let (b2c, b2r) = (*b2_c, *b2_r);
    let mut rng = Rng::new(SEED);

    let inputs: [&[u8]; 6] = [b"A", b"AA", b"AAA", b"AAAA", b"A===", b"AA=="];
    for &v in VARIANTS.iter() {
        for inp in inputs.iter() {
            // >=64 randomized surrounding configurations per (variant, input).
            for _ in 0..64 {
                let bin_maxlen = rng.below(6);
                let want_end = rng.byte() & 1 == 1;
                let want_bl = rng.byte() & 1 == 1;
                let ig: Option<Vec<u8>> = match rng.below(3) {
                    0 => None,
                    1 => Some(cs("")),
                    _ => Some(cs(" \n")),
                };
                let mut call = B2bCall::new(inp, bin_maxlen, v)
                    .b64_end(want_end)
                    .bin_len_out(want_bl);
                if let Some(ref i) = ig {
                    call = call.ig(i);
                }
                call.check("CONFIGS-20 base642bin 'A' shapes", b2c, b2r);
            }
            // Deterministic, generous-buffer variants of the same shapes.
            for &want_end in &[false, true] {
                B2bCall::new(inp, 8, v)
                    .b64_end(want_end)
                    .check("CONFIGS-20 base642bin 'A' shapes (roomy)", b2c, b2r);
            }
        }
    }
}

// ############################################################################
// CONFIGS row 21 — round trip bin2base64 -> base642bin, 4 variants, 0..64
// ############################################################################

#[test]
fn configs_21_base64_roundtrip_all_variants() {
    init_both();
    let (bb_c, bb_r) = fnpair!("sodium_bin2base64", FBin2B64);
    let (bbc, bbr) = (*bb_c, *bb_r);
    let (b2_c, b2_r) = fnpair!("sodium_base642bin", FB642Bin);
    let (b2c, b2r) = (*b2_c, *b2_r);
    let (el_c, _) = fnpair!("sodium_base64_encoded_len", FEncLen);
    let elc = *el_c;
    let mut rng = Rng::new(SEED);

    for &v in VARIANTS.iter() {
        for n in 0..=64usize {
            let bin = rng.bytes(n);
            let need = unsafe { elc(n, v) };
            let enc = bin2b64_check("CONFIGS-21 roundtrip encode", bbc, bbr, need + 8, need, &bin, v);
            let b64 = &enc[..need - 1];
            for &bin_maxlen in &[n, n + 1 + rng.below(16)] {
                for &want_end in &[false, true] {
                    let out = B2bCall::new(b64, bin_maxlen, v)
                        .b64_end(want_end)
                        .check("CONFIGS-21 roundtrip decode", b2c, b2r);
                    assert_eq!(
                        out.ret, 0,
                        "CONFIGS-21: round trip failed (n={n} v={v} b64=\"{}\")",
                        show(b64)
                    );
                    assert_eq!(out.bin_len, Some(n), "CONFIGS-21: wrong *bin_len");
                    assert_eq_bytes("CONFIGS-21 roundtrip payload", &bin, &out.bin[..n]);
                }
            }
        }
    }
}

// ############################################################################
// CONFIGS rows 22–26 — sodium_ip2bin valid forms
// ############################################################################

fn fmt_ipv4(oct: &[u8; 4], rng: &mut Rng) -> Vec<u8> {
    let mut s = String::new();
    for i in 0..4 {
        if i != 0 {
            s.push('.');
        }
        let d = oct[i].to_string();
        let width = (1 + rng.below(3)).max(d.len());
        for _ in d.len()..width {
            s.push('0');
        }
        s.push_str(&d);
    }
    s.into_bytes()
}

fn fmt_group(w: u32, rng: &mut Rng) -> String {
    let d = format!("{w:x}");
    let width = (1 + rng.below(4)).max(d.len());
    let mut s = String::new();
    for _ in d.len()..width {
        s.push('0');
    }
    if rng.byte() & 1 == 1 {
        s.push_str(&d.to_uppercase());
    } else {
        s.push_str(&d);
    }
    s
}

fn word(bin: &[u8], i: usize) -> u32 {
    ((bin[i * 2] as u32) << 8) | bin[i * 2 + 1] as u32
}

fn fmt_ipv6_full(bin: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut s = String::new();
    for i in 0..8 {
        if i != 0 {
            s.push(':');
        }
        s.push_str(&fmt_group(word(bin, i), rng));
    }
    s.into_bytes()
}

fn fmt_ipv6_v4tail(bin: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut s = String::new();
    for i in 0..6 {
        s.push_str(&fmt_group(word(bin, i), rng));
        s.push(':');
    }
    let oct = [bin[12], bin[13], bin[14], bin[15]];
    s.push_str(&String::from_utf8(fmt_ipv4(&oct, rng)).unwrap());
    s.into_bytes()
}

#[test]
fn configs_22_ip2bin_ipv4_dotted_quad() {
    init_both();
    let (c, r) = fnpair!("sodium_ip2bin", FIp2Bin);
    let (fc, fr) = (*c, *r);
    let mut rng = Rng::new(SEED);

    let mut cases: Vec<Vec<u8>> = vec![
        cs("0.0.0.0"),
        cs("255.255.255.255"),
        cs("01.2.3.4"),
        cs("001.002.003.004"),
        cs("127.0.0.1"),
        cs("010.010.010.010"),
    ];
    for _ in 0..96 {
        let b = rng.bytes(4);
        let oct = [b[0], b[1], b[2], b[3]];
        let mut t = fmt_ipv4(&oct, &mut rng);
        t.push(0);
        cases.push(t);
    }

    for t in &cases {
        let n = t.len() - 1; // exclude the NUL from ip_len_
        let (ret, bin) = ip2bin_check("CONFIGS-22 ip2bin IPv4", fc, fr, t, n);
        assert_eq!(
            ret,
            0,
            "CONFIGS-22: legal dotted quad rejected by the C: \"{}\"",
            show(&t[..n])
        );
        // IPv4-mapped layout: 10 zero bytes, 0xff, 0xff, then the octets.
        for i in 0..10 {
            assert_eq!(bin[i], 0, "CONFIGS-22: bin[{i}] must be 0");
        }
        assert_eq!((bin[10], bin[11]), (0xff, 0xff), "CONFIGS-22: missing ffff");
        for i in 16..24 {
            assert_eq!(bin[i], SENTINEL, "CONFIGS-22: wrote past bin[16]");
        }
    }
}

#[test]
fn configs_23_24_25_26_ip2bin_ipv6_zone_and_len() {
    init_both();
    let (c, r) = fnpair!("sodium_ip2bin", FIp2Bin);
    let (fc, fr) = (*c, *r);
    let (bi_c, bi_r) = fnpair!("sodium_bin2ip", FBin2Ip);
    let (bic, bir) = (*bi_c, *bi_r);
    let mut rng = Rng::new(SEED);

    // ---- CONFIGS row 23: full 8-group IPv6, "::", "::1", collapsed forms
    for t in [
        cs("::"),
        cs("::1"),
        cs("2001:db8::1"),
        cs("0:0:0:0:0:0:0:0"),
        cs("1:2:3:4:5:6:7:8"),
        cs("fe80::"),
        cs("::ffff"),
        cs("2001:0DB8:0000:0000:0000:0000:0000:0001"),
        cs("1::8"),
        cs("1:2::7:8"),
    ] {
        let n = t.len() - 1;
        let (ret, bin) = ip2bin_check("CONFIGS-23 ip2bin IPv6 fixed", fc, fr, &t, n);
        assert_eq!(ret, 0, "CONFIGS-23: legal IPv6 rejected: \"{}\"", show(&t[..n]));
        for i in 16..24 {
            assert_eq!(bin[i], SENTINEL, "CONFIGS-23: wrote past bin[16]");
        }
    }
    for _ in 0..96 {
        // Full 8-group form, random padding / case.
        let bin = rng.bytes(16);
        let mut t = fmt_ipv6_full(&bin, &mut rng);
        t.push(0);
        let n = t.len() - 1;
        let (ret, got) = ip2bin_check("CONFIGS-23 ip2bin IPv6 full", fc, fr, &t, n);
        assert_eq!(ret, 0, "CONFIGS-23: legal IPv6 rejected: \"{}\"", show(&t[..n]));
        assert_eq_bytes("CONFIGS-23 parsed bytes", &bin, &got[..16]);

        // Collapsed form: use the (already differentially verified) canonical
        // text produced by bin2ip, which contains "::" whenever a zero run
        // of >= 2 groups exists.
        let mut zbin = rng.bytes(16);
        let start = rng.below(7);
        let runlen = 2 + rng.below(8 - start);
        for i in start..(start + runlen).min(8) {
            zbin[i * 2] = 0;
            zbin[i * 2 + 1] = 0;
        }
        let (ok, txt) = bin2ip_check("CONFIGS-23 canonicalise", bic, bir, &zbin, 48, 56);
        assert!(ok, "CONFIGS-23: bin2ip failed with ip_maxlen=48");
        let mut t2 = cstr_prefix(&txt);
        let n2 = t2.len();
        t2.push(0);
        let (ret2, got2) = ip2bin_check("CONFIGS-23 ip2bin collapsed", fc, fr, &t2, n2);
        assert_eq!(ret2, 0, "CONFIGS-23: canonical text \"{}\" rejected", show(&t2[..n2]));
        assert_eq_bytes("CONFIGS-23 collapsed round trip", &zbin, &got2[..16]);
    }

    // ---- CONFIGS row 24: IPv4-mapped text and 6-group + embedded IPv4
    for t in [
        cs("::ffff:1.2.3.4"),
        cs("::ffff:0.0.0.0"),
        cs("::ffff:255.255.255.255"),
        cs("1:2:3:4:5:6:1.2.3.4"),
        cs("::1.2.3.4"),
        cs("::FFFF:127.0.0.1"),
    ] {
        let n = t.len() - 1;
        let (ret, _) = ip2bin_check("CONFIGS-24 ip2bin v4-in-v6 fixed", fc, fr, &t, n);
        assert_eq!(ret, 0, "CONFIGS-24: legal form rejected: \"{}\"", show(&t[..n]));
    }
    for _ in 0..96 {
        let bin = rng.bytes(16);
        let oct = [bin[12], bin[13], bin[14], bin[15]];
        // "::ffff:a.b.c.d"
        let mut t = b"::ffff:".to_vec();
        t.extend_from_slice(&fmt_ipv4(&oct, &mut rng));
        let n = t.len();
        t.push(0);
        let (ret, got) = ip2bin_check("CONFIGS-24 ip2bin ::ffff:v4", fc, fr, &t, n);
        assert_eq!(ret, 0, "CONFIGS-24: rejected \"{}\"", show(&t[..n]));
        let mut want = vec![0u8; 16];
        want[10] = 0xff;
        want[11] = 0xff;
        want[12..16].copy_from_slice(&oct);
        assert_eq_bytes("CONFIGS-24 ::ffff:v4 bytes", &want, &got[..16]);

        // 6 groups + embedded IPv4
        let mut t = fmt_ipv6_v4tail(&bin, &mut rng);
        let n = t.len();
        t.push(0);
        let (ret, got) = ip2bin_check("CONFIGS-24 ip2bin 6groups+v4", fc, fr, &t, n);
        assert_eq!(ret, 0, "CONFIGS-24: rejected \"{}\"", show(&t[..n]));
        assert_eq_bytes("CONFIGS-24 6groups+v4 bytes", &bin, &got[..16]);
    }

    // ---- CONFIGS row 25: zone id, parsed and DISCARDED
    let zone_chars: Vec<u8> = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-_."
        .to_vec();
    for t in [cs("fe80::1%eth0"), cs("::1%lo"), cs("fe80::1%0")] {
        let n = t.len() - 1;
        let (ret, _) = ip2bin_check("CONFIGS-25 ip2bin zone fixed", fc, fr, &t, n);
        assert_eq!(ret, 0, "CONFIGS-25: legal zone rejected: \"{}\"", show(&t[..n]));
    }
    for _ in 0..96 {
        let bin = rng.bytes(16);
        let base = fmt_ipv6_full(&bin, &mut rng);
        let mut t = base.clone();
        t.push(b'%');
        for _ in 0..1 + rng.below(8) {
            t.push(*rng.pick(&zone_chars));
        }
        let n = t.len();
        t.push(0);
        let (ret, got) = ip2bin_check("CONFIGS-25 ip2bin zone", fc, fr, &t, n);
        assert_eq!(ret, 0, "CONFIGS-25: rejected \"{}\"", show(&t[..n]));
        assert_eq_bytes("CONFIGS-25 zone must be discarded", &bin, &got[..16]);
    }

    // ---- CONFIGS row 26: ip_len_ longer than the string, and shorter
    for _ in 0..96 {
        let bin = rng.bytes(16);
        let txt = fmt_ipv6_full(&bin, &mut rng);
        // ip_len_ longer: NUL terminates the scan.
        let mut buf = txt.clone();
        buf.push(0);
        let tail_n = 1 + rng.below(8);
        let tail = rng.bytes(tail_n);
        buf.extend_from_slice(&tail);
        buf.push(0);
        let (ret, got) = ip2bin_check("CONFIGS-26 ip2bin ip_len_ > strlen", fc, fr, &buf, buf.len());
        assert_eq!(ret, 0, "CONFIGS-26: NUL must stop the scan");
        assert_eq_bytes("CONFIGS-26 long ip_len_ bytes", &bin, &got[..16]);

        // ip_len_ exactly strlen
        ip2bin_check("CONFIGS-26 ip2bin ip_len_ == strlen", fc, fr, &buf, txt.len());
        // ip_len_ shorter: prefix parse (usually invalid, sometimes valid).
        for _ in 0..4 {
            let cut = rng.below(txt.len() + 1);
            ip2bin_check("CONFIGS-26 ip2bin ip_len_ < strlen", fc, fr, &buf, cut);
        }
    }
    // An explicitly valid short prefix: "1.2.3.45" truncated to "1.2.3.4".
    let t = cs("1.2.3.45");
    let (ret, got) = ip2bin_check("CONFIGS-26 ip2bin valid prefix", fc, fr, &t, 7);
    assert_eq!(ret, 0, "CONFIGS-26: prefix \"1.2.3.4\" must parse");
    assert_eq!(&got[12..16], &[1u8, 2, 3, 4], "CONFIGS-26: wrong octets");
    // Zero-length input.
    ip2bin_check("CONFIGS-26 ip2bin ip_len_==0", fc, fr, &t, 0);
}

// ############################################################################
// CONFIGS rows 27–28 — sodium_bin2ip
// ############################################################################

#[test]
fn configs_27_28_bin2ip_maxlen_sweep_and_zero_run() {
    init_both();
    let (c, r) = fnpair!("sodium_bin2ip", FBin2Ip);
    let (fc, fr) = (*c, *r);
    let mut rng = Rng::new(SEED);

    // ---- CONFIGS row 27: ip_maxlen 3..46, both branches
    for _ in 0..96 {
        // IPv6 branch (random, unlikely to be v4-mapped) and IPv4-mapped branch.
        let v6 = rng.bytes(16);
        let mut v4m = vec![0u8; 16];
        v4m[10] = 0xff;
        v4m[11] = 0xff;
        let o = rng.bytes(4);
        v4m[12..16].copy_from_slice(&o);

        for bin in [&v6, &v4m] {
            for ip_maxlen in 3..=46usize {
                bin2ip_check("CONFIGS-27 bin2ip ip_maxlen sweep", fc, fr, bin, ip_maxlen, 64);
            }
            // Also a comfortably large maxlen.
            let (ok, buf) = bin2ip_check("CONFIGS-27 bin2ip roomy", fc, fr, bin, 47, 64);
            assert!(ok, "CONFIGS-27: bin2ip must succeed with ip_maxlen=47");
            let txt = cstr_prefix(&buf);
            assert!(txt.len() <= 45, "CONFIGS-27: unexpectedly long text");
            for i in txt.len() + 1..buf.len() {
                assert_eq!(buf[i], SENTINEL, "CONFIGS-27: wrote past the NUL at {i}");
            }
        }
    }

    // ---- CONFIGS row 28: longest zero run, >= 2, ties keep the FIRST
    // Deterministic tie case: words [0,0,1,0,0,2,3,4] -> "::1:0:0:2:3:4"
    let mut tie = vec![0u8; 16];
    tie[5] = 1; // word 2 = 1
    tie[11] = 2; // word 5 = 2
    tie[13] = 3;
    tie[15] = 4;
    let (ok, buf) = bin2ip_check("CONFIGS-28 bin2ip tie keeps first run", fc, fr, &tie, 48, 56);
    assert!(ok);
    assert_eq!(
        cstr_prefix(&buf),
        b"::1:0:0:2:3:4".to_vec(),
        "CONFIGS-28: ties must keep the FIRST longest zero run"
    );
    // A single zero word must NOT be collapsed.
    let mut one = vec![0u8; 16];
    for i in 0..8 {
        if i != 3 {
            one[i * 2] = 0;
            one[i * 2 + 1] = (i + 1) as u8;
        }
    }
    let (ok, buf) = bin2ip_check("CONFIGS-28 bin2ip single zero word", fc, fr, &one, 48, 56);
    assert!(ok);
    assert_eq!(
        cstr_prefix(&buf),
        b"1:2:3:0:5:6:7:8".to_vec(),
        "CONFIGS-28: a run of 1 must not be collapsed"
    );
    // All-zero -> "::"
    let (ok, buf) = bin2ip_check("CONFIGS-28 bin2ip all zero", fc, fr, &vec![0u8; 16], 48, 56);
    assert!(ok);
    assert_eq!(cstr_prefix(&buf), b"::".to_vec(), "CONFIGS-28: all-zero must be \"::\"");
    // Randomized zero-run shapes.
    for _ in 0..96 {
        let mut bin = rng.bytes(16);
        let nruns = 1 + rng.below(3);
        for _ in 0..nruns {
            let start = rng.below(8);
            let len = 1 + rng.below(8 - start);
            for i in start..start + len {
                bin[i * 2] = 0;
                bin[i * 2 + 1] = 0;
            }
        }
        bin2ip_check("CONFIGS-28 bin2ip random zero runs", fc, fr, &bin, 48, 56);
        bin2ip_check("CONFIGS-28 bin2ip random zero runs tight", fc, fr, &bin, 3 + rng.below(44), 56);
    }
}

// ############################################################################
// CONFIGS row 29 — round trip ip2bin -> bin2ip (canonicalisation)
// ############################################################################

#[test]
fn configs_29_ip_roundtrip_canonicalisation() {
    init_both();
    let (i2_c, i2_r) = fnpair!("sodium_ip2bin", FIp2Bin);
    let (i2c, i2r) = (*i2_c, *i2_r);
    let (b2_c, b2_r) = fnpair!("sodium_bin2ip", FBin2Ip);
    let (b2c, b2r) = (*b2_c, *b2_r);
    let mut rng = Rng::new(SEED);

    // Documented non-identity canonicalisations.
    for (input, want) in [
        ("0:0:0:0:0:0:0:0", "::"),
        ("1:2:3:4:5:6:1.2.3.4", "1:2:3:4:5:6:102:304"),
        ("::ffff:1.2.3.4", "1.2.3.4"),
        ("2001:0DB8:0000:0000:0000:0000:0000:0001", "2001:db8::1"),
        ("fe80::1%eth0", "fe80::1"),
        ("01.02.03.04", "1.2.3.4"),
        ("::", "::"),
        ("::1", "::1"),
    ] {
        let t = cs(input);
        let (ret, bin) = ip2bin_check("CONFIGS-29 ip2bin", i2c, i2r, &t, t.len() - 1);
        assert_eq!(ret, 0, "CONFIGS-29: \"{input}\" must parse");
        let (ok, buf) = bin2ip_check("CONFIGS-29 bin2ip", b2c, b2r, &bin, 48, 56);
        assert!(ok, "CONFIGS-29: bin2ip must succeed");
        assert_eq!(
            String::from_utf8(cstr_prefix(&buf)).unwrap(),
            want,
            "CONFIGS-29: canonical form of \"{input}\""
        );
    }

    // Randomized: bin -> text -> bin must be the identity at the byte level.
    for _ in 0..128 {
        let mut bin = rng.bytes(16);
        match rng.below(3) {
            0 => {}
            1 => {
                // IPv4-mapped
                for i in 0..10 {
                    bin[i] = 0;
                }
                bin[10] = 0xff;
                bin[11] = 0xff;
            }
            _ => {
                let start = rng.below(7);
                let len = 2 + rng.below(8 - start);
                for i in start..(start + len).min(8) {
                    bin[i * 2] = 0;
                    bin[i * 2 + 1] = 0;
                }
            }
        }
        let (ok, buf) = bin2ip_check("CONFIGS-29 bin2ip random", b2c, b2r, &bin, 48, 56);
        assert!(ok);
        let mut t = cstr_prefix(&buf);
        let n = t.len();
        t.push(0);
        let (ret, got) = ip2bin_check("CONFIGS-29 ip2bin random", i2c, i2r, &t, n);
        assert_eq!(ret, 0, "CONFIGS-29: canonical text \"{}\" must re-parse", show(&t[..n]));
        assert_eq_bytes("CONFIGS-29 bin->text->bin identity", &bin, &got[..16]);
    }
}

// ############################################################################
// ERRORS rows 1, 2, 11, 12, 13, 14 — every `misuse` (SIGABRT) branch.
//
// All fork-based rows live in ONE #[test] so that no other test thread can be
// inside libsodium while the process forks. For the same reason the symbol
// lookup (dlsym), every allocation and every `format!` happens in the PARENT:
// a forked child of a multi-threaded process must not touch the loader lock or
// the malloc arena lock, or it can deadlock instead of aborting.
// ############################################################################

/// Resolve `name` in both libraries in the parent, then run `call` with each
/// function pointer in a forked child and require an identical `SIGABRT`.
fn misuse_both<T: Copy + 'static>(what: &str, name: &str, call: impl Fn(T) + Copy) {
    let l = libs();
    let fc: T = *unsafe { sym::<T>(&l.c, name) };
    let fr: T = *unsafe { sym::<T>(&l.r, name) };
    let oc = forked(move || {
        call(fc);
        0
    });
    let or = forked(move || {
        call(fr);
        0
    });
    assert_same_fatal(what, oc, or);
    assert_eq!(
        oc,
        Outcome::Signaled(SIGABRT),
        "{what}: expected SIGABRT from BOTH libraries, got {oc:?}"
    );
}

/// Confirms a call does NOT abort in either library (keeps the misuse rows from
/// being vacuously satisfied by an unconditional abort).
fn no_misuse_both<T: Copy + 'static>(what: &str, name: &str, call: impl Fn(T) + Copy) {
    let l = libs();
    let fc: T = *unsafe { sym::<T>(&l.c, name) };
    let fr: T = *unsafe { sym::<T>(&l.r, name) };
    let oc = forked(move || {
        call(fc);
        7
    });
    let or = forked(move || {
        call(fr);
        7
    });
    assert_same_fatal(what, oc, or);
    assert_eq!(
        oc,
        Outcome::Returned(7),
        "{what}: expected a normal return from BOTH libraries, got {oc:?}"
    );
}

#[test]
fn errors_01_02_11_12_13_14_misuse_branches() {
    init_both();

    // Every row below ends in abort(); without this the kernel would hand each
    // SIGABRT to systemd-coredump, which dominates the runtime. The children
    // inherit the limit from this (parent) process.
    unsafe {
        let rl = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        libc::setrlimit(libc::RLIMIT_CORE, &rl);
    }

    // Scratch buffers allocated ONCE, in the parent.
    let mut outbuf = vec![0u8; 8192];
    let outp = outbuf.as_mut_ptr() as *mut c_char;
    let outu = outbuf.as_mut_ptr();
    let binbuf = vec![0x5au8; 512];
    let binp = binbuf.as_ptr();
    let b64buf = b"AAAA".to_vec();
    let b64p = b64buf.as_ptr() as *const c_char;

    // ---------------- ERRORS row 1: bin_len >= SIZE_MAX/2
    for &bl in &[usize::MAX / 2, usize::MAX / 2 + 1, usize::MAX - 1, usize::MAX] {
        misuse_both::<FBin2Hex>(
            &format!("ERRORS-1 sodium_bin2hex bin_len={bl:#x}"),
            "sodium_bin2hex",
            move |f| unsafe {
                f(outp, 8192, binp, bl);
            },
        );
    }

    // ---------------- ERRORS row 2: hex_maxlen <= bin_len*2
    for &(n, maxlen) in &[
        (0usize, 0usize),
        (1, 0),
        (1, 1),
        (1, 2),
        (2, 4),
        (3, 6),
        (3, 5),
        (16, 32),
        (16, 0),
        (255, 510),
    ] {
        misuse_both::<FBin2Hex>(
            &format!("ERRORS-2 sodium_bin2hex bin_len={n} hex_maxlen={maxlen}"),
            "sodium_bin2hex",
            move |f| unsafe {
                f(outp, maxlen, binp, n);
            },
        );
    }
    // Control: the minimum legal hex_maxlen must NOT abort.
    for &n in &[0usize, 1, 3, 16, 255] {
        no_misuse_both::<FBin2Hex>(
            &format!("ERRORS-2 control sodium_bin2hex n={n} hex_maxlen={}", 2 * n + 1),
            "sodium_bin2hex",
            move |f| unsafe {
                f(outp, 2 * n + 1, binp, n);
            },
        );
    }

    // ---------------- ERRORS row 11: variant not in {1,3,5,7}
    for &v in BAD_VARIANTS.iter() {
        misuse_both::<FEncLen>(
            &format!("ERRORS-11 sodium_base64_encoded_len variant={v}"),
            "sodium_base64_encoded_len",
            move |f| unsafe {
                f(16, v);
            },
        );
        misuse_both::<FBin2B64>(
            &format!("ERRORS-11 sodium_bin2base64 variant={v}"),
            "sodium_bin2base64",
            move |f| unsafe {
                f(outp, 8192, binp, 4, v);
            },
        );
        misuse_both::<FB642Bin>(
            &format!("ERRORS-11 sodium_base642bin variant={v}"),
            "sodium_base642bin",
            move |f| unsafe {
                f(
                    outu,
                    8192,
                    b64p,
                    4,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    v,
                );
            },
        );
    }
    // Control: the four legal variants must NOT abort.
    for &v in VARIANTS.iter() {
        no_misuse_both::<FEncLen>(
            &format!("ERRORS-11 control sodium_base64_encoded_len variant={v}"),
            "sodium_base64_encoded_len",
            move |f| unsafe {
                f(16, v);
            },
        );
        no_misuse_both::<FBin2B64>(
            &format!("ERRORS-11 control sodium_bin2base64 variant={v}"),
            "sodium_bin2base64",
            move |f| unsafe {
                f(outp, 8192, binp, 4, v);
            },
        );
        no_misuse_both::<FB642Bin>(
            &format!("ERRORS-11 control sodium_base642bin variant={v}"),
            "sodium_base642bin",
            move |f| unsafe {
                f(
                    outu,
                    8192,
                    b64p,
                    4,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    v,
                );
            },
        );
    }

    // ---------------- ERRORS row 12: encoded_len bin_len/3 > (SIZE_MAX-5)/4
    let thresh = (usize::MAX - 5) / 4;
    for &bl in &[usize::MAX, usize::MAX - 1, 3 * (thresh + 1), 3 * thresh + 3] {
        for &v in VARIANTS.iter() {
            misuse_both::<FEncLen>(
                &format!("ERRORS-12 sodium_base64_encoded_len bin_len={bl:#x} variant={v}"),
                "sodium_base64_encoded_len",
                move |f| unsafe {
                    f(bl, v);
                },
            );
        }
    }
    // Control: exactly at the threshold must NOT abort.
    for &v in VARIANTS.iter() {
        no_misuse_both::<FEncLen>(
            &format!("ERRORS-12 control sodium_base64_encoded_len bin_len=3*thresh variant={v}"),
            "sodium_base64_encoded_len",
            move |f| unsafe {
                f(3 * thresh, v);
            },
        );
    }

    // ---------------- ERRORS row 13: bin2base64 bin_len/3 > (SIZE_MAX-5)/4
    for &bl in &[usize::MAX, usize::MAX - 1, 3 * (thresh + 1)] {
        for &v in VARIANTS.iter() {
            misuse_both::<FBin2B64>(
                &format!("ERRORS-13 sodium_bin2base64 bin_len={bl:#x} variant={v}"),
                "sodium_bin2base64",
                move |f| unsafe {
                    f(outp, usize::MAX, binp, bl, v);
                },
            );
        }
    }

    // ---------------- ERRORS row 14: b64_maxlen <= required b64_len
    for &v in VARIANTS.iter() {
        for &n in B64_LENS.iter() {
            let need = expected_encoded_len(n, v); // includes the NUL
            let b64_len = need - 1;
            let mut maxlens = vec![b64_len, b64_len / 2, 0];
            maxlens.sort_unstable();
            maxlens.dedup();
            for &maxlen in &maxlens {
                misuse_both::<FBin2B64>(
                    &format!(
                        "ERRORS-14 sodium_bin2base64 n={n} variant={v} b64_maxlen={maxlen} (b64_len={b64_len})"
                    ),
                    "sodium_bin2base64",
                    move |f| unsafe {
                        f(outp, maxlen, binp, n, v);
                    },
                );
            }
            // Control: exactly the minimum legal b64_maxlen must NOT abort.
            no_misuse_both::<FBin2B64>(
                &format!("ERRORS-14 control sodium_bin2base64 n={n} variant={v} b64_maxlen={need}"),
                "sodium_bin2base64",
                move |f| unsafe {
                    f(outp, need, binp, n, v);
                },
            );
        }
    }

    drop(outbuf);
    drop(binbuf);
    drop(b64buf);
}

// ############################################################################
// ERRORS rows 3–10 — sodium_hex2bin rejection branches
// ############################################################################

#[test]
fn errors_03_to_10_hex2bin() {
    init_both();
    let (c, r) = fnpair!("sodium_hex2bin", FHex2Bin);
    let (fc, fr) = (*c, *r);
    let mut rng = Rng::new(SEED);

    // ---- ERRORS row 3: odd number of hex digits consumed.
    for hex in [
        b"abc".to_vec(),
        b"a".to_vec(),
        b"abcde".to_vec(),
        b"ABCDE".to_vec(),
        b"0".to_vec(),
    ] {
        for &want_end in &[false, true] {
            let call = H2bCall::new(&hex, hex.len() / 2 + 1).hex_end(want_end);
            let out = call.check("ERRORS-3 hex2bin odd digit count", fc, fr);
            assert_eq!(out.ret, -1, "ERRORS-3: expected -1");
            assert_eq!(out.errno, libc::EINVAL, "ERRORS-3: expected EINVAL");
            assert_eq!(out.bin_len, Some(0), "ERRORS-3: *bin_len must be reset to 0");
            if let Some(e) = out.hex_end {
                assert_eq!(
                    e as usize,
                    hex.len() - 1,
                    "ERRORS-3: *hex_end must be &hex[hex_pos-1]"
                );
            }
        }
    }
    // Randomized odd-length hex.
    for _ in 0..96 {
        let n = 1 + rng.below(24);
        let bin = rng.bytes(n);
        let mut hex = to_hex_mixed(&bin, &mut rng);
        hex.pop(); // make the digit count odd
        for &want_end in &[false, true] {
            let out = H2bCall::new(&hex, n + 4)
                .hex_end(want_end)
                .check("ERRORS-3 hex2bin odd digit count (random)", fc, fr);
            assert_eq!(out.ret, -1, "ERRORS-3: expected -1");
            assert_eq!(out.errno, libc::EINVAL, "ERRORS-3: expected EINVAL");
            assert_eq!(out.bin_len, Some(0), "ERRORS-3: *bin_len must be 0");
        }
    }

    // ---- ERRORS rows 4 & 5: non-hex char, ignore == NULL.
    let bad_chars: Vec<u8> = vec![
        b'Z', b'z', b'g', b'G', b':', b' ', b'\n', b'-', b'_', b'!', 0x00, 0x7f, 0x80, 0xff,
    ];
    for _ in 0..96 {
        let n = 1 + rng.below(16);
        let bin = rng.bytes(n);
        let prefix_bytes = rng.below(n + 1);
        let mut hex = to_hex(&bin[..prefix_bytes], false);
        let badpos = hex.len();
        hex.push(*rng.pick(&bad_chars));
        hex.extend_from_slice(&to_hex(&bin[prefix_bytes..], false));

        // row 4: hex_end == NULL -> -1 / EINVAL, *bin_len = decoded prefix
        let out4 = H2bCall::new(&hex, n + 4).check("ERRORS-4 hex2bin bad char, hex_end=NULL", fc, fr);
        assert_eq!(out4.ret, -1, "ERRORS-4: expected -1");
        assert_eq!(out4.errno, libc::EINVAL, "ERRORS-4: expected EINVAL");
        assert_eq!(
            out4.bin_len,
            Some(prefix_bytes),
            "ERRORS-4: *bin_len must keep the decoded prefix (NOT reset to 0)"
        );
        assert_eq_bytes(
            "ERRORS-4 decoded prefix",
            &bin[..prefix_bytes],
            &out4.bin[..prefix_bytes],
        );

        // row 5: hex_end != NULL -> 0 (not an error)
        let out5 = H2bCall::new(&hex, n + 4)
            .hex_end(true)
            .check("ERRORS-5 hex2bin bad char, hex_end!=NULL", fc, fr);
        assert_eq!(out5.ret, 0, "ERRORS-5: a bad char with hex_end != NULL is NOT an error");
        assert_eq!(out5.errno, ERRNO_MARK, "ERRORS-5: errno must be untouched");
        assert_eq!(out5.bin_len, Some(prefix_bytes), "ERRORS-5: *bin_len = prefix");
        assert_eq!(
            out5.hex_end,
            Some(badpos as isize),
            "ERRORS-5: *hex_end must point at the bad char"
        );
    }

    // ---- ERRORS row 6: char in `ignore` mid-byte (state == 1).
    let ig = cs(":");
    for hex in [
        b"a:bcd".to_vec(),
        b"aab:bcc".to_vec(),
        b":a:b".to_vec(),
        b"1:".to_vec(),
    ] {
        for &want_end in &[false, true] {
            let out = H2bCall::new(&hex, 8)
                .ig(&ig)
                .hex_end(want_end)
                .check("ERRORS-6 hex2bin ignore char mid-byte", fc, fr);
            assert_eq!(out.ret, -1, "ERRORS-6: expected -1");
            assert_eq!(out.errno, libc::EINVAL, "ERRORS-6: expected EINVAL");
            assert_eq!(out.bin_len, Some(0), "ERRORS-6: *bin_len must be 0");
        }
    }
    for _ in 0..96 {
        let n = 1 + rng.below(12);
        let bin = rng.bytes(n);
        let mut hex = to_hex(&bin, false);
        // insert a ':' at an ODD digit index => mid-byte
        let odd = 2 * rng.below(n) + 1;
        hex.insert(odd, b':');
        for &want_end in &[false, true] {
            let out = H2bCall::new(&hex, n + 4)
                .ig(&ig)
                .hex_end(want_end)
                .check("ERRORS-6 hex2bin ignore mid-byte (random)", fc, fr);
            assert_eq!(out.ret, -1, "ERRORS-6: expected -1");
            assert_eq!(out.errno, libc::EINVAL, "ERRORS-6: expected EINVAL");
            assert_eq!(out.bin_len, Some(0), "ERRORS-6: *bin_len must be 0");
        }
    }

    // ---- ERRORS rows 7 & 8: bin_maxlen exhausted.
    for _ in 0..96 {
        let n = 2 + rng.below(16);
        let bin = rng.bytes(n);
        let hex = to_hex(&bin, false);
        let cap = 1 + rng.below(n - 1); // strictly fewer bytes than needed
        // row 7: hex_end != NULL -> ERANGE survives
        let out7 = H2bCall::new(&hex, cap)
            .hex_end(true)
            .check("ERRORS-7 hex2bin bin_maxlen exhausted, hex_end!=NULL", fc, fr);
        assert_eq!(out7.ret, -1, "ERRORS-7: expected -1");
        assert_eq!(out7.errno, libc::ERANGE, "ERRORS-7: expected ERANGE");
        assert_eq!(out7.bin_len, Some(0), "ERRORS-7: *bin_len must be 0");
        assert_eq!(
            out7.hex_end,
            Some((cap * 2) as isize),
            "ERRORS-7: *hex_end must be at the first unconsumed char"
        );
        // row 8: hex_end == NULL -> EINVAL overwrites ERANGE
        let out8 = H2bCall::new(&hex, cap)
            .check("ERRORS-8 hex2bin bin_maxlen exhausted, hex_end=NULL", fc, fr);
        assert_eq!(out8.ret, -1, "ERRORS-8: expected -1");
        assert_eq!(
            out8.errno,
            libc::EINVAL,
            "ERRORS-8: ERANGE must be OVERWRITTEN by EINVAL"
        );
        assert_eq!(out8.bin_len, Some(0), "ERRORS-8: *bin_len must be 0");
    }

    // ---- ERRORS row 9: bin_maxlen == 0 with >= 1 valid hex digit.
    for _ in 0..96 {
        let n = 1 + rng.below(16);
        let bin = rng.bytes(n);
        let hex = to_hex_mixed(&bin, &mut rng);
        let out_end = H2bCall::new(&hex, 0)
            .hex_end(true)
            .check("ERRORS-9 hex2bin bin_maxlen=0, hex_end!=NULL", fc, fr);
        assert_eq!(out_end.ret, -1, "ERRORS-9: expected -1");
        assert_eq!(out_end.errno, libc::ERANGE, "ERRORS-9: expected ERANGE");
        assert_eq!(out_end.bin_len, Some(0), "ERRORS-9: *bin_len must be 0");
        // The ERANGE `break` happens BEFORE `hex_pos++`, so *hex_end is &hex[0].
        assert_eq!(
            out_end.hex_end,
            Some(0),
            "ERRORS-9: *hex_end must be at the first unconsumed char (hex[0])"
        );

        let out_null =
            H2bCall::new(&hex, 0).check("ERRORS-9 hex2bin bin_maxlen=0, hex_end=NULL", fc, fr);
        assert_eq!(out_null.ret, -1, "ERRORS-9: expected -1");
        assert_eq!(out_null.errno, libc::EINVAL, "ERRORS-9: expected EINVAL");
        assert_eq!(out_null.bin_len, Some(0), "ERRORS-9: *bin_len must be 0");
    }
    // bin_maxlen == 0 with an EMPTY input is legal.
    let empty: Vec<u8> = Vec::new();
    for &want_end in &[false, true] {
        let out = H2bCall::new(&empty, 0)
            .hex_end(want_end)
            .check("ERRORS-9 control hex2bin bin_maxlen=0, hex_len=0", fc, fr);
        assert_eq!(out.ret, 0, "ERRORS-9 control: empty input must succeed");
    }

    // ---- ERRORS row 10: embedded NUL skipped because strchr(ignore, 0) matches.
    for ig_s in ["", ":", " \n", "\t"] {
        let ig = cs(ig_s);
        for _ in 0..24 {
            let n = 1 + rng.below(12);
            let bin = rng.bytes(n);
            let hex = to_hex(&bin, false);
            // Insert NULs at byte boundaries (state == 0) only.
            let mut noisy = Vec::new();
            for _ in 0..rng.below(3) {
                noisy.push(0u8);
            }
            for (i, ch) in hex.chunks(2).enumerate() {
                if i != 0 {
                    for _ in 0..rng.below(3) {
                        noisy.push(0u8);
                    }
                }
                noisy.extend_from_slice(ch);
            }
            for _ in 0..rng.below(3) {
                noisy.push(0u8);
            }
            for &want_end in &[false, true] {
                let out = H2bCall::new(&noisy, n)
                    .ig(&ig)
                    .hex_end(want_end)
                    .check("ERRORS-10 hex2bin embedded NUL quirk", fc, fr);
                assert_eq!(
                    out.ret, 0,
                    "ERRORS-10: an embedded NUL must be treated as ignorable (ignore=\"{ig_s}\")"
                );
                assert_eq!(out.bin_len, Some(n), "ERRORS-10: *bin_len wrong");
                assert_eq_bytes("ERRORS-10 decoded payload", &bin, &out.bin[..n]);
            }
        }
    }
    // Control: with ignore == NULL the same NUL is a hard stop.
    let hex = b"aa\0bb".to_vec();
    let out = H2bCall::new(&hex, 4).check("ERRORS-10 control ignore=NULL", fc, fr);
    assert_eq!(out.ret, -1, "ERRORS-10 control: NUL must stop the scan");
    assert_eq!(out.errno, libc::EINVAL, "ERRORS-10 control: expected EINVAL");
    let out = H2bCall::new(&hex, 4)
        .ig(&cs(":"))
        .check("ERRORS-10 fixed vector", fc, fr);
    assert_eq!(out.ret, 0, "ERRORS-10: \"aa\\0bb\" with ignore=\":\" must decode");
    assert_eq!(out.bin_len, Some(2), "ERRORS-10: must decode 2 bytes");
    assert_eq_bytes("ERRORS-10 payload", &[0xaa, 0xbb], &out.bin[..2]);
}

// ############################################################################
// ERRORS rows 15–30 — sodium_base642bin rejection branches
// ############################################################################

#[test]
fn errors_15_to_30_base642bin() {
    init_both();
    let (c, r) = fnpair!("sodium_base642bin", FB642Bin);
    let (fc, fr) = (*c, *r);
    let (bb_c, bb_r) = fnpair!("sodium_bin2base64", FBin2B64);
    let (bbc, bbr) = (*bb_c, *bb_r);
    let (el_c, _) = fnpair!("sodium_base64_encoded_len", FEncLen);
    let elc = *el_c;
    let mut rng = Rng::new(SEED);

    let np_variants = [V_ORIG_NP, V_URL_NP];
    let pad_variants = [V_ORIG, V_URL];

    // ---- ERRORS row 15: invalid char, b64_end == NULL, no padding obligation.
    // ---- ERRORS row 17: same but b64_end != NULL -> 0.
    let bad_chars: Vec<u8> = vec![b'!', b'*', b' ', b'\n', b'#', b'.', 0x00, 0x7f, 0x80, 0xff];
    for &v in np_variants.iter() {
        for _ in 0..96 {
            let n = 3 * (1 + rng.below(8)); // multiple of 3 => acc_len == 0
            let bin = rng.bytes(n);
            let need = unsafe { elc(n, v) };
            let enc = bin2b64_check("ERRORS-15 encode", bbc, bbr, need + 8, need, &bin, v);
            let mut b64 = enc[..need - 1].to_vec();
            let badpos = b64.len();
            b64.push(*rng.pick(&bad_chars));
            b64.extend_from_slice(b"AAAA");

            let out15 = B2bCall::new(&b64, n + 8, v)
                .check("ERRORS-15 base642bin bad char, b64_end=NULL", fc, fr);
            assert_eq!(out15.ret, -1, "ERRORS-15: expected -1");
            assert_eq!(out15.errno, libc::EINVAL, "ERRORS-15: expected EINVAL");
            assert_eq!(
                out15.bin_len,
                Some(n),
                "ERRORS-15: *bin_len may be NONZERO (partial decode kept)"
            );
            assert!(n > 0, "ERRORS-15: test must produce a NONZERO partial decode");
            assert_eq_bytes("ERRORS-15 partial decode", &bin, &out15.bin[..n]);

            let out17 = B2bCall::new(&b64, n + 8, v)
                .b64_end(true)
                .check("ERRORS-17 base642bin bad char, b64_end!=NULL", fc, fr);
            assert_eq!(out17.ret, 0, "ERRORS-17: expected 0");
            assert_eq!(out17.errno, ERRNO_MARK, "ERRORS-17: errno must be untouched");
            assert_eq!(out17.bin_len, Some(n), "ERRORS-17: *bin_len = prefix");
            assert_eq!(
                out17.b64_end,
                Some(badpos as isize),
                "ERRORS-17: *b64_end must point at the bad char"
            );
        }
    }

    // ---- ERRORS row 16: invalid char where padding is expected.
    for &v in pad_variants.iter() {
        for base in [&b"/w=!"[..], &b"/w!="[..], &b"QQ=Z"[..], &b"QQ!!"[..]] {
            let b64 = tr(base, v); // '/' is not in the URLSAFE alphabet
            for &want_end in &[false, true] {
                let out = B2bCall::new(&b64, 8, v)
                    .b64_end(want_end)
                    .check("ERRORS-16 base642bin bad char in padding", fc, fr);
                assert_eq!(
                    out.ret, -1,
                    "ERRORS-16: expected -1 (b64=\"{}\" variant={v})",
                    show(&b64)
                );
                assert_eq!(
                    out.errno,
                    libc::EINVAL,
                    "ERRORS-16: expected EINVAL (b64=\"{}\" variant={v})",
                    show(&b64)
                );
                assert_eq!(out.bin_len, Some(0), "ERRORS-16: *bin_len must be 0");
            }
        }
        // Randomized: a valid 1- or 2-mod-3 encoding whose padding is corrupted.
        for _ in 0..96 {
            let n = 3 * rng.below(6) + 1 + rng.below(2);
            let bin = rng.bytes(n);
            let need = unsafe { elc(n, v) };
            let enc = bin2b64_check("ERRORS-16 encode", bbc, bbr, need + 8, need, &bin, v);
            let mut b64 = enc[..need - 1].to_vec();
            let last = b64.len() - 1;
            assert_eq!(b64[last], b'=', "test bug: expected a padded encoding");
            b64[last] = *rng.pick(&bad_chars);
            for &want_end in &[false, true] {
                let out = B2bCall::new(&b64, n + 8, v)
                    .b64_end(want_end)
                    .check("ERRORS-16 base642bin corrupted padding", fc, fr);
                assert_eq!(out.ret, -1, "ERRORS-16: expected -1");
                assert_eq!(out.errno, libc::EINVAL, "ERRORS-16: expected EINVAL");
                assert_eq!(out.bin_len, Some(0), "ERRORS-16: *bin_len must be 0");
            }
        }
    }

    // ---- ERRORS row 18: one leftover b64 char (acc_len == 6), errno UNCHANGED.
    for &v in VARIANTS.iter() {
        for b64 in [b"A".to_vec(), b"Q".to_vec(), b"AAAAA".to_vec(), b"QUJDR".to_vec()] {
            for &want_end in &[false, true] {
                let out = B2bCall::new(&b64, 8, v)
                    .b64_end(want_end)
                    .check("ERRORS-18 base642bin leftover char", fc, fr);
                assert_eq!(out.ret, -1, "ERRORS-18: expected -1");
                assert_eq!(
                    out.errno, ERRNO_MARK,
                    "ERRORS-18: errno must be UNCHANGED on this branch"
                );
                assert_eq!(out.bin_len, Some(0), "ERRORS-18: *bin_len must be 0");
            }
        }
    }
    // Randomized: encoding truncated so that consumed % 4 == 1.
    for &v in VARIANTS.iter() {
        for _ in 0..96 {
            let n = 3 * (1 + rng.below(8));
            let bin = rng.bytes(n);
            let need = unsafe { elc(n, v) };
            let enc = bin2b64_check("ERRORS-18 encode", bbc, bbr, need + 8, need, &bin, v);
            let mut b64 = enc[..need - 1].to_vec();
            b64.push(b'A'); // 4k + 1 chars
            for &want_end in &[false, true] {
                let out = B2bCall::new(&b64, n + 8, v)
                    .b64_end(want_end)
                    .check("ERRORS-18 base642bin leftover char (random)", fc, fr);
                assert_eq!(out.ret, -1, "ERRORS-18: expected -1");
                assert_eq!(out.errno, ERRNO_MARK, "ERRORS-18: errno must be UNCHANGED");
                assert_eq!(out.bin_len, Some(0), "ERRORS-18: *bin_len must be 0");
            }
        }
    }

    // ---- ERRORS row 19: nonzero trailing bits, errno UNCHANGED.
    for &v in VARIANTS.iter() {
        for b64 in [
            b"AC".to_vec(),
            b"AB".to_vec(),
            b"AAB".to_vec(),
            b"AAAAAC".to_vec(),
        ] {
            for &want_end in &[false, true] {
                let out = B2bCall::new(&b64, 8, v)
                    .b64_end(want_end)
                    .check("ERRORS-19 base642bin nonzero trailing bits", fc, fr);
                assert_eq!(out.ret, -1, "ERRORS-19: expected -1 (b64=\"{}\")", show(&b64));
                assert_eq!(
                    out.errno, ERRNO_MARK,
                    "ERRORS-19: errno must be UNCHANGED on this branch"
                );
                assert_eq!(out.bin_len, Some(0), "ERRORS-19: *bin_len must be 0");
            }
        }
    }
    // Randomized: every 2- and 3-char group with nonzero trailing bits.
    {
        let alpha_o: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let alpha_u: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        for &v in VARIANTS.iter() {
            let alpha = if (v as u32) & 4 != 0 { alpha_u } else { alpha_o };
            let mut done = 0;
            for _ in 0..200 {
                if done >= 64 {
                    break;
                }
                // 2 chars => acc_len 4, low 4 bits must be 0; pick nonzero.
                let hi = rng.below(64);
                let lo = rng.below(64);
                if (lo & 0xf) == 0 {
                    continue;
                }
                let b64 = vec![alpha[hi], alpha[lo]];
                let out = B2bCall::new(&b64, 8, v)
                    .check("ERRORS-19 base642bin nonzero trailing bits (random 2)", fc, fr);
                assert_eq!(out.ret, -1, "ERRORS-19: expected -1 (b64=\"{}\")", show(&b64));
                assert_eq!(out.errno, ERRNO_MARK, "ERRORS-19: errno must be UNCHANGED");
                assert_eq!(out.bin_len, Some(0), "ERRORS-19: *bin_len must be 0");
                done += 1;
            }
            assert!(done >= 64, "ERRORS-19: not enough randomized cases");
            let mut done = 0;
            for _ in 0..200 {
                if done >= 64 {
                    break;
                }
                // 3 chars => acc_len 2, low 2 bits must be 0; pick nonzero.
                let a = rng.below(64);
                let b = rng.below(64);
                let c3 = rng.below(64);
                if (c3 & 0x3) == 0 {
                    continue;
                }
                let b64 = vec![alpha[a], alpha[b], alpha[c3]];
                let out = B2bCall::new(&b64, 8, v)
                    .check("ERRORS-19 base642bin nonzero trailing bits (random 3)", fc, fr);
                assert_eq!(out.ret, -1, "ERRORS-19: expected -1 (b64=\"{}\")", show(&b64));
                assert_eq!(out.errno, ERRNO_MARK, "ERRORS-19: errno must be UNCHANGED");
                assert_eq!(out.bin_len, Some(0), "ERRORS-19: *bin_len must be 0");
                done += 1;
            }
            assert!(done >= 64, "ERRORS-19: not enough randomized cases");
        }
    }

    // ---- ERRORS rows 20, 21, 22: bin_maxlen exceeded.
    for &v in VARIANTS.iter() {
        for _ in 0..96 {
            let n = 3 * (1 + rng.below(8));
            let bin = rng.bytes(n);
            let need = unsafe { elc(n, v) };
            let enc = bin2b64_check("ERRORS-20 encode", bbc, bbr, need + 8, need, &bin, v);
            let b64 = &enc[..need - 1];
            let cap = rng.below(n); // 0..n-1 => strictly too small

            // row 20 (and row 22 when cap == 0): b64_end != NULL -> ERANGE
            let out20 = B2bCall::new(b64, cap, v)
                .b64_end(true)
                .check("ERRORS-20 base642bin bin_maxlen exceeded, b64_end!=NULL", fc, fr);
            assert_eq!(out20.ret, -1, "ERRORS-20: expected -1");
            assert_eq!(out20.errno, libc::ERANGE, "ERRORS-20: expected ERANGE");
            assert_eq!(out20.bin_len, Some(0), "ERRORS-20: *bin_len must be 0");

            // row 21: b64_end == NULL -> EINVAL overwrites ERANGE
            let out21 = B2bCall::new(b64, cap, v)
                .check("ERRORS-21 base642bin bin_maxlen exceeded, b64_end=NULL", fc, fr);
            assert_eq!(out21.ret, -1, "ERRORS-21: expected -1");
            assert_eq!(
                out21.errno,
                libc::EINVAL,
                "ERRORS-21: ERANGE must be OVERWRITTEN by EINVAL"
            );
            assert_eq!(out21.bin_len, Some(0), "ERRORS-21: *bin_len must be 0");
        }
        // row 22: bin_maxlen == 0 with decodable data, explicitly.
        for b64 in [b"AAAA".to_vec(), b"QUJD".to_vec(), b"QQ==".to_vec(), b"QQ".to_vec()] {
            let a = B2bCall::new(&b64, 0, v)
                .b64_end(true)
                .check("ERRORS-22 base642bin bin_maxlen=0, b64_end!=NULL", fc, fr);
            assert_eq!(a.ret, -1, "ERRORS-22: expected -1");
            assert_eq!(a.errno, libc::ERANGE, "ERRORS-22: expected ERANGE");
            assert_eq!(a.bin_len, Some(0), "ERRORS-22: *bin_len must be 0");
            let b = B2bCall::new(&b64, 0, v)
                .check("ERRORS-22 base642bin bin_maxlen=0, b64_end=NULL", fc, fr);
            assert_eq!(b.ret, -1, "ERRORS-22: expected -1");
            assert_eq!(b.errno, libc::EINVAL, "ERRORS-22: expected EINVAL");
            assert_eq!(b.bin_len, Some(0), "ERRORS-22: *bin_len must be 0");
        }
    }

    // ---- ERRORS row 23: PADDED variant, padding absent/truncated -> ERANGE.
    for &v in pad_variants.iter() {
        for base in [&b"/w"[..], &b"/wE"[..], &b"/w="[..], &b"QQ"[..]] {
            let b64 = tr(base, v); // '/' is not in the URLSAFE alphabet
            for &want_end in &[false, true] {
                let out = B2bCall::new(&b64, 8, v)
                    .b64_end(want_end)
                    .check("ERRORS-23 base642bin missing padding", fc, fr);
                assert_eq!(
                    out.ret, -1,
                    "ERRORS-23: expected -1 (b64=\"{}\" variant={v})",
                    show(&b64)
                );
                assert_eq!(
                    out.errno,
                    libc::ERANGE,
                    "ERRORS-23: skip_padding overrun must give ERANGE, NOT EINVAL (b64=\"{}\" variant={v})",
                    show(&b64)
                );
                assert_eq!(out.bin_len, Some(0), "ERRORS-23: *bin_len must be 0");
            }
        }
        // Randomized: a valid padded encoding with its '=' padding stripped.
        for _ in 0..96 {
            let n = 3 * rng.below(6) + 1 + rng.below(2);
            let bin = rng.bytes(n);
            let need = unsafe { elc(n, v) };
            let enc = bin2b64_check("ERRORS-23 encode", bbc, bbr, need + 8, need, &bin, v);
            let mut b64 = enc[..need - 1].to_vec();
            while b64.last() == Some(&b'=') {
                b64.pop();
            }
            // optionally keep one '=' to test truncated (not absent) padding
            if n % 3 == 1 && rng.byte() & 1 == 1 {
                b64.push(b'=');
            }
            for &want_end in &[false, true] {
                let out = B2bCall::new(&b64, n + 8, v)
                    .b64_end(want_end)
                    .check("ERRORS-23 base642bin stripped padding", fc, fr);
                assert_eq!(out.ret, -1, "ERRORS-23: expected -1 (b64=\"{}\")", show(&b64));
                assert_eq!(out.errno, libc::ERANGE, "ERRORS-23: expected ERANGE");
                assert_eq!(out.bin_len, Some(0), "ERRORS-23: *bin_len must be 0");
            }
        }
    }

    // ---- ERRORS rows 24 & 25: NO_PADDING variant fed padded input.
    for &v in np_variants.iter() {
        for _ in 0..96 {
            let n = 3 * rng.below(6) + 1 + rng.below(2);
            let bin = rng.bytes(n);
            // Encode with the PADDED sibling to get '=' characters.
            let pv = v & !2;
            let need = unsafe { elc(n, pv) };
            let enc = bin2b64_check("ERRORS-24 encode padded", bbc, bbr, need + 8, need, &bin, pv);
            let b64 = &enc[..need - 1];
            let firsteq = b64.iter().position(|&c| c == b'=').unwrap();

            // row 24: b64_end == NULL -> EINVAL, *bin_len NONZERO
            let out24 = B2bCall::new(b64, n + 8, v)
                .check("ERRORS-24 base642bin NO_PADDING fed padding, b64_end=NULL", fc, fr);
            assert_eq!(out24.ret, -1, "ERRORS-24: expected -1");
            assert_eq!(out24.errno, libc::EINVAL, "ERRORS-24: expected EINVAL");
            assert_eq!(out24.bin_len, Some(n), "ERRORS-24: *bin_len must be NONZERO");
            assert!(n > 0);

            // row 25: b64_end != NULL -> 0, *b64_end at the first '='
            let out25 = B2bCall::new(b64, n + 8, v)
                .b64_end(true)
                .check("ERRORS-25 base642bin NO_PADDING fed padding, b64_end!=NULL", fc, fr);
            assert_eq!(out25.ret, 0, "ERRORS-25: expected 0");
            assert_eq!(out25.errno, ERRNO_MARK, "ERRORS-25: errno must be untouched");
            assert_eq!(
                out25.b64_end,
                Some(firsteq as isize),
                "ERRORS-25: *b64_end must be at the first '='"
            );
            assert_eq_bytes("ERRORS-25 decoded payload", &bin, &out25.bin[..n]);
        }
        // Fixed vector from the table ('/' -> '_' for the URLSAFE variant).
        let b64 = tr(b"/w==", v);
        let a = B2bCall::new(&b64, 8, v).check("ERRORS-24 fixed \"/w==\"", fc, fr);
        assert_eq!(a.ret, -1, "ERRORS-24: expected -1 (variant={v})");
        assert_eq!(a.errno, libc::EINVAL, "ERRORS-24: expected EINVAL (variant={v})");
        assert_eq!(a.bin_len, Some(1), "ERRORS-24: *bin_len must be NONZERO (variant={v})");
        let b = B2bCall::new(&b64, 8, v)
            .b64_end(true)
            .check("ERRORS-25 fixed \"/w==\"", fc, fr);
        assert_eq!(b.ret, 0, "ERRORS-25: expected 0 (variant={v})");
        assert_eq!(b.b64_end, Some(2), "ERRORS-25: *b64_end at the first '=' (variant={v})");
    }

    // ---- ERRORS row 26: URLSAFE variants fed '+' or '/'.
    for &v in &[V_URL, V_URL_NP] {
        for _ in 0..96 {
            let n = 3 * (1 + rng.below(6));
            let bin = rng.bytes(n);
            let need = unsafe { elc(n, v) };
            let enc = bin2b64_check("ERRORS-26 encode", bbc, bbr, need + 8, need, &bin, v);
            let mut b64 = enc[..need - 1].to_vec();
            let badpos = b64.len();
            b64.push(if rng.byte() & 1 == 1 { b'+' } else { b'/' });
            b64.extend_from_slice(b"AAA");
            for &want_end in &[false, true] {
                let out = B2bCall::new(&b64, n + 8, v)
                    .b64_end(want_end)
                    .check("ERRORS-26 base642bin URLSAFE fed +//", fc, fr);
                if want_end {
                    assert_eq!(out.ret, 0, "ERRORS-26: row-17 behaviour expected");
                    assert_eq!(
                        out.b64_end,
                        Some(badpos as isize),
                        "ERRORS-26: *b64_end at the invalid char"
                    );
                } else {
                    assert_eq!(out.ret, -1, "ERRORS-26: row-15 behaviour expected");
                    assert_eq!(out.errno, libc::EINVAL, "ERRORS-26: expected EINVAL");
                }
            }
        }
        // '+'/'/' immediately after a 2-char group => the row-23 path.
        for b64 in [b"QQ+=".to_vec(), b"QQ/=".to_vec()] {
            B2bCall::new(&b64, 8, v).check("ERRORS-26 URLSAFE + in padding", fc, fr);
            B2bCall::new(&b64, 8, v)
                .b64_end(true)
                .check("ERRORS-26 URLSAFE + in padding (b64_end)", fc, fr);
        }
    }

    // ---- ERRORS row 27: ORIGINAL variants fed '-' or '_'.
    for &v in &[V_ORIG, V_ORIG_NP] {
        for _ in 0..96 {
            let n = 3 * (1 + rng.below(6));
            let bin = rng.bytes(n);
            let need = unsafe { elc(n, v) };
            let enc = bin2b64_check("ERRORS-27 encode", bbc, bbr, need + 8, need, &bin, v);
            let mut b64 = enc[..need - 1].to_vec();
            let badpos = b64.len();
            b64.push(if rng.byte() & 1 == 1 { b'-' } else { b'_' });
            b64.extend_from_slice(b"AAA");
            for &want_end in &[false, true] {
                let out = B2bCall::new(&b64, n + 8, v)
                    .b64_end(want_end)
                    .check("ERRORS-27 base642bin ORIGINAL fed -/_", fc, fr);
                if want_end {
                    assert_eq!(out.ret, 0, "ERRORS-27: row-17 behaviour expected");
                    assert_eq!(
                        out.b64_end,
                        Some(badpos as isize),
                        "ERRORS-27: *b64_end at the invalid char"
                    );
                } else {
                    assert_eq!(out.ret, -1, "ERRORS-27: row-15 behaviour expected");
                    assert_eq!(out.errno, libc::EINVAL, "ERRORS-27: expected EINVAL");
                }
            }
        }
        // '-'/'_' where padding is expected => the row-23 ERANGE path.
        for b64 in [b"QQ-=".to_vec(), b"QQ_=".to_vec()] {
            B2bCall::new(&b64, 8, v).check("ERRORS-27 ORIGINAL - in padding", fc, fr);
            B2bCall::new(&b64, 8, v)
                .b64_end(true)
                .check("ERRORS-27 ORIGINAL - in padding (b64_end)", fc, fr);
        }
    }

    // ---- ERRORS row 28: `ignore` CONTAINS '=' with a padded variant (quirk).
    let ig_eq = cs("=");
    let ig_eq_ws = cs("= \n");
    for &v in pad_variants.iter() {
        for ig in [&ig_eq, &ig_eq_ws] {
            for &want_end in &[false, true] {
                let b64 = tr(b"/w==", v);
                let out = B2bCall::new(&b64, 8, v)
                    .ig(ig)
                    .b64_end(want_end)
                    .check("ERRORS-28 base642bin ignore contains '='", fc, fr);
                assert_eq!(out.ret, -1, "ERRORS-28: expected -1 (variant={v})");
                assert_eq!(
                    out.errno,
                    libc::ERANGE,
                    "ERRORS-28: the '='-in-ignore quirk must give ERANGE (variant={v})"
                );
                assert_eq!(out.bin_len, Some(0), "ERRORS-28: *bin_len must be 0");
            }
            // Randomized padded encodings with '=' in `ignore`.
            for _ in 0..96 {
                let n = 3 * rng.below(6) + 1 + rng.below(2);
                let bin = rng.bytes(n);
                let need = unsafe { elc(n, v) };
                let enc = bin2b64_check("ERRORS-28 encode", bbc, bbr, need + 8, need, &bin, v);
                let b64 = enc[..need - 1].to_vec();
                let out = B2bCall::new(&b64, n + 8, v)
                    .ig(ig)
                    .check("ERRORS-28 base642bin ignore='=' (random)", fc, fr);
                assert_eq!(out.ret, -1, "ERRORS-28: expected -1 (b64=\"{}\")", show(&b64));
                assert_eq!(out.errno, libc::ERANGE, "ERRORS-28: expected ERANGE");
                assert_eq!(out.bin_len, Some(0), "ERRORS-28: *bin_len must be 0");
            }
        }
        // Control: a multiple-of-3 payload has no padding obligation, so '=' in
        // `ignore` is harmless there.
        for _ in 0..8 {
            let n = 3 * (1 + rng.below(6));
            let bin = rng.bytes(n);
            let need = unsafe { elc(n, v) };
            let enc = bin2b64_check("ERRORS-28 control encode", bbc, bbr, need + 8, need, &bin, v);
            let b64 = enc[..need - 1].to_vec();
            let out = B2bCall::new(&b64, n + 8, v)
                .ig(&ig_eq)
                .check("ERRORS-28 control ignore='=' no padding needed", fc, fr);
            assert_eq!(out.ret, 0, "ERRORS-28 control: expected success");
        }
    }

    // ---- ERRORS row 29: extra '=' beyond the required padding.
    for &v in pad_variants.iter() {
        for base in [&b"/w==="[..], &b"/w===="[..], &b"QQ==="[..], &b"QUJD="[..]] {
            let b64 = tr(base, v);
            let out = B2bCall::new(&b64, 8, v)
                .check("ERRORS-29 base642bin extra '=', b64_end=NULL", fc, fr);
            assert_eq!(
                out.ret, -1,
                "ERRORS-29: expected -1 (b64=\"{}\" variant={v})",
                show(&b64)
            );
            assert_eq!(
                out.errno,
                libc::EINVAL,
                "ERRORS-29: expected EINVAL (b64=\"{}\" variant={v})",
                show(&b64)
            );
            // With b64_end != NULL the leftover '=' is simply not consumed.
            B2bCall::new(&b64, 8, v)
                .b64_end(true)
                .check("ERRORS-29 base642bin extra '=', b64_end!=NULL", fc, fr);
        }
        for _ in 0..96 {
            let n = 3 * rng.below(6) + 1 + rng.below(2);
            let bin = rng.bytes(n);
            let need = unsafe { elc(n, v) };
            let enc = bin2b64_check("ERRORS-29 encode", bbc, bbr, need + 8, need, &bin, v);
            let mut b64 = enc[..need - 1].to_vec();
            for _ in 0..1 + rng.below(3) {
                b64.push(b'=');
            }
            let out = B2bCall::new(&b64, n + 8, v)
                .check("ERRORS-29 base642bin extra '=' (random)", fc, fr);
            assert_eq!(out.ret, -1, "ERRORS-29: expected -1 (b64=\"{}\")", show(&b64));
            assert_eq!(out.errno, libc::EINVAL, "ERRORS-29: expected EINVAL");
            B2bCall::new(&b64, n + 8, v)
                .b64_end(true)
                .check("ERRORS-29 base642bin extra '=' (random, b64_end)", fc, fr);
        }
    }

    // ---- ERRORS row 30: b64_end == NULL and ANY unconsumed trailing byte.
    for &v in VARIANTS.iter() {
        for _ in 0..96 {
            let n = 3 * (1 + rng.below(6));
            let bin = rng.bytes(n);
            let need = unsafe { elc(n, v) };
            let enc = bin2b64_check("ERRORS-30 encode", bbc, bbr, need + 8, need, &bin, v);
            let mut b64 = enc[..need - 1].to_vec();
            let consumed = b64.len();
            // Append trailing junk that stops the scan.
            let junk = *rng.pick(&bad_chars);
            b64.push(junk);
            for _ in 0..rng.below(4) {
                b64.push(*rng.pick(&bad_chars));
            }
            let out = B2bCall::new(&b64, n + 8, v)
                .check("ERRORS-30 base642bin unconsumed trailing byte", fc, fr);
            assert_eq!(out.ret, -1, "ERRORS-30: expected -1");
            assert_eq!(out.errno, libc::EINVAL, "ERRORS-30: expected EINVAL");
            // With b64_end != NULL the very same input is accepted.
            let ok = B2bCall::new(&b64, n + 8, v)
                .b64_end(true)
                .check("ERRORS-30 control with b64_end", fc, fr);
            assert_eq!(ok.ret, 0, "ERRORS-30 control: expected 0 with b64_end != NULL");
            assert_eq!(ok.b64_end, Some(consumed as isize));
            // Also: a SHORTER b64_len than the buffer leaves bytes unconsumed.
            let short = B2bCall::new(&b64, n + 8, v)
                .b64_len(consumed - 1)
                .check("ERRORS-30 truncated b64_len", fc, fr);
            let _ = short;
        }
    }
}

// ############################################################################
// ERRORS rows 31–45 — sodium_ip2bin rejection branches
// ############################################################################

#[test]
fn errors_31_to_45_ip2bin() {
    init_both();
    let (c, r) = fnpair!("sodium_ip2bin", FIp2Bin);
    let (fc, fr) = (*c, *r);
    let mut rng = Rng::new(SEED);

    // (row, inputs)
    let rows: &[(&str, &[&str])] = &[
        (
            "ERRORS-31 zone with an illegal char",
            &["fe80::1%bad!", "fe80::1%a b", "fe80::1%\u{7f}", "::1%+", "::1%/", "::1%:"],
        ),
        ("ERRORS-32 empty zone", &["fe80::1%", "::1%", "::%"]),
        (
            "ERRORS-33 zone but no ':' in the address",
            &["1.2.3.4%eth0", "0.0.0.0%x", "%eth0", "abc%eth0"],
        ),
        (
            "ERRORS-34 IPv4 octet > 255",
            &["256.0.0.1", "1.256.3.4", "1.2.300.4", "999.999.999.999", "0.0.0.256"],
        ),
        (
            "ERRORS-35 IPv4 octet with > 3 digits",
            &["1234.1.1.1", "1.1111.1.1", "0000.0.0.0", "1.2.3.0004"],
        ),
        ("ERRORS-36 IPv4 missing octet", &["1.2.3", "1.2", "1", "1.2.3."]),
        ("ERRORS-37 IPv4 extra octet", &["1.2.3.4.5", "1.2.3.4.5.6"]),
        ("ERRORS-38 empty / dot only", &["", ".", "..", "...."]),
        (
            "ERRORS-39 IPv6 group with > 4 hex digits",
            &["12345::", "::12345", "1:23456:3::", "abcde::1"],
        ),
        (
            "ERRORS-40 IPv6 non-hex char",
            &["g::1", "::g", "1:2:3:4:5:6:7:z", "1:2:3:4:5:6:7:8g", "!::1"],
        ),
        ("ERRORS-41 leading single ':'", &[":1", ":", ":a:b", ":1:2:3:4:5:6:7:8"]),
        ("ERRORS-42 trailing single ':'", &["1:", "1:2:", "fe80:1:", "::1:"]),
        (
            "ERRORS-43 two \"::\" runs",
            &["2001:db8::1::2", "::1::", "1::2::3", "::::"],
        ),
        (
            "ERRORS-44 more than 8 IPv6 groups",
            &["1:2:3:4:5:6:7:8:9", "1:2:3:4:5:6:7:8:9:a", "1:2:3:4:5:6:7:8:1.2.3.4"],
        ),
        (
            "ERRORS-45 \"::\" when 16 bytes are already filled",
            &["1:2:3:4:5:6:7:8::", "::1:2:3:4:5:6:7:8", "1:2:3:4:5:6:7::8"],
        ),
    ];

    for (row, inputs) in rows {
        for s in inputs.iter() {
            let t = cs(s);
            let n = t.len() - 1;
            let (ret, bin) = ip2bin_check(row, fc, fr, &t, n);
            assert_eq!(ret, -1, "{row}: \"{s}\" must be rejected (got {ret})");
            // bin must be completely untouched on failure.
            for (i, &b) in bin.iter().enumerate() {
                assert_eq!(b, SENTINEL, "{row}: bin[{i}] written on the failure path");
            }
            // Same input with ip_len_ covering the NUL as well.
            ip2bin_check(row, fc, fr, &t, t.len());
        }
    }

    // Randomized rejection fuzz: mutate valid addresses and require identical
    // accept/reject decisions from both libraries.
    let mut_chars: Vec<u8> = b"0123456789abcdefABCDEFg:.%!-_ \tZz".to_vec();
    for _ in 0..512 {
        let bin = rng.bytes(16);
        let base: Vec<u8> = if rng.byte() & 1 == 1 {
            fmt_ipv6_full(&bin, &mut rng)
        } else {
            fmt_ipv4(&[bin[0], bin[1], bin[2], bin[3]], &mut rng)
        };
        let mut t = base.clone();
        for _ in 0..1 + rng.below(3) {
            match rng.below(3) {
                0 if !t.is_empty() => {
                    let i = rng.below(t.len());
                    t[i] = *rng.pick(&mut_chars);
                }
                1 => {
                    let i = rng.below(t.len() + 1);
                    t.insert(i, *rng.pick(&mut_chars));
                }
                _ if !t.is_empty() => {
                    let i = rng.below(t.len());
                    t.remove(i);
                }
                _ => {}
            }
        }
        let n = t.len();
        t.push(0);
        ip2bin_check("ERRORS-31..45 ip2bin mutation fuzz", fc, fr, &t, n);
    }
    // Byte-level fuzz over short random strings (hits every early-out).
    for _ in 0..512 {
        let n = rng.below(24);
        let mut t: Vec<u8> = (0..n).map(|_| *rng.pick(&mut_chars)).collect();
        let len = t.len();
        t.push(0);
        ip2bin_check("ERRORS-31..45 ip2bin random-string fuzz", fc, fr, &t, len);
    }
}

// ############################################################################
// ERRORS rows 46–47 — sodium_bin2ip rejection branches
// ############################################################################

#[test]
fn errors_46_47_bin2ip() {
    init_both();
    let (c, r) = fnpair!("sodium_bin2ip", FBin2Ip);
    let (fc, fr) = (*c, *r);
    let mut rng = Rng::new(SEED);

    // ---- ERRORS row 46: ip_maxlen <= 2 -> NULL
    for _ in 0..64 {
        let mut bins: Vec<Vec<u8>> = vec![rng.bytes(16), vec![0u8; 16]];
        let mut v4m = vec![0u8; 16];
        v4m[10] = 0xff;
        v4m[11] = 0xff;
        v4m[12..16].copy_from_slice(&rng.bytes(4));
        bins.push(v4m);
        for bin in &bins {
            for ip_maxlen in 0..=2usize {
                let (ok, buf) = bin2ip_check("ERRORS-46 bin2ip ip_maxlen<=2", fc, fr, bin, ip_maxlen, 64);
                assert!(!ok, "ERRORS-46: ip_maxlen={ip_maxlen} must return NULL");
                for (i, &b) in buf.iter().enumerate() {
                    assert_eq!(b, SENTINEL, "ERRORS-46: buf[{i}] written before the length check");
                }
            }
        }
    }

    // ---- ERRORS row 47: formatted length >= ip_maxlen -> NULL
    // IPv4-mapped branch: "255.255.255.255" is 15 chars, needs ip_maxlen >= 16.
    let mut v4 = vec![0u8; 16];
    v4[10] = 0xff;
    v4[11] = 0xff;
    v4[12..16].copy_from_slice(&[255, 255, 255, 255]);
    for ip_maxlen in 3..=15usize {
        let (ok, _) = bin2ip_check("ERRORS-47 bin2ip IPv4 too long", fc, fr, &v4, ip_maxlen, 64);
        assert!(!ok, "ERRORS-47: ip_maxlen={ip_maxlen} must return NULL for 255.255.255.255");
    }
    let (ok, buf) = bin2ip_check("ERRORS-47 control IPv4 exact fit", fc, fr, &v4, 16, 64);
    assert!(ok, "ERRORS-47 control: ip_maxlen=16 must succeed");
    assert_eq!(cstr_prefix(&buf), b"255.255.255.255".to_vec());

    // IPv6 branch: "ffff:...:ffff" is 39 chars, needs ip_maxlen >= 40.
    let v6 = vec![0xffu8; 16];
    for ip_maxlen in 3..=39usize {
        let (ok, _) = bin2ip_check("ERRORS-47 bin2ip IPv6 too long", fc, fr, &v6, ip_maxlen, 64);
        assert!(!ok, "ERRORS-47: ip_maxlen={ip_maxlen} must return NULL for the all-ffff address");
    }
    let (ok, buf) = bin2ip_check("ERRORS-47 control IPv6 exact fit", fc, fr, &v6, 40, 64);
    assert!(ok, "ERRORS-47 control: ip_maxlen=40 must succeed");
    assert_eq!(
        cstr_prefix(&buf),
        b"ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff".to_vec()
    );

    // Randomized: sweep ip_maxlen across the exact boundary for random addresses.
    for _ in 0..128 {
        let mut bin = rng.bytes(16);
        if rng.byte() & 1 == 1 {
            for i in 0..10 {
                bin[i] = 0;
            }
            bin[10] = 0xff;
            bin[11] = 0xff;
        }
        // Find the true formatted length using a roomy call (C is ground truth).
        let (ok, buf) = bin2ip_check("ERRORS-47 measure", fc, fr, &bin, 47, 64);
        assert!(ok);
        let len = cstr_prefix(&buf).len();
        for ip_maxlen in 3..=(len + 2) {
            let (ok, _) = bin2ip_check("ERRORS-47 boundary sweep", fc, fr, &bin, ip_maxlen, 64);
            assert_eq!(
                ok,
                ip_maxlen > len,
                "ERRORS-47: ip_maxlen={ip_maxlen} vs formatted len={len}"
            );
        }
    }
}
