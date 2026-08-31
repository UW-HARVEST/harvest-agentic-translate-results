//! Area 8 (part 1a) — `crypto_pwhash/crypto_pwhash.c`,
//! `crypto_pwhash/argon2/{pwhash_argon2i.c, pwhash_argon2id.c}`: the public
//! `crypto_pwhash*` surface (constant getters, raw KDF, `$argon2*$` strings).
//!
//! Covers `configs_8.md` rows 8.1 – 8.36 and `errors_8.md` rows 8.1 – 8.52.
//!
//! **Speed:** every argon2 call here uses `memlimit = 8192` (`m_cost = 8`, the
//! documented `MEMLIMIT_MIN`) and `opslimit` at the per-algorithm
//! `OPSLIMIT_MIN` (3 for argon2i, 1 for argon2id) unless a row explicitly
//! needs another value.  The documented INTERACTIVE presets live in their own
//! `#[test]` so they can be skipped independently.
mod common;
use common::*;
use std::ffi::{c_char, c_int};
use std::sync::{Mutex, MutexGuard};

/// Tests inside one binary run in parallel threads and `rng_reset()` rewinds a
/// *process-global* stream, so every test that depends on the deterministic RNG
/// holds this lock while it needs a stable stream position.
static RNG_LOCK: Mutex<()> = Mutex::new(());

fn rng_guard() -> MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ------------------------------------------------------------------- types

type IntGetter = unsafe extern "C" fn() -> c_int;
type SizeGetter = unsafe extern "C" fn() -> usize;
type UllGetter = unsafe extern "C" fn() -> u64;
type StrGetter = unsafe extern "C" fn() -> *const c_char;

/// `crypto_pwhash` / `crypto_pwhash_argon2i` / `crypto_pwhash_argon2id`
type Pwhash = unsafe extern "C" fn(
    *mut u8,       // out
    u64,           // outlen
    *const c_char, // passwd
    u64,           // passwdlen
    *const u8,     // salt (SALTBYTES = 16)
    u64,           // opslimit
    usize,         // memlimit
    c_int,         // alg
) -> c_int;

/// `crypto_pwhash_str` / `crypto_pwhash_argon2i_str` / `..._argon2id_str`
type PwStr = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize) -> c_int;
/// `crypto_pwhash_str_alg`
type PwStrAlg =
    unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize, c_int) -> c_int;
type StrVerify = unsafe extern "C" fn(*const c_char, *const c_char, u64) -> c_int;
type NeedsRehash = unsafe extern "C" fn(*const c_char, u64, usize) -> c_int;

// ----------------------------------------------------------------- helpers

const SALTBYTES: usize = 16;
const STRBYTES: usize = 128;
const SENTINEL: c_int = 0x5EED;
const EFBIG: c_int = 27;

const ALG_I: c_int = 1;
const ALG_ID: c_int = 2;

/// Base64, `sodium_base64_VARIANT_ORIGINAL_NO_PADDING` — the variant the
/// argon2 encoder uses.
fn b64(v: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    let mut i = 0;
    while i + 3 <= v.len() {
        let n = ((v[i] as u32) << 16) | ((v[i + 1] as u32) << 8) | v[i + 2] as u32;
        s.push(A[(n >> 18) as usize & 63] as char);
        s.push(A[(n >> 12) as usize & 63] as char);
        s.push(A[(n >> 6) as usize & 63] as char);
        s.push(A[n as usize & 63] as char);
        i += 3;
    }
    match v.len() - i {
        1 => {
            let n = (v[i] as u32) << 16;
            s.push(A[(n >> 18) as usize & 63] as char);
            s.push(A[(n >> 12) as usize & 63] as char);
        }
        2 => {
            let n = ((v[i] as u32) << 16) | ((v[i + 1] as u32) << 8);
            s.push(A[(n >> 18) as usize & 63] as char);
            s.push(A[(n >> 12) as usize & 63] as char);
            s.push(A[(n >> 6) as usize & 63] as char);
        }
        _ => {}
    }
    s
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

/// One `crypto_pwhash*` call on each library; compares the return code, errno
/// and the whole output buffer (including the guard region).
#[track_caller]
fn pw(
    c: &Pwhash,
    r: &Pwhash,
    label: &str,
    outlen: usize,
    passwd: &[u8],
    passwdlen: Option<u64>,
    salt: &[u8],
    ops: u64,
    mem: usize,
    alg: c_int,
) -> (c_int, c_int, Vec<u8>) {
    let pwlen = passwdlen.unwrap_or(passwd.len() as u64);
    let mut oc = padded(outlen);
    let mut or = padded(outlen);
    set_errno(SENTINEL);
    let rc = unsafe {
        c(
            oc.as_mut_ptr(),
            outlen as u64,
            passwd.as_ptr() as *const c_char,
            pwlen,
            salt.as_ptr(),
            ops,
            mem,
            alg,
        )
    };
    let ec = errno();
    set_errno(SENTINEL);
    let rr = unsafe {
        r(
            or.as_mut_ptr(),
            outlen as u64,
            passwd.as_ptr() as *const c_char,
            pwlen,
            salt.as_ptr(),
            ops,
            mem,
            alg,
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

/// One `*_str` call on each library from a common RNG rewind (the random
/// 16-byte salt has to match, so the whole encoded string must match).
#[track_caller]
fn pwstr(c: &PwStr, r: &PwStr, label: &str, passwd: &[u8], ops: u64, mem: usize) -> (c_int, c_int, String) {
    let mut sc = padded(STRBYTES);
    let mut sr = padded(STRBYTES);
    rng_reset();
    set_errno(SENTINEL);
    let rc = unsafe {
        c(
            sc.as_mut_ptr() as *mut c_char,
            passwd.as_ptr() as *const c_char,
            passwd.len() as u64,
            ops,
            mem,
        )
    };
    let ec = errno();
    set_errno(SENTINEL);
    let rr = unsafe {
        r(
            sr.as_mut_ptr() as *mut c_char,
            passwd.as_ptr() as *const c_char,
            passwd.len() as u64,
            ops,
            mem,
        )
    };
    let er = errno();
    eqi(&format!("{label} ret"), rc, rr);
    assert_eq!(ec, er, "{label}: errno mismatch (C {ec}, Rust {er})");
    eqb(&format!("{label} str"), &sc[..STRBYTES], &sr[..STRBYTES]);
    check_pad(&format!("{label}(C)"), &sc, STRBYTES);
    check_pad(&format!("{label}(Rust)"), &sr, STRBYTES);
    (rc, ec, as_str(&sc[..STRBYTES]))
}

/// A canonical argon2 encoded string built by hand.
fn make_str(kind: &str, m: u32, t: u32, p: u32, salt: &[u8], hash: &[u8]) -> String {
    format!(
        "${kind}$v=19$m={m},t={t},p={p}${}${}",
        b64(salt),
        b64(hash)
    )
}

// ============================== 8.1 – 8.5 constants ======================

#[test]
fn r8_1_alg_identifiers() {
    for (name, want) in [
        ("crypto_pwhash_alg_argon2i13", 1),
        ("crypto_pwhash_alg_argon2id13", 2),
        ("crypto_pwhash_alg_default", 2),
        ("crypto_pwhash_argon2i_alg_argon2i13", 1),
        ("crypto_pwhash_argon2id_alg_argon2id13", 2),
    ] {
        let (c, r) = both::<IntGetter>(name);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, want, "{name}: C returned {cv}, expected {want}");
        eqi(name, cv, rv);
    }
    // ALG_DEFAULT is an alias of ALG_ARGON2ID13.
    let (c, _) = both::<IntGetter>("crypto_pwhash_alg_default");
    let (c2, _) = both::<IntGetter>("crypto_pwhash_alg_argon2id13");
    assert_eq!(unsafe { c() }, unsafe { c2() });
}

#[test]
fn r8_2_to_5_size_and_limit_getters() {
    let sizes: &[(&str, usize)] = &[
        // 8.2 generic (aliases of the argon2id values)
        ("crypto_pwhash_bytes_min", 16),
        ("crypto_pwhash_bytes_max", 4294967295),
        ("crypto_pwhash_passwd_min", 0),
        ("crypto_pwhash_passwd_max", 4294967295),
        ("crypto_pwhash_saltbytes", 16),
        ("crypto_pwhash_strbytes", 128),
        ("crypto_pwhash_memlimit_min", 8192),
        ("crypto_pwhash_memlimit_max", 4398046510080),
        ("crypto_pwhash_memlimit_interactive", 67108864),
        ("crypto_pwhash_memlimit_moderate", 268435456),
        ("crypto_pwhash_memlimit_sensitive", 1073741824),
        // 8.4 argon2i
        ("crypto_pwhash_argon2i_bytes_min", 16),
        ("crypto_pwhash_argon2i_bytes_max", 4294967295),
        ("crypto_pwhash_argon2i_passwd_min", 0),
        ("crypto_pwhash_argon2i_passwd_max", 4294967295),
        ("crypto_pwhash_argon2i_saltbytes", 16),
        ("crypto_pwhash_argon2i_strbytes", 128),
        ("crypto_pwhash_argon2i_memlimit_min", 8192),
        ("crypto_pwhash_argon2i_memlimit_max", 4398046510080),
        ("crypto_pwhash_argon2i_memlimit_interactive", 33554432),
        ("crypto_pwhash_argon2i_memlimit_moderate", 134217728),
        ("crypto_pwhash_argon2i_memlimit_sensitive", 536870912),
        // 8.5 argon2id
        ("crypto_pwhash_argon2id_bytes_min", 16),
        ("crypto_pwhash_argon2id_bytes_max", 4294967295),
        ("crypto_pwhash_argon2id_passwd_min", 0),
        ("crypto_pwhash_argon2id_passwd_max", 4294967295),
        ("crypto_pwhash_argon2id_saltbytes", 16),
        ("crypto_pwhash_argon2id_strbytes", 128),
        ("crypto_pwhash_argon2id_memlimit_min", 8192),
        ("crypto_pwhash_argon2id_memlimit_max", 4398046510080),
        ("crypto_pwhash_argon2id_memlimit_interactive", 67108864),
        ("crypto_pwhash_argon2id_memlimit_moderate", 268435456),
        ("crypto_pwhash_argon2id_memlimit_sensitive", 1073741824),
    ];
    for (name, want) in sizes {
        let (c, r) = both::<SizeGetter>(name);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, *want, "{name}: C returned {cv}, expected {want}");
        assert_eq!(rv, cv, "{name}: Rust {rv} != C {cv}");
    }

    let ulls: &[(&str, u64)] = &[
        // 8.3 generic
        ("crypto_pwhash_opslimit_min", 1),
        ("crypto_pwhash_opslimit_max", 4294967295),
        ("crypto_pwhash_opslimit_interactive", 2),
        ("crypto_pwhash_opslimit_moderate", 3),
        ("crypto_pwhash_opslimit_sensitive", 4),
        // 8.4 argon2i
        ("crypto_pwhash_argon2i_opslimit_min", 3),
        ("crypto_pwhash_argon2i_opslimit_max", 4294967295),
        ("crypto_pwhash_argon2i_opslimit_interactive", 4),
        ("crypto_pwhash_argon2i_opslimit_moderate", 6),
        ("crypto_pwhash_argon2i_opslimit_sensitive", 8),
        // 8.5 argon2id
        ("crypto_pwhash_argon2id_opslimit_min", 1),
        ("crypto_pwhash_argon2id_opslimit_max", 4294967295),
        ("crypto_pwhash_argon2id_opslimit_interactive", 2),
        ("crypto_pwhash_argon2id_opslimit_moderate", 3),
        ("crypto_pwhash_argon2id_opslimit_sensitive", 4),
    ];
    for (name, want) in ulls {
        let (c, r) = both::<UllGetter>(name);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, *want, "{name}: C returned {cv}, expected {want}");
        assert_eq!(rv, cv, "{name}: Rust {rv} != C {cv}");
    }

    for (name, want) in [
        ("crypto_pwhash_strprefix", "$argon2id$"),
        ("crypto_pwhash_argon2i_strprefix", "$argon2i$"),
        ("crypto_pwhash_argon2id_strprefix", "$argon2id$"),
        ("crypto_pwhash_primitive", "argon2id,argon2i"),
    ] {
        let (c, r) = both::<StrGetter>(name);
        unsafe {
            let cs = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
            let rs = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
            eqb(name, &cs, &rs);
            assert_eq!(cs, want.as_bytes(), "{name}: C returned {:?}", as_str(&cs));
        }
    }
}

// ======================= 8.6 – 8.18 / 8.21 / 8.22 crypto_pwhash ==========

#[test]
fn r8_6_to_14_pwhash_parameter_matrix() {
    let (c, r) = both::<Pwhash>("crypto_pwhash");
    let mut rng = Rng::new(0x8_0006);
    let salt = rng.bytes(SALTBYTES);

    // 8.6 / 8.7 / 8.8 / 8.9 argon2i
    let cases: &[(&str, u64, usize, usize, c_int)] = &[
        ("8.6  i  ops=3 mem=8192 out=16", 3, 8192, 16, ALG_I),
        ("8.7  i  ops=3 mem=8192 out=32", 3, 8192, 32, ALG_I),
        ("8.8  i  ops=4 mem=16384 out=64", 4, 16384, 64, ALG_I),
        ("8.9  i  ops=3 mem=65536 out=65", 3, 65536, 65, ALG_I),
        // 8.10 / 8.11 / 8.12 argon2id
        ("8.10 id ops=1 mem=8192 out=16", 1, 8192, 16, ALG_ID),
        ("8.11 id ops=2 mem=8192 out=32", 2, 8192, 32, ALG_ID),
        ("8.12 id ops=3 mem=32768 out=32", 3, 32768, 32, ALG_ID),
        // 8.14 long blake2b_long path
        ("8.14 id out=128", 1, 8192, 128, ALG_ID),
        ("8.14 id out=1024", 1, 8192, 1024, ALG_ID),
    ];
    for &(label, ops, mem, outlen, alg) in cases {
        for trial in 0..3 {
            let pwlen = rng.range(0, 40);
            let passwd = rng.bytes(pwlen);
            let s = if trial == 0 { salt.clone() } else { rng.bytes(SALTBYTES) };
            let (rc, _, _) = pw(
                &c, &r, &format!("{label} trial#{trial}"), outlen, &passwd, None, &s, ops, mem, alg,
            );
            assert_eq!(rc, 0, "{label}: unexpected failure");
        }
    }

    // 8.13 ALG_DEFAULT must be byte-identical to ALG_ARGON2ID13.
    let passwd = b"test".to_vec();
    let (_, _, a) = pw(&c, &r, "8.13 default", 16, &passwd, None, &salt, 1, 8192, 2);
    let (_, _, b) = pw(&c, &r, "8.13 argon2id13", 16, &passwd, None, &salt, 1, 8192, ALG_ID);
    eqb("8.13 ALG_DEFAULT == ALG_ARGON2ID13", &a, &b);

    // argon2i and argon2id must differ for identical parameters.
    let (_, _, ai) = pw(&c, &r, "i vs id (i)", 32, &passwd, None, &salt, 3, 8192, ALG_I);
    let (_, _, aid) = pw(&c, &r, "i vs id (id)", 32, &passwd, None, &salt, 3, 8192, ALG_ID);
    assert_ne!(ai, aid, "argon2i and argon2id produced the same digest");
}

#[test]
fn r8_15_to_18_pwhash_input_shapes() {
    let (c, r) = both::<Pwhash>("crypto_pwhash");
    let mut rng = Rng::new(0x8_0015);
    let salt = rng.bytes(SALTBYTES);

    // 8.15 passwdlen = 0 with a non-NULL passwd pointer (PASSWD_MIN is 0).
    let nonempty = b"ignored".to_vec();
    let (rc, _, empty_pw) = pw(&c, &r, "8.15 passwdlen=0", 16, &nonempty, Some(0), &salt, 1, 8192, ALG_ID);
    assert_eq!(rc, 0);
    // The same digest results from a genuinely empty password buffer.
    let (rc2, _, empty_pw2) = pw(&c, &r, "8.15 empty buffer", 16, &[0u8; 1], Some(0), &salt, 1, 8192, ALG_ID);
    assert_eq!(rc2, 0);
    eqb("8.15 only passwdlen is hashed", &empty_pw, &empty_pw2);

    // 8.16 passwdlen = 1, a 256-byte password, and a >128-byte binary password
    // containing NUL and 0xFF bytes (the password is binary, not a C string).
    let mut long = rng.bytes(200);
    long[0] = 0;
    long[7] = 0xff;
    long[199] = 0;
    for (label, p) in [
        ("8.16 pwdlen=1", vec![0x41u8]),
        ("8.16 pwdlen=256", rng.bytes(256)),
        ("8.16 binary 200 bytes", long.clone()),
    ] {
        let (rc, _, _) = pw(&c, &r, label, 32, &p, None, &salt, 1, 8192, ALG_ID);
        assert_eq!(rc, 0, "{label}");
    }
    // A NUL in the middle really is hashed: truncating there changes the digest.
    let (_, _, full) = pw(&c, &r, "8.16 full", 32, &long, None, &salt, 1, 8192, ALG_ID);
    let (_, _, trunc) = pw(&c, &r, "8.16 trunc", 32, &long[..1], None, &salt, 1, 8192, ALG_ID);
    assert_ne!(full, trunc);

    // 8.17 memlimit is truncated by an integer division by 1024.
    let pwv = b"password".to_vec();
    let (_, _, base) = pw(&c, &r, "8.17 mem=8192", 16, &pwv, None, &salt, 1, 8192, ALG_ID);
    for mem in [8192usize + 512, 9215] {
        let (rc, _, out) = pw(&c, &r, &format!("8.17 mem={mem}"), 16, &pwv, None, &salt, 1, mem, ALG_ID);
        assert_eq!(rc, 0);
        eqb(&format!("8.17 mem={mem} truncates to m_cost=8"), &base, &out);
    }
    // 9216 is the first memlimit that yields m_cost = 9.  The *amount of work*
    // is unchanged (segment_length = 9 / 4 = 2, so memory_blocks is re-rounded
    // to 8), but `argon2_initial_hash` hashes the caller's raw `m_cost`, so the
    // digest still differs.  This is exactly the quirk rows 8.44/8.45 describe.
    let (_, _, nine) = pw(&c, &r, "8.17 mem=9216", 16, &pwv, None, &salt, 1, 9216, ALG_ID);
    assert_ne!(
        base, nine,
        "8.17: m_cost is hashed verbatim, so m_cost 8 and 9 must differ"
    );

    // 8.18 salt values: all-zero, all-0xFF, random.
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for s in [vec![0u8; SALTBYTES], vec![0xffu8; SALTBYTES], rng.bytes(SALTBYTES)] {
        let (rc, _, out) = pw(&c, &r, "8.18 salt", 32, &pwv, None, &s, 1, 8192, ALG_ID);
        assert_eq!(rc, 0);
        assert!(!seen.contains(&out), "8.18: two salts gave the same digest");
        seen.push(out);
    }
}

#[test]
fn r8_21_22_direct_entry_points() {
    let (c, r) = both::<Pwhash>("crypto_pwhash");
    let (ci, ri) = both::<Pwhash>("crypto_pwhash_argon2i");
    let (cd, rd) = both::<Pwhash>("crypto_pwhash_argon2id");
    let mut rng = Rng::new(0x8_0021);
    let salt = rng.bytes(SALTBYTES);

    for trial in 0..4 {
        let pwlen = rng.range(0, 32);
        let p = rng.bytes(pwlen);

        // 8.21 crypto_pwhash_argon2i == crypto_pwhash(ALG_ARGON2I13)
        let (rc, _, direct) = pw(&ci, &ri, "8.21 direct", 16, &p, None, &salt, 3, 8192, ALG_I);
        assert_eq!(rc, 0);
        let (_, _, generic) = pw(&c, &r, "8.21 generic", 16, &p, None, &salt, 3, 8192, ALG_I);
        eqb(&format!("8.21 trial#{trial}"), &generic, &direct);

        // 8.22 crypto_pwhash_argon2id == crypto_pwhash(ALG_ARGON2ID13)
        let (rc, _, direct) = pw(&cd, &rd, "8.22 direct", 16, &p, None, &salt, 1, 8192, ALG_ID);
        assert_eq!(rc, 0);
        let (_, _, generic) = pw(&c, &r, "8.22 generic", 16, &p, None, &salt, 1, 8192, ALG_ID);
        eqb(&format!("8.22 trial#{trial}"), &generic, &direct);
    }
}

#[test]
fn r8_19_20_documented_interactive_presets() {
    // (SLOW) The documented INTERACTIVE presets: 64 MiB / 32 MiB.  Run once.
    let (c, r) = both::<Pwhash>("crypto_pwhash");
    let salt = Rng::new(0x8_0019).bytes(SALTBYTES);
    let p = b"password".to_vec();
    // 8.19 argon2id OPSLIMIT_INTERACTIVE = 2, MEMLIMIT_INTERACTIVE = 64 MiB.
    let (rc, _, _) = pw(&c, &r, "8.19 interactive argon2id", 32, &p, None, &salt, 2, 67108864, ALG_ID);
    assert_eq!(rc, 0);
    // 8.20 argon2i OPSLIMIT_INTERACTIVE = 4, MEMLIMIT_INTERACTIVE = 32 MiB.
    let (rc, _, _) = pw(&c, &r, "8.20 interactive argon2i", 32, &p, None, &salt, 4, 33554432, ALG_I);
    assert_eq!(rc, 0);
}

// ==================== 8.23 – 8.36 encoded-string round trips =============

#[test]
fn r8_23_to_30_str_round_trips() {
    let _rng = rng_guard();
    let (cs, rs) = both::<PwStr>("crypto_pwhash_str");
    let (ca, ra) = both::<PwStrAlg>("crypto_pwhash_str_alg");
    let (cv, rv) = both::<StrVerify>("crypto_pwhash_str_verify");
    let (cis, ris) = both::<PwStr>("crypto_pwhash_argon2i_str");
    let (civ, riv) = both::<StrVerify>("crypto_pwhash_argon2i_str_verify");
    let (cds, rds) = both::<PwStr>("crypto_pwhash_argon2id_str");
    let (cdv, rdv) = both::<StrVerify>("crypto_pwhash_argon2id_str_verify");

    // 8.23 shape of crypto_pwhash_str at (OPSLIMIT_MIN, MEMLIMIT_MIN).
    let p = b"password".to_vec();
    let (rc, _, s) = pwstr(&cs, &rs, "8.23", &p, 1, 8192);
    assert_eq!(rc, 0);
    let prefix = "$argon2id$v=19$m=8,t=1,p=1$";
    assert!(s.starts_with(prefix), "8.23: bad prefix in {s:?}");
    let rest = &s[prefix.len()..];
    let (salt_b64, hash_b64) = rest.split_once('$').expect("8.23: missing '$'");
    assert_eq!(salt_b64.len(), 22, "8.23: 16-byte salt is 22 base64 chars");
    assert_eq!(hash_b64.len(), 43, "8.23: 32-byte hash is 43 base64 chars");
    assert!(
        !salt_b64.contains('=') && !hash_b64.contains('='),
        "8.23: ORIGINAL_NO_PADDING must not emit '=' padding"
    );
    assert_eq!(s.len(), prefix.len() + 22 + 1 + 43);
    // The tail of the 128-byte buffer is all zero (memset before the call).
    {
        let mut buf = padded(STRBYTES);
        rng_reset();
        assert_eq!(
            unsafe {
                cs(
                    buf.as_mut_ptr() as *mut c_char,
                    p.as_ptr() as *const c_char,
                    p.len() as u64,
                    1,
                    8192,
                )
            },
            0
        );
        assert!(
            buf[s.len()..STRBYTES].iter().all(|&b| b == 0),
            "8.23: bytes past the NUL terminator must be zero"
        );
    }

    // 8.24 each call uses a fresh random salt, so two calls differ yet both
    // verify.  C and Rust see the same stream, so call #n must match call #n.
    let mut a = padded(STRBYTES);
    let mut b = padded(STRBYTES);
    let mut ar = padded(STRBYTES);
    let mut br = padded(STRBYTES);
    unsafe {
        rng_reset();
        assert_eq!(
            cs(a.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, 1, 8192),
            0
        );
        assert_eq!(
            cs(b.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, 1, 8192),
            0
        );
        rng_reset();
        assert_eq!(
            rs(ar.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, 1, 8192),
            0
        );
        assert_eq!(
            rs(br.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, 1, 8192),
            0
        );
    }
    eqb("8.24 call#1", &a[..STRBYTES], &ar[..STRBYTES]);
    eqb("8.24 call#2", &b[..STRBYTES], &br[..STRBYTES]);
    assert_ne!(&a[..STRBYTES], &b[..STRBYTES], "8.24: two calls produced the same string");
    for buf in [&a, &b, &ar, &br] {
        let z = cstr(&as_str(&buf[..STRBYTES]));
        unsafe {
            assert_eq!(
                cv(z.as_ptr() as *const c_char, p.as_ptr() as *const c_char, p.len() as u64),
                0
            );
            assert_eq!(
                rv(z.as_ptr() as *const c_char, p.as_ptr() as *const c_char, p.len() as u64),
                0
            );
        }
    }

    // 8.25 empty password round trip.
    let (rc, _, s0) = pwstr(&cs, &rs, "8.25 empty password", &[], 1, 8192);
    assert_eq!(rc, 0);
    let z = cstr(&s0);
    unsafe {
        assert_eq!(cv(z.as_ptr() as *const c_char, [0u8; 1].as_ptr() as *const c_char, 0), 0);
        assert_eq!(rv(z.as_ptr() as *const c_char, [0u8; 1].as_ptr() as *const c_char, 0), 0);
    }

    // 8.26 / 8.27 crypto_pwhash_str_alg.
    for (label, alg, want_prefix, ops) in [
        ("8.26 argon2i", ALG_I, "$argon2i$v=19$m=8,t=3,p=1$", 3u64),
        ("8.27 argon2id", ALG_ID, "$argon2id$v=19$m=8,t=1,p=1$", 1),
    ] {
        let mut sc = padded(STRBYTES);
        let mut sr = padded(STRBYTES);
        rng_reset();
        let rc = unsafe {
            ca(sc.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, ops, 8192, alg)
        };
        let rr = unsafe {
            ra(sr.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, ops, 8192, alg)
        };
        eqi(&format!("{label} ret"), rc, rr);
        assert_eq!(rc, 0);
        eqb(&format!("{label} str"), &sc[..STRBYTES], &sr[..STRBYTES]);
        let got = as_str(&sc[..STRBYTES]);
        assert!(got.starts_with(want_prefix), "{label}: {got:?}");
        // Prefix dispatch in the generic verifier.
        let z = cstr(&got);
        unsafe {
            assert_eq!(
                cv(z.as_ptr() as *const c_char, p.as_ptr() as *const c_char, p.len() as u64),
                0,
                "{label}: crypto_pwhash_str_verify rejected the string"
            );
            assert_eq!(
                rv(z.as_ptr() as *const c_char, p.as_ptr() as *const c_char, p.len() as u64),
                0
            );
        }
    }
    // 8.27 crypto_pwhash_str_alg(ALG_ARGON2ID13) == crypto_pwhash_str.
    {
        let mut x = padded(STRBYTES);
        rng_reset();
        assert_eq!(
            unsafe {
                ca(x.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, 1, 8192, ALG_ID)
            },
            0
        );
        let mut y = padded(STRBYTES);
        rng_reset();
        assert_eq!(
            unsafe {
                cs(y.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, 1, 8192)
            },
            0
        );
        eqb("8.27 str_alg(ARGON2ID13) == str", &y[..STRBYTES], &x[..STRBYTES]);
    }

    // 8.29 argon2i direct round trip, also accepted by the generic verifier.
    let (rc, _, si) = pwstr(&cis, &ris, "8.29 argon2i_str", &p, 3, 8192);
    assert_eq!(rc, 0);
    assert!(si.starts_with("$argon2i$"));
    let z = cstr(&si);
    unsafe {
        assert_eq!(
            civ(z.as_ptr() as *const c_char, p.as_ptr() as *const c_char, p.len() as u64),
            0,
            "8.29: C argon2i_str_verify rejected its own string"
        );
        assert_eq!(
            riv(z.as_ptr() as *const c_char, p.as_ptr() as *const c_char, p.len() as u64),
            0,
            "8.29: Rust argon2i_str_verify rejected its own string"
        );
        assert_eq!(cv(z.as_ptr() as *const c_char, p.as_ptr() as *const c_char, p.len() as u64), 0);
        assert_eq!(rv(z.as_ptr() as *const c_char, p.as_ptr() as *const c_char, p.len() as u64), 0);
    }

    // 8.30 argon2id direct round trip.
    let (rc, _, sd) = pwstr(&cds, &rds, "8.30 argon2id_str", &p, 1, 8192);
    assert_eq!(rc, 0);
    assert!(sd.starts_with("$argon2id$"));
    let z = cstr(&sd);
    unsafe {
        assert_eq!(
            cdv(z.as_ptr() as *const c_char, p.as_ptr() as *const c_char, p.len() as u64),
            0,
            "8.30: C argon2id_str_verify rejected its own string"
        );
        assert_eq!(
            rdv(z.as_ptr() as *const c_char, p.as_ptr() as *const c_char, p.len() as u64),
            0,
            "8.30: Rust argon2id_str_verify rejected its own string"
        );
    }
    // crypto_pwhash_str == crypto_pwhash_argon2id_str.
    {
        let mut x = padded(STRBYTES);
        rng_reset();
        assert_eq!(
            unsafe {
                cds(x.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, 1, 8192)
            },
            0
        );
        let mut y = padded(STRBYTES);
        rng_reset();
        assert_eq!(
            unsafe {
                cs(y.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, 1, 8192)
            },
            0
        );
        eqb("crypto_pwhash_str == crypto_pwhash_argon2id_str", &y[..STRBYTES], &x[..STRBYTES]);
    }
}

#[test]
fn r8_28_interactive_str_alg() {
    // (SLOW) 8.28 crypto_pwhash_str_alg at the documented INTERACTIVE preset.
    let _rng = rng_guard();
    let (c, r) = both::<PwStrAlg>("crypto_pwhash_str_alg");
    let p = b"password".to_vec();
    let mut sc = padded(STRBYTES);
    let mut sr = padded(STRBYTES);
    rng_reset();
    let rc = unsafe {
        c(sc.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, 2, 67108864, ALG_ID)
    };
    let rr = unsafe {
        r(sr.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, 2, 67108864, ALG_ID)
    };
    eqi("8.28 ret", rc, rr);
    assert_eq!(rc, 0);
    eqb("8.28 str", &sc[..STRBYTES], &sr[..STRBYTES]);
    let s = as_str(&sc[..STRBYTES]);
    assert!(
        s.starts_with("$argon2id$v=19$m=65536,t=2,p=1$"),
        "8.28: {s:?}"
    );
}

#[test]
fn r8_31_to_36_needs_rehash() {
    let _rng = rng_guard();
    let (cs, rs) = both::<PwStr>("crypto_pwhash_str");
    let (cn, rn) = both::<NeedsRehash>("crypto_pwhash_str_needs_rehash");
    let (cin, rin) = both::<NeedsRehash>("crypto_pwhash_argon2i_str_needs_rehash");
    let (cdn, rdn) = both::<NeedsRehash>("crypto_pwhash_argon2id_str_needs_rehash");
    let (ca, ra) = both::<PwStrAlg>("crypto_pwhash_str_alg");
    let p = b"password".to_vec();

    #[track_caller]
    fn nr(c: &NeedsRehash, r: &NeedsRehash, label: &str, s: &str, ops: u64, mem: usize) -> c_int {
        let z = cstr(s);
        set_errno(SENTINEL);
        let a = unsafe { c(z.as_ptr() as *const c_char, ops, mem) };
        let ec = errno();
        set_errno(SENTINEL);
        let b = unsafe { r(z.as_ptr() as *const c_char, ops, mem) };
        let er = errno();
        eqi(&format!("{label} ret"), a, b);
        assert_eq!(ec, er, "{label}: errno mismatch (C {ec}, Rust {er})");
        a
    }

    // 8.31 same parameters -> 0.
    let (_, _, sid) = pwstr(&cs, &rs, "8.31 base string", &p, 1, 8192);
    assert_eq!(nr(&cn, &rn, "8.31", &sid, 1, 8192), 0);
    assert_eq!(nr(&cdn, &rdn, "8.31 argon2id direct", &sid, 1, 8192), 0);

    // 8.32 a different opslimit or memlimit -> 1.
    assert_eq!(nr(&cn, &rn, "8.32 ops=2", &sid, 2, 8192), 1);
    assert_eq!(nr(&cn, &rn, "8.32 mem=16384", &sid, 1, 16384), 1);
    assert_eq!(nr(&cn, &rn, "8.32 both", &sid, 5, 65536), 1);

    // 8.33 memlimit is truncated by the division by 1024.
    assert_eq!(nr(&cn, &rn, "8.33 mem=8192+1023", &sid, 1, 8192 + 1023), 0);
    assert_eq!(nr(&cn, &rn, "8.33 mem=9216", &sid, 1, 9216), 1);

    // 8.34 argon2i prefix dispatch.
    let mut sc = padded(STRBYTES);
    let mut sr = padded(STRBYTES);
    rng_reset();
    assert_eq!(
        unsafe {
            ca(sc.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, 3, 8192, ALG_I)
        },
        0
    );
    assert_eq!(
        unsafe {
            ra(sr.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, p.len() as u64, 3, 8192, ALG_I)
        },
        0
    );
    eqb("8.34 argon2i string", &sc[..STRBYTES], &sr[..STRBYTES]);
    let si = as_str(&sc[..STRBYTES]);
    assert_eq!(nr(&cn, &rn, "8.34 (3, 8192)", &si, 3, 8192), 0);
    assert_eq!(nr(&cn, &rn, "8.34 (4, 8192)", &si, 4, 8192), 1);
    assert_eq!(nr(&cin, &rin, "8.34 argon2i direct", &si, 3, 8192), 0);

    // 8.35 `p`/lanes and the argon2 type are *not* compared.  Note that the
    // row's literal example ("m=8,t=1,p=2") is rejected earlier by
    // argon2_decode_string's final validation (m_cost < 8 * lanes -> -14), so
    // both the rejected and the accepted shape are pinned here.
    let salt16 = Rng::new(0x8_0035).bytes(16);
    let hash32 = Rng::new(0x8_0036).bytes(32);
    let p2_bad = make_str("argon2id", 8, 1, 2, &salt16, &hash32);
    assert_eq!(
        nr(&cn, &rn, "8.35 m=8,t=1,p=2 (m < 8*p)", &p2_bad, 1, 8192),
        -1,
        "8.35: m_cost < 8*lanes must be rejected by the decoder"
    );
    let p2_ok = make_str("argon2id", 16, 1, 2, &salt16, &hash32);
    assert_eq!(
        nr(&cn, &rn, "8.35 m=16,t=1,p=2", &p2_ok, 1, 16384),
        0,
        "8.35: lanes/p must not take part in the comparison"
    );

    // 8.36 strlen(str) == 127 is the largest accepted length.  A 42-byte salt
    // encodes to 56 base64 chars, giving 27 + 56 + 1 + 43 = 127.
    let salt42 = Rng::new(0x8_0037).bytes(42);
    let long = make_str("argon2id", 8, 1, 1, &salt42, &hash32);
    assert_eq!(long.len(), 127, "8.36: expected a 127-char string, got {}", long.len());
    assert_eq!(nr(&cdn, &rdn, "8.36 argon2id 127 chars", &long, 1, 8192), 0);
    let long_i = make_str("argon2i", 8, 1, 1, &salt42, &hash32);
    assert_eq!(long_i.len(), 126);
    assert_eq!(nr(&cin, &rin, "8.36 argon2i 126 chars", &long_i, 1, 8192), 0);
    // One byte more and _needs_rehash rejects it on the length check alone,
    // before any decoding (errors row 8.49).
    let too_long = format!("{long}x");
    assert_eq!(too_long.len(), 128);
    assert_eq!(nr(&cdn, &rdn, "8.49 128 chars", &too_long, 1, 8192), -1);
}

// ============================== error surface ============================
//
// errors_8.md rows 8.1 – 8.52.  Two rows of the table cannot be exercised
// safely and are therefore only documented here:
//
//   * 8.7 / 8.18 (`outlen > BYTES_MAX`, i.e. `outlen >= 2^32`): the C code runs
//     `memset(out, 0, outlen)` *before* the check, so reaching the `EFBIG`
//     return requires a caller-supplied buffer larger than 4 GiB.
//   * 8.17 / 8.27 / 8.33 / 8.37 / 8.50 (inner allocation failures) and 8.14 /
//     8.39 (`passwdlen < PASSWD_MIN` with `PASSWD_MIN == 0`) are unreachable.

/// A `crypto_pwhash*` call whose output buffer is pre-filled with a marker, so
/// "out untouched" and "out zeroed" can be told apart.
#[track_caller]
fn pw_marked(
    c: &Pwhash,
    r: &Pwhash,
    label: &str,
    outlen: usize,
    passwd: &[u8],
    passwdlen: Option<u64>,
    salt: &[u8],
    ops: u64,
    mem: usize,
    alg: c_int,
) -> (c_int, c_int, Vec<u8>) {
    const MARK: u8 = 0xC3;
    let pwlen = passwdlen.unwrap_or(passwd.len() as u64);
    let mut oc = padded(outlen);
    let mut or = padded(outlen);
    for i in 0..outlen {
        oc[i] = MARK;
        or[i] = MARK;
    }
    set_errno(SENTINEL);
    let rc = unsafe {
        c(
            oc.as_mut_ptr(),
            outlen as u64,
            passwd.as_ptr() as *const c_char,
            pwlen,
            salt.as_ptr(),
            ops,
            mem,
            alg,
        )
    };
    let ec = errno();
    set_errno(SENTINEL);
    let rr = unsafe {
        r(
            or.as_mut_ptr(),
            outlen as u64,
            passwd.as_ptr() as *const c_char,
            pwlen,
            salt.as_ptr(),
            ops,
            mem,
            alg,
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

// ---------------------------------------- 8.1 – 8.3 crypto_pwhash bad alg

#[test]
fn e8_1_to_3_pwhash_unknown_alg() {
    let (c, r) = both::<Pwhash>("crypto_pwhash");
    let salt = vec![0x11u8; SALTBYTES];
    // Every `alg` with no valid variant, including the FFI extremes.
    for alg in [0, 3, 4, -1, 5, 100, c_int::MAX, c_int::MIN] {
        let (rc, ec, out) = pw_marked(
            &c, &r, &format!("8.1-8.3 alg={alg}"), 32, b"password", None, &salt, 3, 8192, alg,
        );
        assert_eq!(rc, -1, "8.1-8.3 alg={alg}: expected -1");
        assert_eq!(ec, EINVAL, "8.1-8.3 alg={alg}: expected EINVAL");
        // The switch in crypto_pwhash() runs before any memset, so `out` is
        // left exactly as the caller had it.
        assert!(
            out.iter().all(|&b| b == 0xC3),
            "8.1: crypto_pwhash must not touch `out` for an unknown alg"
        );
    }
}

// ------------------------------------- 8.4 crypto_pwhash_str_alg aborts

#[test]
fn e8_4_str_alg_unknown_alg_aborts() {
    let (c, r) = both::<PwStrAlg>("crypto_pwhash_str_alg");
    for alg in [0, 3, 4, -1, c_int::MAX, c_int::MIN] {
        let cc = c.clone();
        let rr = r.clone();
        eq_abort(
            &format!("8.4 crypto_pwhash_str_alg(alg={alg})"),
            move || unsafe {
                let mut o = [0u8; STRBYTES];
                cc(
                    o.as_mut_ptr() as *mut c_char,
                    b"password".as_ptr() as *const c_char,
                    8,
                    1,
                    8192,
                    alg,
                );
            },
            move || unsafe {
                let mut o = [0u8; STRBYTES];
                rr(
                    o.as_mut_ptr() as *mut c_char,
                    b"password".as_ptr() as *const c_char,
                    8,
                    1,
                    8192,
                    alg,
                );
            },
        );
    }
}

// ----------------------------- 8.5 / 8.6 prefix dispatch rejections

#[test]
fn e8_5_6_prefix_dispatch() {
    let (cv, rv) = both::<StrVerify>("crypto_pwhash_str_verify");
    let (cn, rn) = both::<NeedsRehash>("crypto_pwhash_str_needs_rehash");
    let salt16 = Rng::new(0x8_e005).bytes(16);
    let hash32 = Rng::new(0x8_e006).bytes(32);
    let bad = [
        String::new(),
        "$".to_string(),
        "$7$C6..../....DtQGz9OuAiRZTuVjTBFcNw/tvi1MDBFEhmnZFmHTLIA".to_string(),
        make_str("argon2d", 8, 1, 1, &salt16, &hash32),
        make_str("ARGON2ID", 8, 1, 1, &salt16, &hash32),
        make_str("argon2", 8, 1, 1, &salt16, &hash32),
        "argon2id$v=19$m=8,t=1,p=1$aaaa$bbbb".to_string(),
        "$argon2".to_string(),
        "$argon".to_string(),
    ];
    for s in &bad {
        let z = cstr(s);
        set_errno(SENTINEL);
        let a = unsafe { cv(z.as_ptr() as *const c_char, b"pw".as_ptr() as *const c_char, 2) };
        let ec = errno();
        set_errno(SENTINEL);
        let b = unsafe { rv(z.as_ptr() as *const c_char, b"pw".as_ptr() as *const c_char, 2) };
        let er = errno();
        eqi(&format!("8.5 str_verify({s:?})"), a, b);
        assert_eq!(a, -1, "8.5: {s:?} must be rejected");
        assert_eq!(ec, EINVAL, "8.5: {s:?} must set EINVAL");
        assert_eq!(er, ec);

        set_errno(SENTINEL);
        let a = unsafe { cn(z.as_ptr() as *const c_char, 1, 8192) };
        let ec = errno();
        set_errno(SENTINEL);
        let b = unsafe { rn(z.as_ptr() as *const c_char, 1, 8192) };
        let er = errno();
        eqi(&format!("8.6 needs_rehash({s:?})"), a, b);
        assert_eq!(a, -1, "8.6: {s:?} must be rejected");
        assert_eq!(ec, EINVAL, "8.6: {s:?} must set EINVAL");
        assert_eq!(er, ec);
    }
}

// ------------- 8.8 – 8.16 / 8.19 – 8.26 raw-KDF entry-point rejections

#[test]
fn e8_8_to_26_raw_kdf_rejections() {
    let (cg, rg) = both::<Pwhash>("crypto_pwhash");
    let (ci, ri) = both::<Pwhash>("crypto_pwhash_argon2i");
    let (cd, rd) = both::<Pwhash>("crypto_pwhash_argon2id");
    let salt = vec![0x22u8; SALTBYTES];
    let p = b"password".to_vec();

    // (label, entry point index, outlen, passwdlen override, ops, mem, alg,
    //  expected errno)
    struct Case<'a> {
        label: &'a str,
        which: u8, // 0 = argon2i, 1 = argon2id
        outlen: usize,
        pwlen: Option<u64>,
        ops: u64,
        mem: usize,
        alg: c_int,
        eno: c_int,
    }
    let ok_i = (3u64, 8192usize, ALG_I);
    let ok_d = (1u64, 8192usize, ALG_ID);
    let cases: Vec<Case> = vec![
        // 8.8 outlen < BYTES_MIN (argon2i)
        Case { label: "8.8  i  outlen=0",  which: 0, outlen: 0,  pwlen: None, ops: ok_i.0, mem: ok_i.1, alg: ok_i.2, eno: EINVAL },
        Case { label: "8.8  i  outlen=1",  which: 0, outlen: 1,  pwlen: None, ops: ok_i.0, mem: ok_i.1, alg: ok_i.2, eno: EINVAL },
        Case { label: "8.8  i  outlen=15", which: 0, outlen: 15, pwlen: None, ops: ok_i.0, mem: ok_i.1, alg: ok_i.2, eno: EINVAL },
        // 8.9 passwdlen > PASSWD_MAX
        Case { label: "8.9  i  passwdlen=2^32",     which: 0, outlen: 32, pwlen: Some(4294967296), ops: ok_i.0, mem: ok_i.1, alg: ok_i.2, eno: EFBIG },
        Case { label: "8.9  i  passwdlen=u64::MAX", which: 0, outlen: 32, pwlen: Some(u64::MAX),   ops: ok_i.0, mem: ok_i.1, alg: ok_i.2, eno: EFBIG },
        // 8.10 opslimit > OPSLIMIT_MAX
        Case { label: "8.10 i  opslimit=2^32",     which: 0, outlen: 32, pwlen: None, ops: 4294967296, mem: ok_i.1, alg: ok_i.2, eno: EFBIG },
        Case { label: "8.10 i  opslimit=u64::MAX", which: 0, outlen: 32, pwlen: None, ops: u64::MAX,   mem: ok_i.1, alg: ok_i.2, eno: EFBIG },
        // 8.11 memlimit > MEMLIMIT_MAX
        Case { label: "8.11 i  memlimit=MAX+1",  which: 0, outlen: 32, pwlen: None, ops: ok_i.0, mem: 4398046510081, alg: ok_i.2, eno: EFBIG },
        Case { label: "8.11 i  memlimit=MAX",    which: 0, outlen: 32, pwlen: None, ops: ok_i.0, mem: usize::MAX,    alg: ok_i.2, eno: EFBIG },
        // 8.12 opslimit < argon2i OPSLIMIT_MIN (3)
        Case { label: "8.12 i  opslimit=0", which: 0, outlen: 32, pwlen: None, ops: 0, mem: ok_i.1, alg: ok_i.2, eno: EINVAL },
        Case { label: "8.12 i  opslimit=1", which: 0, outlen: 32, pwlen: None, ops: 1, mem: ok_i.1, alg: ok_i.2, eno: EINVAL },
        Case { label: "8.12 i  opslimit=2", which: 0, outlen: 32, pwlen: None, ops: 2, mem: ok_i.1, alg: ok_i.2, eno: EINVAL },
        // 8.13 memlimit < MEMLIMIT_MIN (8192)
        Case { label: "8.13 i  memlimit=0",    which: 0, outlen: 32, pwlen: None, ops: ok_i.0, mem: 0,    alg: ok_i.2, eno: EINVAL },
        Case { label: "8.13 i  memlimit=1024", which: 0, outlen: 32, pwlen: None, ops: ok_i.0, mem: 1024, alg: ok_i.2, eno: EINVAL },
        Case { label: "8.13 i  memlimit=8191", which: 0, outlen: 32, pwlen: None, ops: ok_i.0, mem: 8191, alg: ok_i.2, eno: EINVAL },
        // 8.16 wrong alg for the argon2i entry point
        Case { label: "8.16 i  alg=2",       which: 0, outlen: 32, pwlen: None, ops: ok_i.0, mem: ok_i.1, alg: ALG_ID,     eno: EINVAL },
        Case { label: "8.16 i  alg=0",       which: 0, outlen: 32, pwlen: None, ops: ok_i.0, mem: ok_i.1, alg: 0,          eno: EINVAL },
        Case { label: "8.16 i  alg=-1",      which: 0, outlen: 32, pwlen: None, ops: ok_i.0, mem: ok_i.1, alg: -1,         eno: EINVAL },
        Case { label: "8.16 i  alg=INT_MAX", which: 0, outlen: 32, pwlen: None, ops: ok_i.0, mem: ok_i.1, alg: c_int::MAX, eno: EINVAL },
        Case { label: "8.16 i  alg=INT_MIN", which: 0, outlen: 32, pwlen: None, ops: ok_i.0, mem: ok_i.1, alg: c_int::MIN, eno: EINVAL },
        // 8.19 outlen < BYTES_MIN (argon2id)
        Case { label: "8.19 id outlen=0",  which: 1, outlen: 0,  pwlen: None, ops: ok_d.0, mem: ok_d.1, alg: ok_d.2, eno: EINVAL },
        Case { label: "8.19 id outlen=1",  which: 1, outlen: 1,  pwlen: None, ops: ok_d.0, mem: ok_d.1, alg: ok_d.2, eno: EINVAL },
        Case { label: "8.19 id outlen=15", which: 1, outlen: 15, pwlen: None, ops: ok_d.0, mem: ok_d.1, alg: ok_d.2, eno: EINVAL },
        // 8.20 / 8.21 / 8.22
        Case { label: "8.20 id passwdlen=2^32", which: 1, outlen: 32, pwlen: Some(4294967296), ops: ok_d.0, mem: ok_d.1, alg: ok_d.2, eno: EFBIG },
        Case { label: "8.21 id opslimit=2^32",  which: 1, outlen: 32, pwlen: None, ops: 4294967296, mem: ok_d.1, alg: ok_d.2, eno: EFBIG },
        Case { label: "8.22 id memlimit=MAX+1", which: 1, outlen: 32, pwlen: None, ops: ok_d.0, mem: 4398046510081, alg: ok_d.2, eno: EFBIG },
        // 8.23 opslimit < argon2id OPSLIMIT_MIN (1)
        Case { label: "8.23 id opslimit=0", which: 1, outlen: 32, pwlen: None, ops: 0, mem: ok_d.1, alg: ok_d.2, eno: EINVAL },
        // 8.24 memlimit < 8192
        Case { label: "8.24 id memlimit=0",    which: 1, outlen: 32, pwlen: None, ops: ok_d.0, mem: 0,    alg: ok_d.2, eno: EINVAL },
        Case { label: "8.24 id memlimit=8191", which: 1, outlen: 32, pwlen: None, ops: ok_d.0, mem: 8191, alg: ok_d.2, eno: EINVAL },
        // 8.26 wrong alg for the argon2id entry point
        Case { label: "8.26 id alg=1",       which: 1, outlen: 32, pwlen: None, ops: ok_d.0, mem: ok_d.1, alg: ALG_I,      eno: EINVAL },
        Case { label: "8.26 id alg=0",       which: 1, outlen: 32, pwlen: None, ops: ok_d.0, mem: ok_d.1, alg: 0,          eno: EINVAL },
        Case { label: "8.26 id alg=INT_MIN", which: 1, outlen: 32, pwlen: None, ops: ok_d.0, mem: ok_d.1, alg: c_int::MIN, eno: EINVAL },
    ];

    for k in &cases {
        let (c, r) = if k.which == 0 { (&ci, &ri) } else { (&cd, &rd) };
        let (rc, ec, out) = pw_marked(
            c, r, k.label, k.outlen, &p, k.pwlen, &salt, k.ops, k.mem, k.alg,
        );
        assert_eq!(rc, -1, "{}: expected -1", k.label);
        assert_eq!(ec, k.eno, "{}: errno {} expected {}", k.label, ec, k.eno);
        // Every argon2 entry point memsets `out` before validating.
        assert!(
            out.iter().all(|&b| b == 0),
            "{}: `out` must be zeroed before the check",
            k.label
        );

        // The generic dispatcher forwards to the same code, so it must agree
        // whenever `alg` selects the entry point actually under test.
        if (k.which == 0 && k.alg == ALG_I) || (k.which == 1 && k.alg == ALG_ID) {
            let (grc, gec, gout) = pw_marked(
                &cg, &rg, &format!("{} (generic)", k.label), k.outlen, &p, k.pwlen, &salt,
                k.ops, k.mem, k.alg,
            );
            assert_eq!(grc, rc, "{}: generic dispatcher disagreed", k.label);
            assert_eq!(gec, ec, "{}: generic dispatcher errno disagreed", k.label);
            eqb(&format!("{} generic out", k.label), &out, &gout);
        }
    }
}

#[test]
fn e8_15_25_out_aliases_passwd() {
    // 8.15 / 8.25 `(const void *) out == (const void *) passwd`.
    for (name, ops, alg) in [
        ("crypto_pwhash_argon2i", 3u64, ALG_I),
        ("crypto_pwhash_argon2id", 1, ALG_ID),
        ("crypto_pwhash", 1, ALG_ID),
    ] {
        let (c, r) = both::<Pwhash>(name);
        let salt = vec![0x33u8; SALTBYTES];
        let mut bc = vec![0x44u8; 32];
        let mut br = vec![0x44u8; 32];
        set_errno(SENTINEL);
        let rc = unsafe {
            c(
                bc.as_mut_ptr(),
                32,
                bc.as_ptr() as *const c_char,
                32,
                salt.as_ptr(),
                ops,
                8192,
                alg,
            )
        };
        let ec = errno();
        set_errno(SENTINEL);
        let rr = unsafe {
            r(
                br.as_mut_ptr(),
                32,
                br.as_ptr() as *const c_char,
                32,
                salt.as_ptr(),
                ops,
                8192,
                alg,
            )
        };
        let er = errno();
        eqi(&format!("8.15/8.25 {name} out == passwd"), rc, rr);
        assert_eq!(rc, -1, "8.15/8.25 {name}: expected -1");
        assert_eq!(ec, EINVAL, "8.15/8.25 {name}: expected EINVAL");
        assert_eq!(er, ec);
        eqb(&format!("8.15/8.25 {name} buffer"), &bc, &br);
        assert!(bc.iter().all(|&b| b == 0), "8.15/8.25 {name}: out is memset first");
    }
}

// ------------------------------ 8.28 – 8.37 `*_str` rejections

#[test]
fn e8_28_to_37_str_rejections() {
    let _rng = rng_guard();
    let (cs, rs) = both::<PwStr>("crypto_pwhash_str");
    let (cis, ris) = both::<PwStr>("crypto_pwhash_argon2i_str");
    let (cds, rds) = both::<PwStr>("crypto_pwhash_argon2id_str");
    let (ca, ra) = both::<PwStrAlg>("crypto_pwhash_str_alg");
    let p = b"password".to_vec();

    #[track_caller]
    fn one(
        c: &PwStr,
        r: &PwStr,
        label: &str,
        passwd: &[u8],
        pwlen: u64,
        ops: u64,
        mem: usize,
    ) -> (c_int, c_int, Vec<u8>) {
        let mut sc = padded(STRBYTES);
        let mut sr = padded(STRBYTES);
        for i in 0..STRBYTES {
            sc[i] = 0xC3;
            sr[i] = 0xC3;
        }
        rng_reset();
        set_errno(SENTINEL);
        let rc = unsafe { c(sc.as_mut_ptr() as *mut c_char, passwd.as_ptr() as *const c_char, pwlen, ops, mem) };
        let ec = errno();
        set_errno(SENTINEL);
        let rr = unsafe { r(sr.as_mut_ptr() as *mut c_char, passwd.as_ptr() as *const c_char, pwlen, ops, mem) };
        let er = errno();
        eqi(&format!("{label} ret"), rc, rr);
        assert_eq!(ec, er, "{label}: errno mismatch (C {ec}, Rust {er})");
        eqb(&format!("{label} out"), &sc[..STRBYTES], &sr[..STRBYTES]);
        check_pad(&format!("{label}(C)"), &sc, STRBYTES);
        check_pad(&format!("{label}(Rust)"), &sr, STRBYTES);
        (rc, ec, sc[..STRBYTES].to_vec())
    }

    // (label, argon2i?, pwlen, ops, mem, expected errno)
    let cases: &[(&str, bool, u64, u64, usize, c_int)] = &[
        // 8.28 / 8.34 passwdlen > PASSWD_MAX
        ("8.28 i  passwdlen=2^32", true, 4294967296, 3, 8192, EFBIG),
        ("8.34 id passwdlen=2^32", false, 4294967296, 1, 8192, EFBIG),
        ("8.34 id passwdlen=u64::MAX", false, u64::MAX, 1, 8192, EFBIG),
        // 8.29 / 8.34 opslimit > OPSLIMIT_MAX
        ("8.29 i  opslimit=2^32", true, 8, 4294967296, 8192, EFBIG),
        ("8.34 id opslimit=2^32", false, 8, 4294967296, 8192, EFBIG),
        // 8.30 / 8.34 memlimit > MEMLIMIT_MAX
        ("8.30 i  memlimit=MAX+1", true, 8, 3, 4398046510081, EFBIG),
        ("8.34 id memlimit=MAX+1", false, 8, 1, 4398046510081, EFBIG),
        // 8.31 opslimit < 3 (argon2i)
        ("8.31 i  opslimit=0", true, 8, 0, 8192, EINVAL),
        ("8.31 i  opslimit=1", true, 8, 1, 8192, EINVAL),
        ("8.31 i  opslimit=2", true, 8, 2, 8192, EINVAL),
        // 8.32 / 8.36 memlimit < 8192
        ("8.32 i  memlimit=0", true, 8, 3, 0, EINVAL),
        ("8.32 i  memlimit=8191", true, 8, 3, 8191, EINVAL),
        ("8.36 id memlimit=0", false, 8, 1, 0, EINVAL),
        ("8.36 id memlimit=8191", false, 8, 1, 8191, EINVAL),
        // 8.35 opslimit < 1 (argon2id)
        ("8.35 id opslimit=0", false, 8, 0, 8192, EINVAL),
    ];
    for &(label, is_i, pwlen, ops, mem, eno) in cases {
        let (c, r) = if is_i { (&cis, &ris) } else { (&cds, &rds) };
        let (rc, ec, out) = one(c, r, label, &p, pwlen, ops, mem);
        assert_eq!(rc, -1, "{label}: expected -1");
        assert_eq!(ec, eno, "{label}: errno {ec} expected {eno}");
        assert!(
            out.iter().all(|&b| b == 0),
            "{label}: the whole STRBYTES buffer is zeroed before the check"
        );

        // crypto_pwhash_str forwards verbatim to crypto_pwhash_argon2id_str.
        if !is_i {
            let (grc, gec, gout) = one(&cs, &rs, &format!("{label} (crypto_pwhash_str)"), &p, pwlen, ops, mem);
            assert_eq!(grc, rc, "{label}: crypto_pwhash_str disagreed");
            assert_eq!(gec, ec, "{label}: crypto_pwhash_str errno disagreed");
            eqb(&format!("{label} via crypto_pwhash_str"), &out, &gout);
        }

        // ... and so does crypto_pwhash_str_alg for a known alg.
        let alg = if is_i { ALG_I } else { ALG_ID };
        let mut sc = padded(STRBYTES);
        let mut sr = padded(STRBYTES);
        rng_reset();
        set_errno(SENTINEL);
        let arc = unsafe {
            ca(sc.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, pwlen, ops, mem, alg)
        };
        let aec = errno();
        set_errno(SENTINEL);
        let arr = unsafe {
            ra(sr.as_mut_ptr() as *mut c_char, p.as_ptr() as *const c_char, pwlen, ops, mem, alg)
        };
        let aer = errno();
        eqi(&format!("{label} (str_alg)"), arc, arr);
        assert_eq!(aec, aer);
        assert_eq!(arc, rc, "{label}: crypto_pwhash_str_alg disagreed");
        assert_eq!(aec, ec, "{label}: crypto_pwhash_str_alg errno disagreed");
    }
}

// -------------------- 8.38 / 8.40 – 8.46 `*_str_verify` rejections

#[test]
fn e8_38_to_46_str_verify_rejections() {
    let _rng = rng_guard();
    let (cv, rv) = both::<StrVerify>("crypto_pwhash_str_verify");
    let (civ, riv) = both::<StrVerify>("crypto_pwhash_argon2i_str_verify");
    let (cdv, rdv) = both::<StrVerify>("crypto_pwhash_argon2id_str_verify");
    let (cs, rs) = both::<PwStr>("crypto_pwhash_str");
    let (cis, ris) = both::<PwStr>("crypto_pwhash_argon2i_str");
    let p = b"password".to_vec();

    #[track_caller]
    fn ver(
        c: &StrVerify,
        r: &StrVerify,
        label: &str,
        s: &str,
        passwd: &[u8],
        pwlen: Option<u64>,
    ) -> (c_int, c_int) {
        let z = cstr(s);
        let n = pwlen.unwrap_or(passwd.len() as u64);
        set_errno(SENTINEL);
        let a = unsafe { c(z.as_ptr() as *const c_char, passwd.as_ptr() as *const c_char, n) };
        let ec = errno();
        set_errno(SENTINEL);
        let b = unsafe { r(z.as_ptr() as *const c_char, passwd.as_ptr() as *const c_char, n) };
        let er = errno();
        eqi(&format!("{label} ret"), a, b);
        assert_eq!(ec, er, "{label}: errno mismatch (C {ec}, Rust {er})");
        (a, ec)
    }

    let (_, _, sid) = pwstr(&cs, &rs, "8.38 base argon2id string", &p, 1, 8192);
    let (_, _, si) = pwstr(&cis, &ris, "8.38 base argon2i string", &p, 3, 8192);

    // 8.38 / 8.44 passwdlen > PASSWD_MAX -> EFBIG (checked before any parsing).
    for (label, c, r, s) in [
        ("8.38 argon2i", &civ, &riv, &si),
        ("8.44 argon2id", &cdv, &rdv, &sid),
    ] {
        for n in [4294967296u64, u64::MAX] {
            let (a, ec) = ver(c, r, &format!("{label} passwdlen={n}"), s, &p, Some(n));
            assert_eq!(a, -1);
            assert_eq!(ec, EFBIG, "{label}: expected EFBIG");
        }
    }

    // 8.40 / 8.45 wrong password -> ARGON2_VERIFY_MISMATCH mapped to -1/EINVAL.
    for (label, c, r, s) in [
        ("8.40 argon2i", &civ, &riv, &si),
        ("8.45 argon2id", &cdv, &rdv, &sid),
    ] {
        for wrong in [b"".to_vec(), b"Password".to_vec(), b"password1".to_vec()] {
            let (a, ec) = ver(c, r, &format!("{label} wrong pw {wrong:?}"), s, &wrong, None);
            assert_eq!(a, -1, "{label}: wrong password must be rejected");
            assert_eq!(ec, EINVAL, "{label}: VERIFY_MISMATCH must set EINVAL");
        }
        // ... and the generic verifier agrees.
        let (a, ec) = ver(&cv, &rv, &format!("{label} generic wrong pw"), s, b"nope", None);
        assert_eq!(a, -1);
        assert_eq!(ec, EINVAL);
    }

    // 8.41 / 8.46 a malformed string: `errno` is *not* touched (only
    // VERIFY_MISMATCH sets EINVAL).
    let salt16 = Rng::new(0x8_e041).bytes(16);
    let hash32 = Rng::new(0x8_e042).bytes(32);
    let malformed_id = [
        // 8.42 an argon2id string fed to the argon2i verifier: CC("$argon2i")
        // matches, then CC("$v=") sees "d$v=".
        sid.clone(),
        // 8.43 the empty string.
        String::new(),
        make_str("argon2i", 8, 1, 1, &salt16, &hash32[..8].to_vec()), // hash too short
        "$argon2i$v=19$m=8,t=1,p=1$".to_string(),
        "$argon2i$v=16$m=8,t=1,p=1$aaaaaaaaaaaa$bbbbbbbbbbbbbbbbbbbbbb".to_string(),
    ];
    for s in &malformed_id {
        let (a, ec) = ver(&civ, &riv, &format!("8.41/8.42/8.43 argon2i({s:?})"), s, &p, None);
        assert_eq!(a, -1, "8.41: {s:?} must be rejected");
        assert_eq!(ec, SENTINEL, "8.41: errno must be left alone for {s:?}");
    }
    // 8.46 an argon2i string fed to the argon2id verifier fails CC("$argon2id").
    for s in [si.clone(), String::new(), "$argon2id$".to_string()] {
        let (a, ec) = ver(&cdv, &rdv, &format!("8.46 argon2id({s:?})"), &s, &p, None);
        assert_eq!(a, -1);
        assert_eq!(ec, SENTINEL, "8.46: errno must be left alone for {s:?}");
    }
}

// ------------------------------ 8.47 – 8.52 `_needs_rehash` rejections

#[test]
fn e8_47_to_52_needs_rehash_rejections() {
    let _rng = rng_guard();
    let (cn, rn) = both::<NeedsRehash>("crypto_pwhash_str_needs_rehash");
    let (cin, rin) = both::<NeedsRehash>("crypto_pwhash_argon2i_str_needs_rehash");
    let (cdn, rdn) = both::<NeedsRehash>("crypto_pwhash_argon2id_str_needs_rehash");
    let (cs, rs) = both::<PwStr>("crypto_pwhash_str");
    let p = b"password".to_vec();
    let (_, _, good) = pwstr(&cs, &rs, "8.47 base string", &p, 1, 8192);

    #[track_caller]
    fn nr(c: &NeedsRehash, r: &NeedsRehash, label: &str, s: &str, ops: u64, mem: usize) -> (c_int, c_int) {
        let z = cstr(s);
        set_errno(SENTINEL);
        let a = unsafe { c(z.as_ptr() as *const c_char, ops, mem) };
        let ec = errno();
        set_errno(SENTINEL);
        let b = unsafe { r(z.as_ptr() as *const c_char, ops, mem) };
        let er = errno();
        eqi(&format!("{label} ret"), a, b);
        assert_eq!(ec, er, "{label}: errno mismatch (C {ec}, Rust {er})");
        (a, ec)
    }

    // 8.47 opslimit > UINT32_MAX.
    for ops in [4294967296u64, u64::MAX] {
        let (a, ec) = nr(&cdn, &rdn, &format!("8.47 opslimit={ops}"), &good, ops, 8192);
        assert_eq!(a, -1);
        assert_eq!(ec, EINVAL);
        let (a, ec) = nr(&cn, &rn, &format!("8.47 generic opslimit={ops}"), &good, ops, 8192);
        assert_eq!(a, -1);
        assert_eq!(ec, EINVAL);
    }

    // 8.48 memlimit / 1024 > UINT32_MAX.  The truncating division means the
    // first rejected value is 4294967296 * 1024 = 4398046511104 (the table's
    // "memlimit > 4398046511104" is off by one; the C code is authoritative).
    for mem in [4398046511104usize, 4398046511105, usize::MAX] {
        let (a, ec) = nr(&cdn, &rdn, &format!("8.48 memlimit={mem}"), &good, 1, mem);
        assert_eq!(a, -1);
        assert_eq!(ec, EINVAL);
    }
    // The largest memlimit that still passes the /1024 check is accepted (and
    // simply reports "needs rehash").
    let (a, _) = nr(&cdn, &rdn, "8.48 memlimit=4398046511103", &good, 1, 4398046511103);
    assert_eq!(a, 1, "8.48: memlimit/1024 == UINT32_MAX must not be an error");

    // 8.49 strlen(str) >= crypto_pwhash_STRBYTES (128).
    let salt42 = Rng::new(0x8_e049).bytes(42);
    let hash32 = Rng::new(0x8_e04a).bytes(32);
    let len127 = make_str("argon2id", 8, 1, 1, &salt42, &hash32);
    assert_eq!(len127.len(), 127);
    for extra in 1..6usize {
        let s = format!("{len127}{}", "x".repeat(extra));
        let (a, ec) = nr(&cdn, &rdn, &format!("8.49 len={}", s.len()), &s, 1, 8192);
        assert_eq!(a, -1, "8.49: a {}-char string must be rejected", s.len());
        assert_eq!(ec, EINVAL);
    }

    // 8.51 argon2_decode_string failures -> -1 with EINVAL.
    let bad: Vec<String> = vec![
        String::new(),
        "$argon2id$".to_string(),
        "$argon2id$v=19$".to_string(),
        "$argon2id$v=19$m=8,t=1,p=1$".to_string(),
        make_str("argon2id", 8, 1, 1, &salt42[..4].to_vec(), &hash32), // salt < 8 bytes
        make_str("argon2id", 8, 1, 1, &salt42, &hash32[..8].to_vec()), // hash < 16 bytes
        make_str("argon2id", 0, 1, 1, &salt42, &hash32),               // m = 0
        make_str("argon2id", 8, 0, 1, &salt42, &hash32),               // t = 0
        make_str("argon2id", 8, 1, 0, &salt42, &hash32),               // p = 0
        len127[..len127.len() - 1].to_string(),                        // truncated base64
        "$argon2id$v=20$m=8,t=1,p=1$aaaaaaaaaaaa$bbbbbbbbbbbbbbbbbbbbbb".to_string(),
    ];
    for s in &bad {
        if s.len() >= STRBYTES {
            continue;
        }
        let (a, ec) = nr(&cdn, &rdn, &format!("8.51 {s:?}"), s, 1, 8192);
        assert_eq!(a, -1, "8.51: {s:?} must be rejected");
        assert_eq!(ec, EINVAL, "8.51: {s:?} must set EINVAL");
    }

    // 8.52 a valid string whose t_cost or m_cost differs -> 1.
    for (ops, mem) in [(2u64, 8192usize), (1, 16384), (7, 65536)] {
        let (a, _) = nr(&cdn, &rdn, &format!("8.52 ({ops}, {mem})"), &good, ops, mem);
        assert_eq!(a, 1, "8.52 ({ops}, {mem}): expected 1");
    }
    // The argon2i entry point applies the identical logic (and, per row 8.52,
    // does not compare the argon2 *type*, so it happily decodes... no: the type
    // is part of the prefix, so an argon2id string is rejected here).
    let (a, ec) = nr(&cin, &rin, "8.51 argon2i vs argon2id string", &good, 1, 8192);
    assert_eq!(a, -1);
    assert_eq!(ec, EINVAL);
}
