// Differential test runner: C `.so` vs Rust `.so`, through `dlsym` only.
//
// This target uses `harness = false` deliberately. `driver`'s entire observable
// behaviour is bytes on `stdout`, so verifying it means hijacking file
// descriptor 1 around each call. libtest is hostile to that:
//
//   * it runs test fns on parallel threads, so two captures (and two libc
//     `printf`s) would race on the single process-wide fd 1;
//   * it writes its own progress lines ("test foo ... ok") to that same fd 1
//     from its main thread, which lands them inside a capture window;
//   * its partial "test foo ... " prefix sits unflushed in Rust's `LineWriter`
//     and gets flushed into whatever fd 1 happens to be at the time.
//
// An earlier libtest-based version of this suite produced exactly those
// artefacts (captured output containing "test c05_… FAILED"), i.e. false
// failures caused by the harness rather than by the translation. Running every
// row sequentially in one process, with all diagnostics on stderr, keeps stdout
// pristine and makes the byte comparison trustworthy.

mod common;

#[path = "cases/config.rs"]
mod config;
#[path = "cases/errors.rs"]
mod errors;
#[path = "cases/symbols.rs"]
mod symbols;

use std::panic::{catch_unwind, AssertUnwindSafe};

type Case = (&'static str, fn());

fn cases() -> Vec<Case> {
    let mut v: Vec<Case> = Vec::new();

    // ---- Phase D: symbol parity ------------------------------------------
    v.push(("D:symbol_parity", symbols::every_c_symbol_is_exported_by_rust));
    v.push(("D:no_unresolved", symbols::rust_so_has_no_unresolved_symbols));
    v.push(("D:dlsym_callable", symbols::driver_symbol_is_callable_from_both_libraries));

    // ---- Phase B: CONFIGS.md rows ----------------------------------------
    v.push(("C1:positive_normals", config::c01_positive_normals_randomized));
    v.push(("C2:negative_normals", config::c02_negative_normals_randomized));
    v.push(("C3:powers_of_two", config::c03_exact_powers_of_two_all_exponents));
    v.push(("C4:full_mantissa", config::c04_full_mantissa_no_trimming));
    v.push(("C5:trailing_zero_runs", config::c05_partial_trailing_zero_mantissa_runs));
    v.push(("C6:signed_zeros", config::c06_signed_zeros));
    v.push(("C7:infinities", config::c07_infinities));
    v.push(("C8:nan_family", config::c08_nan_family_randomized_payloads));
    v.push(("C9:subnormals", config::c09_subnormals_randomized));
    v.push(("C10:class_boundaries", config::c10_class_boundaries_with_neighbours));
    v.push(("C11:huge_magnitudes", config::c11_huge_magnitudes_long_fixed_output));
    v.push(("C12:tiny_magnitudes", config::c12_tiny_magnitudes_signed_underflow));
    v.push(("C13:round_half_even", config::c13_round_half_even_ties));
    v.push(("C14:hexfloat_exp_flip", config::c14_hexfloat_exponent_sign_flip));
    v.push(("C15:exponent_sweep", config::c15_exhaustive_exponent_sweep));
    v.push(("C16:random_bit_patterns", config::c16_full_domain_random_bit_patterns));
    v.push(("C17:stateless_repeat", config::c17_repeated_sequential_calls_are_stateless));
    v.push(("C18:stdio_interleaving", config::c18_interleaves_with_caller_stdio));

    // ---- Phase C: ERRORS.md rows -----------------------------------------
    v.push(("E1:all_zero_bits", errors::err_e1_all_zero_bits_no_pointer_to_be_null));
    v.push(("E2:negative_zero", errors::err_e2_negative_zero));
    v.push(("E3:dbl_max_longest", errors::err_e3_dbl_max_longest_output));
    v.push(("E4:positive_infinity", errors::err_e4_positive_infinity));
    v.push(("E5:negative_infinity", errors::err_e5_negative_infinity));
    v.push(("E6:qnan_positive", errors::err_e6_quiet_nan_positive));
    v.push(("E7:qnan_negative", errors::err_e7_quiet_nan_negative));
    v.push(("E8:signalling_nan", errors::err_e8_signalling_nan));
    v.push(("E9:nan_payload", errors::err_e9_nan_payload_preserved));
    v.push(("E10:smallest_subnormal", errors::err_e10_smallest_subnormal));
    v.push(("E11:subnormal_boundary", errors::err_e11_subnormal_normal_boundary));
    v.push(("E12:tiny_signed_zero", errors::err_e12_tiny_negative_signed_zero));
    v.push(("E13:tie_rounding", errors::err_e13_round_half_even_ties));
    v.push(("E14:raw_pattern_sweep", errors::err_e14_raw_bit_pattern_sweep));
    v.push(("E*:output_shape", errors::err_output_shape_invariant_across_all_classes));

    // C19 mutates the process-wide locale, so it runs last even though it
    // restores the "C" locale itself.
    v.push(("C19:non_c_locale", config::c19_non_c_locale_decimal_point));

    v
}

fn main() {
    // Honour `cargo test -- <substring>` for narrowing down a single row.
    let filters: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .collect();

    let all = cases();
    let selected: Vec<&Case> = all
        .iter()
        .filter(|(n, _)| filters.is_empty() || filters.iter().any(|f| n.contains(f.as_str())))
        .collect();

    let i = common::impls();
    eprintln!("differential: C .so   = {}", i.c_path.display());
    eprintln!("differential: Rust .so = {}", i.rust_path.display());
    eprintln!("differential: running {} rows sequentially\n", selected.len());

    let mut failed: Vec<(&str, String)> = Vec::new();

    // Silence the default panic hook: it would splice its multi-line message
    // into the middle of the "  <row> ... " progress line, mangling the report.
    // `catch_unwind` hands us the same message, which we print in the summary.
    if std::env::var_os("DIFFERENTIAL_PANIC_HOOK").is_none() {
        std::panic::set_hook(Box::new(|_| {}));
    }

    for (name, f) in &selected {
        eprint!("  {name:34} ... ");
        let started = std::time::Instant::now();
        let res = catch_unwind(AssertUnwindSafe(*f));
        let ms = started.elapsed().as_millis();
        match res {
            Ok(()) => eprintln!("ok ({ms} ms)"),
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "<non-string panic>".to_string()
                };
                eprintln!("FAILED ({ms} ms)");
                failed.push((name, msg));
            }
        }
    }

    eprintln!();
    if failed.is_empty() {
        eprintln!(
            "differential result: ok. {} rows passed, 0 failed.",
            selected.len()
        );
    } else {
        eprintln!("differential result: FAILED. {} row(s) diverged:\n", failed.len());
        for (name, msg) in &failed {
            eprintln!("---- {name} ----\n{msg}\n");
        }
        std::process::exit(1);
    }
}
