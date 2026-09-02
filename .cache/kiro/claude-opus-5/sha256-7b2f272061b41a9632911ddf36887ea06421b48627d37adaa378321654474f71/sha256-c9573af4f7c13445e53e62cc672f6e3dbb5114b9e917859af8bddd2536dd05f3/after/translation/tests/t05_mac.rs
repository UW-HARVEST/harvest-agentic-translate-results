//! Phase B + C for the MAC / keyed-hash families.
//!
//! CONFIGS rows PB1–PB77, ERRORS rows B1–B21.
//! generichash-blake2b (every outlen/keylen, salt+personal), poly1305,
//! siphash24 / siphashx24 (every `inlen & 7` tail case), and the three HMACs
//! (key shorter than / equal to / longer than the block size).

mod harness;
use harness::*;

use std::ffi::c_int;
use std::ptr;

const SEED: u64 = 0x5EED_0004;
const STATE_MAX: usize = 512;

type Mac4 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int;
type Verify4 = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;
type Update = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;

fn statebytes(name: &str) -> usize {
    let (c, r) = sym::<unsafe extern "C" fn() -> usize>(name);
    let (cv, rv) = unsafe { (c(), r()) };
    assert_eq!(cv, rv, "{name}");
    assert!(cv <= STATE_MAX, "{name} = {cv}");
    cv
}

fn chunkings(total: usize, block: usize) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = vec![vec![total]];
    if total == 0 {
        out.push(vec![]);
        out.push(vec![0, 0]);
        return out;
    }
    out.push(std::iter::repeat(1).take(total.min(40)).chain(
        if total > 40 { vec![total - 40] } else { vec![] },
    ).collect());
    for &s in &[1usize, block / 2, block - 1, block, block + 1, 2 * block] {
        if s < total {
            out.push(vec![s, total - s]);
        }
    }
    if total > block {
        out.push(vec![block - 1, 1, total - block]);
    }
    out
}

// ---------------------------------------------------------------------------
// generichash / blake2b (PB1–PB21, B1–B15)
// ---------------------------------------------------------------------------

type GH = unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int;
type GHSP = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const u8,
    u64,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> c_int;
type GHInit = unsafe extern "C" fn(*mut u8, *const u8, usize, usize) -> c_int;
type GHInitSP =
    unsafe extern "C" fn(*mut u8, *const u8, usize, usize, *const u8, *const u8) -> c_int;
type GHFinal = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int;

#[test]
fn generichash_one_shot_full_matrix() {
    let mut rng = Rng::new(SEED);
    for name in ["crypto_generichash", "crypto_generichash_blake2b"] {
        let (c, r) = sym::<GH>(name);
        // PB1–PB6 + B1: every outlen from 0 (invalid) to 65 (invalid).
        for outlen in 0usize..=66 {
            for keylen in [0usize, 1, 16, 31, 32, 63, 64, 65] {
                for inlen in [0usize, 1, 127, 128, 129, 255, 256, 257] {
                    let msg = rng.bytes(inlen);
                    let key = rng.bytes(keylen);
                    let kp = if keylen == 0 { ptr::null() } else { key.as_ptr() };
                    let mut oc = out_buf(outlen.max(1));
                    let mut or = out_buf(outlen.max(1));
                    unsafe {
                        let rc = c(oc.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64, kp, keylen);
                        let rr = r(or.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64, kp, keylen);
                        assert_eq!(rc, rr, "{name} rc out={outlen} key={keylen} in={inlen}");
                    }
                    eqb(&format!("{name} out={outlen} key={keylen} in={inlen}"), &oc, &or);
                }
            }
        }
    }
}

#[test]
fn generichash_blake2b_salt_personal() {
    let (c, r) = sym::<GHSP>("crypto_generichash_blake2b_salt_personal");
    let mut rng = Rng::new(SEED ^ 1);
    // PB7–PB12: salt and personal each NULL or 16 bytes.
    for outlen in [0usize, 1, 16, 32, 63, 64, 65] {
        for keylen in [0usize, 1, 32, 64, 65] {
            for inlen in [0usize, 1, 128, 200] {
                let msg = rng.bytes(inlen);
                let key = rng.bytes(keylen.max(1));
                let salt = rng.bytes(16);
                let pers = rng.bytes(16);
                for (stag, sp) in [("null", ptr::null()), ("set", salt.as_ptr())] {
                    for (ptag, pp) in [("null", ptr::null()), ("set", pers.as_ptr())] {
                        let kp = if keylen == 0 { ptr::null() } else { key.as_ptr() };
                        let mut oc = out_buf(outlen.max(1));
                        let mut or = out_buf(outlen.max(1));
                        unsafe {
                            let rc = c(oc.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64, kp, keylen, sp, pp);
                            let rr = r(or.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64, kp, keylen, sp, pp);
                            assert_eq!(
                                rc, rr,
                                "salt_personal rc out={outlen} key={keylen} in={inlen} salt={stag} pers={ptag}"
                            );
                        }
                        eqb(
                            &format!("salt_personal out={outlen} key={keylen} in={inlen} salt={stag} pers={ptag}"),
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
fn generichash_streaming_all_chunkings() {
    let mut rng = Rng::new(SEED ^ 2);
    for (pfx, sb_name) in [
        ("crypto_generichash", "crypto_generichash_statebytes"),
        ("crypto_generichash_blake2b", "crypto_generichash_blake2b_statebytes"),
    ] {
        let sb = statebytes(sb_name);
        let (cinit, rinit) = sym::<GHInit>(&format!("{pfx}_init"));
        let (cupd, rupd) = sym::<Update>(&format!("{pfx}_update"));
        let (cfin, rfin) = sym::<GHFinal>(&format!("{pfx}_final"));
        let (cone, _) = sym::<GH>(pfx);

        for outlen in [0usize, 1, 16, 32, 64, 65] {
            for keylen in [0usize, 1, 32, 64, 65] {
                for inlen in [0usize, 1, 127, 128, 129, 255, 256, 257, 400] {
                    let msg = rng.bytes(inlen);
                    let key = rng.bytes(keylen.max(1));
                    let kp = if keylen == 0 { ptr::null() } else { key.as_ptr() };

                    for chunks in chunkings(inlen, 128) {
                        if chunks.iter().sum::<usize>() != inlen {
                            continue;
                        }
                        let mut stc = vec![0xa5u8; STATE_MAX];
                        let mut str_ = vec![0xa5u8; STATE_MAX];
                        let (ic, ir) = unsafe {
                            (
                                cinit(stc.as_mut_ptr(), kp, keylen, outlen),
                                rinit(str_.as_mut_ptr(), kp, keylen, outlen),
                            )
                        };
                        assert_eq!(ic, ir, "{pfx}_init rc out={outlen} key={keylen}");
                        eqb(&format!("{pfx}_init state out={outlen} key={keylen}"), &stc[..sb], &str_[..sb]);
                        if ic != 0 {
                            continue;
                        }
                        let mut off = 0usize;
                        for &n in &chunks {
                            unsafe {
                                let uc = cupd(stc.as_mut_ptr(), msg[off..].as_ptr(), n as u64);
                                let ur = rupd(str_.as_mut_ptr(), msg[off..].as_ptr(), n as u64);
                                assert_eq!(uc, ur, "{pfx}_update rc");
                            }
                            eqb(&format!("{pfx}_update state in={inlen} chunks={chunks:?}"), &stc[..sb], &str_[..sb]);
                            off += n;
                        }
                        // final with the matching outlen and with mismatched
                        // ones. NOTE: crypto_generichash*_final performs NO
                        // range check of its own — blake2b_final() calls
                        // sodium_misuse() for outlen 0 or > 64, so those two
                        // values are exercised in
                        // `generichash_misuse_paths_abort_identically` instead.
                        for flen in [outlen, 1, 32, 64] {
                            if flen == 0 || flen > 64 {
                                continue;
                            }
                            let mut sc2 = stc.clone();
                            let mut sr2 = str_.clone();
                            let mut oc = out_buf(flen.max(1));
                            let mut or = out_buf(flen.max(1));
                            unsafe {
                                let fc = cfin(sc2.as_mut_ptr(), oc.as_mut_ptr(), flen);
                                let fr = rfin(sr2.as_mut_ptr(), or.as_mut_ptr(), flen);
                                assert_eq!(fc, fr, "{pfx}_final rc out={outlen} flen={flen}");
                            }
                            eqb(&format!("{pfx}_final out={outlen} flen={flen} in={inlen}"), &oc, &or);
                            eqb(&format!("{pfx}_final state out={outlen} flen={flen}"), &sc2[..sb], &sr2[..sb]);
                            // B5: final() twice
                            let mut oc2 = out_buf(flen.max(1));
                            let mut or2 = out_buf(flen.max(1));
                            unsafe {
                                let fc = cfin(sc2.as_mut_ptr(), oc2.as_mut_ptr(), flen);
                                let fr = rfin(sr2.as_mut_ptr(), or2.as_mut_ptr(), flen);
                                assert_eq!(fc, fr, "{pfx}_final twice rc");
                            }
                            eqb(&format!("{pfx}_final twice out={outlen} flen={flen}"), &oc2, &or2);
                        }
                        // one-shot equivalence
                        if outlen >= 16 && outlen <= 64 && keylen <= 64 {
                            let mut want = vec![0u8; outlen];
                            unsafe { cone(want.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64, kp, keylen) };
                            let mut sc2 = stc.clone();
                            let mut oc = vec![0u8; outlen];
                            unsafe { cfin(sc2.as_mut_ptr(), oc.as_mut_ptr(), outlen) };
                            eqb(&format!("{pfx} streaming==one-shot out={outlen} in={inlen}"), &want, &oc);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn generichash_blake2b_init_salt_personal_streaming() {
    let sb = statebytes("crypto_generichash_blake2b_statebytes");
    let (cinit, rinit) = sym::<GHInitSP>("crypto_generichash_blake2b_init_salt_personal");
    let (cupd, rupd) = sym::<Update>("crypto_generichash_blake2b_update");
    let (cfin, rfin) = sym::<GHFinal>("crypto_generichash_blake2b_final");
    let (cone, _) = sym::<GHSP>("crypto_generichash_blake2b_salt_personal");
    let mut rng = Rng::new(SEED ^ 3);
    for outlen in [0usize, 1, 16, 32, 64, 65] {
        for keylen in [0usize, 32, 64, 65] {
            for inlen in [0usize, 1, 128, 129, 300] {
                let msg = rng.bytes(inlen);
                let key = rng.bytes(keylen.max(1));
                let salt = rng.bytes(16);
                let pers = rng.bytes(16);
                for (stag, sp) in [("null", ptr::null()), ("set", salt.as_ptr())] {
                    for (ptag, pp) in [("null", ptr::null()), ("set", pers.as_ptr())] {
                        let kp = if keylen == 0 { ptr::null() } else { key.as_ptr() };
                        let mut stc = vec![0xa5u8; STATE_MAX];
                        let mut str_ = vec![0xa5u8; STATE_MAX];
                        let (ic, ir) = unsafe {
                            (
                                cinit(stc.as_mut_ptr(), kp, keylen, outlen, sp, pp),
                                rinit(str_.as_mut_ptr(), kp, keylen, outlen, sp, pp),
                            )
                        };
                        assert_eq!(ic, ir, "init_salt_personal rc out={outlen} key={keylen}");
                        eqb(
                            &format!("init_salt_personal state out={outlen} key={keylen} {stag}/{ptag}"),
                            &stc[..sb],
                            &str_[..sb],
                        );
                        if ic != 0 {
                            continue;
                        }
                        unsafe {
                            cupd(stc.as_mut_ptr(), msg.as_ptr(), inlen as u64);
                            rupd(str_.as_mut_ptr(), msg.as_ptr(), inlen as u64);
                        }
                        let mut oc = out_buf(outlen.max(1));
                        let mut or = out_buf(outlen.max(1));
                        unsafe {
                            let fc = cfin(stc.as_mut_ptr(), oc.as_mut_ptr(), outlen);
                            let fr = rfin(str_.as_mut_ptr(), or.as_mut_ptr(), outlen);
                            assert_eq!(fc, fr, "final rc");
                        }
                        eqb(
                            &format!("init_salt_personal out={outlen} key={keylen} in={inlen} {stag}/{ptag}"),
                            &oc,
                            &or,
                        );
                        if outlen >= 16 && outlen <= 64 && keylen <= 64 {
                            let mut want = vec![0u8; outlen];
                            unsafe {
                                cone(want.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64, kp, keylen, sp, pp)
                            };
                            eqb(
                                &format!("sp streaming==one-shot out={outlen} in={inlen} {stag}/{ptag}"),
                                &want,
                                &oc[..outlen],
                            );
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// poly1305 (PB22–PB32, B16)
// ---------------------------------------------------------------------------

#[test]
fn poly1305_one_shot_streaming_verify() {
    let mut rng = Rng::new(SEED ^ 4);
    for pfx in ["crypto_onetimeauth", "crypto_onetimeauth_poly1305"] {
        let sb = statebytes(&format!("{pfx}_statebytes"));
        let (cone, rone) = sym::<Mac4>(pfx);
        let (cver, rver) = sym::<Verify4>(&format!("{pfx}_verify"));
        let (cinit, rinit) = sym::<unsafe extern "C" fn(*mut u8, *const u8) -> c_int>(&format!("{pfx}_init"));
        let (cupd, rupd) = sym::<Update>(&format!("{pfx}_update"));
        let (cfin, rfin) = sym::<unsafe extern "C" fn(*mut u8, *mut u8) -> c_int>(&format!("{pfx}_final"));

        let mut lens: Vec<usize> = (0..=40).collect();
        lens.extend_from_slice(&[63, 64, 65, 127, 128, 129, 255, 256, 1000, 1024]);
        for len in lens {
            let msg = rng.bytes(len);
            let key = rng.bytes(32);
            let mut oc = out_buf(16);
            let mut or = out_buf(16);
            unsafe {
                let rc = cone(oc.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                let rr = rone(or.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                assert_eq!(rc, rr, "{pfx} rc len={len}");
            }
            eqb(&format!("{pfx} one-shot len={len}"), &oc, &or);

            // verify: correct tag, and every single-bit corruption of the tag
            unsafe {
                let vc = cver(oc.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                let vr = rver(oc.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                assert_eq!(vc, vr, "{pfx}_verify good rc len={len}");
                assert_eq!(vc, 0, "{pfx}_verify good should be 0");
            }
            for bit in 0..128usize {
                let mut bad = oc[..16].to_vec();
                bad[bit / 8] ^= 1 << (bit % 8);
                unsafe {
                    let vc = cver(bad.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                    let vr = rver(bad.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                    assert_eq!(vc, vr, "{pfx}_verify bad bit={bit} len={len}");
                }
            }
            // corrupted message
            if len > 0 {
                let mut m2 = msg.clone();
                m2[rng.below(len)] ^= 0x80;
                unsafe {
                    let vc = cver(oc.as_ptr(), m2.as_ptr(), len as u64, key.as_ptr());
                    let vr = rver(oc.as_ptr(), m2.as_ptr(), len as u64, key.as_ptr());
                    assert_eq!(vc, vr, "{pfx}_verify bad-msg len={len}");
                }
            }

            // streaming with every chunking that straddles the 16-byte block
            for chunks in chunkings(len, 16) {
                if chunks.iter().sum::<usize>() != len {
                    continue;
                }
                let mut stc = vec![0xa5u8; STATE_MAX];
                let mut str_ = vec![0xa5u8; STATE_MAX];
                unsafe {
                    let ic = cinit(stc.as_mut_ptr(), key.as_ptr());
                    let ir = rinit(str_.as_mut_ptr(), key.as_ptr());
                    assert_eq!(ic, ir);
                }
                eqb(&format!("{pfx}_init state"), &stc[..sb], &str_[..sb]);
                let mut off = 0;
                for &n in &chunks {
                    unsafe {
                        let uc = cupd(stc.as_mut_ptr(), msg[off..].as_ptr(), n as u64);
                        let ur = rupd(str_.as_mut_ptr(), msg[off..].as_ptr(), n as u64);
                        assert_eq!(uc, ur, "{pfx}_update rc");
                    }
                    eqb(&format!("{pfx}_update state len={len} chunks={chunks:?}"), &stc[..sb], &str_[..sb]);
                    off += n;
                }
                let mut fc_o = out_buf(16);
                let mut fr_o = out_buf(16);
                unsafe {
                    let fc = cfin(stc.as_mut_ptr(), fc_o.as_mut_ptr());
                    let fr = rfin(str_.as_mut_ptr(), fr_o.as_mut_ptr());
                    assert_eq!(fc, fr, "{pfx}_final rc");
                }
                eqb(&format!("{pfx}_final len={len} chunks={chunks:?}"), &fc_o, &fr_o);
                eqb(&format!("{pfx} streaming==one-shot len={len} chunks={chunks:?}"), &oc[..16], &fc_o[..16]);
            }
        }
        // all-zero key and all-ones key (r clamping edge cases)
        for (tag, key) in [("zero", vec![0u8; 32]), ("ones", vec![0xffu8; 32])] {
            for len in [0usize, 1, 15, 16, 17, 32, 33] {
                let msg = vec![0xa5u8; len];
                let mut oc = out_buf(16);
                let mut or = out_buf(16);
                unsafe {
                    cone(oc.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                    rone(or.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                }
                eqb(&format!("{pfx} key={tag} len={len}"), &oc, &or);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// siphash24 / siphashx24 (PB33–PB54)
// ---------------------------------------------------------------------------

#[test]
fn siphash_every_tail_case() {
    let mut rng = Rng::new(SEED ^ 5);
    for (name, outlen) in [
        ("crypto_shorthash", 8usize),
        ("crypto_shorthash_siphash24", 8),
        ("crypto_shorthash_siphashx24", 16),
    ] {
        let (c, r) = sym::<Mac4>(name);
        // exhaustive over inlen 0..=80 -> every (inlen & 7) tail case at every
        // whole-word count
        for len in 0usize..=80 {
            for _ in 0..8 {
                let msg = rng.bytes(len);
                let key = rng.bytes(16);
                let mut oc = out_buf(outlen);
                let mut or = out_buf(outlen);
                unsafe {
                    let rc = c(oc.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                    let rr = r(or.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                    assert_eq!(rc, rr, "{name} rc len={len}");
                }
                eqb(&format!("{name} len={len}"), &oc, &or);
            }
        }
        for len in [255usize, 256, 257, 1000, 4096] {
            let msg = rng.bytes(len);
            let key = rng.bytes(16);
            let mut oc = out_buf(outlen);
            let mut or = out_buf(outlen);
            unsafe {
                c(oc.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                r(or.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
            }
            eqb(&format!("{name} long len={len}"), &oc, &or);
        }
        // extreme keys
        for (tag, key) in [("zero", vec![0u8; 16]), ("ones", vec![0xffu8; 16])] {
            for len in 0usize..=16 {
                let msg = vec![0x5au8; len];
                let mut oc = out_buf(outlen);
                let mut or = out_buf(outlen);
                unsafe {
                    c(oc.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                    r(or.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                }
                eqb(&format!("{name} key={tag} len={len}"), &oc, &or);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// HMAC-SHA256 / SHA512 / SHA512256 (PB55–PB77, B17–B21)
// ---------------------------------------------------------------------------

struct Hmac {
    pfx: &'static str,
    outlen: usize,
    block: usize,
    /// `_init` accepts an arbitrary key length; the one-shot uses KEYBYTES.
    keybytes: usize,
}

const HMACS: &[Hmac] = &[
    Hmac { pfx: "crypto_auth_hmacsha256", outlen: 32, block: 64, keybytes: 32 },
    Hmac { pfx: "crypto_auth_hmacsha512", outlen: 64, block: 128, keybytes: 32 },
    Hmac { pfx: "crypto_auth_hmacsha512256", outlen: 32, block: 128, keybytes: 32 },
];

#[test]
fn hmac_one_shot_and_verify() {
    let mut rng = Rng::new(SEED ^ 6);
    for h in HMACS {
        let (cone, rone) = sym::<Mac4>(h.pfx);
        let (cver, rver) = sym::<Verify4>(&format!("{}_verify", h.pfx));
        let mut lens: Vec<usize> = (0..=8).collect();
        lens.extend_from_slice(&[
            h.block - 1,
            h.block,
            h.block + 1,
            2 * h.block - 1,
            2 * h.block,
            2 * h.block + 1,
            1000,
        ]);
        for len in lens {
            for _ in 0..4 {
                let msg = rng.bytes(len);
                let key = rng.bytes(h.keybytes);
                let mut oc = out_buf(h.outlen);
                let mut or = out_buf(h.outlen);
                unsafe {
                    let rc = cone(oc.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                    let rr = rone(or.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                    assert_eq!(rc, rr, "{} rc len={len}", h.pfx);
                }
                eqb(&format!("{} len={len}", h.pfx), &oc, &or);
                unsafe {
                    let vc = cver(oc.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                    let vr = rver(oc.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                    assert_eq!(vc, vr, "{}_verify good len={len}", h.pfx);
                    assert_eq!(vc, 0);
                }
                for bit in [0usize, 1, 7, 8, h.outlen * 8 - 1] {
                    let mut bad = oc[..h.outlen].to_vec();
                    bad[bit / 8] ^= 1 << (bit % 8);
                    unsafe {
                        let vc = cver(bad.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                        let vr = rver(bad.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr());
                        assert_eq!(vc, vr, "{}_verify bad bit={bit} len={len}", h.pfx);
                    }
                }
            }
        }
    }
}

#[test]
fn hmac_streaming_all_key_lengths() {
    let mut rng = Rng::new(SEED ^ 7);
    for h in HMACS {
        let sb = statebytes(&format!("{}_statebytes", h.pfx));
        let (cinit, rinit) =
            sym::<unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int>(&format!("{}_init", h.pfx));
        let (cupd, rupd) = sym::<Update>(&format!("{}_update", h.pfx));
        let (cfin, rfin) =
            sym::<unsafe extern "C" fn(*mut u8, *mut u8) -> c_int>(&format!("{}_final", h.pfx));
        let (cone, _) = sym::<Mac4>(h.pfx);

        // PB55–PB70: keylen below / equal / above the block size, which is
        // exactly where _init branches (long keys get pre-hashed).
        let mut keylens: Vec<usize> = vec![1, 16, 31, 32, 33, 63, 64, 65];
        keylens.extend_from_slice(&[
            h.block - 1,
            h.block,
            h.block + 1,
            2 * h.block,
            2 * h.block + 1,
            300,
        ]);
        keylens.sort();
        keylens.dedup();

        for keylen in keylens {
            for inlen in [0usize, 1, h.block - 1, h.block, h.block + 1, 2 * h.block + 5, 500] {
                let msg = rng.bytes(inlen);
                let key = rng.bytes(keylen);
                for chunks in chunkings(inlen, h.block) {
                    if chunks.iter().sum::<usize>() != inlen {
                        continue;
                    }
                    let mut stc = vec![0xa5u8; STATE_MAX];
                    let mut str_ = vec![0xa5u8; STATE_MAX];
                    unsafe {
                        let ic = cinit(stc.as_mut_ptr(), key.as_ptr(), keylen);
                        let ir = rinit(str_.as_mut_ptr(), key.as_ptr(), keylen);
                        assert_eq!(ic, ir, "{}_init rc keylen={keylen}", h.pfx);
                    }
                    eqb(&format!("{}_init state keylen={keylen}", h.pfx), &stc[..sb], &str_[..sb]);
                    let mut off = 0;
                    for &n in &chunks {
                        unsafe {
                            let uc = cupd(stc.as_mut_ptr(), msg[off..].as_ptr(), n as u64);
                            let ur = rupd(str_.as_mut_ptr(), msg[off..].as_ptr(), n as u64);
                            assert_eq!(uc, ur, "{}_update rc", h.pfx);
                        }
                        eqb(
                            &format!("{}_update state keylen={keylen} in={inlen} chunks={chunks:?}", h.pfx),
                            &stc[..sb],
                            &str_[..sb],
                        );
                        off += n;
                    }
                    let mut oc = out_buf(h.outlen);
                    let mut or = out_buf(h.outlen);
                    unsafe {
                        let fc = cfin(stc.as_mut_ptr(), oc.as_mut_ptr());
                        let fr = rfin(str_.as_mut_ptr(), or.as_mut_ptr());
                        assert_eq!(fc, fr, "{}_final rc", h.pfx);
                    }
                    eqb(
                        &format!("{}_final keylen={keylen} in={inlen} chunks={chunks:?}", h.pfx),
                        &oc,
                        &or,
                    );
                    // one-shot equivalence when keylen == KEYBYTES
                    if keylen == h.keybytes {
                        let mut want = vec![0u8; h.outlen];
                        unsafe { cone(want.as_mut_ptr(), msg.as_ptr(), inlen as u64, key.as_ptr()) };
                        eqb(
                            &format!("{} streaming==one-shot keylen={keylen} in={inlen}", h.pfx),
                            &want,
                            &oc[..h.outlen],
                        );
                    }
                }
            }
        }
        // keylen == 0 with a non-NULL pointer
        for inlen in [0usize, 1, 100] {
            let msg = rng.bytes(inlen.max(1));
            let key = [0u8; 1];
            let mut stc = vec![0u8; STATE_MAX];
            let mut str_ = vec![0u8; STATE_MAX];
            unsafe {
                let ic = cinit(stc.as_mut_ptr(), key.as_ptr(), 0);
                let ir = rinit(str_.as_mut_ptr(), key.as_ptr(), 0);
                assert_eq!(ic, ir, "{}_init keylen=0 rc", h.pfx);
                cupd(stc.as_mut_ptr(), msg.as_ptr(), inlen as u64);
                rupd(str_.as_mut_ptr(), msg.as_ptr(), inlen as u64);
            }
            let mut oc = out_buf(h.outlen);
            let mut or = out_buf(h.outlen);
            unsafe {
                cfin(stc.as_mut_ptr(), oc.as_mut_ptr());
                rfin(str_.as_mut_ptr(), or.as_mut_ptr());
            }
            eqb(&format!("{} keylen=0 in={inlen}", h.pfx), &oc, &or);
        }
    }
}

/// B19–B20: `crypto_auth_hmacsha256_init` / `hmacsha512_init` call
/// `sodium_misuse()` when `key == NULL` and `keylen > 0`.
#[test]
fn hmac_init_null_key_aborts_identically() {
    for pfx in [
        "crypto_auth_hmacsha256",
        "crypto_auth_hmacsha512",
        "crypto_auth_hmacsha512256",
    ] {
        for keylen in [0usize, 1, 32, 200] {
            let name = format!("{pfx}_init");
            let n1 = name.clone();
            let n2 = name.clone();
            same_outcome(
                &format!("{name} NULL key keylen={keylen}"),
                move || {
                    let (c, _) =
                        sym::<unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int>(&n1);
                    let mut st = vec![0u8; STATE_MAX];
                    unsafe { c(st.as_mut_ptr(), ptr::null(), keylen) }
                },
                move || {
                    let (_, r) =
                        sym::<unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int>(&n2);
                    let mut st = vec![0u8; STATE_MAX];
                    unsafe { r(st.as_mut_ptr(), ptr::null(), keylen) }
                },
            );
        }
    }
}

/// The generic `crypto_auth` façade (delegates to hmacsha512256).
#[test]
fn crypto_auth_generic() {
    let (cone, rone) = sym::<Mac4>("crypto_auth");
    let (cver, rver) = sym::<Verify4>("crypto_auth_verify");
    let mut rng = Rng::new(SEED ^ 8);
    for len in [0usize, 1, 127, 128, 129, 1000] {
        let msg = rng.bytes(len);
        let key = rng.bytes(32);
        let mut oc = out_buf(32);
        let mut or = out_buf(32);
        unsafe {
            assert_eq!(
                cone(oc.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                rone(or.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr())
            );
        }
        eqb(&format!("crypto_auth len={len}"), &oc, &or);
        unsafe {
            assert_eq!(
                cver(oc.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                rver(oc.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr())
            );
            let mut bad = oc[..32].to_vec();
            bad[0] ^= 1;
            assert_eq!(
                cver(bad.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                rver(bad.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr())
            );
        }
    }
}

// ---------------------------------------------------------------------------
// blake2b `sodium_misuse()` / `assert()` paths.
//
// These are reachable from the PUBLIC API even though the public wrappers look
// like they range-check everything:
//
//  * `crypto_generichash[_blake2b]` forwards `key` unchanged to `blake2b()`,
//    which calls `sodium_misuse()` when `key == NULL && keylen > 0`. The
//    wrapper's own check only rejects `keylen > 64`, so NULL + 1..=64 aborts.
//  * `crypto_generichash[_blake2b]_final` has NO range check at all: it casts
//    `outlen` to `uint8_t` and hands it to `blake2b_final()`, which calls
//    `sodium_misuse()` for `outlen == 0` or `outlen > 64`. It also has a live
//    `assert(outlen <= UINT8_MAX)` (the C is built without `-DNDEBUG`), so
//    `outlen >= 256` aborts through the assertion instead.
//
// The C is the ground truth, so the Rust must abort in exactly the same cases.
// ---------------------------------------------------------------------------

#[test]
fn generichash_misuse_paths_abort_identically() {
    // NULL key with keylen > 0, across every valid outlen bucket.
    for name in ["crypto_generichash", "crypto_generichash_blake2b"] {
        for keylen in [0usize, 1, 32, 64, 65, 200] {
            for outlen in [0usize, 1, 32, 64, 65] {
                let n1 = name.to_string();
                let n2 = name.to_string();
                same_outcome(
                    &format!("{name} NULL key keylen={keylen} outlen={outlen}"),
                    move || {
                        let (c, _) = sym::<GH>(&n1);
                        let mut o = vec![0u8; 128];
                        unsafe { c(o.as_mut_ptr(), outlen, ptr::null(), 0, ptr::null(), keylen) }
                    },
                    move || {
                        let (_, r) = sym::<GH>(&n2);
                        let mut o = vec![0u8; 128];
                        unsafe { r(o.as_mut_ptr(), outlen, ptr::null(), 0, ptr::null(), keylen) }
                    },
                );
            }
        }
    }
    // Same for the salt_personal one-shot.
    for keylen in [0usize, 1, 64, 65] {
        for outlen in [0usize, 32, 65] {
            same_outcome(
                &format!("blake2b_salt_personal NULL key keylen={keylen} outlen={outlen}"),
                move || {
                    let (c, _) = sym::<GHSP>("crypto_generichash_blake2b_salt_personal");
                    let mut o = vec![0u8; 128];
                    unsafe {
                        c(o.as_mut_ptr(), outlen, ptr::null(), 0, ptr::null(), keylen, ptr::null(), ptr::null())
                    }
                },
                move || {
                    let (_, r) = sym::<GHSP>("crypto_generichash_blake2b_salt_personal");
                    let mut o = vec![0u8; 128];
                    unsafe {
                        r(o.as_mut_ptr(), outlen, ptr::null(), 0, ptr::null(), keylen, ptr::null(), ptr::null())
                    }
                },
            );
        }
    }
    // `in == NULL && inlen > 0`
    for name in ["crypto_generichash", "crypto_generichash_blake2b"] {
        let n1 = name.to_string();
        let n2 = name.to_string();
        same_outcome(
            &format!("{name} NULL in with inlen>0"),
            move || {
                let (c, _) = sym::<GH>(&n1);
                let mut o = vec![0u8; 128];
                unsafe { c(o.as_mut_ptr(), 32, ptr::null(), 16, ptr::null(), 0) }
            },
            move || {
                let (_, r) = sym::<GH>(&n2);
                let mut o = vec![0u8; 128];
                unsafe { r(o.as_mut_ptr(), 32, ptr::null(), 16, ptr::null(), 0) }
            },
        );
    }
    // `out == NULL`
    for name in ["crypto_generichash", "crypto_generichash_blake2b"] {
        let n1 = name.to_string();
        let n2 = name.to_string();
        same_outcome(
            &format!("{name} NULL out"),
            move || {
                let (c, _) = sym::<GH>(&n1);
                let inp = [1u8, 2, 3];
                unsafe { c(ptr::null_mut(), 32, inp.as_ptr(), 3, ptr::null(), 0) }
            },
            move || {
                let (_, r) = sym::<GH>(&n2);
                let inp = [1u8, 2, 3];
                unsafe { r(ptr::null_mut(), 32, inp.as_ptr(), 3, ptr::null(), 0) }
            },
        );
    }
}

#[test]
fn generichash_final_out_of_range_outlen_aborts_identically() {
    for pfx in ["crypto_generichash", "crypto_generichash_blake2b"] {
        // outlen 0 and > 64 -> blake2b_final -> sodium_misuse()
        // outlen >= 256     -> live assert(outlen <= UINT8_MAX)
        for flen in [0usize, 65, 100, 255, 256, 257, 1000] {
            let a = format!("{pfx}_init");
            let b = format!("{pfx}_update");
            let d = format!("{pfx}_final");
            let (a2, b2, d2) = (a.clone(), b.clone(), d.clone());
            same_outcome(
                &format!("{pfx}_final outlen={flen}"),
                move || {
                    let (ci, _) = sym::<GHInit>(&a);
                    let (cu, _) = sym::<Update>(&b);
                    let (cf, _) = sym::<GHFinal>(&d);
                    let mut st = vec![0u8; STATE_MAX];
                    let msg = [7u8; 200];
                    let mut o = vec![0u8; 2048];
                    unsafe {
                        if ci(st.as_mut_ptr(), ptr::null(), 0, 32) != 0 {
                            return 9;
                        }
                        cu(st.as_mut_ptr(), msg.as_ptr(), 200);
                        cf(st.as_mut_ptr(), o.as_mut_ptr(), flen)
                    }
                },
                move || {
                    let (_, ri) = sym::<GHInit>(&a2);
                    let (_, ru) = sym::<Update>(&b2);
                    let (_, rf) = sym::<GHFinal>(&d2);
                    let mut st = vec![0u8; STATE_MAX];
                    let msg = [7u8; 200];
                    let mut o = vec![0u8; 2048];
                    unsafe {
                        if ri(st.as_mut_ptr(), ptr::null(), 0, 32) != 0 {
                            return 9;
                        }
                        ru(st.as_mut_ptr(), msg.as_ptr(), 200);
                        rf(st.as_mut_ptr(), o.as_mut_ptr(), flen)
                    }
                },
            );
        }
    }
}
