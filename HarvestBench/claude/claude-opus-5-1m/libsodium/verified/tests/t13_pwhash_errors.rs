//! Phase C — `crypto_pwhash/` error paths (`ERRORS.md` rows `G2-001` …
//! `G2-223`).
//!
//! Three kinds of row:
//!
//! * **`return -1` / error-enum rows** — called directly on both `.so`s. The
//!   return value, `errno` (set to a sentinel first, so "errno untouched" is
//!   observable) and the *entire* output buffer are compared byte for byte.
//! * **`sodium_misuse()` rows** — `crypto_pwhash_str_alg` with an unhandled
//!   `alg`, and `escrypt_PBKDF2_SHA256` with `dkLen > 0x1fffffffe0`. Both run in
//!   a **child process** (once per library) with the observing handler
//!   installed, so the abort *and* the state written before it are compared.
//! * **dead / not-constructible rows** — enumerated in
//!   `documented_dead_and_unreachable_rows` with the reason, so no row is
//!   silently dropped.

mod common;
use common::*;

use std::ffi::{c_char, c_void};

// ===========================================================================
// C signatures (identical to t12_pwhash.rs)
// ===========================================================================

type FnSz = unsafe extern "C" fn() -> usize;

type Pwhash =
    unsafe extern "C" fn(*mut u8, u64, *const c_char, u64, *const u8, u64, usize, i32) -> i32;
type PwStr = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize) -> i32;
type PwStrAlg = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize, i32) -> i32;
type PwStrVerify = unsafe extern "C" fn(*const c_char, *const c_char, u64) -> i32;
type PwNeedsRehash = unsafe extern "C" fn(*const c_char, u64, usize) -> i32;

type Scrypt = unsafe extern "C" fn(*mut u8, u64, *const c_char, u64, *const u8, u64, usize) -> i32;
type ScryptLl =
    unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, u32, u32, *mut u8, usize) -> i32;

type Argon2Hash = unsafe extern "C" fn(
    u32,
    u32,
    u32,
    *const c_void,
    usize,
    *const c_void,
    usize,
    *mut c_void,
    usize,
    *mut c_char,
    usize,
    i32,
) -> i32;
type Argon2HashRaw = unsafe extern "C" fn(
    u32,
    u32,
    u32,
    *const c_void,
    usize,
    *const c_void,
    usize,
    *mut c_void,
    usize,
) -> i32;
type Argon2HashEnc = unsafe extern "C" fn(
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
) -> i32;
type Argon2Verify = unsafe extern "C" fn(*const c_char, *const c_void, usize, i32) -> i32;
type Argon2VerifyT = unsafe extern "C" fn(*const c_char, *const c_void, usize) -> i32;
type Argon2Ctx = unsafe extern "C" fn(*mut Ctx, i32) -> i32;
type Argon2Validate = unsafe extern "C" fn(*const Ctx) -> i32;
type Argon2Decode = unsafe extern "C" fn(*mut Ctx, *const c_char, i32) -> i32;
type Argon2Encode = unsafe extern "C" fn(*mut c_char, usize, *mut Ctx, i32) -> i32;
type Argon2Initialize = unsafe extern "C" fn(*mut Instance, *mut Ctx) -> i32;
type Argon2FillMemory = unsafe extern "C" fn(*mut Instance, u32);
type Argon2FillSegment = unsafe extern "C" fn(*const Instance, Position);
type Argon2Finalize = unsafe extern "C" fn(*const Ctx, *mut Instance);
type Blake2bLong = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> i32;

type EGensalt = unsafe extern "C" fn(u32, u32, u32, *const u8, usize, *mut u8, usize) -> *mut u8;
type EParse = unsafe extern "C" fn(*const u8, *mut u32, *mut u32, *mut u32) -> *const u8;
type ER = unsafe extern "C" fn(*mut Region, *const u8, usize, *const u8, *mut u8, usize) -> *mut u8;
type EKdf = unsafe extern "C" fn(
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
) -> i32;
type ERegion1 = unsafe extern "C" fn(*mut Region) -> i32;
type EAlloc = unsafe extern "C" fn(*mut Region, usize) -> *mut c_void;
type EPbkdf2 = unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, *mut u8, usize);

#[repr(C)]
#[derive(Clone, Copy)]
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
            out: std::ptr::null_mut(),
            outlen: 0,
            pwd: std::ptr::null_mut(),
            pwdlen: 0,
            salt: std::ptr::null_mut(),
            saltlen: 0,
            secret: std::ptr::null_mut(),
            secretlen: 0,
            ad: std::ptr::null_mut(),
            adlen: 0,
            t_cost: 0,
            m_cost: 0,
            lanes: 0,
            threads: 0,
            flags: 0,
        }
    }
}

/// `argon2-core.h`: `argon2_instance_t` (`sizeof == 56`).
#[repr(C)]
#[derive(Clone, Copy)]
struct Instance {
    region: *mut c_void,
    pseudo_rands: *mut u64,
    passes: u32,
    current_pass: u32,
    memory_blocks: u32,
    segment_length: u32,
    lane_length: u32,
    lanes: u32,
    threads: u32,
    type_: i32,
    print_internals: i32,
}

impl Instance {
    fn zeroed() -> Instance {
        Instance {
            region: std::ptr::null_mut(),
            pseudo_rands: std::ptr::null_mut(),
            passes: 0,
            current_pass: 0,
            memory_blocks: 0,
            segment_length: 0,
            lane_length: 0,
            lanes: 0,
            threads: 0,
            type_: 0,
            print_internals: 0,
        }
    }
}

/// `argon2-core.h`: `argon2_position_t` (`sizeof == 16`), passed **by value**.
#[repr(C)]
#[derive(Clone, Copy)]
struct Position {
    pass: u32,
    lane: u32,
    slice: u8,
    index: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Region {
    base: *mut c_void,
    aligned: *mut c_void,
    size: usize,
}

impl Region {
    fn zeroed() -> Region {
        Region {
            base: std::ptr::null_mut(),
            aligned: std::ptr::null_mut(),
            size: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// constants
// ---------------------------------------------------------------------------

const EPERM: i32 = 1;
const ENOMEM: i32 = 12;
const EINVAL: i32 = 22;
const EFBIG: i32 = 27;

const ARGON2_I: i32 = 1;
const ARGON2_ID: i32 = 2;

const ARGON2_OK: i32 = 0;
const ARGON2_OUTPUT_PTR_NULL: i32 = -1;
const ARGON2_OUTPUT_TOO_SHORT: i32 = -2;
const ARGON2_OUTPUT_TOO_LONG: i32 = -3;
const ARGON2_PWD_TOO_LONG: i32 = -5;
const ARGON2_SALT_TOO_SHORT: i32 = -6;
const ARGON2_SALT_TOO_LONG: i32 = -7;
const ARGON2_TIME_TOO_SMALL: i32 = -12;
const ARGON2_MEMORY_TOO_LITTLE: i32 = -14;
const ARGON2_LANES_TOO_FEW: i32 = -16;
const ARGON2_LANES_TOO_MANY: i32 = -17;
const ARGON2_PWD_PTR_MISMATCH: i32 = -18;
const ARGON2_SALT_PTR_MISMATCH: i32 = -19;
const ARGON2_SECRET_PTR_MISMATCH: i32 = -20;
const ARGON2_AD_PTR_MISMATCH: i32 = -21;
const ARGON2_MEMORY_ALLOCATION_ERROR: i32 = -22;
const ARGON2_INCORRECT_PARAMETER: i32 = -25;
const ARGON2_INCORRECT_TYPE: i32 = -26;
const ARGON2_THREADS_TOO_FEW: i32 = -28;
const ARGON2_THREADS_TOO_MANY: i32 = -29;
const ARGON2_ENCODING_FAIL: i32 = -31;
const ARGON2_DECODING_FAIL: i32 = -32;
const ARGON2_VERIFY_MISMATCH: i32 = -35;

/// `crypto_pwhash_argon2id_MEMLIMIT_MAX` on x86-64: `m_cost` becomes
/// 4294967295, i.e. a 4 TB `malloc` that always fails — the cheapest way to
/// reach every `ARGON2_MEMORY_ALLOCATION_ERROR` path.
const MEMLIMIT_MAX: usize = 4398046510080;

// ---------------------------------------------------------------------------
// errno / RNG helpers
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn __errno_location() -> *mut i32;
}

/// Distinctive value written to `errno` before every call, so that "errno left
/// unchanged" rows are directly observable.
const SENTINEL: i32 = 0x5A5A;

fn set_errno(v: i32) {
    unsafe { *__errno_location() = v }
}

fn get_errno() -> i32 {
    unsafe { *__errno_location() }
}

/// `reset_rngs()` touches process-global state, and `cargo test` runs the
/// `#[test]`s of one binary in parallel threads: serialise every
/// `randombytes_buf`-consuming pair.
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn rng_lock() -> std::sync::MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

unsafe fn cstr(p: *const c_char) -> Vec<u8> {
    let mut v = Vec::new();
    let mut i = 0usize;
    loop {
        let b = unsafe { *p.add(i) } as u8;
        if b == 0 {
            break;
        }
        v.push(b);
        i += 1;
        assert!(i < 8192, "unterminated C string");
    }
    v
}

fn nul(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.push(0);
    v
}

/// One observation: return value, `errno` afterwards, output buffer.
#[derive(Debug)]
struct Obs {
    rc: i32,
    errno: i32,
    buf: Vec<u8>,
}

#[track_caller]
fn eq_obs(what: &str, c: &Obs, r: &Obs) {
    eq_i32(&format!("{what} rc"), c.rc, r.rc);
    assert_eq!(
        c.errno, r.errno,
        "{what}: errno mismatch (C={}, Rust={})",
        c.errno, r.errno
    );
    eq_bytes(&format!("{what} buffer"), &c.buf, &r.buf);
}

// ===========================================================================
// crypto_pwhash.c
// ===========================================================================

/// ERRORS G2-001, G2-002, G2-003, G2-004 — `crypto_pwhash` with an `alg` that
/// no `case` matches: `errno = EINVAL`, returns `-1`, and (unlike the two
/// per-algorithm entry points) the output buffer is **not** touched, because
/// the `switch` precedes every `memset`.
#[test]
fn crypto_pwhash_bad_alg() {
    setup();
    let pw = b"password\0";
    let salt = [3u8; 16];
    let (cf, rf) = pair::<Pwhash>("crypto_pwhash");
    for alg in [0i32, 3, 4, 5, 100, -1, -2, i32::MAX, i32::MIN] {
        let mut obs = Vec::new();
        for f in [cf, rf] {
            let mut out = canary(48);
            set_errno(SENTINEL);
            let rc = unsafe {
                f(
                    out.as_mut_ptr(),
                    32,
                    pw.as_ptr() as *const c_char,
                    8,
                    salt.as_ptr(),
                    3,
                    8192,
                    alg,
                )
            };
            obs.push(Obs {
                rc,
                errno: get_errno(),
                buf: out,
            });
        }
        eq_obs(&format!("crypto_pwhash(alg={alg})"), &obs[0], &obs[1]);
        assert_eq!(obs[0].rc, -1);
        assert_eq!(obs[0].errno, EINVAL);
        assert_eq!(obs[0].buf, canary(48), "the alg switch precedes the memset");
    }
}

/// ERRORS G2-011, G2-012, G2-013 — `crypto_pwhash_str_verify` whose `str`
/// matches neither `"$argon2id$"` nor `"$argon2i$"`: `errno = EINVAL`,
/// returns `-1`.
#[test]
fn crypto_pwhash_str_verify_bad_prefix() {
    setup();
    let pw = b"password\0";
    let (cf, rf) = pair::<PwStrVerify>("crypto_pwhash_str_verify");
    let cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),                                              // G2-011
        b"hello".to_vec(),                                         // G2-011
        b"$".to_vec(),
        b"$argon2d$v=19$m=8,t=1,p=1$YWJjZGVmZ2g$YWJjZGVmZ2hpamtsbW5vcA".to_vec(), // G2-011
        b"$argon2i".to_vec(),                                      // G2-012
        b"$argon2id".to_vec(),                                      // G2-012 sibling
        b"$argon2".to_vec(),
        b"$7$C6..../....WZaPV7LSUEKMo34.$kUcEDGyHnjEQb0FGRZR6BdlXbLh0nWNfMWjr7Xl3.g0".to_vec(), // G2-013
        b"argon2id$v=19$".to_vec(),
        b"\x00".to_vec(),
        b"$ARGON2ID$v=19$".to_vec(),
    ];
    for s in &cases {
        let sn = nul(s);
        let mut obs = Vec::new();
        for f in [cf, rf] {
            let _g = rng_lock();
            set_errno(SENTINEL);
            let rc = unsafe { f(sn.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, 8) };
            obs.push(Obs {
                rc,
                errno: get_errno(),
                buf: Vec::new(),
            });
        }
        let tag = format!("str_verify({:?})", String::from_utf8_lossy(s));
        eq_obs(&tag, &obs[0], &obs[1]);
        assert_eq!(obs[0].rc, -1, "{tag}");
        assert_eq!(obs[0].errno, EINVAL, "{tag}");
    }
}

/// ERRORS G2-014 — `crypto_pwhash_str_needs_rehash` with a `str` that matches
/// neither prefix: `errno = EINVAL`, returns `-1`.
#[test]
fn crypto_pwhash_str_needs_rehash_bad_prefix() {
    setup();
    let (cf, rf) = pair::<PwNeedsRehash>("crypto_pwhash_str_needs_rehash");
    for s in [
        &b""[..],
        &b"$7$C6..../....WZaPV7LSUEKMo34."[..],
        &b"$argon2d$v=19$m=8,t=1,p=1$x$y"[..],
        &b"$argon2i"[..],
        &b"$argon2id"[..],
        &b"nonsense"[..],
    ] {
        let sn = nul(s);
        let mut obs = Vec::new();
        for f in [cf, rf] {
            set_errno(SENTINEL);
            let rc = unsafe { f(sn.as_ptr() as *const c_char, 1, 8192) };
            obs.push(Obs {
                rc,
                errno: get_errno(),
                buf: Vec::new(),
            });
        }
        let tag = format!("needs_rehash({:?})", String::from_utf8_lossy(s));
        eq_obs(&tag, &obs[0], &obs[1]);
        assert_eq!(obs[0].rc, -1, "{tag}");
        assert_eq!(obs[0].errno, EINVAL, "{tag}");
    }
}

// ===========================================================================
// pwhash_argon2i.c / pwhash_argon2id.c — the per-algorithm wrappers
// ===========================================================================

/// Drive `crypto_pwhash_argon2i{,d}` / `crypto_pwhash` and compare
/// rc + errno + the whole output buffer.
#[allow(clippy::too_many_arguments)]
fn obs_pw(
    name: &str,
    outbuf: usize,
    outlen: u64,
    pwlen: u64,
    ops: u64,
    mem: usize,
    alg: i32,
    alias: bool,
) -> (Obs, Obs) {
    let (cf, rf) = pair::<Pwhash>(name);
    let salt = [3u8; 16];
    let mut res = Vec::new();
    for f in [cf, rf] {
        let mut out = canary(outbuf);
        let mut pw = vec![0x61u8; 128];
        set_errno(SENTINEL);
        let pwp = if alias {
            out.as_mut_ptr() as *const c_char
        } else {
            pw.as_mut_ptr() as *const c_char
        };
        let rc = unsafe {
            f(
                out.as_mut_ptr(),
                outlen,
                pwp,
                pwlen,
                salt.as_ptr(),
                ops,
                mem,
                alg,
            )
        };
        res.push(Obs {
            rc,
            errno: get_errno(),
            buf: out,
        });
    }
    let b = res.remove(1);
    (res.remove(0), b)
}

/// Every range rejection of `crypto_pwhash_argon2i` and
/// `crypto_pwhash_argon2id`, plus the `crypto_pwhash` delegation.
///
/// ERRORS G2-005, G2-016, G2-017, G2-018, G2-019, G2-020, G2-021, G2-023,
/// G2-024, G2-025 (argon2i) and G2-039, G2-040, G2-041, G2-042, G2-043,
/// G2-044, G2-045, G2-046, G2-047 (argon2id).
///
/// G2-015 / G2-038 (`outlen > BYTES_MAX`) are **not constructible**: the
/// unconditional leading `memset(out, 0, outlen)` runs *before* the check, so
/// triggering it needs a real 4 GiB writable buffer. See
/// `documented_dead_and_unreachable_rows`.
#[test]
fn argon2_wrapper_range_rejections() {
    setup();
    for (name, alg, ops_min, other_alg) in [
        ("crypto_pwhash_argon2i", ARGON2_I, 3u64, ARGON2_ID),
        ("crypto_pwhash_argon2id", ARGON2_ID, 1u64, ARGON2_I),
    ] {
        // --- outlen < BYTES_MIN -> EINVAL, and out[0..outlen] zeroed ------
        for outlen in [0u64, 1, 2, 15] {
            let (c, r) = obs_pw(name, 32, outlen, 8, ops_min, 8192, alg, false);
            let tag = format!("{name}(outlen={outlen})");
            eq_obs(&tag, &c, &r);
            assert_eq!(c.rc, -1, "{tag}");
            assert_eq!(c.errno, EINVAL, "{tag}");
            let mut want = canary(32);
            for x in want.iter_mut().take(outlen as usize) {
                *x = 0;
            }
            assert_eq!(c.buf, want, "{tag}: memset(out, 0, outlen) must have run");
        }

        // --- passwdlen / opslimit / memlimit above MAX -> EFBIG -----------
        for (pwlen, ops, mem, what) in [
            (4294967296u64, ops_min, 8192usize, "passwdlen=2^32"),
            (4294967297, ops_min, 8192, "passwdlen=2^32+1"),
            (u64::MAX, ops_min, 8192, "passwdlen=u64::MAX"),
            (8, 4294967296, 8192, "opslimit=2^32"),
            (8, u64::MAX, 8192, "opslimit=u64::MAX"),
            (8, ops_min, MEMLIMIT_MAX + 1, "memlimit=MAX+1"),
            (8, ops_min, usize::MAX, "memlimit=usize::MAX"),
            (4294967296, 4294967296, usize::MAX, "all three"),
        ] {
            let (c, r) = obs_pw(name, 40, 32, pwlen, ops, mem, alg, false);
            let tag = format!("{name}({what})");
            eq_obs(&tag, &c, &r);
            assert_eq!(c.rc, -1, "{tag}");
            assert_eq!(c.errno, EFBIG, "{tag}");
            let mut want = canary(40);
            for x in want.iter_mut().take(32) {
                *x = 0;
            }
            assert_eq!(c.buf, want, "{tag}");
        }

        // --- opslimit / memlimit below MIN -> EINVAL ----------------------
        let mut below: Vec<(u64, usize, String)> = Vec::new();
        for ops in 0..ops_min {
            below.push((ops, 8192, format!("opslimit={ops}")));
        }
        for mem in [0usize, 1, 1023, 1024, 8191] {
            below.push((ops_min, mem, format!("memlimit={mem}")));
        }
        below.push((0, 0, "both zero".into()));
        for (ops, mem, what) in &below {
            let (c, r) = obs_pw(name, 40, 32, 8, *ops, *mem, alg, false);
            let tag = format!("{name}({what})");
            eq_obs(&tag, &c, &r);
            assert_eq!(c.rc, -1, "{tag}");
            assert_eq!(c.errno, EINVAL, "{tag}");
        }

        // --- G2-023 / G2-045: aliasing out == passwd ----------------------
        let (c, r) = obs_pw(name, 32, 32, 8, ops_min, 8192, alg, true);
        let tag = format!("{name}(out == passwd)");
        eq_obs(&tag, &c, &r);
        assert_eq!(c.rc, -1, "{tag}");
        assert_eq!(c.errno, EINVAL, "{tag}");
        assert_eq!(c.buf, vec![0u8; 32], "{tag}: out is memset first");

        // --- G2-024 / G2-046: unhandled `alg` in the per-algorithm switch --
        for bad in [other_alg, 0, 3, 4, -1, i32::MAX, i32::MIN] {
            let (c, r) = obs_pw(name, 32, 32, 8, ops_min, 8192, bad, false);
            let tag = format!("{name}(alg={bad})");
            eq_obs(&tag, &c, &r);
            assert_eq!(c.rc, -1, "{tag}");
            assert_eq!(c.errno, EINVAL, "{tag}");
        }

        // --- G2-025 / G2-047: the inner argon2*_hash_raw fails ------------
        // memlimit == MEMLIMIT_MAX is *accepted* by the range checks and gives
        // m_cost = 4294967295, i.e. a 4 TB block region that cannot be
        // allocated. The wrapper maps the ARGON2_* code to -1.
        {
            let _g = rng_lock();
            let mut res = Vec::new();
            let (cf, rf) = pair::<Pwhash>(name);
            let salt = [3u8; 16];
            for f in [cf, rf] {
                let mut out = canary(32);
                let pw = [0x61u8; 8];
                set_errno(SENTINEL);
                reset_rngs(0x9100);
                let rc = unsafe {
                    f(
                        out.as_mut_ptr(),
                        32,
                        pw.as_ptr() as *const c_char,
                        8,
                        salt.as_ptr(),
                        ops_min,
                        MEMLIMIT_MAX,
                        alg,
                    )
                };
                res.push(Obs {
                    rc,
                    errno: get_errno(),
                    buf: out,
                });
            }
            let tag = format!("{name}(memlimit=MEMLIMIT_MAX -> alloc failure)");
            eq_obs(&tag, &res[0], &res[1]);
            assert_eq!(res[0].rc, -1, "{tag}");
            // NOTE: `out` is NOT left zeroed. The leading `memset(out, 0,
            // outlen)` runs, but `argon2_hash` then does
            // `randombytes_buf(hash, hashlen)` straight into the caller's
            // buffer, and the allocation failure happens afterwards — so the
            // caller is handed 32 random bytes with a -1 return. Both
            // libraries must produce the *same* bytes (checked by eq_obs).
            assert_ne!(res[0].buf, vec![0u8; 32], "{tag}: randombytes pre-fill");
            assert_ne!(res[0].buf, canary(32), "{tag}");
        }
    }

    // --- G2-005: crypto_pwhash(alg = ARGON2I13) with opslimit 1 or 2 ------
    for ops in [0u64, 1, 2] {
        let (c, r) = obs_pw("crypto_pwhash", 32, 32, 8, ops, 8192, ARGON2_I, false);
        let tag = format!("crypto_pwhash(alg=1, opslimit={ops})");
        eq_obs(&tag, &c, &r);
        assert_eq!(c.rc, -1, "{tag}");
        assert_eq!(c.errno, EINVAL, "{tag}");
    }
    // …while argon2id accepts opslimit = 1 and 2 (asymmetry proof)
    for ops in [1u64, 2] {
        let (c, r) = obs_pw("crypto_pwhash", 40, 32, 8, ops, 8192, ARGON2_ID, false);
        eq_obs(&format!("crypto_pwhash(alg=2, opslimit={ops})"), &c, &r);
        assert_eq!(c.rc, 0, "argon2id must accept opslimit = {ops}");
    }
    // …and argon2id rejects only opslimit = 0
    let (c, r) = obs_pw("crypto_pwhash", 32, 32, 8, 0, 8192, ARGON2_ID, false);
    eq_obs("crypto_pwhash(alg=2, opslimit=0)", &c, &r);
    assert_eq!((c.rc, c.errno), (-1, EINVAL));
}

/// ERRORS G2-026, G2-027, G2-028, G2-029, G2-030, G2-031 (argon2i) and
/// G2-048, G2-049, G2-050, G2-051, G2-052, G2-053 (argon2id) —
/// `crypto_pwhash_*_str` rejections. The full 128-byte buffer is compared;
/// the leading `memset(out, 0, STRBYTES)` means it must come back all-zero.
#[test]
fn argon2_str_rejections() {
    setup();
    for (name, ops_min) in [
        ("crypto_pwhash_argon2i_str", 3u64),
        ("crypto_pwhash_argon2id_str", 1u64),
        ("crypto_pwhash_str", 1u64), // delegates to argon2id
    ] {
        let (cf, rf) = pair::<PwStr>(name);
        let run = |pwlen: u64, ops: u64, mem: usize, seed: u64| -> (Obs, Obs) {
            let _g = rng_lock();
            let mut res = Vec::new();
            for f in [cf, rf] {
                let mut out = canary(128);
                let pw = [0x61u8; 64];
                set_errno(SENTINEL);
                reset_rngs(seed);
                let rc = unsafe {
                    f(
                        out.as_mut_ptr() as *mut c_char,
                        pw.as_ptr() as *const c_char,
                        pwlen,
                        ops,
                        mem,
                    )
                };
                res.push(Obs {
                    rc,
                    errno: get_errno(),
                    buf: out,
                });
            }
            let b = res.remove(1);
            (res.remove(0), b)
        };

        // EFBIG rows
        for (pwlen, ops, mem, what) in [
            (4294967296u64, ops_min, 8192usize, "passwdlen=2^32"),
            (u64::MAX, ops_min, 8192, "passwdlen=u64::MAX"),
            (8, 4294967296, 8192, "opslimit=2^32"),
            (8, u64::MAX, 8192, "opslimit=u64::MAX"),
            (8, ops_min, MEMLIMIT_MAX + 1, "memlimit=MAX+1"),
            (8, ops_min, usize::MAX, "memlimit=usize::MAX"),
        ] {
            let (c, r) = run(pwlen, ops, mem, 0x9200);
            let tag = format!("{name}({what})");
            eq_obs(&tag, &c, &r);
            assert_eq!(c.rc, -1, "{tag}");
            assert_eq!(c.errno, EFBIG, "{tag}");
            assert_eq!(c.buf, vec![0u8; 128], "{tag}: out must be fully zeroed");
        }

        // EINVAL rows
        let mut below: Vec<(u64, usize, String)> = Vec::new();
        for ops in 0..ops_min {
            below.push((ops, 8192, format!("opslimit={ops}")));
        }
        for mem in [0usize, 1, 1024, 8191] {
            below.push((ops_min, mem, format!("memlimit={mem}")));
        }
        for (ops, mem, what) in &below {
            let (c, r) = run(8, *ops, *mem, 0x9201);
            let tag = format!("{name}({what})");
            eq_obs(&tag, &c, &r);
            assert_eq!(c.rc, -1, "{tag}");
            assert_eq!(c.errno, EINVAL, "{tag}");
            assert_eq!(c.buf, vec![0u8; 128], "{tag}");
        }

        // G2-031 / G2-053: the inner argon2*_hash_encoded fails
        // (memlimit = MEMLIMIT_MAX -> 4 TB block region).
        let (c, r) = run(8, ops_min, MEMLIMIT_MAX, 0x9202);
        let tag = format!("{name}(memlimit=MEMLIMIT_MAX -> alloc failure)");
        eq_obs(&tag, &c, &r);
        assert_eq!(c.rc, -1, "{tag}");
        assert_eq!(c.buf, vec![0u8; 128], "{tag}");
    }
}

/// ERRORS G2-032, G2-034, G2-035, G2-036 (argon2i) and G2-054, G2-056,
/// G2-057, G2-058 (argon2id) — `crypto_pwhash_*_str_verify` rejections:
/// `passwdlen > PASSWD_MAX` (`EFBIG`), the wrong password
/// (`ARGON2_VERIFY_MISMATCH` -> `errno = EINVAL`), the *other* variant's
/// string (`ARGON2_DECODING_FAIL`, `errno` untouched) and assorted junk.
#[test]
fn argon2_str_verify_rejections() {
    setup();
    // two real, self-consistent strings, one per variant
    let good_i = build_encoded(ARGON2_I, 8, 3, 1, 16, 32, b"password\0", 8);
    let good_id = build_encoded(ARGON2_ID, 8, 1, 1, 16, 32, b"password\0", 8);

    for (name, mine, theirs) in [
        ("crypto_pwhash_argon2i_str_verify", &good_i, &good_id),
        ("crypto_pwhash_argon2id_str_verify", &good_id, &good_i),
    ] {
        let (cf, rf) = pair::<PwStrVerify>(name);
        let run = |s: &[u8], pw: &[u8], pwlen: u64| -> (Obs, Obs) {
            let _g = rng_lock();
            let mut res = Vec::new();
            for f in [cf, rf] {
                set_errno(SENTINEL);
                reset_rngs(0x9300);
                let rc =
                    unsafe { f(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pwlen) };
                res.push(Obs {
                    rc,
                    errno: get_errno(),
                    buf: Vec::new(),
                });
            }
            let b = res.remove(1);
            (res.remove(0), b)
        };

        // G2-032 / G2-054: passwdlen > PASSWD_MAX -> EFBIG (str not read)
        for pwlen in [4294967296u64, 4294967297, u64::MAX] {
            let (c, r) = run(mine, b"password\0", pwlen);
            let tag = format!("{name}(passwdlen={pwlen})");
            eq_obs(&tag, &c, &r);
            assert_eq!(c.rc, -1, "{tag}");
            assert_eq!(c.errno, EFBIG, "{tag}");
        }

        // the correct password must still verify (control)
        let (c, r) = run(mine, b"password\0", 8);
        eq_obs(&format!("{name}(control)"), &c, &r);
        assert_eq!(c.rc, 0, "{name} control");

        // G2-034 / G2-056: wrong password -> VERIFY_MISMATCH -> errno = EINVAL
        for (pw, pwlen) in [
            (&b"passworD\0"[..], 8u64),
            (&b"password\0"[..], 7),
            (&b"password!\0"[..], 9),
            (&b"\0"[..], 0),
        ] {
            let (c, r) = run(mine, pw, pwlen);
            let tag = format!("{name}(wrong pw {:?})", String::from_utf8_lossy(pw));
            eq_obs(&tag, &c, &r);
            assert_eq!(c.rc, -1, "{tag}");
            assert_eq!(c.errno, EINVAL, "{tag}: VERIFY_MISMATCH maps to EINVAL");
        }

        // G2-035 / G2-057: the other variant's string -> DECODING_FAIL,
        // errno untouched
        let (c, r) = run(theirs, b"password\0", 8);
        let tag = format!("{name}(other variant)");
        eq_obs(&tag, &c, &r);
        assert_eq!(c.rc, -1, "{tag}");
        assert_eq!(
            c.errno, SENTINEL,
            "{tag}: a decoding failure must leave errno alone"
        );

        // G2-036 / G2-058: junk strings
        for s in [
            &b""[..],
            &b"x"[..],
            &b"$"[..],
            &b"$argon2"[..],
            &b"$argon2i"[..],
            &b"$argon2id"[..],
            &b"$argon2id$"[..],
            &b"$argon2i$"[..],
            &b"$argon2id$v=19"[..],
            &b"$argon2id$v=19$m=8,t=1,p=1$"[..],
            &b"$7$C6..../....WZaPV7LSUEKMo34."[..],
        ] {
            let sn = nul(s);
            let (c, r) = run(&sn, b"password\0", 8);
            let tag = format!("{name}({:?})", String::from_utf8_lossy(s));
            eq_obs(&tag, &c, &r);
            assert_eq!(c.rc, -1, "{tag}");
        }
    }
}

/// ERRORS G2-059, G2-060, G2-061, G2-062, G2-063, G2-064, G2-065 — the static
/// `_needs_rehash` helper, reached through all three exported entry points.
#[test]
fn needs_rehash_rejections() {
    setup();
    let good_i = build_encoded(ARGON2_I, 8, 3, 1, 16, 32, b"password\0", 8);
    let good_id = build_encoded(ARGON2_ID, 8, 1, 1, 16, 32, b"password\0", 8);

    let run = |name: &str, s: &[u8], ops: u64, mem: usize| -> (Obs, Obs) {
        let (cf, rf) = pair::<PwNeedsRehash>(name);
        let mut res = Vec::new();
        for f in [cf, rf] {
            set_errno(SENTINEL);
            let rc = unsafe { f(s.as_ptr() as *const c_char, ops, mem) };
            res.push(Obs {
                rc,
                errno: get_errno(),
                buf: Vec::new(),
            });
        }
        let b = res.remove(1);
        (res.remove(0), b)
    };

    for (name, good) in [
        ("crypto_pwhash_argon2i_str_needs_rehash", &good_i),
        ("crypto_pwhash_argon2id_str_needs_rehash", &good_id),
        ("crypto_pwhash_str_needs_rehash", &good_id),
    ] {
        // G2-059: opslimit > UINT32_MAX
        for ops in [4294967296u64, 4294967297, u64::MAX] {
            let (c, r) = run(name, good, ops, 8192);
            let tag = format!("{name}(opslimit={ops})");
            eq_obs(&tag, &c, &r);
            assert_eq!((c.rc, c.errno), (-1, EINVAL), "{tag}");
        }
        // G2-060: memlimit / 1024 > UINT32_MAX
        for mem in [4398046511104usize, usize::MAX] {
            let (c, r) = run(name, good, 1, mem);
            let tag = format!("{name}(memlimit={mem})");
            eq_obs(&tag, &c, &r);
            assert_eq!((c.rc, c.errno), (-1, EINVAL), "{tag}");
        }
        // memlimit / 1024 == UINT32_MAX exactly is accepted by the guard (and
        // then simply mismatches the string) -> 1, not -1.
        let (c, r) = run(name, good, 1, 4294967295 * 1024);
        eq_obs(&format!("{name}(memlimit=UINT32_MAX*1024)"), &c, &r);
        assert_eq!(c.rc, 1, "the boundary value must pass the guard");
    }

    // G2-061: strlen(str) >= crypto_pwhash_STRBYTES (128)
    for extra in [0usize, 1, 80] {
        let mut long = good_id[..good_id.len() - 1].to_vec();
        while long.len() < 128 + extra {
            long.push(b'A');
        }
        let ln = nul(&long);
        for name in [
            "crypto_pwhash_argon2i_str_needs_rehash",
            "crypto_pwhash_argon2id_str_needs_rehash",
            "crypto_pwhash_str_needs_rehash",
        ] {
            let (c, r) = run(name, &ln, 1, 8192);
            let tag = format!("{name}(strlen={})", long.len());
            eq_obs(&tag, &c, &r);
            assert_eq!((c.rc, c.errno), (-1, EINVAL), "{tag}");
        }
    }
    // …and exactly 127 characters is still allowed through the guard
    {
        let mut s = good_id[..good_id.len() - 1].to_vec();
        while s.len() < 127 {
            s.push(b'A');
        }
        let sn = nul(&s);
        let (c, r) = run("crypto_pwhash_argon2id_str_needs_rehash", &sn, 1, 8192);
        eq_obs("needs_rehash(strlen=127)", &c, &r);
        // trailing junk makes argon2_decode_string fail, so still -1/EINVAL,
        // but via the *decode* branch (G2-063), not the length guard.
        assert_eq!((c.rc, c.errno), (-1, EINVAL));
    }

    // G2-062 / G2-063: malformed / truncated / empty strings.
    // `str = ""` also drives `calloc(0, 1)` (which glibc satisfies with a
    // non-NULL pointer, so the `calloc == NULL` half of the row is dead).
    for s in [
        &b""[..],
        &b"$"[..],
        &b"$argon2id$"[..],
        &b"$argon2id$v=19$m=8,t=1,p=1$"[..],
        &b"$argon2i$v=19$m=8,t=3,p=1$"[..],
        &b"$argon2id$v=19$"[..],
        &b"$7$C6..../....WZaPV7LSUEKMo34."[..],
        &b"$argon2id$v=19$m=8,t=1,p=1$YWJjZGVmZ2g"[..],
    ] {
        let sn = nul(s);
        for name in [
            "crypto_pwhash_argon2i_str_needs_rehash",
            "crypto_pwhash_argon2id_str_needs_rehash",
        ] {
            let (c, r) = run(name, &sn, 1, 8192);
            let tag = format!("{name}({:?})", String::from_utf8_lossy(s));
            eq_obs(&tag, &c, &r);
            assert_eq!((c.rc, c.errno), (-1, EINVAL), "{tag}");
        }
    }

    // G2-064: a valid argon2id string through the argon2i entry point
    let (c, r) = run("crypto_pwhash_argon2i_str_needs_rehash", &good_id, 1, 8192);
    eq_obs("argon2i_str_needs_rehash(argon2id string)", &c, &r);
    assert_eq!((c.rc, c.errno), (-1, EINVAL));
    // G2-065: a valid argon2i string through the argon2id entry point
    let (c, r) = run("crypto_pwhash_argon2id_str_needs_rehash", &good_i, 3, 8192);
    eq_obs("argon2id_str_needs_rehash(argon2i string)", &c, &r);
    assert_eq!((c.rc, c.errno), (-1, EINVAL));
}

// ===========================================================================
// argon2.c
// ===========================================================================

/// Build a real encoded string with the C library so decode/verify tests have
/// self-consistent salt and hash fields.
fn build_encoded(
    ty: i32,
    m_cost: u32,
    t_cost: u32,
    lanes: u32,
    saltlen: usize,
    hashlen: usize,
    pwd: &[u8],
    pwdlen: usize,
) -> Vec<u8> {
    let f = sym::<Argon2Hash>(c_lib(), "_sodium_argon2_hash");
    let salt: Vec<u8> = (0..saltlen)
        .map(|i| (i as u8).wrapping_mul(7).wrapping_add(3))
        .collect();
    let mut enc = vec![0u8; 256];
    let rc = unsafe {
        f(
            t_cost,
            m_cost,
            lanes,
            pwd.as_ptr() as *const c_void,
            pwdlen,
            salt.as_ptr() as *const c_void,
            saltlen,
            std::ptr::null_mut(),
            hashlen,
            enc.as_mut_ptr() as *mut c_char,
            256,
            ty,
        )
    };
    assert_eq!(rc, ARGON2_OK, "build_encoded failed: {rc}");
    nul(&unsafe { cstr(enc.as_ptr() as *const c_char) })
}

/// ERRORS G2-066, G2-067, G2-068 — `argon2_ctx`.
#[test]
fn argon2_ctx_rejections() {
    setup();
    let (cf, rf) = pair::<Argon2Ctx>("_sodium_argon2_ctx");

    let run = |tag: &str, make: &dyn Fn(&mut Ctx), ty: i32| -> (Obs, Obs) {
        let mut res = Vec::new();
        for f in [cf, rf] {
            let mut out = canary(40);
            let mut pwd = [0x61u8; 8];
            let mut salt = [0x62u8; 16];
            let mut ctx = Ctx::zeroed();
            ctx.out = out.as_mut_ptr();
            ctx.outlen = 32;
            ctx.pwd = pwd.as_mut_ptr();
            ctx.pwdlen = 8;
            ctx.salt = salt.as_mut_ptr();
            ctx.saltlen = 16;
            ctx.t_cost = 1;
            ctx.m_cost = 8;
            ctx.lanes = 1;
            ctx.threads = 1;
            make(&mut ctx);
            set_errno(SENTINEL);
            let rc = unsafe { f(&raw mut ctx, ty) };
            res.push(Obs {
                rc,
                errno: get_errno(),
                buf: out,
            });
        }
        eq_obs(tag, &res[0], &res[1]);
        let b = res.remove(1);
        (res.remove(0), b)
    };

    // G2-066: validate_inputs failures come back verbatim
    for (what, f, want) in [
        (
            "out=NULL",
            &(|c: &mut Ctx| c.out = std::ptr::null_mut()) as &dyn Fn(&mut Ctx),
            ARGON2_OUTPUT_PTR_NULL,
        ),
        (
            "outlen=0",
            &(|c: &mut Ctx| c.outlen = 0),
            ARGON2_OUTPUT_TOO_SHORT,
        ),
        (
            "saltlen=4",
            &(|c: &mut Ctx| c.saltlen = 4),
            ARGON2_SALT_TOO_SHORT,
        ),
        (
            "m_cost=4",
            &(|c: &mut Ctx| c.m_cost = 4),
            ARGON2_MEMORY_TOO_LITTLE,
        ),
        (
            "t_cost=0",
            &(|c: &mut Ctx| c.t_cost = 0),
            ARGON2_TIME_TOO_SMALL,
        ),
        ("lanes=0", &(|c: &mut Ctx| c.lanes = 0), ARGON2_LANES_TOO_FEW),
        (
            "threads=0",
            &(|c: &mut Ctx| c.threads = 0),
            ARGON2_THREADS_TOO_FEW,
        ),
    ] {
        let (c, _) = run(&format!("argon2_ctx({what})"), f, ARGON2_ID);
        assert_eq!(c.rc, want, "argon2_ctx({what})");
        assert_eq!(c.buf, canary(40), "argon2_ctx({what}) must not write out");
    }
    // NULL context
    for f in [cf, rf] {
        set_errno(SENTINEL);
        let rc = unsafe { f(std::ptr::null_mut(), ARGON2_ID) };
        assert_eq!(rc, ARGON2_INCORRECT_PARAMETER, "argon2_ctx(NULL)");
    }

    // G2-067: type neither Argon2_i nor Argon2_id
    for ty in [0i32, 3, 4, -1, i32::MAX, i32::MIN] {
        let (c, _) = run(&format!("argon2_ctx(type={ty})"), &(|_: &mut Ctx| {}), ty);
        assert_eq!(c.rc, ARGON2_INCORRECT_TYPE, "argon2_ctx(type={ty})");
        assert_eq!(c.buf, canary(40));
    }

    // G2-068: argon2_initialize fails (4 TB block region)
    let (c, _) = run(
        "argon2_ctx(m_cost=4294967295 -> alloc failure)",
        &(|c: &mut Ctx| c.m_cost = 4294967295),
        ARGON2_ID,
    );
    assert_eq!(c.rc, ARGON2_MEMORY_ALLOCATION_ERROR);
    assert_eq!(c.buf, canary(40));
}

/// ERRORS G2-069, G2-070, G2-071, G2-073, G2-074, G2-075, G2-076 —
/// `argon2_hash` and its four thin wrappers.
///
/// G2-072 (`malloc(hashlen) == NULL`) is not constructible without an
/// allocator-failure injection; see `documented_dead_and_unreachable_rows`.
#[test]
fn argon2_hash_rejections() {
    setup();
    let (cf, rf) = pair::<Argon2Hash>("_sodium_argon2_hash");

    #[allow(clippy::too_many_arguments)]
    let run = |tag: &str,
               t_cost: u32,
               m_cost: u32,
               par: u32,
               pwdlen: usize,
               saltlen: usize,
               hashlen: usize,
               want_hash: bool,
               encodedlen: usize,
               ty: i32|
     -> (Obs, Obs) {
        let _g = rng_lock();
        let mut res = Vec::new();
        for f in [cf, rf] {
            let mut h = canary(48);
            let mut e = canary(160);
            let pwd = [0x61u8; 64];
            let salt = [0x62u8; 64];
            set_errno(SENTINEL);
            reset_rngs(0x9400);
            let rc = unsafe {
                f(
                    t_cost,
                    m_cost,
                    par,
                    pwd.as_ptr() as *const c_void,
                    pwdlen,
                    salt.as_ptr() as *const c_void,
                    saltlen,
                    if want_hash {
                        h.as_mut_ptr() as *mut c_void
                    } else {
                        std::ptr::null_mut()
                    },
                    hashlen,
                    if encodedlen > 0 {
                        e.as_mut_ptr() as *mut c_char
                    } else {
                        std::ptr::null_mut()
                    },
                    encodedlen,
                    ty,
                )
            };
            let mut buf = h;
            buf.extend_from_slice(&e);
            res.push(Obs {
                rc,
                errno: get_errno(),
                buf,
            });
        }
        eq_obs(tag, &res[0], &res[1]);
        let b = res.remove(1);
        (res.remove(0), b)
    };

    // G2-069: pwdlen > ARGON2_MAX_PWD_LENGTH. `hash != NULL` so the
    // randombytes_buf() pre-fill happens first and must match exactly.
    for pwdlen in [4294967296usize, 4294967297, usize::MAX] {
        let (c, _) = run(
            &format!("argon2_hash(pwdlen={pwdlen})"),
            1,
            8,
            1,
            pwdlen,
            16,
            16,
            true,
            0,
            ARGON2_ID,
        );
        assert_eq!(c.rc, ARGON2_PWD_TOO_LONG, "pwdlen={pwdlen}");
        assert_ne!(&c.buf[..16], &canary(16)[..], "the pre-fill must be visible");
    }
    // G2-070: hashlen > ARGON2_MAX_OUTLEN. Only reachable with hash == NULL,
    // because otherwise `randombytes_buf(hash, hashlen)` would need a real
    // 4 GiB buffer.
    for hashlen in [4294967296usize, 4294967297, usize::MAX] {
        let (c, _) = run(
            &format!("argon2_hash(hashlen={hashlen})"),
            1,
            8,
            1,
            8,
            16,
            hashlen,
            false,
            0,
            ARGON2_ID,
        );
        assert_eq!(c.rc, ARGON2_OUTPUT_TOO_LONG, "hashlen={hashlen}");
    }
    // G2-071: saltlen > ARGON2_MAX_SALT_LENGTH
    for saltlen in [4294967296usize, usize::MAX] {
        let (c, _) = run(
            &format!("argon2_hash(saltlen={saltlen})"),
            1,
            8,
            1,
            8,
            saltlen,
            16,
            false,
            0,
            ARGON2_ID,
        );
        assert_eq!(c.rc, ARGON2_SALT_TOO_LONG, "saltlen={saltlen}");
    }
    // G2-073: argon2_ctx fails -> the code is returned and `out` freed. With
    // `hash != NULL` the caller's buffer keeps the random pre-fill (it is NOT
    // zeroed by argon2_hash — only the internal `out` is).
    for (what, t, m, par, saltlen, want) in [
        ("t_cost=0", 0u32, 8u32, 1u32, 16usize, ARGON2_TIME_TOO_SMALL),
        ("m_cost=7", 1, 7, 1, 16, ARGON2_MEMORY_TOO_LITTLE),
        ("m_cost=8,lanes=2", 1, 8, 2, 16, ARGON2_MEMORY_TOO_LITTLE),
        ("lanes=0", 1, 8, 0, 16, ARGON2_LANES_TOO_FEW),
        ("lanes=16777216", 1, 8, 16777216, 16, ARGON2_LANES_TOO_MANY),
        ("saltlen=7", 1, 8, 1, 7, ARGON2_SALT_TOO_SHORT),
        (
            "m_cost=4294967295",
            1,
            4294967295,
            1,
            16,
            ARGON2_MEMORY_ALLOCATION_ERROR,
        ),
    ] {
        let (c, _) = run(
            &format!("argon2_hash({what})"),
            t,
            m,
            par,
            8,
            saltlen,
            16,
            true,
            0,
            ARGON2_ID,
        );
        assert_eq!(c.rc, want, "argon2_hash({what})");
    }
    // …and with an out-of-range type
    for ty in [0i32, 3, -1] {
        let (c, _) = run(
            &format!("argon2_hash(type={ty})"),
            1,
            8,
            1,
            8,
            16,
            16,
            true,
            0,
            ty,
        );
        assert_eq!(c.rc, ARGON2_INCORRECT_TYPE, "argon2_hash(type={ty})");
    }

    // G2-074: argon2_encode_string fails -> ARGON2_ENCODING_FAIL, and BOTH
    // the raw hash and the encoded buffer are zeroed.
    //
    // NB the ERRORS table's example "`encodedlen` = 40 for a 32-byte hash" does
    // NOT return ARGON2_ENCODING_FAIL: 28..=49 and 51..=93 abort inside
    // `sodium_bin2base64` (`sodium_misuse()`), see
    // `argon2_encode_string_rejections` and `misuse_paths_match`. Only
    // 0..=27 and 50 are ENCODING_FAIL.
    for encodedlen in [1usize, 2, 12, 26, 27, 50] {
        let (c, _) = run(
            &format!("argon2_hash(encodedlen={encodedlen})"),
            1,
            8,
            1,
            8,
            16,
            32,
            true,
            encodedlen,
            ARGON2_ID,
        );
        assert_eq!(
            c.rc, ARGON2_ENCODING_FAIL,
            "encodedlen={encodedlen} must be too small"
        );
        assert_eq!(
            &c.buf[48..48 + encodedlen],
            &vec![0u8; encodedlen][..],
            "encodedlen={encodedlen}: the encoded buffer must be zeroed"
        );
    }
    // 94 bytes is just enough for "$argon2id$v=19$m=8,t=1,p=1$<22>$<43>" + NUL
    let (c, _) = run(
        "argon2_hash(encodedlen=94)",
        1,
        8,
        1,
        8,
        16,
        32,
        true,
        94,
        ARGON2_ID,
    );
    assert_eq!(c.rc, ARGON2_OK, "94 bytes must be enough");

    // G2-075 / G2-076: the four thin wrappers report the same codes
    let (cr, rr) = pair::<Argon2HashRaw>("_sodium_argon2i_hash_raw");
    let (cr2, rr2) = pair::<Argon2HashRaw>("_sodium_argon2id_hash_raw");
    for (name, cfn, rfn) in [
        ("argon2i_hash_raw", cr, rr),
        ("argon2id_hash_raw", cr2, rr2),
    ] {
        for (what, t, m, par, saltlen, want) in [
            ("t_cost=0", 0u32, 8u32, 1u32, 16usize, ARGON2_TIME_TOO_SMALL),
            ("m_cost=7", 1, 7, 1, 16, ARGON2_MEMORY_TOO_LITTLE),
            ("saltlen=0", 1, 8, 1, 0, ARGON2_SALT_TOO_SHORT),
            (
                "m_cost=4294967295",
                1,
                4294967295,
                1,
                16,
                ARGON2_MEMORY_ALLOCATION_ERROR,
            ),
        ] {
            let _g = rng_lock();
            let mut res = Vec::new();
            for f in [cfn, rfn] {
                let mut h = canary(24);
                let pwd = [0x61u8; 8];
                let salt = [0x62u8; 16];
                set_errno(SENTINEL);
                reset_rngs(0x9401);
                let rc = unsafe {
                    f(
                        t,
                        m,
                        par,
                        pwd.as_ptr() as *const c_void,
                        8,
                        salt.as_ptr() as *const c_void,
                        saltlen,
                        h.as_mut_ptr() as *mut c_void,
                        16,
                    )
                };
                res.push(Obs {
                    rc,
                    errno: get_errno(),
                    buf: h,
                });
            }
            let tag = format!("{name}({what})");
            eq_obs(&tag, &res[0], &res[1]);
            assert_eq!(res[0].rc, want, "{tag}");
        }
    }
    let (ce, re) = pair::<Argon2HashEnc>("_sodium_argon2i_hash_encoded");
    let (ce2, re2) = pair::<Argon2HashEnc>("_sodium_argon2id_hash_encoded");
    for (name, cfn, rfn) in [
        ("argon2i_hash_encoded", ce, re),
        ("argon2id_hash_encoded", ce2, re2),
    ] {
        for (what, t, m, encodedlen, want) in [
            ("t_cost=0", 0u32, 8u32, 128usize, ARGON2_TIME_TOO_SMALL),
            ("m_cost=7", 1, 7, 128, ARGON2_MEMORY_TOO_LITTLE),
            // NB the ENCODING_FAIL / sodium_misuse() boundary depends on the
            // variant literal length ("$argon2i$v=" is one byte shorter than
            // "$argon2id$v="), so only values that fail inside the literals for
            // BOTH variants are used here. The per-variant boundaries are in
            // `argon2_encode_string_rejections`.
            ("encodedlen=1", 1, 8, 1, ARGON2_ENCODING_FAIL),
            ("encodedlen=2", 1, 8, 2, ARGON2_ENCODING_FAIL),
            ("encodedlen=12", 1, 8, 12, ARGON2_ENCODING_FAIL),
            ("encodedlen=26", 1, 8, 26, ARGON2_ENCODING_FAIL),
            (
                "m_cost=4294967295",
                1,
                4294967295,
                128,
                ARGON2_MEMORY_ALLOCATION_ERROR,
            ),
        ] {
            let mut res = Vec::new();
            for f in [cfn, rfn] {
                let mut e = canary(160);
                let pwd = [0x61u8; 8];
                let salt = [0x62u8; 16];
                set_errno(SENTINEL);
                let rc = unsafe {
                    f(
                        t,
                        m,
                        1,
                        pwd.as_ptr() as *const c_void,
                        8,
                        salt.as_ptr() as *const c_void,
                        16,
                        32,
                        e.as_mut_ptr() as *mut c_char,
                        encodedlen,
                    )
                };
                res.push(Obs {
                    rc,
                    errno: get_errno(),
                    buf: e,
                });
            }
            let tag = format!("{name}({what})");
            eq_obs(&tag, &res[0], &res[1]);
            assert_eq!(res[0].rc, want, "{tag}");
        }
    }
}

/// ERRORS G2-080, G2-081, G2-082, G2-083 — `argon2_verify` /
/// `argon2i_verify` / `argon2id_verify`.
///
/// G2-077 (`strlen(encoded) > UINT32_MAX`) and G2-078 / G2-079 (any of the four
/// `malloc(strlen(encoded))` calls returning NULL) are not constructible; see
/// `documented_dead_and_unreachable_rows`.
#[test]
fn argon2_verify_rejections() {
    setup();
    let good_id = build_encoded(ARGON2_ID, 8, 1, 1, 16, 32, b"password\0", 8);
    let good_i = build_encoded(ARGON2_I, 8, 3, 1, 16, 32, b"password\0", 8);

    let run = |name: &str, s: &[u8], pw: &[u8], pwlen: usize, ty: Option<i32>| -> (Obs, Obs) {
        let _g = rng_lock();
        let mut res = Vec::new();
        if let Some(ty) = ty {
            let (cf, rf) = pair::<Argon2Verify>(name);
            for f in [cf, rf] {
                set_errno(SENTINEL);
                reset_rngs(0x9500);
                let rc =
                    unsafe { f(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_void, pwlen, ty) };
                res.push(Obs {
                    rc,
                    errno: get_errno(),
                    buf: Vec::new(),
                });
            }
        } else {
            let (cf, rf) = pair::<Argon2VerifyT>(name);
            for f in [cf, rf] {
                set_errno(SENTINEL);
                reset_rngs(0x9500);
                let rc =
                    unsafe { f(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_void, pwlen) };
                res.push(Obs {
                    rc,
                    errno: get_errno(),
                    buf: Vec::new(),
                });
            }
        }
        let b = res.remove(1);
        (res.remove(0), b)
    };

    // G2-081: wrong password -> ARGON2_VERIFY_MISMATCH
    let (c, r) = run(
        "_sodium_argon2_verify",
        &good_id,
        b"passworD\0",
        8,
        Some(ARGON2_ID),
    );
    eq_obs("argon2_verify(wrong pw)", &c, &r);
    assert_eq!(c.rc, ARGON2_VERIFY_MISMATCH);

    // G2-080: decode failures come back verbatim
    for (s, want) in [
        (&b""[..], ARGON2_DECODING_FAIL),
        (&b"$argon2id$"[..], ARGON2_DECODING_FAIL),
        (&b"$argon2id$v=16$m=8,t=1,p=1$YWJjZGVmZ2g$YWJjZGVmZ2hpamtsbW5vcA"[..], ARGON2_INCORRECT_TYPE),
        (&b"$argon2id$v=19$m=7,t=1,p=1$YWJjZGVmZ2g$YWJjZGVmZ2hpamtsbW5vcA"[..], ARGON2_MEMORY_TOO_LITTLE),
        (&b"$argon2id$v=19$m=8,t=0,p=1$YWJjZGVmZ2g$YWJjZGVmZ2hpamtsbW5vcA"[..], ARGON2_TIME_TOO_SMALL),
        (&b"$argon2id$v=19$m=8,t=1,p=0$YWJjZGVmZ2g$YWJjZGVmZ2hpamtsbW5vcA"[..], ARGON2_LANES_TOO_FEW),
        (&b"$argon2id$v=19$m=8,t=1,p=1$YWJj$YWJjZGVmZ2hpamtsbW5vcA"[..], ARGON2_SALT_TOO_SHORT),
        (&b"$argon2id$v=19$m=8,t=1,p=1$YWJjZGVmZ2g$YWJjZA"[..], ARGON2_OUTPUT_TOO_SHORT),
    ] {
        let sn = nul(s);
        let (c, r) = run(
            "_sodium_argon2_verify",
            &sn,
            b"password\0",
            8,
            Some(ARGON2_ID),
        );
        let tag = format!("argon2_verify({:?})", String::from_utf8_lossy(s));
        eq_obs(&tag, &c, &r);
        assert_eq!(c.rc, want, "{tag}");
    }
    // an unhandled `type` short-circuits inside argon2_decode_string
    for ty in [0i32, 3, -1] {
        let (c, r) = run("_sodium_argon2_verify", &good_id, b"password\0", 8, Some(ty));
        eq_obs(&format!("argon2_verify(type={ty})"), &c, &r);
        assert_eq!(c.rc, ARGON2_INCORRECT_TYPE, "argon2_verify(type={ty})");
    }

    // G2-082: the re-hash itself fails — a decodable string whose `m=` is too
    // large to allocate (4 TB).
    let huge = nul(b"$argon2id$v=19$m=4294967295,t=1,p=1$YWJjZGVmZ2g$YWJjZGVmZ2hpamtsbW5vcA");
    let (c, r) = run("_sodium_argon2_verify", &huge, b"password\0", 8, Some(ARGON2_ID));
    eq_obs("argon2_verify(m=4294967295)", &c, &r);
    assert_eq!(c.rc, ARGON2_MEMORY_ALLOCATION_ERROR);

    // G2-083: the two type-pinned wrappers
    for (name, mine, theirs) in [
        ("_sodium_argon2i_verify", &good_i, &good_id),
        ("_sodium_argon2id_verify", &good_id, &good_i),
    ] {
        let (c, r) = run(name, mine, b"passworD\0", 8, None);
        eq_obs(&format!("{name}(wrong pw)"), &c, &r);
        assert_eq!(c.rc, ARGON2_VERIFY_MISMATCH, "{name}");
        let (c, r) = run(name, theirs, b"password\0", 8, None);
        eq_obs(&format!("{name}(other variant)"), &c, &r);
        assert_eq!(c.rc, ARGON2_DECODING_FAIL, "{name}");
        for s in [&b""[..], &b"$"[..], &b"junk"[..]] {
            let sn = nul(s);
            let (c, r) = run(name, &sn, b"password\0", 8, None);
            eq_obs(&format!("{name}({:?})", String::from_utf8_lossy(s)), &c, &r);
            assert_eq!(c.rc, ARGON2_DECODING_FAIL);
        }
    }
}

// ===========================================================================
// argon2-core.c
// ===========================================================================

/// ERRORS G2-084, G2-085, G2-086, G2-088, G2-091, G2-092, G2-094, G2-097,
/// G2-100, G2-101, G2-102, G2-104, G2-105, G2-107, G2-108 —
/// `argon2_validate_inputs`, exhaustively, in the order the C checks them.
///
/// The (dead) rows G2-087, G2-089, G2-090, G2-093, G2-095, G2-096, G2-098,
/// G2-099, G2-103 are all `uint32_t`-unsatisfiable; see
/// `documented_dead_and_unreachable_rows`.
#[test]
fn argon2_validate_inputs_rejections() {
    setup();
    let (cf, rf) = pair::<Argon2Validate>("_sodium_argon2_validate_inputs");
    let mut out = [0u8; 32];
    let mut pwd = [0u8; 8];
    let mut salt = [0u8; 16];
    let mut secret = [0u8; 16];
    let mut ad = [0u8; 8];

    let mut base = Ctx::zeroed();
    base.out = out.as_mut_ptr();
    base.outlen = 32;
    base.pwd = pwd.as_mut_ptr();
    base.pwdlen = 8;
    base.salt = salt.as_mut_ptr();
    base.saltlen = 16;
    base.lanes = 1;
    base.threads = 1;
    base.m_cost = 8;
    base.t_cost = 1;

    let run = |tag: &str, ctx: &Ctx| -> i32 {
        set_errno(SENTINEL);
        let a = unsafe { cf(ctx as *const Ctx) };
        let ea = get_errno();
        set_errno(SENTINEL);
        let b = unsafe { rf(ctx as *const Ctx) };
        let eb = get_errno();
        eq_i32(&format!("validate_inputs({tag})"), a, b);
        assert_eq!(ea, eb, "validate_inputs({tag}) errno");
        a
    };

    // G2-084: NULL context
    let a = unsafe { cf(std::ptr::null()) };
    let b = unsafe { rf(std::ptr::null()) };
    eq_i32("validate_inputs(NULL)", a, b);
    assert_eq!(a, ARGON2_INCORRECT_PARAMETER);

    // sanity: the base context validates
    assert_eq!(run("base", &base), ARGON2_OK);

    // G2-085: out == NULL
    let mut z = base;
    z.out = std::ptr::null_mut();
    assert_eq!(run("out=NULL", &z), ARGON2_OUTPUT_PTR_NULL);
    // …even with outlen == 0 (the NULL check comes first)
    let mut z = base;
    z.out = std::ptr::null_mut();
    z.outlen = 0;
    assert_eq!(run("out=NULL,outlen=0", &z), ARGON2_OUTPUT_PTR_NULL);

    // G2-086: outlen < ARGON2_MIN_OUTLEN
    for outlen in [0u32, 1, 2, 8, 15] {
        let mut z = base;
        z.outlen = outlen;
        assert_eq!(
            run(&format!("outlen={outlen}"), &z),
            ARGON2_OUTPUT_TOO_SHORT
        );
    }
    let mut z = base;
    z.outlen = 16;
    assert_eq!(run("outlen=16", &z), ARGON2_OK);

    // G2-088: pwd == NULL with pwdlen != 0
    for pwdlen in [1u32, 8, 4294967295] {
        let mut z = base;
        z.pwd = std::ptr::null_mut();
        z.pwdlen = pwdlen;
        assert_eq!(
            run(&format!("pwd=NULL,pwdlen={pwdlen}"), &z),
            ARGON2_PWD_PTR_MISMATCH
        );
    }

    // G2-091: salt == NULL with saltlen != 0
    for saltlen in [1u32, 8, 16, 4294967295] {
        let mut z = base;
        z.salt = std::ptr::null_mut();
        z.saltlen = saltlen;
        assert_eq!(
            run(&format!("salt=NULL,saltlen={saltlen}"), &z),
            ARGON2_SALT_PTR_MISMATCH
        );
    }
    // salt == NULL with saltlen == 0 falls through to the SALT_TOO_SHORT check
    let mut z = base;
    z.salt = std::ptr::null_mut();
    z.saltlen = 0;
    assert_eq!(run("salt=NULL,saltlen=0", &z), ARGON2_SALT_TOO_SHORT);

    // G2-092: saltlen < ARGON2_MIN_SALT_LENGTH
    for saltlen in [0u32, 1, 4, 7] {
        let mut z = base;
        z.saltlen = saltlen;
        assert_eq!(run(&format!("saltlen={saltlen}"), &z), ARGON2_SALT_TOO_SHORT);
    }
    let mut z = base;
    z.saltlen = 8;
    assert_eq!(run("saltlen=8", &z), ARGON2_OK);

    // G2-094: secret == NULL with secretlen != 0
    for secretlen in [1u32, 16, 4294967295] {
        let mut z = base;
        z.secret = std::ptr::null_mut();
        z.secretlen = secretlen;
        assert_eq!(
            run(&format!("secret=NULL,secretlen={secretlen}"), &z),
            ARGON2_SECRET_PTR_MISMATCH
        );
    }
    // a non-NULL secret with any length validates (both bounds are dead)
    for secretlen in [0u32, 1, 16] {
        let mut z = base;
        z.secret = secret.as_mut_ptr();
        z.secretlen = secretlen;
        assert_eq!(run(&format!("secretlen={secretlen}"), &z), ARGON2_OK);
    }

    // G2-097: ad == NULL with adlen != 0
    for adlen in [1u32, 8, 4294967295] {
        let mut z = base;
        z.ad = std::ptr::null_mut();
        z.adlen = adlen;
        assert_eq!(
            run(&format!("ad=NULL,adlen={adlen}"), &z),
            ARGON2_AD_PTR_MISMATCH
        );
    }
    for adlen in [0u32, 1, 8] {
        let mut z = base;
        z.ad = ad.as_mut_ptr();
        z.adlen = adlen;
        assert_eq!(run(&format!("adlen={adlen}"), &z), ARGON2_OK);
    }

    // G2-100: lanes < ARGON2_MIN_LANES
    let mut z = base;
    z.lanes = 0;
    assert_eq!(run("lanes=0", &z), ARGON2_LANES_TOO_FEW);
    // G2-101: lanes > ARGON2_MAX_LANES
    for lanes in [16777216u32, 16777217, 4294967295] {
        let mut z = base;
        z.lanes = lanes;
        z.threads = 1;
        z.m_cost = 4294967295;
        assert_eq!(run(&format!("lanes={lanes}"), &z), ARGON2_LANES_TOO_MANY);
    }

    // G2-102: m_cost < ARGON2_MIN_MEMORY
    for m in [0u32, 1, 4, 7] {
        let mut z = base;
        z.m_cost = m;
        assert_eq!(run(&format!("m_cost={m}"), &z), ARGON2_MEMORY_TOO_LITTLE);
    }
    // G2-104: m_cost < 8 * lanes (the per-lane check)
    for (lanes, m) in [(2u32, 8u32), (2, 15), (4, 16), (4, 31), (8, 63), (100, 799)] {
        let mut z = base;
        z.lanes = lanes;
        z.threads = lanes;
        z.m_cost = m;
        assert_eq!(
            run(&format!("lanes={lanes},m_cost={m}"), &z),
            ARGON2_MEMORY_TOO_LITTLE
        );
    }

    // G2-105: t_cost < ARGON2_MIN_TIME
    let mut z = base;
    z.t_cost = 0;
    assert_eq!(run("t_cost=0", &z), ARGON2_TIME_TOO_SMALL);

    // G2-107: threads < ARGON2_MIN_THREADS
    let mut z = base;
    z.threads = 0;
    assert_eq!(run("threads=0", &z), ARGON2_THREADS_TOO_FEW);
    // G2-108: threads > ARGON2_MAX_THREADS
    for threads in [16777216u32, 16777217, 4294967295] {
        let mut z = base;
        z.threads = threads;
        assert_eq!(
            run(&format!("threads={threads}"), &z),
            ARGON2_THREADS_TOO_MANY
        );
    }

    // check-ordering: the *first* failing test must win
    let mut z = base;
    z.out = std::ptr::null_mut();
    z.outlen = 0;
    z.pwd = std::ptr::null_mut();
    z.pwdlen = 1;
    z.salt = std::ptr::null_mut();
    z.saltlen = 1;
    z.lanes = 0;
    z.m_cost = 0;
    z.t_cost = 0;
    z.threads = 0;
    assert_eq!(run("everything wrong", &z), ARGON2_OUTPUT_PTR_NULL);
}

/// ERRORS G2-109, G2-110, G2-111, G2-113, G2-115 — `argon2_initialize` and
/// the `allocate_memory` failures behind it, driven with a hand-built
/// `argon2_instance_t`.
///
/// G2-112 (`region == NULL`) and G2-114 (`malloc(sizeof(block_region))`
/// failing) are not constructible; see `documented_dead_and_unreachable_rows`.
#[test]
fn argon2_initialize_rejections() {
    setup();
    let (cf, rf) = pair::<Argon2Initialize>("_sodium_argon2_initialize");

    // G2-109: instance == NULL / context == NULL
    let mut ctx = Ctx::zeroed();
    for (a, b, what) in [
        (std::ptr::null_mut::<Instance>(), std::ptr::null_mut::<Ctx>(), "both NULL"),
        (std::ptr::null_mut(), &raw mut ctx, "instance NULL"),
    ] {
        set_errno(SENTINEL);
        let ra = unsafe { cf(a, b) };
        set_errno(SENTINEL);
        let rb = unsafe { rf(a, b) };
        eq_i32(&format!("argon2_initialize({what})"), ra, rb);
        assert_eq!(ra, ARGON2_INCORRECT_PARAMETER, "{what}");
    }
    // instance != NULL, context == NULL
    for f in [cf, rf] {
        let mut inst = Instance::zeroed();
        set_errno(SENTINEL);
        let rc = unsafe { f(&raw mut inst, std::ptr::null_mut()) };
        assert_eq!(rc, ARGON2_INCORRECT_PARAMETER, "context NULL");
    }

    // G2-113 (+ G2-111): memory_blocks == 0 makes `allocate_memory` bail out
    // with ARGON2_MEMORY_ALLOCATION_ERROR before touching malloc for blocks.
    // G2-110: a 4294967295-entry `pseudo_rands` array is a 34 GB malloc which
    // may or may not be satisfied; either outcome yields the same -22 here,
    // because `memory_blocks == 0` fails next.
    for (segment_length, memory_blocks, what) in [
        (0u32, 0u32, "segment_length=0, memory_blocks=0"),
        (2, 0, "memory_blocks=0"),
        (4294967295, 0, "segment_length=UINT32_MAX"),
    ] {
        let mut res = Vec::new();
        for f in [cf, rf] {
            let mut out = [0u8; 32];
            let mut pwd = [0u8; 8];
            let mut salt = [0u8; 16];
            let mut ctx = Ctx::zeroed();
            ctx.out = out.as_mut_ptr();
            ctx.outlen = 32;
            ctx.pwd = pwd.as_mut_ptr();
            ctx.pwdlen = 8;
            ctx.salt = salt.as_mut_ptr();
            ctx.saltlen = 16;
            ctx.t_cost = 1;
            ctx.m_cost = 8;
            ctx.lanes = 1;
            ctx.threads = 1;
            let mut inst = Instance::zeroed();
            inst.passes = 1;
            inst.current_pass = !0u32;
            inst.memory_blocks = memory_blocks;
            inst.segment_length = segment_length;
            inst.lane_length = segment_length.wrapping_mul(4);
            inst.lanes = 1;
            inst.threads = 1;
            inst.type_ = ARGON2_ID;
            set_errno(SENTINEL);
            let rc = unsafe { f(&raw mut inst, &raw mut ctx) };
            res.push(rc);
        }
        eq_i32(&format!("argon2_initialize({what})"), res[0], res[1]);
        assert_eq!(res[0], ARGON2_MEMORY_ALLOCATION_ERROR, "{what}");
    }

    // G2-115: the portable `malloc(memory_size + 63)` failing, reached through
    // the public API with memlimit == MEMLIMIT_MAX (m_cost = 4294967295 -> a
    // 4 TB request).
    let (cp, rp) = pair::<Pwhash>("crypto_pwhash_argon2id");
    let _g = rng_lock();
    let mut res = Vec::new();
    for f in [cp, rp] {
        let mut out = canary(32);
        let pw = [0x61u8; 8];
        let salt = [0x62u8; 16];
        set_errno(SENTINEL);
        reset_rngs(0x9600);
        let rc = unsafe {
            f(
                out.as_mut_ptr(),
                32,
                pw.as_ptr() as *const c_char,
                8,
                salt.as_ptr(),
                1,
                MEMLIMIT_MAX,
                ARGON2_ID,
            )
        };
        res.push(Obs {
            rc,
            errno: get_errno(),
            buf: out,
        });
    }
    eq_obs("allocate_memory(4 TB)", &res[0], &res[1]);
    assert_eq!(res[0].rc, -1);
    // see the note in `argon2_wrapper_range_rejections`: argon2_hash's
    // `randombytes_buf(hash, hashlen)` pre-fill survives the failure.
    assert_ne!(res[0].buf, vec![0u8; 32]);
    assert_ne!(res[0].buf, canary(32));
}

/// ERRORS G2-217, G2-218, G2-221 — the `void` early-return guards of
/// `argon2_fill_memory_blocks`, `argon2_fill_segment_ref` and
/// `argon2_finalize`. They signal nothing, so the differential check is that
/// neither library crashes and that nothing is written.
#[test]
fn argon2_void_null_guards() {
    setup();
    let (cfm, rfm) = pair::<Argon2FillMemory>("_sodium_argon2_fill_memory_blocks");
    let (cfs, rfs) = pair::<Argon2FillSegment>("_sodium_argon2_fill_segment_ref");
    let (cfz, rfz) = pair::<Argon2Finalize>("_sodium_argon2_finalize");

    // G2-217: instance == NULL, and instance->lanes == 0
    for f in [cfm, rfm] {
        unsafe { f(std::ptr::null_mut(), 0) };
        let mut inst = Instance::zeroed(); // lanes == 0
        inst.passes = 1;
        unsafe { f(&raw mut inst, 0) };
        assert_eq!(inst.lanes, 0);
        assert!(inst.region.is_null());
    }
    // G2-218: instance == NULL
    let pos = Position {
        pass: 0,
        lane: 0,
        slice: 0,
        index: 0,
    };
    for f in [cfs, rfs] {
        unsafe { f(std::ptr::null(), pos) };
    }
    // G2-221: context == NULL or instance == NULL
    for f in [cfz, rfz] {
        unsafe { f(std::ptr::null(), std::ptr::null_mut()) };
        let mut inst = Instance::zeroed();
        unsafe { f(std::ptr::null(), &raw mut inst) };
        let mut out = canary(32);
        let mut ctx = Ctx::zeroed();
        ctx.out = out.as_mut_ptr();
        ctx.outlen = 32;
        unsafe { f(&raw const ctx, std::ptr::null_mut()) };
        assert_eq!(out, canary(32), "argon2_finalize(instance=NULL) wrote out");
    }
}

// ===========================================================================
// argon2-encoding.c
// ===========================================================================

/// ERRORS G2-116 … G2-147 — `argon2_decode_string`, exhaustively.
#[test]
fn argon2_decode_string_rejections() {
    setup();
    let (cf, rf) = pair::<Argon2Decode>("_sodium_argon2_decode_string");

    // Runs the decoder on both libraries and compares rc, errno, the decoded
    // scalars and both output buffers.
    let run = |tag: &str, s: &[u8], ty: i32, saltbudget: u32, outbudget: u32| -> i32 {
        let sn = nul(s);
        let mut res = Vec::new();
        for f in [cf, rf] {
            let mut saltbuf = canary(256);
            let mut outbuf = canary(256);
            let mut ctx = Ctx::zeroed();
            ctx.salt = saltbuf.as_mut_ptr();
            ctx.saltlen = saltbudget;
            ctx.out = outbuf.as_mut_ptr();
            ctx.outlen = outbudget;
            set_errno(SENTINEL);
            let rc = unsafe { f(&raw mut ctx, sn.as_ptr() as *const c_char, ty) };
            let mut buf = saltbuf;
            buf.extend_from_slice(&outbuf);
            buf.extend_from_slice(&ctx.m_cost.to_le_bytes());
            buf.extend_from_slice(&ctx.t_cost.to_le_bytes());
            buf.extend_from_slice(&ctx.lanes.to_le_bytes());
            buf.extend_from_slice(&ctx.threads.to_le_bytes());
            buf.extend_from_slice(&ctx.saltlen.to_le_bytes());
            buf.extend_from_slice(&ctx.outlen.to_le_bytes());
            res.push(Obs {
                rc,
                errno: get_errno(),
                buf,
            });
        }
        eq_obs(&format!("decode_string[{tag}] {:?}", String::from_utf8_lossy(s)), &res[0], &res[1]);
        res[0].rc
    };

    // valid base64 fields: 8-byte salt "YWJjZGVmZ2g", 16-byte hash
    const S8: &str = "YWJjZGVmZ2g";
    const H16: &str = "YWJjZGVmZ2hpamtsbW5vcA";
    let ok_id = format!("$argon2id$v=19$m=8,t=1,p=1${S8}${H16}");
    let ok_i = format!("$argon2i$v=19$m=8,t=1,p=1${S8}${H16}");
    // sanity
    assert_eq!(run("control id", ok_id.as_bytes(), ARGON2_ID, 256, 256), ARGON2_OK);
    assert_eq!(run("control i", ok_i.as_bytes(), ARGON2_I, 256, 256), ARGON2_OK);

    // G2-116: type is neither Argon2_i nor Argon2_id
    for ty in [0i32, 3, 4, -1, i32::MAX, i32::MIN] {
        assert_eq!(
            run(&format!("type={ty}"), ok_id.as_bytes(), ty, 256, 256),
            ARGON2_INCORRECT_TYPE
        );
    }

    // G2-117: type == Argon2_id but the string does not start with "$argon2id"
    for s in [
        &ok_i[..],
        "$argon2d$v=19$m=8,t=1,p=1$x$y",
        "argon2id$v=19$",
        "",
        "$argon2i",
        "$argon2",
        "$",
        "$argon2iD$v=19$",
    ] {
        assert_eq!(
            run("id prefix", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }
    // G2-118: type == Argon2_i but the string does not start with "$argon2i"
    // (only 8 bytes are compared, so "$argon2id…" gets past CC and then trips
    // on the `"$v="` check — that is row G2-119).
    for s in [
        "$argon2d$v=19$m=8,t=1,p=1$x$y",
        "argon2i$v=19$",
        "",
        "$argon2",
        "$ARGON2I$v=19$",
    ] {
        assert_eq!(
            run("i prefix", s.as_bytes(), ARGON2_I, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }

    // G2-119: `"$v="` missing after the variant tag
    for s in [
        &format!("$argon2id$m=8,t=1,p=1${S8}${H16}")[..], // legacy, no version
        "$argon2id$v19$m=8,t=1,p=1$x$y",
        "$argon2id$",
        "$argon2id",
        "$argon2id$V=19$",
        "$argon2id$v=$",
    ] {
        assert_eq!(
            run("no $v=", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }
    // the argon2id string fed to Argon2_i leaves "d$v=" -> DECODING_FAIL
    assert_eq!(
        run("argon2id via Argon2_i", ok_id.as_bytes(), ARGON2_I, 256, 256),
        ARGON2_DECODING_FAIL
    );

    // G2-120: the version field is not a minimal decimal / out of range
    for s in [
        "$argon2id$v=$m=8,t=1,p=1$x$y",
        "$argon2id$v=019$m=8,t=1,p=1$x$y",
        "$argon2id$v=00$m=8,t=1,p=1$x$y",
        "$argon2id$v=4294967296$m=8,t=1,p=1$x$y",
        "$argon2id$v=99999999999999999999$m=8,t=1,p=1$x$y",
        "$argon2id$v=18446744073709551616$m=8,t=1,p=1$x$y",
        "$argon2id$v=x19$m=8,t=1,p=1$x$y",
        "$argon2id$v=+19$m=8,t=1,p=1$x$y",
        "$argon2id$v=-19$m=8,t=1,p=1$x$y",
    ] {
        assert_eq!(
            run("bad v=", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }
    // G2-121: version != ARGON2_VERSION_NUMBER
    for v in [0u32, 1, 16, 18, 20, 100, 4294967295] {
        let s = format!("$argon2id$v={v}$m=8,t=1,p=1${S8}${H16}");
        assert_eq!(
            run(&format!("v={v}"), s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_INCORRECT_TYPE,
            "{s:?}"
        );
    }

    // G2-122: `"$m="` missing
    for s in [
        "$argon2id$v=19$t=1,p=1$x$y",
        "$argon2id$v=19",
        "$argon2id$v=19$",
        "$argon2id$v=19$M=8,t=1,p=1$x$y",
        "$argon2id$v=19,m=8,t=1,p=1$x$y",
    ] {
        assert_eq!(
            run("no $m=", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }
    // G2-123, G2-145, G2-146, G2-147: the `m=` decimal
    for s in [
        "$argon2id$v=19$m=,t=1,p=1$x$y",
        "$argon2id$v=19$m=0008,t=1,p=1$x$y",
        "$argon2id$v=19$m=00,t=1,p=1$x$y",
        "$argon2id$v=19$m=4294967296,t=1,p=1$x$y",
        "$argon2id$v=19$m=99999999999999999999,t=1,p=1$x$y", // G2-145
        "$argon2id$v=19$m=18446744073709551616,t=1,p=1$x$y", // G2-146
        "$argon2id$v=19$m=x,t=1,p=1$x$y",
    ] {
        assert_eq!(
            run("bad m=", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }
    // G2-147: a bare "0" IS accepted by decode_decimal, and then fails
    // validate_inputs.
    assert_eq!(
        run(
            "m=0",
            format!("$argon2id$v=19$m=0,t=1,p=1${S8}${H16}").as_bytes(),
            ARGON2_ID,
            256,
            256
        ),
        ARGON2_MEMORY_TOO_LITTLE
    );

    // G2-125: `",t="` missing
    for s in [
        "$argon2id$v=19$m=8,p=1$x$y",
        "$argon2id$v=19$m=8$x$y",
        "$argon2id$v=19$m=8",
        "$argon2id$v=19$m=8,T=1,p=1$x$y",
        "$argon2id$v=19$m=8;t=1,p=1$x$y",
    ] {
        assert_eq!(
            run("no ,t=", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }
    // G2-126: the `t=` decimal
    for s in [
        "$argon2id$v=19$m=8,t=,p=1$x$y",
        "$argon2id$v=19$m=8,t=03,p=1$x$y",
        "$argon2id$v=19$m=8,t=4294967296,p=1$x$y",
        "$argon2id$v=19$m=8,t=99999999999999999999,p=1$x$y",
        "$argon2id$v=19$m=8,t=y,p=1$x$y",
    ] {
        assert_eq!(
            run("bad t=", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }

    // G2-128: `",p="` missing
    for s in [
        "$argon2id$v=19$m=8,t=1$x$y",
        "$argon2id$v=19$m=8,t=1,l=1$x$y",
        "$argon2id$v=19$m=8,t=1",
        "$argon2id$v=19$m=8,t=1,P=1$x$y",
    ] {
        assert_eq!(
            run("no ,p=", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }
    // G2-129: the `p=` decimal
    for s in [
        "$argon2id$v=19$m=8,t=1,p=$x$y",
        "$argon2id$v=19$m=8,t=1,p=01$x$y",
        "$argon2id$v=19$m=8,t=1,p=4294967296$x$y",
        "$argon2id$v=19$m=8,t=1,p=99999999999999999999$x$y",
        "$argon2id$v=19$m=8,t=1,p=z$x$y",
    ] {
        assert_eq!(
            run("bad p=", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }

    // G2-131: the first `"$"` separator before the salt is missing
    for s in [
        "$argon2id$v=19$m=8,t=1,p=1",
        "$argon2id$v=19$m=8,t=1,p=1,salt",
        &format!("$argon2id$v=19$m=8,t=1,p=1{S8}${H16}")[..],
    ] {
        assert_eq!(
            run("no salt $", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }

    // G2-132: the salt base64 is invalid
    for salt in ["A", "AB", "YWJjZGVmZ2g=", "YWJjZGVmZ2g==", "!!!!", "YWJj~GVmZ2g", "  "] {
        let s = format!("$argon2id$v=19$m=8,t=1,p=1${salt}${H16}");
        assert_eq!(
            run("bad salt b64", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }
    // G2-133: the decoded salt does not fit the caller's ctx->saltlen budget
    for budget in [0u32, 1, 7] {
        let s = format!("$argon2id$v=19$m=8,t=1,p=1${S8}${H16}");
        assert_eq!(
            run(
                &format!("salt budget {budget}"),
                s.as_bytes(),
                ARGON2_ID,
                budget,
                256
            ),
            ARGON2_DECODING_FAIL
        );
    }
    // exactly 8 bytes of budget is enough for an 8-byte salt
    assert_eq!(
        run("salt budget 8", ok_id.as_bytes(), ARGON2_ID, 8, 256),
        ARGON2_OK
    );

    // G2-135: the second `"$"` separator is missing
    for s in [
        &format!("$argon2id$v=19$m=8,t=1,p=1${S8}")[..],
        &format!("$argon2id$v=19$m=8,t=1,p=1${S8}{H16}")[..],
    ] {
        assert_eq!(
            run("no hash $", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }
    // G2-136: the output base64 is invalid or over budget.
    //
    // Two distinct outcomes, because `sodium_base642bin` only *fails* on a
    // dangling 6-bit group / non-zero trailing bits; a byte that is simply not
    // in the alphabet ends the input instead (`b64_end` is non-NULL). So:
    //   * "A" (6 dangling bits) and a trailing '=' (which then trips the
    //     "no characters left over" test) -> ARGON2_DECODING_FAIL
    //   * "!!!" / "" / "YWJj~" decode to 0 or 3 bytes and are then rejected by
    //     `argon2_validate_inputs` -> ARGON2_OUTPUT_TOO_SHORT
    for hash in ["A", "YWJjZGVmZ2hpamtsbW5vcA="] {
        let s = format!("$argon2id$v=19$m=8,t=1,p=1${S8}${hash}");
        assert_eq!(
            run("bad hash b64", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }
    for hash in ["!!!", "", "YWJj~", "   ", "YWJjZA"] {
        let s = format!("$argon2id$v=19$m=8,t=1,p=1${S8}${hash}");
        assert_eq!(
            run("short hash b64", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_OUTPUT_TOO_SHORT,
            "{s:?}"
        );
    }
    for budget in [0u32, 1, 15] {
        assert_eq!(
            run(
                &format!("out budget {budget}"),
                ok_id.as_bytes(),
                ARGON2_ID,
                256,
                budget
            ),
            ARGON2_DECODING_FAIL
        );
    }
    assert_eq!(
        run("out budget 16", ok_id.as_bytes(), ARGON2_ID, 256, 16),
        ARGON2_OK
    );

    // G2-137 … G2-143: the decoded parameters fail argon2_validate_inputs
    for (s, want) in [
        (
            format!("$argon2id$v=19$m=7,t=1,p=1${S8}${H16}"),
            ARGON2_MEMORY_TOO_LITTLE,
        ), // G2-137
        (
            format!("$argon2id$v=19$m=8,t=0,p=1${S8}${H16}"),
            ARGON2_TIME_TOO_SMALL,
        ), // G2-138
        (
            format!("$argon2id$v=19$m=8,t=1,p=0${S8}${H16}"),
            ARGON2_LANES_TOO_FEW,
        ), // G2-139
        (
            format!("$argon2id$v=19$m=4294967295,t=1,p=16777216${S8}${H16}"),
            ARGON2_LANES_TOO_MANY,
        ), // G2-140
        (
            format!("$argon2id$v=19$m=8,t=1,p=2${S8}${H16}"),
            ARGON2_MEMORY_TOO_LITTLE,
        ), // G2-141
        (
            format!("$argon2id$v=19$m=8,t=1,p=1$YWJj${H16}"),
            ARGON2_SALT_TOO_SHORT,
        ), // G2-142
        (
            format!("$argon2id$v=19$m=8,t=1,p=1${S8}$YWJjZA"),
            ARGON2_OUTPUT_TOO_SHORT,
        ), // G2-143
        (
            format!("$argon2id$v=19$m=8,t=1,p=1${S8}$YWJjZGVmZ2hpamtsbW4"),
            ARGON2_OUTPUT_TOO_SHORT,
        ), // 15-byte hash
    ] {
        assert_eq!(
            run("validate", s.as_bytes(), ARGON2_ID, 256, 256),
            want,
            "{s:?}"
        );
    }

    // G2-144: trailing characters after the output field
    // Only bytes *outside* the base64 alphabet count as "trailing": a base64
    // character would simply extend the hash field.
    for tail in ["\n", "$", " ", "$$", "=", "!", "~", "\t"] {
        let s = format!("$argon2id$v=19$m=8,t=1,p=1${S8}${H16}{tail}");
        assert_eq!(
            run("trailing", s.as_bytes(), ARGON2_ID, 256, 256),
            ARGON2_DECODING_FAIL,
            "{s:?}"
        );
    }
}

/// ERRORS G2-148, G2-149, G2-151 — `argon2_encode_string`.
///
/// **ERRORS G2-150 is mis-stated in the table**: `sodium_bin2base64` does *not*
/// return `NULL` when the destination is too small — libsodium 1.0.23 calls
/// `sodium_misuse()` there (`sodium/codecs.c:211-213`), so the `SB` macro's
/// `== NULL` branch is dead and the process **aborts** instead. The exact
/// boundaries are:
///
/// | `dst_len` | outcome |
/// |---|---|
/// | `0 ..= 27` | `ARGON2_ENCODING_FAIL` (an `SS`/`SX` literal does not fit) |
/// | `28 ..= 49` | `sodium_misuse()` inside `SB(salt)` |
/// | `50` | `ARGON2_ENCODING_FAIL` (the `SS("$")` between salt and hash) |
/// | `51 ..= 93` | `sodium_misuse()` inside `SB(out)` |
/// | `>= 94` | `ARGON2_OK` |
///
/// The two aborting ranges are covered by `misuse_paths_match`
/// (`encode/dst=…`).
#[test]
fn argon2_encode_string_rejections() {
    setup();
    let (cf, rf) = pair::<Argon2Encode>("_sodium_argon2_encode_string");

    let run = |tag: &str, dst_len: usize, mutate: &dyn Fn(&mut Ctx), ty: i32| -> i32 {
        let mut res = Vec::new();
        for f in [cf, rf] {
            let mut dst = canary(200);
            let mut salt = [0x41u8; 64];
            let mut out = [0x42u8; 64];
            let mut ctx = Ctx::zeroed();
            ctx.salt = salt.as_mut_ptr();
            ctx.saltlen = 16;
            ctx.out = out.as_mut_ptr();
            ctx.outlen = 32;
            ctx.m_cost = 8;
            ctx.t_cost = 1;
            ctx.lanes = 1;
            ctx.threads = 1;
            mutate(&mut ctx);
            set_errno(SENTINEL);
            let rc = unsafe { f(dst.as_mut_ptr() as *mut c_char, dst_len, &raw mut ctx, ty) };
            res.push(Obs {
                rc,
                errno: get_errno(),
                buf: dst,
            });
        }
        eq_obs(&format!("encode_string[{tag}]"), &res[0], &res[1]);
        res[0].rc
    };

    // G2-148: type neither Argon2_i nor Argon2_id (checked before anything is
    // written)
    for ty in [0i32, 3, 4, -1, i32::MAX, i32::MIN] {
        assert_eq!(
            run(&format!("type={ty}"), 200, &|_| {}, ty),
            ARGON2_ENCODING_FAIL
        );
    }

    // G2-149: dst_len too small for an `SS`/`SX` literal chunk. Every value in
    // 0..=27 fails inside the fixed prefix; 50 fails on the `SS("$")` that
    // separates the salt from the hash.
    for dst_len in (0usize..=27).chain([50usize]) {
        assert_eq!(
            run(&format!("dst_len={dst_len}"), dst_len, &|_| {}, ARGON2_ID),
            ARGON2_ENCODING_FAIL,
            "dst_len={dst_len}"
        );
    }
    // 94 = 27 (prefix) + 22 (salt b64) + 1 + 43 (hash b64) + NUL is the exact
    // minimum that succeeds.
    assert_eq!(run("dst_len=94", 94, &|_| {}, ARGON2_ID), ARGON2_OK);
    assert_eq!(run("dst_len=200", 200, &|_| {}, ARGON2_ID), ARGON2_OK);
    // `"$argon2i$v="` is one byte shorter, so every boundary shifts by one:
    // 0..=26 and 49 are ENCODING_FAIL, 27..=48 / 50..=92 abort, >= 93 is OK.
    for dst_len in (0usize..=26).chain([49usize]) {
        assert_eq!(
            run(&format!("i dst_len={dst_len}"), dst_len, &|_| {}, ARGON2_I),
            ARGON2_ENCODING_FAIL,
            "argon2i dst_len={dst_len}"
        );
    }
    assert_eq!(run("i dst_len=93", 93, &|_| {}, ARGON2_I), ARGON2_OK);

    // G2-151: argon2_validate_inputs fails *after* the variant literal has
    // been written, so its code is returned verbatim.
    for (what, mutate, want) in [
        (
            "outlen=0",
            &(|c: &mut Ctx| c.outlen = 0) as &dyn Fn(&mut Ctx),
            ARGON2_OUTPUT_TOO_SHORT,
        ),
        (
            "outlen=15",
            &(|c: &mut Ctx| c.outlen = 15),
            ARGON2_OUTPUT_TOO_SHORT,
        ),
        (
            "saltlen=4",
            &(|c: &mut Ctx| c.saltlen = 4),
            ARGON2_SALT_TOO_SHORT,
        ),
        (
            "out=NULL",
            &(|c: &mut Ctx| c.out = std::ptr::null_mut()),
            ARGON2_OUTPUT_PTR_NULL,
        ),
        ("m_cost=0", &(|c: &mut Ctx| c.m_cost = 0), ARGON2_MEMORY_TOO_LITTLE),
        ("t_cost=0", &(|c: &mut Ctx| c.t_cost = 0), ARGON2_TIME_TOO_SMALL),
        ("lanes=0", &(|c: &mut Ctx| c.lanes = 0), ARGON2_LANES_TOO_FEW),
        (
            "threads=0",
            &(|c: &mut Ctx| c.threads = 0),
            ARGON2_THREADS_TOO_FEW,
        ),
    ] {
        assert_eq!(
            run(&format!("validate {what}"), 200, mutate, ARGON2_ID),
            want,
            "encode_string validate {what}"
        );
    }
}

// ===========================================================================
// blake2b-long.c
// ===========================================================================

/// ERRORS G2-152, G2-153 — `blake2b_long` with `outlen > UINT32_MAX` and
/// `outlen == 0`.
///
/// G2-154 / G2-155 (an inner `crypto_generichash_blake2b_*` call failing) are
/// unreachable for any `outlen` in `1 ..= UINT32_MAX`; see
/// `documented_dead_and_unreachable_rows`.
#[test]
fn blake2b_long_rejections() {
    setup();
    let (cf, rf) = pair::<Blake2bLong>("_sodium_blake2b_long");
    let inp = [0x5Au8; 128];
    for outlen in [0usize, 4294967296, 4294967297, usize::MAX] {
        let mut res = Vec::new();
        for f in [cf, rf] {
            let mut out = canary(64);
            set_errno(SENTINEL);
            let rc = unsafe {
                f(
                    out.as_mut_ptr() as *mut c_void,
                    outlen,
                    inp.as_ptr() as *const c_void,
                    128,
                )
            };
            res.push(Obs {
                rc,
                errno: get_errno(),
                buf: out,
            });
        }
        eq_obs(&format!("blake2b_long(outlen={outlen})"), &res[0], &res[1]);
        assert_eq!(res[0].rc, -1, "outlen={outlen}");
        assert_eq!(
            res[0].buf,
            canary(64),
            "blake2b_long(outlen={outlen}) must not write"
        );
    }
    // inlen = 0 with outlen = 0 also fails; outlen = 1 succeeds (boundary)
    let mut out = canary(64);
    let rc = unsafe {
        cf(
            out.as_mut_ptr() as *mut c_void,
            1,
            inp.as_ptr() as *const c_void,
            0,
        )
    };
    assert_eq!(rc, 0, "outlen = 1 must succeed");
}

// ===========================================================================
// pwhash_scryptsalsa208sha256.c
// ===========================================================================

/// ERRORS G2-158, G2-160, G2-161 — `crypto_pwhash_scryptsalsa208sha256`.
///
/// G2-156 / G2-157 (`passwdlen`/`outlen` above their maxima) and G2-159
/// (`pickparams() != 0`) are dead or not constructible; see
/// `documented_dead_and_unreachable_rows`.
#[test]
fn scrypt_pwhash_rejections() {
    setup();
    let (cf, rf) = pair::<Scrypt>("crypto_pwhash_scryptsalsa208sha256");
    let salt = [7u8; 32];

    let run = |tag: &str, outbuf: usize, outlen: u64, pwlen: u64, ops: u64, mem: usize, alias: bool| -> Obs {
        let mut res = Vec::new();
        for f in [cf, rf] {
            let mut out = canary(outbuf);
            let mut pw = vec![0x61u8; 64];
            set_errno(SENTINEL);
            let pwp = if alias {
                out.as_mut_ptr() as *const c_char
            } else {
                pw.as_mut_ptr() as *const c_char
            };
            let rc = unsafe {
                f(
                    out.as_mut_ptr(),
                    outlen,
                    pwp,
                    pwlen,
                    salt.as_ptr(),
                    ops,
                    mem,
                )
            };
            res.push(Obs {
                rc,
                errno: get_errno(),
                buf: out,
            });
        }
        eq_obs(&format!("scrypt[{tag}]"), &res[0], &res[1]);
        res.remove(0)
    };

    // G2-158: outlen < BYTES_MIN
    for outlen in [0u64, 1, 2, 15] {
        let o = run(&format!("outlen={outlen}"), 32, outlen, 8, 32768, 16777216, false);
        assert_eq!(o.rc, -1, "outlen={outlen}");
        assert_eq!(o.errno, EINVAL, "outlen={outlen}");
        let mut want = canary(32);
        for x in want.iter_mut().take(outlen as usize) {
            *x = 0;
        }
        assert_eq!(o.buf, want, "outlen={outlen}: memset must have run");
    }

    // G2-160: aliasing out == passwd
    let o = run("out == passwd", 32, 32, 8, 32768, 16777216, true);
    assert_eq!(o.rc, -1);
    assert_eq!(o.errno, EINVAL);
    assert_eq!(o.buf, vec![0u8; 32]);

    // G2-161: the derived (N, r, p) is too large for `escrypt_kdf_nosse`.
    // opslimit = OPSLIMIT_MAX with memlimit = MEMLIMIT_MAX gives N = 2^26,
    // r = 8, p = 1 -> a ~68 GB `V` region.
    let o = run("huge N", 40, 32, 8, 4294967295, 68719476736, false);
    assert_eq!(o.rc, -1);
    assert_eq!(o.errno, ENOMEM, "the KDF's errno must be preserved");
    assert_eq!(&o.buf[..32], &vec![0u8; 32][..]);
}

/// ERRORS G2-166 — `crypto_pwhash_scryptsalsa208sha256_str` when `escrypt_r`
/// fails (a 68 GB `V` region). Note that `escrypt_r` `randombytes_buf()`es the
/// whole 102-byte output buffer *before* it fails, so the buffer contents are
/// part of the comparison.
///
/// G2-162, G2-163, G2-164, G2-165 are dead; see
/// `documented_dead_and_unreachable_rows`.
#[test]
fn scrypt_str_rejections() {
    setup();
    let (cf, rf) = pair::<PwStr>("crypto_pwhash_scryptsalsa208sha256_str");
    let _g = rng_lock();
    let mut res = Vec::new();
    for f in [cf, rf] {
        let mut out = canary(102);
        let pw = [0x61u8; 8];
        set_errno(SENTINEL);
        reset_rngs(0x9700);
        let rc = unsafe {
            f(
                out.as_mut_ptr() as *mut c_char,
                pw.as_ptr() as *const c_char,
                8,
                4294967295,
                68719476736,
            )
        };
        res.push(Obs {
            rc,
            errno: get_errno(),
            buf: out,
        });
    }
    eq_obs("scrypt_str(huge N)", &res[0], &res[1]);
    assert_eq!(res[0].rc, -1);
    assert_eq!(res[0].errno, EINVAL);
    assert_ne!(
        res[0].buf,
        vec![0u8; 102],
        "escrypt_r's randombytes_buf() pre-fill must be visible"
    );
}

/// A 101-character `"$7$"` string with the given 14-byte setting prefix,
/// followed by 43 salt characters and 43 hash characters. Only the setting
/// matters for the rejection rows.
fn scrypt_string(prefix14: &[u8]) -> Vec<u8> {
    assert_eq!(prefix14.len(), 14);
    let mut v = prefix14.to_vec();
    v.extend(std::iter::repeat_n(b'.', 43));
    v.push(b'$');
    v.extend(std::iter::repeat_n(b'.', 43));
    assert_eq!(v.len(), 101);
    nul(&v)
}

const ITOA64: &[u8] = b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Encode `v` as `n` little-endian 6-bit itoa64 characters.
fn enc64(v: u32, n: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut x = v;
    for _ in 0..n {
        out.push(ITOA64[(x & 0x3f) as usize]);
        x >>= 6;
    }
    out
}

fn setting14(n_log2: u32, r: u32, p: u32) -> Vec<u8> {
    let mut v = b"$7$".to_vec();
    v.push(ITOA64[(n_log2 & 63) as usize]);
    v.extend(enc64(r, 5));
    v.extend(enc64(p, 5));
    assert_eq!(v.len(), 14);
    v
}

/// ERRORS G2-167, G2-169, G2-170 —
/// `crypto_pwhash_scryptsalsa208sha256_str_verify`.
///
/// Every `escrypt_r` rejection is reached with parameters that are refused
/// *before* any hashing happens (`N < 2`, `N > UINT32_MAX`, `r == 0`,
/// `p == 0`, `r * p >= 2^30`, non-itoa64 bytes, a missing `'$'` after the
/// salt), so the sweep is free. G2-168 is dead.
#[test]
fn scrypt_str_verify_rejections() {
    setup();
    let (cf, rf) = pair::<PwStrVerify>("crypto_pwhash_scryptsalsa208sha256_str_verify");

    let run = |tag: &str, s: &[u8], pw: &[u8], pwlen: u64| -> Obs {
        let _g = rng_lock();
        let mut res = Vec::new();
        for f in [cf, rf] {
            set_errno(SENTINEL);
            reset_rngs(0x9800);
            let rc =
                unsafe { f(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pwlen) };
            res.push(Obs {
                rc,
                errno: get_errno(),
                buf: Vec::new(),
            });
        }
        eq_obs(&format!("scrypt_str_verify[{tag}]"), &res[0], &res[1]);
        res.remove(0)
    };

    // G2-167: sodium_strnlen(str, 102) != 101
    let good = scrypt_string(&setting14(10, 8, 1));
    for len in [0usize, 1, 13, 14, 50, 99, 100] {
        let mut s = good[..len].to_vec();
        s.push(0);
        let o = run(&format!("strnlen={len}"), &s, b"password\0", 8);
        assert_eq!(o.rc, -1, "strnlen={len}");
        assert_eq!(o.errno, SENTINEL, "the length guard must not touch errno");
    }
    // a 101-char string with no NUL at index 101 -> strnlen == 102
    let mut over = good[..101].to_vec();
    over.extend_from_slice(b"XXXX\0");
    let o = run("strnlen=102", &over, b"password\0", 8);
    assert_eq!(o.rc, -1);
    assert_eq!(o.errno, SENTINEL);

    // G2-169: escrypt_r rejections. All of these are 101 characters long, so
    // they pass the length guard.
    // (a) bad "$7$" prefix
    for pfx in [
        &b"$6$"[..],
        &b"7$$"[..],
        &b"$7."[..],
        &b"$$$"[..],
        &b"\0$$"[..],
        &b"$argon2id$"[..],
    ] {
        let mut p14 = pfx.to_vec();
        p14.resize(14, b'.');
        let s = scrypt_string(&p14);
        let o = run(
            &format!("prefix {:?}", String::from_utf8_lossy(pfx)),
            &s,
            b"password\0",
            8,
        );
        assert_eq!(o.rc, -1, "prefix {pfx:?}");
    }
    // (b) N_log2 == 0 -> N = 1 (< 2) and N_log2 == 63 -> N = 2^63 > UINT32_MAX
    for n_log2 in [0u32, 32, 33, 63] {
        let s = scrypt_string(&setting14(n_log2, 8, 1));
        let o = run(&format!("N_log2={n_log2}"), &s, b"password\0", 8);
        assert_eq!(o.rc, -1, "N_log2={n_log2}");
    }
    // (c) r == 0 / p == 0 / r * p >= 2^30
    for (r, p, what) in [
        (0u32, 1u32, "r=0"),
        (8, 0, "p=0"),
        (0, 0, "r=0,p=0"),
        (1, 1073741824, "r*p=2^30"),
        (8, 134217728, "r=8,p=2^27"),
        (32768, 32768, "r=p=32768"),
        (1056964609, 8, "huge r"),
        (8, 1056964609, "huge p"),
    ] {
        let s = scrypt_string(&setting14(10, r, p));
        let o = run(&format!("{what}"), &s, b"password\0", 8);
        assert_eq!(o.rc, -1, "{what}");
    }
    // (d) a non-itoa64 byte at each of the 11 parameter positions
    for pos in 3..14 {
        for bad in [b'$', b'!', b'-', b'~', 0x80u8] {
            let mut p14 = setting14(10, 8, 1);
            p14[pos] = bad;
            let s = scrypt_string(&p14);
            let o = run(&format!("bad byte {bad:#x} at {pos}"), &s, b"password\0", 8);
            assert_eq!(o.rc, -1, "byte {bad:#x} at {pos}");
        }
    }
    // (e) no '$' terminating the salt field -> saltlen = 87, need = 146 > 102
    {
        let mut v = setting14(10, 8, 1);
        v.extend(std::iter::repeat_n(b'.', 87));
        assert_eq!(v.len(), 101);
        let s = nul(&v);
        let o = run("no salt terminator", &s, b"password\0", 8);
        assert_eq!(o.rc, -1);
    }

    // G2-170: a correct 101-char string with the wrong password
    let (sf, _) = pair::<PwStr>("crypto_pwhash_scryptsalsa208sha256_str");
    let real = {
        let _g = rng_lock();
        let mut out = canary(102);
        reset_rngs(0x9801);
        let pw = b"password";
        let rc = unsafe {
            sf(
                out.as_mut_ptr() as *mut c_char,
                pw.as_ptr() as *const c_char,
                8,
                32768,
                16777216,
            )
        };
        assert_eq!(rc, 0);
        out
    };
    for (pw, pwlen) in [
        (&b"passworD\0"[..], 8u64),
        (&b"password\0"[..], 7),
        (&b"\0"[..], 0),
    ] {
        let o = run("wrong password", &real, pw, pwlen);
        assert_eq!(o.rc, -1, "wrong password {pw:?}");
        assert_eq!(o.errno, SENTINEL, "sodium_memcmp must not touch errno");
    }
    // control: the right password verifies
    let o = run("control", &real, b"password\0", 8);
    assert_eq!(o.rc, 0);
}

/// ERRORS G2-172, G2-173 —
/// `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash`. G2-171 is dead
/// (`pickparams` always returns 0).
#[test]
fn scrypt_needs_rehash_rejections() {
    setup();
    let (cf, rf) = pair::<PwNeedsRehash>("crypto_pwhash_scryptsalsa208sha256_str_needs_rehash");
    let run = |tag: &str, s: &[u8], ops: u64, mem: usize| -> Obs {
        let mut res = Vec::new();
        for f in [cf, rf] {
            set_errno(SENTINEL);
            let rc = unsafe { f(s.as_ptr() as *const c_char, ops, mem) };
            res.push(Obs {
                rc,
                errno: get_errno(),
                buf: Vec::new(),
            });
        }
        eq_obs(&format!("scrypt_needs_rehash[{tag}]"), &res[0], &res[1]);
        res.remove(0)
    };

    let good = scrypt_string(&setting14(10, 8, 1));
    // control
    let o = run("control", &good, 32768, 16777216);
    assert_eq!(o.rc, 0);

    // G2-172: sodium_strnlen(str, 102) != 101
    for len in [0usize, 1, 14, 50, 100] {
        let mut s = good[..len].to_vec();
        s.push(0);
        let o = run(&format!("strnlen={len}"), &s, 32768, 16777216);
        assert_eq!((o.rc, o.errno), (-1, EINVAL), "strnlen={len}");
    }
    let mut over = good[..101].to_vec();
    over.extend_from_slice(b"XX\0");
    let o = run("strnlen=102", &over, 32768, 16777216);
    assert_eq!((o.rc, o.errno), (-1, EINVAL));

    // G2-173: escrypt_parse_setting returns NULL
    for pfx in [&b"$6$"[..], &b"7$$"[..], &b"$7."[..]] {
        let mut p14 = pfx.to_vec();
        p14.resize(14, b'.');
        let s = scrypt_string(&p14);
        let o = run("bad prefix", &s, 32768, 16777216);
        assert_eq!((o.rc, o.errno), (-1, EINVAL));
    }
    for pos in 4..14 {
        // position 3 (the N_log2 char) accepts '\0' via strchr's terminator
        // quirk, so use bytes that are definitely outside the table
        for bad in [b'$', b'!', b'-', b'~'] {
            let mut p14 = setting14(10, 8, 1);
            p14[pos] = bad;
            let s = scrypt_string(&p14);
            let o = run(&format!("bad byte at {pos}"), &s, 32768, 16777216);
            assert_eq!((o.rc, o.errno), (-1, EINVAL), "byte {bad:#x} at {pos}");
        }
    }
}

// ===========================================================================
// crypto_scrypt-common.c
// ===========================================================================

/// ERRORS G2-174, G2-175, G2-176, G2-177, G2-178, G2-179 —
/// `escrypt_parse_setting` (and the static `decode64_one` /
/// `decode64_uint32` behind it): the `"$7$"` prefix and the 11 itoa64
/// parameter characters.
#[test]
fn escrypt_parse_setting_rejections() {
    setup();
    let (cf, rf) = pair::<EParse>("_sodium_escrypt_parse_setting");

    let run = |tag: &str, s: &[u8]| -> (bool, u32, u32, u32) {
        let mut res = Vec::new();
        for f in [cf, rf] {
            let (mut n, mut r, mut p) = (0xDEADBEEFu32, 0xDEADBEEFu32, 0xDEADBEEFu32);
            set_errno(SENTINEL);
            let ret = unsafe { f(s.as_ptr(), &raw mut n, &raw mut r, &raw mut p) };
            let off = if ret.is_null() {
                -1i64
            } else {
                (unsafe { ret.offset_from(s.as_ptr()) }) as i64
            };
            res.push((off, n, r, p, get_errno()));
        }
        assert_eq!(
            res[0], res[1],
            "escrypt_parse_setting[{tag}] {:?}: C={:?} Rust={:?}",
            String::from_utf8_lossy(s),
            res[0],
            res[1]
        );
        (res[0].0 < 0, res[0].1, res[0].2, res[0].3)
    };

    // G2-174: bad "$7$" prefix. `*_p` must stay untouched (0xDEADBEEF).
    for pfx in [
        &b"$6$"[..],
        &b"7$$"[..],
        &b"$7."[..],
        &b"$$$"[..],
        &b"$8$"[..],
        &b"aaa"[..],
        &b""[..],
        &b"$"[..],
        &b"$7"[..],
        &b"$argon2id$"[..],
    ] {
        let mut v = pfx.to_vec();
        v.resize(32, b'.');
        let s = nul(&v);
        let (is_null, n, r, p) = run("prefix", &s);
        if pfx.len() >= 3 && &pfx[..3] == b"$7$" {
            assert!(!is_null);
        } else {
            assert!(is_null, "prefix {pfx:?} must be rejected");
            assert_eq!((n, r, p), (0xDEADBEEF, 0xDEADBEEF, 0xDEADBEEF));
        }
    }

    // G2-175 / G2-178: setting[3] outside itoa64. NOTE: `strchr(itoa64, 0)`
    // returns the address of the terminating NUL, so a `'\0'` there decodes to
    // 64 rather than failing — a genuine C quirk the Rust must reproduce.
    for bad in [b'$', b'!', b'-', b'~', 0x80u8, 0xffu8, b' ', b'\n'] {
        let mut v = setting14(10, 8, 1);
        v[3] = bad;
        v.resize(32, b'.');
        let s = nul(&v);
        let (is_null, ..) = run(&format!("N_log2 byte {bad:#x}"), &s);
        assert!(is_null, "N_log2 byte {bad:#x} must be rejected");
    }
    {
        // the '\0' quirk, with enough trailing bytes to stay in bounds
        let mut v = setting14(10, 8, 1);
        v[3] = 0;
        v.resize(32, b'.');
        let s = nul(&v);
        let (is_null, n, ..) = run("N_log2 byte 0x00", &s);
        assert!(!is_null, "strchr(itoa64, 0) hits the NUL terminator");
        assert_eq!(n, 64);
    }

    // G2-176 / G2-177 / G2-179: a non-itoa64 byte in any of the r / p chars.
    // On failure `*dst` is set to 0 (not left alone) — checked by the exact
    // comparison in `run`.
    for pos in 4..14 {
        for bad in [b'$', b'!', b'-', b'~', 0x80u8, 0xffu8, b' '] {
            let mut v = setting14(10, 8, 1);
            v[pos] = bad;
            v.resize(32, b'.');
            let s = nul(&v);
            let (is_null, ..) = run(&format!("byte {bad:#x} at {pos}"), &s);
            assert!(is_null, "byte {bad:#x} at {pos} must be rejected");
        }
        // and the '\0' quirk at each position
        let mut v = setting14(10, 8, 1);
        v[pos] = 0;
        v.resize(32, b'.');
        let s = nul(&v);
        run(&format!("byte 0x00 at {pos}"), &s);
    }
}

/// ERRORS G2-180, G2-181, G2-182, G2-184 — `escrypt_r` called directly.
///
/// G2-183 (`need < saltlen`, a `size_t` wrap) and G2-185 ("Can't happen") are
/// dead; see `documented_dead_and_unreachable_rows`.
#[test]
fn escrypt_r_rejections() {
    setup();
    let (cf, rf) = pair::<ER>("_sodium_escrypt_r");
    let (ci, ri) = pair::<ERegion1>("_sodium_escrypt_init_local");
    let (cl, rl) = pair::<ERegion1>("_sodium_escrypt_free_local");

    let run = |tag: &str, setting: &[u8], buflen: usize, buf_null: bool| -> Obs {
        let _g = rng_lock();
        let mut res = Vec::new();
        for (f, init, freel) in [(cf, ci, cl), (rf, ri, rl)] {
            let mut local = Region::zeroed();
            assert_eq!(unsafe { init(&raw mut local) }, 0);
            let mut buf = canary(buflen.max(1));
            let pw = [0x61u8; 8];
            set_errno(SENTINEL);
            reset_rngs(0x9900);
            let ret = unsafe {
                f(
                    &raw mut local,
                    pw.as_ptr(),
                    8,
                    setting.as_ptr(),
                    if buf_null {
                        std::ptr::null_mut()
                    } else {
                        buf.as_mut_ptr()
                    },
                    buflen,
                )
            };
            let e = get_errno();
            assert_eq!(unsafe { freel(&raw mut local) }, 0);
            res.push(Obs {
                rc: if ret.is_null() { -1 } else { 0 },
                errno: e,
                buf,
            });
        }
        eq_obs(&format!("escrypt_r[{tag}]"), &res[0], &res[1]);
        res.remove(0)
    };

    let good14 = setting14(10, 8, 1);
    let mut good_setting = good14.clone();
    good_setting.extend(std::iter::repeat_n(b'.', 43));
    let good_setting = nul(&good_setting);

    // G2-180: escrypt_parse_setting fails
    for pfx in [&b"$6$"[..], &b"7$$"[..], &b"$7."[..]] {
        let mut v = pfx.to_vec();
        v.resize(14, b'.');
        v.extend(std::iter::repeat_n(b'.', 43));
        let s = nul(&v);
        let o = run("bad prefix", &s, 102, false);
        assert_eq!(o.rc, -1);
        // `buf` was randombytes_buf()ed before the parse, so it is NOT the
        // canary — both libraries must have written the same bytes.
        assert_ne!(o.buf, canary(102));
    }

    // G2-181: buf == NULL (checked *after* parse_setting)
    let o = run("buf=NULL", &good_setting, 102, true);
    assert_eq!(o.rc, -1);
    assert_eq!(o.buf, canary(102), "buf=NULL must not be written");

    // G2-182: need = 14 + saltlen + 1 + 43 + 1 > buflen
    for buflen in [1usize, 14, 50, 100, 101] {
        let o = run(&format!("buflen={buflen}"), &good_setting, buflen, false);
        assert_eq!(o.rc, -1, "buflen={buflen}");
    }
    // an over-long salt field (no '$' -> saltlen = 87 -> need = 146 > 102)
    {
        let mut v = good14.clone();
        v.extend(std::iter::repeat_n(b'.', 87));
        let s = nul(&v);
        let o = run("saltlen=87", &s, 102, false);
        assert_eq!(o.rc, -1);
    }

    // G2-184: escrypt_kdf_nosse fails
    for (n_log2, r, p, what) in [
        (0u32, 8u32, 1u32, "N=1"),
        (63, 8, 1, "N=2^63"),
        (32, 8, 1, "N=2^32"),
        (10, 0, 1, "r=0"),
        (10, 8, 0, "p=0"),
        (10, 1, 1073741824, "r*p=2^30"),
        (10, 1056964609, 8, "huge r"),
    ] {
        let mut v = setting14(n_log2, r, p);
        v.extend(std::iter::repeat_n(b'.', 43));
        let s = nul(&v);
        let o = run(what, &s, 102, false);
        assert_eq!(o.rc, -1, "{what}");
    }
}

/// ERRORS G2-186, G2-187, G2-188, G2-189 — `escrypt_gensalt_r`.
///
/// G2-190, G2-191, G2-192, G2-193, G2-194 are the "Can't happen" branches: the
/// `need > buflen` guard already reserves 14 + saltlen + 1 bytes, so
/// `encode64_uint32` / `encode64` can never run out. See
/// `documented_dead_and_unreachable_rows`.
#[test]
fn escrypt_gensalt_rejections() {
    setup();
    let (cf, rf) = pair::<EGensalt>("_sodium_escrypt_gensalt_r");
    let src = [0x33u8; 32];

    let run = |tag: &str, n_log2: u32, r: u32, p: u32, srclen: usize, buflen: usize| -> Obs {
        let mut res = Vec::new();
        for f in [cf, rf] {
            let mut buf = canary(buflen.max(1) + 8);
            set_errno(SENTINEL);
            let ret = unsafe { f(n_log2, r, p, src.as_ptr(), srclen, buf.as_mut_ptr(), buflen) };
            res.push(Obs {
                rc: if ret.is_null() { -1 } else { 0 },
                errno: get_errno(),
                buf,
            });
        }
        eq_obs(&format!("gensalt_r[{tag}]"), &res[0], &res[1]);
        res.remove(0)
    };

    // G2-186: need = 14 + 43 + 1 = 58 > buflen
    for buflen in [0usize, 1, 14, 40, 57] {
        let o = run(&format!("buflen={buflen}"), 10, 8, 1, 32, buflen);
        assert_eq!(o.rc, -1, "buflen={buflen}");
        assert_eq!(
            o.buf,
            canary(buflen.max(1) + 8),
            "buflen={buflen} must not write"
        );
    }
    // 58 is exactly enough
    let o = run("buflen=58", 10, 8, 1, 32, 58);
    assert_eq!(o.rc, 0);

    // G2-187: `saltlen < srclen` because BYTES2CHARS(srclen) wrapped.
    // srclen = 2^61 makes `srclen * 8` wrap to 0, so saltlen = 0 < srclen.
    for srclen in [
        2305843009213693952usize, // 2^61
        2305843009213693953,
        usize::MAX,
    ] {
        let o = run(&format!("srclen={srclen}"), 10, 8, 1, srclen, 58);
        assert_eq!(o.rc, -1, "srclen={srclen}");
        assert_eq!(o.buf, canary(66), "srclen={srclen} must not write");
    }

    // G2-188: N_log2 > 63
    for n_log2 in [64u32, 65, 100, 4294967295] {
        let o = run(&format!("N_log2={n_log2}"), n_log2, 8, 1, 32, 58);
        assert_eq!(o.rc, -1, "N_log2={n_log2}");
        assert_eq!(o.buf, canary(66));
    }

    // G2-189: r * p >= 2^30
    for (r, p) in [
        (8u32, 134217728u32),
        (1, 1073741824),
        (2, 536870912),
        (32768, 32768),
        (4294967295, 1),
        (1, 4294967295),
        (4294967295, 4294967295),
    ] {
        let o = run(&format!("r={r},p={p}"), 10, r, p, 32, 58);
        assert_eq!(o.rc, -1, "r={r},p={p}");
        assert_eq!(o.buf, canary(66));
    }
    // r * p == 2^30 - 1 is still accepted
    let o = run("r*p=2^30-1", 10, 1, 1073741823, 32, 58);
    assert_eq!(o.rc, 0);
}

/// ERRORS G2-197, G2-198, G2-199, G2-200, G2-201, G2-202, G2-203, G2-204,
/// G2-206, G2-207, G2-208, G2-210, G2-211, G2-212 —
/// `escrypt_kdf_nosse` (called directly and through
/// `crypto_pwhash_scryptsalsa208sha256_ll`), plus `escrypt_alloc_region`.
///
/// G2-195, G2-196, G2-205, G2-209, G2-213 are dead; see
/// `documented_dead_and_unreachable_rows`.
#[test]
fn escrypt_kdf_nosse_rejections() {
    setup();
    let (ck, rk) = pair::<EKdf>("_sodium_escrypt_kdf_nosse");
    let (cll, rll) = pair::<ScryptLl>("crypto_pwhash_scryptsalsa208sha256_ll");
    let (ci, ri) = pair::<ERegion1>("_sodium_escrypt_init_local");
    let (cl, rl) = pair::<ERegion1>("_sodium_escrypt_free_local");

    // Runs both the internal KDF (with an explicit local) and the public `_ll`
    // wrapper, which must agree with each other and across libraries.
    let run = |tag: &str, n: u64, r: u32, p: u32, buflen: usize| -> (i32, i32) {
        let mut res = Vec::new();
        for (f, init, freel) in [(ck, ci, cl), (rk, ri, rl)] {
            let mut local = Region::zeroed();
            assert_eq!(unsafe { init(&raw mut local) }, 0);
            let mut buf = canary(64);
            let pw = [0x61u8; 8];
            let salt = [0x62u8; 8];
            set_errno(SENTINEL);
            let rc = unsafe {
                f(
                    &raw mut local,
                    pw.as_ptr(),
                    8,
                    salt.as_ptr(),
                    8,
                    n,
                    r,
                    p,
                    buf.as_mut_ptr(),
                    buflen,
                )
            };
            let e = get_errno();
            assert_eq!(unsafe { freel(&raw mut local) }, 0);
            res.push(Obs {
                rc,
                errno: e,
                buf,
            });
        }
        eq_obs(&format!("escrypt_kdf_nosse[{tag}]"), &res[0], &res[1]);
        // …and the public `_ll` wrapper (G2-197)
        let mut res2 = Vec::new();
        for f in [cll, rll] {
            let mut buf = canary(64);
            let pw = [0x61u8; 8];
            let salt = [0x62u8; 8];
            set_errno(SENTINEL);
            let rc = unsafe {
                f(
                    pw.as_ptr(),
                    8,
                    salt.as_ptr(),
                    8,
                    n,
                    r,
                    p,
                    buf.as_mut_ptr(),
                    buflen,
                )
            };
            res2.push(Obs {
                rc,
                errno: get_errno(),
                buf,
            });
        }
        eq_obs(&format!("_ll[{tag}]"), &res2[0], &res2[1]);
        assert_eq!(res[0].rc, res2[0].rc, "_ll must forward the KDF rc [{tag}]");
        assert_eq!(
            res[0].errno, res2[0].errno,
            "_ll must preserve the KDF errno [{tag}]"
        );
        (res[0].rc, res[0].errno)
    };

    // G2-198: buflen > ((2^32)-1)*32
    for buflen in [137438953441usize, 137438953442, usize::MAX] {
        let (rc, e) = run(&format!("buflen={buflen}"), 16, 8, 1, buflen);
        assert_eq!((rc, e), (-1, EFBIG), "buflen={buflen}");
    }
    // G2-199: r * p >= 2^30
    for (r, p) in [
        (1u32, 1073741824u32),
        (32768, 32768),
        (2, 536870912),
        (4294967295, 1),
        (4294967295, 4294967295),
    ] {
        let (rc, e) = run(&format!("r={r},p={p}"), 16, r, p, 32);
        assert_eq!((rc, e), (-1, EFBIG), "r={r},p={p}");
    }
    // G2-200: N > UINT32_MAX
    for n in [4294967296u64, 8589934592, 1u64 << 63, u64::MAX] {
        let (rc, e) = run(&format!("N={n}"), n, 8, 1, 32);
        assert_eq!((rc, e), (-1, EFBIG), "N={n}");
    }
    // G2-201: N not a power of two
    for n in [3u64, 5, 6, 7, 100, 1000, 4294967295] {
        let (rc, e) = run(&format!("N={n}"), n, 8, 1, 32);
        assert_eq!((rc, e), (-1, EINVAL), "N={n}");
    }
    // G2-202: N < 2
    for n in [0u64, 1] {
        let (rc, e) = run(&format!("N={n}"), n, 8, 1, 32);
        assert_eq!((rc, e), (-1, EINVAL), "N={n}");
    }
    // G2-203 / G2-204: r == 0 / p == 0
    let (rc, e) = run("r=0", 16, 0, 1, 32);
    assert_eq!((rc, e), (-1, EINVAL));
    let (rc, e) = run("p=0", 16, 8, 0, 32);
    assert_eq!((rc, e), (-1, EINVAL));
    // `r = 0` with a huge `p` makes the product 0, so the `r * p >= 2^30` test
    // does NOT fire and the EINVAL branch wins.
    let (rc, e) = run("r=0,p=2^30", 16, 0, 1073741824, 32);
    assert_eq!((rc, e), (-1, EINVAL));
    let (rc, e) = run("r=1,p=2^30", 16, 1, 1073741824, 32);
    assert_eq!((rc, e), (-1, EFBIG), "the r*p test precedes the r/p == 0 test");
    // …and a bad `N` beats both
    let (rc, e) = run("N=3,r=0,p=0", 3, 0, 0, 32);
    assert_eq!((rc, e), (-1, EINVAL));
    let (rc, e) = run("N=2^32,r=0,p=0", 4294967296, 0, 0, 32);
    assert_eq!((rc, e), (-1, EFBIG), "the N > UINT32_MAX test precedes r/p");

    // G2-206: N > SIZE_MAX / 128 / r
    let (rc, e) = run("N=2^29,r=2^29", 1 << 29, 1 << 29, 1, 32);
    assert_eq!((rc, e), (-1, ENOMEM));

    // G2-207: `need = B_size + V_size` wraps
    let (rc, e) = run("need wrap", 134217728, 1073741818, 1, 32);
    assert_eq!((rc, e), (-1, ENOMEM), "B_size + V_size must wrap");
    // G2-208: `need += XY_size` wraps
    let (rc, e) = run("need += XY_size wrap", 268435456, 536870907, 1, 32);
    assert_eq!((rc, e), (-1, ENOMEM), "need + XY_size must wrap");

    // G2-210: escrypt_alloc_region fails (a 256 GiB V region)
    for (n, r, p) in [(2147483648u64, 1u32, 1u32), (1 << 30, 8, 1), (1 << 26, 8, 1)] {
        let (rc, _) = run(&format!("alloc N={n} r={r}"), n, r, p, 32);
        assert_eq!(rc, -1, "N={n} r={r} p={p} must fail to allocate");
    }

    // G2-211 / G2-212: escrypt_alloc_region directly
    let (ca, ra) = pair::<EAlloc>("_sodium_escrypt_alloc_region");
    for size in [usize::MAX, usize::MAX - 62, usize::MAX - 63, 1usize << 47] {
        let mut res = Vec::new();
        for f in [ca, ra] {
            let mut reg = Region {
                base: 0x1 as *mut c_void,
                aligned: 0x2 as *mut c_void,
                size: 0xdead,
            };
            set_errno(SENTINEL);
            let p = unsafe { f(&raw mut reg, size) };
            res.push((
                p.is_null(),
                reg.base.is_null(),
                reg.aligned.is_null(),
                reg.size,
                get_errno(),
            ));
        }
        assert_eq!(
            res[0], res[1],
            "escrypt_alloc_region({size}): C={:?} Rust={:?}",
            res[0], res[1]
        );
        assert!(res[0].0, "alloc_region({size}) must fail");
        assert!(res[0].1 && res[0].2, "base/aligned must be NULL");
        assert_eq!(res[0].3, 0, "region->size must be 0");
        if size >= usize::MAX - 62 {
            assert_eq!(res[0].4, ENOMEM, "the size + 63 wrap must set ENOMEM");
        }
    }
}

// ===========================================================================
// sodium_misuse() rows — out of process
// ===========================================================================

const MISUSE_CASES: &[&str] = &[
    // ERRORS G2-006 … G2-009: crypto_pwhash_str_alg falls through its switch
    "str_alg/alg=0",
    "str_alg/alg=3",
    "str_alg/alg=4",
    "str_alg/alg=-1",
    "str_alg/alg=intmax",
    "str_alg/alg=intmin",
    // ERRORS G2-214: escrypt_PBKDF2_SHA256 with dkLen > 0x1fffffffe0
    "pbkdf2/dklen=max+1",
    "pbkdf2/dklen=sizemax",
    // ERRORS G2-150 (corrected): argon2_encode_string with a dst_len that
    // fits the literals but not a base64 field -> sodium_bin2base64 ->
    // sodium_misuse(). 28..=49 abort in SB(salt), 51..=93 in SB(out).
    "encode/dst=28",
    "encode/dst=40",
    "encode/dst=49",
    "encode/dst=51",
    "encode/dst=71",
    "encode/dst=93",
    // the same for Argon2_i, whose literal prefix is one byte shorter, so the
    // aborting ranges are 27..=48 and 50..=92
    "encode_i/dst=27",
    "encode_i/dst=48",
    "encode_i/dst=50",
    "encode_i/dst=92",
    // …and the same through argon2_hash's `encoded` output
    "hash_encoded/enc=40",
    "hash_encoded/enc=71",
    // controls: the boundary values that must NOT abort
    "encode/dst=27",
    "encode/dst=50",
    "encode/dst=94",
    "encode_i/dst=26",
    "encode_i/dst=49",
    "encode_i/dst=93",
    "str_alg/alg=1",
    "str_alg/alg=2",
];

#[test]
fn misuse_child() {
    let Some((tag, lib)) = child_case() else {
        return;
    };
    let mut out = canary(128);
    match tag {
        t if t.starts_with("str_alg/") => {
            let alg: i32 = match &t["str_alg/".len()..] {
                "alg=0" => 0,
                "alg=1" => 1,
                "alg=2" => 2,
                "alg=3" => 3,
                "alg=4" => 4,
                "alg=-1" => -1,
                "alg=intmax" => i32::MAX,
                "alg=intmin" => i32::MIN,
                other => panic!("unknown alg {other}"),
            };
            // `crypto_pwhash_str_alg` writes nothing before the switch, so the
            // observation must still be the canary when it aborts.
            set_observation(out.as_ptr(), out.len());
            let f = sym::<PwStrAlg>(lib, "crypto_pwhash_str_alg");
            reset_rngs(0x9A00);
            let pw = b"password";
            let rc = unsafe {
                f(
                    out.as_mut_ptr() as *mut c_char,
                    pw.as_ptr() as *const c_char,
                    8,
                    3,
                    8192,
                    alg,
                )
            };
            println!("OBS rc={rc} out={}", hex(&out));
        }
        t if t.starts_with("pbkdf2/") => {
            let dklen: usize = match &t["pbkdf2/".len()..] {
                "dklen=max+1" => 137438953441,
                "dklen=sizemax" => usize::MAX,
                other => panic!("unknown dkLen {other}"),
            };
            set_observation(out.as_ptr(), out.len());
            let f = sym::<EPbkdf2>(lib, "_sodium_escrypt_PBKDF2_SHA256");
            let pw = [0x61u8; 8];
            let salt = [0x62u8; 8];
            unsafe {
                f(
                    pw.as_ptr(),
                    8,
                    salt.as_ptr(),
                    8,
                    1,
                    out.as_mut_ptr(),
                    dklen,
                )
            };
            println!("OBS returned out={}", hex(&out));
        }
        t if t.starts_with("encode/dst=") || t.starts_with("encode_i/dst=") => {
            let (ty, dst_len): (i32, usize) = if let Some(n) = t.strip_prefix("encode_i/dst=") {
                (ARGON2_I, n.parse().unwrap())
            } else {
                (ARGON2_ID, t["encode/dst=".len()..].parse().unwrap())
            };
            let mut salt = [0x41u8; 64];
            let mut o = [0x42u8; 64];
            let mut ctx = Ctx::zeroed();
            ctx.salt = salt.as_mut_ptr();
            ctx.saltlen = 16;
            ctx.out = o.as_mut_ptr();
            ctx.outlen = 32;
            ctx.m_cost = 8;
            ctx.t_cost = 1;
            ctx.lanes = 1;
            ctx.threads = 1;
            // whatever was written into `dst` before the abort is compared
            set_observation(out.as_ptr(), 128);
            let f = sym::<Argon2Encode>(lib, "_sodium_argon2_encode_string");
            let rc = unsafe { f(out.as_mut_ptr() as *mut c_char, dst_len, &raw mut ctx, ty) };
            println!("OBS rc={rc} dst={}", hex(&out));
        }
        t if t.starts_with("hash_encoded/enc=") => {
            let enc_len: usize = t["hash_encoded/enc=".len()..].parse().unwrap();
            let mut h = canary(32);
            let pwd = [0x61u8; 8];
            let salt = [0x62u8; 16];
            set_observation(out.as_ptr(), 128);
            let f = sym::<Argon2HashEnc>(lib, "_sodium_argon2id_hash_encoded");
            reset_rngs(0x9A01);
            let rc = unsafe {
                f(
                    1,
                    8,
                    1,
                    pwd.as_ptr() as *const c_void,
                    8,
                    salt.as_ptr() as *const c_void,
                    16,
                    32,
                    out.as_mut_ptr() as *mut c_char,
                    enc_len,
                )
            };
            println!("OBS rc={rc} enc={} h={}", hex(&out), hex(&h));
            h[0] ^= 0; // keep `h` alive
        }
        other => panic!("unknown tag {other}"),
    }
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}

/// ERRORS G2-006, G2-007, G2-008, G2-009, G2-010, G2-214, G2-223 — the only
/// two `sodium_misuse()` sites in `crypto_pwhash/`.
///
/// Note the asymmetry with `crypto_pwhash`, which returns `-1` /
/// `errno = EINVAL` for exactly the same out-of-range `alg` values (see
/// `crypto_pwhash_bad_alg`): `crypto_pwhash_str_alg` has no `default:` label
/// and therefore `abort()`s.
#[test]
fn misuse_paths_match() {
    if child_tag().is_some() {
        return;
    }
    setup();
    for &tag in MISUSE_CASES {
        let c = run_child("misuse_child", "c", tag);
        let r = run_child("misuse_child", "r", tag);
        eq_child(tag, &c, &r);
        let aborts = !matches!(
            tag,
            "str_alg/alg=1"
                | "str_alg/alg=2"
                | "encode/dst=27"
                | "encode/dst=50"
                | "encode/dst=94"
                | "encode_i/dst=26"
                | "encode_i/dst=49"
                | "encode_i/dst=93"
        );
        if aborts {
            assert_eq!(
                c.status.code(),
                Some(MISUSE_EXIT),
                "{tag}: C did not reach sodium_misuse (stdout: {}, stderr: {})",
                String::from_utf8_lossy(&c.stdout),
                String::from_utf8_lossy(&c.stderr)
            );
        } else {
            assert_eq!(
                c.status.code(),
                Some(0),
                "{tag}: a handled alg must NOT abort (stdout: {})",
                String::from_utf8_lossy(&c.stdout)
            );
        }
    }
}

// ===========================================================================
// documentation row
// ===========================================================================

/// ERRORS rows that are **dead code** or **not constructible** in this build.
/// Listed explicitly so that no `## G2` row is silently dropped, together with
/// the reason and (where applicable) an assertion that pins the claim.
///
/// * **G2-010** — the `return -1` after `sodium_misuse()` in
///   `crypto_pwhash_str_alg`. `sodium_misuse()` always `abort()`s (or `exit()`s
///   from the installed handler), so the statement is unreachable. Covered
///   indirectly by `misuse_paths_match`, which shows the function never returns.
/// * **G2-015 / G2-038 / G2-157** — `outlen > BYTES_MAX` in
///   `crypto_pwhash_argon2i` / `_argon2id` / `_scryptsalsa208sha256`. All three
///   run an unconditional `memset(out, 0, outlen)` *before* the check, so
///   triggering the check requires a real 4 GiB (argon2) or 128 GiB (scrypt)
///   writable buffer. The *ordering* is pinned below by showing that a
///   too-small `outlen` still zeroes the buffer.
/// * **G2-022 / G2-033 / G2-055 / G2-089 / G2-090** — `x < 0U` comparisons on
///   unsigned values (`PASSWD_MIN` is `0`), and `pwdlen < ARGON2_MIN_PWD_LENGTH`
///   / `> ARGON2_MAX_PWD_LENGTH` on a `uint32_t`. Unsatisfiable.
/// * **G2-037** — `str` not NUL-terminated inside 128 bytes.
///   `argon2_verify` calls unbounded `strlen`, so this is out-of-bounds *reads*,
///   i.e. undefined behaviour, not a defined rejection. Deliberately not tested.
/// * **G2-072** — `malloc(hashlen)` in `argon2_hash` returning NULL. Only
///   reachable for `hashlen` close to `ARGON2_MAX_OUTLEN`, where Linux
///   overcommit satisfies the request and the run then produces a 4 GiB digest.
///   Needs allocator-failure injection.
/// * **G2-077** — `strlen(encoded) > UINT32_MAX` in `argon2_verify`: needs a
///   4 GiB NUL-terminated string.
/// * **G2-078 / G2-079** — the four `malloc(strlen(encoded))` calls in
///   `argon2_verify` returning NULL. `strlen(encoded)` is at most ~128 here.
/// * **G2-087 / G2-093 / G2-096 / G2-099 / G2-103 / G2-106 / G2-124 / G2-127 /
///   G2-130 / G2-134** — `x > 0xFFFFFFFF` tests on `uint32_t` fields.
/// * **G2-095 / G2-098** — `secretlen`/`adlen` `< 0U`.
/// * **G2-112** — `allocate_memory(region == NULL)`. `allocate_memory` is
///   `static` and its only caller passes `&instance->region`.
/// * **G2-113 (overflow half)** — `sizeof(block) * m_cost` overflowing: on
///   64-bit `1024 * 0xFFFFFFFF` fits in `size_t`. The `m_cost == 0` half *is*
///   tested (`argon2_initialize_rejections`).
/// * **G2-114** — `malloc(sizeof(block_region))` (24 bytes) returning NULL.
/// * **G2-115 (wrap half)** — `memory_size + 63 < memory_size`:
///   `memory_size <= 2^42`, so it never wraps. The malloc-failure half is
///   tested.
/// * **G2-124 / G2-127 / G2-130** — `argon2_decode_string`'s
///   `ctx->m_cost/t_cost/lanes > UINT32_MAX` after the assignment to a
///   `uint32_t`.
/// * **G2-154 / G2-155** — an inner `crypto_generichash_blake2b_*` call inside
///   `blake2b_long` returning `< 0`. For `1 <= outlen <= UINT32_MAX` every
///   inner call is made with a legal digest length, so none can fail.
/// * **G2-156** — `passwdlen > SODIUM_SIZE_MAX` for an `unsigned long long`.
/// * **G2-159 / G2-163 (second half) / G2-171 / G2-215** — `pickparams()`
///   ends in an unconditional `return 0`, so every `pickparams(...) != 0`
///   guard is dead. Pinned below.
/// * **G2-162 / G2-163 (first half)** — `passwdlen` above/below the scrypt
///   `PASSWD_MAX`/`PASSWD_MIN` (`2^64-1` / `0`).
/// * **G2-164** — `escrypt_gensalt_r` failing inside
///   `crypto_pwhash_scryptsalsa208sha256_str`: `pickparams` can only produce
///   `N_log2 <= 63` and `r * p <= 8 * 134217727 < 2^30`. Pinned below.
/// * **G2-165 / G2-168 / G2-195** — `escrypt_init_local()` failing: it is
///   `init_region()` + `return 0`.
/// * **G2-196 / G2-209 / G2-213 / G2-222** — `munmap` failures. Compiled out
///   without `HAVE_MMAP`/`MAP_ANON`; the portable branch uses `free()`.
/// * **G2-183** — `need < saltlen` in `escrypt_r`: `need = prefixlen +
///   saltlen + 45`, and `saltlen` comes from `strlen`, so the sum cannot wrap.
/// * **G2-185 / G2-190 / G2-191 / G2-192 / G2-193 / G2-194** — the
///   "Can't happen" `encode64*` failures: the `need > buflen` guard already
///   reserves every byte those helpers write.
/// * **G2-205** — `r > SIZE_MAX / 128 / p` in `escrypt_kdf_nosse`. Combined
///   with the earlier `r * p < 2^30` test this needs `p > 1.4e17`, which does
///   not fit in `uint32_t`. Pinned below.
/// * **G2-216** — the `memory_blocks < 2 * ARGON2_SYNC_POINTS * lanes` clamp in
///   `argon2_ctx`: `argon2_validate_inputs` already rejects
///   `m_cost < 8 * lanes`. Pinned below.
/// * **G2-219 / G2-220** — the NULL guards of the `static`
///   `argon2_initial_hash` and `generate_addresses`. Not exported, and their
///   only callers pass non-NULL pointers.
/// * **G2-223** — documentation row: there are no `assert()` calls in
///   `crypto_pwhash/`. Pinned below.
#[test]
fn documented_dead_and_unreachable_rows() {
    setup();

    // --- G2-015 / G2-038 / G2-157 ordering proof -------------------------
    // The leading memset runs before the range check, which is exactly why the
    // `outlen > BYTES_MAX` rows need a multi-GiB buffer.
    for (name, saltlen) in [
        ("crypto_pwhash_argon2i", 16usize),
        ("crypto_pwhash_argon2id", 16),
    ] {
        let (cf, rf) = pair::<Pwhash>(name);
        for f in [cf, rf] {
            let mut out = canary(64);
            let pw = [0x61u8; 8];
            let salt = vec![0x62u8; saltlen];
            let rc = unsafe {
                f(
                    out.as_mut_ptr(),
                    48,
                    pw.as_ptr() as *const c_char,
                    8,
                    salt.as_ptr(),
                    0, // opslimit below MIN -> rejected AFTER the memset
                    8192,
                    if name.ends_with("2i") { 1 } else { 2 },
                )
            };
            assert_eq!(rc, -1);
            assert_eq!(&out[..48], &vec![0u8; 48][..], "{name}: memset runs first");
            assert_eq!(&out[48..], &canary(16)[..]);
        }
    }
    let (cf, rf) = pair::<Scrypt>("crypto_pwhash_scryptsalsa208sha256");
    for f in [cf, rf] {
        let mut out = canary(64);
        let pw = [0x61u8; 8];
        let salt = [0x62u8; 32];
        let rc = unsafe {
            f(
                out.as_mut_ptr(),
                15, // below BYTES_MIN -> rejected after the memset
                pw.as_ptr() as *const c_char,
                8,
                salt.as_ptr(),
                32768,
                16777216,
            )
        };
        assert_eq!(rc, -1);
        assert_eq!(&out[..15], &vec![0u8; 15][..]);
    }

    // --- G2-159 / G2-171 / G2-215: pickparams never fails ----------------
    // If `pickparams` could fail, `_str_needs_rehash` would return -1/EINVAL
    // for *some* (opslimit, memlimit). Sweep the whole interesting range and
    // show that the only -1 comes from the string checks, never from
    // pickparams.
    let (cnr, rnr) = pair::<PwNeedsRehash>("crypto_pwhash_scryptsalsa208sha256_str_needs_rehash");
    let good = scrypt_string(&setting14(10, 8, 1));
    let mut rng = Rng::new(0x1300_0001);
    for i in 0..200 {
        let ops = if i < 8 {
            [0u64, 1, 32767, 32768, 4294967295, 4294967296, u64::MAX, 1 << 40][i]
        } else {
            rng.next_u64()
        };
        let mem = if i < 8 {
            [0usize, 1, 1024, 16777216, 68719476736, 68719476737, usize::MAX, 1 << 40][i]
        } else {
            rng.next_u64() as usize
        };
        set_errno(SENTINEL);
        let a = unsafe { cnr(good.as_ptr() as *const c_char, ops, mem) };
        let ea = get_errno();
        set_errno(SENTINEL);
        let b = unsafe { rnr(good.as_ptr() as *const c_char, ops, mem) };
        let eb = get_errno();
        eq_i32(&format!("scrypt needs_rehash(ops={ops}, mem={mem})"), a, b);
        assert_eq!(ea, eb);
        assert!(
            a == 0 || a == 1,
            "pickparams must never fail (ops={ops}, mem={mem} -> {a})"
        );
    }

    // --- G2-164: pickparams can never make escrypt_gensalt_r fail --------
    // Replicate `pickparams` and check the two gensalt guards over a wide
    // sweep of (opslimit, memlimit).
    let pickparams = |mut opslimit: u64, memlimit: usize| -> (u32, u32, u32) {
        if opslimit < 32768 {
            opslimit = 32768;
        }
        let r: u32 = 8;
        let mut n_log2: u32;
        let p: u32;
        if opslimit < (memlimit as u64) / 32 {
            p = 1;
            let max_n = opslimit / (r as u64 * 4);
            n_log2 = 1;
            while n_log2 < 63 {
                if 1u64.checked_shl(n_log2).unwrap_or(u64::MAX) > max_n / 2 {
                    break;
                }
                n_log2 += 1;
            }
        } else {
            let max_n = (memlimit as u64) / (r as u64 * 128);
            n_log2 = 1;
            while n_log2 < 63 {
                if 1u64.checked_shl(n_log2).unwrap_or(u64::MAX) > max_n / 2 {
                    break;
                }
                n_log2 += 1;
            }
            let mut maxrp = (opslimit / 4) / (1u64 << n_log2);
            if maxrp > 0x3fffffff {
                maxrp = 0x3fffffff;
            }
            p = (maxrp as u32) / r;
        }
        (n_log2, r, p)
    };
    let mut rng = Rng::new(0x1300_0002);
    for _ in 0..2000 {
        let ops = rng.next_u64();
        let mem = rng.next_u64() as usize;
        let (n_log2, r, p) = pickparams(ops, mem);
        assert!(n_log2 <= 63, "pickparams N_log2 = {n_log2}");
        assert!(
            (r as u64) * (p as u64) < (1u64 << 30),
            "pickparams r*p = {} (ops={ops}, mem={mem})",
            r as u64 * p as u64
        );
    }

    // --- G2-205: r > SIZE_MAX / 128 / p is unreachable on 64-bit ---------
    // With `r * p < 2^30` already enforced, `r > SIZE_MAX / 128 / p` needs
    // `p > SIZE_MAX / 128 / r >= SIZE_MAX / 128 / 2^30`, which exceeds
    // `u32::MAX`.
    // Formally: the branch needs `r > floor(SIZE_MAX/128/p)`, i.e.
    // `r >= floor(SIZE_MAX/128/p) + 1`, hence `r * p > SIZE_MAX/128`. But the
    // earlier `r * p >= 2^30` test already returned, so `r * p < 2^30`. The two
    // are contradictory exactly when `SIZE_MAX/128 >= 2^30`, which holds on
    // every 64-bit target.
    assert!(
        usize::MAX / 128 >= (1usize << 30),
        "the r > SIZE_MAX/128/p branch must be unreachable"
    );

    // --- G2-216: the argon2_ctx memory_blocks clamp is unreachable -------
    // `argon2_validate_inputs` rejects every `m_cost < 8 * lanes`, so the
    // clamp in `argon2_ctx` can never fire.
    let (cv, rv) = pair::<Argon2Validate>("_sodium_argon2_validate_inputs");
    let mut out = [0u8; 32];
    let mut pwd = [0u8; 8];
    let mut salt = [0u8; 16];
    for lanes in [1u32, 2, 3, 4, 7, 8, 16, 64] {
        for m in 0..(8 * lanes) {
            let mut ctx = Ctx::zeroed();
            ctx.out = out.as_mut_ptr();
            ctx.outlen = 32;
            ctx.pwd = pwd.as_mut_ptr();
            ctx.pwdlen = 8;
            ctx.salt = salt.as_mut_ptr();
            ctx.saltlen = 16;
            ctx.lanes = lanes;
            ctx.threads = lanes;
            ctx.m_cost = m;
            ctx.t_cost = 1;
            let a = unsafe { cv(&raw const ctx) };
            let b = unsafe { rv(&raw const ctx) };
            eq_i32(&format!("validate(lanes={lanes}, m_cost={m})"), a, b);
            assert_eq!(
                a, ARGON2_MEMORY_TOO_LITTLE,
                "m_cost = {m} < 8 * {lanes} must be rejected"
            );
        }
    }

    // --- G2-165 / G2-168 / G2-195 / G2-196: init/free_local never fail ----
    let (ci, ri) = pair::<ERegion1>("_sodium_escrypt_init_local");
    let (cl, rl) = pair::<ERegion1>("_sodium_escrypt_free_local");
    let (cfr, rfr) = pair::<ERegion1>("_sodium_escrypt_free_region");
    for (init, freel, freer) in [(ci, cl, cfr), (ri, rl, rfr)] {
        let mut l = Region {
            base: 0x11 as *mut c_void,
            aligned: 0x22 as *mut c_void,
            size: 99,
        };
        assert_eq!(unsafe { init(&raw mut l) }, 0);
        assert_eq!(unsafe { freel(&raw mut l) }, 0);
        assert_eq!(unsafe { freel(&raw mut l) }, 0);
        assert_eq!(unsafe { freer(&raw mut l) }, 0);
    }

    // --- G2-223: no assert() in crypto_pwhash/ ---------------------------
    // The nearest one lives in `crypto_generichash_blake2b_final`
    // (`assert(outlen <= UINT8_MAX)`), and `blake2b_long` never asks for more
    // than 64 bytes there. Pin the constant that guarantees it.
    let bmax = sym::<FnSz>(c_lib(), "crypto_generichash_blake2b_bytes_max");
    let bmax2 = sym::<FnSz>(r_lib(), "crypto_generichash_blake2b_bytes_max");
    let (a, b) = unsafe { (bmax(), bmax2()) };
    eq_usize("crypto_generichash_blake2b_BYTES_MAX", a, b);
    assert_eq!(a, 64);

    // --- G2-022 / G2-033 / G2-055 / G2-089 / G2-090 etc. ------------------
    // `PASSWD_MIN == 0`, so `passwdlen < PASSWD_MIN` is unsatisfiable.
    for p in [
        "crypto_pwhash",
        "crypto_pwhash_argon2i",
        "crypto_pwhash_argon2id",
        "crypto_pwhash_scryptsalsa208sha256",
    ] {
        let f = sym::<FnSz>(c_lib(), &format!("{p}_passwd_min"));
        let g = sym::<FnSz>(r_lib(), &format!("{p}_passwd_min"));
        let (a, b) = unsafe { (f(), g()) };
        eq_usize(&format!("{p}_PASSWD_MIN"), a, b);
        assert_eq!(a, 0, "{p}_PASSWD_MIN must be 0 (dead lower-bound check)");
    }
    // EPERM is never used by this subtree — referenced so the constant list
    // above stays honest about which errnos appear.
    assert_ne!(EPERM, EINVAL);
}

// ===========================================================================
// exhaustive / randomised divergence hunting
//
// `argon2_decode_string`, `escrypt_parse_setting` and
// `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` never run a KDF, so
// every byte of their input can be perturbed exhaustively at essentially zero
// cost. These sweeps back the ERRORS rows above with far more inputs than the
// hand-written cases.
// ===========================================================================

/// ERRORS G2-116 … G2-147, exhaustively: every byte position of a valid
/// argon2 encoded string replaced by every one of 256 byte values, for both
/// `Argon2_i` and `Argon2_id`, plus 4000 random junk strings.
///
/// Return code, `errno`, both decoded buffers and all six decoded scalars must
/// match. `m=`/`t=`/`p=` mutations can only *shrink* the parameters here
/// (`m=8,t=1,p=1`), so no expensive configuration can be produced.
#[test]
fn argon2_decode_string_fuzz() {
    setup();
    let (cf, rf) = pair::<Argon2Decode>("_sodium_argon2_decode_string");

    let one = |s: &[u8], ty: i32| {
        let mut res = Vec::new();
        for f in [cf, rf] {
            let mut saltbuf = canary(256);
            let mut outbuf = canary(256);
            let mut ctx = Ctx::zeroed();
            ctx.salt = saltbuf.as_mut_ptr();
            ctx.saltlen = 256;
            ctx.out = outbuf.as_mut_ptr();
            ctx.outlen = 256;
            set_errno(SENTINEL);
            let rc = unsafe { f(&raw mut ctx, s.as_ptr() as *const c_char, ty) };
            let mut buf = saltbuf;
            buf.extend_from_slice(&outbuf);
            for v in [
                ctx.m_cost,
                ctx.t_cost,
                ctx.lanes,
                ctx.threads,
                ctx.saltlen,
                ctx.outlen,
            ] {
                buf.extend_from_slice(&v.to_le_bytes());
            }
            res.push(Obs {
                rc,
                errno: get_errno(),
                buf,
            });
        }
        eq_obs(
            &format!("decode_fuzz ty={ty} {:?}", String::from_utf8_lossy(s)),
            &res[0],
            &res[1],
        );
    };

    // ---- exhaustive single-byte replacement -----------------------------
    for (ty, base) in [
        (
            ARGON2_ID,
            &b"$argon2id$v=19$m=8,t=1,p=1$YWJjZGVmZ2g$YWJjZGVmZ2hpamtsbW5vcA"[..],
        ),
        (
            ARGON2_I,
            &b"$argon2i$v=19$m=8,t=1,p=1$YWJjZGVmZ2g$YWJjZGVmZ2hpamtsbW5vcA"[..],
        ),
    ] {
        // the unmodified string decodes
        one(&nul(base), ty);
        for pos in 0..base.len() {
            for b in 0u16..=255 {
                let mut m = base.to_vec();
                m[pos] = b as u8;
                // a NUL truncates the string, which is a legitimate input
                one(&nul(&m), ty);
                // and feed each mutant to the *other* type too
                if b % 37 == 0 {
                    one(&nul(&m), if ty == ARGON2_I { ARGON2_ID } else { ARGON2_I });
                }
            }
        }
        // ---- byte insertion / deletion at every position ----------------
        for pos in 0..=base.len() {
            let mut del = base.to_vec();
            if pos < del.len() {
                del.remove(pos);
                one(&nul(&del), ty);
            }
            for b in [b'$', b',', b'=', b'0', b'9', b'A', b'!', b'\n'] {
                let mut ins = base.to_vec();
                ins.insert(pos, b);
                one(&nul(&ins), ty);
            }
        }
    }

    // ---- random junk ---------------------------------------------------
    const ALPHA: &[u8] = b"$argon2idv=,mtp0123456789./AZazXY!\n\t -+";
    let mut rng = Rng::new(0x1301_0001);
    for _ in 0..4000 {
        let n = rng.below(48);
        let mut s: Vec<u8> = (0..n).map(|_| *rng.pick(ALPHA)).collect();
        s.push(0);
        one(&s, ARGON2_ID);
        one(&s, ARGON2_I);
    }
    // ---- random junk that keeps a valid header, so the tail is exercised
    for _ in 0..2000 {
        let mut s = b"$argon2id$v=19$m=8,t=1,p=1$".to_vec();
        let n = rng.below(40);
        s.extend((0..n).map(|_| *rng.pick(b"ABCabc012$=.!/+")));
        s.push(0);
        one(&s, ARGON2_ID);
    }
    // ---- random types --------------------------------------------------
    for _ in 0..200 {
        let ty = rng.next_u32() as i32;
        one(&nul(b"$argon2id$v=19$m=8,t=1,p=1$YWJjZGVmZ2g$YWJjZGVmZ2hpamtsbW5vcA"), ty);
    }
}

/// ERRORS G2-174 … G2-179, exhaustively: every one of the 14 setting bytes of
/// `escrypt_parse_setting` replaced by every one of 256 byte values. The
/// returned pointer offset, all three out-parameters and `errno` must match.
#[test]
fn escrypt_parse_setting_fuzz() {
    setup();
    let (cf, rf) = pair::<EParse>("_sodium_escrypt_parse_setting");

    let one = |s: &[u8], tag: &str| {
        let mut res = Vec::new();
        for f in [cf, rf] {
            let (mut n, mut r, mut p) = (0xDEADBEEFu32, 0xDEADBEEFu32, 0xDEADBEEFu32);
            set_errno(SENTINEL);
            let ret = unsafe { f(s.as_ptr(), &raw mut n, &raw mut r, &raw mut p) };
            let off = if ret.is_null() {
                -1i64
            } else {
                (unsafe { ret.offset_from(s.as_ptr()) }) as i64
            };
            res.push((off, n, r, p, get_errno()));
        }
        assert_eq!(
            res[0], res[1],
            "parse_setting_fuzz[{tag}] {:?}: C={:?} Rust={:?}",
            String::from_utf8_lossy(s),
            res[0],
            res[1]
        );
    };

    // A 40-byte buffer, so a `'\0'` in a parameter position (which
    // `decode64_one` accepts, because `strchr` finds the terminator) still
    // leaves the following reads in bounds.
    let base = {
        let mut v = setting14(10, 8, 1);
        v.extend(std::iter::repeat_n(b'.', 26));
        v
    };
    one(&nul(&base), "base");
    for pos in 0..14 {
        for b in 0u16..=255 {
            let mut m = base.clone();
            m[pos] = b as u8;
            one(&nul(&m), &format!("pos={pos} b={b:#x}"));
        }
    }
    // every (N_log2, r, p) that pickparams can emit, plus random triples
    let mut rng = Rng::new(0x1301_0002);
    for n_log2 in 0u32..64 {
        let mut v = setting14(n_log2, 8, 1);
        v.extend(std::iter::repeat_n(b'.', 26));
        one(&nul(&v), &format!("N_log2={n_log2}"));
    }
    for _ in 0..2000 {
        let n_log2 = rng.below(64) as u32;
        let r = rng.next_u32() & 0x3fff_ffff;
        let p = rng.next_u32() & 0x3fff_ffff;
        let mut v = setting14(n_log2, r, p);
        v.extend(std::iter::repeat_n(b'.', 26));
        one(&nul(&v), &format!("rand N={n_log2} r={r} p={p}"));
    }
    // short buffers: a '\0' right after "$7$" and after each parameter char
    for cut in 3..14 {
        let mut v = base[..cut].to_vec();
        // pad with NULs so the reads stay inside our allocation
        v.extend(std::iter::repeat_n(0u8, 32));
        one(&v, &format!("cut={cut}"));
    }
}

/// ERRORS G2-172, G2-173 exhaustively, and the `pickparams` sweep of G2-171:
/// every byte of a scrypt setting perturbed, and the `str` length walked over
/// its whole 0..=103 range. `_str_needs_rehash` never hashes, so this is free.
#[test]
fn scrypt_needs_rehash_fuzz() {
    setup();
    let (cf, rf) = pair::<PwNeedsRehash>("crypto_pwhash_scryptsalsa208sha256_str_needs_rehash");

    let one = |s: &[u8], ops: u64, mem: usize, tag: &str| {
        let mut res = Vec::new();
        for f in [cf, rf] {
            set_errno(SENTINEL);
            let rc = unsafe { f(s.as_ptr() as *const c_char, ops, mem) };
            res.push((rc, get_errno()));
        }
        assert_eq!(
            res[0], res[1],
            "scrypt_needs_rehash_fuzz[{tag}]: C={:?} Rust={:?}",
            res[0], res[1]
        );
    };

    let good = scrypt_string(&setting14(10, 8, 1));
    // every byte position x every byte value
    for pos in 0..101 {
        for b in 0u16..=255 {
            let mut m = good[..101].to_vec();
            m[pos] = b as u8;
            m.push(0);
            one(&m, 32768, 16777216, &format!("pos={pos} b={b:#x}"));
        }
    }
    // every length from 0 to 103 (the guard wants exactly 101)
    for len in 0..=103usize {
        let mut m: Vec<u8> = (0..len).map(|i| good[i % 101]).collect();
        m.push(0);
        one(&m, 32768, 16777216, &format!("len={len}"));
    }
    // a 101-char string with no NUL inside the first 102 bytes
    let mut noterm = good[..101].to_vec();
    noterm.extend_from_slice(b"ABCDEF\0");
    one(&noterm, 32768, 16777216, "no NUL at 101");

    // random (opslimit, memlimit) against a fixed valid setting
    let mut rng = Rng::new(0x1301_0003);
    for _ in 0..3000 {
        let ops = match rng.below(4) {
            0 => rng.next_u64(),
            1 => rng.below(1 << 20) as u64,
            2 => 32768 + rng.below(1 << 24) as u64,
            _ => (rng.next_u32() as u64) << rng.below(33),
        };
        let mem = match rng.below(4) {
            0 => rng.next_u64() as usize,
            1 => rng.below(1 << 28),
            2 => rng.below(1 << 40),
            _ => (rng.next_u32() as usize) << rng.below(33),
        };
        one(&good, ops, mem, &format!("ops={ops} mem={mem}"));
    }
}
