//! Phase B — valid-path differential tests, CONFIGS.md group 4
//! (pwhash, generichash, auth/hmac, kdf, hash, xof, core salsa/hsalsa/hchacha)

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_ulonglong};

type Sz = usize;

const IN_LENS: &[usize] = &[
    0, 1, 2, 15, 16, 17, 31, 32, 33, 55, 56, 63, 64, 65, 71, 72, 73, 111, 112, 119, 120, 127, 128,
    129, 135, 136, 137, 143, 144, 167, 168, 169, 239, 240, 271, 272, 335, 336, 1000, 4096,
];

fn split_points(rng: &mut Rng, total: usize, n: usize) -> Vec<usize> {
    let mut v: Vec<usize> = Vec::new();
    if n > 0 {
        for _ in 0..n.saturating_sub(1) {
            v.push(rng.below(total + 1));
        }
    }
    v.push(total);
    v.sort_unstable();
    v
}

// ------------------------------------------------------ rows 194-202: hashes

/// Generic one-shot + init/update/final differential for a fixed-size hash.
fn hash_family(prefix: &str, outlen: usize) {
    let (sbc, sbr) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_statebytes"));
    let statebytes = unsafe { sbc() };
    unsafe { assert_eq!(statebytes, sbr(), "{prefix}_statebytes") };

    let (oc, or) =
        pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong) -> c_int>(prefix);
    let (ic, ir) = pair::<unsafe extern "C" fn(*mut u8) -> c_int>(&format!("{prefix}_init"));
    let (uc, ur) = pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong) -> c_int>(
        &format!("{prefix}_update"),
    );
    let (fc, fr) = pair::<unsafe extern "C" fn(*mut u8, *mut u8) -> c_int>(&format!("{prefix}_final"));

    let mut rng = Rng::new(SEED ^ prefix.len() as u64);
    for &l in IN_LENS {
        for _ in 0..4 {
            let m = rng.bytes(l);
            let mut a = buf(outlen);
            let mut b = buf(outlen);
            unsafe {
                same_ret(
                    prefix,
                    oc(a.as_mut_ptr(), m.as_ptr(), l as c_ulonglong),
                    or(b.as_mut_ptr(), m.as_ptr(), l as c_ulonglong),
                );
            }
            same_bytes(&format!("{prefix} one-shot len={l}"), &a, &b);

            for nch in [0usize, 1, 2, 3, 5] {
                let sp = split_points(&mut rng, l, nch);
                let mut sc = buf(statebytes);
                let mut sr = buf(statebytes);
                let mut tc = buf(outlen);
                let mut tr = buf(outlen);
                unsafe {
                    same_ret("init", ic(sc.as_mut_ptr()), ir(sr.as_mut_ptr()));
                    let mut off = 0usize;
                    for &s in &sp {
                        let n = s - off;
                        same_ret(
                            "update",
                            uc(sc.as_mut_ptr(), m[off..].as_ptr(), n as c_ulonglong),
                            ur(sr.as_mut_ptr(), m[off..].as_ptr(), n as c_ulonglong),
                        );
                        off = s;
                    }
                    same_ret(
                        "final",
                        fc(sc.as_mut_ptr(), tc.as_mut_ptr()),
                        fr(sr.as_mut_ptr(), tr.as_mut_ptr()),
                    );
                }
                same_bytes(&format!("{prefix} stream len={l} n={nch}"), &tc, &tr);
                same_bytes(&format!("{prefix} stream==one-shot len={l}"), &a, &tc);
                // state bytes themselves must match after each stage
                same_bytes(&format!("{prefix} state len={l} n={nch}"), &sc, &sr);
            }
        }
    }
    for g in [format!("{prefix}_bytes")] {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&g);
        unsafe { assert_eq!(c(), r(), "{g}") };
    }
}

#[test]
fn rows194_198_sha256_sha512() {
    hash_family("crypto_hash_sha256", 32);
    hash_family("crypto_hash_sha512", 64);
    // crypto_hash (generic) exposes only the one-shot entry point.
    {
        let (oc, or) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong) -> c_int>("crypto_hash");
        let mut rng = Rng::seeded();
        for &l in IN_LENS {
            for _ in 0..4 {
                let m = rng.bytes(l);
                let mut a = buf(64);
                let mut b = buf(64);
                unsafe {
                    same_ret(
                        "crypto_hash",
                        oc(a.as_mut_ptr(), m.as_ptr(), l as c_ulonglong),
                        or(b.as_mut_ptr(), m.as_ptr(), l as c_ulonglong),
                    );
                }
                same_bytes(&format!("crypto_hash len={l}"), &a, &b);
            }
        }
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>("crypto_hash_bytes");
        unsafe { assert_eq!(c(), r(), "crypto_hash_bytes") };
    }
    let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>("crypto_hash_primitive");
    unsafe {
        let a = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
        let b = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
        same_bytes("crypto_hash_primitive", &a, &b);
    }
}

#[test]
fn rows199_202_sha3() {
    hash_family("crypto_hash_sha3256", 32);
    hash_family("crypto_hash_sha3512", 64);
}

// ------------------------------------------------------- rows 203-210: xof

fn xof_family(prefix: &str) {
    let (sbc, sbr) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_statebytes"));
    let statebytes = unsafe { sbc() };
    unsafe { assert_eq!(statebytes, sbr(), "{prefix}_statebytes") };
    let (bbc, bbr) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_blockbytes"));
    let rate = unsafe { bbc() };
    unsafe { assert_eq!(rate, bbr(), "{prefix}_blockbytes") };
    let (dsc, dsr) = pair::<unsafe extern "C" fn() -> u8>(&format!("{prefix}_domain_standard"));
    unsafe { assert_eq!(dsc(), dsr(), "{prefix}_domain_standard") };
    let default_domain = unsafe { dsc() };

    let (oc, or) = pair::<
        unsafe extern "C" fn(*mut u8, Sz, *const u8, c_ulonglong) -> c_int,
    >(prefix);
    let (ic, ir) = pair::<unsafe extern "C" fn(*mut u8) -> c_int>(&format!("{prefix}_init"));
    let (idc, idr) =
        pair::<unsafe extern "C" fn(*mut u8, u8) -> c_int>(&format!("{prefix}_init_with_domain"));
    let (uc, ur) = pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong) -> c_int>(
        &format!("{prefix}_update"),
    );
    let (qc, qr) =
        pair::<unsafe extern "C" fn(*mut u8, *mut u8, Sz) -> c_int>(&format!("{prefix}_squeeze"));

    let outlens: Vec<usize> = vec![0, 1, 2, 31, 32, 33, rate - 1, rate, rate + 1, 2 * rate, 2 * rate + 1, 500];
    let mut rng = Rng::new(SEED ^ rate as u64 ^ prefix.len() as u64);

    for &l in &[0usize, 1, 16, rate - 1, rate, rate + 1, 2 * rate, 1000] {
        for _ in 0..4 {
            let m = rng.bytes(l);
            for &ol in &outlens {
                let mut a = buf(ol + 4);
                let mut b = buf(ol + 4);
                unsafe {
                    same_ret(
                        prefix,
                        oc(a.as_mut_ptr(), ol, m.as_ptr(), l as c_ulonglong),
                        or(b.as_mut_ptr(), ol, m.as_ptr(), l as c_ulonglong),
                    );
                }
                same_bytes(&format!("{prefix} one-shot in={l} out={ol}"), &a, &b);

                // streaming: N updates, single squeeze
                for nch in [0usize, 1, 3] {
                    let sp = split_points(&mut rng, l, nch);
                    let mut sc = buf(statebytes);
                    let mut sr = buf(statebytes);
                    let mut tc = buf(ol + 4);
                    let mut tr = buf(ol + 4);
                    unsafe {
                        same_ret("init", ic(sc.as_mut_ptr()), ir(sr.as_mut_ptr()));
                        let mut off = 0usize;
                        for &s in &sp {
                            let n = s - off;
                            same_ret(
                                "update",
                                uc(sc.as_mut_ptr(), m[off..].as_ptr(), n as c_ulonglong),
                                ur(sr.as_mut_ptr(), m[off..].as_ptr(), n as c_ulonglong),
                            );
                            off = s;
                        }
                        same_ret(
                            "squeeze",
                            qc(sc.as_mut_ptr(), tc.as_mut_ptr(), ol),
                            qr(sr.as_mut_ptr(), tr.as_mut_ptr(), ol),
                        );
                    }
                    same_bytes(&format!("{prefix} stream in={l} out={ol} n={nch}"), &tc, &tr);
                    same_bytes(&format!("{prefix} stream==one-shot in={l} out={ol}"), &a, &tc);
                    same_bytes(&format!("{prefix} state in={l} out={ol}"), &sc, &sr);
                }

                // streaming: incremental multi-squeeze must concatenate to the same bytes
                if ol > 0 {
                    let mut sc = buf(statebytes);
                    let mut sr = buf(statebytes);
                    let mut accc: Vec<u8> = Vec::new();
                    let mut accr: Vec<u8> = Vec::new();
                    unsafe {
                        ic(sc.as_mut_ptr());
                        ir(sr.as_mut_ptr());
                        uc(sc.as_mut_ptr(), m.as_ptr(), l as c_ulonglong);
                        ur(sr.as_mut_ptr(), m.as_ptr(), l as c_ulonglong);
                        let mut left = ol;
                        while left > 0 {
                            let take = 1 + rng.below(left);
                            let mut c1 = buf(take);
                            let mut r1 = buf(take);
                            same_ret(
                                "squeeze inc",
                                qc(sc.as_mut_ptr(), c1.as_mut_ptr(), take),
                                qr(sr.as_mut_ptr(), r1.as_mut_ptr(), take),
                            );
                            accc.extend_from_slice(&c1);
                            accr.extend_from_slice(&r1);
                            left -= take;
                        }
                    }
                    same_bytes(&format!("{prefix} multi-squeeze in={l} out={ol}"), &accc, &accr);
                    same_bytes(
                        &format!("{prefix} multi-squeeze==one-shot in={l} out={ol}"),
                        &a[..ol],
                        &accc,
                    );
                }

                // init_with_domain: default and custom domain bytes
                for dom in [default_domain, 0x01u8, 0x06, 0x1f, 0x80, 0xff] {
                    let mut sc = buf(statebytes);
                    let mut sr = buf(statebytes);
                    let mut tc = buf(ol + 4);
                    let mut tr = buf(ol + 4);
                    unsafe {
                        same_ret(
                            "init_with_domain",
                            idc(sc.as_mut_ptr(), dom),
                            idr(sr.as_mut_ptr(), dom),
                        );
                        uc(sc.as_mut_ptr(), m.as_ptr(), l as c_ulonglong);
                        ur(sr.as_mut_ptr(), m.as_ptr(), l as c_ulonglong);
                        same_ret(
                            "squeeze",
                            qc(sc.as_mut_ptr(), tc.as_mut_ptr(), ol),
                            qr(sr.as_mut_ptr(), tr.as_mut_ptr(), ol),
                        );
                    }
                    same_bytes(
                        &format!("{prefix} domain={dom:#02x} in={l} out={ol}"),
                        &tc,
                        &tr,
                    );
                }
            }
        }
    }
}

#[test]
fn rows203_207_shake() {
    xof_family("crypto_xof_shake128");
    xof_family("crypto_xof_shake256");
}

#[test]
fn rows208_210_turboshake() {
    xof_family("crypto_xof_turboshake128");
    xof_family("crypto_xof_turboshake256");
}

// -------------------------------------------------- rows 184-193: generichash

#[test]
fn rows184_193_generichash() {
    let (sbc, sbr) = pair::<unsafe extern "C" fn() -> Sz>("crypto_generichash_statebytes");
    let statebytes = unsafe { sbc() };
    unsafe { assert_eq!(statebytes, sbr()) };

    let (osc, osr) = pair::<
        unsafe extern "C" fn(*mut u8, Sz, *const u8, c_ulonglong, *const u8, Sz) -> c_int,
    >("crypto_generichash");
    let (obc, obr) = pair::<
        unsafe extern "C" fn(*mut u8, Sz, *const u8, c_ulonglong, *const u8, Sz) -> c_int,
    >("crypto_generichash_blake2b");
    let (spc, spr) = pair::<
        unsafe extern "C" fn(
            *mut u8,
            Sz,
            *const u8,
            c_ulonglong,
            *const u8,
            Sz,
            *const u8,
            *const u8,
        ) -> c_int,
    >("crypto_generichash_blake2b_salt_personal");
    let (ic, ir) = pair::<unsafe extern "C" fn(*mut u8, *const u8, Sz, Sz) -> c_int>(
        "crypto_generichash_init",
    );
    let (ispc, ispr) = pair::<
        unsafe extern "C" fn(*mut u8, *const u8, Sz, Sz, *const u8, *const u8) -> c_int,
    >("crypto_generichash_blake2b_init_salt_personal");
    let (uc, ur) = pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong) -> c_int>(
        "crypto_generichash_update",
    );
    let (fc, fr) =
        pair::<unsafe extern "C" fn(*mut u8, *mut u8, Sz) -> c_int>("crypto_generichash_final");

    let mut rng = Rng::seeded();
    let outlens = [1usize, 2, 15, 16, 31, 32, 33, 63, 64];
    let keylens = [0usize, 1, 15, 16, 31, 32, 63, 64];

    for &l in &[0usize, 1, 63, 64, 127, 128, 129, 255, 256, 1000] {
        let m = rng.bytes(l);
        for &ol in &outlens {
            for &kl in &keylens {
                let key = rng.bytes(kl.max(1));
                // key = NULL variant (only meaningful when kl == 0)
                let kvariants: Vec<(*const u8, Sz)> = if kl == 0 {
                    vec![(std::ptr::null(), 0), (key.as_ptr(), 0)]
                } else {
                    vec![(key.as_ptr(), kl)]
                };
                for (kp, klen) in kvariants {
                    let mut a = buf(ol + 4);
                    let mut b = buf(ol + 4);
                    unsafe {
                        same_ret(
                            "crypto_generichash",
                            osc(a.as_mut_ptr(), ol, m.as_ptr(), l as c_ulonglong, kp, klen),
                            osr(b.as_mut_ptr(), ol, m.as_ptr(), l as c_ulonglong, kp, klen),
                        );
                    }
                    same_bytes(&format!("generichash ol={ol} kl={klen} l={l}"), &a, &b);

                    // primitive-named entry point must agree with the generic one
                    let mut a2 = buf(ol + 4);
                    let mut b2 = buf(ol + 4);
                    unsafe {
                        same_ret(
                            "blake2b",
                            obc(a2.as_mut_ptr(), ol, m.as_ptr(), l as c_ulonglong, kp, klen),
                            obr(b2.as_mut_ptr(), ol, m.as_ptr(), l as c_ulonglong, kp, klen),
                        );
                    }
                    same_bytes(&format!("blake2b ol={ol} kl={klen}"), &a2, &b2);
                    same_bytes("generichash == blake2b", &a, &a2);

                    // salt+personal: NULL/NULL, zero/zero, random/random
                    let salt = rng.bytes(16);
                    let pers = rng.bytes(16);
                    let zeros = vec![0u8; 16];
                    let sp_variants: [(*const u8, *const u8); 4] = [
                        (std::ptr::null(), std::ptr::null()),
                        (zeros.as_ptr(), zeros.as_ptr()),
                        (salt.as_ptr(), pers.as_ptr()),
                        (salt.as_ptr(), std::ptr::null()),
                    ];
                    for (sp_, pp) in sp_variants {
                        let mut a3 = buf(ol + 4);
                        let mut b3 = buf(ol + 4);
                        unsafe {
                            same_ret(
                                "salt_personal",
                                spc(a3.as_mut_ptr(), ol, m.as_ptr(), l as c_ulonglong, kp, klen, sp_, pp),
                                spr(b3.as_mut_ptr(), ol, m.as_ptr(), l as c_ulonglong, kp, klen, sp_, pp),
                            );
                        }
                        same_bytes(&format!("salt_personal ol={ol} kl={klen} l={l}"), &a3, &b3);

                        // streaming with salt+personal
                        let mut sc = buf(statebytes);
                        let mut sr = buf(statebytes);
                        let mut tc = buf(ol + 4);
                        let mut tr = buf(ol + 4);
                        unsafe {
                            same_ret(
                                "init_salt_personal",
                                ispc(sc.as_mut_ptr(), kp, klen, ol, sp_, pp),
                                ispr(sr.as_mut_ptr(), kp, klen, ol, sp_, pp),
                            );
                            uc(sc.as_mut_ptr(), m.as_ptr(), l as c_ulonglong);
                            ur(sr.as_mut_ptr(), m.as_ptr(), l as c_ulonglong);
                            same_ret(
                                "final",
                                fc(sc.as_mut_ptr(), tc.as_mut_ptr(), ol),
                                fr(sr.as_mut_ptr(), tr.as_mut_ptr(), ol),
                            );
                        }
                        same_bytes(&format!("stream salt_personal ol={ol} kl={klen}"), &tc, &tr);
                        same_bytes("stream salt_personal == one-shot", &a3, &tc);
                    }

                    // plain streaming with N chunks
                    for nch in [0usize, 1, 2, 5] {
                        let sp2 = split_points(&mut rng, l, nch);
                        let mut sc = buf(statebytes);
                        let mut sr = buf(statebytes);
                        let mut tc = buf(ol + 4);
                        let mut tr = buf(ol + 4);
                        unsafe {
                            same_ret(
                                "init",
                                ic(sc.as_mut_ptr(), kp, klen, ol),
                                ir(sr.as_mut_ptr(), kp, klen, ol),
                            );
                            let mut off = 0usize;
                            for &s in &sp2 {
                                let n = s - off;
                                same_ret(
                                    "update",
                                    uc(sc.as_mut_ptr(), m[off..].as_ptr(), n as c_ulonglong),
                                    ur(sr.as_mut_ptr(), m[off..].as_ptr(), n as c_ulonglong),
                                );
                                off = s;
                            }
                            same_ret(
                                "final",
                                fc(sc.as_mut_ptr(), tc.as_mut_ptr(), ol),
                                fr(sr.as_mut_ptr(), tr.as_mut_ptr(), ol),
                            );
                        }
                        same_bytes(&format!("gh stream ol={ol} kl={klen} l={l} n={nch}"), &tc, &tr);
                        same_bytes("gh stream == one-shot", &a, &tc);
                        same_bytes("gh state", &sc, &sr);
                    }
                }
            }
        }
    }
    for g in [
        "crypto_generichash_bytes_min",
        "crypto_generichash_bytes_max",
        "crypto_generichash_bytes",
        "crypto_generichash_keybytes_min",
        "crypto_generichash_keybytes_max",
        "crypto_generichash_keybytes",
        "crypto_generichash_blake2b_bytes_min",
        "crypto_generichash_blake2b_bytes_max",
        "crypto_generichash_blake2b_bytes",
        "crypto_generichash_blake2b_keybytes_min",
        "crypto_generichash_blake2b_keybytes_max",
        "crypto_generichash_blake2b_keybytes",
        "crypto_generichash_blake2b_saltbytes",
        "crypto_generichash_blake2b_personalbytes",
        "crypto_generichash_blake2b_statebytes",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(g);
        unsafe { assert_eq!(c(), r(), "{g}") };
    }
}

// ------------------------------------------------------ rows 211-217: hmac

fn hmac_family(prefix: &str, outlen: usize, block: usize) {
    let (sbc, sbr) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_statebytes"));
    let statebytes = unsafe { sbc() };
    unsafe { assert_eq!(statebytes, sbr(), "{prefix}_statebytes") };
    let (kbc, _) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_keybytes"));
    let keybytes = unsafe { kbc() };

    let (oc, or) = pair::<
        unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8) -> c_int,
    >(prefix);
    let (vc, vr) = pair::<
        unsafe extern "C" fn(*const u8, *const u8, c_ulonglong, *const u8) -> c_int,
    >(&format!("{prefix}_verify"));
    let (ic, ir) = pair::<unsafe extern "C" fn(*mut u8, *const u8, Sz) -> c_int>(&format!(
        "{prefix}_init"
    ));
    let (uc, ur) = pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong) -> c_int>(
        &format!("{prefix}_update"),
    );
    let (fc, fr) = pair::<unsafe extern "C" fn(*mut u8, *mut u8) -> c_int>(&format!("{prefix}_final"));

    let mut rng = Rng::new(SEED ^ outlen as u64 ^ block as u64);
    for &l in &[0usize, 1, 16, 63, 64, 127, 128, 129, 1000] {
        let m = rng.bytes(l);
        // fixed-key one-shot + verify
        let k = rng.bytes(keybytes);
        let mut a = buf(outlen);
        let mut b = buf(outlen);
        unsafe {
            same_ret(
                prefix,
                oc(a.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr()),
                or(b.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr()),
            );
        }
        same_bytes(&format!("{prefix} one-shot l={l}"), &a, &b);
        unsafe {
            same_ret(
                &format!("{prefix}_verify ok"),
                vc(a.as_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr()),
                vr(b.as_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr()),
            );
        }
        // init with a wide range of key lengths (incl. > block => prehashed)
        for &kl in &[0usize, 1, 16, 32, block - 1, block, block + 1, 2 * block, 2 * block + 7] {
            // Reuse `k` when the length matches so the streaming result is
            // directly comparable to the one-shot result above.
            let key = if kl == keybytes { k.clone() } else { rng.bytes(kl.max(1)) };
            let kp = if kl == 0 { std::ptr::null() } else { key.as_ptr() };
            for nch in [0usize, 1, 2, 4] {
                let sp = split_points(&mut rng, l, nch);
                let mut sc = buf(statebytes);
                let mut sr = buf(statebytes);
                let mut tc = buf(outlen);
                let mut tr = buf(outlen);
                unsafe {
                    same_ret(
                        "init",
                        ic(sc.as_mut_ptr(), kp, kl),
                        ir(sr.as_mut_ptr(), kp, kl),
                    );
                    let mut off = 0usize;
                    for &s in &sp {
                        let n = s - off;
                        same_ret(
                            "update",
                            uc(sc.as_mut_ptr(), m[off..].as_ptr(), n as c_ulonglong),
                            ur(sr.as_mut_ptr(), m[off..].as_ptr(), n as c_ulonglong),
                        );
                        off = s;
                    }
                    same_ret(
                        "final",
                        fc(sc.as_mut_ptr(), tc.as_mut_ptr()),
                        fr(sr.as_mut_ptr(), tr.as_mut_ptr()),
                    );
                }
                same_bytes(&format!("{prefix} stream l={l} kl={kl} n={nch}"), &tc, &tr);
                same_bytes(&format!("{prefix} state l={l} kl={kl}"), &sc, &sr);
                if kl == keybytes {
                    same_bytes(&format!("{prefix} stream==one-shot l={l}"), &a, &tc);
                }
            }
        }
    }
    for g in [format!("{prefix}_bytes"), format!("{prefix}_keybytes")] {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&g);
        unsafe { assert_eq!(c(), r(), "{g}") };
    }
}

#[test]
fn rows211_217_hmac() {
    hmac_family("crypto_auth_hmacsha256", 32, 64);
    hmac_family("crypto_auth_hmacsha512", 64, 128);
    hmac_family("crypto_auth_hmacsha512256", 32, 128);
    // generic crypto_auth == hmacsha512256
    let (oc, or) = pair::<
        unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8) -> c_int,
    >("crypto_auth");
    let (vc, vr) = pair::<
        unsafe extern "C" fn(*const u8, *const u8, c_ulonglong, *const u8) -> c_int,
    >("crypto_auth_verify");
    let mut rng = Rng::seeded();
    for &l in &[0usize, 1, 64, 128, 1000] {
        let m = rng.bytes(l);
        let k = rng.bytes(32);
        let mut a = buf(32);
        let mut b = buf(32);
        unsafe {
            same_ret(
                "crypto_auth",
                oc(a.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr()),
                or(b.as_mut_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr()),
            );
            same_bytes("crypto_auth", &a, &b);
            same_ret(
                "crypto_auth_verify",
                vc(a.as_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr()),
                vr(b.as_ptr(), m.as_ptr(), l as c_ulonglong, k.as_ptr()),
            );
        }
    }
    for g in ["crypto_auth_bytes", "crypto_auth_keybytes"] {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(g);
        unsafe { assert_eq!(c(), r(), "{g}") };
    }
    let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>("crypto_auth_primitive");
    unsafe {
        let a = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
        let b = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
        same_bytes("crypto_auth_primitive", &a, &b);
    }
}

// -------------------------------------------------------- rows 218-225: kdf

#[test]
fn rows218_219_kdf_blake2b() {
    for name in [
        "crypto_kdf_derive_from_key",
        "crypto_kdf_blake2b_derive_from_key",
    ] {
        let (c, r) = pair::<
            unsafe extern "C" fn(*mut u8, Sz, u64, *const c_char, *const u8) -> c_int,
        >(name);
        let mut rng = Rng::seeded();
        for &sl in &[16usize, 17, 24, 32, 33, 63, 64] {
            for &id in &[0u64, 1, 2, 255, 256, 0xdead_beef, u64::MAX] {
                for _ in 0..6 {
                    let key = rng.bytes(32);
                    let ctx = rng.bytes(8);
                    let mut a = buf(sl + 4);
                    let mut b = buf(sl + 4);
                    unsafe {
                        same_ret(
                            name,
                            c(a.as_mut_ptr(), sl, id, ctx.as_ptr() as *const c_char, key.as_ptr()),
                            r(b.as_mut_ptr(), sl, id, ctx.as_ptr() as *const c_char, key.as_ptr()),
                        );
                    }
                    same_bytes(&format!("{name} sl={sl} id={id}"), &a, &b);
                }
            }
        }
    }
    for g in [
        "crypto_kdf_bytes_min",
        "crypto_kdf_bytes_max",
        "crypto_kdf_contextbytes",
        "crypto_kdf_keybytes",
        "crypto_kdf_blake2b_bytes_min",
        "crypto_kdf_blake2b_bytes_max",
        "crypto_kdf_blake2b_contextbytes",
        "crypto_kdf_blake2b_keybytes",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(g);
        unsafe { assert_eq!(c(), r(), "{g}") };
    }
    let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>("crypto_kdf_primitive");
    unsafe {
        let a = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
        let b = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
        same_bytes("crypto_kdf_primitive", &a, &b);
    }
}

fn hkdf_family(prefix: &str, prklen: usize, bytes_max: usize) {
    let (sbc, sbr) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_statebytes"));
    let statebytes = unsafe { sbc() };
    unsafe { assert_eq!(statebytes, sbr(), "{prefix}_statebytes") };

    let (exc, exr) = pair::<
        unsafe extern "C" fn(*mut u8, *const u8, Sz, *const u8, Sz) -> c_int,
    >(&format!("{prefix}_extract"));
    let (eic, eir) = pair::<unsafe extern "C" fn(*mut u8, *const u8, Sz) -> c_int>(&format!(
        "{prefix}_extract_init"
    ));
    let (euc, eur) = pair::<unsafe extern "C" fn(*mut u8, *const u8, Sz) -> c_int>(&format!(
        "{prefix}_extract_update"
    ));
    let (efc, efr) = pair::<unsafe extern "C" fn(*mut u8, *mut u8) -> c_int>(&format!(
        "{prefix}_extract_final"
    ));
    let (epc, epr) = pair::<
        unsafe extern "C" fn(*mut u8, Sz, *const c_char, Sz, *const u8) -> c_int,
    >(&format!("{prefix}_expand"));

    let mut rng = Rng::new(SEED ^ prklen as u64);
    for &sl in &[0usize, 1, 16, 32, 64, 100] {
        for &il in &[0usize, 1, 16, 32, 64, 200] {
            let salt = rng.bytes(sl.max(1));
            let ikm = rng.bytes(il.max(1));
            let sp = if sl == 0 { std::ptr::null() } else { salt.as_ptr() };
            let mut pc = buf(prklen);
            let mut pr = buf(prklen);
            unsafe {
                same_ret(
                    &format!("{prefix}_extract"),
                    exc(pc.as_mut_ptr(), sp, sl, ikm.as_ptr(), il),
                    exr(pr.as_mut_ptr(), sp, sl, ikm.as_ptr(), il),
                );
            }
            same_bytes(&format!("{prefix}_extract sl={sl} il={il}"), &pc, &pr);

            // streaming extract with N updates
            for nch in [0usize, 1, 2, 4] {
                let splits = split_points(&mut rng, il, nch);
                let mut sc = buf(statebytes);
                let mut sr = buf(statebytes);
                let mut tc = buf(prklen);
                let mut tr = buf(prklen);
                unsafe {
                    same_ret(
                        "extract_init",
                        eic(sc.as_mut_ptr(), sp, sl),
                        eir(sr.as_mut_ptr(), sp, sl),
                    );
                    let mut off = 0usize;
                    for &s in &splits {
                        let n = s - off;
                        same_ret(
                            "extract_update",
                            euc(sc.as_mut_ptr(), ikm[off..].as_ptr(), n),
                            eur(sr.as_mut_ptr(), ikm[off..].as_ptr(), n),
                        );
                        off = s;
                    }
                    same_ret(
                        "extract_final",
                        efc(sc.as_mut_ptr(), tc.as_mut_ptr()),
                        efr(sr.as_mut_ptr(), tr.as_mut_ptr()),
                    );
                }
                same_bytes(&format!("{prefix} stream extract sl={sl} il={il}"), &tc, &tr);
                same_bytes(&format!("{prefix} stream==one-shot extract"), &pc, &tc);
            }

            // expand at length boundaries
            for &ol in &[
                0usize,
                1,
                prklen - 1,
                prklen,
                prklen + 1,
                2 * prklen,
                2 * prklen + 1,
                500,
                bytes_max,
            ] {
                for &cl in &[0usize, 1, 8, 32] {
                    let ctx = rng.bytes(cl.max(1));
                    let cp = if cl == 0 {
                        std::ptr::null()
                    } else {
                        ctx.as_ptr() as *const c_char
                    };
                    let mut a = buf(ol + 4);
                    let mut b = buf(ol + 4);
                    unsafe {
                        same_ret(
                            &format!("{prefix}_expand ol={ol}"),
                            epc(a.as_mut_ptr(), ol, cp, cl, pc.as_ptr()),
                            epr(b.as_mut_ptr(), ol, cp, cl, pr.as_ptr()),
                        );
                    }
                    same_bytes(&format!("{prefix}_expand ol={ol} cl={cl}"), &a, &b);
                }
            }
        }
    }
    for g in [
        format!("{prefix}_keybytes"),
        format!("{prefix}_bytes_min"),
        format!("{prefix}_bytes_max"),
    ] {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&g);
        unsafe { assert_eq!(c(), r(), "{g}") };
    }
}

#[test]
fn rows220_225_hkdf() {
    hkdf_family("crypto_kdf_hkdf_sha256", 32, 0xff * 32);
    hkdf_family("crypto_kdf_hkdf_sha512", 64, 0xff * 64);
}

// ----------------------------------------------------- rows 226-232: core

#[test]
fn rows226_232_core_salsa_hsalsa_hchacha() {
    // (name, outlen, inlen, keylen, constlen)
    let fns: &[(&str, usize, usize, usize)] = &[
        ("crypto_core_salsa20", 64, 16, 32),
        ("crypto_core_salsa2012", 64, 16, 32),
        ("crypto_core_salsa208", 64, 16, 32),
        ("crypto_core_hsalsa20", 32, 16, 32),
        ("crypto_core_hchacha20", 32, 16, 32),
    ];
    let mut rng = Rng::seeded();
    for &(name, ol, il, kl) in fns {
        let (c, r) = pair::<
            unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int,
        >(name);
        for _ in 0..200 {
            let inb = rng.bytes(il);
            let k = rng.bytes(kl);
            let cst = rng.bytes(16);
            for cp in [std::ptr::null(), cst.as_ptr()] {
                let mut a = buf(ol);
                let mut b = buf(ol);
                unsafe {
                    same_ret(
                        name,
                        c(a.as_mut_ptr(), inb.as_ptr(), k.as_ptr(), cp),
                        r(b.as_mut_ptr(), inb.as_ptr(), k.as_ptr(), cp),
                    );
                }
                same_bytes(&format!("{name} c_null={}", cp.is_null()), &a, &b);
            }
        }
        for suffix in ["outputbytes", "inputbytes", "keybytes", "constbytes"] {
            let g = format!("{name}_{suffix}");
            if !libs().has(&g) {
                continue;
            }
            let (gc, gr) = pair::<unsafe extern "C" fn() -> Sz>(&g);
            unsafe { assert_eq!(gc(), gr(), "{g}") };
        }
    }
}

// -------------------------------------------------- rows 172-183: pwhash

#[test]
fn rows172_179_pwhash_argon2() {
    let (pc, pr) = pair::<
        unsafe extern "C" fn(
            *mut u8,
            c_ulonglong,
            *const c_char,
            c_ulonglong,
            *const u8,
            c_ulonglong,
            Sz,
            c_int,
        ) -> c_int,
    >("crypto_pwhash");
    let mut rng = Rng::seeded();
    // (alg, minimal opslimit)
    for (alg, ops_min) in [(1i32, 3u64), (2i32, 1u64)] {
        for &ol in &[16usize, 17, 32, 64] {
            for &ops in &[ops_min, ops_min + 1] {
                for &mem in &[8192usize, 16384] {
                    for &pl in &[0usize, 1, 16, 64] {
                        let pw = rng.bytes(pl.max(1));
                        let salt = rng.bytes(16);
                        let pp = if pl == 0 {
                            std::ptr::null()
                        } else {
                            pw.as_ptr() as *const c_char
                        };
                        let mut a = buf(ol + 4);
                        let mut b = buf(ol + 4);
                        unsafe {
                            same_ret(
                                "crypto_pwhash",
                                pc(a.as_mut_ptr(), ol as c_ulonglong, pp, pl as c_ulonglong, salt.as_ptr(), ops, mem, alg),
                                pr(b.as_mut_ptr(), ol as c_ulonglong, pp, pl as c_ulonglong, salt.as_ptr(), ops, mem, alg),
                            );
                        }
                        same_bytes(
                            &format!("crypto_pwhash alg={alg} ol={ol} ops={ops} mem={mem} pl={pl}"),
                            &a,
                            &b,
                        );
                    }
                }
            }
        }
    }
    // primitive-named entry points
    for (name, alg, ops_min) in [
        ("crypto_pwhash_argon2i", 1i32, 3u64),
        ("crypto_pwhash_argon2id", 2i32, 1u64),
    ] {
        let (c, r) = pair::<
            unsafe extern "C" fn(
                *mut u8,
                c_ulonglong,
                *const c_char,
                c_ulonglong,
                *const u8,
                c_ulonglong,
                Sz,
                c_int,
            ) -> c_int,
        >(name);
        for &ol in &[16usize, 32, 64] {
            for &ops in &[ops_min, ops_min + 2] {
                let pw = rng.bytes(20);
                let salt = rng.bytes(16);
                let mut a = buf(ol);
                let mut b = buf(ol);
                unsafe {
                    same_ret(
                        name,
                        c(a.as_mut_ptr(), ol as c_ulonglong, pw.as_ptr() as *const c_char, 20, salt.as_ptr(), ops, 8192, alg),
                        r(b.as_mut_ptr(), ol as c_ulonglong, pw.as_ptr() as *const c_char, 20, salt.as_ptr(), ops, 8192, alg),
                    );
                }
                same_bytes(&format!("{name} ol={ol} ops={ops}"), &a, &b);
            }
        }
    }
}

#[test]
fn rows176_179_pwhash_str() {
    let strbytes = {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>("crypto_pwhash_strbytes");
        unsafe {
            assert_eq!(c(), r());
            c()
        }
    };
    let mut rng = Rng::seeded();

    // For each *_str producer, the output embeds a random salt so C and Rust
    // strings differ. What must match is CROSS-verification: C must accept
    // Rust's string and vice-versa, and needs_rehash must agree.
    let combos: &[(&str, &str, &str, u64)] = &[
        (
            "crypto_pwhash_argon2i_str",
            "crypto_pwhash_argon2i_str_verify",
            "crypto_pwhash_argon2i_str_needs_rehash",
            3,
        ),
        (
            "crypto_pwhash_argon2id_str",
            "crypto_pwhash_argon2id_str_verify",
            "crypto_pwhash_argon2id_str_needs_rehash",
            1,
        ),
        (
            "crypto_pwhash_str",
            "crypto_pwhash_str_verify",
            "crypto_pwhash_str_needs_rehash",
            1,
        ),
    ];
    for &(sname, vname, nname, ops_min) in combos {
        let (sc, sr) = pair::<
            unsafe extern "C" fn(*mut c_char, *const c_char, c_ulonglong, c_ulonglong, Sz) -> c_int,
        >(sname);
        let (vc, vr) = pair::<
            unsafe extern "C" fn(*const c_char, *const c_char, c_ulonglong) -> c_int,
        >(vname);
        let (nc, nr) =
            pair::<unsafe extern "C" fn(*const c_char, c_ulonglong, Sz) -> c_int>(nname);

        for &ops in &[ops_min, ops_min + 1] {
            for &mem in &[8192usize, 16384] {
                for &pl in &[0usize, 1, 16, 64] {
                    let pw = rng.bytes(pl.max(1));
                    let pp = if pl == 0 {
                        std::ptr::null()
                    } else {
                        pw.as_ptr() as *const c_char
                    };
                    let mut a = vec![0u8; strbytes];
                    let mut b = vec![0u8; strbytes];
                    unsafe {
                        same_ret(
                            sname,
                            sc(a.as_mut_ptr() as *mut c_char, pp, pl as c_ulonglong, ops, mem),
                            sr(b.as_mut_ptr() as *mut c_char, pp, pl as c_ulonglong, ops, mem),
                        );
                    }
                    // Structural prefix (everything up to the salt) must match.
                    let ac = std::ffi::CStr::from_bytes_until_nul(&a).unwrap().to_bytes();
                    let bc = std::ffi::CStr::from_bytes_until_nul(&b).unwrap().to_bytes();
                    let apre: Vec<&[u8]> = ac.split(|&x| x == b'$').collect();
                    let bpre: Vec<&[u8]> = bc.split(|&x| x == b'$').collect();
                    assert_eq!(
                        apre.len(),
                        bpre.len(),
                        "{sname}: field count differs\n C={ac:?}\n R={bc:?}"
                    );
                    for i in 0..apre.len().saturating_sub(2) {
                        assert_eq!(
                            apre[i], bpre[i],
                            "{sname}: field {i} differs (C={:?} R={:?})",
                            String::from_utf8_lossy(apre[i]),
                            String::from_utf8_lossy(bpre[i])
                        );
                    }
                    assert_eq!(ac.len(), bc.len(), "{sname}: encoded length differs");

                    // cross-verify all four directions
                    for (label, s) in [("C-string", &a), ("Rust-string", &b)] {
                        unsafe {
                            same_ret(
                                &format!("{vname} {label} correct pw"),
                                vc(s.as_ptr() as *const c_char, pp, pl as c_ulonglong),
                                vr(s.as_ptr() as *const c_char, pp, pl as c_ulonglong),
                            );
                            // wrong password
                            let bad = b"definitely-not-the-password\0";
                            same_ret(
                                &format!("{vname} {label} wrong pw"),
                                vc(s.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, 27),
                                vr(s.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, 27),
                            );
                            // needs_rehash: same params, and different params
                            same_ret(
                                &format!("{nname} {label} same"),
                                nc(s.as_ptr() as *const c_char, ops, mem),
                                nr(s.as_ptr() as *const c_char, ops, mem),
                            );
                            same_ret(
                                &format!("{nname} {label} diff ops"),
                                nc(s.as_ptr() as *const c_char, ops + 5, mem),
                                nr(s.as_ptr() as *const c_char, ops + 5, mem),
                            );
                            same_ret(
                                &format!("{nname} {label} diff mem"),
                                nc(s.as_ptr() as *const c_char, ops, mem * 2),
                                nr(s.as_ptr() as *const c_char, ops, mem * 2),
                            );
                        }
                    }
                }
            }
        }
    }
    // crypto_pwhash_str_alg with each valid alg
    let (sc, sr) = pair::<
        unsafe extern "C" fn(*mut c_char, *const c_char, c_ulonglong, c_ulonglong, Sz, c_int) -> c_int,
    >("crypto_pwhash_str_alg");
    let (vc, vr) =
        pair::<unsafe extern "C" fn(*const c_char, *const c_char, c_ulonglong) -> c_int>(
            "crypto_pwhash_str_verify",
        );
    for (alg, ops) in [(1i32, 3u64), (2i32, 1u64)] {
        let pw = b"password123\0";
        let mut a = vec![0u8; strbytes];
        let mut b = vec![0u8; strbytes];
        unsafe {
            same_ret(
                "crypto_pwhash_str_alg",
                sc(a.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, 11, ops, 8192, alg),
                sr(b.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, 11, ops, 8192, alg),
            );
            for s in [&a, &b] {
                same_ret(
                    "str_alg cross-verify",
                    vc(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, 11),
                    vr(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, 11),
                );
            }
        }
        // both must produce the same $argon2X$ prefix
        assert_eq!(&a[..12], &b[..12], "str_alg alg={alg} prefix");
    }
}

#[test]
fn rows180_182_scrypt() {
    // low-level _ll: fully deterministic
    let (llc, llr) = pair::<
        unsafe extern "C" fn(*const u8, Sz, *const u8, Sz, u64, u32, u32, *mut u8, Sz) -> c_int,
    >("crypto_pwhash_scryptsalsa208sha256_ll");
    let mut rng = Rng::seeded();
    for &n in &[2u64, 4, 8, 16, 1024] {
        for &r_ in &[1u32, 2, 8] {
            for &p in &[1u32, 2, 4] {
                for &bl in &[1usize, 16, 32, 64, 100] {
                    let pw = rng.bytes(20);
                    let salt = rng.bytes(16);
                    let mut a = buf(bl + 4);
                    let mut b = buf(bl + 4);
                    unsafe {
                        same_ret(
                            "scrypt_ll",
                            llc(pw.as_ptr(), 20, salt.as_ptr(), 16, n, r_, p, a.as_mut_ptr(), bl),
                            llr(pw.as_ptr(), 20, salt.as_ptr(), 16, n, r_, p, b.as_mut_ptr(), bl),
                        );
                    }
                    same_bytes(&format!("scrypt_ll N={n} r={r_} p={p} bl={bl}"), &a, &b);
                }
            }
        }
    }
    // high-level derive: deterministic given salt
    let (hc, hr) = pair::<
        unsafe extern "C" fn(
            *mut u8,
            c_ulonglong,
            *const c_char,
            c_ulonglong,
            *const u8,
            c_ulonglong,
            Sz,
        ) -> c_int,
    >("crypto_pwhash_scryptsalsa208sha256");
    // both pickparams branches: opslimit < memlimit/32 and opslimit >= memlimit/32
    let param_sets: &[(u64, usize)] = &[
        (16384, 16777216),
        (32768, 16777216),
        (1 << 20, 1 << 25),
        (1 << 25, 1 << 24),
        (1 << 26, 1 << 24),
        (524288, 16777216),
    ];
    for &(ops, mem) in param_sets {
        for &ol in &[16usize, 32, 64] {
            let pw = rng.bytes(20);
            let salt = rng.bytes(32);
            let mut a = buf(ol + 4);
            let mut b = buf(ol + 4);
            unsafe {
                same_ret(
                    "scrypt",
                    hc(a.as_mut_ptr(), ol as c_ulonglong, pw.as_ptr() as *const c_char, 20, salt.as_ptr(), ops, mem),
                    hr(b.as_mut_ptr(), ol as c_ulonglong, pw.as_ptr() as *const c_char, 20, salt.as_ptr(), ops, mem),
                );
            }
            same_bytes(&format!("scrypt ops={ops} mem={mem} ol={ol}"), &a, &b);
        }
    }
    // _str / _str_verify / _str_needs_rehash cross-checks
    let strbytes = {
        let (c, r) =
            pair::<unsafe extern "C" fn() -> Sz>("crypto_pwhash_scryptsalsa208sha256_strbytes");
        unsafe {
            assert_eq!(c(), r());
            c()
        }
    };
    let (sc, sr) = pair::<
        unsafe extern "C" fn(*mut c_char, *const c_char, c_ulonglong, c_ulonglong, Sz) -> c_int,
    >("crypto_pwhash_scryptsalsa208sha256_str");
    let (vc, vr) = pair::<
        unsafe extern "C" fn(*const c_char, *const c_char, c_ulonglong) -> c_int,
    >("crypto_pwhash_scryptsalsa208sha256_str_verify");
    let (nc, nr) = pair::<unsafe extern "C" fn(*const c_char, c_ulonglong, Sz) -> c_int>(
        "crypto_pwhash_scryptsalsa208sha256_str_needs_rehash",
    );
    for &(ops, mem) in &param_sets[..3] {
        let pw = b"correct horse battery staple\0";
        let mut a = vec![0u8; strbytes];
        let mut b = vec![0u8; strbytes];
        unsafe {
            same_ret(
                "scrypt_str",
                sc(a.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, 28, ops, mem),
                sr(b.as_mut_ptr() as *mut c_char, pw.as_ptr() as *const c_char, 28, ops, mem),
            );
            // "$7$" + 11 setting chars are param-derived and must match exactly
            assert_eq!(&a[..14], &b[..14], "scrypt_str setting prefix");
            for s in [&a, &b] {
                same_ret(
                    "scrypt_str_verify ok",
                    vc(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, 28),
                    vr(s.as_ptr() as *const c_char, pw.as_ptr() as *const c_char, 28),
                );
                let bad = b"wrong\0";
                same_ret(
                    "scrypt_str_verify bad",
                    vc(s.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, 5),
                    vr(s.as_ptr() as *const c_char, bad.as_ptr() as *const c_char, 5),
                );
                same_ret(
                    "scrypt_needs_rehash same",
                    nc(s.as_ptr() as *const c_char, ops, mem),
                    nr(s.as_ptr() as *const c_char, ops, mem),
                );
                same_ret(
                    "scrypt_needs_rehash diff",
                    nc(s.as_ptr() as *const c_char, ops * 2, mem),
                    nr(s.as_ptr() as *const c_char, ops * 2, mem),
                );
            }
        }
    }
}

#[test]
fn row183_pwhash_getters() {
    for g in [
        "crypto_pwhash_bytes_min",
        "crypto_pwhash_bytes_max",
        "crypto_pwhash_passwd_min",
        "crypto_pwhash_passwd_max",
        "crypto_pwhash_saltbytes",
        "crypto_pwhash_strbytes",
        "crypto_pwhash_opslimit_min",
        "crypto_pwhash_opslimit_max",
        "crypto_pwhash_memlimit_min",
        "crypto_pwhash_memlimit_max",
        "crypto_pwhash_opslimit_interactive",
        "crypto_pwhash_memlimit_interactive",
        "crypto_pwhash_opslimit_moderate",
        "crypto_pwhash_memlimit_moderate",
        "crypto_pwhash_opslimit_sensitive",
        "crypto_pwhash_memlimit_sensitive",
        "crypto_pwhash_argon2i_bytes_min",
        "crypto_pwhash_argon2i_bytes_max",
        "crypto_pwhash_argon2i_saltbytes",
        "crypto_pwhash_argon2i_strbytes",
        "crypto_pwhash_argon2i_opslimit_min",
        "crypto_pwhash_argon2i_opslimit_max",
        "crypto_pwhash_argon2i_memlimit_min",
        "crypto_pwhash_argon2i_memlimit_max",
        "crypto_pwhash_argon2id_bytes_min",
        "crypto_pwhash_argon2id_bytes_max",
        "crypto_pwhash_argon2id_saltbytes",
        "crypto_pwhash_argon2id_strbytes",
        "crypto_pwhash_argon2id_opslimit_min",
        "crypto_pwhash_argon2id_opslimit_max",
        "crypto_pwhash_argon2id_memlimit_min",
        "crypto_pwhash_argon2id_memlimit_max",
        "crypto_pwhash_scryptsalsa208sha256_bytes_min",
        "crypto_pwhash_scryptsalsa208sha256_bytes_max",
        "crypto_pwhash_scryptsalsa208sha256_passwd_min",
        "crypto_pwhash_scryptsalsa208sha256_passwd_max",
        "crypto_pwhash_scryptsalsa208sha256_saltbytes",
        "crypto_pwhash_scryptsalsa208sha256_strbytes",
        "crypto_pwhash_scryptsalsa208sha256_opslimit_min",
        "crypto_pwhash_scryptsalsa208sha256_opslimit_max",
        "crypto_pwhash_scryptsalsa208sha256_memlimit_min",
        "crypto_pwhash_scryptsalsa208sha256_memlimit_max",
    ] {
        if !libs().has(g) {
            continue;
        }
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(g);
        unsafe { assert_eq!(c(), r(), "{g}") };
    }
    for g in [
        "crypto_pwhash_alg_argon2i13",
        "crypto_pwhash_alg_argon2id13",
        "crypto_pwhash_alg_default",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn() -> c_int>(g);
        unsafe { assert_eq!(c(), r(), "{g}") };
    }
    for g in [
        "crypto_pwhash_strprefix",
        "crypto_pwhash_primitive",
        "crypto_pwhash_argon2i_strprefix",
        "crypto_pwhash_argon2id_strprefix",
        "crypto_pwhash_scryptsalsa208sha256_strprefix",
    ] {
        let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>(g);
        unsafe {
            let a = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
            let b = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
            same_bytes(g, &a, &b);
        }
    }
}
