//! Phase B — CONFIGS.md rows 1–20: the lowest-level primitives, called
//! directly through their `.so` exports rather than via any wrapper.

mod common;
use common::*;

type Core4 = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> i32;
type Hash4 = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32;
type Verify4 = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
type Stream4 = unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> i32;
type StreamXor = unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32;
type StreamXorIc64 =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> i32;
type StreamXorIc32 =
    unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> i32;

/// Rows 1–3: salsa20/salsa2012/salsa208/hsalsa20/hchacha20 core functions,
/// with `c == NULL` (built-in sigma constant) and with `c` supplied.
#[test]
fn core_functions() {
    let d = duo();
    let mut rng = Rng::new(0x5A15A0);
    for name in [
        "crypto_core_salsa20",
        "crypto_core_salsa2012",
        "crypto_core_salsa208",
        "crypto_core_hsalsa20",
        "crypto_core_hchacha20",
    ] {
        let (cf, rf) = d.pair::<Core4>(name);
        let (obf, _) = d.pair::<unsafe extern "C" fn() -> usize>(&format!("{name}_outputbytes"));
        let (ibf, _) = d.pair::<unsafe extern "C" fn() -> usize>(&format!("{name}_inputbytes"));
        let (kbf, _) = d.pair::<unsafe extern "C" fn() -> usize>(&format!("{name}_keybytes"));
        let (cbf, _) = d.pair::<unsafe extern "C" fn() -> usize>(&format!("{name}_constbytes"));
        let (ob, ib, kb, cb) = unsafe { (obf(), ibf(), kbf(), cbf()) };

        for iter in 0..200 {
            let inp = rng.bytes(ib);
            let key = rng.bytes(kb);
            let cst = rng.bytes(cb);
            // Half the iterations pass c == NULL to hit the default-sigma branch.
            let cptr = if iter % 2 == 0 {
                std::ptr::null()
            } else {
                cst.as_ptr()
            };
            let mut oc = vec![0u8; ob];
            let mut or = vec![0u8; ob];
            let (rc, rr) = unsafe {
                (
                    cf(oc.as_mut_ptr(), inp.as_ptr(), key.as_ptr(), cptr),
                    rf(or.as_mut_ptr(), inp.as_ptr(), key.as_ptr(), cptr),
                )
            };
            eq_i32(name, rc, rr);
            eq_bytes(&format!("{name} out (c_null={})", iter % 2 == 0), &oc, &or);
        }
        // Edge keys/inputs.
        for fill in [0x00u8, 0xff, 0x80, 0x01] {
            let inp = vec![fill; ib];
            let key = vec![fill; kb];
            let cst = vec![fill; cb];
            let mut oc = vec![0u8; ob];
            let mut or = vec![0u8; ob];
            unsafe {
                cf(oc.as_mut_ptr(), inp.as_ptr(), key.as_ptr(), cst.as_ptr());
                rf(or.as_mut_ptr(), inp.as_ptr(), key.as_ptr(), cst.as_ptr());
            }
            eq_bytes(&format!("{name} fill={fill:#04x}"), &oc, &or);
        }
    }
}

/// Row 4: keccak1600 sponge driven directly — init / xor_bytes / permute /
/// extract_bytes at both shake rates and at 0/partial/exact/multi-rate offsets.
#[test]
fn keccak1600_sponge() {
    let d = duo();
    let (initc, initr) = d.pair::<unsafe extern "C" fn(*mut u8)>("crypto_core_keccak1600_init");
    let (xorc, xorr) = d.pair::<unsafe extern "C" fn(*mut u8, *const u8, usize, usize)>(
        "crypto_core_keccak1600_xor_bytes",
    );
    let (extc, extr) = d.pair::<unsafe extern "C" fn(*const u8, *mut u8, usize, usize)>(
        "crypto_core_keccak1600_extract_bytes",
    );
    let (p24c, p24r) = d.pair::<unsafe extern "C" fn(*mut u8)>("crypto_core_keccak1600_permute_24");
    let (p12c, p12r) = d.pair::<unsafe extern "C" fn(*mut u8)>("crypto_core_keccak1600_permute_12");
    let (sbc, _) = d.pair::<unsafe extern "C" fn() -> usize>("crypto_core_keccak1600_statebytes");
    let sb = unsafe { sbc() };
    assert!(sb >= 200);

    let mut rng = Rng::new(0x4B_4543_4341_4B);
    for rate in [168usize, 136, 104, 72] {
        for &nabsorb in &[0usize, 1, rate - 1, rate, rate + 1, 2 * rate] {
            for twelve in [false, true] {
                let data = rng.bytes(nabsorb.max(1));
                let mut sc = vec![0u8; sb + 64];
                let mut sr = vec![0u8; sb + 64];
                unsafe {
                    initc(sc.as_mut_ptr());
                    initr(sr.as_mut_ptr());
                    // absorb in rate-sized chunks, permuting between
                    let mut off = 0usize;
                    while off < nabsorb {
                        let n = (nabsorb - off).min(rate);
                        xorc(sc.as_mut_ptr(), data[off..].as_ptr(), 0, n);
                        xorr(sr.as_mut_ptr(), data[off..].as_ptr(), 0, n);
                        if twelve {
                            p12c(sc.as_mut_ptr());
                            p12r(sr.as_mut_ptr());
                        } else {
                            p24c(sc.as_mut_ptr());
                            p24r(sr.as_mut_ptr());
                        }
                        off += n;
                    }
                    // whole 200-byte state must be identical
                    eq_bytes(
                        &format!("keccak state rate={rate} n={nabsorb} p12={twelve}"),
                        &sc[..200],
                        &sr[..200],
                    );
                    // squeeze at several offsets/lengths
                    for (o, l) in [
                        (0usize, 1usize),
                        (0, rate),
                        (1, rate - 1),
                        (7, 32),
                        (100, 50),
                    ] {
                        if o + l > 200 {
                            continue;
                        }
                        let mut oc = vec![0u8; l];
                        let mut or = vec![0u8; l];
                        extc(sc.as_ptr(), oc.as_mut_ptr(), o, l);
                        extr(sr.as_ptr(), or.as_mut_ptr(), o, l);
                        eq_bytes(&format!("keccak extract o={o} l={l}"), &oc, &or);
                    }
                }
            }
        }
    }
}

/// Row 5 + ERRORS.md 58: crypto_verify_16/32/64 — equal, and differing in
/// every single bit position.
#[test]
fn crypto_verify_all_bit_positions() {
    let d = duo();
    let mut rng = Rng::new(0x1234_5678);
    for n in [16usize, 32, 64] {
        let (cf, rf) = d.pair::<unsafe extern "C" fn(*const u8, *const u8) -> i32>(&format!(
            "crypto_verify_{n}"
        ));
        for _ in 0..20 {
            let a = rng.bytes(n);
            // equal
            unsafe {
                eq_i32(
                    "verify eq",
                    cf(a.as_ptr(), a.as_ptr()),
                    rf(a.as_ptr(), a.as_ptr()),
                )
            };
            // every bit flipped
            for byte in 0..n {
                for bit in 0..8 {
                    let mut b = a.clone();
                    b[byte] ^= 1 << bit;
                    let (rc, rr) =
                        unsafe { (cf(a.as_ptr(), b.as_ptr()), rf(a.as_ptr(), b.as_ptr())) };
                    eq_i32(&format!("crypto_verify_{n} byte {byte} bit {bit}"), rc, rr);
                    assert_eq!(rc, -1);
                }
            }
        }
    }
}

/// Rows 6–8: poly1305 one-shot, multipart, and verify.
#[test]
fn poly1305() {
    let d = duo();
    let mut rng = Rng::new(0x_9011_0305);
    let (onec, oner) = d.pair::<Hash4>("crypto_onetimeauth_poly1305");
    let (verc, verr) = d.pair::<Verify4>("crypto_onetimeauth_poly1305_verify");
    let (ic, ir) = d.pair::<unsafe extern "C" fn(*mut u8, *const u8) -> i32>(
        "crypto_onetimeauth_poly1305_init",
    );
    let (uc, ur) = d.pair::<unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32>(
        "crypto_onetimeauth_poly1305_update",
    );
    let (fc, fr) = d
        .pair::<unsafe extern "C" fn(*mut u8, *mut u8) -> i32>("crypto_onetimeauth_poly1305_final");
    let (sbc, _) =
        d.pair::<unsafe extern "C" fn() -> usize>("crypto_onetimeauth_poly1305_statebytes");
    let sb = unsafe { sbc() };

    for &len in LENS {
        for _ in 0..8 {
            let msg = rng.bytes(len);
            let key = rng.bytes(32);
            let mut mc = [0u8; 16];
            let mut mr = [0u8; 16];
            let (rc, rr) = unsafe {
                (
                    onec(mc.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                    oner(mr.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                )
            };
            eq_i32("poly1305", rc, rr);
            eq_bytes(&format!("poly1305 len={len}"), &mc, &mr);

            // verify: correct tag, then each byte flipped
            unsafe {
                eq_i32(
                    "poly1305_verify ok",
                    verc(mc.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                    verr(mc.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                );
            }
            for i in 0..16 {
                let mut bad = mc;
                bad[i] ^= 0x40;
                let (a, b) = unsafe {
                    (
                        verc(bad.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                        verr(bad.as_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                    )
                };
                eq_i32(&format!("poly1305_verify bad byte {i}"), a, b);
                assert_eq!(a, -1);
            }

            // multipart with random splits
            for _ in 0..4 {
                let mut cuts = vec![0usize];
                let nsplit = 1 + rng.below(6);
                for _ in 0..nsplit {
                    cuts.push(rng.below(len + 1));
                }
                cuts.push(len);
                cuts.sort_unstable();
                let mut stc = vec![0u8; sb + 64];
                let mut str_ = vec![0u8; sb + 64];
                unsafe {
                    ic(stc.as_mut_ptr(), key.as_ptr());
                    ir(str_.as_mut_ptr(), key.as_ptr());
                    for w in cuts.windows(2) {
                        let (a, b) = (w[0], w[1]);
                        uc(stc.as_mut_ptr(), msg[a..].as_ptr(), (b - a) as u64);
                        ur(str_.as_mut_ptr(), msg[a..].as_ptr(), (b - a) as u64);
                    }
                    let mut o1 = [0u8; 16];
                    let mut o2 = [0u8; 16];
                    fc(stc.as_mut_ptr(), o1.as_mut_ptr());
                    fr(str_.as_mut_ptr(), o2.as_mut_ptr());
                    eq_bytes(
                        &format!("poly1305 multipart len={len} cuts={cuts:?}"),
                        &o1,
                        &o2,
                    );
                    eq_bytes("poly1305 multipart == oneshot (C)", &o1, &mc);
                }
            }
        }
    }
}

/// Rows 9–10: siphash24 / siphashx24 across the length sweep.
#[test]
fn shorthash() {
    let d = duo();
    let mut rng = Rng::new(0x5199);
    for (name, outlen) in [
        ("crypto_shorthash_siphash24", 8usize),
        ("crypto_shorthash_siphashx24", 16),
        ("crypto_shorthash", 8),
    ] {
        let (cf, rf) = d.pair::<Hash4>(name);
        for len in (0..=72usize).chain([100, 255, 256, 1000]) {
            for _ in 0..4 {
                let msg = rng.bytes(len);
                let key = rng.bytes(16);
                let mut oc = vec![0u8; outlen];
                let mut or = vec![0u8; outlen];
                let (rc, rr) = unsafe {
                    (
                        cf(oc.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                        rf(or.as_mut_ptr(), msg.as_ptr(), len as u64, key.as_ptr()),
                    )
                };
                eq_i32(name, rc, rr);
                eq_bytes(&format!("{name} len={len}"), &oc, &or);
            }
        }
    }
}

/// Rows 11–12: curve25519 scalar multiplication, direct and base.
#[test]
fn scalarmult_curve25519() {
    let d = duo();
    let mut rng = Rng::new(0xC0FFEE);
    let (basec, baser) = d.pair::<unsafe extern "C" fn(*mut u8, *const u8) -> i32>(
        "crypto_scalarmult_curve25519_base",
    );
    let (mulc, mulr) = d.pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32>(
        "crypto_scalarmult_curve25519",
    );
    let mut pubkeys = vec![];
    for _ in 0..40 {
        let n = rng.bytes(32);
        let mut pc = [0u8; 32];
        let mut pr = [0u8; 32];
        let (rc, rr) = unsafe {
            (
                basec(pc.as_mut_ptr(), n.as_ptr()),
                baser(pr.as_mut_ptr(), n.as_ptr()),
            )
        };
        eq_i32("scalarmult_base", rc, rr);
        eq_bytes("scalarmult_curve25519_base", &pc, &pr);
        pubkeys.push(pc);
    }
    // edge scalars
    for fill in [0x00u8, 0x01, 0x7f, 0x80, 0xff] {
        let n = vec![fill; 32];
        let mut pc = [0u8; 32];
        let mut pr = [0u8; 32];
        let (rc, rr) = unsafe {
            (
                basec(pc.as_mut_ptr(), n.as_ptr()),
                baser(pr.as_mut_ptr(), n.as_ptr()),
            )
        };
        eq_i32(&format!("base fill={fill:#04x}"), rc, rr);
        eq_bytes(&format!("base fill={fill:#04x}"), &pc, &pr);
        pubkeys.push(pc);
    }
    for p in &pubkeys {
        for _ in 0..4 {
            let n = rng.bytes(32);
            let mut qc = [0u8; 32];
            let mut qr = [0u8; 32];
            let (rc, rr) = unsafe {
                (
                    mulc(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                    mulr(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()),
                )
            };
            eq_i32("crypto_scalarmult_curve25519", rc, rr);
            eq_bytes("crypto_scalarmult_curve25519 out", &qc, &qr);
        }
    }
    // generic wrappers must agree too
    let (gc, gr) =
        d.pair::<unsafe extern "C" fn(*mut u8, *const u8) -> i32>("crypto_scalarmult_base");
    let (gmc, gmr) =
        d.pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32>("crypto_scalarmult");
    for _ in 0..20 {
        let n = rng.bytes(32);
        let mut pc = [0u8; 32];
        let mut pr = [0u8; 32];
        unsafe {
            eq_i32(
                "scalarmult_base gen",
                gc(pc.as_mut_ptr(), n.as_ptr()),
                gr(pr.as_mut_ptr(), n.as_ptr()),
            );
        }
        eq_bytes("crypto_scalarmult_base", &pc, &pr);
        let n2 = rng.bytes(32);
        let mut qc = [0u8; 32];
        let mut qr = [0u8; 32];
        unsafe {
            eq_i32(
                "scalarmult gen",
                gmc(qc.as_mut_ptr(), n2.as_ptr(), pc.as_ptr()),
                gmr(qr.as_mut_ptr(), n2.as_ptr(), pr.as_ptr()),
            );
        }
        eq_bytes("crypto_scalarmult", &qc, &qr);
    }
}

/// Rows 13–19: every stream cipher, keystream / xor / xor_ic forms, `ic`
/// boundary sweep, and in-place operation.
#[test]
fn stream_ciphers() {
    let d = duo();
    let mut rng = Rng::new(0x57DEA3);

    // (name, noncebytes, keybytes, has_ic, ic_is_32bit)
    let specs: &[(&str, usize, usize, bool, bool)] = &[
        ("crypto_stream_salsa20", 8, 32, true, false),
        ("crypto_stream_salsa2012", 8, 32, false, false),
        ("crypto_stream_salsa208", 8, 32, false, false),
        ("crypto_stream_chacha20", 8, 32, true, false),
        ("crypto_stream_chacha20_ietf", 12, 32, true, true),
        ("crypto_stream_xsalsa20", 24, 32, true, false),
        ("crypto_stream_xchacha20", 24, 32, true, false),
        ("crypto_stream", 24, 32, false, false),
    ];

    for &(name, nb, kb, has_ic, ic32) in specs {
        let (ksc, ksr) = d.pair::<Stream4>(name);
        let (xc, xr) = d.pair::<StreamXor>(&format!("{name}_xor"));
        for &len in LENS {
            let key = rng.bytes(kb);
            let nonce = rng.bytes(nb);
            let msg = rng.bytes(len);

            // keystream form
            let mut kc = vec![0u8; len];
            let mut kr = vec![0u8; len];
            let (a, b) = unsafe {
                (
                    ksc(kc.as_mut_ptr(), len as u64, nonce.as_ptr(), key.as_ptr()),
                    ksr(kr.as_mut_ptr(), len as u64, nonce.as_ptr(), key.as_ptr()),
                )
            };
            eq_i32(name, a, b);
            eq_bytes(&format!("{name} keystream len={len}"), &kc, &kr);

            // xor form
            let mut oc = vec![0u8; len];
            let mut or = vec![0u8; len];
            let (a, b) = unsafe {
                (
                    xc(
                        oc.as_mut_ptr(),
                        msg.as_ptr(),
                        len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                    xr(
                        or.as_mut_ptr(),
                        msg.as_ptr(),
                        len as u64,
                        nonce.as_ptr(),
                        key.as_ptr(),
                    ),
                )
            };
            eq_i32(&format!("{name}_xor"), a, b);
            eq_bytes(&format!("{name}_xor len={len}"), &oc, &or);

            // row 19: in-place (c == m)
            let mut ipc = msg.clone();
            let mut ipr = msg.clone();
            unsafe {
                xc(
                    ipc.as_mut_ptr(),
                    ipc.as_ptr(),
                    len as u64,
                    nonce.as_ptr(),
                    key.as_ptr(),
                );
                xr(
                    ipr.as_mut_ptr(),
                    ipr.as_ptr(),
                    len as u64,
                    nonce.as_ptr(),
                    key.as_ptr(),
                );
            }
            eq_bytes(&format!("{name}_xor in-place len={len}"), &ipc, &ipr);
            eq_bytes(&format!("{name}_xor in-place == disjoint"), &ipc, &oc);

            if has_ic {
                let sym = format!("{name}_xor_ic");
                if !d.has(&sym) {
                    continue;
                }
                if ic32 {
                    let (icc, icr) = d.pair::<StreamXorIc32>(&sym);
                    let blocks = ((len as u64) + 63) / 64;
                    let maxic = (1u64 << 32).saturating_sub(blocks);
                    for ic in [0u32, 1, 2, 100, 0xffff, (maxic.saturating_sub(1)) as u32] {
                        let mut oc = vec![0u8; len];
                        let mut or = vec![0u8; len];
                        let (a, b) = unsafe {
                            (
                                icc(
                                    oc.as_mut_ptr(),
                                    msg.as_ptr(),
                                    len as u64,
                                    nonce.as_ptr(),
                                    ic,
                                    key.as_ptr(),
                                ),
                                icr(
                                    or.as_mut_ptr(),
                                    msg.as_ptr(),
                                    len as u64,
                                    nonce.as_ptr(),
                                    ic,
                                    key.as_ptr(),
                                ),
                            )
                        };
                        eq_i32(&format!("{sym} ic={ic}"), a, b);
                        eq_bytes(&format!("{sym} ic={ic} len={len}"), &oc, &or);
                    }
                } else {
                    let (icc, icr) = d.pair::<StreamXorIc64>(&sym);
                    for ic in [0u64, 1, 2, 1 << 31, 1 << 32, (1 << 32) + 1, u64::MAX - 64] {
                        let mut oc = vec![0u8; len];
                        let mut or = vec![0u8; len];
                        let (a, b) = unsafe {
                            (
                                icc(
                                    oc.as_mut_ptr(),
                                    msg.as_ptr(),
                                    len as u64,
                                    nonce.as_ptr(),
                                    ic,
                                    key.as_ptr(),
                                ),
                                icr(
                                    or.as_mut_ptr(),
                                    msg.as_ptr(),
                                    len as u64,
                                    nonce.as_ptr(),
                                    ic,
                                    key.as_ptr(),
                                ),
                            )
                        };
                        eq_i32(&format!("{sym} ic={ic}"), a, b);
                        eq_bytes(&format!("{sym} ic={ic} len={len}"), &oc, &or);
                    }
                }
            }
        }
    }
}

/// Row 16: the `_ietf_ext` extended-nonce chacha20 entry points (lowest-level
/// chacha20 API, not reachable through any convenience wrapper). Note that the
/// C `.so` exports only `crypto_stream_chacha20_ietf_ext` and
/// `..._ext_xor_ic` — `..._ext_xor` stays internal.
#[test]
fn chacha20_ietf_ext() {
    let d = duo();
    let mut rng = Rng::new(0xEE7EE7);
    let (ksc, ksr) = d.pair::<Stream4>("crypto_stream_chacha20_ietf_ext");
    let (icc, icr) = d.pair::<StreamXorIc32>("crypto_stream_chacha20_ietf_ext_xor_ic");
    for &len in LENS {
        let key = rng.bytes(32);
        let nonce = rng.bytes(16);
        let msg = rng.bytes(len);
        let mut kc = vec![0u8; len];
        let mut kr = vec![0u8; len];
        unsafe {
            eq_i32(
                "ietf_ext",
                ksc(kc.as_mut_ptr(), len as u64, nonce.as_ptr(), key.as_ptr()),
                ksr(kr.as_mut_ptr(), len as u64, nonce.as_ptr(), key.as_ptr()),
            );
        }
        eq_bytes(&format!("ietf_ext keystream len={len}"), &kc, &kr);
        for ic in [0u32, 1, 7, 0xffff, 0xffff_fffe] {
            let mut oc = vec![0u8; len];
            let mut or = vec![0u8; len];
            unsafe {
                eq_i32(
                    "ietf_ext_xor_ic",
                    icc(
                        oc.as_mut_ptr(),
                        msg.as_ptr(),
                        len as u64,
                        nonce.as_ptr(),
                        ic,
                        key.as_ptr(),
                    ),
                    icr(
                        or.as_mut_ptr(),
                        msg.as_ptr(),
                        len as u64,
                        nonce.as_ptr(),
                        ic,
                        key.as_ptr(),
                    ),
                );
            }
            eq_bytes(&format!("ietf_ext_xor_ic ic={ic} len={len}"), &oc, &or);
        }
        // in-place through the lowest-level ic entry point
        let mut ipc = msg.clone();
        let mut ipr = msg.clone();
        unsafe {
            icc(
                ipc.as_mut_ptr(),
                ipc.as_ptr(),
                len as u64,
                nonce.as_ptr(),
                3,
                key.as_ptr(),
            );
            icr(
                ipr.as_mut_ptr(),
                ipr.as_ptr(),
                len as u64,
                nonce.as_ptr(),
                3,
                key.as_ptr(),
            );
        }
        eq_bytes(&format!("ietf_ext_xor_ic in-place len={len}"), &ipc, &ipr);
    }
}
