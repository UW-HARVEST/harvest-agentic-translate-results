//! Phase B — CONFIGS.md rows 21–40: the HASH / MAC / XOF surface.
//!
//! Every entry point is exercised through BOTH `.so`s (C ground-truth vs the
//! Rust translation, each loaded via `libloading`) and asserted byte-identical
//! with many randomized, seeded inputs. Multipart state machines are driven
//! through the library's own opaque state buffers, sized with the exported
//! `*_statebytes()` accessor (+64 slack) exactly as an external C consumer
//! would allocate them.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// C signatures (from c_src/libsodium/include/sodium/*.h).
//   * one-shot / update lengths are `unsigned long long`  -> u64
//   * generichash/xof out/key/salt lengths are `size_t`    -> usize
//   * turboshake domain is `unsigned char`                 -> u8
// ---------------------------------------------------------------------------

// crypto_hash_sha256/sha512/sha3256/sha3512/crypto_hash: (out, in, inlen:u64)
type HashOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
// *_init(state)
type HashInit = unsafe extern "C" fn(*mut u8) -> i32;
// *_update(state, in, inlen:u64)
type HashUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
// *_final(state, out)
type HashFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

type SizeFn = unsafe extern "C" fn() -> usize;

// crypto_generichash(out, outlen:usize, in, inlen:u64, key, keylen:usize)
type GhOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> i32;
// crypto_generichash_blake2b_salt_personal(out,outlen,in,inlen,key,keylen,salt,personal)
type GhSaltPersonal = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    u64,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> i32;
// crypto_generichash_init(state, key, keylen:usize, outlen:usize)
type GhInit = unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> i32;
// crypto_generichash_blake2b_init_salt_personal(state,key,keylen,outlen,salt,personal)
type GhInitSaltPersonal =
    unsafe extern "C" fn(*mut u8, *const u8, usize, usize, *const u8, *const u8) -> i32;
// crypto_generichash_update(state, in, inlen:u64)
type GhUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
// crypto_generichash_final(state, out, outlen:usize)
type GhFinal = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> i32;

// crypto_xof_shake128(out, outlen:usize, in, inlen:u64)
type XofOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> i32;
// crypto_xof_shake128_init(state)
type XofInit = unsafe extern "C" fn(*mut u8) -> i32;
// crypto_xof_*_init_with_domain(state, domain:u8)
type XofInitDomain = unsafe extern "C" fn(*mut u8, u8) -> i32;
// crypto_xof_*_update(state, in, inlen:u64)  (absorb)
type XofUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
// crypto_xof_*_squeeze(state, out, outlen:usize)
type XofSqueeze = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> i32;

// crypto_auth(out, in, inlen:u64, k)
type AuthOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
// crypto_auth_verify(h, in, inlen:u64, k)
type AuthVerify = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
// crypto_auth_hmacsha*_init(state, key, keylen:usize)
type HmacInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
// crypto_auth_hmacsha*_update(state, in, inlen:u64)
type HmacUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
// crypto_auth_hmacsha*_final(state, out)
type HmacFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

/// Fetch `*_statebytes()` from BOTH libs, assert agreement, and return an
/// over-allocated opaque state buffer per the CRITICAL RULES.
fn state_buf(d: &'static Duo, name: &str) -> (Vec<u8>, Vec<u8>) {
    let (sbc, sbr) = d.pair::<SizeFn>(&format!("{name}_statebytes"));
    let sc = unsafe { sbc() };
    let sr = unsafe { sbr() };
    eq_i32(&format!("{name}_statebytes"), sc as i32, sr as i32);
    (vec![0u8; sc + 64], vec![0u8; sc + 64])
}

/// Random multipart split of `inlen` into 1..=max_parts chunks (some may be 0).
fn splits(rng: &mut Rng, inlen: usize, max_parts: usize) -> Vec<usize> {
    let parts = 1 + rng.below(max_parts);
    let mut cuts: Vec<usize> = (0..parts.saturating_sub(1))
        .map(|_| rng.below(inlen + 1))
        .collect();
    cuts.sort_unstable();
    let mut out = Vec::with_capacity(parts);
    let mut prev = 0;
    for c in cuts {
        out.push(c - prev);
        prev = c;
    }
    out.push(inlen - prev);
    out
}

// ===========================================================================
// Rows 21–24: SHA-256 / SHA-512 / crypto_hash (generic sha512), one-shot +
// init/update/final multipart.
// ===========================================================================

/// Shared driver for a fixed-output classic hash (sha256/sha512/sha3*).
fn run_classic_hash(d: &'static Duo, name: &str, lens: &[usize], out_bytes: usize, seed: u64) {
    let (osc, osr) = d.pair::<HashOneShot>(name);
    let (bc, br) = d.pair::<SizeFn>(&format!("{name}_bytes"));
    eq_i32(
        &format!("{name}_bytes"),
        unsafe { bc() } as i32,
        unsafe { br() } as i32,
    );
    assert_eq!(
        unsafe { bc() },
        out_bytes,
        "{name}_bytes mismatch vs header"
    );

    let mut rng = Rng::new(seed);
    for &n in lens {
        for _ in 0..24 {
            let msg = rng.bytes(n);
            let mut oc = vec![0u8; out_bytes];
            let mut or = vec![0u8; out_bytes];
            let rc = unsafe { osc(oc.as_mut_ptr(), msg.as_ptr(), n as u64) };
            let rr = unsafe { osr(or.as_mut_ptr(), msg.as_ptr(), n as u64) };
            eq_i32(&format!("{name} oneshot ret n={n}"), rc, rr);
            eq_bytes(&format!("{name} oneshot n={n}"), &oc, &or);
        }
    }
}

/// Shared driver for init/update/final multipart of a classic hash.
fn run_classic_hash_multipart(
    d: &'static Duo,
    name: &str,
    lens: &[usize],
    out_bytes: usize,
    seed: u64,
) {
    let (initc, initr) = d.pair::<HashInit>(&format!("{name}_init"));
    let (updc, updr) = d.pair::<HashUpdate>(&format!("{name}_update"));
    let (finc, finr) = d.pair::<HashFinal>(&format!("{name}_final"));

    let mut rng = Rng::new(seed);
    for &n in lens {
        for _ in 0..24 {
            let msg = rng.bytes(n);
            let parts = splits(&mut rng, n, 10);
            let (mut sc, mut sr) = state_buf(d, name);
            let mut oc = vec![0u8; out_bytes];
            let mut or = vec![0u8; out_bytes];
            unsafe {
                eq_i32(
                    &format!("{name} init"),
                    initc(sc.as_mut_ptr()),
                    initr(sr.as_mut_ptr()),
                );
                let mut off = 0usize;
                for &p in &parts {
                    let ptr = msg[off..].as_ptr();
                    let uc = updc(sc.as_mut_ptr(), ptr, p as u64);
                    let ur = updr(sr.as_mut_ptr(), ptr, p as u64);
                    eq_i32(&format!("{name} update n={n} p={p}"), uc, ur);
                    off += p;
                }
                let fc = finc(sc.as_mut_ptr(), oc.as_mut_ptr());
                let fr = finr(sr.as_mut_ptr(), or.as_mut_ptr());
                eq_i32(&format!("{name} final n={n}"), fc, fr);
            }
            eq_bytes(&format!("{name} multipart n={n} parts={parts:?}"), &oc, &or);
        }
    }
}

#[test]
fn sha256_oneshot() {
    let d = duo();
    let lens = &[0, 1, 55, 56, 57, 63, 64, 65, 119, 120, 127, 128, 1000];
    run_classic_hash(d, "crypto_hash_sha256", lens, 32, 0x5A25_60001);
}

#[test]
fn sha256_multipart() {
    let d = duo();
    let lens = &[0, 1, 55, 56, 57, 63, 64, 65, 119, 120, 127, 128, 200, 1000];
    run_classic_hash_multipart(d, "crypto_hash_sha256", lens, 32, 0x5A25_60002);
}

#[test]
fn sha512_and_generic_oneshot() {
    let d = duo();
    let lens = &[0, 1, 111, 112, 113, 127, 128, 129, 255, 256, 1000];
    run_classic_hash(d, "crypto_hash_sha512", lens, 64, 0x5A51_20001);
    // crypto_hash (generic) == sha512, same signature and output.
    run_classic_hash(d, "crypto_hash", lens, 64, 0x5A51_200A1);
}

#[test]
fn sha512_multipart() {
    let d = duo();
    let lens = &[0, 1, 111, 112, 113, 127, 128, 129, 255, 256, 300, 1000];
    run_classic_hash_multipart(d, "crypto_hash_sha512", lens, 64, 0x5A51_20002);
}

// ===========================================================================
// Rows 25–26: SHA3-256 / SHA3-512 one-shot + multipart across rate boundary.
//   SHA3-256 rate = 136, SHA3-512 rate = 72.
// ===========================================================================

#[test]
fn sha3256_oneshot() {
    let d = duo();
    let lens = &[0, 1, 135, 136, 137, 271, 272, 273, 1000];
    run_classic_hash(d, "crypto_hash_sha3256", lens, 32, 0x53A3_2560);
}

#[test]
fn sha3256_multipart() {
    let d = duo();
    let lens = &[0, 1, 135, 136, 137, 271, 272, 273, 1000];
    run_classic_hash_multipart(d, "crypto_hash_sha3256", lens, 32, 0x53A3_2561);
}

#[test]
fn sha3512_oneshot() {
    let d = duo();
    let lens = &[0, 1, 71, 72, 73, 143, 144, 145, 1000];
    run_classic_hash(d, "crypto_hash_sha3512", lens, 64, 0x53A3_5120);
}

#[test]
fn sha3512_multipart() {
    let d = duo();
    let lens = &[0, 1, 71, 72, 73, 143, 144, 145, 1000];
    run_classic_hash_multipart(d, "crypto_hash_sha3512", lens, 64, 0x53A3_5121);
}

// ===========================================================================
// Rows 27–28: SHAKE128 / SHAKE256 one-shot + init/absorb(update)/squeeze
// incremental, splitting BOTH the absorb side AND the squeeze side across the
// rate. SHAKE128 rate = 168, SHAKE256 rate = 136.
// ===========================================================================

const XOF_OUTLENS: &[usize] = &[0, 1, 31, 32, 33, 135, 136, 168, 169, 336, 1000];
const XOF_INLENS: &[usize] = &[0, 1, 15, 16, 17, 135, 136, 137, 167, 168, 169, 1000];

/// One-shot XOF over the out/in sweep.
fn run_xof_oneshot(d: &'static Duo, name: &str, seed: u64) {
    let (osc, osr) = d.pair::<XofOneShot>(name);
    let mut rng = Rng::new(seed);
    for &outlen in XOF_OUTLENS {
        for &inlen in XOF_INLENS {
            for _ in 0..6 {
                let msg = rng.bytes(inlen);
                let mut oc = vec![0u8; outlen];
                let mut or = vec![0u8; outlen];
                let rc = unsafe { osc(oc.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64) };
                let rr = unsafe { osr(or.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64) };
                eq_i32(
                    &format!("{name} oneshot ret out={outlen} in={inlen}"),
                    rc,
                    rr,
                );
                eq_bytes(&format!("{name} oneshot out={outlen} in={inlen}"), &oc, &or);
            }
        }
    }
}

/// Incremental XOF: init, absorb in random splits, squeeze in random splits.
/// `init_domain`: Some(domain) -> use *_init_with_domain, None -> plain *_init.
fn run_xof_incremental(d: &'static Duo, name: &str, init_domain: Option<u8>, seed: u64) {
    let (updc, updr) = d.pair::<XofUpdate>(&format!("{name}_update"));
    let (sqc, sqr) = d.pair::<XofSqueeze>(&format!("{name}_squeeze"));

    let mut rng = Rng::new(seed);
    for &inlen in XOF_INLENS {
        for &outlen in XOF_OUTLENS {
            for _ in 0..4 {
                let msg = rng.bytes(inlen);
                let (mut sc, mut sr) = state_buf(d, name);
                // init (plain or with domain)
                unsafe {
                    match init_domain {
                        None => {
                            let (ic, ir) = d.pair::<XofInit>(&format!("{name}_init"));
                            eq_i32(
                                &format!("{name} init"),
                                ic(sc.as_mut_ptr()),
                                ir(sr.as_mut_ptr()),
                            );
                        }
                        Some(dom) => {
                            let (ic, ir) =
                                d.pair::<XofInitDomain>(&format!("{name}_init_with_domain"));
                            eq_i32(
                                &format!("{name} init_with_domain 0x{dom:02x}"),
                                ic(sc.as_mut_ptr(), dom),
                                ir(sr.as_mut_ptr(), dom),
                            );
                        }
                    }
                }
                // absorb in random splits crossing the rate
                let aparts = splits(&mut rng, inlen, 6);
                let mut off = 0usize;
                for &p in &aparts {
                    let ptr = msg[off..].as_ptr();
                    let uc = unsafe { updc(sc.as_mut_ptr(), ptr, p as u64) };
                    let ur = unsafe { updr(sr.as_mut_ptr(), ptr, p as u64) };
                    eq_i32(&format!("{name} absorb in={inlen} p={p}"), uc, ur);
                    off += p;
                }
                // squeeze in random splits crossing the rate
                let sparts = splits(&mut rng, outlen, 6);
                let mut oc = vec![0u8; outlen];
                let mut or = vec![0u8; outlen];
                let mut soff = 0usize;
                for &p in &sparts {
                    let cptr = oc[soff..].as_mut_ptr();
                    let rptr = or[soff..].as_mut_ptr();
                    let qc = unsafe { sqc(sc.as_mut_ptr(), cptr, p) };
                    let qr = unsafe { sqr(sr.as_mut_ptr(), rptr, p) };
                    eq_i32(&format!("{name} squeeze out={outlen} p={p}"), qc, qr);
                    soff += p;
                }
                eq_bytes(
                    &format!(
                        "{name} incremental in={inlen} out={outlen} a={aparts:?} s={sparts:?}"
                    ),
                    &oc,
                    &or,
                );
            }
        }
    }
}

#[test]
fn shake128_oneshot() {
    run_xof_oneshot(duo(), "crypto_xof_shake128", 0x5148_1280);
}

#[test]
fn shake256_oneshot() {
    run_xof_oneshot(duo(), "crypto_xof_shake256", 0x5148_2560);
}

#[test]
fn shake128_incremental() {
    run_xof_incremental(duo(), "crypto_xof_shake128", None, 0x5148_1281);
}

#[test]
fn shake256_incremental() {
    run_xof_incremental(duo(), "crypto_xof_shake256", None, 0x5148_2561);
}

// ===========================================================================
// Rows 29–31: TurboSHAKE128 / TurboSHAKE256 one-shot + _init_with_domain with
// domain in {0x01,0x02,0x1f,0x7f}, absorb/squeeze splits.
// ===========================================================================

const TURBO_DOMAINS: &[u8] = &[0x01, 0x02, 0x1f, 0x7f];

#[test]
fn turboshake128_oneshot() {
    run_xof_oneshot(duo(), "crypto_xof_turboshake128", 0x7B00_1280);
}

#[test]
fn turboshake256_oneshot() {
    run_xof_oneshot(duo(), "crypto_xof_turboshake256", 0x7B00_2560);
}

#[test]
fn turboshake128_incremental_default() {
    // plain _init uses the standard domain (0x1f).
    run_xof_incremental(duo(), "crypto_xof_turboshake128", None, 0x7B00_1281);
}

#[test]
fn turboshake256_incremental_default() {
    run_xof_incremental(duo(), "crypto_xof_turboshake256", None, 0x7B00_2561);
}

#[test]
fn turboshake128_init_with_domain() {
    let d = duo();
    for (i, &dom) in TURBO_DOMAINS.iter().enumerate() {
        run_xof_incremental(
            d,
            "crypto_xof_turboshake128",
            Some(dom),
            0x7B00_1290 + i as u64,
        );
    }
}

#[test]
fn turboshake256_init_with_domain() {
    let d = duo();
    for (i, &dom) in TURBO_DOMAINS.iter().enumerate() {
        run_xof_incremental(
            d,
            "crypto_xof_turboshake256",
            Some(dom),
            0x7B00_2590 + i as u64,
        );
    }
}

// ===========================================================================
// Rows 32–36: BLAKE2b generichash — one-shot, salt_personal, init/update/final,
// init_salt_personal, and the generic crypto_generichash* wrappers.
// Sweep outlen x keylen x inlen; salt/personal in {all-zero, random, NULL}.
// ===========================================================================

const GH_OUTLENS: &[usize] = &[1, 16, 31, 32, 33, 63, 64];
const GH_KEYLENS: &[usize] = &[0, 1, 16, 32, 63, 64];
const GH_INLENS: &[usize] = &[0, 1, 127, 128, 129, 1000];

/// salt/personal source selector.
#[derive(Clone, Copy)]
enum SP {
    Zero,
    Rand,
    Null,
}

/// Build salt/personal buffers + pointers for the given mode. Buffers are
/// returned so the caller keeps them alive while the pointers are in use.
fn sp_ptrs(rng: &mut Rng, mode: SP) -> (Vec<u8>, Vec<u8>, *const u8, *const u8) {
    match mode {
        SP::Zero => {
            let s = vec![0u8; 16];
            let p = vec![0u8; 16];
            let sp = s.as_ptr();
            let pp = p.as_ptr();
            (s, p, sp, pp)
        }
        SP::Rand => {
            let s = rng.bytes(16);
            let p = rng.bytes(16);
            let sp = s.as_ptr();
            let pp = p.as_ptr();
            (s, p, sp, pp)
        }
        SP::Null => (Vec::new(), Vec::new(), std::ptr::null(), std::ptr::null()),
    }
}

/// One-shot generichash driver. `salt_personal` = true routes through the
/// `_salt_personal` entry point (row 33); false uses the plain 6-arg form
/// (rows 32/36).
fn run_gh_oneshot(d: &'static Duo, name: &str, salt_personal: bool, seed: u64) {
    let (osc, osr) = d.pair::<GhOneShot>(name);
    let sp_pair = if salt_personal {
        Some(d.pair::<GhSaltPersonal>(&format!("{name}_salt_personal")))
    } else {
        None
    };

    let mut rng = Rng::new(seed);
    for &outlen in GH_OUTLENS {
        for &keylen in GH_KEYLENS {
            for &inlen in GH_INLENS {
                let sp_modes: &[SP] = if salt_personal {
                    &[SP::Zero, SP::Rand, SP::Null]
                } else {
                    &[SP::Null] // plain form takes no salt/personal
                };
                for &spm in sp_modes {
                    for _ in 0..4 {
                        let msg = rng.bytes(inlen);
                        let key = rng.bytes(keylen);
                        let kptr = if keylen == 0 {
                            std::ptr::null()
                        } else {
                            key.as_ptr()
                        };
                        let mut oc = vec![0u8; outlen];
                        let mut or = vec![0u8; outlen];

                        let (rc, rr) = match &sp_pair {
                            None => unsafe {
                                (
                                    osc(
                                        oc.as_mut_ptr(),
                                        outlen,
                                        msg.as_ptr(),
                                        inlen as u64,
                                        kptr,
                                        keylen,
                                    ),
                                    osr(
                                        or.as_mut_ptr(),
                                        outlen,
                                        msg.as_ptr(),
                                        inlen as u64,
                                        kptr,
                                        keylen,
                                    ),
                                )
                            },
                            Some((spc, spr)) => {
                                let (salt, personal, sptr, pptr) = sp_ptrs(&mut rng, spm);
                                let _keep = (salt, personal);
                                unsafe {
                                    (
                                        spc(
                                            oc.as_mut_ptr(),
                                            outlen,
                                            msg.as_ptr(),
                                            inlen as u64,
                                            kptr,
                                            keylen,
                                            sptr,
                                            pptr,
                                        ),
                                        spr(
                                            or.as_mut_ptr(),
                                            outlen,
                                            msg.as_ptr(),
                                            inlen as u64,
                                            kptr,
                                            keylen,
                                            sptr,
                                            pptr,
                                        ),
                                    )
                                }
                            }
                        };
                        eq_i32(
                            &format!("{name} oneshot ret out={outlen} key={keylen} in={inlen}"),
                            rc,
                            rr,
                        );
                        eq_bytes(
                            &format!("{name} oneshot out={outlen} key={keylen} in={inlen}"),
                            &oc,
                            &or,
                        );
                    }
                }
            }
        }
    }
}

/// Multipart generichash driver. `init_salt_personal` routes init through the
/// `_init_salt_personal` entry point.
fn run_gh_multipart(d: &'static Duo, name: &str, init_salt_personal: bool, seed: u64) {
    let (updc, updr) = d.pair::<GhUpdate>(&format!("{name}_update"));
    let (finc, finr) = d.pair::<GhFinal>(&format!("{name}_final"));
    let plain_init = if init_salt_personal {
        None
    } else {
        Some(d.pair::<GhInit>(&format!("{name}_init")))
    };
    let sp_init = if init_salt_personal {
        Some(d.pair::<GhInitSaltPersonal>(&format!("{name}_init_salt_personal")))
    } else {
        None
    };

    let mut rng = Rng::new(seed);
    for &outlen in GH_OUTLENS {
        for &keylen in GH_KEYLENS {
            for &inlen in GH_INLENS {
                let sp_modes: &[SP] = if init_salt_personal {
                    &[SP::Zero, SP::Rand, SP::Null]
                } else {
                    &[SP::Null]
                };
                for &spm in sp_modes {
                    for _ in 0..3 {
                        let msg = rng.bytes(inlen);
                        let key = rng.bytes(keylen);
                        let kptr = if keylen == 0 {
                            std::ptr::null()
                        } else {
                            key.as_ptr()
                        };
                        let (mut sc, mut sr) = state_buf(d, name);
                        unsafe {
                            match (&plain_init, &sp_init) {
                                (Some((ic, ir)), _) => {
                                    eq_i32(
                                        &format!("{name} init out={outlen} key={keylen}"),
                                        ic(sc.as_mut_ptr(), kptr, keylen, outlen),
                                        ir(sr.as_mut_ptr(), kptr, keylen, outlen),
                                    );
                                }
                                (None, Some((ic, ir))) => {
                                    let (salt, personal, sptr, pptr) = sp_ptrs(&mut rng, spm);
                                    let _keep = (salt, personal);
                                    eq_i32(
                                        &format!("{name} init_sp out={outlen} key={keylen}"),
                                        ic(sc.as_mut_ptr(), kptr, keylen, outlen, sptr, pptr),
                                        ir(sr.as_mut_ptr(), kptr, keylen, outlen, sptr, pptr),
                                    );
                                }
                                (None, None) => unreachable!(),
                            }
                        }
                        // update in random splits crossing the 128-byte buffer
                        let parts = splits(&mut rng, inlen, 8);
                        let mut off = 0usize;
                        for &p in &parts {
                            let ptr = msg[off..].as_ptr();
                            let uc = unsafe { updc(sc.as_mut_ptr(), ptr, p as u64) };
                            let ur = unsafe { updr(sr.as_mut_ptr(), ptr, p as u64) };
                            eq_i32(&format!("{name} update in={inlen} p={p}"), uc, ur);
                            off += p;
                        }
                        let mut oc = vec![0u8; outlen];
                        let mut or = vec![0u8; outlen];
                        let fc = unsafe { finc(sc.as_mut_ptr(), oc.as_mut_ptr(), outlen) };
                        let fr = unsafe { finr(sr.as_mut_ptr(), or.as_mut_ptr(), outlen) };
                        eq_i32(&format!("{name} final out={outlen}"), fc, fr);
                        eq_bytes(
                            &format!("{name} multipart out={outlen} key={keylen} in={inlen} parts={parts:?}"),
                            &oc,
                            &or,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn generichash_blake2b_oneshot() {
    run_gh_oneshot(duo(), "crypto_generichash_blake2b", false, 0xB2B_00001);
}

#[test]
fn generichash_blake2b_salt_personal_oneshot() {
    run_gh_oneshot(duo(), "crypto_generichash_blake2b", true, 0xB2B_00002);
}

#[test]
fn generichash_blake2b_multipart() {
    run_gh_multipart(duo(), "crypto_generichash_blake2b", false, 0xB2B_00003);
}

#[test]
fn generichash_blake2b_init_salt_personal_multipart() {
    run_gh_multipart(duo(), "crypto_generichash_blake2b", true, 0xB2B_00004);
}

#[test]
fn generichash_generic_oneshot() {
    // Row 36: generic wrapper == blake2b. No _salt_personal on the generic name.
    run_gh_oneshot(duo(), "crypto_generichash", false, 0xB2B_000A1);
}

#[test]
fn generichash_generic_multipart() {
    run_gh_multipart(duo(), "crypto_generichash", false, 0xB2B_000A2);
}

// ===========================================================================
// Rows 37–40: HMAC-SHA256 / SHA512 / SHA512256 one-shot + init/update/final,
// keylen sweeps crossing the >blocklen re-hash branch. Plus generic
// crypto_auth / crypto_auth_verify (== hmacsha512256, fixed 32-byte key).
// Also every *_verify: correct MAC accepted, and each single byte flipped
// rejected, with return codes compared.
// ===========================================================================

const HMAC_INLENS: &[usize] = &[0, 1, 15, 16, 17, 63, 64, 65, 127, 128, 129, 1000];

/// Verify correct MAC (accepted, ret 0 in both) and every single byte flipped
/// (rejected, ret -1 in both), comparing return codes.
fn verify_sweep(
    what: &str,
    vc: &libloading::Symbol<AuthVerify>,
    vr: &libloading::Symbol<AuthVerify>,
    mac: &[u8],
    msg: &[u8],
    inlen: usize,
    key: &[u8],
) {
    // correct MAC
    let ac = unsafe { vc(mac.as_ptr(), msg.as_ptr(), inlen as u64, key.as_ptr()) };
    let ar = unsafe { vr(mac.as_ptr(), msg.as_ptr(), inlen as u64, key.as_ptr()) };
    eq_i32(&format!("{what} correct"), ac, ar);
    assert_eq!(ac, 0, "{what}: correct MAC should verify (ret 0), got {ac}");
    // every byte flipped -> reject in both
    for i in 0..mac.len() {
        let mut bad = mac.to_vec();
        bad[i] ^= 0xff;
        let bc = unsafe { vc(bad.as_ptr(), msg.as_ptr(), inlen as u64, key.as_ptr()) };
        let br = unsafe { vr(bad.as_ptr(), msg.as_ptr(), inlen as u64, key.as_ptr()) };
        eq_i32(&format!("{what} flip byte {i}"), bc, br);
        assert_eq!(
            bc, -1,
            "{what}: flipped MAC byte {i} should reject (ret -1), got {bc}"
        );
    }
}

/// HMAC driver: one-shot (fixed keybytes key) + multipart with keylen sweep
/// that crosses the >blocklen re-hash branch inside `_init`.
fn run_hmac(d: &'static Duo, name: &str, keylens: &[usize], out_bytes: usize, seed: u64) {
    let (osc, osr) = d.pair::<AuthOneShot>(name);
    let (vc, vr) = d.pair::<AuthVerify>(&format!("{name}_verify"));
    let (initc, initr) = d.pair::<HmacInit>(&format!("{name}_init"));
    let (updc, updr) = d.pair::<HmacUpdate>(&format!("{name}_update"));
    let (finc, finr) = d.pair::<HmacFinal>(&format!("{name}_final"));
    let (kbc, _) = d.pair::<SizeFn>(&format!("{name}_keybytes"));
    let keybytes = unsafe { kbc() };

    let mut rng = Rng::new(seed);
    for &keylen in keylens {
        for &inlen in HMAC_INLENS {
            for _ in 0..8 {
                let msg = rng.bytes(inlen);

                // --- one-shot: the C one-shot takes a FIXED keybytes-long key ---
                let fixed_key = rng.bytes(keybytes);
                let mut oc = vec![0u8; out_bytes];
                let mut or = vec![0u8; out_bytes];
                let rc = unsafe {
                    osc(
                        oc.as_mut_ptr(),
                        msg.as_ptr(),
                        inlen as u64,
                        fixed_key.as_ptr(),
                    )
                };
                let rr = unsafe {
                    osr(
                        or.as_mut_ptr(),
                        msg.as_ptr(),
                        inlen as u64,
                        fixed_key.as_ptr(),
                    )
                };
                eq_i32(&format!("{name} oneshot ret in={inlen}"), rc, rr);
                eq_bytes(&format!("{name} oneshot in={inlen}"), &oc, &or);
                verify_sweep(
                    &format!("{name} oneshot verify"),
                    &vc,
                    &vr,
                    &oc,
                    &msg,
                    inlen,
                    &fixed_key,
                );

                // --- multipart with keylen sweep (crosses >blocklen re-hash) ---
                let key = rng.bytes(keylen);
                let kptr = if keylen == 0 {
                    std::ptr::null()
                } else {
                    key.as_ptr()
                };
                let parts = splits(&mut rng, inlen, 8);
                let (mut sc, mut sr) = state_buf(d, name);
                let mut mc = vec![0u8; out_bytes];
                let mut mr = vec![0u8; out_bytes];
                unsafe {
                    eq_i32(
                        &format!("{name} init keylen={keylen}"),
                        initc(sc.as_mut_ptr(), kptr, keylen),
                        initr(sr.as_mut_ptr(), kptr, keylen),
                    );
                    let mut off = 0usize;
                    for &p in &parts {
                        let ptr = msg[off..].as_ptr();
                        let uc = updc(sc.as_mut_ptr(), ptr, p as u64);
                        let ur = updr(sr.as_mut_ptr(), ptr, p as u64);
                        eq_i32(
                            &format!("{name} update keylen={keylen} in={inlen} p={p}"),
                            uc,
                            ur,
                        );
                        off += p;
                    }
                    eq_i32(
                        &format!("{name} final keylen={keylen} in={inlen}"),
                        finc(sc.as_mut_ptr(), mc.as_mut_ptr()),
                        finr(sr.as_mut_ptr(), mr.as_mut_ptr()),
                    );
                }
                eq_bytes(
                    &format!("{name} multipart keylen={keylen} in={inlen} parts={parts:?}"),
                    &mc,
                    &mr,
                );
            }
        }
    }
}

#[test]
fn hmacsha256() {
    let d = duo();
    let keylens = &[0, 1, 32, 63, 64, 65, 128, 200];
    run_hmac(d, "crypto_auth_hmacsha256", keylens, 32, 0x4DAC_256);
}

#[test]
fn hmacsha512() {
    let d = duo();
    let keylens = &[0, 1, 64, 127, 128, 129, 256];
    run_hmac(d, "crypto_auth_hmacsha512", keylens, 64, 0x4DAC_512);
}

#[test]
fn hmacsha512256() {
    let d = duo();
    let keylens = &[0, 1, 64, 127, 128, 129, 256];
    run_hmac(d, "crypto_auth_hmacsha512256", keylens, 32, 0x4DAC_5122);
}

#[test]
fn auth_generic() {
    // Row 40: crypto_auth / crypto_auth_verify == hmacsha512256, fixed 32-byte key.
    let d = duo();
    let (osc, osr) = d.pair::<AuthOneShot>("crypto_auth");
    let (vc, vr) = d.pair::<AuthVerify>("crypto_auth_verify");
    let (kbc, _) = d.pair::<SizeFn>("crypto_auth_keybytes");
    let (bc, _) = d.pair::<SizeFn>("crypto_auth_bytes");
    let keybytes = unsafe { kbc() };
    let macbytes = unsafe { bc() };
    assert_eq!(keybytes, 32);
    assert_eq!(macbytes, 32);

    let mut rng = Rng::new(0x4DAC_9E1);
    for &inlen in HMAC_INLENS {
        for _ in 0..16 {
            let msg = rng.bytes(inlen);
            let key = rng.bytes(keybytes);
            let mut oc = vec![0u8; macbytes];
            let mut or = vec![0u8; macbytes];
            let rc = unsafe { osc(oc.as_mut_ptr(), msg.as_ptr(), inlen as u64, key.as_ptr()) };
            let rr = unsafe { osr(or.as_mut_ptr(), msg.as_ptr(), inlen as u64, key.as_ptr()) };
            eq_i32(&format!("crypto_auth ret in={inlen}"), rc, rr);
            eq_bytes(&format!("crypto_auth in={inlen}"), &oc, &or);
            verify_sweep("crypto_auth verify", &vc, &vr, &oc, &msg, inlen, &key);
        }
    }
}
