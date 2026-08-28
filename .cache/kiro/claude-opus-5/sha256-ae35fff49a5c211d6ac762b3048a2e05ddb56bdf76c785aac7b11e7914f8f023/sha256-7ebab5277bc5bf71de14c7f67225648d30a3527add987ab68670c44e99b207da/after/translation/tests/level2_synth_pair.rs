//! Level 2: full `synth_pair` — all 23 taps active, accumulation order,
//! the `z += 2` pointer bump, and the `pcm[16 * nch]` store index.

mod common;
use common::*;

/// Randomised full-buffer comparison across a wide range of magnitudes.
#[test]
fn synth_pair_random_full_buffers() {
    let p = Pair::load();
    let mut rng = Rng(0x1234_5678_9ABC_DEF1);

    // Scales chosen so the accumulator lands below, around, and far beyond the
    // clipping thresholds.
    let scales: [f32; 9] = [
        0.0, 1e-8, 1e-4, 0.01, 0.19, 0.5, 1.0, 100.0, 1e6,
    ];

    for &scale in &scales {
        for _ in 0..400 {
            // Fill the *entire* buffer with noise: guarantees that only the
            // documented taps are read (any extra read would diverge).
            let mut z: Vec<f32> = (0..Z_LEN).map(|_| rng.unit() * scale).collect();
            // Occasionally zero a tap to vary the exact accumulation path.
            if rng.next_u32() & 3 == 0 {
                let t = TAPS[(rng.next_u32() as usize) % TAPS.len()];
                z[t] = 0.0;
            }
            for nch in [1, 2] {
                p.check(&z, nch, "random full");
            }
        }
    }
}

/// Random *bit patterns* (including NaNs, infinities, subnormals and huge
/// exponents) on every tap.
#[test]
fn synth_pair_random_bit_patterns() {
    let p = Pair::load();
    let mut rng = Rng(0xDEAD_BEEF_CAFE_0001);
    for _ in 0..3000 {
        let mut z = vec![0.0f32; Z_LEN];
        for &t in TAPS.iter().chain(TAPS2.iter()) {
            z[t] = f32::from_bits(rng.next_u32());
        }
        p.check(&z, 1, "random bits");
    }
}

/// One tap at a time: isolates each weight and each accumulation step so a
/// wrong coefficient or a wrong index cannot hide behind the other terms.
#[test]
fn synth_pair_single_tap_isolation() {
    let p = Pair::load();
    let magnitudes: [f32; 11] = [
        1.0, -1.0, 0.5, -0.5, 1e-3, -1e-3, 12345.0, -12345.0, 1.0 / 29.0, 1e9, -1e9,
    ];
    // Sweep every index that could plausibly be read, not just the known taps:
    // if the Rust code read a wrong offset, the C/Rust outputs would diverge
    // for the index it reads.
    for idx in 0..=900usize {
        for &m in &magnitudes {
            let mut z = vec![0.0f32; Z_LEN];
            z[idx] = m;
            p.check(&z, 1, "single tap");
        }
    }
}

/// Pairwise tap interaction: verifies the exact `+`/`-` pairing inside each
/// parenthesised group, e.g. `(z[14*64] - z[0]) * 29`.
#[test]
fn synth_pair_tap_pairs() {
    let p = Pair::load();
    let all: Vec<usize> = TAPS.iter().chain(TAPS2.iter()).copied().collect();
    let vals: [(f32, f32); 6] = [
        (1.0, 1.0),
        (1.0, -1.0),
        (-1.0, 1.0),
        (1000.0, 1000.0),
        (1e-7, 1e7),
        (0.3333333, -0.6666667),
    ];
    for (i, &a) in all.iter().enumerate() {
        for &b in &all[i + 1..] {
            for &(va, vb) in &vals {
                let mut z = vec![0.0f32; Z_LEN];
                z[a] = va;
                z[b] = vb;
                p.check(&z, 1, "tap pair");
            }
        }
    }
}

/// `pcm[16 * nch]` — the store index must match for every channel count.
#[test]
fn synth_pair_nch_variants() {
    let p = Pair::load();
    let mut rng = Rng(0x0BAD_F00D_0000_0001);
    for nch in 0..=15i32 {
        for _ in 0..60 {
            let z: Vec<f32> = (0..Z_LEN).map(|_| rng.unit() * 0.3).collect();
            p.check(&z, nch, "nch variant");
        }
        // Deterministic non-zero case as well.
        let mut z = vec![0.0f32; Z_LEN];
        z[448] = 0.25;
        z[2] = -1000.0;
        p.check(&z, nch, "nch deterministic");
    }
}

/// `nch == 0` makes both stores target `pcm[0]`; the second write must win.
#[test]
fn synth_pair_nch_zero_aliases_pcm0() {
    let p = Pair::load();
    let mut z = vec![0.0f32; Z_LEN];
    z[448] = 1.0; // first accumulator saturates high
    z[2] = 100.0; // second accumulator = fl(100 * -5) = -500
    let (c, r) = p.run(&z, 0);
    assert_eq!(c, r, "nch=0 aliasing mismatch");
    assert_eq!(c[0], -500, "second store must overwrite pcm[0]");
    // And the other direction.
    let mut z = vec![0.0f32; Z_LEN];
    z[448] = 1.0;
    z[2] = -1e9; // clips high
    let (c, r) = p.run(&z, 0);
    assert_eq!(c, r, "nch=0 aliasing mismatch 2");
    assert_eq!(c[0], i16::MAX);
}

/// Only `pcm[0]` and `pcm[16*nch]` may be written — every other slot must keep
/// the fill sentinel, identically in both implementations.
#[test]
fn synth_pair_writes_only_two_slots() {
    let p = Pair::load();
    let mut rng = Rng(0x5151_5151_0000_0007);
    for nch in [1, 2, 3, 8] {
        let z: Vec<f32> = (0..Z_LEN).map(|_| rng.unit() * 5.0).collect();
        let (c, r) = p.run(&z, nch);
        assert_eq!(c, r);
        for (i, &v) in c.iter().enumerate() {
            if i == 0 || i == (16 * nch) as usize {
                continue;
            }
            assert_eq!(v, PCM_FILL, "pcm[{i}] clobbered (nch={nch})");
        }
    }
}

/// Sequential calls with a shifting window, mimicking how the real MP3
/// synthesis loop reuses one `z` buffer and one `pcm` buffer.
#[test]
fn synth_pair_sequential_window_walk() {
    let p = Pair::load();
    let mut rng = Rng(0xABCD_0123_4567_89AB);
    let big: Vec<f32> = (0..Z_LEN + 64).map(|_| rng.unit() * 0.4).collect();
    for shift in 0..64usize {
        let window = &big[shift..shift + Z_LEN];
        for nch in [1, 2] {
            p.check(window, nch, "window walk");
        }
    }
}

/// Denormal / gradual-underflow behaviour: multiplying tiny taps by the large
/// integer weights must round identically.
#[test]
fn synth_pair_subnormal_inputs() {
    let p = Pair::load();
    let mut rng = Rng(0x7777_1111_2222_3333);
    for _ in 0..2000 {
        let mut z = vec![0.0f32; Z_LEN];
        for &t in TAPS.iter().chain(TAPS2.iter()) {
            // random subnormal / tiny-normal magnitudes
            let mant = rng.next_u32() & 0x007F_FFFF;
            let sign = (rng.next_u32() & 1) << 31;
            let exp = (rng.next_u32() % 4) << 23; // exponents 0..3
            z[t] = f32::from_bits(sign | exp | mant);
        }
        p.check(&z, 2, "subnormal");
    }
}

/// Catastrophic-cancellation cases: terms that nearly cancel expose any
/// difference in accumulation order or in intermediate precision (e.g. if one
/// side contracted a multiply-add into an FMA).
#[test]
fn synth_pair_cancellation() {
    let p = Pair::load();
    let mut rng = Rng(0x0F0F_0F0F_1234_5678);
    for _ in 0..4000 {
        let mut z = vec![0.0f32; Z_LEN];
        let base = rng.unit() * 1e5;
        // Pair each tap with a near-equal partner so the sum is tiny relative
        // to the individual products.
        z[896] = base;            // weight  29
        z[0] = base;              // weight -29  -> cancels
        z[64] = base;             // weight 213
        z[832] = -base;           // weight 213  -> cancels
        z[768] = base;            // weight 459
        z[128] = base;            // weight -459 -> cancels
        z[448] = rng.unit() * 1e-3 * base;
        // second half: -9975 vs 9727 etc.
        z[642] = base;
        z[514] = base * (9975.0 / 64019.0);
        p.check(&z, 1, "cancellation");
    }
}

/// Exhaustive-ish stress: many iterations of fully random buffers with random
/// `nch`, as a final safety net.
#[test]
fn synth_pair_stress() {
    let p = Pair::load();
    let mut rng = Rng(0xFEED_FACE_DEAD_BEEF);
    for _ in 0..20_000 {
        let mut z = vec![0.0f32; Z_LEN];
        let exp_bias = rng.next_u32() % 60;
        for &t in TAPS.iter().chain(TAPS2.iter()) {
            let sign = (rng.next_u32() & 1) << 31;
            let exp = (100 + exp_bias) << 23;
            let mant = rng.next_u32() & 0x007F_FFFF;
            z[t] = f32::from_bits(sign | exp | mant);
        }
        let nch = (rng.next_u32() % 8) as i32;
        p.check(&z, nch, "stress");
    }
}
