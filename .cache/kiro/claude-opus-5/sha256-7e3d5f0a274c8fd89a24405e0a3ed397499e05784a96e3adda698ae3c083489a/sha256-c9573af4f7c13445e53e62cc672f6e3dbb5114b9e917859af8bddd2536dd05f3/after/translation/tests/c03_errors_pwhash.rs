//! Phase C — ERROR-PATH differential tests, ERRORS.md rows 82–130
//! (`crypto_pwhash` — argon2i / argon2id / scrypt).
//!
//! For every invalid-input condition we construct that exact condition, drive
//! BOTH the C `.so` and the Rust `.so`, and assert they agree on the observable
//! error surface: identical integer return / NULL sentinel / errno, or — for
//! the `sodium_misuse()`→`abort()` path (row 83) — the identical process fate
//! in a forked child (`Fate::Signaled(6)` for `SIGABRT`).
//!
//! Ground truth is the C source in
//! `c_src/libsodium/crypto_pwhash/**`. Exact triggers and errno values were
//! read out of:
//!   * `crypto_pwhash/crypto_pwhash.c` (dispatchers, rows 82–85)
//!   * `crypto_pwhash/argon2/pwhash_argon2i.c` / `pwhash_argon2id.c`
//!     (rows 86–106, and `_needs_rehash` row 97)
//!   * `crypto_pwhash/argon2/argon2-encoding.c` (`decode_decimal`, row 107)
//!   * `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c`
//!     (rows 108–116)
//!   * `crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c`
//!     (`escrypt_parse_setting`, rows 117–120; `escrypt_gensalt_r`, row 130)
//!   * `crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c`
//!     (`escrypt_kdf_nosse`, the `_ll` validation, rows 121–128)
//!   * `crypto_pwhash/scryptsalsa208sha256/pbkdf2-sha256.c` (row 129)
//!
//! RUNTIME: every row below is a REJECTION path that returns before doing any
//! real argon2/scrypt work, so they are fast. The two places that need a VALID
//! hash string to then corrupt (rows 96/106/107 and 113) generate it ONCE with
//! the smallest legal parameters via `OnceLock` and reuse it.
//!
//! errno numbers on Linux: EINVAL=22, ERANGE=34, ENOMEM=12, EFBIG=27.

mod common;
use common::*;

use std::sync::OnceLock;

// ---- errno numbers on Linux (per task spec), referenced in notes/asserts ----
const _EINVAL: i32 = 22;
const _ENOMEM: i32 = 12;
const _EFBIG: i32 = 27;

// ---- algorithm identifiers (crypto_pwhash_argon2*.h) ----
const ALG_ARGON2I13: i32 = 1;
const ALG_ARGON2ID13: i32 = 2;

// ---- exact C signatures (include/sodium/crypto_pwhash*.h) ----
// crypto_pwhash / _argon2i / _argon2id:
//   (out, outlen(ull), passwd, passwdlen(ull), salt, opslimit(ull), memlimit(size_t), alg)
type PwhashAlg =
    unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8, u64, usize, i32) -> i32;
// crypto_pwhash_scryptsalsa208sha256:
//   (out, outlen(ull), passwd, passwdlen(ull), salt, opslimit(ull), memlimit(size_t))
type PwhashScrypt = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8, u64, usize) -> i32;
// crypto_pwhash_scryptsalsa208sha256_ll:
//   (passwd, passwdlen, salt, saltlen, N(uint64), r(uint32), p(uint32), buf, buflen)
type PwhashScryptLl =
    unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, u32, u32, *mut u8, usize) -> i32;
// crypto_pwhash_str / _argon2*_str / _scrypt_str: (out, passwd, passwdlen, opslimit, memlimit)
type PwhashStr = unsafe extern "C" fn(*mut u8, *const u8, u64, u64, usize) -> i32;
// crypto_pwhash_str_alg: (out, passwd, passwdlen, opslimit, memlimit, alg)
type PwhashStrAlg = unsafe extern "C" fn(*mut u8, *const u8, u64, u64, usize, i32) -> i32;
// *_str_verify: (str, passwd, passwdlen)
type PwhashStrVerify = unsafe extern "C" fn(*const u8, *const u8, u64) -> i32;
// *_str_needs_rehash: (str, opslimit, memlimit)
type PwhashNeedsRehash = unsafe extern "C" fn(*const u8, u64, usize) -> i32;

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

/// Call a two-library return-plus-errno pair: clear errno, call C, record;
/// clear errno, call Rust, record; assert BOTH return value AND errno match.
#[track_caller]
fn assert_same_ret_errno(what: &str, cf: impl Fn() -> i32, rf: impl Fn() -> i32) {
    let (rc, ec) = with_errno(&cf);
    let (rr, er) = with_errno(&rf);
    eq_i32(&format!("{what} [return]"), rc, rr);
    assert_eq!(ec, er, "{what} [errno]: C errno {ec} != Rust errno {er}");
}

/// Byte-with-NUL of a Rust string for passing as `const char *`.
fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

const SMALL_MEM: usize = 8192; // MEMLIMIT_MIN for argon2

/// Generate ONE valid argon2i hash string (smallest legal params: opslimit = 3
/// (MIN), memlimit = 8192 (MIN)) via the C `.so`, reused across tests.
fn valid_argon2i_str() -> &'static [u8] {
    static S: OnceLock<Vec<u8>> = OnceLock::new();
    S.get_or_init(|| {
        let d = duo();
        let (cf, _) = d.pair::<PwhashStr>("crypto_pwhash_argon2i_str");
        let pw = cstr("pw");
        let mut out = [0u8; 128];
        let rc = unsafe { cf(out.as_mut_ptr(), pw.as_ptr(), 2, 3, SMALL_MEM) };
        assert_eq!(rc, 0, "valid_argon2i_str: C _str failed");
        out.to_vec()
    })
    .as_slice()
}

/// Generate ONE valid argon2id hash string (smallest legal params: opslimit = 1
/// (MIN), memlimit = 8192 (MIN)) via the C `.so`, reused across tests.
fn valid_argon2id_str() -> &'static [u8] {
    static S: OnceLock<Vec<u8>> = OnceLock::new();
    S.get_or_init(|| {
        let d = duo();
        let (cf, _) = d.pair::<PwhashStr>("crypto_pwhash_argon2id_str");
        let pw = cstr("pw");
        let mut out = [0u8; 128];
        let rc = unsafe { cf(out.as_mut_ptr(), pw.as_ptr(), 2, 1, SMALL_MEM) };
        assert_eq!(rc, 0, "valid_argon2id_str: C _str failed");
        out.to_vec()
    })
    .as_slice()
}

/// Generate ONE valid scrypt hash string via the C `.so`, reused across the
/// scrypt verify tests. pickparams clamps opslimit up to 32768 and derives a
/// tiny N from the small memlimit, so this is fast.
fn valid_scrypt_str() -> &'static [u8] {
    static S: OnceLock<Vec<u8>> = OnceLock::new();
    S.get_or_init(|| {
        let d = duo();
        let (cf, _) = d.pair::<PwhashStr>("crypto_pwhash_scryptsalsa208sha256_str");
        let pw = cstr("pw");
        let mut out = [0u8; 102];
        let rc = unsafe { cf(out.as_mut_ptr(), pw.as_ptr(), 2, 32768, 16384) };
        assert_eq!(rc, 0, "valid_scrypt_str: C _str failed");
        out.to_vec()
    })
    .as_slice()
}

// ===========================================================================
// crypto_pwhash.c dispatchers — rows 82–85.
// ===========================================================================

/// ERRORS.md row 82 (and part of 297): `crypto_pwhash` with `alg` not in
/// {1,2} — incl. 0, 3, −1, INT_MAX — hits the `default:` arm →
/// `errno = EINVAL; return -1`. Both `.so`s must return -1 with errno EINVAL.
#[test]
fn pwhash_bad_alg_einval() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashAlg>("crypto_pwhash");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("password");
    let salt = [0u8; 16];
    for &alg in &[0i32, 3, -1, i32::MAX] {
        let call = |f: PwhashAlg| {
            let mut out = [0u8; 32];
            unsafe {
                f(
                    out.as_mut_ptr(),
                    32,
                    pw.as_ptr(),
                    8,
                    salt.as_ptr(),
                    3,
                    SMALL_MEM,
                    alg,
                )
            }
        };
        assert_same_ret_errno(
            &format!("crypto_pwhash bad alg={alg}"),
            || call(cf),
            || call(rf),
        );
        assert_eq!(call(cf), -1);
    }
}

/// ERRORS.md row 83 (and part of 297): `crypto_pwhash_str_alg` with `alg` not
/// in {1,2} falls through the switch to `sodium_misuse()` → `abort()`. With no
/// misuse handler installed both `.so`s must abort with SIGABRT in a forked
/// child (`Fate::Signaled(6)`).
#[test]
fn pwhash_str_alg_bad_alg_aborts() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashStrAlg>("crypto_pwhash_str_alg");
    let cf = *cf;
    let rf = *rf;
    for &alg in &[0i32, 3, -1, i32::MAX] {
        let pw = cstr("password");
        same_fate(
            &format!("crypto_pwhash_str_alg bad alg={alg}"),
            {
                let pw = pw.clone();
                move || {
                    let mut out = [0u8; 128];
                    let _ = unsafe { cf(out.as_mut_ptr(), pw.as_ptr(), 8, 3, SMALL_MEM, alg) };
                }
            },
            move || {
                let mut out = [0u8; 128];
                let _ = unsafe { rf(out.as_mut_ptr(), pw.as_ptr(), 8, 3, SMALL_MEM, alg) };
            },
        );
    }
}

/// ERRORS.md row 84: `crypto_pwhash_str_verify` with a `str` whose prefix is
/// neither `$argon2id$` nor `$argon2i$` → `errno = EINVAL; return -1`.
#[test]
fn pwhash_str_verify_bad_prefix_einval() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashStrVerify>("crypto_pwhash_str_verify");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("password");
    for bad in &["$scrypt$xyz", "not-a-hash", "$argon2xd$junk", "$argon2$", ""] {
        let s = cstr(bad);
        assert_same_ret_errno(
            &format!("crypto_pwhash_str_verify prefix {bad:?}"),
            || unsafe { cf(s.as_ptr(), pw.as_ptr(), 8) },
            || unsafe { rf(s.as_ptr(), pw.as_ptr(), 8) },
        );
    }
}

/// ERRORS.md row 85: `crypto_pwhash_str_needs_rehash` with an unrecognised
/// prefix → `errno = EINVAL; return -1`.
#[test]
fn pwhash_str_needs_rehash_bad_prefix_einval() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashNeedsRehash>("crypto_pwhash_str_needs_rehash");
    let cf = *cf;
    let rf = *rf;
    for bad in &["$scrypt$xyz", "not-a-hash", "$argon2xd$junk", ""] {
        let s = cstr(bad);
        assert_same_ret_errno(
            &format!("crypto_pwhash_str_needs_rehash prefix {bad:?}"),
            || unsafe { cf(s.as_ptr(), 3, SMALL_MEM) },
            || unsafe { rf(s.as_ptr(), 3, SMALL_MEM) },
        );
    }
}

// ===========================================================================
// crypto_pwhash_argon2i — rows 86–91.
// ===========================================================================

/// ERRORS.md rows 86, 88 (reachable opslimit/memlimit forms), 89, 90, 91 for
/// `crypto_pwhash_argon2i`. Each condition returns before any argon2 work.
///
/// Row 86: `outlen < 16` (BYTES_MIN)                         → EINVAL.
/// Row 87: `outlen > BYTES_MAX(4294967295)` — UNREACHABLE on 64-bit (a >4 GiB
///         output buffer cannot be allocated); documented, not driven.
/// Row 88: `opslimit > OPSLIMIT_MAX(4294967295)` OR
///         `memlimit > MEMLIMIT_MAX`                          → EFBIG.
///         (`passwdlen > PASSWD_MAX(4294967295)` is also this EFBIG arm but a
///          >4 GiB password is impractical, so that sub-condition is documented;
///          the opslimit/memlimit forms exercise the identical arm.)
/// Row 89: `opslimit < OPSLIMIT_MIN(3)` OR `memlimit < MEMLIMIT_MIN(8192)`
///                                                            → EINVAL.
/// Row 90: `out == passwd` (aliasing)                        → EINVAL.
/// Row 91: `alg != ARGON2I13` (the `default:` arm)           → EINVAL.
#[test]
fn argon2i_reject_paths() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashAlg>("crypto_pwhash_argon2i");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("password");
    let salt = [0u8; 16];

    // Row 86: outlen < BYTES_MIN(16) -> EINVAL.
    for &outlen in &[0u64, 1, 15] {
        let call = |f: PwhashAlg| {
            let mut out = vec![0u8; outlen.max(1) as usize];
            unsafe {
                f(out.as_mut_ptr(), outlen, pw.as_ptr(), 8, salt.as_ptr(), 3, SMALL_MEM, ALG_ARGON2I13)
            }
        };
        assert_same_ret_errno(
            &format!("argon2i outlen={outlen} (<BYTES_MIN)"),
            || call(cf),
            || call(rf),
        );
    }

    // Row 88: opslimit > OPSLIMIT_MAX(4294967295) -> EFBIG.
    {
        let call = |f: PwhashAlg| {
            let mut out = [0u8; 32];
            unsafe {
                f(out.as_mut_ptr(), 32, pw.as_ptr(), 8, salt.as_ptr(), 4_294_967_296u64, SMALL_MEM, ALG_ARGON2I13)
            }
        };
        assert_same_ret_errno("argon2i opslimit>OPSLIMIT_MAX (EFBIG)", || call(cf), || call(rf));
    }
    // Row 88: memlimit > MEMLIMIT_MAX(4398046510080) -> EFBIG.
    {
        let call = |f: PwhashAlg| {
            let mut out = [0u8; 32];
            unsafe {
                f(out.as_mut_ptr(), 32, pw.as_ptr(), 8, salt.as_ptr(), 3, 4_398_046_510_081usize, ALG_ARGON2I13)
            }
        };
        assert_same_ret_errno("argon2i memlimit>MEMLIMIT_MAX (EFBIG)", || call(cf), || call(rf));
    }

    // Row 89: opslimit < OPSLIMIT_MIN(3) -> EINVAL.
    for &ops in &[0u64, 1, 2] {
        let call = |f: PwhashAlg| {
            let mut out = [0u8; 32];
            unsafe {
                f(out.as_mut_ptr(), 32, pw.as_ptr(), 8, salt.as_ptr(), ops, SMALL_MEM, ALG_ARGON2I13)
            }
        };
        assert_same_ret_errno(
            &format!("argon2i opslimit={ops} (<OPSLIMIT_MIN) EINVAL"),
            || call(cf),
            || call(rf),
        );
    }
    // Row 89: memlimit < MEMLIMIT_MIN(8192) -> EINVAL.
    for &mem in &[0usize, 1, 8191] {
        let call = |f: PwhashAlg| {
            let mut out = [0u8; 32];
            unsafe {
                f(out.as_mut_ptr(), 32, pw.as_ptr(), 8, salt.as_ptr(), 3, mem, ALG_ARGON2I13)
            }
        };
        assert_same_ret_errno(
            &format!("argon2i memlimit={mem} (<MEMLIMIT_MIN) EINVAL"),
            || call(cf),
            || call(rf),
        );
    }

    // Row 90: out == passwd (aliasing) -> EINVAL.
    {
        let call = |f: PwhashAlg| {
            let mut buf = [0u8; 32];
            let p = buf.as_mut_ptr();
            unsafe { f(p, 32, p as *const u8, 8, salt.as_ptr(), 3, SMALL_MEM, ALG_ARGON2I13) }
        };
        assert_same_ret_errno("argon2i out==passwd aliasing EINVAL", || call(cf), || call(rf));
    }

    // Row 91: alg != ARGON2I13 reaches the switch `default:` -> EINVAL.
    for &alg in &[0i32, 2, 3, -1, i32::MAX] {
        let call = |f: PwhashAlg| {
            let mut out = [0u8; 32];
            unsafe {
                f(out.as_mut_ptr(), 32, pw.as_ptr(), 8, salt.as_ptr(), 3, SMALL_MEM, alg)
            }
        };
        assert_same_ret_errno(
            &format!("argon2i alg={alg} default: EINVAL"),
            || call(cf),
            || call(rf),
        );
    }
}

// ===========================================================================
// crypto_pwhash_argon2i_str — rows 92, 93.
// ===========================================================================

/// ERRORS.md rows 92, 93 for `crypto_pwhash_argon2i_str`:
///  * Row 92: opslimit/memlimit above their *_MAX → EFBIG.
///  * Row 93: `opslimit < 3` OR `memlimit < 8192` → EINVAL.
/// Both are pre-checks; no salt is drawn, no hashing occurs.
#[test]
fn argon2i_str_limit_rejects() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashStr>("crypto_pwhash_argon2i_str");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("password");

    {
        let call = |f: PwhashStr| {
            let mut out = [0u8; 128];
            unsafe { f(out.as_mut_ptr(), pw.as_ptr(), 8, 4_294_967_296u64, SMALL_MEM) }
        };
        assert_same_ret_errno("argon2i_str opslimit>MAX (EFBIG)", || call(cf), || call(rf));
    }
    {
        let call = |f: PwhashStr| {
            let mut out = [0u8; 128];
            unsafe { f(out.as_mut_ptr(), pw.as_ptr(), 8, 3, 4_398_046_510_081usize) }
        };
        assert_same_ret_errno("argon2i_str memlimit>MAX (EFBIG)", || call(cf), || call(rf));
    }
    for &ops in &[0u64, 1, 2] {
        let call = |f: PwhashStr| {
            let mut out = [0u8; 128];
            unsafe { f(out.as_mut_ptr(), pw.as_ptr(), 8, ops, SMALL_MEM) }
        };
        assert_same_ret_errno(
            &format!("argon2i_str opslimit={ops} (<MIN) EINVAL"),
            || call(cf),
            || call(rf),
        );
    }
    for &mem in &[0usize, 8191] {
        let call = |f: PwhashStr| {
            let mut out = [0u8; 128];
            unsafe { f(out.as_mut_ptr(), pw.as_ptr(), 8, 3, mem) }
        };
        assert_same_ret_errno(
            &format!("argon2i_str memlimit={mem} (<MIN) EINVAL"),
            || call(cf),
            || call(rf),
        );
    }
}

// ===========================================================================
// crypto_pwhash_argon2i_str_verify / _needs_rehash — rows 94–97.
// ===========================================================================

/// ERRORS.md rows 94, 95, 96 for `crypto_pwhash_argon2i_str_verify`:
///  * Row 94: `passwdlen > PASSWD_MAX(4294967295)` → EFBIG (reachable: the
///    check runs before the password is read, so a huge `passwdlen` with a
///    short valid pointer triggers it).
///  * Row 95: malformed / unparsable `str` → -1 (decode-fail path; both `.so`s
///    must agree on -1 AND errno).
///  * Row 96: valid `str`, WRONG password (ARGON2_VERIFY_MISMATCH) → -1 with
///    errno EINVAL.
#[test]
fn argon2i_str_verify_rejects() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashStrVerify>("crypto_pwhash_argon2i_str_verify");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("password");

    // Row 94.
    {
        let s = cstr("$argon2i$anything");
        let call = |f: PwhashStrVerify| unsafe { f(s.as_ptr(), pw.as_ptr(), 4_294_967_296u64) };
        assert_same_ret_errno(
            "argon2i_str_verify passwdlen>PASSWD_MAX (EFBIG)",
            || call(cf),
            || call(rf),
        );
    }
    // Row 95.
    for bad in &[
        "$argon2i$garbage",
        "$argon2i$v=19$m=8,t=1,p=1$c2FsdA$bad!!",
        "$argon2i$v=19$notparams",
    ] {
        let s = cstr(bad);
        let call = |f: PwhashStrVerify| unsafe { f(s.as_ptr(), pw.as_ptr(), 8) };
        assert_same_ret_errno(
            &format!("argon2i_str_verify malformed {bad:?}"),
            || call(cf),
            || call(rf),
        );
    }
    // Row 96: valid str + WRONG password -> -1 errno EINVAL.
    {
        let s = valid_argon2i_str();
        let wrong = cstr("WRONG");
        let call = |f: PwhashStrVerify| unsafe { f(s.as_ptr(), wrong.as_ptr(), 5) };
        assert_same_ret_errno(
            "argon2i_str_verify wrong password (EINVAL)",
            || call(cf),
            || call(rf),
        );
    }
}

/// ERRORS.md row 97: `crypto_pwhash_argon2i_str_needs_rehash` rejects when
/// `opslimit > UINT32_MAX` OR `memlimit/1024 > UINT32_MAX` OR
/// `strlen(str) >= STRBYTES(128)` → `errno = EINVAL; return -1`.
#[test]
fn argon2i_str_needs_rehash_rejects() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashNeedsRehash>("crypto_pwhash_argon2i_str_needs_rehash");
    let cf = *cf;
    let rf = *rf;
    let short = cstr("$argon2i$short");

    {
        let call =
            |f: PwhashNeedsRehash| unsafe { f(short.as_ptr(), (u32::MAX as u64) + 1, SMALL_MEM) };
        assert_same_ret_errno(
            "argon2i_needs_rehash opslimit>u32::MAX EINVAL",
            || call(cf),
            || call(rf),
        );
    }
    {
        let mem = ((u32::MAX as usize) + 1) * 1024;
        let call = |f: PwhashNeedsRehash| unsafe { f(short.as_ptr(), 3, mem) };
        assert_same_ret_errno(
            "argon2i_needs_rehash memlimit/1024>u32::MAX EINVAL",
            || call(cf),
            || call(rf),
        );
    }
    {
        let long = cstr(
            &"$argon2i$"
                .chars()
                .chain(std::iter::repeat('a').take(140))
                .collect::<String>(),
        );
        let call = |f: PwhashNeedsRehash| unsafe { f(long.as_ptr(), 3, SMALL_MEM) };
        assert_same_ret_errno(
            "argon2i_needs_rehash strlen>=STRBYTES EINVAL",
            || call(cf),
            || call(rf),
        );
    }
}

// ===========================================================================
// crypto_pwhash_argon2id — rows 98–106.  (OPSLIMIT_MIN is 1, not 3.)
// ===========================================================================

/// ERRORS.md rows 98, 99 (reachable opslimit/memlimit forms), 100, 101, 102 for
/// `crypto_pwhash_argon2id`:
///  * Row 98: `outlen < 16`                                → EINVAL.
///  * Row 99: opslimit/memlimit above *_MAX                → EFBIG.
///  * Row 100: `opslimit < 1` (0) OR `memlimit < 8192`     → EINVAL.
///  * Row 101: `out == passwd`                             → EINVAL.
///  * Row 102: `alg != ARGON2ID13`                         → EINVAL.
#[test]
fn argon2id_reject_paths() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashAlg>("crypto_pwhash_argon2id");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("password");
    let salt = [0u8; 16];

    // Row 98.
    for &outlen in &[0u64, 15] {
        let call = |f: PwhashAlg| {
            let mut out = vec![0u8; outlen.max(1) as usize];
            unsafe {
                f(out.as_mut_ptr(), outlen, pw.as_ptr(), 8, salt.as_ptr(), 1, SMALL_MEM, ALG_ARGON2ID13)
            }
        };
        assert_same_ret_errno(
            &format!("argon2id outlen={outlen} (<BYTES_MIN) EINVAL"),
            || call(cf),
            || call(rf),
        );
    }
    // Row 99: opslimit>MAX.
    {
        let call = |f: PwhashAlg| {
            let mut out = [0u8; 32];
            unsafe {
                f(out.as_mut_ptr(), 32, pw.as_ptr(), 8, salt.as_ptr(), 4_294_967_296u64, SMALL_MEM, ALG_ARGON2ID13)
            }
        };
        assert_same_ret_errno("argon2id opslimit>MAX (EFBIG)", || call(cf), || call(rf));
    }
    // Row 99: memlimit>MAX.
    {
        let call = |f: PwhashAlg| {
            let mut out = [0u8; 32];
            unsafe {
                f(out.as_mut_ptr(), 32, pw.as_ptr(), 8, salt.as_ptr(), 1, 4_398_046_510_081usize, ALG_ARGON2ID13)
            }
        };
        assert_same_ret_errno("argon2id memlimit>MAX (EFBIG)", || call(cf), || call(rf));
    }
    // Row 100: opslimit=0 (<MIN=1).
    {
        let call = |f: PwhashAlg| {
            let mut out = [0u8; 32];
            unsafe {
                f(out.as_mut_ptr(), 32, pw.as_ptr(), 8, salt.as_ptr(), 0, SMALL_MEM, ALG_ARGON2ID13)
            }
        };
        assert_same_ret_errno("argon2id opslimit=0 (<MIN=1) EINVAL", || call(cf), || call(rf));
    }
    // Row 100: memlimit < 8192.
    for &mem in &[0usize, 8191] {
        let call = |f: PwhashAlg| {
            let mut out = [0u8; 32];
            unsafe {
                f(out.as_mut_ptr(), 32, pw.as_ptr(), 8, salt.as_ptr(), 1, mem, ALG_ARGON2ID13)
            }
        };
        assert_same_ret_errno(
            &format!("argon2id memlimit={mem} (<MIN) EINVAL"),
            || call(cf),
            || call(rf),
        );
    }
    // Row 101: aliasing.
    {
        let call = |f: PwhashAlg| {
            let mut buf = [0u8; 32];
            let p = buf.as_mut_ptr();
            unsafe { f(p, 32, p as *const u8, 8, salt.as_ptr(), 1, SMALL_MEM, ALG_ARGON2ID13) }
        };
        assert_same_ret_errno("argon2id out==passwd aliasing EINVAL", || call(cf), || call(rf));
    }
    // Row 102: bad alg.
    for &alg in &[0i32, 1, 3, -1, i32::MAX] {
        let call = |f: PwhashAlg| {
            let mut out = [0u8; 32];
            unsafe {
                f(out.as_mut_ptr(), 32, pw.as_ptr(), 8, salt.as_ptr(), 1, SMALL_MEM, alg)
            }
        };
        assert_same_ret_errno(
            &format!("argon2id alg={alg} default: EINVAL"),
            || call(cf),
            || call(rf),
        );
    }
}

/// ERRORS.md rows 103, 104 for `crypto_pwhash_argon2id_str`:
///  * Row 103: opslimit/memlimit above *_MAX → EFBIG.
///  * Row 104: `opslimit < 1` (0) OR `memlimit < 8192` → EINVAL.
#[test]
fn argon2id_str_limit_rejects() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashStr>("crypto_pwhash_argon2id_str");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("password");

    {
        let call = |f: PwhashStr| {
            let mut out = [0u8; 128];
            unsafe { f(out.as_mut_ptr(), pw.as_ptr(), 8, 4_294_967_296u64, SMALL_MEM) }
        };
        assert_same_ret_errno("argon2id_str opslimit>MAX (EFBIG)", || call(cf), || call(rf));
    }
    {
        let call = |f: PwhashStr| {
            let mut out = [0u8; 128];
            unsafe { f(out.as_mut_ptr(), pw.as_ptr(), 8, 1, 4_398_046_510_081usize) }
        };
        assert_same_ret_errno("argon2id_str memlimit>MAX (EFBIG)", || call(cf), || call(rf));
    }
    {
        let call = |f: PwhashStr| {
            let mut out = [0u8; 128];
            unsafe { f(out.as_mut_ptr(), pw.as_ptr(), 8, 0, SMALL_MEM) }
        };
        assert_same_ret_errno("argon2id_str opslimit=0 (<MIN) EINVAL", || call(cf), || call(rf));
    }
    for &mem in &[0usize, 8191] {
        let call = |f: PwhashStr| {
            let mut out = [0u8; 128];
            unsafe { f(out.as_mut_ptr(), pw.as_ptr(), 8, 1, mem) }
        };
        assert_same_ret_errno(
            &format!("argon2id_str memlimit={mem} (<MIN) EINVAL"),
            || call(cf),
            || call(rf),
        );
    }
}

/// ERRORS.md rows 105, 106 for `crypto_pwhash_argon2id_str_verify`:
///  * Row 105: malformed `str` → -1.
///  * Row 106: valid `str`, WRONG password → -1 with errno EINVAL.
#[test]
fn argon2id_str_verify_rejects() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashStrVerify>("crypto_pwhash_argon2id_str_verify");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("password");

    for bad in &[
        "$argon2id$garbage",
        "$argon2id$v=19$m=8,t=1,p=1$c2FsdA$bad!!",
        "$argon2id$v=19$notparams",
    ] {
        let s = cstr(bad);
        let call = |f: PwhashStrVerify| unsafe { f(s.as_ptr(), pw.as_ptr(), 8) };
        assert_same_ret_errno(
            &format!("argon2id_str_verify malformed {bad:?}"),
            || call(cf),
            || call(rf),
        );
    }
    {
        let s = valid_argon2id_str();
        let wrong = cstr("WRONG");
        let call = |f: PwhashStrVerify| unsafe { f(s.as_ptr(), wrong.as_ptr(), 5) };
        assert_same_ret_errno(
            "argon2id_str_verify wrong password (EINVAL)",
            || call(cf),
            || call(rf),
        );
    }
}

// ===========================================================================
// argon2 encoding decode_decimal rejections — row 107.
// ===========================================================================

/// ERRORS.md row 107: `decode_decimal` rejects encoded numeric params that
/// overflow `unsigned long` OR carry non-minimal leading zeros. Reached via
/// `crypto_pwhash_argon2id_str_verify` with a `$argon2id$` string whose `v=`,
/// `m=`, `t=` or `p=` field is malformed → `argon2_decode_string` returns
/// DECODING_FAIL → verify returns -1. Both `.so`s must return the same
/// -1/errno.
#[test]
fn argon2_decode_decimal_rejects() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashStrVerify>("crypto_pwhash_argon2id_str_verify");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("pw");

    let cases = [
        "$argon2id$v=19$m=08,t=1,p=1$c2FsdHNhbHQ$aGFzaGhhc2g", // leading-zero m
        "$argon2id$v=19$m=8,t=01,p=1$c2FsdHNhbHQ$aGFzaGhhc2g", // leading-zero t
        "$argon2id$v=19$m=8,t=1,p=01$c2FsdHNhbHQ$aGFzaGhhc2g", // leading-zero p
        "$argon2id$v=019$m=8,t=1,p=1$c2FsdHNhbHQ$aGFzaGhhc2g", // leading-zero v
        "$argon2id$v=19$m=99999999999999999999,t=1,p=1$c2FsdA$aGg", // overflow m
    ];
    for c in &cases {
        let s = cstr(c);
        let call = |f: PwhashStrVerify| unsafe { f(s.as_ptr(), pw.as_ptr(), 2) };
        assert_same_ret_errno(
            &format!("argon2 decode_decimal reject {c:?}"),
            || call(cf),
            || call(rf),
        );
    }
}

// ===========================================================================
// crypto_pwhash_scryptsalsa208sha256 — rows 108, 110.
// ===========================================================================

/// ERRORS.md rows 108, 110 for `crypto_pwhash_scryptsalsa208sha256`:
///  * Row 108: `outlen < 16` (BYTES_MIN) → EINVAL. (The `pickparams` failure
///    sub-condition never triggers on this build — pickparams always returns 0
///    — so it is documented unreachable; the outlen guard drives the identical
///    EINVAL.)
///  * Row 110: `out == passwd` (aliasing) → EINVAL (outlen >= 16, valid params).
///
/// Row 109 (`passwdlen > PASSWD_MAX` / `outlen > BYTES_MAX`) is documented
/// UNREACHABLE on 64-bit: PASSWD_MAX == SIZE_MAX, BYTES_MAX == 0x1fffffffe0;
/// neither can be exceeded with a real buffer and the C marks both LCOV_EXCL.
#[test]
fn scrypt_reject_paths() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashScrypt>("crypto_pwhash_scryptsalsa208sha256");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("password");
    let salt = [0u8; 32];

    for &outlen in &[0u64, 15] {
        let call = |f: PwhashScrypt| {
            let mut out = vec![0u8; outlen.max(1) as usize];
            unsafe {
                f(out.as_mut_ptr(), outlen, pw.as_ptr(), 8, salt.as_ptr(), 32768, 16384)
            }
        };
        assert_same_ret_errno(
            &format!("scrypt outlen={outlen} (<BYTES_MIN) EINVAL"),
            || call(cf),
            || call(rf),
        );
    }
    {
        let call = |f: PwhashScrypt| {
            let mut buf = [0u8; 16];
            let p = buf.as_mut_ptr();
            unsafe { f(p, 16, p as *const u8, 8, salt.as_ptr(), 32768, 16384) }
        };
        assert_same_ret_errno("scrypt out==passwd aliasing EINVAL", || call(cf), || call(rf));
    }
}

// ===========================================================================
// crypto_pwhash_scryptsalsa208sha256_str / _str_verify — rows 111–113.
// ===========================================================================

/// ERRORS.md row 111: `crypto_pwhash_scryptsalsa208sha256_str` — `pickparams`
/// never fails on this build, so this row is documented UNREACHABLE. To prove
/// the two libraries still agree we drive a boundary configuration (opslimit 0,
/// memlimit 0) and assert both SUCCEED identically (return 0), i.e. there is no
/// spurious divergent rejection.
#[test]
fn scrypt_str_pickparams_unreachable_but_agrees() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashStr>("crypto_pwhash_scryptsalsa208sha256_str");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("pw");
    let mut oc = [0u8; 102];
    let mut or = [0u8; 102];
    let rc = unsafe { cf(oc.as_mut_ptr(), pw.as_ptr(), 2, 0, 0) };
    let rr = unsafe { rf(or.as_mut_ptr(), pw.as_ptr(), 2, 0, 0) };
    eq_i32("scrypt_str tiny params agree", rc, rr);
    assert_eq!(rc, 0, "scrypt_str tiny params: expected success (row 111 unreachable)");
}

/// ERRORS.md rows 112, 113 for `crypto_pwhash_scryptsalsa208sha256_str_verify`:
///  * Row 112: `strnlen(str, STRBYTES) != STRBYTES-1 (101)` → -1 (length gate).
///  * Row 113: correctly-sized valid `str` + WRONG password → -1 (escrypt_r
///    succeeds but sodium_memcmp mismatches).
#[test]
fn scrypt_str_verify_rejects() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashStrVerify>("crypto_pwhash_scryptsalsa208sha256_str_verify");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("password");

    for bad in &["", "$7$", "tooshort"] {
        let s = cstr(bad);
        let call = |f: PwhashStrVerify| unsafe { f(s.as_ptr(), pw.as_ptr(), 8) };
        assert_same_ret_errno(
            &format!("scrypt_str_verify bad length {bad:?}"),
            || call(cf),
            || call(rf),
        );
    }
    {
        let s = valid_scrypt_str();
        let wrong = cstr("WRONG");
        let call = |f: PwhashStrVerify| unsafe { f(s.as_ptr(), wrong.as_ptr(), 5) };
        assert_same_ret_errno("scrypt_str_verify wrong password -1", || call(cf), || call(rf));
    }
}

// ===========================================================================
// crypto_pwhash_scryptsalsa208sha256_str_needs_rehash — rows 114–116.
// ===========================================================================

/// ERRORS.md rows 114, 115, 116 for
/// `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash`:
///  * Row 114: `pickparams` fails → EINVAL. pickparams never fails on this
///    build (documented UNREACHABLE); the first reachable rejection is the
///    length check (row 115).
///  * Row 115: `strnlen(str) != STRBYTES-1 (101)` → EINVAL.
///  * Row 116: correctly-sized (101-byte) string that `escrypt_parse_setting`
///    rejects → EINVAL.
#[test]
fn scrypt_str_needs_rehash_rejects() {
    let d = duo();
    let (cf, rf) =
        d.pair::<PwhashNeedsRehash>("crypto_pwhash_scryptsalsa208sha256_str_needs_rehash");
    let cf = *cf;
    let rf = *rf;

    for bad in &["", "$7$short", "way-too-long-but-not-101-chars-....."] {
        let s = cstr(bad);
        let call = |f: PwhashNeedsRehash| unsafe { f(s.as_ptr(), 32768, 16384) };
        assert_same_ret_errno(
            &format!("scrypt_needs_rehash bad length {bad:?}"),
            || call(cf),
            || call(rf),
        );
    }
    {
        let body: String = std::iter::repeat('a').take(101).collect();
        assert_eq!(body.len(), 101);
        let s = cstr(&body);
        let call = |f: PwhashNeedsRehash| unsafe { f(s.as_ptr(), 32768, 16384) };
        assert_same_ret_errno(
            "scrypt_needs_rehash 101-char non-$7$ (parse_setting fail) EINVAL",
            || call(cf),
            || call(rf),
        );
    }
}

// ===========================================================================
// escrypt_parse_setting rejections via _str_needs_rehash — rows 117–120.
// ===========================================================================

/// ERRORS.md rows 117, 118, 119, 120: `escrypt_parse_setting` rejections,
/// reached through `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash`. Each
/// setting string is EXACTLY 101 bytes (STRBYTES-1) so it passes the length
/// gate and the parser actually runs, then returns NULL → the wrapper sets
/// `errno = EINVAL` and returns -1.
///
///  * Row 117: `setting` not starting `"$7$"`.
///  * Row 118: invalid base64 char in the `N_log2` field (byte after "$7$").
///  * Row 119: invalid `r` field.
///  * Row 120: invalid `p` field.
///
/// itoa64 alphabet is "./0-9A-Za-z"; ',' is NOT in it, so it is a
/// guaranteed-invalid base64 char for decode64_one.
#[test]
fn escrypt_parse_setting_rejects() {
    let d = duo();
    let (cf, rf) =
        d.pair::<PwhashNeedsRehash>("crypto_pwhash_scryptsalsa208sha256_str_needs_rehash");
    let cf = *cf;
    let rf = *rf;

    let make101 = |prefix: &str| -> String {
        let mut s = String::from(prefix);
        while s.len() < 101 {
            s.push('.'); // '.' is itoa64[0] — valid filler.
        }
        assert_eq!(s.len(), 101);
        s
    };

    // Row 117: not starting "$7$".
    let s117 = make101("$8$");
    // Row 118: invalid N_log2 char right after "$7$".
    let s118 = make101("$7$,");
    // Row 119: valid N_log2 ('.'), invalid char in r field.
    let s119 = make101("$7$.,");
    // Row 120: valid N_log2 + 5 valid r chars, then invalid p char.
    let s120 = make101("$7$......,");

    for (name, s) in [
        ("row117 not-$7$", s117),
        ("row118 bad N_log2", s118),
        ("row119 bad r", s119),
        ("row120 bad p", s120),
    ] {
        let cs = cstr(&s);
        let call = |f: PwhashNeedsRehash| unsafe { f(cs.as_ptr(), 32768, 16384) };
        assert_same_ret_errno(
            &format!("escrypt_parse_setting {name}"),
            || call(cf),
            || call(rf),
        );
    }
}

// ===========================================================================
// crypto_pwhash_scryptsalsa208sha256_ll — rows 121–128. Tested DIRECTLY.
// ===========================================================================

/// ERRORS.md rows 121–125 for `crypto_pwhash_scryptsalsa208sha256_ll`
/// (`escrypt_kdf_nosse`). All are cheap parameter-validation rejections that
/// return before allocation / real scrypt work, so a small real output buffer
/// is safe even when claiming a huge `buflen` (row 121 checks buflen BEFORE
/// touching the buffer). The C checks run strictly in the order below, so each
/// case sets ONLY its target trigger and keeps every earlier-checked parameter
/// valid:
///  * Row 121: `buflen > (2^32-1)*32` → EFBIG.
///  * Row 122: `r*p >= 2^30`          → EFBIG.
///  * Row 123: `N > UINT32_MAX`       → EFBIG.
///  * Row 124: `N` not a power of two (3), and `N < 2` (0, 1) → EINVAL.
///  * Row 125: `r == 0` OR `p == 0`   → EINVAL.
#[test]
fn scrypt_ll_reject_paths() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashScryptLl>("crypto_pwhash_scryptsalsa208sha256_ll");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("pw");
    let salt = [0u8; 8];

    let cases: &[(u64, u32, u32, u64, &str)] = &[
        (16, 8, 1, ((1u64 << 32) - 1) * 32 + 1, "row121 buflen>(2^32-1)*32 EFBIG"),
        (16, 1 << 15, 1 << 15, 64, "row122 r*p>=2^30 EFBIG"),
        (1u64 << 32, 8, 1, 64, "row123 N>UINT32_MAX EFBIG"),
        (3, 8, 1, 64, "row124 N=3 not-pow2 EINVAL"),
        (0, 8, 1, 64, "row124 N=0 (<2) EINVAL"),
        (1, 8, 1, 64, "row124 N=1 (<2) EINVAL"),
        (16, 0, 1, 64, "row125 r=0 EINVAL"),
        (16, 8, 0, 64, "row125 p=0 EINVAL"),
    ];

    for &(n, r, p, buflen, label) in cases {
        let call = |f: PwhashScryptLl| {
            let mut buf = [0u8; 64];
            unsafe {
                f(pw.as_ptr(), 2, salt.as_ptr(), salt.len(), n, r, p, buf.as_mut_ptr(), buflen as usize)
            }
        };
        assert_same_ret_errno(label, || call(cf), || call(rf));
    }
}

// NOTE — rows 126, 127, 128 are documented UNREACHABLE on this 64-bit build and
// are NOT faked with a test:
//   * Row 126 (`N > SIZE_MAX/128/r` ENOMEM): after rows 122–124 gate r*p<2^30
//     and N<=2^32, `SIZE_MAX/128/r` (>= ~2^57 for surviving r) always exceeds
//     the maximum surviving N (<=2^32), so this ENOMEM arm is never reached on
//     LP64.
//   * Row 127 (`need < V_size` / `need < XY_size` size_t addition overflow
//     ENOMEM): row 126 catches every allocation-size overflow first on LP64, so
//     these wrap-around additions never underflow.
//   * Row 128 (region allocation failure, OOM -1): only reachable by actually
//     exhausting host memory with a legal-but-huge (N,r,p); not deterministically
//     or safely constructible, so it is documented rather than tested.

// ===========================================================================
// Rows 129, 130 — reachability documented; no faked test.
// ===========================================================================

/// ERRORS.md rows 129 and 130 are documented UNREACHABLE; this test asserts the
/// boundary that PROVES they cannot be hit via the public API:
///
///  * Row 129 (`escrypt_PBKDF2_SHA256` `dkLen > 0x1fffffffe0` → misuse): the
///    only public path to PBKDF2 with attacker-controlled dkLen is
///    `crypto_pwhash_scryptsalsa208sha256_ll`, whose FIRST guard rejects
///    `buflen > (2^32-1)*32 == 0x1fffffffe0` with EFBIG BEFORE PBKDF2 runs, so
///    dkLen can never exceed 0x1fffffffe0 inside PBKDF2. We assert `_ll` with
///    `buflen == 0x1fffffffe0 + 1` returns -1/EFBIG in BOTH libraries, i.e. the
///    misuse is unreachable rather than faking a PBKDF2 abort.
///  * Row 130 (`escrypt_gensalt_r` `N_log2 > 63 || r*p >= 2^30`): `gensalt_r`
///    is only called by `crypto_pwhash_scryptsalsa208sha256_str` with params
///    produced by `pickparams` (guaranteeing `N_log2 < 63` and small `r,p`), and
///    it is not an exported symbol, so no public entry point can feed arbitrary
///    N_log2/r/p to it. Documented, not tested; we assert it is not exported.
#[test]
fn scrypt_pbkdf2_and_gensalt_unreachable_documented() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashScryptLl>("crypto_pwhash_scryptsalsa208sha256_ll");
    let cf = *cf;
    let rf = *rf;
    let pw = cstr("pw");
    let salt = [0u8; 8];
    let call = |f: PwhashScryptLl| {
        let mut buf = [0u8; 32];
        unsafe {
            f(
                pw.as_ptr(),
                2,
                salt.as_ptr(),
                salt.len(),
                16,
                8,
                1,
                buf.as_mut_ptr(),
                (0x1fffffffe0u64 + 1) as usize,
            )
        }
    };
    assert_same_ret_errno("row129 gate: _ll buflen>0x1fffffffe0 EFBIG", || call(cf), || {
        call(rf)
    });
    assert!(
        !d.has("escrypt_gensalt_r"),
        "escrypt_gensalt_r unexpectedly exported; row 130 would then be testable"
    );
}
