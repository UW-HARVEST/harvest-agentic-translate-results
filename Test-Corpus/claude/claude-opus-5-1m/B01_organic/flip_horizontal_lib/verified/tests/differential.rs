//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through `libloading` and compares the resulting pixel buffers
//! byte-for-byte. All randomized rows use a fixed seed so failures reproduce.

mod common;

use common::{assert_same, assert_same_and_noop, both, Case, Rng, PIXEL_SIZE};

/// Number of randomized inputs per property-style row.
const REPS: usize = 200;

// ===========================================================================
// Rows 1-8 — shapes that perform real work
// ===========================================================================

/// CONFIGS 1: `h == 2, w == 1` — smallest input that performs work.
#[test]
fn cfg01_min_working() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0001);
    for i in 0..REPS {
        let case = Case::exact(&mut rng, 1, 2);
        assert_same(&c, &r, &case, &format!("cfg01[{i}]"));
    }
}

/// CONFIGS 2: `h == 2`, random `w` in 2..=64 — a single row swap.
#[test]
fn cfg02_h2_w_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0002);
    for i in 0..REPS {
        let w = rng.range_i32(2, 64);
        let case = Case::exact(&mut rng, w, 2);
        assert_same(&c, &r, &case, &format!("cfg02[{i}] w={w}"));
    }
}

/// CONFIGS 3: `h == 3` — odd height, the middle row must stay untouched.
#[test]
fn cfg03_h3_odd_middle_untouched() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0003);
    for i in 0..REPS {
        let w = rng.range_i32(1, 64);
        let case = Case::exact(&mut rng, w, 3);
        // Differential first.
        assert_same(&c, &r, &case, &format!("cfg03[{i}] w={w}"));
        // Plus the structural property the C guarantees: row 1 is untouched
        // and rows 0/2 are exchanged.
        let stride = w as usize * PIXEL_SIZE;
        let mut buf = case.data.clone();
        let mut img = common::CpImage {
            w: case.w,
            h: case.h,
            pix: buf.as_mut_ptr().cast(),
        };
        unsafe { r.flip(&mut img) };
        assert_eq!(&buf[stride..2 * stride], &case.data[stride..2 * stride], "middle row moved");
        assert_eq!(&buf[..stride], &case.data[2 * stride..3 * stride], "row0 != old row2");
        assert_eq!(&buf[2 * stride..], &case.data[..stride], "row2 != old row0");
    }
}

/// CONFIGS 4: random even `h` in 4..=64, random `w` in 1..=64.
#[test]
fn cfg04_h_even_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0004);
    for i in 0..REPS {
        let h = rng.range_i32(2, 32) * 2; // 4..=64, always even
        let w = rng.range_i32(1, 64);
        let case = Case::exact(&mut rng, w, h);
        assert_same(&c, &r, &case, &format!("cfg04[{i}] w={w} h={h}"));
    }
}

/// CONFIGS 5: random odd `h` in 5..=65, random `w` in 1..=64.
#[test]
fn cfg05_h_odd_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0005);
    for i in 0..REPS {
        let h = rng.range_i32(2, 32) * 2 + 1; // 5..=65, always odd
        let w = rng.range_i32(1, 64);
        let case = Case::exact(&mut rng, w, h);
        assert_same(&c, &r, &case, &format!("cfg05[{i}] w={w} h={h}"));
    }
}

/// CONFIGS 6: `w == 1`, tall images — one pixel per row.
#[test]
fn cfg06_w1_tall() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0006);
    for i in 0..REPS {
        let h = rng.range_i32(2, 128);
        let case = Case::exact(&mut rng, 1, h);
        assert_same(&c, &r, &case, &format!("cfg06[{i}] h={h}"));
    }
}

/// CONFIGS 7: `h == 2` with very wide rows (many inner iterations).
#[test]
fn cfg07_wide_rows() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0007);
    for i in 0..24 {
        let w = rng.range_i32(512, 2048);
        let case = Case::exact(&mut rng, w, 2);
        assert_same(&c, &r, &case, &format!("cfg07[{i}] w={w}"));
    }
}

/// CONFIGS 8: large area in both dimensions (up to ~64K pixels).
#[test]
fn cfg08_large_area() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0008);
    for i in 0..12 {
        let w = rng.range_i32(64, 256);
        let h = rng.range_i32(64, 256);
        let case = Case::exact(&mut rng, w, h);
        assert_same(&c, &r, &case, &format!("cfg08[{i}] w={w} h={h}"));
    }
}

// ===========================================================================
// Rows 9-19 — shapes where one of the two loop guards fails on entry
// ===========================================================================

/// CONFIGS 9 / ERRORS 4: `h == 0`, valid `pix`.
#[test]
fn cfg09_h_zero_valid_pix() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0009);
    for i in 0..REPS {
        let w = rng.range_i32(1, 64);
        let case = Case::sized(&mut rng, w, 0, 64);
        assert_same_and_noop(&c, &r, &case, &format!("cfg09[{i}] w={w}"));
    }
}

/// CONFIGS 10 / ERRORS 4+14: `h == 0`, `pix == NULL` (never dereferenced).
#[test]
fn cfg10_h_zero_null_pix() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_000A);
    for i in 0..64 {
        let w = rng.i32_any();
        let case = Case::null_pix(w, 0);
        assert_same_and_noop(&c, &r, &case, &format!("cfg10[{i}] w={w}"));
    }
}

/// CONFIGS 11 / ERRORS 5: `h == 1`, valid `pix`.
#[test]
fn cfg11_h_one_valid_pix() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_000B);
    for i in 0..REPS {
        let w = rng.range_i32(1, 64);
        let case = Case::sized(&mut rng, w, 1, 64);
        assert_same_and_noop(&c, &r, &case, &format!("cfg11[{i}] w={w}"));
    }
}

/// CONFIGS 12 / ERRORS 14: `h == 1`, `pix == NULL`.
#[test]
fn cfg12_h_one_null_pix() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_000C);
    for i in 0..64 {
        let w = rng.i32_any();
        let case = Case::null_pix(w, 1);
        assert_same_and_noop(&c, &r, &case, &format!("cfg12[{i}] w={w}"));
    }
}

/// CONFIGS 13 / ERRORS 10: `w == 0`, `h >= 2`, valid `pix`. The outer loop
/// spins `h/2` times; the inner guard `0 < 0` is false every time.
#[test]
fn cfg13_w_zero_valid_pix() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_000D);
    for i in 0..REPS {
        let h = rng.range_i32(2, 256);
        let case = Case::sized(&mut rng, 0, h, 64);
        assert_same_and_noop(&c, &r, &case, &format!("cfg13[{i}] h={h}"));
    }
}

/// CONFIGS 14 / ERRORS 10: `w == 0`, `h >= 2`, `pix == NULL`.
#[test]
fn cfg14_w_zero_null_pix() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_000E);
    for i in 0..64 {
        let h = rng.range_i32(2, 256);
        let case = Case::null_pix(0, h);
        assert_same_and_noop(&c, &r, &case, &format!("cfg14[{i}] h={h}"));
    }
}

/// CONFIGS 15 / ERRORS 11+12: `w < 0`. `pix + w*i` produces pointers far below
/// the allocation which are computed but never dereferenced.
#[test]
fn cfg15_w_negative() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_000F);
    // Deterministic boundary values first.
    for &w in &[-1i32, -2, -3, -7, -256, -(1 << 20), -(1 << 28), i32::MIN + 1] {
        for &h in &[2i32, 3, 4, 17, 64] {
            let case = Case::sized(&mut rng, w, h, 64);
            assert_same_and_noop(&c, &r, &case, &format!("cfg15 w={w} h={h}"));
        }
    }
    // Then randomized negatives.
    for i in 0..REPS {
        let w = rng.range_i32(i32::MIN + 1, -1);
        let h = rng.range_i32(2, 64);
        let case = Case::sized(&mut rng, w, h, 64);
        assert_same_and_noop(&c, &r, &case, &format!("cfg15r[{i}] w={w} h={h}"));
    }
}

/// CONFIGS 16 / ERRORS 13: `w == INT_MIN` — `w * i` signed-overflows in `int`.
#[test]
fn cfg16_w_int_min() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0010);
    for &h in &[2i32, 3, 4, 5, 8, 33, 64, 255, 256] {
        let case = Case::sized(&mut rng, i32::MIN, h, 64);
        assert_same_and_noop(&c, &r, &case, &format!("cfg16 h={h}"));
    }
}

/// CONFIGS 17 / ERRORS 6+7+8: `h < 0` -> `flips < 0`, outer loop never entered.
#[test]
fn cfg17_h_negative() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0011);
    for &h in &[-1i32, -2, -3, -4, -5, -100, -(1 << 20), i32::MIN + 1] {
        let w = rng.range_i32(1, 64);
        let case = Case::sized(&mut rng, w, h, 64);
        assert_same_and_noop(&c, &r, &case, &format!("cfg17 w={w} h={h}"));
    }
    for i in 0..REPS {
        let h = rng.range_i32(i32::MIN + 1, -1);
        let w = rng.range_i32(1, 64);
        let case = Case::sized(&mut rng, w, h, 64);
        assert_same_and_noop(&c, &r, &case, &format!("cfg17r[{i}] w={w} h={h}"));
    }
}

/// CONFIGS 18 / ERRORS 9: `h == INT_MIN` -> `flips == -1073741824`.
#[test]
fn cfg18_h_int_min() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0012);
    for &w in &[1i32, 2, 7, 64, 0, -1, i32::MAX, i32::MIN] {
        let case = Case::sized(&mut rng, w, i32::MIN, 64);
        assert_same_and_noop(&c, &r, &case, &format!("cfg18 w={w}"));
    }
}

/// CONFIGS 19 / ERRORS 16: `w == INT_MAX`, `h == 0` — `w` never used.
#[test]
fn cfg19_w_int_max_h_zero() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0013);
    for &h in &[0i32, 1, -1, i32::MIN] {
        let case = Case::sized(&mut rng, i32::MAX, h, 64);
        assert_same_and_noop(&c, &r, &case, &format!("cfg19 h={h}"));
    }
}

// ===========================================================================
// Rows 20-24 — memory footprint, repetition, struct integrity, channel lanes
// ===========================================================================

/// CONFIGS 20 / ERRORS 3: the buffer is LARGER than `w * h` and the region past
/// `w * h` is filled with a poison canary. Both implementations must touch the
/// exact same address range, leaving the canary byte-identical.
#[test]
fn cfg_padding_canary() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0014);
    for i in 0..REPS {
        let w = rng.range_i32(1, 24);
        let h = rng.range_i32(0, 24);
        let used = w as usize * h as usize;
        let pad = 1 + rng.below(64) as usize;
        let mut data = rng.bytes(used * PIXEL_SIZE);
        data.extend(std::iter::repeat_n(0xA5u8, pad * PIXEL_SIZE));
        let case = Case { w, h, data, null_pix: false, calls: 1 };
        assert_same(&c, &r, &case, &format!("cfg20[{i}] w={w} h={h} pad={pad}"));

        // Independently assert the canary survived in the Rust build too.
        let mut buf = case.data.clone();
        let mut img = common::CpImage { w, h, pix: buf.as_mut_ptr().cast() };
        unsafe { r.flip(&mut img) };
        assert!(
            buf[used * PIXEL_SIZE..].iter().all(|&b| b == 0xA5),
            "Rust wrote past w*h (w={w} h={h} pad={pad})"
        );
    }
}

/// CONFIGS 21: two calls must restore the original bytes (the operation is an
/// involution). Compared differentially as well.
#[test]
fn cfg21_double_call_involution() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0015);
    for i in 0..REPS {
        let w = rng.range_i32(1, 32);
        let h = rng.range_i32(0, 32);
        let case = Case::exact(&mut rng, w, h).with_calls(2);
        assert_same(&c, &r, &case, &format!("cfg21[{i}] w={w} h={h}"));
        // And it really is the identity.
        assert_same_and_noop(&c, &r, &case, &format!("cfg21-id[{i}] w={w} h={h}"));
    }
}

/// CONFIGS 22: three calls must equal one call.
#[test]
fn cfg22_triple_call() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0016);
    for i in 0..REPS {
        let w = rng.range_i32(1, 32);
        let h = rng.range_i32(0, 32);
        let base = Case::exact(&mut rng, w, h);
        let thrice = Case { w, h, data: base.data.clone(), null_pix: false, calls: 3 };
        assert_same(&c, &r, &thrice, &format!("cfg22[{i}] w={w} h={h}"));

        // once(C) == thrice(Rust)
        let mut b1 = base.data.clone();
        let mut i1 = common::CpImage { w, h, pix: b1.as_mut_ptr().cast() };
        unsafe { c.flip(&mut i1) };
        let mut b3 = base.data.clone();
        let mut i3 = common::CpImage { w, h, pix: b3.as_mut_ptr().cast() };
        for _ in 0..3 {
            unsafe { r.flip(&mut i3) };
        }
        assert_eq!(b1, b3, "3x Rust != 1x C (w={w} h={h})");
    }
}

/// CONFIGS 23: `cp_image_t` fields must be unmodified by the call. (Checked
/// inside `assert_same` for every row; asserted explicitly here across a wide
/// spread of field values including extremes.)
#[test]
fn cfg23_struct_fields_untouched() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0017);
    let ws = [0i32, 1, 2, 5, 64, -1, -64, i32::MAX, i32::MIN];
    let hs = [0i32, 1, 2, 3, 8, -1, -8, i32::MIN];
    for &w in &ws {
        for &h in &hs {
            // Only shapes that never dereference, plus the small valid ones.
            let derefs = w > 0 && h >= 2;
            let pixels = if derefs { (w as usize) * (h as usize) } else { 64 };
            if derefs && pixels > 4096 {
                continue; // skip absurd allocations (w=INT_MAX etc.)
            }
            let case = Case::sized(&mut rng, w, h, pixels.max(64));
            assert_same(&c, &r, &case, &format!("cfg23 w={w} h={h}"));
        }
    }
}

/// CONFIGS 24: all four channel lanes distinct, including 0x00 / 0xFF, so a
/// mixed-up channel order or a wrong element size would show up immediately.
#[test]
fn cfg24_channel_lanes() {
    let (c, r) = both();
    for &(w, h) in &[(1i32, 2i32), (2, 2), (3, 3), (4, 5), (5, 4), (7, 8), (1, 9)] {
        let px = (w * h) as usize;
        let mut data = Vec::with_capacity(px * PIXEL_SIZE);
        for p in 0..px {
            // r = index, g = 0x00, b = 0xFF, a = !index
            data.push(p as u8);
            data.push(0x00);
            data.push(0xFF);
            data.push(!(p as u8));
        }
        let case = Case { w, h, data, null_pix: false, calls: 1 };
        assert_same(&c, &r, &case, &format!("cfg24 w={w} h={h}"));
    }
}

// ===========================================================================
// Rows 25-27 — cross-product and broad property sweeps
// ===========================================================================

/// CONFIGS 25: full `(w, h)` cross-product over -2..=9 with randomized pixels,
/// on a fixed oversized buffer so every pointer is real.
#[test]
fn cfg25_wh_cross_product() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_0019);
    for w in -2i32..=9 {
        for h in -2i32..=9 {
            for rep in 0..16 {
                // 256-pixel buffer: max in-bounds use is 9*9 = 81 pixels.
                let case = Case::sized(&mut rng, w, h, 256);
                assert_same(&c, &r, &case, &format!("cfg25 w={w} h={h} rep={rep}"));
            }
        }
    }
}

/// CONFIGS 26: randomized `(w, h)` drawn from the WHOLE `int` range, restricted
/// to the shapes that never dereference `pix` (`h <= 1`, or `w <= 0` with a
/// bounded `h` so the empty outer loop terminates promptly).
#[test]
fn cfg26_random_int_range_nonderef() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_001A);

    // Variant A: any `w`, `h <= 1` -> outer loop makes zero trips.
    for i in 0..REPS {
        let w = rng.i32_any();
        let h = rng.range_i32(i32::MIN, 1);
        let case = Case::sized(&mut rng, w, h, 64);
        assert_same_and_noop(&c, &r, &case, &format!("cfg26A[{i}] w={w} h={h}"));
    }
    // Variant B: any `w <= 0`, small `h >= 2` -> inner loop makes zero trips.
    for i in 0..REPS {
        let w = rng.range_i32(i32::MIN, 0);
        let h = rng.range_i32(2, 64);
        let case = Case::sized(&mut rng, w, h, 64);
        assert_same_and_noop(&c, &r, &case, &format!("cfg26B[{i}] w={w} h={h}"));
    }
}

/// CONFIGS 27: broad property sweep — 500 randomized `(w in 1..=32,
/// h in 0..=32)` cases over exact-size buffers of random pixels.
#[test]
fn cfg27_property_sweep() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0000_001B);
    for i in 0..500 {
        let w = rng.range_i32(1, 32);
        let h = rng.range_i32(0, 32);
        let case = Case::exact(&mut rng, w, h);
        assert_same(&c, &r, &case, &format!("cfg27[{i}] w={w} h={h}"));
    }
}
