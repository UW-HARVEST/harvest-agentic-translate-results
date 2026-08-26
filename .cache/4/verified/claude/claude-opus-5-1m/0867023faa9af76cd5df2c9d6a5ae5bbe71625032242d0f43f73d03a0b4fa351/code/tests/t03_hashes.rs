//! Phase B — hashes, MACs, XOFs and KDFs.
//!
//! Covers the one-shot AND the low-level streaming (`_init`/`_update`/`_final`,
//! `_absorb`/`_squeeze`) entry points, with randomised update chunking so that
//! partial-block carry-over across calls is exercised.

mod common;
use common::*;

type HashOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type StInit = unsafe extern "C" fn(*mut u8) -> i32;
type StUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type StFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type Keygen = unsafe extern "C" fn(*mut u8);

/// Lengths straddling the SHA-2 length-padding boundaries (55/56, 111/112),
/// the 64/128-byte block sizes and the Keccak rates (72/136/168).
const LENS: &[usize] = &[
    0, 1, 2, 31, 32, 33, 54, 55, 56, 57, 63, 64, 65, 71, 72, 73, 100, 110, 111, 112, 113, 127, 128,
    129, 135, 136, 137, 143, 144, 145, 167, 168, 169, 200, 255, 256, 257, 335, 336, 337, 511, 512,
    513, 1000, 2000,
];

// ---------------------------------------------------------------------------
// SHA-256 / SHA-512 / SHA3-256 / SHA3-512 / generic crypto_hash
// ---------------------------------------------------------------------------

/// (prefix, digest len, statebytes symbol)
const SHA_FAMILY: &[(&str, usize)] = &[
    ("crypto_hash_sha256", 32),
    ("crypto_hash_sha512", 64),
    ("crypto_hash_sha3256", 32),
    ("crypto_hash_sha3512", 64),
];

#[test]
fn sha_one_shot() {
    setup();
    let mut rng = Rng::new(0x7001);
    for &(prefix, dl) in SHA_FAMILY {
        let (c, r) = pair::<HashOneShot>(prefix);
        for &len in LENS {
            for kind in 0..3 {
                let inp = match kind {
                    0 => rng.bytes(len),
                    1 => vec![0u8; len],
                    _ => vec![0xffu8; len],
                };
                let mut a = canary(dl);
                let mut b = canary(dl);
                let (ra, rb) = unsafe {
                    (
                        c(a.as_mut_ptr(), inp.as_ptr(), len as u64),
                        r(b.as_mut_ptr(), inp.as_ptr(), len as u64),
                    )
                };
                eq_i32(&format!("{prefix} rc"), ra, rb);
                eq_bytes(&format!("{prefix}(len={len}, kind={kind})"), &a, &b);
            }
        }
    }
    // generic crypto_hash == crypto_hash_sha512
    let (c, r) = pair::<HashOneShot>("crypto_hash");
    for &len in LENS {
        let inp = rng.bytes(len);
        let mut a = canary(64);
        let mut b = canary(64);
        let (ra, rb) = unsafe {
            (
                c(a.as_mut_ptr(), inp.as_ptr(), len as u64),
                r(b.as_mut_ptr(), inp.as_ptr(), len as u64),
            )
        };
        eq_i32("crypto_hash rc", ra, rb);
        eq_bytes(&format!("crypto_hash(len={len})"), &a, &b);
    }
}

#[test]
fn sha_streaming_with_random_chunking() {
    setup();
    let mut rng = Rng::new(0x7002);
    for &(prefix, dl) in SHA_FAMILY {
        let (ci, ri) = pair::<StInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cf, rf) = pair::<StFinal>(&format!("{prefix}_final"));
        let sb = format!("{prefix}_statebytes");
        for &len in LENS {
            for style in 0..4u32 {
                if style == 1 && len > 300 {
                    continue; // 1-byte-at-a-time on huge inputs is slow
                }
                let inp = rng.bytes(len);
                let parts = chunks(&mut rng, len, style);
                let mut out = [canary(dl), canary(dl)];
                for (which, (init, upd, fin)) in
                    [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate()
                {
                    let mut st = State::for_sym(&sb);
                    unsafe {
                        let rc = init(st.as_mut_ptr());
                        assert_eq!(rc, 0, "{prefix}_init rc");
                        let mut off = 0usize;
                        for &n in &parts {
                            let rc = upd(st.as_mut_ptr(), inp[off..].as_ptr(), n as u64);
                            assert_eq!(rc, 0, "{prefix}_update rc");
                            off += n;
                        }
                        assert_eq!(off, len);
                        let rc = fin(st.as_mut_ptr(), out[which].as_mut_ptr());
                        assert_eq!(rc, 0, "{prefix}_final rc");
                    }
                }
                let (a, b) = (out[0].clone(), out[1].clone());
                eq_bytes(
                    &format!("{prefix} streaming(len={len}, style={style}, parts={})", parts.len()),
                    &a,
                    &b,
                );
                // streaming must equal one-shot
                let (c1, _) = pair::<HashOneShot>(prefix);
                let mut os = canary(dl);
                unsafe { c1(os.as_mut_ptr(), inp.as_ptr(), len as u64) };
                eq_bytes(&format!("{prefix} streaming==one-shot(len={len})"), &os, &a);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// BLAKE2b / generichash — one-shot, streaming, salt/personal
// ---------------------------------------------------------------------------

type GhOneShot =
    unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> i32;
type GhInit = unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> i32;
type GhFinal = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> i32;
type GhSaltPers = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    u64,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> i32;
type GhInitSaltPers =
    unsafe extern "C" fn(*mut u8, *const u8, usize, usize, *const u8, *const u8) -> i32;

const GH_OUTLENS: &[usize] = &[1, 2, 15, 16, 17, 31, 32, 33, 47, 48, 63, 64];
const GH_KEYLENS: &[usize] = &[0, 1, 15, 16, 17, 31, 32, 33, 63, 64];
const GH_INLENS: &[usize] = &[0, 1, 2, 63, 64, 65, 127, 128, 129, 191, 192, 255, 256, 257, 1000];

#[test]
fn generichash_one_shot() {
    setup();
    let mut rng = Rng::new(0x7100);
    for prefix in ["crypto_generichash", "crypto_generichash_blake2b"] {
        let (c, r) = pair::<GhOneShot>(prefix);
        for &outlen in GH_OUTLENS {
            for &keylen in GH_KEYLENS {
                for &inlen in GH_INLENS {
                    let key = rng.bytes(keylen);
                    let inp = rng.bytes(inlen);
                    // keylen == 0 must be tested both with key=NULL and key!=NULL
                    for key_null in [false, true] {
                        if key_null && keylen != 0 {
                            continue;
                        }
                        let kp = if key_null {
                            std::ptr::null()
                        } else {
                            key.as_ptr()
                        };
                        let mut a = canary(outlen);
                        let mut b = canary(outlen);
                        let (ra, rb) = unsafe {
                            (
                                c(a.as_mut_ptr(), outlen, inp.as_ptr(), inlen as u64, kp, keylen),
                                r(b.as_mut_ptr(), outlen, inp.as_ptr(), inlen as u64, kp, keylen),
                            )
                        };
                        eq_i32(&format!("{prefix} rc (out={outlen} key={keylen})"), ra, rb);
                        eq_bytes(
                            &format!(
                                "{prefix}(out={outlen}, key={keylen}, null={key_null}, in={inlen})"
                            ),
                            &a,
                            &b,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn generichash_streaming() {
    setup();
    let mut rng = Rng::new(0x7101);
    for prefix in ["crypto_generichash", "crypto_generichash_blake2b"] {
        let (ci, ri) = pair::<GhInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cf, rf) = pair::<GhFinal>(&format!("{prefix}_final"));
        let sb = format!("{prefix}_statebytes");
        for &outlen in &[16usize, 32, 64, 1, 17, 63] {
            for &keylen in &[0usize, 16, 32, 64] {
                for &inlen in &[0usize, 1, 127, 128, 129, 255, 256, 1000] {
                    for style in 0..4u32 {
                        if style == 1 && inlen > 300 {
                            continue;
                        }
                        let key = rng.bytes(keylen);
                        let inp = rng.bytes(inlen);
                        let kp = if keylen == 0 {
                            std::ptr::null()
                        } else {
                            key.as_ptr()
                        };
                        let parts = chunks(&mut rng, inlen, style);
                        let mut out = [canary(outlen), canary(outlen)];
                        for (which, (init, upd, fin)) in
                            [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate()
                        {
                            let mut st = State::for_sym(&sb);
                            unsafe {
                                assert_eq!(init(st.as_mut_ptr(), kp, keylen, outlen), 0);
                                let mut off = 0;
                                for &n in &parts {
                                    assert_eq!(
                                        upd(st.as_mut_ptr(), inp[off..].as_ptr(), n as u64),
                                        0
                                    );
                                    off += n;
                                }
                                assert_eq!(fin(st.as_mut_ptr(), out[which].as_mut_ptr(), outlen), 0);
                            }
                        }
                        let (a, b) = (out[0].clone(), out[1].clone());
                        eq_bytes(
                            &format!(
                                "{prefix} stream(out={outlen}, key={keylen}, in={inlen}, style={style})"
                            ),
                            &a,
                            &b,
                        );
                        // must equal the one-shot form
                        let (c1, _) = pair::<GhOneShot>(prefix);
                        let mut os = canary(outlen);
                        unsafe {
                            c1(os.as_mut_ptr(), outlen, inp.as_ptr(), inlen as u64, kp, keylen)
                        };
                        eq_bytes(&format!("{prefix} stream==one-shot"), &os, &a);
                    }
                }
            }
        }
    }
}

#[test]
fn generichash_blake2b_salt_personal() {
    setup();
    let mut rng = Rng::new(0x7102);
    let (c, r) = pair::<GhSaltPers>("crypto_generichash_blake2b_salt_personal");
    for &outlen in &[16usize, 32, 64, 1, 17] {
        for &keylen in &[0usize, 16, 32, 64] {
            for &inlen in &[0usize, 1, 127, 128, 129, 500] {
                for combo in 0..4u32 {
                    let key = rng.bytes(keylen);
                    let salt = rng.bytes(16);
                    let pers = rng.bytes(16);
                    let kp = if keylen == 0 {
                        std::ptr::null()
                    } else {
                        key.as_ptr()
                    };
                    let sp = if combo & 1 == 0 {
                        std::ptr::null()
                    } else {
                        salt.as_ptr()
                    };
                    let pp = if combo & 2 == 0 {
                        std::ptr::null()
                    } else {
                        pers.as_ptr()
                    };
                    let inp = rng.bytes(inlen);
                    let mut a = canary(outlen);
                    let mut b = canary(outlen);
                    let (ra, rb) = unsafe {
                        (
                            c(
                                a.as_mut_ptr(),
                                outlen,
                                inp.as_ptr(),
                                inlen as u64,
                                kp,
                                keylen,
                                sp,
                                pp,
                            ),
                            r(
                                b.as_mut_ptr(),
                                outlen,
                                inp.as_ptr(),
                                inlen as u64,
                                kp,
                                keylen,
                                sp,
                                pp,
                            ),
                        )
                    };
                    eq_i32("blake2b_salt_personal rc", ra, rb);
                    eq_bytes(
                        &format!("blake2b_salt_personal(out={outlen},key={keylen},in={inlen},combo={combo})"),
                        &a,
                        &b,
                    );
                }
            }
        }
    }
}

#[test]
fn generichash_blake2b_init_salt_personal_streaming() {
    setup();
    let mut rng = Rng::new(0x7103);
    let (ci, ri) = pair::<GhInitSaltPers>("crypto_generichash_blake2b_init_salt_personal");
    let (cu, ru) = pair::<StUpdate>("crypto_generichash_blake2b_update");
    let (cf, rf) = pair::<GhFinal>("crypto_generichash_blake2b_final");
    for &outlen in &[16usize, 32, 64] {
        for &keylen in &[0usize, 16, 64] {
            for &inlen in &[0usize, 1, 128, 129, 500] {
                for combo in 0..4u32 {
                    let key = rng.bytes(keylen);
                    let salt = rng.bytes(16);
                    let pers = rng.bytes(16);
                    let inp = rng.bytes(inlen);
                    let kp = if keylen == 0 {
                        std::ptr::null()
                    } else {
                        key.as_ptr()
                    };
                    let sp = if combo & 1 == 0 {
                        std::ptr::null()
                    } else {
                        salt.as_ptr()
                    };
                    let pp = if combo & 2 == 0 {
                        std::ptr::null()
                    } else {
                        pers.as_ptr()
                    };
                    let parts = chunks(&mut rng, inlen, 3);
                    let mut out = [canary(outlen), canary(outlen)];
                    for (which, (init, upd, fin)) in
                        [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate()
                    {
                        let mut st = State::for_sym("crypto_generichash_blake2b_statebytes");
                        unsafe {
                            assert_eq!(init(st.as_mut_ptr(), kp, keylen, outlen, sp, pp), 0);
                            let mut off = 0;
                            for &n in &parts {
                                assert_eq!(upd(st.as_mut_ptr(), inp[off..].as_ptr(), n as u64), 0);
                                off += n;
                            }
                            assert_eq!(fin(st.as_mut_ptr(), out[which].as_mut_ptr(), outlen), 0);
                        }
                    }
                    let (a, b) = (out[0].clone(), out[1].clone());
                    eq_bytes(
                        &format!("blake2b init_salt_personal stream(out={outlen},key={keylen},in={inlen},combo={combo})"),
                        &a,
                        &b,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Poly1305 — one-shot, streaming, verify
// ---------------------------------------------------------------------------

type OtaOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
type OtaVerify = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
type OtaInit = unsafe extern "C" fn(*mut u8, *const u8) -> i32;

#[test]
fn poly1305_one_shot_and_verify() {
    setup();
    let mut rng = Rng::new(0x7200);
    for prefix in ["crypto_onetimeauth", "crypto_onetimeauth_poly1305"] {
        let (c, r) = pair::<OtaOneShot>(prefix);
        let (cv, rv) = pair::<OtaVerify>(&format!("{prefix}_verify"));
        for &len in &[
            0usize, 1, 2, 15, 16, 17, 31, 32, 33, 47, 48, 63, 64, 65, 127, 128, 129, 1000, 2000,
        ] {
            for kind in 0..3 {
                let k = match kind {
                    0 => rng.bytes(32),
                    1 => vec![0u8; 32],
                    _ => vec![0xffu8; 32],
                };
                let inp = match kind {
                    0 => rng.bytes(len),
                    1 => vec![0u8; len],
                    _ => vec![0xffu8; len],
                };
                let mut a = canary(16);
                let mut b = canary(16);
                let (ra, rb) = unsafe {
                    (
                        c(a.as_mut_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                        r(b.as_mut_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                    )
                };
                eq_i32(&format!("{prefix} rc"), ra, rb);
                eq_bytes(&format!("{prefix}(len={len}, kind={kind})"), &a, &b);

                // verify: correct tag
                let (va, vb) = unsafe {
                    (
                        cv(a.as_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                        rv(a.as_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                    )
                };
                eq_i32(&format!("{prefix}_verify ok"), va, vb);
                assert_eq!(va, 0);
                // verify: each byte of the tag flipped
                for i in 0..16 {
                    let mut bad = a.clone();
                    bad[i] ^= 0x80;
                    let (va, vb) = unsafe {
                        (
                            cv(bad.as_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                            rv(bad.as_ptr(), inp.as_ptr(), len as u64, k.as_ptr()),
                        )
                    };
                    eq_i32(&format!("{prefix}_verify bad tag byte {i}"), va, vb);
                    assert_eq!(va, -1);
                }
            }
        }
    }
}

#[test]
fn poly1305_streaming() {
    setup();
    let mut rng = Rng::new(0x7201);
    for prefix in ["crypto_onetimeauth", "crypto_onetimeauth_poly1305"] {
        let (ci, ri) = pair::<OtaInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cf, rf) = pair::<StFinal>(&format!("{prefix}_final"));
        let sb = format!("{prefix}_statebytes");
        for &len in &[0usize, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 1000] {
            for style in 0..4u32 {
                if style == 1 && len > 300 {
                    continue;
                }
                let k = rng.bytes(32);
                let inp = rng.bytes(len);
                let parts = chunks(&mut rng, len, style);
                let mut out = [canary(16), canary(16)];
                for (which, (init, upd, fin)) in
                    [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate()
                {
                    let mut st = State::for_sym(&sb);
                    unsafe {
                        assert_eq!(init(st.as_mut_ptr(), k.as_ptr()), 0);
                        let mut off = 0;
                        for &n in &parts {
                            assert_eq!(upd(st.as_mut_ptr(), inp[off..].as_ptr(), n as u64), 0);
                            off += n;
                        }
                        assert_eq!(fin(st.as_mut_ptr(), out[which].as_mut_ptr()), 0);
                    }
                }
                let (a, b) = (out[0].clone(), out[1].clone());
                eq_bytes(&format!("{prefix} stream(len={len}, style={style})"), &a, &b);
                let (c1, _) = pair::<OtaOneShot>(prefix);
                let mut os = canary(16);
                unsafe { c1(os.as_mut_ptr(), inp.as_ptr(), len as u64, k.as_ptr()) };
                eq_bytes(&format!("{prefix} stream==one-shot(len={len})"), &os, &a);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// XOFs: shake128/256, turboshake128/256
// ---------------------------------------------------------------------------

type XofOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> i32;
type XofInitDomain = unsafe extern "C" fn(*mut u8, u8) -> i32;
type XofSqueeze = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> i32;

/// (prefix, block/rate bytes)
const XOFS: &[(&str, usize)] = &[
    ("crypto_xof_shake128", 168),
    ("crypto_xof_shake256", 136),
    ("crypto_xof_turboshake128", 168),
    ("crypto_xof_turboshake256", 136),
];

#[test]
fn xof_one_shot() {
    setup();
    let mut rng = Rng::new(0x7300);
    for &(prefix, rate) in XOFS {
        let (c, r) = pair::<XofOneShot>(prefix);
        let bb = sym::<unsafe extern "C" fn() -> usize>(c_lib(), &format!("{prefix}_blockbytes"));
        assert_eq!(unsafe { bb() }, rate, "{prefix}_blockbytes");
        for &inlen in &[0usize, 1, rate - 1, rate, rate + 1, 2 * rate, 2 * rate + 1, 1000] {
            for &outlen in &[0usize, 1, 31, 32, rate - 1, rate, rate + 1, 2 * rate + 5, 1000] {
                let inp = rng.bytes(inlen);
                let mut a = canary(outlen.max(1));
                let mut b = canary(outlen.max(1));
                let (ra, rb) = unsafe {
                    (
                        c(a.as_mut_ptr(), outlen, inp.as_ptr(), inlen as u64),
                        r(b.as_mut_ptr(), outlen, inp.as_ptr(), inlen as u64),
                    )
                };
                eq_i32(&format!("{prefix} rc(in={inlen},out={outlen})"), ra, rb);
                eq_bytes(&format!("{prefix}(in={inlen}, out={outlen})"), &a, &b);
            }
        }
    }
}

#[test]
fn xof_streaming_absorb_squeeze() {
    setup();
    let mut rng = Rng::new(0x7301);
    for &(prefix, rate) in XOFS {
        let (ci, ri) = pair::<StInit>(&format!("{prefix}_init"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cs, rs) = pair::<XofSqueeze>(&format!("{prefix}_squeeze"));
        let sb = format!("{prefix}_statebytes");
        for &inlen in &[0usize, 1, rate - 1, rate, rate + 1, 2 * rate + 3, 1000] {
            for style in 0..4u32 {
                if style == 1 && inlen > 300 {
                    continue;
                }
                // squeeze in several successive calls of varying size
                for sq_style in 0..3u32 {
                    let inp = rng.bytes(inlen);
                    let parts = chunks(&mut rng, inlen, style);
                    let total_out = 2 * rate + 37;
                    let sq: Vec<usize> = match sq_style {
                        0 => vec![total_out],
                        1 => vec![1; total_out],
                        _ => chunks(&mut rng, total_out, 3),
                    };
                    let mut out = [canary(total_out), canary(total_out)];
                    for (which, (init, upd, sqz)) in
                        [(ci, cu, cs), (ri, ru, rs)].into_iter().enumerate()
                    {
                        let mut st = State::for_sym(&sb);
                        unsafe {
                            assert_eq!(init(st.as_mut_ptr()), 0, "{prefix}_init");
                            let mut off = 0;
                            for &n in &parts {
                                assert_eq!(
                                    upd(st.as_mut_ptr(), inp[off..].as_ptr(), n as u64),
                                    0,
                                    "{prefix}_update"
                                );
                                off += n;
                            }
                            let mut o = 0usize;
                            for &n in &sq {
                                assert_eq!(
                                    sqz(st.as_mut_ptr(), out[which][o..].as_mut_ptr(), n),
                                    0,
                                    "{prefix}_squeeze"
                                );
                                o += n;
                            }
                            assert_eq!(o, total_out);
                        }
                    }
                    let (a, b) = (out[0].clone(), out[1].clone());
                    eq_bytes(
                        &format!("{prefix} stream(in={inlen}, style={style}, sq={sq_style})"),
                        &a,
                        &b,
                    );
                }
            }
        }
    }
}

#[test]
fn xof_init_with_domain() {
    setup();
    let mut rng = Rng::new(0x7302);
    for &(prefix, rate) in XOFS {
        let (ci, ri) = pair::<XofInitDomain>(&format!("{prefix}_init_with_domain"));
        let (cu, ru) = pair::<StUpdate>(&format!("{prefix}_update"));
        let (cs, rs) = pair::<XofSqueeze>(&format!("{prefix}_squeeze"));
        let sb = format!("{prefix}_statebytes");
        let std_dom =
            sym::<unsafe extern "C" fn() -> u8>(c_lib(), &format!("{prefix}_domain_standard"));
        let std_dom = unsafe { std_dom() };
        // The whole 0..=255 domain byte range is a valid *input*; whether the C
        // rejects some of it is Phase C's job — here we compare behaviour for
        // every value, return code included.
        for dom in 0u16..=255 {
            let dom = dom as u8;
            let inlen = rng.below(2 * rate + 7);
            let inp = rng.bytes(inlen);
            let total_out = rate + 13;
            let mut out = [canary(total_out), canary(total_out)];
            let mut rcs = [0i32; 2];
            for (which, (init, upd, sqz)) in [(ci, cu, cs), (ri, ru, rs)].into_iter().enumerate() {
                let mut st = State::for_sym(&sb);
                unsafe {
                    rcs[which] = init(st.as_mut_ptr(), dom);
                    if rcs[which] == 0 {
                        upd(st.as_mut_ptr(), inp.as_ptr(), inlen as u64);
                        sqz(st.as_mut_ptr(), out[which].as_mut_ptr(), total_out);
                    }
                }
            }
            eq_i32(&format!("{prefix}_init_with_domain({dom:#x}) rc"), rcs[0], rcs[1]);
            let (a, b) = (out[0].clone(), out[1].clone());
            eq_bytes(&format!("{prefix} domain={dom:#x}"), &a, &b);
            if dom == std_dom {
                assert_eq!(rcs[0], 0, "{prefix} standard domain must be accepted");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// crypto_core_keccak1600 — the raw sponge
// ---------------------------------------------------------------------------

type KcInit = unsafe extern "C" fn(*mut u8);
type KcXor = unsafe extern "C" fn(*mut u8, *const u8, usize, usize);
type KcExtract = unsafe extern "C" fn(*const u8, *mut u8, usize, usize);
type KcPermute = unsafe extern "C" fn(*mut u8);

#[test]
fn keccak1600_raw_sponge() {
    setup();
    let mut rng = Rng::new(0x7400);
    let (ci, ri) = pair::<KcInit>("crypto_core_keccak1600_init");
    let (cx, rx) = pair::<KcXor>("crypto_core_keccak1600_xor_bytes");
    let (ce, re) = pair::<KcExtract>("crypto_core_keccak1600_extract_bytes");
    let (c24, r24) = pair::<KcPermute>("crypto_core_keccak1600_permute_24");
    let (c12, r12) = pair::<KcPermute>("crypto_core_keccak1600_permute_12");
    let statebytes =
        unsafe { sym::<unsafe extern "C" fn() -> usize>(c_lib(), "crypto_core_keccak1600_statebytes")() };
    assert_eq!(statebytes, 224);

    for iter in 0..200 {
        // A random program of sponge operations, replayed identically on both.
        let ops: Vec<(u32, usize, usize)> = (0..rng.range(1, 12))
            .map(|_| {
                let op = rng.below(4) as u32;
                let offset = rng.below(200);
                let length = rng.below(200 - offset.min(199)).max(0);
                (op, offset, length)
            })
            .collect();
        let data = rng.bytes(256);
        let mut finals = [canary(200), canary(200)];
        for (which, (init, xorb, ext, p24, p12)) in
            [(ci, cx, ce, c24, c12), (ri, rx, re, r24, r12)]
                .into_iter()
                .enumerate()
        {
            let mut st = State::new(statebytes);
            unsafe {
                init(st.as_mut_ptr());
                for &(op, offset, length) in &ops {
                    match op {
                        0 => xorb(st.as_mut_ptr(), data.as_ptr(), offset, length),
                        1 => {
                            let mut tmp = vec![0u8; length.max(1)];
                            ext(st.as_ptr(), tmp.as_mut_ptr(), offset, length);
                        }
                        2 => p24(st.as_mut_ptr()),
                        _ => p12(st.as_mut_ptr()),
                    }
                }
                ext(st.as_ptr(), finals[which].as_mut_ptr(), 0, 200);
            }
        }
        let (a, b) = (finals[0].clone(), finals[1].clone());
        eq_bytes(&format!("keccak1600 program #{iter} ({} ops)", ops.len()), &a, &b);
    }
}

// ---------------------------------------------------------------------------
// KDFs
// ---------------------------------------------------------------------------

type KdfDerive =
    unsafe extern "C" fn(*mut u8, usize, u64, *const std::ffi::c_char, *const u8) -> i32;

#[test]
fn kdf_blake2b_derive_from_key() {
    setup();
    let mut rng = Rng::new(0x7500);
    for prefix in ["crypto_kdf", "crypto_kdf_blake2b"] {
        let (c, r) = pair::<KdfDerive>(&format!("{prefix}_derive_from_key"));
        let ctxbytes =
            unsafe { sym::<unsafe extern "C" fn() -> usize>(c_lib(), &format!("{prefix}_contextbytes"))() };
        let keybytes =
            unsafe { sym::<unsafe extern "C" fn() -> usize>(c_lib(), &format!("{prefix}_keybytes"))() };
        let bmin =
            unsafe { sym::<unsafe extern "C" fn() -> usize>(c_lib(), &format!("{prefix}_bytes_min"))() };
        let bmax =
            unsafe { sym::<unsafe extern "C" fn() -> usize>(c_lib(), &format!("{prefix}_bytes_max"))() };
        for &sklen in &[bmin, bmin + 1, 32, bmax - 1, bmax] {
            for &id in &[0u64, 1, 2, 0xffff_ffff, 0x1_0000_0000, u64::MAX] {
                for kind in 0..3 {
                    let key = match kind {
                        0 => rng.bytes(keybytes),
                        1 => vec![0u8; keybytes],
                        _ => vec![0xffu8; keybytes],
                    };
                    let ctx = match kind {
                        0 => rng.bytes(ctxbytes),
                        1 => vec![0u8; ctxbytes],
                        _ => vec![0xffu8; ctxbytes],
                    };
                    let mut a = canary(sklen);
                    let mut b = canary(sklen);
                    let (ra, rb) = unsafe {
                        (
                            c(
                                a.as_mut_ptr(),
                                sklen,
                                id,
                                ctx.as_ptr() as *const std::ffi::c_char,
                                key.as_ptr(),
                            ),
                            r(
                                b.as_mut_ptr(),
                                sklen,
                                id,
                                ctx.as_ptr() as *const std::ffi::c_char,
                                key.as_ptr(),
                            ),
                        )
                    };
                    eq_i32(&format!("{prefix}_derive_from_key rc"), ra, rb);
                    eq_bytes(
                        &format!("{prefix}_derive_from_key(len={sklen}, id={id}, kind={kind})"),
                        &a,
                        &b,
                    );
                }
            }
        }
    }
}

type HkdfExtract =
    unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize) -> i32;
type HkdfExpand =
    unsafe extern "C" fn(*mut u8, usize, *const std::ffi::c_char, usize, *const u8) -> i32;
type HkdfExInit = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
type HkdfExUpdate = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
type HkdfExFinal = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;

#[test]
fn hkdf_extract_expand() {
    setup();
    let mut rng = Rng::new(0x7600);
    for prefix in ["crypto_kdf_hkdf_sha256", "crypto_kdf_hkdf_sha512"] {
        let kb =
            unsafe { sym::<unsafe extern "C" fn() -> usize>(c_lib(), &format!("{prefix}_keybytes"))() };
        let bmax =
            unsafe { sym::<unsafe extern "C" fn() -> usize>(c_lib(), &format!("{prefix}_bytes_max"))() };
        let (ce, re) = pair::<HkdfExtract>(&format!("{prefix}_extract"));
        let (cx, rx) = pair::<HkdfExpand>(&format!("{prefix}_expand"));
        for &saltlen in &[0usize, 1, 16, 32, kb, kb + 1, 100, 200] {
            for &ikmlen in &[0usize, 1, 32, 100, 200] {
                let salt = rng.bytes(saltlen);
                let ikm = rng.bytes(ikmlen);
                let sp = if saltlen == 0 {
                    std::ptr::null()
                } else {
                    salt.as_ptr()
                };
                let ip = if ikmlen == 0 {
                    std::ptr::null()
                } else {
                    ikm.as_ptr()
                };
                let mut pa = canary(kb);
                let mut pb = canary(kb);
                let (ra, rb) = unsafe {
                    (
                        ce(pa.as_mut_ptr(), sp, saltlen, ip, ikmlen),
                        re(pb.as_mut_ptr(), sp, saltlen, ip, ikmlen),
                    )
                };
                eq_i32(&format!("{prefix}_extract rc"), ra, rb);
                eq_bytes(&format!("{prefix}_extract(salt={saltlen}, ikm={ikmlen})"), &pa, &pb);

                // expand from the derived PRK
                for &outlen in &[0usize, 1, 32, kb, kb + 1, 2 * kb, 3 * kb + 7, 1000, bmax] {
                    for &ctxlen in &[0usize, 1, 32, 100] {
                        let ctx = rng.bytes(ctxlen);
                        let cp = if ctxlen == 0 {
                            std::ptr::null()
                        } else {
                            ctx.as_ptr() as *const std::ffi::c_char
                        };
                        let mut a = canary(outlen.max(1));
                        let mut b = canary(outlen.max(1));
                        let (ra, rb) = unsafe {
                            (
                                cx(a.as_mut_ptr(), outlen, cp, ctxlen, pa.as_ptr()),
                                rx(b.as_mut_ptr(), outlen, cp, ctxlen, pa.as_ptr()),
                            )
                        };
                        eq_i32(&format!("{prefix}_expand rc(out={outlen})"), ra, rb);
                        eq_bytes(
                            &format!("{prefix}_expand(out={outlen}, ctx={ctxlen})"),
                            &a,
                            &b,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn hkdf_extract_streaming() {
    setup();
    let mut rng = Rng::new(0x7601);
    for prefix in ["crypto_kdf_hkdf_sha256", "crypto_kdf_hkdf_sha512"] {
        let kb =
            unsafe { sym::<unsafe extern "C" fn() -> usize>(c_lib(), &format!("{prefix}_keybytes"))() };
        let (ci, ri) = pair::<HkdfExInit>(&format!("{prefix}_extract_init"));
        let (cu, ru) = pair::<HkdfExUpdate>(&format!("{prefix}_extract_update"));
        let (cf, rf) = pair::<HkdfExFinal>(&format!("{prefix}_extract_final"));
        let (ce, _) = pair::<HkdfExtract>(&format!("{prefix}_extract"));
        let sb = format!("{prefix}_statebytes");
        for &saltlen in &[0usize, 1, 32, 100] {
            for &ikmlen in &[0usize, 1, 32, 100, 300] {
                for style in 0..4u32 {
                    let salt = rng.bytes(saltlen);
                    let ikm = rng.bytes(ikmlen);
                    let sp = if saltlen == 0 {
                        std::ptr::null()
                    } else {
                        salt.as_ptr()
                    };
                    let parts = chunks(&mut rng, ikmlen, style);
                    let mut out = [canary(kb), canary(kb)];
                    for (which, (init, upd, fin)) in
                        [(ci, cu, cf), (ri, ru, rf)].into_iter().enumerate()
                    {
                        let mut st = State::for_sym(&sb);
                        unsafe {
                            assert_eq!(init(st.as_mut_ptr(), sp, saltlen), 0);
                            let mut off = 0;
                            for &n in &parts {
                                assert_eq!(upd(st.as_mut_ptr(), ikm[off..].as_ptr(), n), 0);
                                off += n;
                            }
                            assert_eq!(fin(st.as_mut_ptr(), out[which].as_mut_ptr()), 0);
                        }
                    }
                    let (a, b) = (out[0].clone(), out[1].clone());
                    eq_bytes(
                        &format!("{prefix}_extract streaming(salt={saltlen}, ikm={ikmlen}, style={style})"),
                        &a,
                        &b,
                    );
                    // streaming must equal one-shot extract
                    let mut os = canary(kb);
                    let ip = if ikmlen == 0 {
                        std::ptr::null()
                    } else {
                        ikm.as_ptr()
                    };
                    unsafe { ce(os.as_mut_ptr(), sp, saltlen, ip, ikmlen) };
                    eq_bytes(&format!("{prefix}_extract streaming==one-shot"), &os, &a);
                }
            }
        }
    }
}

#[test]
fn hash_and_kdf_keygens() {
    setup();
    for (name, len) in [
        ("crypto_generichash_keygen", 32usize),
        ("crypto_generichash_blake2b_keygen", 32),
        ("crypto_onetimeauth_keygen", 32),
        ("crypto_onetimeauth_poly1305_keygen", 32),
        ("crypto_kdf_keygen", 32),
        ("crypto_kdf_hkdf_sha256_keygen", 32),
        ("crypto_kdf_hkdf_sha512_keygen", 64),
    ] {
        let (c, r) = pair::<Keygen>(name);
        for seed in 0..8u64 {
            let mut a = canary(len);
            let mut b = canary(len);
            reset_rngs(0xC000 + seed);
            unsafe { c(a.as_mut_ptr()) };
            reset_rngs(0xC000 + seed);
            unsafe { r(b.as_mut_ptr()) };
            eq_bytes(&format!("{name} seed={seed}"), &a, &b);
            assert_ne!(a, canary(len), "{name} wrote nothing");
        }
    }
}
