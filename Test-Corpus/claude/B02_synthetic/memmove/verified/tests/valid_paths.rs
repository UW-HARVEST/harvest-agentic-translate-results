//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`.  Every case is executed against **both**
//! shared objects (`libcdriver.so` built from the pristine C, and
//! `libdriver.so` built from the Rust translation), both reached through
//! `dlopen`/`dlsym`, and the return value plus the *entire* scratch buffer are
//! compared byte for byte.

mod common;

use common::*;

/// Lengths the original CLI can produce (`main.c` rejects `length > 256`).
const LEN_CLI: &[usize] = &[
    1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 254, 255, 256,
];

/// Small lengths only — for the guard-boundary rows.
const LEN_TINY: &[usize] = &[1, 2, 3, 4, 5, 6, 7];

/// Lengths beyond the CLI limit; reachable through the raw C ABI.
/// `<= 512` keeps `rotate_buffer`'s `uint8_t temp[256]` in bounds (ERRORS.md U3).
const LEN_BIG_ROTATE_SAFE: &[usize] = &[257, 258, 300, 383, 384, 400, 511, 512];

/// Lengths that make `interleave_halves` take its `half > 256` in-place branch.
const LEN_HUGE: &[usize] = &[513, 514, 515, 600, 700, 1000, 1023, 1024];

fn draw_param1(rng: &mut Rng) -> i32 {
    match rng.below(12) {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 255,
        6 => 256,
        7 => i32::MIN,
        8 => i32::MAX,
        9 => rng.range_i32(-300, 300),
        10 => rng.range_i32(-40, 40),
        _ => rng.range_i32(1, 300),
    }
}

fn draw_param2(rng: &mut Rng) -> i32 {
    match rng.below(8) {
        0 | 1 | 2 => 0,
        3 => 1,
        4 => -1,
        5 => i32::MIN,
        6 => i32::MAX,
        _ => {
            let v = rng.range_i32(-1000, 1000);
            if v == 0 {
                7
            } else {
                v
            }
        }
    }
}

// ===========================================================================
// Section F — the full 32-way flag cross product
// ===========================================================================

fn flag_row(flags: u32, seed: u64) {
    let mut rng = Rng::new(seed);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            for _ in 0..6 {
                let data = make_input(shape, len, &mut rng);
                let p1 = draw_param1(&mut rng);
                let p2 = draw_param2(&mut rng);
                assert_same(&data, len, flags, p1, p2);
            }
        }
    }
}

macro_rules! flag_tests {
    ($($name:ident = $flags:expr;)*) => {
        $(#[test] fn $name() { flag_row($flags, 0x5EED_0000 + $flags as u64); })*
    };
}

flag_tests! {
    f00_none                            = 0x00;
    f01_rotate                          = 0x01;
    f02_compact                         = 0x02;
    f03_rotate_compact                  = 0x03;
    f04_dedup                           = 0x04;
    f05_rotate_dedup                    = 0x05;
    f06_compact_dedup                   = 0x06;
    f07_rotate_compact_dedup            = 0x07;
    f08_interleave                      = 0x08;
    f09_rotate_interleave               = 0x09;
    f10_compact_interleave              = 0x0A;
    f11_rotate_compact_interleave       = 0x0B;
    f12_dedup_interleave                = 0x0C;
    f13_rotate_dedup_interleave         = 0x0D;
    f14_compact_dedup_interleave        = 0x0E;
    f15_rot_comp_dedup_inter            = 0x0F;
    f16_reverse                         = 0x10;
    f17_rotate_reverse                  = 0x11;
    f18_compact_reverse                 = 0x12;
    f19_rotate_compact_reverse          = 0x13;
    f20_dedup_reverse                   = 0x14;
    f21_rotate_dedup_reverse            = 0x15;
    f22_compact_dedup_reverse           = 0x16;
    f23_rot_comp_dedup_rev              = 0x17;
    f24_interleave_reverse              = 0x18;
    f25_rotate_interleave_reverse       = 0x19;
    f26_compact_interleave_reverse      = 0x1A;
    f27_rot_comp_inter_rev              = 0x1B;
    f28_dedup_interleave_reverse        = 0x1C;
    f29_rot_dedup_inter_rev             = 0x1D;
    f30_comp_dedup_inter_rev            = 0x1E;
    f31_all_five                        = 0x1F;
}

// ===========================================================================
// Section R — rotate_buffer branch matrix
// ===========================================================================

#[test]
fn r1_offset_zero_param1_zero() {
    let mut rng = Rng::new(0x1001);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            let data = make_input(shape, len, &mut rng);
            assert_same(&data, len, 0x01, 0, 0);
        }
    }
}

#[test]
fn r2_offset_zero_multiples_of_length() {
    let mut rng = Rng::new(0x1002);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            let data = make_input(shape, len, &mut rng);
            for k in [-3i64, -2, -1, 1, 2, 3, 7] {
                let p1 = (k * len as i64) as i32;
                assert_same(&data, len, 0x01, p1, 0);
            }
        }
    }
}

#[test]
fn r3_small_offset_branch() {
    let mut rng = Rng::new(0x1003);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            if len < 4 {
                continue;
            }
            let data = make_input(shape, len, &mut rng);
            for off in 1..(len / 2) {
                assert_same(&data, len, 0x01, off as i32, 0);
            }
        }
    }
}

#[test]
fn r4_offset_equals_half_length() {
    let mut rng = Rng::new(0x1004);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            if len < 2 {
                continue;
            }
            let data = make_input(shape, len, &mut rng);
            assert_same(&data, len, 0x01, (len / 2) as i32, 0);
        }
    }
}

#[test]
fn r5_large_offset_branch() {
    let mut rng = Rng::new(0x1005);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            if len < 2 {
                continue;
            }
            let data = make_input(shape, len, &mut rng);
            for off in (len / 2)..len {
                assert_same(&data, len, 0x01, off as i32, 0);
            }
        }
    }
}

#[test]
fn r6_extreme_offsets() {
    let mut rng = Rng::new(0x1006);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            let data = make_input(shape, len, &mut rng);
            assert_same(&data, len, 0x01, 1, 0);
            assert_same(&data, len, 0x01, (len as i32) - 1, 0);
            assert_same(&data, len, 0x01, -1, 0);
            assert_same(&data, len, 0x01, -((len as i32) - 1), 0);
        }
    }
}

#[test]
fn r7_offsets_beyond_length() {
    let mut rng = Rng::new(0x1007);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            let data = make_input(shape, len, &mut rng);
            for p1 in [
                len as i32 + 1,
                len as i32 + 7,
                3 * len as i32 + 5,
                100_000,
                1 << 30,
                i32::MAX,
            ] {
                assert_same(&data, len, 0x01, p1, 0);
            }
        }
    }
}

#[test]
fn r8_negative_offsets() {
    let mut rng = Rng::new(0x1008);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            let data = make_input(shape, len, &mut rng);
            for p1 in [
                -1,
                -2,
                -(len as i32) + 1,
                -(len as i32) - 3,
                -100_000,
                i32::MIN,
                i32::MIN + 1,
            ] {
                assert_same(&data, len, 0x01, p1, 0);
            }
        }
    }
}

#[test]
fn r9_length_one() {
    let mut rng = Rng::new(0x1009);
    for &shape in &ALL_SHAPES {
        let data = make_input(shape, 1, &mut rng);
        for p1 in [i32::MIN, -5, -1, 0, 1, 5, i32::MAX] {
            assert_same(&data, 1, 0x01, p1, 0);
        }
    }
}

#[test]
fn r10_tiny_lengths() {
    let mut rng = Rng::new(0x100A);
    for &shape in &ALL_SHAPES {
        for &len in LEN_TINY {
            let data = make_input(shape, len, &mut rng);
            for p1 in -8..=8 {
                assert_same(&data, len, 0x01, p1, 0);
            }
        }
    }
}

#[test]
fn r11_multi_chunk_small_offset() {
    // The `for (i = 0; i < offset; i += chunk)` loop only runs more than once
    // when `chunk == 256 < offset`, and the small-offset branch additionally
    // needs `offset < len/2`.  So `offset >= 257` and `len >= 2*offset + 1`.
    let mut rng = Rng::new(0x100B);
    let mut iteration_counts = std::collections::BTreeSet::new();
    for &shape in &ALL_SHAPES {
        for &len in &[
            514usize, 515, 516, 600, 700, 1000, 1023, 1024, 1100, 1400, 1537, 2048, 2049, 3000,
        ] {
            let data = make_input(shape, len, &mut rng);
            for off in [
                256usize,
                257,
                258,
                300,
                511,
                512,
                513,
                600,
                700,
                1023,
                1024,
                1025,
                len / 2 - 1,
            ] {
                if off >= 256 && off < len / 2 {
                    // ceil(off / 256) loop iterations
                    iteration_counts.insert(off.div_ceil(256));
                    assert_same(&data, len, 0x01, off as i32, 0);
                }
            }
        }
    }
    // Prove the loop really ran 1, 2, 3, 4 and 5 times somewhere in the sweep.
    for n in 1..=5usize {
        assert!(
            iteration_counts.contains(&n),
            "the multi-chunk loop never ran exactly {n} time(s); observed {iteration_counts:?}"
        );
    }
}

#[test]
fn r12_lengths_between_257_and_512() {
    let mut rng = Rng::new(0x100C);
    for &shape in &ALL_SHAPES {
        for &len in LEN_BIG_ROTATE_SAFE {
            let data = make_input(shape, len, &mut rng);
            for off in [
                1usize,
                2,
                len / 4,
                len / 2 - 1,
                len / 2,
                len / 2 + 1,
                len - 2,
                len - 1,
            ] {
                assert_same(&data, len, 0x01, off as i32, 0);
                assert_same(&data, len, 0x01, -(off as i32), 0);
            }
        }
    }
}

// ===========================================================================
// Section C — compact_runs branch matrix
// ===========================================================================

fn compact_row(param1: i32, lengths: &[usize], seed: u64) {
    let mut rng = Rng::new(seed);
    for &shape in &ALL_SHAPES {
        for &len in lengths {
            for _ in 0..3 {
                let data = make_input(shape, len, &mut rng);
                assert_same(&data, len, 0x02, param1, 0);
            }
        }
    }
}

#[test]
fn c1_threshold_one_grows() {
    compact_row(1, LEN_CLI, 0x2001);
    compact_row(1, LEN_BIG_ROTATE_SAFE, 0x2001_1);
    compact_row(1, LEN_HUGE, 0x2001_2);
}

#[test]
fn c2_threshold_two() {
    compact_row(2, LEN_CLI, 0x2002);
    compact_row(2, LEN_HUGE, 0x2002_1);
}

#[test]
fn c3_threshold_three_explicit() {
    compact_row(3, LEN_CLI, 0x2003);
}

#[test]
fn c4_mid_thresholds() {
    let mut rng = Rng::new(0x2004);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            for _ in 0..8 {
                let data = make_input(shape, len, &mut rng);
                let t = rng.range_i32(4, 254);
                assert_same(&data, len, 0x02, t, 0);
            }
        }
    }
}

#[test]
fn c5_threshold_255() {
    compact_row(255, LEN_CLI, 0x2005);
    compact_row(255, LEN_BIG_ROTATE_SAFE, 0x2005_1);
    compact_row(255, LEN_HUGE, 0x2005_2);
}

#[test]
fn c6_param1_non_positive_defaults_to_three() {
    for p1 in [0, -1, -3, -255, -100_000, i32::MIN] {
        compact_row(p1, LEN_CLI, 0x2006u64.wrapping_add(p1 as i64 as u64));
    }
}

#[test]
fn c7_param1_above_255_defaults_to_three() {
    for p1 in [256, 257, 1000, 65536, i32::MAX] {
        compact_row(p1, LEN_CLI, 0x2007u64.wrapping_add(p1 as i64 as u64));
    }
}

#[test]
fn c8_run_longer_than_255_clamped() {
    let mut rng = Rng::new(0x2008);
    for &len in &[256usize, 257, 300, 400, 510, 511, 512, 513, 600, 1000, 1024] {
        let data = make_input(Shape::Constant, len, &mut rng);
        for t in [1, 2, 3, 4, 200, 254, 255] {
            assert_same(&data, len, 0x02, t, 0);
        }
    }
}

#[test]
fn c9_long_runs_repeated_clamping() {
    let mut rng = Rng::new(0x2009);
    for &len in &[256usize, 300, 511, 512, 600, 760, 1000, 1024] {
        for _ in 0..4 {
            let data = make_input(Shape::LongRuns, len, &mut rng);
            for t in [1, 2, 3, 100, 250, 255] {
                assert_same(&data, len, 0x02, t, 0);
            }
        }
    }
}

#[test]
fn c10_final_run_ends_exactly_at_len() {
    // Hand-built inputs whose last run terminates at `len`, for thresholds on
    // both sides of the final run length.
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for tail in 1usize..=8 {
        for head in 0usize..=8 {
            let mut v: Vec<u8> = Vec::new();
            for i in 0..head {
                v.push((i % 251) as u8);
            }
            for _ in 0..tail {
                v.push(0xEE);
            }
            cases.push(v);
        }
    }
    for data in cases {
        for t in 1..=10 {
            assert_same(&data, data.len(), 0x02, t, 0);
        }
    }
}

#[test]
fn c12_growth_reaches_exactly_twice_the_length() {
    // `threshold == 1` compacts *every* run to two bytes, so the logical length
    // can reach - but must never exceed - `2 * length`, the write window the FFI
    // wrapper hands to the translation.  (Note that the C code writes the run
    // *count* into `buf[write+1]` *before* moving the tail, so the shifted data
    // is the already-overwritten byte, not the original one.  That quirk is part
    // of the contract and is reproduced exactly.)
    let mut rng = Rng::new(0x2010);
    let mut saw_max = false;
    for &len in &[
        1usize, 2, 3, 4, 5, 8, 16, 17, 63, 127, 128, 255, 256, 257, 300, 512, 513, 700, 1000, 1024,
    ] {
        for &shape in &ALL_SHAPES {
            for _ in 0..4 {
                let data = make_input(shape, len, &mut rng);
                let (c, _) = run_both(&data, len, 0x02, 1, 0);
                assert!(
                    c.ret <= 2 * len,
                    "compact_runs exceeded the 2*length window: ret={} len={len} shape={shape:?}",
                    c.ret
                );
                if c.ret == 2 * len {
                    // `buf[ret-1]` is written by the last `{value, count}` pair,
                    // so the very last byte of the window is exercised here.
                    saw_max = true;
                }
            }
        }
    }
    assert!(saw_max, "the 2*length upper bound was never actually reached");
}

#[test]
fn c11_single_byte() {
    for b in [0u8, 1, 127, 128, 254, 255] {
        let data = vec![b];
        for t in [1, 2, 3, 255, 256, 0, -1, i32::MIN, i32::MAX] {
            assert_same(&data, 1, 0x02, t, 0);
        }
    }
}

// ===========================================================================
// Section D — remove_duplicates branch matrix
// ===========================================================================

fn dedup_row(param2: i32, lengths: &[usize], seed: u64) {
    let mut rng = Rng::new(seed);
    for &shape in &ALL_SHAPES {
        for &len in lengths {
            for _ in 0..4 {
                let data = make_input(shape, len, &mut rng);
                assert_same(&data, len, 0x04, 0, param2);
            }
        }
    }
}

#[test]
fn d1_unordered_path() {
    dedup_row(0, LEN_CLI, 0x3001);
    dedup_row(0, LEN_HUGE, 0x3001_1);
}

#[test]
fn d2_order_preserving_path() {
    dedup_row(1, LEN_CLI, 0x3002);
    dedup_row(1, LEN_HUGE, 0x3002_1);
}

#[test]
fn d3_other_nonzero_param2() {
    for p2 in [-1, 2, -42, i32::MIN, i32::MAX, 0x0100_0000] {
        dedup_row(p2, LEN_CLI, 0x3003u64.wrapping_add(p2 as i64 as u64));
    }
}

#[test]
fn d4_length_one() {
    let mut rng = Rng::new(0x3004);
    for &shape in &ALL_SHAPES {
        let data = make_input(shape, 1, &mut rng);
        for p2 in [0, 1, -1] {
            assert_same(&data, 1, 0x04, 0, p2);
        }
    }
}

#[test]
fn d5_all_distinct_full_alphabet() {
    let mut rng = Rng::new(0x3005);
    for &len in &[255usize, 256, 257, 300, 512, 700, 1024] {
        let data = make_input(Shape::AllDistinct, len, &mut rng);
        for p2 in [0, 1] {
            assert_same(&data, len, 0x04, 0, p2);
        }
        // A permutation of the full byte range as well.
        let mut perm: Vec<u8> = (0..=255u8).collect();
        for i in (1..perm.len()).rev() {
            let j = rng.below(i + 1);
            perm.swap(i, j);
        }
        let mut d2 = Vec::with_capacity(len);
        while d2.len() < len {
            let take = (len - d2.len()).min(perm.len());
            d2.extend_from_slice(&perm[..take]);
        }
        for p2 in [0, 1] {
            assert_same(&d2, len, 0x04, 0, p2);
        }
    }
}

#[test]
fn d6_constant_collapses_to_one() {
    let mut rng = Rng::new(0x3006);
    for &len in LEN_CLI {
        let data = make_input(Shape::Constant, len, &mut rng);
        for p2 in [0, 1] {
            assert_same(&data, len, 0x04, 0, p2);
        }
    }
}

// ===========================================================================
// Section I — interleave_halves branch matrix
// ===========================================================================

#[test]
fn i1_even_lengths_temp_branch() {
    let mut rng = Rng::new(0x4001);
    for &shape in &ALL_SHAPES {
        for len in (2..=512).step_by(2) {
            let data = make_input(shape, len, &mut rng);
            assert_same(&data, len, 0x08, 0, 0);
        }
    }
}

#[test]
fn i2_odd_lengths_temp_branch() {
    let mut rng = Rng::new(0x4002);
    for &shape in &ALL_SHAPES {
        for len in (3..=511).step_by(2) {
            let data = make_input(shape, len, &mut rng);
            assert_same(&data, len, 0x08, 0, 0);
        }
    }
}

#[test]
fn i3_smallest_sizes() {
    let mut rng = Rng::new(0x4003);
    for &shape in &ALL_SHAPES {
        for len in [1usize, 2, 3, 4, 5] {
            for _ in 0..8 {
                let data = make_input(shape, len, &mut rng);
                assert_same(&data, len, 0x08, 0, 0);
            }
        }
    }
}

#[test]
fn i4_half_exactly_256() {
    let mut rng = Rng::new(0x4004);
    for &shape in &ALL_SHAPES {
        for len in [510usize, 511, 512, 513] {
            for _ in 0..4 {
                let data = make_input(shape, len, &mut rng);
                assert_same(&data, len, 0x08, 0, 0);
            }
        }
    }
}

#[test]
fn i5_in_place_branch() {
    let mut rng = Rng::new(0x4005);
    for &shape in &ALL_SHAPES {
        for &len in LEN_HUGE {
            if len < 514 {
                continue;
            }
            for _ in 0..3 {
                let data = make_input(shape, len, &mut rng);
                assert_same(&data, len, 0x08, 0, 0);
            }
        }
    }
}

#[test]
fn i6_in_place_branch_via_compact_growth() {
    // `flags = 0x0A`, threshold 1: compact doubles the length, so `new_len`
    // crosses 513 and interleave takes the in-place branch.
    let mut rng = Rng::new(0x4006);
    for &shape in &ALL_SHAPES {
        for &len in &[258usize, 260, 300, 383, 400, 511, 512] {
            for _ in 0..3 {
                let data = make_input(shape, len, &mut rng);
                assert_same(&data, len, 0x0A, 1, 0);
            }
        }
    }
}

// ===========================================================================
// Section V — reverse_segments branch matrix
// ===========================================================================

#[test]
fn v1_seg_size_one_early_return() {
    let mut rng = Rng::new(0x5001);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            let data = make_input(shape, len, &mut rng);
            assert_same(&data, len, 0x10, 1, 0);
        }
    }
}

#[test]
fn v2_seg_size_two() {
    let mut rng = Rng::new(0x5002);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            let data = make_input(shape, len, &mut rng);
            assert_same(&data, len, 0x10, 2, 0);
        }
    }
}

#[test]
fn v3_seg_size_three() {
    let mut rng = Rng::new(0x5003);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            let data = make_input(shape, len, &mut rng);
            assert_same(&data, len, 0x10, 3, 0);
        }
    }
}

#[test]
fn v4_default_seg_size_four() {
    let mut rng = Rng::new(0x5004);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            let data = make_input(shape, len, &mut rng);
            for p1 in [0, -1, -4, -100_000, i32::MIN] {
                assert_same(&data, len, 0x10, p1, 0);
            }
        }
    }
}

#[test]
fn v5_v6_v7_remainder_classes() {
    let mut rng = Rng::new(0x5005);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            if len < 4 {
                continue;
            }
            let data = make_input(shape, len, &mut rng);
            let mut seen_r0 = false;
            let mut seen_r1 = false;
            let mut seen_rn = false;
            for seg in 2..=len {
                match len % seg {
                    0 => seen_r0 = true,
                    1 => seen_r1 = true,
                    _ => seen_rn = true,
                }
                assert_same(&data, len, 0x10, seg as i32, 0);
            }
            assert!(seen_r0, "len={len}: no remainder==0 segment size");
            let _ = (seen_r1, seen_rn);
        }
    }
}

#[test]
fn v8_v9_single_segment() {
    let mut rng = Rng::new(0x5006);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            if len < 4 {
                continue;
            }
            let data = make_input(shape, len, &mut rng);
            assert_same(&data, len, 0x10, len as i32, 0);
            assert_same(&data, len, 0x10, len as i32 - 1, 0);
        }
    }
}

#[test]
fn v10_seg_size_above_length_skipped() {
    let mut rng = Rng::new(0x5007);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            let data = make_input(shape, len, &mut rng);
            for p1 in [len as i32 + 1, len as i32 + 2, 100_000, i32::MAX] {
                assert_same(&data, len, 0x10, p1, 0);
            }
        }
    }
}

#[test]
fn v11_lengths_below_four_skipped() {
    let mut rng = Rng::new(0x5008);
    for &shape in &ALL_SHAPES {
        for len in [1usize, 2, 3] {
            let data = make_input(shape, len, &mut rng);
            for p1 in [-1, 0, 1, 2, 3, 4, 5] {
                assert_same(&data, len, 0x10, p1, 0);
            }
        }
    }
}

#[test]
fn v12_random_segment_sizes() {
    let mut rng = Rng::new(0x5009);
    for _ in 0..4000 {
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let len = 4 + rng.below(1021);
        let data = make_input(shape, len, &mut rng);
        let seg = rng.range_i32(2, 300);
        assert_same(&data, len, 0x10, seg, 0);
    }
}

// ===========================================================================
// Section X — pipeline interactions
// ===========================================================================

#[test]
fn x1_compact_then_dedup() {
    let mut rng = Rng::new(0x6001);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            for p1 in [1, 2, 3, 5, 255] {
                for p2 in [0, 1] {
                    let data = make_input(shape, len, &mut rng);
                    assert_same(&data, len, 0x06, p1, p2);
                }
            }
        }
    }
}

#[test]
fn x2_dedup_shrinks_below_two() {
    let mut rng = Rng::new(0x6002);
    for &len in LEN_CLI {
        for shape in [Shape::Constant, Shape::SmallAlphabet, Shape::Alternating] {
            for p2 in [0, 1] {
                let data = make_input(shape, len, &mut rng);
                assert_same(&data, len, 0x0C, 0, p2);
            }
        }
    }
}

#[test]
fn x3_dedup_shrinks_below_four() {
    let mut rng = Rng::new(0x6003);
    for &len in LEN_CLI {
        for shape in [Shape::Constant, Shape::SmallAlphabet, Shape::Alternating] {
            for p1 in [0, 1, 2, 3, 4] {
                for p2 in [0, 1] {
                    let data = make_input(shape, len, &mut rng);
                    assert_same(&data, len, 0x14, p1, p2);
                }
            }
        }
    }
}

#[test]
fn x4_dedup_then_interleave_then_reverse() {
    let mut rng = Rng::new(0x6004);
    for &len in LEN_CLI {
        for shape in [Shape::Constant, Shape::SmallAlphabet, Shape::Alternating] {
            for p1 in [0, 1, 2, 3, 4, 5] {
                for p2 in [0, 1] {
                    let data = make_input(shape, len, &mut rng);
                    assert_same(&data, len, 0x1C, p1, p2);
                }
            }
        }
    }
}

#[test]
fn x5_shared_param1_threshold_and_segsize() {
    let mut rng = Rng::new(0x6005);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            for p1 in [1, 2, 3, 4, 8, 255, 256, 0, -3] {
                let data = make_input(shape, len, &mut rng);
                assert_same(&data, len, 0x12, p1, 0);
            }
        }
    }
}

#[test]
fn x6_shared_param1_offset_threshold_segsize() {
    let mut rng = Rng::new(0x6006);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            for p1 in 1..=40 {
                let data = make_input(shape, len, &mut rng);
                assert_same(&data, len, 0x13, p1, 0);
            }
        }
    }
}

#[test]
fn x7_full_pipeline_param1_one() {
    let mut rng = Rng::new(0x6007);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            for p2 in [0, 1] {
                let data = make_input(shape, len, &mut rng);
                assert_same(&data, len, 0x1F, 1, p2);
            }
        }
    }
}

#[test]
fn x8_full_pipeline_param_matrix() {
    let mut rng = Rng::new(0x6008);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            for p1 in [2, 3, 4, 5, 255, 256, -7, 0, i32::MIN, i32::MAX] {
                for p2 in [0, 1, -1] {
                    let data = make_input(shape, len, &mut rng);
                    assert_same(&data, len, 0x1F, p1, p2);
                }
            }
        }
    }
}

#[test]
fn x9_unknown_flag_bits_ignored() {
    let mut rng = Rng::new(0x6009);
    for &shape in &ALL_SHAPES {
        for &len in LEN_CLI {
            for base in 0x00u32..0x20 {
                let data = make_input(shape, len, &mut rng);
                let p1 = draw_param1(&mut rng);
                let p2 = draw_param2(&mut rng);
                // The three encodings must behave identically to each other …
                assert_same(&data, len, base, p1, p2);
                assert_same(&data, len, base | 0xFFFF_FFE0, p1, p2);
                assert_same(&data, len, base | 0x20, p1, p2);
            }
        }
    }
}

#[test]
fn x9b_unknown_bits_equal_masked_value() {
    // Stronger form of X9: C(flags) must equal C(flags & 0x1F) *and*
    // Rust(flags) must equal Rust(flags & 0x1F).
    let cf = c_process_buffer();
    let rf = rust_process_buffer();
    let mut rng = Rng::new(0x600A);
    for _ in 0..3000 {
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let len = 1 + rng.below(256);
        let data = make_input(shape, len, &mut rng);
        let flags = rng.next_u32();
        let p1 = draw_param1(&mut rng);
        let p2 = draw_param2(&mut rng);

        for f in [cf, rf] {
            let cap = window(len, flags | 0x02) + GUARD;
            let mut a = vec![0xA5u8; cap];
            a[..len].copy_from_slice(&data);
            let mut b = a.clone();
            let ra = unsafe { f(a.as_mut_ptr(), len, flags, p1, p2) };
            let rb = unsafe { f(b.as_mut_ptr(), len, flags & 0x1F, p1, p2) };
            assert_eq!(ra, rb, "flags={flags:#x} len={len} p1={p1} p2={p2}");
            assert_eq!(a, b, "flags={flags:#x} len={len} p1={p1} p2={p2}");
        }
        assert_same(&data, len, flags, p1, p2);
    }
}

#[test]
fn x10_random_fuzz() {
    let mut rng = Rng::new(0x600B);
    for _ in 0..20_000 {
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let len = 1 + rng.below(256);
        let data = make_input(shape, len, &mut rng);
        let flags = rng.next_u32();
        let p1 = if rng.below(2) == 0 {
            rng.next_u32() as i32
        } else {
            draw_param1(&mut rng)
        };
        let p2 = if rng.below(2) == 0 {
            rng.next_u32() as i32
        } else {
            draw_param2(&mut rng)
        };
        assert_same(&data, len, flags, p1, p2);
    }
}

#[test]
fn x10b_random_fuzz_large_lengths() {
    // `length > 512` is only safe for the C code when rotation either is off or
    // stays in the small-offset branch (ERRORS.md U3), so bit 0 is cleared here.
    let mut rng = Rng::new(0x600C);
    for _ in 0..4000 {
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let len = 257 + rng.below(768);
        let data = make_input(shape, len, &mut rng);
        let flags = rng.next_u32() & !0x01;
        let p1 = draw_param1(&mut rng);
        let p2 = draw_param2(&mut rng);
        assert_same(&data, len, flags, p1, p2);
    }
}
