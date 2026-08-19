//! Phase C (FFI half) — error/boundary-path differential tests that go through
//! the exported `driver` symbol **in-process** with `libloading`, covering
//! `ERRORS.md` rows 20–22 plus the generic across-the-FFI-boundary boundaries.
//!
//! `harness = false` for the same reason as `phase_b_ffi`: the comparison
//! redirects file descriptor 1 to capture what the loaded `.so` prints, so
//! nothing else may write there while a capture is open.

mod common;

use common::*;

/// Reference value of the C expression `2*x + 300` with 32-bit wraparound,
/// formatted exactly like `printf("%d\n", y)`.
fn expected(x: i32) -> String {
    format!("{}\n", 2i32.wrapping_mul(x).wrapping_add(300))
}

fn main() {
    eprintln!("C   .so: {}", c_so().display());
    eprintln!("Rust.so: {}", rust_so().display());

    let mut r = Runner::new();

    // ---- Row 20: `2*x` overflows `int` (UB in C, wraps on the target). ----
    r.case("err20_ffi_multiply_overflow", || {
        let xs: Vec<i32> = vec![
            i32::MIN,
            i32::MIN + 1,
            i32::MAX,
            i32::MAX - 1,
            1_073_741_824,
            -1_073_741_824,
            1_073_741_825,
            -1_073_741_825,
        ];
        assert_same_driver_batch("err20", &xs);
        // and pin the C's actual sentinel, not just "they agree"
        let got = c_lib().driver_batch(&xs);
        let want: String = xs.iter().map(|&x| expected(x)).collect();
        assert_eq!(show(&got), show(want.as_bytes()), "[err20] C sentinel drift");
    });

    // ---- Row 21: `y += 300` overflows although `2*x` did not. ----
    r.case("err21_ffi_add_overflow", || {
        let mut xs = Vec::new();
        for d in -400..=400i32 {
            xs.push((i32::MAX / 2).wrapping_add(d));
            xs.push((i32::MIN / 2).wrapping_add(d));
        }
        assert_same_driver_batch("err21", &xs);
        let got = rust_lib().driver_batch(&xs);
        let want: String = xs.iter().map(|&x| expected(x)).collect();
        assert_eq!(
            show(&got),
            show(want.as_bytes()),
            "[err21] Rust deviates from the wrapping reference"
        );
    });

    // ---- Row 22: extreme `c_int` arguments across the FFI boundary. ----
    r.case("err22_ffi_extreme_args", || {
        let xs: Vec<i32> = vec![
            0,
            -1,
            1,
            i32::MIN,
            i32::MAX,
            0x8000_0000u32 as i32,
            0xFFFF_FFFFu32 as i32,
            0xDEAD_BEEFu32 as i32,
            0xAAAA_AAAAu32 as i32,
            0x5555_5555,
            0x0000_0001,
            0x7FFF_FFFF,
        ];
        assert_same_driver_batch("err22", &xs);
    });

    // ---- Generic: "out-of-range enum" analogue. -----------------------------
    // The API declares no enum, so every one of the 2^32 bit patterns is a
    // valid `int`. The nearest equivalent to "a value with no valid variant" is
    // therefore a dense sweep of small discriminant-like values (and their
    // negatives), plus every byte-repeated pattern, passed straight across the
    // boundary.
    r.case("generic_ffi_no_invalid_variant_exists", || {
        let mut xs: Vec<i32> = Vec::new();
        for v in -300..=300i32 {
            xs.push(v);
        }
        for b in 0..=255u32 {
            let p = b | (b << 8) | (b << 16) | (b << 24);
            xs.push(p as i32);
        }
        for k in 0..32u32 {
            xs.push((1u32 << k) as i32);
            xs.push(!(1u32 << k) as i32);
        }
        assert_same_driver_batch("generic/variants", &xs);
    });

    // ---- Generic: the export is stateless across repeated calls. -----------
    r.case("generic_ffi_repeated_calls_are_stateless", || {
        let xs = vec![7i32; 64];
        assert_same_driver_batch("generic/stateless", &xs);
        let got = rust_lib().driver_batch(&xs);
        assert_eq!(
            show(&got),
            show("314\n".repeat(64).as_bytes()),
            "[generic/stateless] repeated driver(7) drifted"
        );
    });

    // ---- Generic: interleaving the two libraries must not disturb either. --
    r.case("generic_ffi_interleaved_libraries", || {
        let mut rng = Rng::new(0xFF11);
        for _ in 0..256 {
            let x = rng.next_i32();
            let c = c_lib().driver(x);
            let rr = rust_lib().driver(x);
            assert_eq!(
                show(&c),
                show(&rr),
                "[generic/interleaved] driver({x}) diverged"
            );
            assert_eq!(
                show(&c),
                show(expected(x).as_bytes()),
                "[generic/interleaved] C sentinel drift for {x}"
            );
        }
    });

    // ---- Generic: the dev-profile cdylib must reject/wrap identically. -----
    r.case("generic_ffi_dev_profile_overflow_checks", || {
        // With `-C overflow-checks=on`, a translation that used plain `*`/`+`
        // instead of wrapping arithmetic would panic here instead of wrapping.
        let xs: Vec<i32> = vec![
            i32::MIN,
            i32::MAX,
            1_073_741_824,
            -1_073_741_824,
            i32::MAX / 2,
            i32::MIN / 2,
        ];
        let c = c_lib().driver_batch(&xs);
        let d = rust_lib_dev().driver_batch(&xs);
        assert_eq!(
            show(&c),
            show(&d),
            "[generic/dev-overflow] dev-profile cdylib diverged from C"
        );
    });

    // ---- Symbol-level boundary: a name that must NOT exist. ----------------
    r.case("generic_ffi_no_extra_symbols_resolve", || {
        for missing in [
            &b"scanf_d\0"[..],
            b"run\0",
            b"driver_impl\0",
            b"rust_main\0",
            b"prog\0",
        ] {
            assert_eq!(
                c_lib().has_symbol(missing),
                rust_lib().has_symbol(missing),
                "symbol {:?} resolves on one side only",
                String::from_utf8_lossy(missing)
            );
        }
    });

    r.finish("phase_c_ffi");
}
