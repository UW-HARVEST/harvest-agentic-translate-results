//! Differential tests: `driver` in the C `.so` vs. the Rust `cdylib`.
//!
//! The public API is a single function (`c_src/include/driver.h`), whose only
//! observable effect is the hex dump it prints. The internal helper `print_hex`
//! is `static` in C, so it has no exported symbol and is exercised only through
//! `driver` — the test order below therefore goes from the byte formatting it
//! produces up to whole-function behaviour.

mod common;

use common::{
    assert_driver_matches, c_so_path, hex, rust_so_path, run_both, sym, DriverFn,
};

/// Both shared objects must expose `driver` under exactly that name.
fn driver_symbol_is_exported_by_both() {
    let _c = sym::<DriverFn>(common::c_lib(), "driver");
    let _r = sym::<DriverFn>(common::rust_lib(), "driver");
}

/// `print_hex` has internal linkage in C, so neither library should export it.
fn print_hex_is_not_exported_by_either() {
    for lib in [common::c_lib(), common::rust_lib()] {
        let missing = unsafe { lib.get::<DriverFn>(b"print_hex\0") }.is_err();
        assert!(missing, "print_hex must not be an exported symbol");
    }
}

/// Lowest observable level: the shape of what `print_hex` emits.
///
/// The C output defines the contract — an even number of lowercase hex digits
/// followed by a single newline — and Rust must reproduce it exactly.
fn output_is_lowercase_hex_followed_by_newline() {
    let (c_out, r_out) = run_both(0);
    assert_eq!(c_out, r_out, "driver(0) mismatch");

    let s = String::from_utf8(c_out).expect("output is ASCII");
    assert!(s.ends_with('\n'), "output must end with a newline: {s:?}");
    let body = &s[..s.len() - 1];
    assert!(!body.contains('\n'), "exactly one newline expected: {s:?}");
    assert_eq!(body.len() % 2, 0, "two hex digits per byte: {body:?}");
    assert!(
        body.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "digits must be lowercase hex: {body:?}"
    );
}

/// The dump covers `sizeof(house_t)`. Pin that down from the C side so a Rust
/// struct-layout divergence (e.g. wrong alignment or a `repr` slip) is caught as
/// a length change rather than only as a content change.
fn dump_length_matches_c_struct_size() {
    let (c_out, r_out) = run_both(1);
    assert_eq!(c_out, r_out);
    // int + int + double, 8-byte aligned => 16 bytes => 32 hex digits + '\n'.
    assert_eq!(c_out.len(), 33, "unexpected dump length: {:?}", hex(&c_out));
}

/// The struct is zero-initialised and then fully assigned, so the constant
/// fields must appear identically in both dumps for any input.
fn constant_fields_agree() {
    for x in [0, 7, -7] {
        let (c_out, r_out) = run_both(x);
        assert_eq!(c_out, r_out, "driver({x}) mismatch");
        let s = String::from_utf8(c_out).unwrap();
        // Bytes 4..8 hold `bedrooms`, bytes 8..16 hold `bathrooms`.
        let bedrooms = &s[8..16];
        let bathrooms = &s[16..32];
        assert_eq!(
            (bedrooms, bathrooms),
            ("03000000", "0000000000000040"),
            "unexpected constant field bytes for driver({x}): {s}"
        );
    }
}

/// Boundary and sign-sensitive values for the `int` parameter.
fn boundary_inputs_match() {
    let cases = [
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        127,
        -128,
        255,
        256,
        -256,
        32767,
        -32768,
        65535,
        65536,
        0x0000_00ff,
        0x0000_ff00,
        0x00ff_0000,
        0x7f00_0000,
        0x1234_5678,
        -0x1234_5678,
        0x0f0f_0f0f,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    for x in cases {
        assert_driver_matches(x);
    }
}

/// Every single-bit pattern, which walks each byte lane of `floors`
/// independently (including the sign bit).
fn single_bit_inputs_match() {
    for bit in 0..32 {
        assert_driver_matches(1i32.wrapping_shl(bit));
        assert_driver_matches(!(1i32.wrapping_shl(bit)));
    }
}

/// A deterministic pseudo-random sweep over the full `int` range.
fn pseudo_random_sweep_matches() {
    // xorshift32, fixed seed: reproducible without pulling in a rand dependency.
    let mut state: u32 = 0x1234_5678;
    for _ in 0..512 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        assert_driver_matches(state as i32);
    }
}

/// A dense sweep of small values, covering the common case exhaustively.
fn small_value_sweep_matches() {
    for x in -300..=300 {
        assert_driver_matches(x);
    }
}

/// Repeated calls must stay identical: the C version rebuilds `house` on the
/// stack each time, so no state may leak between invocations in either library.
fn repeated_calls_are_stable() {
    let first = run_both(99);
    assert_eq!(first.0, first.1);
    for _ in 0..20 {
        let again = run_both(99);
        assert_eq!(again.0, again.1);
        assert_eq!(again.0, first.0, "output changed across repeated calls");
    }
}

/// Interleaving the two libraries must not change either one's output, which
/// also confirms neither leaves the shared `stdout` stream in an odd state.
fn interleaved_calls_match() {
    let c = common::c_driver();
    let r = common::rust_driver();
    let _guard = common::OUT_LOCK.lock().unwrap();
    let mut prev: Option<Vec<u8>> = None;
    for i in 0..10 {
        let x = i * 37 - 100;
        let a = common::capture_stdout(|| unsafe { c(x) });
        let b = common::capture_stdout(|| unsafe { r(x) });
        let a2 = common::capture_stdout(|| unsafe { c(x) });
        assert_eq!(a, b, "driver({x}) mismatch under interleaving");
        assert_eq!(a, a2, "C output not reproducible for driver({x})");
        if let Some(p) = &prev {
            assert_ne!(*p, a, "distinct inputs unexpectedly produced equal output");
        }
        prev = Some(a);
    }
}

/// Step 8 as an automated check: every symbol the C `.so` exports dynamically
/// must also be exported, under the same name, by the Rust `.so`.
fn exported_symbol_sets_match() {
    fn defined_globals(path: &std::path::Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .expect("run nm");
        assert!(
            out.status.success(),
            "nm failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    let c_syms = defined_globals(&c_so_path());
    let rust_syms = defined_globals(&rust_so_path());
    assert!(
        c_syms.contains(&"driver".to_string()),
        "sanity check: C .so should export `driver`, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}"
    );
}

/// Sequential runner (see the `harness = false` note in Cargo.toml).
///
/// Progress is reported on stderr so that nothing this runner prints can land
/// in a stdout capture. Checks are ordered lowest-level first.
fn main() {
    let checks: &[(&str, fn())] = &[
        // Exported surface.
        ("driver_symbol_is_exported_by_both", driver_symbol_is_exported_by_both),
        ("print_hex_is_not_exported_by_either", print_hex_is_not_exported_by_either),
        ("exported_symbol_sets_match", exported_symbol_sets_match),
        // print_hex's byte formatting, observed through driver.
        ("output_is_lowercase_hex_followed_by_newline", output_is_lowercase_hex_followed_by_newline),
        ("dump_length_matches_c_struct_size", dump_length_matches_c_struct_size),
        ("constant_fields_agree", constant_fields_agree),
        // driver over its full input domain.
        ("boundary_inputs_match", boundary_inputs_match),
        ("single_bit_inputs_match", single_bit_inputs_match),
        ("small_value_sweep_matches", small_value_sweep_matches),
        ("pseudo_random_sweep_matches", pseudo_random_sweep_matches),
        // Call-sequence behaviour.
        ("repeated_calls_are_stable", repeated_calls_are_stable),
        ("interleaved_calls_match", interleaved_calls_match),
    ];

    let filter = std::env::args().skip(1).find(|a| !a.starts_with('-'));

    eprintln!("\nC .so   : {}", c_so_path().display());
    eprintln!("Rust .so: {}\n", rust_so_path().display());

    let mut passed = 0usize;
    let mut failed: Vec<&str> = Vec::new();

    for (name, f) in checks {
        if let Some(pat) = &filter {
            if !name.contains(pat.as_str()) {
                continue;
            }
        }
        eprint!("test {name} ... ");
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(()) => {
                eprintln!("ok");
                passed += 1;
            }
            Err(_) => {
                eprintln!("FAILED");
                failed.push(name);
            }
        }
    }

    eprintln!("\nresult: {passed} passed; {} failed", failed.len());
    if !failed.is_empty() {
        eprintln!("failures: {failed:?}");
        std::process::exit(1);
    }
}
