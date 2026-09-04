//! Differential tests for AREA `mac`:
//!   * `crypto_onetimeauth/crypto_onetimeauth.c`
//!   * `crypto_onetimeauth/poly1305/onetimeauth_poly1305.c`
//!   * `crypto_onetimeauth/poly1305/donna/poly1305_donna.c` (+ `poly1305_donna32.h`,
//!     the branch actually compiled because `HAVE_TI_MODE` is undefined)
//!   * `crypto_auth/crypto_auth.c`
//!   * `crypto_auth/hmacsha256/auth_hmacsha256.c`
//!   * `crypto_auth/hmacsha512/auth_hmacsha512.c`
//!   * `crypto_auth/hmacsha512256/auth_hmacsha512256.c`
//!
//! Everything is called through `dlopen`ed C and Rust shared objects.

#![allow(dead_code)]

#[macro_use]
mod common;

use core::ffi::{c_char, c_int};

// ----------------------------------------------------------------- fn types --

type OneShotFn = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type VerifyFn = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;
type OtaInitFn = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type HmacInitFn = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type UpdateFn = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type FinalFn = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type KeygenFn = unsafe extern "C" fn(*mut u8);
type IntFn = unsafe extern "C" fn() -> c_int;

/// `crypto_onetimeauth_poly1305_implementation` — the exported vtable struct.
/// The C fields are plain (non-nullable in practice) function pointers.
#[repr(C)]
#[derive(Clone, Copy)]
struct Poly1305Impl {
    onetimeauth: OneShotFn,
    onetimeauth_verify: VerifyFn,
    onetimeauth_init: OtaInitFn,
    onetimeauth_update: UpdateFn,
    onetimeauth_final: FinalFn,
}

// ------------------------------------------------------------------ helpers --

/// 64-byte-aligned scratch buffer for opaque `*_state` objects.
/// (`crypto_onetimeauth_poly1305_state` is `CRYPTO_ALIGN(16)`; the donna code
/// additionally uses `CRYPTO_ALIGN(64)` for its own stack copy.)
#[repr(align(64))]
struct Aligned([u8; 512]);

impl Aligned {
    fn filled(b: u8) -> Self {
        Aligned([b; 512])
    }
}

fn cstr(p: *const c_char) -> String {
    assert!(!p.is_null(), "primitive() returned NULL");
    unsafe { std::ffi::CStr::from_ptr(p) }
        .to_string_lossy()
        .into_owned()
}

/// Compare a `size_t`-returning getter across both libraries and against the
/// value the C headers mandate.
macro_rules! chk_size {
    ($name:expr, $expected:expr) => {{
        let (c, r) = both!($name, SizeFn);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, rv, concat!($name, ": C/Rust mismatch"));
        assert_eq!(cv, $expected as usize, concat!($name, ": unexpected value"));
        cv
    }};
}

macro_rules! chk_str {
    ($name:expr, $expected:expr) => {{
        let (c, r) = both!($name, StrFn);
        let (cv, rv) = unsafe { (cstr(c()), cstr(r())) };
        assert_eq!(cv, rv, concat!($name, ": C/Rust mismatch"));
        assert_eq!(cv, $expected, concat!($name, ": unexpected value"));
    }};
}

/// Random chunking plan for a message of `n` bytes.  Deliberately produces
/// 0-length chunks, 1-byte chunks and chunk sizes around the 16-byte poly1305
/// block buffer / 64-and-128-byte sha block boundaries.
fn split_plan(rng: &mut common::Rng, n: usize) -> Vec<usize> {
    let mut v = Vec::new();
    let mut rem = n;
    while rem > 0 {
        if rng.below(5) == 0 {
            v.push(0); // zero-length update in the middle of the stream
        }
        let c = match rng.below(6) {
            0 => 1,
            1 => rng.below(rem) + 1,
            2 => 15,
            3 => 16,
            4 => 17,
            _ => 64,
        };
        let c = if c > rem { rem } else { c };
        v.push(c);
        rem -= c;
    }
    if v.is_empty() || rng.below(2) == 0 {
        v.push(0); // leading/trailing 0-length update (covers n == 0)
    }
    v
}

/// Explicit plans that straddle the 16-byte poly1305 buffer in every
/// interesting way.  Each entry is (total, chunks).
fn straddling_plans() -> Vec<(usize, Vec<usize>)> {
    let raw: Vec<Vec<usize>> = vec![
        vec![0],
        vec![0, 0, 0],
        vec![1],
        vec![1, 1],
        vec![15, 1],
        vec![1, 15],
        vec![15, 2],
        vec![8, 8],
        vec![8, 9],
        vec![16, 1],
        vec![1, 16],
        vec![17, 15],
        vec![16, 16, 1],
        vec![3, 0, 13, 0, 1],
        vec![31, 1, 1],
        vec![5, 5, 5, 5, 5],
        vec![32, 33],
        vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        vec![64, 63, 1],
        vec![100, 0, 28],
        vec![7, 9, 16, 0, 1, 15],
    ];
    raw.into_iter().map(|p| (p.iter().sum(), p)).collect()
}

const LENS: [usize; 10] = [0, 1, 15, 16, 17, 31, 32, 33, 64, 1000];
/// Lengths that straddle the sha-256 (64) and sha-512 (128) block boundaries as
/// well as their length-padding boundaries (55/56 and 111/112).
const SHA_LENS: [usize; 17] = [
    0, 1, 31, 32, 55, 56, 63, 64, 65, 111, 112, 119, 120, 127, 128, 129, 1000,
];

// =============================================================== poly1305 ====

#[test]
fn mac1_sizes_and_primitives() {
    // sizeof(crypto_onetimeauth_poly1305_state) == sizeof(opaque[256]) == 256
    chk_size!("crypto_onetimeauth_poly1305_statebytes", 256);
    chk_size!("crypto_onetimeauth_poly1305_bytes", 16);
    chk_size!("crypto_onetimeauth_poly1305_keybytes", 32);
    chk_size!("crypto_onetimeauth_statebytes", 256);
    chk_size!("crypto_onetimeauth_bytes", 16);
    chk_size!("crypto_onetimeauth_keybytes", 32);
    chk_str!("crypto_onetimeauth_primitive", "poly1305");

    // crypto_auth_hmacsha256_state = 2 * crypto_hash_sha256_state (104 bytes)
    chk_size!("crypto_auth_hmacsha256_statebytes", 208);
    chk_size!("crypto_auth_hmacsha256_bytes", 32);
    chk_size!("crypto_auth_hmacsha256_keybytes", 32);
    // crypto_auth_hmacsha512_state = 2 * crypto_hash_sha512_state (208 bytes)
    chk_size!("crypto_auth_hmacsha512_statebytes", 416);
    chk_size!("crypto_auth_hmacsha512_bytes", 64);
    chk_size!("crypto_auth_hmacsha512_keybytes", 32);
    chk_size!("crypto_auth_hmacsha512256_statebytes", 416);
    chk_size!("crypto_auth_hmacsha512256_bytes", 32);
    chk_size!("crypto_auth_hmacsha512256_keybytes", 32);
    chk_size!("crypto_auth_bytes", 32);
    chk_size!("crypto_auth_keybytes", 32);
    chk_str!("crypto_auth_primitive", "hmacsha512256");
}

/// One-shot `crypto_onetimeauth_poly1305` / `crypto_onetimeauth` over the
/// mandated length set plus random lengths, with a canary-guarded output buffer.
#[test]
fn mac2_poly1305_oneshot() {
    let (c1, r1) = both!("crypto_onetimeauth_poly1305", OneShotFn);
    let (cg, rg) = both!("crypto_onetimeauth", OneShotFn);
    let mut rng = common::Rng::new(0x0001_1305);

    let mut lens: Vec<usize> = LENS.to_vec();
    for _ in 0..24 {
        lens.push(rng.below(600));
    }

    for n in lens {
        for _ in 0..3 {
            let msg = rng.bytes(n);
            let key = rng.bytes(32);
            // 24-byte output buffers with a canary so over/under-writes show up.
            let (mut co, mut ro) = ([0xA5u8; 24], [0xA5u8; 24]);
            let rc = unsafe { c1(co.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            let rr = unsafe { r1(ro.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            common::eqi(&format!("poly1305 oneshot rc n={n}"), rc, rr);
            assert_eq!(rc, 0, "poly1305 oneshot must return 0");
            common::eqb(&format!("poly1305 oneshot n={n}"), &co, &ro);
            assert_eq!(&co[16..], &[0xA5u8; 8], "poly1305 wrote past 16 bytes");

            // generic dispatcher must give exactly the same tag
            let (mut cgo, mut rgo) = ([0xA5u8; 24], [0xA5u8; 24]);
            let rc2 = unsafe { cg(cgo.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            let rr2 = unsafe { rg(rgo.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            common::eqi(&format!("onetimeauth oneshot rc n={n}"), rc2, rr2);
            common::eqb(&format!("onetimeauth oneshot n={n}"), &cgo, &rgo);
            common::eqb(&format!("onetimeauth==poly1305 n={n}"), &cgo, &co);
        }
    }

    // in == NULL with inlen == 0 (the header only marks out/k as nonnull)
    let key = rng.bytes(32);
    let (mut co, mut ro) = ([0xA5u8; 24], [0xA5u8; 24]);
    let rc = unsafe { c1(co.as_mut_ptr(), core::ptr::null(), 0, key.as_ptr()) };
    let rr = unsafe { r1(ro.as_mut_ptr(), core::ptr::null(), 0, key.as_ptr()) };
    common::eqi("poly1305 oneshot in=NULL rc", rc, rr);
    common::eqb("poly1305 oneshot in=NULL", &co, &ro);
}

/// Streaming poly1305: `init` / n × `update` / `final`, with the FULL 256-byte
/// opaque state buffer compared after `init` and after every `update`.
///
/// The state is a `unsigned char opaque[256]` in which only the first
/// `sizeof(poly1305_state_internal_t)` == 144 bytes are ever touched, and only
/// the first 137 of those carry data (7 trailing bytes are struct padding after
/// `unsigned char final`).  Because both buffers start from the same canary and
/// neither library writes padding, a full 256-byte comparison is valid and is
/// what we do — it also proves the `sodium_memzero(st, sizeof *st)` in
/// `poly1305_finish` clears exactly the same range in both.
#[test]
fn mac3_poly1305_streaming_state_layout() {
    let (ci, ri) = both!("crypto_onetimeauth_poly1305_init", OtaInitFn);
    let (cu, ru) = both!("crypto_onetimeauth_poly1305_update", UpdateFn);
    let (cf, rf) = both!("crypto_onetimeauth_poly1305_final", FinalFn);
    let (c1, r1) = both!("crypto_onetimeauth_poly1305", OneShotFn);
    let sb = chk_size!("crypto_onetimeauth_poly1305_statebytes", 256);

    let mut rng = common::Rng::new(0xBEEF_1305);

    let mut plans: Vec<(usize, Vec<usize>)> = straddling_plans();
    for n in LENS {
        for _ in 0..6 {
            plans.push((n, split_plan(&mut rng, n)));
        }
    }
    for _ in 0..40 {
        let n = rng.below(400);
        plans.push((n, split_plan(&mut rng, n)));
    }

    for (n, plan) in plans {
        let msg = rng.bytes(n);
        let key = rng.bytes(32);
        let mut cs = Aligned::filled(0x5A);
        let mut rs = Aligned::filled(0x5A);
        let tag = format!("poly1305 stream n={n} plan={plan:?}");

        let rc = unsafe { ci(cs.0.as_mut_ptr(), key.as_ptr()) };
        let rr = unsafe { ri(rs.0.as_mut_ptr(), key.as_ptr()) };
        common::eqi(&format!("{tag} init rc"), rc, rr);
        assert_eq!(rc, 0);
        common::eqb(&format!("{tag} state after init"), &cs.0[..sb], &rs.0[..sb]);
        // nothing beyond statebytes may be touched
        assert!(cs.0[sb..].iter().all(|&b| b == 0x5A));
        assert!(rs.0[sb..].iter().all(|&b| b == 0x5A));

        let mut off = 0usize;
        for (i, &len) in plan.iter().enumerate() {
            let p = if len == 0 && off == n {
                // exercise a genuinely NULL in-pointer for a 0-length update
                core::ptr::null()
            } else {
                unsafe { msg.as_ptr().add(off) }
            };
            let rc = unsafe { cu(cs.0.as_mut_ptr(), p, len as u64) };
            let rr = unsafe { ru(rs.0.as_mut_ptr(), p, len as u64) };
            common::eqi(&format!("{tag} update#{i} rc"), rc, rr);
            assert_eq!(rc, 0);
            common::eqb(
                &format!("{tag} state after update#{i} len={len}"),
                &cs.0[..sb],
                &rs.0[..sb],
            );
            off += len;
        }
        assert_eq!(off, n, "plan does not cover the message");

        let (mut co, mut ro) = ([0xA5u8; 24], [0xA5u8; 24]);
        let rc = unsafe { cf(cs.0.as_mut_ptr(), co.as_mut_ptr()) };
        let rr = unsafe { rf(rs.0.as_mut_ptr(), ro.as_mut_ptr()) };
        common::eqi(&format!("{tag} final rc"), rc, rr);
        assert_eq!(rc, 0);
        common::eqb(&format!("{tag} tag"), &co, &ro);
        assert_eq!(&co[16..], &[0xA5u8; 8], "final wrote past 16 bytes");
        // poly1305_finish() memzero's exactly sizeof(poly1305_state_internal_t)
        common::eqb(&format!("{tag} state after final"), &cs.0[..], &rs.0[..]);
        assert!(
            cs.0[..144].iter().all(|&b| b == 0),
            "C did not zero the internal state"
        );
        assert!(
            cs.0[144..].iter().all(|&b| b == 0x5A),
            "C zeroed more than sizeof(poly1305_state_internal_t)"
        );

        // streaming must equal one-shot
        let (mut c1o, mut r1o) = ([0u8; 16], [0u8; 16]);
        unsafe {
            c1(c1o.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr());
            r1(r1o.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr());
        }
        common::eqb(&format!("{tag} oneshot==stream (C)"), &c1o, &co[..16]);
        common::eqb(&format!("{tag} oneshot==stream (Rust)"), &r1o, &ro[..16]);

        // calling final again on the zeroed state is fully deterministic:
        // leftover/h/pad are all zero, so both libraries must agree.
        let (mut co2, mut ro2) = ([0xA5u8; 24], [0xA5u8; 24]);
        let rc = unsafe { cf(cs.0.as_mut_ptr(), co2.as_mut_ptr()) };
        let rr = unsafe { rf(rs.0.as_mut_ptr(), ro2.as_mut_ptr()) };
        common::eqi(&format!("{tag} final#2 rc"), rc, rr);
        common::eqb(&format!("{tag} final#2 tag"), &co2, &ro2);
    }
}

/// The generic `crypto_onetimeauth_{init,update,final}` dispatchers on the
/// `crypto_onetimeauth_state` typedef.
#[test]
fn mac4_onetimeauth_generic_streaming() {
    let (ci, ri) = both!("crypto_onetimeauth_init", OtaInitFn);
    let (cu, ru) = both!("crypto_onetimeauth_update", UpdateFn);
    let (cf, rf) = both!("crypto_onetimeauth_final", FinalFn);
    let (cv, rv) = both!("crypto_onetimeauth_verify", VerifyFn);
    let sb = chk_size!("crypto_onetimeauth_statebytes", 256);
    let mut rng = common::Rng::new(0x0007_1305);

    for n in LENS {
        for _ in 0..4 {
            let plan = split_plan(&mut rng, n);
            let msg = rng.bytes(n);
            let key = rng.bytes(32);
            let mut cs = Aligned::filled(0x33);
            let mut rs = Aligned::filled(0x33);
            let tag = format!("onetimeauth stream n={n}");

            common::eqi(
                &format!("{tag} init"),
                unsafe { ci(cs.0.as_mut_ptr(), key.as_ptr()) },
                unsafe { ri(rs.0.as_mut_ptr(), key.as_ptr()) },
            );
            common::eqb(&format!("{tag} state/init"), &cs.0[..sb], &rs.0[..sb]);
            let mut off = 0;
            for &len in &plan {
                let p = unsafe { msg.as_ptr().add(off) };
                common::eqi(
                    &format!("{tag} update"),
                    unsafe { cu(cs.0.as_mut_ptr(), p, len as u64) },
                    unsafe { ru(rs.0.as_mut_ptr(), p, len as u64) },
                );
                common::eqb(&format!("{tag} state/update"), &cs.0[..sb], &rs.0[..sb]);
                off += len;
            }
            let (mut co, mut ro) = ([0xA5u8; 24], [0xA5u8; 24]);
            common::eqi(
                &format!("{tag} final"),
                unsafe { cf(cs.0.as_mut_ptr(), co.as_mut_ptr()) },
                unsafe { rf(rs.0.as_mut_ptr(), ro.as_mut_ptr()) },
            );
            common::eqb(&format!("{tag} tag"), &co, &ro);

            // verify accepts the streamed tag through the generic entry point
            common::eqi(
                &format!("{tag} verify ok"),
                unsafe { cv(co.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) },
                unsafe { rv(ro.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) },
            );
            assert_eq!(
                unsafe { cv(co.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) },
                0
            );
        }
    }
}

/// `crypto_onetimeauth_poly1305_verify` / `crypto_onetimeauth_verify`:
/// correct tag → 0; every single bit of the tag flipped → -1; altered and
/// truncated (zero-extended) keys → -1.
#[test]
fn mac5_poly1305_verify() {
    let (c1, r1) = both!("crypto_onetimeauth_poly1305", OneShotFn);
    let (cv, rv) = both!("crypto_onetimeauth_poly1305_verify", VerifyFn);
    let (cgv, rgv) = both!("crypto_onetimeauth_verify", VerifyFn);
    let mut rng = common::Rng::new(0x0EF1_1305);

    for n in LENS {
        let msg = rng.bytes(n);
        let key = rng.bytes(32);
        let mut good = [0u8; 16];
        unsafe { c1(good.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };

        // correct tag
        let rc = unsafe { cv(good.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
        let rr = unsafe { rv(good.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
        common::eqi(&format!("poly1305_verify ok n={n}"), rc, rr);
        assert_eq!(rc, 0, "correct tag must verify with 0");
        let rc = unsafe { cgv(good.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
        let rr = unsafe { rgv(good.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
        common::eqi(&format!("onetimeauth_verify ok n={n}"), rc, rr);
        assert_eq!(rc, 0);

        // every bit of the 16-byte tag flipped -> -1
        for bit in 0..128usize {
            let mut bad = good;
            bad[bit / 8] ^= 1u8 << (bit % 8);
            let rc = unsafe { cv(bad.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            let rr = unsafe { rv(bad.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            common::eqi(&format!("poly1305_verify bad bit={bit} n={n}"), rc, rr);
            assert_eq!(rc, -1, "flipped tag bit {bit} must return -1");
            let rc = unsafe { cgv(bad.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            let rr = unsafe { rgv(bad.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            common::eqi(&format!("onetimeauth_verify bad bit={bit} n={n}"), rc, rr);
        }

        // altered key bytes: every byte of r (0..16) and of pad (16..32)
        for i in 0..32usize {
            let mut k2 = key.clone();
            k2[i] ^= 0x80;
            let rc = unsafe { cv(good.as_ptr(), msg.as_ptr(), n as u64, k2.as_ptr()) };
            let rr = unsafe { rv(good.as_ptr(), msg.as_ptr(), n as u64, k2.as_ptr()) };
            common::eqi(&format!("poly1305_verify altkey i={i} n={n}"), rc, rr);
            // r is masked, so a few high bits of key[0..16] are ignored; only
            // require the two libraries to agree, and cross-check with the tag.
            let mut t2 = [0u8; 16];
            unsafe { c1(t2.as_mut_ptr(), msg.as_ptr(), n as u64, k2.as_ptr()) };
            let expect = if t2 == good { 0 } else { -1 };
            assert_eq!(rc, expect, "poly1305_verify altkey i={i} n={n}");
        }

        // "truncated" key: only the first 16 bytes kept, rest zeroed
        let mut k3 = key.clone();
        for b in k3[16..].iter_mut() {
            *b = 0;
        }
        let rc = unsafe { cv(good.as_ptr(), msg.as_ptr(), n as u64, k3.as_ptr()) };
        let rr = unsafe { rv(good.as_ptr(), msg.as_ptr(), n as u64, k3.as_ptr()) };
        common::eqi(&format!("poly1305_verify trunckey n={n}"), rc, rr);

        // all-zero key
        let k4 = [0u8; 32];
        let (mut ct, mut rt) = ([0xA5u8; 24], [0xA5u8; 24]);
        unsafe {
            c1(ct.as_mut_ptr(), msg.as_ptr(), n as u64, k4.as_ptr());
            r1(rt.as_mut_ptr(), msg.as_ptr(), n as u64, k4.as_ptr());
        }
        common::eqb(&format!("poly1305 zerokey n={n}"), &ct, &rt);
        let rc = unsafe { cv(ct.as_ptr(), msg.as_ptr(), n as u64, k4.as_ptr()) };
        let rr = unsafe { rv(rt.as_ptr(), msg.as_ptr(), n as u64, k4.as_ptr()) };
        common::eqi(&format!("poly1305_verify zerokey n={n}"), rc, rr);
        assert_eq!(rc, 0);

        // all-0xff key (exercises the h >= p reduction path in poly1305_finish)
        let k5 = [0xffu8; 32];
        let (mut ct, mut rt) = ([0xA5u8; 24], [0xA5u8; 24]);
        unsafe {
            c1(ct.as_mut_ptr(), msg.as_ptr(), n as u64, k5.as_ptr());
            r1(rt.as_mut_ptr(), msg.as_ptr(), n as u64, k5.as_ptr());
        }
        common::eqb(&format!("poly1305 ffkey n={n}"), &ct, &rt);
    }
}

/// The exported implementation vtable
/// `crypto_onetimeauth_poly1305_donna_implementation`: read the struct out of
/// each `.so` and call all five function pointers through both libraries.
#[test]
fn mac6_poly1305_donna_implementation_struct() {
    // `both_data!` yields the *address* of the data symbol in each .so.
    let (cp, rp) = both_data!(
        "crypto_onetimeauth_poly1305_donna_implementation",
        Poly1305Impl
    );
    assert!(!cp.is_null() && !rp.is_null());
    let cimpl: Poly1305Impl = unsafe { *cp };
    let rimpl: Poly1305Impl = unsafe { *rp };
    let mut rng = common::Rng::new(0x0D00_1305);

    for n in LENS {
        let msg = rng.bytes(n);
        let key = rng.bytes(32);

        // .onetimeauth
        let (mut co, mut ro) = ([0xA5u8; 24], [0xA5u8; 24]);
        let rc = unsafe { (cimpl.onetimeauth)(co.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
        let rr = unsafe { (rimpl.onetimeauth)(ro.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
        common::eqi(&format!("impl.onetimeauth rc n={n}"), rc, rr);
        assert_eq!(rc, 0);
        common::eqb(&format!("impl.onetimeauth n={n}"), &co, &ro);

        // .onetimeauth_verify (good + one flipped bit)
        let rc = unsafe {
            (cimpl.onetimeauth_verify)(co.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr())
        };
        let rr = unsafe {
            (rimpl.onetimeauth_verify)(ro.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr())
        };
        common::eqi(&format!("impl.verify ok n={n}"), rc, rr);
        assert_eq!(rc, 0);
        let mut bad = co;
        bad[0] ^= 1;
        let rc =
            unsafe { (cimpl.onetimeauth_verify)(bad.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
        let rr =
            unsafe { (rimpl.onetimeauth_verify)(bad.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
        common::eqi(&format!("impl.verify bad n={n}"), rc, rr);
        assert_eq!(rc, -1);

        // .onetimeauth_init / _update / _final
        let mut cs = Aligned::filled(0x77);
        let mut rs = Aligned::filled(0x77);
        let rc = unsafe { (cimpl.onetimeauth_init)(cs.0.as_mut_ptr(), key.as_ptr()) };
        let rr = unsafe { (rimpl.onetimeauth_init)(rs.0.as_mut_ptr(), key.as_ptr()) };
        common::eqi(&format!("impl.init rc n={n}"), rc, rr);
        common::eqb(&format!("impl.init state n={n}"), &cs.0[..], &rs.0[..]);
        let plan = split_plan(&mut rng, n);
        let mut off = 0;
        for &len in &plan {
            let p = unsafe { msg.as_ptr().add(off) };
            let rc = unsafe { (cimpl.onetimeauth_update)(cs.0.as_mut_ptr(), p, len as u64) };
            let rr = unsafe { (rimpl.onetimeauth_update)(rs.0.as_mut_ptr(), p, len as u64) };
            common::eqi(&format!("impl.update rc n={n}"), rc, rr);
            common::eqb(&format!("impl.update state n={n}"), &cs.0[..], &rs.0[..]);
            off += len;
        }
        let (mut cfo, mut rfo) = ([0xA5u8; 24], [0xA5u8; 24]);
        let rc = unsafe { (cimpl.onetimeauth_final)(cs.0.as_mut_ptr(), cfo.as_mut_ptr()) };
        let rr = unsafe { (rimpl.onetimeauth_final)(rs.0.as_mut_ptr(), rfo.as_mut_ptr()) };
        common::eqi(&format!("impl.final rc n={n}"), rc, rr);
        common::eqb(&format!("impl.final tag n={n}"), &cfo, &rfo);
        common::eqb(&format!("impl.final state n={n}"), &cs.0[..], &rs.0[..]);
        common::eqb(&format!("impl stream==oneshot n={n}"), &cfo[..16], &co[..16]);
    }
}

/// `_crypto_onetimeauth_poly1305_pick_best_implementation`: with no
/// `HAVE_TI_MODE`/`HAVE_EMMINTRIN_H` it unconditionally selects donna and
/// returns 0.  Call it and check the selected implementation still behaves.
#[test]
fn mac7_pick_best_implementation() {
    let (cp, rp) = both!("_crypto_onetimeauth_poly1305_pick_best_implementation", IntFn);
    for _ in 0..3 {
        let (rc, rr) = unsafe { (cp(), rp()) };
        common::eqi("pick_best_implementation rc", rc, rr);
        assert_eq!(rc, 0);
    }

    let (c1, r1) = both!("crypto_onetimeauth_poly1305", OneShotFn);
    let mut rng = common::Rng::new(0x0C1C_1305);
    for _ in 0..20 {
        let n = rng.below(300);
        let msg = rng.bytes(n);
        let key = rng.bytes(32);
        let (mut co, mut ro) = ([0xA5u8; 24], [0xA5u8; 24]);
        unsafe {
            c1(co.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr());
            r1(ro.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr());
        }
        common::eqb(&format!("post-pick poly1305 n={n}"), &co, &ro);
    }
}

/// `crypto_onetimeauth_poly1305_keygen` / `crypto_onetimeauth_keygen`: the value
/// is random so it cannot be compared, but the written extent can be.
#[test]
fn mac8_poly1305_keygen() {
    for name in 0..2 {
        let (c, r) = if name == 0 {
            both!("crypto_onetimeauth_poly1305_keygen", KeygenFn)
        } else {
            both!("crypto_onetimeauth_keygen", KeygenFn)
        };
        for (cf, which) in [(c, "C"), (r, "Rust")] {
            let mut buf = [0x5Au8; 64];
            unsafe { cf(buf.as_mut_ptr()) };
            assert!(
                buf[32..].iter().all(|&b| b == 0x5A),
                "{which} keygen wrote past 32 bytes"
            );
            assert!(
                buf[..32].iter().any(|&b| b != 0x5A),
                "{which} keygen wrote nothing"
            );
        }
    }
}

// =================================================================== hmac ====

struct HmacApi {
    oneshot: OneShotFn,
    verify: VerifyFn,
    init: HmacInitFn,
    update: UpdateFn,
    final_: FinalFn,
    keygen: KeygenFn,
}

/// Shared body for hmacsha256 / hmacsha512 / hmacsha512256.
///
/// The state is a plain struct of two `crypto_hash_sha*_state`s
/// (`{u32/u64 state[8]; count; unsigned char buf[]}`) with no padding, so the
/// whole `statebytes()` range is compared after `init`, after every `update`
/// and after `final`.
fn hmac_suite(name: &str, c: &HmacApi, r: &HmacApi, statebytes: usize, outlen: usize, block: usize) {
    let mut rng = common::Rng::new(0x4841_4D41 ^ (block as u64) ^ (outlen as u64));

    // ---- key lengths shorter than / equal to / longer than the block size ---
    let mut keylens: Vec<usize> = vec![0, 1, 2, 31, 32, 33, block - 1, block, block + 1, block * 2, block * 3 + 7];
    keylens.sort_unstable();
    keylens.dedup();

    for &kl in &keylens {
        for &n in &[0usize, 1, 63, 64, 65, 127, 128, 129, 200] {
            let key = rng.bytes(kl);
            let msg = rng.bytes(n);
            let mut cs = Aligned::filled(0x11);
            let mut rs = Aligned::filled(0x11);
            let tag = format!("{name} keylen={kl} n={n}");

            let kp = if kl == 0 {
                core::ptr::null() // key == NULL with keylen == 0 is legal
            } else {
                key.as_ptr()
            };
            let rc = unsafe { (c.init)(cs.0.as_mut_ptr(), kp, kl) };
            let rr = unsafe { (r.init)(rs.0.as_mut_ptr(), kp, kl) };
            common::eqi(&format!("{tag} init rc"), rc, rr);
            assert_eq!(rc, 0, "{tag} init must return 0");
            common::eqb(
                &format!("{tag} state after init"),
                &cs.0[..statebytes],
                &rs.0[..statebytes],
            );
            assert!(cs.0[statebytes..].iter().all(|&b| b == 0x11));
            assert!(rs.0[statebytes..].iter().all(|&b| b == 0x11));

            let plan = split_plan(&mut rng, n);
            let mut off = 0;
            for (i, &len) in plan.iter().enumerate() {
                let p = if len == 0 && off == n {
                    core::ptr::null()
                } else {
                    unsafe { msg.as_ptr().add(off) }
                };
                let rc = unsafe { (c.update)(cs.0.as_mut_ptr(), p, len as u64) };
                let rr = unsafe { (r.update)(rs.0.as_mut_ptr(), p, len as u64) };
                common::eqi(&format!("{tag} update#{i} rc"), rc, rr);
                assert_eq!(rc, 0);
                common::eqb(
                    &format!("{tag} state after update#{i} len={len}"),
                    &cs.0[..statebytes],
                    &rs.0[..statebytes],
                );
                off += len;
            }

            let (mut co, mut ro) = ([0xA5u8; 80], [0xA5u8; 80]);
            let rc = unsafe { (c.final_)(cs.0.as_mut_ptr(), co.as_mut_ptr()) };
            let rr = unsafe { (r.final_)(rs.0.as_mut_ptr(), ro.as_mut_ptr()) };
            common::eqi(&format!("{tag} final rc"), rc, rr);
            assert_eq!(rc, 0);
            common::eqb(&format!("{tag} mac"), &co, &ro);
            assert!(
                co[outlen..].iter().all(|&b| b == 0xA5),
                "{tag}: final wrote past {outlen} bytes"
            );
            common::eqb(
                &format!("{tag} state after final"),
                &cs.0[..statebytes],
                &rs.0[..statebytes],
            );
        }
    }

    // ---- one-shot (keylen fixed at KEYBYTES = 32) over sha block boundaries --
    for &n in SHA_LENS.iter() {
        for _ in 0..3 {
            let key = rng.bytes(32);
            let msg = rng.bytes(n);
            let (mut co, mut ro) = ([0xA5u8; 80], [0xA5u8; 80]);
            let rc = unsafe { (c.oneshot)(co.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            let rr = unsafe { (r.oneshot)(ro.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            common::eqi(&format!("{name} oneshot rc n={n}"), rc, rr);
            assert_eq!(rc, 0);
            common::eqb(&format!("{name} oneshot n={n}"), &co, &ro);
            assert!(co[outlen..].iter().all(|&b| b == 0xA5));

            // one-shot must equal init(KEYBYTES)/update/final
            let mut cs = Aligned::filled(0);
            unsafe {
                (c.init)(cs.0.as_mut_ptr(), key.as_ptr(), 32);
                (c.update)(cs.0.as_mut_ptr(), msg.as_ptr(), n as u64);
            }
            let mut cst = [0u8; 80];
            unsafe { (c.final_)(cs.0.as_mut_ptr(), cst.as_mut_ptr()) };
            common::eqb(
                &format!("{name} oneshot==stream n={n}"),
                &cst[..outlen],
                &co[..outlen],
            );

            // ---- verify: correct tag -> 0 ----
            let rc = unsafe { (c.verify)(co.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            let rr = unsafe { (r.verify)(ro.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            common::eqi(&format!("{name} verify ok n={n}"), rc, rr);
            assert_eq!(rc, 0);

            // ---- verify: every byte of the tag flipped -> -1 ----
            for i in 0..outlen {
                let mut bad = co;
                bad[i] ^= 0xff;
                let rc = unsafe { (c.verify)(bad.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
                let rr = unsafe { (r.verify)(bad.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
                common::eqi(&format!("{name} verify badbyte={i} n={n}"), rc, rr);
                assert_eq!(rc, -1);
            }
        }
    }

    // ---- verify: every single bit of the tag flipped (one message) ----
    {
        let key = rng.bytes(32);
        let msg = rng.bytes(77);
        let mut good = [0u8; 80];
        unsafe { (c.oneshot)(good.as_mut_ptr(), msg.as_ptr(), 77, key.as_ptr()) };
        for bit in 0..outlen * 8 {
            let mut bad = good;
            bad[bit / 8] ^= 1u8 << (bit % 8);
            let rc = unsafe { (c.verify)(bad.as_ptr(), msg.as_ptr(), 77, key.as_ptr()) };
            let rr = unsafe { (r.verify)(bad.as_ptr(), msg.as_ptr(), 77, key.as_ptr()) };
            common::eqi(&format!("{name} verify bitflip={bit}"), rc, rr);
            assert_eq!(rc, -1);
        }
        // altered key -> -1
        for i in 0..32 {
            let mut k2 = key.clone();
            k2[i] ^= 0x01;
            let rc = unsafe { (c.verify)(good.as_ptr(), msg.as_ptr(), 77, k2.as_ptr()) };
            let rr = unsafe { (r.verify)(good.as_ptr(), msg.as_ptr(), 77, k2.as_ptr()) };
            common::eqi(&format!("{name} verify altkey={i}"), rc, rr);
            assert_eq!(rc, -1);
        }
        // altered message -> -1
        let mut m2 = msg.clone();
        m2[0] ^= 0x01;
        let rc = unsafe { (c.verify)(good.as_ptr(), m2.as_ptr(), 77, key.as_ptr()) };
        let rr = unsafe { (r.verify)(good.as_ptr(), m2.as_ptr(), 77, key.as_ptr()) };
        common::eqi(&format!("{name} verify altmsg"), rc, rr);
        assert_eq!(rc, -1);
        // truncated message (shorter inlen) -> -1
        let rc = unsafe { (c.verify)(good.as_ptr(), msg.as_ptr(), 76, key.as_ptr()) };
        let rr = unsafe { (r.verify)(good.as_ptr(), msg.as_ptr(), 76, key.as_ptr()) };
        common::eqi(&format!("{name} verify short inlen"), rc, rr);
        assert_eq!(rc, -1);
    }

    // ---- oneshot / verify with in == NULL, inlen == 0 ----
    {
        let key = rng.bytes(32);
        let (mut co, mut ro) = ([0xA5u8; 80], [0xA5u8; 80]);
        let rc = unsafe { (c.oneshot)(co.as_mut_ptr(), core::ptr::null(), 0, key.as_ptr()) };
        let rr = unsafe { (r.oneshot)(ro.as_mut_ptr(), core::ptr::null(), 0, key.as_ptr()) };
        common::eqi(&format!("{name} oneshot in=NULL rc"), rc, rr);
        common::eqb(&format!("{name} oneshot in=NULL"), &co, &ro);
        let rc = unsafe { (c.verify)(co.as_ptr(), core::ptr::null(), 0, key.as_ptr()) };
        let rr = unsafe { (r.verify)(ro.as_ptr(), core::ptr::null(), 0, key.as_ptr()) };
        common::eqi(&format!("{name} verify in=NULL rc"), rc, rr);
        assert_eq!(rc, 0);
    }

    // ---- keygen: random value, but the written extent must be exactly 32 ----
    for (kg, which) in [(c.keygen, "C"), (r.keygen, "Rust")] {
        let mut buf = [0x5Au8; 64];
        unsafe { kg(buf.as_mut_ptr()) };
        assert!(
            buf[32..].iter().all(|&b| b == 0x5A),
            "{name}: {which} keygen wrote past 32 bytes"
        );
        assert!(
            buf[..32].iter().any(|&b| b != 0x5A),
            "{name}: {which} keygen wrote nothing"
        );
    }
}

#[test]
fn mac9_hmacsha256() {
    let l = common::libs();
    let c = HmacApi {
        oneshot: getsym!(l.c, "crypto_auth_hmacsha256", OneShotFn),
        verify: getsym!(l.c, "crypto_auth_hmacsha256_verify", VerifyFn),
        init: getsym!(l.c, "crypto_auth_hmacsha256_init", HmacInitFn),
        update: getsym!(l.c, "crypto_auth_hmacsha256_update", UpdateFn),
        final_: getsym!(l.c, "crypto_auth_hmacsha256_final", FinalFn),
        keygen: getsym!(l.c, "crypto_auth_hmacsha256_keygen", KeygenFn),
    };
    let r = HmacApi {
        oneshot: getsym!(l.r, "crypto_auth_hmacsha256", OneShotFn),
        verify: getsym!(l.r, "crypto_auth_hmacsha256_verify", VerifyFn),
        init: getsym!(l.r, "crypto_auth_hmacsha256_init", HmacInitFn),
        update: getsym!(l.r, "crypto_auth_hmacsha256_update", UpdateFn),
        final_: getsym!(l.r, "crypto_auth_hmacsha256_final", FinalFn),
        keygen: getsym!(l.r, "crypto_auth_hmacsha256_keygen", KeygenFn),
    };
    let sb = chk_size!("crypto_auth_hmacsha256_statebytes", 208);
    hmac_suite("hmacsha256", &c, &r, sb, 32, 64);
}

#[test]
fn mac10_hmacsha512() {
    let l = common::libs();
    let c = HmacApi {
        oneshot: getsym!(l.c, "crypto_auth_hmacsha512", OneShotFn),
        verify: getsym!(l.c, "crypto_auth_hmacsha512_verify", VerifyFn),
        init: getsym!(l.c, "crypto_auth_hmacsha512_init", HmacInitFn),
        update: getsym!(l.c, "crypto_auth_hmacsha512_update", UpdateFn),
        final_: getsym!(l.c, "crypto_auth_hmacsha512_final", FinalFn),
        keygen: getsym!(l.c, "crypto_auth_hmacsha512_keygen", KeygenFn),
    };
    let r = HmacApi {
        oneshot: getsym!(l.r, "crypto_auth_hmacsha512", OneShotFn),
        verify: getsym!(l.r, "crypto_auth_hmacsha512_verify", VerifyFn),
        init: getsym!(l.r, "crypto_auth_hmacsha512_init", HmacInitFn),
        update: getsym!(l.r, "crypto_auth_hmacsha512_update", UpdateFn),
        final_: getsym!(l.r, "crypto_auth_hmacsha512_final", FinalFn),
        keygen: getsym!(l.r, "crypto_auth_hmacsha512_keygen", KeygenFn),
    };
    let sb = chk_size!("crypto_auth_hmacsha512_statebytes", 416);
    hmac_suite("hmacsha512", &c, &r, sb, 64, 128);
}

#[test]
fn mac11_hmacsha512256() {
    let l = common::libs();
    let c = HmacApi {
        oneshot: getsym!(l.c, "crypto_auth_hmacsha512256", OneShotFn),
        verify: getsym!(l.c, "crypto_auth_hmacsha512256_verify", VerifyFn),
        init: getsym!(l.c, "crypto_auth_hmacsha512256_init", HmacInitFn),
        update: getsym!(l.c, "crypto_auth_hmacsha512256_update", UpdateFn),
        final_: getsym!(l.c, "crypto_auth_hmacsha512256_final", FinalFn),
        keygen: getsym!(l.c, "crypto_auth_hmacsha512256_keygen", KeygenFn),
    };
    let r = HmacApi {
        oneshot: getsym!(l.r, "crypto_auth_hmacsha512256", OneShotFn),
        verify: getsym!(l.r, "crypto_auth_hmacsha512256_verify", VerifyFn),
        init: getsym!(l.r, "crypto_auth_hmacsha512256_init", HmacInitFn),
        update: getsym!(l.r, "crypto_auth_hmacsha512256_update", UpdateFn),
        final_: getsym!(l.r, "crypto_auth_hmacsha512256_final", FinalFn),
        keygen: getsym!(l.r, "crypto_auth_hmacsha512256_keygen", KeygenFn),
    };
    let sb = chk_size!("crypto_auth_hmacsha512256_statebytes", 416);
    hmac_suite("hmacsha512256", &c, &r, sb, 32, 128);
}

/// `crypto_auth` / `crypto_auth_verify` / `crypto_auth_keygen`: pure
/// dispatchers onto hmacsha512256.
#[test]
fn mac12_crypto_auth_generic() {
    let (ca, ra) = both!("crypto_auth", OneShotFn);
    let (cv, rv) = both!("crypto_auth_verify", VerifyFn);
    let (c256, _r256) = both!("crypto_auth_hmacsha512256", OneShotFn);
    let (ckg, rkg) = both!("crypto_auth_keygen", KeygenFn);
    let mut rng = common::Rng::new(0x0A07_4811);

    for &n in SHA_LENS.iter() {
        for _ in 0..3 {
            let key = rng.bytes(32);
            let msg = rng.bytes(n);
            let (mut co, mut ro) = ([0xA5u8; 48], [0xA5u8; 48]);
            let rc = unsafe { ca(co.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            let rr = unsafe { ra(ro.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            common::eqi(&format!("crypto_auth rc n={n}"), rc, rr);
            assert_eq!(rc, 0);
            common::eqb(&format!("crypto_auth n={n}"), &co, &ro);
            assert!(co[32..].iter().all(|&b| b == 0xA5));

            // must be identical to the hmacsha512256 primitive
            let mut ref_ = [0xA5u8; 48];
            unsafe { c256(ref_.as_mut_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            common::eqb(&format!("crypto_auth==hmacsha512256 n={n}"), &ref_, &co);

            let rc = unsafe { cv(co.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            let rr = unsafe { rv(ro.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
            common::eqi(&format!("crypto_auth_verify ok n={n}"), rc, rr);
            assert_eq!(rc, 0);

            for i in 0..32 {
                let mut bad = co;
                bad[i] ^= 0xff;
                let rc = unsafe { cv(bad.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
                let rr = unsafe { rv(bad.as_ptr(), msg.as_ptr(), n as u64, key.as_ptr()) };
                common::eqi(&format!("crypto_auth_verify bad i={i} n={n}"), rc, rr);
                assert_eq!(rc, -1);
            }
        }
    }

    for (kg, which) in [(ckg, "C"), (rkg, "Rust")] {
        let mut buf = [0x5Au8; 64];
        unsafe { kg(buf.as_mut_ptr()) };
        assert!(
            buf[32..].iter().all(|&b| b == 0x5A),
            "{which} crypto_auth_keygen wrote past 32 bytes"
        );
        assert!(buf[..32].iter().any(|&b| b != 0x5A));
    }
}

/// `crypto_auth_hmacsha512256_*` must be bit-identical to the first 32 bytes of
/// `crypto_auth_hmacsha512_*` (the C `final` does `memcpy(out, out0, 32)`), and
/// the two states must stay byte-identical because `hmacsha512256_init/update`
/// are straight casts onto the hmacsha512 functions.
#[test]
fn mac13_hmacsha512256_is_truncated_hmacsha512() {
    let (c512i, r512i) = both!("crypto_auth_hmacsha512_init", HmacInitFn);
    let (c256i, r256i) = both!("crypto_auth_hmacsha512256_init", HmacInitFn);
    let (c512u, r512u) = both!("crypto_auth_hmacsha512_update", UpdateFn);
    let (c256u, r256u) = both!("crypto_auth_hmacsha512256_update", UpdateFn);
    let (c512f, r512f) = both!("crypto_auth_hmacsha512_final", FinalFn);
    let (c256f, r256f) = both!("crypto_auth_hmacsha512256_final", FinalFn);
    let mut rng = common::Rng::new(0x5122_5656);

    for &kl in &[0usize, 1, 32, 128, 129, 300] {
        for &n in &[0usize, 1, 64, 127, 128, 129, 500] {
            let key = rng.bytes(kl);
            let kp = if kl == 0 { core::ptr::null() } else { key.as_ptr() };
            let msg = rng.bytes(n);

            for (i512, u512, f512, i256, u256, f256, which) in [
                (c512i, c512u, c512f, c256i, c256u, c256f, "C"),
                (r512i, r512u, r512f, r256i, r256u, r256f, "Rust"),
            ] {
                // hmacsha512256_init/_update are straight casts onto the
                // hmacsha512 ones: the state must be byte-identical.
                let mut a = Aligned::filled(0x22);
                let mut b = Aligned::filled(0x22);
                let mut full = [0xA5u8; 80];
                let mut trunc = [0xA5u8; 80];
                unsafe {
                    i512(a.0.as_mut_ptr(), kp, kl);
                    i256(b.0.as_mut_ptr(), kp, kl);
                    u512(a.0.as_mut_ptr(), msg.as_ptr(), n as u64);
                    u256(b.0.as_mut_ptr(), msg.as_ptr(), n as u64);
                }
                common::eqb(
                    &format!("{which} hmacsha512 state == hmacsha512256 state kl={kl} n={n}"),
                    &a.0[..416],
                    &b.0[..416],
                );
                unsafe {
                    f512(a.0.as_mut_ptr(), full.as_mut_ptr());
                    f256(b.0.as_mut_ptr(), trunc.as_mut_ptr());
                }
                common::eqb(
                    &format!("{which} hmacsha512256_final == hmacsha512_final[..32] kl={kl} n={n}"),
                    &full[..32],
                    &trunc[..32],
                );
                assert!(
                    trunc[32..].iter().all(|&b| b == 0xA5),
                    "{which} hmacsha512256_final wrote past 32 bytes"
                );
            }
        }
    }
}

/// `*_init` with `key == NULL` and `keylen == 0` is explicitly allowed by the C
/// (`sodium_misuse()` only fires when `keylen > 0`), and must produce the same
/// state / MAC as a zero-length non-NULL key.
#[test]
fn mac14_init_null_key_zero_len() {
    let l = common::libs();
    let cases: [(HmacInitFn, HmacInitFn, FinalFn, FinalFn, UpdateFn, UpdateFn, usize, usize, &str); 3] = [
        (
            getsym!(l.c, "crypto_auth_hmacsha256_init", HmacInitFn),
            getsym!(l.r, "crypto_auth_hmacsha256_init", HmacInitFn),
            getsym!(l.c, "crypto_auth_hmacsha256_final", FinalFn),
            getsym!(l.r, "crypto_auth_hmacsha256_final", FinalFn),
            getsym!(l.c, "crypto_auth_hmacsha256_update", UpdateFn),
            getsym!(l.r, "crypto_auth_hmacsha256_update", UpdateFn),
            208,
            32,
            "hmacsha256",
        ),
        (
            getsym!(l.c, "crypto_auth_hmacsha512_init", HmacInitFn),
            getsym!(l.r, "crypto_auth_hmacsha512_init", HmacInitFn),
            getsym!(l.c, "crypto_auth_hmacsha512_final", FinalFn),
            getsym!(l.r, "crypto_auth_hmacsha512_final", FinalFn),
            getsym!(l.c, "crypto_auth_hmacsha512_update", UpdateFn),
            getsym!(l.r, "crypto_auth_hmacsha512_update", UpdateFn),
            416,
            64,
            "hmacsha512",
        ),
        (
            getsym!(l.c, "crypto_auth_hmacsha512256_init", HmacInitFn),
            getsym!(l.r, "crypto_auth_hmacsha512256_init", HmacInitFn),
            getsym!(l.c, "crypto_auth_hmacsha512256_final", FinalFn),
            getsym!(l.r, "crypto_auth_hmacsha512256_final", FinalFn),
            getsym!(l.c, "crypto_auth_hmacsha512256_update", UpdateFn),
            getsym!(l.r, "crypto_auth_hmacsha512256_update", UpdateFn),
            416,
            32,
            "hmacsha512256",
        ),
    ];

    let mut rng = common::Rng::new(0x4E554C4C);
    for (ci, ri, cf, rf, cu, ru, sb, outlen, name) in cases {
        let msg = rng.bytes(97);
        // key == NULL, keylen == 0
        let mut cs = Aligned::filled(0x44);
        let mut rs = Aligned::filled(0x44);
        let rc = unsafe { ci(cs.0.as_mut_ptr(), core::ptr::null(), 0) };
        let rr = unsafe { ri(rs.0.as_mut_ptr(), core::ptr::null(), 0) };
        common::eqi(&format!("{name} init(NULL,0) rc"), rc, rr);
        assert_eq!(rc, 0);
        common::eqb(&format!("{name} init(NULL,0) state"), &cs.0[..sb], &rs.0[..sb]);
        unsafe {
            cu(cs.0.as_mut_ptr(), msg.as_ptr(), 97);
            ru(rs.0.as_mut_ptr(), msg.as_ptr(), 97);
        }
        let (mut co, mut ro) = ([0xA5u8; 80], [0xA5u8; 80]);
        unsafe {
            cf(cs.0.as_mut_ptr(), co.as_mut_ptr());
            rf(rs.0.as_mut_ptr(), ro.as_mut_ptr());
        }
        common::eqb(&format!("{name} init(NULL,0) mac"), &co, &ro);

        // non-NULL pointer, keylen == 0 must yield exactly the same MAC
        let dummy = [0u8; 1];
        let mut cs2 = Aligned::filled(0x44);
        unsafe {
            ci(cs2.0.as_mut_ptr(), dummy.as_ptr(), 0);
            cu(cs2.0.as_mut_ptr(), msg.as_ptr(), 97);
        }
        let mut co2 = [0xA5u8; 80];
        unsafe { cf(cs2.0.as_mut_ptr(), co2.as_mut_ptr()) };
        common::eqb(
            &format!("{name} init(ptr,0) == init(NULL,0)"),
            &co[..outlen],
            &co2[..outlen],
        );
    }
}

/// `*_init` with `keylen > blocksize`: the key is pre-hashed *using
/// `state->ictx` as scratch*, then replaced by the 32/64-byte digest.  Check the
/// resulting MAC equals `HMAC(SHA(key), msg)`.
#[test]
fn mac15_long_key_is_prehashed() {
    let (c256, r256) = both!("crypto_auth_hmacsha256_init", HmacInitFn);
    let (c256u, r256u) = both!("crypto_auth_hmacsha256_update", UpdateFn);
    let (c256f, r256f) = both!("crypto_auth_hmacsha256_final", FinalFn);
    let (csha256, _) = both!(
        "crypto_hash_sha256",
        unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int
    );
    let (c512, r512) = both!("crypto_auth_hmacsha512_init", HmacInitFn);
    let (c512u, r512u) = both!("crypto_auth_hmacsha512_update", UpdateFn);
    let (c512f, r512f) = both!("crypto_auth_hmacsha512_final", FinalFn);
    let (csha512, _) = both!(
        "crypto_hash_sha512",
        unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int
    );
    let mut rng = common::Rng::new(0x10_46_4B45);

    for &kl in &[65usize, 100, 128, 129, 200, 1000] {
        let key = rng.bytes(kl);
        let msg = rng.bytes(133);

        // --- sha256 (block 64): pre-hash whenever kl > 64 ---
        let mut a = Aligned::filled(0x66);
        let mut b = Aligned::filled(0x66);
        unsafe {
            c256(a.0.as_mut_ptr(), key.as_ptr(), kl);
            r256(b.0.as_mut_ptr(), key.as_ptr(), kl);
            c256u(a.0.as_mut_ptr(), msg.as_ptr(), 133);
            r256u(b.0.as_mut_ptr(), msg.as_ptr(), 133);
        }
        common::eqb(&format!("sha256 longkey kl={kl} state"), &a.0[..208], &b.0[..208]);
        let (mut ca, mut ra) = ([0u8; 32], [0u8; 32]);
        unsafe {
            c256f(a.0.as_mut_ptr(), ca.as_mut_ptr());
            r256f(b.0.as_mut_ptr(), ra.as_mut_ptr());
        }
        common::eqb(&format!("sha256 longkey kl={kl} mac"), &ca, &ra);
        // equals HMAC with the 32-byte digest of the key
        let mut kh = [0u8; 32];
        unsafe { csha256(kh.as_mut_ptr(), key.as_ptr(), kl as u64) };
        let mut d = Aligned::filled(0x66);
        let mut cd = [0u8; 32];
        unsafe {
            c256(d.0.as_mut_ptr(), kh.as_ptr(), 32);
            c256u(d.0.as_mut_ptr(), msg.as_ptr(), 133);
            c256f(d.0.as_mut_ptr(), cd.as_mut_ptr());
        }
        common::eqb(&format!("sha256 longkey kl={kl} == HMAC(SHA256(key))"), &cd, &ca);

        // --- sha512 (block 128): pre-hash only when kl > 128 ---
        let mut a = Aligned::filled(0x66);
        let mut b = Aligned::filled(0x66);
        unsafe {
            c512(a.0.as_mut_ptr(), key.as_ptr(), kl);
            r512(b.0.as_mut_ptr(), key.as_ptr(), kl);
            c512u(a.0.as_mut_ptr(), msg.as_ptr(), 133);
            r512u(b.0.as_mut_ptr(), msg.as_ptr(), 133);
        }
        common::eqb(&format!("sha512 longkey kl={kl} state"), &a.0[..416], &b.0[..416]);
        let (mut ca, mut ra) = ([0u8; 64], [0u8; 64]);
        unsafe {
            c512f(a.0.as_mut_ptr(), ca.as_mut_ptr());
            r512f(b.0.as_mut_ptr(), ra.as_mut_ptr());
        }
        common::eqb(&format!("sha512 longkey kl={kl} mac"), &ca, &ra);
        if kl > 128 {
            let mut kh = [0u8; 64];
            unsafe { csha512(kh.as_mut_ptr(), key.as_ptr(), kl as u64) };
            let mut d = Aligned::filled(0x66);
            let mut cd = [0u8; 64];
            unsafe {
                c512(d.0.as_mut_ptr(), kh.as_ptr(), 64);
                c512u(d.0.as_mut_ptr(), msg.as_ptr(), 133);
                c512f(d.0.as_mut_ptr(), cd.as_mut_ptr());
            }
            common::eqb(&format!("sha512 longkey kl={kl} == HMAC(SHA512(key))"), &cd, &ca);
        }
    }
}

/// Known-answer cross-check: RFC 4231 HMAC-SHA-256 test case 2
/// ("Jefe" / "what do ya want for nothing?").  Guards against both libraries
/// being wrong in the same way for the short-key path.
#[test]
fn mac16_rfc4231_kat() {
    let (ci, ri) = both!("crypto_auth_hmacsha256_init", HmacInitFn);
    let (cu, ru) = both!("crypto_auth_hmacsha256_update", UpdateFn);
    let (cf, rf) = both!("crypto_auth_hmacsha256_final", FinalFn);
    let key = b"Jefe";
    let msg = b"what do ya want for nothing?";
    let expect: [u8; 32] = [
        0x5b, 0xdc, 0xc1, 0x46, 0xbf, 0x60, 0x75, 0x4e, 0x6a, 0x04, 0x24, 0x26, 0x08, 0x95, 0x75,
        0xc7, 0x5a, 0x00, 0x3f, 0x08, 0x9d, 0x27, 0x39, 0x83, 0x9d, 0xec, 0x58, 0xb9, 0x64, 0xec,
        0x38, 0x43,
    ];
    for (i, u, f, which) in [(ci, cu, cf, "C"), (ri, ru, rf, "Rust")] {
        let mut st = Aligned::filled(0);
        let mut out = [0u8; 32];
        unsafe {
            i(st.0.as_mut_ptr(), key.as_ptr(), key.len());
            u(st.0.as_mut_ptr(), msg.as_ptr(), msg.len() as u64);
            f(st.0.as_mut_ptr(), out.as_mut_ptr());
        }
        common::eqb(&format!("{which} RFC4231 case 2"), &expect, &out);
    }
}

/// Poly1305 RFC 8439 §2.5.2 known-answer vector.
#[test]
fn mac17_poly1305_kat() {
    let (c1, r1) = both!("crypto_onetimeauth_poly1305", OneShotFn);
    let key: [u8; 32] = [
        0x85, 0xd6, 0xbe, 0x78, 0x57, 0x55, 0x6d, 0x33, 0x7f, 0x44, 0x52, 0xfe, 0x42, 0xd5, 0x06,
        0xa8, 0x01, 0x03, 0x80, 0x8a, 0xfb, 0x0d, 0xb2, 0xfd, 0x4a, 0xbf, 0xf6, 0xaf, 0x41, 0x49,
        0xf5, 0x1b,
    ];
    let msg = b"Cryptographic Forum Research Group";
    let expect: [u8; 16] = [
        0xa8, 0x06, 0x1d, 0xc1, 0x30, 0x51, 0x36, 0xc6, 0xc2, 0x2b, 0x8b, 0xaf, 0x0c, 0x01, 0x27,
        0xa9,
    ];
    for (f, which) in [(c1, "C"), (r1, "Rust")] {
        let mut out = [0u8; 16];
        unsafe { f(out.as_mut_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()) };
        common::eqb(&format!("{which} RFC8439 poly1305"), &expect, &out);
    }
}
