//! High-volume stress sweeps.
//!
//! Phases B and C establish coverage of every configuration and every rejection
//! branch. These tests attack the same surface with far denser input sweeps,
//! hunting for value-dependent divergence (overflow, shift, and division corner
//! cases) that a per-row sample could miss.
//!
//! `diff_stream` compares return values only, because interleaving the two
//! libraries' calls makes their output streams inseparable. Byte-for-byte
//! stdout/stderr equality is asserted by the `diff`-based rows in
//! `phase_b_configs.rs` and `phase_c_errors.rs`.

mod common;

use common::*;

fn apply(lib: &Lib, value: i32, flags: u32) -> i64 {
    let mut storage = flags;
    let r = unsafe { (lib.apply_bit_operations)(value, &mut storage) };
    ((storage as i64) << 32) | (r as u32 as i64)
}

fn perform(lib: &Lib, a: i32, b: i32, flags: u32) -> i64 {
    let mut storage = flags;
    let r = unsafe { (lib.perform_operation)(a, b, &mut storage) };
    ((storage as i64) << 32) | (r as u32 as i64)
}

/// Sweep the whole `i32` range with a large stride, so the entire value space is
/// visited without enumerating all 2^32 points.
fn strided_i32(stride: u32) -> impl Iterator<Item = i32> + Clone {
    (0u64..=(u32::MAX as u64 / stride as u64)).map(move |i| (i as u32).wrapping_mul(stride) as i32)
}

#[test]
fn stress_apply_bit_operations_all_flags_strided_values() {
    let _g = lock();
    env_clear_all();
    // 256 flag bytes x ~4295 strided values.
    let inputs = (0u32..256).flat_map(|byte| strided_i32(1_000_003).map(move |v| (v, byte)));
    let n = diff_stream("stress: apply x 256 flags", inputs, |lib, &(v, f)| {
        apply(lib, v, f)
    });
    assert!(n > 1_000_000, "expected a >1M-input sweep, got {n}");
}

#[test]
fn stress_apply_bit_operations_exhaustive_near_boundaries() {
    let _g = lock();
    env_clear_all();
    // Exhaustive over the 2^20 values adjacent to 0, INT_MIN, INT_MAX, and the
    // sign-flip points of `<< 1`, for the four flag combinations that matter.
    const FLAG_SETS: [(u32, u32, u32); 4] = [(0, 0, 0), (0, 1, 3), (1, 0, 3), (1, 1, 3)];
    const ANCHORS: [i64; 6] = [
        0,
        i32::MIN as i64,
        i32::MAX as i64 - 0x10_0000,
        0x4000_0000 - 0x8_0000,
        -0x4000_0000 - 0x8_0000,
        -0x10_0000,
    ];
    let inputs = FLAG_SETS.into_iter().flat_map(|(verbose, cache, ll)| {
        let f = flags_word(verbose, 0, 0, cache, ll, 0, 0);
        ANCHORS
            .into_iter()
            .flat_map(move |a| (0..0x10_0000i64).map(move |d| ((a + d) as i32, f)))
    });
    let n = diff_stream("stress: apply exhaustive windows", inputs, |lib, &(v, f)| {
        apply(lib, v, f)
    });
    assert_eq!(n, 4 * 6 * 0x10_0000, "sweep size");
}

#[test]
fn stress_perform_operation_dense_grid_all_log_levels() {
    let _g = lock();
    env_clear_all();
    // For every log_level 0..7 and both optimize states, a 2-D grid: one axis
    // strided over the whole i32 range, the other over boundary + random values.
    let mut rng = Rng::new(SEED ^ 0xF00D);
    let bs: Vec<i32> = BOUNDS
        .iter()
        .copied()
        .chain((0..24).map(|_| rng.next_i32()))
        .collect();
    let bs_len = bs.len();
    let inputs = (0u32..2).flat_map(move |opt| {
        let bs = bs.clone();
        (0u32..8).flat_map(move |ll| {
            let f = flags_word(0, 0, opt, 1, ll, 0, 0);
            let bs = bs.clone();
            strided_i32(9_999_991).flat_map(move |a| {
                bs.clone().into_iter().map(move |b| (a, b, f))
            })
        })
    });
    let n = diff_stream("stress: perform dense grid", inputs, |lib, &(a, b, f)| {
        perform(lib, a, b, f)
    });
    assert_eq!(n as usize, 2 * 8 * 430 * bs_len, "sweep size");
}

#[test]
fn stress_perform_operation_exhaustive_small_grid() {
    let _g = lock();
    env_clear_all();
    // Exhaustive over a small signed window on both axes, for all 256 flag
    // bytes: pins the sign/rounding behaviour of `val2 / 2` and the multiply.
    let inputs = (0u32..256).flat_map(|byte| {
        (-24i32..=24).flat_map(move |a| (-24i32..=24).map(move |b| (a, b, byte)))
    });
    let n = diff_stream(
        "stress: perform exhaustive small grid",
        inputs,
        |lib, &(a, b, f)| perform(lib, a, b, f),
    );
    assert_eq!(n, 256 * 49 * 49, "sweep size");
}

#[test]
fn stress_envy_many_random_tuples() {
    let _g = lock();
    let mut total = 0u64;
    for (verbose, debug, opt, off, mult) in [
        (false, false, false, None, None),
        (false, false, true, None, None),
        (false, false, false, Some("0"), Some("0")),
        (false, false, false, Some("-2147483648"), Some("-2147483648")),
        (false, false, true, Some("2147483647"), Some("2147483647")),
        (false, false, false, Some("1"), Some("-1")),
        (false, false, true, Some("-1"), Some("1")),
        // The chatty configurations too — printf output is discarded here but
        // the returned values still must agree on every one of these inputs.
        (true, false, false, None, None),
        (false, true, false, None, None),
        (true, true, true, Some("-5"), Some("-7")),
    ] {
        env_clear_all();
        if verbose {
            env_set("PROG_VERBOSE", "1");
        }
        if debug {
            env_set("PROG_DEBUG", "1");
        }
        if opt {
            env_set("PROG_OPTIMIZE", "1");
        }
        if let Some(v) = off {
            env_set("PROG_BASE_OFFSET", v);
        }
        if let Some(v) = mult {
            env_set("PROG_MULTIPLIER", v);
        }

        let mut rng = Rng::new(
            SEED ^ 0xE7E7
                ^ verbose as u64
                ^ ((debug as u64) << 1)
                ^ ((opt as u64) << 2)
                ^ ((off.map_or(0, |s| s.len() as u64)) << 8),
        );
        let randoms: Vec<(i32, i32, i32, i32)> = (0..40_000)
            .map(|_| {
                (
                    rng.next_i32(),
                    rng.next_i32(),
                    rng.next_i32(),
                    rng.next_i32(),
                )
            })
            .collect();
        // Plus an exhaustive small window, where every guard and the sign of
        // `result` flip.
        let window = (-3i32..=3).flat_map(|a| {
            (-3i32..=3).flat_map(move |b| {
                (-3i32..=3).flat_map(move |c| (-3i32..=3).map(move |d| (a, b, c, d)))
            })
        });

        total += diff_stream(
            "stress: envy random tuples",
            randoms.into_iter().chain(window),
            |lib, &(a, b, c, d)| unsafe { (lib.envy)(a, b, c, d) as i64 },
        );
    }
    env_clear_all();
    assert!(total > 400_000, "expected >400k envy inputs, got {total}");
}

#[test]
fn stress_parse_env_numeric_long_and_odd_values() {
    let _g = lock();
    // Long values (the C `strchr` scans the whole string), values whose rejected
    // character sits at the very end of a long string, and dense digit shapes.
    let mut cases: Vec<String> = Vec::new();
    for len in [1usize, 2, 15, 16, 17, 255, 256, 257, 1023, 4096] {
        cases.push("7".repeat(len));
        cases.push("0".repeat(len));
        cases.push(format!("{},", "9".repeat(len)));
        cases.push(format!("{};", "9".repeat(len)));
        cases.push(format!("{}12", " ".repeat(len)));
        cases.push(format!("-{}", "1".repeat(len)));
        cases.push(format!("{}5", "0".repeat(len)));
    }
    let mut rng = Rng::new(SEED ^ 0xA11);
    for _ in 0..200 {
        let n = (rng.next_u32() % 64) as usize + 1;
        let s: String = (0..n)
            .map(|_| {
                let alphabet = b"0123456789+- \t,;abcxyz.";
                alphabet[(rng.next_u32() as usize) % alphabet.len()] as char
            })
            .collect();
        cases.push(s);
    }
    assert!(cases.len() > 250);

    diff("stress: parse_env_numeric long/odd", move |lib| {
        let mut out = Vec::new();
        let name = std::ffi::CString::new("PROG_BASE_OFFSET").unwrap();
        for s in &cases {
            env_clear_all();
            env_set("PROG_BASE_OFFSET", s);
            for d in [0i32, -1, 64, i32::MIN, i32::MAX] {
                out.push(unsafe { (lib.parse_env_numeric)(name.as_ptr(), d) as i64 });
            }
        }
        env_clear_all();
        out
    });
}

#[test]
fn stress_interleaved_entry_points_are_stateless() {
    let _g = lock();
    // Neither library keeps state between calls; interleaving the five entry
    // points in a pseudo-random order, with the environment mutating in
    // between, must not change any answer.
    diff("stress: interleaved entry points", |lib| {
        let mut out = Vec::new();
        let mut rng = Rng::new(SEED ^ 0x1234_5678);
        let names = [
            "PROG_BASE_OFFSET",
            "PROG_MULTIPLIER",
            "PROG_OPTIMIZE",
            "NO_SUCH_VAR_QQQ",
        ];
        let values = ["1", "0", "", "5,", "9;", "-12", "2147483647"];
        env_clear_all();
        for _ in 0..4000 {
            match rng.next_u32() % 5 {
                0 => {
                    let n = std::ffi::CString::new(names[(rng.next_u32() % 4) as usize]).unwrap();
                    out.push(unsafe { (lib.parse_env_numeric)(n.as_ptr(), rng.next_i32()) } as i64);
                }
                1 => {
                    let mut s = rng.next_u32();
                    unsafe { (lib.init_config_from_env)(&mut s) };
                    out.push(s as i64);
                }
                2 => {
                    let mut s = rng.next_u32() & !0x2; // keep debug clear
                    out.push(unsafe {
                        (lib.perform_operation)(rng.next_i32(), rng.next_i32(), &mut s)
                    } as i64);
                    out.push(s as i64);
                }
                3 => {
                    let mut s = rng.next_u32();
                    out.push(unsafe { (lib.apply_bit_operations)(rng.next_i32(), &mut s) } as i64);
                    out.push(s as i64);
                }
                _ => {
                    let k = names[(rng.next_u32() % 4) as usize];
                    if rng.next_u32() % 3 == 0 {
                        env_unset(k);
                    } else {
                        env_set(k, values[(rng.next_u32() % 7) as usize]);
                    }
                    out.push(unsafe {
                        (lib.envy)(
                            rng.next_i32(),
                            rng.next_i32(),
                            rng.next_i32(),
                            rng.next_i32(),
                        )
                    } as i64);
                }
            }
        }
        env_clear_all();
        out
    });
}
