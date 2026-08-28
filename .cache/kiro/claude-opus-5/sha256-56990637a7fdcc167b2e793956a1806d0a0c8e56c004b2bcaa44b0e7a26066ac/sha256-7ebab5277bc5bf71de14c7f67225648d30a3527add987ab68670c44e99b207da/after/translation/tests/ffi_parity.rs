//! Differential tests: every exported function is driven through `dlopen`ed
//! symbols on *both* the C reference build and the Rust `cdylib`, so the
//! `#[no_mangle]` wrappers are exercised exactly as an external caller sees
//! them. Ordered lowest-level first: pack -> addsample -> update_md5.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Level 0: tflac_pack_u64le
// ---------------------------------------------------------------------------

fn pack_case(c: &Impl, r: &Impl, n: u64, seed: u64, case: &str) {
    // 32-byte scratch so a stray write past the 8 documented bytes is caught.
    let base = fresh_arena(seed);
    let mut bc = base[..32].to_vec();
    let mut br = base[..32].to_vec();
    c.pack_u64le(&mut bc, n);
    r.pack_u64le(&mut br, n);
    assert_arenas_eq(&bc, &br, case);
    // Independent oracle: the C contract is plain little-endian.
    assert_eq!(&bc[..8], &n.to_le_bytes(), "{case}: not little-endian");
}

#[test]
fn pack_u64le_matches() {
    let (c, r) = load_pair();

    for (i, n) in [
        0u64,
        1,
        0xFF,
        0x100,
        u64::MAX,
        u64::MAX - 1,
        0x0102_0304_0506_0708,
        0x8000_0000_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
        0xDEAD_BEEF_CAFE_BABE,
        u32::MAX as u64,
        u32::MAX as u64 + 1,
    ]
    .into_iter()
    .enumerate()
    {
        pack_case(&c, &r, n, 0x1000 + i as u64, &format!("pack fixed n={n:#x}"));
    }

    // Every single set bit, to confirm each of the eight shifts.
    for bit in 0..64 {
        let n = 1u64 << bit;
        pack_case(&c, &r, n, 0x2000 + bit, &format!("pack bit {bit}"));
    }

    let mut rng = Rng::new(0xA5A5_1234);
    for i in 0..5000 {
        let n = rng.next_u64();
        pack_case(&c, &r, n, 0x3000 + i, &format!("pack rand #{i} n={n:#x}"));
    }
}

// ---------------------------------------------------------------------------
// Level 1: tflac_md5_addsample
// ---------------------------------------------------------------------------

struct Md5State {
    pos: u32,
    total: u64,
    buffer: [u8; MD5_BUFFER_LEN],
}

impl Md5State {
    fn random(rng: &mut Rng) -> Md5State {
        let mut buffer = [0u8; MD5_BUFFER_LEN];
        for b in buffer.iter_mut() {
            *b = (rng.next_u32() & 0xFF) as u8;
        }
        Md5State {
            pos: rng.next_u32(),
            total: rng.next_u64(),
            buffer,
        }
    }

    fn into_arena(self, seed: u64) -> Vec<u8> {
        let mut a = fresh_arena(seed);
        write_u32(&mut a, MD5_POS_OFF, self.pos);
        write_u64(&mut a, MD5_TOTAL_OFF, self.total);
        a[MD5_BUFFER_OFF..MD5_BUFFER_OFF + MD5_BUFFER_LEN].copy_from_slice(&self.buffer);
        // Bytes 4..8 are the C struct's tail padding; keep them equal but
        // non-zero so nobody can accidentally rely on them.
        a
    }
}

fn addsample_case(c: &Impl, r: &Impl, st: Md5State, bits: u32, val: u64, seed: u64, case: &str) {
    let mut ac = st.into_arena(seed);
    let mut ar = ac.clone();
    c.md5_addsample(&mut ac, bits, val);
    r.md5_addsample(&mut ar, bits, val);
    assert_arenas_eq(&ac, &ar, case);
}

#[test]
fn md5_addsample_matches_structured() {
    let (c, r) = load_pair();

    // Exhaustive over every `pos` byte value plus a spread of bit counts. This
    // covers the pos%64 wrap, the `pos >= 64` branch and the backwards
    // `while (bytes--)` copy loop (including its reads past buffer[71]).
    let bit_counts: [u32; 14] = [
        0, 1, 7, 8, 16, 24, 32, 64, 65, 128, 256, 512, 4096, 8 * 8,
    ];
    let mut n = 0u64;
    for pos in 0u32..=255 {
        for &bits in &bit_counts {
            let st = Md5State {
                pos,
                total: 0x0102_0304_0506_0708u64.wrapping_mul(pos as u64 + 1),
                buffer: core::array::from_fn(|i| (i as u8).wrapping_mul(3).wrapping_add(pos as u8)),
            };
            addsample_case(
                &c,
                &r,
                st,
                bits,
                0xF0E1_D2C3_B4A5_9687u64 ^ (bits as u64),
                0x4000 + n,
                &format!("addsample pos={pos} bits={bits}"),
            );
            n += 1;
        }
    }

    // Boundary `pos` values around the u32 wrap, and boundary `bits` values.
    for &pos in &[
        0u32,
        63,
        64,
        65,
        71,
        72,
        127,
        128,
        u32::MAX - 8,
        u32::MAX - 1,
        u32::MAX,
    ] {
        for &bits in &[0u32, 8, 64, u32::MAX - 7, u32::MAX, u32::MAX / 2] {
            for &total in &[0u64, u64::MAX, u64::MAX - 63, 1] {
                let st = Md5State {
                    pos,
                    total,
                    buffer: core::array::from_fn(|i| (0xA0 ^ i as u8)),
                };
                addsample_case(
                    &c,
                    &r,
                    st,
                    bits,
                    u64::MAX,
                    0x5000 + n,
                    &format!("addsample edge pos={pos} bits={bits} total={total}"),
                );
                n += 1;
            }
        }
    }
}

#[test]
fn md5_addsample_matches_fuzz() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0xBEEF_0001);

    for i in 0..20_000u64 {
        let mut st = Md5State::random(&mut rng);
        // Mix fully-random `pos` with realistic small ones so both the common
        // path and the wrap-around path get plenty of coverage.
        if i % 2 == 0 {
            st.pos = rng.below(200);
        }
        let bits = if i % 3 == 0 { 64 } else { rng.next_u32() };
        let val = rng.next_u64();
        addsample_case(
            &c,
            &r,
            st,
            bits,
            val,
            0x6000 + i,
            &format!("addsample fuzz #{i} bits={bits}"),
        );
    }
}

// ---------------------------------------------------------------------------
// Level 2: update_md5
// ---------------------------------------------------------------------------

fn update_case(
    c: &Impl,
    r: &Impl,
    md5: Md5State,
    cur_blocksize: u32,
    channels: u32,
    samples: &[i32],
    seed: u64,
    case: &str,
) {
    let mut ac = md5.into_arena(seed);
    write_u32(&mut ac, TFLAC_CUR_BLOCKSIZE_OFF, cur_blocksize);
    write_u32(&mut ac, TFLAC_CHANNELS_OFF, channels);
    let mut ar = ac.clone();

    let rc = c.update_md5(&mut ac, samples);
    let rr = r.update_md5(&mut ar, samples);

    assert_eq!(rc, rr, "{case}: return value C={rc} Rust={rr}");
    assert_arenas_eq(&ac, &ar, case);

    // Oracle for the return value: b = cur_blocksize*channels - 5*8, wrapping.
    let expect = cur_blocksize
        .wrapping_mul(channels)
        .wrapping_sub(5 * 8);
    assert_eq!(rc, expect, "{case}: return value disagrees with model");
}

fn random_samples(rng: &mut Rng, len: usize) -> Vec<i32> {
    (0..len).map(|_| rng.next_i32()).collect()
}

#[test]
fn update_md5_matches_structured() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0xC0FFEE_11);

    // `samples` must cover index 135: the loop strides 32 elements per
    // iteration while only reading 8, so slots 8..31 of each stride are
    // never touched. Fill everything anyway; a divergence in stride would
    // then show up as a value mismatch rather than an OOB read.
    let samples = random_samples(&mut rng, UPDATE_MD5_SAMPLES_READ + 64);

    let mut n = 0u64;
    for pos in 0u32..=127 {
        let st = Md5State {
            pos,
            total: (pos as u64).wrapping_mul(0x1111_2222_3333_4444),
            buffer: core::array::from_fn(|i| (i as u8).wrapping_add(pos as u8)),
        };
        update_case(
            &c,
            &r,
            st,
            4096,
            2,
            &samples,
            0x7000 + n,
            &format!("update pos={pos}"),
        );
        n += 1;
    }

    // blocksize/channels combinations, including ones that underflow `b`.
    for &(bs, ch) in &[
        (0u32, 0u32),
        (1, 1),
        (5, 1),
        (4096, 2),
        (4096, 8),
        (u32::MAX, 1),
        (1, u32::MAX),
        (0xFFFF, 0xFFFF),
        (40, 1),
        (39, 1),
        (8, 1),
    ] {
        let st = Md5State {
            pos: 0,
            total: 0,
            buffer: [0x5A; MD5_BUFFER_LEN],
        };
        update_case(
            &c,
            &r,
            st,
            bs,
            ch,
            &samples,
            0x8000 + n,
            &format!("update bs={bs} ch={ch}"),
        );
        n += 1;
    }

    // Sign extension: `(tflac_uint)samples[i]` sign-extends before `& 0xFF`.
    for &v in &[
        -1i32,
        i32::MIN,
        i32::MAX,
        -256,
        -255,
        0x0000_0080u32 as i32,
        -128,
    ] {
        let all = vec![v; UPDATE_MD5_SAMPLES_READ + 64];
        let st = Md5State {
            pos: 0,
            total: 0,
            buffer: [0; MD5_BUFFER_LEN],
        };
        update_case(
            &c,
            &r,
            st,
            64,
            1,
            &all,
            0x9000 + n,
            &format!("update sign v={v}"),
        );
        n += 1;
    }

    // Distinct value per slot so a wrong stride cannot alias to the right answer.
    let ramp: Vec<i32> = (0..(UPDATE_MD5_SAMPLES_READ + 64) as i32)
        .map(|i| i.wrapping_mul(0x0101_0101))
        .collect();
    let st = Md5State {
        pos: 3,
        total: 7,
        buffer: [0xFF; MD5_BUFFER_LEN],
    };
    update_case(&c, &r, st, 4096, 2, &ramp, 0xA000, "update ramp");
}

#[test]
fn update_md5_matches_fuzz() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0xD00D_5EED);

    for i in 0..5_000u64 {
        let mut st = Md5State::random(&mut rng);
        if i % 2 == 0 {
            st.pos = rng.below(200);
        }
        let (bs, ch) = if i % 4 == 0 {
            (rng.next_u32(), rng.next_u32())
        } else {
            (rng.below(8192), rng.below(9) + 1)
        };
        let samples = random_samples(&mut rng, UPDATE_MD5_SAMPLES_READ + 32);
        update_case(
            &c,
            &r,
            st,
            bs,
            ch,
            &samples,
            0xB000 + i,
            &format!("update fuzz #{i}"),
        );
    }
}

// ---------------------------------------------------------------------------
// Sequential / stateful use: repeated calls threading the same context
// ---------------------------------------------------------------------------

#[test]
fn update_md5_sequential_state_matches() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0xFEED_FACE);

    for trial in 0..200u64 {
        let st = Md5State {
            pos: rng.below(72),
            total: rng.next_u64() >> 8,
            buffer: core::array::from_fn(|_| (rng.next_u32() & 0xFF) as u8),
        };
        let mut ac = st.into_arena(0xC000 + trial);
        write_u32(&mut ac, TFLAC_CUR_BLOCKSIZE_OFF, rng.below(4096) + 1);
        write_u32(&mut ac, TFLAC_CHANNELS_OFF, rng.below(8) + 1);
        let mut ar = ac.clone();

        for step in 0..12 {
            let samples = random_samples(&mut rng, UPDATE_MD5_SAMPLES_READ + 8);
            let rc = c.update_md5(&mut ac, &samples);
            let rr = r.update_md5(&mut ar, &samples);
            assert_eq!(rc, rr, "trial {trial} step {step}: return mismatch");
            assert_arenas_eq(&ac, &ar, &format!("trial {trial} step {step}"));
        }

        // Also thread addsample calls through the same live context.
        for step in 0..12 {
            let bits = if step % 2 == 0 { 64 } else { rng.next_u32() };
            let val = rng.next_u64();
            c.md5_addsample(&mut ac, bits, val);
            r.md5_addsample(&mut ar, bits, val);
            assert_arenas_eq(&ac, &ar, &format!("trial {trial} addsample step {step}"));
        }

        assert_eq!(read_u32(&ac, MD5_POS_OFF), read_u32(&ar, MD5_POS_OFF));
        assert_eq!(read_u64(&ac, MD5_TOTAL_OFF), read_u64(&ar, MD5_TOTAL_OFF));
    }
}
