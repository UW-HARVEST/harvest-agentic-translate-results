//! Differential tests for `doubleneg` — the composed pipeline whose observable
//! behaviour includes **stdout**.
//!
//! This target uses `harness = false` (see `Cargo.toml`) and runs every row
//! sequentially. That is deliberate: comparing output requires redirecting file
//! descriptor 1, which is process-global, and the libtest harness itself writes
//! progress lines to fd 1 from another thread. A `harness = false` runner is the
//! only way to guarantee that nothing else writes to fd 1 during a capture.
//!
//! Covers:
//!   * `CONFIGS.md` rows 38–50 (valid-path pipeline configurations)
//!   * `ERRORS.md`  rows 33–40 (pipeline rejection / UB branches)
//!
//! Both implementations are loaded from their `.so` via `libloading`.

mod common;

use std::ffi::{c_char, c_int};

use common::{apis, assert_bytes_eq, capture_stdout, Rng};

// ---------------------------------------------------------------------------
// runner
// ---------------------------------------------------------------------------

struct Runner {
    filter: Option<String>,
    passed: usize,
    failed: Vec<String>,
    skipped: usize,
}

impl Runner {
    fn new() -> Self {
        let filter = std::env::args()
            .skip(1)
            .find(|a| !a.starts_with("--"))
            .map(|s| s.to_string());
        Runner {
            filter,
            passed: 0,
            failed: Vec::new(),
            skipped: 0,
        }
    }

    fn row(&mut self, name: &str, body: impl FnOnce()) {
        if let Some(f) = &self.filter {
            if !name.contains(f.as_str()) {
                self.skipped += 1;
                return;
            }
        }
        print!("test {name} ... ");
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
        match res {
            Ok(()) => {
                println!("ok");
                self.passed += 1;
            }
            Err(_) => {
                println!("FAILED");
                self.failed.push(name.to_string());
            }
        }
        let _ = std::io::stdout().flush();
    }

    fn finish(self) {
        println!();
        if self.failed.is_empty() {
            println!(
                "test result: ok. {} passed; 0 failed; {} filtered out",
                self.passed, self.skipped
            );
        } else {
            println!("failures:");
            for f in &self.failed {
                println!("    {f}");
            }
            println!(
                "\ntest result: FAILED. {} passed; {} failed; {} filtered out",
                self.passed,
                self.failed.len(),
                self.skipped
            );
            std::process::exit(101);
        }
    }
}

// ---------------------------------------------------------------------------
// differential driver
// ---------------------------------------------------------------------------

const START_MARKER: &[u8] = b"=== Starting foo() execution ===\n";

/// Compare `doubleneg` return value AND stdout, byte-for-byte.
#[track_caller]
fn diff_e6(p1: c_int, p2: c_int, p3: c_int, p4: c_int) {
    let (c, r) = apis();
    let (cv, cout) = capture_stdout(|| unsafe { (c.doubleneg)(p1, p2, p3, p4) });
    let (rv, rout) = capture_stdout(|| unsafe { (r.doubleneg)(p1, p2, p3, p4) });
    let ctx = format!("doubleneg({p1}, {p2}, {p3}, {p4})");
    // Guard against a contaminated capture (foreign writer on fd 1).
    assert!(
        cout.starts_with(START_MARKER) && rout.starts_with(START_MARKER),
        "{ctx}: capture contaminated by a foreign fd-1 writer"
    );
    assert_bytes_eq(&cout, &rout, &ctx);
    assert_eq!(cv, rv, "{ctx} return value: C = {cv} vs Rust = {rv}");
}

fn rng(tag: u64) -> Rng {
    Rng::new(0x5EED_D0B1_E000_0000 ^ tag)
}

fn main() {
    let mut t = Runner::new();

    // =======================================================================
    // CONFIGS.md rows 38–50 — valid-path pipeline configurations
    // =======================================================================

    t.row("cfg_row_38_e6_all_zero_baseline", || {
        diff_e6(0, 0, 0, 0);
    });

    t.row("cfg_row_39_e6_param2_zero", || {
        let mut g = rng(39);
        for _ in 0..40 {
            diff_e6(g.next_i32(), 0, g.next_i32(), g.next_i32());
        }
        for p1 in [0i32, 1, -1, 100, 93, i32::MIN, i32::MAX] {
            diff_e6(p1, 0, 3, 4);
        }
    });

    t.row("cfg_row_40_e6_single_nonzero_param", || {
        diff_e6(5, 0, 0, 0);
        diff_e6(0, 5, 0, 0);
        diff_e6(0, 0, 5, 0);
        diff_e6(0, 0, 0, 5);
        diff_e6(-5, 0, 0, 0);
        diff_e6(0, -5, 0, 0);
        diff_e6(0, 0, -5, 0);
        diff_e6(0, 0, 0, -5);
    });

    t.row("cfg_row_41_e6_all_negation_combinations", || {
        for mask in 0..16u32 {
            let p = |bit: u32, v: c_int| if mask & (1 << bit) != 0 { v } else { 0 };
            diff_e6(p(0, 13), p(1, 7), p(2, 5), p(3, 3));
            diff_e6(p(0, -13), p(1, -7), p(2, -5), p(3, -3));
        }
    });

    t.row("cfg_row_42_e6_direct_search_branch_is_proved_taken", || {
        // The buffer is `(char)((param1 + 7i) % 256)` for i in 0..256. Reading it
        // back as `unsigned char` gives `(param1 + 7i) mod 256`, and since
        // gcd(7, 256) == 1 the 256 iterations cover EVERY byte value. So
        // `memchr(buffer, 100, 256)` can never return NULL inside `doubleneg`:
        // the `if (direct_search != NULL)` branch is provably always taken.
        // Verify that against the C build over a wide seed sweep, then compare
        // both implementations on those seeds.
        let (c, _) = apis();
        for seed in [
            0i32, 1, 100, -100, 156, -156, i32::MIN, i32::MAX, 12345, -98765, 7, -7, 255, -255,
        ] {
            let mut buf = vec![0i8; 256];
            unsafe { (c.create_numeric_buffer)(buf.as_mut_ptr(), 256, seed) };
            let mut seen = [false; 256];
            for &b in &buf {
                seen[b as u8 as usize] = true;
            }
            assert!(
                seen.iter().all(|&s| s),
                "seed {seed}: expected all 256 byte values in the buffer"
            );
            diff_e6(seed, 3, 5, 7);
        }
        // The NULL / -1 path itself is reachable only below `doubleneg`, and is
        // covered directly through the `find_value_in_buffer` export.
        let (cc, rr) = apis();
        let empty: [i8; 4] = [1, 2, 3, 4];
        let cv = unsafe { (cc.find_value_in_buffer)(empty.as_ptr(), 0, 100) };
        let rv = unsafe { (rr.find_value_in_buffer)(empty.as_ptr(), 0, 100) };
        assert_eq!(cv, -1);
        assert_eq!(cv, rv);
    });

    t.row("cfg_row_43_e6_direct_search_present", || {
        for seed in [100i32, 93, 86, 79, 72, -100, -156, 356] {
            diff_e6(seed, 1, 1, 1);
        }
    });

    t.row("cfg_row_44_e6_search_value_not_found_branch", || {
        let mut g = rng(44);
        for _ in 0..40 {
            diff_e6(
                g.small_i32(300),
                g.small_i32(300),
                g.small_i32(300),
                g.small_i32(300),
            );
        }
        diff_e6(43, 42, 42, 42);
        diff_e6(-42, -42, -42, -42);
    });

    t.row("cfg_row_45_e6_converted_int_out_of_range", || {
        let (c, _) = apis();
        let cases = [
            (i32::MAX, 1, 9),
            (i32::MIN, 1, 9),
            (1000000, 1, 9),
            (-1000000, 1, 9),
            (i32::MAX, -1, 3),
            (123456789, 2, 9),
        ];
        let mut saw_int_min = false;
        for (a, b, cc) in cases {
            let d = unsafe { (c.calculate_with_doubles)(a, b, cc) };
            if unsafe { (c.convert_double_to_int)(d) } == i32::MIN {
                saw_int_min = true;
            }
            diff_e6(a, b, cc, 1);
        }
        assert!(saw_int_min, "expected at least one out-of-range conversion");
    });

    t.row("cfg_row_46_e6_extreme_params", || {
        let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
        for &p1 in &extremes {
            for &p2 in &extremes {
                diff_e6(p1, p2, 7, -7);
            }
        }
        for &p3 in &extremes {
            for &p4 in &extremes {
                diff_e6(31, -17, p3, p4);
            }
        }
    });

    t.row("cfg_row_47_e6_negative_params", || {
        let mut g = rng(47);
        for _ in 0..60 {
            diff_e6(
                -(g.next_i32() & 0x7FFF_FFFF),
                -(g.next_i32() & 0x7FFF_FFFF),
                -(g.next_i32() & 0x7FFF_FFFF),
                -(g.next_i32() & 0x7FFF_FFFF),
            );
        }
        diff_e6(-1, -1, -1, -1);
        diff_e6(-256, -256, -256, -256);
        diff_e6(-255, -257, -511, -513);
    });

    t.row("cfg_row_48_e6_randomized_full_range", || {
        let mut g = rng(48);
        for _ in 0..300 {
            diff_e6(g.next_i32(), g.next_i32(), g.next_i32(), g.next_i32());
        }
    });

    t.row("cfg_row_49_e6_randomized_small_magnitude", || {
        let mut g = rng(49);
        for _ in 0..300 {
            diff_e6(
                g.small_i32(600),
                g.small_i32(600),
                g.small_i32(600),
                g.small_i32(600),
            );
        }
        for _ in 0..120 {
            diff_e6(
                g.small_i32(3),
                g.small_i32(3),
                g.small_i32(3),
                g.small_i32(3),
            );
        }
    });

    t.row("cfg_row_50_e6_stdout_formatting_classes", || {
        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            (1, 0, 0, 0),         // %e of 0.000000e+00
            (1, 3, 9, 1),         // large positive
            (-1, 3, 9, 1),        // large negative
            (1, 3, -9, 1),        // tiny positive
            (-1, 3, -9, 1),       // tiny negative
            (i32::MAX, 1, 9, 1),  // overflowing conversion -> %d INT_MIN
            (i32::MIN, 1, 9, 1),  // -> %d INT_MIN
            (i32::MIN, -1, 9, 1), // 2^31 * 1e9
            (0, 1, 0, 0),         // exactly 0.0 from division
            (100, 1, 1, 1),       // byte 100 at offset 0
        ];
        for &(a, b, c, d) in cases {
            diff_e6(a, b, c, d);
        }
        // Sanity-check that the formatting-sensitive renderings really occur.
        let (capi, _) = apis();
        let (_, out) = capture_stdout(|| unsafe { (capi.doubleneg)(1, 3, -9, 1) });
        let s = String::from_utf8_lossy(&out).to_string();
        assert!(s.contains("e-"), "expected a negative %e exponent in:\n{s}");
        let (_, out2) = capture_stdout(|| unsafe { (capi.doubleneg)(i32::MAX, 1, 9, 1) });
        let s2 = String::from_utf8_lossy(&out2).to_string();
        assert!(
            s2.contains("-2147483648"),
            "expected INT_MIN rendering in:\n{s2}"
        );
    });

    // =======================================================================
    // ERRORS.md rows 33–40 — pipeline rejection / UB branches
    // =======================================================================

    t.row("err_row_33_e6_search_hit_branch_is_proved_taken", || {
        // Every `search_values[i]` is `x % 256` with `x` an `int`, so it lies in
        // [-255, 255]; `(char)` narrowing maps it onto a byte that the buffer
        // always contains (see cfg_row_42). Therefore `pos >= 0` always holds
        // inside `doubleneg` and the `"Value %d not found"` branch is
        // unreachable there. Prove it against the C build, then diff.
        let (c, _) = apis();
        let mut g = rng(133);
        for _ in 0..60 {
            let (p1, p2, p3, p4) = (
                g.small_i32(1000),
                g.small_i32(1000),
                g.small_i32(1000),
                g.small_i32(1000),
            );
            let mut buf = vec![0i8; 256];
            unsafe { (c.create_numeric_buffer)(buf.as_mut_ptr(), 256, p1) };
            for sv in [p2 % 256, p3 % 256, p4 % 256, 42] {
                let pos = unsafe { (c.find_value_in_buffer)(buf.as_ptr(), 256, sv) };
                assert!(pos >= 0, "seed={p1} sv={sv}: expected a hit, got {pos}");
            }
            diff_e6(p1, p2, p3, p4);
        }
        diff_e6(1, -1000, -1000, -1000);
        // The `-1` sentinel itself is exercised directly on the export.
        let (cc, rr) = apis();
        let buf = [0x01i8; 16];
        let cv = unsafe { (cc.find_value_in_buffer)(buf.as_ptr(), 16, 0x02) };
        let rv = unsafe { (rr.find_value_in_buffer)(buf.as_ptr(), 16, 0x02) };
        assert_eq!(cv, -1, "expected the not-found sentinel from C");
        assert_eq!(cv, rv);
    });

    t.row("err_row_34_e6_direct_search_null_is_unreachable", || {
        // Exhaustively check a large seed range: `memchr(buffer, 100, 256)` is
        // never NULL, so the `if (direct_search != NULL)` false branch cannot be
        // reached through `doubleneg`. Both builds must agree on the seeds we do
        // drive, and the NULL/-1 behaviour is covered on the export directly.
        let (c, r) = apis();
        for seed in -2000..=2000i32 {
            let mut cb = vec![0i8; 256];
            let mut rb = vec![0i8; 256];
            unsafe { (c.create_numeric_buffer)(cb.as_mut_ptr(), 256, seed) };
            unsafe { (r.create_numeric_buffer)(rb.as_mut_ptr(), 256, seed) };
            assert_eq!(cb, rb, "buffer diverged for seed {seed}");
            assert!(
                cb.iter().any(|&b| (b as u8) == 100),
                "seed {seed}: byte 100 unexpectedly absent"
            );
        }
        for seed in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0] {
            diff_e6(seed, 2, 3, 4);
        }
        // NULL result / -1 sentinel, exercised where it IS reachable.
        let buf = [0x7Fi8; 8];
        let cv = unsafe { (c.find_value_in_buffer)(buf.as_ptr(), 8, 100) };
        let rv = unsafe { (r.find_value_in_buffer)(buf.as_ptr(), 8, 100) };
        assert_eq!(cv, -1);
        assert_eq!(cv, rv);
    });

    t.row("err_row_35_e6_param1_extremes_overflow", || {
        for p1 in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
            for p2 in [i32::MIN, -1, 0, 1, i32::MAX] {
                diff_e6(p1, p2, 0, 0);
            }
        }
    });

    t.row("err_row_36_e6_param2_zero_branch", || {
        for p1 in [0i32, 1, -1, 12345, -12345, i32::MIN, i32::MAX] {
            for p3 in [0i32, 9, -9, i32::MIN, i32::MAX] {
                diff_e6(p1, 0, p3, 1);
            }
        }
    });

    t.row("err_row_37_e6_search_byte_overflow", || {
        diff_e6(i32::MAX, i32::MIN, 1, 1);
        diff_e6(i32::MIN, i32::MAX, 1, 1);
        diff_e6(i32::MAX, i32::MAX, 1, 1);
        diff_e6(i32::MIN, i32::MIN, 1, 1);
        let mut g = rng(137);
        for _ in 0..40 {
            diff_e6(i32::MAX - (g.next_u32() % 32) as i32, i32::MIN, g.next_i32(), g.next_i32());
        }
    });

    t.row("err_row_38_e6_converted_int_negative_remainder", || {
        // converted_int == INT_MIN  =>  INT_MIN % 1000 == -648
        let (c, _) = apis();
        let mut saw = false;
        for (a, b, cc) in [
            (i32::MAX, 1, 9),
            (i32::MIN, 1, 9),
            (2000000, 1, 9),
            (-2000000, 1, 9),
            (i32::MAX, 2, 9),
        ] {
            let d = unsafe { (c.calculate_with_doubles)(a, b, cc) };
            let i = unsafe { (c.convert_double_to_int)(d) };
            if i == i32::MIN {
                assert_eq!(i % 1000, -648);
                saw = true;
            }
            diff_e6(a, b, cc, 0);
        }
        assert!(saw, "expected converted_int == INT_MIN");
    });

    t.row("err_row_39_e6_all_zero_deterministic", || {
        diff_e6(0, 0, 0, 0);
        // Same inputs twice must give identical output in both builds (no
        // dependence on the uninitialised `char buffer[256]`).
        for _ in 0..5 {
            diff_e6(0, 0, 0, 0);
        }
    });

    t.row("err_row_40_e6_result_accumulation_overflow", || {
        // Drive `result` towards int overflow: converted_int % 1000 is bounded,
        // so overflow is only reachable through repeated extremes; verify the
        // wrapped value matches regardless.
        for p in [i32::MIN, i32::MAX] {
            diff_e6(p, p, p, p);
            diff_e6(p, 1, 9, p);
        }
        let mut g = rng(140);
        for _ in 0..60 {
            diff_e6(g.next_i32(), g.next_i32(), 9, g.next_i32());
        }
    });

    // =======================================================================
    // Cross-check: `doubleneg`'s internal use of the low-level entry points is
    // consistent with calling them directly through the `.so` exports.
    // =======================================================================

    t.row("pipeline_lowlevel_consistency", || {
        let (c, r) = apis();
        let mut g = rng(200);
        for _ in 0..2000 {
            let (p1, p2, p3, p4) = (g.next_i32(), g.next_i32(), g.next_i32(), g.next_i32());
            let mut cb = vec![0i8; 256];
            let mut rb = vec![0i8; 256];
            unsafe { (c.create_numeric_buffer)(cb.as_mut_ptr(), 256, p1) };
            unsafe { (r.create_numeric_buffer)(rb.as_mut_ptr(), 256, p1) };
            assert_eq!(cb, rb, "buffer fill diverged for seed {p1}");
            for sv in [
                p2.wrapping_rem(256),
                p3.wrapping_rem(256),
                p4.wrapping_rem(256),
                42,
                100,
            ] {
                let cv = unsafe { (c.find_value_in_buffer)(cb.as_ptr() as *const c_char, 256, sv) };
                let rv = unsafe { (r.find_value_in_buffer)(rb.as_ptr() as *const c_char, 256, sv) };
                assert_eq!(cv, rv, "search diverged seed={p1} sv={sv}");
            }
            for i in 0..10i32 {
                let sb = p1.wrapping_add(i.wrapping_mul(p2)).wrapping_rem(256);
                let cv = unsafe { (c.find_value_in_buffer)(cb.as_ptr() as *const c_char, 256, sb) };
                let rv = unsafe { (r.find_value_in_buffer)(rb.as_ptr() as *const c_char, 256, sb) };
                assert_eq!(cv >= 0, rv >= 0, "found-flag diverged seed={p1} sb={sb}");
            }
        }
    });

    t.row("pipeline_heavy_fuzz", || {
        // A deep randomized sweep over the whole pipeline. `DNV_FUZZ_ITERS` can
        // raise the count; the default keeps the suite fast.
        let iters: usize = std::env::var("DNV_FUZZ_ITERS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3000);
        let mut g = rng(300);
        for k in 0..iters {
            // Mix magnitude classes so both the small-`%256` space and the
            // overflow-inducing extremes get hit.
            let pick = |g: &mut Rng, k: usize| -> c_int {
                match k % 4 {
                    0 => g.next_i32(),
                    1 => g.small_i32(300),
                    2 => g.small_i32(3),
                    _ => match g.next_u64() % 6 {
                        0 => i32::MIN,
                        1 => i32::MAX,
                        2 => 0,
                        3 => -1,
                        4 => 256,
                        _ => -256,
                    },
                }
            };
            let p1 = pick(&mut g, k);
            let p2 = pick(&mut g, k + 1);
            let p3 = pick(&mut g, k + 2);
            let p4 = pick(&mut g, k + 3);
            diff_e6(p1, p2, p3, p4);
        }
    });

    t.finish();
}
