//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses with identical stdin, and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status.
//!
//! Nothing here links the Rust crate as a library. Both programs are driven
//! exactly the way a shell would drive them.
//!
//! The C program is configured and built with CMake into `target/c_build` so
//! that `c_src/` is never written to.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating and building the two binaries
// ---------------------------------------------------------------------------

/// `translation/`
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The directory that holds `c_src/` and `translation/`.
fn workspace_root() -> PathBuf {
    crate_dir()
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn c_source_dir() -> PathBuf {
    workspace_root().join("c_src")
}

/// Where CMake artifacts for the reference program go. Deliberately outside
/// `c_src/` so the C tree stays pristine.
fn c_build_dir() -> PathBuf {
    crate_dir().join("target").join("c_build")
}

/// Configure + build the C reference program once per test process, and return
/// the path to the executable.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let src = c_source_dir();
        assert!(
            src.join("CMakeLists.txt").is_file(),
            "expected {} to exist",
            src.join("CMakeLists.txt").display()
        );

        let build = c_build_dir();
        std::fs::create_dir_all(&build).expect("could not create the C build directory");

        // If a previously built reference binary is already present, reuse it.
        let exe = build.join("driver");
        if exe.is_file() {
            return exe;
        }

        let configure = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build)
            .output()
            .expect("failed to run `cmake` (is CMake installed?)");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let compile = Command::new("cmake")
            .arg("--build")
            .arg(&build)
            .output()
            .expect("failed to run `cmake --build`");
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

/// The Rust executable under test. Prefers an explicit override, then the
/// release build, then the binary Cargo built for this test run.
fn rust_binary() -> &'static Path {
    static RUST_BIN: OnceLock<PathBuf> = OnceLock::new();
    RUST_BIN.get_or_init(|| {
        if let Some(p) = std::env::var_os("RUST_DRIVER_BIN") {
            let p = PathBuf::from(p);
            assert!(p.is_file(), "RUST_DRIVER_BIN={} is not a file", p.display());
            return p;
        }
        let release = crate_dir().join("target").join("release").join("driver");
        if release.is_file() {
            return release;
        }
        PathBuf::from(env!("CARGO_BIN_EXE_driver"))
    })
}

// ---------------------------------------------------------------------------
// Running the programs
// ---------------------------------------------------------------------------

fn run(program: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut sink = child.stdin.take().expect("stdin was piped");
        // A short write is fine: the program may exit before consuming
        // everything (e.g. huge inputs), which closes the pipe.
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to collect output of {}: {e}", program.display()))
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:02x?}"),
    }
}

/// Assert that both programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_binary(), stdin_bytes);
    let r = run(rust_binary(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (stdin = {}):\n  C    = {}\n  Rust = {}",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (stdin = {}):\n  C    = {}\n  Rust = {}",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for {label} (stdin = {}): C = {:?}, Rust = {:?}",
        show(stdin_bytes),
        c.status,
        r.status
    );
}

#[track_caller]
fn assert_all_same(label: &str, inputs: &[&[u8]]) {
    for (i, input) in inputs.iter().enumerate() {
        assert_same(&format!("{label}[{i}]"), input);
    }
}

// ---------------------------------------------------------------------------
// Phase A: both programs are built and runnable
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_build_and_run() {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.is_file(), "C binary missing at {}", c.display());
    assert!(r.is_file(), "Rust binary missing at {}", r.display());

    // A trivial run must succeed for both, otherwise every later comparison
    // would be measuring nothing.
    let co = run(c, b"1\n");
    let ro = run(r, b"1\n");
    assert!(co.status.success(), "C reference exited with {:?}", co.status);
    assert!(ro.status.success(), "Rust binary exited with {:?}", ro.status);
    assert!(!co.stdout.is_empty(), "C reference produced no stdout");
    assert_eq!(co.stdout, ro.stdout);
}

/// Pin the reference program's observable contract so a regression in the test
/// harness itself (e.g. comparing a program against itself) cannot hide.
/// 16 lowercase hex bytes plus one trailing newline, exit 0, empty stderr.
#[test]
fn reference_output_shape_is_pinned() {
    let out = run(c_binary(), b"1\n");
    assert_eq!(
        out.stdout, b"01000000030000000000000000000040\n",
        "unexpected reference stdout: {}",
        show(&out.stdout)
    );
    assert!(out.stderr.is_empty());
    assert_eq!(out.status.code(), Some(0));

    // And the Rust program must agree.
    assert_same("pinned shape", b"1\n");
}

// ---------------------------------------------------------------------------
// Phase B: the inputs the C program branches on
//
// The C program has one input-dependent decision: the result of
// `scanf("%d", &x)`. Its three outcomes are
//   * successful conversion  -> x is the converted value
//   * matching failure       -> x keeps its initial 0
//   * input failure (EOF)    -> x keeps its initial 0
// Everything downstream (`driver` / `print_hex`) is unconditional, but the
// value of `x` is serialized byte-for-byte, so every distinct converted value
// is its own observable case.
// ---------------------------------------------------------------------------

/// Empty input: `scanf` reports input failure, `x` stays 0.
#[test]
fn empty_input() {
    assert_same("empty stdin", b"");
}

/// Whitespace-only input. `%d` skips whitespace, then hits EOF: input failure.
/// Covers every character `isspace()` accepts in the C locale.
#[test]
fn whitespace_only_input() {
    assert_all_same(
        "whitespace only",
        &[
            b" ",
            b"\n",
            b"\t",
            b"\r",
            b"\x0b",
            b"\x0c",
            b"   \n\t\r\x0b\x0c   ",
            b"\n\n\n\n",
        ],
    );
}

/// A single well-formed item, the happy path.
#[test]
fn single_item() {
    assert_all_same(
        "single item",
        &[b"0", b"1", b"2", b"3", b"7", b"42", b"1000000", b"123456789"],
    );
}

/// `scanf` reads across newlines and other whitespace before the number.
#[test]
fn scanf_skips_leading_whitespace_across_newlines() {
    assert_all_same(
        "leading whitespace",
        &[
            b"   42",
            b"\n42",
            b"\n\n\n42\n",
            b"\t\t42",
            b"\r\n42",
            b" \n \t \x0b \x0c 42",
            b"\n\n\n\n\n\n\n\n9\n\n\n",
        ],
    );
}

/// Optional sign handling, including redundant zeros and signed zero.
#[test]
fn sign_handling() {
    assert_all_same(
        "signs",
        &[
            b"-1",
            b"+1",
            b"-0",
            b"+0",
            b"-42",
            b"+42",
            b"-0000000001",
            b"+0000000001",
            b" -12 ",
            b"\n+7\n",
        ],
    );
}

/// Leading zeros are consumed as digits and do not affect the value.
#[test]
fn leading_zeros() {
    assert_all_same(
        "leading zeros",
        &[b"0", b"00", b"0000012", b"0000000000000000000000000000005", b"-000012"],
    );
}

/// Matching failure: the first non-whitespace character cannot start an
/// integer, so nothing is stored and `x` stays 0.
#[test]
fn matching_failure_leaves_x_zero() {
    assert_all_same(
        "matching failure",
        &[
            b"abc",
            b"x",
            b"X",
            b"e5",
            b".5",
            b",5",
            b"-",
            b"+",
            b"- 5",
            b"+ 5",
            b"--5",
            b"++5",
            b"+-5",
            b"-+5",
            b"-\n5",
            b"/",
            b":",
            b"@",
            b"\x7f",
            b"   abc",
            b"\nzz\n",
        ],
    );
}

/// Conversion stops at the first character that cannot extend the number; the
/// trailing garbage is simply never read.
#[test]
fn conversion_stops_at_first_non_digit() {
    assert_all_same(
        "trailing garbage",
        &[
            b"42abc",
            b"7x",
            b"1.5",
            b"1,5",
            b"1e10",
            b"0x1f",
            b"0b101",
            b"-3junk",
            b"12-34",
            b"5+5",
        ],
    );
}

/// Only the first item is ever read; the rest of stdin is ignored.
#[test]
fn only_first_item_is_read() {
    assert_all_same(
        "multiple items",
        &[
            b"1 2",
            b"7 8 9",
            b"1\n2\n3\n",
            b"2147483647\n2",
            b"  5  \n  6  \n",
            b"3 abc",
            b"3 -4",
        ],
    );
}

/// Boundary values of a 32-bit `int`, which is the type the value is stored
/// into and the type serialized in the struct image.
#[test]
fn int_boundaries() {
    assert_all_same(
        "int boundaries",
        &[
            b"2147483647",  // INT_MAX
            b"-2147483648", // INT_MIN
            b"2147483646",
            b"-2147483647",
            b"65535",
            b"65536",
            b"-65536",
            b"255",
            b"256",
            b"-1", // all bits set
        ],
    );
}

/// Values past `INT_MAX`/`INT_MIN` but inside `long`: glibc converts into a
/// `long` and stores it through an `int *`, truncating to 32 bits. The C
/// behavior is what defines the expected bytes here.
#[test]
fn values_beyond_int_are_truncated_like_c() {
    assert_all_same(
        "int truncation",
        &[
            b"2147483648",
            b"2147483649",
            b"-2147483649",
            b"-2147483650",
            b"4294967295",
            b"4294967296",
            b"4294967297",
            b"-4294967296",
            b"8589934592",
            b"1000000000000",
            b"-1000000000000",
        ],
    );
}

/// `long` boundaries and past them, where glibc's `strtol` saturates before the
/// truncating store. Again the C run defines the expectation.
#[test]
fn long_boundaries_and_overflow() {
    assert_all_same(
        "long overflow",
        &[
            b"9223372036854775806",
            b"9223372036854775807", // LONG_MAX
            b"9223372036854775808", // LONG_MAX + 1
            b"9223372036854775809",
            b"-9223372036854775807",
            b"-9223372036854775808", // LONG_MIN
            b"-9223372036854775809", // LONG_MIN - 1
            b"18446744073709551615",
            b"18446744073709551616",
            b"99999999999999999999999999999999",
            b"-99999999999999999999999999999999",
        ],
    );
}

// ---------------------------------------------------------------------------
// Phase C: input classes not otherwise reached
// ---------------------------------------------------------------------------

/// Non-ASCII and NUL bytes in the stream. A NUL is not whitespace and not a
/// digit, so it is a matching failure when it comes first.
#[test]
fn non_ascii_and_nul_bytes() {
    assert_all_same(
        "raw bytes",
        &[
            b"\x00",
            b"\x005",
            b"5\x00",
            b"\x005\x00",
            b"\xff",
            b"\xff5",
            b"5\xff",
            b"\xc3\xa9",
            b"\xc3\xa97",
            b"7\xc3\xa9",
            b"\x01\x02\x03",
            b"\x80\x81",
        ],
    );
}

/// "The maximum the code handles" from the reading side: digit runs far longer
/// than any plausible internal buffer, both saturating and not.
#[test]
fn very_long_digit_runs() {
    let long_nines: Vec<u8> = vec![b'9'; 100_000];
    let long_neg_nines: Vec<u8> = std::iter::once(b'-')
        .chain(std::iter::repeat(b'9').take(100_000))
        .collect();
    // 100k leading zeros then a small value: never overflows.
    let long_zeros: Vec<u8> = std::iter::repeat(b'0')
        .take(100_000)
        .chain(std::iter::once(b'5'))
        .collect();
    // 100k leading whitespace characters before a value.
    let long_ws: Vec<u8> = std::iter::repeat(b' ')
        .take(100_000)
        .chain(b"123".iter().copied())
        .collect();
    // A long run of digits followed by garbage.
    let long_then_junk: Vec<u8> = std::iter::repeat(b'1')
        .take(50_000)
        .chain(b"zzz".iter().copied())
        .collect();

    assert_all_same(
        "long runs",
        &[
            &long_nines,
            &long_neg_nines,
            &long_zeros,
            &long_ws,
            &long_then_junk,
        ],
    );
}

/// Stdin that yields a read error rather than data. `scanf` fails, `x` stays 0.
#[test]
fn stdin_is_a_directory() {
    // Reading from a directory fd fails with EISDIR on Linux.
    let dir = std::env::temp_dir();
    let open = |p: &Path| std::fs::File::open(p).expect("could not open temp dir as a file");

    let c = Command::new(c_binary())
        .stdin(Stdio::from(open(&dir)))
        .output()
        .expect("failed to run C reference with a directory on stdin");
    let r = Command::new(rust_binary())
        .stdin(Stdio::from(open(&dir)))
        .output()
        .expect("failed to run Rust binary with a directory on stdin");

    assert_eq!(c.stdout, r.stdout, "stdout mismatch with a directory on stdin");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch with a directory on stdin");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch with a directory on stdin"
    );
}

/// Stdin closed outright (no file descriptor 0 to read).
#[test]
fn stdin_closed() {
    let c = Command::new(c_binary())
        .stdin(Stdio::null())
        .output()
        .expect("failed to run C reference with /dev/null stdin");
    let r = Command::new(rust_binary())
        .stdin(Stdio::null())
        .output()
        .expect("failed to run Rust binary with /dev/null stdin");

    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

/// `main()` takes no parameters, so argv must not change anything.
#[test]
fn command_line_arguments_are_ignored() {
    for args in [
        vec!["99"],
        vec!["-h"],
        vec!["--help"],
        vec!["a", "b", "c"],
        vec![""],
    ] {
        let c = Command::new(c_binary())
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .expect("failed to run C reference with arguments");
        let r = Command::new(rust_binary())
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .expect("failed to run Rust binary with arguments");
        assert_eq!(c.stdout, r.stdout, "stdout mismatch for argv {args:?}");
        assert_eq!(c.stderr, r.stderr, "stderr mismatch for argv {args:?}");
        assert_eq!(
            c.status.code(),
            r.status.code(),
            "exit status mismatch for argv {args:?}"
        );
    }
}

/// stdout redirected to a file rather than a pipe: `printf` buffering differs
/// (fully buffered instead of line buffered) but the bytes written must not.
#[test]
fn stdout_to_a_file_matches() {
    let dir = std::env::temp_dir();
    let unique = std::process::id();
    let c_path = dir.join(format!("driver_c_{unique}.out"));
    let r_path = dir.join(format!("driver_r_{unique}.out"));

    for input in [&b""[..], b"5", b"abc", b"-2147483648", b"  9\n"] {
        let write_to = |p: &Path| std::fs::File::create(p).expect("could not create output file");

        let mut c = Command::new(c_binary())
            .stdin(Stdio::piped())
            .stdout(Stdio::from(write_to(&c_path)))
            .spawn()
            .expect("failed to spawn C reference");
        let _ = c.stdin.take().unwrap().write_all(input);
        let c_status = c.wait().expect("C reference did not exit");

        let mut r = Command::new(rust_binary())
            .stdin(Stdio::piped())
            .stdout(Stdio::from(write_to(&r_path)))
            .spawn()
            .expect("failed to spawn Rust binary");
        let _ = r.stdin.take().unwrap().write_all(input);
        let r_status = r.wait().expect("Rust binary did not exit");

        let c_bytes = std::fs::read(&c_path).expect("could not read C output file");
        let r_bytes = std::fs::read(&r_path).expect("could not read Rust output file");
        assert_eq!(
            c_bytes,
            r_bytes,
            "file stdout mismatch for {}:\n  C    = {}\n  Rust = {}",
            show(input),
            show(&c_bytes),
            show(&r_bytes)
        );
        assert_eq!(c_status.code(), r_status.code());
    }

    let _ = std::fs::remove_file(&c_path);
    let _ = std::fs::remove_file(&r_path);
}

/// Powers of two and bit patterns, so every byte position of the serialized
/// `floors` field takes non-zero values and any byte-order or offset error
/// shows up.
#[test]
fn every_byte_of_the_value_is_exercised() {
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for shift in 0..31 {
        let v: i32 = 1i32 << shift;
        inputs.push(v.to_string().into_bytes());
        inputs.push((-v).to_string().into_bytes());
    }
    for v in [
        i32::MIN,
        i32::MAX,
        -1,
        0x0102_0304,
        -0x0102_0304,
        0x7f7f_7f7f,
        0x00ff_00ff,
        -0x00ff_00ff,
    ] {
        inputs.push(v.to_string().into_bytes());
    }
    let refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();
    assert_all_same("bit patterns", &refs);
}

/// A deterministic sweep over generated inputs: mixed whitespace, signs,
/// digits and stray characters, plus values spanning far past 64 bits. This is
/// a wide net over the same single decision the C program makes, run with a
/// fixed seed so failures reproduce.
#[test]
fn deterministic_sweep() {
    // xorshift64*, so the sweep needs no dependencies and is reproducible.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    const ALPHABET: &[u8] = b"0123456789+- \t\n\r\x0b\x0cabxXeE.,\x00\xff";

    let mut rng = Rng(0x1234_5678_9abc_def1);
    let mut inputs: Vec<Vec<u8>> = Vec::new();

    // Random short byte strings over an alphabet of interesting characters.
    for _ in 0..400 {
        let len = rng.below(18);
        inputs.push((0..len).map(|_| ALPHABET[rng.below(ALPHABET.len())]).collect());
    }

    // Random wide-range numeric literals, some with padding or trailing junk.
    for _ in 0..200 {
        let magnitude = rng.next() >> rng.below(64) as u32;
        let negative = rng.next() & 1 == 1;
        let mut s = Vec::new();
        s.extend_from_slice(match rng.below(4) {
            0 => b"",
            1 => b" ",
            2 => b"\n",
            _ => b"\t ",
        });
        if negative {
            s.push(b'-');
        }
        s.extend_from_slice(magnitude.to_string().as_bytes());
        // Sometimes append extra digits to push it past 64 bits.
        for _ in 0..rng.below(6) {
            s.push(b'0' + (rng.below(10) as u8));
        }
        s.extend_from_slice(match rng.below(4) {
            0 => b"",
            1 => b"\n",
            2 => b" x",
            _ => b"zz",
        });
        inputs.push(s);
    }

    let refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();
    assert_all_same("sweep", &refs);
}

/// FNV-1a 64, so the guard below needs no dependency.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The C tree is the ground truth, so it must be byte-identical to what these
/// expectations were derived from. Pinned by content hash rather than git so
/// the check is active regardless of how the tree was obtained.
#[test]
fn c_sources_are_unmodified() {
    let expected: &[(&str, usize, u64)] = &[
        ("src/main.c", 1650, 0xe894_27e8_f472_ce6b),
        ("CMakeLists.txt", 1200, 0xcf8f_06bd_8744_85bc),
    ];

    for &(rel, len, hash) in expected {
        let path = c_source_dir().join(rel);
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));
        assert_eq!(
            bytes.len(),
            len,
            "{} changed size ({} bytes, expected {len}); c_src must not be modified",
            path.display(),
            bytes.len()
        );
        assert_eq!(
            fnv1a64(&bytes),
            hash,
            "{} content changed; c_src must not be modified",
            path.display()
        );
    }
}
