//! Phase B — `crypto_pwhash/` : argon2i, argon2id and scryptsalsa208sha256.
//!
//! Covers every `CONFIGS.md` `## G2` row (`G2-001` … `G2-148`): the public
//! `crypto_pwhash*` entry points, the exported argon2 low-level API
//! (`_sodium_argon2_*`, `_sodium_argon2i*`, `_sodium_blake2b_long`) and the
//! exported scrypt internals (`_sodium_escrypt_*`).
//!
//! ## Cost control
//!
//! Argon2 and scrypt are deliberately expensive, so the *bulk* of the
//! randomised inputs run at the cheapest legal parameter point
//! (argon2i `opslimit = 3`, argon2id `opslimit = 1`, `memlimit = 8192`;
//! scrypt `_ll` with tiny explicit `N`/`r`/`p`, or `opslimit = 32768` /
//! `memlimit = 16777216` which yields `N = 1024, r = 8, p = 1`).
//! The `INTERACTIVE` presets are run exactly **once** each; the `MODERATE`
//! and `SENSITIVE` presets are **not** run at all (hundreds of MiB and
//! several seconds per call) — for those rows only the constant accessors are
//! compared, and each downscale is named in a comment next to the row id.

mod common;
use common::*;

use std::ffi::{c_char, c_void};

// ===========================================================================
// C signatures
// ===========================================================================

type FnSz = unsafe extern "C" fn() -> usize;
type FnU64 = unsafe extern "C" fn() -> u64;
type FnI32 = unsafe extern "C" fn() -> i32;
type FnCStr = unsafe extern "C" fn() -> *const c_char;

type Pwhash =
    unsafe extern "C" fn(*mut u8, u64, *const c_char, u64, *const u8, u64, usize, i32) -> i32;
type PwStr = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize) -> i32;
type PwStrAlg = unsafe extern "C" fn(*mut c_char, *const c_char, u64, u64, usize, i32) -> i32;
type PwStrVerify = unsafe extern "C" fn(*const c_char, *const c_char, u64) -> i32;
type PwNeedsRehash = unsafe extern "C" fn(*const c_char, u64, usize) -> i32;

type Scrypt = unsafe extern "C" fn(*mut u8, u64, *const c_char, u64, *const u8, u64, usize) -> i32;
type ScryptLl = unsafe extern "C" fn(
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
type Blake2bLong = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> i32;

type EGensalt = unsafe extern "C" fn(u32, u32, u32, *const u8, usize, *mut u8, usize) -> *mut u8;
type EParse = unsafe extern "C" fn(*const u8, *mut u32, *mut u32, *mut u32) -> *const u8;
type ER = unsafe extern "C" fn(*mut Region, *const u8, usize, *const u8, *mut u8, usize) -> *mut u8;
type ERegion1 = unsafe extern "C" fn(*mut Region) -> i32;
type EAlloc = unsafe extern "C" fn(*mut Region, usize) -> *mut c_void;
type EPbkdf2 = unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, *mut u8, usize);

/// `argon2.h`: `typedef struct Argon2_Context { ... } argon2_context;`
/// (`sizeof == 96`, offsets 0, 8, 16, 24, 32, 40, 48, 56, 64, 72, 76, 80, 84,
/// 88, 92 on x86-64).
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

/// `crypto_scrypt.h`: `typedef struct { void *base, *aligned; size_t size; }
/// escrypt_region_t;` (also used as `escrypt_local_t`).
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

const ARGON2_I: i32 = 1;
const ARGON2_ID: i32 = 2;
const ARGON2_OK: i32 = 0;
const ARGON2_FLAG_CLEAR_PASSWORD: u32 = 1;
const ARGON2_FLAG_CLEAR_SECRET: u32 = 2;

// ===========================================================================
// helpers
// ===========================================================================

/// `reset_rngs()` rewinds *process-global* state shared by both libraries, and
/// `cargo test` runs the `#[test]`s of one binary in parallel threads, so every
/// `reset_rngs(); call C; reset_rngs(); call Rust` sequence must be serialised.
/// Every helper below that drives a `randombytes_buf`-consuming entry point
/// (`crypto_pwhash_*_str`, `*_str_verify` -> `argon2_verify` -> `argon2_hash`,
/// `escrypt_r`, `argon2_hash` with `hash != NULL`) takes this lock.
static RNG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn rng_lock() -> std::sync::MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Read a NUL-terminated C string.
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
        assert!(i < 4096, "unterminated C string");
    }
    v
}

/// NUL-terminate a byte string so that `strlen`-based C code is in bounds.
fn nul(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.push(0);
    v
}

fn sz(name: &str) -> usize {
    let (c, r) = pair::<FnSz>(name);
    let (a, b) = unsafe { (c(), r()) };
    eq_usize(name, a, b);
    a
}

fn u64v(name: &str) -> u64 {
    let (c, r) = pair::<FnU64>(name);
    let (a, b) = unsafe { (c(), r()) };
    assert_eq!(a, b, "{name}: C={a} Rust={b}");
    a
}

fn i32v(name: &str) -> i32 {
    let (c, r) = pair::<FnI32>(name);
    let (a, b) = unsafe { (c(), r()) };
    eq_i32(name, a, b);
    a
}

fn strv(name: &str) -> Vec<u8> {
    let (c, r) = pair::<FnCStr>(name);
    let (a, b) = unsafe { (cstr(c()), cstr(r())) };
    eq_bytes(name, &a, &b);
    a
}

/// A password buffer whose backing allocation is always non-empty (so that the
/// pointer we hand to C is a real, readable address even when `len == 0`).
fn pwbuf(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut v = rng.bytes(len);
    v.push(0xEE);
    v
}

/// Run one `crypto_pwhash`-shaped entry point on both libraries, comparing the
/// return value and the *entire* output buffer (with a canary tail so an
/// over-write past `outlen` is visible).
fn cmp_pw(
    name: &str,
    tag: &str,
    outlen: usize,
    pw: &[u8],
    pwlen: u64,
    salt: &[u8],
    ops: u64,
    mem: usize,
    alg: i32,
) -> (i32, Vec<u8>) {
    let (c, r) = pair::<Pwhash>(name);
    let mut a = canary(outlen + 8);
    let mut b = canary(outlen + 8);
    let (ra, rb) = unsafe {
        (
            c(
                a.as_mut_ptr(),
                outlen as u64,
                pw.as_ptr() as *const c_char,
                pwlen,
                salt.as_ptr(),
                ops,
                mem,
                alg,
            ),
            r(
                b.as_mut_ptr(),
                outlen as u64,
                pw.as_ptr() as *const c_char,
                pwlen,
                salt.as_ptr(),
                ops,
                mem,
                alg,
            ),
        )
    };
    eq_i32(&format!("{name} rc [{tag}]"), ra, rb);
    eq_bytes(&format!("{name} out [{tag}]"), &a, &b);
    assert_eq!(
        &a[outlen..],
        &canary(8)[..],
        "{name} [{tag}] wrote past outlen"
    );
    (ra, a[..outlen].to_vec())
}

/// `crypto_pwhash_*_str` on both libraries. `escrypt_r` / `argon2*_str` consume
/// `randombytes_buf`, so both RNG streams are rewound to `seed` first.
fn cmp_str(
    name: &str,
    tag: &str,
    pw: &[u8],
    pwlen: u64,
    ops: u64,
    mem: usize,
    strbytes: usize,
    seed: u64,
) -> (i32, Vec<u8>) {
    let _g = rng_lock();
    let (c, r) = pair::<PwStr>(name);
    let mut a = canary(strbytes);
    let mut b = canary(strbytes);
    reset_rngs(seed);
    let ra = unsafe {
        c(
            a.as_mut_ptr() as *mut c_char,
            pw.as_ptr() as *const c_char,
            pwlen,
            ops,
            mem,
        )
    };
    reset_rngs(seed);
    let rb = unsafe {
        r(
            b.as_mut_ptr() as *mut c_char,
            pw.as_ptr() as *const c_char,
            pwlen,
            ops,
            mem,
        )
    };
    eq_i32(&format!("{name} rc [{tag}]"), ra, rb);
    eq_bytes(&format!("{name} full buffer [{tag}]"), &a, &b);
    // also compare as a C string (NUL-terminated, printable ASCII)
    let (ca, cb) = unsafe {
        (
            cstr(a.as_ptr() as *const c_char),
            cstr(b.as_ptr() as *const c_char),
        )
    };
    eq_bytes(&format!("{name} C string [{tag}]"), &ca, &cb);
    if ra == 0 {
        assert!(
            ca.iter().all(|&x| (0x20..0x7f).contains(&x)),
            "{name} [{tag}] produced non-ASCII: {:?}",
            String::from_utf8_lossy(&ca)
        );
        assert!(ca.len() < strbytes, "{name} [{tag}] not NUL-terminated");
    }
    (ra, a)
}

fn cmp_str_alg(
    name: &str,
    tag: &str,
    pw: &[u8],
    pwlen: u64,
    ops: u64,
    mem: usize,
    alg: i32,
    seed: u64,
) -> (i32, Vec<u8>) {
    let _g = rng_lock();
    let (c, r) = pair::<PwStrAlg>(name);
    let mut a = canary(128);
    let mut b = canary(128);
    reset_rngs(seed);
    let ra = unsafe {
        c(
            a.as_mut_ptr() as *mut c_char,
            pw.as_ptr() as *const c_char,
            pwlen,
            ops,
            mem,
            alg,
        )
    };
    reset_rngs(seed);
    let rb = unsafe {
        r(
            b.as_mut_ptr() as *mut c_char,
            pw.as_ptr() as *const c_char,
            pwlen,
            ops,
            mem,
            alg,
        )
    };
    eq_i32(&format!("{name} rc [{tag}]"), ra, rb);
    eq_bytes(&format!("{name} full buffer [{tag}]"), &a, &b);
    (ra, a)
}

fn cmp_verify(name: &str, tag: &str, s: &[u8], pw: &[u8], pwlen: u64, seed: u64) -> i32 {
    let _g = rng_lock();
    let (c, r) = pair::<PwStrVerify>(name);
    reset_rngs(seed);
    let ra = unsafe {
        c(
            s.as_ptr() as *const c_char,
            pw.as_ptr() as *const c_char,
            pwlen,
        )
    };
    reset_rngs(seed);
    let rb = unsafe {
        r(
            s.as_ptr() as *const c_char,
            pw.as_ptr() as *const c_char,
            pwlen,
        )
    };
    eq_i32(&format!("{name} rc [{tag}]"), ra, rb);
    ra
}

fn cmp_needs_rehash(name: &str, tag: &str, s: &[u8], ops: u64, mem: usize) -> i32 {
    let (c, r) = pair::<PwNeedsRehash>(name);
    let (ra, rb) = unsafe {
        (
            c(s.as_ptr() as *const c_char, ops, mem),
            r(s.as_ptr() as *const c_char, ops, mem),
        )
    };
    eq_i32(&format!("{name} rc [{tag}]"), ra, rb);
    ra
}

// ===========================================================================
// G2-115 … G2-148 — constant accessors
// ===========================================================================

/// CONFIGS G2-115, G2-116, G2-117, G2-118, G2-119, G2-120, G2-121, G2-122,
/// G2-123, G2-124, G2-125, G2-126, G2-127, G2-128, G2-129, G2-130, G2-131,
/// G2-132, G2-133, G2-134, G2-135, G2-136, G2-137, G2-138, G2-139, G2-140,
/// G2-141, G2-142, G2-143, G2-144, G2-145, G2-146, G2-147, G2-148.
///
/// Also stands in for the CONFIGS rows whose *work* was downscaled: G2-029,
/// G2-030 (argon2i MODERATE/SENSITIVE) and G2-031, G2-032 (argon2id
/// MODERATE/SENSITIVE) — their `opslimit`/`memlimit` pairs are asserted here
/// instead of being hashed with.
#[test]
fn pwhash_constant_accessors_match() {
    setup();

    // ---- crypto_pwhash (aliases argon2id) -------------------------------
    assert_eq!(strv("crypto_pwhash_primitive"), b"argon2id,argon2i"); // G2-115
    assert_eq!(i32v("crypto_pwhash_alg_argon2i13"), 1); // G2-116
    assert_eq!(i32v("crypto_pwhash_alg_argon2id13"), 2);
    assert_eq!(i32v("crypto_pwhash_alg_default"), 2);
    assert_eq!(sz("crypto_pwhash_bytes_min"), 16); // G2-117
    assert_eq!(sz("crypto_pwhash_bytes_max"), 4294967295);
    assert_eq!(sz("crypto_pwhash_passwd_min"), 0); // G2-118
    assert_eq!(sz("crypto_pwhash_passwd_max"), 4294967295);
    assert_eq!(sz("crypto_pwhash_saltbytes"), 16); // G2-119
    assert_eq!(sz("crypto_pwhash_strbytes"), 128);
    assert_eq!(strv("crypto_pwhash_strprefix"), b"$argon2id$");
    assert_eq!(u64v("crypto_pwhash_opslimit_min"), 1); // G2-120
    assert_eq!(u64v("crypto_pwhash_opslimit_max"), 4294967295);
    assert_eq!(sz("crypto_pwhash_memlimit_min"), 8192);
    assert_eq!(sz("crypto_pwhash_memlimit_max"), 4398046510080);
    assert_eq!(u64v("crypto_pwhash_opslimit_interactive"), 2); // G2-121
    assert_eq!(sz("crypto_pwhash_memlimit_interactive"), 67108864);
    assert_eq!(u64v("crypto_pwhash_opslimit_moderate"), 3); // G2-122
    assert_eq!(sz("crypto_pwhash_memlimit_moderate"), 268435456);
    assert_eq!(u64v("crypto_pwhash_opslimit_sensitive"), 4); // G2-123
    assert_eq!(sz("crypto_pwhash_memlimit_sensitive"), 1073741824);

    // ---- argon2i --------------------------------------------------------
    assert_eq!(i32v("crypto_pwhash_argon2i_alg_argon2i13"), 1); // G2-124
    assert_eq!(sz("crypto_pwhash_argon2i_bytes_min"), 16); // G2-125
    assert_eq!(sz("crypto_pwhash_argon2i_bytes_max"), 4294967295);
    assert_eq!(sz("crypto_pwhash_argon2i_passwd_min"), 0); // G2-126
    assert_eq!(sz("crypto_pwhash_argon2i_passwd_max"), 4294967295);
    assert_eq!(sz("crypto_pwhash_argon2i_saltbytes"), 16); // G2-127
    assert_eq!(sz("crypto_pwhash_argon2i_strbytes"), 128);
    assert_eq!(strv("crypto_pwhash_argon2i_strprefix"), b"$argon2i$");
    assert_eq!(u64v("crypto_pwhash_argon2i_opslimit_min"), 3); // G2-128 (three!)
    assert_eq!(u64v("crypto_pwhash_argon2i_opslimit_max"), 4294967295);
    assert_eq!(sz("crypto_pwhash_argon2i_memlimit_min"), 8192); // G2-129
    assert_eq!(sz("crypto_pwhash_argon2i_memlimit_max"), 4398046510080);
    assert_eq!(u64v("crypto_pwhash_argon2i_opslimit_interactive"), 4); // G2-130
    assert_eq!(sz("crypto_pwhash_argon2i_memlimit_interactive"), 33554432);
    assert_eq!(u64v("crypto_pwhash_argon2i_opslimit_moderate"), 6); // G2-131/G2-029
    assert_eq!(sz("crypto_pwhash_argon2i_memlimit_moderate"), 134217728);
    assert_eq!(u64v("crypto_pwhash_argon2i_opslimit_sensitive"), 8); // G2-132/G2-030
    assert_eq!(sz("crypto_pwhash_argon2i_memlimit_sensitive"), 536870912);

    // ---- argon2id -------------------------------------------------------
    assert_eq!(i32v("crypto_pwhash_argon2id_alg_argon2id13"), 2); // G2-133
    assert_eq!(sz("crypto_pwhash_argon2id_bytes_min"), 16); // G2-134
    assert_eq!(sz("crypto_pwhash_argon2id_bytes_max"), 4294967295);
    assert_eq!(sz("crypto_pwhash_argon2id_passwd_min"), 0); // G2-135
    assert_eq!(sz("crypto_pwhash_argon2id_passwd_max"), 4294967295);
    assert_eq!(sz("crypto_pwhash_argon2id_saltbytes"), 16); // G2-136
    assert_eq!(sz("crypto_pwhash_argon2id_strbytes"), 128);
    assert_eq!(strv("crypto_pwhash_argon2id_strprefix"), b"$argon2id$");
    assert_eq!(u64v("crypto_pwhash_argon2id_opslimit_min"), 1); // G2-137 (one!)
    assert_eq!(u64v("crypto_pwhash_argon2id_opslimit_max"), 4294967295);
    assert_eq!(sz("crypto_pwhash_argon2id_memlimit_min"), 8192); // G2-138
    assert_eq!(sz("crypto_pwhash_argon2id_memlimit_max"), 4398046510080);
    assert_eq!(u64v("crypto_pwhash_argon2id_opslimit_interactive"), 2); // G2-139
    assert_eq!(sz("crypto_pwhash_argon2id_memlimit_interactive"), 67108864);
    assert_eq!(u64v("crypto_pwhash_argon2id_opslimit_moderate"), 3); // G2-140/G2-031
    assert_eq!(sz("crypto_pwhash_argon2id_memlimit_moderate"), 268435456);
    assert_eq!(u64v("crypto_pwhash_argon2id_opslimit_sensitive"), 4); // G2-141/G2-032
    assert_eq!(sz("crypto_pwhash_argon2id_memlimit_sensitive"), 1073741824);

    // ---- scryptsalsa208sha256 ------------------------------------------
    let p = "crypto_pwhash_scryptsalsa208sha256";
    assert_eq!(sz(&format!("{p}_bytes_min")), 16); // G2-142
    assert_eq!(sz(&format!("{p}_bytes_max")), 137438953440);
    assert_eq!(sz(&format!("{p}_passwd_min")), 0); // G2-143
    assert_eq!(sz(&format!("{p}_passwd_max")), usize::MAX);
    assert_eq!(sz(&format!("{p}_saltbytes")), 32); // G2-144
    assert_eq!(sz(&format!("{p}_strbytes")), 102);
    assert_eq!(strv(&format!("{p}_strprefix")), b"$7$");
    assert_eq!(u64v(&format!("{p}_opslimit_min")), 32768); // G2-145
    assert_eq!(u64v(&format!("{p}_opslimit_max")), 4294967295);
    assert_eq!(sz(&format!("{p}_memlimit_min")), 16777216); // G2-146
    assert_eq!(sz(&format!("{p}_memlimit_max")), 68719476736);
    assert_eq!(u64v(&format!("{p}_opslimit_interactive")), 524288); // G2-147
    assert_eq!(sz(&format!("{p}_memlimit_interactive")), 16777216);
    assert_eq!(u64v(&format!("{p}_opslimit_sensitive")), 33554432); // G2-148
    assert_eq!(sz(&format!("{p}_memlimit_sensitive")), 1073741824);
    // G2-148 also records that scrypt has *no* MODERATE accessors.
    assert!(
        !has_sym(&format!("{p}_opslimit_moderate")),
        "scrypt must not export an _opslimit_moderate accessor"
    );
    assert!(!has_sym(&format!("{p}_memlimit_moderate")));
}

// ===========================================================================
// G2-001 … G2-024, G2-043, G2-044 — the public `crypto_pwhash` KDF
// ===========================================================================

/// CONFIGS G2-001, G2-002, G2-003, G2-005, G2-006, G2-007, G2-008, G2-009,
/// G2-023, G2-043 — argon2i at its cheapest legal point, sweeping the
/// `blake2b_long` output-length boundaries (16/17/32/64/65/96/97/1000) and the
/// `t_cost` = 3 (MIN) / 4 (INTERACTIVE ops) pass counts, driven through BOTH
/// `crypto_pwhash(alg=1)` and `crypto_pwhash_argon2i` with randomised
/// passwords and salts.
#[test]
fn pwhash_argon2i_cheap_matrix() {
    setup();
    let mut rng = Rng::new(0x1200_0001);
    // (outlen, passwdlen) pairs pinned by the CONFIGS rows …
    let pinned: &[(usize, usize)] = &[
        (16, 0),   // G2-001
        (32, 1),   // G2-002
        (32, 8),   // G2-003 (with opslimit 4 below)
        (1000, 8), // G2-005
        (64, 100), // G2-006
        (65, 8),   // G2-007
        (96, 8),   // G2-008
        (97, 8),   // G2-009
        (17, 8),   // G2-023
    ];
    for &ops in &[3u64, 4] {
        // 3 = argon2i OPSLIMIT_MIN, 4 = argon2i OPSLIMIT_INTERACTIVE (ops only)
        for &(outlen, pwlen) in pinned {
            for zero_salt in [true, false] {
                let pw = pwbuf(&mut rng, pwlen);
                let salt = if zero_salt {
                    vec![0u8; 16]
                } else {
                    rng.bytes(16)
                };
                let tag = format!("ops={ops} out={outlen} pw={pwlen} zsalt={zero_salt}");
                let (rc1, d1) = cmp_pw(
                    "crypto_pwhash",
                    &tag,
                    outlen,
                    &pw,
                    pwlen as u64,
                    &salt,
                    ops,
                    8192,
                    ARGON2_I,
                );
                assert_eq!(rc1, 0, "crypto_pwhash argon2i [{tag}] must succeed");
                // G2-043: the direct entry point must agree bit-for-bit
                let (rc2, d2) = cmp_pw(
                    "crypto_pwhash_argon2i",
                    &tag,
                    outlen,
                    &pw,
                    pwlen as u64,
                    &salt,
                    ops,
                    8192,
                    ARGON2_I,
                );
                assert_eq!(rc2, 0);
                eq_bytes(&format!("crypto_pwhash == _argon2i [{tag}]"), &d1, &d2);
            }
        }
    }
    // randomised sweep at the cheapest point
    for i in 0..40 {
        let outlen = rng.range(16, 200);
        let pwlen = rng.below(200);
        let pw = pwbuf(&mut rng, pwlen);
        let salt = rng.bytes(16);
        let tag = format!("rand#{i} out={outlen} pw={pwlen}");
        let (rc, _) = cmp_pw(
            "crypto_pwhash",
            &tag,
            outlen,
            &pw,
            pwlen as u64,
            &salt,
            3,
            8192,
            ARGON2_I,
        );
        assert_eq!(rc, 0);
    }
}

/// CONFIGS G2-010, G2-011, G2-012, G2-013, G2-014, G2-016, G2-017, G2-018,
/// G2-019, G2-021, G2-022, G2-023, G2-044 — argon2id: all four `t_cost`
/// values 1..4 at minimum memory (covers every slice and both the
/// data-independent and data-dependent addressing halves), plus the
/// `segment_length` boundaries `m_cost` = 8/9/10/11/12/512/516.
#[test]
fn pwhash_argon2id_cheap_matrix() {
    setup();
    let mut rng = Rng::new(0x1200_0002);

    // G2-010 .. G2-014, G2-021, G2-022, G2-023: t_cost 1..4 at MEMLIMIT_MIN
    for ops in 1u64..=4 {
        for &(outlen, pwlen) in &[(16usize, 0usize), (32, 8), (17, 1), (64, 100)] {
            for zero_salt in [true, false] {
                let pw = pwbuf(&mut rng, pwlen);
                let salt = if zero_salt {
                    vec![0u8; 16]
                } else {
                    rng.bytes(16)
                };
                let tag = format!("ops={ops} out={outlen} pw={pwlen} zsalt={zero_salt}");
                let (rc1, d1) = cmp_pw(
                    "crypto_pwhash",
                    &tag,
                    outlen,
                    &pw,
                    pwlen as u64,
                    &salt,
                    ops,
                    8192,
                    ARGON2_ID,
                );
                assert_eq!(rc1, 0);
                // G2-044
                let (rc2, d2) = cmp_pw(
                    "crypto_pwhash_argon2id",
                    &tag,
                    outlen,
                    &pw,
                    pwlen as u64,
                    &salt,
                    ops,
                    8192,
                    ARGON2_ID,
                );
                assert_eq!(rc2, 0);
                eq_bytes(&format!("crypto_pwhash == _argon2id [{tag}]"), &d1, &d2);
            }
        }
    }

    // G2-016 (m_cost 512 == ARGON2_ADDRESSES_IN_BLOCK boundary),
    // G2-017 (m_cost 516 -> a second address block),
    // G2-018 (m_cost 9/10/11 all truncate to segment_length 2),
    // G2-019 (m_cost 12 -> segment_length 3).
    for &mem in &[9216usize, 10240, 11264, 12288, 524288, 528384] {
        let pw = pwbuf(&mut rng, 8);
        let salt = rng.bytes(16);
        let tag = format!("mem={mem}");
        let (rc, _) = cmp_pw(
            "crypto_pwhash",
            &tag,
            32,
            &pw,
            8,
            &salt,
            1,
            mem,
            ARGON2_ID,
        );
        assert_eq!(rc, 0);
    }
    // G2-018 explicitly: `m_cost` 9/10/11 all truncate to `segment_length` 2 /
    // `memory_blocks` 8 (i.e. the *work* is the same as `m_cost` 8), but the
    // raw `context->m_cost` is also folded into the initial BLAKE2b hash
    // (`argon2-core.c:397`), so the digests must all be DIFFERENT. Verified on
    // both libraries.
    let pw = pwbuf(&mut rng, 8);
    let salt = rng.bytes(16);
    let mut ds = vec![cmp_pw("crypto_pwhash", "m=8", 32, &pw, 8, &salt, 1, 8192, ARGON2_ID).1];
    for &mem in &[9216usize, 10240, 11264] {
        ds.push(
            cmp_pw(
                "crypto_pwhash",
                &format!("m={mem}"),
                32,
                &pw,
                8,
                &salt,
                1,
                mem,
                ARGON2_ID,
            )
            .1,
        );
    }
    for i in 0..ds.len() {
        for j in (i + 1)..ds.len() {
            assert_ne!(
                ds[i], ds[j],
                "m_cost {i}/{j}: the un-rounded m_cost enters the initial hash"
            );
        }
    }

    // randomised sweep
    for i in 0..40 {
        let outlen = rng.range(16, 200);
        let pwlen = rng.below(200);
        let pw = pwbuf(&mut rng, pwlen);
        let salt = rng.bytes(16);
        let (rc, _) = cmp_pw(
            "crypto_pwhash",
            &format!("rand#{i}"),
            outlen,
            &pw,
            pwlen as u64,
            &salt,
            1,
            8192,
            ARGON2_ID,
        );
        assert_eq!(rc, 0);
    }
}

/// CONFIGS G2-004 (argon2i `MEMLIMIT_INTERACTIVE` = 32 MiB, `opslimit` = 3)
/// and G2-015 (argon2id `MEMLIMIT_INTERACTIVE` = 64 MiB, `opslimit` = 2) —
/// the canonical INTERACTIVE points. Run **once each** (~0.5 s + ~0.7 s per
/// library); the MODERATE/SENSITIVE rows G2-029…G2-032 are downscaled to a
/// constant comparison in `pwhash_constant_accessors_match`.
#[test]
fn pwhash_interactive_presets_once() {
    setup();
    let mut rng = Rng::new(0x1200_0003);
    let pw = pwbuf(&mut rng, 8);
    let salt = rng.bytes(16);
    // G2-004
    let (rc, _) = cmp_pw(
        "crypto_pwhash",
        "argon2i INTERACTIVE mem",
        32,
        &pw,
        8,
        &salt,
        3,
        33554432,
        ARGON2_I,
    );
    assert_eq!(rc, 0);
    // G2-015
    let (rc, _) = cmp_pw(
        "crypto_pwhash",
        "argon2id INTERACTIVE",
        32,
        &pw,
        8,
        &salt,
        2,
        67108864,
        ARGON2_ID,
    );
    assert_eq!(rc, 0);
}

/// CONFIGS G2-020, G2-021, G2-022 — passwords containing embedded NUL bytes
/// (hashed by explicit length, never by `strlen`), the empty password, and
/// zero vs random salts, for both algorithms.
#[test]
fn pwhash_password_and_salt_shapes() {
    setup();
    let mut rng = Rng::new(0x1200_0004);
    let nul_pws: Vec<Vec<u8>> = vec![
        b"a\0b\0c\0d\0".to_vec(),
        b"\0\0\0\0\0\0\0\0".to_vec(),
        b"\0abc".to_vec(),
        b"abc\0".to_vec(),
        vec![0u8; 100],
    ];
    for (alg, ops) in [(ARGON2_I, 3u64), (ARGON2_ID, 1u64)] {
        // G2-020: NUL-containing passwords must all give distinct digests
        let mut digests = Vec::new();
        for (i, p) in nul_pws.iter().enumerate() {
            let mut pw = p.clone();
            let n = p.len();
            pw.push(0xEE);
            let salt = [7u8; 16];
            let (rc, d) = cmp_pw(
                "crypto_pwhash",
                &format!("alg={alg} nulpw#{i}"),
                32,
                &pw,
                n as u64,
                &salt,
                ops,
                8192,
                alg,
            );
            assert_eq!(rc, 0);
            digests.push(d);
        }
        for i in 0..digests.len() {
            for j in (i + 1)..digests.len() {
                assert_ne!(
                    digests[i], digests[j],
                    "alg={alg}: NUL-containing passwords #{i}/#{j} collided \
                     (password would have been treated as a C string)"
                );
            }
        }
        // G2-021: passwdlen == 0 with a valid pointer
        let pw = pwbuf(&mut rng, 0);
        let salt = rng.bytes(16);
        let (rc, empty) = cmp_pw(
            "crypto_pwhash",
            &format!("alg={alg} empty pw"),
            32,
            &pw,
            0,
            &salt,
            ops,
            8192,
            alg,
        );
        assert_eq!(rc, 0);
        // …and it must differ from a 1-byte password
        let pw1 = pwbuf(&mut rng, 1);
        let (_, one) = cmp_pw(
            "crypto_pwhash",
            &format!("alg={alg} 1-byte pw"),
            32,
            &pw1,
            1,
            &salt,
            ops,
            8192,
            alg,
        );
        assert_ne!(empty, one);
        // G2-022: zero vs random salt
        let pw = pwbuf(&mut rng, 8);
        let (_, z) = cmp_pw(
            "crypto_pwhash",
            &format!("alg={alg} zero salt"),
            32,
            &pw,
            8,
            &[0u8; 16],
            ops,
            8192,
            alg,
        );
        let rsalt = rng.bytes(16);
        let (_, r) = cmp_pw(
            "crypto_pwhash",
            &format!("alg={alg} rand salt"),
            32,
            &pw,
            8,
            &rsalt,
            ops,
            8192,
            alg,
        );
        assert_ne!(z, r);
    }
}

/// CONFIGS G2-024 — `opslimit = 4294967295` (`OPSLIMIT_MAX`) is *accepted* by
/// the range checks. Actually hashing with `t_cost = 0xFFFFFFFF` is
/// astronomically slow, so instead we observe the *classification*: with a
/// deliberately unhandled `alg` the range checks are still reached first, so
/// `opslimit = OPSLIMIT_MAX` yields `EINVAL` (from the `switch (alg)` default)
/// while `opslimit = OPSLIMIT_MAX + 1` yields `EFBIG` (from the range check).
#[test]
fn pwhash_opslimit_max_passes_the_range_checks() {
    setup();
    const EINVAL: i32 = 22;
    const EFBIG: i32 = 27;
    let pw = b"password\0";
    let salt = [1u8; 16];
    // NB: `crypto_pwhash` dispatches on `alg` *before* any range check, so the
    // classification below is only observable on the two per-algorithm entry
    // points (`pwhash_argon2i.c:154-165` / `pwhash_argon2id.c:150-161`).
    for name in ["crypto_pwhash_argon2i", "crypto_pwhash_argon2id"] {
        // `alg` intentionally unhandled by both entry points
        let bogus_alg = 7;
        for (ops, want) in [(4294967295u64, EINVAL), (4294967296u64, EFBIG)] {
            let (c, r) = pair::<Pwhash>(name);
            for (which, f) in [("C", c), ("R", r)].into_iter() {
                let mut out = canary(32);
                let _ = std::fs::metadata("/");
                let rc = unsafe {
                    f(
                        out.as_mut_ptr(),
                        32,
                        pw.as_ptr() as *const c_char,
                        8,
                        salt.as_ptr(),
                        ops,
                        8192,
                        bogus_alg,
                    )
                };
                let e = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                assert_eq!(rc, -1, "{which} {name} ops={ops}");
                assert_eq!(e, want, "{which} {name} ops={ops} errno");
            }
        }
    }
    // and `crypto_pwhash` itself: the `alg` switch always wins -> EINVAL
    let (c, r) = pair::<Pwhash>("crypto_pwhash");
    for (which, f) in [("C", c), ("R", r)] {
        for ops in [4294967295u64, 4294967296] {
            let mut out = canary(32);
            let _ = std::fs::metadata("/");
            let rc = unsafe {
                f(
                    out.as_mut_ptr(),
                    32,
                    pw.as_ptr() as *const c_char,
                    8,
                    salt.as_ptr(),
                    ops,
                    8192,
                    7,
                )
            };
            let e = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            assert_eq!((rc, e), (-1, EINVAL), "{which} crypto_pwhash alg=7 ops={ops}");
        }
    }
}

// ===========================================================================
// G2-025 … G2-042 — the encoded-string API
// ===========================================================================

/// CONFIGS G2-025, G2-026, G2-027, G2-028 — `crypto_pwhash_str` (always
/// argon2id) and `crypto_pwhash_str_alg` for both algorithms. The full
/// 128-byte `crypto_pwhash_STRBYTES` buffer is compared (canary-filled first,
/// so the `memset(out, 0, STRBYTES)` and the trailing NULs are checked too).
///
/// The MODERATE/SENSITIVE preset rows G2-029…G2-032 are downscaled — see
/// `pwhash_constant_accessors_match`.
#[test]
fn pwhash_str_and_str_alg() {
    setup();
    let mut rng = Rng::new(0x1200_0005);

    // G2-025: crypto_pwhash_str delegates to argon2id (so opslimit >= 1 is OK)
    for &pwlen in &[0usize, 1, 8, 100] {
        let pw = pwbuf(&mut rng, pwlen);
        let (rc, buf) = cmp_str(
            "crypto_pwhash_str",
            &format!("pw={pwlen}"),
            &pw,
            pwlen as u64,
            1,
            8192,
            128,
            0xA100 + pwlen as u64,
        );
        assert_eq!(rc, 0);
        let s = unsafe { cstr(buf.as_ptr() as *const c_char) };
        assert!(
            s.starts_with(b"$argon2id$v=19$m=8,t=1,p=1$"),
            "crypto_pwhash_str prefix: {:?}",
            String::from_utf8_lossy(&s)
        );
    }

    // G2-026: passwdlen = 0 with MEMLIMIT_INTERACTIVE (~0.7 s per library,
    // run once); shape "$argon2id$v=19$m=65536,t=2,p=1$<22>$<43>".
    let pw = pwbuf(&mut rng, 0);
    let (rc, buf) = cmp_str(
        "crypto_pwhash_str",
        "INTERACTIVE",
        &pw,
        0,
        2,
        67108864,
        128,
        0xA200,
    );
    assert_eq!(rc, 0);
    let s = unsafe { cstr(buf.as_ptr() as *const c_char) };
    assert!(s.starts_with(b"$argon2id$v=19$m=65536,t=2,p=1$"));
    // 31-char prefix + 22 salt chars + '$' + 43 hash chars = 97
    assert_eq!(s.len(), 97, "{:?}", String::from_utf8_lossy(&s));
    assert_eq!(&buf[97..], &vec![0u8; 31][..], "tail must stay zeroed");

    // G2-027 / G2-028: crypto_pwhash_str_alg
    for (alg, ops, prefix) in [
        (ARGON2_I, 3u64, &b"$argon2i$v=19$m=8,t=3,p=1$"[..]),
        (ARGON2_ID, 1u64, &b"$argon2id$v=19$m=8,t=1,p=1$"[..]),
    ] {
        for &pwlen in &[0usize, 8, 64] {
            let pw = pwbuf(&mut rng, pwlen);
            let (rc, buf) = cmp_str_alg(
                "crypto_pwhash_str_alg",
                &format!("alg={alg} pw={pwlen}"),
                &pw,
                pwlen as u64,
                ops,
                8192,
                alg,
                0xA300 + pwlen as u64,
            );
            assert_eq!(rc, 0);
            let s = unsafe { cstr(buf.as_ptr() as *const c_char) };
            assert!(
                s.starts_with(prefix),
                "alg={alg}: {:?}",
                String::from_utf8_lossy(&s)
            );
        }
    }
}

/// CONFIGS G2-033, G2-034, G2-035, G2-036 — `crypto_pwhash_str` /
/// `_str_alg` → `crypto_pwhash_str_verify` round trips (argon2id and argon2i
/// prefix dispatch), including the empty password and NUL-containing
/// passwords, plus a one-byte mutation of every position of the hash string
/// (must fail identically in C and Rust).
#[test]
fn pwhash_str_round_trips_and_mutations() {
    setup();
    let mut rng = Rng::new(0x1200_0006);
    let passwords: Vec<Vec<u8>> = vec![
        b"".to_vec(),          // G2-035
        b"x".to_vec(),
        b"correct horse".to_vec(),
        b"a\0b\0c\0d\0".to_vec(), // G2-036
        rng.bytes(100),
    ];
    for (pi, p) in passwords.iter().enumerate() {
        let n = p.len();
        let pw = {
            let mut v = p.clone();
            v.push(0xEE);
            v
        };
        for (alg, ops) in [(ARGON2_ID, 1u64), (ARGON2_I, 3u64)] {
            let (rc, buf) = cmp_str_alg(
                "crypto_pwhash_str_alg",
                &format!("rt alg={alg} pw#{pi}"),
                &pw,
                n as u64,
                ops,
                8192,
                alg,
                0xB000 + pi as u64,
            );
            assert_eq!(rc, 0);
            // G2-033 / G2-034: verify with the same password
            let rc = cmp_verify(
                "crypto_pwhash_str_verify",
                &format!("rt alg={alg} pw#{pi}"),
                &buf,
                &pw,
                n as u64,
                0xB100,
            );
            assert_eq!(rc, 0, "alg={alg} pw#{pi} round trip must verify");
            // the alg-specific verifier must agree
            let vname = if alg == ARGON2_I {
                "crypto_pwhash_argon2i_str_verify"
            } else {
                "crypto_pwhash_argon2id_str_verify"
            };
            assert_eq!(
                cmp_verify(vname, &format!("direct alg={alg} pw#{pi}"), &buf, &pw, n as u64, 0xB101),
                0
            );
            // wrong password must be rejected identically. For the empty
            // password there is no byte to flip, so a 1-byte password is used
            // instead.
            let (wrong, wlen) = if n == 0 {
                (b"x\0".to_vec(), 1usize)
            } else {
                let mut w = pw.clone();
                w[0] ^= 0x01;
                (w, n)
            };
            assert_eq!(
                cmp_verify(
                    "crypto_pwhash_str_verify",
                    &format!("wrong pw alg={alg} pw#{pi}"),
                    &buf,
                    &wrong,
                    wlen as u64,
                    0xB102
                ),
                -1
            );
            // one-byte mutation of every position of the encoded string
            let slen = unsafe { cstr(buf.as_ptr() as *const c_char) }.len();
            for pos in 0..slen {
                let mut m = buf.clone();
                m[pos] ^= 0x01;
                let rc = cmp_verify(
                    "crypto_pwhash_str_verify",
                    &format!("mutate alg={alg} pw#{pi} pos={pos}"),
                    &m,
                    &pw,
                    n as u64,
                    0xB200 + pos as u64,
                );
                assert_eq!(
                    rc, -1,
                    "alg={alg} pw#{pi}: mutating byte {pos} still verified"
                );
            }
            // truncation at every length must also be rejected identically
            for cut in [0usize, 1, 9, 10, 25, 30, slen / 2, slen - 1] {
                let mut m = buf[..cut].to_vec();
                m.push(0);
                cmp_verify(
                    "crypto_pwhash_str_verify",
                    &format!("truncate alg={alg} pw#{pi} cut={cut}"),
                    &m,
                    &pw,
                    n as u64,
                    0xB300 + cut as u64,
                );
            }
        }
    }
}

/// CONFIGS G2-037, G2-038, G2-039, G2-040, G2-041, G2-042 —
/// `crypto_pwhash_str_needs_rehash` and the two alg-specific variants.
#[test]
fn pwhash_str_needs_rehash() {
    setup();
    let mut rng = Rng::new(0x1200_0007);
    let pw = pwbuf(&mut rng, 8);

    for (alg, ops, nr) in [
        (ARGON2_ID, 1u64, "crypto_pwhash_argon2id_str_needs_rehash"), // G2-042
        (ARGON2_I, 3u64, "crypto_pwhash_argon2i_str_needs_rehash"),   // G2-041
    ] {
        let (rc, buf) = cmp_str_alg(
            "crypto_pwhash_str_alg",
            &format!("nr alg={alg}"),
            &pw,
            8,
            ops,
            8192,
            alg,
            0xC000 + alg as u64,
        );
        assert_eq!(rc, 0);
        // G2-037: same parameters -> 0
        assert_eq!(
            cmp_needs_rehash("crypto_pwhash_str_needs_rehash", "same", &buf, ops, 8192),
            0
        );
        assert_eq!(cmp_needs_rehash(nr, "same/direct", &buf, ops, 8192), 0);
        // G2-038: different t_cost -> 1
        assert_eq!(
            cmp_needs_rehash(
                "crypto_pwhash_str_needs_rehash",
                "other ops",
                &buf,
                ops + 1,
                8192
            ),
            1
        );
        assert_eq!(cmp_needs_rehash(nr, "other ops/direct", &buf, ops + 1, 8192), 1);
        // G2-039: different m_cost -> 1
        assert_eq!(
            cmp_needs_rehash(
                "crypto_pwhash_str_needs_rehash",
                "other mem",
                &buf,
                ops,
                16384
            ),
            1
        );
        // G2-040: memlimit truncation — 8192..9215 all map to m=8 -> 0
        for &mem in &[8192usize, 8192 + 512, 9000, 9215] {
            assert_eq!(
                cmp_needs_rehash(
                    "crypto_pwhash_str_needs_rehash",
                    &format!("trunc mem={mem}"),
                    &buf,
                    ops,
                    mem
                ),
                0,
                "memlimit {mem} must truncate to m=8"
            );
        }
        assert_eq!(
            cmp_needs_rehash(
                "crypto_pwhash_str_needs_rehash",
                "mem=9216",
                &buf,
                ops,
                9216
            ),
            1
        );
    }
}

// ===========================================================================
// G2-045 … G2-064 — the exported low-level argon2 API
// ===========================================================================

/// Run `argon2_hash` on both libraries. `argon2_hash` calls
/// `randombytes_buf(hash, hashlen)` when `hash != NULL`, so both RNG streams
/// are rewound first.
#[allow(clippy::too_many_arguments)]
fn cmp_argon2_hash(
    tag: &str,
    t_cost: u32,
    m_cost: u32,
    par: u32,
    pwd: &[u8],
    pwdlen: usize,
    salt: &[u8],
    saltlen: usize,
    hashlen: usize,
    want_hash: bool,
    encodedlen: usize,
    ty: i32,
    seed: u64,
) -> (i32, Vec<u8>, Vec<u8>) {
    let _g = rng_lock();
    let (c, r) = pair::<Argon2Hash>("_sodium_argon2_hash");
    let mut ha = canary(hashlen.max(1));
    let mut hb = canary(hashlen.max(1));
    let mut ea = canary(encodedlen.max(1));
    let mut eb = canary(encodedlen.max(1));
    let mut rcs = [0i32; 2];
    for (i, f) in [c, r].into_iter().enumerate() {
        let (hp, ep) = if i == 0 {
            (
                if want_hash {
                    ha.as_mut_ptr() as *mut c_void
                } else {
                    std::ptr::null_mut()
                },
                if encodedlen > 0 {
                    ea.as_mut_ptr() as *mut c_char
                } else {
                    std::ptr::null_mut()
                },
            )
        } else {
            (
                if want_hash {
                    hb.as_mut_ptr() as *mut c_void
                } else {
                    std::ptr::null_mut()
                },
                if encodedlen > 0 {
                    eb.as_mut_ptr() as *mut c_char
                } else {
                    std::ptr::null_mut()
                },
            )
        };
        reset_rngs(seed);
        rcs[i] = unsafe {
            f(
                t_cost,
                m_cost,
                par,
                pwd.as_ptr() as *const c_void,
                pwdlen,
                salt.as_ptr() as *const c_void,
                saltlen,
                hp,
                hashlen,
                ep,
                encodedlen,
                ty,
            )
        };
    }
    eq_i32(&format!("argon2_hash rc [{tag}]"), rcs[0], rcs[1]);
    eq_bytes(&format!("argon2_hash hash [{tag}]"), &ha, &hb);
    eq_bytes(&format!("argon2_hash encoded [{tag}]"), &ea, &eb);
    (rcs[0], ha, ea)
}

/// CONFIGS G2-045, G2-046, G2-047, G2-048, G2-049 — the four `hash`/`encoded`
/// output-mode combinations of `argon2_hash`, plus the observation that
/// `randombytes_buf(hash, hashlen)` pre-fills the caller's buffer *before*
/// validation (so an early error leaves random bytes there, unlike the public
/// `crypto_pwhash_*` wrappers which `memset`).
#[test]
fn argon2_hash_output_modes() {
    setup();
    let mut rng = Rng::new(0x1200_0008);
    let pwd = pwbuf(&mut rng, 8);
    let salt8 = rng.bytes(8); // ARGON2_MIN_SALT_LENGTH — not reachable via the
                              // public 16-byte-fixed wrappers
    let salt16 = rng.bytes(16);

    // G2-045: raw-hash-only mode with an 8-byte salt
    let (rc, h, _) = cmp_argon2_hash(
        "raw only", 1, 8, 1, &pwd, 8, &salt8, 8, 16, true, 0, ARGON2_ID, 0xD001,
    );
    assert_eq!(rc, ARGON2_OK);
    assert_ne!(h, canary(16));

    // G2-046: encoded-only mode
    let (rc, _, e) = cmp_argon2_hash(
        "encoded only", 1, 8, 1, &pwd, 8, &salt16, 16, 32, false, 128, ARGON2_ID, 0xD002,
    );
    assert_eq!(rc, ARGON2_OK);
    let s = unsafe { cstr(e.as_ptr() as *const c_char) };
    assert!(s.starts_with(b"$argon2id$v=19$m=8,t=1,p=1$"));

    // G2-047: both outputs
    let (rc, h2, e2) = cmp_argon2_hash(
        "both", 1, 8, 1, &pwd, 8, &salt16, 16, 32, true, 128, ARGON2_ID, 0xD003,
    );
    assert_eq!(rc, ARGON2_OK);
    assert_ne!(h2, canary(32));
    assert_eq!(
        unsafe { cstr(e2.as_ptr() as *const c_char) },
        s,
        "encoded output must not depend on `hash != NULL`"
    );

    // G2-048: neither output — full KDF, result discarded
    let (rc, _, _) = cmp_argon2_hash(
        "neither", 1, 8, 1, &pwd, 8, &salt16, 16, 32, false, 0, ARGON2_ID, 0xD004,
    );
    assert_eq!(rc, ARGON2_OK);

    // G2-049: the random pre-fill survives an early rejection.
    // `saltlen = 4` < ARGON2_MIN_SALT_LENGTH so `argon2_ctx` fails with
    // ARGON2_SALT_TOO_SHORT and `hash` keeps the randombytes_buf() bytes.
    let short_salt = rng.bytes(4);
    let (rc, h3, _) = cmp_argon2_hash(
        "prefill visible",
        1,
        8,
        1,
        &pwd,
        8,
        &short_salt,
        4,
        16,
        true,
        0,
        ARGON2_ID,
        0xD005,
    );
    assert_eq!(rc, -6, "expected ARGON2_SALT_TOO_SHORT");
    assert_ne!(h3, canary(16), "hash buffer must hold the random pre-fill");
    assert_ne!(h3, vec![0u8; 16], "the wrapper must NOT have zeroed it");
}

/// Drive `argon2_ctx` on both libraries with a hand-built `argon2_context`.
#[allow(clippy::too_many_arguments)]
fn cmp_argon2_ctx(
    tag: &str,
    outlen: u32,
    pwd: &[u8],
    salt: &[u8],
    secret: Option<&[u8]>,
    ad: Option<&[u8]>,
    t_cost: u32,
    m_cost: u32,
    lanes: u32,
    threads: u32,
    flags: u32,
    ty: i32,
) -> (i32, Vec<u8>, (u32, u32, Vec<u8>, Vec<u8>)) {
    let (c, r) = pair::<Argon2Ctx>("_sodium_argon2_ctx");
    let mut rcs = [0i32; 2];
    let mut outs: Vec<Vec<u8>> = Vec::new();
    let mut post: Vec<(u32, u32, Vec<u8>, Vec<u8>)> = Vec::new();
    for f in [c, r] {
        let mut out = canary(outlen as usize + 8);
        let mut pwdv = pwd.to_vec();
        pwdv.push(0xEE);
        let mut saltv = salt.to_vec();
        saltv.push(0xEE);
        let mut secv = secret.map(|s| s.to_vec()).unwrap_or_default();
        secv.push(0xEE);
        let mut adv = ad.map(|s| s.to_vec()).unwrap_or_default();
        adv.push(0xEE);
        let mut ctx = Ctx::zeroed();
        ctx.out = out.as_mut_ptr();
        ctx.outlen = outlen;
        ctx.pwd = pwdv.as_mut_ptr();
        ctx.pwdlen = pwd.len() as u32;
        ctx.salt = saltv.as_mut_ptr();
        ctx.saltlen = salt.len() as u32;
        if let Some(s) = secret {
            ctx.secret = secv.as_mut_ptr();
            ctx.secretlen = s.len() as u32;
        }
        if let Some(s) = ad {
            ctx.ad = adv.as_mut_ptr();
            ctx.adlen = s.len() as u32;
        }
        ctx.t_cost = t_cost;
        ctx.m_cost = m_cost;
        ctx.lanes = lanes;
        ctx.threads = threads;
        ctx.flags = flags;
        let rc = unsafe { f(&raw mut ctx, ty) };
        rcs.rotate_left(0);
        let idx = outs.len();
        rcs[idx] = rc;
        outs.push(out);
        post.push((
            ctx.pwdlen,
            ctx.secretlen,
            pwdv[..pwd.len()].to_vec(),
            secv[..secret.map(|s| s.len()).unwrap_or(0)].to_vec(),
        ));
    }
    eq_i32(&format!("argon2_ctx rc [{tag}]"), rcs[0], rcs[1]);
    eq_bytes(&format!("argon2_ctx out [{tag}]"), &outs[0], &outs[1]);
    assert_eq!(post[0].0, post[1].0, "argon2_ctx pwdlen [{tag}]");
    assert_eq!(post[0].1, post[1].1, "argon2_ctx secretlen [{tag}]");
    eq_bytes(&format!("argon2_ctx pwd after [{tag}]"), &post[0].2, &post[1].2);
    eq_bytes(
        &format!("argon2_ctx secret after [{tag}]"),
        &post[0].3,
        &post[1].3,
    );
    assert_eq!(
        &outs[0][outlen as usize..],
        &canary(8)[..],
        "argon2_ctx [{tag}] wrote past outlen"
    );
    let o = outs[0][..outlen as usize].to_vec();
    let p = post.remove(0);
    (rcs[0], o, p)
}

/// CONFIGS G2-050, G2-051, G2-052, G2-053, G2-054, G2-055, G2-056, G2-057,
/// G2-058, G2-059 — hand-built `argon2_context`s: multi-lane (2 and 4 lanes,
/// `m_cost == 8 * lanes` exactly), multi-pass, the keyed (`secret`) and
/// associated-data variants (not reachable through any `crypto_pwhash_*`
/// wrapper), `ARGON2_FLAG_CLEAR_PASSWORD` / `ARGON2_FLAG_CLEAR_SECRET`,
/// `threads != lanes`, and `Argon2_i` vs `Argon2_id` from block 0.
#[test]
fn argon2_ctx_hand_built_contexts() {
    setup();
    let mut rng = Rng::new(0x1200_0009);
    let pwd = rng.bytes(8);
    let salt = rng.bytes(16);

    // G2-050: lanes = 2, m_cost = 16 (== 8 * lanes), t_cost = 1
    let (rc, d2) = {
        let (rc, d, _) = cmp_argon2_ctx(
            "lanes=2 m=16 t=1", 32, &pwd, &salt, None, None, 1, 16, 2, 2, 0, ARGON2_ID,
        );
        (rc, d)
    };
    assert_eq!(rc, ARGON2_OK);
    // G2-051: lanes = 2, t_cost = 2 -> cross-lane index_alpha on pass 1
    let (rc, d3, _) = cmp_argon2_ctx(
        "lanes=2 m=16 t=2", 32, &pwd, &salt, None, None, 2, 16, 2, 2, 0, ARGON2_ID,
    );
    assert_eq!(rc, ARGON2_OK);
    assert_ne!(d2, d3);
    // G2-052: lanes = 4, m_cost = 32
    let (rc, _, _) = cmp_argon2_ctx(
        "lanes=4 m=32 t=1", 32, &pwd, &salt, None, None, 1, 32, 4, 4, 0, ARGON2_ID,
    );
    assert_eq!(rc, ARGON2_OK);
    // G2-053: lanes = 4, m_cost = 4096, t_cost = 3 (multi-address-block +
    // multi-lane + multi-pass together)
    let (rc, _, _) = cmp_argon2_ctx(
        "lanes=4 m=4096 t=3", 32, &pwd, &salt, None, None, 3, 4096, 4, 4, 0, ARGON2_ID,
    );
    assert_eq!(rc, ARGON2_OK);
    // G2-054: type folded into the initial hash AND into the addressing mode
    let (_, di, _) = cmp_argon2_ctx(
        "type=i", 32, &pwd, &salt, None, None, 1, 8, 1, 1, 0, ARGON2_I,
    );
    let (_, did, _) = cmp_argon2_ctx(
        "type=id", 32, &pwd, &salt, None, None, 1, 8, 1, 1, 0, ARGON2_ID,
    );
    assert_ne!(di, did);
    // G2-055: secret = NULL/0 and ad = NULL/0
    let (rc, base, _) = cmp_argon2_ctx(
        "no secret/ad", 32, &pwd, &salt, None, None, 1, 8, 1, 1, 0, ARGON2_ID,
    );
    assert_eq!(rc, ARGON2_OK);
    // …and an explicitly empty (non-NULL, length 0) secret/ad must give the
    // same digest, because only the lengths are absorbed.
    let (rc, same, _) = cmp_argon2_ctx(
        "empty secret/ad",
        32,
        &pwd,
        &salt,
        Some(&[]),
        Some(&[]),
        1,
        8,
        1,
        1,
        0,
        ARGON2_ID,
    );
    assert_eq!(rc, ARGON2_OK);
    eq_bytes("empty secret/ad == NULL secret/ad", &base, &same);
    // G2-056: keyed argon2 (secret != NULL, secretlen = 16)
    let secret = rng.bytes(16);
    let (rc, keyed, _) = cmp_argon2_ctx(
        "secret=16", 32, &pwd, &salt, Some(&secret), None, 1, 8, 1, 1, 0, ARGON2_ID,
    );
    assert_eq!(rc, ARGON2_OK);
    assert_ne!(keyed, base);
    // …with ARGON2_FLAG_CLEAR_SECRET the secret buffer is zeroed and
    // `secretlen` reset to 0
    let (rc, keyed2, post) = cmp_argon2_ctx(
        "secret=16 CLEAR",
        32,
        &pwd,
        &salt,
        Some(&secret),
        None,
        1,
        8,
        1,
        1,
        ARGON2_FLAG_CLEAR_SECRET,
        ARGON2_ID,
    );
    assert_eq!(rc, ARGON2_OK);
    eq_bytes("CLEAR_SECRET must not change the digest", &keyed, &keyed2);
    assert_eq!(post.1, 0, "CLEAR_SECRET must reset secretlen");
    assert_eq!(post.3, vec![0u8; 16], "CLEAR_SECRET must zero the secret");
    // G2-057: associated data
    let ad = rng.bytes(8);
    let (rc, withad, _) = cmp_argon2_ctx(
        "ad=8", 32, &pwd, &salt, None, Some(&ad), 1, 8, 1, 1, 0, ARGON2_ID,
    );
    assert_eq!(rc, ARGON2_OK);
    assert_ne!(withad, base);
    // G2-058: ARGON2_FLAG_CLEAR_PASSWORD
    let (rc, cleared, post) = cmp_argon2_ctx(
        "CLEAR_PASSWORD",
        32,
        &pwd,
        &salt,
        None,
        None,
        1,
        8,
        1,
        1,
        ARGON2_FLAG_CLEAR_PASSWORD,
        ARGON2_ID,
    );
    assert_eq!(rc, ARGON2_OK);
    eq_bytes("CLEAR_PASSWORD must not change the digest", &base, &cleared);
    assert_eq!(post.0, 0, "CLEAR_PASSWORD must reset pwdlen");
    assert_eq!(post.2, vec![0u8; 8], "CLEAR_PASSWORD must zero the password");
    // G2-059: threads != lanes (instance.threads is stored but never used)
    let (rc, a, _) = cmp_argon2_ctx(
        "lanes=4 threads=4", 32, &pwd, &salt, None, None, 1, 32, 4, 4, 0, ARGON2_ID,
    );
    assert_eq!(rc, ARGON2_OK);
    let (rc, b, _) = cmp_argon2_ctx(
        "lanes=4 threads=1", 32, &pwd, &salt, None, None, 1, 32, 4, 1, 0, ARGON2_ID,
    );
    assert_eq!(rc, ARGON2_OK);
    eq_bytes("threads must not affect the digest", &a, &b);

    // G2-073 boundary (also a CONFIGS row): m_cost == 8 * lanes accepted
    for (lanes, m) in [(1u32, 8u32), (2, 16), (4, 32), (8, 64)] {
        let (rc, _, _) = cmp_argon2_ctx(
            &format!("boundary lanes={lanes} m={m}"),
            32,
            &pwd,
            &salt,
            None,
            None,
            1,
            m,
            lanes,
            lanes,
            0,
            ARGON2_ID,
        );
        assert_eq!(rc, ARGON2_OK, "m_cost == 8*lanes must be accepted");
    }
}

/// CONFIGS G2-060, G2-061, G2-062, G2-063, G2-064 —
/// `argon2i_hash_raw` / `argon2id_hash_raw` / `argon2i_hash_encoded` /
/// `argon2id_hash_encoded`, incl. `hashlen` = 16 / 32 / 64 through a 128-byte
/// encoded buffer, and the cross-check that each thin wrapper equals the
/// corresponding `argon2_hash` call.
#[test]
fn argon2_hash_raw_and_encoded_wrappers() {
    setup();
    let mut rng = Rng::new(0x1200_000A);
    let pwd = pwbuf(&mut rng, 8);
    let salt8 = rng.bytes(8);
    let salt16 = rng.bytes(16);

    // G2-060 / G2-061
    for (name, ty, t_cost, salt) in [
        ("_sodium_argon2i_hash_raw", ARGON2_I, 3u32, &salt8),
        ("_sodium_argon2id_hash_raw", ARGON2_ID, 1u32, &salt16),
    ] {
        let saltlen = salt.len();
        for &hashlen in &[16usize, 32, 64] {
            let (c, r) = pair::<Argon2HashRaw>(name);
            let mut a = canary(hashlen + 8);
            let mut b = canary(hashlen + 8);
            let mut rcs = [0i32; 2];
            {
                let _g = rng_lock();
                for (i, (f, buf)) in [(c, &mut a), (r, &mut b)].into_iter().enumerate() {
                    reset_rngs(0xE000 + hashlen as u64);
                    rcs[i] = unsafe {
                        f(
                            t_cost,
                            8,
                            1,
                            pwd.as_ptr() as *const c_void,
                            8,
                            salt.as_ptr() as *const c_void,
                            saltlen,
                            buf.as_mut_ptr() as *mut c_void,
                            hashlen,
                        )
                    };
                }
            }
            eq_i32(&format!("{name} rc h={hashlen}"), rcs[0], rcs[1]);
            eq_bytes(&format!("{name} h={hashlen}"), &a, &b);
            assert_eq!(rcs[0], ARGON2_OK);
            assert_eq!(&a[hashlen..], &canary(8)[..]);
            // must equal argon2_hash(hash=..., encoded=NULL, encodedlen=0)
            let (_, h, _) = cmp_argon2_hash(
                &format!("{name} xcheck h={hashlen}"),
                t_cost,
                8,
                1,
                &pwd,
                8,
                salt,
                saltlen,
                hashlen,
                true,
                0,
                ty,
                0xE000 + hashlen as u64,
            );
            eq_bytes(
                &format!("{name} == argon2_hash h={hashlen}"),
                &a[..hashlen],
                &h[..hashlen],
            );
        }
    }

    // G2-062 / G2-063 / G2-064
    for (name, ty, t_cost, prefix) in [
        (
            "_sodium_argon2i_hash_encoded",
            ARGON2_I,
            3u32,
            &b"$argon2i$v=19$m=8,t=3,p=1$"[..],
        ),
        (
            "_sodium_argon2id_hash_encoded",
            ARGON2_ID,
            1u32,
            &b"$argon2id$v=19$m=8,t=1,p=1$"[..],
        ),
    ] {
        for &hashlen in &[16usize, 32, 64] {
            let (c, r) = pair::<Argon2HashEnc>(name);
            let mut a = canary(128);
            let mut b = canary(128);
            let mut rcs = [0i32; 2];
            {
                let _g = rng_lock();
                for (i, (f, buf)) in [(c, &mut a), (r, &mut b)].into_iter().enumerate() {
                    reset_rngs(0xE100 + hashlen as u64);
                    rcs[i] = unsafe {
                        f(
                            t_cost,
                            8,
                            1,
                            pwd.as_ptr() as *const c_void,
                            8,
                            salt8.as_ptr() as *const c_void,
                            8,
                            hashlen,
                            buf.as_mut_ptr() as *mut c_char,
                            128,
                        )
                    };
                }
            }
            eq_i32(&format!("{name} rc h={hashlen}"), rcs[0], rcs[1]);
            eq_bytes(&format!("{name} h={hashlen}"), &a, &b);
            assert_eq!(rcs[0], ARGON2_OK, "{name} h={hashlen}");
            let s = unsafe { cstr(a.as_ptr() as *const c_char) };
            assert!(
                s.starts_with(prefix),
                "{name}: {:?}",
                String::from_utf8_lossy(&s)
            );
            // base64 hash field grows 22 / 43 / 86 chars with hashlen
            let want_b64 = (hashlen * 4 + 2) / 3;
            let last = s.iter().rposition(|&x| x == b'$').unwrap();
            assert_eq!(
                s.len() - last - 1,
                want_b64,
                "{name} h={hashlen} base64 field width"
            );
            // the encoded string must verify against the same password
            let sn = nul(&s);
            let (cv, rv) = pair::<Argon2Verify>("_sodium_argon2_verify");
            let _g = rng_lock(); // argon2_verify -> argon2_hash consumes randombytes
            let (ra, rb) = unsafe {
                (
                    cv(sn.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, 8, ty),
                    rv(sn.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, 8, ty),
                )
            };
            eq_i32(&format!("{name} verify h={hashlen}"), ra, rb);
            assert_eq!(ra, ARGON2_OK);
        }
    }
}

/// CONFIGS G2-065, G2-066 — `argon2_verify` / `argon2i_verify` /
/// `argon2id_verify` on hand-written encoded strings, including `p=2`
/// (two lanes, `m=16`).
#[test]
fn argon2_verify_hand_written_strings() {
    setup();
    let pwd = b"password\0";

    // G2-065: build the string with the encoder so the salt/hash fields are
    // exactly what the decoder expects, then verify with all three entry
    // points.
    for (ty, tname, vname) in [
        (ARGON2_I, "i", "_sodium_argon2i_verify"),
        (ARGON2_ID, "id", "_sodium_argon2id_verify"),
    ] {
        for (m, t, p) in [(8u32, 3u32, 1u32), (16, 1, 2) /* G2-066 */] {
            let enc = build_encoded(ty, m, t, p, 8, 16, pwd, 8);
            let _g = rng_lock(); // argon2_verify -> argon2_hash consumes randombytes
            let (cv, rv) = pair::<Argon2Verify>("_sodium_argon2_verify");
            let (ra, rb) = unsafe {
                (
                    cv(enc.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, 8, ty),
                    rv(enc.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, 8, ty),
                )
            };
            eq_i32(&format!("argon2_verify({tname}) m={m},t={t},p={p}"), ra, rb);
            assert_eq!(
                ra,
                ARGON2_OK,
                "argon2_verify({tname}) m={m},t={t},p={p}: {:?}",
                String::from_utf8_lossy(&enc)
            );
            let (cv, rv) = pair::<Argon2VerifyT>(vname);
            let (ra, rb) = unsafe {
                (
                    cv(enc.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, 8),
                    rv(enc.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, 8),
                )
            };
            eq_i32(&format!("{vname} m={m},t={t},p={p}"), ra, rb);
            assert_eq!(ra, ARGON2_OK);
            // a wrong password must be rejected identically
            let bad = b"passworD\0";
            let (ra, rb) = unsafe {
                (
                    cv(enc.as_ptr() as *const c_char, bad.as_ptr() as *const c_void, 8),
                    rv(enc.as_ptr() as *const c_char, bad.as_ptr() as *const c_void, 8),
                )
            };
            eq_i32(&format!("{vname} wrong pw"), ra, rb);
            assert_eq!(ra, -35, "expected ARGON2_VERIFY_MISMATCH");
        }
    }
}

/// Produce an encoded argon2 string for the given parameters using the C
/// library's own `argon2_hash` (so the salt/hash fields are self-consistent).
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
    let salt: Vec<u8> = (0..saltlen).map(|i| (i as u8).wrapping_mul(7).wrapping_add(3)).collect();
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
    let s = unsafe { cstr(enc.as_ptr() as *const c_char) };
    nul(&s)
}

/// CONFIGS G2-067 — `argon2_decode_string` on a well-formed string with
/// pre-sized `ctx` buffers: `m_cost`, `t_cost`, `lanes`, `threads == lanes`,
/// `salt`+`saltlen` and `out`+`outlen` must all come back identical.
#[test]
fn argon2_decode_string_success() {
    setup();
    let pwd = b"password\0";
    for (ty, m, t, p) in [
        (ARGON2_ID, 8u32, 1u32, 1u32),
        (ARGON2_I, 8, 3, 1),
        (ARGON2_ID, 4096, 2, 2),
    ] {
        let enc = build_encoded(ty, m, t, p, 16, 32, pwd, 8);
        let (c, r) = pair::<Argon2Decode>("_sodium_argon2_decode_string");
        let mut res = Vec::new();
        for f in [c, r] {
            let n = enc.len();
            let mut saltbuf = canary(n);
            let mut outbuf = canary(n);
            let mut ctx = Ctx::zeroed();
            ctx.salt = saltbuf.as_mut_ptr();
            ctx.saltlen = n as u32;
            ctx.out = outbuf.as_mut_ptr();
            ctx.outlen = n as u32;
            let rc = unsafe { f(&raw mut ctx, enc.as_ptr() as *const c_char, ty) };
            res.push((
                rc,
                ctx.m_cost,
                ctx.t_cost,
                ctx.lanes,
                ctx.threads,
                ctx.saltlen,
                ctx.outlen,
                saltbuf,
                outbuf,
            ));
        }
        let (a, b) = (&res[0], &res[1]);
        eq_i32(&format!("decode_string rc ty={ty} m={m}"), a.0, b.0);
        assert_eq!(a.0, ARGON2_OK, "decode_string must succeed");
        assert_eq!((a.1, a.2, a.3, a.4, a.5, a.6), (b.1, b.2, b.3, b.4, b.5, b.6));
        assert_eq!((a.1, a.2, a.3, a.4, a.5, a.6), (m, t, p, p, 16, 32));
        eq_bytes("decode_string salt", &a.7, &b.7);
        eq_bytes("decode_string out", &a.8, &b.8);
    }
}

/// CONFIGS G2-068, G2-069, G2-070 — `argon2_encode_string`: the standard
/// 128-byte destination, `Argon2_i` vs `Argon2_id` (only the literal prefix
/// differs) and the `u32_to_string` maximum-width path
/// (`m_cost` = `t_cost` = 4294967295, `lanes` = 16777215).
#[test]
fn argon2_encode_string_success() {
    setup();
    let mut rng = Rng::new(0x1200_000B);
    let (c, r) = pair::<Argon2Encode>("_sodium_argon2_encode_string");
    let salt = rng.bytes(16);
    let out = rng.bytes(32);

    let run = |ty: i32, m: u32, t: u32, lanes: u32, dst_len: usize| -> (i32, Vec<u8>) {
        let mut res: Vec<(i32, Vec<u8>)> = Vec::new();
        for f in [c, r] {
            let mut dst = canary(dst_len.max(1));
            let mut s = salt.clone();
            let mut o = out.clone();
            let mut ctx = Ctx::zeroed();
            ctx.out = o.as_mut_ptr();
            ctx.outlen = o.len() as u32;
            ctx.salt = s.as_mut_ptr();
            ctx.saltlen = s.len() as u32;
            ctx.t_cost = t;
            ctx.m_cost = m;
            ctx.lanes = lanes;
            ctx.threads = lanes;
            let rc = unsafe { f(dst.as_mut_ptr() as *mut c_char, dst_len, &raw mut ctx, ty) };
            res.push((rc, dst));
        }
        eq_i32(
            &format!("encode_string rc ty={ty} m={m} t={t} p={lanes} dst={dst_len}"),
            res[0].0,
            res[1].0,
        );
        eq_bytes(
            &format!("encode_string dst ty={ty} m={m} t={t} p={lanes} dst={dst_len}"),
            &res[0].1,
            &res[1].1,
        );
        res.remove(0)
    };

    // G2-068
    let (rc, dst) = run(ARGON2_ID, 8, 1, 1, 128);
    assert_eq!(rc, ARGON2_OK);
    let s = unsafe { cstr(dst.as_ptr() as *const c_char) };
    assert_eq!(&s[..27], b"$argon2id$v=19$m=8,t=1,p=1$");
    assert_eq!(s.len(), 27 + 22 + 1 + 43);
    // G2-069: only the prefix differs
    let (rc, dsti) = run(ARGON2_I, 8, 1, 1, 128);
    assert_eq!(rc, ARGON2_OK);
    let si = unsafe { cstr(dsti.as_ptr() as *const c_char) };
    assert_eq!(&si[..26], b"$argon2i$v=19$m=8,t=1,p=1$");
    assert_eq!(&si[26..], &s[27..]);
    // G2-070: maximum-width u32_to_string
    let (rc, dstx) = run(ARGON2_ID, 4294967295, 4294967295, 16777215, 128);
    assert_eq!(rc, ARGON2_OK);
    let sx = unsafe { cstr(dstx.as_ptr() as *const c_char) };
    assert!(
        sx.starts_with(b"$argon2id$v=19$m=4294967295,t=4294967295,p=16777215$"),
        "{:?}",
        String::from_utf8_lossy(&sx)
    );
}

/// CONFIGS G2-071, G2-072, G2-073 — `argon2_validate_inputs` accepting the
/// exact minimal configuration, `pwd == NULL` with `pwdlen == 0`, and the
/// `m_cost == 8 * lanes` boundary.
#[test]
fn argon2_validate_inputs_accepting_configs() {
    setup();
    let (c, r) = pair::<Argon2Validate>("_sodium_argon2_validate_inputs");
    let mut out = [0u8; 16];
    let mut pwd = [0u8; 8];
    let mut salt = [0u8; 16];

    let mut base = Ctx::zeroed();
    base.out = out.as_mut_ptr();
    base.outlen = 16;
    base.pwd = pwd.as_mut_ptr();
    base.pwdlen = 8;
    base.salt = salt.as_mut_ptr();
    base.saltlen = 16;
    base.lanes = 1;
    base.threads = 1;
    base.m_cost = 8;
    base.t_cost = 1;

    let run = |tag: &str, ctx: &Ctx| -> i32 {
        let (a, b) = unsafe { (c(ctx as *const Ctx), r(ctx as *const Ctx)) };
        eq_i32(&format!("validate_inputs [{tag}]"), a, b);
        a
    };

    // G2-071
    assert_eq!(run("minimal", &base), ARGON2_OK);
    // G2-072: pwd = NULL with pwdlen = 0
    let mut z = base;
    z.pwd = std::ptr::null_mut();
    z.pwdlen = 0;
    assert_eq!(run("pwd=NULL len=0", &z), ARGON2_OK);
    // …the same for salt/secret/ad
    let mut z = base;
    z.secret = std::ptr::null_mut();
    z.secretlen = 0;
    z.ad = std::ptr::null_mut();
    z.adlen = 0;
    assert_eq!(run("secret/ad NULL", &z), ARGON2_OK);
    // G2-073: the m_cost == 8 * lanes boundary
    for (lanes, m) in [(1u32, 8u32), (2, 16), (4, 32), (16777215, 16777215 * 8)] {
        let mut z = base;
        z.lanes = lanes;
        z.threads = lanes;
        z.m_cost = m;
        assert_eq!(
            run(&format!("boundary lanes={lanes}"), &z),
            ARGON2_OK,
            "m_cost == 8*lanes must validate"
        );
    }
    // maximum legal outlen/saltlen/lanes/threads/t_cost values validate too
    let mut z = base;
    z.outlen = 4294967295;
    z.saltlen = 4294967295;
    z.t_cost = 4294967295;
    z.m_cost = 4294967295;
    z.lanes = 16777215;
    z.threads = 16777215;
    assert_eq!(run("all maxima", &z), ARGON2_OK);
}

/// CONFIGS G2-074, G2-075, G2-076 — `blake2b_long`: the `outlen <= 64` short
/// branch, the long branch (with the `while (toproduce > 64)` loop and its
/// 64/65/96/97 boundaries) and the empty-input case.
#[test]
fn blake2b_long_output_lengths() {
    setup();
    let mut rng = Rng::new(0x1200_000C);
    let (c, r) = pair::<Blake2bLong>("_sodium_blake2b_long");
    let outlens: &[usize] = &[
        1, 2, 15, 16, 17, 31, 32, 33, 63, 64, // short branch (G2-074)
        65, 66, 95, 96, 97, 98, 127, 128, 129, 160, 191, 192, 193, 1024, 4096, // long (G2-075)
    ];
    for &inlen in &[0usize /* G2-076 */, 1, 4, 64, 72, 1024, 1025] {
        let inp = rng.bytes(inlen.max(1));
        for &outlen in outlens {
            let mut a = canary(outlen + 8);
            let mut b = canary(outlen + 8);
            let (ra, rb) = unsafe {
                (
                    c(
                        a.as_mut_ptr() as *mut c_void,
                        outlen,
                        inp.as_ptr() as *const c_void,
                        inlen,
                    ),
                    r(
                        b.as_mut_ptr() as *mut c_void,
                        outlen,
                        inp.as_ptr() as *const c_void,
                        inlen,
                    ),
                )
            };
            eq_i32(&format!("blake2b_long rc out={outlen} in={inlen}"), ra, rb);
            eq_bytes(&format!("blake2b_long out={outlen} in={inlen}"), &a, &b);
            assert_eq!(ra, 0);
            assert_eq!(
                &a[outlen..],
                &canary(8)[..],
                "blake2b_long out={outlen} wrote past the end"
            );
        }
    }
}

/// CONFIGS G2-077, G2-078, G2-079, G2-080, G2-081, G2-082 —
/// `argon2_fill_segment_ref` branch coverage. The function takes an
/// `argon2_instance_t` that is only ever built inside `argon2_ctx`, so the
/// branches (`pass == 0` / `slice == 0`, `slice 1..3`, `pass >= 1` slices
/// `0..2` and `3`, and the `curr_offset % lane_length` wrap) are driven
/// through parameter combinations that are guaranteed to hit each of them:
/// `t_cost` 1..3 x `lanes` 1/2/4 x `segment_length` 2 / 3 / 129 / 256.
/// Since every resulting digest matches byte-for-byte, all of the branches
/// agree. `_crypto_pwhash_argon2_pick_best_implementation` (G2-082) is also
/// called directly on both libraries.
#[test]
fn argon2_fill_segment_branch_coverage() {
    setup();
    let mut rng = Rng::new(0x1200_000D);
    // G2-082: the portable build always installs argon2_fill_segment_ref.
    let (c, r) = pair::<FnI32>("_crypto_pwhash_argon2_pick_best_implementation");
    let (a, b) = unsafe { (c(), r()) };
    eq_i32("pick_best_implementation", a, b);
    assert_eq!(a, 0);
    assert!(has_sym("_sodium_argon2_fill_segment_ref"));
    assert!(has_sym("_sodium_argon2_fill_memory_blocks"));

    let pwd = rng.bytes(8);
    let salt = rng.bytes(16);
    // (lanes, m_cost) -> segment_length = m_cost / (lanes * 4)
    for &(lanes, m_cost) in &[
        (1u32, 8u32),   // segment_length 2, lane_length 8  (G2-077, G2-078, G2-081)
        (1, 12),        // segment_length 3
        (1, 516),       // segment_length 129 -> 2 address blocks
        (1, 1024),      // segment_length 256 -> 2 address blocks
        (2, 16),        // multi-lane
        (4, 32),        // 4 lanes
        (4, 1024),      // 4 lanes x segment_length 64
    ] {
        for t_cost in 1u32..=3 {
            // t_cost >= 2 reaches the `pass >= 1` index_alpha branches
            // (slices 0..2 and the slice == SYNC_POINTS-1 special case)
            for ty in [ARGON2_I, ARGON2_ID] {
                let (rc, _, _) = cmp_argon2_ctx(
                    &format!("fill lanes={lanes} m={m_cost} t={t_cost} ty={ty}"),
                    32,
                    &pwd,
                    &salt,
                    None,
                    None,
                    t_cost,
                    m_cost,
                    lanes,
                    lanes,
                    0,
                    ty,
                );
                assert_eq!(rc, ARGON2_OK);
            }
        }
    }
}

// ===========================================================================
// G2-083 … G2-114 — scryptsalsa208sha256
// ===========================================================================

fn cmp_scrypt(
    tag: &str,
    outlen: usize,
    pw: &[u8],
    pwlen: u64,
    salt: &[u8],
    ops: u64,
    mem: usize,
) -> (i32, Vec<u8>) {
    let (c, r) = pair::<Scrypt>("crypto_pwhash_scryptsalsa208sha256");
    let mut a = canary(outlen + 8);
    let mut b = canary(outlen + 8);
    let (ra, rb) = unsafe {
        (
            c(
                a.as_mut_ptr(),
                outlen as u64,
                pw.as_ptr() as *const c_char,
                pwlen,
                salt.as_ptr(),
                ops,
                mem,
            ),
            r(
                b.as_mut_ptr(),
                outlen as u64,
                pw.as_ptr() as *const c_char,
                pwlen,
                salt.as_ptr(),
                ops,
                mem,
            ),
        )
    };
    eq_i32(&format!("scrypt rc [{tag}]"), ra, rb);
    eq_bytes(&format!("scrypt out [{tag}]"), &a, &b);
    assert_eq!(&a[outlen..], &canary(8)[..], "scrypt [{tag}] overran");
    (ra, a[..outlen].to_vec())
}

/// CONFIGS G2-083, G2-084, G2-085, G2-086, G2-087, G2-089, G2-090, G2-092 —
/// every `pickparams` branch of `crypto_pwhash_scryptsalsa208sha256`.
///
/// G2-088 (`OPSLIMIT_SENSITIVE` / `MEMLIMIT_SENSITIVE`, `N = 2^20`, ~1 GiB of
/// `V`, several seconds per library) is **downscaled**: only the constants are
/// compared (see `pwhash_constant_accessors_match`) and the same `pickparams`
/// branch B is covered here at `N = 16384`.
#[test]
fn scrypt_pickparams_branches() {
    setup();
    let mut rng = Rng::new(0x1200_000E);
    let pw = pwbuf(&mut rng, 8);
    let zsalt = vec![0u8; 32];
    let rsalt = rng.bytes(32);

    // G2-083 / G2-092: branch A, N = 1024 (~10 ms). Cheap enough to repeat.
    for &outlen in &[16usize, 32, 64, 17, 100] {
        let (rc, d0) = cmp_scrypt(
            &format!("MIN/MIN out={outlen} zero salt"),
            outlen,
            &pw,
            8,
            &zsalt,
            32768,
            16777216,
        );
        assert_eq!(rc, 0);
        let (rc, d1) = cmp_scrypt(
            &format!("MIN/MIN out={outlen} rand salt"),
            outlen,
            &pw,
            8,
            &rsalt,
            32768,
            16777216,
        );
        assert_eq!(rc, 0);
        assert_ne!(d0, d1, "salt content must matter");
    }

    // G2-089: opslimit below OPSLIMIT_MIN is silently clamped to 32768, so
    // 0 / 1 / 32767 / 32768 must all give the identical digest.
    let base = cmp_scrypt("clamp base", 32, &pw, 8, &rsalt, 32768, 16777216).1;
    for &ops in &[0u64, 1, 32767] {
        let (rc, d) = cmp_scrypt(
            &format!("clamp ops={ops}"),
            32,
            &pw,
            8,
            &rsalt,
            ops,
            16777216,
        );
        assert_eq!(rc, 0, "opslimit {ops} must NOT be rejected");
        eq_bytes(&format!("opslimit {ops} clamps to 32768"), &base, &d);
    }

    // G2-090: memlimit above MEMLIMIT_MAX is not rejected either (branch A,
    // opslimit 32768 < memlimit/32, so N = 1024 -> cheap).
    let (rc, _) = cmp_scrypt(
        "memlimit > MAX",
        32,
        &pw,
        8,
        &rsalt,
        32768,
        68719476737,
    );
    assert_eq!(rc, 0, "memlimit above MEMLIMIT_MAX must NOT be rejected");

    // G2-085: memlimit = 0 -> branch B with maxN = 0 -> N = 2, p = 512.
    let (rc, d85) = cmp_scrypt("memlimit=0", 64, &pw, 8, &rsalt, 32768, 0);
    assert_eq!(rc, 0);
    // G2-086: memlimit = 1024 -> same (N = 2, p = 512)
    let (rc, d86) = cmp_scrypt("memlimit=1024", 64, &pw, 8, &rsalt, 32768, 1024);
    assert_eq!(rc, 0);
    eq_bytes("memlimit 0 and 1024 both give (N=2,r=8,p=512)", &d85, &d86);

    // branch B with p = 1 at the same cost as branch A: memlimit exactly
    // 1048576 makes `opslimit < memlimit/32` false while still yielding
    // (N = 1024, r = 8, p = 1).
    let (rc, d_b) = cmp_scrypt("branch B N=1024 p=1", 32, &pw, 8, &rsalt, 32768, 1048576);
    assert_eq!(rc, 0);
    eq_bytes("branch A and branch B agree at (1024,8,1)", &base, &d_b);

    // G2-084: INTERACTIVE preset, N = 16384 (run once).
    let (rc, _) = cmp_scrypt("INTERACTIVE", 32, &pw, 8, &rsalt, 524288, 16777216);
    assert_eq!(rc, 0);
    // G2-087: opslimit 1048576 / memlimit 16 MiB -> N = 16384, p = 2 (run once).
    let (rc, _) = cmp_scrypt("N=16384 p=2", 32, &pw, 8, &rsalt, 1048576, 16777216);
    assert_eq!(rc, 0);
}

/// CONFIGS G2-091 — scrypt password shapes: empty, 1 byte, 100 bytes and
/// passwords containing embedded NULs (the password is an HMAC-SHA256 *key*,
/// consumed by explicit length).
///
/// Note the HMAC key-padding property: any key shorter than the 64-byte block
/// is zero-padded, so `""` and `"\0\0\0\0"` are the *same* key and must give
/// the identical digest. That equality is itself checked below (it is a real
/// C behaviour the Rust must reproduce), so those two are excluded from the
/// pairwise-distinctness sweep.
#[test]
fn scrypt_password_shapes() {
    setup();
    let mut rng = Rng::new(0x1200_000F);
    let salt = rng.bytes(32);

    let run = |tag: &str, p: &[u8]| -> Vec<u8> {
        let mut pw = p.to_vec();
        pw.push(0xEE);
        let (rc, d) = cmp_scrypt(tag, 32, &pw, p.len() as u64, &salt, 32768, 16777216);
        assert_eq!(rc, 0);
        d
    };

    let p1 = rng.bytes(1);
    let p100 = rng.bytes(100);
    // distinct HMAC keys once zero-padded to 64 bytes
    let distinct: Vec<(&str, Vec<u8>)> = vec![
        ("empty", vec![]),
        ("1 byte", p1.clone()),
        ("100 bytes", p100.clone()),
        ("a\\0b\\0c\\0d\\0", b"a\0b\0c\0d\0".to_vec()),
        ("abcd", b"abcd".to_vec()),
        ("\\0x", b"\0x".to_vec()),
        ("x\\0y", b"x\0y".to_vec()),
        ("65 bytes", rng.bytes(65)),
    ];
    let mut digests = Vec::new();
    for (tag, p) in &distinct {
        digests.push(run(tag, p));
    }
    for i in 0..digests.len() {
        for j in (i + 1)..digests.len() {
            assert_ne!(
                digests[i], digests[j],
                "scrypt password `{}`/`{}` collided",
                distinct[i].0, distinct[j].0
            );
        }
    }
    // NULs are data, not terminators
    assert_ne!(run("a\\0b\\0c\\0d\\0 again", b"a\0b\0c\0d\0"), run("a", b"a"));
    // HMAC zero-padding equivalence must be reproduced exactly
    let z = run("\\0\\0\\0\\0", b"\0\0\0\0");
    eq_bytes("HMAC key zero-padding: \"\" == \"\\0\\0\\0\\0\"", &digests[0], &z);
    let z2 = run("x\\0", b"x\0");
    let x = run("x", b"x");
    eq_bytes("HMAC key zero-padding: \"x\" == \"x\\0\"", &x, &z2);
}

/// CONFIGS G2-093, G2-094, G2-095, G2-096, G2-097, G2-098, G2-099, G2-100,
/// G2-101 — `crypto_pwhash_scryptsalsa208sha256_ll` shapes.
///
/// G2-102 (reusing one `escrypt_local_t` across calls) is **not constructible**
/// through `_ll`, which initialises and frees a fresh local per call; the
/// `local->size < need` fast path it guards is exercised instead by
/// `escrypt_r_direct_calls`, which reuses one local for several settings.
#[test]
fn scrypt_ll_shapes() {
    setup();
    let mut rng = Rng::new(0x1200_0010);
    let (c, r) = pair::<ScryptLl>("crypto_pwhash_scryptsalsa208sha256_ll");

    let run = |tag: &str, pw: &[u8], pwlen: usize, salt: &[u8], saltlen: usize, n: u64, rr: u32, p: u32, buflen: usize| -> (i32, Vec<u8>) {
        let mut a = canary(buflen + 8);
        let mut b = canary(buflen + 8);
        let (ra, rb) = unsafe {
            (
                c(pw.as_ptr(), pwlen, salt.as_ptr(), saltlen, n, rr, p, a.as_mut_ptr(), buflen),
                r(pw.as_ptr(), pwlen, salt.as_ptr(), saltlen, n, rr, p, b.as_mut_ptr(), buflen),
            )
        };
        eq_i32(&format!("_ll rc [{tag}]"), ra, rb);
        eq_bytes(&format!("_ll buf [{tag}]"), &a, &b);
        assert_eq!(&a[buflen..], &canary(8)[..], "_ll [{tag}] overran buf");
        (ra, a[..buflen].to_vec())
    };

    let pw = pwbuf(&mut rng, 8);
    let salt = rng.bytes(8);

    // G2-093 (N=2,r=1,p=1), G2-094 (N=16,r=1), G2-095 (N=16,r=8),
    // G2-096 (p=4), G2-097 (N=1024,r=4,p=2)
    for &(n, rr, p, buflen) in &[
        (2u64, 1u32, 1u32, 32usize),
        (16, 1, 1, 32),
        (16, 8, 1, 64),
        (16, 1, 4, 32),
        (1024, 4, 2, 32),
        (4, 2, 3, 32),
        (8, 3, 1, 32),
        (2, 8, 8, 32),
    ] {
        let (rc, _) = run(
            &format!("N={n} r={rr} p={p} buflen={buflen}"),
            &pw,
            8,
            &salt,
            8,
            n,
            rr,
            p,
            buflen,
        );
        assert_eq!(rc, 0, "N={n} r={rr} p={p}");
    }

    // G2-098: N = 2 vs 1024 give different results; N = 2^31 is *accepted* by
    // the parameter checks (<= UINT32_MAX, a power of two) but its 256 GiB
    // `V` region cannot be allocated, so both libraries return -1.
    let (_, d2) = run("N=2", &pw, 8, &salt, 8, 2, 1, 1, 32);
    let (_, d1024) = run("N=1024", &pw, 8, &salt, 8, 1024, 1, 1, 32);
    assert_ne!(d2, d1024);
    let (rc, _) = run("N=2^31", &pw, 8, &salt, 8, 2147483648, 1, 1, 32);
    assert_eq!(rc, -1, "N = 2^31 needs 256 GiB of V; allocation must fail");

    // G2-099: buflen = 0 -> PBKDF2 writes nothing, returns 0
    let mut a = canary(8);
    let mut b = canary(8);
    let (ra, rb) = unsafe {
        (
            c(pw.as_ptr(), 8, salt.as_ptr(), 8, 16, 1, 1, a.as_mut_ptr(), 0),
            r(pw.as_ptr(), 8, salt.as_ptr(), 8, 16, 1, 1, b.as_mut_ptr(), 0),
        )
    };
    eq_i32("_ll buflen=0 rc", ra, rb);
    assert_eq!(ra, 0);
    eq_bytes("_ll buflen=0 buf", &a, &b);
    assert_eq!(a, canary(8), "buflen = 0 must not write anything");

    // G2-100: the PBKDF2 tail block
    for &buflen in &[1usize, 2, 31, 32, 33, 63, 64, 65, 96, 97, 200] {
        let (rc, _) = run(
            &format!("buflen={buflen}"),
            &pw,
            8,
            &salt,
            8,
            16,
            1,
            1,
            buflen,
        );
        assert_eq!(rc, 0);
    }

    // G2-101: empty password and empty salt
    let e = pwbuf(&mut rng, 0);
    let (rc, _) = run("empty pw+salt", &e, 0, &e, 0, 2, 1, 1, 32);
    assert_eq!(rc, 0);
    for (pl, sl) in [(0usize, 8usize), (8, 0), (0, 0)] {
        let p = pwbuf(&mut rng, pl);
        let s = pwbuf(&mut rng, sl);
        let (rc, _) = run(&format!("pw={pl} salt={sl}"), &p, pl, &s, sl, 2, 1, 1, 32);
        assert_eq!(rc, 0);
    }

    // randomised sweep at tiny cost
    for i in 0..30 {
        let n = 1u64 << rng.range(1, 8);
        let rr = rng.range(1, 8) as u32;
        let p = rng.range(1, 4) as u32;
        let buflen = rng.range(16, 96);
        let pl = rng.below(40);
        let sl = rng.below(40);
        let pv = pwbuf(&mut rng, pl);
        let sv = pwbuf(&mut rng, sl);
        let (rc, _) = run(
            &format!("rand#{i} N={n} r={rr} p={p}"),
            &pv,
            pl,
            &sv,
            sl,
            n,
            rr,
            p,
            buflen,
        );
        assert_eq!(rc, 0);
    }
}

/// CONFIGS G2-103, G2-104, G2-105, G2-106, G2-107, G2-108 — the scrypt
/// encoded-string API: `_str` (exactly 101 chars + NUL in a 102-byte
/// `STRBYTES` buffer), `_str_verify` round trips and `_str_needs_rehash`.
#[test]
fn scrypt_str_verify_and_needs_rehash() {
    setup();
    let mut rng = Rng::new(0x1200_0011);
    let name = "crypto_pwhash_scryptsalsa208sha256_str";

    // G2-103: default MIN/MIN parameters -> "$7$" + 1 + 5 + 5 + 43 salt chars
    for &pwlen in &[0usize, 1, 8, 100] {
        let pw = pwbuf(&mut rng, pwlen);
        let (rc, buf) = cmp_str(
            name,
            &format!("pw={pwlen}"),
            &pw,
            pwlen as u64,
            32768,
            16777216,
            102,
            0xF000 + pwlen as u64,
        );
        assert_eq!(rc, 0);
        let s = unsafe { cstr(buf.as_ptr() as *const c_char) };
        assert_eq!(s.len(), 101, "{:?}", String::from_utf8_lossy(&s));
        assert_eq!(&s[..3], b"$7$");
        assert_eq!(buf[101], 0, "must be NUL-terminated at index 101");
        // G2-105 / G2-106: round trip
        assert_eq!(
            cmp_verify(
                "crypto_pwhash_scryptsalsa208sha256_str_verify",
                &format!("rt pw={pwlen}"),
                &buf,
                &pw,
                pwlen as u64,
                0xF100
            ),
            0
        );
        // wrong password (the empty password has no byte to flip)
        let (wrong, wlen) = if pwlen == 0 {
            (b"x\0".to_vec(), 1usize)
        } else {
            let mut w = pw.clone();
            w[0] ^= 1;
            (w, pwlen)
        };
        assert_eq!(
            cmp_verify(
                "crypto_pwhash_scryptsalsa208sha256_str_verify",
                &format!("wrong pw={pwlen}"),
                &buf,
                &wrong,
                wlen as u64,
                0xF101
            ),
            -1
        );
        // One-byte mutation of every position. Layout of the 101-char string:
        //   0..3   "$7$"
        //   3      N_log2 (one itoa64 char)
        //   4..9   r  (5 itoa64 chars, little-endian 6-bit groups, 30 bits)
        //   9..14  p  (5 itoa64 chars, ditto)
        //   14..57 43 salt chars, 57 '$', 58..101 43 hash chars
        // Flipping bit 0 of the HIGH-order `r`/`p` characters (indices 5..9 and
        // 10..14) multiplies `r` or `p` by up to 2^24. For the *middle*
        // characters that yields e.g. `p = 4097`, i.e. 4000x the KDF work
        // (~40 s per library), so those four+four positions are skipped here.
        // `t13_pwhash_errors::scrypt_str_verify_rejections` instead drives the
        // r/p fields with hand-built values that are rejected *before* any
        // hashing (`r = 0`, `p = 0`, `r * p >= 2^30`, non-itoa64 bytes), which
        // covers the same code with no cost.
        let skip: &[usize] = &[5, 6, 7, 8, 10, 11, 12, 13];
        for pos in 0..101 {
            if skip.contains(&pos) {
                continue;
            }
            let mut m = buf.clone();
            m[pos] ^= 0x01;
            let rc = cmp_verify(
                "crypto_pwhash_scryptsalsa208sha256_str_verify",
                &format!("mutate pos={pos}"),
                &m,
                &pw,
                pwlen as u64,
                0xF200 + pos as u64,
            );
            assert_eq!(rc, -1, "mutating byte {pos} still verified");
        }
        // G2-107 / G2-108: needs_rehash
        let nr = "crypto_pwhash_scryptsalsa208sha256_str_needs_rehash";
        assert_eq!(cmp_needs_rehash(nr, "same", &buf, 32768, 16777216), 0);
        assert_eq!(cmp_needs_rehash(nr, "other ops", &buf, 524288, 16777216), 1);
        assert_eq!(cmp_needs_rehash(nr, "other mem", &buf, 32768, 1024), 1);
        break; // one password is enough for the mutation sweep
    }

    // G2-106: NUL-containing password
    let p = b"a\0b\0c\0d\0";
    let mut pw = p.to_vec();
    pw.push(0xEE);
    let (rc, buf) = cmp_str(name, "nul pw", &pw, 8, 32768, 16777216, 102, 0xF300);
    assert_eq!(rc, 0);
    assert_eq!(
        cmp_verify(
            "crypto_pwhash_scryptsalsa208sha256_str_verify",
            "nul pw rt",
            &buf,
            &pw,
            8,
            0xF301
        ),
        0
    );

    // G2-104: INTERACTIVE preset (N = 16384), run once
    let pw0 = pwbuf(&mut rng, 0);
    let (rc, buf) = cmp_str(name, "INTERACTIVE", &pw0, 0, 524288, 16777216, 102, 0xF400);
    assert_eq!(rc, 0);
    assert_eq!(
        cmp_verify(
            "crypto_pwhash_scryptsalsa208sha256_str_verify",
            "INTERACTIVE rt",
            &buf,
            &pw0,
            0,
            0xF401
        ),
        0
    );
    let nr = "crypto_pwhash_scryptsalsa208sha256_str_needs_rehash";
    assert_eq!(cmp_needs_rehash(nr, "INTERACTIVE same", &buf, 524288, 16777216), 0);
    assert_eq!(cmp_needs_rehash(nr, "INTERACTIVE other", &buf, 32768, 16777216), 1);
}

/// CONFIGS G2-109, G2-110, G2-111 — `escrypt_gensalt_r` over the whole
/// `N_log2` = 0..63 range (each rendering as one `itoa64` character) and at
/// the `r * p` acceptance boundary, plus `escrypt_parse_setting` round-tripping
/// the 14-byte prefix it produced.
#[test]
fn escrypt_gensalt_and_parse_setting() {
    setup();
    let mut rng = Rng::new(0x1200_0012);
    let (cg, rg) = pair::<EGensalt>("_sodium_escrypt_gensalt_r");
    let (cp, rp) = pair::<EParse>("_sodium_escrypt_parse_setting");
    let src = rng.bytes(32);

    // G2-109: N_log2 = 0..63
    for n_log2 in 0u32..=63 {
        let mut a = canary(58);
        let mut b = canary(58);
        let (pa, pb) = unsafe {
            (
                cg(n_log2, 8, 1, src.as_ptr(), 32, a.as_mut_ptr(), 58),
                rg(n_log2, 8, 1, src.as_ptr(), 32, b.as_mut_ptr(), 58),
            )
        };
        assert_eq!(
            pa.is_null(),
            pb.is_null(),
            "gensalt_r(N_log2={n_log2}) null-ness"
        );
        eq_bytes(&format!("gensalt_r(N_log2={n_log2})"), &a, &b);
        assert!(!pa.is_null(), "N_log2 = {n_log2} must be accepted");
        let s = unsafe { cstr(a.as_ptr() as *const c_char) };
        assert_eq!(s.len(), 57, "{:?}", String::from_utf8_lossy(&s));
        assert_eq!(&s[..3], b"$7$");
        const ITOA64: &[u8] = b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        assert_eq!(s[3], ITOA64[n_log2 as usize]);
        // G2-111: parse_setting must recover (N_log2, r, p) and return
        // setting + 14
        let sn = nul(&s);
        for (which, f) in [("C", cp), ("R", rp)] {
            let (mut n, mut rr, mut pp) = (0u32, 0u32, 0u32);
            let ret = unsafe { f(sn.as_ptr(), &raw mut n, &raw mut rr, &raw mut pp) };
            assert!(!ret.is_null(), "{which} parse_setting");
            assert_eq!(
                unsafe { ret.offset_from(sn.as_ptr()) },
                14,
                "{which} parse_setting must return setting + 14"
            );
            assert_eq!((n, rr, pp), (n_log2, 8, 1), "{which} parse_setting values");
        }
    }

    // G2-110: r * p just below 2^30
    for &(rr, pp) in &[(8u32, 134217727u32), (1, 1073741823), (2, 536870911), (32768, 32767)] {
        let mut a = canary(58);
        let mut b = canary(58);
        let (pa, pb) = unsafe {
            (
                cg(14, rr, pp, src.as_ptr(), 32, a.as_mut_ptr(), 58),
                rg(14, rr, pp, src.as_ptr(), 32, b.as_mut_ptr(), 58),
            )
        };
        assert_eq!(pa.is_null(), pb.is_null(), "gensalt_r(r={rr},p={pp})");
        eq_bytes(&format!("gensalt_r(r={rr},p={pp})"), &a, &b);
        assert!(
            !pa.is_null(),
            "r*p = {} < 2^30 must be accepted",
            rr as u64 * pp as u64
        );
        let sn = nul(&unsafe { cstr(a.as_ptr() as *const c_char) });
        for (which, f) in [("C", cp), ("R", rp)] {
            let (mut n, mut r2, mut p2) = (0u32, 0u32, 0u32);
            let ret = unsafe { f(sn.as_ptr(), &raw mut n, &raw mut r2, &raw mut p2) };
            assert!(!ret.is_null(), "{which}");
            assert_eq!((n, r2, p2), (14, rr, pp), "{which} round trip");
        }
    }

    // randomised srclen / buflen sweep (all still valid)
    for i in 0..40 {
        let srclen = rng.below(40);
        let s = rng.bytes(srclen.max(1));
        let saltlen = (srclen * 8 + 5) / 6;
        let need = 14 + saltlen + 1;
        let buflen = need + rng.below(8);
        let mut a = canary(buflen);
        let mut b = canary(buflen);
        let nl = rng.below(64) as u32;
        let (pa, pb) = unsafe {
            (
                cg(nl, 8, 1, s.as_ptr(), srclen, a.as_mut_ptr(), buflen),
                rg(nl, 8, 1, s.as_ptr(), srclen, b.as_mut_ptr(), buflen),
            )
        };
        assert_eq!(pa.is_null(), pb.is_null(), "gensalt rand#{i}");
        eq_bytes(&format!("gensalt rand#{i} srclen={srclen} buflen={buflen}"), &a, &b);
    }
}

/// CONFIGS G2-112 — `escrypt_r` with a full 101-char `"$7$…"` string whose
/// salt field is terminated by `'$'` (so `strrchr` picks `saltlen = 43`) vs a
/// bare 57-char setting with no trailing `'$'` (so `saltlen = strlen`).
/// One `escrypt_local_t` is reused across the calls, which also covers the
/// `local->size < need` reuse fast path (CONFIGS G2-102).
#[test]
fn escrypt_r_direct_calls() {
    setup();
    let mut rng = Rng::new(0x1200_0013);
    let (ce, re) = pair::<ER>("_sodium_escrypt_r");
    let (cg, _) = pair::<EGensalt>("_sodium_escrypt_gensalt_r");
    let (cinit, rinit) = pair::<ERegion1>("_sodium_escrypt_init_local");
    let (cfree, rfree) = pair::<ERegion1>("_sodium_escrypt_free_local");
    let pw = pwbuf(&mut rng, 8);
    let src = rng.bytes(32);

    // build a 57-char setting with the C library
    let mut setting = vec![0u8; 64];
    let p = unsafe { cg(10, 8, 1, src.as_ptr(), 32, setting.as_mut_ptr(), 58) };
    assert!(!p.is_null());
    let bare = nul(&unsafe { cstr(setting.as_ptr() as *const c_char) });
    assert_eq!(bare.len(), 58);

    let mut locals = [Region::zeroed(), Region::zeroed()];
    unsafe {
        assert_eq!(cinit(&raw mut locals[0]), 0);
        assert_eq!(rinit(&raw mut locals[1]), 0);
    }

    // 1) bare setting, no trailing '$' -> saltlen = strlen(salt) = 43
    let mut outs = [canary(102), canary(102)];
    let mut nulls = [false; 2];
    let g = rng_lock(); // escrypt_r randombytes_buf()es its output buffer
    for (i, f) in [ce, re].into_iter().enumerate() {
        reset_rngs(0x1234_0001);
        let ret = unsafe {
            f(
                &raw mut locals[i],
                pw.as_ptr(),
                8,
                bare.as_ptr(),
                outs[i].as_mut_ptr(),
                102,
            )
        };
        nulls[i] = ret.is_null();
    }
    assert_eq!(nulls[0], nulls[1], "escrypt_r(bare setting) null-ness");
    assert!(!nulls[0]);
    let (a, b) = (outs[0].clone(), outs[1].clone());
    eq_bytes("escrypt_r(bare setting)", &a, &b);
    let full = unsafe { cstr(a.as_ptr() as *const c_char) };
    assert_eq!(full.len(), 101);

    // 2) the resulting 101-char string, whose salt field IS '$'-terminated
    let fulln = nul(&full);
    let mut outs2 = [canary(102), canary(102)];
    for (i, f) in [ce, re].into_iter().enumerate() {
        reset_rngs(0x1234_0002);
        let ret = unsafe {
            f(
                &raw mut locals[i],
                pw.as_ptr(),
                8,
                fulln.as_ptr(),
                outs2[i].as_mut_ptr(),
                102,
            )
        };
        assert!(!ret.is_null(), "escrypt_r(full string) #{i}");
    }
    let (a2, b2) = (outs2[0].clone(), outs2[1].clone());
    eq_bytes("escrypt_r(full string)", &a2, &b2);
    eq_bytes("escrypt_r bare and '$'-terminated agree", &a, &a2);

    // several more settings through the SAME local (local->size reuse)
    for (i, n_log2) in [1u32, 4, 8, 10, 6, 2].into_iter().enumerate() {
        let mut st = vec![0u8; 64];
        let p = unsafe { cg(n_log2, 8, 1, src.as_ptr(), 32, st.as_mut_ptr(), 58) };
        assert!(!p.is_null());
        let stn = nul(&unsafe { cstr(st.as_ptr() as *const c_char) });
        let mut o = [canary(102), canary(102)];
        for (k, f) in [ce, re].into_iter().enumerate() {
            reset_rngs(0x1234_0100 + i as u64);
            let ret = unsafe {
                f(
                    &raw mut locals[k],
                    pw.as_ptr(),
                    8,
                    stn.as_ptr(),
                    o[k].as_mut_ptr(),
                    102,
                )
            };
            assert!(!ret.is_null(), "escrypt_r reuse N_log2={n_log2} #{k}");
        }
        let (x, y) = (o[0].clone(), o[1].clone());
        eq_bytes(&format!("escrypt_r reuse N_log2={n_log2}"), &x, &y);
    }

    drop(g);
    unsafe {
        assert_eq!(cfree(&raw mut locals[0]), 0);
        assert_eq!(rfree(&raw mut locals[1]), 0);
    }
    assert!(locals[0].base.is_null() && locals[1].base.is_null());
    assert_eq!(locals[0].size, 0);
    assert_eq!(locals[1].size, 0);
}

/// CONFIGS G2-113 — `escrypt_PBKDF2_SHA256` with `c = 1` (the only value used
/// anywhere in this subtree), across the `dkLen` block-count boundaries and
/// random key/salt lengths.
#[test]
fn escrypt_pbkdf2_sha256_direct() {
    setup();
    let mut rng = Rng::new(0x1200_0014);
    let (c, r) = pair::<EPbkdf2>("_sodium_escrypt_PBKDF2_SHA256");
    for &dklen in &[0usize, 1, 31, 32, 33, 63, 64, 65, 96, 128, 1000] {
        for &(pl, sl) in &[(0usize, 0usize), (1, 1), (8, 32), (64, 64), (65, 100), (200, 3)] {
            let pw = pwbuf(&mut rng, pl);
            let salt = pwbuf(&mut rng, sl);
            let mut a = canary(dklen + 8);
            let mut b = canary(dklen + 8);
            unsafe {
                c(pw.as_ptr(), pl, salt.as_ptr(), sl, 1, a.as_mut_ptr(), dklen);
                r(pw.as_ptr(), pl, salt.as_ptr(), sl, 1, b.as_mut_ptr(), dklen);
            }
            eq_bytes(
                &format!("PBKDF2_SHA256(dkLen={dklen}, pw={pl}, salt={sl})"),
                &a,
                &b,
            );
            assert_eq!(&a[dklen..], &canary(8)[..], "PBKDF2 overran dkLen");
        }
    }
    // c > 1 is dead code in libsodium, but the parameter is still part of the
    // exported signature — compare it anyway.
    let pw = pwbuf(&mut rng, 8);
    let salt = pwbuf(&mut rng, 16);
    for &cnt in &[1u64, 2, 3, 10] {
        let mut a = canary(64);
        let mut b = canary(64);
        unsafe {
            c(pw.as_ptr(), 8, salt.as_ptr(), 16, cnt, a.as_mut_ptr(), 64);
            r(pw.as_ptr(), 8, salt.as_ptr(), 16, cnt, b.as_mut_ptr(), 64);
        }
        eq_bytes(&format!("PBKDF2_SHA256(c={cnt})"), &a, &b);
    }
}

/// CONFIGS G2-114 — `escrypt_alloc_region` / `escrypt_free_region` /
/// `escrypt_init_local` / `escrypt_free_local` on the portable branch
/// (`malloc(size + 63)` then align up to 64).
#[test]
fn escrypt_region_and_local() {
    setup();
    let (ca, ra) = pair::<EAlloc>("_sodium_escrypt_alloc_region");
    let (cf, rf) = pair::<ERegion1>("_sodium_escrypt_free_region");
    let (ci, ri) = pair::<ERegion1>("_sodium_escrypt_init_local");
    let (cl, rl) = pair::<ERegion1>("_sodium_escrypt_free_local");

    for &size in &[1usize, 63, 64, 65, 1024, 4096, 100_000, 1 << 20] {
        for (which, alloc, free) in [("C", ca, cf), ("R", ra, rf)] {
            let mut reg = Region {
                base: 0x1 as *mut c_void,
                aligned: 0x2 as *mut c_void,
                size: 0xdead,
            };
            let p = unsafe { alloc(&raw mut reg, size) };
            assert!(!p.is_null(), "{which} alloc_region({size})");
            assert_eq!(p, reg.aligned, "{which}: return value must be `aligned`");
            assert_eq!(reg.size, size, "{which}: region->size");
            assert!(!reg.base.is_null());
            assert_eq!(
                reg.aligned as usize % 64,
                0,
                "{which}: aligned must be 64-byte aligned"
            );
            assert!(
                reg.aligned as usize >= reg.base as usize
                    && (reg.aligned as usize) < reg.base as usize + 64,
                "{which}: aligned must be base rounded up to 64"
            );
            // the whole region must be writable
            unsafe { std::ptr::write_bytes(p as *mut u8, 0x5A, size) };
            assert_eq!(unsafe { free(&raw mut reg) }, 0, "{which} free_region");
            assert!(reg.base.is_null() && reg.aligned.is_null());
            assert_eq!(reg.size, 0);
        }
    }

    // init_local just zeroes the struct and returns 0; free_local on a
    // never-allocated local must also return 0.
    for (which, init, freel) in [("C", ci, cl), ("R", ri, rl)] {
        let mut l = Region {
            base: 0x11 as *mut c_void,
            aligned: 0x22 as *mut c_void,
            size: 0x33,
        };
        assert_eq!(unsafe { init(&raw mut l) }, 0, "{which} init_local");
        assert!(l.base.is_null() && l.aligned.is_null());
        assert_eq!(l.size, 0);
        assert_eq!(unsafe { freel(&raw mut l) }, 0, "{which} free_local");
        assert_eq!(unsafe { freel(&raw mut l) }, 0, "{which} free_local twice");
    }
}

/// CONFIGS G2-050 … G2-059 and G2-077 … G2-081, randomised: 250 random
/// `argon2_context`s over `lanes` 1..8, `m_cost` `8*lanes ..= 512`, `t_cost`
/// 1..=3, both types, and random `pwd`/`salt`/`secret`/`ad` lengths and flag
/// combinations. Each run compares the full digest, so any divergence in
/// `index_alpha`, `generate_addresses`, `fill_block{,_with_xor}` or
/// `argon2_finalize`'s lane XOR shows up immediately.
///
/// Kept cheap on purpose: `m_cost <= 512` blocks x `t_cost <= 3` is ~1500
/// block compressions per run (well under a millisecond).
#[test]
fn argon2_ctx_randomised_parameters() {
    setup();
    let mut rng = Rng::new(0x1200_0100);
    for i in 0..250 {
        let lanes = rng.range(1, 8) as u32;
        // m_cost must be >= 8 * lanes; sweep the interesting neighbourhood of
        // the segment_length boundaries too.
        let kind = rng.below(4);
        let jitter = rng.below(4) as u32;
        let mult = rng.range(2, 40) as u32;
        let wide = rng.range((8 * lanes) as usize, 512) as u32;
        let m_cost = match kind {
            0 => 8 * lanes, // exact minimum
            1 => 8 * lanes + jitter,
            2 => 4 * lanes * mult,
            _ => wide,
        };
        let t_cost = rng.range(1, 3) as u32;
        let ty = if rng.bool() { ARGON2_I } else { ARGON2_ID };
        let outlen = rng.range(16, 80) as u32;
        let pn = rng.below(70);
        let pwd = rng.bytes(pn);
        let sn = rng.range(8, 40);
        let salt = rng.bytes(sn);
        let secret = if rng.bool() {
            let n = rng.below(33);
            Some(rng.bytes(n))
        } else {
            None
        };
        let ad = if rng.bool() {
            let n = rng.below(33);
            Some(rng.bytes(n))
        } else {
            None
        };
        let flags = (rng.below(4) as u32) & 3;
        let threads = if rng.bool() { lanes } else { rng.range(1, 8) as u32 };
        let (rc, _, _) = cmp_argon2_ctx(
            &format!(
                "rand#{i} lanes={lanes} m={m_cost} t={t_cost} ty={ty} out={outlen} \
                 pwd={} salt={} secret={:?} ad={:?} flags={flags} threads={threads}",
                pwd.len(),
                salt.len(),
                secret.as_ref().map(|s| s.len()),
                ad.as_ref().map(|s| s.len()),
            ),
            outlen,
            &pwd,
            &salt,
            secret.as_deref(),
            ad.as_deref(),
            t_cost,
            m_cost,
            lanes,
            threads,
            flags,
            ty,
        );
        assert_eq!(rc, ARGON2_OK, "rand#{i} must succeed");
    }
}

/// CONFIGS G2-001 … G2-024 and G2-043 / G2-044, randomised: 120 random
/// `crypto_pwhash` calls at the cheapest legal `opslimit`, sweeping `memlimit`
/// over the whole 8 KiB … 1 MiB range (so `segment_length` and the number of
/// address blocks per segment vary) and `outlen` across both `blake2b_long`
/// branches, with the two direct entry points cross-checked.
#[test]
fn pwhash_randomised_memlimit_sweep() {
    setup();
    let mut rng = Rng::new(0x1200_0101);
    for i in 0..120 {
        let (alg, ops, direct) = if rng.bool() {
            (ARGON2_I, 3u64, "crypto_pwhash_argon2i")
        } else {
            (ARGON2_ID, 1u64, "crypto_pwhash_argon2id")
        };
        // 8..=1024 KiB; the +/- 1023 jitter exercises the truncating division
        let base_mem = rng.range(8192, 1024 * 1024);
        let mem = base_mem + rng.below(1024);
        let outlen = match rng.below(4) {
            0 => rng.range(16, 64),
            1 => rng.range(65, 200),
            2 => *rng.pick(&[16usize, 17, 63, 64, 65, 96, 97, 128, 129]),
            _ => rng.range(16, 400),
        };
        let pwlen = rng.below(150);
        let pw = pwbuf(&mut rng, pwlen);
        let salt = rng.bytes(16);
        let tag = format!("rand#{i} alg={alg} mem={mem} out={outlen} pw={pwlen}");
        let (rc, d1) = cmp_pw(
            "crypto_pwhash",
            &tag,
            outlen,
            &pw,
            pwlen as u64,
            &salt,
            ops,
            mem,
            alg,
        );
        assert_eq!(rc, 0, "{tag}");
        let (rc, d2) = cmp_pw(direct, &tag, outlen, &pw, pwlen as u64, &salt, ops, mem, alg);
        assert_eq!(rc, 0, "{tag}");
        eq_bytes(&format!("crypto_pwhash == {direct} [{tag}]"), &d1, &d2);
    }
}

/// CONFIGS G2-093 … G2-101, randomised: 120 random `_ll` shapes with `N` a
/// power of two in 2..=4096, `r` 1..=16, `p` 1..=6 and `buflen` 0..=200,
/// including empty passwords and salts. Cheap (`128*r*N` stays under 8 MiB).
#[test]
fn scrypt_ll_randomised() {
    setup();
    let mut rng = Rng::new(0x1200_0102);
    let (c, r) = pair::<ScryptLl>("crypto_pwhash_scryptsalsa208sha256_ll");
    for i in 0..120 {
        let n = 1u64 << rng.range(1, 12);
        let rr = rng.range(1, 16) as u32;
        let p = rng.range(1, 6) as u32;
        // keep 128 * r * N under ~8 MiB
        let n = if 128 * (rr as u64) * n > 8 * 1024 * 1024 {
            1u64 << rng.range(1, 6)
        } else {
            n
        };
        let buflen = rng.below(201);
        let pl = rng.below(120);
        let sl = rng.below(120);
        let pw = pwbuf(&mut rng, pl);
        let salt = pwbuf(&mut rng, sl);
        let mut a = canary(buflen + 8);
        let mut b = canary(buflen + 8);
        let (ra, rb) = unsafe {
            (
                c(pw.as_ptr(), pl, salt.as_ptr(), sl, n, rr, p, a.as_mut_ptr(), buflen),
                r(pw.as_ptr(), pl, salt.as_ptr(), sl, n, rr, p, b.as_mut_ptr(), buflen),
            )
        };
        let tag = format!("rand#{i} N={n} r={rr} p={p} buflen={buflen} pw={pl} salt={sl}");
        eq_i32(&format!("_ll rc [{tag}]"), ra, rb);
        eq_bytes(&format!("_ll buf [{tag}]"), &a, &b);
        assert_eq!(ra, 0, "{tag}");
        assert_eq!(&a[buflen..], &canary(8)[..], "{tag} overran buf");
    }
}
