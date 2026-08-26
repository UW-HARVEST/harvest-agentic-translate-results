//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH `.so` files via `libloading` and compares
//! `(return value, full 32-byte struct post-state)` byte-for-byte across many
//! randomised inputs drawn from a fixed-seed PRNG.

mod common;

use common::{load_pair, Bw, Pair, Rng, UINT_BITS};

/// How many randomised inputs each row gets by default.
const N: u32 = 20_000;

// ---------------------------------------------------------------------------
// Row 1 — struct ABI parity
// ---------------------------------------------------------------------------

/// The Rust `#[repr(C)]` struct must have exactly the C layout, otherwise every
/// other row would be comparing different fields. Probe it from the outside:
/// fill the struct with a distinctive pattern, ask each `.so` to mutate it, and
/// require the mutated byte-sets to be identical.
#[test]
fn row01_struct_abi_layout() {
    // Sanity: our mirror struct matches the documented C layout.
    assert_eq!(std::mem::size_of::<Bw>(), 32, "sizeof(struct tflac_bitwriter)");
    assert_eq!(std::mem::align_of::<Bw>(), 8, "alignof(struct tflac_bitwriter)");
    let probe = Bw::zeroed();
    let base = &probe as *const Bw as usize;
    assert_eq!(&probe.val as *const _ as usize - base, 0, "offsetof(val)");
    assert_eq!(&probe.bits as *const _ as usize - base, 8, "offsetof(bits)");
    assert_eq!(&probe.pos as *const _ as usize - base, 12, "offsetof(pos)");
    assert_eq!(&probe.len as *const _ as usize - base, 16, "offsetof(len)");
    assert_eq!(&probe.tot as *const _ as usize - base, 20, "offsetof(tot)");
    assert_eq!(&probe.buffer as *const _ as usize - base, 24, "offsetof(buffer)");

    let p = load_pair();

    // A pattern where every byte is distinct, so any layout shift shows up.
    let pattern = Bw {
        val: 0x0102_0304_0506_0708,
        bits: 0x0B0A_0910,
        pos: 0x1413_1211,
        len: 0x1817_1615,
        tot: 0x1C1B_1A19,
        buffer: 0x2423_2221_2019_1E1Du64 as *mut u8,
    };

    for &(bits, val) in &[(0u32, 0u64), (1, 1), (13, 0xDEAD_BEEF), (64, u64::MAX), (200, 7)] {
        let (c, r) = p.call(&pattern, bits, val);
        let before = pattern.bytes();
        let cdirty: Vec<usize> = (0..32).filter(|&i| c.post[i] != before[i]).collect();
        let rdirty: Vec<usize> = (0..32).filter(|&i| r.post[i] != before[i]).collect();
        assert_eq!(
            cdirty, rdirty,
            "the two implementations mutate different byte offsets \
             (bits={bits}, val={val:#x}): C touched {cdirty:?}, Rust touched {rdirty:?}"
        );
        // Bytes 12..20 (`pos`, `len`) and 24..32 (`buffer`) must never be touched.
        for i in (12..20).chain(24..32) {
            assert!(!cdirty.contains(&i), "C unexpectedly wrote byte {i}");
            assert!(!rdirty.contains(&i), "Rust unexpectedly wrote byte {i}");
        }
        assert_eq!(c, r, "post-state mismatch for bits={bits} val={val:#x}");
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a pre-state with a given `bw->bits`, randomising everything else.
fn pre_with_bwbits(rng: &mut Rng, bwbits: u32) -> Bw {
    Bw {
        val: rng.interesting_u64(),
        bits: bwbits,
        pos: rng.next_u32(),
        len: rng.next_u32(),
        tot: rng.next_u32(),
        buffer: rng.next_u64() as *mut u8,
    }
}

/// Drive `n` randomised cases through a row-specific case generator.
fn sweep<F>(ctx: &str, seed: u64, n: u32, mut gen: F)
where
    F: FnMut(&mut Rng) -> (Bw, u32, u64),
{
    let p: Pair = load_pair();
    let mut rng = Rng::new(seed);
    for _ in 0..n {
        let (pre, bits, val) = gen(&mut rng);
        p.assert_same(ctx, &pre, bits, val);
    }
}

// ---------------------------------------------------------------------------
// Rows 2–6 — the `bw->bits + bits` guard boundary
// ---------------------------------------------------------------------------

/// Row 2 — mid-range `bits`, empty writer, loop not entered.
#[test]
fn row02_midrange_bits_empty_writer() {
    sweep("row02 bits=1..62, bw->bits=0", 0x0202_0202, N, |rng| {
        let bits = rng.range(1, 62);
        let mut pre = pre_with_bwbits(rng, 0);
        pre.val = 0; // an empty writer really starts at val=0
        (pre, bits, rng.interesting_u64())
    });
}

/// Row 3 — accumulate into a partially filled writer, still below the guard.
#[test]
fn row03_partially_filled_below_guard() {
    sweep("row03 bw->bits+bits < 64", 0x0303_0303, N, |rng| {
        let bwbits = rng.range(0, 62);
        let bits = rng.range(0, 63 - bwbits); // sum <= 63
        let pre = pre_with_bwbits(rng, bwbits);
        (pre, bits, rng.interesting_u64())
    });
}

/// Row 4 — `bw->bits + bits == 63`, one below the guard (loop NOT entered).
#[test]
fn row04_sum_exactly_63() {
    sweep("row04 bw->bits+bits == 63", 0x0404_0404, N, |rng| {
        let bits = rng.range(0, 63);
        let pre = pre_with_bwbits(rng, 63 - bits);
        (pre, bits, rng.interesting_u64())
    });
}

/// Row 5 — `bw->bits + bits == 64`, exactly at the guard (loop entered).
#[test]
fn row05_sum_exactly_64() {
    sweep("row05 bw->bits+bits == 64", 0x0505_0505, N, |rng| {
        let bits = rng.range(0, 64);
        let pre = pre_with_bwbits(rng, 64 - bits);
        (pre, bits, rng.interesting_u64())
    });
}

/// Row 6 — `bw->bits + bits == 65`, one past the guard.
#[test]
fn row06_sum_exactly_65() {
    sweep("row06 bw->bits+bits == 65", 0x0606_0606, N, |rng| {
        let bits = rng.range(1, 65);
        let pre = pre_with_bwbits(rng, 65 - bits);
        (pre, bits, rng.interesting_u64())
    });
}

// ---------------------------------------------------------------------------
// Rows 7–8 — `bits == 0` (out-of-range `64 - bits` shift count)
// ---------------------------------------------------------------------------

/// Row 7 — `bits == 0` with `bw->bits < 64`: `val <<= 64` is masked to `<<= 0`
/// and the loop is not entered.
#[test]
fn row07_bits_zero_loop_not_entered() {
    sweep("row07 bits=0, bw->bits<64", 0x0707_0707, N, |rng| {
        let bw = rng.range(0, 63);
        let pre = pre_with_bwbits(rng, bw);
        (pre, 0, rng.interesting_u64())
    });
}

/// Row 8 — `bits == 0` with `bw->bits >= 64`: the guard is true, `b` collapses
/// to `bits == 0`, so nothing progresses and the `i < 100` cap ends the loop.
#[test]
fn row08_bits_zero_loop_hits_cap() {
    sweep("row08 bits=0, bw->bits>=64 (cap)", 0x0808_0808, 5_000, |rng| {
        let bwbits = match rng.below(5) {
            0 => 64,
            1 => 65,
            2 => 127,
            3 => u32::MAX,
            _ => rng.range(64, 4_000),
        };
        let pre = pre_with_bwbits(rng, bwbits);
        (pre, 0, rng.interesting_u64())
    });
}

// ---------------------------------------------------------------------------
// Rows 9–13 — `bits` at and past the width of `tflac_uint`
// ---------------------------------------------------------------------------

/// Row 9 — `bits == 63`, empty writer: the `b > bits` ternary ties (`b == 63`).
#[test]
fn row09_bits_63() {
    sweep("row09 bits=63", 0x0909_0909, N, |rng| {
        let bw = rng.range(0, 1);
        let pre = pre_with_bwbits(rng, bw);
        (pre, 63, rng.interesting_u64())
    });
}

/// Row 10 — `bits == 64` exactly: `64 - bits == 0`, so no shift masking, and
/// the loop is entered for every `bw->bits`.
#[test]
fn row10_bits_exactly_64() {
    sweep("row10 bits=64", 0x0A0A_0A0A, N, |rng| {
        let pre = pre_with_bwbits(rng, 0);
        (pre, UINT_BITS, rng.interesting_u64())
    });
}

/// Row 11 — `bits == 64` with a partially filled writer: several iterations
/// with a shrinking `b`.
#[test]
fn row11_bits_64_partially_filled() {
    sweep("row11 bits=64, bw->bits=1..63", 0x0B0B_0B0B, N, |rng| {
        let bw = rng.range(1, 63);
        let pre = pre_with_bwbits(rng, bw);
        (pre, UINT_BITS, rng.interesting_u64())
    });
}

/// Row 12 — `bits` in `65..127`: past the maximum width; `64 - bits` underflows
/// and is masked back into `1..63`.
#[test]
fn row12_bits_65_to_127() {
    sweep("row12 bits=65..127", 0x0C0C_0C0C, N, |rng| {
        let bits = rng.range(65, 127);
        let bw = rng.interesting_bwbits();
        let pre = pre_with_bwbits(rng, bw);
        (pre, bits, rng.interesting_u64())
    });
}

/// Row 13 — `bits` an exact multiple of 64 (`128`, `192`, `256`, …): the shift
/// count masks back to 0.
#[test]
fn row13_bits_multiple_of_64() {
    sweep("row13 bits=128,192,256,...", 0x0D0D_0D0D, 5_000, |rng| {
        let k = rng.range(2, 64);
        let bits = 64u32.wrapping_mul(k);
        let bw = rng.interesting_bwbits();
        let pre = pre_with_bwbits(rng, bw);
        (pre, bits, rng.interesting_u64())
    });
}

// ---------------------------------------------------------------------------
// Rows 14–15 — 32-bit wraparound of `bits` and of the guard sum
// ---------------------------------------------------------------------------

/// Row 14 — `bits == u32::MAX` and its neighbourhood.
#[test]
fn row14_bits_uint32_max() {
    sweep("row14 bits near u32::MAX", 0x0E0E_0E0E, 5_000, |rng| {
        let bits = u32::MAX - rng.below(64);
        let bw = rng.range(0, 70);
        let pre = pre_with_bwbits(rng, bw);
        (pre, bits, rng.interesting_u64())
    });
}

/// Row 15 — `bw->bits + bits` wraps past 2^32 to something `< 64`, so the guard
/// is FALSE and the loop is skipped even though `bits` is enormous.
#[test]
fn row15_guard_sum_wraps_below_64() {
    let p = load_pair();
    let mut rng = Rng::new(0x0F0F_0F0F);
    // Deterministic wraparound corners first.
    for bwbits in [64u32, 65, 100, 4096, u32::MAX] {
        for target in 0u32..64 {
            let bits = target.wrapping_sub(bwbits); // (bwbits + bits) mod 2^32 == target
            for _ in 0..8 {
                let pre = pre_with_bwbits(&mut rng, bwbits);
                p.assert_same("row15 guard sum wraps", &pre, bits, rng.interesting_u64());
            }
        }
    }
    // Randomised wraparound.
    for _ in 0..5_000 {
        let bwbits = rng.range(64, u32::MAX / 2);
        let target = rng.range(0, 63);
        let bits = target.wrapping_sub(bwbits);
        let pre = pre_with_bwbits(&mut rng, bwbits);
        p.assert_same("row15 guard sum wraps (random)", &pre, bits, rng.interesting_u64());
    }
}

// ---------------------------------------------------------------------------
// Rows 16–18 — the `i < 100` iteration cap
// ---------------------------------------------------------------------------

/// Row 16 — `bw->bits == 63`, `bits == 1`: `b = 63 - 63 = 0`, no progress, so
/// only the `i < 100` cap terminates the loop.
#[test]
fn row16_iteration_cap_bwbits_63_bits_1() {
    let p = load_pair();
    let mut rng = Rng::new(0x1010_1010);
    for _ in 0..N {
        let pre = pre_with_bwbits(&mut rng, 63);
        p.assert_same("row16 cap: bw->bits=63 bits=1", &pre, 1, rng.interesting_u64());
    }
}

/// Row 17 — cap reached with a non-zero `bits` left over for the tail.
#[test]
fn row17_iteration_cap_bwbits_63_bits_2_to_64() {
    sweep("row17 cap: bw->bits=63 bits=2..64", 0x1111_1111, N, |rng| {
        let bits = rng.range(2, 64);
        let pre = pre_with_bwbits(rng, 63);
        (pre, bits, rng.interesting_u64())
    });
}

/// Row 18 — cap reached from `bw->bits > 63`, where `63 - bw->bits` underflows
/// so the ternary takes `bits`, then stalls at `b == 0`.
#[test]
fn row18_iteration_cap_bwbits_above_63() {
    let p = load_pair();
    let mut rng = Rng::new(0x1212_1212);
    for bwbits in [64u32, 65, 66, 127, 128, 1000, u32::MAX - 1, u32::MAX] {
        for bits in [0u32, 1, 2, 62, 63, 64, 65, 128, u32::MAX] {
            for _ in 0..40 {
                let pre = pre_with_bwbits(&mut rng, bwbits);
                p.assert_same("row18 cap: bw->bits>63", &pre, bits, rng.interesting_u64());
            }
        }
    }
    for _ in 0..5_000 {
        let bwbits = rng.range(64, 100_000);
        let bits = rng.interesting_bits();
        let pre = pre_with_bwbits(&mut rng, bwbits);
        p.assert_same("row18 cap: bw->bits>63 (random)", &pre, bits, rng.interesting_u64());
    }
}

// ---------------------------------------------------------------------------
// Row 19 — `bw->bits` at u32::MAX (`bw->bits += b` wraps)
// ---------------------------------------------------------------------------

#[test]
fn row19_bwbits_uint32_max_wraps() {
    let p = load_pair();
    let mut rng = Rng::new(0x1313_1313);
    for bwbits in [u32::MAX, u32::MAX - 1, u32::MAX - 63, u32::MAX - 64] {
        for bits in [0u32, 1, 2, 63, 64, 65, 128, u32::MAX] {
            for _ in 0..40 {
                let pre = pre_with_bwbits(&mut rng, bwbits);
                p.assert_same("row19 bw->bits wraps", &pre, bits, rng.interesting_u64());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — `bw->val &= mask` (clearing bit 0) is observable
// ---------------------------------------------------------------------------

#[test]
fn row20_val_mask_clears_bit0() {
    let p = load_pair();
    let mut rng = Rng::new(0x1414_1414);
    let vals = [
        0u64,
        1,
        u64::MAX,
        u64::MAX - 1,
        0xAAAA_AAAA_AAAA_AAAB,
        0x5555_5555_5555_5555,
        0x8000_0000_0000_0001,
    ];
    for &bwval in &vals {
        for bits in [0u32, 1, 32, 63, 64, 65, 127, 128, u32::MAX] {
            for bwbits in [0u32, 1, 32, 62, 63, 64, 65, 128, u32::MAX] {
                for &arg in &vals {
                    let pre = Bw {
                        val: bwval,
                        bits: bwbits,
                        pos: rng.next_u32(),
                        len: rng.next_u32(),
                        tot: rng.next_u32(),
                        buffer: rng.next_u64() as *mut u8,
                    };
                    p.assert_same("row20 mask clears bit 0", &pre, bits, arg);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 21 — `val` argument bit patterns, incl. every single-bit value
// ---------------------------------------------------------------------------

#[test]
fn row21_val_bit_patterns() {
    let p = load_pair();
    let mut rng = Rng::new(0x1515_1515);
    let mut vals: Vec<u64> = vec![0, 1, u64::MAX, 0xAAAA_AAAA_AAAA_AAAA, 0x5555_5555_5555_5555];
    for b in 0..64 {
        vals.push(1u64 << b);
        vals.push(u64::MAX >> b);
        vals.push(u64::MAX << b);
    }
    for &val in &vals {
        for bits in [0u32, 1, 7, 8, 31, 32, 33, 63, 64, 65, 96, 128, u32::MAX] {
            for bwbits in [0u32, 1, 7, 31, 32, 62, 63, 64, 65, 100, u32::MAX] {
                let pre = Bw {
                    val: rng.interesting_u64(),
                    bits: bwbits,
                    pos: 0,
                    len: 0,
                    tot: 0,
                    buffer: std::ptr::null_mut(),
                };
                p.assert_same("row21 val bit patterns", &pre, bits, val);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 22 — `bw->tot` overflow wraps
// ---------------------------------------------------------------------------

#[test]
fn row22_tot_counter_overflow() {
    let p = load_pair();
    let mut rng = Rng::new(0x1616_1616);
    for tot in [0u32, 1, u32::MAX, u32::MAX - 1, u32::MAX - 63, 0xFFFF_FF00, 0x8000_0000] {
        for bits in [0u32, 1, 2, 63, 64, 65, 128, 255, u32::MAX] {
            for bwbits in [0u32, 32, 63, 64, u32::MAX] {
                let mut pre = pre_with_bwbits(&mut rng, bwbits);
                pre.tot = tot;
                p.assert_same("row22 tot overflow", &pre, bits, rng.interesting_u64());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 23 — `pos` / `len` / `buffer` must be preserved verbatim
// ---------------------------------------------------------------------------

#[test]
fn row23_untouched_fields_preserved() {
    let p = load_pair();
    let mut rng = Rng::new(0x1717_1717);
    // A real heap allocation, plus NULL, plus junk pointers.
    let mut heap = vec![0u8; 64];
    let heap_ptr = heap.as_mut_ptr();
    let buffers: [*mut u8; 4] =
        [std::ptr::null_mut(), heap_ptr, 1usize as *mut u8, usize::MAX as *mut u8];
    for &buffer in &buffers {
        // Includes the "capacity violated" shapes pos > len and len == 0.
        for &(pos, len) in &[(0u32, 0u32), (5, 0), (u32::MAX, 0), (0, u32::MAX), (7, 3), (3, 7)] {
            for bits in [0u32, 1, 64, 65, u32::MAX] {
                for bwbits in [0u32, 63, 64, u32::MAX] {
                    let pre = Bw {
                        val: rng.interesting_u64(),
                        bits: bwbits,
                        pos,
                        len,
                        tot: rng.next_u32(),
                        buffer,
                    };
                    let (c, r) = p.call(&pre, bits, rng.interesting_u64());
                    assert_eq!(c, r, "row23 divergence (pos={pos} len={len} buffer={buffer:?})");
                    // And confirm neither side disturbed the three fields.
                    let before = pre.bytes();
                    for i in (12..20).chain(24..32) {
                        assert_eq!(c.post[i], before[i], "C mutated byte {i} of pos/len/buffer");
                        assert_eq!(r.post[i], before[i], "Rust mutated byte {i} of pos/len/buffer");
                    }
                }
            }
        }
    }
    // Keep `heap` alive until here.
    assert_eq!(heap.len(), 64);
}

// ---------------------------------------------------------------------------
// Row 24 — long chained call sequences (the composed pipeline)
// ---------------------------------------------------------------------------

#[test]
fn row24_long_call_sequences() {
    let p = load_pair();
    for seed in 0..40u64 {
        let mut rng = Rng::new(0x2400_0000 ^ seed);
        let pre = Bw {
            val: rng.interesting_u64(),
            bits: rng.interesting_bwbits(),
            pos: rng.next_u32(),
            len: rng.next_u32(),
            tot: rng.next_u32(),
            buffer: rng.next_u64() as *mut u8,
        };
        let steps: Vec<(u32, u64)> =
            (0..2_000).map(|_| (rng.interesting_bits(), rng.interesting_u64())).collect();
        p.assert_same_sequence("row24 chained sequence", &pre, &steps);
    }
    // A "realistic consumer" sequence: only small, legal widths, starting empty.
    for seed in 0..20u64 {
        let mut rng = Rng::new(0x2401_0000 ^ seed);
        let steps: Vec<(u32, u64)> = (0..5_000)
            .map(|_| {
                let bits = rng.range(1, 32);
                let val = rng.next_u64() & (u64::MAX >> (64 - bits));
                (bits, val)
            })
            .collect();
        p.assert_same_sequence("row24 realistic bit-writer usage", &Bw::zeroed(), &steps);
    }
}

// ---------------------------------------------------------------------------
// Row 25 — unconstrained full-range fuzz
// ---------------------------------------------------------------------------

#[test]
fn row25_unconstrained_fuzz() {
    let p = load_pair();
    let mut rng = Rng::new(0x2525_2525_2525_2525);
    for _ in 0..1_000_000 {
        let pre = rng.interesting_pre();
        let bits = rng.interesting_bits();
        let val = rng.interesting_u64();
        p.assert_same("row25 unconstrained fuzz", &pre, bits, val);
    }
    // A second pass with *uniform* (unbiased) values, to avoid the bias of the
    // "interesting" generators hiding a plain-random divergence.
    let mut rng = Rng::new(0xFEED_FACE_CAFE_BEEF);
    for _ in 0..500_000 {
        let pre = Bw {
            val: rng.next_u64(),
            bits: rng.next_u32(),
            pos: rng.next_u32(),
            len: rng.next_u32(),
            tot: rng.next_u32(),
            buffer: rng.next_u64() as *mut u8,
        };
        p.assert_same("row25 uniform fuzz", &pre, rng.next_u32(), rng.next_u64());
    }
}

// ---------------------------------------------------------------------------
// Row 26 — exhaustive sweep of the two structural axes
// ---------------------------------------------------------------------------

#[test]
fn row26_exhaustive_bits_cross_bwbits() {
    let p = load_pair();
    let vals: [u64; 6] = [
        0,
        1,
        u64::MAX,
        0xAAAA_AAAA_AAAA_AAAA,
        0x5555_5555_5555_5555,
        0x0123_4567_89AB_CDEF,
    ];
    let bwvals: [u64; 4] = [0, 1, u64::MAX, 0xF0F0_F0F0_0F0F_0F0F];
    for bits in 0u32..=130 {
        for bwbits in 0u32..=130 {
            for &val in &vals {
                for &bwval in &bwvals {
                    let pre = Bw {
                        val: bwval,
                        bits: bwbits,
                        pos: 0xAAAA_AAAA,
                        len: 0xBBBB_BBBB,
                        tot: 0xFFFF_FFF0,
                        buffer: 0x1234_5678u64 as *mut u8,
                    };
                    p.assert_same("row26 exhaustive bits x bw->bits", &pre, bits, val);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 27 — the near-2^32 `bw->bits` band, where `63 - bw->bits` is SMALL
// ---------------------------------------------------------------------------

/// When `bw->bits` sits just below 2^32, `63 - bw->bits` wraps to a *small
/// positive* number (`64` for `bw->bits == u32::MAX`, `65` for `u32::MAX - 1`,
/// …) instead of a huge one. That makes the ternary select `b = 63 - bw->bits`
/// even though `bw->bits >= 64`, and `bw->bits += b` then wraps to exactly 63.
/// This is a structurally distinct path from rows 18/19.
#[test]
fn row27_near_2pow32_bwbits_band() {
    let p = load_pair();
    let mut rng = Rng::new(0x2727_2727);
    for delta in 0u32..400 {
        let bwbits = u32::MAX - delta;
        // b_raw = (63 - bw->bits) mod 2^32 == 64 + delta
        let b_raw = 63u32.wrapping_sub(bwbits);
        assert_eq!(b_raw, 64 + delta, "b_raw shape assumption");
        for bits in [
            0,
            1,
            b_raw - 1, // ternary takes `bits`
            b_raw,     // tie -> takes b_raw
            b_raw + 1, // takes b_raw
            b_raw * 2,
            u32::MAX,
        ] {
            for _ in 0..6 {
                let pre = pre_with_bwbits(&mut rng, bwbits);
                p.assert_same("row27 near-2^32 bw->bits band", &pre, bits, rng.interesting_u64());
            }
        }
    }
    // Randomised inside the band.
    for _ in 0..40_000 {
        let bwbits = u32::MAX - rng.below(5_000);
        let bits = rng.interesting_bits();
        let pre = pre_with_bwbits(&mut rng, bwbits);
        p.assert_same("row27 near-2^32 band (random)", &pre, bits, rng.interesting_u64());
    }
}

// ---------------------------------------------------------------------------
// Row 28 — the three reachable loop regimes, driven separately
// ---------------------------------------------------------------------------

/// Classify a `(bw->bits, bits)` pair by replaying the C loop's control flow:
/// returns `(entered, progressing_iters)`.
fn loop_regime(mut bw_bits: u32, mut bits: u32) -> (bool, u32) {
    let mut entered = false;
    let mut prog = 0u32;
    let mut i = 0i32;
    while bw_bits.wrapping_add(bits) >= 64 && i < 100 {
        entered = true;
        let mut b = 64u32.wrapping_sub(bw_bits).wrapping_sub(1);
        b = if b > bits { bits } else { b };
        if b == 0 {
            break; // stalled: every remaining spin is idempotent
        }
        prog += 1;
        bw_bits = bw_bits.wrapping_add(b);
        bits = bits.wrapping_sub(b);
        i += 1;
    }
    (entered, prog)
}

#[test]
fn row28_three_loop_regimes() {
    let p = load_pair();
    let mut rng = Rng::new(0x2828_2828);
    let mut seen = [0u64; 3]; // [not entered, entered+stall, entered+1 progress]

    // Dense structured search so all three regimes are certainly hit.
    let mut cands: Vec<u32> = (0u32..=200).collect();
    for k in 0..32 {
        cands.push(1u32 << k);
        cands.push((1u32 << k).wrapping_sub(1));
    }
    for d in 0..200u32 {
        cands.push(u32::MAX - d);
    }
    cands.sort_unstable();
    cands.dedup();

    for &bwbits in &cands {
        for &bits in &cands {
            let (entered, prog) = loop_regime(bwbits, bits);
            let idx = if !entered {
                0
            } else if prog == 0 {
                1
            } else {
                2
            };
            // Sample each regime heavily but keep the run bounded.
            if seen[idx] % 7 == 0 {
                let pre = pre_with_bwbits(&mut rng, bwbits);
                p.assert_same(
                    &format!("row28 regime {idx} (entered={entered}, prog={prog})"),
                    &pre,
                    bits,
                    rng.interesting_u64(),
                );
            }
            seen[idx] += 1;
        }
    }

    assert!(seen[0] > 0, "regime (a) loop-not-entered was never reached");
    assert!(seen[1] > 0, "regime (b) entered-then-immediate-stall was never reached");
    assert!(seen[2] > 0, "regime (c) exactly-one-progressing-iteration was never reached");
    // The brute-force result recorded in CONFIGS.md: never more than 1 progress step.
    assert_eq!(seen.len(), 3);
}

// ---------------------------------------------------------------------------
// Row 29 — `b >= 64` inside the loop, so `val <<= b` is an out-of-range shift
// ---------------------------------------------------------------------------

#[test]
fn row29_in_loop_shift_count_out_of_range() {
    let p = load_pair();
    let mut rng = Rng::new(0x2929_2929);
    // bw->bits >= 64 makes `63 - bw->bits` huge, so b = bits; pick bits >= 64
    // so that the in-loop `val <<= b` shift count needs hardware masking.
    for bwbits in [64u32, 65, 100, 1000, 0x1000, 0x8000_0000] {
        for bits in [64u32, 65, 96, 127, 128, 191, 192, 255, 256, 4096, u32::MAX] {
            for _ in 0..60 {
                let pre = pre_with_bwbits(&mut rng, bwbits);
                p.assert_same("row29 in-loop b>=64", &pre, bits, rng.interesting_u64());
            }
        }
    }
    for _ in 0..40_000 {
        let bwbits = rng.range(64, 1 << 20);
        let bits = rng.range(64, u32::MAX / 2);
        let pre = pre_with_bwbits(&mut rng, bwbits);
        p.assert_same("row29 in-loop b>=64 (random)", &pre, bits, rng.interesting_u64());
    }
}
