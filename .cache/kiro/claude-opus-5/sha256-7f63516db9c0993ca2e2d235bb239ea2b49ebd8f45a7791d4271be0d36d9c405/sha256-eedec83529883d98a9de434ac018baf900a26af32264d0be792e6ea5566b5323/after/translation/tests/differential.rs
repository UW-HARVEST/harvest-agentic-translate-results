//! Differential tests: run the C program and the Rust program as subprocesses
//! and require byte-identical stdout, byte-identical stderr, and an identical
//! exit status for every input.
//!
//! The Rust code is never called as a library here; only the built binary is
//! driven, the same way a shell would drive it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary under test, supplied by cargo.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C binary, building it with cmake on first use if necessary.
///
/// `c_src/` is only ever read and built out-of-tree into `c_src/build`; no
/// source file in it is modified.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.is_file() {
            return exe;
        }

        std::fs::create_dir_all(&build).expect("could not create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("failed to run `cmake ..` (is cmake installed?)");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let compile = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("failed to run `cmake --build .`");
        assert!(
            compile.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        assert!(
            exe.is_file(),
            "C build reported success but {} is missing",
            exe.display()
        );
        exe
    })
}

/// Runs `prog` with `stdin_data` on stdin, capturing stdout and stderr.
fn run(prog: &Path, stdin_data: &[u8]) -> Output {
    let mut child = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", prog.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        // The program may exit without draining stdin (it reads one integer),
        // so a broken pipe here is expected and not a failure.
        let _ = sink.write_all(stdin_data);
        let _ = sink.flush();
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {}: {e}", prog.display()))
}

/// Asserts that both programs agree on stdout, stderr and exit status.
fn assert_same(label: &str, stdin_data: &[u8]) {
    let c = run(c_bin(), stdin_data);
    let r = run(rust_bin(), stdin_data);

    let show = |b: &[u8]| match std::str::from_utf8(b) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{b:x?}"),
    };

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {})\n C: {}\n R: {}",
        show(stdin_data),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {})\n C: {}\n R: {}",
        show(stdin_data),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for {label} (input {}): C={:?} Rust={:?}",
        show(stdin_data),
        c.status,
        r.status
    );
}

fn assert_same_str(label: &str, stdin_data: &str) {
    assert_same(label, stdin_data.as_bytes());
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"1\n");
    let r = run(rust_bin(), b"1\n");
    assert!(c.status.success(), "C program did not exit 0: {:?}", c.status);
    assert!(
        r.status.success(),
        "Rust program did not exit 0: {:?}",
        r.status
    );
    assert!(
        !c.stdout.is_empty(),
        "C program produced no stdout; the comparison would be vacuous"
    );
    assert_eq!(c.stdout, r.stdout);
}

/// Pins the exact expected bytes so a silent change on *both* sides is caught.
#[test]
fn golden_output_for_one() {
    let expected = "\
The house has 2 floors, 5 bedrooms, and 2.5 bathrooms
The house has 3 floors, 5 bedrooms, and 2.5 bathrooms
The house has 3 floors, 5 bedrooms, and 3.5 bathrooms
The house has 3 floors, 6 bedrooms, and 3.5 bathrooms
The house has 3 floors, 6 bedrooms, and 3.5 bathrooms
The house has 4 floors, 6 bedrooms, and 3.5 bathrooms
The house has 4 floors, 6 bedrooms, and 4.5 bathrooms
The house has 4 floors, 7 bedrooms, and 4.5 bathrooms
";
    let c = run(c_bin(), b"1\n");
    let r = run(rust_bin(), b"1\n");
    assert_eq!(String::from_utf8_lossy(&c.stdout), expected);
    assert_eq!(String::from_utf8_lossy(&r.stdout), expected);
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
}

// ---------------------------------------------------------------------------
// Phase B: the branches the C program actually takes.
//
// main() is straight-line apart from `scanf("%d", &x)`, which has three
// outcomes: successful conversion, matching failure (x keeps its initial 0),
// and input failure / EOF (also keeps 0). run() is then called twice against
// mutable global state, so the second call observes the first call's writes.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_eof() {
    // scanf returns EOF, x stays 0. The program must still print 8 lines.
    assert_same_str("empty input", "");
}

#[test]
fn whitespace_only_input() {
    // %d skips whitespace, then hits EOF: input failure, x stays 0.
    assert_same_str("spaces then EOF", "   ");
    assert_same_str("tabs/newlines then EOF", " \t\n \n\t ");
    assert_same_str("newline only", "\n");
    assert_same_str("vertical tab / form feed / CR", "\x0b\x0c\r");
}

#[test]
fn single_value() {
    assert_same_str("zero", "0");
    assert_same_str("one", "1");
    assert_same_str("with trailing newline", "1\n");
    assert_same_str("negative", "-3");
    assert_same_str("negative zero", "-0");
    assert_same_str("explicit plus", "+7");
}

#[test]
fn scanf_skips_leading_whitespace_across_newlines() {
    // %d crosses newlines, unlike fgets.
    assert_same_str("newlines before number", "\n\n\n\n42");
    assert_same_str("tabs before number", "\t\t7");
    assert_same_str("mixed whitespace", "  \t\r\n  12\n");
    assert_same_str("cr before number", "\r6");
}

#[test]
fn only_the_first_number_is_read() {
    // scanf is called once; the rest of stdin is never consumed, and both
    // programs must ignore it identically.
    assert_same_str("two numbers", "3 9");
    assert_same_str("numbers on separate lines", "11\n22\n");
    assert_same_str("number then junk", "5abc");
    assert_same_str("number then lots of text", "8\nthis is never read\nnor this\n");
}

#[test]
fn matching_failure_leaves_x_at_zero() {
    // No digits consumable -> scanf returns 0, x is still the initializer 0.
    assert_same_str("letters", "abc");
    assert_same_str("leading dot", ".5");
    assert_same_str("sign only", "-");
    assert_same_str("plus only", "+");
    assert_same_str("sign then space", "- 5");
    assert_same_str("double sign", "--5");
    assert_same_str("plus minus", "+-5");
    assert_same_str("punctuation", "!!!");
    assert_same_str("underscore", "_1");
}

#[test]
fn base_ten_only() {
    // %d is decimal: "0x10" yields 0, "1e5" yields 1, "5.9" yields 5.
    assert_same_str("hex-looking", "0x10");
    assert_same_str("exponent-looking", "1e5");
    assert_same_str("decimal point", "5.9");
    assert_same_str("thousands comma", "1,000");
    assert_same_str("octal-looking", "0755");
}

#[test]
fn leading_zeros() {
    assert_same_str("many leading zeros", "00000000000000000000005");
    assert_same_str("negative with leading zeros", "-00000000000000000000005");
    assert_same_str("all zeros", "000000000000000000000000");
}

#[test]
fn int_boundaries_and_signed_overflow() {
    // bedrooms starts at 5 and `bedrooms += extra` runs once per run() call,
    // i.e. twice, so these exercise wrapping in both additions.
    assert_same_str("INT_MAX", "2147483647");
    assert_same_str("INT_MAX-1", "2147483646");
    assert_same_str("INT_MIN", "-2147483648");
    assert_same_str("INT_MIN+1", "-2147483647");
    assert_same_str("2^30", "1073741824");
    assert_same_str("-2^30", "-1073741824");
    assert_same_str("2^31", "2147483648");
    assert_same_str("-(2^31)-1", "-2147483649");
    assert_same_str("65536", "65536");
    assert_same_str("UINT32_MAX", "4294967295");
    assert_same_str("2^32", "4294967296");
}

#[test]
fn out_of_range_conversion_is_clamped_then_truncated() {
    // glibc converts %d through a long, clamping at LONG_MAX/LONG_MIN, then
    // truncates to int. LONG_MAX -> -1, LONG_MIN -> 0.
    assert_same_str("LONG_MAX", "9223372036854775807");
    assert_same_str("LONG_MAX+1", "9223372036854775808");
    assert_same_str("LONG_MIN", "-9223372036854775808");
    assert_same_str("LONG_MIN-1", "-9223372036854775809");
    assert_same_str("2^64", "18446744073709551616");
    assert_same_str("twenty nines", "99999999999999999999");
    assert_same_str("negative twenty nines", "-99999999999999999999");
}

#[test]
fn very_long_digit_strings() {
    let nines = "9".repeat(4000);
    assert_same("4000 nines", nines.as_bytes());

    let neg_nines = format!("-{nines}");
    assert_same("4000 negative nines", neg_nines.as_bytes());

    let padded = format!("{}7", "0".repeat(4000));
    assert_same("4000 zeros then 7", padded.as_bytes());

    let huge = "1234567890".repeat(500);
    assert_same("5000 mixed digits", huge.as_bytes());
}

#[test]
fn non_ascii_and_embedded_nul_bytes() {
    assert_same("NUL first", b"\x005");
    assert_same("NUL after digit", b"5\x00");
    assert_same("high byte first", b"\xff5");
    assert_same("utf-8 text", "héllo".as_bytes());
    assert_same("invalid utf-8", b"\xc3\x28\x31");
    assert_same("whitespace then NUL", b"   \x00");
}

#[test]
fn large_input_is_not_fully_consumed() {
    // A megabyte of trailing data after the number: scanf stops early, the
    // writer's broken pipe must not change observable behavior.
    let mut data = b"5\n".to_vec();
    data.extend(std::iter::repeat(b'x').take(1 << 20));
    assert_same("1 MiB of trailing junk", &data);
}

// ---------------------------------------------------------------------------
// Phase C: a deterministic sweep so no arithmetic corner stays untried.
// ---------------------------------------------------------------------------

#[test]
fn deterministic_sweep_over_int_range() {
    // Fixed-seed xorshift: reproducible, no rand dependency.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let mut cases: Vec<String> = Vec::new();

    // Powers of two and their neighbours, both signs.
    for bit in 0..32u32 {
        let v = 1i64 << bit;
        for delta in [-1i64, 0, 1] {
            cases.push((v + delta).to_string());
            cases.push((-(v + delta)).to_string());
        }
    }

    // Values that make 5 + 2*extra land near the wrapping boundary.
    for anchor in [
        i32::MAX as i64,
        i32::MIN as i64,
        (i32::MAX as i64) / 2,
        (i32::MIN as i64) / 2,
    ] {
        for delta in -3i64..=3 {
            cases.push((anchor + delta).to_string());
        }
    }

    // Pseudorandom 32-bit and 64-bit magnitudes.
    for _ in 0..120 {
        cases.push((next() as i32).to_string());
    }
    for _ in 0..60 {
        cases.push((next() as i64).to_string());
    }

    for case in &cases {
        assert_same("sweep", case.as_bytes());
        assert_same("sweep + newline", format!("{case}\n").as_bytes());
    }
}

#[test]
fn stdin_closed_entirely() {
    // Not an input failure via EOF but a read error on fd 0; scanf still
    // leaves x at 0 and the program still exits 0.
    let c = Command::new(c_bin())
        .stdin(Stdio::null())
        .output()
        .expect("spawn C");
    let r = Command::new(rust_bin())
        .stdin(Stdio::null())
        .output()
        .expect("spawn Rust");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

#[test]
fn stdin_is_a_directory() {
    // read(2) on a directory fd fails with EISDIR: another input-failure path.
    let dir = std::fs::File::open(repo_root()).expect("open repo root");
    let dir2 = dir.try_clone().expect("clone dir handle");

    let c = Command::new(c_bin())
        .stdin(Stdio::from(dir))
        .output()
        .expect("spawn C");
    let r = Command::new(rust_bin())
        .stdin(Stdio::from(dir2))
        .output()
        .expect("spawn Rust");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

#[test]
fn argv_is_ignored() {
    // main() takes no parameters, so extra arguments must change nothing.
    for args in [vec!["ignored"], vec!["-h"], vec!["a", "b", "c"]] {
        let c = Command::new(c_bin())
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .expect("spawn C");
        let r = Command::new(rust_bin())
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .expect("spawn Rust");
        assert_eq!(c.stdout, r.stdout, "stdout differs for argv {args:?}");
        assert_eq!(c.stderr, r.stderr, "stderr differs for argv {args:?}");
        assert_eq!(
            c.status.code(),
            r.status.code(),
            "exit status differs for argv {args:?}"
        );
    }
}

#[test]
fn bathrooms_formatting_is_stable_across_both_runs() {
    // %.1f on 2.5 / 3.5 / 4.5: the two run() calls must show 2.5, 3.5 then
    // 3.5, 4.5 because the global carries over. Guards against a translation
    // that resets state between calls.
    let out = run(rust_bin(), b"0");
    let text = String::from_utf8(out.stdout).expect("utf-8 output");
    let baths: Vec<&str> = text
        .lines()
        .map(|l| l.rsplit(", and ").next().unwrap().trim_end_matches(" bathrooms"))
        .collect();
    assert_eq!(
        baths,
        vec!["2.5", "2.5", "3.5", "3.5", "3.5", "3.5", "4.5", "4.5"]
    );

    let c_out = run(c_bin(), b"0");
    assert_eq!(String::from_utf8_lossy(&c_out.stdout), text);
}
