//! Phase B — valid-path differential tests for module group G3:
//! `crypto_generichash/` (blake2b), `crypto_shorthash/` (siphash24,
//! siphashx24), `crypto_onetimeauth/` (poly1305), `crypto_auth/`
//! (hmacsha256/512/512256), `crypto_kdf/` (blake2b, hkdf_sha256, hkdf_sha512),
//! `crypto_hash/` (sha256, sha512, sha3), `crypto_xof/` (shake128/256,
//! turboshake128/256), `crypto_core/keccak1600`.
//!
//! Covers CONFIGS.md rows 281-509 (see the per-test row annotations).
//!
//! Opaque state buffers are sized from the exported `*_statebytes()` accessor
//! and allocated 64-byte aligned. Only OUTPUT bytes (and return values) are
//! compared: the C `*_state` structs are fixed-size `opaque[]` arrays whose
//! tail bytes are never written, so comparing raw state bytes would report
//! spurious mismatches from uninitialised padding.
#![allow(clippy::too_many_arguments)]
mod common;
use common::*;
use std::os::raw::{c_char, c_int};
use std::ptr;

const SEED: u64 = 0x6337_D1FF_5EED_0001;

// ===========================================================================
// small helpers
// ===========================================================================

type Sz = unsafe extern "C" fn() -> usize;
type Uc = unsafe extern "C" fn() -> u8;
type Prim = unsafe extern "C" fn() -> *const c_char;
type Keygen = unsafe extern "C" fn(*mut u8);
type PickBest = unsafe extern "C" fn() -> c_int;

/// One-shot `f(out, in, inlen, key)` shape (hmac / poly1305 / shorthash).
type OneShotK = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
/// `f(h, in, inlen, key)` shape (`*_verify`).
type VerifyK = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;
/// One-shot `f(out, in, inlen)` shape (sha256 / sha512 / sha3 / crypto_hash).
type OneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;

/// `init(state)` / `update(state, in, inlen)` / `final(state, out)`.
type StInit = unsafe extern "C" fn(*mut u8) -> c_int;
type StUpd = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type StFin = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;

/// hmac `init(state, key, keylen)`
type HmacInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
/// poly1305 `init(state, key)`
type PolyInit = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;

// generichash / blake2b
type Gh = unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int;
type GhSp =
    unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize, *const u8, *const u8)
        -> c_int;
type GhInit = unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> c_int;
type GhInitSp =
    unsafe extern "C" fn(*mut u8, *const u8, usize, usize, *const u8, *const u8) -> c_int;
type GhFin = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int;

// xof
type XofOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> c_int;
type XofInitDom = unsafe extern "C" fn(*mut u8, u8) -> c_int;
type XofSqueeze = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int;

// keccak1600
type KecInit = unsafe extern "C" fn(*mut u8);
type KecXor = unsafe extern "C" fn(*mut u8, *const u8, usize, usize);
type KecExtract = unsafe extern "C" fn(*const u8, *mut u8, usize, usize);
type KecPermute = unsafe extern "C" fn(*mut u8);

// kdf
type KdfDerive = unsafe extern "C" fn(*mut u8, usize, u64, *const c_char, *const u8) -> c_int;
type HkdfExtract = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize) -> c_int;
type HkdfExtractInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type HkdfExtractUpd = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type HkdfExtractFin = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type HkdfExpand = unsafe extern "C" fn(*mut u8, usize, *const c_char, usize, *const u8) -> c_int;

macro_rules! pair2 {
    ($t:ty, $n:expr) => {{
        let (a, b) = pair::<$t>($n);
        [*a, *b]
    }};
}

/// A 64-byte-aligned, zero-filled opaque state buffer.
struct St {
    p: *mut u8,
    n: usize,
}

impl St {
    fn new(n: usize) -> St {
        assert!(n > 0);
        let layout = std::alloc::Layout::from_size_align(n, 64).unwrap();
        let p = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!p.is_null(), "state allocation failed");
        St { p, n }
    }
    fn ptr(&self) -> *mut u8 {
        self.p
    }
}

impl Drop for St {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(
                self.p,
                std::alloc::Layout::from_size_align(self.n, 64).unwrap(),
            )
        }
    }
}

/// Read a `size_t f(void)` accessor from both libraries, assert they agree and
/// return the shared value.
fn sz2(name: &str) -> usize {
    unsafe {
        let f = pair2!(Sz, name);
        let a = f[0]();
        let b = f[1]();
        assert_eq!(a, b, "{name}(): C={a} Rust={b}");
        a
    }
}

fn uc2(name: &str) -> u8 {
    unsafe {
        let f = pair2!(Uc, name);
        let a = f[0]();
        let b = f[1]();
        assert_eq!(a, b, "{name}(): C={a} Rust={b}");
        a
    }
}

fn prim2(name: &str) -> String {
    unsafe {
        let f = pair2!(Prim, name);
        let a = std::ffi::CStr::from_ptr(f[0]()).to_string_lossy().into_owned();
        let b = std::ffi::CStr::from_ptr(f[1]()).to_string_lossy().into_owned();
        assert_eq!(a, b, "{name}(): C={a:?} Rust={b:?}");
        a
    }
}

/// Split `data` into consecutive chunks of the given sizes; any tail becomes a
/// final chunk.
fn split<'a>(data: &'a [u8], sizes: &[usize]) -> Vec<&'a [u8]> {
    let mut v: Vec<&[u8]> = Vec::new();
    let mut off = 0usize;
    for &s in sizes {
        assert!(off + s <= data.len(), "split sizes exceed data length");
        v.push(&data[off..off + s]);
        off += s;
    }
    if off < data.len() {
        v.push(&data[off..]);
    }
    v
}

/// Run `init` / N x `update` / `final` for the `(state)`-only init shape.
unsafe fn run_stream(
    init: StInit,
    upd: StUpd,
    fin: StFin,
    nstate: usize,
    outlen: usize,
    chunks: &[&[u8]],
) -> (Vec<u8>, Vec<c_int>) {
    let st = St::new(nstate);
    let mut rets = vec![init(st.ptr())];
    for ch in chunks {
        let p = if ch.is_empty() { ptr::null() } else { ch.as_ptr() };
        rets.push(upd(st.ptr(), p, ch.len() as u64));
    }
    let mut out = vec![0u8; outlen];
    rets.push(fin(st.ptr(), out.as_mut_ptr()));
    (out, rets)
}

// ===========================================================================
// crypto_generichash / blake2b
// ===========================================================================

/// CONFIGS.md rows 281-290: one-shot `crypto_generichash` and
/// `crypto_generichash_blake2b`, unkeyed / keyed, full outlen and keylen
/// sweeps, `key != NULL && keylen == 0`, empty input.
#[test]
fn g3_cfg_281_290_generichash_oneshot() {
    unsafe {
        let gh = pair2!(Gh, "crypto_generichash");
        let b2 = pair2!(Gh, "crypto_generichash_blake2b");
        let mut rng = Rng::new(SEED);

        // --- row 281: unkeyed, outlen=32, inlen=0 with in == NULL -----------
        for _ in 0..4 {
            let mut o = [vec![0xAAu8; 32], vec![0xAAu8; 32]];
            let mut r = [0i32; 2];
            for i in 0..2 {
                r[i] = gh[i](o[i].as_mut_ptr(), 32, ptr::null(), 0, ptr::null(), 0);
            }
            eq_i32("row281 crypto_generichash empty", r[0], r[1]);
            eq_bytes("row281 crypto_generichash empty", &o[0], &o[1]);
            assert_eq!(r[0], 0, "row281: C rejected a valid call");
        }

        // --- rows 282-285, 290: outlen sweep 1..=64, several input lengths --
        for outlen in 1..=64usize {
            for &inlen in &[0usize, 1, 2, 63, 64, 65, 127, 128, 129, 200] {
                let input = rng.bytes(inlen);
                let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
                let mut o = [vec![0xAAu8; outlen], vec![0xAAu8; outlen]];
                let mut ob = [vec![0x55u8; outlen], vec![0x55u8; outlen]];
                for i in 0..2 {
                    let ra = gh[i](o[i].as_mut_ptr(), outlen, ip, inlen as u64, ptr::null(), 0);
                    let rb = b2[i](ob[i].as_mut_ptr(), outlen, ip, inlen as u64, ptr::null(), 0);
                    assert_eq!(ra, 0, "rows282-285 lib{i} outlen={outlen} inlen={inlen}");
                    assert_eq!(rb, 0, "row290 lib{i} outlen={outlen} inlen={inlen}");
                }
                let ctx = format!("rows282-285 unkeyed outlen={outlen} inlen={inlen}");
                eq_bytes(&ctx, &o[0], &o[1]);
                eq_bytes(&format!("row290 blake2b outlen={outlen} inlen={inlen}"), &ob[0], &ob[1]);
                // row 290: the generic wrapper must be bit-identical to blake2b
                eq_bytes(&format!("row290 wrapper==blake2b C outlen={outlen}"), &o[0], &ob[0]);
                eq_bytes(&format!("row290 wrapper==blake2b Rust outlen={outlen}"), &o[1], &ob[1]);
            }
        }

        // --- rows 286-287, 289: keylen sweep 0..=64 -------------------------
        for keylen in 0..=64usize {
            let key = rng.bytes(keylen.max(1));
            for &inlen in &[0usize, 1, 64, 128, 129, 250] {
                let input = rng.bytes(inlen);
                let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
                for &outlen in &[16usize, 32, 64] {
                    let mut o = [vec![0xAAu8; outlen], vec![0xAAu8; outlen]];
                    let mut ob = [vec![0x55u8; outlen], vec![0x55u8; outlen]];
                    for i in 0..2 {
                        let ra = gh[i](
                            o[i].as_mut_ptr(),
                            outlen,
                            ip,
                            inlen as u64,
                            key.as_ptr(),
                            keylen,
                        );
                        let rb = b2[i](
                            ob[i].as_mut_ptr(),
                            outlen,
                            ip,
                            inlen as u64,
                            key.as_ptr(),
                            keylen,
                        );
                        assert_eq!(ra, 0, "rows286-287 lib{i} keylen={keylen}");
                        assert_eq!(rb, 0, "rows286-287 blake2b lib{i} keylen={keylen}");
                    }
                    let ctx = format!("rows286-289 keylen={keylen} outlen={outlen} inlen={inlen}");
                    eq_bytes(&ctx, &o[0], &o[1]);
                    eq_bytes(&format!("{ctx} blake2b"), &ob[0], &ob[1]);
                    eq_bytes(&format!("{ctx} wrapper==blake2b C"), &o[0], &ob[0]);
                }
            }
        }

        // --- row 288: key != NULL but keylen == 0 -> unkeyed path -----------
        for &inlen in &[0usize, 1, 100, 300] {
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let key = rng.bytes(32);
            let mut with_key0 = [vec![0u8; 32], vec![0u8; 32]];
            let mut with_null = [vec![0u8; 32], vec![0u8; 32]];
            for i in 0..2 {
                assert_eq!(
                    gh[i](with_key0[i].as_mut_ptr(), 32, ip, inlen as u64, key.as_ptr(), 0),
                    0
                );
                assert_eq!(
                    gh[i](with_null[i].as_mut_ptr(), 32, ip, inlen as u64, ptr::null(), 0),
                    0
                );
            }
            eq_bytes(&format!("row288 keylen0 inlen={inlen}"), &with_key0[0], &with_key0[1]);
            eq_bytes(&format!("row288 == unkeyed C inlen={inlen}"), &with_key0[0], &with_null[0]);
            eq_bytes(
                &format!("row288 == unkeyed Rust inlen={inlen}"),
                &with_key0[1],
                &with_null[1],
            );
        }
    }
}

/// CONFIGS.md rows 291-302: `crypto_generichash_init/_update/_final` and
/// `crypto_generichash_blake2b_init/_update/_final` streaming matrix.
#[test]
fn g3_cfg_291_302_generichash_streaming() {
    unsafe {
        let nstate = sz2("crypto_generichash_statebytes");
        assert_eq!(nstate, sz2("crypto_generichash_blake2b_statebytes"));
        let mut rng = Rng::new(SEED ^ 0x291);

        // Both the generic wrapper and the blake2b entry points, in one matrix.
        for (fam, iname, uname, fname) in [
            (
                "crypto_generichash",
                "crypto_generichash_init",
                "crypto_generichash_update",
                "crypto_generichash_final",
            ),
            (
                "crypto_generichash_blake2b",
                "crypto_generichash_blake2b_init",
                "crypto_generichash_blake2b_update",
                "crypto_generichash_blake2b_final",
            ),
        ] {
            let init = pair2!(GhInit, iname);
            let upd = pair2!(StUpd, uname);
            let fin = pair2!(GhFin, fname);
            let oneshot = pair2!(Gh, fam);

            // splits covering rows 291-298, 300 and the keyed variants
            let shapes: Vec<Vec<usize>> = vec![
                vec![],           // row 292: no update at all
                vec![0],          // row 291: single update with inlen=0
                vec![0, 0],       // several zero-length updates
                vec![1; 256],     // row 293: 1 byte per update (256 calls)
                vec![127],        // row 294
                vec![128],        // row 295
                vec![129],        // row 296
                vec![255],        // row 297
                vec![256],        // row 297
                vec![257],        // row 297
                vec![127, 1],     // row 298
                vec![1, 127],     // row 298
                vec![128, 128],   // row 298
                vec![64, 64, 64, 64], // row 298
                vec![100, 200],   // row 298
                vec![63, 65],
                vec![0, 128, 0, 1],
                vec![130, 0, 126],
            ];

            for keylen in [0usize, 16, 32, 64] {
                let key = rng.bytes(keylen.max(1));
                let kp = if keylen == 0 { ptr::null() } else { key.as_ptr() };
                for outlen in [1usize, 16, 32, 63, 64] {
                    for sh in &shapes {
                        let total: usize = sh.iter().sum();
                        let data = rng.bytes(total);
                        let chunks = split(&data, sh);
                        let mut outs: [Vec<u8>; 2] = [vec![0xAAu8; outlen], vec![0xAAu8; outlen]];
                        let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
                        for i in 0..2 {
                            let st = St::new(nstate);
                            rets[i].push(init[i](st.ptr(), kp, keylen, outlen));
                            for ch in &chunks {
                                let p = if ch.is_empty() { ptr::null() } else { ch.as_ptr() };
                                rets[i].push(upd[i](st.ptr(), p, ch.len() as u64));
                            }
                            rets[i].push(fin[i](st.ptr(), outs[i].as_mut_ptr(), outlen));
                        }
                        let ctx = format!(
                            "rows291-302 {fam} keylen={keylen} outlen={outlen} split={sh:?}"
                        );
                        assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
                        assert!(rets[0].iter().all(|&r| r == 0), "{ctx}: C returned {:?}", rets[0]);
                        eq_bytes(&ctx, &outs[0], &outs[1]);

                        // row 301: streaming == one-shot for every triple
                        for i in 0..2 {
                            let mut os = vec![0u8; outlen];
                            let ip = if total == 0 { ptr::null() } else { data.as_ptr() };
                            assert_eq!(
                                oneshot[i](
                                    os.as_mut_ptr(),
                                    outlen,
                                    ip,
                                    total as u64,
                                    kp,
                                    keylen
                                ),
                                0
                            );
                            eq_bytes(&format!("row301 {ctx} lib{i} streaming==oneshot"), &os, &outs[i]);
                        }
                    }
                }
            }
        }
    }
}

/// CONFIGS.md rows 303-308: one-shot `crypto_generichash_blake2b_salt_personal`.
#[test]
fn g3_cfg_303_308_generichash_salt_personal() {
    unsafe {
        let sp = pair2!(GhSp, "crypto_generichash_blake2b_salt_personal");
        let b2 = pair2!(Gh, "crypto_generichash_blake2b");
        let mut rng = Rng::new(SEED ^ 0x303);
        assert_eq!(sz2("crypto_generichash_blake2b_saltbytes"), 16);
        assert_eq!(sz2("crypto_generichash_blake2b_personalbytes"), 16);

        let zero16 = [0u8; 16];
        for &inlen in &[0usize, 1, 64, 127, 128, 129, 300] {
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let salt: Vec<u8> = (0..16).map(|_| rng.byte() | 1).collect();
            let personal: Vec<u8> = (0..16).map(|_| rng.byte() | 2).collect();
            for keylen in [0usize, 16, 32, 64] {
                let key = rng.bytes(keylen.max(1));
                let kp = if keylen == 0 { ptr::null() } else { key.as_ptr() };
                for &outlen in &[16usize, 32, 64] {
                    // the 5 (salt, personal) pointer configurations
                    let cfgs: [(*const u8, *const u8, &str); 5] = [
                        (ptr::null(), ptr::null(), "row303 NULL/NULL"),
                        (salt.as_ptr(), ptr::null(), "row304 salt/NULL"),
                        (ptr::null(), personal.as_ptr(), "row305 NULL/personal"),
                        (salt.as_ptr(), personal.as_ptr(), "row306+308 salt/personal"),
                        (zero16.as_ptr(), zero16.as_ptr(), "row307 zero/zero"),
                    ];
                    let mut digests: Vec<[Vec<u8>; 2]> = Vec::new();
                    for (s, p, label) in cfgs {
                        let mut o = [vec![0xAAu8; outlen], vec![0xAAu8; outlen]];
                        for i in 0..2 {
                            let r = sp[i](
                                o[i].as_mut_ptr(),
                                outlen,
                                ip,
                                inlen as u64,
                                kp,
                                keylen,
                                s,
                                p,
                            );
                            assert_eq!(r, 0, "{label} lib{i}: unexpected {r}");
                        }
                        let ctx = format!("{label} keylen={keylen} outlen={outlen} inlen={inlen}");
                        eq_bytes(&ctx, &o[0], &o[1]);
                        digests.push(o);
                    }
                    // row 303: NULL/NULL == plain crypto_generichash_blake2b
                    for i in 0..2 {
                        let mut plain = vec![0u8; outlen];
                        assert_eq!(
                            b2[i](plain.as_mut_ptr(), outlen, ip, inlen as u64, kp, keylen),
                            0
                        );
                        eq_bytes(
                            &format!("row303 NULL/NULL == blake2b lib{i} keylen={keylen}"),
                            &plain,
                            &digests[0][i],
                        );
                        // row 307: all-zero salt+personal == the NULL/NULL case
                        eq_bytes(
                            &format!("row307 zero == NULL lib{i} keylen={keylen}"),
                            &digests[0][i],
                            &digests[4][i],
                        );
                        // domain separation: non-zero salt/personal must differ
                        assert_ne!(
                            digests[0][i], digests[1][i],
                            "row304 lib{i}: non-zero salt did not change the digest"
                        );
                        assert_ne!(
                            digests[0][i], digests[2][i],
                            "row305 lib{i}: non-zero personal did not change the digest"
                        );
                    }
                }
            }
        }
    }
}

/// CONFIGS.md rows 309-312: `crypto_generichash_blake2b_init_salt_personal`
/// streaming, incl. equality with the one-shot `_salt_personal`.
#[test]
fn g3_cfg_309_312_generichash_init_salt_personal() {
    unsafe {
        let init = pair2!(GhInitSp, "crypto_generichash_blake2b_init_salt_personal");
        let upd = pair2!(StUpd, "crypto_generichash_blake2b_update");
        let fin = pair2!(GhFin, "crypto_generichash_blake2b_final");
        let sp = pair2!(GhSp, "crypto_generichash_blake2b_salt_personal");
        let nstate = sz2("crypto_generichash_blake2b_statebytes");
        let mut rng = Rng::new(SEED ^ 0x309);

        let shapes: Vec<Vec<usize>> = vec![
            vec![],
            vec![0],
            vec![127],
            vec![128],
            vec![129],
            vec![127, 1],
            vec![1, 127],
            vec![128, 128],
            vec![1; 130],
            vec![64, 64, 64],
        ];
        for pass in 0..2 {
            // pass 0 -> row 309 (NULL salt/personal), pass 1 -> rows 310/311
            let salt: Vec<u8> = (0..16).map(|_| rng.byte() | 1).collect();
            let personal: Vec<u8> = (0..16).map(|_| rng.byte() | 4).collect();
            let (sptr, pptr) = if pass == 0 {
                (ptr::null(), ptr::null())
            } else {
                (salt.as_ptr(), personal.as_ptr())
            };
            for keylen in [0usize, 16, 32, 64] {
                let key = rng.bytes(keylen.max(1));
                let kp = if keylen == 0 { ptr::null() } else { key.as_ptr() };
                for outlen in [16usize, 32, 64] {
                    for sh in &shapes {
                        let total: usize = sh.iter().sum();
                        let data = rng.bytes(total);
                        let chunks = split(&data, sh);
                        let mut outs: [Vec<u8>; 2] = [vec![0xAAu8; outlen], vec![0xAAu8; outlen]];
                        let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
                        for i in 0..2 {
                            let st = St::new(nstate);
                            rets[i].push(init[i](st.ptr(), kp, keylen, outlen, sptr, pptr));
                            for ch in &chunks {
                                let p = if ch.is_empty() { ptr::null() } else { ch.as_ptr() };
                                rets[i].push(upd[i](st.ptr(), p, ch.len() as u64));
                            }
                            rets[i].push(fin[i](st.ptr(), outs[i].as_mut_ptr(), outlen));
                        }
                        let ctx = format!(
                            "rows309-311 pass={pass} keylen={keylen} outlen={outlen} split={sh:?}"
                        );
                        assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
                        assert!(rets[0].iter().all(|&r| r == 0), "{ctx}: C returned {:?}", rets[0]);
                        eq_bytes(&ctx, &outs[0], &outs[1]);
                        // row 312: streaming == one-shot _salt_personal
                        for i in 0..2 {
                            let mut os = vec![0u8; outlen];
                            let ip = if total == 0 { ptr::null() } else { data.as_ptr() };
                            assert_eq!(
                                sp[i](
                                    os.as_mut_ptr(),
                                    outlen,
                                    ip,
                                    total as u64,
                                    kp,
                                    keylen,
                                    sptr,
                                    pptr
                                ),
                                0
                            );
                            eq_bytes(&format!("row312 {ctx} lib{i}"), &os, &outs[i]);
                        }
                    }
                }
            }
        }
    }
}

/// CONFIGS.md rows 313-317: generichash constant accessors, `_keygen`,
/// and `_crypto_generichash_blake2b_pick_best_implementation`.
#[test]
fn g3_cfg_313_317_generichash_constants_keygen_pickbest() {
    install_det_random();
    unsafe {
        // row 313
        assert_eq!(sz2("crypto_generichash_statebytes"), 384);
        assert_eq!(sz2("crypto_generichash_blake2b_statebytes"), 384);
        // row 314
        assert_eq!(sz2("crypto_generichash_bytes_min"), 16);
        assert_eq!(sz2("crypto_generichash_bytes_max"), 64);
        assert_eq!(sz2("crypto_generichash_bytes"), 32);
        assert_eq!(sz2("crypto_generichash_keybytes_min"), 16);
        assert_eq!(sz2("crypto_generichash_keybytes_max"), 64);
        assert_eq!(sz2("crypto_generichash_keybytes"), 32);
        assert_eq!(prim2("crypto_generichash_primitive"), "blake2b");
        // row 315
        assert_eq!(sz2("crypto_generichash_blake2b_bytes_min"), 16);
        assert_eq!(sz2("crypto_generichash_blake2b_bytes_max"), 64);
        assert_eq!(sz2("crypto_generichash_blake2b_bytes"), 32);
        assert_eq!(sz2("crypto_generichash_blake2b_keybytes_min"), 16);
        assert_eq!(sz2("crypto_generichash_blake2b_keybytes_max"), 64);
        assert_eq!(sz2("crypto_generichash_blake2b_keybytes"), 32);
        assert_eq!(sz2("crypto_generichash_blake2b_saltbytes"), 16);
        assert_eq!(sz2("crypto_generichash_blake2b_personalbytes"), 16);

        // row 316: keygen (deterministic randombytes installed in both libs)
        let gh = pair2!(Gh, "crypto_generichash");
        for name in [
            "crypto_generichash_keygen",
            "crypto_generichash_blake2b_keygen",
        ] {
            let kg = pair2!(Keygen, name);
            det_reseed(0x1234_5678_9ABC_DEF0);
            let mut k0 = vec![0xAAu8; 40];
            kg[0](k0.as_mut_ptr());
            det_reseed(0x1234_5678_9ABC_DEF0);
            let mut k1 = vec![0xAAu8; 40];
            kg[1](k1.as_mut_ptr());
            eq_bytes(&format!("row316 {name}"), &k0, &k1);
            assert_eq!(&k0[32..], &[0xAAu8; 8], "row316 {name}: wrote past 32 bytes");
            assert_ne!(&k0[..32], &[0xAAu8; 32], "row316 {name}: wrote nothing");
            // keyed round-trip with the generated key
            let msg = [1u8, 2, 3, 4, 5];
            let mut o = [vec![0u8; 32], vec![0u8; 32]];
            for i in 0..2 {
                assert_eq!(
                    gh[i](o[i].as_mut_ptr(), 32, msg.as_ptr(), 5, k0.as_ptr(), 32),
                    0
                );
            }
            eq_bytes(&format!("row316 {name} keyed round-trip"), &o[0], &o[1]);
        }

        // row 317: pick_best_implementation must return the same thing and the
        // digest matrix must be unchanged afterwards.
        let pb = pair2!(PickBest, "_crypto_generichash_blake2b_pick_best_implementation");
        let mut rng = Rng::new(SEED ^ 0x317);
        let mut before: Vec<Vec<u8>> = Vec::new();
        let cases: Vec<(usize, usize, usize)> = vec![
            (0, 0, 32),
            (1, 0, 16),
            (128, 32, 64),
            (129, 64, 33),
            (300, 16, 17),
            (1000, 32, 63),
        ];
        for &(inlen, keylen, outlen) in &cases {
            let input = rng.bytes(inlen);
            let key = rng.bytes(keylen.max(1));
            let kp = if keylen == 0 { ptr::null() } else { key.as_ptr() };
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let mut o = vec![0u8; outlen];
            assert_eq!(gh[0](o.as_mut_ptr(), outlen, ip, inlen as u64, kp, keylen), 0);
            before.push(o);
            before.push(input);
            before.push(key);
        }
        let ra = pb[0]();
        let rb = pb[1]();
        eq_i32("row317 pick_best_implementation", ra, rb);
        for (n, &(inlen, keylen, outlen)) in cases.iter().enumerate() {
            let input = &before[n * 3 + 1];
            let key = &before[n * 3 + 2];
            let kp = if keylen == 0 { ptr::null() } else { key.as_ptr() };
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let mut o = [vec![0u8; outlen], vec![0u8; outlen]];
            for i in 0..2 {
                assert_eq!(gh[i](o[i].as_mut_ptr(), outlen, ip, inlen as u64, kp, keylen), 0);
            }
            eq_bytes(&format!("row317 after pick_best case{n}"), &o[0], &o[1]);
            eq_bytes(
                &format!("row317 digest changed by pick_best case{n}"),
                &before[n * 3],
                &o[0],
            );
        }
    }
}

// ===========================================================================
// crypto_hash (sha256 / sha512 / sha3-256 / sha3-512)
// ===========================================================================

/// Shared driver for the four `crypto_hash_*` families: one-shot length sweep,
/// streaming with many chunk shapes, streaming == one-shot.
fn hash_family(
    label: &str,
    oneshot_name: &str,
    init_name: &str,
    upd_name: &str,
    fin_name: &str,
    outlen: usize,
    sweep_max: usize,
    big: usize,
    shapes: &[Vec<usize>],
    seed: u64,
) {
    unsafe {
        let one = pair2!(OneShot, oneshot_name);
        let init = pair2!(StInit, init_name);
        let upd = pair2!(StUpd, upd_name);
        let fin = pair2!(StFin, fin_name);
        let nstate = sz2(&format!("{oneshot_name}_statebytes"));
        assert_eq!(sz2(&format!("{oneshot_name}_bytes")), outlen);
        let mut rng = Rng::new(seed);

        // one-shot: empty + full length sweep + one big input
        for inlen in (0..=sweep_max).chain(std::iter::once(big)) {
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let mut o = [vec![0xAAu8; outlen], vec![0xAAu8; outlen]];
            let mut r = [0i32; 2];
            for i in 0..2 {
                r[i] = one[i](o[i].as_mut_ptr(), ip, inlen as u64);
            }
            let ctx = format!("{label} one-shot inlen={inlen}");
            eq_i32(&ctx, r[0], r[1]);
            eq_bytes(&ctx, &o[0], &o[1]);
            assert_eq!(r[0], 0, "{ctx}: C returned {}", r[0]);
        }

        // streaming
        for sh in shapes {
            let total: usize = sh.iter().sum();
            let data = rng.bytes(total);
            let chunks = split(&data, sh);
            let got: [(Vec<u8>, Vec<c_int>); 2] = [
                run_stream(init[0], upd[0], fin[0], nstate, outlen, &chunks),
                run_stream(init[1], upd[1], fin[1], nstate, outlen, &chunks),
            ];
            let ctx = format!("{label} streaming split={sh:?}");
            assert_eq!(got[0].1, got[1].1, "{ctx}: return values differ");
            eq_bytes(&ctx, &got[0].0, &got[1].0);
            // streaming == one-shot
            for i in 0..2 {
                let mut os = vec![0u8; outlen];
                let ip = if total == 0 { ptr::null() } else { data.as_ptr() };
                assert_eq!(one[i](os.as_mut_ptr(), ip, total as u64), 0);
                eq_bytes(&format!("{ctx} lib{i} streaming==oneshot"), &os, &got[i].0);
            }
        }
    }
}

/// CONFIGS.md rows 318-327: `crypto_hash_sha256` one-shot + streaming.
#[test]
fn g3_cfg_318_327_sha256() {
    let shapes: Vec<Vec<usize>> = vec![
        vec![],               // row 321: no update
        vec![0],              // row 322: inlen=0 update
        vec![0, 0, 0],
        vec![1; 200],         // row 323
        vec![64, 64],         // row 324
        vec![63, 1],          // row 324
        vec![1, 63],          // row 324
        vec![55, 9],          // row 324
        vec![56, 8],          // row 324
        vec![10, 190],        // row 325
        vec![0, 64, 0, 64, 0],
        vec![119, 1],
        vec![127, 1],
        vec![128, 72],
    ];
    hash_family(
        "row318-326 sha256",
        "crypto_hash_sha256",
        "crypto_hash_sha256_init",
        "crypto_hash_sha256_update",
        "crypto_hash_sha256_final",
        32,
        200,
        1000,
        &shapes,
        SEED ^ 0x318,
    );
    // row 327
    assert_eq!(sz2("crypto_hash_sha256_bytes"), 32);
    assert!(sz2("crypto_hash_sha256_statebytes") >= 104);
}

/// CONFIGS.md rows 328-336: `crypto_hash_sha512` one-shot + streaming, and the
/// `crypto_hash` default alias.
#[test]
fn g3_cfg_328_336_sha512_and_crypto_hash() {
    let shapes: Vec<Vec<usize>> = vec![
        vec![],                 // row 331
        vec![0],                // row 331
        vec![0, 0],
        vec![1; 300],           // row 332
        vec![128, 128],         // row 333
        vec![127, 1],           // row 333
        vec![1, 127],           // row 333
        vec![111, 17],          // row 333
        vec![112, 16],          // row 333
        vec![10, 290],
        vec![239, 1],
        vec![255, 1],
        vec![256, 44],
    ];
    hash_family(
        "row328-334 sha512",
        "crypto_hash_sha512",
        "crypto_hash_sha512_init",
        "crypto_hash_sha512_update",
        "crypto_hash_sha512_final",
        64,
        300,
        2000,
        &shapes,
        SEED ^ 0x328,
    );
    // row 335
    assert_eq!(sz2("crypto_hash_sha512_bytes"), 64);
    assert!(sz2("crypto_hash_sha512_statebytes") >= 208);
    // row 336: crypto_hash is the sha512 alias
    assert_eq!(sz2("crypto_hash_bytes"), 64);
    assert_eq!(prim2("crypto_hash_primitive"), "sha512");
    unsafe {
        let ch = pair2!(OneShot, "crypto_hash");
        let sha = pair2!(OneShot, "crypto_hash_sha512");
        let mut rng = Rng::new(SEED ^ 0x336);
        for inlen in [0usize, 1, 111, 112, 127, 128, 129, 255, 256, 1000] {
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let mut o = [vec![0xAAu8; 64], vec![0xAAu8; 64]];
            let mut s = [vec![0u8; 64], vec![0u8; 64]];
            for i in 0..2 {
                assert_eq!(ch[i](o[i].as_mut_ptr(), ip, inlen as u64), 0);
                assert_eq!(sha[i](s[i].as_mut_ptr(), ip, inlen as u64), 0);
            }
            eq_bytes(&format!("row336 crypto_hash inlen={inlen}"), &o[0], &o[1]);
            eq_bytes(&format!("row336 crypto_hash==sha512 C inlen={inlen}"), &o[0], &s[0]);
            eq_bytes(&format!("row336 crypto_hash==sha512 Rust inlen={inlen}"), &o[1], &s[1]);
        }
    }
}

/// CONFIGS.md rows 337-344 and 350: `crypto_hash_sha3256`.
#[test]
fn g3_cfg_337_344_sha3_256() {
    let shapes: Vec<Vec<usize>> = vec![
        vec![],                 // row 340
        vec![0],                // row 340
        vec![0, 0],
        vec![1; 300],           // row 341
        vec![100, 36],          // row 342
        vec![135, 1],           // row 342
        vec![136, 1],           // row 342
        vec![1, 135],           // row 342
        vec![136, 136],         // row 342/343
        vec![50, 150],          // row 342
        vec![136],              // row 343: offset == rate at final
        vec![272],              // row 343
        vec![68, 68],           // row 343: total = 136
        vec![134, 1],
        vec![135],
        vec![271, 1],
    ];
    hash_family(
        "rows337-343+350 sha3256",
        "crypto_hash_sha3256",
        "crypto_hash_sha3256_init",
        "crypto_hash_sha3256_update",
        "crypto_hash_sha3256_final",
        32,
        300,
        1000,
        &shapes,
        SEED ^ 0x337,
    );
    // row 344
    assert_eq!(sz2("crypto_hash_sha3256_bytes"), 32);
    assert_eq!(sz2("crypto_hash_sha3256_statebytes"), 256);
}

/// CONFIGS.md rows 345-350: `crypto_hash_sha3512`.
#[test]
fn g3_cfg_345_350_sha3_512() {
    let shapes: Vec<Vec<usize>> = vec![
        vec![],                 // row 348
        vec![0],                // row 348
        vec![1; 200],           // row 348
        vec![71, 1],            // row 348
        vec![72, 1],            // row 348
        vec![1, 71],            // row 348
        vec![72, 72],           // row 348
        vec![30, 42],           // row 348
        vec![72],
        vec![144],
        vec![70, 1],
        vec![143, 1],
    ];
    hash_family(
        "rows345-348+350 sha3512",
        "crypto_hash_sha3512",
        "crypto_hash_sha3512_init",
        "crypto_hash_sha3512_update",
        "crypto_hash_sha3512_final",
        64,
        200,
        1000,
        &shapes,
        SEED ^ 0x345,
    );
    // row 349
    assert_eq!(sz2("crypto_hash_sha3512_bytes"), 64);
    assert_eq!(sz2("crypto_hash_sha3512_statebytes"), 256);
}

// ===========================================================================
// crypto_xof
// ===========================================================================

/// Shared driver for the four XOF families.
fn xof_family(fam: &str, rate: usize, seed: u64) {
    unsafe {
        let one = pair2!(XofOneShot, fam);
        let init = pair2!(StInit, &format!("{fam}_init"));
        let initd = pair2!(XofInitDom, &format!("{fam}_init_with_domain"));
        let upd = pair2!(StUpd, &format!("{fam}_update"));
        let sq = pair2!(XofSqueeze, &format!("{fam}_squeeze"));
        let nstate = sz2(&format!("{fam}_statebytes"));
        assert_eq!(nstate, 256, "{fam}_statebytes");
        assert_eq!(sz2(&format!("{fam}_blockbytes")), rate, "{fam}_blockbytes");
        assert_eq!(
            uc2(&format!("{fam}_domain_standard")),
            0x1F,
            "{fam}_domain_standard"
        );
        let mut rng = Rng::new(seed);

        // --- one-shot outlen sweep (incl. outlen=0) ------------------------
        let outlens: Vec<usize> = (0..=8usize)
            .chain([
                rate - 2,
                rate - 1,
                rate,
                rate + 1,
                rate + 2,
                2 * rate - 1,
                2 * rate,
                2 * rate + 1,
                255,
                256,
                511,
                512,
            ])
            .collect();
        for &inlen in &[0usize, 1, rate - 1, rate, rate + 1] {
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            for &outlen in &outlens {
                let mut o = [vec![0xAAu8; outlen + 4], vec![0xAAu8; outlen + 4]];
                let mut r = [0i32; 2];
                for i in 0..2 {
                    r[i] = one[i](o[i].as_mut_ptr(), outlen, ip, inlen as u64);
                }
                let ctx = format!("{fam} one-shot inlen={inlen} outlen={outlen}");
                eq_i32(&ctx, r[0], r[1]);
                eq_bytes(&ctx, &o[0], &o[1]);
                assert_eq!(r[0], 0, "{ctx}: C returned {}", r[0]);
                assert_eq!(&o[0][outlen..], &[0xAAu8; 4], "{ctx}: wrote past outlen");
            }
        }

        // --- one-shot inlen sweep 0..=400 ----------------------------------
        for inlen in 0..=400usize {
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let outlen = 64;
            let mut o = [vec![0xAAu8; outlen], vec![0xAAu8; outlen]];
            for i in 0..2 {
                assert_eq!(one[i](o[i].as_mut_ptr(), outlen, ip, inlen as u64), 0);
            }
            eq_bytes(&format!("{fam} inlen sweep inlen={inlen}"), &o[0], &o[1]);
        }

        // --- streaming: update shapes x squeeze shapes ----------------------
        let up_shapes: Vec<Vec<usize>> = vec![
            vec![],                     // squeeze with no update
            vec![0],
            vec![1; 400],               // 1 byte per update
            vec![rate - 1, 1],
            vec![rate, 1],
            vec![1, rate - 1],
            vec![rate, rate],
            vec![100, 68],
            vec![rate],
            vec![2 * rate],
            vec![rate - 1],
            vec![0, rate, 0, 1],
        ];
        let sq_shapes: Vec<Vec<usize>> = vec![
            vec![32],
            vec![1; 512],               // 1 byte at a time
            vec![rate - 1, 1],
            vec![rate, 1],
            vec![rate, rate],
            vec![100, 412],
            vec![2 * rate, 512 - 2 * rate],
            vec![0, 64, 0, 64, 0],      // zero-length squeezes are no-ops
            vec![512],
        ];
        for ush in &up_shapes {
            let total: usize = ush.iter().sum();
            let data = rng.bytes(total);
            let chunks = split(&data, ush);
            for ssh in &sq_shapes {
                let outlen: usize = ssh.iter().sum();
                let mut outs: [Vec<u8>; 2] = [vec![0xAAu8; outlen], vec![0xAAu8; outlen]];
                let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
                for i in 0..2 {
                    let st = St::new(nstate);
                    rets[i].push(init[i](st.ptr()));
                    for ch in &chunks {
                        let p = if ch.is_empty() { ptr::null() } else { ch.as_ptr() };
                        rets[i].push(upd[i](st.ptr(), p, ch.len() as u64));
                    }
                    let mut off = 0usize;
                    for &n in ssh {
                        let p = outs[i].as_mut_ptr().add(off);
                        rets[i].push(sq[i](st.ptr(), p, n));
                        off += n;
                    }
                }
                let ctx = format!("{fam} stream up={ush:?} sq={ssh:?}");
                assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
                assert!(rets[0].iter().all(|&r| r == 0), "{ctx}: C returned {:?}", rets[0]);
                eq_bytes(&ctx, &outs[0], &outs[1]);
                // must match the one-shot for the same input/output length
                for i in 0..2 {
                    let mut os = vec![0u8; outlen];
                    let ip = if total == 0 { ptr::null() } else { data.as_ptr() };
                    assert_eq!(one[i](os.as_mut_ptr(), outlen, ip, total as u64), 0);
                    eq_bytes(&format!("{ctx} lib{i} stream==oneshot"), &os, &outs[i]);
                }
            }
        }

        // --- init_with_domain: 0x1F equals init; sweep 0x01..=0x7F ---------
        for domain in 1u8..=0x7F {
            for &inlen in &[0usize, 1, rate - 1, rate, rate + 1, 2 * rate - 1] {
                let input = rng.bytes(inlen);
                let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
                let outlen = 96usize;
                let mut o = [vec![0xAAu8; outlen], vec![0xAAu8; outlen]];
                let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
                for i in 0..2 {
                    let st = St::new(nstate);
                    rets[i].push(initd[i](st.ptr(), domain));
                    rets[i].push(upd[i](st.ptr(), ip, inlen as u64));
                    rets[i].push(sq[i](st.ptr(), o[i].as_mut_ptr(), outlen));
                }
                let ctx = format!("{fam} init_with_domain d={domain:#04x} inlen={inlen}");
                assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
                eq_bytes(&ctx, &o[0], &o[1]);
                if domain == 0x1F {
                    // row 362 / equivalent: standard domain == plain _init
                    for i in 0..2 {
                        let st = St::new(nstate);
                        let mut p = vec![0u8; outlen];
                        init[i](st.ptr());
                        upd[i](st.ptr(), ip, inlen as u64);
                        sq[i](st.ptr(), p.as_mut_ptr(), outlen);
                        eq_bytes(&format!("{ctx} == _init lib{i}"), &p, &o[i]);
                    }
                }
            }
        }

        // --- the `offset == rate-1` single-byte pad branch: inlen%rate==rate-1
        for k in 0..3usize {
            let inlen = k * rate + rate - 1;
            let input = rng.bytes(inlen);
            for &domain in &[0x01u8, 0x02, 0x06, 0x0B, 0x1F, 0x7F] {
                let outlen = 200usize;
                let mut o = [vec![0xAAu8; outlen], vec![0xAAu8; outlen]];
                for i in 0..2 {
                    let st = St::new(nstate);
                    assert_eq!(initd[i](st.ptr(), domain), 0);
                    assert_eq!(upd[i](st.ptr(), input.as_ptr(), inlen as u64), 0);
                    assert_eq!(sq[i](st.ptr(), o[i].as_mut_ptr(), outlen), 0);
                }
                eq_bytes(
                    &format!("{fam} single-byte pad inlen={inlen} d={domain:#04x}"),
                    &o[0],
                    &o[1],
                );
            }
        }
    }
}

/// CONFIGS.md rows 351-365: `crypto_xof_shake128`.
#[test]
fn g3_cfg_351_365_shake128() {
    xof_family("crypto_xof_shake128", 168, SEED ^ 0x351);
}

/// CONFIGS.md rows 366-371: `crypto_xof_shake256`.
#[test]
fn g3_cfg_366_371_shake256() {
    xof_family("crypto_xof_shake256", 136, SEED ^ 0x366);
}

/// CONFIGS.md rows 372-378: `crypto_xof_turboshake128` (12-round permutation).
#[test]
fn g3_cfg_372_378_turboshake128() {
    xof_family("crypto_xof_turboshake128", 168, SEED ^ 0x372);
}

/// CONFIGS.md rows 379-384: `crypto_xof_turboshake256`.
#[test]
fn g3_cfg_379_384_turboshake256() {
    xof_family("crypto_xof_turboshake256", 136, SEED ^ 0x379);
}

// ===========================================================================
// crypto_core_keccak1600
// ===========================================================================

/// CONFIGS.md rows 385-393: raw Keccak-f[1600] state API.
#[test]
fn g3_cfg_385_393_keccak1600() {
    unsafe {
        // row 392
        let nstate = sz2("crypto_core_keccak1600_statebytes");
        assert!(nstate >= 200, "keccak1600 statebytes = {nstate}");
        let init = pair2!(KecInit, "crypto_core_keccak1600_init");
        let xorb = pair2!(KecXor, "crypto_core_keccak1600_xor_bytes");
        let extr = pair2!(KecExtract, "crypto_core_keccak1600_extract_bytes");
        let p24 = pair2!(KecPermute, "crypto_core_keccak1600_permute_24");
        let p12 = pair2!(KecPermute, "crypto_core_keccak1600_permute_12");
        // row 393: the `ref` backend is the one selected on non-ARM builds; it
        // must be reachable directly and behave identically.
        let rinit = pair2!(KecInit, "_sodium_keccak1600_ref_init");
        let rxor = pair2!(KecXor, "_sodium_keccak1600_ref_xor_bytes");
        let rextr = pair2!(KecExtract, "_sodium_keccak1600_ref_extract_bytes");
        let rp24 = pair2!(KecPermute, "_sodium_keccak1600_ref_permute_24");
        let rp12 = pair2!(KecPermute, "_sodium_keccak1600_ref_permute_12");

        let mut rng = Rng::new(SEED ^ 0x385);

        // rows 385/386: permutation of the all-zero state, 24 and 12 rounds
        for (rounds, perm, rperm) in [(24u32, p24, rp24), (12, p12, rp12)] {
            let mut out = [vec![0xAAu8; 200], vec![0xAAu8; 200]];
            let mut rout = [vec![0xAAu8; 200], vec![0xAAu8; 200]];
            for i in 0..2 {
                let st = St::new(nstate);
                init[i](st.ptr());
                perm[i](st.ptr());
                extr[i](st.ptr(), out[i].as_mut_ptr(), 0, 200);
                let st2 = St::new(nstate);
                rinit[i](st2.ptr());
                rperm[i](st2.ptr());
                rextr[i](st2.ptr(), rout[i].as_mut_ptr(), 0, 200);
            }
            eq_bytes(&format!("rows385/386 permute_{rounds} of zero"), &out[0], &out[1]);
            eq_bytes(&format!("row393 ref permute_{rounds}"), &rout[0], &rout[1]);
            for i in 0..2 {
                eq_bytes(
                    &format!("row393 lib{i} public==ref permute_{rounds}"),
                    &out[i],
                    &rout[i],
                );
            }
        }

        // rows 387-390: xor_bytes / extract_bytes offset+length shapes
        let spans: [(usize, usize); 9] = [
            (0, 200),
            (0, 0),
            (199, 1),
            (7, 1),
            (5, 11),
            (3, 13),
            (0, 8),
            (8, 192),
            (63, 74),
        ];
        for &(off, len) in &spans {
            let data = rng.bytes(len.max(1));
            let mut out = [vec![0xAAu8; 200], vec![0xAAu8; 200]];
            for i in 0..2 {
                let st = St::new(nstate);
                init[i](st.ptr());
                xorb[i](st.ptr(), data.as_ptr(), off, len);
                // row 388: XOR must accumulate, not overwrite
                xorb[i](st.ptr(), data.as_ptr(), off, len);
                xorb[i](st.ptr(), data.as_ptr(), off, len);
                extr[i](st.ptr(), out[i].as_mut_ptr(), 0, 200);
            }
            eq_bytes(&format!("rows387-390 span off={off} len={len}"), &out[0], &out[1]);
            // after an even number of XORs of the same data the state is zero
            // again; verify the accumulate semantics explicitly on lib C+Rust
            for i in 0..2 {
                let st = St::new(nstate);
                init[i](st.ptr());
                xorb[i](st.ptr(), data.as_ptr(), off, len);
                xorb[i](st.ptr(), data.as_ptr(), off, len);
                let mut z = vec![0xAAu8; 200];
                extr[i](st.ptr(), z.as_mut_ptr(), 0, 200);
                assert!(
                    z.iter().all(|&b| b == 0),
                    "row388 lib{i} off={off} len={len}: double XOR did not cancel"
                );
            }
            // partial extraction shapes
            if len > 0 {
                let mut e = [vec![0xAAu8; len + 4], vec![0xAAu8; len + 4]];
                for i in 0..2 {
                    let st = St::new(nstate);
                    init[i](st.ptr());
                    xorb[i](st.ptr(), data.as_ptr(), 0, 200.min(data.len()));
                    p24[i](st.ptr());
                    extr[i](st.ptr(), e[i].as_mut_ptr(), off, len);
                }
                eq_bytes(&format!("row390 extract off={off} len={len}"), &e[0], &e[1]);
                assert_eq!(&e[0][len..], &[0xAAu8; 4], "row390: wrote past length");
            }
        }

        // row 391: repeated permutation, with xor'd input in between
        for reps in 1..=5usize {
            let data = rng.bytes(200);
            let mut out = [vec![0u8; 200], vec![0u8; 200]];
            let mut rout = [vec![0u8; 200], vec![0u8; 200]];
            for i in 0..2 {
                let st = St::new(nstate);
                init[i](st.ptr());
                for _ in 0..reps {
                    xorb[i](st.ptr(), data.as_ptr(), 0, 200);
                    p24[i](st.ptr());
                    p12[i](st.ptr());
                }
                extr[i](st.ptr(), out[i].as_mut_ptr(), 0, 200);
                let st2 = St::new(nstate);
                rinit[i](st2.ptr());
                for _ in 0..reps {
                    rxor[i](st2.ptr(), data.as_ptr(), 0, 200);
                    rp24[i](st2.ptr());
                    rp12[i](st2.ptr());
                }
                rextr[i](st2.ptr(), rout[i].as_mut_ptr(), 0, 200);
            }
            eq_bytes(&format!("row391 reps={reps}"), &out[0], &out[1]);
            eq_bytes(&format!("row393 ref reps={reps}"), &rout[0], &rout[1]);
            for i in 0..2 {
                eq_bytes(&format!("row393 lib{i} public==ref reps={reps}"), &out[i], &rout[i]);
            }
        }
    }
}

// ===========================================================================
// crypto_auth (HMAC)
// ===========================================================================

/// Shared driver for the three HMAC families.
/// `block` is the HMAC block size (64 for sha256, 128 for sha512/512256).
fn hmac_family(fam: &str, outlen: usize, block: usize, seed: u64) {
    unsafe {
        let one = pair2!(OneShotK, fam);
        let ver = pair2!(VerifyK, &format!("{fam}_verify"));
        let init = pair2!(HmacInit, &format!("{fam}_init"));
        let upd = pair2!(StUpd, &format!("{fam}_update"));
        let fin = pair2!(StFin, &format!("{fam}_final"));
        let nstate = sz2(&format!("{fam}_statebytes"));
        assert_eq!(sz2(&format!("{fam}_bytes")), outlen, "{fam}_bytes");
        assert_eq!(sz2(&format!("{fam}_keybytes")), 32, "{fam}_keybytes");
        let mut rng = Rng::new(seed);

        // --- one-shot: 32-byte key, inlen sweep -----------------------------
        let sweep_max = if block == 64 { 200usize } else { 300 };
        for inlen in (0..=sweep_max).chain([1000usize]) {
            let key = rng.bytes(32);
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let mut o = [vec![0xAAu8; outlen], vec![0xAAu8; outlen]];
            let mut r = [0i32; 2];
            for i in 0..2 {
                r[i] = one[i](o[i].as_mut_ptr(), ip, inlen as u64, key.as_ptr());
            }
            let ctx = format!("{fam} one-shot inlen={inlen}");
            eq_i32(&ctx, r[0], r[1]);
            eq_bytes(&ctx, &o[0], &o[1]);
            assert_eq!(r[0], 0);
            // streaming with keylen=32 must reproduce the one-shot MAC
            for i in 0..2 {
                let st = St::new(nstate);
                let mut s = vec![0u8; outlen];
                assert_eq!(init[i](st.ptr(), key.as_ptr(), 32), 0);
                assert_eq!(upd[i](st.ptr(), ip, inlen as u64), 0);
                assert_eq!(fin[i](st.ptr(), s.as_mut_ptr()), 0);
                eq_bytes(&format!("{ctx} lib{i} stream==oneshot"), &s, &o[i]);
            }
            // verify: matching MAC returns 0
            for i in 0..2 {
                assert_eq!(
                    ver[i](o[i].as_ptr(), ip, inlen as u64, key.as_ptr()),
                    0,
                    "{ctx} lib{i}: verify of the correct MAC failed"
                );
            }
        }

        // --- keylen sweep, incl. the block-hashing threshold ---------------
        let mut keylens: Vec<usize> = (0..=block).collect();
        keylens.extend([block + 1, block + 36, block + 72, 2 * block, 2 * block + 44, 1000]);
        let msg_shapes: Vec<Vec<usize>> = vec![
            vec![],
            vec![0],
            vec![1; 130],
            vec![block - 1, 1],
            vec![block, 1],
            vec![1, block - 1],
            vec![block, block],
            vec![7, 3, 200],
        ];
        for &keylen in &keylens {
            let key = rng.bytes(keylen.max(1));
            let kp = if keylen == 0 { ptr::null() } else { key.as_ptr() };
            for sh in &msg_shapes {
                let total: usize = sh.iter().sum();
                let data = rng.bytes(total);
                let chunks = split(&data, sh);
                let mut outs: [Vec<u8>; 2] = [vec![0xAAu8; outlen], vec![0xAAu8; outlen]];
                let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
                for i in 0..2 {
                    let st = St::new(nstate);
                    rets[i].push(init[i](st.ptr(), kp, keylen));
                    for ch in &chunks {
                        let p = if ch.is_empty() { ptr::null() } else { ch.as_ptr() };
                        rets[i].push(upd[i](st.ptr(), p, ch.len() as u64));
                    }
                    rets[i].push(fin[i](st.ptr(), outs[i].as_mut_ptr()));
                }
                let ctx = format!("{fam} keylen={keylen} split={sh:?}");
                assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
                assert!(rets[0].iter().all(|&r| r == 0), "{ctx}: C returned {:?}", rets[0]);
                eq_bytes(&ctx, &outs[0], &outs[1]);
            }
        }

        // --- long key K must equal short key SHA(K) -------------------------
        let inner_hash = if block == 64 {
            "crypto_hash_sha256"
        } else {
            "crypto_hash_sha512"
        };
        let hlen = if block == 64 { 32usize } else { 64 };
        let hash = pair2!(OneShot, inner_hash);
        for &keylen in &[block + 1, block + 100, 2 * block, 1000usize] {
            let key = rng.bytes(keylen);
            let msg = rng.bytes(77);
            let mut long_mac = [vec![0u8; outlen], vec![0u8; outlen]];
            let mut short_mac = [vec![0u8; outlen], vec![0u8; outlen]];
            for i in 0..2 {
                let st = St::new(nstate);
                assert_eq!(init[i](st.ptr(), key.as_ptr(), keylen), 0);
                assert_eq!(upd[i](st.ptr(), msg.as_ptr(), 77), 0);
                assert_eq!(fin[i](st.ptr(), long_mac[i].as_mut_ptr()), 0);

                let mut hk = vec![0u8; hlen];
                assert_eq!(hash[i](hk.as_mut_ptr(), key.as_ptr(), keylen as u64), 0);
                let st2 = St::new(nstate);
                assert_eq!(init[i](st2.ptr(), hk.as_ptr(), hlen), 0);
                assert_eq!(upd[i](st2.ptr(), msg.as_ptr(), 77), 0);
                assert_eq!(fin[i](st2.ptr(), short_mac[i].as_mut_ptr()), 0);
            }
            eq_bytes(&format!("{fam} long key keylen={keylen}"), &long_mac[0], &long_mac[1]);
            for i in 0..2 {
                eq_bytes(
                    &format!("{fam} lib{i} long key == hashed key keylen={keylen}"),
                    &long_mac[i],
                    &short_mac[i],
                );
            }
        }

        // --- verify: mismatching MACs --------------------------------------
        let key = rng.bytes(32);
        let key2 = rng.bytes(32);
        let msg = rng.bytes(64);
        let mut msg2 = msg.clone();
        msg2[13] ^= 1;
        let mut mac = [vec![0u8; outlen], vec![0u8; outlen]];
        for i in 0..2 {
            assert_eq!(one[i](mac[i].as_mut_ptr(), msg.as_ptr(), 64, key.as_ptr()), 0);
        }
        eq_bytes(&format!("{fam} verify base mac"), &mac[0], &mac[1]);
        let mut bad: Vec<(String, Vec<u8>, Vec<u8>, Vec<u8>)> = Vec::new();
        let mut f0 = mac[0].clone();
        f0[0] ^= 1;
        bad.push(("flip bit0 byte0".into(), f0, msg.clone(), key.clone()));
        let mut f1 = mac[0].clone();
        f1[outlen - 1] ^= 0x80;
        bad.push(("flip bit7 last byte".into(), f1, msg.clone(), key.clone()));
        bad.push(("all-zero h".into(), vec![0u8; outlen], msg.clone(), key.clone()));
        bad.push(("wrong key".into(), mac[0].clone(), msg.clone(), key2.clone()));
        bad.push(("wrong message".into(), mac[0].clone(), msg2.clone(), key.clone()));
        for (label, h, m, k) in bad {
            let mut r = [0i32; 2];
            for i in 0..2 {
                r[i] = ver[i](h.as_ptr(), m.as_ptr(), m.len() as u64, k.as_ptr());
            }
            eq_i32(&format!("{fam} verify {label}"), r[0], r[1]);
            assert_eq!(r[0], -1, "{fam} verify {label}: C accepted a bad MAC");
        }

        // --- keygen ---------------------------------------------------------
        install_det_random();
        let kg = pair2!(Keygen, &format!("{fam}_keygen"));
        det_reseed(0xABCD_EF01_2345_6789);
        let mut k0 = vec![0xAAu8; 40];
        kg[0](k0.as_mut_ptr());
        det_reseed(0xABCD_EF01_2345_6789);
        let mut k1 = vec![0xAAu8; 40];
        kg[1](k1.as_mut_ptr());
        eq_bytes(&format!("{fam}_keygen"), &k0, &k1);
        assert_eq!(&k0[32..], &[0xAAu8; 8], "{fam}_keygen wrote past 32 bytes");
        let mut o = [vec![0u8; outlen], vec![0u8; outlen]];
        for i in 0..2 {
            assert_eq!(one[i](o[i].as_mut_ptr(), msg.as_ptr(), 64, k0.as_ptr()), 0);
            assert_eq!(ver[i](o[i].as_ptr(), msg.as_ptr(), 64, k0.as_ptr()), 0);
        }
        eq_bytes(&format!("{fam}_keygen round-trip"), &o[0], &o[1]);
    }
}

/// CONFIGS.md rows 394-407: `crypto_auth_hmacsha256`.
#[test]
fn g3_cfg_394_407_hmacsha256() {
    hmac_family("crypto_auth_hmacsha256", 32, 64, SEED ^ 0x394);
    // row 407
    assert_eq!(sz2("crypto_auth_hmacsha256_bytes"), 32);
    assert_eq!(sz2("crypto_auth_hmacsha256_keybytes"), 32);
    assert!(sz2("crypto_auth_hmacsha256_statebytes") >= 208);
}

/// CONFIGS.md rows 408-414: `crypto_auth_hmacsha512`.
#[test]
fn g3_cfg_408_414_hmacsha512() {
    hmac_family("crypto_auth_hmacsha512", 64, 128, SEED ^ 0x408);
    // row 414
    assert_eq!(sz2("crypto_auth_hmacsha512_bytes"), 64);
    assert_eq!(sz2("crypto_auth_hmacsha512_keybytes"), 32);
    assert!(sz2("crypto_auth_hmacsha512_statebytes") >= 416);
}

/// CONFIGS.md rows 415-422: `crypto_auth_hmacsha512256`, its relationship to
/// hmacsha512, and the `crypto_auth` default alias.
#[test]
fn g3_cfg_415_422_hmacsha512256_and_crypto_auth() {
    hmac_family("crypto_auth_hmacsha512256", 32, 128, SEED ^ 0x415);
    // row 420
    assert_eq!(sz2("crypto_auth_hmacsha512256_bytes"), 32);
    assert_eq!(sz2("crypto_auth_hmacsha512256_keybytes"), 32);
    // row 422
    assert_eq!(sz2("crypto_auth_bytes"), 32);
    assert_eq!(sz2("crypto_auth_keybytes"), 32);
    assert_eq!(prim2("crypto_auth_primitive"), "hmacsha512256");

    unsafe {
        let h512 = pair2!(OneShotK, "crypto_auth_hmacsha512");
        let h512256 = pair2!(OneShotK, "crypto_auth_hmacsha512256");
        let auth = pair2!(OneShotK, "crypto_auth");
        let averify = pair2!(VerifyK, "crypto_auth_verify");
        let vverify = pair2!(VerifyK, "crypto_auth_hmacsha512256_verify");
        let mut rng = Rng::new(SEED ^ 0x417);
        for inlen in [0usize, 1, 111, 112, 127, 128, 129, 256, 300] {
            let key = rng.bytes(32);
            let msg = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { msg.as_ptr() };
            let mut t32 = [vec![0u8; 32], vec![0u8; 32]];
            let mut t64 = [vec![0u8; 64], vec![0u8; 64]];
            let mut ta = [vec![0u8; 32], vec![0u8; 32]];
            for i in 0..2 {
                assert_eq!(h512256[i](t32[i].as_mut_ptr(), ip, inlen as u64, key.as_ptr()), 0);
                assert_eq!(h512[i](t64[i].as_mut_ptr(), ip, inlen as u64, key.as_ptr()), 0);
                assert_eq!(auth[i](ta[i].as_mut_ptr(), ip, inlen as u64, key.as_ptr()), 0);
            }
            eq_bytes(&format!("row415 hmacsha512256 inlen={inlen}"), &t32[0], &t32[1]);
            // row 417: truncation of the hmacsha512 MAC
            for i in 0..2 {
                eq_bytes(
                    &format!("row417 lib{i} inlen={inlen} truncation"),
                    &t64[i][..32],
                    &t32[i],
                );
                // row 421: crypto_auth == hmacsha512256
                eq_bytes(&format!("row421 lib{i} inlen={inlen}"), &ta[i], &t32[i]);
                assert_eq!(averify[i](ta[i].as_ptr(), ip, inlen as u64, key.as_ptr()), 0);
                assert_eq!(vverify[i](t32[i].as_ptr(), ip, inlen as u64, key.as_ptr()), 0);
            }
            // row 421: mismatching crypto_auth_verify
            let mut bogus = ta[0].clone();
            bogus[5] ^= 0x20;
            let mut r = [0i32; 2];
            for i in 0..2 {
                r[i] = averify[i](bogus.as_ptr(), ip, inlen as u64, key.as_ptr());
            }
            eq_i32(&format!("row421 crypto_auth_verify bad inlen={inlen}"), r[0], r[1]);
            assert_eq!(r[0], -1);
        }
        // row 422: crypto_auth_keygen
        install_det_random();
        let kg = pair2!(Keygen, "crypto_auth_keygen");
        det_reseed(0x0011_2233_4455_6677);
        let mut k0 = vec![0xAAu8; 40];
        kg[0](k0.as_mut_ptr());
        det_reseed(0x0011_2233_4455_6677);
        let mut k1 = vec![0xAAu8; 40];
        kg[1](k1.as_mut_ptr());
        eq_bytes("row422 crypto_auth_keygen", &k0, &k1);
        assert_eq!(&k0[32..], &[0xAAu8; 8]);
    }
}

// ===========================================================================
// crypto_onetimeauth (poly1305)
// ===========================================================================

#[repr(C)]
struct PolyImpl {
    onetimeauth: Option<OneShotK>,
    onetimeauth_verify: Option<VerifyK>,
    onetimeauth_init: Option<PolyInit>,
    onetimeauth_update: Option<StUpd>,
    onetimeauth_final: Option<StFin>,
}

/// CONFIGS.md rows 423-440: poly1305 one-shot, streaming, verify, keygen,
/// constants, the exported `donna` implementation struct, the
/// `pick_best_implementation` hook and the `crypto_onetimeauth` alias.
#[test]
fn g3_cfg_423_440_poly1305() {
    unsafe {
        let fam = "crypto_onetimeauth_poly1305";
        let one = pair2!(OneShotK, fam);
        let ver = pair2!(VerifyK, &format!("{fam}_verify"));
        let init = pair2!(PolyInit, &format!("{fam}_init"));
        let upd = pair2!(StUpd, &format!("{fam}_update"));
        let fin = pair2!(StFin, &format!("{fam}_final"));
        let nstate = sz2(&format!("{fam}_statebytes"));
        // row 437
        assert_eq!(sz2(&format!("{fam}_bytes")), 16);
        assert_eq!(sz2(&format!("{fam}_keybytes")), 32);
        assert_eq!(nstate, 256);
        let mut rng = Rng::new(SEED ^ 0x423);

        // --- rows 423-426: one-shot -----------------------------------------
        let mut keys: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xFFu8; 32]];
        for _ in 0..3 {
            keys.push(rng.bytes(32));
        }
        for inlen in (0..=64usize).chain([127usize, 128, 129, 1024]) {
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            for (ki, key) in keys.iter().enumerate() {
                let mut o = [vec![0xAAu8; 16], vec![0xAAu8; 16]];
                let mut r = [0i32; 2];
                for i in 0..2 {
                    r[i] = one[i](o[i].as_mut_ptr(), ip, inlen as u64, key.as_ptr());
                }
                let ctx = format!("rows423-426 poly1305 inlen={inlen} key{ki}");
                eq_i32(&ctx, r[0], r[1]);
                eq_bytes(&ctx, &o[0], &o[1]);
                assert_eq!(r[0], 0);
                // row 434: verify accepts the correct tag
                for i in 0..2 {
                    assert_eq!(ver[i](o[i].as_ptr(), ip, inlen as u64, key.as_ptr()), 0);
                }
            }
        }

        // --- rows 427-433: streaming ----------------------------------------
        let shapes: Vec<Vec<usize>> = vec![
            vec![],           // row 428
            vec![0],          // row 429
            vec![0, 0, 0],
            vec![1; 64],      // row 430
            vec![15, 1],      // row 431
            vec![16, 1],      // row 431
            vec![1, 15],      // row 431
            vec![1, 31],      // row 431
            vec![8, 8],       // row 431
            vec![8, 24],      // row 431
            vec![16, 16],     // row 431
            vec![17, 15],     // row 431
            vec![31, 33],     // row 431
            vec![5, 11],      // row 432: second chunk exactly completes leftover
            vec![20, 5],      // row 433: 9 leftover bytes at _final
            vec![1, 0, 15, 0],
            vec![64],         // row 427: single update
            vec![1024],
            vec![100, 3],
        ];
        for sh in &shapes {
            let total: usize = sh.iter().sum();
            let data = rng.bytes(total);
            let chunks = split(&data, sh);
            for (ki, key) in keys.iter().enumerate() {
                let mut outs: [Vec<u8>; 2] = [vec![0xAAu8; 16], vec![0xAAu8; 16]];
                let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
                for i in 0..2 {
                    let st = St::new(nstate);
                    rets[i].push(init[i](st.ptr(), key.as_ptr()));
                    for ch in &chunks {
                        let p = if ch.is_empty() { ptr::null() } else { ch.as_ptr() };
                        rets[i].push(upd[i](st.ptr(), p, ch.len() as u64));
                    }
                    rets[i].push(fin[i](st.ptr(), outs[i].as_mut_ptr()));
                }
                let ctx = format!("rows427-433 poly1305 split={sh:?} key{ki}");
                assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
                assert!(rets[0].iter().all(|&r| r == 0), "{ctx}: C returned {:?}", rets[0]);
                eq_bytes(&ctx, &outs[0], &outs[1]);
                // row 427: streaming == one-shot
                for i in 0..2 {
                    let mut os = vec![0u8; 16];
                    let ip = if total == 0 { ptr::null() } else { data.as_ptr() };
                    assert_eq!(one[i](os.as_mut_ptr(), ip, total as u64, key.as_ptr()), 0);
                    eq_bytes(&format!("{ctx} lib{i} stream==oneshot"), &os, &outs[i]);
                }
            }
        }

        // --- row 435: verify mismatches --------------------------------------
        let key = rng.bytes(32);
        let key2 = rng.bytes(32);
        let msg = rng.bytes(48);
        let mut msg2 = msg.clone();
        msg2[7] ^= 4;
        let mut tag = vec![0u8; 16];
        assert_eq!(one[0](tag.as_mut_ptr(), msg.as_ptr(), 48, key.as_ptr()), 0);
        let mut cases: Vec<(String, Vec<u8>, Vec<u8>, Vec<u8>)> = Vec::new();
        let mut t = tag.clone();
        t[0] ^= 1;
        cases.push(("flip byte0".into(), t, msg.clone(), key.clone()));
        let mut t = tag.clone();
        t[15] ^= 0x80;
        cases.push(("flip byte15".into(), t, msg.clone(), key.clone()));
        cases.push(("all-zero h".into(), vec![0u8; 16], msg.clone(), key.clone()));
        cases.push(("wrong key".into(), tag.clone(), msg.clone(), key2.clone()));
        cases.push(("wrong message".into(), tag.clone(), msg2.clone(), key.clone()));
        for (label, h, m, k) in cases {
            let mut r = [0i32; 2];
            for i in 0..2 {
                r[i] = ver[i](h.as_ptr(), m.as_ptr(), m.len() as u64, k.as_ptr());
            }
            eq_i32(&format!("row435 poly1305 verify {label}"), r[0], r[1]);
            assert_eq!(r[0], -1, "row435 {label}: C accepted a bad tag");
        }

        // --- row 436: keygen -------------------------------------------------
        install_det_random();
        let kg = pair2!(Keygen, &format!("{fam}_keygen"));
        det_reseed(0x7777_8888_9999_AAAA);
        let mut k0 = vec![0xAAu8; 40];
        kg[0](k0.as_mut_ptr());
        det_reseed(0x7777_8888_9999_AAAA);
        let mut k1 = vec![0xAAu8; 40];
        kg[1](k1.as_mut_ptr());
        eq_bytes("row436 poly1305_keygen", &k0, &k1);
        assert_eq!(&k0[32..], &[0xAAu8; 8]);
        let mut o = [vec![0u8; 16], vec![0u8; 16]];
        for i in 0..2 {
            assert_eq!(one[i](o[i].as_mut_ptr(), msg.as_ptr(), 48, k0.as_ptr()), 0);
            assert_eq!(ver[i](o[i].as_ptr(), msg.as_ptr(), 48, k0.as_ptr()), 0);
        }
        eq_bytes("row436 keygen round-trip", &o[0], &o[1]);

        // --- row 438: the exported `donna` implementation struct -------------
        // NOTE: `libloading::Symbol<T>` derefs to the *stored address*
        // reinterpreted as `T`, so a data symbol must be fetched as
        // `Symbol<*const T>` and then dereferenced once.
        let dimpl = pair2!(*const PolyImpl, "crypto_onetimeauth_poly1305_donna_implementation");
        let impls: [&PolyImpl; 2] = [&*dimpl[0], &*dimpl[1]];
        for inlen in [0usize, 1, 15, 16, 17, 32, 33, 64, 1024] {
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let mut o = [vec![0xAAu8; 16], vec![0xAAu8; 16]];
            let mut so = [vec![0xBBu8; 16], vec![0xBBu8; 16]];
            for i in 0..2 {
                let im = impls[i];
                assert_eq!(
                    (im.onetimeauth.unwrap())(o[i].as_mut_ptr(), ip, inlen as u64, key.as_ptr()),
                    0
                );
                assert_eq!(
                    (im.onetimeauth_verify.unwrap())(
                        o[i].as_ptr(),
                        ip,
                        inlen as u64,
                        key.as_ptr()
                    ),
                    0
                );
                let st = St::new(nstate);
                assert_eq!((im.onetimeauth_init.unwrap())(st.ptr(), key.as_ptr()), 0);
                assert_eq!((im.onetimeauth_update.unwrap())(st.ptr(), ip, inlen as u64), 0);
                assert_eq!((im.onetimeauth_final.unwrap())(st.ptr(), so[i].as_mut_ptr()), 0);
                // must match the dispatched public API
                let mut pub_tag = vec![0u8; 16];
                assert_eq!(one[i](pub_tag.as_mut_ptr(), ip, inlen as u64, key.as_ptr()), 0);
                eq_bytes(&format!("row438 lib{i} donna==public inlen={inlen}"), &pub_tag, &o[i]);
                eq_bytes(&format!("row438 lib{i} donna stream inlen={inlen}"), &pub_tag, &so[i]);
            }
            eq_bytes(&format!("row438 donna one-shot inlen={inlen}"), &o[0], &o[1]);
            eq_bytes(&format!("row438 donna streaming inlen={inlen}"), &so[0], &so[1]);
        }

        // --- row 439: pick_best_implementation ------------------------------
        let pb = pair2!(PickBest, "_crypto_onetimeauth_poly1305_pick_best_implementation");
        let ra = pb[0]();
        let rb = pb[1]();
        eq_i32("row439 poly1305 pick_best_implementation", ra, rb);
        for inlen in [0usize, 1, 16, 17, 33, 64, 1024] {
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            let mut o = [vec![0u8; 16], vec![0u8; 16]];
            for i in 0..2 {
                assert_eq!(one[i](o[i].as_mut_ptr(), ip, inlen as u64, key.as_ptr()), 0);
            }
            eq_bytes(&format!("row439 after pick_best inlen={inlen}"), &o[0], &o[1]);
        }

        // --- row 440: the crypto_onetimeauth alias --------------------------
        assert_eq!(sz2("crypto_onetimeauth_bytes"), 16);
        assert_eq!(sz2("crypto_onetimeauth_keybytes"), 32);
        assert_eq!(sz2("crypto_onetimeauth_statebytes"), nstate);
        assert_eq!(prim2("crypto_onetimeauth_primitive"), "poly1305");
        let aone = pair2!(OneShotK, "crypto_onetimeauth");
        let aver = pair2!(VerifyK, "crypto_onetimeauth_verify");
        let ainit = pair2!(PolyInit, "crypto_onetimeauth_init");
        let aupd = pair2!(StUpd, "crypto_onetimeauth_update");
        let afin = pair2!(StFin, "crypto_onetimeauth_final");
        for sh in [vec![0usize], vec![16, 1], vec![1; 33], vec![48]] {
            let total: usize = sh.iter().sum();
            let data = rng.bytes(total);
            let chunks = split(&data, &sh);
            let mut outs: [Vec<u8>; 2] = [vec![0u8; 16], vec![0u8; 16]];
            let mut refs: [Vec<u8>; 2] = [vec![0u8; 16], vec![0u8; 16]];
            for i in 0..2 {
                let st = St::new(nstate);
                assert_eq!(ainit[i](st.ptr(), key.as_ptr()), 0);
                for ch in &chunks {
                    let p = if ch.is_empty() { ptr::null() } else { ch.as_ptr() };
                    assert_eq!(aupd[i](st.ptr(), p, ch.len() as u64), 0);
                }
                assert_eq!(afin[i](st.ptr(), outs[i].as_mut_ptr()), 0);
                let ip = if total == 0 { ptr::null() } else { data.as_ptr() };
                let mut a = vec![0u8; 16];
                assert_eq!(aone[i](a.as_mut_ptr(), ip, total as u64, key.as_ptr()), 0);
                assert_eq!(aver[i](a.as_ptr(), ip, total as u64, key.as_ptr()), 0);
                assert_eq!(one[i](refs[i].as_mut_ptr(), ip, total as u64, key.as_ptr()), 0);
                eq_bytes(&format!("row440 lib{i} alias one-shot split={sh:?}"), &a, &refs[i]);
                eq_bytes(&format!("row440 lib{i} alias streaming split={sh:?}"), &outs[i], &refs[i]);
            }
            eq_bytes(&format!("row440 alias streaming split={sh:?}"), &outs[0], &outs[1]);
        }
        // row 440: crypto_onetimeauth_keygen
        let akg = pair2!(Keygen, "crypto_onetimeauth_keygen");
        det_reseed(0x1010_2020_3030_4040);
        let mut a0 = vec![0xAAu8; 40];
        akg[0](a0.as_mut_ptr());
        det_reseed(0x1010_2020_3030_4040);
        let mut a1 = vec![0xAAu8; 40];
        akg[1](a1.as_mut_ptr());
        eq_bytes("row440 crypto_onetimeauth_keygen", &a0, &a1);
        assert_eq!(&a0[32..], &[0xAAu8; 8]);
    }
}

// ===========================================================================
// crypto_kdf
// ===========================================================================

/// CONFIGS.md rows 441-452: `crypto_kdf_blake2b_derive_from_key`, the
/// `crypto_kdf_derive_from_key` wrapper and the kdf constants.
#[test]
fn g3_cfg_441_452_kdf_blake2b() {
    unsafe {
        let d = pair2!(KdfDerive, "crypto_kdf_blake2b_derive_from_key");
        let w = pair2!(KdfDerive, "crypto_kdf_derive_from_key");
        let sp = pair2!(GhSp, "crypto_generichash_blake2b_salt_personal");
        let mut rng = Rng::new(SEED ^ 0x441);

        // row 452 / 451
        assert_eq!(sz2("crypto_kdf_blake2b_bytes_min"), 16);
        assert_eq!(sz2("crypto_kdf_blake2b_bytes_max"), 64);
        assert_eq!(sz2("crypto_kdf_blake2b_contextbytes"), 8);
        assert_eq!(sz2("crypto_kdf_blake2b_keybytes"), 32);
        assert_eq!(sz2("crypto_kdf_bytes_min"), 16);
        assert_eq!(sz2("crypto_kdf_bytes_max"), 64);
        assert_eq!(sz2("crypto_kdf_contextbytes"), 8);
        assert_eq!(sz2("crypto_kdf_keybytes"), 32);
        assert_eq!(prim2("crypto_kdf_primitive"), "blake2b");

        let ctxs: Vec<[u8; 8]> = vec![
            [0u8; 8],
            [0xFFu8; 8],
            *b"context1",
            *b"ctx\0\0abc",
            [1, 0, 2, 0, 3, 0, 4, 0],
        ];
        let keys: Vec<Vec<u8>> = vec![
            vec![0u8; 32],
            vec![0xFFu8; 32],
            rng.bytes(32),
            rng.bytes(32),
        ];
        let ids: Vec<u64> = vec![
            0,
            1,
            2,
            255,
            256,
            0xFFFF_FFFF,
            0x1_0000_0000,
            u64::MAX,
            0x0102_0304_0506_0708,
        ];

        for subkey_len in 16..=64usize {
            for &id in &ids {
                let ctx = &ctxs[(subkey_len ^ (id as usize)) % ctxs.len()];
                let key = &keys[subkey_len % keys.len()];
                let mut o = [vec![0xAAu8; subkey_len + 4], vec![0xAAu8; subkey_len + 4]];
                let mut ow = [vec![0x55u8; subkey_len + 4], vec![0x55u8; subkey_len + 4]];
                let mut r = [0i32; 2];
                let mut rw = [0i32; 2];
                for i in 0..2 {
                    r[i] = d[i](
                        o[i].as_mut_ptr(),
                        subkey_len,
                        id,
                        ctx.as_ptr() as *const c_char,
                        key.as_ptr(),
                    );
                    rw[i] = w[i](
                        ow[i].as_mut_ptr(),
                        subkey_len,
                        id,
                        ctx.as_ptr() as *const c_char,
                        key.as_ptr(),
                    );
                }
                let label = format!("rows441-450 len={subkey_len} id={id:#x}");
                eq_i32(&label, r[0], r[1]);
                eq_i32(&format!("{label} wrapper"), rw[0], rw[1]);
                assert_eq!(r[0], 0, "{label}: C rejected a valid call");
                eq_bytes(&label, &o[0], &o[1]);
                // row 450: the wrapper must be identical to the blake2b entry
                for i in 0..2 {
                    eq_bytes(
                        &format!("{label} row450 lib{i}"),
                        &o[i][..subkey_len],
                        &ow[i][..subkey_len],
                    );
                    assert_eq!(&o[i][subkey_len..], &[0xAAu8; 4], "{label}: overrun");
                    assert_eq!(&ow[i][subkey_len..], &[0x55u8; 4], "{label}: wrapper overrun");
                }
                // row 448: equals generichash_blake2b_salt_personal with
                // salt = LE64(subkey_id) || zeros, personal = ctx || zeros
                let mut salt = [0u8; 16];
                salt[..8].copy_from_slice(&id.to_le_bytes());
                let mut personal = [0u8; 16];
                personal[..8].copy_from_slice(ctx);
                for i in 0..2 {
                    let mut g = vec![0u8; subkey_len];
                    assert_eq!(
                        sp[i](
                            g.as_mut_ptr(),
                            subkey_len,
                            ptr::null(),
                            0,
                            key.as_ptr(),
                            32,
                            salt.as_ptr(),
                            personal.as_ptr()
                        ),
                        0
                    );
                    eq_bytes(&format!("{label} row448 lib{i}"), &g, &o[i][..subkey_len]);
                }
            }
        }

        // row 447: domain separation between two different contexts
        let key = rng.bytes(32);
        for i in 0..2 {
            let mut a = vec![0u8; 32];
            let mut b = vec![0u8; 32];
            assert_eq!(
                d[i](a.as_mut_ptr(), 32, 7, b"ctxAAAAA".as_ptr() as *const c_char, key.as_ptr()),
                0
            );
            assert_eq!(
                d[i](b.as_mut_ptr(), 32, 7, b"ctxBBBBB".as_ptr() as *const c_char, key.as_ptr()),
                0
            );
            assert_ne!(a, b, "row447 lib{i}: contexts are not domain separated");
        }

        // row 451: crypto_kdf_keygen
        install_det_random();
        let kg = pair2!(Keygen, "crypto_kdf_keygen");
        det_reseed(0xFEED_FACE_CAFE_0001);
        let mut k0 = vec![0xAAu8; 40];
        kg[0](k0.as_mut_ptr());
        det_reseed(0xFEED_FACE_CAFE_0001);
        let mut k1 = vec![0xAAu8; 40];
        kg[1](k1.as_mut_ptr());
        eq_bytes("row451 crypto_kdf_keygen", &k0, &k1);
        assert_eq!(&k0[32..], &[0xAAu8; 8]);
    }
}

/// Shared driver for the two HKDF families.
fn hkdf_family(fam: &str, keybytes: usize, block: usize, bytes_max: usize, seed: u64) {
    unsafe {
        let extract = pair2!(HkdfExtract, &format!("{fam}_extract"));
        let einit = pair2!(HkdfExtractInit, &format!("{fam}_extract_init"));
        let eupd = pair2!(HkdfExtractUpd, &format!("{fam}_extract_update"));
        let efin = pair2!(HkdfExtractFin, &format!("{fam}_extract_final"));
        let expand = pair2!(HkdfExpand, &format!("{fam}_expand"));
        let nstate = sz2(&format!("{fam}_statebytes"));
        assert_eq!(sz2(&format!("{fam}_keybytes")), keybytes);
        assert_eq!(sz2(&format!("{fam}_bytes_min")), 0);
        assert_eq!(sz2(&format!("{fam}_bytes_max")), bytes_max);
        // statebytes must equal the underlying hmac state size
        let hmac_state = if keybytes == 32 {
            sz2("crypto_auth_hmacsha256_statebytes")
        } else {
            sz2("crypto_auth_hmacsha512_statebytes")
        };
        assert_eq!(nstate, hmac_state, "{fam}_statebytes vs hmac statebytes");

        let mut rng = Rng::new(seed);

        // --- extract: salt_len and ikm_len matrix ---------------------------
        let mut salt_lens: Vec<usize> =
            vec![0, 1, 13, 32, 64, block - 1, block, block + 1, block + 72, 200, 300];
        salt_lens.sort_unstable();
        salt_lens.dedup();
        let ikm_lens: Vec<usize> = vec![0, 1, 22, 32, 64, 80, block, block + 1, 160, 1000];
        for &sl in &salt_lens {
            let salt = rng.bytes(sl.max(1));
            for &il in &ikm_lens {
                let ikm = rng.bytes(il.max(1));
                let sp = if sl == 0 { ptr::null() } else { salt.as_ptr() };
                let mut prk = [vec![0xAAu8; keybytes + 4], vec![0xAAu8; keybytes + 4]];
                let mut r = [0i32; 2];
                for i in 0..2 {
                    r[i] = extract[i](prk[i].as_mut_ptr(), sp, sl, ikm.as_ptr(), il);
                }
                let ctx = format!("{fam}_extract salt_len={sl} ikm_len={il}");
                eq_i32(&ctx, r[0], r[1]);
                eq_bytes(&ctx, &prk[0], &prk[1]);
                assert_eq!(r[0], 0);
                assert_eq!(&prk[0][keybytes..], &[0xAAu8; 4], "{ctx}: overrun");
            }
        }

        // --- streaming extract ---------------------------------------------
        let shapes: Vec<Vec<usize>> = vec![
            vec![],
            vec![0],
            vec![0, 0],
            vec![1; 130],
            vec![block - 1, 1],
            vec![block, 1],
            vec![1, block - 1],
            vec![block, block],
            vec![32, 0, 32],
            vec![200],
        ];
        for &sl in &[0usize, 1, 32, block, block + 1] {
            let salt = rng.bytes(sl.max(1));
            let sptr = if sl == 0 { ptr::null() } else { salt.as_ptr() };
            for sh in &shapes {
                let total: usize = sh.iter().sum();
                let data = rng.bytes(total);
                let chunks = split(&data, sh);
                let mut prks: [Vec<u8>; 2] = [vec![0xAAu8; keybytes], vec![0xAAu8; keybytes]];
                let mut rets: [Vec<c_int>; 2] = [Vec::new(), Vec::new()];
                for i in 0..2 {
                    let st = St::new(nstate);
                    rets[i].push(einit[i](st.ptr(), sptr, sl));
                    for ch in &chunks {
                        let p = if ch.is_empty() { ptr::null() } else { ch.as_ptr() };
                        rets[i].push(eupd[i](st.ptr(), p, ch.len()));
                    }
                    rets[i].push(efin[i](st.ptr(), prks[i].as_mut_ptr()));
                }
                let ctx = format!("{fam} streaming extract salt_len={sl} split={sh:?}");
                assert_eq!(rets[0], rets[1], "{ctx}: return values differ");
                assert!(rets[0].iter().all(|&r| r == 0), "{ctx}: C returned {:?}", rets[0]);
                eq_bytes(&ctx, &prks[0], &prks[1]);
                // must equal the one-shot extract
                for i in 0..2 {
                    let mut p = vec![0u8; keybytes];
                    // `ikm` is declared `nonnull(4)`: an empty Vec's pointer is
                    // non-NULL (dangling but never dereferenced when len == 0).
                    assert_eq!(extract[i](p.as_mut_ptr(), sptr, sl, data.as_ptr(), total), 0);
                    eq_bytes(&format!("{ctx} lib{i} stream==oneshot"), &p, &prks[i]);
                }
            }
        }

        // --- expand: out_len and ctx_len matrix -----------------------------
        let prks: Vec<Vec<u8>> = vec![
            vec![0u8; keybytes],
            vec![0xFFu8; keybytes],
            rng.bytes(keybytes),
        ];
        let mut out_lens: Vec<usize> = vec![
            0,
            1,
            keybytes - 1,
            keybytes,
            keybytes + 1,
            2 * keybytes,
            3 * keybytes,
            3 * keybytes + 4,
            4 * keybytes,
            6 * keybytes,
            100,
            128,
            192,
            200,
            bytes_max - 1,
            bytes_max,
        ];
        out_lens.sort_unstable();
        out_lens.dedup();
        let ctx_lens: Vec<usize> = vec![0, 1, 8, 10, 32, 64, 100, block, 1000];
        for (pi, prk) in prks.iter().enumerate() {
            for &ol in &out_lens {
                for &cl in &ctx_lens {
                    let cbuf = rng.bytes(cl.max(1));
                    let cp = if cl == 0 {
                        ptr::null()
                    } else {
                        cbuf.as_ptr() as *const c_char
                    };
                    let mut o = [vec![0xAAu8; ol + 4], vec![0xAAu8; ol + 4]];
                    let mut r = [0i32; 2];
                    for i in 0..2 {
                        r[i] = expand[i](o[i].as_mut_ptr(), ol, cp, cl, prk.as_ptr());
                    }
                    let ctx = format!("{fam}_expand prk{pi} out_len={ol} ctx_len={cl}");
                    eq_i32(&ctx, r[0], r[1]);
                    eq_bytes(&ctx, &o[0], &o[1]);
                    assert_eq!(r[0], 0, "{ctx}: C rejected a valid call");
                    assert_eq!(&o[0][ol..], &[0xAAu8; 4], "{ctx}: overrun");
                }
            }
        }

        // ctx with embedded NULs / non-ASCII bytes
        let weird: [u8; 10] = [b'a', 0, b'b', 0xFF, 0x80, 0, 0, b'z', 0x7F, 0x01];
        for &ol in &[1usize, keybytes, 100] {
            let mut o = [vec![0u8; ol], vec![0u8; ol]];
            for i in 0..2 {
                assert_eq!(
                    expand[i](
                        o[i].as_mut_ptr(),
                        ol,
                        weird.as_ptr() as *const c_char,
                        weird.len(),
                        prks[2].as_ptr()
                    ),
                    0
                );
            }
            eq_bytes(&format!("{fam}_expand weird ctx out_len={ol}"), &o[0], &o[1]);
        }

        // --- rows 474/487: the RFC 5869 test-vector chain -------------------
        // RFC 5869 A.1: IKM = 0x0b x 22, salt = 0x000102..0c (13 bytes),
        // info = 0xf0f1..f9 (10 bytes), L = 42. (There is no official SHA-512
        // vector in the RFC; the same inputs are reused for hkdf_sha512.)
        {
            let ikm = vec![0x0Bu8; 22];
            let salt: Vec<u8> = (0u8..13).collect();
            let info: Vec<u8> = (0xF0u8..0xFA).collect();
            let mut prk = [vec![0u8; keybytes], vec![0u8; keybytes]];
            let mut okm = [vec![0u8; 42], vec![0u8; 42]];
            for i in 0..2 {
                assert_eq!(
                    extract[i](prk[i].as_mut_ptr(), salt.as_ptr(), 13, ikm.as_ptr(), 22),
                    0
                );
                assert_eq!(
                    expand[i](
                        okm[i].as_mut_ptr(),
                        42,
                        info.as_ptr() as *const c_char,
                        10,
                        prk[i].as_ptr()
                    ),
                    0
                );
            }
            eq_bytes(&format!("{fam} RFC5869 A.1 prk"), &prk[0], &prk[1]);
            eq_bytes(&format!("{fam} RFC5869 A.1 okm"), &okm[0], &okm[1]);
            // RFC 5869 A.3: zero-length salt and info
            let mut prk2 = [vec![0u8; keybytes], vec![0u8; keybytes]];
            let mut okm2 = [vec![0u8; 42], vec![0u8; 42]];
            for i in 0..2 {
                assert_eq!(
                    extract[i](prk2[i].as_mut_ptr(), ptr::null(), 0, ikm.as_ptr(), 22),
                    0
                );
                assert_eq!(
                    expand[i](okm2[i].as_mut_ptr(), 42, ptr::null(), 0, prk2[i].as_ptr()),
                    0
                );
            }
            eq_bytes(&format!("{fam} RFC5869 A.3 prk"), &prk2[0], &prk2[1]);
            eq_bytes(&format!("{fam} RFC5869 A.3 okm"), &okm2[0], &okm2[1]);
        }

        // --- further extract + expand chains --------------------------------
        for &(sl, il, cl, ol) in &[
            (13usize, 22usize, 10usize, 42usize),
            (0, 22, 0, 82),
            (80, 80, 80, 82),
            (block + 1, 1000, 1000, 3 * keybytes + 1),
        ] {
            let salt = rng.bytes(sl.max(1));
            let ikm = rng.bytes(il.max(1));
            let info = rng.bytes(cl.max(1));
            let sp = if sl == 0 { ptr::null() } else { salt.as_ptr() };
            let ip = if cl == 0 {
                ptr::null()
            } else {
                info.as_ptr() as *const c_char
            };
            let mut out = [vec![0u8; ol], vec![0u8; ol]];
            let mut prk = [vec![0u8; keybytes], vec![0u8; keybytes]];
            for i in 0..2 {
                assert_eq!(extract[i](prk[i].as_mut_ptr(), sp, sl, ikm.as_ptr(), il), 0);
                assert_eq!(expand[i](out[i].as_mut_ptr(), ol, ip, cl, prk[i].as_ptr()), 0);
            }
            eq_bytes(&format!("{fam} chain sl={sl} il={il} cl={cl} ol={ol} prk"), &prk[0], &prk[1]);
            eq_bytes(&format!("{fam} chain sl={sl} il={il} cl={cl} ol={ol}"), &out[0], &out[1]);
        }

        // --- keygen ---------------------------------------------------------
        install_det_random();
        let kg = pair2!(Keygen, &format!("{fam}_keygen"));
        det_reseed(0x2222_3333_4444_5555);
        let mut k0 = vec![0xAAu8; keybytes + 8];
        kg[0](k0.as_mut_ptr());
        det_reseed(0x2222_3333_4444_5555);
        let mut k1 = vec![0xAAu8; keybytes + 8];
        kg[1](k1.as_mut_ptr());
        eq_bytes(&format!("{fam}_keygen"), &k0, &k1);
        assert_eq!(&k0[keybytes..], &[0xAAu8; 8], "{fam}_keygen wrote past keybytes");
        let mut o = [vec![0u8; 64], vec![0u8; 64]];
        for i in 0..2 {
            assert_eq!(
                expand[i](o[i].as_mut_ptr(), 64, b"info\0\0\0\0".as_ptr() as *const c_char, 8, k0.as_ptr()),
                0
            );
        }
        eq_bytes(&format!("{fam}_keygen expand round-trip"), &o[0], &o[1]);
    }
}

/// CONFIGS.md rows 453-476: `crypto_kdf_hkdf_sha256_*`.
#[test]
fn g3_cfg_453_476_hkdf_sha256() {
    hkdf_family("crypto_kdf_hkdf_sha256", 32, 64, 8160, SEED ^ 0x453);
}

/// CONFIGS.md rows 477-488: `crypto_kdf_hkdf_sha512_*`.
#[test]
fn g3_cfg_477_488_hkdf_sha512() {
    hkdf_family("crypto_kdf_hkdf_sha512", 64, 128, 16320, SEED ^ 0x477);
}

// ===========================================================================
// crypto_shorthash
// ===========================================================================

/// CONFIGS.md rows 489-501: `crypto_shorthash_siphash24`.
#[test]
fn g3_cfg_489_501_siphash24() {
    unsafe {
        let f = pair2!(OneShotK, "crypto_shorthash_siphash24");
        // row 501
        assert_eq!(sz2("crypto_shorthash_siphash24_bytes"), 8);
        assert_eq!(sz2("crypto_shorthash_siphash24_keybytes"), 16);
        let mut rng = Rng::new(SEED ^ 0x489);
        // row 500: canonical vector key, all-zero, all-0xFF, random
        let canon: Vec<u8> = (0u8..16).collect();
        let keys: Vec<Vec<u8>> = vec![
            canon,
            vec![0u8; 16],
            vec![0xFFu8; 16],
            rng.bytes(16),
            rng.bytes(16),
        ];
        // rows 489-499: inlen sweep 0..=64 plus 255/256 and a few larger
        for inlen in (0..=64usize).chain([100usize, 255, 256, 257, 1000]) {
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            for (ki, k) in keys.iter().enumerate() {
                let mut o = [vec![0xAAu8; 8 + 4], vec![0xAAu8; 8 + 4]];
                let mut r = [0i32; 2];
                for i in 0..2 {
                    r[i] = f[i](o[i].as_mut_ptr(), ip, inlen as u64, k.as_ptr());
                }
                let ctx = format!("rows489-500 siphash24 inlen={inlen} key{ki}");
                eq_i32(&ctx, r[0], r[1]);
                eq_bytes(&ctx, &o[0], &o[1]);
                assert_eq!(r[0], 0);
                assert_eq!(&o[0][8..], &[0xAAu8; 4], "{ctx}: overrun");
            }
        }
    }
}

/// CONFIGS.md rows 502-509: `crypto_shorthash_siphashx24` and the
/// `crypto_shorthash` default alias.
#[test]
fn g3_cfg_502_509_siphashx24_and_shorthash() {
    unsafe {
        let x = pair2!(OneShotK, "crypto_shorthash_siphashx24");
        let s = pair2!(OneShotK, "crypto_shorthash_siphash24");
        let g = pair2!(OneShotK, "crypto_shorthash");
        // rows 508, 509
        assert_eq!(sz2("crypto_shorthash_siphashx24_bytes"), 16);
        assert_eq!(sz2("crypto_shorthash_siphashx24_keybytes"), 16);
        assert_eq!(sz2("crypto_shorthash_bytes"), 8);
        assert_eq!(sz2("crypto_shorthash_keybytes"), 16);
        assert_eq!(prim2("crypto_shorthash_primitive"), "siphash24");

        let mut rng = Rng::new(SEED ^ 0x502);
        let canon: Vec<u8> = (0u8..16).collect();
        let keys: Vec<Vec<u8>> = vec![
            canon,
            vec![0u8; 16],
            vec![0xFFu8; 16],
            rng.bytes(16),
        ];
        let mut ever_equal_prefix = false;
        for inlen in (0..=64usize).chain([100usize, 255, 256, 1000]) {
            let input = rng.bytes(inlen);
            let ip = if inlen == 0 { ptr::null() } else { input.as_ptr() };
            for (ki, k) in keys.iter().enumerate() {
                let mut o = [vec![0xAAu8; 16 + 4], vec![0xAAu8; 16 + 4]];
                let mut sh = [vec![0xAAu8; 8 + 4], vec![0xAAu8; 8 + 4]];
                let mut gh = [vec![0xAAu8; 8 + 4], vec![0xAAu8; 8 + 4]];
                let mut r = [0i32; 2];
                for i in 0..2 {
                    r[i] = x[i](o[i].as_mut_ptr(), ip, inlen as u64, k.as_ptr());
                    assert_eq!(s[i](sh[i].as_mut_ptr(), ip, inlen as u64, k.as_ptr()), 0);
                    assert_eq!(g[i](gh[i].as_mut_ptr(), ip, inlen as u64, k.as_ptr()), 0);
                }
                let ctx = format!("rows502-506 siphashx24 inlen={inlen} key{ki}");
                eq_i32(&ctx, r[0], r[1]);
                eq_bytes(&ctx, &o[0], &o[1]);
                assert_eq!(r[0], 0);
                assert_eq!(&o[0][16..], &[0xAAu8; 4], "{ctx}: overrun");
                // row 509: crypto_shorthash is the siphash24 alias
                for i in 0..2 {
                    eq_bytes(&format!("row509 lib{i} inlen={inlen} key{ki}"), &gh[i], &sh[i]);
                }
                eq_bytes(&format!("row509 crypto_shorthash inlen={inlen}"), &gh[0], &gh[1]);
                // row 507: the 128-bit output's first 8 bytes are NOT the
                // 64-bit siphash24 output (different finalization). We only
                // require C and Rust to agree; the assertion below records
                // that they really do differ in practice.
                if o[0][..8] == sh[0][..8] {
                    ever_equal_prefix = true;
                }
                assert_eq!(
                    o[0][..8] == sh[0][..8],
                    o[1][..8] == sh[1][..8],
                    "row507 inlen={inlen} key{ki}: C/Rust disagree on whether the \
                     siphashx24 prefix equals siphash24"
                );
            }
        }
        assert!(
            !ever_equal_prefix,
            "row507: siphashx24 prefix unexpectedly equalled siphash24"
        );

        // row 509: crypto_shorthash_keygen writes 16 bytes
        install_det_random();
        let kg = pair2!(Keygen, "crypto_shorthash_keygen");
        det_reseed(0x9999_8888_7777_6666);
        let mut k0 = vec![0xAAu8; 24];
        kg[0](k0.as_mut_ptr());
        det_reseed(0x9999_8888_7777_6666);
        let mut k1 = vec![0xAAu8; 24];
        kg[1](k1.as_mut_ptr());
        eq_bytes("row509 crypto_shorthash_keygen", &k0, &k1);
        assert_eq!(&k0[16..], &[0xAAu8; 8], "row509 keygen wrote past 16 bytes");
        assert_ne!(&k0[..16], &[0xAAu8; 16], "row509 keygen wrote nothing");
    }
}
