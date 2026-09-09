//! Phase C — error-path differential tests for module group G2
//! (`crypto_pwhash/`: argon2i, argon2id, scryptsalsa208sha256).
//!
//! Covers ERRORS.md rows 100-281 plus generic FFI-boundary probing
//! (zero/oversized lengths, one step past every documented limit, and
//! out-of-range `alg` / `argon2_type` ints — C enums accept any `int`).
//!
//! Both libraries export the argon2/scrypt internals under their `_sodium_*`
//! names, so the internal `ARGON2_*` enums and the `escrypt_*` NULL returns can
//! be compared directly instead of only through the `-1` public boundary.
//!
//! Rows whose rejection is a `sodium_misuse()` abort are compared by
//! re-executing this binary in two child processes (one per library) and
//! comparing how the two children died — see `zz_abort_child`.
#![allow(clippy::too_many_arguments, unused_unsafe, dead_code)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

// ===========================================================================
// FFI types & signatures
// ===========================================================================

/// `argon2_context` from `crypto_pwhash/argon2/argon2.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ctx {
    out: *mut u8,
    outlen: u32,
    pwd: *mut u8,
    pwdlen: u32,
    salt: *mut u8,
    saltlen: u32,
    secret: *mut u8,
    secretlen: u32,
    ad: *mut u8,
    adlen: u32,
    t_cost: u32,
    m_cost: u32,
    lanes: u32,
    threads: u32,
    flags: u32,
}

impl Ctx {
    fn zeroed() -> Ctx {
        Ctx {
            out: ptr::null_mut(),
            outlen: 0,
            pwd: ptr::null_mut(),
            pwdlen: 0,
            salt: ptr::null_mut(),
            saltlen: 0,
            secret: ptr::null_mut(),
            secretlen: 0,
            ad: ptr::null_mut(),
            adlen: 0,
            t_cost: 0,
            m_cost: 0,
            lanes: 0,
            threads: 0,
            flags: 0,
        }
    }
}

/// `escrypt_region_t` / `escrypt_local_t` from `crypto_scrypt.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Region {
    base: *mut c_void,
    aligned: *mut c_void,
    size: usize,
}

impl Region {
    fn zeroed() -> Region {
        Region {
            base: ptr::null_mut(),
            aligned: ptr::null_mut(),
            size: 0,
        }
    }
}

type FnPwhash = unsafe extern "C" fn(
    *mut u8,
    u64,
    *const c_char,
    u64,
    *const u8,
    u64,
    usize,
    c_int,
) -> c_int;
type FnStr = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize) -> c_int;
type FnStrAlg =
    unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize, c_int) -> c_int;
type FnVerify = unsafe extern "C" fn(*const c_char, *const c_char, u64) -> c_int;
type FnRehash = unsafe extern "C" fn(*const c_char, u64, usize) -> c_int;
type FnScrypt =
    unsafe extern "C" fn(*mut u8, u64, *const c_char, u64, *const u8, u64, usize) -> c_int;
type FnScryptLl = unsafe extern "C" fn(
    *const u8,
    usize,
    *const u8,
    usize,
    u64,
    u32,
    u32,
    *mut u8,
    usize,
) -> c_int;

type FnValidate = unsafe extern "C" fn(*const Ctx) -> c_int;
type FnArgon2Ctx = unsafe extern "C" fn(*mut Ctx, c_int) -> c_int;
type FnDecode = unsafe extern "C" fn(*mut Ctx, *const c_char, c_int) -> c_int;
type FnEncode = unsafe extern "C" fn(*mut c_char, usize, *mut Ctx, c_int) -> c_int;
type FnHashRaw = unsafe extern "C" fn(
    u32,
    u32,
    u32,
    *const c_void,
    usize,
    *const c_void,
    usize,
    *mut c_void,
    usize,
) -> c_int;
type FnHashEncoded = unsafe extern "C" fn(
    u32,
    u32,
    u32,
    *const c_void,
    usize,
    *const c_void,
    usize,
    usize,
    *mut c_char,
    usize,
) -> c_int;
type FnArgon2Verify =
    unsafe extern "C" fn(*const c_char, *const c_void, usize, c_int) -> c_int;
type FnArgon2iVerify = unsafe extern "C" fn(*const c_char, *const c_void, usize) -> c_int;
type FnBlake2bLong = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> c_int;
type FnInitLocal = unsafe extern "C" fn(*mut Region) -> c_int;
type FnAllocRegion = unsafe extern "C" fn(*mut Region, usize) -> *mut c_void;
type FnParseSetting =
    unsafe extern "C" fn(*const u8, *mut u32, *mut u32, *mut u32) -> *const u8;
type FnEscryptR =
    unsafe extern "C" fn(*mut Region, *const u8, usize, *const u8, *mut u8, usize) -> *mut u8;
type FnGensalt =
    unsafe extern "C" fn(u32, u32, u32, *const u8, usize, *mut u8, usize) -> *mut u8;
type FnKdf = unsafe extern "C" fn(
    *mut Region,
    *const u8,
    usize,
    *const u8,
    usize,
    u64,
    u32,
    u32,
    *mut u8,
    usize,
) -> c_int;
type FnPbkdf2 =
    unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, *mut u8, usize);

const EFBIG: i32 = 27;

const ARGON2I: c_int = 1;
const ARGON2ID: c_int = 2;

// internal ARGON2_* enum values (argon2.h)
const ARGON2_OK: c_int = 0;
const ARGON2_OUTPUT_PTR_NULL: c_int = -1;
const ARGON2_OUTPUT_TOO_SHORT: c_int = -2;
const ARGON2_OUTPUT_TOO_LONG: c_int = -3;
const ARGON2_PWD_TOO_LONG: c_int = -5;
const ARGON2_SALT_TOO_SHORT: c_int = -6;
const ARGON2_SALT_TOO_LONG: c_int = -7;
const ARGON2_TIME_TOO_SMALL: c_int = -12;
const ARGON2_MEMORY_TOO_LITTLE: c_int = -14;
const ARGON2_LANES_TOO_FEW: c_int = -16;
const ARGON2_LANES_TOO_MANY: c_int = -17;
const ARGON2_PWD_PTR_MISMATCH: c_int = -18;
const ARGON2_SALT_PTR_MISMATCH: c_int = -19;
const ARGON2_SECRET_PTR_MISMATCH: c_int = -20;
const ARGON2_AD_PTR_MISMATCH: c_int = -21;
const ARGON2_INCORRECT_PARAMETER: c_int = -25;
const ARGON2_INCORRECT_TYPE: c_int = -26;
const ARGON2_THREADS_TOO_FEW: c_int = -28;
const ARGON2_THREADS_TOO_MANY: c_int = -29;
const ARGON2_ENCODING_FAIL: c_int = -31;
const ARGON2_DECODING_FAIL: c_int = -32;
const ARGON2_VERIFY_MISMATCH: c_int = -35;

const PW: &[u8] = b"password";
const SALT16: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
const ARGON2ID_VEC: &str =
    "$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw";
const ARGON2I_VEC: &str =
    "$argon2i$v=19$m=8,t=3,p=1$AAECAwQFBgcICQoLDA0ODw$56X05/g05c3CDPF09EAaJqr6g8q8sB5TpA/CTwJJeSs";
/// A valid 101-character `$7$` string (passwd = "password", opslimit = 32768,
/// memlimit = 16777216 -> N_log2 = 10, r = 8, p = 1).
const SCRYPT_VEC: &str =
    "$7$86..../....1k.5TWlEQPkD532QliOWX.4dWKOckFwXFgNVvmBDXj/$OQtZd41caEZx0X/cihG3vM.d48NmKXEZK.Ns6eyGzr1";

// ===========================================================================
// child-process driver for the two sodium_misuse() rows
// ===========================================================================
#[test]
fn zz_abort_child() {
    let Some((case, is_c)) = child_case() else {
        return; // normal test run: nothing to do
    };
    let l = libs();
    let h = if is_c { l.c } else { l.rs };
    macro_rules! sym {
        ($t:ty, $n:literal) => {{
            let s: libloading::Symbol<$t> =
                unsafe { h.get(concat!($n, "\0").as_bytes()) }.unwrap();
            s
        }};
    }
    unsafe {
        match case.as_str() {
            // --- row 101: crypto_pwhash_str_alg with alg not in {1,2} ------
            c if c.starts_with("str_alg_alg_") => {
                let alg: c_int = match &c["str_alg_alg_".len()..] {
                    "m1" => -1,
                    "min" => i32::MIN,
                    "max" => i32::MAX,
                    s => s.parse().unwrap(),
                };
                let f = sym!(FnStrAlg, "crypto_pwhash_str_alg");
                let mut out = [0u8; 128];
                // opslimit 3 is accepted by BOTH algorithms (argon2i's
                // OPSLIMIT_MIN is 3, argon2id's is 1), so alg == 1 / 2 really
                // do reach a successful hash instead of a range rejection.
                let rc = f(
                    out.as_mut_ptr() as *mut c_char,
                    PW.as_ptr() as *const c_char,
                    PW.len() as u64,
                    3,
                    8192,
                    alg,
                );
                // reached only for alg == 1 or 2
                std::process::exit(if rc == 0 { 0 } else { 70 });
            }
            // --- row 260: escrypt_PBKDF2_SHA256 with dkLen > 0x1fffffffe0 --
            // The check is the very first statement, so `buf` is never touched.
            "pbkdf2_dklen_misuse" => {
                let f = sym!(FnPbkdf2, "_sodium_escrypt_PBKDF2_SHA256");
                let mut buf = [0u8; 64];
                f(
                    PW.as_ptr(),
                    PW.len(),
                    SALT16.as_ptr(),
                    16,
                    1,
                    buf.as_mut_ptr(),
                    0x1fff_ffff_e1,
                );
            }
            "pbkdf2_dklen_misuse_max" => {
                let f = sym!(FnPbkdf2, "_sodium_escrypt_PBKDF2_SHA256");
                let mut buf = [0u8; 64];
                f(
                    PW.as_ptr(),
                    PW.len(),
                    SALT16.as_ptr(),
                    16,
                    1,
                    buf.as_mut_ptr(),
                    usize::MAX,
                );
            }
            // control: a legal dkLen must NOT abort
            "pbkdf2_dklen_ok" => {
                let f = sym!(FnPbkdf2, "_sodium_escrypt_PBKDF2_SHA256");
                let mut buf = [0u8; 64];
                f(
                    PW.as_ptr(),
                    PW.len(),
                    SALT16.as_ptr(),
                    16,
                    1,
                    buf.as_mut_ptr(),
                    64,
                );
                std::process::exit((buf[0] & 0x3f) as i32);
            }
            // --- row 225: sodium_bin2base64 with a destination that is too
            // small. ERRORS.md says "returns NULL", but in this build
            // sodium_bin2base64 detects `b64_maxlen <= b64_len` with
            // sodium_misuse() and never returns, so argon2_encode_string dies
            // instead of reporting ARGON2_ENCODING_FAIL. Verified identical in
            // C and Rust.
            c if c.starts_with("encode_b64_misuse_") => {
                let dst_len: usize = c["encode_b64_misuse_".len()..].parse().unwrap();
                let f = sym!(FnEncode, "_sodium_argon2_encode_string");
                let mut out = [0u8; 32];
                let mut salt = [0u8; 16];
                let mut ctx = Ctx::zeroed();
                ctx.out = out.as_mut_ptr();
                ctx.outlen = 32;
                ctx.salt = salt.as_mut_ptr();
                ctx.saltlen = 16;
                ctx.t_cost = 3;
                ctx.m_cost = 8;
                ctx.lanes = 1;
                ctx.threads = 1;
                let mut dst = vec![0u8; dst_len + 8];
                let rc = f(dst.as_mut_ptr() as *mut c_char, dst_len, &mut ctx, ARGON2ID);
                std::process::exit(if rc == 0 { 0 } else { 71 });
            }
            c if c.starts_with("hash_encoded_b64_misuse_") => {
                let el: usize = c["hash_encoded_b64_misuse_".len()..].parse().unwrap();
                let f = sym!(FnHashEncoded, "_sodium_argon2i_hash_encoded");
                let mut dst = vec![0u8; el + 8];
                let rc = f(
                    3,
                    8,
                    1,
                    PW.as_ptr() as *const c_void,
                    PW.len(),
                    SALT16.as_ptr() as *const c_void,
                    16,
                    32,
                    dst.as_mut_ptr() as *mut c_char,
                    el,
                );
                std::process::exit(if rc == 0 { 0 } else { 72 });
            }
            other => panic!("unknown abort case {other}"),
        }
    }
    // Reached only when the call did NOT abort.
    #[allow(unreachable_code)]
    {
        std::process::exit(0);
    }
}

// ===========================================================================
// helpers: every one calls C and Rust and compares ret + errno + buffers
// ===========================================================================
#[track_caller]
fn dt_pwhash(
    sym: &str,
    outlen: usize,
    pw: &[u8],
    salt: &[u8],
    ops: u64,
    mem: usize,
    alg: c_int,
    ctx: &str,
) -> (c_int, i32) {
    unsafe {
        let (c, r) = pair::<FnPwhash>(sym);
        let mut co = vec![0xA5u8; outlen + 8];
        let mut ro = vec![0xA5u8; outlen + 8];
        set_errno(0);
        let cr = c(
            co.as_mut_ptr(),
            outlen as u64,
            pw.as_ptr() as *const c_char,
            pw.len() as u64,
            salt.as_ptr(),
            ops,
            mem,
            alg,
        );
        let ce = errno();
        set_errno(0);
        let rr = r(
            ro.as_mut_ptr(),
            outlen as u64,
            pw.as_ptr() as *const c_char,
            pw.len() as u64,
            salt.as_ptr(),
            ops,
            mem,
            alg,
        );
        let re = errno();
        eq_i32(ctx, cr, rr);
        assert_eq!(ce, re, "{ctx}: errno differs (C={ce}, Rust={re})");
        eq_bytes(ctx, &co, &ro);
        assert_eq!(&co[outlen..], &[0xA5u8; 8], "{ctx}: wrote past outlen");
        (cr, ce)
    }
}

/// `crypto_pwhash*` where `out` and `passwd` are THE SAME pointer (aliasing).
#[track_caller]
fn dt_pwhash_aliased(sym: &str, outlen: usize, ops: u64, mem: usize, alg: c_int, ctx: &str) {
    unsafe {
        let (c, r) = pair::<FnPwhash>(sym);
        let mut cb = vec![0x33u8; outlen + 8];
        let mut rb = vec![0x33u8; outlen + 8];
        set_errno(0);
        let cr = c(
            cb.as_mut_ptr(),
            outlen as u64,
            cb.as_ptr() as *const c_char,
            outlen as u64,
            SALT16.as_ptr(),
            ops,
            mem,
            alg,
        );
        let ce = errno();
        set_errno(0);
        let rr = r(
            rb.as_mut_ptr(),
            outlen as u64,
            rb.as_ptr() as *const c_char,
            outlen as u64,
            SALT16.as_ptr(),
            ops,
            mem,
            alg,
        );
        let re = errno();
        eq_i32(ctx, cr, rr);
        assert_eq!(ce, re, "{ctx}: errno differs (C={ce}, Rust={re})");
        eq_bytes(ctx, &cb, &rb);
    }
}

#[track_caller]
fn dt_str(sym: &str, pw: &[u8], ops: u64, mem: usize, ctx: &str) -> (c_int, i32) {
    let n = if sym.contains("scrypt") { 102 } else { 128 };
    install_det_random();
    unsafe {
        let (c, r) = pair::<FnStr>(sym);
        let mut cb = vec![0x11u8; n];
        let mut rb = vec![0x11u8; n];
        det_reseed(0xABCD);
        set_errno(0);
        let cr = c(
            cb.as_mut_ptr() as *mut c_char,
            pw.as_ptr() as *const c_char,
            pw.len() as u64,
            ops,
            mem,
        );
        let ce = errno();
        det_reseed(0xABCD);
        set_errno(0);
        let rr = r(
            rb.as_mut_ptr() as *mut c_char,
            pw.as_ptr() as *const c_char,
            pw.len() as u64,
            ops,
            mem,
        );
        let re = errno();
        eq_i32(ctx, cr, rr);
        assert_eq!(ce, re, "{ctx}: errno differs (C={ce}, Rust={re})");
        eq_bytes(ctx, &cb, &rb);
        (cr, ce)
    }
}

#[track_caller]
fn dt_verify(sym: &str, s: &[u8], pw: &[u8], pwlen: u64, ctx: &str) -> (c_int, i32) {
    install_det_random();
    let mut z = s.to_vec();
    z.push(0);
    unsafe {
        let (c, r) = pair::<FnVerify>(sym);
        det_reseed(0xC0FFEE);
        set_errno(0);
        let cr = c(
            z.as_ptr() as *const c_char,
            pw.as_ptr() as *const c_char,
            pwlen,
        );
        let ce = errno();
        det_reseed(0xC0FFEE);
        set_errno(0);
        let rr = r(
            z.as_ptr() as *const c_char,
            pw.as_ptr() as *const c_char,
            pwlen,
        );
        let re = errno();
        eq_i32(ctx, cr, rr);
        assert_eq!(ce, re, "{ctx}: errno differs (C={ce}, Rust={re})");
        (cr, ce)
    }
}

#[track_caller]
fn dt_rehash(sym: &str, s: &[u8], ops: u64, mem: usize, ctx: &str) -> (c_int, i32) {
    let mut z = s.to_vec();
    z.push(0);
    unsafe {
        let (c, r) = pair::<FnRehash>(sym);
        set_errno(0);
        let cr = c(z.as_ptr() as *const c_char, ops, mem);
        let ce = errno();
        set_errno(0);
        let rr = r(z.as_ptr() as *const c_char, ops, mem);
        let re = errno();
        eq_i32(ctx, cr, rr);
        assert_eq!(ce, re, "{ctx}: errno differs (C={ce}, Rust={re})");
        (cr, ce)
    }
}

#[track_caller]
fn dt_scrypt(
    outlen: usize,
    pw: &[u8],
    salt: &[u8],
    ops: u64,
    mem: usize,
    ctx: &str,
) -> (c_int, i32) {
    unsafe {
        let (c, r) = pair::<FnScrypt>("crypto_pwhash_scryptsalsa208sha256");
        let mut co = vec![0xA5u8; outlen + 8];
        let mut ro = vec![0xA5u8; outlen + 8];
        set_errno(0);
        let cr = c(
            co.as_mut_ptr(),
            outlen as u64,
            pw.as_ptr() as *const c_char,
            pw.len() as u64,
            salt.as_ptr(),
            ops,
            mem,
        );
        let ce = errno();
        set_errno(0);
        let rr = r(
            ro.as_mut_ptr(),
            outlen as u64,
            pw.as_ptr() as *const c_char,
            pw.len() as u64,
            salt.as_ptr(),
            ops,
            mem,
        );
        let re = errno();
        eq_i32(ctx, cr, rr);
        assert_eq!(ce, re, "{ctx}: errno differs (C={ce}, Rust={re})");
        eq_bytes(ctx, &co, &ro);
        (cr, ce)
    }
}

#[track_caller]
fn dt_ll(
    pw: &[u8],
    salt: &[u8],
    n: u64,
    r_: u32,
    p: u32,
    buflen: usize,
    ctx: &str,
) -> (c_int, i32) {
    unsafe {
        let (c, r) = pair::<FnScryptLl>("crypto_pwhash_scryptsalsa208sha256_ll");
        // For the rejection cases buflen is a *claim*, not a real buffer size;
        // every rejected path returns before writing anything, which the guard
        // pattern below proves.
        let real = core::cmp::min(buflen, 256) + 8;
        let mut cb = vec![0xA5u8; real];
        let mut rb = vec![0xA5u8; real];
        set_errno(0);
        let cr = c(
            pw.as_ptr(),
            pw.len(),
            salt.as_ptr(),
            salt.len(),
            n,
            r_,
            p,
            cb.as_mut_ptr(),
            buflen,
        );
        let ce = errno();
        set_errno(0);
        let rr = r(
            pw.as_ptr(),
            pw.len(),
            salt.as_ptr(),
            salt.len(),
            n,
            r_,
            p,
            rb.as_mut_ptr(),
            buflen,
        );
        let re = errno();
        eq_i32(ctx, cr, rr);
        assert_eq!(ce, re, "{ctx}: errno differs (C={ce}, Rust={re})");
        eq_bytes(ctx, &cb, &rb);
        (cr, ce)
    }
}

/// `_sodium_argon2_validate_inputs`
#[track_caller]
fn dt_validate(c0: &Ctx, ctx: &str) -> c_int {
    unsafe {
        let (c, r) = pair::<FnValidate>("_sodium_argon2_validate_inputs");
        set_errno(0);
        let cr = c(c0 as *const Ctx);
        let ce = errno();
        set_errno(0);
        let rr = r(c0 as *const Ctx);
        let re = errno();
        eq_i32(ctx, cr, rr);
        assert_eq!(ce, re, "{ctx}: errno differs (C={ce}, Rust={re})");
        cr
    }
}

/// `_sodium_argon2_decode_string`; both libraries get their own output buffers,
/// and the resulting contexts are compared field by field.
#[track_caller]
fn dt_decode(template: &Ctx, s: &[u8], type_: c_int, ctx: &str) -> c_int {
    let mut z = s.to_vec();
    z.push(0);
    let cap = template.saltlen.max(template.outlen) as usize + 8;
    let mut csalt = vec![0u8; cap];
    let mut cout = vec![0u8; cap];
    let mut rsalt = vec![0u8; cap];
    let mut rout = vec![0u8; cap];
    let mut cc = *template;
    let mut rc_ = *template;
    cc.salt = csalt.as_mut_ptr();
    cc.out = cout.as_mut_ptr();
    rc_.salt = rsalt.as_mut_ptr();
    rc_.out = rout.as_mut_ptr();
    unsafe {
        let (c, r) = pair::<FnDecode>("_sodium_argon2_decode_string");
        set_errno(0);
        let cr = c(&mut cc, z.as_ptr() as *const c_char, type_);
        let ce = errno();
        set_errno(0);
        let rr = r(&mut rc_, z.as_ptr() as *const c_char, type_);
        let re = errno();
        eq_i32(ctx, cr, rr);
        assert_eq!(ce, re, "{ctx}: errno differs (C={ce}, Rust={re})");
        // compare the decoded scalar fields and the decoded buffers
        assert_eq!(
            (cc.outlen, cc.saltlen, cc.t_cost, cc.m_cost, cc.lanes, cc.threads),
            (
                rc_.outlen,
                rc_.saltlen,
                rc_.t_cost,
                rc_.m_cost,
                rc_.lanes,
                rc_.threads
            ),
            "{ctx}: decoded context fields differ"
        );
        eq_bytes(&format!("{ctx} salt"), &csalt, &rsalt);
        eq_bytes(&format!("{ctx} out"), &cout, &rout);
        cr
    }
}

/// `_sodium_argon2_encode_string`
#[track_caller]
fn dt_encode(template: &Ctx, dst_len: usize, type_: c_int, ctx: &str) -> c_int {
    let mut salt = vec![0x5au8; template.saltlen as usize + 1];
    let mut out = vec![0xa5u8; template.outlen as usize + 1];
    let mut cc = *template;
    let mut rc_ = *template;
    cc.salt = salt.as_mut_ptr();
    cc.out = out.as_mut_ptr();
    rc_.salt = salt.as_mut_ptr();
    rc_.out = out.as_mut_ptr();
    let mut cdst = vec![0x77u8; dst_len + 8];
    let mut rdst = vec![0x77u8; dst_len + 8];
    unsafe {
        let (c, r) = pair::<FnEncode>("_sodium_argon2_encode_string");
        set_errno(0);
        let cr = c(cdst.as_mut_ptr() as *mut c_char, dst_len, &mut cc, type_);
        let ce = errno();
        set_errno(0);
        let rr = r(rdst.as_mut_ptr() as *mut c_char, dst_len, &mut rc_, type_);
        let re = errno();
        eq_i32(ctx, cr, rr);
        assert_eq!(ce, re, "{ctx}: errno differs (C={ce}, Rust={re})");
        eq_bytes(ctx, &cdst, &rdst);
        assert_eq!(&cdst[dst_len..], &[0x77u8; 8], "{ctx}: wrote past dst_len");
        cr
    }
}

#[track_caller]
fn dt_hash_raw(
    sym: &str,
    t: u32,
    m: u32,
    par: u32,
    pw: &[u8],
    pwdlen: usize,
    salt: &[u8],
    saltlen: usize,
    hashlen: usize,
    ctx: &str,
) -> (c_int, i32) {
    // argon2_hash() starts with `randombytes_buf(hash, hashlen)`, so the
    // caller's buffer is overwritten even on the failure paths — the shared
    // deterministic implementation makes that comparable.
    install_det_random();
    let cap = core::cmp::min(hashlen, 256) + 8;
    let mut ch = vec![0xA5u8; cap];
    let mut rh = vec![0xA5u8; cap];
    unsafe {
        let (c, r) = pair::<FnHashRaw>(sym);
        det_reseed(0x1234_5678);
        set_errno(0);
        let cr = c(
            t,
            m,
            par,
            pw.as_ptr() as *const c_void,
            pwdlen,
            salt.as_ptr() as *const c_void,
            saltlen,
            ch.as_mut_ptr() as *mut c_void,
            hashlen,
        );
        let ce = errno();
        det_reseed(0x1234_5678);
        set_errno(0);
        let rr = r(
            t,
            m,
            par,
            pw.as_ptr() as *const c_void,
            pwdlen,
            salt.as_ptr() as *const c_void,
            saltlen,
            rh.as_mut_ptr() as *mut c_void,
            hashlen,
        );
        let re = errno();
        eq_i32(ctx, cr, rr);
        assert_eq!(ce, re, "{ctx}: errno differs (C={ce}, Rust={re})");
        eq_bytes(ctx, &ch, &rh);
        (cr, ce)
    }
}

#[track_caller]
fn dt_parse_setting(setting: &[u8], ctx: &str) -> (bool, u32, u32, u32) {
    let mut z = setting.to_vec();
    z.push(0);
    unsafe {
        let (c, r) = pair::<FnParseSetting>("_sodium_escrypt_parse_setting");
        let mut cn = 0xdead_beefu32;
        let mut cr_ = 0xdead_beefu32;
        let mut cp = 0xdead_beefu32;
        set_errno(0);
        let cres = c(z.as_ptr(), &mut cn, &mut cr_, &mut cp);
        let ce = errno();
        let mut rn = 0xdead_beefu32;
        let mut rr_ = 0xdead_beefu32;
        let mut rp = 0xdead_beefu32;
        set_errno(0);
        let rres = r(z.as_ptr(), &mut rn, &mut rr_, &mut rp);
        let re = errno();
        let coff = if cres.is_null() {
            usize::MAX
        } else {
            cres as usize - z.as_ptr() as usize
        };
        let roff = if rres.is_null() {
            usize::MAX
        } else {
            rres as usize - z.as_ptr() as usize
        };
        assert_eq!(coff, roff, "{ctx}: returned pointer offset differs");
        assert_eq!(
            (cn, cr_, cp),
            (rn, rr_, rp),
            "{ctx}: out params differ (C={:?} Rust={:?})",
            (cn, cr_, cp),
            (rn, rr_, rp)
        );
        assert_eq!(ce, re, "{ctx}: errno differs");
        (!cres.is_null(), cn, cr_, cp)
    }
}

// ===========================================================================
// rows 101 / 260: the two sodium_misuse() abort paths
// ===========================================================================
#[test]
fn g2_err_row_101_str_alg_bad_alg_aborts() {
    // C enums accept any int, so these are all real inputs across the FFI.
    for case in [
        "str_alg_alg_0",
        "str_alg_alg_3",
        "str_alg_alg_4",
        "str_alg_alg_255",
        "str_alg_alg_m1",
        "str_alg_alg_min",
        "str_alg_alg_max",
        "str_alg_alg_100",
    ] {
        let t = diff_abort_case(case);
        assert!(
            t.signal.is_some(),
            "row101 {case}: expected sodium_misuse()/SIGABRT, got {t:?}"
        );
    }
    // controls: the two legal algorithms must NOT abort
    for case in ["str_alg_alg_1", "str_alg_alg_2"] {
        let t = diff_abort_case(case);
        assert_eq!(
            t,
            Term {
                code: Some(0),
                signal: None
            },
            "row101 {case}: must succeed, got {t:?}"
        );
    }
}

#[test]
fn g2_err_row_260_pbkdf2_dklen_aborts() {
    for case in ["pbkdf2_dklen_misuse", "pbkdf2_dklen_misuse_max"] {
        let t = diff_abort_case(case);
        assert!(
            t.signal.is_some(),
            "row260 {case}: expected sodium_misuse()/SIGABRT, got {t:?}"
        );
    }
    let t = diff_abort_case("pbkdf2_dklen_ok");
    assert!(t.signal.is_none(), "row260 control must not abort: {t:?}");
}

// ===========================================================================
// rows 100 / 102 / 103: the generic dispatchers
// ===========================================================================
#[test]
fn g2_err_rows_100_102_103_dispatchers() {
    // row 100: alg not in {1,2} -> EINVAL / -1. `alg` is an `int` in C, so
    // every one of these is a legal FFI input.
    for alg in [
        0i32,
        3,
        4,
        100,
        255,
        -1,
        -2,
        i32::MIN,
        i32::MAX,
        0x1_0001,
    ] {
        let ctx = format!("row100 crypto_pwhash alg={alg}");
        let (rc, e) = dt_pwhash("crypto_pwhash", 32, PW, &SALT16, 1, 8192, alg, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}: expected errno=EINVAL, got {e}");
    }
    // rows 102/103: str_verify / str_needs_rehash with a non-argon2 prefix.
    let bad: &[&[u8]] = &[
        b"",
        b"x",
        b"xyz",
        b"$",
        b"$7$",
        SCRYPT_VEC.as_bytes(),
        b"$argon2d$v=19$m=8,t=1,p=1$AAA$AAA",
        b"$argon2$v=19$m=8,t=1,p=1$AAA$AAA",
        b"$argon2ix$v=19$m=8,t=1,p=1$AAA$AAA",
        b"argon2id$v=19$m=8,t=1,p=1$AAA$AAA",
        b"$ARGON2ID$v=19$m=8,t=1,p=1$AAA$AAA",
        b"$argon2i",   // one byte short of the prefix
        b"$argon2id",  // one byte short of the prefix
    ];
    for s in bad {
        let ctx = format!("row102 str_verify {:?}", String::from_utf8_lossy(s));
        let (rc, e) = dt_verify(
            "crypto_pwhash_str_verify",
            s,
            PW,
            PW.len() as u64,
            &ctx,
        );
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}: expected errno=EINVAL, got {e}");
        let ctx = format!("row103 str_needs_rehash {:?}", String::from_utf8_lossy(s));
        let (rc, e) = dt_rehash("crypto_pwhash_str_needs_rehash", s, 1, 8192, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}: expected errno=EINVAL, got {e}");
    }
}

// ===========================================================================
// rows 104-113 / 115-123: the argon2i / argon2id one-shot range checks
//
// Row 104/115 (outlen > BYTES_MAX = 4294967295) is NOT tested: the C code
// executes `memset(out, 0, outlen)` BEFORE the check, so triggering it needs a
// real 4 GiB output buffer.
// ===========================================================================
#[test]
fn g2_err_rows_104_113_argon2i_range_checks() {
    // row 105: outlen < BYTES_MIN (16) -> EINVAL, out[0..outlen) already zeroed
    for outlen in 0..16usize {
        let ctx = format!("row105 argon2i outlen={outlen}");
        let (rc, e) = dt_pwhash(
            "crypto_pwhash_argon2i",
            outlen,
            PW,
            &SALT16,
            3,
            8192,
            ARGON2I,
            &ctx,
        );
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
    }
    // row 106: passwdlen > PASSWD_MAX (4294967295) -> EFBIG.
    // passwdlen is never dereferenced before the check.
    for pwdlen in [4294967296u64, u64::MAX, 4294967295 + 1] {
        unsafe {
            let (c, r) = pair::<FnPwhash>("crypto_pwhash_argon2i");
            let mut cb = [0xA5u8; 40];
            let mut rb = [0xA5u8; 40];
            set_errno(0);
            let cr = c(
                cb.as_mut_ptr(),
                32,
                PW.as_ptr() as *const c_char,
                pwdlen,
                SALT16.as_ptr(),
                3,
                8192,
                ARGON2I,
            );
            let ce = errno();
            set_errno(0);
            let rr = r(
                rb.as_mut_ptr(),
                32,
                PW.as_ptr() as *const c_char,
                pwdlen,
                SALT16.as_ptr(),
                3,
                8192,
                ARGON2I,
            );
            let re = errno();
            let ctx = format!("row106 argon2i passwdlen={pwdlen}");
            eq_i32(&ctx, cr, rr);
            assert_eq!(ce, re, "{ctx}: errno differs");
            eq_bytes(&ctx, &cb, &rb);
            assert_eq!(cr, -1, "{ctx}");
            assert_eq!(ce, EFBIG, "{ctx}: expected EFBIG, got {ce}");
        }
    }
    // row 117: the argon2id equivalent of row 106
    unsafe {
        let (c, r) = pair::<FnPwhash>("crypto_pwhash_argon2id");
        let mut cb = [0xA5u8; 40];
        let mut rb = [0xA5u8; 40];
        set_errno(0);
        let cr = c(
            cb.as_mut_ptr(),
            32,
            PW.as_ptr() as *const c_char,
            4294967296,
            SALT16.as_ptr(),
            1,
            8192,
            ARGON2ID,
        );
        let ce = errno();
        set_errno(0);
        let rr = r(
            rb.as_mut_ptr(),
            32,
            PW.as_ptr() as *const c_char,
            4294967296,
            SALT16.as_ptr(),
            1,
            8192,
            ARGON2ID,
        );
        let re = errno();
        eq_i32("row117 argon2id passwdlen=2^32", cr, rr);
        assert_eq!(ce, re, "row117: errno differs");
        eq_bytes("row117 argon2id passwdlen=2^32", &cb, &rb);
        assert_eq!(cr, -1);
        assert_eq!(ce, EFBIG, "row117: expected EFBIG, got {ce}");
    }
    // row 107: opslimit > OPSLIMIT_MAX -> EFBIG (one step past the limit)
    for ops in [4294967296u64, u64::MAX, 4294967295 + 1] {
        let ctx = format!("row107 argon2i opslimit={ops}");
        let (rc, e) = dt_pwhash(
            "crypto_pwhash_argon2i",
            32,
            PW,
            &SALT16,
            ops,
            8192,
            ARGON2I,
            &ctx,
        );
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EFBIG, "{ctx}");
    }
    // row 108: memlimit > MEMLIMIT_MAX -> EFBIG
    let memmax: usize = 4398046510080;
    for mem in [memmax + 1, usize::MAX, usize::MAX / 2] {
        let ctx = format!("row108 argon2i memlimit={mem}");
        let (rc, e) = dt_pwhash(
            "crypto_pwhash_argon2i",
            32,
            PW,
            &SALT16,
            3,
            mem,
            ARGON2I,
            &ctx,
        );
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EFBIG, "{ctx}");
    }
    // row 109: opslimit < OPSLIMIT_MIN (3) -> EINVAL (argon2i only!)
    for ops in [0u64, 1, 2] {
        let ctx = format!("row109 argon2i opslimit={ops}");
        let (rc, e) = dt_pwhash(
            "crypto_pwhash_argon2i",
            32,
            PW,
            &SALT16,
            ops,
            8192,
            ARGON2I,
            &ctx,
        );
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
        // ... while argon2id accepts opslimit >= 1 (asymmetric OPSLIMIT_MIN)
        let ctx = format!("row109/120 argon2id opslimit={ops}");
        let (rc, _) = dt_pwhash(
            "crypto_pwhash_argon2id",
            32,
            PW,
            &SALT16,
            ops,
            8192,
            ARGON2ID,
            &ctx,
        );
        assert_eq!(rc, if ops == 0 { -1 } else { 0 }, "{ctx}");
    }
    // row 110: memlimit < MEMLIMIT_MIN (8192) -> EINVAL, incl. 8191 and 0
    for mem in [0usize, 1, 1023, 1024, 8191] {
        for (sym, ops, alg) in [
            ("crypto_pwhash_argon2i", 3u64, ARGON2I),
            ("crypto_pwhash_argon2id", 1, ARGON2ID),
        ] {
            let ctx = format!("rows110/121 {sym} memlimit={mem}");
            let (rc, e) = dt_pwhash(sym, 32, PW, &SALT16, ops, mem, alg, &ctx);
            assert_eq!(rc, -1, "{ctx}");
            assert_eq!(e, EINVAL, "{ctx}");
        }
    }
    // row 112: out == passwd (aliasing) -> EINVAL
    dt_pwhash_aliased(
        "crypto_pwhash_argon2i",
        32,
        3,
        8192,
        ARGON2I,
        "row112 argon2i out==passwd",
    );
    dt_pwhash_aliased(
        "crypto_pwhash_argon2id",
        32,
        1,
        8192,
        ARGON2ID,
        "row122 argon2id out==passwd",
    );
    dt_pwhash_aliased(
        "crypto_pwhash_scryptsalsa208sha256",
        32,
        32768,
        16777216,
        0,
        "row233 scrypt out==passwd",
    );
    // row 113: alg != ALG_ARGON2I13 for the argon2i entry point -> EINVAL
    for alg in [0i32, 2, 3, 255, -1, i32::MIN, i32::MAX] {
        let ctx = format!("row113 crypto_pwhash_argon2i alg={alg}");
        let (rc, e) = dt_pwhash(
            "crypto_pwhash_argon2i",
            32,
            PW,
            &SALT16,
            3,
            8192,
            alg,
            &ctx,
        );
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
    }
    // row 123: alg != ALG_ARGON2ID13 for the argon2id entry point -> EINVAL
    for alg in [0i32, 1, 3, 255, -1, i32::MIN, i32::MAX] {
        let ctx = format!("row123 crypto_pwhash_argon2id alg={alg}");
        let (rc, e) = dt_pwhash(
            "crypto_pwhash_argon2id",
            32,
            PW,
            &SALT16,
            1,
            8192,
            alg,
            &ctx,
        );
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
    }
    // rows 116-119: the argon2id equivalents of 105/107/108
    for outlen in 0..16usize {
        let ctx = format!("row116 argon2id outlen={outlen}");
        let (rc, e) = dt_pwhash(
            "crypto_pwhash_argon2id",
            outlen,
            PW,
            &SALT16,
            1,
            8192,
            ARGON2ID,
            &ctx,
        );
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
    }
    for ops in [4294967296u64, u64::MAX] {
        let ctx = format!("row118 argon2id opslimit={ops}");
        let (rc, e) = dt_pwhash(
            "crypto_pwhash_argon2id",
            32,
            PW,
            &SALT16,
            ops,
            8192,
            ARGON2ID,
            &ctx,
        );
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EFBIG, "{ctx}");
    }
    for mem in [memmax + 1, usize::MAX] {
        let ctx = format!("row119 argon2id memlimit={mem}");
        let (rc, e) = dt_pwhash(
            "crypto_pwhash_argon2id",
            32,
            PW,
            &SALT16,
            1,
            mem,
            ARGON2ID,
            &ctx,
        );
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EFBIG, "{ctx}");
    }
    // exact boundaries: the largest ACCEPTED opslimit is 4294967295 and the
    // smallest accepted memlimit is 8192. (A 4 TiB memlimit is not attempted.)
    let ctx = "boundary argon2id memlimit=8192 (accepted)";
    assert_eq!(
        dt_pwhash(
            "crypto_pwhash_argon2id",
            32,
            PW,
            &SALT16,
            1,
            8192,
            ARGON2ID,
            ctx
        )
        .0,
        0
    );
    let ctx = "boundary argon2i opslimit=3 (accepted)";
    assert_eq!(
        dt_pwhash(
            "crypto_pwhash_argon2i",
            32,
            PW,
            &SALT16,
            3,
            8192,
            ARGON2I,
            ctx
        )
        .0,
        0
    );
}

// ===========================================================================
// rows 125-136: crypto_pwhash_argon2i_str / crypto_pwhash_argon2id_str
// ===========================================================================
#[test]
fn g2_err_rows_125_136_str_range_checks() {
    let memmax: usize = 4398046510080;
    for (sym, opsmin) in [
        ("crypto_pwhash_argon2i_str", 3u64),
        ("crypto_pwhash_argon2id_str", 1),
    ] {
        // rows 126/132: opslimit > 4294967295 -> EFBIG, out fully zeroed
        for ops in [4294967296u64, u64::MAX] {
            let ctx = format!("rows126/132 {sym} opslimit={ops}");
            let (rc, e) = dt_str(sym, PW, ops, 8192, &ctx);
            assert_eq!(rc, -1, "{ctx}");
            assert_eq!(e, EFBIG, "{ctx}");
        }
        // rows 127/133: memlimit > MEMLIMIT_MAX -> EFBIG
        for mem in [memmax + 1, usize::MAX] {
            let ctx = format!("rows127/133 {sym} memlimit={mem}");
            let (rc, e) = dt_str(sym, PW, opsmin, mem, &ctx);
            assert_eq!(rc, -1, "{ctx}");
            assert_eq!(e, EFBIG, "{ctx}");
        }
        // rows 128/134: opslimit < OPSLIMIT_MIN -> EINVAL
        for ops in 0..opsmin {
            let ctx = format!("rows128/134 {sym} opslimit={ops}");
            let (rc, e) = dt_str(sym, PW, ops, 8192, &ctx);
            assert_eq!(rc, -1, "{ctx}");
            assert_eq!(e, EINVAL, "{ctx}");
        }
        // rows 129/135: memlimit < 8192 -> EINVAL
        for mem in [0usize, 1, 8191] {
            let ctx = format!("rows129/135 {sym} memlimit={mem}");
            let (rc, e) = dt_str(sym, PW, opsmin, mem, &ctx);
            assert_eq!(rc, -1, "{ctx}");
            assert_eq!(e, EINVAL, "{ctx}");
        }
    }
    // rows 125/131: passwdlen > PASSWD_MAX -> EFBIG, out fully zeroed.
    for sym in ["crypto_pwhash_argon2i_str", "crypto_pwhash_argon2id_str"] {
        unsafe {
            let (c, r) = pair::<FnStr>(sym);
            let mut cb = [0x11u8; 128];
            let mut rb = [0x11u8; 128];
            set_errno(0);
            let cr = c(
                cb.as_mut_ptr() as *mut c_char,
                PW.as_ptr() as *const c_char,
                4294967296,
                3,
                8192,
            );
            let ce = errno();
            set_errno(0);
            let rr = r(
                rb.as_mut_ptr() as *mut c_char,
                PW.as_ptr() as *const c_char,
                4294967296,
                3,
                8192,
            );
            let re = errno();
            let ctx = format!("rows125/131 {sym} passwdlen=4294967296");
            eq_i32(&ctx, cr, rr);
            assert_eq!(ce, re, "{ctx}: errno differs");
            eq_bytes(&ctx, &cb, &rb);
            assert_eq!(cr, -1, "{ctx}");
            assert_eq!(ce, EFBIG, "{ctx}");
            assert!(cb.iter().all(|&b| b == 0), "{ctx}: out not fully zeroed");
        }
    }
    // rows 130/136: argon2*_hash_encoded() != ARGON2_OK. Not reachable through
    // the public wrapper (which always passes a 128-byte buffer, m>=8, p=1),
    // so it is exercised on the internal entry point the wrapper calls: a
    // too-small `encodedlen` makes argon2_encode_string fail with
    // ARGON2_ENCODING_FAIL (-31), which the wrapper maps to -1.
    //
    // The exact behaviour depends on WHICH segment runs out of room (the
    // "$argon2i$v=19$m=8,t=3,p=1$" prefix is 26 chars, the base64 salt 22, the
    // base64 hash 43, total strlen 92 -> 93 bytes needed):
    //   encodedlen  0..=26 -> ARGON2_ENCODING_FAIL (an SS() literal/decimal)
    //   encodedlen 27..=48 -> sodium_misuse() inside sodium_bin2base64(salt)
    //   encodedlen      49 -> ARGON2_ENCODING_FAIL (the SS("$") separator)
    //   encodedlen 50..=92 -> sodium_misuse() inside sodium_bin2base64(hash)
    //   encodedlen     93+ -> ARGON2_OK
    // The misuse cases are compared in g2_err_row_225_encode_b64_misuse.
    for sym in [
        "_sodium_argon2i_hash_encoded",
        "_sodium_argon2id_hash_encoded",
    ] {
        for encodedlen in [0usize, 1, 8, 16, 24, 25, 26] {
            unsafe {
                let (c, r) = pair::<FnHashEncoded>(sym);
                let mut cb = vec![0x11u8; encodedlen + 8];
                let mut rb = vec![0x11u8; encodedlen + 8];
                set_errno(0);
                let cr = c(
                    3,
                    8,
                    1,
                    PW.as_ptr() as *const c_void,
                    PW.len(),
                    SALT16.as_ptr() as *const c_void,
                    16,
                    32,
                    cb.as_mut_ptr() as *mut c_char,
                    encodedlen,
                );
                let ce = errno();
                set_errno(0);
                let rr = r(
                    3,
                    8,
                    1,
                    PW.as_ptr() as *const c_void,
                    PW.len(),
                    SALT16.as_ptr() as *const c_void,
                    16,
                    32,
                    rb.as_mut_ptr() as *mut c_char,
                    encodedlen,
                );
                let re = errno();
                let ctx = format!("rows130/136 {sym} encodedlen={encodedlen}");
                eq_i32(&ctx, cr, rr);
                assert_eq!(ce, re, "{ctx}: errno differs");
                eq_bytes(&ctx, &cb, &rb);
                assert_eq!(
                    &cb[encodedlen..],
                    &[0x11u8; 8],
                    "{ctx}: wrote past encodedlen"
                );
                assert_eq!(
                    cr,
                    if encodedlen == 0 {
                        ARGON2_OK // `if (encoded && encodedlen)` skips encoding
                    } else {
                        ARGON2_ENCODING_FAIL
                    },
                    "{ctx}"
                );
            }
            // encodedlen 49 is the second ENCODING_FAIL window (the SS("$")
            // between the base64 salt and the base64 hash).
            let el = if sym.contains("argon2id") { 50 } else { 49 };
            unsafe {
                let (c, r) = pair::<FnHashEncoded>(sym);
                let mut cb = vec![0x11u8; el + 8];
                let mut rb = vec![0x11u8; el + 8];
                set_errno(0);
                let cr = c(
                    3,
                    8,
                    1,
                    PW.as_ptr() as *const c_void,
                    PW.len(),
                    SALT16.as_ptr() as *const c_void,
                    16,
                    32,
                    cb.as_mut_ptr() as *mut c_char,
                    el,
                );
                let ce = errno();
                set_errno(0);
                let rr = r(
                    3,
                    8,
                    1,
                    PW.as_ptr() as *const c_void,
                    PW.len(),
                    SALT16.as_ptr() as *const c_void,
                    16,
                    32,
                    rb.as_mut_ptr() as *mut c_char,
                    el,
                );
                let re = errno();
                let ctx = format!("rows130/136 {sym} encodedlen={el} (SS separator)");
                eq_i32(&ctx, cr, rr);
                assert_eq!(ce, re, "{ctx}: errno differs");
                eq_bytes(&ctx, &cb, &rb);
                assert_eq!(cr, ARGON2_ENCODING_FAIL, "{ctx}");
            }
        }
    }
}

// ===========================================================================
// row 225: sodium_bin2base64 with a too-small destination inside
// argon2_encode_string. In this build sodium_bin2base64 calls sodium_misuse()
// instead of returning NULL, so the whole process dies — compared in two child
// processes, one per library.
// ===========================================================================
#[test]
fn g2_err_row_225_encode_b64_misuse() {
    // argon2id prefix is 27 chars: 28..=49 dies in the salt encode,
    // 51..=93 in the hash encode; 50 and <=27 report ENCODING_FAIL instead.
    for dst_len in [28usize, 29, 48, 49, 51, 60, 92, 93] {
        let case = format!("encode_b64_misuse_{dst_len}");
        let t = diff_abort_case(&case);
        assert!(
            t.signal.is_some(),
            "row225 encode_string dst_len={dst_len}: expected sodium_misuse(), got {t:?}"
        );
    }
    for dst_len in [27usize, 50, 94] {
        let case = format!("encode_b64_misuse_{dst_len}");
        let t = diff_abort_case(&case);
        assert!(
            t.signal.is_none(),
            "row225 encode_string dst_len={dst_len}: must NOT abort, got {t:?}"
        );
    }
    // the same window through argon2i_hash_encoded (prefix 26 chars)
    for el in [27usize, 28, 48, 50, 91, 92] {
        let case = format!("hash_encoded_b64_misuse_{el}");
        let t = diff_abort_case(&case);
        assert!(
            t.signal.is_some(),
            "row225 hash_encoded encodedlen={el}: expected sodium_misuse(), got {t:?}"
        );
    }
    for el in [26usize, 49, 93] {
        let case = format!("hash_encoded_b64_misuse_{el}");
        let t = diff_abort_case(&case);
        assert!(
            t.signal.is_none(),
            "row225 hash_encoded encodedlen={el}: must NOT abort, got {t:?}"
        );
    }
}

// ===========================================================================
// rows 137-145: *_str_verify
// ===========================================================================
#[test]
fn g2_err_rows_137_145_str_verify() {
    // rows 137/142: passwdlen > PASSWD_MAX -> EFBIG (passwd is not read)
    for sym in [
        "crypto_pwhash_argon2i_str_verify",
        "crypto_pwhash_argon2id_str_verify",
        "crypto_pwhash_str_verify",
    ] {
        for pwlen in [4294967296u64, u64::MAX] {
            let ctx = format!("rows137/142 {sym} passwdlen={pwlen}");
            let s = if sym.contains("argon2i_") {
                ARGON2I_VEC
            } else {
                ARGON2ID_VEC
            };
            let (rc, e) = dt_verify(sym, s.as_bytes(), PW, pwlen, &ctx);
            assert_eq!(rc, -1, "{ctx}");
            assert_eq!(e, EFBIG, "{ctx}");
        }
    }
    // rows 139/144: password mismatch -> ARGON2_VERIFY_MISMATCH -> -1 + EINVAL
    let mut wrong = PW.to_vec();
    wrong[0] ^= 0x80;
    for (sym, s) in [
        ("crypto_pwhash_argon2i_str_verify", ARGON2I_VEC),
        ("crypto_pwhash_argon2id_str_verify", ARGON2ID_VEC),
    ] {
        let ctx = format!("rows139/144 {sym} mismatch");
        let (rc, e) = dt_verify(sym, s.as_bytes(), &wrong, wrong.len() as u64, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(
            e, EINVAL,
            "{ctx}: VERIFY_MISMATCH must set errno=EINVAL, got {e}"
        );
    }
    // rows 140/145: malformed / wrong-variant strings -> -1 with errno
    // UNTOUCHED (only VERIFY_MISMATCH sets EINVAL). The harness pre-sets errno
    // to 0, so C and Rust must both leave it at 0.
    let malformed: &[&str] = &[
        "$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw",  // no hash field
        "$argon2id$v=19$m=8,t=1,p=1$",                        // no salt/hash
        "$argon2id$v=19$m=8,t=1,p=1",                         // no '$'
        "$argon2id$v=16$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
        "$argon2id$v=20$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
        "$argon2id$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
        "$argon2id$v=19$m=0,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
        "$argon2id$v=19$m=7,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
        "$argon2id$v=19$m=8,t=0,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
        "$argon2id$v=19$m=8,t=1,p=0$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
        "$argon2id$v=19$m=8,t=1,p=2$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
        "$argon2id$v=19$m=8,t=1,p=1$AAEC$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw", // salt 3 bytes
        "$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJ",                      // hash 3 bytes
        "$argon2id$v=19$m=08,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
        "$argon2id$v=019$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
        "$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw==$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
        "$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw$",
        "$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw\n",
        "$argon2id$v=19$m=4294967296,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
    ];
    for s in malformed {
        let ctx = format!("rows140/145 malformed {s:?}");
        let (rc, e) = dt_verify(
            "crypto_pwhash_str_verify",
            s.as_bytes(),
            PW,
            PW.len() as u64,
            &ctx,
        );
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(
            e, 0,
            "{ctx}: a decode failure must NOT set errno, got {e}"
        );
    }
    // row 141: an "$argon2id$..." string given to the argon2i verifier (and
    // vice versa) -> ARGON2_DECODING_FAIL, errno untouched.
    let (rc, e) = dt_verify(
        "crypto_pwhash_argon2i_str_verify",
        ARGON2ID_VEC.as_bytes(),
        PW,
        PW.len() as u64,
        "row141 argon2id string to the argon2i verifier",
    );
    assert_eq!(rc, -1);
    assert_eq!(e, 0, "row141: errno must be untouched, got {e}");
    let (rc, e) = dt_verify(
        "crypto_pwhash_argon2id_str_verify",
        ARGON2I_VEC.as_bytes(),
        PW,
        PW.len() as u64,
        "row145 argon2i string to the argon2id verifier",
    );
    assert_eq!(rc, -1);
    assert_eq!(e, 0, "row145: errno must be untouched, got {e}");
}

// ===========================================================================
// rows 146-151: _needs_rehash (both variants)
// ===========================================================================
#[test]
fn g2_err_rows_146_151_needs_rehash() {
    let base = ARGON2ID_VEC.as_bytes();
    // row 146: opslimit > UINT32_MAX -> EINVAL
    for ops in [4294967296u64, u64::MAX] {
        for sym in [
            "crypto_pwhash_str_needs_rehash",
            "crypto_pwhash_argon2id_str_needs_rehash",
        ] {
            let ctx = format!("row146 {sym} opslimit={ops}");
            let (rc, e) = dt_rehash(sym, base, ops, 8192, &ctx);
            assert_eq!(rc, -1, "{ctx}");
            assert_eq!(e, EINVAL, "{ctx}");
        }
    }
    // row 147: memlimit/1024 > UINT32_MAX, i.e. memlimit >= 4398046511104
    for mem in [4398046511104usize, usize::MAX] {
        let ctx = format!("row147 memlimit={mem}");
        let (rc, e) = dt_rehash("crypto_pwhash_str_needs_rehash", base, 1, mem, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
    }
    // ... and one step below is accepted
    let (rc, _) = dt_rehash(
        "crypto_pwhash_str_needs_rehash",
        base,
        4294967295,
        4398046511103,
        "row147 memlimit=4398046511103 (largest accepted)",
    );
    assert_eq!(rc, 1);
    // row 148: strlen(str) >= crypto_pwhash_STRBYTES (128) -> EINVAL.
    // (an over-long but otherwise syntactically valid argon2id string)
    let long = format!(
        "$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw${}",
        "A".repeat(100)
    );
    assert!(long.len() >= 128);
    let (rc, e) = dt_rehash(
        "crypto_pwhash_str_needs_rehash",
        long.as_bytes(),
        1,
        8192,
        "row148 strlen>=128",
    );
    assert_eq!(rc, -1);
    assert_eq!(e, EINVAL, "row148");
    // row 149: strlen(str) == 0 -> calloc(0,1); the decode then fails.
    // (only reachable through the variant entry points, since the generic
    //  dispatcher rejects the empty prefix first)
    for sym in [
        "crypto_pwhash_argon2i_str_needs_rehash",
        "crypto_pwhash_argon2id_str_needs_rehash",
    ] {
        let ctx = format!("row149 {sym} empty string");
        let (rc, e) = dt_rehash(sym, b"", 1, 8192, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
    }
    // row 150: decode failure / wrong variant -> EINVAL
    for (sym, s) in [
        ("crypto_pwhash_argon2i_str_needs_rehash", ARGON2ID_VEC),
        ("crypto_pwhash_argon2id_str_needs_rehash", ARGON2I_VEC),
    ] {
        let ctx = format!("row150 {sym} wrong variant");
        let (rc, e) = dt_rehash(sym, s.as_bytes(), 1, 8192, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
    }
    for s in [
        "$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw",
        "$argon2id$v=16$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
        "$argon2id$v=19$m=8,t=1,p=0$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
        "$argon2id$v=19$m=x,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$OgCJY8Zff/pzMakLq9H9F+qfA70FNWuFfOtD97cICtw",
    ] {
        let ctx = format!("row150 malformed {s:?}");
        let (rc, e) = dt_rehash("crypto_pwhash_str_needs_rehash", s.as_bytes(), 1, 8192, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
    }
    // row 151: parameter mismatch is NOT an error -> returns 1, errno untouched
    let (rc, e) = dt_rehash(
        "crypto_pwhash_str_needs_rehash",
        base,
        2,
        8192,
        "row151 mismatch",
    );
    assert_eq!(rc, 1);
    assert_eq!(e, 0, "row151: errno must be untouched");
}

// ===========================================================================
// rows 152-165: argon2_ctx / argon2_hash / argon2_verify
// ===========================================================================
#[test]
fn g2_err_rows_152_165_argon2_hash_and_verify() {
    // rows 152/159: argon2_validate_inputs failure propagates verbatim through
    // argon2_hash (via the *_hash_raw entry points the public API uses).
    for sym in ["_sodium_argon2i_hash_raw", "_sodium_argon2id_hash_raw"] {
        // hashlen < ARGON2_MIN_OUTLEN -> ARGON2_OUTPUT_TOO_SHORT
        for hashlen in [0usize, 1, 15] {
            let ctx = format!("rows152/159 {sym} hashlen={hashlen}");
            let (rc, _) = dt_hash_raw(sym, 3, 8, 1, PW, PW.len(), &SALT16, 16, hashlen, &ctx);
            assert_eq!(rc, ARGON2_OUTPUT_TOO_SHORT, "{ctx}");
        }
        // saltlen < ARGON2_MIN_SALT_LENGTH -> ARGON2_SALT_TOO_SHORT
        for saltlen in [0usize, 1, 7] {
            let ctx = format!("rows152/159 {sym} saltlen={saltlen}");
            let (rc, _) = dt_hash_raw(sym, 3, 8, 1, PW, PW.len(), &SALT16, saltlen, 32, &ctx);
            assert_eq!(rc, ARGON2_SALT_TOO_SHORT, "{ctx}");
        }
        // m_cost < ARGON2_MIN_MEMORY / m_cost < 8*lanes / t_cost == 0 / p == 0
        for (t, m, p, want) in [
            (3u32, 0u32, 1u32, ARGON2_MEMORY_TOO_LITTLE),
            (3, 7, 1, ARGON2_MEMORY_TOO_LITTLE),
            (3, 8, 2, ARGON2_MEMORY_TOO_LITTLE),
            (3, 8, 0, ARGON2_LANES_TOO_FEW),
            (0, 8, 1, ARGON2_TIME_TOO_SMALL),
            (3, 8, 0xff_ffff + 1, ARGON2_LANES_TOO_MANY),
        ] {
            let ctx = format!("rows152/159 {sym} t={t} m={m} p={p}");
            let (rc, _) = dt_hash_raw(sym, t, m, p, PW, PW.len(), &SALT16, 16, 32, &ctx);
            assert_eq!(rc, want, "{ctx}");
        }
        // rows 155/156/157: the argon2_hash-level length checks, which fire
        // before any allocation (the pointers are never dereferenced).
        let (rc, _) = dt_hash_raw(
            sym,
            3,
            8,
            1,
            PW,
            4294967296,
            &SALT16,
            16,
            32,
            &format!("row155 {sym} pwdlen=2^32"),
        );
        assert_eq!(rc, ARGON2_PWD_TOO_LONG);
        // row 156 (hashlen > ARGON2_MAX_OUTLEN) is NOT tested: argon2_hash
        // starts with `randombytes_buf(hash, hashlen)`, so reaching the check
        // needs a real 4 GiB output buffer. Same reason as ERRORS.md rows
        // 104/115 for the public wrappers.
        let (rc, _) = dt_hash_raw(
            sym,
            3,
            8,
            1,
            PW,
            PW.len(),
            &SALT16,
            4294967296,
            32,
            &format!("row157 {sym} saltlen=2^32"),
        );
        assert_eq!(rc, ARGON2_SALT_TOO_LONG);
    }
    // row 153: argon2_ctx with a type that is neither Argon2_i nor Argon2_id.
    // C accepts any int for an enum parameter, so 0/3/-1/255 are real inputs.
    let mut out = [0u8; 32];
    let mut pwd = PW.to_vec();
    let mut salt = SALT16;
    for ty in [0i32, 3, 4, 255, -1, i32::MAX] {
        let mut cc = Ctx::zeroed();
        cc.out = out.as_mut_ptr();
        cc.outlen = 32;
        cc.pwd = pwd.as_mut_ptr();
        cc.pwdlen = pwd.len() as u32;
        cc.salt = salt.as_mut_ptr();
        cc.saltlen = 16;
        cc.t_cost = 1;
        cc.m_cost = 8;
        cc.lanes = 1;
        cc.threads = 1;
        unsafe {
            let (c, r) = pair::<FnArgon2Ctx>("_sodium_argon2_ctx");
            let mut a = cc;
            let mut b = cc;
            let mut ca = [0u8; 32];
            let mut cb = [0u8; 32];
            a.out = ca.as_mut_ptr();
            b.out = cb.as_mut_ptr();
            set_errno(0);
            let cr = c(&mut a, ty);
            let ce = errno();
            set_errno(0);
            let rr = r(&mut b, ty);
            let re = errno();
            let ctx = format!("row153 argon2_ctx type={ty}");
            eq_i32(&ctx, cr, rr);
            assert_eq!(ce, re, "{ctx}: errno differs");
            eq_bytes(&ctx, &ca, &cb);
            assert_eq!(cr, ARGON2_INCORRECT_TYPE, "{ctx}");
        }
    }
    // rows 163/164/165: argon2_verify's failure modes, on the generic entry
    // point with an explicit `type` int.
    unsafe {
        let (c, r) = pair::<FnArgon2Verify>("_sodium_argon2_verify");
        // row 164: recomputed hash != decoded hash
        let mut z = ARGON2ID_VEC.as_bytes().to_vec();
        z.push(0);
        let mut wrong = PW.to_vec();
        wrong[1] ^= 1;
        for (ty, want, label) in [
            (ARGON2ID, ARGON2_VERIFY_MISMATCH, "row164 mismatch"),
            (ARGON2I, ARGON2_DECODING_FAIL, "row163 wrong variant"),
            (0, ARGON2_INCORRECT_TYPE, "row202 type=0"),
            (7, ARGON2_INCORRECT_TYPE, "row202 type=7"),
            (-1, ARGON2_INCORRECT_TYPE, "row202 type=-1"),
        ] {
            set_errno(0);
            let cr = c(
                z.as_ptr() as *const c_char,
                wrong.as_ptr() as *const c_void,
                wrong.len(),
                ty,
            );
            let ce = errno();
            set_errno(0);
            let rr = r(
                z.as_ptr() as *const c_char,
                wrong.as_ptr() as *const c_void,
                wrong.len(),
                ty,
            );
            let re = errno();
            let ctx = format!("{label} argon2_verify type={ty}");
            eq_i32(&ctx, cr, rr);
            assert_eq!(ce, re, "{ctx}: errno differs");
            assert_eq!(cr, want, "{ctx}");
        }
        // row 165: argon2_hash on the decoded params fails. The decoded string
        // has an m_cost that validate_inputs accepts but a 4-byte hash that it
        // does not, so the failure comes out of the decoder itself; a valid
        // decode with a hash shorter than ARGON2_MIN_OUTLEN cannot exist.
        // Instead: a decoded p that exceeds the salt/memory relation.
        let (cv, rv) = pair::<FnArgon2iVerify>("_sodium_argon2id_verify");
        let bad = b"$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAAAAAAAAAAAAAAAAAAAAA\0";
        set_errno(0);
        let cr = cv(
            bad.as_ptr() as *const c_char,
            PW.as_ptr() as *const c_void,
            PW.len(),
        );
        let ce = errno();
        set_errno(0);
        let rr = rv(
            bad.as_ptr() as *const c_char,
            PW.as_ptr() as *const c_void,
            PW.len(),
        );
        let re = errno();
        eq_i32("row165 argon2id_verify 16-byte hash", cr, rr);
        assert_eq!(ce, re);
        assert_eq!(cr, ARGON2_VERIFY_MISMATCH);
    }
    // row 154 needs an allocation failure inside argon2_initialize; only the
    // NULL-argument branch (row 195) is reachable without an OOM injection.
    unsafe {
        let (c, r) = pair::<unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int>(
            "_sodium_argon2_initialize",
        );
        set_errno(0);
        let cr = c(ptr::null_mut(), ptr::null_mut());
        let ce = errno();
        set_errno(0);
        let rr = r(ptr::null_mut(), ptr::null_mut());
        let re = errno();
        eq_i32("row195 argon2_initialize(NULL,NULL)", cr, rr);
        assert_eq!(ce, re);
        assert_eq!(cr, ARGON2_INCORRECT_PARAMETER);
    }
}

// ===========================================================================
// rows 166-190: argon2_validate_inputs, called directly
// ===========================================================================
#[test]
fn g2_err_rows_166_190_validate_inputs() {
    let mut out = [0u8; 64];
    let mut pwd = [0u8; 32];
    let mut salt = [0u8; 32];
    let mut extra = [0u8; 32];
    let good = {
        let mut c = Ctx::zeroed();
        c.out = out.as_mut_ptr();
        c.outlen = 32;
        c.pwd = pwd.as_mut_ptr();
        c.pwdlen = 32;
        c.salt = salt.as_mut_ptr();
        c.saltlen = 16;
        c.t_cost = 1;
        c.m_cost = 8;
        c.lanes = 1;
        c.threads = 1;
        c
    };
    assert_eq!(
        dt_validate(&good, "baseline valid context"),
        ARGON2_OK,
        "the baseline context must validate"
    );
    // row 166: context == NULL
    unsafe {
        let (c, r) = pair::<FnValidate>("_sodium_argon2_validate_inputs");
        set_errno(0);
        let cr = c(ptr::null());
        let ce = errno();
        set_errno(0);
        let rr = r(ptr::null());
        let re = errno();
        eq_i32("row166 validate(NULL)", cr, rr);
        assert_eq!(ce, re);
        assert_eq!(cr, ARGON2_INCORRECT_PARAMETER);
    }
    // one variation per remaining row
    let cases: Vec<(&str, Ctx, c_int)> = vec![
        // row 167: out == NULL
        ("row167 out=NULL", {
            let mut c = good;
            c.out = ptr::null_mut();
            c
        }, ARGON2_OUTPUT_PTR_NULL),
        // row 168: outlen < ARGON2_MIN_OUTLEN (16)
        ("row168 outlen=0", {
            let mut c = good;
            c.outlen = 0;
            c
        }, ARGON2_OUTPUT_TOO_SHORT),
        ("row168 outlen=15", {
            let mut c = good;
            c.outlen = 15;
            c
        }, ARGON2_OUTPUT_TOO_SHORT),
        ("row168 outlen=16 (boundary, accepted)", {
            let mut c = good;
            c.outlen = 16;
            c
        }, ARGON2_OK),
        // row 170: pwd == NULL && pwdlen != 0
        ("row170 pwd=NULL pwdlen=32", {
            let mut c = good;
            c.pwd = ptr::null_mut();
            c
        }, ARGON2_PWD_PTR_MISMATCH),
        ("row170 pwd=NULL pwdlen=0 (accepted)", {
            let mut c = good;
            c.pwd = ptr::null_mut();
            c.pwdlen = 0;
            c
        }, ARGON2_OK),
        // row 173: salt == NULL && saltlen != 0
        ("row173 salt=NULL saltlen=16", {
            let mut c = good;
            c.salt = ptr::null_mut();
            c
        }, ARGON2_SALT_PTR_MISMATCH),
        // NOTE salt == NULL && saltlen == 0 then fails SALT_TOO_SHORT (MIN=8)
        ("row174 salt=NULL saltlen=0", {
            let mut c = good;
            c.salt = ptr::null_mut();
            c.saltlen = 0;
            c
        }, ARGON2_SALT_TOO_SHORT),
        // row 174: saltlen < ARGON2_MIN_SALT_LENGTH (8)
        ("row174 saltlen=0", {
            let mut c = good;
            c.saltlen = 0;
            c
        }, ARGON2_SALT_TOO_SHORT),
        ("row174 saltlen=7", {
            let mut c = good;
            c.saltlen = 7;
            c
        }, ARGON2_SALT_TOO_SHORT),
        ("row174 saltlen=8 (boundary, accepted)", {
            let mut c = good;
            c.saltlen = 8;
            c
        }, ARGON2_OK),
        // row 176: secret == NULL && secretlen != 0
        ("row176 secret=NULL secretlen=1", {
            let mut c = good;
            c.secretlen = 1;
            c
        }, ARGON2_SECRET_PTR_MISMATCH),
        ("row177/178 secret!=NULL secretlen=0 (accepted, MIN=0)", {
            let mut c = good;
            c.secret = extra.as_mut_ptr();
            c.secretlen = 0;
            c
        }, ARGON2_OK),
        // row 179: ad == NULL && adlen != 0
        ("row179 ad=NULL adlen=1", {
            let mut c = good;
            c.adlen = 1;
            c
        }, ARGON2_AD_PTR_MISMATCH),
        ("row180/181 ad!=NULL adlen=0 (accepted, MIN=0)", {
            let mut c = good;
            c.ad = extra.as_mut_ptr();
            c.adlen = 0;
            c
        }, ARGON2_OK),
        // row 182: lanes < ARGON2_MIN_LANES (1)
        ("row182 lanes=0", {
            let mut c = good;
            c.lanes = 0;
            c
        }, ARGON2_LANES_TOO_FEW),
        // row 183: lanes > ARGON2_MAX_LANES (0xFFFFFF)
        ("row183 lanes=0x1000000", {
            let mut c = good;
            c.lanes = 0x100_0000;
            c
        }, ARGON2_LANES_TOO_MANY),
        ("row183 lanes=0xFFFFFF (boundary) -> m_cost < 8*lanes", {
            let mut c = good;
            c.lanes = 0xff_ffff;
            c
        }, ARGON2_MEMORY_TOO_LITTLE),
        // row 184: m_cost < ARGON2_MIN_MEMORY (8)
        ("row184 m_cost=0", {
            let mut c = good;
            c.m_cost = 0;
            c
        }, ARGON2_MEMORY_TOO_LITTLE),
        ("row184 m_cost=7", {
            let mut c = good;
            c.m_cost = 7;
            c
        }, ARGON2_MEMORY_TOO_LITTLE),
        ("row184 m_cost=8 (boundary, accepted)", {
            let mut c = good;
            c.m_cost = 8;
            c
        }, ARGON2_OK),
        // row 185: m_cost > ARGON2_MAX_MEMORY == 0xFFFFFFFF on 64-bit, i.e.
        // unreachable through a uint32_t field. The largest value is accepted:
        ("row185 m_cost=0xFFFFFFFF (accepted on 64-bit)", {
            let mut c = good;
            c.m_cost = 0xffff_ffff;
            c
        }, ARGON2_OK),
        // row 186: m_cost < 8 * lanes
        ("row186 m_cost=8 lanes=2", {
            let mut c = good;
            c.m_cost = 8;
            c.lanes = 2;
            c.threads = 2;
            c
        }, ARGON2_MEMORY_TOO_LITTLE),
        ("row186 m_cost=15 lanes=2", {
            let mut c = good;
            c.m_cost = 15;
            c.lanes = 2;
            c.threads = 2;
            c
        }, ARGON2_MEMORY_TOO_LITTLE),
        ("row186 m_cost=16 lanes=2 (boundary, accepted)", {
            let mut c = good;
            c.m_cost = 16;
            c.lanes = 2;
            c.threads = 2;
            c
        }, ARGON2_OK),
        ("row186 m_cost=31 lanes=4", {
            let mut c = good;
            c.m_cost = 31;
            c.lanes = 4;
            c.threads = 4;
            c
        }, ARGON2_MEMORY_TOO_LITTLE),
        // row 187: t_cost < ARGON2_MIN_TIME (1)
        ("row187 t_cost=0", {
            let mut c = good;
            c.t_cost = 0;
            c
        }, ARGON2_TIME_TOO_SMALL),
        // row 188: t_cost > ARGON2_MAX_TIME is unreachable (uint32_t);
        // the largest value is accepted.
        ("row188 t_cost=0xFFFFFFFF (accepted)", {
            let mut c = good;
            c.t_cost = 0xffff_ffff;
            c
        }, ARGON2_OK),
        // row 189: threads < ARGON2_MIN_THREADS (1). Not shadowed here because
        // validate_inputs is called directly with lanes != threads.
        ("row189 threads=0", {
            let mut c = good;
            c.threads = 0;
            c
        }, ARGON2_THREADS_TOO_FEW),
        // row 190: threads > ARGON2_MAX_THREADS (0xFFFFFF)
        ("row190 threads=0x1000000", {
            let mut c = good;
            c.threads = 0x100_0000;
            c
        }, ARGON2_THREADS_TOO_MANY),
        ("row190 threads=0xFFFFFF (boundary, accepted)", {
            let mut c = good;
            c.threads = 0xff_ffff;
            c
        }, ARGON2_OK),
    ];
    for (label, c, want) in cases {
        let got = dt_validate(&c, label);
        assert_eq!(got, want, "{label}: C returned {got}, expected {want}");
    }
    // rows 171/172/175/177/178/180/181 are statically unreachable
    // (ARGON2_MIN_PWD_LENGTH / MIN_AD_LENGTH / MIN_SECRET are all 0 and the
    // corresponding MAX values equal UINT32_MAX, the field width), so only
    // their accepted side is asserted above.
}

// ===========================================================================
// rows 198-226: argon2_decode_string / argon2_encode_string
// ===========================================================================
#[test]
fn g2_err_rows_198_226_encoding() {
    // Template: the ctx the public verifier builds — salt/out buffers as long
    // as the encoded string, so only the *content* can make the decode fail.
    let template = {
        let mut c = Ctx::zeroed();
        c.saltlen = 160;
        c.outlen = 160;
        c
    };
    // (string, type, expected internal ARGON2_* code, row)
    let cases: &[(&str, c_int, c_int, &str)] = &[
        // --- row 201: wrong prefix for the requested type -------------------
        ("", ARGON2ID, ARGON2_DECODING_FAIL, "row201"),
        ("$", ARGON2ID, ARGON2_DECODING_FAIL, "row201"),
        ("$argon2i$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_DECODING_FAIL, "row201"),
        ("$argon2d$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2I, ARGON2_DECODING_FAIL, "row201"),
        ("$ARGON2ID$v=19$m=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row201"),
        // --- row 203: the "$v=" segment is mandatory ------------------------
        ("$argon2id$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_DECODING_FAIL, "row203"),
        ("$argon2i$m=8,t=3,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2I, ARGON2_DECODING_FAIL, "row203"),
        // "$argon2id$..." fed to the argon2i decoder: "$argon2i" matches, then
        // the leftover is "d$v=" and CC("$v=") fails.
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2I, ARGON2_DECODING_FAIL, "rows203/230"),
        // --- rows 198/199/200/204: decode_decimal on the version ------------
        ("$argon2id$v=$m=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row198"),
        ("$argon2id$v=x$m=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row198"),
        ("$argon2id$v=019$m=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row199"),
        ("$argon2id$v=0019$m=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row199"),
        ("$argon2id$v=4294967296$m=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row204"),
        ("$argon2id$v=99999999999999999999999999$m=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row200"),
        // --- row 205: version != 19 ----------------------------------------
        ("$argon2id$v=16$m=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_INCORRECT_TYPE, "row205"),
        ("$argon2id$v=20$m=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_INCORRECT_TYPE, "row205"),
        ("$argon2id$v=0$m=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_INCORRECT_TYPE, "row205"),
        ("$argon2id$v=18$m=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_INCORRECT_TYPE, "row205"),
        ("$argon2id$v=4294967295$m=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_INCORRECT_TYPE, "row205"),
        // --- row 206: missing "$m=" ----------------------------------------
        ("$argon2id$v=19m=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row206"),
        ("$argon2id$v=19$n=8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row206"),
        ("$argon2id$v=19", ARGON2ID, ARGON2_DECODING_FAIL, "row206"),
        // --- row 207: m not minimal / > UINT32_MAX -------------------------
        ("$argon2id$v=19$m=08,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row207"),
        ("$argon2id$v=19$m=,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row207"),
        ("$argon2id$v=19$m=x,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row207"),
        ("$argon2id$v=19$m=4294967296,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row207"),
        ("$argon2id$v=19$m=-8,t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row207"),
        // --- row 209: missing ",t=" ---------------------------------------
        ("$argon2id$v=19$m=8;t=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row209"),
        ("$argon2id$v=19$m=8,u=1,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row209"),
        ("$argon2id$v=19$m=8", ARGON2ID, ARGON2_DECODING_FAIL, "row209"),
        // --- row 210: t not minimal / too large ---------------------------
        ("$argon2id$v=19$m=8,t=01,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row210"),
        ("$argon2id$v=19$m=8,t=,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row210"),
        ("$argon2id$v=19$m=8,t=4294967296,p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row210"),
        // --- row 212: missing ",p=" ---------------------------------------
        ("$argon2id$v=19$m=8,t=1;p=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row212"),
        ("$argon2id$v=19$m=8,t=1,q=1$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row212"),
        ("$argon2id$v=19$m=8,t=1", ARGON2ID, ARGON2_DECODING_FAIL, "row212"),
        // --- row 213: p not minimal / too large ---------------------------
        ("$argon2id$v=19$m=8,t=1,p=01$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row213"),
        ("$argon2id$v=19$m=8,t=1,p=$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row213"),
        ("$argon2id$v=19$m=8,t=1,p=4294967296$AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row213"),
        // --- row 215: missing "$" before the salt --------------------------
        ("$argon2id$v=19$m=8,t=1,p=1,AAA$AAA", ARGON2ID, ARGON2_DECODING_FAIL, "row215"),
        ("$argon2id$v=19$m=8,t=1,p=1", ARGON2ID, ARGON2_DECODING_FAIL, "row215"),
        // --- row 216: invalid base64 in the salt field ---------------------
        ("$argon2id$v=19$m=8,t=1,p=1$!!!!!!!!!!!!!!!!!!!!!!$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_DECODING_FAIL, "row216"),
        ("$argon2id$v=19$m=8,t=1,p=1$-_-_-_-_-_-_-_-_-_-_-_$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_DECODING_FAIL, "row216"),
        // --- row 218: missing "$" between salt and hash --------------------
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_DECODING_FAIL, "row218"),
        // --- row 219: invalid base64 in the hash field ---------------------
        // NOTE sodium_base642bin is called with a non-NULL b64_end, so it
        // *succeeds* with bin_len = 0 and reports the offending character; the
        // failure therefore surfaces as ARGON2_OUTPUT_TOO_SHORT from
        // argon2_validate_inputs, not as ARGON2_DECODING_FAIL. (The salt field
        // above instead fails the following CC("$"), giving DECODING_FAIL.)
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$!!!!", ARGON2ID, ARGON2_OUTPUT_TOO_SHORT, "row219"),
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$-_-_", ARGON2ID, ARGON2_OUTPUT_TOO_SHORT, "row219"),
        // --- row 220: '=' padding in either base64 field -------------------
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw==$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_DECODING_FAIL, "row220"),
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0OD==", ARGON2ID, ARGON2_DECODING_FAIL, "row220"),
        // --- row 221: validate_inputs on the decoded parameters ------------
        ("$argon2id$v=19$m=0,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_MEMORY_TOO_LITTLE, "row221 m<8"),
        ("$argon2id$v=19$m=7,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_MEMORY_TOO_LITTLE, "row221 m=7"),
        ("$argon2id$v=19$m=8,t=1,p=2$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_MEMORY_TOO_LITTLE, "row221/186 m<8*p"),
        ("$argon2id$v=19$m=8,t=0,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_TIME_TOO_SMALL, "row221 t=0"),
        ("$argon2id$v=19$m=8,t=1,p=0$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_LANES_TOO_FEW, "row221 p=0"),
        // lanes are validated BEFORE m_cost, so p > ARGON2_MAX_LANES gives
        // LANES_TOO_MANY rather than MEMORY_TOO_LITTLE.
        ("$argon2id$v=19$m=8,t=1,p=16777216$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_LANES_TOO_MANY, "row183/221 p > MAX_LANES"),
        ("$argon2id$v=19$m=8,t=1,p=16777215$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_MEMORY_TOO_LITTLE, "row183/221 p == MAX_LANES -> m < 8*p"),
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQ$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_SALT_TOO_SHORT, "row221/174 salt 5 bytes"),
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQ", ARGON2ID, ARGON2_OUTPUT_TOO_SHORT, "row221/168 hash 5 bytes"),
        // --- row 222: trailing characters ---------------------------------
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw$extra", ARGON2ID, ARGON2_DECODING_FAIL, "row222"),
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw\n", ARGON2ID, ARGON2_DECODING_FAIL, "row222"),
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw ", ARGON2ID, ARGON2_DECODING_FAIL, "row222"),
        // --- row 202: type is neither Argon2_i nor Argon2_id ---------------
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", 0, ARGON2_INCORRECT_TYPE, "row202"),
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", 3, ARGON2_INCORRECT_TYPE, "row202"),
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", -1, ARGON2_INCORRECT_TYPE, "row202"),
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", 255, ARGON2_INCORRECT_TYPE, "row202"),
        // --- valid control -------------------------------------------------
        ("$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw", ARGON2ID, ARGON2_OK, "control valid"),
    ];
    for (s, ty, want, row) in cases {
        let ctx = format!("{row} decode_string type={ty} {s:?}");
        let got = dt_decode(&template, s.as_bytes(), *ty, &ctx);
        assert_eq!(got, *want, "{ctx}: C returned {got}, expected {want}");
    }
    // row 217: a salt field that decodes to more bytes than ctx->saltlen.
    // The base642bin call is bounded by maxsaltlen, so an over-long salt is
    // rejected with ARGON2_DECODING_FAIL.
    let small = {
        let mut c = Ctx::zeroed();
        c.saltlen = 8;
        c.outlen = 160;
        c
    };
    let got = dt_decode(
        &small,
        b"$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw",
        ARGON2ID,
        "rows216/217 salt longer than ctx->saltlen",
    );
    assert_eq!(got, ARGON2_DECODING_FAIL);
    let small = {
        let mut c = Ctx::zeroed();
        c.saltlen = 160;
        c.outlen = 8;
        c
    };
    let got = dt_decode(
        &small,
        b"$argon2id$v=19$m=8,t=1,p=1$AAECAwQFBgcICQoLDA0ODw$AAECAwQFBgcICQoLDA0ODwAAECAwQFBgcICQoLDA0ODw",
        ARGON2ID,
        "row219 hash longer than ctx->outlen",
    );
    assert_eq!(got, ARGON2_DECODING_FAIL);

    // ---------------------------------------------------------------------
    // rows 223-226: argon2_encode_string
    // ---------------------------------------------------------------------
    let enc_ctx = {
        let mut c = Ctx::zeroed();
        c.outlen = 32;
        c.saltlen = 16;
        c.t_cost = 3;
        c.m_cost = 8;
        c.lanes = 1;
        c.threads = 1;
        c
    };
    // row 224: a dst_len too small for one of the literal / decimal segments
    // (`SS`/`SX`) -> ARGON2_ENCODING_FAIL. For "$argon2id$..." the prefix is 27
    // chars, so 0..=27 fails in the prefix and 50 fails on the "$" separator
    // between the base64 salt and the base64 hash. The values in between hit
    // sodium_bin2base64's sodium_misuse() and are covered by
    // g2_err_row_225_encode_b64_misuse instead.
    for dst_len in [0usize, 1, 11, 12, 13, 20, 26, 27, 50] {
        let ctx = format!("row224 encode_string dst_len={dst_len}");
        let got = dt_encode(&enc_ctx, dst_len, ARGON2ID, &ctx);
        assert_eq!(got, ARGON2_ENCODING_FAIL, "{ctx}");
    }
    // ... and the exact required length (strlen 93 + NUL) succeeds
    let got = dt_encode(&enc_ctx, 94, ARGON2ID, "encode_string dst_len=94 (exact)");
    assert_eq!(got, ARGON2_OK);
    let got = dt_encode(&enc_ctx, 128, ARGON2ID, "encode_string dst_len=128");
    assert_eq!(got, ARGON2_OK);
    // row 223: type is neither Argon2_i nor Argon2_id -> ARGON2_ENCODING_FAIL
    for ty in [0i32, 3, 4, 255, -1, i32::MAX] {
        let ctx = format!("row223 encode_string type={ty}");
        let got = dt_encode(&enc_ctx, 128, ty, &ctx);
        assert_eq!(got, ARGON2_ENCODING_FAIL, "{ctx}");
    }
    // row 226: validate_inputs failure propagates verbatim (NOT ENCODING_FAIL)
    for (label, mutate, want) in [
        ("row226 outlen=8", 1u32, ARGON2_OUTPUT_TOO_SHORT),
        ("row226 saltlen=4", 2, ARGON2_SALT_TOO_SHORT),
        ("row226 m_cost=0", 3, ARGON2_MEMORY_TOO_LITTLE),
        ("row226 t_cost=0", 4, ARGON2_TIME_TOO_SMALL),
        ("row226 lanes=0", 5, ARGON2_LANES_TOO_FEW),
    ] {
        let mut c = enc_ctx;
        match mutate {
            1 => c.outlen = 8,
            2 => c.saltlen = 4,
            3 => c.m_cost = 0,
            4 => c.t_cost = 0,
            _ => {
                c.lanes = 0;
            }
        }
        let got = dt_encode(&c, 128, ARGON2ID, label);
        assert_eq!(got, want, "{label}: C returned {got}, expected {want}");
    }
}

// ===========================================================================
// rows 227/228: blake2b_long
// ===========================================================================
#[test]
fn g2_err_rows_227_228_blake2b_long() {
    unsafe {
        let (c, r) = pair::<FnBlake2bLong>("_sodium_blake2b_long");
        // row 227: outlen > UINT32_MAX -> -1 before anything is written
        for outlen in [4294967296usize, usize::MAX, usize::MAX / 2] {
            let mut cb = [0xA5u8; 64];
            let mut rb = [0xA5u8; 64];
            set_errno(0);
            let cr = c(
                cb.as_mut_ptr() as *mut c_void,
                outlen,
                PW.as_ptr() as *const c_void,
                PW.len(),
            );
            let ce = errno();
            set_errno(0);
            let rr = r(
                rb.as_mut_ptr() as *mut c_void,
                outlen,
                PW.as_ptr() as *const c_void,
                PW.len(),
            );
            let re = errno();
            let ctx = format!("row227 blake2b_long outlen={outlen}");
            eq_i32(&ctx, cr, rr);
            assert_eq!(ce, re, "{ctx}: errno differs");
            eq_bytes(&ctx, &cb, &rb);
            assert_eq!(cr, -1, "{ctx}");
            assert_eq!(cb, [0xA5u8; 64], "{ctx}: buffer was touched");
        }
        // row 228: outlen == 0 makes crypto_generichash_blake2b_init fail
        // (it rejects outlen <= 0), and blake2b_long returns that value.
        let mut cb = [0xA5u8; 64];
        let mut rb = [0xA5u8; 64];
        set_errno(0);
        let cr = c(
            cb.as_mut_ptr() as *mut c_void,
            0,
            PW.as_ptr() as *const c_void,
            PW.len(),
        );
        let ce = errno();
        set_errno(0);
        let rr = r(
            rb.as_mut_ptr() as *mut c_void,
            0,
            PW.as_ptr() as *const c_void,
            PW.len(),
        );
        let re = errno();
        eq_i32("row228 blake2b_long outlen=0", cr, rr);
        assert_eq!(ce, re);
        eq_bytes("row228 blake2b_long outlen=0", &cb, &rb);
        assert_eq!(cr, -1);
        // control: valid lengths across the 64-byte BLAKE2b boundary
        for outlen in [1usize, 16, 32, 64, 65, 100, 128, 129] {
            let mut cb = vec![0xA5u8; outlen + 8];
            let mut rb = vec![0xA5u8; outlen + 8];
            set_errno(0);
            let cr = c(
                cb.as_mut_ptr() as *mut c_void,
                outlen,
                PW.as_ptr() as *const c_void,
                PW.len(),
            );
            set_errno(0);
            let rr = r(
                rb.as_mut_ptr() as *mut c_void,
                outlen,
                PW.as_ptr() as *const c_void,
                PW.len(),
            );
            let ctx = format!("blake2b_long control outlen={outlen}");
            eq_i32(&ctx, cr, rr);
            eq_bytes(&ctx, &cb, &rb);
            assert_eq!(cr, 0, "{ctx}");
        }
    }
}

// ===========================================================================
// rows 229-234: crypto_pwhash_scryptsalsa208sha256 (one-shot)
//
// Row 230 (outlen > BYTES_MAX = 137438953440) is NOT tested: the C code
// memsets `out` over the full `outlen` BEFORE the check.
// Row 229 (passwdlen > SODIUM_SIZE_MAX) and row 232 (pickparams != 0) are
// statically unreachable.
// ===========================================================================
#[test]
fn g2_err_rows_229_234_scrypt_oneshot() {
    let salt32: Vec<u8> = (0..32u8).collect();
    // row 231: outlen < BYTES_MIN (16) -> EINVAL
    for outlen in 0..16usize {
        let ctx = format!("row231 scrypt outlen={outlen}");
        let (rc, e) = dt_scrypt(outlen, PW, &salt32, 32768, 16777216, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
    }
    // outlen == 16 is the boundary and must succeed
    let (rc, _) = dt_scrypt(16, PW, &salt32, 32768, 16777216, "boundary outlen=16");
    assert_eq!(rc, 0);
    // row 232: opslimit / memlimit are NOT range-checked here — every value
    // below the documented minima is silently accepted.
    for (ops, mem) in [
        (0u64, 0usize),
        (1, 1),
        (0, 16777216),
        (32767, 16777215),
        (32768, 0),
    ] {
        let ctx = format!("row232 scrypt ops={ops} mem={mem} (no range check)");
        let (rc, _) = dt_scrypt(16, PW, &salt32, ops, mem, &ctx);
        assert_eq!(rc, 0, "{ctx}: must SUCCEED, not be rejected");
    }
    // row 234: crypto_pwhash_scryptsalsa208sha256_ll() fails on the (N, r, p)
    // pickparams derived. opslimit = UINT64_MAX with memlimit = 0 gives
    // N = 2, r = 8, p = 0x3fffffff/8 = 134217727, so B_size = 128*r*p = 128 GiB
    // and the region allocation fails -> -1 with the _ll layer's errno.
    let ctx = "row234 scrypt ops=UINT64_MAX mem=0 -> _ll allocation failure";
    let (rc, e) = dt_scrypt(16, PW, &salt32, u64::MAX, 0, ctx);
    assert_eq!(rc, -1, "{ctx}");
    assert_eq!(e, ENOMEM, "{ctx}: expected the _ll layer's ENOMEM, got {e}");
    // row 233 (out == passwd) is covered in g2_err_rows_104_113_argon2i_range_checks.
    // Row 229 (passwdlen > PASSWD_MAX == SODIUM_SIZE_MAX) and row 232's
    // `pickparams() != 0` arm are statically unreachable; row 230
    // (outlen > BYTES_MAX) needs a 137 GiB output buffer because `out` is
    // memset over the full outlen before the check.
}

// ===========================================================================
// rows 235-243: scrypt _str / _str_verify
// ===========================================================================
#[test]
fn g2_err_rows_235_243_scrypt_str_and_verify() {
    // rows 240/242/243: str_verify.
    // row 240: sodium_strnlen(str,102) != 101 -> -1, errno NOT set.
    let v = SCRYPT_VEC.as_bytes();
    let too_short = v[..100].to_vec();
    let mut too_long = v.to_vec();
    too_long.push(b'x');
    let mut with_nul = v.to_vec();
    with_nul[50] = 0;
    let bad_len: Vec<Vec<u8>> = vec![
        vec![],
        b"$".to_vec(),
        b"$7$".to_vec(),
        too_short.clone(),
        v[..50].to_vec(),
        too_long.clone(),
        {
            let mut x = v.to_vec();
            x.extend_from_slice(b"yyy");
            x
        },
        with_nul.clone(),
    ];
    for (i, s) in bad_len.iter().enumerate() {
        let ctx = format!("row240 scrypt str_verify bad length #{i} (len {})", s.len());
        let (rc, e) = dt_verify(
            "crypto_pwhash_scryptsalsa208sha256_str_verify",
            s,
            PW,
            PW.len() as u64,
            &ctx,
        );
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, 0, "{ctx}: the length check must NOT set errno, got {e}");
    }
    // row 242: exactly 101 chars but not a valid "$7$" setting.
    // (a) bad "$7$" prefix
    let mut bad_prefix = v.to_vec();
    bad_prefix[1] = b'8';
    // (b) an N_log2 character outside the itoa64 alphabet
    let mut bad_nlog2 = v.to_vec();
    bad_nlog2[3] = b'$';
    // (c) an r character outside the alphabet
    let mut bad_r = v.to_vec();
    bad_r[5] = b'-';
    // (d) a p character outside the alphabet
    let mut bad_p = v.to_vec();
    bad_p[10] = b'+';
    // (e) the salt field made longer than 43 chars (need > 102)
    let mut long_salt = v.to_vec();
    long_salt[57] = b'A'; // erase the '$' between salt and hash
    // (f) N_log2 = 0 -> N = 1 -> escrypt_kdf rejects it with EINVAL (row 273)
    let mut nlog2_zero = v.to_vec();
    nlog2_zero[3] = b'.';
    for (label, s, want_errno) in [
        ("row242(a) bad $7$ prefix", &bad_prefix, None),
        ("row242(b) bad N_log2 char", &bad_nlog2, None),
        ("row242(c) bad r char", &bad_r, None),
        ("row242(d) bad p char", &bad_p, None),
        ("row242/271(e) salt field > 43 chars", &long_salt, None),
        ("row242/273(f) N_log2=0 -> N=1", &nlog2_zero, Some(EINVAL)),
    ] {
        assert_eq!(s.len(), 101, "{label}: must stay 101 chars");
        let (rc, e) = dt_verify(
            "crypto_pwhash_scryptsalsa208sha256_str_verify",
            s,
            PW,
            PW.len() as u64,
            label,
        );
        assert_eq!(rc, -1, "{label}");
        if let Some(w) = want_errno {
            assert_eq!(e, w, "{label}: expected errno={w}, got {e}");
        }
    }
    // row 243: wrong password -> the sodium_memcmp result, errno untouched
    for pw in [&b""[..], b"Password", b"passwore", b"passwordx"] {
        let ctx = format!("row243 wrong password {:?}", String::from_utf8_lossy(pw));
        let (rc, e) = dt_verify(
            "crypto_pwhash_scryptsalsa208sha256_str_verify",
            v,
            pw,
            pw.len() as u64,
            &ctx,
        );
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, 0, "{ctx}: errno must be untouched, got {e}");
    }
    // rows 235-238 relate to _str: passwdlen > SODIUM_SIZE_MAX (unreachable on
    // 64-bit), pickparams != 0 (unreachable), escrypt_gensalt_r == NULL
    // (unreachable: pickparams caps N_log2 at 63 and r*p at 0x3fffffff) and
    // escrypt_init_local != 0 (statically 0).
    // What IS observable is that no (opslimit, memlimit) pair is *range*-checked:
    for (ops, mem) in [(0u64, 0usize), (1, 1), (0, usize::MAX), (32767, 8)] {
        let ctx = format!("rows236-238 scrypt_str ops={ops} mem={mem}");
        let (rc, _) = dt_str(
            "crypto_pwhash_scryptsalsa208sha256_str",
            PW,
            ops,
            mem,
            &ctx,
        );
        assert_eq!(rc, 0, "{ctx}: _str must accept any opslimit/memlimit");
    }
    // row 239: escrypt_r() == NULL -> errno=EINVAL, -1. Reachable after all:
    // opslimit=UINT64_MAX with memlimit=0 makes pickparams pick N=2, r=8,
    // p=0x3fffffff/8 = 134217727, i.e. B_size = 128*r*p = 128 GiB, so the KDF's
    // allocation fails and escrypt_r returns NULL.
    let ctx = "row239 scrypt_str ops=UINT64_MAX mem=0 -> escrypt_r == NULL";
    let (rc, e) = dt_str(
        "crypto_pwhash_scryptsalsa208sha256_str",
        PW,
        u64::MAX,
        0,
        ctx,
    );
    assert_eq!(rc, -1, "{ctx}");
    assert_eq!(e, EINVAL, "{ctx}: expected errno=EINVAL, got {e}");
}

// ===========================================================================
// rows 244-247: scrypt _str_needs_rehash
// ===========================================================================
#[test]
fn g2_err_rows_244_247_scrypt_needs_rehash() {
    let sym = "crypto_pwhash_scryptsalsa208sha256_str_needs_rehash";
    let v = SCRYPT_VEC.as_bytes();
    // row 245: sodium_strnlen(str,102) != 101 -> EINVAL
    let mut cases: Vec<Vec<u8>> = vec![
        vec![],
        b"$7$".to_vec(),
        v[..100].to_vec(),
        {
            let mut x = v.to_vec();
            x.push(b'x');
            x
        },
    ];
    let mut nul = v.to_vec();
    nul[10] = 0;
    cases.push(nul);
    for (i, s) in cases.iter().enumerate() {
        let ctx = format!("row245 needs_rehash bad length #{i} (len {})", s.len());
        let (rc, e) = dt_rehash(sym, s, 32768, 16777216, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
    }
    // row 246: escrypt_parse_setting == NULL -> EINVAL
    let mut bad_prefix = v.to_vec();
    bad_prefix[2] = b'8';
    let mut bad_n = v.to_vec();
    bad_n[3] = b'$';
    let mut bad_r = v.to_vec();
    bad_r[6] = b'*';
    let mut bad_p = v.to_vec();
    bad_p[13] = 0x7f;
    for (label, s) in [
        ("row246 bad prefix", &bad_prefix),
        ("row246 bad N_log2 char", &bad_n),
        ("row246 bad r char", &bad_r),
        ("row246 bad p char", &bad_p),
    ] {
        let (rc, e) = dt_rehash(sym, s, 32768, 16777216, label);
        assert_eq!(rc, -1, "{label}");
        assert_eq!(e, EINVAL, "{label}");
    }
    // row 247: a parameter difference is NOT an error -> returns 1
    let (rc, e) = dt_rehash(sym, v, 32768, 1024, "row247 param mismatch");
    assert_eq!(rc, 1);
    assert_eq!(e, 0, "row247: errno must be untouched");
}

// ===========================================================================
// rows 248-259: crypto_pwhash_scryptsalsa208sha256_ll / escrypt_kdf_nosse
// ===========================================================================
#[test]
fn g2_err_rows_250_259_ll_range_checks() {
    let pw = PW;
    let salt = &SALT16;
    // row 250: buflen > ((2^32)-1)*32 == 137438953440 -> EFBIG
    for buflen in [137438953441usize, usize::MAX, usize::MAX / 2] {
        let ctx = format!("row250 _ll buflen={buflen}");
        let (rc, e) = dt_ll(pw, salt, 2, 1, 1, buflen, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EFBIG, "{ctx}");
    }
    // row 251: r*p >= 2^30 -> EFBIG
    for (r_, p) in [
        (1u32, 1073741824u32),
        (1073741824, 1),
        (32768, 32768),
        (2, 536870912),
        (536870912, 2),
        (65536, 16384),
        (u32::MAX, u32::MAX),
    ] {
        let ctx = format!("row251 _ll r={r_} p={p}");
        let (rc, e) = dt_ll(pw, salt, 2, r_, p, 32, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EFBIG, "{ctx}");
    }
    // one step below the r*p limit is accepted (and cheap for N=2)
    let (rc, _) = dt_ll(pw, salt, 2, 1, 1073741823, 32, "boundary r*p=2^30-1");
    // 128*r*p = ~137 GiB of allocation, so this must fail to allocate rather
    // than be rejected by the r*p check; either way C and Rust must agree.
    assert_eq!(rc, -1, "r*p = 2^30-1 needs 128 GiB; expected an alloc failure");
    // row 252: N > UINT32_MAX -> EFBIG
    for n in [4294967296u64, 1u64 << 33, u64::MAX, u64::MAX - 1] {
        let ctx = format!("row252 _ll N={n}");
        let (rc, e) = dt_ll(pw, salt, n, 1, 1, 32, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EFBIG, "{ctx}");
    }
    // row 253: N not a power of two -> EINVAL
    for n in [3u64, 5, 6, 7, 9, 1000, 1023, 1025, 4294967295] {
        let ctx = format!("row253 _ll N={n}");
        let (rc, e) = dt_ll(pw, salt, n, 1, 1, 32, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
    }
    // row 254: N < 2 -> EINVAL
    for n in [0u64, 1] {
        let ctx = format!("row254 _ll N={n}");
        let (rc, e) = dt_ll(pw, salt, n, 1, 1, 32, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
    }
    // rows 255/256: r == 0 / p == 0 -> EINVAL
    for (r_, p) in [(0u32, 1u32), (1, 0), (0, 0)] {
        let ctx = format!("rows255/256 _ll r={r_} p={p}");
        let (rc, e) = dt_ll(pw, salt, 2, r_, p, 32, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
    }
    // row 257: N > SIZE_MAX/128/r -> ENOMEM (no allocation is attempted).
    // N=2^31, r=2^26: r*p = 2^26 < 2^30, N <= UINT32_MAX, power of two,
    // SIZE_MAX/128/r == 2^31 - 1 < N.
    for (n, r_, p) in [
        (1u64 << 31, 1u32 << 26, 1u32),
        (1 << 31, 1 << 27, 1),
        (1 << 30, 1 << 28, 1),
    ] {
        let ctx = format!("row257 _ll N={n} r={r_} p={p}");
        let (rc, e) = dt_ll(pw, salt, n, r_, p, 32, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, ENOMEM, "{ctx}: expected ENOMEM, got {e}");
    }
    // row 259: escrypt_alloc_region fails. N=2^31, r=2^25 passes every check
    // above (SIZE_MAX/128/r == 2^32-1 >= N) and then needs V_size = 2^63
    // bytes, which mmap cannot possibly satisfy.
    let ctx = "row259 _ll allocation failure (V_size = 2^63)";
    let (rc, _) = dt_ll(pw, salt, 1u64 << 31, 1u32 << 25, 1, 32, ctx);
    assert_eq!(rc, -1, "{ctx}");
    // row 248 (escrypt_init_local != 0) and row 258 (size_t overflow of
    // `need`) are statically unreachable; row 249 (escrypt_free_local
    // failure) needs a munmap failure, exercised directly on
    // escrypt_free_region below.
}

// ===========================================================================
// rows 261-268: escrypt_parse_setting / decode64_one / decode64_uint32
// ===========================================================================
#[test]
fn g2_err_rows_261_268_parse_setting() {
    let v = SCRYPT_VEC.as_bytes();
    // control: the valid setting parses to (10, 8, 1)
    let (ok, n, r_, p) = dt_parse_setting(v, "control valid $7$ setting");
    assert!(ok);
    assert_eq!((n, r_, p), (10, 8, 1), "control setting");
    // row 261: setting[0..3] != "$7$"
    for (i, ch) in [(0usize, b'x'), (1, b'8'), (2, b'x'), (0, 0), (1, 0), (2, 0)] {
        let mut s = v.to_vec();
        s[i] = ch;
        let ctx = format!("row261 setting[{i}] = {ch:#x}");
        let (ok, _, _, _) = dt_parse_setting(&s, &ctx);
        assert!(!ok, "{ctx}: must return NULL");
    }
    for s in [&b""[..], b"$", b"$7", b"$8$", b"7$"] {
        let ctx = format!("row261 short setting {:?}", String::from_utf8_lossy(s));
        let (ok, _, _, _) = dt_parse_setting(s, &ctx);
        assert!(!ok, "{ctx}");
    }
    // NOTE on the NUL quirk: decode64_one uses strchr(itoa64, src), and
    // strchr matches the terminating NUL, so a 0x00 byte "decodes" to 64
    // instead of failing. Both libraries must reproduce that.
    for off in [3usize, 4, 8, 9, 13] {
        let mut s = v.to_vec();
        s[off] = 0;
        let ctx = format!("rows262-266 NUL quirk at offset {off}");
        let (ok, _, _, _) = dt_parse_setting(&s, &ctx);
        assert!(
            ok,
            "{ctx}: strchr() matches the terminating NUL, so this must succeed"
        );
    }
    // rows 262/265: setting[3] (N_log2) outside "./0-9A-Za-z"
    for ch in [b'$', b'-', b'+', 0xffu8, b'!', b'~', b' ', b','] {
        let mut s = v.to_vec();
        s[3] = ch;
        let ctx = format!("rows262/265 N_log2 char {ch:#x}");
        let (ok, n, _, _) = dt_parse_setting(&s, &ctx);
        assert!(!ok, "{ctx}: must return NULL");
        assert_eq!(n, 0, "{ctx}: *N_log2_p must be zeroed");
    }
    // rows 263/266: any of the 5 r characters outside the alphabet
    for off in 4..9usize {
        for ch in [b'$', b'-', b'+', 0xffu8] {
            let mut s = v.to_vec();
            s[off] = ch;
            let ctx = format!("rows263/266 r char at {off} = {ch:#x}");
            let (ok, _, r_, _) = dt_parse_setting(&s, &ctx);
            assert!(!ok, "{ctx}: must return NULL");
            assert_eq!(r_, 0, "{ctx}: *r_p must be zeroed");
        }
    }
    // row 264: any of the 5 p characters outside the alphabet
    for off in 9..14usize {
        for ch in [b'$', b'-', b'+', 0xffu8] {
            let mut s = v.to_vec();
            s[off] = ch;
            let ctx = format!("row264 p char at {off} = {ch:#x}");
            let (ok, _, _, p) = dt_parse_setting(&s, &ctx);
            assert!(!ok, "{ctx}: must return NULL");
            assert_eq!(p, 0, "{ctx}: *p_p must be zeroed");
        }
    }
    // the whole itoa64 alphabet must be accepted for N_log2
    const ITOA64: &[u8] = b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    for (i, ch) in ITOA64.iter().enumerate() {
        let mut s = v.to_vec();
        s[3] = *ch;
        let ctx = format!("row262 N_log2 char '{}' -> {i}", *ch as char);
        let (ok, n, _, _) = dt_parse_setting(&s, &ctx);
        assert!(ok, "{ctx}: must parse");
        assert_eq!(n, i as u32, "{ctx}");
    }
    // rows 267/268 (encode64_uint32 / encode64 running out of destination) are
    // unreachable: escrypt_gensalt_r's `need > buflen` pre-check already
    // guarantees enough room for every subsequent encode.
}

// ===========================================================================
// rows 269-279: escrypt_r / escrypt_gensalt_r
// ===========================================================================
#[test]
fn g2_err_rows_269_279_escrypt_r_and_gensalt() {
    install_det_random();
    let v = SCRYPT_VEC.as_bytes();
    let mut setting = v[..57].to_vec(); // the "$7$" setting, without the hash
    setting.push(0);

    // ---- escrypt_r ----------------------------------------------------
    #[track_caller]
    fn dt_escrypt_r(
        setting: &[u8],
        buflen: usize,
        pass_null_buf: bool,
        ctx: &str,
    ) -> bool {
        install_det_random();
        unsafe {
            let (c, r) = pair::<FnEscryptR>("_sodium_escrypt_r");
            let (ci, ri) = pair::<FnInitLocal>("_sodium_escrypt_init_local");
            let (cf, rf) = pair::<FnInitLocal>("_sodium_escrypt_free_local");
            let mut cl = Region::zeroed();
            let mut rl = Region::zeroed();
            assert_eq!(ci(&mut cl), 0);
            assert_eq!(ri(&mut rl), 0);
            let mut cb = vec![0x99u8; buflen.max(1) + 8];
            let mut rb = vec![0x99u8; buflen.max(1) + 8];
            det_reseed(0x5EED);
            set_errno(0);
            let cres = c(
                &mut cl,
                PW.as_ptr(),
                PW.len(),
                setting.as_ptr(),
                if pass_null_buf {
                    ptr::null_mut()
                } else {
                    cb.as_mut_ptr()
                },
                buflen,
            );
            let ce = errno();
            det_reseed(0x5EED);
            set_errno(0);
            let rres = r(
                &mut rl,
                PW.as_ptr(),
                PW.len(),
                setting.as_ptr(),
                if pass_null_buf {
                    ptr::null_mut()
                } else {
                    rb.as_mut_ptr()
                },
                buflen,
            );
            let re = errno();
            assert_eq!(cf(&mut cl), 0);
            assert_eq!(rf(&mut rl), 0);
            assert_eq!(
                cres.is_null(),
                rres.is_null(),
                "{ctx}: NULL-ness differs (C null={}, Rust null={})",
                cres.is_null(),
                rres.is_null()
            );
            assert_eq!(ce, re, "{ctx}: errno differs (C={ce}, Rust={re})");
            eq_bytes(ctx, &cb, &rb);
            !cres.is_null()
        }
    }
    // control: the valid setting with buflen 102 succeeds
    assert!(dt_escrypt_r(&setting, 102, false, "control escrypt_r"));
    // row 269: escrypt_parse_setting == NULL
    for (i, ch) in [(0usize, b'x'), (3, b'$'), (5, b'-'), (11, b'+')] {
        let mut s = setting.clone();
        s[i] = ch;
        let ctx = format!("row269 escrypt_r bad setting byte {i}");
        assert!(!dt_escrypt_r(&s, 102, false, &ctx), "{ctx}");
    }
    // row 270: buf == NULL
    assert!(
        !dt_escrypt_r(&setting, 102, true, "row270 escrypt_r buf=NULL"),
        "row270"
    );
    // row 271: need = 14 + saltlen + 1 + 43 + 1 > buflen
    for buflen in [0usize, 1, 57, 100, 101] {
        let ctx = format!("row271 escrypt_r buflen={buflen}");
        assert!(!dt_escrypt_r(&setting, buflen, false, &ctx), "{ctx}");
    }
    // row 273: escrypt_kdf fails — N_log2 = 0 gives N = 1 (< 2) -> EINVAL
    let mut n0 = setting.clone();
    n0[3] = b'.';
    assert!(
        !dt_escrypt_r(&n0, 102, false, "row273 escrypt_r N_log2=0 -> N=1"),
        "row273"
    );
    // ... and r = 0 from the setting ("....." decodes to 0)
    let mut r0 = setting.clone();
    r0[4..9].copy_from_slice(b".....");
    assert!(
        !dt_escrypt_r(&r0, 102, false, "row273 escrypt_r r=0"),
        "row273 r=0"
    );
    let mut p0 = setting.clone();
    p0[9..14].copy_from_slice(b".....");
    assert!(
        !dt_escrypt_r(&p0, 102, false, "row273 escrypt_r p=0"),
        "row273 p=0"
    );

    // ---- escrypt_gensalt_r -------------------------------------------
    #[track_caller]
    fn dt_gensalt(
        n_log2: u32,
        r_: u32,
        p: u32,
        srclen: usize,
        buflen: usize,
        ctx: &str,
    ) -> bool {
        let src = vec![0x5au8; core::cmp::min(srclen, 4096)];
        unsafe {
            let (c, r) = pair::<FnGensalt>("_sodium_escrypt_gensalt_r");
            let mut cb = vec![0x88u8; buflen + 8];
            let mut rb = vec![0x88u8; buflen + 8];
            set_errno(0);
            let cres = c(
                n_log2,
                r_,
                p,
                src.as_ptr(),
                srclen,
                cb.as_mut_ptr(),
                buflen,
            );
            let ce = errno();
            set_errno(0);
            let rres = r(
                n_log2,
                r_,
                p,
                src.as_ptr(),
                srclen,
                rb.as_mut_ptr(),
                buflen,
            );
            let re = errno();
            assert_eq!(
                cres.is_null(),
                rres.is_null(),
                "{ctx}: NULL-ness differs"
            );
            assert_eq!(ce, re, "{ctx}: errno differs");
            eq_bytes(ctx, &cb, &rb);
            assert_eq!(&cb[buflen..], &[0x88u8; 8], "{ctx}: wrote past buflen");
            !cres.is_null()
        }
    }
    // control: STRSALTBYTES = 32 -> saltlen 43, need 58
    assert!(dt_gensalt(10, 8, 1, 32, 58, "control gensalt"));
    // row 275: need = 14 + BYTES2CHARS(srclen) + 1 > buflen
    for buflen in [0usize, 1, 14, 43, 57] {
        let ctx = format!("row275 gensalt buflen={buflen}");
        assert!(!dt_gensalt(10, 8, 1, 32, buflen, &ctx), "{ctx}");
    }
    // row 276: saltlen < srclen (BYTES2CHARS wraps for srclen >= 2^61)
    for srclen in [1usize << 61, (1usize << 61) + 1, usize::MAX / 4] {
        let ctx = format!("row276 gensalt srclen={srclen}");
        assert!(!dt_gensalt(10, 8, 1, srclen, 256, &ctx), "{ctx}");
    }
    // row 277: N_log2 > 63
    for n in [64u32, 65, 255, u32::MAX] {
        let ctx = format!("row277 gensalt N_log2={n}");
        assert!(!dt_gensalt(n, 8, 1, 32, 58, &ctx), "{ctx}");
    }
    // N_log2 == 63 is the boundary and is accepted
    assert!(dt_gensalt(63, 8, 1, 32, 58, "row277 gensalt N_log2=63"));
    // row 278: r*p >= 2^30
    for (r_, p) in [
        (1u32, 1073741824u32),
        (1073741824, 1),
        (32768, 32768),
        (u32::MAX, u32::MAX),
        (2, 536870912),
    ] {
        let ctx = format!("row278 gensalt r={r_} p={p}");
        assert!(!dt_gensalt(10, r_, p, 32, 58, &ctx), "{ctx}");
    }
    // r*p == 2^30 - 1 is accepted
    assert!(dt_gensalt(10, 1, 1073741823, 32, 58, "row278 gensalt r*p=2^30-1"));
    // rows 274/279 ("can't happen" branches after the pre-checks) are
    // statically unreachable.
}

// ===========================================================================
// rows 280/281: escrypt_alloc_region / escrypt_free_region
// ===========================================================================
#[test]
fn g2_err_rows_280_281_alloc_free_region() {
    unsafe {
        let (ca, ra) = pair::<FnAllocRegion>("_sodium_escrypt_alloc_region");
        let (cfr, rfr) = pair::<FnInitLocal>("_sodium_escrypt_free_region");
        // row 280: an impossible allocation size -> NULL, base=NULL, size=0.
        // Neither library imports mmap/posix_memalign for this region
        // allocator, so the compiled path is the `malloc(size + 63)` fallback:
        // `size = SIZE_MAX` and `SIZE_MAX - 63` overflow the +63 and set
        // errno = ENOMEM, while 1<<62 simply fails to allocate.
        for size in [usize::MAX, usize::MAX - 63, 1usize << 62, usize::MAX - 62] {
            let mut cregion = Region {
                base: 0x1234 as *mut c_void,
                aligned: 0x5678 as *mut c_void,
                size: 99,
            };
            let mut rregion = cregion;
            set_errno(0);
            let cp = ca(&mut cregion, size);
            let ce = errno();
            set_errno(0);
            let rp = ra(&mut rregion, size);
            let re = errno();
            let ctx = format!("row280 alloc_region size={size}");
            assert_eq!(cp.is_null(), rp.is_null(), "{ctx}: NULL-ness differs");
            assert_eq!(ce, re, "{ctx}: errno differs (C={ce}, Rust={re})");
            assert_eq!(
                (cregion.base.is_null(), cregion.size),
                (rregion.base.is_null(), rregion.size),
                "{ctx}: region fields differ"
            );
            assert!(cp.is_null(), "{ctx}: expected a NULL return from C");
            assert_eq!(cregion.size, 0, "{ctx}: size must be reset to 0");
            assert!(cregion.base.is_null(), "{ctx}: base must be NULL");
        }
        // row 281: `munmap()` failure. That branch is only compiled when
        // MAP_ANON && HAVE_MMAP; this build uses the malloc fallback (neither
        // library imports mmap for the region allocator), so escrypt_free_region
        // can only ever return 0 here — the row is dead in this configuration.
        // What IS testable is that both libraries take the same branch:
        // size == 0 is accepted by malloc(63), giving a non-NULL region.
        {
            let mut cregion = Region::zeroed();
            let mut rregion = Region::zeroed();
            let cp = ca(&mut cregion, 0);
            let rp = ra(&mut rregion, 0);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "row280 alloc_region size=0: NULL-ness differs"
            );
            assert_eq!(cregion.size, rregion.size);
            assert_eq!(cregion.size, 0, "row280: size stays 0");
            assert_eq!(cfr(&mut cregion), 0);
            assert_eq!(rfr(&mut rregion), 0);
        }
        // control: a zeroed region frees cleanly and stays zeroed
        let mut cregion = Region::zeroed();
        let mut rregion = Region::zeroed();
        set_errno(0);
        let crc = cfr(&mut cregion);
        set_errno(0);
        let rrc = rfr(&mut rregion);
        eq_i32("row281 free_region(zeroed)", crc, rrc);
        assert_eq!(crc, 0);
        assert_eq!(cregion, Region::zeroed());
        assert_eq!(rregion, Region::zeroed());
        // control: a real allocation round-trips
        let mut cregion = Region::zeroed();
        let mut rregion = Region::zeroed();
        let cp = ca(&mut cregion, 4096);
        let rp = ra(&mut rregion, 4096);
        assert!(!cp.is_null() && !rp.is_null());
        assert_eq!(cregion.size, 4096);
        assert_eq!(rregion.size, 4096);
        assert_eq!(cfr(&mut cregion), 0);
        assert_eq!(rfr(&mut rregion), 0);
    }
}

// ===========================================================================
// generic FFI-boundary probing beyond the table
// ===========================================================================
#[test]
fn g2_err_generic_boundaries() {
    let salt32: Vec<u8> = (0..32u8).collect();
    // ---- one step past / exactly on every documented public limit --------
    // argon2i: OPSLIMIT_MIN=3, argon2id: OPSLIMIT_MIN=1
    for (sym, alg, opsmin) in [
        ("crypto_pwhash_argon2i", ARGON2I, 3u64),
        ("crypto_pwhash_argon2id", ARGON2ID, 1),
    ] {
        // just below / exactly on the accepted minimum
        let ctx = format!("boundary {sym} opslimit={}", opsmin - 1);
        assert_eq!(
            dt_pwhash(sym, 16, PW, &SALT16, opsmin - 1, 8192, alg, &ctx).0,
            -1
        );
        let ctx = format!("boundary {sym} opslimit={opsmin}");
        assert_eq!(dt_pwhash(sym, 16, PW, &SALT16, opsmin, 8192, alg, &ctx).0, 0);
        // memlimit 8191 / 8192
        let ctx = format!("boundary {sym} memlimit=8191");
        assert_eq!(
            dt_pwhash(sym, 16, PW, &SALT16, opsmin, 8191, alg, &ctx).0,
            -1
        );
        let ctx = format!("boundary {sym} memlimit=8192");
        assert_eq!(dt_pwhash(sym, 16, PW, &SALT16, opsmin, 8192, alg, &ctx).0, 0);
        // outlen 15 / 16
        let ctx = format!("boundary {sym} outlen=15");
        assert_eq!(
            dt_pwhash(sym, 15, PW, &SALT16, opsmin, 8192, alg, &ctx).0,
            -1
        );
        let ctx = format!("boundary {sym} outlen=16");
        assert_eq!(dt_pwhash(sym, 16, PW, &SALT16, opsmin, 8192, alg, &ctx).0, 0);
        // opslimit exactly OPSLIMIT_MAX+1
        let ctx = format!("boundary {sym} opslimit=2^32");
        assert_eq!(
            dt_pwhash(sym, 16, PW, &SALT16, 4294967296, 8192, alg, &ctx).0,
            -1
        );
    }
    // scrypt: BYTES_MIN=16 (and nothing else is range-checked)
    assert_eq!(
        dt_scrypt(15, PW, &salt32, 32768, 16777216, "boundary scrypt outlen=15").0,
        -1
    );
    assert_eq!(
        dt_scrypt(16, PW, &salt32, 32768, 16777216, "boundary scrypt outlen=16").0,
        0
    );
    // ---- zero lengths ----------------------------------------------------
    // passwdlen 0 everywhere
    for (sym, alg, ops) in [
        ("crypto_pwhash_argon2i", ARGON2I, 3u64),
        ("crypto_pwhash_argon2id", ARGON2ID, 1),
    ] {
        let ctx = format!("zero-length passwd {sym}");
        assert_eq!(dt_pwhash(sym, 16, b"", &SALT16, ops, 8192, alg, &ctx).0, 0);
    }
    assert_eq!(
        dt_scrypt(16, b"", &salt32, 32768, 16777216, "zero-length passwd scrypt").0,
        0
    );
    // _ll with buflen 0 and passwdlen/saltlen 0
    assert_eq!(dt_ll(b"", b"", 2, 1, 1, 0, "zero-length _ll").0, 0);
    // ---- the full alg int space for every entry point that takes one ----
    for alg in [
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -3,
        -2,
        -1,
        0,
        3,
        4,
        5,
        100,
        255,
        256,
        65535,
        0x7fff_fffe,
        i32::MAX,
    ] {
        let ctx = format!("alg int space crypto_pwhash alg={alg}");
        let (rc, e) = dt_pwhash("crypto_pwhash", 16, PW, &SALT16, 1, 8192, alg, &ctx);
        assert_eq!(rc, -1, "{ctx}");
        assert_eq!(e, EINVAL, "{ctx}");
        // the variant entry points reject everything but their own algorithm
        for (sym, own, ops) in [
            ("crypto_pwhash_argon2i", ARGON2I, 3u64),
            ("crypto_pwhash_argon2id", ARGON2ID, 1),
        ] {
            if alg == own {
                continue;
            }
            let ctx = format!("alg int space {sym} alg={alg}");
            let (rc, e) = dt_pwhash(sym, 16, PW, &SALT16, ops, 8192, alg, &ctx);
            assert_eq!(rc, -1, "{ctx}");
            assert_eq!(e, EINVAL, "{ctx}");
        }
    }
    // ---- errno hygiene: a successful call must not clobber errno ---------
    unsafe {
        let (c, r) = pair::<FnPwhash>("crypto_pwhash");
        let mut cb = [0u8; 16];
        let mut rb = [0u8; 16];
        set_errno(12345);
        let cr = c(
            cb.as_mut_ptr(),
            16,
            PW.as_ptr() as *const c_char,
            PW.len() as u64,
            SALT16.as_ptr(),
            1,
            8192,
            ARGON2ID,
        );
        let ce = errno();
        set_errno(12345);
        let rr = r(
            rb.as_mut_ptr(),
            16,
            PW.as_ptr() as *const c_char,
            PW.len() as u64,
            SALT16.as_ptr(),
            1,
            8192,
            ARGON2ID,
        );
        let re = errno();
        eq_i32("errno hygiene crypto_pwhash", cr, rr);
        assert_eq!(cr, 0);
        assert_eq!(ce, re, "errno hygiene: C left errno={ce}, Rust left {re}");
    }
}

// ===========================================================================
// ERRORS.md rows 100-281: coverage notes for the rows that are NOT asserted
// above, with the reason. Everything else has an explicit assertion.
//
// Needs a multi-gigabyte buffer (the C code writes `out` BEFORE the check, so
// triggering the check is a genuine 4 GiB / 137 GiB allocation):
//   row 104  crypto_pwhash_argon2i   outlen > BYTES_MAX  (memset(out,0,outlen))
//   row 115  crypto_pwhash_argon2id  outlen > BYTES_MAX  (memset(out,0,outlen))
//   row 156  argon2_hash             hashlen > ARGON2_MAX_OUTLEN
//                                    (randombytes_buf(hash, hashlen) first)
//   row 161  argon2_verify           strlen(encoded) > UINT32_MAX
//   row 230  crypto_pwhash_scrypt*   outlen > BYTES_MAX  (memset(out,0,outlen))
//
// Needs an injected allocation failure (malloc/mmap returning NULL):
//   row 114  crypto_pwhash_argon2i  argon2i_hash_raw() != ARGON2_OK
//   row 124  crypto_pwhash_argon2id argon2id_hash_raw() != ARGON2_OK
//            -> the *callee's* failure returns are covered in full by
//               g2_err_rows_152_165_argon2_hash_and_verify (rows 152/155/157/
//               159), which is the identical call the wrappers make.
//   row 154  argon2_ctx: argon2_initialize() allocation failure
//   row 158  argon2_hash: malloc(hashlen) == NULL
//   row 162  argon2_verify: malloc(ctx.adlen/saltlen/outlen) == NULL
//   rows 191-194, 196, 197  allocate_memory() / argon2_initialize() OOM paths
//            -> row 195 (instance == NULL || context == NULL) IS asserted.
//   row 259  escrypt_alloc_region() == NULL -- asserted via an impossible
//            V_size of 2^63 (see g2_err_rows_250_259_ll_range_checks).
//
// Statically unreachable (dead branches in this configuration):
//   row 111  passwdlen < crypto_pwhash_argon2i_PASSWD_MIN   (MIN == 0)
//   row 138  passwdlen < PASSWD_MIN in argon2i_str_verify   (MIN == 0)
//   row 143  passwdlen < PASSWD_MIN in argon2id_str_verify  (MIN == 0)
//   row 169  validate_inputs outlen  > ARGON2_MAX_OUTLEN    (uint32_t field)
//   rows 171/172  pwdlen  < MIN_PWD_LENGTH / > MAX_PWD_LENGTH (0 / uint32_t)
//   row 175  saltlen > ARGON2_MAX_SALT_LENGTH               (uint32_t field)
//   rows 177/178  secretlen < MIN_SECRET / > MAX_SECRET     (0 / uint32_t)
//   rows 180/181  adlen  < MIN_AD_LENGTH / > MAX_AD_LENGTH  (0 / uint32_t)
//   row 185  m_cost > ARGON2_MAX_MEMORY == 0xFFFFFFFF on 64-bit
//   row 188  t_cost > ARGON2_MAX_TIME  == 0xFFFFFFFF        (uint32_t field)
//            -> for all of the above the *accepted* boundary value is asserted
//               in g2_err_rows_166_190_validate_inputs.
//   rows 208/211/214  the `> UINT32_MAX` re-checks after DECIMAL_U32
//   row 229  passwdlen > PASSWD_MAX == SODIUM_SIZE_MAX
//   row 232  pickparams() != 0 (pickparams always returns 0)
//   rows 235/236  _str passwdlen > SODIUM_SIZE_MAX / < PASSWD_MIN
//   row 237  _str escrypt_gensalt_r() == NULL (pickparams caps N_log2 at 63
//            and r*p at 0x3fffffff, so neither gensalt reject can fire)
//            -> both gensalt rejects ARE asserted directly (rows 275-278).
//   rows 238/241/248  escrypt_init_local() != 0 (statically returns 0)
//   row 244  _str_needs_rehash pickparams() != 0
//   row 258  size_t overflow of `need` in escrypt_kdf (bounded by row 257)
//   row 272  escrypt_r `need < saltlen` (saltlen is bounded by strlen)
//   rows 267/268/274/279  encode64/encode64_uint32 running out of destination:
//            every caller pre-checks `need > buflen`, so these "can't happen"
//            branches are unreachable.
//   rows 249/281  escrypt_free_local()/escrypt_free_region() munmap failure:
//            neither library imports mmap for the region allocator (the
//            `malloc(size + 63)` fallback is compiled), so the munmap branch
//            does not exist in this build.
