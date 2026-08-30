//! Differential tests for the raw hash primitives each backend exports, the
//! NIST AES-CTR-DRBG in `app/src/rng.c` and the public `crypto_sign_*` API.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::os::raw::{c_int, c_ulong};

// ---------------------------------------------------------------------------
// lib/blake: blake256 / blake512 and their streaming interface.
// ---------------------------------------------------------------------------

/// `blakestate256` (`unsigned int h[8], s[4], t[2]; int buflen, nullt;
/// unsigned char buf[64];`).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct BlakeState256 {
    h: [u32; 8],
    s: [u32; 4],
    t: [u32; 2],
    buflen: c_int,
    nullt: c_int,
    buf: [u8; 64],
}

/// `blakestate512`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct BlakeState512 {
    h: [u64; 8],
    s: [u64; 4],
    t: [u64; 2],
    buflen: c_int,
    nullt: c_int,
    buf: [u8; 128],
}

fn as_bytes<T: Copy>(v: &T) -> &[u8] {
    unsafe { core::slice::from_raw_parts(v as *const T as *const u8, core::mem::size_of::<T>()) }
}

#[test]
fn blake_one_shot() {
    if BACKEND != "blake" {
        return;
    }
    let l = libs();
    type FnHash = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
    let mut rng = Rng::new(0xb001);
    for (name, outlen) in [("blake256", 32usize), ("blake512", 64)] {
        let c = unsafe { l.c_backend::<FnHash>(name) };
        let r = unsafe { l.r::<FnHash>(name) };
        for inlen in [0usize, 1, 54, 55, 56, 63, 64, 65, 111, 112, 113, 127, 128, 200, 500] {
            let inp = rng.vec(inlen);
            let mut co = vec![0xAAu8; outlen + 8];
            let mut ro = vec![0xAAu8; outlen + 8];
            let (cr, rr) = unsafe {
                (
                    c(co.as_mut_ptr(), inp.as_ptr(), inlen as u64),
                    r(ro.as_mut_ptr(), inp.as_ptr(), inlen as u64),
                )
            };
            assert_eq!(cr, rr, "{name} return value (inlen={inlen})");
            assert_bytes_eq(&format!("{name}(inlen={inlen})"), &co, &ro);
        }
    }
}

#[test]
fn blake_streaming() {
    if BACKEND != "blake" {
        return;
    }
    let l = libs();
    let mut rng = Rng::new(0xb002);

    // BLAKE-256
    {
        type FnInit = unsafe extern "C" fn(*mut BlakeState256);
        type FnUpdate = unsafe extern "C" fn(*mut BlakeState256, *const u8, u64);
        type FnFinal = unsafe extern "C" fn(*mut BlakeState256, *mut u8);
        type FnCompress = unsafe extern "C" fn(*mut BlakeState256, *const u8);
        let (ci, ri) = unsafe {
            (
                l.c_backend::<FnInit>("blake256_init"),
                l.r::<FnInit>("blake256_init"),
            )
        };
        let (cu, ru) = unsafe {
            (
                l.c_backend::<FnUpdate>("blake256_update"),
                l.r::<FnUpdate>("blake256_update"),
            )
        };
        let (cf, rf) = unsafe {
            (
                l.c_backend::<FnFinal>("blake256_final"),
                l.r::<FnFinal>("blake256_final"),
            )
        };
        let (cx, rx) = unsafe {
            (
                l.c_backend::<FnCompress>("blake256_compress"),
                l.r::<FnCompress>("blake256_compress"),
            )
        };

        let zero = BlakeState256 {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 64],
        };
        let mut cs = zero;
        let mut rs = zero;
        unsafe {
            ci(&mut cs);
            ri(&mut rs);
        }
        assert_bytes_eq("blake256_init state", as_bytes(&cs), as_bytes(&rs));

        // A raw compress on a random block and a random state.
        let mut cs2 = cs;
        let mut rs2 = rs;
        let block = rng.vec(64);
        unsafe {
            cx(&mut cs2, block.as_ptr());
            rx(&mut rs2, block.as_ptr());
        }
        assert_bytes_eq("blake256_compress state", as_bytes(&cs2), as_bytes(&rs2));

        // Multi-chunk update.  `hash_blake.c` passes byte counts here even
        // though the reference code expects bits, so both flavours are tried.
        for scale in [1u64, 8] {
            for chunks in [
                vec![1usize],
                vec![0, 7, 64],
                vec![55, 1],
                vec![64, 64],
                vec![63, 2, 130],
                vec![100, 100, 100],
            ] {
                let mut cs = zero;
                let mut rs = zero;
                unsafe {
                    ci(&mut cs);
                    ri(&mut rs);
                }
                for &n in &chunks {
                    let data = rng.vec(n.max(1));
                    unsafe {
                        cu(&mut cs, data.as_ptr(), n as u64 * scale);
                        ru(&mut rs, data.as_ptr(), n as u64 * scale);
                    }
                    assert_bytes_eq(
                        &format!("blake256_update state (scale={scale}, chunks={chunks:?})"),
                        as_bytes(&cs),
                        as_bytes(&rs),
                    );
                }
                let mut cd = vec![0xAAu8; 40];
                let mut rd = vec![0xAAu8; 40];
                unsafe {
                    cf(&mut cs, cd.as_mut_ptr());
                    rf(&mut rs, rd.as_mut_ptr());
                }
                assert_bytes_eq(
                    &format!("blake256_final digest (scale={scale}, chunks={chunks:?})"),
                    &cd,
                    &rd,
                );
                assert_bytes_eq(
                    &format!("blake256_final state (scale={scale}, chunks={chunks:?})"),
                    as_bytes(&cs),
                    as_bytes(&rs),
                );
            }
        }
    }

    // BLAKE-512
    {
        type FnInit = unsafe extern "C" fn(*mut BlakeState512);
        type FnUpdate = unsafe extern "C" fn(*mut BlakeState512, *const u8, u64);
        type FnFinal = unsafe extern "C" fn(*mut BlakeState512, *mut u8);
        type FnCompress = unsafe extern "C" fn(*mut BlakeState512, *const u8);
        let (ci, ri) = unsafe {
            (
                l.c_backend::<FnInit>("blake512_init"),
                l.r::<FnInit>("blake512_init"),
            )
        };
        let (cu, ru) = unsafe {
            (
                l.c_backend::<FnUpdate>("blake512_update"),
                l.r::<FnUpdate>("blake512_update"),
            )
        };
        let (cf, rf) = unsafe {
            (
                l.c_backend::<FnFinal>("blake512_final"),
                l.r::<FnFinal>("blake512_final"),
            )
        };
        let (cx, rx) = unsafe {
            (
                l.c_backend::<FnCompress>("blake512_compress"),
                l.r::<FnCompress>("blake512_compress"),
            )
        };
        let zero = BlakeState512 {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 128],
        };
        let mut cs = zero;
        let mut rs = zero;
        unsafe {
            ci(&mut cs);
            ri(&mut rs);
        }
        assert_bytes_eq("blake512_init state", as_bytes(&cs), as_bytes(&rs));

        let mut cs2 = cs;
        let mut rs2 = rs;
        let block = rng.vec(128);
        unsafe {
            cx(&mut cs2, block.as_ptr());
            rx(&mut rs2, block.as_ptr());
        }
        assert_bytes_eq("blake512_compress state", as_bytes(&cs2), as_bytes(&rs2));

        for scale in [1u64, 8] {
            for chunks in [
                vec![1usize],
                vec![0, 7, 128],
                vec![110, 1],
                vec![128, 128],
                vec![127, 2, 260],
                vec![200, 200],
            ] {
                let mut cs = zero;
                let mut rs = zero;
                unsafe {
                    ci(&mut cs);
                    ri(&mut rs);
                }
                for &n in &chunks {
                    let data = rng.vec(n.max(1));
                    unsafe {
                        cu(&mut cs, data.as_ptr(), n as u64 * scale);
                        ru(&mut rs, data.as_ptr(), n as u64 * scale);
                    }
                    assert_bytes_eq(
                        &format!("blake512_update state (scale={scale}, chunks={chunks:?})"),
                        as_bytes(&cs),
                        as_bytes(&rs),
                    );
                }
                let mut cd = vec![0xAAu8; 72];
                let mut rd = vec![0xAAu8; 72];
                unsafe {
                    cf(&mut cs, cd.as_mut_ptr());
                    rf(&mut rs, rd.as_mut_ptr());
                }
                assert_bytes_eq(
                    &format!("blake512_final digest (scale={scale}, chunks={chunks:?})"),
                    &cd,
                    &rd,
                );
            }
        }
    }
}

/// `blake512.c` defines `cst` without `static`, so it is an exported data
/// symbol; the tables must be byte-identical.
#[test]
fn blake_cst_table() {
    if BACKEND != "blake" {
        return;
    }
    let l = libs();
    let c = unsafe { l.c_backend::<*const u64>("cst") };
    let r = unsafe { l.r::<*const u64>("cst") };
    let cv = unsafe { core::slice::from_raw_parts(*c as *const u8, 16 * 8) };
    let rv = unsafe { core::slice::from_raw_parts(*r as *const u8, 16 * 8) };
    assert_bytes_eq("cst table", cv, rv);
}

// ---------------------------------------------------------------------------
// lib/sha2
// ---------------------------------------------------------------------------

#[test]
fn sha2_primitives() {
    if BACKEND != "sha2" {
        return;
    }
    let l = libs();
    type FnHash = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type FnIncInit = unsafe extern "C" fn(*mut u8);
    type FnIncBlocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type FnIncFinal = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);

    let mut rng = Rng::new(0xc001);
    // (name, output bytes, state bytes, block bytes)
    for (n, outlen, statelen, blocklen) in
        [("sha256", 32usize, 40usize, 64usize), ("sha512", 64, 72, 128)]
    {
        let c = unsafe { l.c_backend::<FnHash>(n) };
        let r = unsafe { l.r::<FnHash>(n) };
        for inlen in [0usize, 1, 55, 56, 63, 64, 65, 111, 112, 119, 120, 127, 128, 300] {
            let inp = rng.vec(inlen);
            let mut co = vec![0xAAu8; outlen + 8];
            let mut ro = vec![0xAAu8; outlen + 8];
            unsafe {
                c(co.as_mut_ptr(), inp.as_ptr(), inlen);
                r(ro.as_mut_ptr(), inp.as_ptr(), inlen);
            }
            assert_bytes_eq(&format!("{n}(inlen={inlen})"), &co, &ro);
        }

        let (ci, ri) = unsafe {
            (
                l.c_backend::<FnIncInit>(&format!("{n}_inc_init")),
                l.r::<FnIncInit>(&format!("{n}_inc_init")),
            )
        };
        let (cb, rb) = unsafe {
            (
                l.c_backend::<FnIncBlocks>(&format!("{n}_inc_blocks")),
                l.r::<FnIncBlocks>(&format!("{n}_inc_blocks")),
            )
        };
        let (cfin, rfin) = unsafe {
            (
                l.c_backend::<FnIncFinal>(&format!("{n}_inc_finalize")),
                l.r::<FnIncFinal>(&format!("{n}_inc_finalize")),
            )
        };

        for nblocks in [0usize, 1, 2, 3] {
            for taillen in [0usize, 1, 17, blocklen - 1, blocklen, blocklen + 5] {
                let mut cs = vec![0xAAu8; statelen + 8];
                let mut rs = vec![0xAAu8; statelen + 8];
                unsafe {
                    ci(cs.as_mut_ptr());
                    ri(rs.as_mut_ptr());
                }
                assert_bytes_eq(&format!("{n}_inc_init state"), &cs, &rs);

                let blocks = rng.vec((nblocks * blocklen).max(1));
                unsafe {
                    cb(cs.as_mut_ptr(), blocks.as_ptr(), nblocks);
                    rb(rs.as_mut_ptr(), blocks.as_ptr(), nblocks);
                }
                assert_bytes_eq(
                    &format!("{n}_inc_blocks state (nblocks={nblocks})"),
                    &cs,
                    &rs,
                );

                let tail = rng.vec(taillen.max(1));
                let mut co = vec![0xAAu8; outlen + 8];
                let mut ro = vec![0xAAu8; outlen + 8];
                unsafe {
                    cfin(co.as_mut_ptr(), cs.as_mut_ptr(), tail.as_ptr(), taillen);
                    rfin(ro.as_mut_ptr(), rs.as_mut_ptr(), tail.as_ptr(), taillen);
                }
                assert_bytes_eq(
                    &format!("{n}_inc_finalize out (nblocks={nblocks}, taillen={taillen})"),
                    &co,
                    &ro,
                );
                assert_bytes_eq(
                    &format!("{n}_inc_finalize state (nblocks={nblocks}, taillen={taillen})"),
                    &cs,
                    &rs,
                );
            }
        }
    }
}

/// `SPX_seed_state` fills the `state_seeded[_512]` members of `spx_ctx`.
#[test]
fn sha2_seed_state() {
    if BACKEND != "sha2" {
        return;
    }
    let l = libs();
    type FnSeedState = unsafe extern "C" fn(*mut u8);
    let c = unsafe { l.c_backend::<FnSeedState>("SPX_seed_state") };
    let r = unsafe { l.r::<FnSeedState>("SPX_seed_state") };
    for tag in [0u8, 0x5a, 0xff] {
        let mut cc = Ctx::seeded(tag);
        let mut rc = Ctx::seeded(tag);
        unsafe {
            c(cc.as_mut_ptr());
            r(rc.as_mut_ptr());
        }
        assert_bytes_eq(&format!("seed_state ctx (tag={tag})"), &cc.bytes, &rc.bytes);
    }
}

// ---------------------------------------------------------------------------
// lib/shake
// ---------------------------------------------------------------------------

#[test]
fn shake_primitives() {
    if BACKEND != "shake" {
        return;
    }
    let l = libs();
    const RATE: usize = 136;
    type FnShake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
    type FnAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type FnSqueezeBlocks = unsafe extern "C" fn(*mut u8, usize, *mut u64);
    type FnIncInit = unsafe extern "C" fn(*mut u64);
    type FnIncAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type FnIncFinalize = unsafe extern "C" fn(*mut u64);
    type FnIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);

    let mut rng = Rng::new(0xd001);

    let c = unsafe { l.c_backend::<FnShake>("shake256") };
    let r = unsafe { l.r::<FnShake>("shake256") };
    for inlen in [0usize, 1, 135, 136, 137, 271, 272, 273, 500] {
        for outlen in [1usize, 31, 32, 135, 136, 137, 300] {
            let inp = rng.vec(inlen.max(1));
            let mut co = vec![0xAAu8; outlen + 8];
            let mut ro = vec![0xAAu8; outlen + 8];
            unsafe {
                c(co.as_mut_ptr(), outlen, inp.as_ptr(), inlen);
                r(ro.as_mut_ptr(), outlen, inp.as_ptr(), inlen);
            }
            assert_bytes_eq(&format!("shake256(inlen={inlen}, outlen={outlen})"), &co, &ro);
        }
    }

    // shake256_absorb expects a full-rate-multiple input, then squeezeblocks.
    let (ca, ra) = unsafe {
        (
            l.c_backend::<FnAbsorb>("shake256_absorb"),
            l.r::<FnAbsorb>("shake256_absorb"),
        )
    };
    let (cq, rq) = unsafe {
        (
            l.c_backend::<FnSqueezeBlocks>("shake256_squeezeblocks"),
            l.r::<FnSqueezeBlocks>("shake256_squeezeblocks"),
        )
    };
    for inlen in [0usize, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 300] {
        let inp = rng.vec(inlen.max(1));
        let mut cs = [0u64; 25];
        let mut rs = [0u64; 25];
        unsafe {
            ca(cs.as_mut_ptr(), inp.as_ptr(), inlen);
            ra(rs.as_mut_ptr(), inp.as_ptr(), inlen);
        }
        assert_bytes_eq(
            &format!("shake256_absorb state (inlen={inlen})"),
            as_bytes(&cs),
            as_bytes(&rs),
        );
        for nblocks in [1usize, 2, 3] {
            let mut cs2 = cs;
            let mut rs2 = rs;
            let mut co = vec![0xAAu8; nblocks * RATE + 8];
            let mut ro = vec![0xAAu8; nblocks * RATE + 8];
            unsafe {
                cq(co.as_mut_ptr(), nblocks, cs2.as_mut_ptr());
                rq(ro.as_mut_ptr(), nblocks, rs2.as_mut_ptr());
            }
            assert_bytes_eq(
                &format!("shake256_squeezeblocks out (inlen={inlen}, nblocks={nblocks})"),
                &co,
                &ro,
            );
            assert_bytes_eq(
                &format!("shake256_squeezeblocks state (inlen={inlen}, nblocks={nblocks})"),
                as_bytes(&cs2),
                as_bytes(&rs2),
            );
        }
    }

    // Incremental API (26 uint64_t of state).
    let (ci, ri) = unsafe {
        (
            l.c_backend::<FnIncInit>("shake256_inc_init"),
            l.r::<FnIncInit>("shake256_inc_init"),
        )
    };
    let (cab, rab) = unsafe {
        (
            l.c_backend::<FnIncAbsorb>("shake256_inc_absorb"),
            l.r::<FnIncAbsorb>("shake256_inc_absorb"),
        )
    };
    let (cfi, rfi) = unsafe {
        (
            l.c_backend::<FnIncFinalize>("shake256_inc_finalize"),
            l.r::<FnIncFinalize>("shake256_inc_finalize"),
        )
    };
    let (csq, rsq) = unsafe {
        (
            l.c_backend::<FnIncSqueeze>("shake256_inc_squeeze"),
            l.r::<FnIncSqueeze>("shake256_inc_squeeze"),
        )
    };
    for chunks in [
        vec![0usize],
        vec![1],
        vec![RATE - 1, 1],
        vec![RATE, RATE],
        vec![7, 200, 3],
        vec![135, 2, 300],
    ] {
        let mut cs = [0u64; 26];
        let mut rs = [0u64; 26];
        unsafe {
            ci(cs.as_mut_ptr());
            ri(rs.as_mut_ptr());
        }
        assert_bytes_eq("shake256_inc_init state", as_bytes(&cs), as_bytes(&rs));
        for &n in &chunks {
            let data = rng.vec(n.max(1));
            unsafe {
                cab(cs.as_mut_ptr(), data.as_ptr(), n);
                rab(rs.as_mut_ptr(), data.as_ptr(), n);
            }
            assert_bytes_eq(
                &format!("shake256_inc_absorb state (chunks={chunks:?})"),
                as_bytes(&cs),
                as_bytes(&rs),
            );
        }
        unsafe {
            cfi(cs.as_mut_ptr());
            rfi(rs.as_mut_ptr());
        }
        assert_bytes_eq(
            &format!("shake256_inc_finalize state (chunks={chunks:?})"),
            as_bytes(&cs),
            as_bytes(&rs),
        );
        for outlen in [1usize, 30, RATE, RATE + 7, 400] {
            let mut cs2 = cs;
            let mut rs2 = rs;
            let mut co = vec![0xAAu8; outlen + 8];
            let mut ro = vec![0xAAu8; outlen + 8];
            unsafe {
                csq(co.as_mut_ptr(), outlen, cs2.as_mut_ptr());
                rsq(ro.as_mut_ptr(), outlen, rs2.as_mut_ptr());
            }
            assert_bytes_eq(
                &format!("shake256_inc_squeeze out (chunks={chunks:?}, outlen={outlen})"),
                &co,
                &ro,
            );
            assert_bytes_eq(
                &format!("shake256_inc_squeeze state (chunks={chunks:?}, outlen={outlen})"),
                as_bytes(&cs2),
                as_bytes(&rs2),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// lib/haraka
// ---------------------------------------------------------------------------

#[test]
fn haraka_primitives() {
    if BACKEND != "haraka" {
        return;
    }
    let l = libs();
    type FnTweak = unsafe extern "C" fn(*mut u8);
    type FnPerm = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
    type FnS = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8);
    type FnIncInit = unsafe extern "C" fn(*mut u8);
    type FnIncAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8);
    type FnIncFinalize = unsafe extern "C" fn(*mut u8);
    type FnIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const u8);

    // tweak_constants is what initialize_hash_function calls; check it fills
    // the round-constant tables in spx_ctx identically.
    let ct = unsafe { l.c_backend::<FnTweak>("SPX_tweak_constants") };
    let rt = unsafe { l.r::<FnTweak>("SPX_tweak_constants") };
    for tag in [0u8, 0x5a, 0xff] {
        let mut cc = Ctx::seeded(tag);
        let mut rc = Ctx::seeded(tag);
        unsafe {
            ct(cc.as_mut_ptr());
            rt(rc.as_mut_ptr());
        }
        assert_bytes_eq(
            &format!("tweak_constants ctx (tag={tag})"),
            &cc.bytes,
            &rc.bytes,
        );
    }

    let mut cc = Ctx::seeded(0x3c);
    let mut rc = Ctx::seeded(0x3c);
    unsafe {
        ct(cc.as_mut_ptr());
        rt(rc.as_mut_ptr());
    }

    let mut rng = Rng::new(0xe001);

    // haraka512 / haraka512_perm take a 64-byte input; haraka256 a 32-byte one.
    for (name, inlen, outlen) in [
        ("SPX_haraka512", 64usize, 32usize),
        ("SPX_haraka512_perm", 64, 64),
        ("SPX_haraka256", 32, 32),
    ] {
        let c = unsafe { l.c_backend::<FnPerm>(name) };
        let r = unsafe { l.r::<FnPerm>(name) };
        for _ in 0..8 {
            let inp = rng.vec(inlen);
            let mut co = vec![0xAAu8; outlen + 8];
            let mut ro = vec![0xAAu8; outlen + 8];
            unsafe {
                c(co.as_mut_ptr(), inp.as_ptr(), cc.as_ptr());
                r(ro.as_mut_ptr(), inp.as_ptr(), rc.as_ptr());
            }
            assert_bytes_eq(name, &co, &ro);
        }
    }

    // The Haraka sponge, one-shot and incremental (65 bytes of state).
    let cs_ = unsafe { l.c_backend::<FnS>("SPX_haraka_S") };
    let rs_ = unsafe { l.r::<FnS>("SPX_haraka_S") };
    for inlen in [0usize, 1, 31, 32, 33, 63, 64, 65, 200] {
        for outlen in [1usize, 16, 31, 32, 33, 100] {
            let inp = rng.vec(inlen.max(1));
            let mut co = vec![0xAAu8; outlen + 8];
            let mut ro = vec![0xAAu8; outlen + 8];
            unsafe {
                cs_(
                    co.as_mut_ptr(),
                    outlen as u64,
                    inp.as_ptr(),
                    inlen as u64,
                    cc.as_ptr(),
                );
                rs_(
                    ro.as_mut_ptr(),
                    outlen as u64,
                    inp.as_ptr(),
                    inlen as u64,
                    rc.as_ptr(),
                );
            }
            assert_bytes_eq(&format!("haraka_S(inlen={inlen}, outlen={outlen})"), &co, &ro);
        }
    }

    let (ci, ri) = unsafe {
        (
            l.c_backend::<FnIncInit>("SPX_haraka_S_inc_init"),
            l.r::<FnIncInit>("SPX_haraka_S_inc_init"),
        )
    };
    let (cab, rab) = unsafe {
        (
            l.c_backend::<FnIncAbsorb>("SPX_haraka_S_inc_absorb"),
            l.r::<FnIncAbsorb>("SPX_haraka_S_inc_absorb"),
        )
    };
    let (cfi, rfi) = unsafe {
        (
            l.c_backend::<FnIncFinalize>("SPX_haraka_S_inc_finalize"),
            l.r::<FnIncFinalize>("SPX_haraka_S_inc_finalize"),
        )
    };
    let (csq, rsq) = unsafe {
        (
            l.c_backend::<FnIncSqueeze>("SPX_haraka_S_inc_squeeze"),
            l.r::<FnIncSqueeze>("SPX_haraka_S_inc_squeeze"),
        )
    };
    for chunks in [
        vec![0usize],
        vec![1],
        vec![31, 1],
        vec![32, 32],
        vec![7, 100, 3],
        vec![63, 2, 200],
    ] {
        let mut cs = vec![0xAAu8; 65 + 8];
        let mut rs = vec![0xAAu8; 65 + 8];
        unsafe {
            ci(cs.as_mut_ptr());
            ri(rs.as_mut_ptr());
        }
        assert_bytes_eq("haraka_S_inc_init state", &cs, &rs);
        for &n in &chunks {
            let data = rng.vec(n.max(1));
            unsafe {
                cab(cs.as_mut_ptr(), data.as_ptr(), n, cc.as_ptr());
                rab(rs.as_mut_ptr(), data.as_ptr(), n, rc.as_ptr());
            }
            assert_bytes_eq(
                &format!("haraka_S_inc_absorb state (chunks={chunks:?})"),
                &cs,
                &rs,
            );
        }
        unsafe {
            cfi(cs.as_mut_ptr());
            rfi(rs.as_mut_ptr());
        }
        assert_bytes_eq(
            &format!("haraka_S_inc_finalize state (chunks={chunks:?})"),
            &cs,
            &rs,
        );
        for outlen in [1usize, 16, 32, 39, 100] {
            let mut cs2 = cs.clone();
            let mut rs2 = rs.clone();
            let mut co = vec![0xAAu8; outlen + 8];
            let mut ro = vec![0xAAu8; outlen + 8];
            unsafe {
                csq(co.as_mut_ptr(), outlen, cs2.as_mut_ptr(), cc.as_ptr());
                rsq(ro.as_mut_ptr(), outlen, rs2.as_mut_ptr(), rc.as_ptr());
            }
            assert_bytes_eq(
                &format!("haraka_S_inc_squeeze out (chunks={chunks:?}, outlen={outlen})"),
                &co,
                &ro,
            );
            assert_bytes_eq(
                &format!("haraka_S_inc_squeeze state (chunks={chunks:?}, outlen={outlen})"),
                &cs2,
                &rs2,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// app/src/rng.c: the NIST AES-CTR-DRBG.
// ---------------------------------------------------------------------------

type FnAes256Ecb = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type FnDrbgUpdate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type FnRandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);
type FnRandombytes = unsafe extern "C" fn(*mut u8, u64) -> c_int;
type FnSeedexpanderInit =
    unsafe extern "C" fn(*mut AesXofStruct, *mut u8, *mut u8, c_ulong) -> c_int;
type FnSeedexpander = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, c_ulong) -> c_int;

#[test]
fn aes256_ecb() {
    let l = libs();
    let c = unsafe { l.c::<FnAes256Ecb>("AES256_ECB") };
    let r = unsafe { l.r::<FnAes256Ecb>("AES256_ECB") };
    let mut rng = Rng::new(0xf001);
    for _ in 0..32 {
        let mut key = rng.vec(32);
        let mut ctr = rng.vec(16);
        let mut co = vec![0xAAu8; 24];
        let mut ro = vec![0xAAu8; 24];
        unsafe {
            c(key.as_mut_ptr(), ctr.as_mut_ptr(), co.as_mut_ptr());
            r(key.as_mut_ptr(), ctr.as_mut_ptr(), ro.as_mut_ptr());
        }
        assert_bytes_eq("AES256_ECB", &co, &ro);
    }
}

#[test]
fn aes256_ctr_drbg_update() {
    let l = libs();
    let c = unsafe { l.c::<FnDrbgUpdate>("AES256_CTR_DRBG_Update") };
    let r = unsafe { l.r::<FnDrbgUpdate>("AES256_CTR_DRBG_Update") };
    let mut rng = Rng::new(0xf002);
    for with_data in [false, true] {
        for _ in 0..8 {
            let mut data = rng.vec(48);
            let key0 = rng.vec(32);
            let v0 = rng.vec(16);
            let mut ck = key0.clone();
            let mut cv = v0.clone();
            let mut rk = key0.clone();
            let mut rv = v0.clone();
            let dptr = if with_data {
                data.as_mut_ptr()
            } else {
                core::ptr::null_mut()
            };
            unsafe {
                c(dptr, ck.as_mut_ptr(), cv.as_mut_ptr());
                r(dptr, rk.as_mut_ptr(), rv.as_mut_ptr());
            }
            assert_bytes_eq(&format!("DRBG_Update Key (data={with_data})"), &ck, &rk);
            assert_bytes_eq(&format!("DRBG_Update V (data={with_data})"), &cv, &rv);
        }
    }
}

/// `randombytes_init` + a sequence of `randombytes` calls, plus the exported
/// `DRBG_ctx` state after each step.
#[test]
fn drbg_randombytes() {
    if URANDOM {
        // With the `urandom` feature the Rust cdylib exports the
        // `randombytes.c` implementation, matching `libsphincs_core.so`
        // instead of `libsphincs_core_det.so`; see tests/ffi_urandom.rs.
        return;
    }
    let _guard = drbg_lock();
    let l = libs();
    let cinit = unsafe { l.c::<FnRandombytesInit>("randombytes_init") };
    let rinit = unsafe { l.r::<FnRandombytesInit>("randombytes_init") };
    let crb = unsafe { l.c::<FnRandombytes>("randombytes") };
    let rrb = unsafe { l.r::<FnRandombytes>("randombytes") };
    let cctx = unsafe { l.c::<*mut Drbg>("DRBG_ctx") };
    let rctx = unsafe { l.r::<*mut Drbg>("DRBG_ctx") };

    let mut rng = Rng::new(0xf003);
    for with_pers in [false, true] {
        let mut entropy = rng.vec(48);
        let mut pers = rng.vec(48);
        let pptr = if with_pers {
            pers.as_mut_ptr()
        } else {
            core::ptr::null_mut()
        };
        unsafe {
            cinit(entropy.as_mut_ptr(), pptr);
            rinit(entropy.as_mut_ptr(), pptr);
        }
        let (cs, rs) = unsafe { (**cctx, **rctx) };
        assert_bytes_eq(
            &format!("DRBG_ctx after randombytes_init (pers={with_pers})"),
            as_bytes(&cs),
            as_bytes(&rs),
        );

        for xlen in [0u64, 1, 15, 16, 17, 32, 48, 100, 1000] {
            let mut co = vec![0xAAu8; xlen as usize + 8];
            let mut ro = vec![0xAAu8; xlen as usize + 8];
            let (cr, rr) = unsafe { (crb(co.as_mut_ptr(), xlen), rrb(ro.as_mut_ptr(), xlen)) };
            assert_eq!(cr, rr, "randombytes return value (xlen={xlen})");
            assert_bytes_eq(&format!("randombytes(xlen={xlen})"), &co, &ro);
            let (cs, rs) = unsafe { (**cctx, **rctx) };
            assert_bytes_eq(
                &format!("DRBG_ctx after randombytes(xlen={xlen})"),
                as_bytes(&cs),
                as_bytes(&rs),
            );
        }
    }
}

#[test]
fn seedexpander() {
    let l = libs();
    let cinit = unsafe { l.c::<FnSeedexpanderInit>("seedexpander_init") };
    let rinit = unsafe { l.r::<FnSeedexpanderInit>("seedexpander_init") };
    let cse = unsafe { l.c::<FnSeedexpander>("seedexpander") };
    let rse = unsafe { l.r::<FnSeedexpander>("seedexpander") };

    let mut rng = Rng::new(0xf004);

    // The maxlen guard.
    {
        let mut seed = rng.vec(32);
        let mut div = rng.vec(8);
        let mut cctx = AesXofStruct::zeroed();
        let mut rctx = AesXofStruct::zeroed();
        let (cr, rr) = unsafe {
            (
                cinit(&mut cctx, seed.as_mut_ptr(), div.as_mut_ptr(), 0x1_0000_0000),
                rinit(&mut rctx, seed.as_mut_ptr(), div.as_mut_ptr(), 0x1_0000_0000),
            )
        };
        assert_eq!(cr, rr, "seedexpander_init(maxlen too large) return value");
        assert_eq!(cr, -1, "expected RNG_BAD_MAXLEN");
    }

    for maxlen in [1u64, 100, 0xffff, 0x00ff_ffff, 0xffff_ffff] {
        let mut seed = rng.vec(32);
        let mut div = rng.vec(8);
        let mut cctx = AesXofStruct::zeroed();
        let mut rctx = AesXofStruct::zeroed();
        let (cr, rr) = unsafe {
            (
                cinit(&mut cctx, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen as c_ulong),
                rinit(&mut rctx, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen as c_ulong),
            )
        };
        assert_eq!(cr, rr, "seedexpander_init return value (maxlen={maxlen})");
        assert_bytes_eq(
            &format!("seedexpander_init ctx (maxlen={maxlen})"),
            as_bytes(&cctx),
            as_bytes(&rctx),
        );

        // A NULL output buffer must be rejected identically.
        let (cr, rr) = unsafe {
            (
                cse(&mut cctx, core::ptr::null_mut(), 1),
                rse(&mut rctx, core::ptr::null_mut(), 1),
            )
        };
        assert_eq!(cr, rr, "seedexpander(NULL) return value");
        assert_eq!(cr, -2, "expected RNG_BAD_OUTBUF");

        for xlen in [0u64, 1, 15, 16, 17, 33, 64, 200] {
            let mut co = vec![0xAAu8; xlen as usize + 8];
            let mut ro = vec![0xAAu8; xlen as usize + 8];
            let (cr, rr) = unsafe {
                (
                    cse(&mut cctx, co.as_mut_ptr(), xlen as c_ulong),
                    rse(&mut rctx, ro.as_mut_ptr(), xlen as c_ulong),
                )
            };
            assert_eq!(
                cr, rr,
                "seedexpander return value (maxlen={maxlen}, xlen={xlen})"
            );
            assert_bytes_eq(
                &format!("seedexpander out (maxlen={maxlen}, xlen={xlen})"),
                &co,
                &ro,
            );
            assert_bytes_eq(
                &format!("seedexpander ctx (maxlen={maxlen}, xlen={xlen})"),
                as_bytes(&cctx),
                as_bytes(&rctx),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// app/src/sign.c: the public API.
// ---------------------------------------------------------------------------

type FnSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type FnSignature = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> c_int;
type FnVerify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int;
type FnSign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type FnSignOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;

fn keypair(seed: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let l = libs();
    let c = unsafe { l.c::<FnSeedKeypair>("crypto_sign_seed_keypair") };
    let r = unsafe { l.r::<FnSeedKeypair>("crypto_sign_seed_keypair") };
    let mut cpk = vec![0xAAu8; SPX_PK_BYTES + 8];
    let mut csk = vec![0xAAu8; SPX_SK_BYTES + 8];
    let mut rpk = vec![0xAAu8; SPX_PK_BYTES + 8];
    let mut rsk = vec![0xAAu8; SPX_SK_BYTES + 8];
    let (cr, rr) = unsafe {
        (
            c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()),
            r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()),
        )
    };
    assert_eq!(cr, rr, "crypto_sign_seed_keypair return value");
    assert_bytes_eq("crypto_sign_seed_keypair pk", &cpk, &rpk);
    assert_bytes_eq("crypto_sign_seed_keypair sk", &csk, &rsk);
    (cpk, csk, rpk, rsk)
}

#[test]
fn crypto_sign_seed_keypair() {
    let mut rng = Rng::new(0x1001);
    for round in 0..3 {
        let seed = if round == 0 {
            vec![0u8; 3 * SPX_N]
        } else {
            rng.vec(3 * SPX_N)
        };
        keypair(&seed);
    }
}

#[test]
fn crypto_sign_signature_and_verify() {
    if URANDOM {
        // `crypto_sign_signature` draws `optrand` from `randombytes`, which is
        // not the DRBG in this configuration, so the two signatures cannot be
        // compared byte for byte.  tests/ffi_urandom.rs covers this build.
        return;
    }
    let _guard = drbg_lock();
    let l = libs();
    let csig_fn = unsafe { l.c::<FnSignature>("crypto_sign_signature") };
    let rsig_fn = unsafe { l.r::<FnSignature>("crypto_sign_signature") };
    let cver = unsafe { l.c::<FnVerify>("crypto_sign_verify") };
    let rver = unsafe { l.r::<FnVerify>("crypto_sign_verify") };

    let mut rng = Rng::new(0x1002);
    let seed = rng.vec(3 * SPX_N);
    let (cpk, csk, rpk, rsk) = keypair(&seed);
    // `crypto_sign_signature` draws `optrand` from the DRBG, so both libraries
    // are re-seeded to the same state immediately before each call.
    let entropy: [u8; 48] = rng.vec(48).try_into().unwrap();

    for mlen in [0usize, 1, 33, 64, 137] {
        let m = rng.vec(mlen);
        let mut cs = vec![0xAAu8; SPX_BYTES + 8];
        let mut rs = vec![0xAAu8; SPX_BYTES + 8];
        let mut cl: usize = 0;
        let mut rl: usize = 0;
        reseed_drbgs(&entropy);
        let cr = unsafe { csig_fn(cs.as_mut_ptr(), &mut cl, m.as_ptr(), mlen, csk.as_ptr()) };
        reseed_drbgs(&entropy);
        let rr = unsafe { rsig_fn(rs.as_mut_ptr(), &mut rl, m.as_ptr(), mlen, rsk.as_ptr()) };
        assert_eq!(cr, rr, "crypto_sign_signature return value (mlen={mlen})");
        assert_eq!(cl, rl, "siglen (mlen={mlen})");
        assert_eq!(cl, SPX_BYTES, "siglen must be SPX_BYTES");
        assert_bytes_eq(&format!("crypto_sign_signature (mlen={mlen})"), &cs, &rs);

        // Cross-verify: each library must accept the other's signature.
        for (name, sig, pk) in [
            ("C sig / C pk", &cs, &cpk),
            ("Rust sig / C pk", &rs, &cpk),
            ("C sig / Rust pk", &cs, &rpk),
            ("Rust sig / Rust pk", &rs, &rpk),
        ] {
            let (cv, rv) = unsafe {
                (
                    cver(sig.as_ptr(), cl, m.as_ptr(), mlen, pk.as_ptr()),
                    rver(sig.as_ptr(), cl, m.as_ptr(), mlen, pk.as_ptr()),
                )
            };
            assert_eq!(cv, rv, "crypto_sign_verify {name} (mlen={mlen})");
            assert_eq!(cv, 0, "crypto_sign_verify {name} should accept");
        }

        // Rejection paths must agree too.
        let (cv, rv) = unsafe {
            (
                cver(cs.as_ptr(), cl - 1, m.as_ptr(), mlen, cpk.as_ptr()),
                rver(rs.as_ptr(), cl - 1, m.as_ptr(), mlen, rpk.as_ptr()),
            )
        };
        assert_eq!(cv, rv, "crypto_sign_verify wrong siglen (mlen={mlen})");
        assert_ne!(cv, 0, "wrong siglen must be rejected");

        let mut bad = cs.clone();
        bad[SPX_BYTES / 2] ^= 0x01;
        let mut badr = rs.clone();
        badr[SPX_BYTES / 2] ^= 0x01;
        let (cv, rv) = unsafe {
            (
                cver(bad.as_ptr(), cl, m.as_ptr(), mlen, cpk.as_ptr()),
                rver(badr.as_ptr(), cl, m.as_ptr(), mlen, rpk.as_ptr()),
            )
        };
        assert_eq!(cv, rv, "crypto_sign_verify tampered sig (mlen={mlen})");
        assert_ne!(cv, 0, "tampered signature must be rejected");
    }
}

#[test]
fn crypto_sign_and_open() {
    if URANDOM {
        return; // see crypto_sign_signature_and_verify
    }
    let _guard = drbg_lock();
    let l = libs();
    let cs_fn = unsafe { l.c::<FnSign>("crypto_sign") };
    let rs_fn = unsafe { l.r::<FnSign>("crypto_sign") };
    let co_fn = unsafe { l.c::<FnSignOpen>("crypto_sign_open") };
    let ro_fn = unsafe { l.r::<FnSignOpen>("crypto_sign_open") };

    let mut rng = Rng::new(0x1003);
    let seed = rng.vec(3 * SPX_N);
    let (cpk, csk, rpk, rsk) = keypair(&seed);
    let entropy: [u8; 48] = rng.vec(48).try_into().unwrap();

    for mlen in [0usize, 1, 32, 100] {
        // Large enough for both the accepted path (mlen bytes) and the rejected
        // path, where crypto_sign_open zeroes smlen bytes of the output.
        let OUTBUF = SPX_BYTES + 16 + mlen;
        let m = rng.vec(mlen);
        let mut csm = vec![0xAAu8; SPX_BYTES + mlen + 8];
        let mut rsm = vec![0xAAu8; SPX_BYTES + mlen + 8];
        let mut cl: u64 = 0;
        let mut rl: u64 = 0;
        reseed_drbgs(&entropy);
        let cr = unsafe {
            cs_fn(
                csm.as_mut_ptr(),
                &mut cl,
                m.as_ptr(),
                mlen as u64,
                csk.as_ptr(),
            )
        };
        reseed_drbgs(&entropy);
        let rr = unsafe {
            rs_fn(
                rsm.as_mut_ptr(),
                &mut rl,
                m.as_ptr(),
                mlen as u64,
                rsk.as_ptr(),
            )
        };
        assert_eq!(cr, rr, "crypto_sign return value (mlen={mlen})");
        assert_eq!(cl, rl, "smlen (mlen={mlen})");
        assert_eq!(cl as usize, SPX_BYTES + mlen, "smlen must be SPX_BYTES+mlen");
        assert_bytes_eq(&format!("crypto_sign (mlen={mlen})"), &csm, &rsm);

        for (name, sm, pk) in [
            ("C sm / C pk", &csm, &cpk),
            ("Rust sm / C pk", &rsm, &cpk),
            ("C sm / Rust pk", &csm, &rpk),
            ("Rust sm / Rust pk", &rsm, &rpk),
        ] {
            // `crypto_sign_open` does `memset(m, 0, smlen)` when it rejects, so
            // the output buffer has to hold `smlen` bytes even though only
            // `mlen` of them are the message.  The whole buffer is compared.
            let mut cm = vec![0xAAu8; OUTBUF];
            let mut rm = vec![0xAAu8; OUTBUF];
            let mut cml: u64 = 0xdead_beef;
            let mut rml: u64 = 0xdead_beef;
            let (cv, rv) = unsafe {
                (
                    co_fn(cm.as_mut_ptr(), &mut cml, sm.as_ptr(), cl, pk.as_ptr()),
                    ro_fn(rm.as_mut_ptr(), &mut rml, sm.as_ptr(), cl, pk.as_ptr()),
                )
            };
            assert_eq!(cv, rv, "crypto_sign_open {name} (mlen={mlen})");
            assert_eq!(cv, 0, "crypto_sign_open {name} should accept");
            assert_eq!(cml, rml, "crypto_sign_open mlen {name}");
            assert_bytes_eq(&format!("crypto_sign_open message {name}"), &cm, &rm);
            assert_eq!(&cm[..mlen], &m[..], "recovered message must equal input");
        }

        // Rejection: too-short smlen and a flipped byte.
        let mut cm = vec![0xAAu8; OUTBUF];
        let mut rm = vec![0xAAu8; OUTBUF];
        let mut cml: u64 = 0xdead_beef;
        let mut rml: u64 = 0xdead_beef;
        let short = SPX_BYTES as u64 - 1;
        let (cv, rv) = unsafe {
            (
                co_fn(cm.as_mut_ptr(), &mut cml, csm.as_ptr(), short, cpk.as_ptr()),
                ro_fn(rm.as_mut_ptr(), &mut rml, rsm.as_ptr(), short, rpk.as_ptr()),
            )
        };
        assert_eq!(cv, rv, "crypto_sign_open short smlen (mlen={mlen})");
        assert_ne!(cv, 0, "short smlen must be rejected");
        assert_eq!(cml, rml, "crypto_sign_open mlen on rejection");
        assert_bytes_eq("crypto_sign_open message on short rejection", &cm, &rm);

        let mut cbad = csm.clone();
        let mut rbad = rsm.clone();
        cbad[3] ^= 0x80;
        rbad[3] ^= 0x80;
        let mut cm = vec![0xAAu8; OUTBUF];
        let mut rm = vec![0xAAu8; OUTBUF];
        let mut cml: u64 = 0xdead_beef;
        let mut rml: u64 = 0xdead_beef;
        let (cv, rv) = unsafe {
            (
                co_fn(cm.as_mut_ptr(), &mut cml, cbad.as_ptr(), cl, cpk.as_ptr()),
                ro_fn(rm.as_mut_ptr(), &mut rml, rbad.as_ptr(), cl, rpk.as_ptr()),
            )
        };
        assert_eq!(cv, rv, "crypto_sign_open tampered (mlen={mlen})");
        assert_ne!(cv, 0, "tampered sm must be rejected");
        assert_eq!(cml, rml, "crypto_sign_open mlen on tamper rejection");
        assert_bytes_eq("crypto_sign_open message on tamper rejection", &cm, &rm);
    }
}

/// `crypto_sign_keypair` draws from `randombytes`, so it is only deterministic
/// once the DRBG has been seeded identically in both libraries.
#[test]
fn crypto_sign_keypair_from_seeded_drbg() {
    if URANDOM {
        return; // crypto_sign_keypair draws its seed from /dev/urandom here
    }
    let _guard = drbg_lock();
    let l = libs();
    type FnKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
    let c = unsafe { l.c::<FnKeypair>("crypto_sign_keypair") };
    let r = unsafe { l.r::<FnKeypair>("crypto_sign_keypair") };

    let mut rng = Rng::new(0x1004);
    for round in 0..2 {
        let entropy: [u8; 48] = rng.vec(48).try_into().unwrap();
        let mut cpk = vec![0xAAu8; SPX_PK_BYTES + 8];
        let mut csk = vec![0xAAu8; SPX_SK_BYTES + 8];
        let mut rpk = vec![0xAAu8; SPX_PK_BYTES + 8];
        let mut rsk = vec![0xAAu8; SPX_SK_BYTES + 8];
        reseed_drbgs(&entropy);
        let cr = unsafe { c(cpk.as_mut_ptr(), csk.as_mut_ptr()) };
        reseed_drbgs(&entropy);
        let rr = unsafe { r(rpk.as_mut_ptr(), rsk.as_mut_ptr()) };
        assert_eq!(cr, rr, "crypto_sign_keypair return value (round={round})");
        assert_bytes_eq(&format!("crypto_sign_keypair pk (round={round})"), &cpk, &rpk);
        assert_bytes_eq(&format!("crypto_sign_keypair sk (round={round})"), &csk, &rsk);
    }
}
