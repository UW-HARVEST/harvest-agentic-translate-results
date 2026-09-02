//! Phase B + C for the password-hashing family (`crypto_pwhash/`).
//!
//! Differential tests: BOTH shared objects are loaded through libloading and
//! every call goes through the `#[no_mangle] extern "C"` exports, exactly as an
//! external C consumer would. The C at `c_src/libsodium/crypto_pwhash/` is the
//! ground truth; whenever C and Rust disagree the C wins and the assertion
//! stays.
//!
//! Scope: crypto_pwhash.c, argon2/pwhash_argon2i.c, argon2/pwhash_argon2id.c,
//! argon2/argon2.c, argon2/argon2-core.c, argon2/argon2-encoding.c,
//! scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c,
//! scryptsalsa208sha256/crypto_scrypt-common.c,
//! scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c and the four
//! public headers.
//!
//! PERFORMANCE: these are deliberately slow KDFs. Only the CHEAPEST parameters
//! are used — argon2 memlimit 8192/16384 with opslimit at each algorithm's
//! minimum (argon2i 3, argon2id 1) and minimum+1; scrypt via the low-level
//! `_ll` with N in {2,4,16,1024}, small r, p in {1,2}. INTERACTIVE limits are
//! never used. The whole file is intended to finish well under 300 s.

mod harness;
use harness::*;

use std::ffi::{c_int, c_uint};
use std::os::raw::c_char;

const SEED: u64 = 0x5EED_0011;

// Algorithm identifiers (crypto_pwhash.h).
const ALG_ARGON2I13: c_int = 1;
const ALG_ARGON2ID13: c_int = 2;

// Shared limits (from the headers, verified below at runtime too).
const BYTES_MIN: u64 = 16;
const ARGON2_MEMLIMIT_MIN: usize = 8192;
const ARGON2I_OPSLIMIT_MIN: u64 = 3;
const ARGON2ID_OPSLIMIT_MIN: u64 = 1;
const SALTBYTES: usize = 16; // argon2 SALTBYTES
const STRBYTES: usize = 128; // argon2 STRBYTES

// libc errno values we care about.
const EINVAL: c_int = 22;
const EFBIG: c_int = 27;
const ENOMEM: c_int = 12;

// ---------------------------------------------------------------------------
// Typed function-pointer aliases.
// ---------------------------------------------------------------------------

// crypto_pwhash / crypto_pwhash_argon2i / crypto_pwhash_argon2id
type PwHash = unsafe extern "C" fn(
    *mut u8,       // out
    u64,           // outlen
    *const c_char, // passwd
    u64,           // passwdlen
    *const u8,     // salt
    u64,           // opslimit
    usize,         // memlimit
    c_int,         // alg
) -> c_int;

// crypto_pwhash_str
type PwStr = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize) -> c_int;
// crypto_pwhash_str_alg
type PwStrAlg = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize, c_int) -> c_int;
// crypto_pwhash_str_verify
type PwStrVerify = unsafe extern "C" fn(*const c_char, *const c_char, u64) -> c_int;
// crypto_pwhash_str_needs_rehash
type PwNeedsRehash = unsafe extern "C" fn(*const c_char, u64, usize) -> c_int;

// scrypt high-level
type ScryptHash =
    unsafe extern "C" fn(*mut u8, u64, *const c_char, u64, *const u8, u64, usize) -> c_int;
// crypto_pwhash_scryptsalsa208sha256_ll
type ScryptLl = unsafe extern "C" fn(
    *const u8, // passwd
    usize,     // passwdlen
    *const u8, // salt
    usize,     // saltlen
    u64,       // N
    c_uint,    // r
    c_uint,    // p
    *mut u8,   // buf
    usize,     // buflen
) -> c_int;

// ---------------------------------------------------------------------------
// errno helpers. libsodium sets errno to EINVAL/EFBIG/ENOMEM before returning
// -1 on several paths; we zero it before each call and read it after.
// ---------------------------------------------------------------------------

fn clear_errno() {
    unsafe {
        *libc::__errno_location() = 0;
    }
}

fn read_errno() -> c_int {
    unsafe { *libc::__errno_location() }
}

/// Call `f` (which invokes libsodium), returning `(rc, errno)`. errno is only
/// meaningful when rc == -1, so callers compare it only in that case.
fn with_errno<F: FnOnce() -> c_int>(f: F) -> (c_int, c_int) {
    clear_errno();
    let rc = f();
    (rc, read_errno())
}

// ---------------------------------------------------------------------------
// Section 1 + 3: crypto_pwhash / _argon2i / _argon2id happy paths and every
// error gate. Return code, errno, and the full output buffer + canary are all
// asserted byte-for-byte.
// ---------------------------------------------------------------------------

struct ArgonFn {
    /// symbol name of the KDF
    name: &'static str,
    /// the algorithm id this KDF accepts as its `alg` argument
    alg: c_int,
    /// minimum opslimit for this algorithm
    ops_min: u64,
}

fn argon_fns() -> Vec<ArgonFn> {
    vec![
        ArgonFn { name: "crypto_pwhash_argon2i", alg: ALG_ARGON2I13, ops_min: ARGON2I_OPSLIMIT_MIN },
        ArgonFn { name: "crypto_pwhash_argon2id", alg: ALG_ARGON2ID13, ops_min: ARGON2ID_OPSLIMIT_MIN },
    ]
}

/// Run one argon2 call on BOTH libraries and assert rc, errno and buffer.
fn diff_argon(
    label: &str,
    f: PwHash,
    g: PwHash,
    outlen: u64,
    passwd: &[u8],
    salt: &[u8],
    opslimit: u64,
    memlimit: usize,
    alg: c_int,
) {
    // payload = outlen (may be 0), plus 16 canary bytes.
    let mut oc = out_buf(outlen as usize);
    let mut or = out_buf(outlen as usize);
    let pw = passwd.as_ptr() as *const c_char;
    let (rc, ec) = with_errno(|| unsafe {
        f(oc.as_mut_ptr(), outlen, pw, passwd.len() as u64, salt.as_ptr(), opslimit, memlimit, alg)
    });
    let (rr, er) = with_errno(|| unsafe {
        g(or.as_mut_ptr(), outlen, pw, passwd.len() as u64, salt.as_ptr(), opslimit, memlimit, alg)
    });
    assert_eq!(rc, rr, "{label}: rc C={rc} Rust={rr}");
    if rc == -1 {
        assert_eq!(ec, er, "{label}: errno C={ec} Rust={er}");
    }
    eqb(&format!("{label}: output+canary"), &oc, &or);
}

#[test]
fn pwhash_argon2_happy_matrix() {
    let mut rng = Rng::new(SEED);
    for a in argon_fns() {
        let (c, r) = sym::<PwHash>(a.name);
        // fixed random 16-byte salt from the seeded Rng
        let salt = rng.bytes(SALTBYTES);
        for &outlen in &[16u64, 17, 32, 64] {
            for &pwlen in &[0usize, 1, 8, 64, 1000] {
                let passwd = rng.bytes(pwlen);
                for &opslimit in &[a.ops_min, a.ops_min + 1] {
                    for &memlimit in &[ARGON2_MEMLIMIT_MIN, 16384usize] {
                        let label = format!(
                            "{} out={outlen} pw={pwlen} ops={opslimit} mem={memlimit} alg={}",
                            a.name, a.alg
                        );
                        diff_argon(
                            &label, c, r, outlen, &passwd, &salt, opslimit, memlimit, a.alg,
                        );
                    }
                }
            }
        }
    }
}

/// The generic `crypto_pwhash` façade dispatches on `alg` to argon2i / argon2id.
#[test]
fn crypto_pwhash_generic_happy_matrix() {
    let (c, r) = sym::<PwHash>("crypto_pwhash");
    let mut rng = Rng::new(SEED ^ 0x11);
    let salt = rng.bytes(SALTBYTES);
    for (alg, ops_min) in [(ALG_ARGON2I13, ARGON2I_OPSLIMIT_MIN), (ALG_ARGON2ID13, ARGON2ID_OPSLIMIT_MIN)] {
        for &outlen in &[16u64, 17, 32, 64] {
            for &pwlen in &[0usize, 1, 8, 64, 1000] {
                let passwd = rng.bytes(pwlen);
                for &opslimit in &[ops_min, ops_min + 1] {
                    for &memlimit in &[ARGON2_MEMLIMIT_MIN, 16384usize] {
                        let label = format!(
                            "crypto_pwhash out={outlen} pw={pwlen} ops={opslimit} mem={memlimit} alg={alg}"
                        );
                        diff_argon(
                            &label, c, r, outlen, &passwd, &salt, opslimit, memlimit, alg,
                        );
                    }
                }
            }
        }
    }
}

/// Section 3: every error gate for argon2i / argon2id / crypto_pwhash — outlen,
/// opslimit and memlimit below-min / above-max, and out==passwd aliasing.
/// Return code AND errno are both asserted. These gates all return before the
/// expensive KDF runs, so this test is cheap.
#[test]
fn pwhash_argon2_error_gates() {
    let mut rng = Rng::new(SEED ^ 0x22);
    // The generic façade routes to the same argon2i/argon2id gates.
    let mut targets: Vec<(String, c_int, u64)> = vec![
        ("crypto_pwhash_argon2i".to_string(), ALG_ARGON2I13, ARGON2I_OPSLIMIT_MIN),
        ("crypto_pwhash_argon2id".to_string(), ALG_ARGON2ID13, ARGON2ID_OPSLIMIT_MIN),
    ];
    // crypto_pwhash with each valid alg exercises the same downstream gates.
    targets.push(("crypto_pwhash".to_string(), ALG_ARGON2I13, ARGON2I_OPSLIMIT_MIN));
    targets.push(("crypto_pwhash".to_string(), ALG_ARGON2ID13, ARGON2ID_OPSLIMIT_MIN));

    // argon2 OPSLIMIT_MAX / MEMLIMIT_MAX from the headers.
    const OPS_MAX: u64 = 4294967295;
    // MEMLIMIT_MAX is huge on 64-bit; use a value comfortably above it via u64
    // wrap is not possible for size_t, so exercise the too-small memlimit gate
    // (below MIN) and the too-large opslimit gate instead for the upper bound.
    let mem_ok = ARGON2_MEMLIMIT_MIN;

    for (name, alg, ops_min) in targets {
        let (c, r) = sym::<PwHash>(&name);
        let salt = rng.bytes(SALTBYTES);
        let passwd = rng.bytes(8);

        // outlen below BYTES_MIN (0 and 15) -> EINVAL
        for &outlen in &[0u64, 1, 15] {
            diff_argon(
                &format!("{name} outlen<MIN outlen={outlen} alg={alg}"),
                c, r, outlen, &passwd, &salt, ops_min, mem_ok, alg,
            );
        }
        // outlen at exactly BYTES_MIN with a bad opslimit isolates the ops gate.
        // opslimit below min -> EINVAL
        if ops_min > 0 {
            diff_argon(
                &format!("{name} ops<MIN alg={alg}"),
                c, r, BYTES_MIN, &passwd, &salt, ops_min - 1, mem_ok, alg,
            );
        }
        // opslimit above max -> EFBIG
        diff_argon(
            &format!("{name} ops>MAX alg={alg}"),
            c, r, BYTES_MIN, &passwd, &salt, OPS_MAX + 1, mem_ok, alg,
        );
        // memlimit below min (0 and MIN-1) -> EINVAL
        for &memlimit in &[0usize, ARGON2_MEMLIMIT_MIN - 1] {
            diff_argon(
                &format!("{name} mem<MIN mem={memlimit} alg={alg}"),
                c, r, BYTES_MIN, &passwd, &salt, ops_min, memlimit, alg,
            );
        }
        // out == passwd aliasing. Build a single buffer used as both out and
        // passwd. outlen and passwdlen are legal so the aliasing gate is the
        // one that fires (-> EINVAL). Both are >= BYTES_MIN.
        {
            let mut buf = out_buf(32);
            let mut buf2 = out_buf(32);
            let (rc, ec) = with_errno(|| unsafe {
                c(
                    buf.as_mut_ptr(),
                    32,
                    buf.as_ptr() as *const c_char,
                    32,
                    salt.as_ptr(),
                    ops_min,
                    mem_ok,
                    alg,
                )
            });
            let (rr, er) = with_errno(|| unsafe {
                r(
                    buf2.as_mut_ptr(),
                    32,
                    buf2.as_ptr() as *const c_char,
                    32,
                    salt.as_ptr(),
                    ops_min,
                    mem_ok,
                    alg,
                )
            });
            assert_eq!(rc, rr, "{name} out==passwd rc C={rc} Rust={rr}");
            if rc == -1 {
                assert_eq!(ec, er, "{name} out==passwd errno C={ec} Rust={er}");
            }
            eqb(&format!("{name} out==passwd buffer"), &buf, &buf2);
        }
    }
}

// ---------------------------------------------------------------------------
// Section 2: out-of-range enum for crypto_pwhash and crypto_pwhash_str_alg.
//
//  * crypto_pwhash returns -1 / EINVAL for an unknown alg (no misuse), so a
//    plain differential compare is used.
//  * crypto_pwhash_str_alg falls through to sodium_misuse() for an unknown alg,
//    which aborts the process — so same_outcome() (forked children) is used.
// ---------------------------------------------------------------------------

const BAD_ALGS: &[c_int] = &[0, 3, 4, -1, 255, i32::MIN, i32::MAX];

#[test]
fn crypto_pwhash_bad_alg_returns_einval() {
    let (c, r) = sym::<PwHash>("crypto_pwhash");
    let mut rng = Rng::new(SEED ^ 0x33);
    let salt = rng.bytes(SALTBYTES);
    let passwd = rng.bytes(8);
    for &alg in BAD_ALGS {
        diff_argon(
            &format!("crypto_pwhash bad alg={alg}"),
            c, r, 32, &passwd, &salt, ARGON2ID_OPSLIMIT_MIN, ARGON2_MEMLIMIT_MIN, alg,
        );
    }
}

#[test]
fn crypto_pwhash_str_alg_bad_alg_aborts_identically() {
    for &alg in BAD_ALGS {
        same_outcome(
            &format!("crypto_pwhash_str_alg bad alg={alg}"),
            move || {
                let (c, _) = sym::<PwStrAlg>("crypto_pwhash_str_alg");
                let mut out = vec![0u8; STRBYTES];
                let pw = b"password";
                unsafe {
                    c(
                        out.as_mut_ptr() as *mut c_char,
                        pw.as_ptr() as *const c_char,
                        pw.len() as u64,
                        ARGON2ID_OPSLIMIT_MIN,
                        ARGON2_MEMLIMIT_MIN,
                        alg,
                    )
                }
            },
            move || {
                let (_, r) = sym::<PwStrAlg>("crypto_pwhash_str_alg");
                let mut out = vec![0u8; STRBYTES];
                let pw = b"password";
                unsafe {
                    r(
                        out.as_mut_ptr() as *mut c_char,
                        pw.as_ptr() as *const c_char,
                        pw.len() as u64,
                        ARGON2ID_OPSLIMIT_MIN,
                        ARGON2_MEMLIMIT_MIN,
                        alg,
                    )
                }
            },
        );
    }
}

// ---------------------------------------------------------------------------
// Section 4: crypto_pwhash_str / _str_alg. The encoded output embeds a random
// salt so it differs run-to-run; instead we assert:
//  (a) both libraries return the same code,
//  (b) both outputs are NUL-terminated within STRBYTES,
//  (c) the '$'-separated parameter prefix (everything up to and including the
//      '$' before the salt) is byte-identical, and
//  (d) each library's own string verifies under BOTH libraries' str_verify.
// ---------------------------------------------------------------------------

/// Length of the C string in `buf` (index of first NUL), or None if unterminated
/// within `max`.
fn cstr_len(buf: &[u8], max: usize) -> Option<usize> {
    buf.iter().take(max).position(|&b| b == 0)
}

/// The argon2 encoded format is `$argon2X$v=..$m=..,t=..,p=..$<salt>$<hash>`.
/// The parameter prefix is everything up to and including the '$' that precedes
/// the salt, i.e. the 4th '$' (indices of '$': 0 at start, then after type,
/// after v=.., after m=..,t=..,p=..). We take the substring through the 4th '$'.
fn argon_param_prefix(s: &[u8]) -> &[u8] {
    let mut dollars = 0;
    for (i, &b) in s.iter().enumerate() {
        if b == b'$' {
            dollars += 1;
            if dollars == 4 {
                return &s[..=i];
            }
        }
    }
    s
}

fn check_str_pair(label: &str, cout: &[u8], rout: &[u8], rc: c_int, rr: c_int) {
    assert_eq!(rc, rr, "{label}: rc C={rc} Rust={rr}");
    if rc != 0 {
        return; // error path: nothing more to compare (rc equality already checked)
    }
    let cl = cstr_len(cout, STRBYTES).unwrap_or_else(|| panic!("{label}: C output not NUL-terminated within STRBYTES"));
    let rl = cstr_len(rout, STRBYTES).unwrap_or_else(|| panic!("{label}: Rust output not NUL-terminated within STRBYTES"));
    let cp = argon_param_prefix(&cout[..cl]);
    let rp = argon_param_prefix(&rout[..rl]);
    assert_eq!(
        cp, rp,
        "{label}: parameter prefix differs\n C={}\n R={}",
        String::from_utf8_lossy(cp),
        String::from_utf8_lossy(rp)
    );
}

#[test]
fn crypto_pwhash_str_and_alg_cross_verify() {
    let (c_str, r_str) = sym::<PwStr>("crypto_pwhash_str");
    let (c_alg, r_alg) = sym::<PwStrAlg>("crypto_pwhash_str_alg");
    let (cv, rv) = sym::<PwStrVerify>("crypto_pwhash_str_verify");
    let mut rng = Rng::new(SEED ^ 0x44);

    // Each entry: a human label, and a closure producing (c_out, c_rc, r_out, r_rc).
    for pwlen in [0usize, 1, 16] {
        let passwd = rng.bytes(pwlen);
        let pw = passwd.as_ptr() as *const c_char;

        // --- crypto_pwhash_str (argon2id, cheapest legal opslimit/memlimit) ---
        {
            let mut cout = vec![0u8; STRBYTES];
            let mut rout = vec![0u8; STRBYTES];
            let rc = unsafe {
                c_str(cout.as_mut_ptr() as *mut c_char, pw, pwlen as u64, ARGON2ID_OPSLIMIT_MIN, ARGON2_MEMLIMIT_MIN)
            };
            let rr = unsafe {
                r_str(rout.as_mut_ptr() as *mut c_char, pw, pwlen as u64, ARGON2ID_OPSLIMIT_MIN, ARGON2_MEMLIMIT_MIN)
            };
            let label = format!("crypto_pwhash_str pw={pwlen}");
            check_str_pair(&label, &cout, &rout, rc, rr);
            if rc == 0 {
                cross_verify(&label, cv, rv, &cout, &rout, &passwd);
            }
        }

        // --- crypto_pwhash_str_alg for each valid alg ---
        for (alg, ops_min) in [(ALG_ARGON2I13, ARGON2I_OPSLIMIT_MIN), (ALG_ARGON2ID13, ARGON2ID_OPSLIMIT_MIN)] {
            let mut cout = vec![0u8; STRBYTES];
            let mut rout = vec![0u8; STRBYTES];
            let rc = unsafe {
                c_alg(cout.as_mut_ptr() as *mut c_char, pw, pwlen as u64, ops_min, ARGON2_MEMLIMIT_MIN, alg)
            };
            let rr = unsafe {
                r_alg(rout.as_mut_ptr() as *mut c_char, pw, pwlen as u64, ops_min, ARGON2_MEMLIMIT_MIN, alg)
            };
            let label = format!("crypto_pwhash_str_alg alg={alg} pw={pwlen}");
            check_str_pair(&label, &cout, &rout, rc, rr);
            if rc == 0 {
                cross_verify(&label, cv, rv, &cout, &rout, &passwd);
            }
        }
    }
}

/// (d) each library's own encoded string verifies under BOTH libraries' verify.
fn cross_verify(
    label: &str,
    cv: PwStrVerify,
    rv: PwStrVerify,
    cout: &[u8],
    rout: &[u8],
    passwd: &[u8],
) {
    let pw = passwd.as_ptr() as *const c_char;
    let pwlen = passwd.len() as u64;
    // C's string under C-verify and Rust-verify.
    let cc = unsafe { cv(cout.as_ptr() as *const c_char, pw, pwlen) };
    let cr = unsafe { rv(cout.as_ptr() as *const c_char, pw, pwlen) };
    assert_eq!(cc, 0, "{label}: C string under C verify should be 0, got {cc}");
    assert_eq!(cr, 0, "{label}: C string under Rust verify should be 0, got {cr}");
    // Rust's string under C-verify and Rust-verify.
    let rc = unsafe { cv(rout.as_ptr() as *const c_char, pw, pwlen) };
    let rr = unsafe { rv(rout.as_ptr() as *const c_char, pw, pwlen) };
    assert_eq!(rc, 0, "{label}: Rust string under C verify should be 0, got {rc}");
    assert_eq!(rr, 0, "{label}: Rust string under Rust verify should be 0, got {rr}");
}

// ---------------------------------------------------------------------------
// Section 5: crypto_pwhash_str_verify and crypto_pwhash_str_needs_rehash on
// FIXED, deterministic encoded strings. Return code and errno are asserted for
// every one. No hashing is triggered for the malformed / mismatching cases.
// ---------------------------------------------------------------------------

// A valid argon2i string produced by libsodium (m=8, t=3, p=1) for password "pleaseletmein".
// (These are canonical libsodium test vectors, deterministic.)
const VALID_ARGON2I: &str =
    "$argon2i$v=19$m=4096,t=3,p=1$X1NhbHQAAAAAAAAAAAAAAA$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI";
const ARGON2I_PW: &[u8] = b"^T5H$JYt39n%K*j:W]!1s?vg!:jGi]Ax?..l7[p0v:1jHTpla9;]bUN;?bWyCbtqg ";

// A valid argon2id string (m=4096, t=3, p=1) for password "pleaseletmein".
const VALID_ARGON2ID: &str =
    "$argon2id$v=19$m=4096,t=3,p=1$X1NhbHQAAAAAAAAAAAAAAA$sDV8zPvvkfOGCw26RHsjSMvv7K2vmQq/6cxAcmxSEnE";
const ARGON2ID_PW: &[u8] = b"^T5H$JYt39n%K*j:W]!1s?vg!:jGi]Ax?..l7[p0v:1jHTpla9;]bUN;?bWyCbtqg ";

// A valid scrypt "$7$" string for password "Hello world!".
const VALID_SCRYPT: &str =
    "$7$C6..../....SodiumChloride$kBGj9fHznVYFQMEn/qDCfrDevf9YDtcDdKvEqHJLV8D";
const SCRYPT_PW: &[u8] = b"Hello world!";

fn malformed_strings() -> Vec<(&'static str, &'static str)> {
    vec![
        ("wrong prefix", "$argon3i$v=19$m=4096,t=3,p=1$X1NhbHQAAAAAAAAAAAAAAA$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI"),
        ("wrong version", "$argon2i$v=17$m=4096,t=3,p=1$X1NhbHQAAAAAAAAAAAAAAA$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI"),
        ("missing $v=", "$argon2i$m=4096,t=3,p=1$X1NhbHQAAAAAAAAAAAAAAA$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI"),
        ("missing m=", "$argon2i$v=19$t=3,p=1$X1NhbHQAAAAAAAAAAAAAAA$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI"),
        ("missing t=", "$argon2i$v=19$m=4096,p=1$X1NhbHQAAAAAAAAAAAAAAA$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI"),
        ("missing p=", "$argon2i$v=19$m=4096,t=3$X1NhbHQAAAAAAAAAAAAAAA$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI"),
        ("non-numeric m", "$argon2i$v=19$m=abc,t=3,p=1$X1NhbHQAAAAAAAAAAAAAAA$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI"),
        ("non-numeric t", "$argon2i$v=19$m=4096,t=xx,p=1$X1NhbHQAAAAAAAAAAAAAAA$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI"),
        ("leading-zero m", "$argon2i$v=19$m=04096,t=3,p=1$X1NhbHQAAAAAAAAAAAAAAA$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI"),
        ("leading-zero t", "$argon2i$v=19$m=4096,t=03,p=1$X1NhbHQAAAAAAAAAAAAAAA$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI"),
        ("bad base64 salt", "$argon2i$v=19$m=4096,t=3,p=1$!!!!bHQAAAAAAAAAAAAAAA$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI"),
        ("truncated salt", "$argon2i$v=19$m=4096,t=3,p=1$X1Nh$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI"),
        ("truncated hash", "$argon2i$v=19$m=4096,t=3,p=1$X1NhbHQAAAAAAAAAAAAAAA$bWh"),
        ("trailing garbage", "$argon2i$v=19$m=4096,t=3,p=1$X1NhbHQAAAAAAAAAAAAAAA$bWh++MKN1OiFHKgIWTLvIi1iHicmHH7+Fv3K88ifFfI$junk"),
        ("empty string", ""),
        ("only dollar", "$"),
        ("scrypt wrong prefix", "$8$C6..../....SodiumChloride$kBGj9fHznVYFQMEn/qDCfrDevf9YDtcDdKvEqHJLV8D"),
    ]
}

fn nul_terminated(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

#[test]
fn str_verify_fixed_vectors() {
    let (cv, rv) = sym::<PwStrVerify>("crypto_pwhash_str_verify");

    // valid strings: verify against the correct password (rc 0) and a wrong one (rc -1).
    let valids: &[(&str, &str, &[u8])] = &[
        ("argon2i", VALID_ARGON2I, ARGON2I_PW),
        ("argon2id", VALID_ARGON2ID, ARGON2ID_PW),
        ("scrypt", VALID_SCRYPT, SCRYPT_PW),
    ];
    for (tag, s, pw) in valids {
        let cs = nul_terminated(s);
        // correct password
        let (rc, ec) = with_errno(|| unsafe {
            cv(cs.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64)
        });
        let (rr, er) = with_errno(|| unsafe {
            rv(cs.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64)
        });
        assert_eq!(rc, rr, "verify {tag} good: rc C={rc} Rust={rr}");
        if rc == -1 {
            assert_eq!(ec, er, "verify {tag} good: errno C={ec} Rust={er}");
        }
        // wrong password
        let bad = b"wrong-password";
        let (rc, ec) = with_errno(|| unsafe {
            cv(cs.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, bad.len() as u64)
        });
        let (rr, er) = with_errno(|| unsafe {
            rv(cs.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, bad.len() as u64)
        });
        assert_eq!(rc, rr, "verify {tag} bad-pw: rc C={rc} Rust={rr}");
        if rc == -1 {
            assert_eq!(ec, er, "verify {tag} bad-pw: errno C={ec} Rust={er}");
        }
    }

    // malformed strings: rc and errno must match. Password is arbitrary.
    let pw = b"password";
    for (tag, s) in malformed_strings() {
        let cs = nul_terminated(s);
        let (rc, ec) = with_errno(|| unsafe {
            cv(cs.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64)
        });
        let (rr, er) = with_errno(|| unsafe {
            rv(cs.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64)
        });
        assert_eq!(rc, rr, "verify malformed [{tag}]: rc C={rc} Rust={rr}");
        if rc == -1 {
            assert_eq!(ec, er, "verify malformed [{tag}]: errno C={ec} Rust={er}");
        }
    }
}

#[test]
fn str_needs_rehash_fixed_vectors() {
    let (cn, rn) = sym::<PwNeedsRehash>("crypto_pwhash_str_needs_rehash");

    // Valid argon2 strings: probe rehash decision at several ops/mem values.
    // The encoded params are m=4096 (memlimit 4096*1024) t=3. We compare C vs
    // Rust rc+errno for a spread of opslimit/memlimit inputs.
    let argon_valids: &[(&str, &str)] = &[
        ("argon2i", VALID_ARGON2I),
        ("argon2id", VALID_ARGON2ID),
    ];
    // memlimit values are bytes; the encoded m=4096 corresponds to 4096 KiB.
    let mem_kib_4096: usize = 4096 * 1024;
    for (tag, s) in argon_valids {
        let cs = nul_terminated(s);
        for &(ops, mem) in &[
            (3u64, mem_kib_4096),           // exact match -> 0
            (2u64, mem_kib_4096),           // different t -> 1
            (3u64, ARGON2_MEMLIMIT_MIN),    // different m -> 1
            (1u64, ARGON2_MEMLIMIT_MIN),
        ] {
            let (rc, ec) = with_errno(|| unsafe { cn(cs.as_ptr() as *const c_char, ops, mem) });
            let (rr, er) = with_errno(|| unsafe { rn(cs.as_ptr() as *const c_char, ops, mem) });
            assert_eq!(rc, rr, "needs_rehash {tag} ops={ops} mem={mem}: rc C={rc} Rust={rr}");
            if rc == -1 {
                assert_eq!(ec, er, "needs_rehash {tag} ops={ops} mem={mem}: errno C={ec} Rust={er}");
            }
        }
    }

    // malformed strings under needs_rehash: rc and errno must match.
    for (tag, s) in malformed_strings() {
        let cs = nul_terminated(s);
        let (rc, ec) = with_errno(|| unsafe {
            cn(cs.as_ptr() as *const c_char, ARGON2ID_OPSLIMIT_MIN, ARGON2_MEMLIMIT_MIN)
        });
        let (rr, er) = with_errno(|| unsafe {
            rn(cs.as_ptr() as *const c_char, ARGON2ID_OPSLIMIT_MIN, ARGON2_MEMLIMIT_MIN)
        });
        assert_eq!(rc, rr, "needs_rehash malformed [{tag}]: rc C={rc} Rust={rr}");
        if rc == -1 {
            assert_eq!(ec, er, "needs_rehash malformed [{tag}]: errno C={ec} Rust={er}");
        }
    }
}

// ---------------------------------------------------------------------------
// Section 6: scrypt.
//
//   * crypto_pwhash_scryptsalsa208sha256_ll with N in {2,4,16,1024} (and a
//     non-power-of-two 3 that must be rejected), r in {1,8}, p in {1,2},
//     saltlen 0/1/32, passwdlen 0/1/64, outlen 16/32/64. Assert rc, errno and
//     the full output buffer + canary.
//   * The high-level scrypt one-shot + _str / _str_verify / _str_needs_rehash
//     with the cheapest legal opslimit/memlimit pair, and their error gates.
// ---------------------------------------------------------------------------

#[test]
fn scrypt_ll_matrix() {
    let (c, r) = sym::<ScryptLl>("crypto_pwhash_scryptsalsa208sha256_ll");
    let mut rng = Rng::new(SEED ^ 0x66);

    // Valid power-of-two Ns (kept small so runtime stays tiny) plus a
    // non-power-of-two that MUST be rejected with EINVAL.
    let ns: &[u64] = &[2, 4, 16, 1024, 3];
    for &n in ns {
        for &rr_ in &[1u32, 8] {
            for &pp in &[1u32, 2] {
                for &saltlen in &[0usize, 1, 32] {
                    for &pwlen in &[0usize, 1, 64] {
                        for &outlen in &[16usize, 32, 64] {
                            let passwd = rng.bytes(pwlen);
                            let salt = rng.bytes(saltlen);
                            let mut oc = out_buf(outlen);
                            let mut orr = out_buf(outlen);
                            // NULL-safe pointers for zero-length slices.
                            let pwp = passwd.as_ptr();
                            let sp = salt.as_ptr();
                            let (rc, ec) = with_errno(|| unsafe {
                                c(pwp, pwlen, sp, saltlen, n, rr_, pp, oc.as_mut_ptr(), outlen)
                            });
                            let (rd, er) = with_errno(|| unsafe {
                                r(pwp, pwlen, sp, saltlen, n, rr_, pp, orr.as_mut_ptr(), outlen)
                            });
                            let label = format!(
                                "scrypt_ll N={n} r={rr_} p={pp} salt={saltlen} pw={pwlen} out={outlen}"
                            );
                            assert_eq!(rc, rd, "{label}: rc C={rc} Rust={rd}");
                            if rc == -1 {
                                assert_eq!(ec, er, "{label}: errno C={ec} Rust={er}");
                            }
                            eqb(&format!("{label}: output+canary"), &oc, &orr);
                        }
                    }
                }
            }
        }
    }

    // Explicit error gates with pinpoint errno expectations (documented in
    // escrypt_kdf_nosse): N=0/1 -> EINVAL, r=0 -> EINVAL, p=0 -> EINVAL,
    // r*p >= 2^30 -> EFBIG.
    let passwd = rng.bytes(8);
    let salt = rng.bytes(16);
    let gate_cases: &[(&str, u64, u32, u32)] = &[
        ("N=0", 0, 8, 1),
        ("N=1", 1, 8, 1),
        ("N=5 (odd)", 5, 8, 1),
        ("r=0", 4, 0, 1),
        ("p=0", 4, 8, 0),
        ("rp>=2^30", 4, 32768, 32768),
    ];
    for &(tag, n, rr_, pp) in gate_cases {
        let mut oc = out_buf(32);
        let mut orr = out_buf(32);
        let (rc, ec) = with_errno(|| unsafe {
            c(passwd.as_ptr(), passwd.len(), salt.as_ptr(), salt.len(), n, rr_, pp, oc.as_mut_ptr(), 32)
        });
        let (rd, er) = with_errno(|| unsafe {
            r(passwd.as_ptr(), passwd.len(), salt.as_ptr(), salt.len(), n, rr_, pp, orr.as_mut_ptr(), 32)
        });
        assert_eq!(rc, rd, "scrypt_ll gate [{tag}]: rc C={rc} Rust={rd}");
        if rc == -1 {
            assert_eq!(ec, er, "scrypt_ll gate [{tag}]: errno C={ec} Rust={er}");
        }
        eqb(&format!("scrypt_ll gate [{tag}] buffer"), &oc, &orr);
    }
}

// Cheapest legal high-level scrypt parameters. OPSLIMIT_MIN=32768,
// MEMLIMIT_MIN=16777216 (16 MiB) — this maps (via pickparams) to N=1024, r=8,
// p=1, which is inexpensive. INTERACTIVE limits are deliberately not used.
const SCRYPT_OPS_MIN: u64 = 32768;
const SCRYPT_MEM_MIN: usize = 16777216;
const SCRYPT_STRBYTES: usize = 102;

#[test]
fn scrypt_highlevel_and_gates() {
    let (c, r) = sym::<ScryptHash>("crypto_pwhash_scryptsalsa208sha256");
    let mut rng = Rng::new(SEED ^ 0x77);
    // scrypt SALTBYTES == 32
    let salt = rng.bytes(32);

    // Happy path: a few outlen / passwdlen with the cheapest legal params.
    for &outlen in &[16u64, 32, 64] {
        for &pwlen in &[0usize, 1, 16] {
            let passwd = rng.bytes(pwlen);
            let mut oc = out_buf(outlen as usize);
            let mut orr = out_buf(outlen as usize);
            let pw = passwd.as_ptr() as *const c_char;
            let (rc, ec) = with_errno(|| unsafe {
                c(oc.as_mut_ptr(), outlen, pw, pwlen as u64, salt.as_ptr(), SCRYPT_OPS_MIN, SCRYPT_MEM_MIN)
            });
            let (rd, er) = with_errno(|| unsafe {
                r(orr.as_mut_ptr(), outlen, pw, pwlen as u64, salt.as_ptr(), SCRYPT_OPS_MIN, SCRYPT_MEM_MIN)
            });
            let label = format!("scrypt out={outlen} pw={pwlen}");
            assert_eq!(rc, rd, "{label}: rc C={rc} Rust={rd}");
            if rc == -1 {
                assert_eq!(ec, er, "{label}: errno C={ec} Rust={er}");
            }
            eqb(&format!("{label}: output+canary"), &oc, &orr);
        }
    }

    // Error gate: outlen below BYTES_MIN (0, 15) -> EINVAL.
    for &outlen in &[0u64, 1, 15] {
        let passwd = rng.bytes(8);
        let mut oc = out_buf(outlen as usize);
        let mut orr = out_buf(outlen as usize);
        let pw = passwd.as_ptr() as *const c_char;
        let (rc, ec) = with_errno(|| unsafe {
            c(oc.as_mut_ptr(), outlen, pw, 8, salt.as_ptr(), SCRYPT_OPS_MIN, SCRYPT_MEM_MIN)
        });
        let (rd, er) = with_errno(|| unsafe {
            r(orr.as_mut_ptr(), outlen, pw, 8, salt.as_ptr(), SCRYPT_OPS_MIN, SCRYPT_MEM_MIN)
        });
        assert_eq!(rc, rd, "scrypt outlen<MIN out={outlen}: rc C={rc} Rust={rd}");
        if rc == -1 {
            assert_eq!(ec, er, "scrypt outlen<MIN out={outlen}: errno C={ec} Rust={er}");
        }
        eqb(&format!("scrypt outlen<MIN out={outlen} buffer"), &oc, &orr);
    }
}

#[test]
fn scrypt_str_roundtrip_and_needs_rehash() {
    let (c_str, r_str) = sym::<PwStr>("crypto_pwhash_scryptsalsa208sha256_str");
    let (cv, rv) = sym::<PwStrVerify>("crypto_pwhash_scryptsalsa208sha256_str_verify");
    let (cn, rn) = sym::<PwNeedsRehash>("crypto_pwhash_scryptsalsa208sha256_str_needs_rehash");
    let mut rng = Rng::new(SEED ^ 0x88);

    for pwlen in [0usize, 1, 16] {
        let passwd = rng.bytes(pwlen);
        let pw = passwd.as_ptr() as *const c_char;
        let mut cout = vec![0u8; SCRYPT_STRBYTES];
        let mut rout = vec![0u8; SCRYPT_STRBYTES];
        let rc = unsafe {
            c_str(cout.as_mut_ptr() as *mut c_char, pw, pwlen as u64, SCRYPT_OPS_MIN, SCRYPT_MEM_MIN)
        };
        let rr = unsafe {
            r_str(rout.as_mut_ptr() as *mut c_char, pw, pwlen as u64, SCRYPT_OPS_MIN, SCRYPT_MEM_MIN)
        };
        assert_eq!(rc, rr, "scrypt_str pw={pwlen}: rc C={rc} Rust={rr}");
        if rc == 0 {
            // both NUL-terminated within STRBYTES
            let cl = cstr_len(&cout, SCRYPT_STRBYTES)
                .unwrap_or_else(|| panic!("scrypt_str pw={pwlen}: C not NUL-terminated"));
            let rl = cstr_len(&rout, SCRYPT_STRBYTES)
                .unwrap_or_else(|| panic!("scrypt_str pw={pwlen}: Rust not NUL-terminated"));
            // The '$7$<N><r><p>' setting prefix (through the '$' before the salt,
            // i.e. the 3rd '$') is deterministic given the parameters.
            let cp = scrypt_setting_prefix(&cout[..cl]);
            let rp = scrypt_setting_prefix(&rout[..rl]);
            assert_eq!(
                cp, rp,
                "scrypt_str pw={pwlen}: setting prefix differs\n C={}\n R={}",
                String::from_utf8_lossy(cp),
                String::from_utf8_lossy(rp)
            );
            // (d) each library's own string verifies under BOTH verifiers.
            cross_verify(&format!("scrypt_str pw={pwlen}"), cv, rv, &cout, &rout, &passwd);

            // needs_rehash on each produced string with matching and mismatching
            // parameters; rc+errno must agree.
            for &(ops, mem) in &[
                (SCRYPT_OPS_MIN, SCRYPT_MEM_MIN),
                (SCRYPT_OPS_MIN * 2, SCRYPT_MEM_MIN),
                (SCRYPT_OPS_MIN, SCRYPT_MEM_MIN * 2),
            ] {
                let (nc, nec) = with_errno(|| unsafe { cn(cout.as_ptr() as *const c_char, ops, mem) });
                let (nr, ner) = with_errno(|| unsafe { rn(cout.as_ptr() as *const c_char, ops, mem) });
                assert_eq!(nc, nr, "scrypt needs_rehash pw={pwlen} ops={ops} mem={mem}: rc C={nc} Rust={nr}");
                if nc == -1 {
                    assert_eq!(nec, ner, "scrypt needs_rehash pw={pwlen}: errno C={nec} Rust={ner}");
                }
            }
        }
    }

    // scrypt str_verify / needs_rehash on the fixed valid vector + malformed ones.
    {
        let cs = nul_terminated(VALID_SCRYPT);
        // correct password
        let (rc, ec) = with_errno(|| unsafe {
            cv(cs.as_ptr() as *const c_char, SCRYPT_PW.as_ptr() as *const c_char, SCRYPT_PW.len() as u64)
        });
        let (rr, er) = with_errno(|| unsafe {
            rv(cs.as_ptr() as *const c_char, SCRYPT_PW.as_ptr() as *const c_char, SCRYPT_PW.len() as u64)
        });
        assert_eq!(rc, rr, "scrypt verify good: rc C={rc} Rust={rr}");
        if rc == -1 {
            assert_eq!(ec, er, "scrypt verify good: errno C={ec} Rust={er}");
        }
    }
    // malformed under scrypt verify + needs_rehash.
    let pw = b"password";
    for (tag, s) in malformed_strings() {
        let cs = nul_terminated(s);
        let (rc, ec) = with_errno(|| unsafe {
            cv(cs.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64)
        });
        let (rr, er) = with_errno(|| unsafe {
            rv(cs.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64)
        });
        assert_eq!(rc, rr, "scrypt verify malformed [{tag}]: rc C={rc} Rust={rr}");
        if rc == -1 {
            assert_eq!(ec, er, "scrypt verify malformed [{tag}]: errno C={ec} Rust={er}");
        }
        let (nc, nec) = with_errno(|| unsafe {
            cn(cs.as_ptr() as *const c_char, SCRYPT_OPS_MIN, SCRYPT_MEM_MIN)
        });
        let (nr, ner) = with_errno(|| unsafe {
            rn(cs.as_ptr() as *const c_char, SCRYPT_OPS_MIN, SCRYPT_MEM_MIN)
        });
        assert_eq!(nc, nr, "scrypt needs_rehash malformed [{tag}]: rc C={nc} Rust={nr}");
        if nc == -1 {
            assert_eq!(nec, ner, "scrypt needs_rehash malformed [{tag}]: errno C={nec} Rust={ner}");
        }
    }
}

/// scrypt `$7$` setting prefix. Per `escrypt_gensalt_r` the encoding is
/// `$7$` (3 bytes) + N_log2 (1 char) + r (encode64_uint32 of 30 bits = 5 chars)
/// + p (5 chars), and only THEN the random salt — with no '$' delimiter before
/// the salt. So the deterministic, parameter-only prefix is exactly the first
/// 14 bytes (3 + 1 + 5 + 5). We return that fixed-length setting; the salt and
/// hash that follow differ run-to-run.
fn scrypt_setting_prefix(s: &[u8]) -> &[u8] {
    const SETTING_LEN: usize = 3 + 1 + 5 + 5;
    &s[..SETTING_LEN.min(s.len())]
}

// Reference the errno constants so an unused-const warning never masks a real
// one; these document the exact values libsodium sets.
#[test]
fn errno_constants_are_platform_values() {
    assert_eq!(EINVAL, libc::EINVAL);
    assert_eq!(EFBIG, libc::EFBIG);
    assert_eq!(ENOMEM, libc::ENOMEM);
}
