//! Phase B — valid-path differential tests for the **lowest-level entry point**,
//! the exported `void driver(int)` symbol, called in-process through
//! `libloading` on *both* shared objects (`CONFIGS.md` rows 1–7).
//!
//! This binary runs without libtest (`harness = false` in `Cargo.toml`): the
//! comparison captures whatever the loaded `.so` writes to file descriptor 1,
//! and libtest's own progress output would otherwise be interleaved into that
//! capture from other threads. Progress here goes to `stderr` instead, and only
//! outside a capture window.
//!
//! The Rust implementation is never called directly — every value goes through
//! `dlopen` + `dlsym("driver")`, exactly like an external C caller.

mod common;

use common::*;

fn main() {
    // Force both libraries to load (and report a clear error if they cannot)
    // before any capture window is opened.
    eprintln!("C   .so: {}", c_so().display());
    eprintln!("Rust.so: {}", rust_so().display());
    assert!(
        c_lib().has_symbol(b"driver\0"),
        "C .so does not export `driver`"
    );
    assert!(
        rust_lib().has_symbol(b"driver\0"),
        "Rust .so does not export `driver`"
    );

    let mut r = Runner::new();

    // Row 1 — smallest magnitudes, no overflow anywhere.
    r.case("row01_driver_small_values", || {
        let xs: Vec<i32> = vec![0, 1, -1, 2, -2, 10, -10, 3, -3, 100, -100];
        assert_same_driver_batch("row01", &xs);
    });

    // Row 2 — uniform random `i32` over the full range, 4096 samples.
    r.case("row02_driver_random_full_range", || {
        let mut rng = Rng::new(0xC0FFEE_1234_5678);
        let xs: Vec<i32> = (0..4096).map(|_| rng.next_i32()).collect();
        assert_same_driver_batch("row02", &xs);
    });

    // Row 3 — domain extremes, where `2*x` overflows `int`.
    r.case("row03_driver_domain_extremes", || {
        let xs = vec![i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX, -1, 0];
        assert_same_driver_batch("row03", &xs);
    });

    // Row 4 — every single-bit magnitude and its neighbours, both signs.
    r.case("row04_driver_powers_of_two", || {
        let mut xs = Vec::new();
        for k in 0..32u32 {
            let v = 1i32.wrapping_shl(k);
            xs.push(v);
            xs.push(v.wrapping_neg());
            xs.push(v.wrapping_sub(1));
            xs.push(v.wrapping_add(1));
        }
        assert_same_driver_batch("row04", &xs);
    });

    // Row 5 — the band where `2*x` fits but `y += 300` overflows.
    r.case("row05_driver_positive_add_overflow_band", || {
        let mid = i32::MAX / 2; // 1073741823
        let xs: Vec<i32> = (-200..=200).map(|d| mid.wrapping_add(d)).collect();
        assert_same_driver_batch("row05", &xs);
    });

    // Row 6 — the negative mirror of row 5.
    r.case("row06_driver_negative_overflow_band", || {
        let mid = i32::MIN / 2; // -1073741824
        let xs: Vec<i32> = (-200..=200).map(|d| mid.wrapping_add(d)).collect();
        assert_same_driver_batch("row06", &xs);
    });

    // Row 7 — `printf("%d\n")` formatting boundaries: sign flips and every
    // digit-count change of the *result*.
    r.case("row07_driver_format_boundaries", || {
        let mut xs = vec![-150, -151, -149, -50, -5, 0, 1, -1];
        for p in 0..10u32 {
            let ten = 10i64.pow(p);
            for d in [-1i64, 0, 1] {
                for y in [ten + d, -(ten + d)] {
                    if (y - 300) % 2 == 0 {
                        let x = (y - 300) / 2;
                        if x >= i32::MIN as i64 && x <= i32::MAX as i64 {
                            xs.push(x as i32);
                        }
                    }
                }
            }
        }
        assert_same_driver_batch("row07", &xs);
    });

    // Row 25 (FFI half) — the `dev`-profile `cdylib`, i.e. overflow checks ON,
    // must agree with the C `.so` for the same overflowing arguments.
    r.case("row25_driver_dev_profile_cdylib", || {
        let mut rng = Rng::new(0x25f);
        let mut xs = vec![
            0,
            1,
            -1,
            i32::MIN,
            i32::MAX,
            1_073_741_824,
            -1_073_741_824,
            1_073_741_823,
        ];
        for _ in 0..1024 {
            xs.push(rng.next_i32());
        }
        let c = c_lib().driver_batch(&xs);
        let d = rust_lib_dev().driver_batch(&xs);
        assert_eq!(
            show(&c),
            show(&d),
            "[row25] dev-profile cdylib diverges from C"
        );
    });

    r.finish("phase_b_ffi");
}
