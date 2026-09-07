// Phase B — valid-path differential tests, GATED on CONFIGS.md.
//
// One test per CONFIGS.md row (C1..C20). Every test drives BOTH the C `.so` and
// the Rust `.so` through their exported `driver` symbol and compares the stdout
// bytes exactly. Randomized rows use a fixed seed so failures are reproducible.

mod common;

use common::*;

const SEED: u64 = 0x5EED_0000_D1FF_2025;

/// How many randomized inputs each property-style row uses.
const N: usize = 400;

// ---------------------------------------------------------------------------
// C1..C8 — the pruned cross product of the three branch axes (A1 x A2 x A3).
// ---------------------------------------------------------------------------

#[test]
fn c1_all_valid_success_path() {
    // x==1, y==2, z==3 : the only path that reaches `printf("Ok!\n")`.
    assert_same_and_eq(1, 2, 3, OK);

    // Repeat to be sure it is not first-call-only.
    for _ in 0..8 {
        assert_same_and_eq(1, 2, 3, OK);
    }
}

#[test]
fn c2_x_ok_y_ok_z_bad() {
    let mut rng = Rng::new(SEED ^ 2);
    // Hand-picked neighbours first, then randomized `z != 3`.
    for z in [2, 4, 0, -3, i32::MIN, i32::MAX] {
        assert_same_and_eq(1, 2, z, ERR_Z);
    }
    for _ in 0..N {
        let z = rng.i32_not(3);
        assert_same_and_eq(1, 2, z, ERR_Z);
    }
}

#[test]
fn c3_x_ok_y_bad_z_ok() {
    let mut rng = Rng::new(SEED ^ 3);
    for y in [1, 3, 0, -2, 123, i32::MIN, i32::MAX] {
        assert_same_and_eq(1, y, 3, ERR_Y);
    }
    for _ in 0..N {
        let y = rng.i32_not(2);
        assert_same_and_eq(1, y, 3, ERR_Y);
    }
}

#[test]
fn c4_x_ok_y_bad_z_bad_short_circuits_on_y() {
    // A4: the `z` check must be skipped; only the y-error line may appear.
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..N {
        let y = rng.i32_not(2);
        let z = rng.i32_not(3);
        assert_same_and_eq(1, y, z, ERR_Y);
    }
}

#[test]
fn c5_x_bad_y_ok_z_ok() {
    let mut rng = Rng::new(SEED ^ 5);
    for x in [0, 2, -1, 123, i32::MIN, i32::MAX] {
        assert_same_and_eq(x, 2, 3, ERR_X);
    }
    for _ in 0..N {
        let x = rng.i32_not(1);
        assert_same_and_eq(x, 2, 3, ERR_X);
    }
}

#[test]
fn c6_x_bad_y_ok_z_bad_short_circuits_on_x() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..N {
        let x = rng.i32_not(1);
        let z = rng.i32_not(3);
        assert_same_and_eq(x, 2, z, ERR_X);
    }
}

#[test]
fn c7_x_bad_y_bad_z_ok_short_circuits_on_x() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..N {
        let x = rng.i32_not(1);
        let y = rng.i32_not(2);
        assert_same_and_eq(x, y, 3, ERR_X);
    }
}

#[test]
fn c8_all_three_bad_reports_only_x() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..N {
        let x = rng.i32_not(1);
        let y = rng.i32_not(2);
        let z = rng.i32_not(3);
        assert_same_and_eq(x, y, z, ERR_X);
    }
}

// ---------------------------------------------------------------------------
// C9..C11 — per-parameter boundary sweeps with the other two valid.
// ---------------------------------------------------------------------------

#[test]
fn c9_x_boundary_sweep() {
    for x in [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        2,
        i32::MAX - 1,
        i32::MAX,
    ] {
        assert_same_and_eq(x, 2, 3, expected(x, 2, 3));
    }
}

#[test]
fn c10_y_boundary_sweep() {
    for y in [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        2,
        3,
        123, // the C static's (dead) initialiser
        i32::MAX - 1,
        i32::MAX,
    ] {
        assert_same_and_eq(1, y, 3, expected(1, y, 3));
    }
}

#[test]
fn c11_z_boundary_sweep() {
    for z in [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        2,
        3,
        4,
        i32::MAX - 1,
        i32::MAX,
    ] {
        assert_same_and_eq(1, 2, z, expected(1, 2, z));
    }
}

// ---------------------------------------------------------------------------
// C12 — exhaustive cross product over the special/boundary value set.
// ---------------------------------------------------------------------------

#[test]
fn c12_exhaustive_special_value_cross_product() {
    // 12^3 = 1728 configurations. Batched into one capture per `x` so the test
    // stays fast while still comparing byte-for-byte.
    for &x in SPECIAL_VALUES.iter() {
        let mut calls = Vec::new();
        let mut want = String::new();
        for &y in SPECIAL_VALUES.iter() {
            for &z in SPECIAL_VALUES.iter() {
                calls.push((x, y, z));
                want.push_str(expected(x, y, z));
            }
        }
        let got = assert_same_seq(&calls);
        assert_eq!(
            got,
            want.as_bytes(),
            "C output for the x={x} slice does not match the model derived from driver.c"
        );
    }
}

// ---------------------------------------------------------------------------
// C13/C14 — randomized property rows.
// ---------------------------------------------------------------------------

#[test]
fn c13_uniform_random_triples_full_i32_range() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..(N * 3) {
        let (x, y, z) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
        assert_same_and_eq(x, y, z, expected(x, y, z));
    }
}

#[test]
fn c14_biased_random_triples_reach_deep_stages() {
    let mut rng = Rng::new(SEED ^ 14);
    let mut hit_ok = 0usize;
    let mut hit_z = 0usize;
    let mut hit_y = 0usize;
    let mut hit_x = 0usize;

    for _ in 0..(N * 4) {
        let (x, y, z) = (
            rng.biased_i32_favouring(1),
            rng.biased_i32_favouring(2),
            rng.biased_i32_favouring(3),
        );
        assert_same_and_eq(x, y, z, expected(x, y, z));
        match expected_code(x, y, z) {
            0 => hit_ok += 1,
            3 => hit_z += 1,
            2 => hit_y += 1,
            _ => hit_x += 1,
        }
    }

    // Coverage guard: the bias must actually reach every branch, otherwise this
    // row would silently degenerate into "x != 1" only.
    assert!(hit_ok > 0, "success path never exercised");
    assert!(hit_z > 0, "z-rejection never exercised");
    assert!(hit_y > 0, "y-rejection never exercised");
    assert!(hit_x > 0, "x-rejection never exercised");
}

// ---------------------------------------------------------------------------
// C15..C19 — the global-state axis (A6): the file-scope `static int y`.
// ---------------------------------------------------------------------------

#[test]
fn c15_repeated_success_calls_are_idempotent() {
    let calls = [(1, 2, 3), (1, 2, 3), (1, 2, 3)];
    let got = assert_same_seq(&calls);
    assert_eq!(got, OK.repeat(3).as_bytes());
}

#[test]
fn c16_failure_then_success_static_is_overwritten() {
    let calls = [(9, 9, 9), (1, 2, 3)];
    let got = assert_same_seq(&calls);
    assert_eq!(got, format!("{ERR_X}{OK}").as_bytes());

    // Also the case where the *y* stage is what poisoned the static.
    let calls = [(1, 7, 3), (1, 2, 3)];
    let got = assert_same_seq(&calls);
    assert_eq!(got, format!("{ERR_Y}{OK}").as_bytes());
}

#[test]
fn c17_success_then_failure_no_state_leak() {
    let calls = [(1, 2, 3), (1, 7, 3), (1, 2, 3), (1, 2, 8)];
    let got = assert_same_seq(&calls);
    assert_eq!(got, format!("{OK}{ERR_Y}{OK}{ERR_Z}").as_bytes());
}

#[test]
fn c18_long_random_call_sequence_in_one_process() {
    let mut rng = Rng::new(SEED ^ 18);
    let mut calls = Vec::with_capacity(600);
    let mut want = String::new();
    for _ in 0..600 {
        let (x, y, z) = (
            rng.biased_i32_favouring(1),
            rng.biased_i32_favouring(2),
            rng.biased_i32_favouring(3),
        );
        calls.push((x, y, z));
        want.push_str(expected(x, y, z));
    }
    let got = assert_same_seq(&calls);
    assert_eq!(got, want.as_bytes(), "C sequence output diverges from the driver.c model");
}

#[test]
fn c19_first_call_after_dlopen_in_fresh_process() {
    // A separate process per case, so the very first `driver` call really is
    // the first write to the file-scope static. Verifies the `= 123`
    // initialiser is unobservable in C and in Rust alike.
    for args in [[1, 2, 3], [0, 0, 0], [1, 123, 3], [1, 2, 0]] {
        let c = run_first_call_subprocess(false, args);
        let r = run_first_call_subprocess(true, args);
        assert_eq!(
            c, r,
            "fresh-process first-call divergence for driver({}, {}, {})",
            args[0], args[1], args[2]
        );
        assert_eq!(c, expected(args[0], args[1], args[2]).as_bytes());
    }
}

/// Spawns this same test binary in "one-shot" mode (see `main`-less harness
/// trick below) via the dedicated `single_call` integration test binary.
fn run_first_call_subprocess(rust: bool, args: [i32; 3]) -> Vec<u8> {
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .arg("--exact")
        .arg("c19_oneshot_worker")
        .arg("--nocapture")
        .env("DRIVER_ONESHOT", if rust { "rust" } else { "c" })
        .env("DRIVER_ARGS", format!("{},{},{}", args[0], args[1], args[2]))
        .output()
        .expect("spawn one-shot worker");
    assert!(
        out.status.success(),
        "one-shot worker failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The worker writes ONLY the captured library output between markers.
    let s = out.stdout;
    let begin = b"<<<BEGIN>>>";
    let end = b"<<<END>>>";
    let i = find(&s, begin).expect("BEGIN marker") + begin.len();
    let j = find(&s, end).expect("END marker");
    s[i..j].to_vec()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Worker used by `c19_first_call_after_dlopen_in_fresh_process`. It is a no-op
/// unless `DRIVER_ONESHOT` is set, so a normal `cargo test` run does nothing
/// here beyond the env check.
#[test]
fn c19_oneshot_worker() {
    let Ok(which) = std::env::var("DRIVER_ONESHOT") else {
        return;
    };
    let raw = std::env::var("DRIVER_ARGS").expect("DRIVER_ARGS");
    let v: Vec<i32> = raw.split(',').map(|s| s.parse().unwrap()).collect();
    let (x, y, z) = (v[0], v[1], v[2]);

    let bytes = if which == "rust" {
        rust_output(x, y, z)
    } else {
        c_output(x, y, z)
    };

    use std::io::Write;
    let mut so = std::io::stdout();
    so.write_all(b"<<<BEGIN>>>").unwrap();
    so.write_all(&bytes).unwrap();
    so.write_all(b"<<<END>>>").unwrap();
    so.flush().unwrap();
}

// ---------------------------------------------------------------------------
// C20 — output formatting axis (A8): exact literal bytes and `%d` rendering.
// ---------------------------------------------------------------------------

#[test]
fn c20_exact_message_bytes_and_result_codes() {
    // All four reachable `result` values, and the exact byte content of every
    // message. The C build lowers the no-argument `printf`s to `puts`, while the
    // Rust translation uses `printf("%s", ...)`; the bytes must still match.
    let cases: [(i32, i32, i32, &str, i32); 4] = [
        (1, 2, 3, OK, 0),
        (0, 2, 3, ERR_X, 1),
        (1, 0, 3, ERR_Y, 2),
        (1, 2, 0, ERR_Z, 3),
    ];

    for (x, y, z, want, code) in cases {
        let c = c_output(x, y, z);
        let r = rust_output(x, y, z);
        assert_eq!(c, r, "divergence for driver({x}, {y}, {z})");
        assert_eq!(c, want.as_bytes(), "unexpected C bytes for driver({x}, {y}, {z})");

        // The trailing line must be exactly `Result: <code>\n`.
        let tail = format!("Result: {code}\n");
        assert!(
            c.ends_with(tail.as_bytes()),
            "output for driver({x}, {y}, {z}) does not end with {tail:?}"
        );

        // `Operation failed` appears iff the result code is non-zero (E4/A5).
        let has_fail = find(&c, b"Operation failed\n").is_some();
        assert_eq!(
            has_fail,
            code != 0,
            "`Operation failed` presence wrong for driver({x}, {y}, {z})"
        );

        // Exactly one `Error: ` line, and only on failure (E5).
        let n_err = c
            .windows(7)
            .filter(|w| *w == b"Error: ")
            .count();
        assert_eq!(n_err, usize::from(code != 0), "wrong number of Error: lines");

        // No trailing NUL, no stray bytes: the whole stream is printable ASCII
        // plus newlines.
        assert!(
            c.iter().all(|&b| b == b'\n' || (0x20..0x7f).contains(&b)),
            "non-printable byte in output"
        );
    }
}

// ---------------------------------------------------------------------------
// No binary executable exists (CMakeLists has no add_executable, Cargo.toml has
// no [[bin]]), so the "compare binaries' stdout" clause is vacuous. Assert that
// invariant so the claim cannot silently rot.
// ---------------------------------------------------------------------------

#[test]
fn project_builds_no_binary_executable() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cmake = std::fs::read_to_string(root.parent().unwrap().join("c_src/CMakeLists.txt"))
        .expect("read CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds an executable — Phase B must compare binary stdout too"
    );

    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).expect("read Cargo.toml");
    assert!(
        !cargo.contains("[[bin]]"),
        "translation now builds a binary — Phase B must compare binary stdout too"
    );
}
