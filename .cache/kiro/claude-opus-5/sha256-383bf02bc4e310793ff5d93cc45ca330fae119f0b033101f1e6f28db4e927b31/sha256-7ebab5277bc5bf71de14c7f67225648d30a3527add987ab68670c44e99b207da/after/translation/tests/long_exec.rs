//! Differential test for the top-level exported function `long_exec`.
//!
//! A full `long_exec` run is ~470 s for the (unoptimised) C library and ~310 s
//! for the Rust one, so the two sides cannot be exercised inside a single
//! bounded command. Each side is therefore captured by its own `#[ignore]`d
//! test that persists its observable results to disk, and a third test compares
//! the two recordings. `./run_long_exec_difftest.sh` drives all three.
//!
//! Observable results captured per side:
//!   * every byte written to file descriptor 1 (the `printf("%d\n", ...)`)
//!   * the final contents of the exported `array` global (1 MiB)

mod common;

use common::{assert_arrays_equal, load_both, strip_harness_noise, ARRAY_SIZE};
use std::ffi::c_int;
use std::path::PathBuf;

/// Seed passed to `long_exec`; overridable so the script can sweep seeds.
fn seed() -> u32 {
    std::env::var("DIFFTEST_SEED")
        .ok()
        .map(|s| s.parse().expect("DIFFTEST_SEED must be a u32"))
        .unwrap_or(42)
}

fn results_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("difftest-results");
    std::fs::create_dir_all(&dir).expect("create results dir");
    dir
}

fn stdout_path(side: &str, seed: u32) -> PathBuf {
    results_dir().join(format!("long_exec-{side}-{seed}.stdout"))
}

fn array_path(side: &str, seed: u32) -> PathBuf {
    results_dir().join(format!("long_exec-{side}-{seed}.array"))
}

/// Record one side's observable behaviour. `pick` selects which of the two
/// loaded libraries to drive, so only one full run happens per invocation.
fn record(side: &str) {
    let seed = seed();
    let guard = load_both();
    let (c, rust) = &*guard;
    let imp = match side {
        "C" => c,
        "Rust" => rust,
        other => panic!("unknown side {other}"),
    };

    // `long_exec` starts by overwriting the whole array from `rand()`, so the
    // incoming state does not matter; scribble on it anyway to prove that.
    imp.write_array(&vec![-1 as c_int; ARRAY_SIZE]);

    let out = imp.capture_long_exec(seed);
    let final_array = imp.read_array_bytes();

    std::fs::write(stdout_path(side, seed), &out).expect("write stdout recording");
    std::fs::write(array_path(side, seed), &final_array).expect("write array recording");

    let printed = strip_harness_noise(&out);
    eprintln!(
        "{side}: seed={seed} printed={:?} final array {} bytes",
        String::from_utf8_lossy(&printed),
        final_array.len()
    );
    assert!(
        !printed.is_empty(),
        "{side}: long_exec printed nothing — capture plumbing is broken"
    );
}

#[test]
#[ignore = "full-scale run, ~470 s; driven by run_long_exec_difftest.sh"]
fn record_c() {
    record("C");
}

#[test]
#[ignore = "full-scale run, ~310 s; driven by run_long_exec_difftest.sh"]
fn record_rust() {
    record("Rust");
}

#[test]
#[ignore = "compares recordings produced by record_c and record_rust"]
fn compare_recordings() {
    let seed = seed();

    let read = |path: PathBuf| {
        std::fs::read(&path).unwrap_or_else(|e| {
            panic!(
                "missing recording {} ({e}) — run record_c and record_rust first",
                path.display()
            )
        })
    };

    let c_out = strip_harness_noise(&read(stdout_path("C", seed)));
    let rust_out = strip_harness_noise(&read(stdout_path("Rust", seed)));
    assert!(
        !c_out.is_empty(),
        "seed={seed}: the C recording contains no program output"
    );
    assert_eq!(
        c_out,
        rust_out,
        "seed={seed}: stdout differs\n  C    = {:?}\n  Rust = {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out)
    );

    let c_arr = read(array_path("C", seed));
    let rust_arr = read(array_path("Rust", seed));
    let as_ints = |b: &[u8]| -> Vec<c_int> {
        b.chunks_exact(4)
            .map(|c| i32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    };
    assert_arrays_equal(
        &format!("seed={seed}: final `array` after long_exec"),
        &as_ints(&c_arr),
        &as_ints(&rust_arr),
    );

    eprintln!(
        "seed={seed}: long_exec matches — stdout {:?}, 1 MiB array identical",
        String::from_utf8_lossy(&c_out)
    );
}
