//! Phase B, CONFIGS.md rows 48-62: the hash primitives of the selected
//! backend.  Each block is `cfg`-gated on the backend the crate was built with,
//! because `lib/CMakeLists.txt` only ever compiles one of them.

mod common;

use common::params::*;
use common::*;

/// Input lengths that straddle every block/rate boundary of every primitive.
fn inlens() -> Vec<usize> {
    let mut v: Vec<usize> = (0..=200).collect();
    v.extend_from_slice(&[
        255, 256, 257, 271, 272, 273, 335, 336, 337, 511, 512, 513, 1000, 1023, 1024, 4096,
    ]);
    v
}

fn outlens() -> Vec<usize> {
    vec![
        0, 1, 2, 31, 32, 33, 63, 64, 65, 71, 72, 73, 100, 135, 136, 137, 200, 271, 272, 273, 500,
    ]
}

// ===========================================================================
// blake
// ===========================================================================

#[cfg(backend_blake)]
mod blake {
    use super::*;

    type Blake = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
    type Init256 = unsafe extern "C" fn(*mut BlakeState256);
    type Update256 = unsafe extern "C" fn(*mut BlakeState256, *const u8, u64);
    type Final256 = unsafe extern "C" fn(*mut BlakeState256, *mut u8);
    type Compress256 = unsafe extern "C" fn(*mut BlakeState256, *const u8);
    type Init512 = unsafe extern "C" fn(*mut BlakeState512);
    type Update512 = unsafe extern "C" fn(*mut BlakeState512, *const u8, u64);
    type Final512 = unsafe extern "C" fn(*mut BlakeState512, *mut u8);
    type Compress512 = unsafe extern "C" fn(*mut BlakeState512, *const u8);
    type Mgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);

    #[test]
    fn row48_blake_one_shot() {
        let libs = load();
        let mut rng = Rng::new(48);
        let (c256, r256) = libs.pair::<Blake>("blake256");
        let (c512, r512) = libs.pair::<Blake>("blake512");
        for inlen in inlens() {
            for rep in 0..3 {
                let inp = match rep {
                    0 => vec![0u8; inlen],
                    1 => vec![0xFFu8; inlen],
                    _ => rng.bytes(inlen),
                };
                let mut a = vec![0xA5u8; 32 + 8];
                let mut b = vec![0xA5u8; 32 + 8];
                let (ra, rb) = unsafe {
                    (
                        c256(a.as_mut_ptr(), inp.as_ptr(), inlen as u64),
                        r256(b.as_mut_ptr(), inp.as_ptr(), inlen as u64),
                    )
                };
                assert_eq!(ra, rb, "blake256 return (inlen={inlen})");
                eq(&format!("blake256(inlen={inlen}, rep={rep})"), &a, &b);

                let mut a = vec![0xA5u8; 64 + 8];
                let mut b = vec![0xA5u8; 64 + 8];
                let (ra, rb) = unsafe {
                    (
                        c512(a.as_mut_ptr(), inp.as_ptr(), inlen as u64),
                        r512(b.as_mut_ptr(), inp.as_ptr(), inlen as u64),
                    )
                };
                assert_eq!(ra, rb, "blake512 return (inlen={inlen})");
                eq(&format!("blake512(inlen={inlen}, rep={rep})"), &a, &b);
            }
        }
    }

    #[test]
    fn row49_blake_incremental() {
        let libs = load();
        let mut rng = Rng::new(49);
        let (ic, ir) = libs.pair::<Init256>("blake256_init");
        let (uc, ur) = libs.pair::<Update256>("blake256_update");
        let (fc, fr) = libs.pair::<Final256>("blake256_final");
        let (ic5, ir5) = libs.pair::<Init512>("blake512_init");
        let (uc5, ur5) = libs.pair::<Update512>("blake512_update");
        let (fc5, fr5) = libs.pair::<Final512>("blake512_final");
        let (oc, _or) = libs.pair::<Blake>("blake256");
        let (oc5, _or5) = libs.pair::<Blake>("blake512");

        // `blake*_update` takes its length in bits; hash_blake.c passes byte
        // counts, but the one-shot `blake256` passes inlen*8.  Only whole-byte
        // lengths are used here, matching every call site in the reference.
        for total in [0usize, 1, 31, 32, 55, 56, 63, 64, 65, 111, 112, 127, 128, 129, 200, 300] {
            let data = rng.bytes(total);
            for nchunks in 1..=4usize {
                // deterministic chunk split
                let mut bounds = vec![0usize];
                for k in 1..nchunks {
                    bounds.push(total * k / nchunks);
                }
                bounds.push(total);

                let mut sa = BlakeState256::zeroed();
                let mut sb = BlakeState256::zeroed();
                unsafe {
                    ic(&mut sa);
                    ir(&mut sb);
                    eq("blake256_init state", sa.as_bytes(), sb.as_bytes());
                    for w in bounds.windows(2) {
                        let (s, e) = (w[0], w[1]);
                        uc(&mut sa, data[s..].as_ptr(), ((e - s) * 8) as u64);
                        ur(&mut sb, data[s..].as_ptr(), ((e - s) * 8) as u64);
                        eq(
                            &format!("blake256_update state (total={total}, chunk {s}..{e})"),
                            sa.as_bytes(),
                            sb.as_bytes(),
                        );
                    }
                    let mut da = [0xA5u8; 40];
                    let mut db = [0xA5u8; 40];
                    fc(&mut sa, da.as_mut_ptr());
                    fr(&mut sb, db.as_mut_ptr());
                    eq(&format!("blake256_final digest (total={total})"), &da, &db);
                    eq("blake256_final state", sa.as_bytes(), sb.as_bytes());
                    if nchunks == 1 {
                        let mut one = [0u8; 32];
                        oc(one.as_mut_ptr(), data.as_ptr(), total as u64);
                        assert_eq!(&da[..32], &one[..], "incremental != one-shot (blake256)");
                    }
                }

                let mut sa = BlakeState512::zeroed();
                let mut sb = BlakeState512::zeroed();
                unsafe {
                    ic5(&mut sa);
                    ir5(&mut sb);
                    eq("blake512_init state", sa.as_bytes(), sb.as_bytes());
                    for w in bounds.windows(2) {
                        let (s, e) = (w[0], w[1]);
                        uc5(&mut sa, data[s..].as_ptr(), ((e - s) * 8) as u64);
                        ur5(&mut sb, data[s..].as_ptr(), ((e - s) * 8) as u64);
                        eq(
                            &format!("blake512_update state (total={total}, chunk {s}..{e})"),
                            sa.as_bytes(),
                            sb.as_bytes(),
                        );
                    }
                    let mut da = [0xA5u8; 72];
                    let mut db = [0xA5u8; 72];
                    fc5(&mut sa, da.as_mut_ptr());
                    fr5(&mut sb, db.as_mut_ptr());
                    eq(&format!("blake512_final digest (total={total})"), &da, &db);
                    eq("blake512_final state", sa.as_bytes(), sb.as_bytes());
                    if nchunks == 1 {
                        let mut one = [0u8; 64];
                        oc5(one.as_mut_ptr(), data.as_ptr(), total as u64);
                        assert_eq!(&da[..64], &one[..], "incremental != one-shot (blake512)");
                    }
                }
            }
        }
    }

    #[test]
    fn row50_blake_compress() {
        let libs = load();
        let mut rng = Rng::new(50);
        let (cc, cr) = libs.pair::<Compress256>("blake256_compress");
        let (cc5, cr5) = libs.pair::<Compress512>("blake512_compress");
        for nullt in [0i32, 1] {
            for _ in 0..64 {
                let mut sa = BlakeState256::zeroed();
                for h in sa.h.iter_mut() {
                    *h = rng.next_u32();
                }
                for s in sa.s.iter_mut() {
                    *s = rng.next_u32();
                }
                for t in sa.t.iter_mut() {
                    *t = rng.next_u32();
                }
                sa.nullt = nullt;
                let sb = sa;
                let mut sa = sa;
                let mut sb = sb;
                let block = rng.bytes(64);
                unsafe {
                    cc(&mut sa, block.as_ptr());
                    cr(&mut sb, block.as_ptr());
                }
                eq(
                    &format!("blake256_compress (nullt={nullt})"),
                    sa.as_bytes(),
                    sb.as_bytes(),
                );

                let mut ta = BlakeState512::zeroed();
                for h in ta.h.iter_mut() {
                    *h = rng.next_u64();
                }
                for s in ta.s.iter_mut() {
                    *s = rng.next_u64();
                }
                for t in ta.t.iter_mut() {
                    *t = rng.next_u64();
                }
                ta.nullt = nullt;
                let mut tb = ta;
                let block = rng.bytes(128);
                unsafe {
                    cc5(&mut ta, block.as_ptr());
                    cr5(&mut tb, block.as_ptr());
                }
                eq(
                    &format!("blake512_compress (nullt={nullt})"),
                    ta.as_bytes(),
                    tb.as_bytes(),
                );
            }
        }
    }

    #[test]
    fn row51_blake_mgf1() {
        let libs = load();
        let mut rng = Rng::new(51);
        let (mc, mr) = libs.pair::<Mgf1>("SPX_blake256_mgf1");
        let (mc5, mr5) = libs.pair::<Mgf1>("SPX_blake512_mgf1");
        for inlen in [0usize, 1, 4, 16, 32, 48, SPX_N + SPX_ADDR_BYTES, 100, 200] {
            let inp = rng.bytes(inlen.max(1));
            for outlen in outlens() {
                let mut a = vec![0xA5u8; outlen + 8];
                let mut b = vec![0xA5u8; outlen + 8];
                unsafe {
                    mc(a.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                    mr(b.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                }
                eq(
                    &format!("SPX_blake256_mgf1(outlen={outlen}, inlen={inlen})"),
                    &a,
                    &b,
                );
                let mut a = vec![0xA5u8; outlen + 8];
                let mut b = vec![0xA5u8; outlen + 8];
                unsafe {
                    mc5(a.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                    mr5(b.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                }
                eq(
                    &format!("SPX_blake512_mgf1(outlen={outlen}, inlen={inlen})"),
                    &a,
                    &b,
                );
            }
        }
    }

    /// The `cst` table `blake512.c` exports without `static`.
    #[test]
    fn row48b_cst_data_symbol() {
        let libs = load();
        let cc = libs.c::<*const [u64; 16]>("cst");
        let cr = libs.r::<*const [u64; 16]>("cst");
        unsafe {
            let a = **cc;
            let b = **cr;
            assert_eq!(a, b, "exported `cst` table differs");
            assert_eq!(a[0], 0x243F_6A88_85A3_08D3);
            assert_eq!(a[15], 0x6369_20D8_7157_4E69);
        }
    }
}

// ===========================================================================
// sha2
// ===========================================================================

#[cfg(backend_sha2)]
mod sha2 {
    use super::*;

    type Sha = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type IncInit = unsafe extern "C" fn(*mut u8);
    type IncBlocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
    type IncFinalize = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, usize);
    type Mgf1 = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);
    type SeedState = unsafe extern "C" fn(*mut u8);

    #[test]
    fn row52_sha_one_shot() {
        let libs = load();
        let mut rng = Rng::new(52);
        let (c2, r2) = libs.pair::<Sha>("sha256");
        let (c5, r5) = libs.pair::<Sha>("sha512");
        for inlen in inlens() {
            for rep in 0..3 {
                let inp = match rep {
                    0 => vec![0u8; inlen],
                    1 => vec![0xFFu8; inlen],
                    _ => rng.bytes(inlen),
                };
                let mut a = vec![0xA5u8; 32 + 8];
                let mut b = vec![0xA5u8; 32 + 8];
                unsafe {
                    c2(a.as_mut_ptr(), inp.as_ptr(), inlen);
                    r2(b.as_mut_ptr(), inp.as_ptr(), inlen);
                }
                eq(&format!("sha256(inlen={inlen}, rep={rep})"), &a, &b);
                let mut a = vec![0xA5u8; 64 + 8];
                let mut b = vec![0xA5u8; 64 + 8];
                unsafe {
                    c5(a.as_mut_ptr(), inp.as_ptr(), inlen);
                    r5(b.as_mut_ptr(), inp.as_ptr(), inlen);
                }
                eq(&format!("sha512(inlen={inlen}, rep={rep})"), &a, &b);
            }
        }
    }

    #[test]
    fn row53_sha_incremental() {
        let libs = load();
        let mut rng = Rng::new(53);
        let (i2c, i2r) = libs.pair::<IncInit>("sha256_inc_init");
        let (b2c, b2r) = libs.pair::<IncBlocks>("sha256_inc_blocks");
        let (f2c, f2r) = libs.pair::<IncFinalize>("sha256_inc_finalize");
        let (i5c, i5r) = libs.pair::<IncInit>("sha512_inc_init");
        let (b5c, b5r) = libs.pair::<IncBlocks>("sha512_inc_blocks");
        let (f5c, f5r) = libs.pair::<IncFinalize>("sha512_inc_finalize");
        let (o2c, _) = libs.pair::<Sha>("sha256");
        let (o5c, _) = libs.pair::<Sha>("sha512");

        for &nblocks in &[0usize, 1, 2, 5] {
            for tail in [0usize, 1, 32, 55, 56, 57, 63, 64, 65, 111, 112, 113, 127, 128, 130] {
                // ---- sha256 (block 64, state 40) -------------------------
                let data = rng.bytes(nblocks * 64 + tail);
                let mut sa = [0xA5u8; 40];
                let mut sb = [0xA5u8; 40];
                let mut da = [0x5Au8; 40];
                let mut db = [0x5Au8; 40];
                unsafe {
                    i2c(sa.as_mut_ptr());
                    i2r(sb.as_mut_ptr());
                    eq("sha256_inc_init state", &sa, &sb);
                    if nblocks > 0 {
                        b2c(sa.as_mut_ptr(), data.as_ptr(), nblocks);
                        b2r(sb.as_mut_ptr(), data.as_ptr(), nblocks);
                        eq(
                            &format!("sha256_inc_blocks state (n={nblocks})"),
                            &sa,
                            &sb,
                        );
                    }
                    f2c(
                        da.as_mut_ptr(),
                        sa.as_mut_ptr(),
                        data[nblocks * 64..].as_ptr(),
                        tail,
                    );
                    f2r(
                        db.as_mut_ptr(),
                        sb.as_mut_ptr(),
                        data[nblocks * 64..].as_ptr(),
                        tail,
                    );
                    eq(
                        &format!("sha256_inc_finalize digest (n={nblocks}, tail={tail})"),
                        &da,
                        &db,
                    );
                    eq("sha256_inc_finalize state", &sa, &sb);
                    let mut one = [0u8; 32];
                    o2c(one.as_mut_ptr(), data.as_ptr(), data.len());
                    assert_eq!(&da[..32], &one[..], "sha256 incremental != one-shot");
                }

                // ---- sha512 (block 128, state 72) ------------------------
                let data = rng.bytes(nblocks * 128 + tail);
                let mut sa = [0xA5u8; 72];
                let mut sb = [0xA5u8; 72];
                let mut da = [0x5Au8; 72];
                let mut db = [0x5Au8; 72];
                unsafe {
                    i5c(sa.as_mut_ptr());
                    i5r(sb.as_mut_ptr());
                    eq("sha512_inc_init state", &sa, &sb);
                    if nblocks > 0 {
                        b5c(sa.as_mut_ptr(), data.as_ptr(), nblocks);
                        b5r(sb.as_mut_ptr(), data.as_ptr(), nblocks);
                        eq(
                            &format!("sha512_inc_blocks state (n={nblocks})"),
                            &sa,
                            &sb,
                        );
                    }
                    f5c(
                        da.as_mut_ptr(),
                        sa.as_mut_ptr(),
                        data[nblocks * 128..].as_ptr(),
                        tail,
                    );
                    f5r(
                        db.as_mut_ptr(),
                        sb.as_mut_ptr(),
                        data[nblocks * 128..].as_ptr(),
                        tail,
                    );
                    eq(
                        &format!("sha512_inc_finalize digest (n={nblocks}, tail={tail})"),
                        &da,
                        &db,
                    );
                    eq("sha512_inc_finalize state", &sa, &sb);
                    let mut one = [0u8; 64];
                    o5c(one.as_mut_ptr(), data.as_ptr(), data.len());
                    assert_eq!(&da[..64], &one[..], "sha512 incremental != one-shot");
                }
            }
        }
    }

    #[test]
    fn row54_mgf1() {
        let libs = load();
        let mut rng = Rng::new(54);
        let (mc, mr) = libs.pair::<Mgf1>("SPX_mgf1_256");
        let (mc5, mr5) = libs.pair::<Mgf1>("SPX_mgf1_512");
        for inlen in [0usize, 1, 4, 16, 32, 46, SPX_N + SPX_SHA256_ADDR_BYTES, 100, 200] {
            let inp = rng.bytes(inlen.max(1));
            for outlen in outlens() {
                let mut a = vec![0xA5u8; outlen + 8];
                let mut b = vec![0xA5u8; outlen + 8];
                unsafe {
                    mc(a.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                    mr(b.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                }
                eq(&format!("SPX_mgf1_256(out={outlen}, in={inlen})"), &a, &b);
                let mut a = vec![0xA5u8; outlen + 8];
                let mut b = vec![0xA5u8; outlen + 8];
                unsafe {
                    mc5(a.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                    mr5(b.as_mut_ptr(), outlen as u64, inp.as_ptr(), inlen as u64);
                }
                eq(&format!("SPX_mgf1_512(out={outlen}, in={inlen})"), &a, &b);
            }
        }
    }

    #[test]
    fn row55_seed_state() {
        let libs = load();
        let (fc, fr) = libs.pair::<SeedState>("SPX_seed_state");
        let mut rng = Rng::new(55);
        for rep in 0..64 {
            let ps = match rep {
                0 => vec![0u8; SPX_N],
                1 => vec![0xFFu8; SPX_N],
                _ => rng.bytes(SPX_N),
            };
            let ss = rng.bytes(SPX_N);
            let mut ca = Ctx::new();
            let mut cb = Ctx::new();
            ca.set_seeds(&ps, &ss);
            cb.set_seeds(&ps, &ss);
            unsafe {
                fc(ca.ptr_mut());
                fr(cb.ptr_mut());
            }
            eq("SPX_seed_state spx_ctx", ca.bytes(), cb.bytes());
        }
    }
}

// ===========================================================================
// shake
// ===========================================================================

#[cfg(backend_shake)]
mod shake {
    use super::*;

    type Shake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
    type Absorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type SqueezeBlocks = unsafe extern "C" fn(*mut u8, usize, *mut u64);
    type IncInit = unsafe extern "C" fn(*mut u64);
    type IncAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type IncFinalize = unsafe extern "C" fn(*mut u64);
    type IncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);

    fn state_bytes(s: &[u64]) -> Vec<u8> {
        let mut v = Vec::with_capacity(s.len() * 8);
        for x in s {
            v.extend_from_slice(&x.to_ne_bytes());
        }
        v
    }

    #[test]
    fn row56_shake256_one_shot() {
        let libs = load();
        let mut rng = Rng::new(56);
        let (fc, fr) = libs.pair::<Shake>("shake256");
        for inlen in inlens() {
            for rep in 0..3 {
                let inp = match rep {
                    0 => vec![0u8; inlen],
                    1 => vec![0xFFu8; inlen],
                    _ => rng.bytes(inlen),
                };
                for outlen in [0usize, 1, SPX_N, 32, 135, 136, 137, 272, 300] {
                    let mut a = vec![0xA5u8; outlen + 8];
                    let mut b = vec![0xA5u8; outlen + 8];
                    unsafe {
                        fc(a.as_mut_ptr(), outlen, inp.as_ptr(), inlen);
                        fr(b.as_mut_ptr(), outlen, inp.as_ptr(), inlen);
                    }
                    eq(
                        &format!("shake256(outlen={outlen}, inlen={inlen}, rep={rep})"),
                        &a,
                        &b,
                    );
                }
            }
        }
    }

    #[test]
    fn row57_shake256_absorb_squeezeblocks() {
        let libs = load();
        let mut rng = Rng::new(57);
        let (ac, ar) = libs.pair::<Absorb>("shake256_absorb");
        let (sc, sr) = libs.pair::<SqueezeBlocks>("shake256_squeezeblocks");
        for inlen in inlens() {
            let inp = rng.bytes(inlen);
            let mut sa = vec![0xDEAD_BEEF_DEAD_BEEFu64; 25];
            let mut sb = sa.clone();
            unsafe {
                ac(sa.as_mut_ptr(), inp.as_ptr(), inlen);
                ar(sb.as_mut_ptr(), inp.as_ptr(), inlen);
            }
            eq(
                &format!("shake256_absorb state (inlen={inlen})"),
                &state_bytes(&sa),
                &state_bytes(&sb),
            );
            for nblocks in 0..4usize {
                let mut a = vec![0xA5u8; nblocks * 136 + 8];
                let mut b = vec![0xA5u8; nblocks * 136 + 8];
                unsafe {
                    sc(a.as_mut_ptr(), nblocks, sa.as_mut_ptr());
                    sr(b.as_mut_ptr(), nblocks, sb.as_mut_ptr());
                }
                eq(
                    &format!("shake256_squeezeblocks(n={nblocks}, inlen={inlen})"),
                    &a,
                    &b,
                );
                eq(
                    "shake256_squeezeblocks state",
                    &state_bytes(&sa),
                    &state_bytes(&sb),
                );
            }
        }
    }

    #[test]
    fn row58_shake256_incremental() {
        let libs = load();
        let mut rng = Rng::new(58);
        let (ic, ir) = libs.pair::<IncInit>("shake256_inc_init");
        let (ac, ar) = libs.pair::<IncAbsorb>("shake256_inc_absorb");
        let (nc, nr) = libs.pair::<IncFinalize>("shake256_inc_finalize");
        let (qc, qr) = libs.pair::<IncSqueeze>("shake256_inc_squeeze");
        for total in [0usize, 1, 67, 135, 136, 137, 200, 271, 272, 273, 500] {
            let data = rng.bytes(total);
            for nchunks in 1..=4usize {
                let mut bounds = vec![0usize];
                for k in 1..nchunks {
                    bounds.push(total * k / nchunks);
                }
                bounds.push(total);
                let mut sa = vec![0xAAAA_AAAA_AAAA_AAAAu64; 26];
                let mut sb = sa.clone();
                unsafe {
                    ic(sa.as_mut_ptr());
                    ir(sb.as_mut_ptr());
                    eq("shake256_inc_init", &state_bytes(&sa), &state_bytes(&sb));
                    for w in bounds.windows(2) {
                        let (s, e) = (w[0], w[1]);
                        ac(sa.as_mut_ptr(), data[s..].as_ptr(), e - s);
                        ar(sb.as_mut_ptr(), data[s..].as_ptr(), e - s);
                        eq(
                            &format!("shake256_inc_absorb state ({s}..{e} of {total})"),
                            &state_bytes(&sa),
                            &state_bytes(&sb),
                        );
                    }
                    nc(sa.as_mut_ptr());
                    nr(sb.as_mut_ptr());
                    eq(
                        "shake256_inc_finalize state",
                        &state_bytes(&sa),
                        &state_bytes(&sb),
                    );
                    for &out in &[1usize, 31, 136, 137, 60] {
                        let mut a = vec![0xA5u8; out + 8];
                        let mut b = vec![0xA5u8; out + 8];
                        qc(a.as_mut_ptr(), out, sa.as_mut_ptr());
                        qr(b.as_mut_ptr(), out, sb.as_mut_ptr());
                        eq(&format!("shake256_inc_squeeze({out})"), &a, &b);
                        eq(
                            "shake256_inc_squeeze state",
                            &state_bytes(&sa),
                            &state_bytes(&sb),
                        );
                    }
                }
            }
        }
    }
}

// ===========================================================================
// haraka
// ===========================================================================

#[cfg(backend_haraka)]
mod haraka {
    use super::*;

    type TweakConstants = unsafe extern "C" fn(*mut u8);
    type Perm = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
    type HarakaS = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8);
    type IncInit = unsafe extern "C" fn(*mut u8);
    type IncAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8);
    type IncFinalize = unsafe extern "C" fn(*mut u8);
    type IncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const u8);

    #[test]
    fn row59_tweak_constants() {
        let libs = load();
        let (fc, fr) = libs.pair::<TweakConstants>("SPX_tweak_constants");
        let mut rng = Rng::new(59);
        for rep in 0..32 {
            let ps = match rep {
                0 => vec![0u8; SPX_N],
                1 => vec![0xFFu8; SPX_N],
                _ => rng.bytes(SPX_N),
            };
            let ss = rng.bytes(SPX_N);
            let mut ca = Ctx::new();
            let mut cb = Ctx::new();
            ca.set_seeds(&ps, &ss);
            cb.set_seeds(&ps, &ss);
            unsafe {
                fc(ca.ptr_mut());
                fr(cb.ptr_mut());
            }
            eq("SPX_tweak_constants spx_ctx", ca.bytes(), cb.bytes());
        }
    }

    #[test]
    fn row60_haraka_perm_and_blocks() {
        let libs = load();
        let mut rng = Rng::new(60);
        let (pc, pr) = libs.pair::<Perm>("SPX_haraka512_perm");
        let (h5c, h5r) = libs.pair::<Perm>("SPX_haraka512");
        let (h2c, h2r) = libs.pair::<Perm>("SPX_haraka256");
        let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
        for rep in 0..64 {
            let inp64 = match rep {
                0 => vec![0u8; 64],
                1 => vec![0xFFu8; 64],
                _ => rng.bytes(64),
            };
            let mut a = vec![0xA5u8; 64 + 8];
            let mut b = vec![0xA5u8; 64 + 8];
            unsafe {
                pc(a.as_mut_ptr(), inp64.as_ptr(), cc.ptr());
                pr(b.as_mut_ptr(), inp64.as_ptr(), cr.ptr());
            }
            eq("SPX_haraka512_perm", &a, &b);

            let mut a = vec![0xA5u8; 32 + 8];
            let mut b = vec![0xA5u8; 32 + 8];
            unsafe {
                h5c(a.as_mut_ptr(), inp64.as_ptr(), cc.ptr());
                h5r(b.as_mut_ptr(), inp64.as_ptr(), cr.ptr());
            }
            eq("SPX_haraka512", &a, &b);

            let inp32 = &inp64[..32];
            let mut a = vec![0xA5u8; 32 + 8];
            let mut b = vec![0xA5u8; 32 + 8];
            unsafe {
                h2c(a.as_mut_ptr(), inp32.as_ptr(), cc.ptr());
                h2r(b.as_mut_ptr(), inp32.as_ptr(), cr.ptr());
            }
            eq("SPX_haraka256", &a, &b);
        }
    }

    #[test]
    fn row61_haraka_s_one_shot() {
        let libs = load();
        let mut rng = Rng::new(61);
        let (fc, fr) = libs.pair::<HarakaS>("SPX_haraka_S");
        let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
        for inlen in inlens() {
            let inp = rng.bytes(inlen);
            for outlen in [0usize, 1, 31, 32, 33, 64, 65, 100] {
                let mut a = vec![0xA5u8; outlen + 8];
                let mut b = vec![0xA5u8; outlen + 8];
                unsafe {
                    fc(
                        a.as_mut_ptr(),
                        outlen as u64,
                        inp.as_ptr(),
                        inlen as u64,
                        cc.ptr(),
                    );
                    fr(
                        b.as_mut_ptr(),
                        outlen as u64,
                        inp.as_ptr(),
                        inlen as u64,
                        cr.ptr(),
                    );
                }
                eq(
                    &format!("SPX_haraka_S(outlen={outlen}, inlen={inlen})"),
                    &a,
                    &b,
                );
            }
        }
    }

    #[test]
    fn row62_haraka_s_incremental() {
        let libs = load();
        let mut rng = Rng::new(62);
        let (ic, ir) = libs.pair::<IncInit>("SPX_haraka_S_inc_init");
        let (ac, ar) = libs.pair::<IncAbsorb>("SPX_haraka_S_inc_absorb");
        let (nc, nr) = libs.pair::<IncFinalize>("SPX_haraka_S_inc_finalize");
        let (qc, qr) = libs.pair::<IncSqueeze>("SPX_haraka_S_inc_squeeze");
        let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
        for total in [0usize, 1, 31, 32, 33, 63, 64, 65, 100, 200] {
            let data = rng.bytes(total);
            for nchunks in 1..=4usize {
                let mut bounds = vec![0usize];
                for k in 1..nchunks {
                    bounds.push(total * k / nchunks);
                }
                bounds.push(total);
                let mut sa = [0xA5u8; 65];
                let mut sb = [0xA5u8; 65];
                unsafe {
                    ic(sa.as_mut_ptr());
                    ir(sb.as_mut_ptr());
                    eq("SPX_haraka_S_inc_init", &sa, &sb);
                    for w in bounds.windows(2) {
                        let (s, e) = (w[0], w[1]);
                        ac(sa.as_mut_ptr(), data[s..].as_ptr(), e - s, cc.ptr());
                        ar(sb.as_mut_ptr(), data[s..].as_ptr(), e - s, cr.ptr());
                        eq(
                            &format!("SPX_haraka_S_inc_absorb ({s}..{e} of {total})"),
                            &sa,
                            &sb,
                        );
                    }
                    nc(sa.as_mut_ptr());
                    nr(sb.as_mut_ptr());
                    eq("SPX_haraka_S_inc_finalize", &sa, &sb);
                    for &out in &[1usize, 31, 32, 33, 64, 7] {
                        let mut a = vec![0xA5u8; out + 8];
                        let mut b = vec![0xA5u8; out + 8];
                        qc(a.as_mut_ptr(), out, sa.as_mut_ptr(), cc.ptr());
                        qr(b.as_mut_ptr(), out, sb.as_mut_ptr(), cr.ptr());
                        eq(&format!("SPX_haraka_S_inc_squeeze({out})"), &a, &b);
                        eq("SPX_haraka_S_inc_squeeze state", &sa, &sb);
                    }
                }
            }
        }
    }
}
