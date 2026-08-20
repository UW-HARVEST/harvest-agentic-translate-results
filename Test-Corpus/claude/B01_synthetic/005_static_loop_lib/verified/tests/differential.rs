//! Differential tests: C `.so` vs Rust `.so`, both loaded through `libloading`.
//!
//! * module `configs` — one test per row of `CONFIGS.md` (valid paths, Phase B)
//! * module `errors`  — one test per row of `ERRORS.md`  (boundary/rejection
//!                      paths, Phase C)
//! * module `symbols` — `nm -D` parity between the two libraries (Phase D)
//!
//! Every test drives BOTH libraries via their exported C symbols and asserts the
//! results are identical. The Rust implementation is never called directly.

mod common;

use common::{expected_driver_output, harness, lines_of, Rng};

/// Every `±2^k` for k in 0..=31, plus the arithmetic extremes.
fn power_of_two_values() -> Vec<i32> {
    let mut v = Vec::new();
    for k in 0..31 {
        let p = 1i32 << k;
        v.push(p);
        v.push(-p);
    }
    v.push(i32::MIN); // -2^31
    v.push(i32::MAX);
    v.push(i32::MIN + 1);
    v.push(i32::MAX - 1);
    v.push(0);
    v
}

/// Largest `stride` for which `i * stride` never overflows in `driver`
/// (`i` peaks at 9): `INT_MAX / 9`.
const SAFE_STRIDE_MAX: i32 = i32::MAX / 9; // 238_609_294

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================
mod configs {
    use super::*;

    /// C1 — `static_sum` with `update = 0` (degenerate zero update), repeated.
    #[test]
    fn cfg_c1_static_sum_zero_update() {
        let mut h = harness();
        let mut rng = Rng::new(0xC001);
        for _ in 0..50 {
            let before = h.sum(0);
            let after = h.sum(0);
            assert_eq!(before, after, "static_sum(0) must not change the total");
        }
        // Same from an arbitrary non-zero state.
        for _ in 0..50 {
            h.set_state(rng.i32_any());
            let a = h.sum(0);
            let b = h.sum(0);
            assert_eq!(a, b);
        }
    }

    /// C2 — small positive updates.
    #[test]
    fn cfg_c2_static_sum_small_positive() {
        let mut h = harness();
        let mut rng = Rng::new(0xC002);
        h.sum(1);
        for _ in 0..500 {
            let u = rng.range(1, 1000);
            h.sum(u);
        }
    }

    /// C3 — small negative updates.
    #[test]
    fn cfg_c3_static_sum_small_negative() {
        let mut h = harness();
        let mut rng = Rng::new(0xC003);
        h.sum(-1);
        for _ in 0..500 {
            let u = rng.range(-1000, -1);
            h.sum(u);
        }
    }

    /// C4 — mixed-sign small updates (state walks around 0, sign flips).
    #[test]
    fn cfg_c4_static_sum_mixed_small() {
        let mut h = harness();
        let mut rng = Rng::new(0xC004);
        h.set_state(0);
        for _ in 0..1000 {
            let u = rng.range(-1000, 1000);
            h.sum(u);
        }
    }

    /// C5 — full-range random updates (wrap-heavy).
    #[test]
    fn cfg_c5_static_sum_full_range_random() {
        let mut h = harness();
        let mut rng = Rng::new(0xC005);
        for _ in 0..2000 {
            let u = rng.i32_any();
            h.sum(u);
        }
    }

    /// C6 — boundary constants as `update`.
    #[test]
    fn cfg_c6_static_sum_boundary_constants() {
        let mut h = harness();
        let consts = power_of_two_values();
        for &u in &consts {
            h.sum(u);
        }
        // ...and again from a freshly-zeroed state, one constant at a time, so
        // each constant is also exercised in isolation.
        for &u in &consts {
            h.set_state(0);
            let got = h.sum(u);
            assert_eq!(got, u, "0 + {u} must be {u}");
        }
    }

    /// C7 — accumulator driven to exactly INT_MAX / INT_MIN, then stepped past.
    #[test]
    fn cfg_c7_static_sum_exact_wrap_points() {
        let mut h = harness();

        h.set_state(i32::MAX);
        let got = h.sum(1);
        assert_eq!(got, i32::MIN, "INT_MAX + 1 must wrap to INT_MIN");

        h.set_state(i32::MIN);
        let got = h.sum(-1);
        assert_eq!(got, i32::MAX, "INT_MIN - 1 must wrap to INT_MAX");

        h.set_state(i32::MAX);
        assert_eq!(h.sum(i32::MAX), -2);

        h.set_state(i32::MIN);
        assert_eq!(h.sum(i32::MIN), 0);

        h.set_state(-1);
        assert_eq!(h.sum(i32::MIN), i32::MAX);
    }

    /// C8 — call-count shape: 1 / 2 / many identical calls.
    #[test]
    fn cfg_c8_static_sum_call_count_shapes() {
        let mut h = harness();
        let mut rng = Rng::new(0xC008);

        // exactly one call
        h.set_state(0);
        let u = rng.range(-100_000, 100_000);
        assert_eq!(h.sum(u), u);

        // exactly two calls
        h.set_state(0);
        h.sum(u);
        assert_eq!(h.sum(u), u.wrapping_mul(2));

        // many calls with the same argument (accumulates, eventually wraps)
        for n in [3usize, 10, 100, 1000] {
            h.set_state(0);
            let step = rng.i32_any();
            let mut last = 0i32;
            for _ in 0..n {
                last = h.sum(step);
            }
            assert_eq!(last, step.wrapping_mul(n as i32));
        }
    }

    /// C9 — `driver(0)`: prints the current total ten times.
    #[test]
    fn cfg_c9_driver_stride_zero() {
        let mut h = harness();
        let mut rng = Rng::new(0xC009);

        for _ in 0..25 {
            let s = rng.i32_any();
            h.set_state(s);
            let out = h.driver(0);
            assert_eq!(out, expected_driver_output(s, 0));
            let ls = lines_of(&out);
            assert_eq!(ls.len(), 10);
            assert!(ls.iter().all(|l| l == &s.to_string()));
            assert_eq!(h.state(), s, "driver(0) must not change the total");
        }
    }

    /// C10 — canonical `driver(1)` from a zeroed accumulator.
    #[test]
    fn cfg_c10_driver_stride_one_fresh() {
        let mut h = harness();
        h.set_state(0);
        let out = h.driver(1);
        assert_eq!(
            out, b"0\n1\n3\n6\n10\n15\n21\n28\n36\n45\n",
            "canonical driver(1) output"
        );
        assert_eq!(h.state(), 45);
    }

    /// C11 — `driver(-1)`: output contains minus signs.
    #[test]
    fn cfg_c11_driver_stride_negative_one() {
        let mut h = harness();
        h.set_state(0);
        let out = h.driver(-1);
        assert_eq!(out, b"0\n-1\n-3\n-6\n-10\n-15\n-21\n-28\n-36\n-45\n");
        assert_eq!(h.state(), -45);
    }

    /// C12 — small random strides.
    #[test]
    fn cfg_c12_driver_small_random_strides() {
        let mut h = harness();
        let mut rng = Rng::new(0xC012);
        for _ in 0..200 {
            let stride = rng.range(-1000, 1000);
            let before = h.state();
            let out = h.driver(stride);
            assert_eq!(out, expected_driver_output(before, stride));
        }
    }

    /// C13 — small random strides on top of a deliberately pre-set state.
    #[test]
    fn cfg_c13_driver_with_preset_state() {
        let mut h = harness();
        let mut rng = Rng::new(0xC013);
        for _ in 0..200 {
            let start = if rng.bool() {
                rng.i32_any()
            } else {
                rng.range(-5, 5)
            };
            h.set_state(start);
            let stride = rng.range(-1000, 1000);
            let out = h.driver(stride);
            assert_eq!(out, expected_driver_output(start, stride));
        }
    }

    /// C14 — large positive strides so `i * stride` overflows inside the loop.
    #[test]
    fn cfg_c14_driver_large_positive_stride_overflow() {
        let mut h = harness();
        let mut rng = Rng::new(0xC014);
        for _ in 0..200 {
            let stride = rng.range(SAFE_STRIDE_MAX + 2, i32::MAX);
            let before = h.state();
            let out = h.driver(stride);
            assert_eq!(out, expected_driver_output(before, stride));
        }
    }

    /// C15 — large negative strides so `i * stride` overflows inside the loop.
    #[test]
    fn cfg_c15_driver_large_negative_stride_overflow() {
        let mut h = harness();
        let mut rng = Rng::new(0xC015);
        for _ in 0..200 {
            let stride = rng.range(i32::MIN, -(SAFE_STRIDE_MAX + 2));
            let before = h.state();
            let out = h.driver(stride);
            assert_eq!(out, expected_driver_output(before, stride));
        }
    }

    /// C16 — boundary constants as `stride`.
    #[test]
    fn cfg_c16_driver_boundary_constants() {
        let mut h = harness();
        for &stride in &power_of_two_values() {
            let before = h.state();
            let out = h.driver(stride);
            assert_eq!(
                out,
                expected_driver_output(before, stride),
                "driver({stride}) from state {before}"
            );
            // ...and from a zeroed accumulator.
            h.set_state(0);
            let out0 = h.driver(stride);
            assert_eq!(out0, expected_driver_output(0, stride));
        }
    }

    /// C17 — the `i * stride` multiplication boundary (`INT_MAX / 9`).
    #[test]
    fn cfg_c17_driver_multiply_boundary() {
        let mut h = harness();
        let interesting = [
            SAFE_STRIDE_MAX - 1,
            SAFE_STRIDE_MAX,     // largest overflow-free stride
            SAFE_STRIDE_MAX + 1, // one step past: overflows at i == 9
            SAFE_STRIDE_MAX + 2,
            -(SAFE_STRIDE_MAX - 1),
            -SAFE_STRIDE_MAX,
            -(SAFE_STRIDE_MAX + 1),
            -(SAFE_STRIDE_MAX + 2),
            i32::MAX / 8,
            i32::MAX / 8 + 1,
            i32::MIN / 9,
            i32::MIN / 9 - 1,
        ];
        for &stride in &interesting {
            h.set_state(0);
            let out = h.driver(stride);
            assert_eq!(
                out,
                expected_driver_output(0, stride),
                "driver({stride}) at the multiply boundary"
            );
            // also from a random-ish non-zero state
            h.set_state(0x1234_5678);
            let out2 = h.driver(stride);
            assert_eq!(out2, expected_driver_output(0x1234_5678, stride));
        }
    }

    /// C18 — `driver` called many times in a row (state accumulates).
    #[test]
    fn cfg_c18_driver_repeated_calls() {
        let mut h = harness();
        let mut rng = Rng::new(0xC018);
        h.set_state(0);
        let mut model = 0i32;
        for _ in 0..100 {
            let stride = if rng.bool() {
                rng.range(-50, 50)
            } else {
                rng.i32_any()
            };
            let out = h.driver(stride);
            assert_eq!(out, expected_driver_output(model, stride));
            model = model.wrapping_add(stride.wrapping_mul(45));
            assert_eq!(h.state(), model);
        }
    }

    /// C19 — randomly interleaved `static_sum` / `driver` operations.
    #[test]
    fn cfg_c19_interleaved_random_operations() {
        let mut h = harness();
        let mut rng = Rng::new(0xC019);
        h.set_state(0);
        let mut model = 0i32;
        for _ in 0..1500 {
            if rng.next_u64() % 3 == 0 {
                let stride = match rng.next_u64() % 3 {
                    0 => rng.range(-10, 10),
                    1 => rng.i32_any(),
                    _ => 0,
                };
                let out = h.driver(stride);
                assert_eq!(out, expected_driver_output(model, stride));
                model = model.wrapping_add(stride.wrapping_mul(45));
            } else {
                let u = match rng.next_u64() % 3 {
                    0 => rng.range(-10, 10),
                    1 => rng.i32_any(),
                    _ => i32::MAX,
                };
                let got = h.sum(u);
                model = model.wrapping_add(u);
                assert_eq!(got, model);
            }
        }
    }

    /// C20 — printed values span every digit width plus both signs.
    #[test]
    fn cfg_c20_driver_all_digit_widths() {
        let mut h = harness();
        let mut widths: Vec<i32> = vec![0, 7, 42, 999, 1234, 54321, 987_654, 1_234_567];
        widths.push(12_345_678);
        widths.push(123_456_789);
        widths.push(1_234_567_890);
        widths.push(i32::MAX);
        let negatives: Vec<i32> = widths.iter().map(|v| v.wrapping_neg()).collect();
        widths.extend(negatives);
        widths.push(i32::MIN);

        for &v in &widths {
            h.set_state(v);
            let out = h.driver(0);
            assert_eq!(out, expected_driver_output(v, 0));
            let ls = lines_of(&out);
            assert_eq!(ls.len(), 10);
            assert_eq!(ls[0], v.to_string(), "printf(\"%d\") formatting of {v}");
        }
    }

    /// C21 — state written via `static_sum` is observable through `driver`.
    #[test]
    fn cfg_c21_state_visible_across_entry_points() {
        let mut h = harness();
        let mut rng = Rng::new(0xC021);
        for _ in 0..100 {
            let x = rng.i32_any();
            h.set_state(0);
            let got = h.sum(x);
            assert_eq!(got, x);
            let out = h.driver(0);
            let ls = lines_of(&out);
            assert_eq!(ls.len(), 10);
            assert!(
                ls.iter().all(|l| l == &x.to_string()),
                "driver(0) must print the state {x} ten times, got {ls:?}"
            );
        }
    }

    /// C22 — the accumulator wraps part-way through `driver`'s ten iterations.
    #[test]
    fn cfg_c22_driver_wraps_mid_loop() {
        let mut h = harness();
        let mut rng = Rng::new(0xC022);

        // Deterministic: start just below INT_MAX with a small stride so the
        // wrap happens at a known iteration.
        for start in [i32::MAX - 1, i32::MAX - 10, i32::MAX - 44, i32::MAX - 45] {
            for stride in [1, 2, 3, 7] {
                h.set_state(start);
                let out = h.driver(stride);
                assert_eq!(
                    out,
                    expected_driver_output(start, stride),
                    "mid-loop wrap: start={start} stride={stride}"
                );
            }
        }
        // ...and the negative end.
        for start in [i32::MIN + 1, i32::MIN + 10, i32::MIN + 45] {
            for stride in [-1, -2, -3, -7] {
                h.set_state(start);
                let out = h.driver(stride);
                assert_eq!(out, expected_driver_output(start, stride));
            }
        }
        // Randomized mid-loop wraps.
        for _ in 0..100 {
            let start = i32::MAX - rng.range(0, 500);
            let stride = rng.range(1, 200);
            h.set_state(start);
            let out = h.driver(stride);
            assert_eq!(out, expected_driver_output(start, stride));
        }
    }

    /// C23 — long mixed fuzz over both entry points and every value generator.
    #[test]
    fn cfg_c23_long_mixed_fuzz() {
        let mut h = harness();
        let mut rng = Rng::new(0xC023_5EED);
        h.set_state(0);
        let mut model = 0i32;

        let pick = |rng: &mut Rng| -> i32 {
            match rng.next_u64() % 7 {
                0 => 0,
                1 => rng.range(-3, 3),
                2 => rng.range(-1000, 1000),
                3 => rng.i32_any(),
                4 => i32::MAX,
                5 => i32::MIN,
                _ => 1i32 << (rng.next_u64() % 31) as u32,
            }
        };

        for _ in 0..5000 {
            if rng.bool() {
                let u = pick(&mut rng);
                let got = h.sum(u);
                model = model.wrapping_add(u);
                assert_eq!(got, model);
            } else {
                let stride = pick(&mut rng);
                let out = h.driver(stride);
                assert_eq!(out, expected_driver_output(model, stride));
                model = model.wrapping_add(stride.wrapping_mul(45));
            }
        }
    }

    /// C24 — `%d` output of exactly INT_MIN / INT_MAX, byte-exact.
    #[test]
    fn cfg_c24_driver_prints_int_extremes() {
        let mut h = harness();

        h.set_state(i32::MIN);
        let out = h.driver(0);
        assert_eq!(out, "-2147483648\n".repeat(10).into_bytes());

        h.set_state(i32::MAX);
        let out = h.driver(0);
        assert_eq!(out, "2147483647\n".repeat(10).into_bytes());

        // Reached by wrapping rather than by being set directly.
        h.set_state(i32::MAX);
        let out = h.driver(1);
        assert_eq!(out, expected_driver_output(i32::MAX, 1));
        assert!(String::from_utf8_lossy(&out).contains("-214748"));
    }

    /// C25 — `driver` always prints exactly ten lines and its net effect on the
    /// accumulator is `45 * stride` (wrapping).
    #[test]
    fn cfg_c25_driver_line_count_and_net_effect() {
        let mut h = harness();
        let mut rng = Rng::new(0xC025);
        for _ in 0..250 {
            let start = rng.i32_any();
            let stride = match rng.next_u64() % 4 {
                0 => 0,
                1 => rng.range(-100, 100),
                2 => rng.i32_any(),
                _ => rng.range(SAFE_STRIDE_MAX, i32::MAX),
            };
            h.set_state(start);
            let out = h.driver(stride);
            assert_eq!(
                out.iter().filter(|&&b| b == b'\n').count(),
                10,
                "driver must always emit exactly 10 lines"
            );
            assert_eq!(lines_of(&out).len(), 10);
            assert_eq!(
                h.state(),
                start.wrapping_add(stride.wrapping_mul(45)),
                "net effect of driver({stride}) from {start}"
            );
        }
    }

    /// C26 — a genuinely freshly `dlopen`'d instance of each library: the
    /// hidden accumulator must start at 0 in both (`static int sum = 0;` vs.
    /// `static mut SUM: c_int = 0;`), and stay in lockstep from there.
    #[test]
    fn cfg_c26_fresh_library_instances_start_at_zero() {
        use common::{c_so_path, capture_stdout, load_fresh_copy, sym, DriverFn, SumFn};

        // Serialise against the shared harness: fd 1 is process-global.
        let _guard = harness();
        let mut rng = Rng::new(0xC026);

        for round in 0..8 {
            let (c_lib, c_tmp) = load_fresh_copy(&c_so_path(), "c");
            let (r_lib, r_tmp) = load_fresh_copy(&common::rust_so_path(), "r");
            let c_sum: SumFn = sym(&c_lib, b"static_sum\0");
            let r_sum: SumFn = sym(&r_lib, b"static_sum\0");
            let c_drv: DriverFn = sym(&c_lib, b"driver\0");
            let r_drv: DriverFn = sym(&r_lib, b"driver\0");

            // Initial state must be exactly 0 in both.
            let c0 = unsafe { c_sum(0) };
            let r0 = unsafe { r_sum(0) };
            assert_eq!(c0, 0, "fresh C instance must start at 0 (round {round})");
            assert_eq!(r0, 0, "fresh Rust instance must start at 0 (round {round})");

            // First real update: both must return exactly that update.
            let u = rng.i32_any();
            let c1 = unsafe { c_sum(u) };
            let r1 = unsafe { r_sum(u) };
            assert_eq!(c1, u);
            assert_eq!(c1, r1);

            // driver() on the fresh pair, from the same state.
            let stride = if rng.bool() { rng.range(-9, 9) } else { rng.i32_any() };
            let c_out = capture_stdout(|| unsafe { c_drv(stride) });
            let r_out = capture_stdout(|| unsafe { r_drv(stride) });
            assert_eq!(
                c_out,
                r_out,
                "fresh-instance driver({stride}) diverged:\n C   = {:?}\n Rust= {:?}",
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out)
            );
            assert_eq!(c_out, expected_driver_output(u, stride));

            // A short paired sequence on the fresh pair.
            for _ in 0..50 {
                let v = rng.i32_any();
                let a = unsafe { c_sum(v) };
                let b = unsafe { r_sum(v) };
                assert_eq!(a, b, "fresh-instance static_sum({v}) diverged");
            }

            drop(c_lib);
            drop(r_lib);
            let _ = std::fs::remove_file(&c_tmp);
            let _ = std::fs::remove_file(&r_tmp);
        }
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows
// ===========================================================================
mod errors {
    use super::*;

    /// E1 — there is no rejection path: any `int` is accepted and returns the
    /// new running total (no sentinel, no errno).
    #[test]
    fn err_e1_no_rejection_path_any_int() {
        let mut h = harness();
        let mut rng = Rng::new(0xE001);
        for _ in 0..1000 {
            let u = rng.i32_any();
            let before = h.state();
            let got = h.sum(u);
            // C never returns an error sentinel: the result is always exactly
            // the wrapped sum, whatever the input.
            assert_eq!(got, before.wrapping_add(u));
        }
    }

    /// E2 — `update = INT_MAX` on a positive total: signed overflow wraps.
    #[test]
    fn err_e2_static_sum_overflow_int_max() {
        let mut h = harness();
        let mut rng = Rng::new(0xE002);
        for _ in 0..100 {
            let start = rng.range(1, i32::MAX);
            h.set_state(start);
            let got = h.sum(i32::MAX);
            assert_eq!(got, start.wrapping_add(i32::MAX));
        }
        h.set_state(1);
        assert_eq!(h.sum(i32::MAX), i32::MIN);
    }

    /// E3 — `update = INT_MIN` on a negative total: signed underflow wraps.
    #[test]
    fn err_e3_static_sum_underflow_int_min() {
        let mut h = harness();
        let mut rng = Rng::new(0xE003);
        for _ in 0..100 {
            let start = rng.range(i32::MIN, -1);
            h.set_state(start);
            let got = h.sum(i32::MIN);
            assert_eq!(got, start.wrapping_add(i32::MIN));
        }
        h.set_state(-1);
        assert_eq!(h.sum(i32::MIN), i32::MAX);
    }

    /// E4 — repeated `INT_MAX` updates: wraps every time, never saturates.
    #[test]
    fn err_e4_static_sum_repeated_overflow() {
        let mut h = harness();
        h.set_state(0);
        let mut model = 0i32;
        for _ in 0..64 {
            let got = h.sum(i32::MAX);
            model = model.wrapping_add(i32::MAX);
            assert_eq!(got, model);
        }
        h.set_state(0);
        let mut model = 0i32;
        for _ in 0..64 {
            let got = h.sum(i32::MIN);
            model = model.wrapping_add(i32::MIN);
            assert_eq!(got, model);
        }
    }

    /// E5 — degenerate `update = 0`.
    #[test]
    fn err_e5_static_sum_zero_update() {
        let mut h = harness();
        for start in [0, 1, -1, i32::MAX, i32::MIN, 0x5555_5555u32 as i32] {
            h.set_state(start);
            assert_eq!(h.sum(0), start);
            assert_eq!(h.sum(0), start);
        }
    }

    /// E6 — `driver(INT_MAX)`: `i * stride` overflows from i == 2 on.
    #[test]
    fn err_e6_driver_stride_int_max() {
        let mut h = harness();
        for start in [0, 1, -1, i32::MAX, i32::MIN] {
            h.set_state(start);
            let out = h.driver(i32::MAX);
            assert_eq!(out, expected_driver_output(start, i32::MAX));
            assert_eq!(lines_of(&out).len(), 10);
        }
    }

    /// E7 — `driver(INT_MIN)`: `i * stride` overflows from i == 2 on.
    #[test]
    fn err_e7_driver_stride_int_min() {
        let mut h = harness();
        for start in [0, 1, -1, i32::MAX, i32::MIN] {
            h.set_state(start);
            let out = h.driver(i32::MIN);
            assert_eq!(out, expected_driver_output(start, i32::MIN));
            assert_eq!(lines_of(&out).len(), 10);
        }
    }

    /// E8 — one step past the largest overflow-free stride, and the largest
    /// safe stride itself.
    #[test]
    fn err_e8_driver_stride_one_past_safe_range() {
        let mut h = harness();
        for stride in [
            SAFE_STRIDE_MAX,
            SAFE_STRIDE_MAX + 1,
            -SAFE_STRIDE_MAX,
            -SAFE_STRIDE_MAX - 1,
        ] {
            h.set_state(0);
            let out = h.driver(stride);
            assert_eq!(out, expected_driver_output(0, stride));
        }
        // Sanity on the boundary itself: 9 * SAFE_STRIDE_MAX must not overflow,
        // 9 * (SAFE_STRIDE_MAX + 1) must.
        assert!(9i32.checked_mul(SAFE_STRIDE_MAX).is_some());
        assert!(9i32.checked_mul(SAFE_STRIDE_MAX + 1).is_none());
    }

    /// E9 — `driver(0)`.
    #[test]
    fn err_e9_driver_stride_zero() {
        let mut h = harness();
        for start in [0, 12345, -12345, i32::MAX, i32::MIN] {
            h.set_state(start);
            let out = h.driver(0);
            assert_eq!(out, expected_driver_output(start, 0));
            assert_eq!(h.state(), start);
        }
    }

    /// E10 — negative stride is accepted verbatim (no sign validation).
    #[test]
    fn err_e10_driver_negative_stride() {
        let mut h = harness();
        let mut rng = Rng::new(0xE010);
        for _ in 0..100 {
            let stride = rng.range(i32::MIN, -1);
            let before = h.state();
            let out = h.driver(stride);
            assert_eq!(out, expected_driver_output(before, stride));
        }
    }

    /// E11 — "out of range enum" values: the prototypes take plain `int`, so a
    /// bit pattern with no valid variant is just an `int` and must be accepted
    /// identically by both libraries.
    #[test]
    fn err_e11_out_of_range_enum_like_values() {
        let mut h = harness();
        let weird: [i32; 12] = [
            i32::MAX,
            i32::MIN,
            0xDEAD_BEEFu32 as i32,
            0xFFFF_FFFFu32 as i32, // -1
            0x8000_0000u32 as i32, // INT_MIN
            0x7FFF_FFFF,
            0xCAFE_BABEu32 as i32,
            -12345678,
            999,
            -999,
            0x0000_FFFF,
            0xFFFF_0000u32 as i32,
        ];
        for &v in &weird {
            h.set_state(0);
            assert_eq!(h.sum(v), v, "static_sum must accept {v:#x} verbatim");
            h.set_state(0);
            let out = h.driver(v);
            assert_eq!(
                out,
                expected_driver_output(0, v),
                "driver must accept {v:#x} verbatim"
            );
        }
    }

    /// E12 — the trip count is a literal `10`: no stride can make `driver`
    /// print 0 lines or an oversized number of lines.
    #[test]
    fn err_e12_driver_always_ten_lines() {
        let mut h = harness();
        let mut rng = Rng::new(0xE012);
        for _ in 0..150 {
            let stride = if rng.bool() { rng.i32_any() } else { 0 };
            let out = h.driver(stride);
            assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 10);
            assert!(!out.is_empty());
            assert!(out.ends_with(b"\n"));
        }
    }

    /// E13 — structural: neither entry point takes a pointer, so there is no
    /// null-pointer input to differ on. Verified against the public header.
    #[test]
    fn err_e13_no_pointer_parameters() {
        let header = std::fs::read_to_string(
            common::manifest_dir().join("c_src/include/staticloop.h"),
        )
        .expect("read public header");
        let decls: Vec<&str> = header
            .lines()
            .filter(|l| l.contains("static_sum") || l.contains("driver"))
            .collect();
        assert_eq!(decls.len(), 2, "public header must declare exactly 2 fns");
        for d in &decls {
            assert!(!d.contains('*'), "no pointer parameters expected in {d:?}");
        }
        assert!(decls.iter().any(|d| d.contains("int static_sum(int update);")));
        assert!(decls.iter().any(|d| d.contains("void driver(int update);")));

        // Both symbols are nevertheless reachable and behave identically.
        let mut h = harness();
        h.set_state(0);
        assert_eq!(h.sum(0), 0);
        let out = h.driver(0);
        assert_eq!(out, expected_driver_output(0, 0));
    }

    /// E14 — wrap at both ends of the range.
    #[test]
    fn err_e14_static_sum_wrap_at_both_ends() {
        let mut h = harness();

        h.set_state(i32::MIN);
        assert_eq!(h.sum(-1), i32::MAX);

        h.set_state(i32::MAX);
        assert_eq!(h.sum(1), i32::MIN);

        // one more step each way
        h.set_state(i32::MIN);
        assert_eq!(h.sum(-2), i32::MAX - 1);

        h.set_state(i32::MAX);
        assert_eq!(h.sum(2), i32::MIN + 1);

        // and via driver
        h.set_state(i32::MAX - 1);
        let out = h.driver(1);
        assert_eq!(out, expected_driver_output(i32::MAX - 1, 1));
    }
}

// ===========================================================================
// Phase D — symbol parity
// ===========================================================================
mod symbols {
    use super::common::{c_so_path, rust_so_path};
    use std::process::Command;

    /// Diagnostic: shows exactly which two shared objects are under test, and
    /// asserts they are two distinct real files (so the suite cannot silently
    /// compare a library against itself).
    #[test]
    fn loaded_library_paths() {
        let c = c_so_path();
        let r = rust_so_path();
        eprintln!("C   .so under test: {}", c.display());
        eprintln!("Rust .so under test: {}", r.display());
        assert!(c.is_file(), "missing C .so: {}", c.display());
        assert!(r.is_file(), "missing Rust .so: {}", r.display());
        let c_canon = c.canonicalize().unwrap();
        let r_canon = r.canonicalize().unwrap();
        assert_ne!(
            c_canon, r_canon,
            "the C and Rust libraries must be two different files"
        );
        // The Rust .so must come from the same build profile as this test binary.
        let exe = std::env::current_exe().unwrap();
        let exe_s = exe.to_string_lossy().to_string();
        let r_s = r_canon.to_string_lossy().to_string();
        for profile in ["/release/", "/debug/"] {
            if exe_s.contains(profile) {
                assert!(
                    r_s.contains(profile),
                    "test binary is {profile} but the Rust .so is {r_s}"
                );
            }
        }
    }

    fn exported_symbols(path: &std::path::Path) -> Option<Vec<String>> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let mut syms: Vec<String> = text
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(str::to_string))
            .collect();
        syms.sort();
        syms.dedup();
        Some(syms)
    }

    /// Every symbol the C `.so` exports must also be exported by the Rust
    /// `.so`, with the exact same name.
    #[test]
    fn symbol_parity_c_subset_of_rust() {
        let c = c_so_path();
        let r = rust_so_path();
        let (Some(cs), Some(rs)) = (exported_symbols(&c), exported_symbols(&r)) else {
            eprintln!("`nm` unavailable — skipping symbol parity check");
            return;
        };
        assert!(!cs.is_empty(), "C library exported no symbols?");
        let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
             C   = {cs:?}\nRust= {rs:?}"
        );
        assert!(cs.contains(&"static_sum".to_string()));
        assert!(cs.contains(&"driver".to_string()));
    }

    /// The Rust `.so` must not have unresolved non-libc dependencies.
    #[test]
    fn rust_so_has_no_unexpected_undefined_symbols() {
        let r = rust_so_path();
        let out = match Command::new("nm").args(["-D", "-u"]).arg(&r).output() {
            Ok(o) if o.status.success() => o,
            _ => {
                eprintln!("`nm` unavailable — skipping undefined-symbol check");
                return;
            }
        };
        let text = String::from_utf8_lossy(&out.stdout);
        let bad: Vec<&str> = text
            .lines()
            .filter_map(|l| l.split_whitespace().last())
            .filter(|s| {
                // Everything here must come from libc / libgcc / the loader.
                let base = s.split('@').next().unwrap_or(s);
                !(base.starts_with("_Unwind_")
                    || base.starts_with("__")
                    || base.starts_with("_ITM_")
                    || base.starts_with("pthread_")
                    || base.starts_with("dl_")
                    || matches!(
                        base,
                        "printf"
                            | "abort"
                            | "bcmp"
                            | "calloc"
                            | "close"
                            | "free"
                            | "fstat"
                            | "fstat64"
                            | "getcwd"
                            | "getenv"
                            | "gettid"
                            | "lseek"
                            | "lseek64"
                            | "malloc"
                            | "memcmp"
                            | "memcpy"
                            | "memmove"
                            | "memset"
                            | "mmap"
                            | "mmap64"
                            | "munmap"
                            | "open"
                            | "open64"
                            | "posix_memalign"
                            | "read"
                            | "readlink"
                            | "realloc"
                            | "realpath"
                            | "stat"
                            | "stat64"
                            | "statx"
                            | "strlen"
                            | "syscall"
                            | "write"
                            | "writev"
                    ))
            })
            .collect();
        assert!(
            bad.is_empty(),
            "unexpected undefined (non-libc) symbols in the Rust .so: {bad:?}"
        );
    }
}
