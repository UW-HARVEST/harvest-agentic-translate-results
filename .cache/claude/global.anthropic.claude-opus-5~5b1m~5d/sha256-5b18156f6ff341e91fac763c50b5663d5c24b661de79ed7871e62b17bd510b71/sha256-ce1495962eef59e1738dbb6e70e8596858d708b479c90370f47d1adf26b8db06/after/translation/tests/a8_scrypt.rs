//! Area 8 (part 2) — `crypto_pwhash/scryptsalsa208sha256/*`.
//!
//! Covers `configs_8.md` rows 8.91 – 8.122 and `errors_8.md` rows
//! 8.149 – 8.201.
//!
//! **Speed:** every scrypt parameter set here is at (or just above) the
//! smallest value that still reaches the branch under test — `N <= 16384`,
//! `r <= 8`, `p <= 512`, and the high-level entry points are always driven at
//! `OPSLIMIT_MIN` / `MEMLIMIT_MIN` (32768 / 16 MiB) unless a row explicitly
//! needs a different `pickparams` branch.  The `.so` under test is an
//! unoptimized `dev`-profile build, so the documented SENSITIVE preset
//! (1 GiB, `N = 2^20`) is deliberately not exercised.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::sync::{Mutex, MutexGuard};

/// Tests inside one binary run in parallel threads, and `rng_reset()` rewinds a
/// *process-global* stream, so every test that depends on the deterministic RNG
/// has to hold this lock for as long as it needs a stable stream position.
static RNG_LOCK: Mutex<()> = Mutex::new(());

fn rng_guard() -> MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ------------------------------------------------------------------- types

type SizeGetter = unsafe extern "C" fn() -> usize;
type UllGetter = unsafe extern "C" fn() -> u64;
type StrGetter = unsafe extern "C" fn() -> *const c_char;

type Ll = unsafe extern "C" fn(
    *const u8, // passwd
    usize,     // passwdlen
    *const u8, // salt
    usize,     // saltlen
    u64,       // N
    u32,       // r
    u32,       // p
    *mut u8,   // buf
    usize,     // buflen
) -> c_int;

type Scrypt = unsafe extern "C" fn(
    *mut u8,       // out
    u64,           // outlen
    *const c_char, // passwd
    u64,           // passwdlen
    *const u8,     // salt (SALTBYTES = 32)
    u64,           // opslimit
    usize,         // memlimit
) -> c_int;

type ScryptStr = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize) -> c_int;
type ScryptStrVerify = unsafe extern "C" fn(*const c_char, *const c_char, u64) -> c_int;
type ScryptNeedsRehash = unsafe extern "C" fn(*const c_char, u64, usize) -> c_int;

#[repr(C)]
#[derive(Clone, Copy)]
struct EscryptRegion {
    base: *mut c_void,
    aligned: *mut c_void,
    size: usize,
}

impl EscryptRegion {
    fn zeroed() -> Self {
        EscryptRegion {
            base: std::ptr::null_mut(),
            aligned: std::ptr::null_mut(),
            size: 0,
        }
    }
}

type InitLocal = unsafe extern "C" fn(*mut EscryptRegion) -> c_int;
type FreeLocal = unsafe extern "C" fn(*mut EscryptRegion) -> c_int;
type AllocRegion = unsafe extern "C" fn(*mut EscryptRegion, usize) -> *mut c_void;
type FreeRegion = unsafe extern "C" fn(*mut EscryptRegion) -> c_int;

type KdfNosse = unsafe extern "C" fn(
    *mut EscryptRegion,
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

type EscryptR = unsafe extern "C" fn(
    *mut EscryptRegion,
    *const u8, // passwd
    usize,     // passwdlen
    *const u8, // setting
    *mut u8,   // buf
    usize,     // buflen
) -> *mut u8;

type GensaltR =
    unsafe extern "C" fn(u32, u32, u32, *const u8, usize, *mut u8, usize) -> *mut u8;

type ParseSetting =
    unsafe extern "C" fn(*const u8, *mut u32, *mut u32, *mut u32) -> *const u8;

type Pbkdf2 = unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, *mut u8, usize);

// ----------------------------------------------------------------- helpers

const SALTBYTES: usize = 32;
const STRBYTES: usize = 102;
const SENTINEL: c_int = 0x5EED;

fn h(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0);
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
        .collect()
}

fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

fn as_str(b: &[u8]) -> String {
    let end = b.iter().position(|&x| x == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

/// Read the `randombytes_buf` stream of the C library, so a test can predict
/// the salt a `*_str` call will pick after `rng_reset()`.
fn stream_bytes(n: usize) -> Vec<u8> {
    type Buf = unsafe extern "C" fn(*mut c_void, usize);
    let (c, _) = both::<Buf>("randombytes_buf");
    let mut v = vec![0u8; n];
    unsafe { c(v.as_mut_ptr() as *mut c_void, n) };
    v
}

/// `crypto_pwhash_scryptsalsa208sha256_ll` on both libraries.
#[track_caller]
fn ll(
    c: &Ll,
    r: &Ll,
    label: &str,
    pw: &[u8],
    salt: &[u8],
    n: u64,
    rr: u32,
    p: u32,
    buflen: usize,
) -> (c_int, c_int, Vec<u8>) {
    let mut bc = padded(buflen);
    let mut br = padded(buflen);
    set_errno(SENTINEL);
    let rc = unsafe {
        c(
            pw.as_ptr(),
            pw.len(),
            salt.as_ptr(),
            salt.len(),
            n,
            rr,
            p,
            bc.as_mut_ptr(),
            buflen,
        )
    };
    let ec = errno();
    set_errno(SENTINEL);
    let rrc = unsafe {
        r(
            pw.as_ptr(),
            pw.len(),
            salt.as_ptr(),
            salt.len(),
            n,
            rr,
            p,
            br.as_mut_ptr(),
            buflen,
        )
    };
    let er = errno();
    eqi(&format!("{label} ret"), rc, rrc);
    assert_eq!(ec, er, "{label}: errno mismatch (C {ec}, Rust {er})");
    eqb(&format!("{label} buf"), &bc[..buflen], &br[..buflen]);
    check_pad(&format!("{label}(C)"), &bc, buflen);
    check_pad(&format!("{label}(Rust)"), &br, buflen);
    (rc, ec, bc[..buflen].to_vec())
}

/// The high-level `crypto_pwhash_scryptsalsa208sha256` on both libraries.
#[track_caller]
fn hi(
    c: &Scrypt,
    r: &Scrypt,
    label: &str,
    outlen: usize,
    pw: &[u8],
    salt: &[u8],
    ops: u64,
    mem: usize,
) -> (c_int, c_int, Vec<u8>) {
    assert_eq!(salt.len(), SALTBYTES);
    let mut oc = padded(outlen);
    let mut or = padded(outlen);
    set_errno(SENTINEL);
    let rc = unsafe {
        c(
            oc.as_mut_ptr(),
            outlen as u64,
            pw.as_ptr() as *const c_char,
            pw.len() as u64,
            salt.as_ptr(),
            ops,
            mem,
        )
    };
    let ec = errno();
    set_errno(SENTINEL);
    let rr = unsafe {
        r(
            or.as_mut_ptr(),
            outlen as u64,
            pw.as_ptr() as *const c_char,
            pw.len() as u64,
            salt.as_ptr(),
            ops,
            mem,
        )
    };
    let er = errno();
    eqi(&format!("{label} ret"), rc, rr);
    assert_eq!(ec, er, "{label}: errno mismatch (C {ec}, Rust {er})");
    eqb(&format!("{label} out"), &oc[..outlen], &or[..outlen]);
    check_pad(&format!("{label}(C)"), &oc, outlen);
    check_pad(&format!("{label}(Rust)"), &or, outlen);
    (rc, ec, oc[..outlen].to_vec())
}

// ------------------------------------------------------- 8.91 constants

#[test]
fn r8_91_constants() {
    for (name, want) in [
        ("crypto_pwhash_scryptsalsa208sha256_bytes_min", 16usize),
        ("crypto_pwhash_scryptsalsa208sha256_bytes_max", 137438953440),
        ("crypto_pwhash_scryptsalsa208sha256_passwd_min", 0),
        ("crypto_pwhash_scryptsalsa208sha256_passwd_max", usize::MAX),
        ("crypto_pwhash_scryptsalsa208sha256_saltbytes", 32),
        ("crypto_pwhash_scryptsalsa208sha256_strbytes", 102),
        ("crypto_pwhash_scryptsalsa208sha256_memlimit_min", 16777216),
        ("crypto_pwhash_scryptsalsa208sha256_memlimit_max", 68719476736),
        ("crypto_pwhash_scryptsalsa208sha256_memlimit_interactive", 16777216),
        ("crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive", 1073741824),
    ] {
        let (c, r) = both::<SizeGetter>(name);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, want, "{name}: C returned {cv}, expected {want}");
        assert_eq!(rv, cv, "{name}: Rust {rv} != C {cv}");
    }
    for (name, want) in [
        ("crypto_pwhash_scryptsalsa208sha256_opslimit_min", 32768u64),
        ("crypto_pwhash_scryptsalsa208sha256_opslimit_max", 4294967295),
        ("crypto_pwhash_scryptsalsa208sha256_opslimit_interactive", 524288),
        ("crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive", 33554432),
    ] {
        let (c, r) = both::<UllGetter>(name);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, want, "{name}: C returned {cv}, expected {want}");
        assert_eq!(rv, cv, "{name}: Rust {rv} != C {cv}");
    }
    let (c, r) = both::<StrGetter>("crypto_pwhash_scryptsalsa208sha256_strprefix");
    unsafe {
        let cs = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
        let rs = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
        eqb("scrypt strprefix", &cs, &rs);
        assert_eq!(cs, b"$7$".to_vec());
    }
}

// --------------------------------------------------- 8.92 – 8.101  _ll valid

#[test]
fn r8_92_to_98_ll_parameter_matrix() {
    let (c, r) = both::<Ll>("crypto_pwhash_scryptsalsa208sha256_ll");
    let empty: Vec<u8> = Vec::new();

    // 8.92 smallest legal parameter set.
    let (rc, _, _) = ll(&c, &r, "8.92 N=2,r=1,p=1", &empty, &empty, 2, 1, 1, 16);
    assert_eq!(rc, 0);

    // 8.93 the classic RFC 7914 vector shape (N=16, r=1, p=1, empty pw/salt).
    let (rc, _, out32) = ll(&c, &r, "8.93 N=16 buflen=32", &empty, &empty, 16, 1, 1, 32);
    assert_eq!(rc, 0);
    let (_, _, out64) = ll(&c, &r, "8.93 N=16 buflen=64", &empty, &empty, 16, 1, 1, 64);
    // RFC 7914 §11 first scrypt test vector.
    let want = h(
        "77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442\
         fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906",
    );
    eqb("8.93 RFC 7914 scrypt vector", &want, &out64);
    eqb("8.93 truncated prefix", &want[..32], &out32);

    // 8.94 / 8.95 / 8.96 / 8.97 / 8.98
    for (label, n, rr, p, buflen) in [
        ("8.94 N=1024,r=1,p=1", 1024u64, 1u32, 1u32, 64usize),
        ("8.95 N=1024,r=8,p=1", 1024, 8, 1, 64),
        ("8.96 N=1024,r=8,p=2", 1024, 8, 2, 64),
        ("8.97 N=16384,r=8,p=1", 16384, 8, 1, 64),
        ("8.98 N=16,r=8,p=16", 16, 8, 16, 64),
    ] {
        let mut rng = Rng::new(0x8_0094 ^ n);
        for trial in 0..3 {
            let pwlen = rng.range(0, 40);
            let saltlen = rng.range(0, 40);
            let pw = rng.bytes(pwlen);
            let salt = rng.bytes(saltlen);
            let (rc, _, _) = ll(&c, &r, &format!("{label} trial#{trial}"), &pw, &salt, n, rr, p, buflen);
            assert_eq!(rc, 0, "{label}: unexpected failure");
        }
    }
}

#[test]
fn r8_99_ll_buflen_variations() {
    // buflen variations at N=16,r=1,p=1; 31/33/100 exercise the partial `clen`
    // copy of the last PBKDF2 block.
    let (c, r) = both::<Ll>("crypto_pwhash_scryptsalsa208sha256_ll");
    let mut rng = Rng::new(0x8_0099);
    for buflen in [1usize, 16, 31, 32, 33, 64, 100] {
        for trial in 0..3 {
            let pwlen = rng.range(0, 24);
            let saltlen = rng.range(0, 24);
            let pw = rng.bytes(pwlen);
            let salt = rng.bytes(saltlen);
            let (rc, _, _) = ll(
                &c,
                &r,
                &format!("8.99 buflen={buflen} trial#{trial}"),
                &pw,
                &salt,
                16,
                1,
                1,
                buflen,
            );
            assert_eq!(rc, 0);
        }
    }
    // buflen == 0 also runs (the PBKDF2 loop body never executes).
    let (rc, _, _) = ll(&c, &r, "8.99 buflen=0", b"pw", b"salt", 16, 1, 1, 0);
    assert_eq!(rc, 0);
}

#[test]
fn r8_100_101_ll_salt_and_password_shapes() {
    let (c, r) = both::<Ll>("crypto_pwhash_scryptsalsa208sha256_ll");
    // 8.100 saltlen and passwdlen variations, including binary data with NUL
    // bytes (neither input is a C string).
    let mut rng = Rng::new(0x8_0100);
    for saltlen in [0usize, 1, 32, 64] {
        for pwlen in [0usize, 1, 64] {
            let mut pw = rng.bytes(pwlen);
            let mut salt = rng.bytes(saltlen);
            if pwlen > 2 {
                pw[1] = 0;
                pw[2] = 0xff;
            }
            if saltlen > 2 {
                salt[0] = 0;
                salt[1] = 0xff;
            }
            let (rc, _, _) = ll(
                &c,
                &r,
                &format!("8.100 saltlen={saltlen} pwlen={pwlen}"),
                &pw,
                &salt,
                16,
                1,
                1,
                32,
            );
            assert_eq!(rc, 0);
        }
    }

    // 8.101 repeated calls through the escrypt_init_local/alloc/free path are
    // reproducible.
    let pw = b"repeat";
    let salt = b"salty-salt";
    let (_, _, a) = ll(&c, &r, "8.101 call#1", pw, salt, 1024, 8, 1, 64);
    let (_, _, b) = ll(&c, &r, "8.101 call#2", pw, salt, 1024, 8, 1, 64);
    let (_, _, d) = ll(&c, &r, "8.101 call#3", pw, salt, 16, 1, 1, 64);
    eqb("8.101 reproducible", &a, &b);
    assert_ne!(a, d, "8.101: different N produced the same output");
}

// -------------------------------------- 8.102 – 8.107 high-level pickparams

#[test]
fn r8_102_to_107_high_level() {
    let (c, r) = both::<Scrypt>("crypto_pwhash_scryptsalsa208sha256");
    let (cll, rll) = both::<Ll>("crypto_pwhash_scryptsalsa208sha256_ll");
    let mut rng = Rng::new(0x8_0102);
    let salt = rng.bytes(SALTBYTES);
    let pw = b"password".to_vec();

    // 8.102 first pickparams branch (opslimit < memlimit/32): r=8, p=1, N=1024.
    let (rc, _, out) = hi(&c, &r, "8.102", 32, &pw, &salt, 32768, 16777216);
    assert_eq!(rc, 0);
    let (_, _, ref_out) = ll(&cll, &rll, "8.102 reference _ll", &pw, &salt, 1024, 8, 1, 32);
    eqb("8.102 == _ll(N=1024,r=8,p=1)", &ref_out, &out);

    // 8.103 second branch (opslimit == memlimit/32): N=16384, r=8, p=1.
    let (rc, _, out) = hi(&c, &r, "8.103", 32, &pw, &salt, 524288, 16777216);
    assert_eq!(rc, 0);
    let (_, _, ref_out) = ll(&cll, &rll, "8.103 reference _ll", &pw, &salt, 16384, 8, 1, 32);
    eqb("8.103 == _ll(N=16384,r=8,p=1)", &ref_out, &out);

    // 8.104 second branch with p > 1: N=512, r=8, p=2.
    let (rc, _, out) = hi(&c, &r, "8.104", 32, &pw, &salt, 32768, 524288);
    assert_eq!(rc, 0);
    let (_, _, ref_out) = ll(&cll, &rll, "8.104 reference _ll", &pw, &salt, 512, 8, 2, 32);
    eqb("8.104 == _ll(N=512,r=8,p=2)", &ref_out, &out);

    // 8.105 / 8.154 memlimit = 0 is *not* rejected: N=2, r=8, p=512.
    let (rc, _, out) = hi(&c, &r, "8.105 memlimit=0", 32, &pw, &salt, 32768, 0);
    assert_eq!(rc, 0, "8.105: memlimit = 0 must still succeed");
    let (_, _, ref_out) = ll(&cll, &rll, "8.105 reference _ll", &pw, &salt, 2, 8, 512, 32);
    eqb("8.105 == _ll(N=2,r=8,p=512)", &ref_out, &out);

    // 8.106 / 8.154 opslimit = 0 is silently clamped to 32768.
    let (rc, _, out0) = hi(&c, &r, "8.106 opslimit=0", 32, &pw, &salt, 0, 16777216);
    assert_eq!(rc, 0);
    let (_, _, out_min) = hi(&c, &r, "8.106 opslimit=OPSLIMIT_MIN", 32, &pw, &salt, 32768, 16777216);
    eqb("8.106 opslimit 0 == OPSLIMIT_MIN", &out_min, &out0);

    // 8.107 outlen variations and an empty password.
    for outlen in [16usize, 32, 64, 100] {
        for pwlen in [0usize, 1, 8] {
            let p = rng.bytes(pwlen);
            let (rc, _, _) = hi(
                &c,
                &r,
                &format!("8.107 outlen={outlen} pwlen={pwlen}"),
                outlen,
                &p,
                &salt,
                32768,
                16777216,
            );
            assert_eq!(rc, 0);
        }
    }

    // Random salts, several seeds.
    for i in 0..4 {
        let s = rng.bytes(SALTBYTES);
        let plen = rng.range(0, 48);
        let p = rng.bytes(plen);
        let (rc, _, _) = hi(&c, &r, &format!("8.107 random#{i}"), 32, &p, &s, 32768, 16777216);
        assert_eq!(rc, 0);
    }
    // All-zero and all-0xFF salts.
    for s in [vec![0u8; SALTBYTES], vec![0xffu8; SALTBYTES]] {
        let (rc, _, _) = hi(&c, &r, "8.107 fixed salt", 32, &pw, &s, 32768, 16777216);
        assert_eq!(rc, 0);
    }
}

// --------------------------------------------- 8.109 – 8.111 `$7$` strings

#[test]
fn r8_109_to_111_str_round_trip() {
    let _rng = rng_guard();
    let (cs, rs) = both::<ScryptStr>("crypto_pwhash_scryptsalsa208sha256_str");
    let (cv, rv) = both::<ScryptStrVerify>("crypto_pwhash_scryptsalsa208sha256_str_verify");
    let (cn, rn) = both::<ScryptNeedsRehash>("crypto_pwhash_scryptsalsa208sha256_str_needs_rehash");

    let ops = 32768u64;
    let mem = 16777216usize;

    for pw in [b"password".to_vec(), Vec::new(), b"\x00\xffbinary".to_vec()] {
        // Both libraries are given the same RNG stream, so the random 32-byte
        // salt (and therefore the whole string) must match byte for byte.
        rng_reset();
        let mut sc = padded(STRBYTES);
        let mut sr = padded(STRBYTES);
        let rc = unsafe {
            cs(
                sc.as_mut_ptr() as *mut c_char,
                pw.as_ptr() as *const c_char,
                pw.len() as u64,
                ops,
                mem,
            )
        };
        let rr = unsafe {
            rs(
                sr.as_mut_ptr() as *mut c_char,
                pw.as_ptr() as *const c_char,
                pw.len() as u64,
                ops,
                mem,
            )
        };
        eqi("8.109 str ret", rc, rr);
        assert_eq!(rc, 0);
        eqb("8.109 str", &sc[..STRBYTES], &sr[..STRBYTES]);
        check_pad("8.109 str(C)", &sc, STRBYTES);
        check_pad("8.109 str(Rust)", &sr, STRBYTES);

        // 8.109 exactly 101 chars + NUL, starting with "$7$".
        let body = as_str(&sc[..STRBYTES]);
        assert_eq!(body.len(), 101, "8.109: expected 101 chars, got {}", body.len());
        assert_eq!(sc[101], 0, "8.109: byte 101 must be the NUL terminator");
        assert!(body.starts_with("$7$"), "8.109: bad prefix {body:?}");

        // 8.110 round trip, in BOTH directions (C string verified by Rust and
        // vice versa — the strings are equal here, so verify both symbols).
        let sz = cstr(&body);
        for (label, f) in [("C verify", &cv), ("Rust verify", &rv)] {
            let got = unsafe {
                f(
                    sz.as_ptr() as *const c_char,
                    pw.as_ptr() as *const c_char,
                    pw.len() as u64,
                )
            };
            assert_eq!(got, 0, "8.110 {label}: correct password rejected");
        }
        // Wrong password.
        let bad = b"wrong-password".to_vec();
        let gc = unsafe {
            cv(
                sz.as_ptr() as *const c_char,
                bad.as_ptr() as *const c_char,
                bad.len() as u64,
            )
        };
        let gr = unsafe {
            rv(
                sz.as_ptr() as *const c_char,
                bad.as_ptr() as *const c_char,
                bad.len() as u64,
            )
        };
        eqi("8.163 wrong password", gc, gr);
        assert_eq!(gc, -1);

        // 8.111 needs_rehash.
        for (label, ops2, mem2, want) in [
            ("same params", ops, mem, 0),
            ("different N_log2", 524288u64, mem, 1),
            ("different N_log2 and p", 32768, 524288usize, 1),
        ] {
            let gc = unsafe { cn(sz.as_ptr() as *const c_char, ops2, mem2) };
            let gr = unsafe { rn(sz.as_ptr() as *const c_char, ops2, mem2) };
            eqi(&format!("8.111 {label}"), gc, gr);
            assert_eq!(gc, want, "8.111 {label}: expected {want}, got {gc}");
        }
    }

    // Two successive calls use two different random salts, so the strings
    // differ yet both verify.
    rng_reset();
    let pw = b"password".to_vec();
    let mut a = padded(STRBYTES);
    let mut b = padded(STRBYTES);
    unsafe {
        assert_eq!(
            cs(a.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, pw.len() as u64, ops, mem),
            0
        );
        assert_eq!(
            cs(b.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, pw.len() as u64, ops, mem),
            0
        );
    }
    let mut ar = padded(STRBYTES);
    let mut br = padded(STRBYTES);
    unsafe {
        assert_eq!(
            rs(ar.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, pw.len() as u64, ops, mem),
            0
        );
        assert_eq!(
            rs(br.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, pw.len() as u64, ops, mem),
            0
        );
    }
    eqb("8.109 call#1 C vs Rust", &a[..STRBYTES], &ar[..STRBYTES]);
    eqb("8.109 call#2 C vs Rust", &b[..STRBYTES], &br[..STRBYTES]);
    assert_ne!(&a[..STRBYTES], &b[..STRBYTES], "8.109: two calls produced the same string");
    // Cross-library verification: C's string #1 checked by Rust, Rust's #2 by C.
    let s1 = cstr(&as_str(&a[..STRBYTES]));
    let s2 = cstr(&as_str(&br[..STRBYTES]));
    unsafe {
        assert_eq!(
            rv(s1.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64),
            0,
            "Rust failed to verify a C-produced $7$ string"
        );
        assert_eq!(
            cv(s2.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64),
            0,
            "C failed to verify a Rust-produced $7$ string"
        );
    }
}

// ------------------------------- 8.112 – 8.115 gensalt / parse_setting

#[test]
fn r8_112_to_115_gensalt_and_parse_setting() {
    let _rng = rng_guard();
    let (cg, rg) = both::<GensaltR>("_sodium_escrypt_gensalt_r");
    let (cp, rp) = both::<ParseSetting>("_sodium_escrypt_parse_setting");
    let mut rng = Rng::new(0x8_0112);
    let salt32 = rng.bytes(32);

    // 8.112 (N_log2, r, p) round trip through gensalt -> parse_setting.
    for (nl, rr, p) in [
        (10u32, 8u32, 1u32),
        (14, 8, 1),
        (9, 8, 2),
        (1, 1, 1),
        (1, 8, 512),
        (63, 1, 1),
        (0, 1, 1),
    ] {
        let buflen = 58usize; // 14 + 43 + 1
        let mut bc = padded(buflen);
        let mut br = padded(buflen);
        let (pc, pr) = unsafe {
            (
                cg(nl, rr, p, salt32.as_ptr(), 32, bc.as_mut_ptr(), buflen),
                rg(nl, rr, p, salt32.as_ptr(), 32, br.as_mut_ptr(), buflen),
            )
        };
        assert!(!pc.is_null(), "8.112 gensalt({nl},{rr},{p}) failed in C");
        assert_eq!(pc as usize, bc.as_ptr() as usize);
        assert_eq!(pr as usize, br.as_ptr() as usize);
        eqb(&format!("8.112 setting({nl},{rr},{p})"), &bc[..buflen], &br[..buflen]);
        check_pad("8.112 gensalt(C)", &bc, buflen);
        check_pad("8.112 gensalt(Rust)", &br, buflen);
        let setting = as_str(&bc[..buflen]);
        assert_eq!(setting.len(), 57, "8.112: setting length");
        assert!(setting.starts_with("$7$"));

        let sz = cstr(&setting);
        let mut a = [0u32; 3];
        let mut b = [0u32; 3];
        let (qc, qr) = unsafe {
            (
                cp(sz.as_ptr(), &mut a[0], &mut a[1], &mut a[2]),
                rp(sz.as_ptr(), &mut b[0], &mut b[1], &mut b[2]),
            )
        };
        assert!(!qc.is_null(), "8.115 parse_setting failed in C");
        assert_eq!(
            qc as usize - sz.as_ptr() as usize,
            14,
            "8.115: parse_setting must return setting + 14"
        );
        assert_eq!(
            qr as usize - sz.as_ptr() as usize,
            14,
            "8.115: Rust parse_setting returned a different offset"
        );
        assert_eq!(a, [nl, rr, p], "8.112 round trip mismatch (C)");
        assert_eq!(b, a, "8.112 round trip mismatch (Rust)");
    }

    // 8.113 srclen variations, with buflen exactly `need` (tightest accepting).
    for (srclen, need) in [(0usize, 15usize), (1, 17), (16, 37), (32, 58)] {
        let src = rng.bytes(srclen);
        for buflen in [need, need + 1, need + 40] {
            let mut bc = padded(buflen);
            let mut br = padded(buflen);
            let (pc, pr) = unsafe {
                (
                    cg(10, 8, 1, src.as_ptr(), srclen, bc.as_mut_ptr(), buflen),
                    rg(10, 8, 1, src.as_ptr(), srclen, br.as_mut_ptr(), buflen),
                )
            };
            assert_eq!(pc.is_null(), pr.is_null(), "8.113 srclen={srclen} buflen={buflen}");
            assert!(!pc.is_null(), "8.113 srclen={srclen} buflen={buflen}: rejected");
            eqb(
                &format!("8.113 srclen={srclen} buflen={buflen}"),
                &bc[..buflen],
                &br[..buflen],
            );
            check_pad("8.113(C)", &bc, buflen);
            check_pad("8.113(Rust)", &br, buflen);
            assert_eq!(as_str(&bc[..buflen]).len(), need - 1);
        }
    }

    // 8.114 r * p just under the 2^30 limit is accepted by gensalt.
    let buflen = 58usize;
    let mut bc = padded(buflen);
    let mut br = padded(buflen);
    let (pc, pr) = unsafe {
        (
            cg(10, 1, 1073741823, salt32.as_ptr(), 32, bc.as_mut_ptr(), buflen),
            rg(10, 1, 1073741823, salt32.as_ptr(), 32, br.as_mut_ptr(), buflen),
        )
    };
    assert!(!pc.is_null() && !pr.is_null(), "8.114: r*p = 2^30-1 must be accepted");
    eqb("8.114 setting", &bc[..buflen], &br[..buflen]);
    let sz = cstr(&as_str(&bc[..buflen]));
    let mut a = [0u32; 3];
    let mut b = [0u32; 3];
    unsafe {
        assert!(!cp(sz.as_ptr(), &mut a[0], &mut a[1], &mut a[2]).is_null());
        assert!(!rp(sz.as_ptr(), &mut b[0], &mut b[1], &mut b[2]).is_null());
    }
    assert_eq!(a, [10, 1, 1073741823]);
    assert_eq!(b, a);

    // 8.115 parse a real crypto_pwhash_scryptsalsa208sha256_str output, and a
    // bare setting with no trailing "$hash".
    let (cs, _) = both::<ScryptStr>("crypto_pwhash_scryptsalsa208sha256_str");
    rng_reset();
    let mut out = padded(STRBYTES);
    let pw = b"password";
    assert_eq!(
        unsafe {
            cs(
                out.as_mut_ptr() as *mut c_char,
                pw.as_ptr() as *const c_char,
                pw.len() as u64,
                32768,
                16777216,
            )
        },
        0
    );
    let full = cstr(&as_str(&out[..STRBYTES]));
    let mut a = [0u32; 3];
    let mut b = [0u32; 3];
    let (qc, qr) = unsafe {
        (
            cp(full.as_ptr(), &mut a[0], &mut a[1], &mut a[2]),
            rp(full.as_ptr(), &mut b[0], &mut b[1], &mut b[2]),
        )
    };
    assert_eq!(qc as usize - full.as_ptr() as usize, 14);
    assert_eq!(qr as usize - full.as_ptr() as usize, 14);
    assert_eq!(a, [10, 8, 1], "8.115: pickparams(32768, 16 MiB) = (10, 8, 1)");
    assert_eq!(b, a);
}

// ---------------------------------------------- 8.116 – 8.118 escrypt_r

#[test]
fn r8_116_to_118_escrypt_r() {
    let _rng = rng_guard();
    let (cg, rg) = both::<GensaltR>("_sodium_escrypt_gensalt_r");
    let (cr, rr) = both::<EscryptR>("_sodium_escrypt_r");
    let (ci, ri) = both::<InitLocal>("_sodium_escrypt_init_local");
    let (cf, rf) = both::<FreeLocal>("_sodium_escrypt_free_local");
    let (cs, _) = both::<ScryptStr>("crypto_pwhash_scryptsalsa208sha256_str");
    let pw = b"password".to_vec();

    // The salt that crypto_pwhash_scryptsalsa208sha256_str will draw first.
    rng_reset();
    let salt32 = stream_bytes(32);

    // 8.116 escrypt_r with a gensalt(10, 8, 1) setting must reproduce
    // crypto_pwhash_scryptsalsa208sha256_str exactly.
    let mut setting_c = padded(58);
    let mut setting_r = padded(58);
    unsafe {
        assert!(!cg(10, 8, 1, salt32.as_ptr(), 32, setting_c.as_mut_ptr(), 58).is_null());
        assert!(!rg(10, 8, 1, salt32.as_ptr(), 32, setting_r.as_mut_ptr(), 58).is_null());
    }
    eqb("8.116 setting", &setting_c[..58], &setting_r[..58]);
    let setting = cstr(&as_str(&setting_c[..58]));

    let mut local_c = EscryptRegion::zeroed();
    let mut local_r = EscryptRegion::zeroed();
    unsafe {
        assert_eq!(ci(&mut local_c), 0);
        assert_eq!(ri(&mut local_r), 0);
    }
    let mut bc = padded(STRBYTES);
    let mut br = padded(STRBYTES);
    rng_reset(); // escrypt_r starts by randomising `buf`
    let (pc, pr) = unsafe {
        (
            cr(
                &mut local_c,
                pw.as_ptr(),
                pw.len(),
                setting.as_ptr(),
                bc.as_mut_ptr(),
                STRBYTES,
            ),
            rr(
                &mut local_r,
                pw.as_ptr(),
                pw.len(),
                setting.as_ptr(),
                br.as_mut_ptr(),
                STRBYTES,
            ),
        )
    };
    assert!(!pc.is_null() && !pr.is_null(), "8.116: escrypt_r failed");
    eqb("8.116 escrypt_r output", &bc[..STRBYTES], &br[..STRBYTES]);
    check_pad("8.116(C)", &bc, STRBYTES);
    check_pad("8.116(Rust)", &br, STRBYTES);
    assert_eq!(as_str(&bc[..STRBYTES]).len(), 101);

    rng_reset();
    let mut expect = padded(STRBYTES);
    assert_eq!(
        unsafe {
            cs(
                expect.as_mut_ptr() as *mut c_char,
                pw.as_ptr() as *const c_char,
                pw.len() as u64,
                32768,
                16777216,
            )
        },
        0
    );
    eqb(
        "8.116 escrypt_r == crypto_pwhash_scryptsalsa208sha256_str",
        &expect[..STRBYTES],
        &bc[..STRBYTES],
    );

    // 8.117 the setting may itself be a full password string (this is exactly
    // how _str_verify works): strrchr bounds the salt and the recomputed string
    // equals the input.
    let full = cstr(&as_str(&expect[..STRBYTES]));
    let mut vc = padded(STRBYTES);
    let mut vr = padded(STRBYTES);
    rng_reset();
    let (pc, pr) = unsafe {
        (
            cr(&mut local_c, pw.as_ptr(), pw.len(), full.as_ptr(), vc.as_mut_ptr(), STRBYTES),
            rr(&mut local_r, pw.as_ptr(), pw.len(), full.as_ptr(), vr.as_mut_ptr(), STRBYTES),
        )
    };
    assert!(!pc.is_null() && !pr.is_null(), "8.117: escrypt_r failed");
    eqb("8.117 recomputed", &vc[..STRBYTES], &vr[..STRBYTES]);
    assert_eq!(
        as_str(&vc[..STRBYTES]),
        as_str(&expect[..STRBYTES]),
        "8.117: recomputed string differs from the input"
    );

    // 8.118 a shorter salt in the setting (16 bytes -> 22 salt chars):
    // need = 14 + 22 + 1 + 43 + 1 = 81 <= buflen, output is 80 chars + NUL.
    let salt16 = &salt32[..16];
    let mut s16c = padded(37);
    let mut s16r = padded(37);
    unsafe {
        assert!(!cg(10, 8, 1, salt16.as_ptr(), 16, s16c.as_mut_ptr(), 37).is_null());
        assert!(!rg(10, 8, 1, salt16.as_ptr(), 16, s16r.as_mut_ptr(), 37).is_null());
    }
    eqb("8.118 short setting", &s16c[..37], &s16r[..37]);
    let s16 = cstr(&as_str(&s16c[..37]));
    let mut oc = padded(STRBYTES);
    let mut or = padded(STRBYTES);
    rng_reset();
    let (pc, pr) = unsafe {
        (
            cr(&mut local_c, pw.as_ptr(), pw.len(), s16.as_ptr(), oc.as_mut_ptr(), STRBYTES),
            rr(&mut local_r, pw.as_ptr(), pw.len(), s16.as_ptr(), or.as_mut_ptr(), STRBYTES),
        )
    };
    assert!(!pc.is_null() && !pr.is_null(), "8.118: escrypt_r failed");
    eqb("8.118 output", &oc[..STRBYTES], &or[..STRBYTES]);
    assert_eq!(as_str(&oc[..STRBYTES]).len(), 80, "8.118: expected 80 chars");

    unsafe {
        assert_eq!(cf(&mut local_c), 0);
        assert_eq!(rf(&mut local_r), 0);
    }
}

// ------------------------ 8.119 regions, 8.198 / 8.199 alloc failure

#[test]
fn r8_119_198_199_regions() {
    let (ci, ri) = both::<InitLocal>("_sodium_escrypt_init_local");
    let (cfl, rfl) = both::<FreeLocal>("_sodium_escrypt_free_local");
    let (ca, ra) = both::<AllocRegion>("_sodium_escrypt_alloc_region");
    let (cfr, rfr) = both::<FreeRegion>("_sodium_escrypt_free_region");

    for size in [1024usize, 65536, 128 * 8 * (1024 + 1) + 256 * 8 + 64] {
        let mut rc = EscryptRegion::zeroed();
        let mut rr = EscryptRegion::zeroed();
        unsafe {
            assert_eq!(ci(&mut rc), 0, "escrypt_init_local (C)");
            assert_eq!(ri(&mut rr), 0, "escrypt_init_local (Rust)");
            assert!(rc.base.is_null() && rc.aligned.is_null() && rc.size == 0);
            assert!(rr.base.is_null() && rr.aligned.is_null() && rr.size == 0);

            let pc = ca(&mut rc, size);
            let pr = ra(&mut rr, size);
            assert!(!pc.is_null(), "escrypt_alloc_region({size}) failed in C");
            assert!(!pr.is_null(), "escrypt_alloc_region({size}) failed in Rust");
            assert_eq!(pc, rc.aligned);
            assert_eq!(pr, rr.aligned);
            assert_eq!(rc.size, size);
            assert_eq!(rr.size, size);
            // `region->aligned` is at least 64-byte aligned in both builds.
            assert_eq!(rc.aligned as usize % 64, 0, "C region not 64-byte aligned");
            assert_eq!(rr.aligned as usize % 64, 0, "Rust region not 64-byte aligned");
            // The region is writable for its whole length.
            std::ptr::write_bytes(rc.aligned as *mut u8, 0x5a, size);
            std::ptr::write_bytes(rr.aligned as *mut u8, 0x5a, size);

            assert_eq!(cfr(&mut rc), 0);
            assert_eq!(rfr(&mut rr), 0);
            assert!(rc.base.is_null() && rc.size == 0);
            assert!(rr.base.is_null() && rr.size == 0);
            // Idempotent after init_region.
            assert_eq!(cfr(&mut rc), 0);
            assert_eq!(rfr(&mut rr), 0);
            assert_eq!(cfl(&mut rc), 0);
            assert_eq!(rfl(&mut rr), 0);
        }
    }

    // 8.198 / 8.199 an impossible size fails cleanly: NULL, base = NULL,
    // size = 0.  (A 2^62-byte anonymous mapping exceeds the 47-bit user
    // address space, so mmap fails before MAP_POPULATE can touch anything.)
    let mut rc = EscryptRegion::zeroed();
    let mut rr = EscryptRegion::zeroed();
    unsafe {
        assert_eq!(ci(&mut rc), 0);
        assert_eq!(ri(&mut rr), 0);
        let pc = ca(&mut rc, 1usize << 62);
        let pr = ra(&mut rr, 1usize << 62);
        assert_eq!(pc.is_null(), pr.is_null(), "8.198: NULL-ness mismatch");
        assert!(pc.is_null(), "8.198: a 2^62-byte region was allocated?!");
        assert!(rc.base.is_null() && rc.size == 0);
        assert!(rr.base.is_null() && rr.size == 0);
        assert_eq!(cfr(&mut rc), 0);
        assert_eq!(rfr(&mut rr), 0);
    }
}

// ------------------------------------------ 8.120 / 8.121 PBKDF2-SHA256

#[test]
fn r8_120_121_pbkdf2() {
    let (c, r) = both::<Pbkdf2>("_sodium_escrypt_PBKDF2_SHA256");

    // 8.120 known PBKDF2-HMAC-SHA256 vector from RFC 7914 §11.
    {
        let pw = b"passwd";
        let salt = b"salt";
        let want = h(
            "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc\
             49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783",
        );
        let mut oc = padded(64);
        let mut or = padded(64);
        unsafe {
            c(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), 1, oc.as_mut_ptr(), 64);
            r(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), 1, or.as_mut_ptr(), 64);
        }
        eqb("8.120 PBKDF2 C vs Rust", &oc[..64], &or[..64]);
        eqb("8.120 RFC 7914 PBKDF2 vector", &want, &oc[..64]);
    }

    // 8.120 dkLen / passwdlen / saltlen matrix, c = 1 (the only value scrypt
    // itself uses).
    let mut rng = Rng::new(0x8_0120);
    for dklen in [0usize, 1, 32, 33, 64, 100, 128] {
        for pwlen in [0usize, 1, 32] {
            for saltlen in [0usize, 1, 32] {
                let pw = rng.bytes(pwlen);
                let salt = rng.bytes(saltlen);
                let mut oc = padded(dklen);
                let mut or = padded(dklen);
                unsafe {
                    c(pw.as_ptr(), pwlen, salt.as_ptr(), saltlen, 1, oc.as_mut_ptr(), dklen);
                    r(pw.as_ptr(), pwlen, salt.as_ptr(), saltlen, 1, or.as_mut_ptr(), dklen);
                }
                let label = format!("8.120 dkLen={dklen} pwlen={pwlen} saltlen={saltlen}");
                eqb(&label, &oc[..dklen], &or[..dklen]);
                check_pad(&label, &oc, dklen);
                check_pad(&label, &or, dklen);
            }
        }
    }

    // 8.121 c = 2 and c = 4096 exercise the inner U-chain loop that scrypt
    // never uses.
    for iters in [2u64, 4096] {
        let pw = b"password";
        let salt = b"NaCl";
        let mut oc = padded(32);
        let mut or = padded(32);
        unsafe {
            c(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), iters, oc.as_mut_ptr(), 32);
            r(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), iters, or.as_mut_ptr(), 32);
        }
        eqb(&format!("8.121 c={iters}"), &oc[..32], &or[..32]);
        check_pad("8.121", &oc, 32);
    }
}

#[test]
fn r8_201_pbkdf2_dklen_misuse_aborts() {
    // 8.201 dkLen > 0x1fffffffe0 -> sodium_misuse() -> abort().  The check
    // precedes every write, so a tiny output buffer is safe inside the child.
    let (c, r) = both::<Pbkdf2>("_sodium_escrypt_PBKDF2_SHA256");
    for dklen in [0x1fffffffe1usize, usize::MAX] {
        let cc = c.clone();
        let rr = r.clone();
        eq_abort(
            &format!("8.201 escrypt_PBKDF2_SHA256(dkLen={dklen})"),
            move || unsafe {
                let mut o = [0u8; 64];
                cc(b"p".as_ptr(), 1, b"s".as_ptr(), 1, 1, o.as_mut_ptr(), dklen);
            },
            move || unsafe {
                let mut o = [0u8; 64];
                rr(b"p".as_ptr(), 1, b"s".as_ptr(), 1, 1, o.as_mut_ptr(), dklen);
            },
        );
    }
    // dkLen exactly at the limit is *not* rejected by the check (it would only
    // fail on the impossible allocation), so it is not exercised here.
}

// -------------------------------------------- 8.122 escrypt_kdf_nosse direct

#[test]
fn r8_122_kdf_nosse_local_reuse() {
    let (c, r) = both::<KdfNosse>("_sodium_escrypt_kdf_nosse");
    let (ci, ri) = both::<InitLocal>("_sodium_escrypt_init_local");
    let (cf, rf) = both::<FreeLocal>("_sodium_escrypt_free_local");

    let mut lc = EscryptRegion::zeroed();
    let mut lr = EscryptRegion::zeroed();
    unsafe {
        assert_eq!(ci(&mut lc), 0);
        assert_eq!(ri(&mut lr), 0);
    }
    let pw = b"kdf-local";
    let salt = b"kdf-salt";

    // Growing `need` forces the `local->size < need` re-allocation branch;
    // shrinking `need` must reuse the region without re-allocating.
    let plan: &[(u64, u32, u32)] = &[
        (2, 1, 1),      // need = 128*1*1 + 128*1*2 + 256+64 = 704
        (16, 1, 1),     // grows
        (1024, 8, 1),   // grows a lot
        (16, 1, 1),     // shrinks -> region reused
        (2, 1, 1),      // shrinks -> region reused
        (1024, 8, 2),   // grows again
    ];
    let mut prev_c = 0usize;
    let mut sizes_c: Vec<usize> = Vec::new();
    for (i, &(n, rr_, p)) in plan.iter().enumerate() {
        let mut bc = padded(64);
        let mut br = padded(64);
        let (a, b) = unsafe {
            (
                c(&mut lc, pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), n, rr_, p, bc.as_mut_ptr(), 64),
                r(&mut lr, pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), n, rr_, p, br.as_mut_ptr(), 64),
            )
        };
        eqi(&format!("8.122 step#{i} ret"), a, b);
        assert_eq!(a, 0);
        eqb(&format!("8.122 step#{i} out"), &bc[..64], &br[..64]);
        check_pad("8.122(C)", &bc, 64);
        check_pad("8.122(Rust)", &br, 64);
        assert_eq!(lc.size, lr.size, "8.122 step#{i}: local->size mismatch");
        sizes_c.push(lc.size);
        // The region only ever grows.
        assert!(lc.size >= prev_c, "8.122 step#{i}: region shrank");
        prev_c = lc.size;
    }
    // Steps 3 and 4 shrink `need`, so the region must be unchanged there.
    assert_eq!(sizes_c[2], sizes_c[3], "8.122: region re-allocated on a shrink");
    assert_eq!(sizes_c[3], sizes_c[4], "8.122: region re-allocated on a shrink");

    // Reproducibility across a fresh local.
    let mut bc = padded(64);
    let mut br = padded(64);
    unsafe {
        assert_eq!(cf(&mut lc), 0);
        assert_eq!(rf(&mut lr), 0);
        assert_eq!(ci(&mut lc), 0);
        assert_eq!(ri(&mut lr), 0);
        assert_eq!(
            c(&mut lc, pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), 1024, 8, 2, bc.as_mut_ptr(), 64),
            0
        );
        assert_eq!(
            r(&mut lr, pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), 1024, 8, 2, br.as_mut_ptr(), 64),
            0
        );
        assert_eq!(cf(&mut lc), 0);
        assert_eq!(rf(&mut lr), 0);
    }
    eqb("8.122 fresh-local reproducibility", &bc[..64], &br[..64]);
}

// ================================= error surface ==========================

// -------------------------- 8.151 / 8.153 high-level rejections

#[test]
fn r8_151_153_high_level_errors() {
    let (c, r) = both::<Scrypt>("crypto_pwhash_scryptsalsa208sha256");
    let salt = vec![0x11u8; SALTBYTES];

    // 8.151 outlen < BYTES_MIN (16).
    for outlen in [0usize, 1, 15] {
        let (rc, ec, _) = hi(&c, &r, &format!("8.151 outlen={outlen}"), outlen, b"pw", &salt, 32768, 16777216);
        assert_eq!(rc, -1, "8.151 outlen={outlen}");
        assert_eq!(ec, EINVAL, "8.151 outlen={outlen}: errno");
    }

    // 8.153 out aliases passwd.
    let mut buf_c = vec![0x22u8; 64];
    let mut buf_r = vec![0x22u8; 64];
    set_errno(SENTINEL);
    let rc = unsafe {
        c(
            buf_c.as_mut_ptr(),
            32,
            buf_c.as_ptr() as *const c_char,
            32,
            salt.as_ptr(),
            32768,
            16777216,
        )
    };
    let ec = errno();
    set_errno(SENTINEL);
    let rr = unsafe {
        r(
            buf_r.as_mut_ptr(),
            32,
            buf_r.as_ptr() as *const c_char,
            32,
            salt.as_ptr(),
            32768,
            16777216,
        )
    };
    let er = errno();
    eqi("8.153 out == passwd", rc, rr);
    assert_eq!(rc, -1);
    assert_eq!(ec, EINVAL);
    assert_eq!(er, ec);
    eqb("8.153 buffer state", &buf_c, &buf_r);
}

// ----------------------------- 8.161 – 8.163 / 8.166 – 8.168 string errors

#[test]
fn r8_161_to_168_string_errors() {
    let _rng = rng_guard();
    let (cv, rv) = both::<ScryptStrVerify>("crypto_pwhash_scryptsalsa208sha256_str_verify");
    let (cn, rn) = both::<ScryptNeedsRehash>("crypto_pwhash_scryptsalsa208sha256_str_needs_rehash");
    let (cs, _) = both::<ScryptStr>("crypto_pwhash_scryptsalsa208sha256_str");

    rng_reset();
    let mut out = padded(STRBYTES);
    let pw = b"password";
    assert_eq!(
        unsafe {
            cs(
                out.as_mut_ptr() as *mut c_char,
                pw.as_ptr() as *const c_char,
                pw.len() as u64,
                32768,
                16777216,
            )
        },
        0
    );
    let good = as_str(&out[..STRBYTES]);
    assert_eq!(good.len(), 101);

    // 8.161 / 8.166 sodium_strnlen(str, 102) != 101.
    let mut bad_len: Vec<String> = vec![
        String::new(),
        good[..100].to_string(),
        good[..50].to_string(),
        format!("{good}x"),          // 102 chars
        format!("{good}xxxxxxxxxx"), // longer than the window
    ];
    // 8.162 / 8.167 101-char strings with a malformed setting.
    let mut bad_setting: Vec<String> = Vec::new();
    bad_setting.push(format!("$6${}", &good[3..])); // wrong prefix
    bad_setting.push(format!("7${}", &good[2..])); // no leading '$'
    bad_setting.push(format!("$7${}{}", "$", &good[4..])); // N_log2 char not in itoa64
    bad_setting.push(format!("$7${}{}", "-", &good[4..]));
    bad_setting.push(format!("$7${}{}", "*", &good[4..]));
    bad_setting.push(format!("{}${}", &good[..5], &good[6..])); // '$' inside the r field
    bad_setting.push(format!("{}${}", &good[..10], &good[11..])); // '$' inside the p field
    for s in &bad_setting {
        assert_eq!(s.len(), 101, "malformed setting must still be 101 chars: {s:?}");
    }
    bad_len.append(&mut bad_setting.clone());

    for s in bad_len.iter().chain(bad_setting.iter()) {
        let sz = cstr(s);
        let gc = unsafe {
            cv(sz.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64)
        };
        let gr = unsafe {
            rv(sz.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, pw.len() as u64)
        };
        eqi(&format!("8.161/8.162 str_verify({s:?})"), gc, gr);
        assert_eq!(gc, -1, "8.161/8.162: {s:?} should be rejected");

        set_errno(SENTINEL);
        let nc = unsafe { cn(sz.as_ptr() as *const c_char, 32768, 16777216) };
        let ec = errno();
        set_errno(SENTINEL);
        let nr = unsafe { rn(sz.as_ptr() as *const c_char, 32768, 16777216) };
        let er = errno();
        eqi(&format!("8.166/8.167 needs_rehash({s:?})"), nc, nr);
        assert_eq!(nc, -1, "8.166/8.167: {s:?} should be rejected");
        assert_eq!(ec, EINVAL, "8.166/8.167 errno for {s:?}");
        assert_eq!(er, ec);
    }

    // 8.163 wrong password on a well-formed string.
    for wrong in [b"".to_vec(), b"Password".to_vec(), b"password ".to_vec()] {
        let sz = cstr(&good);
        let gc = unsafe {
            cv(sz.as_ptr() as *const c_char, wrong.as_ptr() as *const c_char, wrong.len() as u64)
        };
        let gr = unsafe {
            rv(sz.as_ptr() as *const c_char, wrong.as_ptr() as *const c_char, wrong.len() as u64)
        };
        eqi("8.163 wrong password", gc, gr);
        assert_eq!(gc, -1);
    }

    // 8.168 valid string, parameters differ -> 1 (non-zero, non-error).
    let sz = cstr(&good);
    for (ops, mem) in [(524288u64, 16777216usize), (32768, 524288), (32768, 0), (0, 0)] {
        let nc = unsafe { cn(sz.as_ptr() as *const c_char, ops, mem) };
        let nr = unsafe { rn(sz.as_ptr() as *const c_char, ops, mem) };
        eqi(&format!("8.168 needs_rehash({ops}, {mem})"), nc, nr);
        assert!(nc == 0 || nc == 1, "8.168: unexpected {nc}");
    }
}

// -------------------------------- 8.169 – 8.180 `_ll` rejections

#[test]
fn r8_169_to_180_ll_errors() {
    let (c, r) = both::<Ll>("crypto_pwhash_scryptsalsa208sha256_ll");
    let pw = b"pw".to_vec();
    let salt = b"salt".to_vec();

    // Every one of these checks precedes any write to `buf`, so a 64-byte
    // output buffer is safe even for absurd `buflen` values.
    let cases: &[(&str, u64, u32, u32, usize, c_int)] = &[
        // 8.169 buflen > (2^32-1)*32
        ("8.169 buflen=137438953441", 16, 1, 1, 137438953441, 27 /* EFBIG */),
        ("8.169 buflen=SIZE_MAX", 16, 1, 1, usize::MAX, 27),
        // 8.170 r * p >= 2^30
        ("8.170 r=1,p=2^30", 16, 1, 1073741824, 32, 27),
        ("8.170 r=32768,p=32768", 16, 32768, 32768, 32, 27),
        ("8.170 r=2,p=2^29", 16, 2, 536870912, 32, 27),
        // 8.171 N > UINT32_MAX
        ("8.171 N=2^32", 1u64 << 32, 1, 1, 32, 27),
        ("8.171 N=2^40", 1u64 << 40, 1, 1, 32, 27),
        // 8.172 N not a power of two
        ("8.172 N=3", 3, 1, 1, 32, EINVAL),
        ("8.172 N=1000", 1000, 1, 1, 32, EINVAL),
        ("8.172 N=1023", 1023, 1, 1, 32, EINVAL),
        // 8.173 N < 2 (N = 0 also passes the power-of-two test)
        ("8.173 N=0", 0, 1, 1, 32, EINVAL),
        ("8.173 N=1", 1, 1, 1, 32, EINVAL),
        // 8.174 / 8.175 r == 0 / p == 0
        ("8.174 r=0", 16, 0, 1, 32, EINVAL),
        ("8.175 p=0", 16, 1, 0, 32, EINVAL),
        ("8.174+8.175 r=0,p=0", 16, 0, 0, 32, EINVAL),
        // 8.178 N > SIZE_MAX / 128 / r
        ("8.178 r=2^30-1,N=2^28", 1u64 << 28, 1073741823, 1, 32, ENOMEM),
        // 8.179 need = B_size + V_size wraps
        ("8.179 r=2^30-1,N=2^27", 1u64 << 27, 1073741823, 1, 32, ENOMEM),
        // 8.180 need += XY_size wraps
        ("8.180 r=536870910,N=2^28", 1u64 << 28, 536870910, 1, 32, ENOMEM),
    ];
    for &(label, n, rr, p, buflen, want_errno) in cases {
        let mut bc = padded(64);
        let mut br = padded(64);
        set_errno(SENTINEL);
        let a = unsafe {
            c(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), n, rr, p, bc.as_mut_ptr(), buflen)
        };
        let ec = errno();
        set_errno(SENTINEL);
        let b = unsafe {
            r(pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), n, rr, p, br.as_mut_ptr(), buflen)
        };
        let er = errno();
        eqi(&format!("{label} ret"), a, b);
        assert_eq!(a, -1, "{label}: expected -1");
        assert_eq!(ec, want_errno, "{label}: C errno {ec}, expected {want_errno}");
        assert_eq!(er, ec, "{label}: Rust errno {er}, C {ec}");
        // Nothing was written.
        check_pad(label, &bc, 64);
        check_pad(label, &br, 64);
        assert!(bc[..64].iter().all(|&x| x == 0), "{label}: C wrote to buf");
        assert!(br[..64].iter().all(|&x| x == 0), "{label}: Rust wrote to buf");
    }
}

// ---------------------------- 8.182 – 8.185 parse_setting rejections

/// `escrypt_parse_setting` reads a *fixed* 11 characters after `"$7$"`, and
/// `decode64_one` is implemented with `strchr(itoa64, c)`, which also matches
/// the terminating NUL of `itoa64` (yielding the out-of-range value 64).  A
/// setting that ends early therefore makes the C code read past its NUL, so
/// every setting handed to `parse_setting` below is materialised in a
/// fixed-size buffer whose bytes past the terminator are the non-itoa64
/// filler `'!'` — that makes the behaviour fully determined for both
/// implementations.
fn setting_buf(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v.resize(64, b'!');
    v
}

#[test]
fn r8_182_to_185_parse_setting_errors() {
    let (cp, rp) = both::<ParseSetting>("_sodium_escrypt_parse_setting");
    // A valid 57-char setting to mutate.
    let (cg, _) = both::<GensaltR>("_sodium_escrypt_gensalt_r");
    let salt32 = vec![0x33u8; 32];
    let mut sc = padded(58);
    unsafe {
        assert!(!cg(10, 8, 1, salt32.as_ptr(), 32, sc.as_mut_ptr(), 58).is_null());
    }
    let good = as_str(&sc[..58]);

    let mut bad: Vec<String> = vec![
        // 8.182 prefix
        String::new(),
        "$".to_string(),
        "$7".to_string(),
        "$6$abcdefghijkl".to_string(),
        "7$abcdefghijkl".to_string(),
        format!("$8${}", &good[3..]),
        format!("${}", &good[2..]),
        // 8.183 N_log2 char not in itoa64
        format!("$7$${}", &good[4..]),
        format!("$7$-{}", &good[4..]),
        format!("$7$*{}", &good[4..]),
        // A setting that stops right after "$7$": decode64_one accepts the NUL
        // (strchr quirk, value 64) and the reader then hits the '!' filler.
        "$7$".to_string(),
    ];
    // 8.184 a non-itoa64 char inside the 5-char r field.
    for i in 4..9 {
        let mut s = good.clone().into_bytes();
        s[i] = b'$';
        bad.push(String::from_utf8(s).unwrap());
    }
    // 8.185 a non-itoa64 char inside the 5-char p field.
    for i in 9..14 {
        let mut s = good.clone().into_bytes();
        s[i] = b'!';
        bad.push(String::from_utf8(s).unwrap());
    }
    // 8.184 / 8.185 strings that end early inside the `r` or `p` field.  Note
    // that a truncation which lands exactly on the *last* character of a field
    // still parses, because `decode64_one` accepts the NUL as value 64; only
    // the following `'!'` filler byte makes the reader fail.  These cases are
    // therefore compared differentially and only the ones where the C code
    // rejects are asserted to be NULL.
    let truncated: Vec<String> = (4..14).map(|i| good[..i].to_string()).collect();

    for s in &bad {
        let sz = setting_buf(s);
        let mut a = [0xdeadbeefu32; 3];
        let mut b = [0xdeadbeefu32; 3];
        let (qc, qr) = unsafe {
            (
                cp(sz.as_ptr(), &mut a[0], &mut a[1], &mut a[2]),
                rp(sz.as_ptr(), &mut b[0], &mut b[1], &mut b[2]),
            )
        };
        assert!(qc.is_null(), "8.182-8.185: {s:?} should not parse");
        assert_eq!(qc.is_null(), qr.is_null(), "8.182-8.185: {s:?} NULL-ness mismatch");
        assert_eq!(a, b, "8.182-8.185: {s:?} out-parameter mismatch {a:?} vs {b:?}");
    }

    let mut rejected = 0usize;
    for s in &truncated {
        let sz = setting_buf(s);
        let mut a = [0xdeadbeefu32; 3];
        let mut b = [0xdeadbeefu32; 3];
        let (qc, qr) = unsafe {
            (
                cp(sz.as_ptr(), &mut a[0], &mut a[1], &mut a[2]),
                rp(sz.as_ptr(), &mut b[0], &mut b[1], &mut b[2]),
            )
        };
        assert_eq!(
            qc.is_null(),
            qr.is_null(),
            "8.184/8.185: truncated {s:?} NULL-ness mismatch"
        );
        if !qc.is_null() {
            assert_eq!(
                qc as usize - sz.as_ptr() as usize,
                qr as usize - sz.as_ptr() as usize,
                "8.184/8.185: truncated {s:?} end-pointer mismatch"
            );
        } else {
            rejected += 1;
        }
        assert_eq!(a, b, "8.184/8.185: truncated {s:?} out-parameters {a:?} vs {b:?}");
    }
    assert!(rejected >= 8, "8.184/8.185: expected most truncations to be rejected");
}

// ------------------------- 8.186 / 8.189 / 8.190 gensalt rejections

#[test]
fn r8_186_189_190_gensalt_errors() {
    let (cg, rg) = both::<GensaltR>("_sodium_escrypt_gensalt_r");
    let src = vec![0x44u8; 32];

    // 8.186 need > buflen (need = 58 for srclen = 32).
    for buflen in [0usize, 1, 14, 57] {
        let mut bc = padded(buflen.max(1));
        let mut br = padded(buflen.max(1));
        let (pc, pr) = unsafe {
            (
                cg(10, 8, 1, src.as_ptr(), 32, bc.as_mut_ptr(), buflen),
                rg(10, 8, 1, src.as_ptr(), 32, br.as_mut_ptr(), buflen),
            )
        };
        assert!(pc.is_null(), "8.186 buflen={buflen}: should fail");
        assert_eq!(pc.is_null(), pr.is_null(), "8.186 buflen={buflen}");
    }

    // 8.189 N_log2 > 63.
    for nl in [64u32, 65, 255, u32::MAX] {
        let mut bc = padded(58);
        let mut br = padded(58);
        let (pc, pr) = unsafe {
            (
                cg(nl, 8, 1, src.as_ptr(), 32, bc.as_mut_ptr(), 58),
                rg(nl, 8, 1, src.as_ptr(), 32, br.as_mut_ptr(), 58),
            )
        };
        assert!(pc.is_null(), "8.189 N_log2={nl}: should fail");
        assert_eq!(pc.is_null(), pr.is_null(), "8.189 N_log2={nl}");
    }

    // 8.190 r * p >= 2^30.
    for (rr, p) in [(1u32, 1073741824u32), (2, 536870912), (32768, 32768), (u32::MAX, 1)] {
        let mut bc = padded(58);
        let mut br = padded(58);
        let (pc, pr) = unsafe {
            (
                cg(10, rr, p, src.as_ptr(), 32, bc.as_mut_ptr(), 58),
                rg(10, rr, p, src.as_ptr(), 32, br.as_mut_ptr(), 58),
            )
        };
        assert!(pc.is_null(), "8.190 r={rr} p={p}: should fail");
        assert_eq!(pc.is_null(), pr.is_null(), "8.190 r={rr} p={p}");
    }
}

// -------------------- 8.192 / 8.193 / 8.194 / 8.196 escrypt_r rejections

#[test]
fn r8_192_to_196_escrypt_r_errors() {
    let _rng = rng_guard();
    let (cr, rr) = both::<EscryptR>("_sodium_escrypt_r");
    let (cg, _) = both::<GensaltR>("_sodium_escrypt_gensalt_r");
    let (ci, ri) = both::<InitLocal>("_sodium_escrypt_init_local");
    let (cf, rf) = both::<FreeLocal>("_sodium_escrypt_free_local");
    let pw = b"password".to_vec();
    let salt32 = vec![0x55u8; 32];

    let mut lc = EscryptRegion::zeroed();
    let mut lr = EscryptRegion::zeroed();
    unsafe {
        assert_eq!(ci(&mut lc), 0);
        assert_eq!(ri(&mut lr), 0);
    }

    let mut sc = padded(58);
    unsafe {
        assert!(!cg(10, 8, 1, salt32.as_ptr(), 32, sc.as_mut_ptr(), 58).is_null());
    }
    let good = cstr(&as_str(&sc[..58]));

    // 8.192 escrypt_parse_setting failure; note randombytes_buf(buf, buflen)
    // has already scrambled the caller's buffer, so the buffers are compared
    // from a common RNG rewind.
    for bad in ["", "$6$xxxxxxxxxxx", "$7$$abcdefghij", "$7$A$bcdefghij"] {
        let sz = cstr(bad);
        let mut bc = padded(STRBYTES);
        let mut br = padded(STRBYTES);
        rng_reset();
        let pc = unsafe { cr(&mut lc, pw.as_ptr(), pw.len(), sz.as_ptr(), bc.as_mut_ptr(), STRBYTES) };
        let pr = unsafe { rr(&mut lr, pw.as_ptr(), pw.len(), sz.as_ptr(), br.as_mut_ptr(), STRBYTES) };
        assert!(pc.is_null(), "8.192 {bad:?}: should fail");
        assert_eq!(pc.is_null(), pr.is_null(), "8.192 {bad:?}");
        eqb(&format!("8.192 scrambled buf {bad:?}"), &bc[..STRBYTES], &br[..STRBYTES]);
        check_pad("8.192", &bc, STRBYTES);
        check_pad("8.192", &br, STRBYTES);
    }

    // 8.193 buf == NULL.
    let pc = unsafe {
        cr(&mut lc, pw.as_ptr(), pw.len(), good.as_ptr(), std::ptr::null_mut(), STRBYTES)
    };
    let pr = unsafe {
        rr(&mut lr, pw.as_ptr(), pw.len(), good.as_ptr(), std::ptr::null_mut(), STRBYTES)
    };
    assert!(pc.is_null() && pr.is_null(), "8.193: buf == NULL must return NULL");

    // 8.194 need = 14 + 43 + 1 + 43 + 1 = 102 > buflen.
    for buflen in [0usize, 1, 58, 101] {
        let mut bc = padded(buflen.max(1));
        let mut br = padded(buflen.max(1));
        rng_reset();
        let pc = unsafe { cr(&mut lc, pw.as_ptr(), pw.len(), good.as_ptr(), bc.as_mut_ptr(), buflen) };
        let pr = unsafe { rr(&mut lr, pw.as_ptr(), pw.len(), good.as_ptr(), br.as_mut_ptr(), buflen) };
        assert!(pc.is_null(), "8.194 buflen={buflen}: should fail");
        assert_eq!(pc.is_null(), pr.is_null(), "8.194 buflen={buflen}");
        eqb(&format!("8.194 buf buflen={buflen}"), &bc[..buflen], &br[..buflen]);
    }

    // 8.196 escrypt_kdf failure through the setting: N_log2 = 0 -> N = 1 < 2,
    // and N_log2 = 63 -> N > UINT32_MAX.
    for nl in [0u32, 63] {
        let mut s = padded(58);
        unsafe {
            assert!(!cg(nl, 8, 1, salt32.as_ptr(), 32, s.as_mut_ptr(), 58).is_null());
        }
        let sz = cstr(&as_str(&s[..58]));
        let mut bc = padded(STRBYTES);
        let mut br = padded(STRBYTES);
        rng_reset();
        let pc = unsafe { cr(&mut lc, pw.as_ptr(), pw.len(), sz.as_ptr(), bc.as_mut_ptr(), STRBYTES) };
        let pr = unsafe { rr(&mut lr, pw.as_ptr(), pw.len(), sz.as_ptr(), br.as_mut_ptr(), STRBYTES) };
        assert!(pc.is_null(), "8.196 N_log2={nl}: should fail");
        assert_eq!(pc.is_null(), pr.is_null(), "8.196 N_log2={nl}");
        eqb(&format!("8.196 buf N_log2={nl}"), &bc[..STRBYTES], &br[..STRBYTES]);
    }
    // ... and r = 0 / p = 0 cannot be produced by gensalt, but can be written
    // by hand: itoa64[0] = '.', so "00000" encodes 0.
    for (field, s) in [
        ("r=0", format!("$7$A{}{}", ".....", "....0")),
        ("p=0", format!("$7$A{}{}", "....0", ".....")),
    ] {
        // Append a salt so that `need` fits into 102 bytes.
        let full = format!("{s}{}", "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopq");
        let sz = cstr(&full);
        let mut bc = padded(STRBYTES);
        let mut br = padded(STRBYTES);
        rng_reset();
        let pc = unsafe { cr(&mut lc, pw.as_ptr(), pw.len(), sz.as_ptr(), bc.as_mut_ptr(), STRBYTES) };
        let pr = unsafe { rr(&mut lr, pw.as_ptr(), pw.len(), sz.as_ptr(), br.as_mut_ptr(), STRBYTES) };
        assert_eq!(pc.is_null(), pr.is_null(), "8.196 {field}");
        assert!(pc.is_null(), "8.196 {field}: should fail");
        eqb(&format!("8.196 buf {field}"), &bc[..STRBYTES], &br[..STRBYTES]);
    }

    unsafe {
        assert_eq!(cf(&mut lc), 0);
        assert_eq!(rf(&mut lr), 0);
    }
}

#[test]
fn r8_108_sensitive_preset() {
    // (SLOW) 8.108 the documented SENSITIVE preset: opslimit = 33554432,
    // memlimit = 1 GiB.  `pickparams` takes the second branch (opslimit ==
    // memlimit/32) and yields N = 2^20, r = 8, p = 1, i.e. a 1 GiB region.
    // Run once, and only here, so it can be skipped with `--skip r8_108`.
    let (c, r) = both::<Scrypt>("crypto_pwhash_scryptsalsa208sha256");
    let salt = vec![0x77u8; SALTBYTES];
    let (rc, _, _) = hi(&c, &r, "8.108 sensitive", 32, b"password", &salt, 33554432, 1073741824);
    assert_eq!(rc, 0);
}
