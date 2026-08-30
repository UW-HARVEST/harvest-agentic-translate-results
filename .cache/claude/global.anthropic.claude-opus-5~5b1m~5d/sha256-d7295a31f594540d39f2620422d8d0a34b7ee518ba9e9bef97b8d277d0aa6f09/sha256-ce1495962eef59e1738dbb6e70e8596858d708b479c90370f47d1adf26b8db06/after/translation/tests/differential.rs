// Differential test runner: C `.so` vs Rust `.so`, both loaded with
// `libloading` and called only through their exported symbols.
//
// A custom harness (`harness = false`) is used instead of libtest because
// `driver` reports its results by printing to stdout. Capturing that requires
// redirecting the process-global file descriptor 1, which is only sound if
//
//   * exactly one test runs at a time (libtest would run them in parallel),
//   * nothing else writes to fd 1 while a capture is in flight (libtest's own
//     "test foo ... ok" progress output does).
//
// This runner therefore executes cases strictly sequentially and writes all of
// its own diagnostics to *stderr*, leaving fd 1 exclusively to the libraries
// under test.
//
// Case selection: pass substrings as arguments, e.g.
//   cargo test --test differential -- c9 e3
// Pass `--list` to print the case names.

mod common;
mod parts;

use std::panic::{self, AssertUnwindSafe};

pub type Case = (&'static str, fn());

fn all_cases() -> Vec<Case> {
    let mut v: Vec<Case> = Vec::new();
    // `first_call` MUST come first: it asserts behaviour of the very first
    // `driver` call each library sees after `dlopen`.
    v.extend(parts::first_call::cases());
    v.extend(parts::phase_b::cases());
    v.extend(parts::phase_c::cases());
    v.extend(parts::phase_d::cases());
    v
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Ignore libtest-style flags that cargo/CI may pass through.
    let filters: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| !a.starts_with("--"))
        .collect();
    let list_only = args.iter().any(|a| a == "--list");

    let cases = all_cases();

    if list_only {
        for (name, _) in &cases {
            eprintln!("{name}");
        }
        return;
    }

    // Silence Rust's own panic banner; failures are reported by this runner.
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    let mut passed = 0usize;
    let mut failed: Vec<(&str, String)> = Vec::new();
    let mut skipped = 0usize;

    eprintln!("\nrunning {} differential cases (sequential)\n", cases.len());

    for (name, f) in &cases {
        if !filters.is_empty() && !filters.iter().any(|needle| name.contains(needle)) {
            skipped += 1;
            continue;
        }
        eprint!("  {name} ... ");
        let started = std::time::Instant::now();
        let result = panic::catch_unwind(AssertUnwindSafe(f));
        let elapsed = started.elapsed();
        match result {
            Ok(()) => {
                passed += 1;
                eprintln!("ok ({:.2?})", elapsed);
            }
            Err(payload) => {
                let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "<non-string panic payload>".to_string()
                };
                eprintln!("FAILED");
                failed.push((name, msg));
            }
        }
    }

    panic::set_hook(default_hook);

    if !failed.is_empty() {
        eprintln!("\nfailures:\n");
        for (name, msg) in &failed {
            eprintln!("---- {name} ----\n{msg}\n");
        }
    }

    eprintln!(
        "\nresult: {}. {} passed; {} failed; {} filtered out\n",
        if failed.is_empty() { "ok" } else { "FAILED" },
        passed,
        failed.len(),
        skipped
    );

    if !failed.is_empty() {
        std::process::exit(1);
    }
}
