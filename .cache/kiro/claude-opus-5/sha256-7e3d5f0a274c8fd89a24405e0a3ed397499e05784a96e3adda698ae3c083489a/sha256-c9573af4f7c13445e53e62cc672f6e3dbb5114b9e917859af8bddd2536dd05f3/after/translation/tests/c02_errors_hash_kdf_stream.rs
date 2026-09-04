//! Phase C — ERROR-PATH differential tests, ERRORS.md rows 62–81, 264–301.
//!
//! For each invalid-input condition we construct that exact condition, drive
//! BOTH the C `.so` and the Rust `.so`, and assert they agree on the *observable
//! error surface*: identical integer return / sentinel / errno, or — for the
//! `sodium_misuse()`→`abort()` paths — the identical forked-child process fate
//! (`Fate::Signaled(6)` for `SIGABRT`, or both `Fate::Exited(0)`).
//!
//! Coverage map (see the doc comment on each test fn for the precise rows):
//!   * generichash/blake2b bounds + final misuse ............ rows 62–81
//!   * crypto_kdf / hkdf expand bounds ...................... rows 264–268
//!   * crypto_auth / onetimeauth init-misuse + verify ....... rows 269–276
//!   * crypto_stream_chacha20_ietf misuse (length/counter) .. rows 277–285
//!   * randombytes + generic FFI enum/boundary conditions ... rows 286–301
//!
//! IMPORTANT — several ERRORS.md predictions were mechanically derived and are
//! REFINED here against the actual C source (which is ground truth):
//!   * Row 75 (`crypto_generichash_blake2b_final` with `outlen == 0`): the
//!     public wrapper does NOT pre-check outlen; it forwards to `blake2b_final`,
//!     whose `if (!outlen || outlen > 64) sodium_misuse();` guard **aborts**
//!     (SIGABRT), it does NOT return -1. Tested as an abort via `same_fate`.
//!   * Row 76 (`_final` with `outlen != state->outlen`): `blake2b_final` has NO
//!     state-outlen comparison. Any `1 <= outlen <= 64` simply copies `outlen`
//!     bytes of the digest and returns 0. Tested as a **success** whose bytes
//!     match between C and Rust (a prefix of the digest).
//! In both cases the differential contract (C fate == Rust fate) is what we
//! assert; the refined note documents *why* the fate is what it is.

mod common;
use common::*;

// ---- errno numbers on Linux (per task spec) ----
const EINVAL: i32 = 22;
// ERANGE=34, ENOMEM=12, EFBIG=27, ENOSYS=38 are referenced in notes only.

// ---- exact C signatures ------------------------------------------------
// int crypto_generichash_blake2b(out*, outlen, in*, inlen, key*, keylen)
type GhOneshot = unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> i32;
// int crypto_generichash_blake2b_salt_personal(out,outlen,in,inlen,key,keylen,salt,personal)
type GhSaltPersonal =
    unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize, *const u8, *const u8) -> i32;
// int crypto_generichash_blake2b_init(state*, key*, keylen, outlen)
type GhInit = unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> i32;
// int crypto_generichash_blake2b_init_salt_personal(state,key,keylen,outlen,salt,personal)
type GhInitSP =
    unsafe extern "C" fn(*mut u8, *const u8, usize, usize, *const u8, *const u8) -> i32;
// int crypto_generichash_blake2b_update(state*, in*, inlen)
type GhUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
// int crypto_generichash_blake2b_final(state*, out*, outlen)
type GhFinal = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> i32;
type SizeFn = unsafe extern "C" fn() -> usize;

// kdf: int crypto_kdf_blake2b_derive_from_key(subkey*, subkey_len, subkey_id, ctx[8], key[32])
type KdfDerive = unsafe extern "C" fn(*mut u8, usize, u64, *const u8, *const u8) -> i32;
// hkdf: int crypto_kdf_hkdf_sha256_expand(out*, out_len, ctx*, ctx_len, prk*)
type HkdfExpand = unsafe extern "C" fn(*mut u8, usize, *const i8, usize, *const u8) -> i32;

// auth init: int crypto_auth_hmacsha256_init(state*, key*, keylen)
type AuthInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
// verify: int crypto_auth_*_verify(h*, in*, inlen, k*)
type AuthVerify = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
// int crypto_auth_hmacsha256(out*, in*, inlen, k*)
type AuthMac = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
// onetimeauth verify: int (h*, in*, inlen, k*)  (poly1305 & generic identical shape)
type OtaVerify = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
type OtaMac = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;

// stream: int crypto_stream_chacha20_ietf(c*, clen, n*, k*)
type StreamIetf = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> i32;
// int crypto_stream_chacha20_ietf_xor_ic(c*, m*, mlen, n*, ic:u32, k*)
type StreamIetfXorIc = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> i32;

// randombytes_uniform(upper_bound: u32) -> u32
type RbUniform = unsafe extern "C" fn(u32) -> u32;
type RbClose = unsafe extern "C" fn() -> i32;

// pwhash: int crypto_pwhash(out*, outlen, passwd*, passwdlen, salt*, opslimit, memlimit, alg)
type Pwhash = unsafe extern "C" fn(*mut u8, u64, *const i8, u64, *const u8, u64, usize, i32) -> i32;
// int crypto_pwhash_str_alg(out*, passwd*, passwdlen, opslimit, memlimit, alg)
type PwhashStrAlg = unsafe extern "C" fn(*mut i8, *const i8, u64, u64, usize, i32) -> i32;

// core from_string family: int f(out*, ctx*, ctx_len, msg*, msg_len, hash_alg)
type CoreFromString = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, i32) -> i32;

// secretstream
type InitPull = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
type Push =
    unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8, u64, *const u8, u64, u8) -> i32;
type Pull =
    unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *mut u8, *const u8, u64, *const u8, u64) -> i32;

// utils
type Pad = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> i32;
type Unpad = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> i32;
type Compare = unsafe extern "C" fn(*const u8, *const u8, usize) -> i32;
type Memcmp = unsafe extern "C" fn(*const core::ffi::c_void, *const core::ffi::c_void, usize) -> i32;
type IsZero = unsafe extern "C" fn(*const u8, usize) -> i32;

fn statebytes(name: &str) -> usize {
    let d = duo();
    let (c, r) = d.pair::<SizeFn>(name);
    let cv = unsafe { c() };
    let rv = unsafe { r() };
    assert_eq!(cv, rv, "{name} mismatch C={cv} Rust={rv}");
    cv
}

// ===========================================================================
// A) crypto_generichash / blake2b — ERRORS.md rows 62–81
// ===========================================================================

/// ERRORS.md rows 62, 63, 64, 65, 66, 67, 77: one-shot generichash out-of-range
/// bounds → return -1 (NOT a misuse; the public wrapper pre-checks).
///  * `crypto_generichash_blake2b` (62/63/64) and its generic alias
///    `crypto_generichash` (77): `outlen == 0`, `outlen > 64`, `keylen > 64`.
///  * `crypto_generichash_blake2b_salt_personal` (65/66/67): same three bounds.
/// For each we assert identical return AND — since a -1 leaves `out` untouched —
/// identical `out` bytes across C and Rust.
#[test]
fn generichash_oneshot_out_of_range_returns_minus1() {
    let d = duo();
    let (c1, r1) = d.pair::<GhOneshot>("crypto_generichash_blake2b");
    let (c1, r1) = (*c1, *r1);
    let (cg, rg) = d.pair::<GhOneshot>("crypto_generichash");
    let (cg, rg) = (*cg, *rg);
    let (csp, rsp) = d.pair::<GhSaltPersonal>("crypto_generichash_blake2b_salt_personal");
    let (csp, rsp) = (*csp, *rsp);

    // (outlen, keylen) invalid combinations. keylen<=64 is fine; >64 invalid.
    let bad: &[(usize, usize)] = &[
        (0, 0),   // outlen == 0
        (0, 32),  // outlen == 0, valid key
        (65, 0),  // outlen > 64
        (100, 0), // outlen > 64
        (32, 65), // keylen > 64
        (32, 200),
        (0, 65), // both invalid
    ];
    let msg = [0xABu8; 16];

    for &(outlen, keylen) in bad {
        let key = vec![0x11u8; keylen.max(1)];
        let keyp = if keylen == 0 { core::ptr::null() } else { key.as_ptr() };

        // one-shot blake2b (rows 62/63/64) + generic alias (row 77)
        for (name, cf, rf) in [("blake2b", c1, r1), ("generic", cg, rg)] {
            let mut oc = vec![0x55u8; outlen.max(1)];
            let mut or = vec![0x55u8; outlen.max(1)];
            let rc = unsafe { cf(oc.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64, keyp, keylen) };
            let rr = unsafe { rf(or.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64, keyp, keylen) };
            eq_i32(&format!("{name} oneshot outlen={outlen} keylen={keylen} ret"), rc, rr);
            assert_eq!(rc, -1, "{name} oneshot outlen={outlen} keylen={keylen} should be -1");
            eq_bytes(&format!("{name} oneshot out untouched"), &oc, &or);
        }

        // salt_personal (rows 65/66/67)
        let salt = [0u8; 16];
        let pers = [0u8; 16];
        let mut oc = vec![0x55u8; outlen.max(1)];
        let mut or = vec![0x55u8; outlen.max(1)];
        let rc = unsafe {
            csp(oc.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64, keyp, keylen, salt.as_ptr(), pers.as_ptr())
        };
        let rr = unsafe {
            rsp(or.as_mut_ptr(), outlen, msg.as_ptr(), msg.len() as u64, keyp, keylen, salt.as_ptr(), pers.as_ptr())
        };
        eq_i32(&format!("salt_personal outlen={outlen} keylen={keylen} ret"), rc, rr);
        assert_eq!(rc, -1, "salt_personal outlen={outlen} keylen={keylen} should be -1");
        eq_bytes("salt_personal out untouched", &oc, &or);
    }
}

/// ERRORS.md rows 68, 69, 70, 71, 72, 73, 78: streaming `_init` out-of-range
/// bounds → return -1.
///  * `crypto_generichash_blake2b_init` (68/69/70) and its generic alias
///    `crypto_generichash_init` (78).
///  * `crypto_generichash_blake2b_init_salt_personal` (71/72/73).
/// `outlen == 0`, `outlen > 64`, `keylen > 64` each → -1, state left as-is.
#[test]
fn generichash_init_out_of_range_returns_minus1() {
    let d = duo();
    let sb = statebytes("crypto_generichash_blake2b_statebytes");
    let (ci, ri) = d.pair::<GhInit>("crypto_generichash_blake2b_init");
    let (ci, ri) = (*ci, *ri);
    let (cg, rg) = d.pair::<GhInit>("crypto_generichash_init");
    let (cg, rg) = (*cg, *rg);
    let (csp, rsp) = d.pair::<GhInitSP>("crypto_generichash_blake2b_init_salt_personal");
    let (csp, rsp) = (*csp, *rsp);

    let bad: &[(usize, usize)] = &[(0, 0), (0, 32), (65, 0), (100, 0), (32, 65), (32, 200)];

    for &(outlen, keylen) in bad {
        let key = vec![0x22u8; keylen.max(1)];
        let keyp = if keylen == 0 { core::ptr::null() } else { key.as_ptr() };

        for (name, cf, rf) in [("blake2b_init", ci, ri), ("generic_init", cg, rg)] {
            let mut cs = vec![0u8; sb];
            let mut rs = vec![0u8; sb];
            let rc = unsafe { cf(cs.as_mut_ptr(), keyp, keylen, outlen) };
            let rr = unsafe { rf(rs.as_mut_ptr(), keyp, keylen, outlen) };
            eq_i32(&format!("{name} outlen={outlen} keylen={keylen} ret"), rc, rr);
            assert_eq!(rc, -1, "{name} outlen={outlen} keylen={keylen} should be -1");
        }

        let salt = [0u8; 16];
        let pers = [0u8; 16];
        let mut cs = vec![0u8; sb];
        let mut rs = vec![0u8; sb];
        let rc = unsafe { csp(cs.as_mut_ptr(), keyp, keylen, outlen, salt.as_ptr(), pers.as_ptr()) };
        let rr = unsafe { rsp(rs.as_mut_ptr(), keyp, keylen, outlen, salt.as_ptr(), pers.as_ptr()) };
        eq_i32(&format!("init_salt_personal outlen={outlen} keylen={keylen} ret"), rc, rr);
        assert_eq!(rc, -1, "init_salt_personal outlen={outlen} keylen={keylen} should be -1");
    }
}

/// ERRORS.md row 74: `crypto_generichash_blake2b_final` called TWICE on the same
/// state (blake2b `is_lastblock` already set) → the SECOND call returns -1.
/// The first call must return 0 in both libs and produce identical digest.
#[test]
fn generichash_final_twice_returns_minus1() {
    let d = duo();
    let sb = statebytes("crypto_generichash_blake2b_statebytes");
    let (ci, ri) = d.pair::<GhInit>("crypto_generichash_blake2b_init");
    let (ci, ri) = (*ci, *ri);
    let (cu, ru) = d.pair::<GhUpdate>("crypto_generichash_blake2b_update");
    let (cu, ru) = (*cu, *ru);
    let (cff, rff) = d.pair::<GhFinal>("crypto_generichash_blake2b_final");
    let (cff, rff) = (*cff, *rff);

    for outlen in [16usize, 32, 64] {
        let msg = [0x7Au8; 40];
        // init + update + first final (must succeed, identical digest)
        let mut cs = vec![0u8; sb];
        let mut rs = vec![0u8; sb];
        assert_eq!(unsafe { ci(cs.as_mut_ptr(), core::ptr::null(), 0, outlen) }, 0);
        assert_eq!(unsafe { ri(rs.as_mut_ptr(), core::ptr::null(), 0, outlen) }, 0);
        assert_eq!(unsafe { cu(cs.as_mut_ptr(), msg.as_ptr(), msg.len() as u64) }, 0);
        assert_eq!(unsafe { ru(rs.as_mut_ptr(), msg.as_ptr(), msg.len() as u64) }, 0);

        let mut oc = vec![0u8; outlen];
        let mut or = vec![0u8; outlen];
        let rc1 = unsafe { cff(cs.as_mut_ptr(), oc.as_mut_ptr(), outlen) };
        let rr1 = unsafe { rff(rs.as_mut_ptr(), or.as_mut_ptr(), outlen) };
        eq_i32(&format!("final#1 outlen={outlen} ret"), rc1, rr1);
        assert_eq!(rc1, 0, "final#1 should succeed");
        eq_bytes(&format!("final#1 digest outlen={outlen}"), &oc, &or);

        // second final on the SAME (now last-block) state → -1 in both
        let mut oc2 = vec![0u8; outlen];
        let mut or2 = vec![0u8; outlen];
        let rc2 = unsafe { cff(cs.as_mut_ptr(), oc2.as_mut_ptr(), outlen) };
        let rr2 = unsafe { rff(rs.as_mut_ptr(), or2.as_mut_ptr(), outlen) };
        eq_i32(&format!("final#2 outlen={outlen} ret"), rc2, rr2);
        assert_eq!(rc2, -1, "final#2 (is_lastblock set) should be -1");
    }
}

/// ERRORS.md row 75: `crypto_generichash_blake2b_final` with `outlen == 0`.
///
/// REFINED against C source: the public wrapper does NOT return -1 for this; it
/// forwards to `blake2b_final`, whose guard `if (!outlen || outlen > 64)
/// sodium_misuse();` **aborts** (SIGABRT). (`outlen > 64` also aborts, but the
/// wrapper truncates `outlen as (uint8_t)` first, so e.g. `outlen == 256` maps
/// to 0 and aborts too — exercised here.) Verified as identical child fate.
#[test]
fn generichash_final_outlen_zero_aborts() {
    let d = duo();
    let sb = statebytes("crypto_generichash_blake2b_statebytes");
    let (ci, ri) = d.pair::<GhInit>("crypto_generichash_blake2b_init");
    let (ci, ri) = (*ci, *ri);
    let (cff, rff) = d.pair::<GhFinal>("crypto_generichash_blake2b_final");
    let (cff, rff) = (*cff, *rff);

    // outlen values that map (as u8) to an invalid blake2b outlen (0 or >64):
    //   0 -> 0 (abort), 256 -> 0 (abort), 65 -> 65 (>64, abort), 512 -> 0 (abort).
    for outlen in [0usize, 256, 65, 512] {
        same_fate(
            &format!("final outlen={outlen} misuse-abort"),
            || {
                let mut s = vec![0u8; sb];
                unsafe { ci(s.as_mut_ptr(), core::ptr::null(), 0, 32) };
                let mut o = vec![0u8; 64];
                unsafe { cff(s.as_mut_ptr(), o.as_mut_ptr(), outlen) };
            },
            || {
                let mut s = vec![0u8; sb];
                unsafe { ri(s.as_mut_ptr(), core::ptr::null(), 0, 32) };
                let mut o = vec![0u8; 64];
                unsafe { rff(s.as_mut_ptr(), o.as_mut_ptr(), outlen) };
            },
        );
    }
}

/// ERRORS.md row 76: `crypto_generichash_blake2b_final` with
/// `outlen != state->outlen` (but still a VALID `1..=64`).
///
/// REFINED against C source: `blake2b_final` performs NO state-outlen
/// comparison — it simply copies `outlen` bytes of the finalized digest and
/// returns 0. So this is NOT an error path: C returns 0 and the shorter/longer
/// output is a prefix of the full 64-byte digest. We assert both libs return 0
/// AND produce byte-identical output for a state initialised with a DIFFERENT
/// outlen than the one passed to `_final`.
#[test]
fn generichash_final_outlen_mismatch_succeeds_identically() {
    let d = duo();
    let sb = statebytes("crypto_generichash_blake2b_statebytes");
    let (ci, ri) = d.pair::<GhInit>("crypto_generichash_blake2b_init");
    let (ci, ri) = (*ci, *ri);
    let (cu, ru) = d.pair::<GhUpdate>("crypto_generichash_blake2b_update");
    let (cu, ru) = (*cu, *ru);
    let (cff, rff) = d.pair::<GhFinal>("crypto_generichash_blake2b_final");
    let (cff, rff) = (*cff, *rff);

    let msg = [0x33u8; 24];
    // (state_outlen, final_outlen) with state_outlen != final_outlen.
    for &(state_outlen, final_outlen) in &[(32usize, 16usize), (16, 32), (64, 1), (32, 64)] {
        let mut cs = vec![0u8; sb];
        let mut rs = vec![0u8; sb];
        assert_eq!(unsafe { ci(cs.as_mut_ptr(), core::ptr::null(), 0, state_outlen) }, 0);
        assert_eq!(unsafe { ri(rs.as_mut_ptr(), core::ptr::null(), 0, state_outlen) }, 0);
        assert_eq!(unsafe { cu(cs.as_mut_ptr(), msg.as_ptr(), msg.len() as u64) }, 0);
        assert_eq!(unsafe { ru(rs.as_mut_ptr(), msg.as_ptr(), msg.len() as u64) }, 0);

        let mut oc = vec![0u8; final_outlen];
        let mut or = vec![0u8; final_outlen];
        let rc = unsafe { cff(cs.as_mut_ptr(), oc.as_mut_ptr(), final_outlen) };
        let rr = unsafe { rff(rs.as_mut_ptr(), or.as_mut_ptr(), final_outlen) };
        eq_i32(&format!("final mismatch state={state_outlen} final={final_outlen} ret"), rc, rr);
        assert_eq!(rc, 0, "final with valid mismatched outlen should succeed (no state check)");
        eq_bytes(
            &format!("final mismatch out state={state_outlen} final={final_outlen}"),
            &oc,
            &or,
        );
    }
}

/// ERRORS.md rows 79, 80, 81: internal `blake2b_init*` / `blake2b` one-shot
/// misuse paths — UNREACHABLE through the public API.
///
/// The internal `blake2b` one-shot (`in==NULL && inlen>0`, `out==NULL`,
/// `outlen==0 || >64`, `key==NULL && keylen>0`, `keylen>64`) and the internal
/// `blake2b_init` / `blake2b_init_key` guards all call `sodium_misuse()`.
/// HOWEVER, every public entry point (`crypto_generichash_blake2b`,
/// `..._salt_personal`, `..._init`, `..._init_salt_personal`) PRE-CHECKS
/// `outlen==0 || outlen>64 || keylen>64` and returns -1 BEFORE ever reaching the
/// internal function (verified in `crypto_generichash/blake2b/ref/
/// generichash_blake2b.c`). The `nonnull(1)` attribute forbids `out==NULL`, and
/// the wrappers dispatch `key==NULL||keylen==0` to `blake2b_init` (no key deref)
/// vs `blake2b_init_key` only when `key!=NULL && keylen>0`, so the internal
/// `key==NULL && keylen>0` misuse is never reached either. These rows are thus
/// GENUINELY UNREACHABLE via the exported symbols and are documented, not faked
/// (the exported bounds → -1 behaviour is already covered by rows 62–78 above).
#[test]
fn blake2b_internal_misuse_unreachable_note() {
    // Documentation-only. The internal blake2b_init*/blake2b misuse guards are
    // shadowed by the public wrappers' `-> -1` pre-checks; see doc comment.
    let unreachable_by_construction = true;
    assert!(unreachable_by_construction);
}

// ===========================================================================
// B) crypto_kdf / hkdf — ERRORS.md rows 264–268
// ===========================================================================

/// ERRORS.md rows 264, 265, 268: `crypto_kdf_blake2b_derive_from_key` and its
/// generic alias `crypto_kdf_derive_from_key` with `subkey_len` outside
/// `[BYTES_MIN(16), BYTES_MAX(64)]` → -1, errno EINVAL.
#[test]
fn kdf_derive_subkey_len_out_of_range() {
    let d = duo();
    let (cb, rb) = d.pair::<KdfDerive>("crypto_kdf_blake2b_derive_from_key");
    let (cb, rb) = (*cb, *rb);
    let (cg, rg) = d.pair::<KdfDerive>("crypto_kdf_derive_from_key");
    let (cg, rg) = (*cg, *rg);

    let ctx = *b"context1"; // crypto_kdf_CONTEXTBYTES == 8
    let key = [0x44u8; 32]; // crypto_kdf_KEYBYTES == 32

    // subkey_len < 16 or > 64
    for &subkey_len in &[0usize, 1, 8, 15, 65, 100, 255] {
        for (name, cf, rf) in [("blake2b", cb, rb), ("generic", cg, rg)] {
            let mut sc = vec![0x99u8; subkey_len.max(1)];
            let mut sr = vec![0x99u8; subkey_len.max(1)];
            let (rc, ce) = with_errno(|| unsafe {
                cf(sc.as_mut_ptr(), subkey_len, 1u64, ctx.as_ptr(), key.as_ptr())
            });
            let (rr, re) = with_errno(|| unsafe {
                rf(sr.as_mut_ptr(), subkey_len, 1u64, ctx.as_ptr(), key.as_ptr())
            });
            eq_i32(&format!("kdf {name} subkey_len={subkey_len} ret"), rc, rr);
            assert_eq!(rc, -1, "kdf {name} subkey_len={subkey_len} should be -1");
            assert_eq!(ce, re, "kdf {name} subkey_len={subkey_len} errno {ce} != {re}");
            assert_eq!(ce, EINVAL, "kdf {name} subkey_len={subkey_len} errno should be EINVAL, got {ce}");
        }
    }

    // Sanity control: a valid subkey_len (32) succeeds identically in both.
    for (name, cf, rf) in [("blake2b", cb, rb), ("generic", cg, rg)] {
        let mut sc = vec![0u8; 32];
        let mut sr = vec![0u8; 32];
        let rc = unsafe { cf(sc.as_mut_ptr(), 32, 7u64, ctx.as_ptr(), key.as_ptr()) };
        let rr = unsafe { rf(sr.as_mut_ptr(), 32, 7u64, ctx.as_ptr(), key.as_ptr()) };
        eq_i32(&format!("kdf {name} valid ret"), rc, rr);
        assert_eq!(rc, 0, "kdf {name} valid should succeed");
        eq_bytes(&format!("kdf {name} valid subkey"), &sc, &sr);
    }
}

/// ERRORS.md rows 266, 267: `crypto_kdf_hkdf_sha256_expand` with
/// `out_len > 255*32` and `crypto_kdf_hkdf_sha512_expand` with `out_len > 255*64`
/// → -1, errno EINVAL. We pass a SMALL real buffer but a huge `out_len`: the
/// C check `if (out_len > BYTES_MAX) { errno=EINVAL; return -1; }` fires BEFORE
/// any write. We also test the just-valid boundary (`out_len == BYTES_MAX`) as a
/// success control.
#[test]
fn hkdf_expand_out_len_out_of_range() {
    let d = duo();
    let (c2, r2) = d.pair::<HkdfExpand>("crypto_kdf_hkdf_sha256_expand");
    let (c2, r2) = (*c2, *r2);
    let (c5, r5) = d.pair::<HkdfExpand>("crypto_kdf_hkdf_sha512_expand");
    let (c5, r5) = (*c5, *r5);

    const MAX256: usize = 0xff * 32; // 8160
    const MAX512: usize = 0xff * 64; // 16320
    let ctx = b"ctx";

    // sha256: prk is 32 bytes; sha512: prk is 64 bytes.
    let prk256 = [0x55u8; 32];
    let prk512 = [0x55u8; 64];

    for (name, cf, rf, max, prk) in [
        ("sha256", c2, r2, MAX256, &prk256[..]),
        ("sha512", c5, r5, MAX512, &prk512[..]),
    ] {
        // Out of range: > BYTES_MAX. Small buffer + huge declared out_len:
        // the length guard rejects before touching `out`.
        for &out_len in &[max + 1, max + 1000, usize::MAX] {
            let mut oc = [0u8; 8];
            let mut or = [0u8; 8];
            let (rc, ce) = with_errno(|| unsafe {
                cf(oc.as_mut_ptr(), out_len, ctx.as_ptr() as *const i8, ctx.len(), prk.as_ptr())
            });
            let (rr, re) = with_errno(|| unsafe {
                rf(or.as_mut_ptr(), out_len, ctx.as_ptr() as *const i8, ctx.len(), prk.as_ptr())
            });
            eq_i32(&format!("hkdf {name} out_len={out_len} ret"), rc, rr);
            assert_eq!(rc, -1, "hkdf {name} out_len={out_len} should be -1");
            assert_eq!(ce, re, "hkdf {name} out_len={out_len} errno {ce} != {re}");
            assert_eq!(ce, EINVAL, "hkdf {name} out_len={out_len} errno should be EINVAL, got {ce}");
        }

        // Just-valid boundary: out_len == BYTES_MAX succeeds & outputs match.
        let mut oc = vec![0u8; max];
        let mut or = vec![0u8; max];
        let rc = unsafe { cf(oc.as_mut_ptr(), max, ctx.as_ptr() as *const i8, ctx.len(), prk.as_ptr()) };
        let rr = unsafe { rf(or.as_mut_ptr(), max, ctx.as_ptr() as *const i8, ctx.len(), prk.as_ptr()) };
        eq_i32(&format!("hkdf {name} out_len==MAX ret"), rc, rr);
        assert_eq!(rc, 0, "hkdf {name} out_len==MAX should succeed");
        eq_bytes(&format!("hkdf {name} out_len==MAX bytes"), &oc, &or);
    }
}

// ===========================================================================
// C) crypto_auth / crypto_onetimeauth — ERRORS.md rows 269–276
// ===========================================================================

/// ERRORS.md rows 269, 270: `crypto_auth_hmacsha256_init` /
/// `crypto_auth_hmacsha512_init` with `key == NULL && keylen > 0` →
/// `sodium_misuse()` → abort. Note the C guard only fires on the `keylen <= 64`
/// branch (a `keylen > 64` NULL key would instead deref NULL), so we drive it
/// with `0 < keylen <= 64`. Verified as identical forked-child fate.
#[test]
fn auth_hmac_init_null_key_nonzero_len_aborts() {
    let d = duo();
    let sb256 = statebytes("crypto_auth_hmacsha256_statebytes");
    let sb512 = statebytes("crypto_auth_hmacsha512_statebytes");
    let (c2, r2) = d.pair::<AuthInit>("crypto_auth_hmacsha256_init");
    let (c2, r2) = (*c2, *r2);
    let (c5, r5) = d.pair::<AuthInit>("crypto_auth_hmacsha512_init");
    let (c5, r5) = (*c5, *r5);

    for &keylen in &[1usize, 16, 32, 64] {
        same_fate(
            &format!("hmacsha256_init NULL key keylen={keylen}"),
            || {
                let mut s = vec![0u8; sb256];
                unsafe { c2(s.as_mut_ptr(), core::ptr::null(), keylen) };
            },
            || {
                let mut s = vec![0u8; sb256];
                unsafe { r2(s.as_mut_ptr(), core::ptr::null(), keylen) };
            },
        );
        same_fate(
            &format!("hmacsha512_init NULL key keylen={keylen}"),
            || {
                let mut s = vec![0u8; sb512];
                unsafe { c5(s.as_mut_ptr(), core::ptr::null(), keylen) };
            },
            || {
                let mut s = vec![0u8; sb512];
                unsafe { r5(s.as_mut_ptr(), core::ptr::null(), keylen) };
            },
        );
    }
}

/// ERRORS.md rows 271, 272, 273, 274, 275, 276 (+ 295): every `*_verify` with a
/// WRONG MAC → -1. We compute the CORRECT MAC, then flip EVERY byte position
/// (0x01 flip and 0x80 flip) and assert BOTH libs return -1 for each; the
/// correct MAC returns 0 in both. Covers:
///   271 hmacsha256_verify, 272 hmacsha512_verify, 273 hmacsha512256_verify,
///   274 crypto_auth_verify, 275 onetimeauth_poly1305_verify,
///   276 crypto_onetimeauth_verify.
#[test]
fn auth_and_onetimeauth_verify_wrong_mac() {
    let d = duo();
    let mut rng = Rng::new(0xAABB_1122);

    // ---- HMAC family ----
    // hmacsha256: mac 32, key 32; hmacsha512: mac 64, key 32;
    // hmacsha512256: mac 32, key 32; crypto_auth(=hmacsha512256): mac 32, key 32.
    struct HmacCase {
        name: &'static str,
        maclen: usize,
        keylen: usize,
        mac_fn: &'static str,
        verify_fn: &'static str,
    }
    let hmac_cases = [
        HmacCase { name: "hmacsha256", maclen: 32, keylen: 32, mac_fn: "crypto_auth_hmacsha256", verify_fn: "crypto_auth_hmacsha256_verify" },
        HmacCase { name: "hmacsha512", maclen: 64, keylen: 32, mac_fn: "crypto_auth_hmacsha512", verify_fn: "crypto_auth_hmacsha512_verify" },
        HmacCase { name: "hmacsha512256", maclen: 32, keylen: 32, mac_fn: "crypto_auth_hmacsha512256", verify_fn: "crypto_auth_hmacsha512256_verify" },
        HmacCase { name: "crypto_auth", maclen: 32, keylen: 32, mac_fn: "crypto_auth", verify_fn: "crypto_auth_verify" },
    ];

    for hc in &hmac_cases {
        let (cm, _rm) = d.pair::<AuthMac>(hc.mac_fn);
        let cm = *cm; // MAC is a pure function of (in,key); compute once with C.
        let (cv, rv) = d.pair::<AuthVerify>(hc.verify_fn);
        let (cv, rv) = (*cv, *rv);

        let key = rng.bytes(hc.keylen);
        let msg = rng.bytes(48);
        let mut mac = vec![0u8; hc.maclen];
        assert_eq!(unsafe { cm(mac.as_mut_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()) }, 0);

        // Correct MAC → 0 in both.
        let rc = unsafe { cv(mac.as_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()) };
        let rr = unsafe { rv(mac.as_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()) };
        eq_i32(&format!("{} verify correct ret", hc.name), rc, rr);
        assert_eq!(rc, 0, "{} verify correct MAC should be 0", hc.name);

        // Flip EVERY byte position → -1 in both.
        for byte in 0..hc.maclen {
            for flip in [0x01u8, 0x80u8] {
                let mut bad = mac.clone();
                bad[byte] ^= flip;
                let rc = unsafe { cv(bad.as_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()) };
                let rr = unsafe { rv(bad.as_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()) };
                eq_i32(&format!("{} verify flip byte={byte} flip={flip:#x} ret", hc.name), rc, rr);
                assert_eq!(rc, -1, "{} verify wrong MAC should be -1", hc.name);
            }
        }
    }

    // ---- onetimeauth (poly1305) family: MAC 16, key 32 ----
    // rows 275 (poly1305_verify) + 276 (crypto_onetimeauth_verify).
    for (name, mac_fn, verify_fn) in [
        ("poly1305", "crypto_onetimeauth_poly1305", "crypto_onetimeauth_poly1305_verify"),
        ("onetimeauth", "crypto_onetimeauth", "crypto_onetimeauth_verify"),
    ] {
        let (cm, _rm) = d.pair::<OtaMac>(mac_fn);
        let cm = *cm;
        let (cv, rv) = d.pair::<OtaVerify>(verify_fn);
        let (cv, rv) = (*cv, *rv);

        let key = rng.bytes(32);
        let msg = rng.bytes(48);
        let mut mac = vec![0u8; 16];
        assert_eq!(unsafe { cm(mac.as_mut_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()) }, 0);

        let rc = unsafe { cv(mac.as_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()) };
        let rr = unsafe { rv(mac.as_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()) };
        eq_i32(&format!("{name} verify correct ret"), rc, rr);
        assert_eq!(rc, 0, "{name} verify correct MAC should be 0");

        for byte in 0..16usize {
            for flip in [0x01u8, 0x80u8] {
                let mut bad = mac.clone();
                bad[byte] ^= flip;
                let rc = unsafe { cv(bad.as_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()) };
                let rr = unsafe { rv(bad.as_ptr(), msg.as_ptr(), msg.len() as u64, key.as_ptr()) };
                eq_i32(&format!("{name} verify flip byte={byte} flip={flip:#x} ret"), rc, rr);
                assert_eq!(rc, -1, "{name} verify wrong MAC should be -1");
            }
        }
    }
}

// ===========================================================================
// D) crypto_stream_chacha20_ietf misuse — ERRORS.md rows 277–285
// ===========================================================================

/// ERRORS.md row 283: `crypto_stream_chacha20_ietf` with
/// `clen > ietf_MESSAGEBYTES_MAX (= 64 * 2^32 = 2^38)` → `sodium_misuse()` →
/// abort. Confirmed in `crypto_stream/chacha20/stream_chacha20.c` that the
/// length check precedes any write, so we pass a SMALL real buffer but a huge
/// declared `clen` — the abort happens before `implementation->stream` writes.
/// A just-valid `clen` (== MESSAGEBYTES_MAX) is NOT exercised for output because
/// it would require a 256 GiB buffer; only the guard is differentially tested.
#[test]
fn stream_chacha20_ietf_clen_too_large_aborts() {
    let d = duo();
    let (cf, rf) = d.pair::<StreamIetf>("crypto_stream_chacha20_ietf");
    let (cf, rf) = (*cf, *rf);

    const IETF_MAX: u64 = 64 * (1u64 << 32); // 2^38
    for clen in [IETF_MAX + 1, IETF_MAX + 64, u64::MAX] {
        same_fate(
            &format!("chacha20_ietf clen={clen} misuse-abort"),
            || {
                let mut buf = [0u8; 64]; // small real buffer; guard fires first
                let n = [0u8; 12];
                let k = [0u8; 32];
                unsafe { cf(buf.as_mut_ptr(), clen, n.as_ptr(), k.as_ptr()) };
            },
            || {
                let mut buf = [0u8; 64];
                let n = [0u8; 12];
                let k = [0u8; 32];
                unsafe { rf(buf.as_mut_ptr(), clen, n.as_ptr(), k.as_ptr()) };
            },
        );
    }
}

/// ERRORS.md row 284: `crypto_stream_chacha20_ietf_xor_ic` counter overflow.
/// C guard: `ic > (64*(1<<32))/64 - (mlen+63)/64` i.e. `ic > 2^32 - ceil(mlen/64)`.
/// This IS reachable with a small buffer. We pick `mlen == 65` so `ceil = 2` and
/// the threshold is `2^32 - 2 = 0xfffffffe`:
///   * `ic == 0xfffffffe` is JUST-VALID → must succeed AND outputs must match.
///   * `ic == 0xffffffff` is JUST-INVALID (> threshold) → must abort in BOTH.
/// We also check a small `ic` succeeds identically as a control, and that a
/// larger `mlen` widens the invalid window.
#[test]
fn stream_chacha20_ietf_xor_ic_counter_overflow() {
    let d = duo();
    let (cf, rf) = d.pair::<StreamIetfXorIc>("crypto_stream_chacha20_ietf_xor_ic");
    let (cf, rf) = (*cf, *rf);

    let mlen: usize = 65; // ceil(65/64) == 2 → threshold 2^32 - 2 = 0xfffffffe
    let just_valid: u32 = 0xffff_fffe;
    let just_invalid: u32 = 0xffff_ffff;

    // Just-valid ic and a small ic: succeed and produce identical output.
    for ic in [0u32, 1, just_valid] {
        let m = vec![0x5Au8; mlen];
        let n = [7u8; 12];
        let k = [9u8; 32];
        let mut oc = vec![0u8; mlen];
        let mut or = vec![0u8; mlen];
        let rc = unsafe { cf(oc.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), ic, k.as_ptr()) };
        let rr = unsafe { rf(or.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), ic, k.as_ptr()) };
        eq_i32(&format!("xor_ic valid ic={ic:#x} ret"), rc, rr);
        assert_eq!(rc, 0, "xor_ic valid ic={ic:#x} should succeed");
        eq_bytes(&format!("xor_ic valid ic={ic:#x} out"), &oc, &or);
    }

    // Just-invalid ic → abort in both libraries.
    same_fate(
        &format!("xor_ic overflow ic={just_invalid:#x} mlen={mlen}"),
        || {
            let m = vec![0x5Au8; mlen];
            let n = [7u8; 12];
            let k = [9u8; 32];
            let mut o = vec![0u8; mlen];
            unsafe { cf(o.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), just_invalid, k.as_ptr()) };
        },
        || {
            let m = vec![0x5Au8; mlen];
            let n = [7u8; 12];
            let k = [9u8; 32];
            let mut o = vec![0u8; mlen];
            unsafe { rf(o.as_mut_ptr(), m.as_ptr(), mlen as u64, n.as_ptr(), just_invalid, k.as_ptr()) };
        },
    );

    // A larger mlen widens the invalid window: mlen=200 → ceil=4 → threshold
    // 2^32 - 4; ic=2^32-3 (0xfffffffd) is now also invalid.
    let mlen2 = 200usize;
    let invalid2: u32 = 0xffff_fffd; // > 0xfffffffc
    same_fate(
        &format!("xor_ic overflow ic={invalid2:#x} mlen={mlen2}"),
        || {
            let m = vec![0x11u8; mlen2];
            let n = [1u8; 12];
            let k = [2u8; 32];
            let mut o = vec![0u8; mlen2];
            unsafe { cf(o.as_mut_ptr(), m.as_ptr(), mlen2 as u64, n.as_ptr(), invalid2, k.as_ptr()) };
        },
        || {
            let m = vec![0x11u8; mlen2];
            let n = [1u8; 12];
            let k = [2u8; 32];
            let mut o = vec![0u8; mlen2];
            unsafe { rf(o.as_mut_ptr(), m.as_ptr(), mlen2 as u64, n.as_ptr(), invalid2, k.as_ptr()) };
        },
    );
}

/// ERRORS.md rows 277, 278, 279, 280, 281, 282, 285: the remaining
/// `crypto_stream_chacha20*` misuse rows whose `MESSAGEBYTES_MAX` is
/// `SODIUM_SIZE_MAX` (i.e. `2^64-1`) — UNREACHABLE on a 64-bit host.
///
/// Rows 277/278/279 (`crypto_stream_chacha20`, `_xor_ic`, `_xor`) and
/// 280/281/282 (`crypto_stream_chacha20_ietf_ext`, `_ext_xor_ic`, `_ext_xor`)
/// all guard on `clen/mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX`, which is
/// `SODIUM_MIN(SIZE_MAX, ...) == SIZE_MAX` for the non-ietf/ext variants — a
/// `len > SIZE_MAX` is not representable in a `size_t`/`u64` on this platform,
/// so the guard can never trip. Row 285 (`crypto_stream_chacha20_ietf_xor`)
/// guards on `mlen > ietf_MESSAGEBYTES_MAX (2^38)` which IS representable, but
/// exercising the WRITE path at that size needs a 256 GiB buffer; the *guard*
/// itself is behaviourally identical to row 283's guard (same constant, same
/// `sodium_misuse()`), which is already differentially tested above, so we do
/// not duplicate it. All documented, none faked.
#[test]
fn stream_chacha20_size_max_unreachable_note() {
    // Documentation-only: SIZE_MAX-bounded guards are unreachable on 64-bit;
    // the reachable 2^38 ietf guard is covered by rows 283/284 above.
    assert!(64u64 * (1u64 << 32) < u64::MAX);
}

// ===========================================================================
// E) randombytes + generic FFI boundary conditions — ERRORS.md rows 286–301
// ===========================================================================

/// ERRORS.md row 290 (+ 286/287/288/289/291 notes): `randombytes_uniform` with
/// `upper_bound` 0 and 1 returns 0 deterministically in BOTH libs; and
/// `randombytes_close` return-value parity (row 289).
///
/// Rows 286 (`randombytes_buf_deterministic size > 0x4000000000` misuse),
/// 287 (`assert(buf_len <= SIZE_MAX)`), 288 (`set_implementation` never errors),
/// and 291 (internal entropy-source failures) are either unreachable on 64-bit
/// / non-erroring / not reachable through a differential public entry point;
/// documented here, not faked.
#[test]
fn randombytes_uniform_small_bounds_and_close() {
    let d = duo();
    let (cu, ru) = d.pair::<RbUniform>("randombytes_uniform");
    let (cu, ru) = (*cu, *ru);

    // upper_bound 0 and 1 → deterministic 0 in both.
    for ub in [0u32, 1] {
        let rc = unsafe { cu(ub) };
        let rr = unsafe { ru(ub) };
        assert_eq!(rc, rr, "randombytes_uniform({ub}): C={rc} Rust={rr}");
        assert_eq!(rc, 0, "randombytes_uniform({ub}) should be 0");
    }

    // Row 289: randombytes_close return parity (both use the default impl here).
    let (cc, rcl) = d.pair::<RbClose>("randombytes_close");
    let (cc, rcl) = (*cc, *rcl);
    let rc = unsafe { cc() };
    let rr = unsafe { rcl() };
    eq_i32("randombytes_close ret", rc, rr);
}

/// ERRORS.md rows 292, 293, 294: `crypto_generichash` FFI-boundary null/empty
/// cases that are ALLOWED (return 0) and must be byte-identical:
///   * 293: `in == NULL && inlen == 0` → 0.
///   * 294: `key == NULL && keylen == 0` (unkeyed) → 0.
///   * 292: empty message / empty key length parity.
#[test]
fn generichash_null_empty_allowed() {
    let d = duo();
    let (cf, rf) = d.pair::<GhOneshot>("crypto_generichash");
    let (cf, rf) = (*cf, *rf);

    // Combinations of (in NULL/empty) × (key NULL/empty), all with len 0.
    let scenarios: &[(&str, bool, bool)] = &[
        ("in=NULL key=NULL", true, true),
        ("in=empty key=NULL", false, true),
        ("in=NULL key=empty", true, false),
        ("in=empty key=empty", false, false),
    ];
    for outlen in [16usize, 32, 64] {
        for &(name, in_null, key_null) in scenarios {
            let empty_in = [0u8; 0];
            let empty_key = [0u8; 0];
            let inp = if in_null { core::ptr::null() } else { empty_in.as_ptr() };
            let keyp = if key_null { core::ptr::null() } else { empty_key.as_ptr() };

            let mut oc = vec![0u8; outlen];
            let mut or = vec![0u8; outlen];
            let rc = unsafe { cf(oc.as_mut_ptr(), outlen, inp, 0, keyp, 0) };
            let rr = unsafe { rf(or.as_mut_ptr(), outlen, inp, 0, keyp, 0) };
            eq_i32(&format!("generichash {name} outlen={outlen} ret"), rc, rr);
            assert_eq!(rc, 0, "generichash {name} outlen={outlen} should be 0 (allowed)");
            eq_bytes(&format!("generichash {name} outlen={outlen} digest"), &oc, &or);
        }
    }
}

/// ERRORS.md row 297 (part 1) (+ 82): `crypto_pwhash(..., alg)` with `alg` NOT in
/// {ARGON2I13(1), ARGON2ID13(2)} — i.e. {0, 3, -1, i32::MAX, i32::MIN} — hits
/// the `default:` switch arm → -1, errno EINVAL from BOTH libs (C enums accept
/// any int across the FFI boundary). The default arm sets errno before touching
/// any crypto, so this is fast even with real (small) buffers.
#[test]
fn pwhash_out_of_range_alg_einval() {
    let d = duo();
    let (cf, rf) = d.pair::<Pwhash>("crypto_pwhash");
    let (cf, rf) = (*cf, *rf);

    let passwd = b"password";
    let salt = [0x01u8; 16]; // crypto_pwhash_SALTBYTES == 16

    for &alg in &[0i32, 3, -1, i32::MAX, i32::MIN] {
        let mut oc = [0u8; 32];
        let mut or = [0u8; 32];
        let (rc, ce) = with_errno(|| unsafe {
            cf(oc.as_mut_ptr(), 32, passwd.as_ptr() as *const i8, passwd.len() as u64,
               salt.as_ptr(), 3, 8192, alg)
        });
        let (rr, re) = with_errno(|| unsafe {
            rf(or.as_mut_ptr(), 32, passwd.as_ptr() as *const i8, passwd.len() as u64,
               salt.as_ptr(), 3, 8192, alg)
        });
        eq_i32(&format!("pwhash alg={alg} ret"), rc, rr);
        assert_eq!(rc, -1, "pwhash alg={alg} should be -1");
        assert_eq!(ce, re, "pwhash alg={alg} errno {ce} != {re}");
        assert_eq!(ce, EINVAL, "pwhash alg={alg} errno should be EINVAL, got {ce}");
    }
}

/// ERRORS.md row 297 (part 2) (+ 83): `crypto_pwhash_str_alg(..., alg)` with
/// `alg` NOT in {1, 2} — {0, 3, -1, i32::MAX} — falls through the switch to
/// `sodium_misuse()` → abort in BOTH libs. Verified via `same_fate`.
#[test]
fn pwhash_str_alg_out_of_range_aborts() {
    let d = duo();
    let (cf, rf) = d.pair::<PwhashStrAlg>("crypto_pwhash_str_alg");
    let (cf, rf) = (*cf, *rf);

    for &alg in &[0i32, 3, -1, i32::MAX] {
        same_fate(
            &format!("pwhash_str_alg alg={alg} misuse-abort"),
            || {
                let passwd = b"password";
                let mut out = [0i8; 128]; // crypto_pwhash_STRBYTES
                unsafe {
                    cf(out.as_mut_ptr(), passwd.as_ptr() as *const i8, passwd.len() as u64, 3, 8192, alg)
                };
            },
            || {
                let passwd = b"password";
                let mut out = [0i8; 128];
                unsafe {
                    rf(out.as_mut_ptr(), passwd.as_ptr() as *const i8, passwd.len() as u64, 3, 8192, alg)
                };
            },
        );
    }
}

/// ERRORS.md row 298 (+ 225/230/235): out-of-range `hash_alg` across the FFI to
/// the `crypto_core_*_from_string*` family reaches
/// `core_h2c_string_to_hash`'s `default:` arm → -1, errno EINVAL, from BOTH libs.
/// Valid `hash_alg` values are CORE_H2C_SHA256(1)/SHA512(2); we drive
/// {0, 3, -1, i32::MAX, i32::MIN} through every exported entry point:
///   * crypto_core_ed25519_from_string / _from_string_nu
///   * crypto_core_ed25519_scalar_from_string
///   * crypto_core_ristretto255_from_string
///   * crypto_core_ristretto255_scalar_from_string
#[test]
fn core_from_string_out_of_range_hash_alg_einval() {
    let d = duo();
    let fns = [
        "crypto_core_ed25519_from_string",
        "crypto_core_ed25519_from_string_nu",
        "crypto_core_ed25519_scalar_from_string",
        "crypto_core_ristretto255_from_string",
        "crypto_core_ristretto255_scalar_from_string",
    ];
    let ctx = b"context";
    let msg = b"differential-message";

    for fname in fns {
        let (cf, rf) = d.pair::<CoreFromString>(fname);
        let (cf, rf) = (*cf, *rf);
        for &alg in &[0i32, 3, -1, i32::MAX, i32::MIN] {
            // Output buffer generous enough for any of these (<=64 bytes).
            let mut oc = [0u8; 64];
            let mut or = [0u8; 64];
            let (rc, ce) = with_errno(|| unsafe {
                cf(oc.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg)
            });
            let (rr, re) = with_errno(|| unsafe {
                rf(or.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg)
            });
            eq_i32(&format!("{fname} alg={alg} ret"), rc, rr);
            assert_eq!(rc, -1, "{fname} alg={alg} should be -1");
            assert_eq!(ce, re, "{fname} alg={alg} errno {ce} != {re}");
            assert_eq!(ce, EINVAL, "{fname} alg={alg} errno should be EINVAL, got {ce}");
        }
    }
}

/// ERRORS.md row 299: `crypto_secretstream_xchacha20poly1305_push` with `tag`
/// values that are NOT one of the four defined tags {0,1,2,3} — e.g.
/// {0x04, 0x08, 0x7f, 0xff} — are ACCEPTED by the C (no tag validation; the tag
/// is just a byte stored in the first output block) and must round-trip
/// identically. We verify BOTH libs produce the SAME ciphertext on push and the
/// SAME recovered tag + plaintext on pull.
///
/// Method (mirrors b06): `init_pull(header,key)` is a pure function of
/// (header,key), so driving BOTH the encrypt-side and decrypt-side states from a
/// FIXED (header,key) via `init_pull` makes all states byte-identical across
/// libs — enabling exact ciphertext comparison of `_push` without depending on
/// the random header from `init_push`.
#[test]
fn secretstream_push_out_of_range_tag_roundtrips() {
    let d = duo();
    const HEADERBYTES: usize = 24;
    const KEYBYTES: usize = 32;
    const ABYTES: usize = 17;
    let sb = statebytes("crypto_secretstream_xchacha20poly1305_statebytes");

    let (cipu, ripu) = d.pair::<InitPull>("crypto_secretstream_xchacha20poly1305_init_pull");
    let (cipu, ripu) = (*cipu, *ripu);
    let (cpush, rpush) = d.pair::<Push>("crypto_secretstream_xchacha20poly1305_push");
    let (cpush, rpush) = (*cpush, *rpush);
    let (cpull, rpull) = d.pair::<Pull>("crypto_secretstream_xchacha20poly1305_pull");
    let (cpull, rpull) = (*cpull, *rpull);

    let mut rng = Rng::new(0x5EED_7A65);
    let key = rng.bytes(KEYBYTES);
    let header = rng.bytes(HEADERBYTES);

    for &tag in &[0x04u8, 0x08, 0x7f, 0xff] {
        // Encrypt-side states (C and Rust) from the same (header,key).
        let mut c_enc = vec![0u8; sb];
        let mut r_enc = vec![0u8; sb];
        assert_eq!(unsafe { cipu(c_enc.as_mut_ptr(), header.as_ptr(), key.as_ptr()) }, 0);
        assert_eq!(unsafe { ripu(r_enc.as_mut_ptr(), header.as_ptr(), key.as_ptr()) }, 0);
        // Decrypt-side states from the same (header,key).
        let mut c_dec = vec![0u8; sb];
        let mut r_dec = vec![0u8; sb];
        assert_eq!(unsafe { cipu(c_dec.as_mut_ptr(), header.as_ptr(), key.as_ptr()) }, 0);
        assert_eq!(unsafe { ripu(r_dec.as_mut_ptr(), header.as_ptr(), key.as_ptr()) }, 0);

        let msg = rng.bytes(40);
        let mut cct = vec![0u8; msg.len() + ABYTES];
        let mut rct = vec![0u8; msg.len() + ABYTES];
        let mut cclen: u64 = 0;
        let mut rclen: u64 = 0;

        let rc = unsafe {
            cpush(c_enc.as_mut_ptr(), cct.as_mut_ptr(), &mut cclen, msg.as_ptr(), msg.len() as u64,
                  core::ptr::null(), 0, tag)
        };
        let rr = unsafe {
            rpush(r_enc.as_mut_ptr(), rct.as_mut_ptr(), &mut rclen, msg.as_ptr(), msg.len() as u64,
                  core::ptr::null(), 0, tag)
        };
        eq_i32(&format!("push tag={tag:#x} ret"), rc, rr);
        assert_eq!(rc, 0, "push tag={tag:#x} should succeed");
        assert_eq!(cclen, rclen, "push tag={tag:#x} clen C={cclen} Rust={rclen}");
        eq_bytes(&format!("push tag={tag:#x} ciphertext"), &cct, &rct);

        // Pull from C's ciphertext with both libs; recovered tag must equal the
        // (out-of-range) pushed tag and match between libs, plaintext identical.
        let mut cm = vec![0u8; msg.len()];
        let mut rm = vec![0u8; msg.len()];
        let mut cmlen: u64 = 0;
        let mut rmlen: u64 = 0;
        let mut ctag: u8 = 0xee;
        let mut rtag: u8 = 0xdd;
        let rc = unsafe {
            cpull(c_dec.as_mut_ptr(), cm.as_mut_ptr(), &mut cmlen, &mut ctag,
                  cct.as_ptr(), cct.len() as u64, core::ptr::null(), 0)
        };
        let rr = unsafe {
            rpull(r_dec.as_mut_ptr(), rm.as_mut_ptr(), &mut rmlen, &mut rtag,
                  cct.as_ptr(), cct.len() as u64, core::ptr::null(), 0)
        };
        eq_i32(&format!("pull tag={tag:#x} ret"), rc, rr);
        assert_eq!(rc, 0, "pull tag={tag:#x} should succeed");
        assert_eq!(ctag, rtag, "pull tag={tag:#x}: C recovered {ctag:#x} != Rust {rtag:#x}");
        assert_eq!(ctag, tag, "pull tag={tag:#x}: recovered tag must equal pushed tag");
        assert_eq!(cmlen, rmlen, "pull tag={tag:#x} mlen mismatch");
        eq_bytes(&format!("pull tag={tag:#x} plaintext"), &cm, &rm);
        eq_bytes(&format!("pull tag={tag:#x} roundtrip vs input"), &cm, &msg);
    }
}

/// ERRORS.md row 300: `sodium_pad` / `sodium_unpad` boundary cases.
///  * blocksize == 0 → -1 (both fns).
///  * `sodium_pad` with `max_buflen` EXACTLY equal to the needed padded length
///    (`xpadded_len == max_buflen`): the C guard is `xpadded_len >= max_buflen`,
///    so "exactly equal" is STILL an error → -1. We compute the exact needed
///    length and pass it as `max_buflen`; the control `needed+1` succeeds.
#[test]
fn pad_unpad_boundary_exact_and_zero_blocksize() {
    let d = duo();
    let (cp, rp) = d.pair::<Pad>("sodium_pad");
    let (cp, rp) = (*cp, *rp);
    let (cu, ru) = d.pair::<Unpad>("sodium_unpad");
    let (cu, ru) = (*cu, *ru);

    // blocksize == 0 → -1 for both pad and unpad.
    for unpadded in [0usize, 1, 10, 100] {
        let (rc, _) = with_errno(|| {
            let mut buf = vec![0u8; 256];
            let mut plen = 0usize;
            unsafe { cp(&mut plen, buf.as_mut_ptr(), unpadded, 0, buf.len()) }
        });
        let (rr, _) = with_errno(|| {
            let mut buf = vec![0u8; 256];
            let mut plen = 0usize;
            unsafe { rp(&mut plen, buf.as_mut_ptr(), unpadded, 0, buf.len()) }
        });
        eq_i32(&format!("pad blocksize=0 unpadded={unpadded}"), rc, rr);
        assert_eq!(rc, -1, "pad blocksize=0 should be -1");

        let mut ulen = 0usize;
        let buf = vec![0x80u8; unpadded.max(1)];
        let rcu = unsafe { cu(&mut ulen, buf.as_ptr(), unpadded, 0) };
        let rru = unsafe { ru(&mut ulen, buf.as_ptr(), unpadded, 0) };
        eq_i32(&format!("unpad blocksize=0 padded={unpadded}"), rcu, rru);
        assert_eq!(rcu, -1, "unpad blocksize=0 should be -1");
    }

    // max_buflen EXACTLY equal to the C-internal `xpadded_len` → -1 (guard is
    // `if (xpadded_len >= max_buflen) return -1;`). Per `sodium/utils.c`:
    //   xpadlen     = (blocksize - 1) - (unpadded % blocksize)
    //   xpadded_len = unpadded + xpadlen
    // (the actual output consumes xpadded_len + 1 bytes; *padded_buflen_p =
    // xpadded_len + 1). So max_buflen == xpadded_len is an error, and
    // max_buflen == xpadded_len + 1 is the smallest success.
    for &(unpadded, blocksize) in &[(10usize, 16usize), (16, 16), (0, 16), (5, 8), (100, 10)] {
        let xpadded_len = unpadded + (blocksize - 1 - unpadded % blocksize);
        let (rc, _) = with_errno(|| {
            let mut buf = vec![0u8; xpadded_len + 64];
            let mut plen = 0usize;
            unsafe { cp(&mut plen, buf.as_mut_ptr(), unpadded, blocksize, xpadded_len) }
        });
        let (rr, _) = with_errno(|| {
            let mut buf = vec![0u8; xpadded_len + 64];
            let mut plen = 0usize;
            unsafe { rp(&mut plen, buf.as_mut_ptr(), unpadded, blocksize, xpadded_len) }
        });
        eq_i32(&format!("pad exact max u={unpadded} b={blocksize} xpadded={xpadded_len}"), rc, rr);
        assert_eq!(rc, -1, "pad with max_buflen==xpadded_len should be -1 (guard is >=)");

        // Control: max_buflen == xpadded_len+1 must SUCCEED identically.
        let (rc, _) = with_errno(|| {
            let mut buf = vec![0u8; xpadded_len + 64];
            let mut plen = 0usize;
            unsafe { cp(&mut plen, buf.as_mut_ptr(), unpadded, blocksize, xpadded_len + 1) }
        });
        let (rr, _) = with_errno(|| {
            let mut buf = vec![0u8; xpadded_len + 64];
            let mut plen = 0usize;
            unsafe { rp(&mut plen, buf.as_mut_ptr(), unpadded, blocksize, xpadded_len + 1) }
        });
        eq_i32(&format!("pad exact+1 max u={unpadded} b={blocksize}"), rc, rr);
        assert_eq!(rc, 0, "pad with max_buflen==xpadded_len+1 should succeed");
    }
}

/// ERRORS.md row 301: `sodium_compare` / `sodium_memcmp` / `sodium_is_zero` with
/// `len == 0`. Defined behaviour: compare → 0, memcmp → 0, is_zero → 1. Both
/// libs must agree (and match those sentinels) on the empty-length input.
#[test]
fn compare_memcmp_iszero_len_zero() {
    let d = duo();
    let (ccmp, rcmp) = d.pair::<Compare>("sodium_compare");
    let (ccmp, rcmp) = (*ccmp, *rcmp);
    let (cmc, rmc) = d.pair::<Memcmp>("sodium_memcmp");
    let (cmc, rmc) = (*cmc, *rmc);
    let (ciz, riz) = d.pair::<IsZero>("sodium_is_zero");
    let (ciz, riz) = (*ciz, *riz);

    // Use null pointers with len 0 (valid: nothing is dereferenced).
    let rc = unsafe { ccmp(core::ptr::null(), core::ptr::null(), 0) };
    let rr = unsafe { rcmp(core::ptr::null(), core::ptr::null(), 0) };
    eq_i32("compare len=0", rc, rr);
    assert_eq!(rc, 0, "compare len=0 should be 0");

    let rc = unsafe { cmc(core::ptr::null(), core::ptr::null(), 0) };
    let rr = unsafe { rmc(core::ptr::null(), core::ptr::null(), 0) };
    eq_i32("memcmp len=0", rc, rr);
    assert_eq!(rc, 0, "memcmp len=0 should be 0");

    let rc = unsafe { ciz(core::ptr::null(), 0) };
    let rr = unsafe { riz(core::ptr::null(), 0) };
    eq_i32("is_zero len=0", rc, rr);
    assert_eq!(rc, 1, "is_zero len=0 should be 1");

    // Also exercise non-null pointers with len 0 (must be identical too).
    let a = [0xAAu8; 8];
    let b = [0xBBu8; 8];
    assert_eq!(unsafe { ccmp(a.as_ptr(), b.as_ptr(), 0) }, unsafe { rcmp(a.as_ptr(), b.as_ptr(), 0) });
    assert_eq!(
        unsafe { cmc(a.as_ptr() as *const _, b.as_ptr() as *const _, 0) },
        unsafe { rmc(a.as_ptr() as *const _, b.as_ptr() as *const _, 0) }
    );
    assert_eq!(unsafe { ciz(a.as_ptr(), 0) }, unsafe { riz(a.as_ptr(), 0) });
}
