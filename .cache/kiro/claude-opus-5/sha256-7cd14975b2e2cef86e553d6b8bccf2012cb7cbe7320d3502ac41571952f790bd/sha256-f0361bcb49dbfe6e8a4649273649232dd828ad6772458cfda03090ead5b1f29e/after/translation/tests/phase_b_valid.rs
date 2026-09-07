//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md`, each driving `ITERS` randomized
//! inputs through BOTH the C `.so` and the Rust `.so` (loaded with
//! `libloading`) and asserting byte-identical results.

mod harness;

use harness::{Bitwriter, ITERS, Pair, Rng, SEED};

/// Per-row seed derivation so each row gets an independent but reproducible
/// stream.
fn rng_for(row: u64) -> Rng {
    Rng::new(SEED ^ (row.wrapping_mul(0x9E37_79B9_7F4A_7C15)))
}

/* ================= rows 1–20: `bits` / `bw->bits` shape ================= */

#[test]
fn cfg_row01_s0_p0() {
    let p = Pair::load();
    let mut r = rng_for(1);
    for i in 0..ITERS {
        let bw = r.bw_with_bits(0);
        let val = r.next_u64();
        p.check("row01 S=0 P=0", i, &bw, 0, val);
    }
}

#[test]
fn cfg_row02_s0_p1_to_63() {
    let p = Pair::load();
    let mut r = rng_for(2);
    for i in 0..ITERS {
        let bits = r.range_u32(1, 63);
        let bw = r.bw_with_bits(0);
        let val = r.next_u64();
        p.check("row02 S=0 P=1..=63", i, &bw, bits, val);
    }
}

#[test]
fn cfg_row03_s0_p64() {
    let p = Pair::load();
    let mut r = rng_for(3);
    for i in 0..ITERS {
        let bw = r.bw_with_bits(0);
        let val = r.next_u64();
        p.check("row03 S=0 P=64 (loop cap)", i, &bw, 64, val);
    }
}

#[test]
fn cfg_row04_s0_p65_to_127() {
    let p = Pair::load();
    let mut r = rng_for(4);
    for i in 0..ITERS {
        let bits = r.range_u32(65, 127);
        let bw = r.bw_with_bits(0);
        let val = r.next_u64();
        p.check("row04 S=0 P=65..=127", i, &bw, bits, val);
    }
}

#[test]
fn cfg_row05_s1_31_p1_31_no_loop() {
    let p = Pair::load();
    let mut r = rng_for(5);
    for i in 0..ITERS {
        let s = r.range_u32(1, 31);
        let bits = r.range_u32(1, 31);
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        p.check("row05 S=1..=31 P=1..=31 (no loop)", i, &bw, bits, val);
    }
}

#[test]
fn cfg_row06_s32_62_sum_lt_64() {
    let p = Pair::load();
    let mut r = rng_for(6);
    for i in 0..ITERS {
        let s = r.range_u32(32, 62);
        let bits = r.range_u32(1, 63 - s);
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        p.check("row06 S=32..=62 sum<64", i, &bw, bits, val);
    }
}

#[test]
fn cfg_row07_s1_62_sum_ge_64() {
    let p = Pair::load();
    let mut r = rng_for(7);
    for i in 0..ITERS {
        let s = r.range_u32(1, 62);
        let bits = r.range_u32(64 - s, 63);
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        p.check("row07 S=1..=62 sum>=64 (loop)", i, &bw, bits, val);
    }
}

#[test]
fn cfg_row08_s63_p0() {
    let p = Pair::load();
    let mut r = rng_for(8);
    for i in 0..ITERS {
        let bw = r.bw_with_bits(63);
        let val = r.next_u64();
        p.check("row08 S=63 P=0", i, &bw, 0, val);
    }
}

#[test]
fn cfg_row09_s63_p1() {
    let p = Pair::load();
    let mut r = rng_for(9);
    for i in 0..ITERS {
        let bw = r.bw_with_bits(63);
        let val = r.next_u64();
        p.check("row09 S=63 P=1 (sum==64)", i, &bw, 1, val);
    }
}

#[test]
fn cfg_row10_s63_p2_to_63() {
    let p = Pair::load();
    let mut r = rng_for(10);
    for i in 0..ITERS {
        let bits = r.range_u32(2, 63);
        let bw = r.bw_with_bits(63);
        let val = r.next_u64();
        p.check("row10 S=63 P=2..=63", i, &bw, bits, val);
    }
}

#[test]
fn cfg_row11_s64_p0() {
    let p = Pair::load();
    let mut r = rng_for(11);
    for i in 0..ITERS {
        let bw = r.bw_with_bits(64);
        let val = r.next_u64();
        p.check("row11 S=64 P=0 (b wraps)", i, &bw, 0, val);
    }
}

#[test]
fn cfg_row12_s64_p1_to_64() {
    let p = Pair::load();
    let mut r = rng_for(12);
    for i in 0..ITERS {
        let bits = r.range_u32(1, 64);
        let bw = r.bw_with_bits(64);
        let val = r.next_u64();
        p.check("row12 S=64 P=1..=64", i, &bw, bits, val);
    }
}

#[test]
fn cfg_row13_s65_127_p0() {
    let p = Pair::load();
    let mut r = rng_for(13);
    for i in 0..ITERS {
        let s = r.range_u32(65, 127);
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        p.check("row13 S=65..=127 P=0", i, &bw, 0, val);
    }
}

#[test]
fn cfg_row14_s65_127_p1_127() {
    let p = Pair::load();
    let mut r = rng_for(14);
    for i in 0..ITERS {
        let s = r.range_u32(65, 127);
        let bits = r.range_u32(1, 127);
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        p.check("row14 S=65..=127 P=1..=127", i, &bw, bits, val);
    }
}

#[test]
fn cfg_row15_smax_p1_wrap_no_loop() {
    let p = Pair::load();
    let mut r = rng_for(15);
    for i in 0..ITERS {
        let bw = r.bw_with_bits(u32::MAX);
        let val = r.next_u64();
        p.check("row15 S=U32MAX P=1 (sum wraps to 0)", i, &bw, 1, val);
    }
}

#[test]
fn cfg_row16_smax_p0_loop() {
    let p = Pair::load();
    let mut r = rng_for(16);
    for i in 0..ITERS {
        let bw = r.bw_with_bits(u32::MAX);
        let val = r.next_u64();
        p.check("row16 S=U32MAX P=0", i, &bw, 0, val);
    }
}

#[test]
fn cfg_row17_s_and_p_high_bit_wrap() {
    let p = Pair::load();
    let mut r = rng_for(17);
    for i in 0..ITERS {
        let bw = r.bw_with_bits(0x8000_0000);
        let val = r.next_u64();
        p.check(
            "row17 S=P=0x80000000 (sum wraps to 0)",
            i,
            &bw,
            0x8000_0000,
            val,
        );
    }
}

#[test]
fn cfg_row18_s0_pmax() {
    let p = Pair::load();
    let mut r = rng_for(18);
    for i in 0..ITERS {
        let bw = r.bw_with_bits(0);
        let val = r.next_u64();
        p.check("row18 S=0 P=U32MAX", i, &bw, u32::MAX, val);
    }
}

#[test]
fn cfg_row19_srand_pmax() {
    let p = Pair::load();
    let mut r = rng_for(19);
    for i in 0..ITERS {
        let s = r.next_u32();
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        p.check("row19 S=rand P=U32MAX", i, &bw, u32::MAX, val);
    }
}

#[test]
fn cfg_row20_fully_random() {
    let p = Pair::load();
    let mut r = rng_for(20);
    for i in 0..ITERS * 4 {
        let s = r.next_u32();
        let bits = r.next_u32();
        let bw = r.bw_with_bits(s);
        let val = r.next_u64();
        p.check("row20 fully random S,P,val", i, &bw, bits, val);
    }
}

/* ================= rows 21–25: `val` shape ================= */

#[test]
fn cfg_row21_val_zero() {
    let p = Pair::load();
    let mut r = rng_for(21);
    for i in 0..ITERS {
        let s = r.range_u32(0, 64);
        let bits = r.range_u32(0, 64);
        let bw = r.bw_with_bits(s);
        p.check("row21 val=0", i, &bw, bits, 0);
    }
}

#[test]
fn cfg_row22_val_max() {
    let p = Pair::load();
    let mut r = rng_for(22);
    for i in 0..ITERS {
        let s = r.range_u32(0, 64);
        let bits = r.range_u32(0, 64);
        let bw = r.bw_with_bits(s);
        p.check("row22 val=U64MAX", i, &bw, bits, u64::MAX);
    }
}

#[test]
fn cfg_row23_val_low_bits_only() {
    let p = Pair::load();
    let mut r = rng_for(23);
    for i in 0..ITERS {
        let s = r.range_u32(0, 64);
        let bits = r.range_u32(0, 64);
        let bw = r.bw_with_bits(s);
        let mask = if bits >= 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        let val = r.next_u64() & mask;
        p.check("row23 val = low `bits` bits only", i, &bw, bits, val);
    }
}

#[test]
fn cfg_row24_val_top_bit_only() {
    let p = Pair::load();
    let mut r = rng_for(24);
    for i in 0..ITERS {
        let s = r.range_u32(0, 64);
        let bits = r.range_u32(0, 64);
        let bw = r.bw_with_bits(s);
        p.check("row24 val = 1<<63", i, &bw, bits, 1u64 << 63);
    }
}

#[test]
fn cfg_row25_val_single_random_bit() {
    let p = Pair::load();
    let mut r = rng_for(25);
    for i in 0..ITERS {
        let s = r.range_u32(0, 64);
        let bits = r.range_u32(0, 64);
        let bw = r.bw_with_bits(s);
        let val = 1u64 << (r.next_u32() % 64);
        p.check("row25 val = single random bit", i, &bw, bits, val);
    }
}

/* ================= rows 26–29: accumulator / tot / untouched ============ */

#[test]
fn cfg_row26_acc_zero() {
    let p = Pair::load();
    let mut r = rng_for(26);
    for i in 0..ITERS {
        let s = r.range_u32(0, 64);
        let bits = r.range_u32(0, 64);
        let mut bw = r.bw_with_bits(s);
        bw.val = 0;
        let val = r.next_u64();
        p.check("row26 bw->val = 0", i, &bw, bits, val);
    }
}

#[test]
fn cfg_row27_acc_max() {
    let p = Pair::load();
    let mut r = rng_for(27);
    for i in 0..ITERS {
        let s = r.range_u32(0, 64);
        let bits = r.range_u32(0, 64);
        let mut bw = r.bw_with_bits(s);
        bw.val = u64::MAX;
        let val = r.next_u64();
        p.check(
            "row27 bw->val = U64MAX (mask clears bit 0)",
            i,
            &bw,
            bits,
            val,
        );
    }
}

#[test]
fn cfg_row28_tot_near_wrap() {
    let p = Pair::load();
    let mut r = rng_for(28);
    for i in 0..ITERS {
        let s = r.range_u32(0, 64);
        let bits = r.range_u32(0, 64);
        let mut bw = r.bw_with_bits(s);
        bw.tot = u32::MAX - r.range_u32(0, 64);
        let val = r.next_u64();
        p.check("row28 bw->tot near U32MAX (wrap)", i, &bw, bits, val);
    }
}

#[test]
fn cfg_row29_untouched_fields() {
    let p = Pair::load();
    let mut r = rng_for(29);
    // A real, non-null, page-aligned-ish sentinel pointer that must never be
    // dereferenced by `bitwriter_add`.
    let mut scratch = vec![0xA5u8; 64];
    let wild = scratch.as_mut_ptr();
    for i in 0..ITERS {
        let s = r.range_u32(0, 64);
        let bits = r.range_u32(0, 64);
        let bw = Bitwriter {
            val: r.next_u64(),
            bits: s,
            pos: r.next_u32(),
            len: r.next_u32(),
            tot: r.next_u32(),
            buffer: wild,
        };
        let val = r.next_u64();
        p.check(
            "row29 wild non-null buffer, random pos/len",
            i,
            &bw,
            bits,
            val,
        );
    }
    assert!(
        scratch.iter().all(|&b| b == 0xA5),
        "buffer contents were modified"
    );
}

/* ================= rows 30–33: composed sequences ================= */

fn sequence_row(
    row: u64,
    name: &str,
    seqs: u64,
    gen_bits: impl Fn(&mut Rng) -> u32,
    random_seed_bw: bool,
) {
    let p = Pair::load();
    let mut r = rng_for(row);
    for s in 0..seqs {
        let init = if random_seed_bw {
            let b = r.next_u32();
            r.bw_with_bits(b)
        } else {
            Bitwriter::default()
        };
        let ops: Vec<(u32, u64)> = (0..64).map(|_| (gen_bits(&mut r), r.next_u64())).collect();
        p.check_sequence(name, s, &init, &ops);
    }
}

#[test]
fn cfg_row30_seq_bits_1_to_32() {
    sequence_row(
        30,
        "row30 seq bits=1..=32",
        400,
        |r| r.range_u32(1, 32),
        false,
    );
}

#[test]
fn cfg_row31_seq_bits_0_to_64() {
    sequence_row(
        31,
        "row31 seq bits=0..=64",
        400,
        |r| r.range_u32(0, 64),
        false,
    );
}

#[test]
fn cfg_row32_seq_bits_full_u32() {
    sequence_row(32, "row32 seq bits=full u32", 400, |r| r.next_u32(), false);
}

#[test]
fn cfg_row33_seq_random_initial_state() {
    sequence_row(
        33,
        "row33 seq from random bw seed",
        400,
        |r| r.range_u32(0, 96),
        true,
    );
}
