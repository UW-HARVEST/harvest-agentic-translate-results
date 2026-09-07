//! C-vs-Rust differential test runner.
//!
//! This target uses `harness = false` on purpose. `driver`'s only observable
//! output is what it writes to the process's libc `stdout`, so every comparison
//! has to redirect file descriptor 1 — which is process-global. Under libtest's
//! default thread pool, libtest's own progress lines and other tests' output
//! land inside a capture and corrupt the comparison. A custom sequential runner
//! owns fd 1 for the whole run, so captures are clean.
//!
//! Both libraries are always driven through `libloading` + their exported
//! `extern "C"` symbols; the Rust crate is never called directly.

#[path = "cases/common/mod.rs"]
mod common;

#[path = "cases/phase_b_configs.rs"]
mod phase_b_configs;

#[path = "cases/phase_c_errors.rs"]
mod phase_c_errors;

#[path = "cases/phase_d_symbols.rs"]
mod phase_d_symbols;

use std::io::Write;
use std::panic::{catch_unwind, AssertUnwindSafe};

struct Runner {
    filter: Option<String>,
    passed: usize,
    failed: Vec<(&'static str, String)>,
    skipped: usize,
}

impl Runner {
    fn run(&mut self, phase: &str, name: &'static str, f: impl FnOnce()) {
        if let Some(filter) = &self.filter {
            if !name.contains(filter.as_str()) {
                self.skipped += 1;
                return;
            }
        }
        // Write progress to stderr: fd 1 belongs to the captures.
        eprint!("test {phase}::{name} ... ");
        let _ = std::io::stderr().flush();
        let result = catch_unwind(AssertUnwindSafe(f));
        match result {
            Ok(()) => {
                eprintln!("ok");
                self.passed += 1;
            }
            Err(payload) => {
                let msg = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "<non-string panic payload>".to_string());
                eprintln!("FAILED");
                self.failed.push((name, msg));
            }
        }
    }
}

fn main() {
    // Silence the default panic printer; the runner reports failures itself.
    let quiet = std::env::var_os("DIFF_TEST_VERBOSE").is_none();
    if quiet {
        std::panic::set_hook(Box::new(|_| {}));
    }

    let filter = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .filter(|a| !a.is_empty());

    let mut r = Runner {
        filter,
        passed: 0,
        failed: Vec::new(),
        skipped: 0,
    };

    eprintln!(
        "\nC   .so: {}\nRust.so: {}\n",
        common::c_so_path().display(),
        common::rust_so_path().display()
    );

    // ---- Phase D: symbol parity (run first — it gates everything else) ----
    eprintln!("--- Phase D: symbol parity ---");
    r.run(
        "phase_d",
        "symbols_exported_by_c_are_all_exported_by_rust",
        phase_d_symbols::symbols_exported_by_c_are_all_exported_by_rust,
    );
    r.run(
        "phase_d",
        "rust_exports_nothing_extra",
        phase_d_symbols::rust_exports_nothing_extra,
    );
    r.run(
        "phase_d",
        "rust_has_no_undefined_non_libc_symbols",
        phase_d_symbols::rust_has_no_undefined_non_libc_symbols,
    );
    r.run(
        "phase_d",
        "c_source_inventory_is_fully_covered",
        phase_d_symbols::c_source_inventory_is_fully_covered,
    );

    // ---- Phase B: CONFIGS.md rows ----
    eprintln!("--- Phase B: CONFIGS.md valid-path rows ---");
    macro_rules! b {
        ($($name:ident),* $(,)?) => { $( r.run("phase_b", stringify!($name), phase_b_configs::$name); )* };
    }
    b!(
        cfg_row01_zero,
        cfg_row02_all_ones,
        cfg_row03_int_max,
        cfg_row04_int_min,
        cfg_row05_single_low_nibble_byte_each_position,
        cfg_row06_single_midrange_byte_each_position,
        cfg_row07_single_high_bit_byte_each_position,
        cfg_row08_exhaustive_byte_per_lane,
        cfg_row09_every_single_bit,
        cfg_row10_random_negative,
        cfg_row11_random_positive,
        cfg_row12_random_full_range,
        cfg_row13_all_bytes_low_nibble,
        cfg_row14_all_bytes_high_bit,
        cfg_row15_all_byte_permutations,
        cfg_row16_single_call_exact_framing,
        cfg_row17_many_calls_one_stream,
        cfg_row18_interleaved_c_and_rust_same_stdout,
        cfg_row19_full_lifecycle_reload,
    );

    // ---- Phase C: ERRORS.md rows ----
    eprintln!("--- Phase C: ERRORS.md error-path rows ---");
    macro_rules! c {
        ($($name:ident),* $(,)?) => { $( r.run("phase_c", stringify!($name), phase_c_errors::$name); )* };
    }
    c!(
        err_rows1_2_internal_print_hex_not_reachable,
        err_row3_int_min,
        err_row4_int_max,
        err_row5_minus_one,
        err_row6_zero,
        err_row7_oversized_arg_truncation,
        err_row8_one_past_unsigned_range,
        err_row9_no_hidden_state,
        err_generic_alternate_call_signatures_agree,
        err_generic_no_extra_exports,
    );

    eprintln!(
        "\nresult: {} passed; {} failed; {} filtered out",
        r.passed,
        r.failed.len(),
        r.skipped
    );
    if !r.failed.is_empty() {
        eprintln!("\nfailures:");
        for (name, msg) in &r.failed {
            eprintln!("---- {name} ----\n{msg}\n");
        }
        std::process::exit(1);
    }
}
