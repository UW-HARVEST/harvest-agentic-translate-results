//! Phase B + C for `crypto_hash/` and `crypto_xof/`.
//!
//! CONFIGS rows PA94–PA142, ERRORS rows A97–A103.
//! Covers SHA-256/512 one-shot and streaming across the 64/128-byte block
//! boundaries, SHA3-256/512 across their 136/72 rates including the
//! FINALIZED-phase `ret = -1` recovery path, and all four XOFs including
//! `init_with_domain`, multi-squeeze, and absorb-after-squeeze.

mod harness;
use harness::*;

use std::ffi::c_int;

const SEED: u64 = 0x5EED_0003;
const STATE_MAX: usize = 512;

type OneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type Init = unsafe extern "C" fn(*mut u8) -> c_int;
type Update = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type Final = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;

fn statebytes(name: &str) -> usize {
    let (c, r) = sym::<unsafe extern "C" fn() -> usize>(name);
    let (cv, rv) = unsafe { (c(), r()) };
    assert_eq!(cv, rv, "{name}");
    assert!(cv <= STATE_MAX, "{name} = {cv} > STATE_MAX");
    cv
}

/// Chunkings that straddle every interesting block/rate boundary.
fn chunkings(total: usize, block: usize) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = vec![vec![total]];
    if total == 0 {
        out.push(vec![]);
        out.push(vec![0, 0]);
        return out;
    }
    out.push(vec![0; 3].into_iter().chain(std::iter::once(total)).collect());
    out.push((0..total).map(|_| 1).collect()); // byte at a time
    for &split in &[1usize, block / 2, block - 1, block, block + 1, 2 * block] {
        if split < total {
            out.push(vec![split, total - split]);
        }
    }
    // three-way split at block-1 / 1 / rest
    if total > block {
        out.push(vec![block - 1, 1, total - block]);
        out.push(vec![1, block - 1, total - block]);
    }
    out
}

struct HashSpec {
    prefix: &'static str,
    outlen: usize,
    block: usize,
}

const HASHES: &[HashSpec] = &[
    HashSpec { prefix: "crypto_hash_sha256", outlen: 32, block: 64 },
    HashSpec { prefix: "crypto_hash_sha512", outlen: 64, block: 128 },
    HashSpec { prefix: "crypto_hash_sha3256", outlen: 32, block: 136 },
    HashSpec { prefix: "crypto_hash_sha3512", outlen: 64, block: 72 },
];

#[test]
fn hash_one_shot_all_lengths() {
    let mut rng = Rng::new(SEED);
    for h in HASHES {
        let (c, r) = sym::<OneShot>(h.prefix);
        let mut lens: Vec<usize> = (0..=2 * h.block + 3).collect();
        lens.extend_from_slice(&[
            3 * h.block,
            3 * h.block + 1,
            4 * h.block - 1,
            1000,
            4096,
            4097,
        ]);
        for len in lens {
            for _ in 0..3 {
                let msg = rng.bytes(len);
                let mut oc = out_buf(h.outlen);
                let mut or = out_buf(h.outlen);
                unsafe {
                    let rc = c(oc.as_mut_ptr(), msg.as_ptr(), len as u64);
                    let rr = r(or.as_mut_ptr(), msg.as_ptr(), len as u64);
                    assert_eq!(rc, rr, "{} rc len={len}", h.prefix);
                }
                eqb(&format!("{} len={len}", h.prefix), &oc, &or);
            }
            // all-zero and all-ones messages
            for (tag, msg) in [("zeros", vec![0u8; len]), ("ones", vec![0xffu8; len])] {
                let mut oc = out_buf(h.outlen);
                let mut or = out_buf(h.outlen);
                unsafe {
                    c(oc.as_mut_ptr(), msg.as_ptr(), len as u64);
                    r(or.as_mut_ptr(), msg.as_ptr(), len as u64);
                }
                eqb(&format!("{} {tag} len={len}", h.prefix), &oc, &or);
            }
        }
    }
}

#[test]
fn hash_streaming_all_chunkings() {
    let mut rng = Rng::new(SEED ^ 1);
    for h in HASHES {
        let sb = statebytes(&format!("{}_statebytes", h.prefix));
        let (cinit, rinit) = sym::<Init>(&format!("{}_init", h.prefix));
        let (cupd, rupd) = sym::<Update>(&format!("{}_update", h.prefix));
        let (cfin, rfin) = sym::<Final>(&format!("{}_final", h.prefix));
        let (cone, _) = sym::<OneShot>(h.prefix);

        let mut lens: Vec<usize> = vec![0, 1, h.block - 1, h.block, h.block + 1];
        lens.extend_from_slice(&[2 * h.block - 1, 2 * h.block, 2 * h.block + 1, 3 * h.block + 7, 1000]);
        for len in lens {
            let msg = rng.bytes(len);
            // Reference: the C one-shot result.
            let mut refout = vec![0u8; h.outlen];
            unsafe { cone(refout.as_mut_ptr(), msg.as_ptr(), len as u64) };

            for chunks in chunkings(len, h.block) {
                if chunks.iter().sum::<usize>() != len {
                    continue;
                }
                let mut stc = vec![0xa5u8; STATE_MAX];
                let mut str_ = vec![0xa5u8; STATE_MAX];
                unsafe {
                    let ic = cinit(stc.as_mut_ptr());
                    let ir = rinit(str_.as_mut_ptr());
                    assert_eq!(ic, ir, "{}_init rc", h.prefix);
                }
                eqb(&format!("{}_init state", h.prefix), &stc[..sb], &str_[..sb]);

                let mut off = 0usize;
                for (i, &n) in chunks.iter().enumerate() {
                    unsafe {
                        let uc = cupd(stc.as_mut_ptr(), msg[off..].as_ptr(), n as u64);
                        let ur = rupd(str_.as_mut_ptr(), msg[off..].as_ptr(), n as u64);
                        assert_eq!(uc, ur, "{}_update rc len={len} chunk#{i}={n}", h.prefix);
                    }
                    eqb(
                        &format!("{}_update state len={len} chunk#{i}={n}", h.prefix),
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
                    assert_eq!(fc, fr, "{}_final rc len={len}", h.prefix);
                }
                eqb(&format!("{}_final out len={len} chunks={chunks:?}", h.prefix), &oc, &or);
                eqb(
                    &format!("{} streaming != one-shot len={len} chunks={chunks:?}", h.prefix),
                    &refout,
                    &oc[..h.outlen],
                );
                // the post-final state must match too
                eqb(&format!("{}_final state len={len}", h.prefix), &stc[..sb], &str_[..sb]);
            }
        }
    }
}

/// A97–A101: the SHA-3 FINALIZED-phase recovery paths (`ret = -1`), plus the
/// same "use after final" sequences on SHA-256/512, which have no such guard.
#[test]
fn hash_use_after_final() {
    let mut rng = Rng::new(SEED ^ 2);
    for h in HASHES {
        let sb = statebytes(&format!("{}_statebytes", h.prefix));
        let (cinit, rinit) = sym::<Init>(&format!("{}_init", h.prefix));
        let (cupd, rupd) = sym::<Update>(&format!("{}_update", h.prefix));
        let (cfin, rfin) = sym::<Final>(&format!("{}_final", h.prefix));

        for len in [0usize, 1, h.block - 1, h.block, h.block + 1, 3 * h.block] {
            let msg = rng.bytes(len.max(1));
            let mut stc = vec![0u8; STATE_MAX];
            let mut str_ = vec![0u8; STATE_MAX];
            let mut oc = out_buf(h.outlen);
            let mut or = out_buf(h.outlen);
            unsafe {
                cinit(stc.as_mut_ptr());
                rinit(str_.as_mut_ptr());
                cupd(stc.as_mut_ptr(), msg.as_ptr(), len as u64);
                rupd(str_.as_mut_ptr(), msg.as_ptr(), len as u64);
                cfin(stc.as_mut_ptr(), oc.as_mut_ptr());
                rfin(str_.as_mut_ptr(), or.as_mut_ptr());
            }
            eqb(&format!("{} first final len={len}", h.prefix), &oc, &or);

            // A100/A102: update() after final() -> ret = -1 on SHA-3.
            unsafe {
                let uc = cupd(stc.as_mut_ptr(), msg.as_ptr(), len as u64);
                let ur = rupd(str_.as_mut_ptr(), msg.as_ptr(), len as u64);
                assert_eq!(uc, ur, "{}_update after final rc len={len}", h.prefix);
            }
            eqb(&format!("{} state after post-final update len={len}", h.prefix), &stc[..sb], &str_[..sb]);

            // A101: final() again -> ret = -1 on SHA-3.
            let mut oc2 = out_buf(h.outlen);
            let mut or2 = out_buf(h.outlen);
            unsafe {
                let fc = cfin(stc.as_mut_ptr(), oc2.as_mut_ptr());
                let fr = rfin(str_.as_mut_ptr(), or2.as_mut_ptr());
                assert_eq!(fc, fr, "{}_final twice rc len={len}", h.prefix);
            }
            eqb(&format!("{} second final len={len}", h.prefix), &oc2, &or2);

            // final() straight after init(), no update at all
            let mut stc = vec![0u8; STATE_MAX];
            let mut str_ = vec![0u8; STATE_MAX];
            let mut oc = out_buf(h.outlen);
            let mut or = out_buf(h.outlen);
            unsafe {
                cinit(stc.as_mut_ptr());
                rinit(str_.as_mut_ptr());
                let fc = cfin(stc.as_mut_ptr(), oc.as_mut_ptr());
                let fr = rfin(str_.as_mut_ptr(), or.as_mut_ptr());
                assert_eq!(fc, fr, "{}_final on fresh state rc", h.prefix);
            }
            eqb(&format!("{} final on fresh state", h.prefix), &oc, &or);

            // A98/A99: update() with inlen == 0 on a fresh state
            let mut stc = vec![0u8; STATE_MAX];
            let mut str_ = vec![0u8; STATE_MAX];
            unsafe {
                cinit(stc.as_mut_ptr());
                rinit(str_.as_mut_ptr());
                let uc = cupd(stc.as_mut_ptr(), msg.as_ptr(), 0);
                let ur = rupd(str_.as_mut_ptr(), msg.as_ptr(), 0);
                assert_eq!(uc, ur, "{}_update inlen=0 rc", h.prefix);
            }
            eqb(&format!("{} update inlen=0 state", h.prefix), &stc[..sb], &str_[..sb]);
        }
    }
}

/// The generic `crypto_hash` façade (delegates to sha512).
#[test]
fn crypto_hash_generic() {
    let (c, r) = sym::<OneShot>("crypto_hash");
    let mut rng = Rng::new(SEED ^ 3);
    for len in [0usize, 1, 55, 56, 63, 64, 111, 112, 127, 128, 129, 1000] {
        let msg = rng.bytes(len);
        let mut oc = out_buf(64);
        let mut or = out_buf(64);
        unsafe {
            let rc = c(oc.as_mut_ptr(), msg.as_ptr(), len as u64);
            let rr = r(or.as_mut_ptr(), msg.as_ptr(), len as u64);
            assert_eq!(rc, rr, "crypto_hash rc len={len}");
        }
        eqb(&format!("crypto_hash len={len}"), &oc, &or);
    }
}

// ---------------------------------------------------------------------------
// crypto_xof — shake128/256, turboshake128/256 (PA118–PA142, A102–A103)
// ---------------------------------------------------------------------------

struct XofSpec {
    prefix: &'static str,
    rate: usize,
    has_domain: bool,
}

const XOFS: &[XofSpec] = &[
    XofSpec { prefix: "crypto_xof_shake128", rate: 168, has_domain: true },
    XofSpec { prefix: "crypto_xof_shake256", rate: 136, has_domain: true },
    XofSpec { prefix: "crypto_xof_turboshake128", rate: 168, has_domain: true },
    XofSpec { prefix: "crypto_xof_turboshake256", rate: 136, has_domain: true },
];

type XofOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> c_int;
type XofInitDom = unsafe extern "C" fn(*mut u8, u8) -> c_int;
type XofSqueeze = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int;

#[test]
fn xof_one_shot_all_lengths() {
    let mut rng = Rng::new(SEED ^ 4);
    for x in XOFS {
        let (c, r) = sym::<XofOneShot>(x.prefix);
        let bb = {
            let (c, r) = sym::<unsafe extern "C" fn() -> usize>(&format!("{}_blockbytes", x.prefix));
            let (a, b) = unsafe { (c(), r()) };
            assert_eq!(a, b);
            a
        };
        assert_eq!(bb, x.rate, "{} blockbytes", x.prefix);
        for inlen in [0usize, 1, x.rate - 1, x.rate, x.rate + 1, 2 * x.rate, 2 * x.rate + 1, 1000] {
            let msg = rng.bytes(inlen);
            for outlen in [0usize, 1, 31, 32, x.rate - 1, x.rate, x.rate + 1, 2 * x.rate + 5, 1000] {
                let mut oc = out_buf(outlen);
                let mut or = out_buf(outlen);
                unsafe {
                    let rc = c(oc.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64);
                    let rr = r(or.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64);
                    assert_eq!(rc, rr, "{} rc in={inlen} out={outlen}", x.prefix);
                }
                eqb(&format!("{} in={inlen} out={outlen}", x.prefix), &oc, &or);
            }
        }
    }
}

#[test]
fn xof_streaming_domains_and_multisqueeze() {
    let mut rng = Rng::new(SEED ^ 5);
    for x in XOFS {
        let sb = statebytes(&format!("{}_statebytes", x.prefix));
        let (cinit, rinit) = sym::<Init>(&format!("{}_init", x.prefix));
        let (cupd, rupd) = sym::<Update>(&format!("{}_update", x.prefix));
        let (csq, rsq) = sym::<XofSqueeze>(&format!("{}_squeeze", x.prefix));
        let dom_name = format!("{}_init_with_domain", x.prefix);
        let dom = if x.has_domain && has(&dom_name) {
            Some(sym::<XofInitDom>(&dom_name))
        } else {
            None
        };
        let std_dom = {
            let (c, r) = sym::<unsafe extern "C" fn() -> u8>(&format!("{}_domain_standard", x.prefix));
            let (a, b) = unsafe { (c(), r()) };
            assert_eq!(a, b, "{}_domain_standard", x.prefix);
            a
        };

        // PA118–PA136: init vs init_with_domain over the whole domain-byte range.
        let mut domains: Vec<Option<u8>> = vec![None];
        for d in [0u8, 1, 0x06, 0x1f, 0x7f, 0x80, std_dom, 0xfe, 0xff] {
            domains.push(Some(d));
        }
        for d in 0u8..=255 {
            if d % 17 == 0 {
                domains.push(Some(d));
            }
        }

        for dom_opt in &domains {
            if dom_opt.is_some() && dom.is_none() {
                continue;
            }
            for inlen in [0usize, 1, x.rate - 1, x.rate, x.rate + 1, 2 * x.rate + 3] {
                let msg = rng.bytes(inlen.max(1));
                for chunks in chunkings(inlen, x.rate) {
                    if chunks.iter().sum::<usize>() != inlen {
                        continue;
                    }
                    let mut stc = vec![0xa5u8; STATE_MAX];
                    let mut str_ = vec![0xa5u8; STATE_MAX];
                    unsafe {
                        let (ic, ir) = match dom_opt {
                            None => (cinit(stc.as_mut_ptr()), rinit(str_.as_mut_ptr())),
                            Some(d) => {
                                let (cd, rd) = dom.unwrap();
                                (cd(stc.as_mut_ptr(), *d), rd(str_.as_mut_ptr(), *d))
                            }
                        };
                        assert_eq!(ic, ir, "{} init rc dom={dom_opt:?}", x.prefix);
                    }
                    eqb(&format!("{} init state dom={dom_opt:?}", x.prefix), &stc[..sb], &str_[..sb]);

                    let mut off = 0;
                    for &n in &chunks {
                        unsafe {
                            let uc = cupd(stc.as_mut_ptr(), msg[off.min(msg.len())..].as_ptr(), n as u64);
                            let ur = rupd(str_.as_mut_ptr(), msg[off.min(msg.len())..].as_ptr(), n as u64);
                            assert_eq!(uc, ur, "{} update rc", x.prefix);
                        }
                        eqb(&format!("{} update state dom={dom_opt:?}", x.prefix), &stc[..sb], &str_[..sb]);
                        off += n;
                    }

                    // PA137–PA142: multiple successive squeezes straddling the rate,
                    // including a 0-byte squeeze.
                    for seq in [
                        vec![0usize],
                        vec![1],
                        vec![x.rate],
                        vec![x.rate - 1, 1],
                        vec![1, x.rate - 1, 1],
                        vec![x.rate + 1, x.rate + 1],
                        vec![0, 1, 0, 32, 0],
                        vec![7, 7, 7, 7, 7, 7, 7, 7, 7, 7],
                        vec![2 * x.rate, 3],
                    ] {
                        let mut sc2 = stc.clone();
                        let mut sr2 = str_.clone();
                        for (i, &n) in seq.iter().enumerate() {
                            let mut oc = out_buf(n);
                            let mut or = out_buf(n);
                            unsafe {
                                let qc = csq(sc2.as_mut_ptr(), oc.as_mut_ptr(), n);
                                let qr = rsq(sr2.as_mut_ptr(), or.as_mut_ptr(), n);
                                assert_eq!(qc, qr, "{} squeeze rc dom={dom_opt:?} seq={seq:?} #{i}", x.prefix);
                            }
                            eqb(
                                &format!("{} squeeze out dom={dom_opt:?} in={inlen} seq={seq:?} #{i}", x.prefix),
                                &oc,
                                &or,
                            );
                            eqb(
                                &format!("{} squeeze state dom={dom_opt:?} seq={seq:?} #{i}", x.prefix),
                                &sc2[..sb],
                                &sr2[..sb],
                            );
                        }
                        // A103: absorb after squeeze -> ret = -1
                        unsafe {
                            let uc = cupd(sc2.as_mut_ptr(), msg.as_ptr(), 4);
                            let ur = rupd(sr2.as_mut_ptr(), msg.as_ptr(), 4);
                            assert_eq!(uc, ur, "{} update-after-squeeze rc", x.prefix);
                        }
                        eqb(
                            &format!("{} state after update-after-squeeze seq={seq:?}", x.prefix),
                            &sc2[..sb],
                            &sr2[..sb],
                        );
                        // and squeezing again afterwards must still agree
                        let mut oc = out_buf(64);
                        let mut or = out_buf(64);
                        unsafe {
                            let qc = csq(sc2.as_mut_ptr(), oc.as_mut_ptr(), 64);
                            let qr = rsq(sr2.as_mut_ptr(), or.as_mut_ptr(), 64);
                            assert_eq!(qc, qr, "{} squeeze-after-reabsorb rc", x.prefix);
                        }
                        eqb(&format!("{} squeeze-after-reabsorb", x.prefix), &oc, &or);
                    }
                }
            }
        }
    }
}

/// The streaming XOF must reproduce the one-shot result for the standard domain.
#[test]
fn xof_streaming_matches_one_shot() {
    let mut rng = Rng::new(SEED ^ 6);
    for x in XOFS {
        let (cone, _) = sym::<XofOneShot>(x.prefix);
        let (cinit, rinit) = sym::<Init>(&format!("{}_init", x.prefix));
        let (cupd, rupd) = sym::<Update>(&format!("{}_update", x.prefix));
        let (csq, rsq) = sym::<XofSqueeze>(&format!("{}_squeeze", x.prefix));
        for inlen in [0usize, 1, 63, 64, x.rate - 1, x.rate, x.rate + 1, 999] {
            let msg = rng.bytes(inlen);
            for outlen in [1usize, 32, x.rate, x.rate + 13, 700] {
                let mut want = vec![0u8; outlen];
                unsafe { cone(want.as_mut_ptr(), outlen, msg.as_ptr(), inlen as u64) };
                let mut stc = vec![0u8; STATE_MAX];
                let mut str_ = vec![0u8; STATE_MAX];
                let mut oc = out_buf(outlen);
                let mut or = out_buf(outlen);
                unsafe {
                    cinit(stc.as_mut_ptr());
                    rinit(str_.as_mut_ptr());
                    cupd(stc.as_mut_ptr(), msg.as_ptr(), inlen as u64);
                    rupd(str_.as_mut_ptr(), msg.as_ptr(), inlen as u64);
                    csq(stc.as_mut_ptr(), oc.as_mut_ptr(), outlen);
                    rsq(str_.as_mut_ptr(), or.as_mut_ptr(), outlen);
                }
                eqb(&format!("{} streaming vs C one-shot in={inlen} out={outlen}", x.prefix), &want, &oc[..outlen]);
                eqb(&format!("{} streaming C vs Rust in={inlen} out={outlen}", x.prefix), &oc, &or);
            }
        }
    }
}
