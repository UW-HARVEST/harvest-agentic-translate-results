// CONFIGS.md rows 17 & 18 / ERRORS.md rows 1-2, 17-18: the genuinely full
// `long_exec` differential -- srand -> 262144 rand() -> ITERATIONS (2000)
// worker calls -> xor fold -> printf("%d\n"), with stdout captured at the file
// descriptor level and compared byte-for-byte.
//
// This is expensive: ~470 s for the C `.so` and ~56 s for the optimised Rust
// `.so` per seed (5.24e10 `step()` evaluations each). The tests are therefore
// `#[ignore]`d and driven by `run_full_long_exec.sh`, which runs each library
// in its own background process so no single command exceeds the time budget.
//
// Select which library to run with LONG_ONLY=c | rust, and which seed with
// LONG_SEED=<u32>. With neither set, both are run in-process (slow).

mod common;

use common::*;

fn seed_from_env() -> u32 {
    std::env::var("LONG_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

/// Run `long_exec` on one library and print `<name> <seed> <stdout-bytes>` in a
/// form `run_full_long_exec.sh` can diff across processes.
fn run_one(which: &str, seed: u32) -> String {
    let h = harness();
    let target = if which == "c" {
        &h.c
    } else {
        h.rust
            .iter()
            .find(|t| t.name.contains("release"))
            .or_else(|| h.rust.first())
            .expect("no Rust target")
    };
    let raw = capture_stdout(|| target.long_exec(seed));
    println!("RAW target={} seed={seed} stdout={:?}", target.name, raw);
    let out = extract_printf_line(&raw);
    println!("RESULT target={} seed={seed} value={:?}", target.name, out);
    out
}

/// Pull `long_exec`'s `printf("%d\n", …)` line out of the captured fd-1 text.
///
/// A run longer than 60 s makes libtest's progress reporter ("test ... has been
/// running for over 60 seconds") write to fd 1 as well, which lands in the same
/// capture. That line is the test framework's, not the library's, so it is
/// stripped -- but the result must still be EXACTLY ONE integer line, which is
/// what `printf("%d\n", xor_result)` is required to produce.
fn extract_printf_line(raw: &str) -> String {
    let (ints, others): (Vec<&str>, Vec<&str>) = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .partition(|l| l.trim().parse::<i32>().is_ok());
    for o in &others {
        assert!(
            o.contains("has been running for over"),
            "unexpected extra output on stdout: {o:?} (full capture {raw:?})"
        );
    }
    assert_eq!(
        ints.len(),
        1,
        "long_exec must print exactly one `%d` line; capture was {raw:?}"
    );
    // And it must have been newline-terminated, as `printf("%d\n")` requires.
    assert!(
        raw.ends_with('\n'),
        "long_exec's output is not newline terminated: {raw:?}"
    );
    format!("{}\n", ints[0].trim())
}

/// Runs whichever half of the comparison `LONG_ONLY` selects, writing the
/// captured stdout to `target/full_run_<which>_<seed>.txt` for cross-process
/// diffing by the driver script.
#[test]
#[ignore = "expensive: 5.24e10 step() evaluations (~470 s for C, ~56 s for Rust)"]
fn full_long_exec_single_library() {
    let which = std::env::var("LONG_ONLY").unwrap_or_else(|_| "c".to_string());
    let seed = seed_from_env();
    let out = run_one(&which, seed);
    let path = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/target"))
        .join(format!("full_run_{which}_{seed}.txt"));
    std::fs::write(&path, out.as_bytes()).expect("cannot write result file");
    eprintln!("wrote {}", path.display());
}

/// In-process version: runs BOTH libraries back to back and compares their
/// captured stdout directly. ~530 s per seed, so it is `#[ignore]`d as well.
#[test]
#[ignore = "expensive: runs both libraries in one process (~9 minutes per seed)"]
fn full_long_exec_differential() {
    let h = harness();
    let seed = seed_from_env();

    let c_out = extract_printf_line(&capture_stdout(|| h.c.long_exec(seed)));
    eprintln!("C   seed={seed} -> {c_out:?}");

    for t in &h.rust {
        let r_out = extract_printf_line(&capture_stdout(|| t.long_exec(seed)));
        eprintln!("{:<13} seed={seed} -> {r_out:?}", t.name);
        assert_eq!(
            c_out, r_out,
            "[{}] long_exec(seed={seed}) printed {r_out:?} but C printed {c_out:?}",
            t.name
        );
    }

    // CONFIGS.md row 18 / ERRORS.md row 17: re-entry after the previous run
    // left `array` dirty must reproduce the identical output, because
    // `long_exec` re-seeds the whole array first.
    let c_again = extract_printf_line(&capture_stdout(|| h.c.long_exec(seed)));
    assert_eq!(
        c_out, c_again,
        "C long_exec is not idempotent across calls; row 17 premise is wrong"
    );
}
