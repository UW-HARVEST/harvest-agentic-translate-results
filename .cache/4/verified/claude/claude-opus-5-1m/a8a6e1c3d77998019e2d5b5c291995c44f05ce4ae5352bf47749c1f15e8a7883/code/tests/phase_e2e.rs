// Phase B, row C15 — the real end-to-end `long_exec` one-shot.
//
// `long_exec` runs `ITERATIONS` (2000) full passes over the 1 MiB array, i.e.
// 2000 * 262144 * 100 ~= 5.2e10 arithmetic steps.  That is ~520 s for the C
// `.so` and ~275 s for the release Rust `.so`, so the three tests below are
// `#[ignore]`d and are meant to be run one at a time (each stays well under a
// 600 s budget):
//
//   LONG_E2E_SEED=42 cargo test --release --test phase_e2e -- --ignored --exact e2e_c
//   LONG_E2E_SEED=42 cargo test --release --test phase_e2e -- --ignored --exact e2e_rust
//   LONG_E2E_SEED=42 cargo test --release --test phase_e2e -- --ignored --exact e2e_compare
//
// Each run records the captured stdout bytes *and* the final XOR reduction of
// the exported `array` object into `target/`, and the third test compares them
// byte-for-byte.

mod common;

use common::*;
use std::path::PathBuf;

/// Seed used for the end-to-end run; override with `LONG_E2E_SEED=<u32>` so
/// several seeds can be run as separate (parallel) processes.
fn seed() -> u32 {
    match std::env::var("LONG_E2E_SEED") {
        Ok(s) => s.trim().parse::<u32>().expect("LONG_E2E_SEED must be a u32"),
        Err(_) => 42,
    }
}

fn record_path(who: &str) -> PathBuf {
    manifest_dir()
        .join("target")
        .join(format!("e2e_{who}_seed{}.txt", seed()))
}

/// Full 1 MiB dump of the final `array` state, so the two implementations can be
/// compared byte-for-byte and not only through the XOR checksum.
fn dump_path(who: &str) -> PathBuf {
    manifest_dir()
        .join("target")
        .join(format!("e2e_{who}_seed{}.bin", seed()))
}

/// Runs `long_exec(seed())` on one library, capturing both observation channels
/// (stdout bytes and the final state of the exported `array`).
fn run_one(who: &str, lib: &Lib) {
    let seed = seed();
    // Start from a known state so the run is reproducible.
    lib.zero_array();
    let t0 = std::time::Instant::now();
    let stdout = capture_stdout(who, || lib.long_exec(seed));
    let secs = t0.elapsed().as_secs_f64();
    let xor = lib.xor_array();
    let arr = lib.read_array();
    let record = format!(
        "stdout={:?}\nxor_of_array={}\nfirst4={:?}\nlast4={:?}\n",
        String::from_utf8_lossy(&stdout),
        xor,
        &arr[..4],
        &arr[ARRAY_SIZE - 4..]
    );
    println!(
        "[{who}] seed={seed} {} in {secs:.1}s: {}",
        lib.path.display(),
        record
    );
    std::fs::write(record_path(who), &record).expect("write record");
    std::fs::write(dump_path(who), lib.read_bytes()).expect("write array dump");

    // The printed value must be the XOR of the final array contents.
    let text = String::from_utf8(stdout).expect("printf output is ASCII");
    assert_eq!(text, format!("{xor}\n"), "[{who}] printed line vs array XOR");
}

#[test]
#[ignore = "runs the full 2000-iteration C pipeline (~520 s)"]
fn e2e_c() {
    let h = harness();
    run_one("c", h.c);
}

#[test]
#[ignore = "runs the full 2000-iteration Rust pipeline (~275 s release / ~920 s debug)"]
fn e2e_rust() {
    let h = harness();
    run_one("rust", h.rust);
}

#[test]
#[ignore = "compares the recordings produced by e2e_c and e2e_rust"]
fn e2e_compare() {
    let c = std::fs::read(record_path("c")).expect("run e2e_c first");
    let r = std::fs::read(record_path("rust")).expect("run e2e_rust first");
    println!("C   : {}", String::from_utf8_lossy(&c));
    println!("Rust: {}", String::from_utf8_lossy(&r));

    // Byte-for-byte comparison of the whole final array, when both dumps exist.
    match (std::fs::read(dump_path("c")), std::fs::read(dump_path("rust"))) {
        (Ok(cd), Ok(rd)) => {
            assert_eq!(cd.len(), ARRAY_SIZE * 4, "C dump size");
            assert_eq!(rd.len(), ARRAY_SIZE * 4, "Rust dump size");
            let diffs = cd
                .chunks(4)
                .zip(rd.chunks(4))
                .filter(|(a, b)| a != b)
                .count();
            assert_eq!(
                diffs, 0,
                "{diffs} of {ARRAY_SIZE} elements of the final array differ"
            );
            println!("full 1 MiB final array is byte-identical");
        }
        _ => println!("(no full array dumps found — comparing the text records only)"),
    }
    assert_eq!(
        c,
        r,
        "end-to-end long_exec({}) recordings differ",
        seed()
    );
}
