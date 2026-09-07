//! Phase B rows 59-72: the backend primitives, one cfg-gated module per
//! `HASH_BACKEND`. Each family is exercised one-shot AND incrementally, with
//! chunk sizes that straddle the block/rate boundary, and the *whole internal
//! state image* is compared between steps -- not just the final digest.

mod common;
use common::*;

// ===========================================================================
// BLAKE  (rows 59-62)
// ===========================================================================
#[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
mod blake {
    use super::common::*;

    /// `blakestate256 { unsigned int h[8], s[4], t[2]; int buflen, nullt;
    ///                  unsigned char buf[64]; }`
    const ST256: usize = 4 * (8 + 4 + 2) + 4 + 4 + 64; // 128
    /// `blakestate512 { unsigned long long h[8], s[4], t[2]; int buflen, nullt;
    ///                  unsigned char buf[128]; }`
    const ST512: usize = 8 * (8 + 4 + 2) + 4 + 4 + 128; // 248

    type FOneShot = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
    type FInit = unsafe extern "C" fn(*mut u8);
    /// NOTE: `blake*_update`'s length argument is in **bits** -- `blake256()`
    /// calls it as `blake256_update(&S, in, inlen*8)`.
    type FUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64);
    type FFinal = unsafe extern "C" fn(*mut u8, *mut u8);
    type FCompress = unsafe extern "C" fn(*mut u8, *const u8);
    type FMgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);

    fn incremental(
        which: &str,
        st_len: usize,
        out_len: usize,
        block: usize,
        chunks: &[usize],
    ) {
        let (ci, ri) = crate::both!(
            if which == "256" { "blake256_init" } else { "blake512_init" },
            FInit
        );
        let (cu, ru) = crate::both!(
            if which == "256" { "blake256_update" } else { "blake512_update" },
            FUpdate
        );
        let (cf, rf) = crate::both!(
            if which == "256" { "blake256_final" } else { "blake512_final" },
            FFinal
        );
        let (co, ro) = crate::both!(if which == "256" { "blake256" } else { "blake512" }, FOneShot);

        let mut rng = Rng::new(RNG_SEED ^ 59 ^ (block as u64));
        let mut lens: Vec<usize> = vec![
            0,
            1,
            block - 10,
            block - 9,
            block - 8,
            block - 1,
            block,
            block + 1,
            2 * block,
            2 * block + 1,
            1000,
        ];
        lens.sort_unstable();
        lens.dedup();

        for &n in &lens {
            let msg = rng.bytes(n.max(1));
            let msg = &msg[..n];

            // --- one-shot ---
            let mut cb = vec![0xA5u8; out_len + 16];
            let mut rb = vec![0xA5u8; out_len + 16];
            let (crc, rrc) = unsafe {
                (
                    co(cb.as_mut_ptr(), msg.as_ptr(), n as u64),
                    ro(rb.as_mut_ptr(), msg.as_ptr(), n as u64),
                )
            };
            eq(&format!("blake{which}({n}) return"), crc, rrc);
            eq_bytes(&format!("blake{which}({n}) digest"), &cb, &rb);

            // --- incremental, in every chunk size ---
            for &ck in chunks {
                let mut cs = vec![0x5Au8; st_len];
                let mut rs = vec![0x5Au8; st_len];
                unsafe {
                    ci(cs.as_mut_ptr());
                    ri(rs.as_mut_ptr());
                }
                eq_bytes(&format!("blake{which}_init state"), &cs, &rs);

                let mut off = 0usize;
                while off < n {
                    let take = ck.min(n - off);
                    unsafe {
                        cu(cs.as_mut_ptr(), msg[off..].as_ptr(), (take * 8) as u64);
                        ru(rs.as_mut_ptr(), msg[off..].as_ptr(), (take * 8) as u64);
                    }
                    eq_bytes(
                        &format!("blake{which}_update state (n={n} chunk={ck} off={off})"),
                        &cs,
                        &rs,
                    );
                    off += take;
                }
                let mut cd = vec![0xA5u8; out_len + 16];
                let mut rd = vec![0xA5u8; out_len + 16];
                unsafe {
                    cf(cs.as_mut_ptr(), cd.as_mut_ptr());
                    rf(rs.as_mut_ptr(), rd.as_mut_ptr());
                }
                eq_bytes(
                    &format!("blake{which}_final digest (n={n} chunk={ck})"),
                    &cd,
                    &rd,
                );
                eq_bytes(
                    &format!("blake{which}_final state (n={n} chunk={ck})"),
                    &cs,
                    &rs,
                );
                // NOTE: we deliberately do NOT assert
                // `incremental == one-shot` here. `blake256_update`'s guard
                //   if (left && (((datalen >> 3) & 0x3F) >= fill))
                // makes the reference BLAKE implementation's result depend on
                // HOW the input was chunked (e.g. n=129 fed in 65-byte chunks
                // differs from the one-shot hash in the C itself). That is C
                // behaviour, so the only thing under test is C-vs-Rust
                // agreement -- which is asserted for every intermediate state
                // above and for the digest just now.
            }
        }
    }

    #[test]
    fn row59_blake256() {
        incremental("256", ST256, 32, 64, &[1, 7, 63, 64, 65, 1000]);
    }

    #[test]
    fn row60_blake512() {
        incremental("512", ST512, 64, 128, &[1, 7, 127, 128, 129, 1000]);
    }

    #[test]
    fn row61_blake_compress() {
        let (c256, r256) = crate::both!("blake256_compress", FCompress);
        let (c512, r512) = crate::both!("blake512_compress", FCompress);
        let (ci256, ri256) = crate::both!("blake256_init", FInit);
        let (ci512, ri512) = crate::both!("blake512_init", FInit);
        let mut rng = Rng::new(RNG_SEED ^ 61);
        for iter in 0..N_ITER {
            // Start from a properly initialised state, then feed random blocks.
            let mut cs = vec![0u8; ST256];
            let mut rs = vec![0u8; ST256];
            unsafe {
                ci256(cs.as_mut_ptr());
                ri256(rs.as_mut_ptr());
            }
            for _ in 0..3 {
                let blk = match iter {
                    0 => vec![0x00u8; 64],
                    1 => vec![0xFFu8; 64],
                    _ => rng.bytes(64),
                };
                unsafe {
                    c256(cs.as_mut_ptr(), blk.as_ptr());
                    r256(rs.as_mut_ptr(), blk.as_ptr());
                }
                eq_bytes("blake256_compress state", &cs, &rs);
            }

            let mut cs = vec![0u8; ST512];
            let mut rs = vec![0u8; ST512];
            unsafe {
                ci512(cs.as_mut_ptr());
                ri512(rs.as_mut_ptr());
            }
            for _ in 0..3 {
                let blk = match iter {
                    0 => vec![0x00u8; 128],
                    1 => vec![0xFFu8; 128],
                    _ => rng.bytes(128),
                };
                unsafe {
                    c512(cs.as_mut_ptr(), blk.as_ptr());
                    r512(rs.as_mut_ptr(), blk.as_ptr());
                }
                eq_bytes("blake512_compress state", &cs, &rs);
            }
        }
    }

    #[test]
    fn row62_blake_mgf1() {
        for (name, ob) in [("SPX_blake256_mgf1", 32usize), ("SPX_blake512_mgf1", 64)] {
            let l = libs();
            let c: libloading::Symbol<FMgf1> = sym(&l.c, name);
            let r: libloading::Symbol<FMgf1> = sym(&l.r, name);
            let mut rng = Rng::new(RNG_SEED ^ 62 ^ ob as u64);
            for &outlen in &[0usize, 1, ob - 1, ob, ob + 1, 2 * ob, 2 * ob + 1, 100, 300] {
                for &inlen in &[0usize, 1, 4, 32, 64, 100] {
                    let inp = rng.bytes(inlen.max(1));
                    // mgf1 appends a 4-byte counter to a copy of `in`, so it
                    // needs inlen+4 readable bytes -- give it the room the C
                    // callers give it.
                    let mut inbuf = vec![0u8; inlen + 8];
                    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
                    let mut cb = vec![0xA5u8; outlen + 16];
                    let mut rb = vec![0xA5u8; outlen + 16];
                    unsafe {
                        c(cb.as_mut_ptr(), outlen as u64, inbuf.as_ptr(), inlen as u64);
                        r(rb.as_mut_ptr(), outlen as u64, inbuf.as_ptr(), inlen as u64);
                    }
                    eq_bytes(&format!("{name}(outlen={outlen}, inlen={inlen})"), &cb, &rb);
                }
            }
        }
    }
}

// ===========================================================================
// SHA2  (rows 63-66)
// ===========================================================================
#[cfg(feature = "sha2")]
mod sha2 {
    use super::common::*;

    type FIncInit = unsafe extern "C" fn(*mut u8);
    type FIncBlocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type FIncFinalize = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
    type FOneShot = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type FMgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);
    type FSeedState = unsafe extern "C" fn(*mut u8);

    fn sha_family(bits: u32, st_len: usize, out_len: usize, block: usize) {
        let p = format!("sha{bits}");
        let (ci, ri) = {
            let l = libs();
            let n = format!("{p}_inc_init");
            (
                sym::<FIncInit>(&l.c, &n),
                sym::<FIncInit>(&l.r, &n),
            )
        };
        let (cb_, rb_) = {
            let l = libs();
            let n = format!("{p}_inc_blocks");
            (
                sym::<FIncBlocks>(&l.c, &n),
                sym::<FIncBlocks>(&l.r, &n),
            )
        };
        let (cf, rf) = {
            let l = libs();
            let n = format!("{p}_inc_finalize");
            (
                sym::<FIncFinalize>(&l.c, &n),
                sym::<FIncFinalize>(&l.r, &n),
            )
        };
        let (co, ro) = {
            let l = libs();
            (sym::<FOneShot>(&l.c, &p), sym::<FOneShot>(&l.r, &p))
        };

        let mut rng = Rng::new(RNG_SEED ^ 63 ^ bits as u64);

        // --- one-shot over every boundary ---
        let mut lens: Vec<usize> = vec![
            0,
            1,
            block - 10,
            block - 9,
            block - 8,
            block - 1,
            block,
            block + 1,
            2 * block,
            1000,
        ];
        lens.sort_unstable();
        lens.dedup();
        for &n in &lens {
            let msg = rng.bytes(n.max(1));
            let mut cd = vec![0xA5u8; out_len + 16];
            let mut rd = vec![0xA5u8; out_len + 16];
            unsafe {
                co(cd.as_mut_ptr(), msg.as_ptr(), n);
                ro(rd.as_mut_ptr(), msg.as_ptr(), n);
            }
            eq_bytes(&format!("{p}({n})"), &cd, &rd);
        }

        // --- incremental: inc_init, inc_blocks x k, inc_finalize(tail) ---
        for &nblocks in &[0usize, 1, 2, 5] {
            for &tail in &[0usize, 1, block - 9, block - 8, block - 1] {
                let data = rng.bytes(nblocks * block + tail + 1);
                let mut cs = vec![0x5Au8; st_len];
                let mut rs = vec![0x5Au8; st_len];
                unsafe {
                    ci(cs.as_mut_ptr());
                    ri(rs.as_mut_ptr());
                }
                eq_bytes(&format!("{p}_inc_init state"), &cs, &rs);
                if nblocks > 0 {
                    unsafe {
                        cb_(cs.as_mut_ptr(), data.as_ptr(), nblocks);
                        rb_(rs.as_mut_ptr(), data.as_ptr(), nblocks);
                    }
                }
                // Also exercise the inblocks == 0 no-op explicitly.
                unsafe {
                    cb_(cs.as_mut_ptr(), data.as_ptr(), 0);
                    rb_(rs.as_mut_ptr(), data.as_ptr(), 0);
                }
                eq_bytes(
                    &format!("{p}_inc_blocks state (nblocks={nblocks})"),
                    &cs,
                    &rs,
                );
                let mut cd = vec![0xA5u8; out_len + 16];
                let mut rd = vec![0xA5u8; out_len + 16];
                unsafe {
                    cf(
                        cd.as_mut_ptr(),
                        cs.as_mut_ptr(),
                        data[nblocks * block..].as_ptr(),
                        tail,
                    );
                    rf(
                        rd.as_mut_ptr(),
                        rs.as_mut_ptr(),
                        data[nblocks * block..].as_ptr(),
                        tail,
                    );
                }
                eq_bytes(
                    &format!("{p}_inc_finalize digest (nblocks={nblocks}, tail={tail})"),
                    &cd,
                    &rd,
                );
                eq_bytes(
                    &format!("{p}_inc_finalize state (nblocks={nblocks}, tail={tail})"),
                    &cs,
                    &rs,
                );
                // The incremental result must equal the one-shot hash.
                let total = nblocks * block + tail;
                let mut od = vec![0u8; out_len];
                unsafe { co(od.as_mut_ptr(), data.as_ptr(), total) };
                eq_bytes(
                    &format!("{p} incremental == one-shot (nblocks={nblocks}, tail={tail})"),
                    &cd[..out_len],
                    &od,
                );
            }
        }
    }

    #[test]
    fn row63_sha256() {
        sha_family(256, 40, 32, 64);
    }

    #[test]
    fn row64_sha512() {
        sha_family(512, 72, 64, 128);
    }

    #[test]
    fn row65_mgf1() {
        for (name, ob) in [("SPX_mgf1_256", 32usize), ("SPX_mgf1_512", 64)] {
            let l = libs();
            let c: libloading::Symbol<FMgf1> = sym(&l.c, name);
            let r: libloading::Symbol<FMgf1> = sym(&l.r, name);
            let mut rng = Rng::new(RNG_SEED ^ 65 ^ ob as u64);
            for &outlen in &[0usize, 1, ob - 1, ob, ob + 1, 2 * ob, 2 * ob + 1, 100, 300] {
                for &inlen in &[0usize, 1, 4, 32, 64, 100] {
                    let inp = rng.bytes(inlen.max(1));
                    let mut inbuf = vec![0u8; inlen + 8];
                    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
                    let mut cb = vec![0xA5u8; outlen + 16];
                    let mut rb = vec![0xA5u8; outlen + 16];
                    unsafe {
                        c(cb.as_mut_ptr(), outlen as u64, inbuf.as_ptr(), inlen as u64);
                        r(rb.as_mut_ptr(), outlen as u64, inbuf.as_ptr(), inlen as u64);
                    }
                    eq_bytes(&format!("{name}(outlen={outlen}, inlen={inlen})"), &cb, &rb);
                }
            }
        }
    }

    #[test]
    fn row66_seed_state() {
        let (c, r) = crate::both!("SPX_seed_state", FSeedState);
        let mut rng = Rng::new(RNG_SEED ^ 66);
        for iter in 0..N_ITER + 2 {
            let ps = match iter {
                0 => vec![0x00u8; SPX_N],
                1 => vec![0xFFu8; SPX_N],
                _ => rng.bytes(SPX_N),
            };
            let ss = rng.bytes(SPX_N);
            let mut cc = new_ctx_buf();
            let mut rc = new_ctx_buf();
            for buf in [&mut cc, &mut rc] {
                buf[..SPX_N].copy_from_slice(&ps);
                buf[SPX_N..2 * SPX_N].copy_from_slice(&ss);
            }
            unsafe {
                c(cc.as_mut_ptr());
                r(rc.as_mut_ptr());
            }
            eq_bytes(&format!("seed_state ctx image (pub_seed={})", hex(&ps)), &cc, &rc);
        }
    }
}

// ===========================================================================
// SHAKE  (rows 67-68)
// ===========================================================================
#[cfg(all(feature = "shake", not(feature = "sha2")))]
mod shake {
    use super::common::*;

    const RATE: usize = 136;
    /// `uint64_t s_inc[26]`
    const INC_BYTES: usize = 26 * 8;
    /// `uint64_t s[25]`
    const ST_BYTES: usize = 25 * 8;

    type FIncInit = unsafe extern "C" fn(*mut u64);
    type FIncAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type FIncFinalize = unsafe extern "C" fn(*mut u64);
    type FIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
    type FAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type FSqueezeBlocks = unsafe extern "C" fn(*mut u8, usize, *mut u64);
    type FOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);

    fn state_bytes(v: &[u64]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_ne_bytes()).collect()
    }

    #[test]
    fn row67_shake256_incremental() {
        let (ci, ri) = crate::both!("shake256_inc_init", FIncInit);
        let (ca, ra) = crate::both!("shake256_inc_absorb", FIncAbsorb);
        let (cf, rf) = crate::both!("shake256_inc_finalize", FIncFinalize);
        let (cq, rq) = crate::both!("shake256_inc_squeeze", FIncSqueeze);
        let (co, ro) = crate::both!("shake256", FOneShot);

        let mut rng = Rng::new(RNG_SEED ^ 67);
        let mut lens: Vec<usize> = vec![0, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 1000];
        lens.sort_unstable();
        lens.dedup();

        for &n in &lens {
            let msg = rng.bytes(n.max(1));
            for &chunk in &[1usize, 7, RATE - 1, RATE, RATE + 1, 1000] {
                for &outlen in &[0usize, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 300] {
                    let mut cs = vec![0u64; 26];
                    let mut rs = vec![0u64; 26];
                    unsafe {
                        ci(cs.as_mut_ptr());
                        ri(rs.as_mut_ptr());
                    }
                    eq_bytes(
                        "shake256_inc_init state",
                        &state_bytes(&cs),
                        &state_bytes(&rs),
                    );
                    // Explicit inlen == 0 absorb (must be a no-op).
                    unsafe {
                        ca(cs.as_mut_ptr(), msg.as_ptr(), 0);
                        ra(rs.as_mut_ptr(), msg.as_ptr(), 0);
                    }
                    let mut off = 0usize;
                    while off < n {
                        let take = chunk.min(n - off);
                        unsafe {
                            ca(cs.as_mut_ptr(), msg[off..].as_ptr(), take);
                            ra(rs.as_mut_ptr(), msg[off..].as_ptr(), take);
                        }
                        eq_bytes(
                            &format!("shake256_inc_absorb state (n={n} chunk={chunk} off={off})"),
                            &state_bytes(&cs),
                            &state_bytes(&rs),
                        );
                        off += take;
                    }
                    unsafe {
                        cf(cs.as_mut_ptr());
                        rf(rs.as_mut_ptr());
                    }
                    eq_bytes(
                        &format!("shake256_inc_finalize state (n={n} chunk={chunk})"),
                        &state_bytes(&cs),
                        &state_bytes(&rs),
                    );
                    // Squeeze in one go, and in pieces, comparing state each time.
                    let mut cb = vec![0xA5u8; outlen + 16];
                    let mut rb = vec![0xA5u8; outlen + 16];
                    unsafe {
                        cq(cb.as_mut_ptr(), outlen, cs.as_mut_ptr());
                        rq(rb.as_mut_ptr(), outlen, rs.as_mut_ptr());
                    }
                    eq_bytes(
                        &format!("shake256_inc_squeeze out (n={n} chunk={chunk} outlen={outlen})"),
                        &cb,
                        &rb,
                    );
                    eq_bytes(
                        &format!("shake256_inc_squeeze state (n={n} chunk={chunk} outlen={outlen})"),
                        &state_bytes(&cs),
                        &state_bytes(&rs),
                    );
                    // ...and check the incremental digest matches the one-shot.
                    if outlen > 0 {
                        let mut od = vec![0u8; outlen];
                        unsafe { co(od.as_mut_ptr(), outlen, msg.as_ptr(), n) };
                        eq_bytes(
                            &format!("shake256 inc == one-shot (n={n} outlen={outlen})"),
                            &cb[..outlen],
                            &od,
                        );
                        let mut od2 = vec![0u8; outlen];
                        unsafe { ro(od2.as_mut_ptr(), outlen, msg.as_ptr(), n) };
                        eq_bytes(
                            &format!("shake256 one-shot C vs Rust (n={n} outlen={outlen})"),
                            &od,
                            &od2,
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn row67b_shake256_incremental_multi_squeeze() {
        let (ci, ri) = crate::both!("shake256_inc_init", FIncInit);
        let (ca, ra) = crate::both!("shake256_inc_absorb", FIncAbsorb);
        let (cf, rf) = crate::both!("shake256_inc_finalize", FIncFinalize);
        let (cq, rq) = crate::both!("shake256_inc_squeeze", FIncSqueeze);
        let mut rng = Rng::new(RNG_SEED ^ 671);
        let msg = rng.bytes(300);
        let mut cs = vec![0u64; 26];
        let mut rs = vec![0u64; 26];
        unsafe {
            ci(cs.as_mut_ptr());
            ri(rs.as_mut_ptr());
            ca(cs.as_mut_ptr(), msg.as_ptr(), msg.len());
            ra(rs.as_mut_ptr(), msg.as_ptr(), msg.len());
            cf(cs.as_mut_ptr());
            rf(rs.as_mut_ptr());
        }
        for &piece in &[0usize, 1, 7, RATE - 1, RATE, RATE + 1, 0, 200] {
            let mut cb = vec![0xA5u8; piece + 8];
            let mut rb = vec![0xA5u8; piece + 8];
            unsafe {
                cq(cb.as_mut_ptr(), piece, cs.as_mut_ptr());
                rq(rb.as_mut_ptr(), piece, rs.as_mut_ptr());
            }
            eq_bytes(&format!("multi-squeeze piece={piece} out"), &cb, &rb);
            eq_bytes(
                &format!("multi-squeeze piece={piece} state"),
                &state_bytes(&cs),
                &state_bytes(&rs),
            );
        }
    }

    #[test]
    fn row68_shake256_absorb_squeezeblocks() {
        let (ca, ra) = crate::both!("shake256_absorb", FAbsorb);
        let (cq, rq) = crate::both!("shake256_squeezeblocks", FSqueezeBlocks);
        let mut rng = Rng::new(RNG_SEED ^ 68);
        for &n in &[0usize, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 1000] {
            let msg = rng.bytes(n.max(1));
            for &nblocks in &[0usize, 1, 2, 3] {
                let mut cs = vec![0u64; 25];
                let mut rs = vec![0u64; 25];
                unsafe {
                    ca(cs.as_mut_ptr(), msg.as_ptr(), n);
                    ra(rs.as_mut_ptr(), msg.as_ptr(), n);
                }
                eq_bytes(
                    &format!("shake256_absorb state (n={n})"),
                    &state_bytes(&cs),
                    &state_bytes(&rs),
                );
                let mut cb = vec![0xA5u8; nblocks * RATE + 16];
                let mut rb = vec![0xA5u8; nblocks * RATE + 16];
                unsafe {
                    cq(cb.as_mut_ptr(), nblocks, cs.as_mut_ptr());
                    rq(rb.as_mut_ptr(), nblocks, rs.as_mut_ptr());
                }
                eq_bytes(
                    &format!("shake256_squeezeblocks out (n={n} nblocks={nblocks})"),
                    &cb,
                    &rb,
                );
                eq_bytes(
                    &format!("shake256_squeezeblocks state (n={n} nblocks={nblocks})"),
                    &state_bytes(&cs),
                    &state_bytes(&rs),
                );
            }
        }
        // ST_BYTES / INC_BYTES are documented sizes; keep them referenced.
        assert_eq!(ST_BYTES, 200);
        assert_eq!(INC_BYTES, 208);
    }
}

// ===========================================================================
// HARAKA  (rows 69-72)
// ===========================================================================
#[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
mod haraka {
    use super::common::*;

    const RATE: usize = 32;
    const S_INC: usize = 65;

    type FTweak = unsafe extern "C" fn(*mut u8);
    type FPerm = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
    type FIncInit = unsafe extern "C" fn(*mut u8);
    type FIncAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8);
    type FIncFinalize = unsafe extern "C" fn(*mut u8);
    type FIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const u8);
    type FHarakaS = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8);

    #[test]
    fn row69_tweak_constants() {
        let (c, r) = crate::both!("SPX_tweak_constants", FTweak);
        let mut rng = Rng::new(RNG_SEED ^ 69);
        for iter in 0..N_ITER + 2 {
            let (ps, ss) = match iter {
                0 => (vec![0x00u8; SPX_N], vec![0x00u8; SPX_N]),
                1 => (vec![0xFFu8; SPX_N], vec![0xFFu8; SPX_N]),
                _ => (rng.bytes(SPX_N), rng.bytes(SPX_N)),
            };
            let mut cc = new_ctx_buf();
            let mut rc = new_ctx_buf();
            for buf in [&mut cc, &mut rc] {
                buf[..SPX_N].copy_from_slice(&ps);
                buf[SPX_N..2 * SPX_N].copy_from_slice(&ss);
            }
            unsafe {
                c(cc.as_mut_ptr());
                r(rc.as_mut_ptr());
            }
            eq_bytes(
                &format!("tweak_constants ctx image (pub_seed={})", hex(&ps)),
                &cc,
                &rc,
            );
        }
    }

    #[test]
    fn row70_haraka_perm_and_blocks() {
        let mut rng = Rng::new(RNG_SEED ^ 70);
        let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        for (name, in_len, out_len) in [
            ("SPX_haraka512_perm", 64usize, 64usize),
            ("SPX_haraka512", 64, 32),
            ("SPX_haraka256", 32, 32),
        ] {
            let l = libs();
            let c: libloading::Symbol<FPerm> = sym(&l.c, name);
            let r: libloading::Symbol<FPerm> = sym(&l.r, name);
            let mut inputs: Vec<Vec<u8>> = vec![vec![0x00u8; in_len], vec![0xFFu8; in_len]];
            for _ in 0..N_ITER * 2 {
                inputs.push(rng.bytes(in_len));
            }
            for inp in &inputs {
                let mut cb = vec![0xA5u8; out_len + 16];
                let mut rb = vec![0xA5u8; out_len + 16];
                unsafe {
                    c(cb.as_mut_ptr(), inp.as_ptr(), cctx.as_ptr());
                    r(rb.as_mut_ptr(), inp.as_ptr(), rctx.as_ptr());
                }
                eq_bytes(&format!("{name}(in={})", hex(&inp[..8])), &cb, &rb);
            }
        }
    }

    #[test]
    fn row71_haraka_sponge_incremental() {
        let (ci, ri) = crate::both!("SPX_haraka_S_inc_init", FIncInit);
        let (ca, ra) = crate::both!("SPX_haraka_S_inc_absorb", FIncAbsorb);
        let (cf, rf) = crate::both!("SPX_haraka_S_inc_finalize", FIncFinalize);
        let (cq, rq) = crate::both!("SPX_haraka_S_inc_squeeze", FIncSqueeze);
        let mut rng = Rng::new(RNG_SEED ^ 71);
        let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));

        for &n in &[0usize, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 300] {
            let msg = rng.bytes(n.max(1));
            for &chunk in &[1usize, 7, RATE - 1, RATE, RATE + 1, 1000] {
                for &outlen in &[0usize, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 100] {
                    let mut cs = vec![0u8; S_INC];
                    let mut rs = vec![0u8; S_INC];
                    unsafe {
                        ci(cs.as_mut_ptr());
                        ri(rs.as_mut_ptr());
                    }
                    eq_bytes("haraka_S_inc_init state", &cs, &rs);
                    unsafe {
                        ca(cs.as_mut_ptr(), msg.as_ptr(), 0, cctx.as_ptr());
                        ra(rs.as_mut_ptr(), msg.as_ptr(), 0, rctx.as_ptr());
                    }
                    let mut off = 0usize;
                    while off < n {
                        let take = chunk.min(n - off);
                        unsafe {
                            ca(cs.as_mut_ptr(), msg[off..].as_ptr(), take, cctx.as_ptr());
                            ra(rs.as_mut_ptr(), msg[off..].as_ptr(), take, rctx.as_ptr());
                        }
                        eq_bytes(
                            &format!("haraka_S_inc_absorb state (n={n} chunk={chunk} off={off})"),
                            &cs,
                            &rs,
                        );
                        off += take;
                    }
                    unsafe {
                        cf(cs.as_mut_ptr());
                        rf(rs.as_mut_ptr());
                    }
                    eq_bytes(
                        &format!("haraka_S_inc_finalize state (n={n} chunk={chunk})"),
                        &cs,
                        &rs,
                    );
                    let mut cb = vec![0xA5u8; outlen + 16];
                    let mut rb = vec![0xA5u8; outlen + 16];
                    unsafe {
                        cq(cb.as_mut_ptr(), outlen, cs.as_mut_ptr(), cctx.as_ptr());
                        rq(rb.as_mut_ptr(), outlen, rs.as_mut_ptr(), rctx.as_ptr());
                    }
                    eq_bytes(
                        &format!("haraka_S_inc_squeeze out (n={n} chunk={chunk} outlen={outlen})"),
                        &cb,
                        &rb,
                    );
                    eq_bytes(
                        &format!("haraka_S_inc_squeeze state (n={n} chunk={chunk} outlen={outlen})"),
                        &cs,
                        &rs,
                    );
                }
            }
        }
    }

    #[test]
    fn row72_haraka_S_one_shot() {
        let (c, r) = crate::both!("SPX_haraka_S", FHarakaS);
        let mut rng = Rng::new(RNG_SEED ^ 72);
        let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
        for &inlen in &[0usize, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 1000] {
            let msg = rng.bytes(inlen.max(1));
            for &outlen in &[0usize, 1, RATE - 1, RATE, RATE + 1, 2 * RATE, 100, 300] {
                let mut cb = vec![0xA5u8; outlen + 16];
                let mut rb = vec![0xA5u8; outlen + 16];
                unsafe {
                    c(
                        cb.as_mut_ptr(),
                        outlen as u64,
                        msg.as_ptr(),
                        inlen as u64,
                        cctx.as_ptr(),
                    );
                    r(
                        rb.as_mut_ptr(),
                        outlen as u64,
                        msg.as_ptr(),
                        inlen as u64,
                        rctx.as_ptr(),
                    );
                }
                eq_bytes(
                    &format!("haraka_S(inlen={inlen}, outlen={outlen})"),
                    &cb,
                    &rb,
                );
            }
        }
    }
}
