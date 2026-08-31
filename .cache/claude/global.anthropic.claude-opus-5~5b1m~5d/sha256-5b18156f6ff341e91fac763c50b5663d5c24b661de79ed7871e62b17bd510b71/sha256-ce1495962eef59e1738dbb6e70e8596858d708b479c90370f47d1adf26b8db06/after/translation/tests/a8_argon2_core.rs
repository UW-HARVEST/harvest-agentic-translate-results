//! Area 8 (part 1b) — `crypto_pwhash/argon2/{argon2.c, argon2-core.c,
//! argon2-fill-block-ref.c, blake2b-long.c}`: the low-level entry points that
//! the public `crypto_pwhash*` API cannot reach (lanes, threads, secret, ad,
//! flags, arbitrary salt/out lengths).
//!
//! Covers `configs_8.md` rows 8.37 – 8.75 and 8.87 – 8.90, and `errors_8.md`
//! rows 8.53 – 8.111.
//!
//! **Speed:** `m_cost` never exceeds 1024 KiB and `t_cost` never exceeds 3, so
//! the whole file runs in well under a second.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr::{null_mut, NonNull};
use std::sync::{Mutex, MutexGuard};

static RNG_LOCK: Mutex<()> = Mutex::new(());

fn rng_guard() -> MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ------------------------------------------------------------------- types

/// `argon2_context` from `argon2.h`, field for field.
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

/// `argon2_instance_t` from `argon2-core.h`, field for field.
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
    ty: c_int,
    print_internals: c_int,
}

/// `argon2_position_t` from `argon2-core.h`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Position {
    pass: u32,
    lane: u32,
    slice: u8,
    index: u32,
}

type Argon2Ctx = unsafe extern "C" fn(*mut Ctx, c_int) -> c_int;
type ValidateInputs = unsafe extern "C" fn(*const Ctx) -> c_int;
type Initialize = unsafe extern "C" fn(*mut Instance, *mut Ctx) -> c_int;
type FillMemoryBlocks = unsafe extern "C" fn(*mut Instance, u32);
type FillSegment = unsafe extern "C" fn(*const Instance, Position);
type Finalize = unsafe extern "C" fn(*const Ctx, *mut Instance);

type Argon2Hash = unsafe extern "C" fn(
    u32,           // t_cost
    u32,           // m_cost
    u32,           // parallelism
    *const c_void, // pwd
    usize,         // pwdlen
    *const c_void, // salt
    usize,         // saltlen
    *mut c_void,   // hash
    usize,         // hashlen
    *mut c_char,   // encoded
    usize,         // encodedlen
    c_int,         // type
) -> c_int;

type HashRaw = unsafe extern "C" fn(
    u32, u32, u32, *const c_void, usize, *const c_void, usize, *mut c_void, usize,
) -> c_int;

type HashEncoded = unsafe extern "C" fn(
    u32, u32, u32, *const c_void, usize, *const c_void, usize, usize, *mut c_char, usize,
) -> c_int;

type Verify = unsafe extern "C" fn(*const c_char, *const c_void, usize) -> c_int;
type VerifyT = unsafe extern "C" fn(*const c_char, *const c_void, usize, c_int) -> c_int;
type Blake2bLong = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> c_int;
type IntGetter = unsafe extern "C" fn() -> c_int;

// ----------------------------------------------------------------- constants

const ARGON2_OK: c_int = 0;
const OUTPUT_PTR_NULL: c_int = -1;
const OUTPUT_TOO_SHORT: c_int = -2;
const OUTPUT_TOO_LONG: c_int = -3;
const PWD_TOO_LONG: c_int = -5;
const SALT_TOO_SHORT: c_int = -6;
const SALT_TOO_LONG: c_int = -7;
const TIME_TOO_SMALL: c_int = -12;
const MEMORY_TOO_LITTLE: c_int = -14;
const LANES_TOO_FEW: c_int = -16;
const LANES_TOO_MANY: c_int = -17;
const PWD_PTR_MISMATCH: c_int = -18;
const SALT_PTR_MISMATCH: c_int = -19;
const SECRET_PTR_MISMATCH: c_int = -20;
const AD_PTR_MISMATCH: c_int = -21;
const INCORRECT_PARAMETER: c_int = -25;
const INCORRECT_TYPE: c_int = -26;
const THREADS_TOO_FEW: c_int = -28;
const THREADS_TOO_MANY: c_int = -29;
const ENCODING_FAIL: c_int = -31;
const VERIFY_MISMATCH: c_int = -35;

const T_I: c_int = 1; // Argon2_i
const T_ID: c_int = 2; // Argon2_id

const FLAG_CLEAR_PASSWORD: u32 = 1;
const FLAG_CLEAR_SECRET: u32 = 2;

const ARGON2_SYNC_POINTS: u32 = 4;

// ------------------------------------------------------------------ helpers

fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

fn as_str(b: &[u8]) -> String {
    let end = b.iter().position(|&x| x == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

/// Description of one `argon2_context`.  `None` in a buffer field means the
/// pointer is NULL; `Some(v)` means a real allocation of `v.len()` bytes.  The
/// `*_len` overrides set the struct's length field independently of the buffer,
/// which is how the `*_PTR_MISMATCH` / `*_TOO_SHORT` branches are reached.
#[derive(Clone)]
struct Spec {
    out_null: bool,
    out_cap: Option<usize>,
    outlen: u32,
    pwd: Option<Vec<u8>>,
    pwdlen: Option<u32>,
    salt: Option<Vec<u8>>,
    saltlen: Option<u32>,
    secret: Option<Vec<u8>>,
    secretlen: Option<u32>,
    ad: Option<Vec<u8>>,
    adlen: Option<u32>,
    t_cost: u32,
    m_cost: u32,
    lanes: u32,
    threads: u32,
    flags: u32,
}

impl Default for Spec {
    /// The minimal legal context of row 8.37: `outlen = 16`, no password,
    /// 8-byte salt, no secret, no ad, `t_cost = 1`, `m_cost = 8` (= `8*lanes`),
    /// `lanes = threads = 1`, `flags = ARGON2_DEFAULT_FLAGS`.
    fn default() -> Self {
        Spec {
            out_null: false,
            out_cap: None,
            outlen: 16,
            pwd: None,
            pwdlen: None,
            salt: Some((0u8..8).collect()),
            saltlen: None,
            secret: None,
            secretlen: None,
            ad: None,
            adlen: None,
            t_cost: 1,
            m_cost: 8,
            lanes: 1,
            threads: 1,
            flags: 0,
        }
    }
}

/// An owned set of buffers plus the `Ctx` that points into them.
struct Live {
    ctx: Ctx,
    out: Vec<u8>,
    out_cap: usize,
    pwd: Option<Vec<u8>>,
    salt: Option<Vec<u8>>,
    secret: Option<Vec<u8>>,
    ad: Option<Vec<u8>>,
}

/// A `Vec` whose `as_mut_ptr()` is a real allocation even when the length is 0
/// (so a non-NULL, zero-length buffer can be handed to C).
fn owned(v: &[u8]) -> Vec<u8> {
    let mut x = Vec::with_capacity(v.len() + 1);
    x.extend_from_slice(v);
    x
}

impl Live {
    fn new(s: &Spec) -> Box<Live> {
        let cap = s.out_cap.unwrap_or(s.outlen as usize);
        let mut l = Box::new(Live {
            ctx: Ctx {
                out: null_mut(),
                outlen: s.outlen,
                pwd: null_mut(),
                pwdlen: 0,
                salt: null_mut(),
                saltlen: 0,
                secret: null_mut(),
                secretlen: 0,
                ad: null_mut(),
                adlen: 0,
                t_cost: s.t_cost,
                m_cost: s.m_cost,
                lanes: s.lanes,
                threads: s.threads,
                flags: s.flags,
            },
            out: padded(cap),
            out_cap: cap,
            pwd: s.pwd.as_deref().map(owned),
            salt: s.salt.as_deref().map(owned),
            secret: s.secret.as_deref().map(owned),
            ad: s.ad.as_deref().map(owned),
        });
        l.ctx.out = if s.out_null { null_mut() } else { l.out.as_mut_ptr() };
        l.ctx.pwdlen = s.pwdlen.unwrap_or(l.pwd.as_ref().map_or(0, |v| v.len() as u32));
        l.ctx.saltlen = s.saltlen.unwrap_or(l.salt.as_ref().map_or(0, |v| v.len() as u32));
        l.ctx.secretlen = s
            .secretlen
            .unwrap_or(l.secret.as_ref().map_or(0, |v| v.len() as u32));
        l.ctx.adlen = s.adlen.unwrap_or(l.ad.as_ref().map_or(0, |v| v.len() as u32));
        l.ctx.pwd = l.pwd.as_mut().map_or(null_mut(), |v| v.as_mut_ptr());
        l.ctx.salt = l.salt.as_mut().map_or(null_mut(), |v| v.as_mut_ptr());
        l.ctx.secret = l.secret.as_mut().map_or(null_mut(), |v| v.as_mut_ptr());
        l.ctx.ad = l.ad.as_mut().map_or(null_mut(), |v| v.as_mut_ptr());
        l
    }
}

/// Everything observable after an `argon2_ctx` call.
#[derive(Debug, PartialEq, Eq)]
struct Run {
    ret: c_int,
    out: Vec<u8>,
    pwd: Vec<u8>,
    secret: Vec<u8>,
    pwdlen: u32,
    secretlen: u32,
    saltlen: u32,
    adlen: u32,
}

fn run_one(f: &Argon2Ctx, s: &Spec, ty: c_int) -> Run {
    let mut l = Live::new(s);
    let ret = unsafe { f(&mut l.ctx, ty) };
    check_pad("argon2_ctx out", &l.out, l.out_cap);
    Run {
        ret,
        out: l.out[..l.out_cap].to_vec(),
        pwd: l.pwd.clone().unwrap_or_default(),
        secret: l.secret.clone().unwrap_or_default(),
        pwdlen: l.ctx.pwdlen,
        secretlen: l.ctx.secretlen,
        saltlen: l.ctx.saltlen,
        adlen: l.ctx.adlen,
    }
}

/// Differential `argon2_ctx`.
#[track_caller]
fn ctx_case(c: &Argon2Ctx, r: &Argon2Ctx, label: &str, s: &Spec, ty: c_int) -> Run {
    let a = run_one(c, s, ty);
    let b = run_one(r, s, ty);
    eqi(&format!("{label} ret"), a.ret, b.ret);
    eqb(&format!("{label} out"), &a.out, &b.out);
    eqb(&format!("{label} pwd after"), &a.pwd, &b.pwd);
    eqb(&format!("{label} secret after"), &a.secret, &b.secret);
    assert_eq!(a.pwdlen, b.pwdlen, "{label}: ctx.pwdlen mismatch");
    assert_eq!(a.secretlen, b.secretlen, "{label}: ctx.secretlen mismatch");
    assert_eq!(a.saltlen, b.saltlen, "{label}: ctx.saltlen mismatch");
    assert_eq!(a.adlen, b.adlen, "{label}: ctx.adlen mismatch");
    a
}

/// Differential `argon2_validate_inputs`.
#[track_caller]
fn validate_case(c: &ValidateInputs, r: &ValidateInputs, label: &str, s: &Spec) -> c_int {
    let lc = Live::new(s);
    let lr = Live::new(s);
    let a = unsafe { c(&lc.ctx) };
    let b = unsafe { r(&lr.ctx) };
    eqi(&format!("{label} validate"), a, b);
    check_pad(label, &lc.out, lc.out_cap);
    check_pad(label, &lr.out, lr.out_cap);
    a
}

/// Differential `argon2_hash`.
#[track_caller]
#[allow(clippy::too_many_arguments)]
fn hash_case(
    c: &Argon2Hash,
    r: &Argon2Hash,
    label: &str,
    t_cost: u32,
    m_cost: u32,
    par: u32,
    pwd: Option<&[u8]>,
    pwdlen: Option<usize>,
    salt: Option<&[u8]>,
    saltlen: Option<usize>,
    hashlen: Option<usize>,
    hash_cap: usize,
    encodedlen: Option<usize>,
    ty: c_int,
) -> (c_int, Vec<u8>, Vec<u8>) {
    let pwd_v = pwd.map(owned);
    let salt_v = salt.map(owned);
    let plen = pwdlen.unwrap_or(pwd_v.as_ref().map_or(0, |v| v.len()));
    let slen = saltlen.unwrap_or(salt_v.as_ref().map_or(0, |v| v.len()));
    let hlen = hashlen.unwrap_or(hash_cap);

    let run = |f: &Argon2Hash| -> (c_int, Vec<u8>, Vec<u8>) {
        let mut hb = padded(hash_cap);
        let mut eb = padded(encodedlen.unwrap_or(0).max(1));
        let ecap = encodedlen.unwrap_or(0);
        rng_reset();
        let ret = unsafe {
            f(
                t_cost,
                m_cost,
                par,
                pwd_v.as_ref().map_or(std::ptr::null(), |v| v.as_ptr() as *const c_void),
                plen,
                salt_v.as_ref().map_or(std::ptr::null(), |v| v.as_ptr() as *const c_void),
                slen,
                if hashlen.is_some() && hash_cap == 0 {
                    std::ptr::null_mut()
                } else {
                    hb.as_mut_ptr() as *mut c_void
                },
                hlen,
                if encodedlen.is_none() {
                    std::ptr::null_mut()
                } else {
                    eb.as_mut_ptr() as *mut c_char
                },
                ecap,
                ty,
            )
        };
        check_pad(&format!("{label} hash"), &hb, hash_cap);
        check_pad(&format!("{label} encoded"), &eb, ecap.max(1));
        (ret, hb[..hash_cap].to_vec(), eb[..ecap].to_vec())
    };

    let (ac, ah, ae) = run(c);
    let (bc, bh, be) = run(r);
    eqi(&format!("{label} ret"), ac, bc);
    eqb(&format!("{label} hash"), &ah, &bh);
    eqb(&format!("{label} encoded"), &ae, &be);
    (ac, ah, ae)
}

// ======================== 8.37 – 8.43 argon2_ctx basics ==================

#[test]
fn r8_37_to_43_ctx_minimal_and_lanes() {
    let _rng = rng_guard();
    let (c, r) = both::<Argon2Ctx>("_sodium_argon2_ctx");

    // 8.37 minimal legal context, Argon2_i.
    let base = Spec::default();
    let a = ctx_case(&c, &r, "8.37 minimal Argon2_i", &base, T_I);
    assert_eq!(a.ret, ARGON2_OK);
    // 8.38 same context, Argon2_id -> different digest.
    let b = ctx_case(&c, &r, "8.38 minimal Argon2_id", &base, T_ID);
    assert_eq!(b.ret, ARGON2_OK);
    assert_ne!(a.out, b.out, "8.38: Argon2_i and Argon2_id agreed");

    // 8.39 lanes = threads = 2, m_cost = 16 (= 8*lanes).
    // 8.40 lanes = threads = 4, m_cost = 32 (multi-lane XOR in argon2_finalize).
    // 8.43 lanes = threads = 8, m_cost = 64.
    let mut per_lane: Vec<(u32, Vec<u8>)> = Vec::new();
    for (lanes, m) in [(1u32, 8u32), (2, 16), (4, 32), (8, 64)] {
        for ty in [T_I, T_ID] {
            let s = Spec {
                outlen: 32,
                lanes,
                threads: lanes,
                m_cost: m,
                ..Spec::default()
            };
            let g = ctx_case(
                &c,
                &r,
                &format!("8.39-8.43 lanes={lanes} m_cost={m} ty={ty}"),
                &s,
                ty,
            );
            assert_eq!(g.ret, ARGON2_OK);
            if ty == T_ID {
                assert!(
                    !per_lane.iter().any(|(_, o)| *o == g.out),
                    "8.39-8.43: lanes={lanes} collided with another lane count"
                );
                per_lane.push((lanes, g.out));
            }
        }
    }

    // 8.41 threads < lanes is legal and never changes the digest.
    // 8.42 threads > lanes is legal too (threads <= ARGON2_MAX_THREADS).
    for (lanes, m) in [(2u32, 16u32), (4, 32)] {
        let mut digests: Vec<Vec<u8>> = Vec::new();
        for threads in [1u32, lanes, lanes + 4, 0xFFFFFF] {
            let s = Spec {
                outlen: 32,
                lanes,
                threads,
                m_cost: m,
                ..Spec::default()
            };
            let g = ctx_case(
                &c,
                &r,
                &format!("8.41/8.42 lanes={lanes} threads={threads}"),
                &s,
                T_ID,
            );
            assert_eq!(g.ret, ARGON2_OK, "8.41/8.42 threads={threads}");
            digests.push(g.out);
        }
        for d in &digests[1..] {
            eqb("8.41/8.42 threads must not affect the digest", &digests[0], d);
        }
    }
}

// ===================== 8.44 – 8.46 m_cost rounding ======================

#[test]
fn r8_44_to_46_m_cost_rounding() {
    let _rng = rng_guard();
    let (c, r) = both::<Argon2Ctx>("_sodium_argon2_ctx");

    // 8.44 lanes = 1: m_cost 8..11 all give segment_length 2 — but note that
    // `argon2_initial_hash` hashes the caller's raw m_cost, so the *digests*
    // still differ; what is identical is the amount of work.  m_cost = 12 moves
    // to segment_length 3.
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for m in [8u32, 9, 10, 11, 12] {
        let s = Spec { outlen: 32, m_cost: m, ..Spec::default() };
        let g = ctx_case(&c, &r, &format!("8.44 m_cost={m}"), &s, T_ID);
        assert_eq!(g.ret, ARGON2_OK);
        assert!(
            !seen.contains(&g.out),
            "8.44: m_cost={m} collided (m_cost is hashed verbatim, so all differ)"
        );
        seen.push(g.out);
    }

    // 8.45 lanes = 2: m_cost 16..23 -> segment_length 2, m_cost 24 -> 3.
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for m in [16u32, 17, 19, 23, 24] {
        let s = Spec {
            outlen: 32,
            lanes: 2,
            threads: 2,
            m_cost: m,
            ..Spec::default()
        };
        let g = ctx_case(&c, &r, &format!("8.45 lanes=2 m_cost={m}"), &s, T_ID);
        assert_eq!(g.ret, ARGON2_OK);
        assert!(!seen.contains(&g.out), "8.45: m_cost={m} collided");
        seen.push(g.out);
    }

    // 8.46 moderate m_cost: 512 (segment_length == ARGON2_ADDRESSES_IN_BLOCK)
    // and 1024 (segment_length 256, which forces a second address block in
    // `generate_addresses`).
    for m in [512u32, 1024] {
        for ty in [T_I, T_ID] {
            let s = Spec { outlen: 32, m_cost: m, ..Spec::default() };
            let g = ctx_case(&c, &r, &format!("8.46 m_cost={m} ty={ty}"), &s, ty);
            assert_eq!(g.ret, ARGON2_OK, "8.46 m_cost={m}");
        }
    }
}

// ====================== 8.47 – 8.51 t_cost / addressing =================

#[test]
fn r8_47_to_51_t_cost_and_addressing() {
    let _rng = rng_guard();
    let (c, r) = both::<Argon2Ctx>("_sodium_argon2_ctx");

    // 8.47 t_cost = 1 (single pass, fill_block only)
    // 8.48 t_cost = 2 (fill_block_with_xor and the pass != 0 index_alpha branch)
    // 8.49 t_cost = 3
    // 8.50 Argon2_id: t_cost 1 mixes data-independent slices 0-1 with
    //      data-dependent slices 2-3; t_cost 2's second pass is fully
    //      data-dependent.
    // 8.51 Argon2_i: every pass and slice is data-independent.
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for ty in [T_I, T_ID] {
        for t in [1u32, 2, 3] {
            for (lanes, m) in [(1u32, 8u32), (2, 16), (1, 512)] {
                let s = Spec {
                    outlen: 32,
                    t_cost: t,
                    m_cost: m,
                    lanes,
                    threads: lanes,
                    ..Spec::default()
                };
                let g = ctx_case(
                    &c,
                    &r,
                    &format!("8.47-8.51 ty={ty} t_cost={t} lanes={lanes} m={m}"),
                    &s,
                    ty,
                );
                assert_eq!(g.ret, ARGON2_OK);
                assert!(
                    !seen.contains(&g.out),
                    "8.47-8.51: ty={ty} t={t} lanes={lanes} m={m} collided"
                );
                seen.push(g.out);
            }
        }
    }
}

// =========================== 8.52 outlen matrix =========================

#[test]
fn r8_52_outlen_matrix() {
    let _rng = rng_guard();
    let (c, r) = both::<Argon2Ctx>("_sodium_argon2_ctx");
    // 16 (MIN) / 24 / 32 / 48 / 64 take the short blake2b_long path;
    // 65 / 96 / 128 / 1024 take the long one.
    for outlen in [16u32, 17, 24, 32, 48, 63, 64, 65, 96, 100, 128, 1024] {
        for ty in [T_I, T_ID] {
            let s = Spec { outlen, ..Spec::default() };
            let g = ctx_case(&c, &r, &format!("8.52 outlen={outlen} ty={ty}"), &s, ty);
            assert_eq!(g.ret, ARGON2_OK, "8.52 outlen={outlen}");
            assert!(
                g.out.iter().any(|&b| b != 0),
                "8.52 outlen={outlen}: output left untouched"
            );
        }
    }
}

// ====================== 8.53 – 8.57 salt / pwd / secret / ad =============

#[test]
fn r8_53_to_57_optional_inputs() {
    let _rng = rng_guard();
    let (c, r) = both::<Argon2Ctx>("_sodium_argon2_ctx");
    let mut rng = Rng::new(0x8_0053);

    // 8.53 saltlen 8 (min) / 16 / 32 / 64, all-zero and random salts.
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for saltlen in [8usize, 9, 16, 32, 64] {
        for zero in [true, false] {
            let salt = if zero { vec![0u8; saltlen] } else { rng.bytes(saltlen) };
            let s = Spec { outlen: 32, salt: Some(salt), ..Spec::default() };
            let g = ctx_case(&c, &r, &format!("8.53 saltlen={saltlen} zero={zero}"), &s, T_ID);
            assert_eq!(g.ret, ARGON2_OK);
            assert!(!seen.contains(&g.out), "8.53: saltlen={saltlen} zero={zero} collided");
            seen.push(g.out);
        }
    }

    // 8.54 pwd absent (NULL, 0) vs present with pwdlen 0 vs 1 vs 64.  The first
    // two must agree: only `pwdlen` and the bytes are hashed.
    let s_null = Spec { outlen: 32, ..Spec::default() };
    let s_empty = Spec { outlen: 32, pwd: Some(Vec::new()), ..Spec::default() };
    let a = ctx_case(&c, &r, "8.54 pwd NULL", &s_null, T_ID);
    let b = ctx_case(&c, &r, "8.54 pwd non-NULL, pwdlen 0", &s_empty, T_ID);
    eqb("8.54 NULL pwd == empty pwd", &a.out, &b.out);
    let mut seen = vec![a.out.clone()];
    for pwdlen in [1usize, 8, 64] {
        let s = Spec { outlen: 32, pwd: Some(rng.bytes(pwdlen)), ..Spec::default() };
        let g = ctx_case(&c, &r, &format!("8.54 pwdlen={pwdlen}"), &s, T_ID);
        assert_eq!(g.ret, ARGON2_OK);
        assert!(!seen.contains(&g.out), "8.54: pwdlen={pwdlen} collided");
        seen.push(g.out);
    }

    // 8.55 keyed argon2: secret absent vs non-NULL with secretlen 0 vs 8/16/32.
    let s_secret_empty = Spec { outlen: 32, secret: Some(Vec::new()), ..Spec::default() };
    let e = ctx_case(&c, &r, "8.55 secret non-NULL, secretlen 0", &s_secret_empty, T_ID);
    eqb("8.55 NULL secret == empty secret", &a.out, &e.out);
    let mut seen = vec![a.out.clone()];
    for secretlen in [8usize, 16, 32] {
        let s = Spec { outlen: 32, secret: Some(rng.bytes(secretlen)), ..Spec::default() };
        let g = ctx_case(&c, &r, &format!("8.55 secretlen={secretlen}"), &s, T_ID);
        assert_eq!(g.ret, ARGON2_OK);
        assert!(!seen.contains(&g.out), "8.55: secretlen={secretlen} collided");
        seen.push(g.out);
    }

    // 8.56 associated data absent vs non-NULL with adlen 0 vs 8/16/64.
    let s_ad_empty = Spec { outlen: 32, ad: Some(Vec::new()), ..Spec::default() };
    let d = ctx_case(&c, &r, "8.56 ad non-NULL, adlen 0", &s_ad_empty, T_ID);
    eqb("8.56 NULL ad == empty ad", &a.out, &d.out);
    let mut seen = vec![a.out.clone()];
    for adlen in [8usize, 16, 64] {
        let s = Spec { outlen: 32, ad: Some(rng.bytes(adlen)), ..Spec::default() };
        let g = ctx_case(&c, &r, &format!("8.56 adlen={adlen}"), &s, T_ID);
        assert_eq!(g.ret, ARGON2_OK);
        assert!(!seen.contains(&g.out), "8.56: adlen={adlen} collided");
        seen.push(g.out);
    }

    // 8.57 secret and ad together, Argon2_id, t_cost 2, m_cost 16, lanes 2.
    let s = Spec {
        outlen: 32,
        secret: Some(rng.bytes(16)),
        ad: Some(rng.bytes(16)),
        t_cost: 2,
        m_cost: 16,
        lanes: 2,
        threads: 2,
        ..Spec::default()
    };
    let g = ctx_case(&c, &r, "8.57 secret + ad", &s, T_ID);
    assert_eq!(g.ret, ARGON2_OK);
}

// ============================ 8.58 – 8.60 flags =========================

#[test]
fn r8_58_to_60_clear_flags() {
    let _rng = rng_guard();
    let (c, r) = both::<Argon2Ctx>("_sodium_argon2_ctx");
    let mut rng = Rng::new(0x8_0058);
    let pwd = rng.bytes(16);
    let secret = rng.bytes(16);

    let plain = Spec {
        outlen: 32,
        pwd: Some(pwd.clone()),
        secret: Some(secret.clone()),
        ..Spec::default()
    };
    let base = ctx_case(&c, &r, "8.58 default flags", &plain, T_ID);
    assert_eq!(base.ret, ARGON2_OK);
    eqb("8.58 password untouched with default flags", &pwd, &base.pwd);
    eqb("8.58 secret untouched with default flags", &secret, &base.secret);
    assert_eq!(base.pwdlen, 16);
    assert_eq!(base.secretlen, 16);

    // 8.58 ARGON2_FLAG_CLEAR_PASSWORD.
    let s = Spec { flags: FLAG_CLEAR_PASSWORD, ..plain.clone() };
    let g = ctx_case(&c, &r, "8.58 CLEAR_PASSWORD", &s, T_ID);
    assert_eq!(g.ret, ARGON2_OK);
    eqb("8.58 digest unchanged by CLEAR_PASSWORD", &base.out, &g.out);
    assert!(g.pwd.iter().all(|&b| b == 0), "8.58: pwd must be zeroed");
    assert_eq!(g.pwdlen, 0, "8.58: ctx.pwdlen must be 0 afterwards");
    eqb("8.58 secret untouched", &secret, &g.secret);
    assert_eq!(g.secretlen, 16);

    // 8.59 ARGON2_FLAG_CLEAR_SECRET.
    let s = Spec { flags: FLAG_CLEAR_SECRET, ..plain.clone() };
    let g = ctx_case(&c, &r, "8.59 CLEAR_SECRET", &s, T_ID);
    assert_eq!(g.ret, ARGON2_OK);
    eqb("8.59 digest unchanged by CLEAR_SECRET", &base.out, &g.out);
    assert!(g.secret.iter().all(|&b| b == 0), "8.59: secret must be zeroed");
    assert_eq!(g.secretlen, 0, "8.59: ctx.secretlen must be 0 afterwards");
    eqb("8.59 pwd untouched", &pwd, &g.pwd);
    assert_eq!(g.pwdlen, 16);

    // 8.60 both flags.
    let s = Spec {
        flags: FLAG_CLEAR_PASSWORD | FLAG_CLEAR_SECRET,
        ..plain.clone()
    };
    let g = ctx_case(&c, &r, "8.60 both flags", &s, T_ID);
    assert_eq!(g.ret, ARGON2_OK);
    eqb("8.60 digest unchanged", &base.out, &g.out);
    assert!(g.pwd.iter().all(|&b| b == 0));
    assert!(g.secret.iter().all(|&b| b == 0));
    assert_eq!(g.pwdlen, 0);
    assert_eq!(g.secretlen, 0);

    // Unknown flag bits are ignored.
    let s = Spec { flags: 0xFFFF_FFFC, ..plain.clone() };
    let g = ctx_case(&c, &r, "flags with unknown bits", &s, T_ID);
    assert_eq!(g.ret, ARGON2_OK);
    eqb("unknown flag bits are ignored", &base.out, &g.out);
    eqb("pwd untouched", &pwd, &g.pwd);
}

// ==================== 8.61 – 8.66 argon2_hash output modes ==============

#[test]
fn r8_61_to_66_argon2_hash_modes() {
    let _rng = rng_guard();
    let (c, r) = both::<Argon2Hash>("_sodium_argon2_hash");
    let (cir, rir) = both::<HashRaw>("_sodium_argon2i_hash_raw");
    let (cie, rie) = both::<HashEncoded>("_sodium_argon2i_hash_encoded");
    let pwd = b"password".to_vec();
    let salt = b"saltsalt".to_vec();

    // 8.61 raw-only mode; must equal argon2i_hash_raw.
    let (ret, raw, _) = hash_case(
        &c, &r, "8.61 raw only", 1, 8, 1, Some(&pwd), None, Some(&salt), None, None, 16, None, T_I,
    );
    assert_eq!(ret, ARGON2_OK);
    {
        let mut a = padded(16);
        let mut b = padded(16);
        rng_reset();
        let x = unsafe {
            cir(
                1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), a.as_mut_ptr() as *mut c_void, 16,
            )
        };
        rng_reset();
        let y = unsafe {
            rir(
                1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), b.as_mut_ptr() as *mut c_void, 16,
            )
        };
        eqi("8.61 argon2i_hash_raw", x, y);
        assert_eq!(x, ARGON2_OK);
        eqb("8.61 argon2i_hash_raw C vs Rust", &a[..16], &b[..16]);
        eqb("8.61 argon2_hash == argon2i_hash_raw", &raw, &a[..16]);
    }

    // 8.62 encoded-only mode; must equal argon2i_hash_encoded.
    let (ret, _, enc) = hash_case(
        &c, &r, "8.62 encoded only", 1, 8, 1, Some(&pwd), None, Some(&salt), None, Some(16), 0,
        Some(128), T_I,
    );
    assert_eq!(ret, ARGON2_OK);
    {
        let mut a = padded(128);
        let mut b = padded(128);
        rng_reset();
        let x = unsafe {
            cie(
                1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), 16, a.as_mut_ptr() as *mut c_char, 128,
            )
        };
        rng_reset();
        let y = unsafe {
            rie(
                1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), 16, b.as_mut_ptr() as *mut c_char, 128,
            )
        };
        eqi("8.62 argon2i_hash_encoded", x, y);
        assert_eq!(x, ARGON2_OK);
        eqb("8.62 argon2i_hash_encoded C vs Rust", &a[..128], &b[..128]);
        assert_eq!(
            as_str(&enc),
            as_str(&a[..128]),
            "8.62: argon2_hash's encoded output differs from argon2i_hash_encoded"
        );
        assert_eq!(
            as_str(&enc),
            format!("$argon2i$v=19$m=8,t=1,p=1$c2FsdHNhbHQ${}", as_str(&enc).rsplit('$').next().unwrap()),
            "8.62: unexpected encoded shape {:?}", as_str(&enc)
        );
    }

    // 8.63 both outputs in one call: the raw digest must match the base64 in the
    // encoded string.
    let (ret, raw2, enc2) = hash_case(
        &c, &r, "8.63 raw + encoded", 1, 8, 1, Some(&pwd), None, Some(&salt), None, None, 16,
        Some(128), T_I,
    );
    assert_eq!(ret, ARGON2_OK);
    eqb("8.63 raw digest is stable", &raw, &raw2);
    let tail = as_str(&enc2).rsplit('$').next().unwrap().to_string();
    assert_eq!(tail.len(), 22, "8.63: 16-byte digest is 22 base64 chars");

    // 8.64 encoded != NULL but encodedlen == 0: the `if (encoded && encodedlen)`
    // guard skips encoding and `encoded` is left untouched.
    {
        let mut a = padded(128);
        let mut b = padded(128);
        for i in 0..128 {
            a[i] = 0xC3;
            b[i] = 0xC3;
        }
        let mut ha = padded(16);
        let mut hb = padded(16);
        rng_reset();
        let x = unsafe {
            c(
                1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), ha.as_mut_ptr() as *mut c_void, 16,
                a.as_mut_ptr() as *mut c_char, 0, T_I,
            )
        };
        rng_reset();
        let y = unsafe {
            r(
                1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), hb.as_mut_ptr() as *mut c_void, 16,
                b.as_mut_ptr() as *mut c_char, 0, T_I,
            )
        };
        eqi("8.64 encodedlen=0", x, y);
        assert_eq!(x, ARGON2_OK);
        eqb("8.64 hash", &ha[..16], &hb[..16]);
        eqb("8.64 encoded buffer", &a[..128], &b[..128]);
        assert!(
            a[..128].iter().all(|&v| v == 0xC3),
            "8.64: `encoded` must be left untouched when encodedlen == 0"
        );
        eqb("8.64 digest unchanged", &raw, &ha[..16]);
    }

    // 8.65 both outputs suppressed: still ARGON2_OK after the full KDF.
    {
        rng_reset();
        let x = unsafe {
            c(
                1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), std::ptr::null_mut(), 16,
                std::ptr::null_mut(), 0, T_I,
            )
        };
        rng_reset();
        let y = unsafe {
            r(
                1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), std::ptr::null_mut(), 16,
                std::ptr::null_mut(), 0, T_I,
            )
        };
        eqi("8.65 hash == NULL && encoded == NULL", x, y);
        assert_eq!(x, ARGON2_OK, "8.65: must still return ARGON2_OK");
    }

    // 8.66 parallelism > 1 (argon2_hash sets lanes = threads = parallelism).
    for (par, m) in [(2u32, 16u32), (4, 32)] {
        for ty in [T_I, T_ID] {
            let (ret, _, _) = hash_case(
                &c, &r, &format!("8.66 parallelism={par} ty={ty}"), 1, m, par, Some(&pwd), None,
                Some(&salt), None, None, 32, None, ty,
            );
            assert_eq!(ret, ARGON2_OK, "8.66 parallelism={par}");
        }
    }
}

// ================ 8.67 – 8.69 argon2{i,id}_hash_raw matrices =============

#[test]
fn r8_67_to_69_hash_raw_matrix() {
    let _rng = rng_guard();
    let (ci, ri) = both::<HashRaw>("_sodium_argon2i_hash_raw");
    let (cd, rd) = both::<HashRaw>("_sodium_argon2id_hash_raw");
    let mut rng = Rng::new(0x8_0067);

    #[track_caller]
    fn raw(
        c: &HashRaw,
        r: &HashRaw,
        label: &str,
        t: u32,
        m: u32,
        par: u32,
        pwd: &[u8],
        salt: &[u8],
        hashlen: usize,
    ) -> (c_int, Vec<u8>) {
        let mut a = padded(hashlen);
        let mut b = padded(hashlen);
        rng_reset();
        let x = unsafe {
            c(
                t, m, par, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), a.as_mut_ptr() as *mut c_void, hashlen,
            )
        };
        rng_reset();
        let y = unsafe {
            r(
                t, m, par, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), b.as_mut_ptr() as *mut c_void, hashlen,
            )
        };
        eqi(&format!("{label} ret"), x, y);
        eqb(&format!("{label} hash"), &a[..hashlen], &b[..hashlen]);
        check_pad(&format!("{label}(C)"), &a, hashlen);
        check_pad(&format!("{label}(Rust)"), &b, hashlen);
        (x, a[..hashlen].to_vec())
    }

    // 8.67 t_cost 1, m_cost 8, parallelism 1; pwdlen 0..32, saltlen 8/16,
    // hashlen 16/32/64.
    for pwdlen in [0usize, 1, 8, 32] {
        for saltlen in [8usize, 16] {
            for hashlen in [16usize, 32, 64] {
                let pwd = rng.bytes(pwdlen);
                let salt = rng.bytes(saltlen);
                let (x, hi) = raw(
                    &ci, &ri,
                    &format!("8.67 i pwdlen={pwdlen} saltlen={saltlen} hashlen={hashlen}"),
                    1, 8, 1, &pwd, &salt, hashlen,
                );
                assert_eq!(x, ARGON2_OK);
                // 8.69 the same matrix with Argon2_id; digests must differ.
                let (y, hd) = raw(
                    &cd, &rd,
                    &format!("8.69 id pwdlen={pwdlen} saltlen={saltlen} hashlen={hashlen}"),
                    1, 8, 1, &pwd, &salt, hashlen,
                );
                assert_eq!(y, ARGON2_OK);
                assert_ne!(hi, hd, "8.69: argon2i and argon2id digests agreed");
            }
        }
    }

    // 8.68 t_cost 2, m_cost 32, parallelism 4 (m_cost == 8 * parallelism).
    let pwd = rng.bytes(16);
    let salt = rng.bytes(16);
    let (x, hi) = raw(&ci, &ri, "8.68 i t=2 m=32 p=4", 2, 32, 4, &pwd, &salt, 32);
    assert_eq!(x, ARGON2_OK);
    let (y, hd) = raw(&cd, &rd, "8.68 id t=2 m=32 p=4", 2, 32, 4, &pwd, &salt, 32);
    assert_eq!(y, ARGON2_OK);
    assert_ne!(hi, hd);
}

// ============= 8.70 – 8.75 hash_encoded / verify round trips =============

#[test]
fn r8_70_to_75_encoded_and_verify() {
    let _rng = rng_guard();
    let (cie, rie) = both::<HashEncoded>("_sodium_argon2i_hash_encoded");
    let (cde, rde) = both::<HashEncoded>("_sodium_argon2id_hash_encoded");
    let (civ, riv) = both::<Verify>("_sodium_argon2i_verify");
    let (cdv, rdv) = both::<Verify>("_sodium_argon2id_verify");
    let (cv, rv) = both::<VerifyT>("_sodium_argon2_verify");

    #[track_caller]
    fn enc(
        c: &HashEncoded,
        r: &HashEncoded,
        label: &str,
        t: u32,
        m: u32,
        par: u32,
        pwd: &[u8],
        salt: &[u8],
        hashlen: usize,
        encodedlen: usize,
    ) -> (c_int, String) {
        let mut a = padded(encodedlen);
        let mut b = padded(encodedlen);
        rng_reset();
        let x = unsafe {
            c(
                t, m, par, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), hashlen,
                a.as_mut_ptr() as *mut c_char, encodedlen,
            )
        };
        rng_reset();
        let y = unsafe {
            r(
                t, m, par, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), hashlen,
                b.as_mut_ptr() as *mut c_char, encodedlen,
            )
        };
        eqi(&format!("{label} ret"), x, y);
        eqb(&format!("{label} encoded"), &a[..encodedlen], &b[..encodedlen]);
        check_pad(&format!("{label}(C)"), &a, encodedlen);
        check_pad(&format!("{label}(Rust)"), &b, encodedlen);
        (x, as_str(&a[..encodedlen]))
    }

    #[track_caller]
    fn ver(c: &Verify, r: &Verify, label: &str, s: &str, pwd: &[u8]) -> c_int {
        let z = cstr(s);
        let a = unsafe {
            c(z.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, pwd.len())
        };
        let b = unsafe {
            r(z.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, pwd.len())
        };
        eqi(label, a, b);
        a
    }

    let pwd = b"password".to_vec();

    // 8.70 argon2i, saltlen 8, hashlen 16 -> "$argon2i$v=19$m=8,t=1,p=1$<11>$<22>"
    let salt8 = b"saltsalt".to_vec();
    let (x, s70) = enc(&cie, &rie, "8.70", 1, 8, 1, &pwd, &salt8, 16, 128);
    assert_eq!(x, ARGON2_OK);
    let parts: Vec<&str> = s70.split('$').collect();
    assert_eq!(parts[1], "argon2i");
    assert_eq!(parts[2], "v=19");
    assert_eq!(parts[3], "m=8,t=1,p=1");
    assert_eq!(parts[4].len(), 11, "8.70: 8-byte salt is 11 base64 chars");
    assert_eq!(parts[5].len(), 22, "8.70: 16-byte hash is 22 base64 chars");
    // encodedlen exactly strlen(result) + 1 is still accepted.
    let (x2, s70b) = enc(&cie, &rie, "8.70 tight encodedlen", 1, 8, 1, &pwd, &salt8, 16, s70.len() + 1);
    assert_eq!(x2, ARGON2_OK, "8.70: encodedlen == strlen + 1 must be accepted");
    assert_eq!(s70b, s70);

    // 8.71 argon2id, saltlen 16, hashlen 32.
    let salt16 = b"0123456789abcdef".to_vec();
    let (x, s71) = enc(&cde, &rde, "8.71", 1, 8, 1, &pwd, &salt16, 32, 128);
    assert_eq!(x, ARGON2_OK);
    let parts: Vec<&str> = s71.split('$').collect();
    assert_eq!(parts[1], "argon2id");
    assert_eq!(parts[3], "m=8,t=1,p=1");
    assert_eq!(parts[4].len(), 22);
    assert_eq!(parts[5].len(), 43);

    // 8.72 argon2id with parallelism 2, m_cost 16, t_cost 2.
    let (x, s72) = enc(&cde, &rde, "8.72", 2, 16, 2, &pwd, &salt16, 32, 128);
    assert_eq!(x, ARGON2_OK);
    assert!(
        s72.starts_with("$argon2id$v=19$m=16,t=2,p=2$"),
        "8.72: {s72:?}"
    );

    // 8.73 round trips with the correct password, including pwdlen = 0.
    assert_eq!(ver(&civ, &riv, "8.73 argon2i_verify", &s70, &pwd), ARGON2_OK);
    assert_eq!(ver(&cdv, &rdv, "8.73 argon2id_verify s71", &s71, &pwd), ARGON2_OK);
    assert_eq!(ver(&cdv, &rdv, "8.73 argon2id_verify s72 (p=2)", &s72, &pwd), ARGON2_OK);
    let (x, s_empty) = enc(&cde, &rde, "8.73 empty password", 1, 8, 1, &[], &salt16, 32, 128);
    assert_eq!(x, ARGON2_OK);
    assert_eq!(ver(&cdv, &rdv, "8.73 argon2id_verify empty pwd", &s_empty, &[]), ARGON2_OK);
    // Wrong password -> ARGON2_VERIFY_MISMATCH (errors row 8.98).
    assert_eq!(
        ver(&cdv, &rdv, "8.98 wrong password", &s71, b"Password"),
        VERIFY_MISMATCH
    );
    assert_eq!(ver(&cdv, &rdv, "8.98 empty vs non-empty", &s71, &[]), VERIFY_MISMATCH);

    // 8.74 the generic argon2_verify with an explicit type, hashlen 16 and 64.
    for (ty, kind) in [(T_I, "argon2i"), (T_ID, "argon2id")] {
        for hashlen in [16usize, 64] {
            let (c2, r2) = if ty == T_I { (&cie, &rie) } else { (&cde, &rde) };
            let (x, s) = enc(
                c2, r2, &format!("8.74 {kind} hashlen={hashlen}"), 1, 8, 1, &pwd, &salt16,
                hashlen, 256,
            );
            assert_eq!(x, ARGON2_OK);
            let z = cstr(&s);
            let a = unsafe {
                cv(z.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, pwd.len(), ty)
            };
            let b = unsafe {
                rv(z.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, pwd.len(), ty)
            };
            eqi(&format!("8.74 argon2_verify {kind} hashlen={hashlen}"), a, b);
            assert_eq!(a, ARGON2_OK, "8.74 {kind} hashlen={hashlen}");
            // The wrong explicit type must be rejected.
            let other = if ty == T_I { T_ID } else { T_I };
            let a = unsafe {
                cv(z.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, pwd.len(), other)
            };
            let b = unsafe {
                rv(z.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, pwd.len(), other)
            };
            eqi(&format!("8.74 argon2_verify wrong type {kind}"), a, b);
            assert_ne!(a, ARGON2_OK);
        }
    }

    // 8.75 an encoded string with p=2 verifies because argon2_decode_string
    // copies `lanes` into `threads`.
    let z = cstr(&s72);
    let a = unsafe {
        cv(z.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, pwd.len(), T_ID)
    };
    let b = unsafe {
        rv(z.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, pwd.len(), T_ID)
    };
    eqi("8.75 argon2_verify with p=2", a, b);
    assert_eq!(a, ARGON2_OK, "8.75: a p=2 string must verify");

    // 8.100 a type that is neither Argon2_i nor Argon2_id.
    for ty in [0, 3, 4, -1, c_int::MAX, c_int::MIN] {
        let a = unsafe {
            cv(z.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, pwd.len(), ty)
        };
        let b = unsafe {
            rv(z.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, pwd.len(), ty)
        };
        eqi(&format!("8.100 argon2_verify(type={ty})"), a, b);
        assert_eq!(a, INCORRECT_TYPE, "8.100 type={ty}: expected -26");
    }
}

// ==================== 8.87 / 8.88 blake2b_long ==========================

#[test]
fn r8_87_88_blake2b_long() {
    let (c, r) = both::<Blake2bLong>("_sodium_blake2b_long");
    let mut rng = Rng::new(0x8_0087);
    for inlen in [0usize, 1, 32, 64, 72, 1024] {
        let input = rng.bytes(inlen);
        // 16/32/64 take the short path; 65/100/128/1024 the long one.  100 is
        // not a multiple of 32 and so exercises the final partial `toproduce`.
        for outlen in [16usize, 32, 63, 64, 65, 96, 100, 128, 129, 1024] {
            let mut a = padded(outlen);
            let mut b = padded(outlen);
            let x = unsafe {
                c(
                    a.as_mut_ptr() as *mut c_void,
                    outlen,
                    input.as_ptr() as *const c_void,
                    inlen,
                )
            };
            let y = unsafe {
                r(
                    b.as_mut_ptr() as *mut c_void,
                    outlen,
                    input.as_ptr() as *const c_void,
                    inlen,
                )
            };
            let label = format!("8.87/8.88 blake2b_long(outlen={outlen}, inlen={inlen})");
            eqi(&label, x, y);
            assert_eq!(x, 0, "{label}: unexpected failure");
            eqb(&label, &a[..outlen], &b[..outlen]);
            check_pad(&label, &a, outlen);
            check_pad(&label, &b, outlen);
        }
    }
}

// ============ 8.89 / 8.90 fill_segment_ref and implementation choice ======

#[test]
fn r8_89_90_manual_pipeline_and_pick_best() {
    let _rng = rng_guard();
    let (cctx, rctx) = both::<Argon2Ctx>("_sodium_argon2_ctx");
    let (cinit, rinit) = both::<Initialize>("_sodium_argon2_initialize");
    let (cfmb, rfmb) = both::<FillMemoryBlocks>("_sodium_argon2_fill_memory_blocks");
    let (cfs, rfs) = both::<FillSegment>("_sodium_argon2_fill_segment_ref");
    let (cfin, rfin) = both::<Finalize>("_sodium_argon2_finalize");

    /// Reproduce `argon2_ctx` out of its exported building blocks.  `by_segment`
    /// selects between `argon2_fill_memory_blocks` and driving
    /// `argon2_fill_segment_ref` directly (the `(pass, slice)` and
    /// `data_independent_addressing` combinations of row 8.89).
    fn pipeline(
        init: &Initialize,
        fmb: &FillMemoryBlocks,
        fs: &FillSegment,
        fin: &Finalize,
        s: &Spec,
        ty: c_int,
        by_segment: bool,
    ) -> Vec<u8> {
        let mut l = Live::new(s);
        let mut blocks = l.ctx.m_cost;
        if blocks < 2 * ARGON2_SYNC_POINTS * l.ctx.lanes {
            blocks = 2 * ARGON2_SYNC_POINTS * l.ctx.lanes;
        }
        let seg = blocks / (l.ctx.lanes * ARGON2_SYNC_POINTS);
        let blocks = seg * l.ctx.lanes * ARGON2_SYNC_POINTS;
        let mut inst = Instance {
            region: null_mut(),
            pseudo_rands: null_mut(),
            passes: l.ctx.t_cost,
            current_pass: !0u32,
            memory_blocks: blocks,
            segment_length: seg,
            lane_length: seg * ARGON2_SYNC_POINTS,
            lanes: l.ctx.lanes,
            threads: l.ctx.threads,
            ty,
            print_internals: 0,
        };
        unsafe {
            assert_eq!(init(&mut inst, &mut l.ctx), ARGON2_OK, "argon2_initialize failed");
            for pass in 0..inst.passes {
                if by_segment {
                    for slice in 0..ARGON2_SYNC_POINTS {
                        for lane in 0..inst.lanes {
                            fs(
                                &inst,
                                Position { pass, lane, slice: slice as u8, index: 0 },
                            );
                        }
                    }
                } else {
                    fmb(&mut inst, pass);
                }
            }
            fin(&l.ctx, &mut inst);
        }
        check_pad("pipeline out", &l.out, l.out_cap);
        l.out[..l.out_cap].to_vec()
    }

    // All four data_independent_addressing / starting_index combinations are
    // reached by t_cost >= 2 with both types (pass 0 slice 0 has
    // starting_index 2; pass 0 slice > 0 and pass > 0 have starting_index 0).
    for ty in [T_I, T_ID] {
        for (t, lanes, m) in [(1u32, 1u32, 8u32), (2, 1, 8), (2, 2, 16), (3, 1, 512)] {
            let s = Spec {
                outlen: 32,
                pwd: Some(b"password".to_vec()),
                t_cost: t,
                m_cost: m,
                lanes,
                threads: lanes,
                ..Spec::default()
            };
            let reference = ctx_case(
                &cctx,
                &rctx,
                &format!("8.89 reference ty={ty} t={t} lanes={lanes}"),
                &s,
                ty,
            );
            assert_eq!(reference.ret, ARGON2_OK);
            for by_segment in [false, true] {
                let a = pipeline(&cinit, &cfmb, &cfs, &cfin, &s, ty, by_segment);
                let b = pipeline(&rinit, &rfmb, &rfs, &rfin, &s, ty, by_segment);
                let label = format!(
                    "8.89 manual pipeline ty={ty} t={t} lanes={lanes} by_segment={by_segment}"
                );
                eqb(&label, &a, &b);
                eqb(&format!("{label} == argon2_ctx"), &reference.out, &a);
            }
        }
    }

    // 8.90 _crypto_pwhash_argon2_pick_best_implementation returns 0 and leaves
    // `fill_segment` at argon2_fill_segment_ref, so digests are unchanged.
    let s = Spec {
        outlen: 32,
        pwd: Some(b"password".to_vec()),
        t_cost: 2,
        m_cost: 16,
        lanes: 2,
        threads: 2,
        ..Spec::default()
    };
    let before = ctx_case(&cctx, &rctx, "8.90 before", &s, T_ID);
    let (cp, rp) = both::<IntGetter>("_crypto_pwhash_argon2_pick_best_implementation");
    let (a, b) = unsafe { (cp(), rp()) };
    eqi("_crypto_pwhash_argon2_pick_best_implementation", a, b);
    assert_eq!(a, 0);
    let after = ctx_case(&cctx, &rctx, "8.90 after", &s, T_ID);
    eqb("8.90 digest unchanged by pick_best_implementation", &before.out, &after.out);
}

// ============================ error surface =============================
//
// errors_8.md rows 8.53 – 8.111.  Rows that cannot be reached on this platform,
// or cannot be reached safely, are documented rather than executed:
//
//   * 8.60 / 8.63 / 8.66 / 8.69 / 8.72 / 8.76 / 8.79 (`*_TOO_LONG` against
//     `UINT32_MAX`) are unreachable through `argon2_context` because the field
//     is itself a `uint32_t`; the `argon2_hash` variants (rows 8.83 – 8.85)
//     are exercised below.
//   * 8.62 / 8.68 / 8.71 (`*_TOO_SHORT` against a minimum of 0) are unreachable.
//   * 8.55 / 8.82 / 8.86 / 8.96 / 8.99 / 8.102 – 8.106 need an allocation
//     failure; 8.106 in particular would `mmap(MAP_POPULATE)` 4 TiB.
//   * 8.109 is a silent-failure path that cannot fire because `outlen >= 16`.

#[test]
fn e8_53_to_56_ctx_type_and_null() {
    let _rng = rng_guard();
    let (c, r) = both::<Argon2Ctx>("_sodium_argon2_ctx");

    // 8.54 a `type` with no valid variant (Argon2_d is not compiled in).
    for ty in [0, 3, 4, -1, 5, c_int::MAX, c_int::MIN] {
        let s = Spec::default();
        let g = ctx_case(&c, &r, &format!("8.54 type={ty}"), &s, ty);
        assert_eq!(g.ret, INCORRECT_TYPE, "8.54 type={ty}: expected -26");
        assert!(
            g.out.iter().all(|&b| b == 0),
            "8.54 type={ty}: nothing must be written"
        );
    }

    // 8.53 the validation code is returned verbatim (spot check; the full table
    // is in e8_57_to_81_validate_inputs).
    let s = Spec { t_cost: 0, ..Spec::default() };
    assert_eq!(
        ctx_case(&c, &r, "8.53 t_cost=0 propagates", &s, T_ID).ret,
        TIME_TOO_SMALL
    );

    // 8.56 context == NULL -> ARGON2_INCORRECT_PARAMETER via validate_inputs.
    for ty in [T_I, T_ID, 0, 7] {
        let a = unsafe { c(null_mut(), ty) };
        let b = unsafe { r(null_mut(), ty) };
        eqi(&format!("8.56 argon2_ctx(NULL, {ty})"), a, b);
        assert_eq!(a, INCORRECT_PARAMETER, "8.56: expected -25");
    }
}

#[test]
fn e8_57_to_81_validate_inputs() {
    let (c, r) = both::<ValidateInputs>("_sodium_argon2_validate_inputs");
    let (cc, rc) = both::<Argon2Ctx>("_sodium_argon2_ctx");

    // 8.57 context == NULL.
    let a = unsafe { c(std::ptr::null()) };
    let b = unsafe { r(std::ptr::null()) };
    eqi("8.57 validate_inputs(NULL)", a, b);
    assert_eq!(a, INCORRECT_PARAMETER);

    // The baseline must validate.
    assert_eq!(
        validate_case(&c, &r, "baseline", &Spec::default()),
        ARGON2_OK
    );

    let cases: Vec<(&str, Spec, c_int)> = vec![
        // 8.58 out == NULL
        ("8.58 out=NULL", Spec { out_null: true, ..Spec::default() }, OUTPUT_PTR_NULL),
        // 8.59 outlen < ARGON2_MIN_OUTLEN (16)
        ("8.59 outlen=0", Spec { outlen: 0, out_cap: Some(1), ..Spec::default() }, OUTPUT_TOO_SHORT),
        ("8.59 outlen=1", Spec { outlen: 1, ..Spec::default() }, OUTPUT_TOO_SHORT),
        ("8.59 outlen=15", Spec { outlen: 15, ..Spec::default() }, OUTPUT_TOO_SHORT),
        // 8.61 pwd == NULL with pwdlen != 0
        ("8.61 pwd=NULL pwdlen=1", Spec { pwdlen: Some(1), ..Spec::default() }, PWD_PTR_MISMATCH),
        ("8.61 pwd=NULL pwdlen=MAX", Spec { pwdlen: Some(u32::MAX), ..Spec::default() }, PWD_PTR_MISMATCH),
        // 8.64 salt == NULL with saltlen != 0
        ("8.64 salt=NULL saltlen=8", Spec { salt: None, saltlen: Some(8), ..Spec::default() }, SALT_PTR_MISMATCH),
        // 8.65 saltlen < ARGON2_MIN_SALT_LENGTH (8)
        ("8.65 salt=NULL saltlen=0", Spec { salt: None, ..Spec::default() }, SALT_TOO_SHORT),
        ("8.65 saltlen=1", Spec { salt: Some(vec![1]), ..Spec::default() }, SALT_TOO_SHORT),
        ("8.65 saltlen=7", Spec { salt: Some(vec![1; 7]), ..Spec::default() }, SALT_TOO_SHORT),
        // 8.67 secret == NULL with secretlen != 0
        ("8.67 secret=NULL secretlen=1", Spec { secretlen: Some(1), ..Spec::default() }, SECRET_PTR_MISMATCH),
        // 8.70 ad == NULL with adlen != 0
        ("8.70 ad=NULL adlen=1", Spec { adlen: Some(1), ..Spec::default() }, AD_PTR_MISMATCH),
        // 8.73 lanes < ARGON2_MIN_LANES (1)
        ("8.73 lanes=0", Spec { lanes: 0, ..Spec::default() }, LANES_TOO_FEW),
        // 8.74 lanes > ARGON2_MAX_LANES (0xFFFFFF)
        ("8.74 lanes=0x1000000", Spec { lanes: 0x100_0000, ..Spec::default() }, LANES_TOO_MANY),
        ("8.74 lanes=MAX", Spec { lanes: u32::MAX, ..Spec::default() }, LANES_TOO_MANY),
        // 8.75 m_cost < ARGON2_MIN_MEMORY (8)
        ("8.75 m_cost=0", Spec { m_cost: 0, ..Spec::default() }, MEMORY_TOO_LITTLE),
        ("8.75 m_cost=1", Spec { m_cost: 1, ..Spec::default() }, MEMORY_TOO_LITTLE),
        ("8.75 m_cost=7", Spec { m_cost: 7, ..Spec::default() }, MEMORY_TOO_LITTLE),
        // 8.77 the second memory check: m_cost >= 8 but m_cost < 8 * lanes
        ("8.77 lanes=4 m_cost=31", Spec { lanes: 4, threads: 4, m_cost: 31, ..Spec::default() }, MEMORY_TOO_LITTLE),
        ("8.77 lanes=2 m_cost=15", Spec { lanes: 2, threads: 2, m_cost: 15, ..Spec::default() }, MEMORY_TOO_LITTLE),
        ("8.77 lanes=0xFFFFFF m_cost=8", Spec { lanes: 0xFF_FFFF, threads: 1, m_cost: 8, ..Spec::default() }, MEMORY_TOO_LITTLE),
        // 8.78 t_cost < ARGON2_MIN_TIME (1)
        ("8.78 t_cost=0", Spec { t_cost: 0, ..Spec::default() }, TIME_TOO_SMALL),
        // 8.80 threads < ARGON2_MIN_THREADS (1)
        ("8.80 threads=0", Spec { threads: 0, ..Spec::default() }, THREADS_TOO_FEW),
        // 8.81 threads > ARGON2_MAX_THREADS (0xFFFFFF)
        ("8.81 threads=0x1000000", Spec { threads: 0x100_0000, ..Spec::default() }, THREADS_TOO_MANY),
        ("8.81 threads=MAX", Spec { threads: u32::MAX, ..Spec::default() }, THREADS_TOO_MANY),
    ];

    for (label, s, want) in &cases {
        let got = validate_case(&c, &r, label, s);
        assert_eq!(got, *want, "{label}: expected {want}, got {got}");
        // argon2_ctx returns the validation code verbatim (errors row 8.53).
        let g = ctx_case(&cc, &rc, &format!("{label} (via argon2_ctx)"), s, T_ID);
        assert_eq!(g.ret, *want, "{label}: argon2_ctx must propagate {want}");
    }

    // Check ordering: `out == NULL` wins over every later problem.
    let s = Spec {
        out_null: true,
        outlen: 0,
        out_cap: Some(1),
        t_cost: 0,
        m_cost: 0,
        lanes: 0,
        threads: 0,
        salt: None,
        ..Spec::default()
    };
    assert_eq!(
        validate_case(&c, &r, "ordering: out=NULL first", &s),
        OUTPUT_PTR_NULL
    );
    // ... and lanes are checked before m_cost, m_cost before t_cost, t_cost
    // before threads.
    let s = Spec { lanes: 0, m_cost: 0, t_cost: 0, threads: 0, ..Spec::default() };
    assert_eq!(validate_case(&c, &r, "ordering: lanes first", &s), LANES_TOO_FEW);
    let s = Spec { m_cost: 0, t_cost: 0, threads: 0, ..Spec::default() };
    assert_eq!(validate_case(&c, &r, "ordering: m_cost", &s), MEMORY_TOO_LITTLE);
    let s = Spec { t_cost: 0, threads: 0, ..Spec::default() };
    assert_eq!(validate_case(&c, &r, "ordering: t_cost", &s), TIME_TOO_SMALL);

    // A non-NULL secret / ad with length 0 is legal (the minimum is 0).
    for s in [
        Spec { secret: Some(Vec::new()), ..Spec::default() },
        Spec { ad: Some(Vec::new()), ..Spec::default() },
        Spec { pwd: Some(Vec::new()), ..Spec::default() },
    ] {
        assert_eq!(validate_case(&c, &r, "non-NULL zero-length buffer", &s), ARGON2_OK);
    }
}

#[test]
fn e8_83_to_88_argon2_hash_rejections() {
    let _rng = rng_guard();
    let (c, r) = both::<Argon2Hash>("_sodium_argon2_hash");
    let pwd = b"password".to_vec();
    let salt = b"saltsalt".to_vec();

    // 8.83 pwdlen > ARGON2_MAX_PWD_LENGTH.  Reachable because `pwdlen` is a
    // size_t.  The check runs *after* randombytes_buf(hash, hashlen), so the
    // caller's buffer is already scrambled — hence the rng_reset() inside
    // hash_case, which makes the two libraries agree on those bytes.
    let (ret, h, _) = hash_case(
        &c, &r, "8.83 pwdlen=2^32", 1, 8, 1, Some(&pwd), Some(1usize << 32), Some(&salt), None,
        None, 16, None, T_I,
    );
    assert_eq!(ret, PWD_TOO_LONG, "8.83: expected -5");
    assert!(h.iter().any(|&b| b != 0), "8.83: `hash` is randomised before the check");

    // 8.84 hashlen > ARGON2_MAX_OUTLEN.  `hash` has to be NULL, otherwise the
    // preceding randombytes_buf(hash, hashlen) would write 4 GiB.
    for hashlen in [1usize << 32, usize::MAX] {
        rng_reset();
        let a = unsafe {
            c(
                1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), null_mut(), hashlen, null_mut(), 0, T_I,
            )
        };
        rng_reset();
        let b = unsafe {
            r(
                1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), null_mut(), hashlen, null_mut(), 0, T_I,
            )
        };
        eqi(&format!("8.84 hashlen={hashlen}"), a, b);
        assert_eq!(a, OUTPUT_TOO_LONG, "8.84: expected -3");
    }

    // 8.85 saltlen > ARGON2_MAX_SALT_LENGTH.
    for saltlen in [1usize << 32, usize::MAX] {
        let (ret, _, _) = hash_case(
            &c, &r, &format!("8.85 saltlen={saltlen}"), 1, 8, 1, Some(&pwd), None, Some(&salt),
            Some(saltlen), None, 16, None, T_I,
        );
        assert_eq!(ret, SALT_TOO_LONG, "8.85: expected -7");
    }

    // 8.87 any argon2_ctx failure is returned verbatim.
    let cases: &[(&str, u32, u32, u32, usize, usize, c_int)] = &[
        // label, t_cost, m_cost, parallelism, saltlen, hashlen, expected
        ("8.87 saltlen=4", 1, 8, 1, 4, 16, SALT_TOO_SHORT),
        ("8.87 saltlen=0", 1, 8, 1, 0, 16, SALT_TOO_SHORT),
        ("8.87 m_cost=4", 1, 4, 1, 8, 16, MEMORY_TOO_LITTLE),
        ("8.87 m_cost=0", 1, 0, 1, 8, 16, MEMORY_TOO_LITTLE),
        ("8.87 t_cost=0", 0, 8, 1, 8, 16, TIME_TOO_SMALL),
        ("8.87 parallelism=0", 1, 8, 0, 8, 16, LANES_TOO_FEW),
        ("8.87 parallelism too many", 1, 8, 0x100_0000, 8, 16, LANES_TOO_MANY),
        ("8.87 hashlen=8", 1, 8, 1, 8, 8, OUTPUT_TOO_SHORT),
        ("8.87 hashlen=0", 1, 8, 1, 8, 0, OUTPUT_TOO_SHORT),
        ("8.87 m_cost < 8*parallelism", 1, 8, 2, 8, 16, MEMORY_TOO_LITTLE),
    ];
    for &(label, t, m, par, saltlen, hashlen, want) in cases {
        let s: Vec<u8> = (0..saltlen).map(|i| i as u8).collect();
        let (ret, _, _) = hash_case(
            &c, &r, label, t, m, par, Some(&pwd), None, Some(&s), None, Some(hashlen),
            hashlen.max(1), None, T_I,
        );
        assert_eq!(ret, want, "{label}: expected {want}, got {ret}");
    }

    // 8.88 / 8.89 the encoded buffer is too small -> ARGON2_ENCODING_FAIL, and
    // both `hash` and `encoded` are zeroed.
    //
    // NOTE: this only holds while the *fixed* part of the string does not fit.
    // "$argon2i$v=19$m=8,t=1,p=1$" is 26 characters, so `encodedlen <= 26`
    // fails inside an `SS`/`SX` step and returns -31.  From 27 onwards the
    // failure moves into `sodium_bin2base64`, which calls `sodium_misuse()` and
    // *aborts* instead of returning NULL — see e8_148_encoding_buffer_aborts
    // (errors row 8.148 predicts NULL; the C code is authoritative).
    for encodedlen in [1usize, 2, 10, 12, 20, 26] {
        let run = |f: &Argon2Hash| -> (c_int, Vec<u8>, Vec<u8>) {
            let mut hb = padded(16);
            let mut eb = padded(encodedlen);
            for i in 0..encodedlen {
                eb[i] = 0xC3;
            }
            rng_reset();
            let ret = unsafe {
                f(
                    1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                    salt.as_ptr() as *const c_void, salt.len(), hb.as_mut_ptr() as *mut c_void, 16,
                    eb.as_mut_ptr() as *mut c_char, encodedlen, T_I,
                )
            };
            check_pad("8.88 hash", &hb, 16);
            check_pad("8.88 encoded", &eb, encodedlen);
            (ret, hb[..16].to_vec(), eb[..encodedlen].to_vec())
        };
        let (a, ah, ae) = run(&c);
        let (b, bh, be) = run(&r);
        eqi(&format!("8.88 encodedlen={encodedlen}"), a, b);
        assert_eq!(a, ENCODING_FAIL, "8.88 encodedlen={encodedlen}: expected -31");
        eqb(&format!("8.88 hash encodedlen={encodedlen}"), &ah, &bh);
        eqb(&format!("8.88 encoded encodedlen={encodedlen}"), &ae, &be);
        assert!(
            ae.iter().all(|&v| v == 0),
            "8.88: `encoded` must be zeroed on ENCODING_FAIL"
        );
    }
    // 8.89 the same through argon2i_hash_encoded / argon2id_hash_encoded.
    for (name, ty) in [
        ("_sodium_argon2i_hash_encoded", T_I),
        ("_sodium_argon2id_hash_encoded", T_ID),
    ] {
        let (ce, re) = both::<HashEncoded>(name);
        for encodedlen in [1usize, 10, 26] {
            let mut a = padded(encodedlen);
            let mut b = padded(encodedlen);
            rng_reset();
            let x = unsafe {
                ce(
                    1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                    salt.as_ptr() as *const c_void, salt.len(), 32,
                    a.as_mut_ptr() as *mut c_char, encodedlen,
                )
            };
            rng_reset();
            let y = unsafe {
                re(
                    1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                    salt.as_ptr() as *const c_void, salt.len(), 32,
                    b.as_mut_ptr() as *mut c_char, encodedlen,
                )
            };
            eqi(&format!("8.89 {name} encodedlen={encodedlen}"), x, y);
            assert_eq!(x, ENCODING_FAIL, "8.89 {name}: expected -31");
            eqb(&format!("8.89 {name} buf"), &a[..encodedlen], &b[..encodedlen]);
            let _ = ty;
        }
    }
}

#[test]
fn e8_90_to_94_hash_raw_rejections() {
    let _rng = rng_guard();
    let pwd = b"password".to_vec();
    for name in ["_sodium_argon2i_hash_raw", "_sodium_argon2id_hash_raw"] {
        let (c, r) = both::<HashRaw>(name);
        let cases: &[(&str, u32, u32, u32, usize, usize, c_int)] = &[
            // 8.90 hashlen < 16
            ("8.90 hashlen=8", 1, 8, 1, 8, 8, OUTPUT_TOO_SHORT),
            ("8.90 hashlen=15", 1, 8, 1, 8, 15, OUTPUT_TOO_SHORT),
            // 8.91 saltlen < 8
            ("8.91 saltlen=4", 1, 8, 1, 4, 16, SALT_TOO_SHORT),
            ("8.91 saltlen=7", 1, 8, 1, 7, 16, SALT_TOO_SHORT),
            // 8.92 parallelism = 0
            ("8.92 parallelism=0", 1, 8, 0, 8, 16, LANES_TOO_FEW),
            // 8.93 t_cost = 0
            ("8.93 t_cost=0", 0, 8, 1, 8, 16, TIME_TOO_SMALL),
            // 8.94 m_cost < 8, and m_cost < 8 * parallelism
            ("8.94 m_cost=7", 1, 7, 1, 8, 16, MEMORY_TOO_LITTLE),
            ("8.94 m_cost=8 parallelism=2", 1, 8, 2, 8, 16, MEMORY_TOO_LITTLE),
            ("8.94 m_cost=31 parallelism=4", 1, 31, 4, 8, 16, MEMORY_TOO_LITTLE),
        ];
        for &(label, t, m, par, saltlen, hashlen, want) in cases {
            let salt: Vec<u8> = (0..saltlen).map(|i| i as u8).collect();
            let mut a = padded(hashlen.max(1));
            let mut b = padded(hashlen.max(1));
            rng_reset();
            let x = unsafe {
                c(
                    t, m, par, pwd.as_ptr() as *const c_void, pwd.len(),
                    salt.as_ptr() as *const c_void, saltlen, a.as_mut_ptr() as *mut c_void, hashlen,
                )
            };
            rng_reset();
            let y = unsafe {
                r(
                    t, m, par, pwd.as_ptr() as *const c_void, pwd.len(),
                    salt.as_ptr() as *const c_void, saltlen, b.as_mut_ptr() as *mut c_void, hashlen,
                )
            };
            eqi(&format!("{name} {label}"), x, y);
            assert_eq!(x, want, "{name} {label}: expected {want}, got {x}");
            eqb(&format!("{name} {label} buf"), &a[..hashlen.max(1)], &b[..hashlen.max(1)]);
        }
    }
}

#[test]
fn e8_97_98_verify_rejections() {
    let _rng = rng_guard();
    let (cv, rv) = both::<VerifyT>("_sodium_argon2_verify");
    let (cde, _rde) = both::<HashEncoded>("_sodium_argon2id_hash_encoded");
    let pwd = b"password".to_vec();
    let salt = b"0123456789abcdef".to_vec();

    // Build a good argon2id string.
    let mut good = padded(128);
    rng_reset();
    assert_eq!(
        unsafe {
            cde(
                1, 8, 1, pwd.as_ptr() as *const c_void, pwd.len(),
                salt.as_ptr() as *const c_void, salt.len(), 32,
                good.as_mut_ptr() as *mut c_char, 128,
            )
        },
        ARGON2_OK
    );
    let good_s = as_str(&good[..128]);

    // 8.97 the decode result is returned verbatim.
    // The `argon2_validate_inputs` call at the end of argon2_decode_string
    // checks `outlen` first, then salt, then lanes, then m_cost, then t_cost —
    // so every string below keeps the *earlier* fields legal.
    let good_salt = "QUJDREVGR0hJSktMTU5PUA"; // base64 of 16 bytes
    let good_hash = "YWJjZGVmZ2hpamtsbW5vcA"; // base64 of 16 bytes
    let bad: Vec<(String, c_int)> = vec![
        (String::new(), -32),
        ("$argon2id$".to_string(), -32),
        ("$argon2id$v=19$".to_string(), -32),
        ("$argon2id$v=19$m=8,t=1,p=1$".to_string(), -32),
        (format!("$argon2id$v=20$m=8,t=1,p=1${good_salt}${good_hash}"), -26),
        (format!("$argon2id$v=19$m=0,t=1,p=1${good_salt}${good_hash}"), -14),
        (format!("$argon2id$v=19$m=8,t=0,p=1${good_salt}${good_hash}"), -12),
        (format!("$argon2id$v=19$m=8,t=1,p=0${good_salt}${good_hash}"), -16),
        (format!("$argon2id$v=19$m=8,t=1,p=2${good_salt}${good_hash}"), -14),
        (format!("$argon2id$v=19$m=8,t=1,p=1$QUJDREVGRw${good_hash}"), -6),
        (format!("$argon2id$v=19$m=8,t=1,p=1${good_salt}$YWJjZGVmZ2hpamtsbW5v"), -2),
        (format!("$argon2id$v=19$m=8,t=1,p=1${good_salt}${good_hash}x"), -32),
        (format!("$argon2i$v=19$m=8,t=1,p=1${good_salt}${good_hash}"), -32),
    ];
    for (s, want) in bad.iter().map(|(a, b)| (a.as_str(), *b)) {
        let z = cstr(s);
        let a = unsafe {
            cv(z.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, pwd.len(), T_ID)
        };
        let b = unsafe {
            rv(z.as_ptr() as *const c_char, pwd.as_ptr() as *const c_void, pwd.len(), T_ID)
        };
        eqi(&format!("8.97 argon2_verify({s:?})"), a, b);
        assert_eq!(a, want, "8.97 {s:?}: expected {want}, got {a}");
    }

    // 8.98 a wrong password -> ARGON2_VERIFY_MISMATCH (-35).
    for wrong in [b"".to_vec(), b"Password".to_vec(), b"password\0".to_vec()] {
        let z = cstr(&good_s);
        let a = unsafe {
            cv(z.as_ptr() as *const c_char, wrong.as_ptr() as *const c_void, wrong.len(), T_ID)
        };
        let b = unsafe {
            rv(z.as_ptr() as *const c_char, wrong.as_ptr() as *const c_void, wrong.len(), T_ID)
        };
        eqi("8.98 wrong password", a, b);
        assert_eq!(a, VERIFY_MISMATCH, "8.98: expected -35");
    }
}

#[test]
fn e8_101_110_111_null_arguments() {
    let _rng = rng_guard();
    // 8.101 argon2_initialize with a NULL instance or context.
    let (ci, ri) = both::<Initialize>("_sodium_argon2_initialize");
    let mut l = Live::new(&Spec::default());
    let mut inst = Instance {
        region: null_mut(),
        pseudo_rands: null_mut(),
        passes: 1,
        current_pass: !0,
        memory_blocks: 8,
        segment_length: 2,
        lane_length: 8,
        lanes: 1,
        threads: 1,
        ty: T_ID,
        print_internals: 0,
    };
    let a = unsafe { ci(null_mut(), &mut l.ctx) };
    let b = unsafe { ri(null_mut(), &mut l.ctx) };
    eqi("8.101 argon2_initialize(NULL, ctx)", a, b);
    assert_eq!(a, INCORRECT_PARAMETER, "8.101: expected -25");
    let a = unsafe { ci(&mut inst, null_mut()) };
    let b = unsafe { ri(&mut inst, null_mut()) };
    eqi("8.101 argon2_initialize(inst, NULL)", a, b);
    assert_eq!(a, INCORRECT_PARAMETER, "8.101: expected -25");

    // 8.110 argon2_fill_memory_blocks(NULL, pass) and instance->lanes == 0
    // return early; the function is `void`, so the only observable behaviour is
    // that neither library crashes and nothing is written.
    let (cf, rf) = both::<FillMemoryBlocks>("_sodium_argon2_fill_memory_blocks");
    unsafe {
        cf(null_mut(), 0);
        rf(null_mut(), 0);
        let mut zero_lanes = inst;
        zero_lanes.lanes = 0;
        cf(&mut zero_lanes, 0);
        let mut zero_lanes_r = inst;
        zero_lanes_r.lanes = 0;
        rf(&mut zero_lanes_r, 0);
    }

    // 8.111 argon2_fill_segment_ref(NULL, position) returns early.
    let (cs, rs) = both::<FillSegment>("_sodium_argon2_fill_segment_ref");
    let pos = Position { pass: 0, lane: 0, slice: 0, index: 0 };
    unsafe {
        cs(std::ptr::null(), pos);
        rs(std::ptr::null(), pos);
    }
    // Reaching this line without a fault is the assertion.
    let _ = NonNull::<u8>::dangling();
}

#[test]
fn e8_107_108_blake2b_long_rejections() {
    let (c, r) = both::<Blake2bLong>("_sodium_blake2b_long");
    let input = vec![0x5au8; 64];

    // 8.107 outlen > UINT32_MAX -> -1, with no write (the check is first).
    for outlen in [1usize << 32, usize::MAX] {
        let mut a = padded(64);
        let mut b = padded(64);
        let x = unsafe {
            c(a.as_mut_ptr() as *mut c_void, outlen, input.as_ptr() as *const c_void, input.len())
        };
        let y = unsafe {
            r(b.as_mut_ptr() as *mut c_void, outlen, input.as_ptr() as *const c_void, input.len())
        };
        eqi(&format!("8.107 blake2b_long(outlen={outlen})"), x, y);
        assert_eq!(x, -1, "8.107: expected -1");
        eqb("8.107 buffer untouched", &a[..64], &b[..64]);
        assert!(a[..64].iter().all(|&v| v == 0));
    }

    // 8.108 outlen == 0 is rejected by crypto_generichash_blake2b_init.
    let mut a = padded(64);
    let mut b = padded(64);
    let x = unsafe {
        c(a.as_mut_ptr() as *mut c_void, 0, input.as_ptr() as *const c_void, input.len())
    };
    let y = unsafe {
        r(b.as_mut_ptr() as *mut c_void, 0, input.as_ptr() as *const c_void, input.len())
    };
    eqi("8.108 blake2b_long(outlen=0)", x, y);
    assert_eq!(x, -1, "8.108: expected -1");
    eqb("8.108 buffer untouched", &a[..64], &b[..64]);
}

#[test]
fn e8_148_encoding_buffer_aborts() {
    // errors_8.md row 8.148 expects `sodium_bin2base64` to return NULL when
    // `dst_len` is too small, which would give ARGON2_ENCODING_FAIL.  In
    // libsodium 1.0.23 `sodium_bin2base64` instead calls `sodium_misuse()`, so
    // the whole process aborts.  Both libraries must abort identically.
    //
    // For argon2i with saltlen 8 / hashlen 16 the thresholds are:
    //   encodedlen <= 26  -> ARGON2_ENCODING_FAIL (checked in e8_83_to_88)
    //   27 ..= 37         -> abort inside SB(salt, 8)
    //   38 ..= 60         -> abort inside SB(out, 16)
    //   >= 61             -> success (the string is 60 chars + NUL)
    let (c, r) = both::<Argon2Hash>("_sodium_argon2_hash");
    for encodedlen in [27usize, 30, 37, 38, 45, 60] {
        let cc = c.clone();
        let rr = r.clone();
        eq_abort(
            &format!("8.148 argon2_hash(encodedlen={encodedlen})"),
            move || unsafe {
                let mut hb = [0u8; 16];
                let mut eb = vec![0u8; encodedlen];
                cc(
                    1, 8, 1, b"password".as_ptr() as *const c_void, 8,
                    b"saltsalt".as_ptr() as *const c_void, 8, hb.as_mut_ptr() as *mut c_void, 16,
                    eb.as_mut_ptr() as *mut c_char, encodedlen, T_I,
                );
            },
            move || unsafe {
                let mut hb = [0u8; 16];
                let mut eb = vec![0u8; encodedlen];
                rr(
                    1, 8, 1, b"password".as_ptr() as *const c_void, 8,
                    b"saltsalt".as_ptr() as *const c_void, 8, hb.as_mut_ptr() as *mut c_void, 16,
                    eb.as_mut_ptr() as *mut c_char, encodedlen, T_I,
                );
            },
        );
    }
    // 61 is the tightest accepting size (60 characters plus the NUL).
    let mut a = padded(61);
    let mut b = padded(61);
    rng_reset();
    let x = unsafe {
        c(
            1, 8, 1, b"password".as_ptr() as *const c_void, 8,
            b"saltsalt".as_ptr() as *const c_void, 8, null_mut(), 16,
            a.as_mut_ptr() as *mut c_char, 61, T_I,
        )
    };
    rng_reset();
    let y = unsafe {
        r(
            1, 8, 1, b"password".as_ptr() as *const c_void, 8,
            b"saltsalt".as_ptr() as *const c_void, 8, null_mut(), 16,
            b.as_mut_ptr() as *mut c_char, 61, T_I,
        )
    };
    eqi("8.148 encodedlen=61", x, y);
    assert_eq!(x, ARGON2_OK, "8.148: encodedlen 61 must be accepted");
    eqb("8.148 encodedlen=61 string", &a[..61], &b[..61]);
    assert_eq!(as_str(&a[..61]).len(), 60);
}
