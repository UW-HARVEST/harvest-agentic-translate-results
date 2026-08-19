//! Phase B — FFI-level differential tests for `driver`.
//!
//! CONFIGS.md rows 48–56.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! called through their exported `driver` symbol; the Rust side is never
//! called directly, so the `#[no_mangle] extern "C"` wrapper is under test
//! too. `driver` writes to stdout, so the comparison captures file descriptor
//! 1 around each batch of calls.

mod common;

use common::corpus;
use common::{c_so, capture_fd1, rust_so};
use libloading::{Library, Symbol};

type DriverFn = unsafe extern "C" fn(f32);

struct Loaded {
    _lib: Library,
    f: DriverFn,
}

fn load(path: &std::path::Path) -> Loaded {
    unsafe {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
        let sym: Symbol<DriverFn> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("dlsym driver in {path:?}: {e}"));
        let f = *sym;
        Loaded { _lib: lib, f }
    }
}

fn c_driver() -> Loaded {
    load(&c_so())
}

fn rust_driver() -> Loaded {
    load(&rust_so())
}

/// Call `driver` once per value and split the captured stdout into lines.
fn emit(d: &Loaded, values: &[f32]) -> Vec<String> {
    let bytes = capture_fd1(|| {
        for &v in values {
            unsafe { (d.f)(v) };
        }
    });
    let text = String::from_utf8(bytes).expect("driver emitted non-UTF-8");
    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    // `driver` always ends with '\n', so the trailing element is empty
    assert_eq!(lines.pop().as_deref(), Some(""), "missing trailing newline");
    lines
}

#[track_caller]
fn compare(values: &[f32], group: &str) {
    let c = c_driver();
    let r = rust_driver();
    let cl = emit(&c, values);
    let rl = emit(&r, values);
    assert_eq!(
        cl.len(),
        values.len(),
        "C driver emitted {} lines for {} values in `{group}`",
        cl.len(),
        values.len()
    );
    assert_eq!(
        rl.len(),
        values.len(),
        "Rust driver emitted {} lines for {} values in `{group}`",
        rl.len(),
        values.len()
    );
    let mut bad = Vec::new();
    for (i, (a, b)) in cl.iter().zip(rl.iter()).enumerate() {
        if a != b {
            bad.push(format!(
                "  [{i}] value bits={:08x} ({:?}): C={a} RUST={b}",
                values[i].to_bits(),
                values[i]
            ));
            if bad.len() >= 20 {
                break;
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} divergence(s) in `{group}`:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

/// Sanity check on the harness itself: `driver` must print 8 lowercase hex
/// digits plus a newline, and the bytes must be the little-endian object
/// representation of the float.
fn test_output_format_is_eight_lowercase_hex_digits() {
    let vals = [1.0f32, -0.0, f32::INFINITY, f32::from_bits(0x1234_5678)];
    for d in [c_driver(), rust_driver()] {
        let lines = emit(&d, &vals);
        for (v, l) in vals.iter().zip(&lines) {
            assert_eq!(l.len(), 8, "expected 8 hex digits, got {l:?}");
            assert!(
                l.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "expected lowercase hex, got {l:?}"
            );
            let expected: String = v
                .to_ne_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            assert_eq!(*l, expected, "wrong object representation for {v:?}");
        }
    }
}

/// Row 48 — signed zeros.
fn test_driver_signed_zeros() {
    compare(&[0.0f32, -0.0f32, 0.0f32 * -1.0, -(0.0f32)], "signed zeros");
}

/// Row 49 — subnormal and smallest-normal boundaries.
fn test_driver_subnormal_and_min_normal() {
    let v: Vec<f32> = [
        0x0000_0001u32,
        0x0000_0002,
        0x0000_0003,
        0x003f_ffff,
        0x007f_fffe,
        0x007f_ffff,
        0x0080_0000,
        0x0080_0001,
    ]
    .iter()
    .flat_map(|&b| [f32::from_bits(b), f32::from_bits(b | 0x8000_0000)])
    .collect();
    compare(&v, "subnormal / min normal");
}

/// Row 50 — ordinary finite values including `FLT_MAX`.
fn test_driver_ordinary_finite() {
    compare(
        &[
            1.0f32, -1.0, 0.5, -0.5, 2.0, -2.0, 3.14159, -3.14159,
            f32::MAX, f32::MIN, 1e30, -1e30, 1e-30, -1e-30, 16777216.0,
        ],
        "ordinary finite",
    );
}

/// Row 51 — infinities.
fn test_driver_infinities() {
    compare(&[f32::INFINITY, f32::NEG_INFINITY], "infinities");
}

/// Row 52 — quiet NaNs.
fn test_driver_quiet_nans() {
    compare(
        &[f32::from_bits(0x7fc0_0000), f32::from_bits(0xffc0_0000)],
        "quiet NaN",
    );
}

/// Row 53 — signalling NaNs and every "invalid variant" bit pattern. A C
/// `float` parameter accepts any 32-bit pattern, including the 2 × (2^23 - 1)
/// NaN payloads that no arithmetic ever produces; `driver` must print all of
/// them byte for byte without normalising anything.
fn test_driver_all_bit_pattern_classes() {
    let mut v = Vec::new();
    for payload in [1u32, 2, 0x20_0000, 0x3f_ffff, 0x40_0000, 0x7f_fffe, 0x7f_ffff] {
        v.push(f32::from_bits(0x7f80_0000 | payload)); // +NaN
        v.push(f32::from_bits(0xff80_0000 | payload)); // -NaN
    }
    // signalling NaNs specifically (payload MSB clear)
    v.push(f32::from_bits(0x7fa0_0000));
    v.push(f32::from_bits(0xffa0_0000));
    compare(&v, "NaN payload classes");
}

/// Row 54 — each byte lane swept over all 256 values, so every byte of the
/// object representation is exercised in every position.
fn test_driver_byte_lane_sweep() {
    let mut v = Vec::new();
    for lane in 0..4 {
        for byte in 0..=255u32 {
            v.push(f32::from_bits(byte << (8 * lane)));
            v.push(f32::from_bits(0xffff_ffff & !(0xffu32 << (8 * lane)) | (byte << (8 * lane))));
        }
    }
    compare(&v, "byte lane sweep");
}

/// Row 55 — uniform random patterns, fixed seed.
fn test_driver_random_patterns() {
    for seed in [1u64, 2, 3, 0xC0FF_EE] {
        let v = corpus::driver_values(seed, 20_000);
        compare(&v, &format!("random patterns seed={seed}"));
    }
}

/// Row 56 — stride sweep of the whole 2^32 space with a prime step, so every
/// exponent and mantissa residue class is hit.
fn test_driver_full_space_stride_sweep() {
    let v: Vec<f32> = corpus::driver_full_sweep(65_521).collect();
    assert!(v.len() > 60_000, "sweep too short: {}", v.len());
    // chunk it so the capture files stay small
    for (i, chunk) in v.chunks(20_000).enumerate() {
        compare(chunk, &format!("stride sweep chunk {i}"));
    }
}

/// Repeated calls must be independent — no state carried between them, and no
/// buffering difference that would reorder or merge lines.
fn test_driver_is_stateless_across_calls() {
    let mut v = Vec::new();
    for _ in 0..500 {
        v.extend_from_slice(&[
            f32::INFINITY,
            0.0,
            f32::from_bits(0x7fc0_0000),
            -0.0,
            1.0,
        ]);
    }
    compare(&v, "stateless repetition");
}

// ---------------------------------------------------------------------------
// Single test entry point
// ---------------------------------------------------------------------------
//
// `driver` writes to file descriptor 1, and capturing that descriptor is
// necessarily process-wide. libtest, running with its default parallelism,
// writes its own progress lines to the same descriptor from another thread,
// which would interleave into the capture. Driving every row from one
// `#[test]` removes the interleaving entirely (libtest prints only before the
// first row starts and after the last one finishes) while still reporting each
// row separately.

#[test]
fn ffi_differential_all_rows() {
    type Row = (&'static str, fn());
    let rows: &[Row] = &[
        ("format sanity", test_output_format_is_eight_lowercase_hex_digits as fn()),
        ("row 48 signed zeros", test_driver_signed_zeros),
        ("row 49 subnormal / min normal", test_driver_subnormal_and_min_normal),
        ("row 50 ordinary finite", test_driver_ordinary_finite),
        ("row 51 infinities", test_driver_infinities),
        ("row 52 quiet NaNs", test_driver_quiet_nans),
        ("row 53 all bit-pattern classes", test_driver_all_bit_pattern_classes),
        ("row 54 byte lane sweep", test_driver_byte_lane_sweep),
        ("row 55 random patterns", test_driver_random_patterns),
        ("row 56 full-space stride sweep", test_driver_full_space_stride_sweep),
        ("statelessness", test_driver_is_stateless_across_calls),
    ];

    let mut failed = Vec::new();
    for (name, f) in rows {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(*f)) {
            Ok(()) => eprintln!("  ffi row ok: {name}"),
            Err(p) => {
                let msg = p
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "<non-string panic>".to_string());
                failed.push(format!("  {name}:\n{msg}"));
            }
        }
    }
    assert!(
        failed.is_empty(),
        "{} FFI row(s) failed:\n{}",
        failed.len(),
        failed.join("\n")
    );
}
