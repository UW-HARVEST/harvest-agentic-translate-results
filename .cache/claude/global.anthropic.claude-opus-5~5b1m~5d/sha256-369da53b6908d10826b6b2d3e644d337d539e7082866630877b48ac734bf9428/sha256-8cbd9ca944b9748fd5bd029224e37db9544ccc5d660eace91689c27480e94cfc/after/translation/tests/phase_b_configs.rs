//! Phase B — valid-path differential tests.
//! One test per row of `CONFIGS.md`, each with many randomized inputs
//! (fixed seed) rather than a single hand-picked value.

mod common;
use common::*;

const SEED: u64 = 0x0BAD_C0FF_EE12_3456;
const N: usize = 512;

/// Row 1 — zeroed bw, bits == 0 (shift-by-64 path, loop skipped).
#[test]
fn row01_bits_zero() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..N {
        check_call(Bitwriter::zeroed(), 0, rng.interesting_u64(), "row01");
    }
}

/// Row 2 — zeroed bw, bits == 1.
#[test]
fn row02_bits_one() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..N {
        check_call(Bitwriter::zeroed(), 1, rng.interesting_u64(), "row02");
    }
}

/// Row 3 — zeroed bw, full sweep of bits 1..=63.
#[test]
fn row03_bits_mid_sweep() {
    let mut rng = Rng::new(SEED ^ 3);
    for bits in 1u32..=63 {
        for _ in 0..64 {
            check_call(Bitwriter::zeroed(), bits, rng.interesting_u64(), "row03");
        }
    }
}

/// Row 4 — zeroed bw, bits == 63.
#[test]
fn row04_bits_63() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..N {
        check_call(Bitwriter::zeroed(), 63, rng.interesting_u64(), "row04");
    }
}

/// Row 5 — zeroed bw, bits == 64 (loop entered, 64 - bits == 0).
#[test]
fn row05_bits_64() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..N {
        check_call(Bitwriter::zeroed(), 64, rng.interesting_u64(), "row05");
    }
}

/// Row 6 — bits == 65 and 100 (`64 - bits` underflows u32).
#[test]
fn row06_bits_over_64() {
    let mut rng = Rng::new(SEED ^ 6);
    for bits in [65u32, 66, 100, 127, 128, 129] {
        for _ in 0..N {
            check_call(Bitwriter::zeroed(), bits, rng.interesting_u64(), "row06");
        }
    }
}

/// Row 7 — extreme bits values (long loop → hits the `i < 100` cap).
#[test]
fn row07_bits_extreme() {
    let mut rng = Rng::new(SEED ^ 7);
    for bits in [0xFFFFu32, 0x1_0000, 0x7FFF_FFFF, 0x8000_0000, u32::MAX] {
        for _ in 0..N {
            check_call(Bitwriter::zeroed(), bits, rng.interesting_u64(), "row07");
        }
    }
}

/// Row 8 — full 65x65 cross-product of `bw->bits` x `bits` (axes A x B).
#[test]
fn row08_cross_product_state_bits_x_bits() {
    let mut rng = Rng::new(SEED ^ 8);
    for state_bits in 0u32..=64 {
        for bits in 0u32..=64 {
            for _ in 0..3 {
                let st = Bitwriter {
                    val: rng.interesting_u64(),
                    bits: state_bits,
                    pos: rng.next_u32(),
                    len: rng.next_u32(),
                    tot: rng.next_u32(),
                    buffer: rng.next_u64() as *mut u8,
                };
                check_call(st, bits, rng.interesting_u64(), "row08");
            }
        }
    }
}

/// Row 9 — bw->bits == 63 forces b == 0 → spin to the i<100 cap.
#[test]
fn row09_state_bits_63_spin() {
    let mut rng = Rng::new(SEED ^ 9);
    for bits in 1u32..=64 {
        for _ in 0..8 {
            let st = Bitwriter {
                val: rng.interesting_u64(),
                bits: 63,
                pos: rng.next_u32(),
                len: rng.next_u32(),
                tot: rng.next_u32(),
                buffer: std::ptr::null_mut(),
            };
            check_call(st, bits, rng.interesting_u64(), "row09");
        }
    }
}

/// Row 10 — bw->bits == 64 exactly (64-64-1 underflows → clamp b = bits).
#[test]
fn row10_state_bits_64() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..N * 4 {
        let st = Bitwriter {
            val: rng.interesting_u64(),
            bits: 64,
            pos: rng.next_u32(),
            len: rng.next_u32(),
            tot: rng.next_u32(),
            buffer: std::ptr::null_mut(),
        };
        check_call(st, rng.interesting_bits(), rng.interesting_u64(), "row10");
    }
}

/// Row 11 — bw->bits > 64.
#[test]
fn row11_state_bits_above_64() {
    let mut rng = Rng::new(SEED ^ 11);
    for sb in [65u32, 66, 100, 128, 1000, 0xFFFF, 0x8000_0000, u32::MAX] {
        for _ in 0..N {
            let st = Bitwriter {
                val: rng.interesting_u64(),
                bits: sb,
                pos: rng.next_u32(),
                len: rng.next_u32(),
                tot: rng.next_u32(),
                buffer: std::ptr::null_mut(),
            };
            check_call(st, rng.interesting_bits(), rng.interesting_u64(), "row11");
        }
    }
}

/// Row 12 — bw->bits + bits wraps u32 back below 64 (loop skipped).
#[test]
fn row12_sum_wraps_below_64() {
    let mut rng = Rng::new(SEED ^ 12);
    let pairs: [(u32, u32); 8] = [
        (u32::MAX, 1),
        (u32::MAX, 2),
        (u32::MAX, 64),
        (0xFFFF_FFC0, 0x41),
        (0xFFFF_FF00, 0x13F),
        (0x8000_0000, 0x8000_0000),
        (0x8000_0000, 0x8000_003F),
        (0xFFFF_FFFE, 3),
    ];
    for (sb, bits) in pairs {
        // sanity: this configuration really does wrap below 64
        assert!(sb.wrapping_add(bits) < 64, "bad pair {sb} {bits}");
        for _ in 0..N {
            let st = Bitwriter {
                val: rng.interesting_u64(),
                bits: sb,
                pos: rng.next_u32(),
                len: rng.next_u32(),
                tot: rng.next_u32(),
                buffer: std::ptr::null_mut(),
            };
            check_call(st, bits, rng.interesting_u64(), "row12");
        }
    }
}

/// Row 13 — pre-seeded bw->val, incl. odd values (bit-0 mask observability).
#[test]
fn row13_seeded_val() {
    let mut rng = Rng::new(SEED ^ 13);
    let seeds: [u64; 8] = [
        0,
        1,
        u64::MAX,
        0xFFFF_FFFF_FFFF_FFFF,
        0x0000_0000_0000_0003,
        0xAAAA_AAAA_AAAA_AAAB,
        1 << 63,
        0x8000_0000_0000_0001,
    ];
    for sv in seeds {
        // loop-skipped and loop-taken bits
        for bits in [0u32, 1, 32, 63, 64, 65, 200] {
            for sb in [0u32, 1, 32, 63, 64] {
                let st = Bitwriter {
                    val: sv,
                    bits: sb,
                    pos: 0,
                    len: 0,
                    tot: 0,
                    buffer: std::ptr::null_mut(),
                };
                check_call(st, bits, rng.interesting_u64(), "row13");
            }
        }
    }
}

/// Row 14 — val boundary patterns x boundary bits.
#[test]
fn row14_val_patterns() {
    let vals: [u64; 8] = [
        0,
        1,
        u64::MAX,
        1 << 63,
        0xAAAA_AAAA_AAAA_AAAA,
        0x5555_5555_5555_5555,
        0xFFFF_FFFF_0000_0000,
        0x0000_0000_FFFF_FFFF,
    ];
    for v in vals {
        for bits in [0u32, 1, 32, 63, 64, 65] {
            for sb in [0u32, 1, 31, 32, 62, 63, 64, 65] {
                let st = Bitwriter {
                    val: 0,
                    bits: sb,
                    pos: 7,
                    len: 9,
                    tot: 11,
                    buffer: std::ptr::null_mut(),
                };
                check_call(st, bits, v, "row14");
            }
        }
    }
}

/// Row 15 — bw->tot near u32::MAX → wrapping overflow.
#[test]
fn row15_tot_overflow() {
    let mut rng = Rng::new(SEED ^ 15);
    for tot in [u32::MAX, 0xFFFF_FFF0, 0xFFFF_FF00, 0x8000_0000] {
        for bits in [1u32, 16, 64, 65, 0xFFFF, u32::MAX] {
            for _ in 0..32 {
                let st = Bitwriter {
                    val: rng.interesting_u64(),
                    bits: rng.below(64),
                    pos: 0,
                    len: 0,
                    tot,
                    buffer: std::ptr::null_mut(),
                };
                check_call(st, bits, rng.interesting_u64(), "row15");
            }
        }
    }
}

/// Row 16 — non-zero pos/len and a real heap `buffer` must survive untouched.
#[test]
fn row16_untouched_fields_real_pointer() {
    let mut rng = Rng::new(SEED ^ 16);
    let mut backing = vec![0u8; 256];
    let ptr = backing.as_mut_ptr();
    for _ in 0..N * 2 {
        let st = Bitwriter {
            val: rng.interesting_u64(),
            bits: rng.interesting_state_bits(),
            pos: rng.next_u32(),
            len: rng.next_u32(),
            tot: rng.next_u32(),
            buffer: ptr,
        };
        check_call(st, rng.interesting_bits(), rng.interesting_u64(), "row16");
    }
    // The backing buffer itself must never be written by either impl.
    assert!(backing.iter().all(|&b| b == 0), "buffer contents modified");
}

/// Row 17 — inconsistent writer state (null buffer, len == 0, pos > len).
#[test]
fn row17_inconsistent_state() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..N * 2 {
        let st = Bitwriter {
            val: rng.interesting_u64(),
            bits: rng.interesting_state_bits(),
            pos: 0xDEAD_BEEF,
            len: 0,
            tot: rng.next_u32(),
            buffer: std::ptr::null_mut(),
        };
        check_call(st, rng.interesting_bits(), rng.interesting_u64(), "row17");
    }
}

/// Row 18 — realistic pipeline: 500 chained calls, bits in 0..=64.
#[test]
fn row18_sequence_realistic() {
    let mut rng = Rng::new(SEED ^ 18);
    for round in 0..40 {
        let ops: Vec<(u32, u64)> = (0..500)
            .map(|_| (rng.below(64), rng.interesting_u64()))
            .collect();
        let start = if round % 2 == 0 {
            Bitwriter::zeroed()
        } else {
            Bitwriter {
                val: rng.interesting_u64(),
                bits: rng.below(64),
                pos: rng.next_u32(),
                len: rng.next_u32(),
                tot: rng.next_u32(),
                buffer: std::ptr::null_mut(),
            }
        };
        check_sequence(start, &ops, "row18");
    }
}

/// Row 19 — adversarial pipeline: 500 chained calls, arbitrary u32 bits.
#[test]
fn row19_sequence_adversarial() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..40 {
        let ops: Vec<(u32, u64)> = (0..500)
            .map(|_| (rng.interesting_bits(), rng.interesting_u64()))
            .collect();
        check_sequence(rng.state(), &ops, "row19");
    }
}

/// Row 20 — full fuzz: every argument and every struct field randomized.
#[test]
fn row20_full_fuzz() {
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..200_000 {
        let st = rng.state();
        check_call(st, rng.interesting_bits(), rng.interesting_u64(), "row20");
    }
}

/// Row 21 — struct ABI: size / align / field offsets.
#[test]
fn row21_struct_layout_matches_c() {
    use std::mem::{align_of, size_of};
    assert_eq!(size_of::<Bitwriter>(), 32, "sizeof(tflac_bitwriter)");
    assert_eq!(align_of::<Bitwriter>(), 8, "alignof(tflac_bitwriter)");

    // Offsets are proven behaviourally: give each field a distinct value, let
    // the C library write `tot` (offset 20) and `bits` (offset 8) / `val`
    // (offset 0), and confirm the Rust view sees the same bytes in the same
    // places.  If Rust's field offsets disagreed with C's, the raw-byte
    // comparison in `check_call` would fail.
    let st = Bitwriter {
        val: 0x0123_4567_89AB_CDEF,
        bits: 0x1111_1111,
        pos: 0x2222_2222,
        len: 0x3333_3333,
        tot: 0x4444_4444,
        buffer: 0x5555_5555_5555_5555u64 as *mut u8,
    };
    let bytes = st.as_bytes();
    assert_eq!(&bytes[0..8], &0x0123_4567_89AB_CDEFu64.to_ne_bytes());
    assert_eq!(&bytes[8..12], &0x1111_1111u32.to_ne_bytes());
    assert_eq!(&bytes[12..16], &0x2222_2222u32.to_ne_bytes());
    assert_eq!(&bytes[16..20], &0x3333_3333u32.to_ne_bytes());
    assert_eq!(&bytes[20..24], &0x4444_4444u32.to_ne_bytes());
    assert_eq!(&bytes[24..32], &0x5555_5555_5555_5555u64.to_ne_bytes());

    check_call(st, 5, 0xF0F0_F0F0_F0F0_F0F0, "row21");
}
