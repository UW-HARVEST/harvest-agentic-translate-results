// Phase B -- valid-path differential tests, one test per CONFIGS.md row.
// Both libraries are driven exclusively through their `.so` exports.

mod common;
use common::*;
use std::os::raw::c_int;

// ---------------------------------------------------------------- row 1
#[test]
fn cfg_01_classify_mode_exact_literals() {
    let l = libs();
    for m in MODES {
        let buf = cstring(m);
        eq_i32(
            format!("classify_mode({m:?})"),
            l.c.classify_mode_bytes(&buf),
            l.rust.classify_mode_bytes(&buf),
        );
    }
}

// ---------------------------------------------------------------- row 2
#[test]
fn cfg_02_classify_mode_random_strings() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..4096 {
        let len = rng.below(17) as usize;
        let mut buf: Vec<u8> = (0..len).map(|_| (rng.below(255) as u8) + 1).collect();
        buf.push(0);
        eq_i32(
            format!("classify_mode({:?})", &buf[..buf.len() - 1]),
            l.c.classify_mode_bytes(&buf),
            l.rust.classify_mode_bytes(&buf),
        );
    }
}

// ---------------------------------------------------------------- row 3
#[test]
fn cfg_03_classify_mode_near_misses() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..4096 {
        let base = MODES[rng.below(4) as usize];
        let mut bytes = base.as_bytes().to_vec();
        match rng.below(4) {
            0 => {
                // mutate one character
                let i = rng.below(bytes.len() as u64) as usize;
                bytes[i] = b'a' + (rng.below(26) as u8);
            }
            1 => {
                // truncate
                let i = rng.below(bytes.len() as u64) as usize;
                bytes.truncate(i);
            }
            2 => bytes.push(b'a' + (rng.below(26) as u8)),
            _ => {
                // case flip
                let i = rng.below(bytes.len() as u64) as usize;
                bytes[i] = bytes[i].to_ascii_uppercase();
            }
        }
        bytes.push(0);
        eq_i32(
            format!("classify_mode near-miss {:?}", &bytes[..bytes.len() - 1]),
            l.c.classify_mode_bytes(&bytes),
            l.rust.classify_mode_bytes(&bytes),
        );
    }
}

// ---------------------------------------------------------------- row 4
#[test]
fn cfg_04_apply_multiplier_levels_base_zero() {
    let l = libs();
    for level in 0..=4 {
        eq_i32(
            format!("apply_multiplier(0, {level})"),
            l.c.apply_multiplier(0, level),
            l.rust.apply_multiplier(0, level),
        );
    }
}

// ---------------------------------------------------------------- row 5
#[test]
fn cfg_05_apply_multiplier_levels_random_base() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..4096 {
        let base = rng.next_i32();
        for level in 0..=4 {
            eq_i32(
                format!("apply_multiplier({base}, {level})"),
                l.c.apply_multiplier(base, level),
                l.rust.apply_multiplier(base, level),
            );
        }
    }
}

// ---------------------------------------------------------------- row 6
#[test]
fn cfg_06_apply_multiplier_boundary_bases() {
    let l = libs();
    let bases = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        0xA0,
        i32::MAX - 1,
        i32::MAX,
        i32::MAX - 0x1C,
        i32::MAX - 0x24D,
    ];
    for &base in &bases {
        for level in -2..=6 {
            eq_i32(
                format!("apply_multiplier({base}, {level})"),
                l.c.apply_multiplier(base, level),
                l.rust.apply_multiplier(base, level),
            );
        }
    }
}

// ---------------------------------------------------------------- row 7
#[test]
fn cfg_07_apply_multiplier_random_pairs() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..4096 {
        let base = rng.next_i32();
        let level = rng.next_i32();
        eq_i32(
            format!("apply_multiplier({base}, {level})"),
            l.c.apply_multiplier(base, level),
            l.rust.apply_multiplier(base, level),
        );
    }
}

// ---------------------------------------------------------------- row 8
#[test]
fn cfg_08_convert_time_factor_in_range() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..4096 {
        // |factor| * 1e12 < 2^31  =>  |factor| < 2.147e-3
        let mantissa = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        let sign = if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        let factor = sign * mantissa * 2.1474e-3;
        let (a, b) = (l.c.convert_time_factor(factor), l.rust.convert_time_factor(factor));
        eq_i32(format!("convert_time_factor({factor:e})"), a, b);
        assert_ne!(a, i32::MIN, "row 8 should stay in range for {factor:e}");
    }
}

// ---------------------------------------------------------------- row 9
#[test]
fn cfg_09_convert_time_factor_random_magnitudes() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..8192 {
        let f = rng.next_f64_magnitudes(60);
        eq_i32(
            format!("convert_time_factor({f:e})"),
            l.c.convert_time_factor(f),
            l.rust.convert_time_factor(f),
        );
    }
}

// ---------------------------------------------------------------- row 10
#[test]
fn cfg_10_convert_time_factor_random_bits() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..8192 {
        let f = rng.next_f64_bits();
        eq_i32(
            format!("convert_time_factor(bits 0x{:016X})", f.to_bits()),
            l.c.convert_time_factor(f),
            l.rust.convert_time_factor(f),
        );
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn cfg_11_convert_negative_overflow_in_range() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..4096 {
        // |value| * 1e15 < 2^31  =>  |value| < 2.147e-6
        let mantissa = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        let sign = if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        let v = sign * mantissa * 2.1474e-6;
        let (a, b) = (
            l.c.convert_negative_overflow(v),
            l.rust.convert_negative_overflow(v),
        );
        eq_i32(format!("convert_negative_overflow({v:e})"), a, b);
        assert_ne!(a, i32::MIN, "row 11 should stay in range for {v:e}");
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn cfg_12_convert_negative_overflow_random_magnitudes() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..8192 {
        let v = rng.next_f64_magnitudes(60);
        eq_i32(
            format!("convert_negative_overflow({v:e})"),
            l.c.convert_negative_overflow(v),
            l.rust.convert_negative_overflow(v),
        );
    }
}

// ---------------------------------------------------------------- row 13
#[test]
fn cfg_13_convert_negative_overflow_random_bits() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..8192 {
        let v = rng.next_f64_bits();
        eq_i32(
            format!("convert_negative_overflow(bits 0x{:016X})", v.to_bits()),
            l.c.convert_negative_overflow(v),
            l.rust.convert_negative_overflow(v),
        );
    }
}

// ---------------------------------------------------------------- row 14
#[test]
fn cfg_14_get_modified_time_small_offsets() {
    let l = libs();
    for days in [-1000i32, -7, -1, 0, 1, 7, 1000] {
        for hours in [-100i32, -23, -1, 0, 1, 23, 100] {
            eq_i64(
                format!("get_modified_time({days}, {hours})"),
                l.c.get_modified_time(days, hours),
                l.rust.get_modified_time(days, hours),
            );
        }
    }
}

// ---------------------------------------------------------------- row 15
#[test]
fn cfg_15_get_modified_time_random_offsets() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..4096 {
        let d = rng.next_i32();
        let h = rng.next_i32();
        eq_i64(
            format!("get_modified_time({d}, {h})"),
            l.c.get_modified_time(d, h),
            l.rust.get_modified_time(d, h),
        );
    }
}

// ---------------------------------------------------------------- row 16
#[test]
fn cfg_16_get_modified_time_boundary_offsets() {
    let l = libs();
    let days = [i32::MIN, i32::MIN + 1, -24856, -24855, -1, 0, 1, 24855, 24856, i32::MAX];
    let hours = [i32::MIN, i32::MIN + 1, -596523, -1, 0, 1, 23, 596523, i32::MAX];
    for &d in &days {
        for &h in &hours {
            eq_i64(
                format!("get_modified_time({d}, {h})"),
                l.c.get_modified_time(d, h),
                l.rust.get_modified_time(d, h),
            );
        }
    }
}

// ---------------------------------------------------------------- row 17
#[test]
fn cfg_17_hash_time_value_boundaries() {
    let l = libs();
    let now_shifted = l.c.get_modified_time(0, 0);
    let vals = [
        0i64,
        1,
        -1,
        i64::MIN,
        i64::MIN + 1,
        i64::MAX,
        i64::MAX - 1,
        0xFFFF_FFFF,
        0x1_0000_0000,
        -0x1_0000_0000,
        0x0101_0101_0101_0101u64 as i64,
        0x7F7F_7F7F_7F7F_7F7Fu64 as i64,
        now_shifted,
    ];
    for &t in &vals {
        eq_i32(
            format!("hash_time_value({t})"),
            l.c.hash_time_value(t),
            l.rust.hash_time_value(t),
        );
    }
}

// ---------------------------------------------------------------- row 18
#[test]
fn cfg_18_hash_time_value_random() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..8192 {
        let t = rng.next_i64();
        eq_i32(
            format!("hash_time_value({t})"),
            l.c.hash_time_value(t),
            l.rust.hash_time_value(t),
        );
    }
}

// ---------------------------------------------------------------- row 19
#[test]
fn cfg_19_hash_time_value_single_byte_lanes() {
    let l = libs();
    for lane in 0..8u32 {
        for b in 0..=255u64 {
            let t = ((b << (lane * 8)) as u64) as i64;
            eq_i32(
                format!("hash_time_value(lane {lane}, byte {b}) = {t}"),
                l.c.hash_time_value(t),
                l.rust.hash_time_value(t),
            );
        }
    }
}

// ---------------------------------------------------------------- row 20
#[test]
fn cfg_20_modeselect_mode_x_complexity() {
    let l = libs();
    for ms in 0..4 {
        for cx in 0..5 {
            eq_i32(
                format!("modeselect({ms}, 0, {cx}, 0)"),
                l.c.modeselect(ms, 0, cx, 0),
                l.rust.modeselect(ms, 0, cx, 0),
            );
        }
    }
}

// ---------------------------------------------------------------- row 21
#[test]
fn cfg_21_modeselect_mode_complexity_seed_offset() {
    let l = libs();
    for ms in 0..4 {
        for cx in 0..5 {
            for seed in [0, 1, 23, 24, 25] {
                for off in [0, 1, -1] {
                    eq_i32(
                        format!("modeselect({ms}, {off}, {cx}, {seed})"),
                        l.c.modeselect(ms, off, cx, seed),
                        l.rust.modeselect(ms, off, cx, seed),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------- row 22
#[test]
fn cfg_22_modeselect_random_full_range() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..2048 {
        // mode_selector >= 0 keeps `mode_selector % 4` an in-bounds index; a
        // negative selector is UB in the C (see ERRORS.md row 22).
        let ms = (rng.next_i32() & i32::MAX) as c_int;
        let off = rng.next_i32();
        let cx = rng.next_i32();
        let seed = rng.next_i32();
        eq_i32(
            format!("modeselect({ms}, {off}, {cx}, {seed})"),
            l.c.modeselect(ms, off, cx, seed),
            l.rust.modeselect(ms, off, cx, seed),
        );
    }
}

// ---------------------------------------------------------------- row 23
#[test]
fn cfg_23_modeselect_seed_in_range_conversion() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 23);
    // seed*1e8 * 1e12 must fit in int  =>  |seed| < 2.147e31/1e20 -> only 0
    // fits for the *product*; so instead exercise the neighbourhood of 0 where
    // result1 is not the overflow sentinel, plus small magnitudes.
    for seed in -3..=3 {
        for _ in 0..64 {
            let ms = (rng.next_i32() & i32::MAX) as c_int;
            let off = rng.next_i32() % 1000;
            let cx = rng.next_i32();
            eq_i32(
                format!("modeselect({ms}, {off}, {cx}, {seed})"),
                l.c.modeselect(ms, off, cx, seed),
                l.rust.modeselect(ms, off, cx, seed),
            );
        }
    }
    // Confirm that seed == 0 really produces the non-sentinel conversion result.
    assert_eq!(l.c.convert_time_factor(0.0), 0);
    assert_eq!(l.rust.convert_time_factor(0.0), 0);
}

// ---------------------------------------------------------------- row 24
#[test]
fn cfg_24_modeselect_time_offset_in_range_conversion() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 24);
    for off in -3..=3 {
        for _ in 0..64 {
            let ms = (rng.next_i32() & i32::MAX) as c_int;
            let cx = rng.next_i32();
            let seed = rng.next_i32();
            eq_i32(
                format!("modeselect({ms}, {off}, {cx}, {seed})"),
                l.c.modeselect(ms, off, cx, seed),
                l.rust.modeselect(ms, off, cx, seed),
            );
        }
    }
    assert_eq!(l.c.convert_negative_overflow(-0.0), 0);
    assert_eq!(l.rust.convert_negative_overflow(-0.0), 0);
}

// ---------------------------------------------------------------- row 25
#[test]
fn cfg_25_modeselect_boundary_tuples() {
    let l = libs();
    let vals = [0i32, 1, 3, 4, 5, 23, 24, 25, i32::MAX, i32::MAX - 1];
    for &ms in &vals {
        for &off in &vals {
            for &cx in &vals {
                for &seed in &vals {
                    eq_i32(
                        format!("modeselect({ms}, {off}, {cx}, {seed})"),
                        l.c.modeselect(ms, off, cx, seed),
                        l.rust.modeselect(ms, off, cx, seed),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------- row 27
#[test]
fn cfg_27_pipeline_get_modified_time_into_hash() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..4096 {
        let days = rng.next_i32();
        let hours = rng.next_i32() % 24;
        // Compose the two low-level entry points exactly as modeselect does,
        // crossing the FFI boundary at every step.
        let tc = l.c.get_modified_time(days, hours);
        let tr = l.rust.get_modified_time(days, hours);
        eq_i64(format!("pipeline get_modified_time({days}, {hours})"), tc, tr);

        // Hash the C value with both libs and the Rust value with both libs.
        eq_i32(
            format!("pipeline hash(C time {tc})"),
            l.c.hash_time_value(tc),
            l.rust.hash_time_value(tc),
        );
        eq_i32(
            format!("pipeline hash(Rust time {tr})"),
            l.c.hash_time_value(tr),
            l.rust.hash_time_value(tr),
        );
        // And the cross-composition C->Rust vs Rust->C.
        eq_i32(
            format!("pipeline cross ({days}, {hours})"),
            l.rust.hash_time_value(tc),
            l.c.hash_time_value(tr),
        );
    }
}

// ---------------------------------------------------------------- row 28
#[test]
fn cfg_28_pipeline_classify_into_modeselect_arithmetic() {
    let l = libs();
    // Reproduce modeselect's own fold using the low-level exports and check the
    // two libraries agree on every intermediate as well as the final value.
    for ms in 0..8 {
        let buf = cstring(MODES[(ms % 4) as usize]);
        let mvc = l.c.classify_mode_bytes(&buf);
        let mvr = l.rust.classify_mode_bytes(&buf);
        eq_i32(format!("classify_mode via modes[{}]", ms % 4), mvc, mvr);

        for cx in 0..7 {
            let level = cx % 5;
            eq_i32(
                format!("apply_multiplier(0xA0, {level})"),
                l.c.apply_multiplier(0xA0, level),
                l.rust.apply_multiplier(0xA0, level),
            );
            eq_i32(
                format!("modeselect({ms}, 0, {cx}, 0) full pipeline"),
                l.c.modeselect(ms, 0, cx, 0),
                l.rust.modeselect(ms, 0, cx, 0),
            );
        }
    }
}
