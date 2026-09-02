//! Phase B — valid-path differential tests, rows 21..=34 of `CONFIGS.md`:
//! the full `long_exec` pipeline.
//!
//! A single C `long_exec` performs 2000 * 100 * 262144 ~= 5.2e10 kernel
//! applications, which is ~470 s of CPU with the `-O0` build that
//! `c_src/CMakeLists.txt` configures (no `CMAKE_BUILD_TYPE`, hence no `-O`
//! flag).  Running 21 of them inside one `cargo test` invocation would take
//! about three hours, so the C side is produced **once**, out of process, by
//! `tools/gen_reference.sh` (which dlopens nothing but the C `.so`) and cached
//! byte-for-byte under `tests/reference/`:
//!
//!   * `c.exec.<seed>.out` — the exact bytes `long_exec` wrote to stdout
//!   * `c.exec.<seed>.bin` — the exact 1 MiB final contents of `array`
//!
//! The tests here load the **Rust** `.so` with `libloading`, call its exported
//! `long_exec`, capture its stdout bytes with `dup2`, and require both the
//! stdout bytes and the whole final array to be identical to the C reference.
//! `long_exec_live_c` (`#[ignore]`d, ~8 min) re-derives one row from the C `.so`
//! in-process so the cache can be re-validated on demand.

mod harness;

use harness::{assert_arrays_eq, rand_fill, read_reference_stdout, ARRAY_SIZE};

/// Seeds whose C reference includes the full 1 MiB array dump
/// (`c.exec.<seed>.bin`), compared element-for-element.
const SEEDS: &[u32] = &[
    0,          // row 21 - glibc srand(0) aliases srand(1)
    1,          // row 22
    2,          // row 23
    3,          // row 24
    7,          // row 25
    42,         // row 26
    12345,      // row 27
    999983,     // row 28
    2147483648, // row 29 - 2^31, sign bit set
    4294967295, // row 30 - UINT_MAX
    5,
    100,
    777,
    31337,
    65535,
    123456789,
    2000000000,
    4000000000,
];

/// `CONFIGS.md` row 30b — 24 further seeds.  For these the C reference is the
/// exact stdout bytes plus an FNV-1a fingerprint of the 1 MiB array
/// (`c.exec.<seed>.hash`, produced by `tools/runner.c`'s `hash` op) instead of a
/// full dump, purely to keep 42 MiB of binary fixtures out of the crate.  Both
/// were also compared as full 1 MiB dumps out of process with `cmp` when the
/// reference was generated.
const SEEDS_HASHED: &[u32] = &[
    4, 6, 8, 9, 10, 11, 13, 17, 19, 23, 29, 97, 128, 255, 256, 1000, 4096, 54321, 88888888,
    1000003, 16777216, 2147483647, 3000000000, 4294967294,
];

fn reference_array(row: &str, file: &str) -> Vec<i32> {
    let path = harness::reference_dir().join(file);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "[{row}] missing C reference {}: {e}\n\
             regenerate with: tools/gen_reference.sh  (~8 min wall, runs in parallel)",
            path.display()
        )
    });
    assert_eq!(bytes.len(), ARRAY_SIZE * 4, "[{row}] bad reference size");
    bytes
        .chunks_exact(4)
        .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn check_seed(seed: u32) {
    let row = format!("long_exec(seed={seed})");
    let expect_out = read_reference_stdout(&row, &format!("c.exec.{seed}.out"));
    let expect_arr = reference_array(&row, &format!("c.exec.{seed}.bin"));

    let _g = harness::lock();
    let rl = harness::rust();
    let got_out = rl.long_exec_capture(seed);

    assert_eq!(
        got_out,
        expect_out,
        "[{row}] stdout bytes differ\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&expect_out),
        String::from_utf8_lossy(&got_out)
    );
    assert_arrays_eq(&row, 2000, &expect_arr, &expect_arr, rl.array());

    // The printed value must be the XOR of the final array, per the C source.
    let xor = rl.array().iter().fold(0i32, |a, &b| a ^ b);
    let printed = String::from_utf8_lossy(&got_out).trim().parse::<i32>().unwrap();
    assert_eq!(printed, xor, "[{row}] printed value is not the array XOR");

    // The hash fixture must agree with the dump fixture (cross-check of the
    // two reference formats).
    let hash_file = harness::reference_dir().join(format!("c.exec.{seed}.hash"));
    if let Ok(txt) = std::fs::read_to_string(&hash_file) {
        assert_eq!(
            txt.trim(),
            format!("{:016x}", harness::fnv1a(&expect_arr)),
            "[{row}] .hash and .bin fixtures disagree"
        );
    }
}

/// Same check, but the C array reference is the FNV-1a fingerprint of the exact
/// 1 MiB image rather than the image itself.
fn check_seed_hashed(seed: u32) {
    let row = format!("long_exec(seed={seed}) [hashed reference]");
    let expect_out = read_reference_stdout(&row, &format!("c.exec.{seed}.out"));
    let expect_hash = String::from_utf8(read_reference_stdout(
        &row,
        &format!("c.exec.{seed}.hash"),
    ))
    .unwrap()
    .trim()
    .to_owned();

    let _g = harness::lock();
    let rl = harness::rust();
    let got_out = rl.long_exec_capture(seed);
    assert_eq!(
        got_out,
        expect_out,
        "[{row}] stdout bytes differ\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&expect_out),
        String::from_utf8_lossy(&got_out)
    );
    let got_hash = format!("{:016x}", harness::fnv1a(rl.array()));
    assert_eq!(got_hash, expect_hash, "[{row}] final array fingerprint differs");
}

#[test]
fn rows21_30_and_extra_seeds() {
    for &s in SEEDS {
        check_seed(s);
    }
}

/// `CONFIGS.md` row 30b — 24 further seeds against hashed references.
#[test]
fn row30b_additional_seeds() {
    for &s in SEEDS_HASHED {
        check_seed_hashed(s);
    }
}

/// `ERRORS.md` row 4: a negative `int` handed to the `unsigned int` parameter
/// must be reinterpreted, not sign-extended, so `-1` is exactly `UINT_MAX`.
#[test]
fn negative_seed_bit_pattern_aliases_uint_max() {
    let row = "long_exec(seed = -1 as u32)";
    let expect_out = read_reference_stdout(row, "c.exec.4294967295.out");
    let expect_arr = reference_array(row, "c.exec.4294967295.bin");
    let _g = harness::lock();
    let rl = harness::rust();
    let got = rl.long_exec_capture((-1i32) as u32);
    assert_eq!(got, expect_out, "[{row}] stdout differs");
    assert_arrays_eq(row, 2000, &expect_arr, &expect_arr, rl.array());
}

/// `CONFIGS.md` row 32: `long_exec(42)` then one `perform_expensive_operations`.
/// The second op must consume the post-`long_exec` array state.
#[test]
fn row32_exec_then_pxo() {
    let row = "row 32: exec(42) -> pxo(1)";
    let expect_out = read_reference_stdout(row, "c.row32.out");
    let expect_arr = reference_array(row, "c.row32.bin");
    let _g = harness::lock();
    let rl = harness::rust();
    let got = harness::capture_stdout(|| {
        rl.long_exec(42);
        rl.pxo(1);
    });
    assert_eq!(got, expect_out, "[{row}] stdout differs");
    assert_arrays_eq(row, 2000, &expect_arr, &expect_arr, rl.array());
}

/// `CONFIGS.md` row 33: `long_exec` is idempotent in the seed — calling it twice
/// with 42 then once with 7 must print three lines and leave the seed-7 image.
#[test]
fn row33_exec_repeated_and_reseeded() {
    let row = "row 33: exec(42) -> exec(42) -> exec(7)";
    let expect_out = read_reference_stdout(row, "c.row33.out");
    let expect_arr = reference_array(row, "c.row33.bin");
    let _g = harness::lock();
    let rl = harness::rust();
    let got = harness::capture_stdout(|| {
        rl.long_exec(42);
        rl.long_exec(42);
        rl.long_exec(7);
    });
    assert_eq!(
        got,
        expect_out,
        "[{row}] stdout differs\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&expect_out),
        String::from_utf8_lossy(&got)
    );
    assert_arrays_eq(row, 2000, &expect_arr, &expect_arr, rl.array());
}

/// `CONFIGS.md` row 34: a dirty array before `long_exec` must be fully
/// discarded by the `rand()` fill.
#[test]
fn row34_dirty_array_then_exec() {
    let row = "row 34: fill(rand 99) -> pxo(1) -> exec(42)";
    let expect_out = read_reference_stdout(row, "c.row34.out");
    let expect_arr = reference_array(row, "c.row34.bin");
    let input = rand_fill(99);
    let _g = harness::lock();
    let rl = harness::rust();
    rl.array_mut().copy_from_slice(&input);
    let got = harness::capture_stdout(|| {
        rl.pxo(1);
        rl.long_exec(42);
    });
    assert_eq!(got, expect_out, "[{row}] stdout differs");
    assert_arrays_eq(row, 2000, &expect_arr, &expect_arr, rl.array());
}

/// Stream parity, both fds.  The C library writes only to stdout and never a
/// byte to stderr (verified: every `tools/gen_reference.sh` capture has a
/// zero-length stderr file).  In its **default** configuration the Rust library
/// must match that exactly.  The opt-in `debug-stats` feature deliberately adds
/// stderr diagnostics — that is what the feature is for — so the expectation
/// flips, and stdout plus the final array stay byte-identical either way.
#[test]
fn stderr_parity() {
    let _g = harness::lock();
    let cl = harness::c();
    let rl = harness::rust();

    // Cheap on the C side: `perform_expensive_operations` is ~0.24 s.
    let c_err = harness::capture_stderr(|| cl.pxo(1));
    assert!(
        c_err.is_empty(),
        "the C library wrote to stderr: {:?}",
        String::from_utf8_lossy(&c_err)
    );
    let r_err = harness::capture_stderr(|| rl.pxo(1));
    assert!(
        r_err.is_empty(),
        "Rust perform_expensive_operations wrote to stderr: {:?}",
        String::from_utf8_lossy(&r_err)
    );

    let mut exec_err = Vec::new();
    // discard the library's stdout while we look at fd 2
    let _ = harness::capture_stdout(|| exec_err = rl.long_exec_capture_stderr(42));
    #[cfg(not(feature = "debug-stats"))]
    assert!(
        exec_err.is_empty(),
        "default-feature Rust long_exec wrote to stderr, the C never does: {:?}",
        String::from_utf8_lossy(&exec_err)
    );
    #[cfg(feature = "debug-stats")]
    assert!(
        exec_err.starts_with(b"DBG "),
        "the debug-stats feature should emit its diagnostics on stderr, got {:?}",
        String::from_utf8_lossy(&exec_err)
    );

    // Either way stdout and the final array are unchanged by the feature.
    let out = rl.long_exec_capture(42);
    assert_eq!(
        out,
        read_reference_stdout("stderr parity", "c.exec.42.out"),
        "stdout diverged from the C reference"
    );
    let expect = reference_array("stderr parity", "c.exec.42.bin");
    assert_arrays_eq("stderr parity", 2000, &expect, &expect, rl.array());
}

/// `CONFIGS.md` row 35 — the accelerated-path cross-check.
///
/// The Rust `long_exec` does **not** run the nested loop: `src/fast.rs` computes
/// `f^200000` by exact function-iteration algebra.  This test reaches the naive
/// nested loop through the FFI instead (`srand(seed)` + `rand()` fill, then 2000
/// `perform_expensive_operations()` calls) and requires the two to agree
/// element-for-element.  Combined with `tools/sweep.sh` — which proves the Rust
/// `f^100` equals the C `f^100` for **all 2^32** inputs — this pins the
/// accelerated path to the C semantics independently of the end-to-end seeds.
///
/// ~5 min per seed in release, hence `#[ignore]`d.  Also run out of process for
/// several seeds via `tools/runner.c fill:libcrand:S pxo:2000 hash`.
#[test]
#[ignore = "naive f^200000 over 262144 elements: ~5 min per seed"]
fn accelerated_equals_naive_through_ffi() {
    let seed = 42u32;
    let _g = harness::lock();
    let rl = harness::rust();

    // Reproduce long_exec's own fill using the same libc rand() the library uses.
    let fill: Vec<i32> = unsafe {
        libc::srand(seed);
        (0..ARRAY_SIZE).map(|_| libc::rand() as i32).collect()
    };

    // Naive route: 2000 * f^100 through the exported low-level entry point.
    rl.array_mut().copy_from_slice(&fill);
    rl.pxo(2000);
    let naive = rl.array().to_vec();

    // Accelerated route.
    rl.long_exec_capture(seed);
    let fast = rl.array().to_vec();

    assert_arrays_eq("row 35: accelerated vs naive", 2000, &fill, &naive, &fast);

    // ... and both must equal the cached C reference.
    let expect = reference_array("row 35", &format!("c.exec.{seed}.bin"));
    assert_arrays_eq("row 35: naive vs C", 2000, &fill, &expect, &naive);
}

/// Re-derive one cached row live from the C `.so` in-process.  ~8 min, so it is
/// `#[ignore]`d; run with `cargo test --release -- --ignored --nocapture`.
#[test]
#[ignore = "runs the real C long_exec: ~470 s of CPU"]
fn long_exec_live_c() {
    let seed = 42u32;
    let _g = harness::lock();
    let cl = harness::c();
    let rl = harness::rust();
    let c_out = cl.long_exec_capture(seed);
    let c_arr = cl.array().to_vec();
    let r_out = rl.long_exec_capture(seed);
    assert_eq!(c_out, r_out, "live stdout differs");
    assert_arrays_eq("live exec(42)", 2000, &c_arr, &c_arr, rl.array());

    // ... and the cached reference must agree with what we just measured.
    assert_eq!(
        c_out,
        read_reference_stdout("live", "c.exec.42.out"),
        "cached reference stdout is stale"
    );
    assert_eq!(
        c_arr,
        reference_array("live", "c.exec.42.bin"),
        "cached reference array is stale"
    );
}
