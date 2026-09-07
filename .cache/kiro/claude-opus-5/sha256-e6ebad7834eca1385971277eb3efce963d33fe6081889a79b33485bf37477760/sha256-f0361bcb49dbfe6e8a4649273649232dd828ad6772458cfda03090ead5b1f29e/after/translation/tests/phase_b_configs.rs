//! Phase B -- valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every row drives the only public entry point (`dequantize_granule`) through
//! `dlsym` on both `.so`s, with the `bs_t` / `L12_scale_info` state set up by
//! hand, and compares the return value, the mutated `bs_t`, and every byte of
//! `grbuf` / `sci` / `buf` afterwards. Each row runs many randomized inputs
//! from a fixed seed.

mod common;

use common::*;

/// Comfortable bitstream: `limit` is far beyond anything these rows consume.
const BUF_MED: usize = 65536;
/// For the rows whose `ba` asks for tens of thousands of bits per band.
const BUF_BIG: usize = 512 * 1024;

const ITERS: usize = 400;

/// Random `ba` drawn from `set`, written into every slot the C can reach.
fn ba_from(inp: &mut Inputs, rng: &mut Rng, set: &[u8]) {
    let picks: Vec<u8> = (0..(SCI_REGION - OFF_BITALLOC)).map(|_| rng.pick(set)).collect();
    let mut it = picks.into_iter();
    inp.ba_all(|_| it.next().unwrap());
}

fn linear_set() -> Vec<u8> {
    (1..=16u8).collect()
}

fn each<F: FnMut(&mut Rng, usize) -> Inputs>(row: &str, seed: u64, iters: usize, mut mk: F) {
    let libs = Libs::load();
    let mut rng = Rng::new(seed);
    for i in 0..iters {
        let inp = mk(&mut rng, i);
        assert_same(&libs, row, i, &inp);
    }
}

// --------------------------------------------------------------------------
// C1 -- total_bands == 0: inner loop never runs
// --------------------------------------------------------------------------
#[test]
fn c01_total_bands_zero() {
    each("C1", 0x0001, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        let set: Vec<u8> = (0..=255u8).collect();
        ba_from(&mut inp, rng, &set);
        inp.total_bands(0).group_size(3);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C2 -- every bitalloc == 0: the `ba != 0` guard rejects every band
// --------------------------------------------------------------------------
#[test]
fn c02_all_bitalloc_zero() {
    each("C2", 0x0002, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        inp.ba_all(|_| 0).total_bands(1).group_size(3);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C3 / C4 -- linear path (ba 1..=16), group_size 1 and 12
// --------------------------------------------------------------------------
#[test]
fn c03_linear_group_size_1() {
    each("C3", 0x0003, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        ba_from(&mut inp, rng, &linear_set());
        inp.total_bands(1).group_size(1).pos(0);
        inp.done()
    });
}

#[test]
fn c04_linear_group_size_12() {
    each("C4", 0x0004, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        ba_from(&mut inp, rng, &linear_set());
        inp.total_bands(1).group_size(12);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C5 -- total_bands == 32: i covers all 64 in-array bitalloc indices
// --------------------------------------------------------------------------
#[test]
fn c05_all_64_bitalloc_indices() {
    each("C5", 0x0005, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        ba_from(&mut inp, rng, &linear_set());
        inp.total_bands(32).group_size(12);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C6 -- sparse bands: mixture of skipped (0) and linear bands
// --------------------------------------------------------------------------
#[test]
fn c06_sparse_bands() {
    each("C6", 0x0006, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        let mut set = vec![0u8; 4];
        set.extend(1..=16u8);
        ba_from(&mut inp, rng, &set);
        inp.total_bands(32).group_size(3);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C7 / C8 -- boundary bitalloc values of the linear path
// --------------------------------------------------------------------------
#[test]
fn c07_bitalloc_16_max_linear() {
    each("C7", 0x0007, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        inp.ba_all(|_| 16).total_bands(1).group_size(12);
        inp.done()
    });
}

#[test]
fn c08_bitalloc_1_min_linear() {
    each("C8", 0x0008, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        inp.ba_all(|_| 1).total_bands(1).group_size(12);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C9 / C10 -- grouped path with n <= 32
// --------------------------------------------------------------------------
#[test]
fn c09_grouped_small_mod() {
    each("C9", 0x0009, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        ba_from(&mut inp, rng, &[17, 18, 19, 20, 21]);
        inp.total_bands(8).group_size(3);
        inp.done()
    });
}

#[test]
fn c10_grouped_mod3_group12() {
    each("C10", 0x000A, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        inp.ba_all(|_| 17).total_bands(8).group_size(12);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C11 -- grouped path with n > 32: `next << shl` has an out-of-range shift
//        count, which C leaves undefined and x86 masks to 5 bits.
// --------------------------------------------------------------------------
#[test]
fn c11_grouped_shift_count_over_32() {
    each("C11", 0x000B, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_BIG);
        ba_from(&mut inp, rng, &(22..=31u8).collect::<Vec<u8>>());
        inp.total_bands(4).group_size(3);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C12 -- ba == 48: `2 << 31` overflows to 0, so mod == 1 and every sample is 0
// --------------------------------------------------------------------------
#[test]
fn c12_ba48_mod_one() {
    each("C12", 0x000C, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        inp.ba_all(|_| 48).total_bands(2).group_size(12);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C13 -- ba >= 49: the `2 << (ba-17)` shift count itself exceeds 31 and wraps
//        mod 32. Restricted to the values whose bit demand cannot overflow
//        `bs->pos`; the overflowing ones are row E19/E24 in the error tests.
// --------------------------------------------------------------------------
#[test]
fn c13_ba_over_48_shift_wrap() {
    each("C13", 0x000D, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_BIG);
        let set: Vec<u8> = (49..=255u8).filter(|&b| ba_is_pos_safe(b)).collect();
        assert!(!set.is_empty());
        ba_from(&mut inp, rng, &set);
        inp.total_bands(2).group_size(3);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C14 -- sweep every one of the 256 byte values of `ba`
// --------------------------------------------------------------------------
#[test]
fn c14_sweep_all_ba_values() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x000E);
    for ba in 0..=255u8 {
        for rep in 0..8 {
            // The huge-`n` values are exercised deliberately: with a small
            // `limit` the guard fires on the first call, so no wild pointer is
            // formed.
            let mut inp = Inputs::new(&mut rng, BUF_MED);
            inp.ba_all(|_| ba).total_bands(1).group_size(3).pos(rep).limit(256);
            assert_same(&libs, "C14", ba as usize * 8 + rep as usize, &inp.done());
        }
    }
}

// --------------------------------------------------------------------------
// C15 / C16 / C17 / C18 -- bitalloc index running out of its 64-byte array
// --------------------------------------------------------------------------
fn safe_any_set() -> Vec<u8> {
    (0..=255u8).filter(|&b| ba_is_pos_safe(b)).collect()
}

#[test]
fn c15_index_into_scfcod() {
    each("C15", 0x000F, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_BIG);
        ba_from(&mut inp, rng, &safe_any_set());
        inp.total_bands(33).group_size(3);
        inp.done()
    });
}

#[test]
fn c16_index_covers_all_scfcod() {
    each("C16", 0x0010, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_BIG);
        ba_from(&mut inp, rng, &safe_any_set());
        inp.total_bands(64).group_size(3);
        inp.done()
    });
}

#[test]
fn c17_index_past_struct_end() {
    each("C17", 0x0011, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_BIG);
        ba_from(&mut inp, rng, &safe_any_set());
        inp.total_bands(65).group_size(3);
        inp.done()
    });
}

#[test]
fn c18_total_bands_255() {
    each("C18", 0x0012, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_BIG);
        // 2040 band slots; keep the per-band bit demand modest so `bs->pos`
        // stays in range and the run exercises the full 510-band walk.
        ba_from(&mut inp, rng, &(0..=21u8).collect::<Vec<u8>>());
        inp.total_bands(255).group_size(3);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C19 / C20 -- bit alignment and deep start position
// --------------------------------------------------------------------------
#[test]
fn c19_unaligned_start_positions() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x0013);
    for pos in 0..8i32 {
        for i in 0..64 {
            let mut inp = Inputs::new(&mut rng, BUF_MED);
            ba_from(&mut inp, &mut rng, &linear_set());
            inp.total_bands(8).group_size(3).pos(pos);
            assert_same(&libs, "C19", (pos * 64 + i) as usize, &inp.done());
        }
    }
}

#[test]
fn c20_large_start_position() {
    each("C20", 0x0014, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_BIG);
        ba_from(&mut inp, rng, &linear_set());
        let pos = rng.range(0, 2_000_000) as i32;
        inp.total_bands(8).group_size(3).pos(pos);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C21 / C22 -- the exhaustion guard exactly on, and inside, the run
// --------------------------------------------------------------------------
#[test]
fn c21_limit_exactly_on_boundary() {
    // ba == 8, total_bands == 4 -> 4 j * 8 bands * 3 samples * 8 bits.
    let libs = Libs::load();
    let mut rng = Rng::new(0x0015);
    let total_bits = 4 * 8 * 3 * 8;
    for (i, delta) in [-1i32, 0, 1].iter().enumerate() {
        for rep in 0..64 {
            let mut inp = Inputs::new(&mut rng, BUF_MED);
            inp.ba_all(|_| 8).total_bands(4).group_size(3).pos(0).limit(total_bits + delta);
            assert_same(&libs, "C21", i * 64 + rep, &inp.done());
        }
    }
}

#[test]
fn c22_limit_mid_run() {
    each("C22", 0x0016, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        ba_from(&mut inp, rng, &linear_set());
        let lim = rng.range(0, 20_000) as i32;
        inp.total_bands(16).group_size(12).limit(lim);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C23 / C24 -- degenerate bitstream content
// --------------------------------------------------------------------------
#[test]
fn c23_buf_all_zero() {
    each("C23", 0x0017, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        let mut set = linear_set();
        set.extend([17u8, 18, 19, 20, 21]);
        ba_from(&mut inp, rng, &set);
        inp.buf_fill(0x00).total_bands(16).group_size(12);
        inp.done()
    });
}

#[test]
fn c24_buf_all_ones() {
    each("C24", 0x0018, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        let mut set = linear_set();
        set.extend([17u8, 18, 19, 20, 21]);
        ba_from(&mut inp, rng, &set);
        inp.buf_fill(0xFF).total_bands(16).group_size(12);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C25 / C26 / C27 / C28 -- group_size shapes
// --------------------------------------------------------------------------
#[test]
fn c25_group_size_zero() {
    each("C25", 0x0019, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        let mut set = linear_set();
        set.extend([0u8, 17, 18, 19, 20, 21]);
        ba_from(&mut inp, rng, &set);
        inp.total_bands(8).group_size(0);
        inp.done()
    });
}

#[test]
fn c26_group_size_negative() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x001A);
    for (i, gs) in [-1i32, -7, -33].iter().enumerate() {
        for rep in 0..64 {
            let mut inp = Inputs::new(&mut rng, BUF_MED);
            let mut set = linear_set();
            set.extend([0u8, 17, 18, 19, 20, 21]);
            ba_from(&mut inp, &mut rng, &set);
            inp.total_bands(8).group_size(*gs);
            assert_same(&libs, "C26", i * 64 + rep, &inp.done());
        }
    }
}

#[test]
fn c27_group_size_oversized() {
    each("C27", 0x001B, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_BIG);
        ba_from(&mut inp, rng, &linear_set());
        inp.total_bands(8).group_size(64);
        inp.done()
    });
}

#[test]
fn c28_group_size_2_odd_band_count() {
    each("C28", 0x001C, ITERS, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_MED);
        let mut set = linear_set();
        set.extend([0u8, 17, 18, 19, 20, 21, 48]);
        ba_from(&mut inp, rng, &set);
        inp.total_bands(17).group_size(2);
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C29 -- joint fuzz over every axis at once
// --------------------------------------------------------------------------
#[test]
fn c29_joint_fuzz() {
    each("C29", 0x001D, 6000, |rng, _| {
        let mut inp = Inputs::new(rng, BUF_BIG);
        let set = safe_any_set();
        ba_from(&mut inp, rng, &set);
        let tb = rng.range(0, 70) as u8;
        let gs = rng.range(-8, 16) as i32;
        let pos = rng.range(-64, 100_000) as i32;
        let lim = rng.range(-64, 400_000) as i32;
        inp.total_bands(tb).group_size(gs).pos(pos).limit(lim);
        inp.stereo_bands(rng.u8());
        inp.done()
    });
}

// --------------------------------------------------------------------------
// C30 -- there is no binary/driver to compare stdout for. Assert that, rather
//        than silently skipping the requirement.
// --------------------------------------------------------------------------
#[test]
fn c30_no_binary_target_exists() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cmake = std::fs::read_to_string(root.parent().unwrap().join("c_src/CMakeLists.txt"))
        .expect("read CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "the C project now builds an executable; a stdout differential test is required"
    );
    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).expect("read Cargo.toml");
    assert!(!cargo.contains("[[bin]]"), "the crate now has a binary target");
    assert!(
        !root.join("src/main.rs").exists(),
        "src/main.rs appeared; a stdout differential test is required"
    );
}
