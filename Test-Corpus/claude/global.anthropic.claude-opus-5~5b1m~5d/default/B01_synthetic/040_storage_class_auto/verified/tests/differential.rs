//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses over the same stdin bytes and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status.
//!
//! Nothing here links the Rust code as a library; both programs are driven
//! exactly the way a shell would drive them, because that is how the
//! translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C reference binary, building it with CMake on first use
/// so that `cargo test` works from a clean checkout.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = manifest_dir().join("..").join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let conf = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run `cmake ..` (is cmake installed?)");
            assert!(
                conf.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&conf.stdout),
                String::from_utf8_lossy(&conf.stderr)
            );
            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run `cmake --build .`");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(exe.exists(), "C reference binary missing at {}", exe.display());
        exe
    })
    .as_path()
}

struct Output {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
}

fn run(bin: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut si = child.stdin.take().expect("stdin pipe");
        let buf = input.to_vec();
        // Write on a helper thread so a program that never drains stdin
        // (or exits early) cannot deadlock the test on a full pipe.
        std::thread::spawn(move || {
            let _ = si.write_all(&buf);
            let _ = si.flush();
        });
    }

    let out = child.wait_with_output().expect("wait for child");
    Output {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

/// Assert the C and Rust programs agree on all three observable channels.
#[track_caller]
fn assert_same(desc: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(&rust_bin(), input);

    let show = |b: &[u8]| String::from_utf8_lossy(b).into_owned();

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {desc} (input {:?}):\n  C: {:?}\n  R: {:?}",
        Preview(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {desc} (input {:?}):\n  C: {:?}\n  R: {:?}",
        Preview(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status, r.status,
        "exit status mismatch for {desc} (input {:?}): C={:?} R={:?}",
        Preview(input),
        c.status,
        r.status
    );
}

/// Truncating debug wrapper so a failure on a megabyte of input stays readable.
struct Preview<'a>(&'a [u8]);
impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let b = self.0;
        if b.len() <= 64 {
            write!(f, "{}", String::from_utf8_lossy(b).escape_debug())
        } else {
            write!(
                f,
                "{}...<{} bytes total>",
                String::from_utf8_lossy(&b[..64]).escape_debug(),
                b.len()
            )
        }
    }
}

fn check_all(cases: &[(&str, &[u8])]) {
    for (desc, input) in cases {
        assert_same(desc, input);
    }
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program branches on
// ---------------------------------------------------------------------------

/// `scanf` returns EOF and leaves `x` at its initialiser of 0, so the program
/// must still print `2*0 + 300`.
#[test]
fn empty_and_whitespace_only_input() {
    check_all(&[
        ("empty input", b""),
        ("single newline", b"\n"),
        ("many newlines", b"\n\n\n"),
        ("single space", b" "),
        ("tab only", b"\t"),
        ("all C whitespace", b" \t\n\x0b\x0c\r"),
    ]);
}

#[test]
fn single_plain_value() {
    check_all(&[
        ("zero", b"0"),
        ("one", b"1"),
        ("five", b"5"),
        ("five with newline", b"5\n"),
        ("large-ish", b"12345"),
    ]);
}

#[test]
fn signs() {
    check_all(&[
        ("negative", b"-5"),
        ("explicit plus", b"+5"),
        ("negative zero", b"-0"),
        ("plus zero", b"+0"),
        ("negative large", b"-12345"),
    ]);
}

/// `scanf`'s `%d` skips leading whitespace of every kind, including newlines.
#[test]
fn leading_whitespace_is_skipped_across_newlines() {
    check_all(&[
        ("spaces then value", b"   7"),
        ("newlines then value", b"\n\n7"),
        ("mixed whitespace", b" \t\n \r\n 7"),
        ("vt/ff then value", b"\x0b\x0c9"),
    ]);
}

/// A matching failure leaves `x` untouched at 0.
#[test]
fn matching_failure_leaves_x_at_zero() {
    check_all(&[
        ("alphabetic", b"abc"),
        ("leading dot", b".5"),
        ("sign only, minus", b"-"),
        ("sign only, plus", b"+"),
        ("double minus", b"--5"),
        ("plus then minus", b"+-5"),
        ("sign then space", b"+ 5"),
        ("space, sign, space", b" - 5"),
        ("punctuation", b",;!"),
        ("letter before digit", b"abc5"),
    ]);
}

/// Conversion stops at the first non-digit; the rest of stdin is never read.
#[test]
fn conversion_stops_at_first_non_digit() {
    check_all(&[
        ("digits then letters", b"5abc"),
        ("hex-looking input", b"0x10"),
        ("two numbers, only first read", b"5 6"),
        ("value then junk", b"42abc"),
        ("digits then NUL", b"5\x00"),
        ("float-looking input", b"3.9"),
        ("exponent-looking input", b"1e9"),
    ]);
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    check_all(&[
        ("007", b"007"),
        ("0000000000000000000001", b"000000000000000000001"),
        ("010", b"010"),
    ]);
}

/// `2*x + 300` overflows `int` for large `x`; the C wraps at `-O0` and the Rust
/// must wrap identically.
#[test]
fn int_overflow_in_arithmetic_wraps() {
    check_all(&[
        ("INT_MAX", b"2147483647"),
        ("INT_MAX-1", b"2147483646"),
        ("INT_MIN", b"-2147483648"),
        ("INT_MIN+1", b"-2147483647"),
        ("2^30, doubling overflows", b"1073741824"),
        ("2^30-1", b"1073741823"),
        ("2^30+1", b"1073741825"),
        ("-2^30", b"-1073741824"),
        ("-2^30-1", b"-1073741825"),
        // Values where 2*x+300 lands exactly on the int boundary.
        ("1073741673", b"1073741673"),
        ("1073741674", b"1073741674"),
    ]);
}

/// Values too large for `int` are converted as a `long` and truncated on
/// assignment; values too large for `long` saturate first.
#[test]
fn scanf_out_of_int_range_truncates() {
    check_all(&[
        ("INT_MAX+1", b"2147483648"),
        ("INT_MAX+2", b"2147483649"),
        ("INT_MIN-1", b"-2147483649"),
        ("UINT_MAX", b"4294967295"),
        ("2^32", b"4294967296"),
        ("2^32+1", b"4294967297"),
    ]);
}

#[test]
fn scanf_out_of_long_range_saturates() {
    check_all(&[
        ("LONG_MAX", b"9223372036854775807"),
        ("LONG_MAX+1", b"9223372036854775808"),
        ("LONG_MIN", b"-9223372036854775808"),
        ("LONG_MIN-1", b"-9223372036854775809"),
        ("twenty nines", b"99999999999999999999"),
        ("negative twenty nines", b"-99999999999999999999"),
        ("10^25", b"10000000000000000000000000"),
        ("-10^25", b"-10000000000000000000000000"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase C: input classes not reached above
// ---------------------------------------------------------------------------

/// Non-ASCII and NUL bytes are not digits, so they are matching failures.
#[test]
fn non_ascii_and_nul_bytes() {
    check_all(&[
        ("leading NUL", b"\x005"),
        ("high bytes", b"\xff\xfe5"),
        ("0x80", b"\x80"),
        ("fullwidth digit (UTF-8)", "５".as_bytes()),
        ("invalid UTF-8 only", b"\xc3\x28"),
        ("all high bytes", b"\xff\xff\xff\xff"),
    ]);
}

/// Exercises the Rust reader's internal buffering and one-byte pushback around
/// its refill boundary, which the C `FILE*` stream handles transparently.
#[test]
fn input_spanning_the_read_buffer_boundary() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for n in [4093usize, 4094, 4095, 4096, 4097, 4098, 8191, 8192, 8193, 65536] {
        let mut v = vec![b' '; n];
        v.push(b'7');
        cases.push((format!("{n} spaces then 7"), v));

        // A value whose digits straddle the refill boundary, followed by a
        // non-digit that must terminate the conversion.
        let mut v = vec![b' '; n.saturating_sub(2)];
        v.extend_from_slice(b"42abc");
        cases.push((format!("value at offset {} then junk", n - 2), v));

        // A digit run that ends exactly at the boundary.
        let mut v = vec![b'1'; n];
        cases.push((format!("{n} ones"), v.clone()));
        v.push(b'x');
        cases.push((format!("{n} ones then x"), v));
    }
    for (desc, input) in &cases {
        assert_same(desc, input);
    }
}

#[test]
fn very_long_inputs() {
    let long_digits = vec![b'9'; 100_000];
    let neg_long: Vec<u8> = std::iter::once(b'-').chain(long_digits.iter().copied()).collect();
    let mut zeros_then_value = vec![b'0'; 5000];
    zeros_then_value.push(b'3');
    let mut zeros_then_big = vec![b'0'; 4090];
    zeros_then_big.extend_from_slice(b"2147483648");
    let mut mb_ws = vec![b' '; 1_000_000];
    mb_ws.push(b'8');
    let mut mb_newlines = vec![b'\n'; 1_000_000];
    mb_newlines.extend_from_slice(b"123");
    let only_whitespace = vec![b'\n'; 200_000];

    check_all(&[
        ("100k nines", &long_digits),
        ("minus 100k nines", &neg_long),
        ("5000 zeros then 3", &zeros_then_value),
        ("4090 zeros then INT_MAX+1", &zeros_then_big),
        ("1MB spaces then 8", &mb_ws),
        ("1MB newlines then 123", &mb_newlines),
        ("200k newlines only", &only_whitespace),
    ]);
}

/// `main` ignores `argc`/`argv`, so extra arguments must not change anything.
#[test]
fn extra_arguments_are_ignored() {
    let c = Command::new(c_bin())
        .args(["alpha", "beta", "gamma"])
        .stdin(Stdio::null())
        .output()
        .expect("run C binary with args");
    let r = Command::new(rust_bin())
        .args(["alpha", "beta", "gamma"])
        .stdin(Stdio::null())
        .output()
        .expect("run Rust binary with args");
    assert_eq!(c.stdout, r.stdout, "stdout mismatch with extra argv");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch with extra argv");
    assert_eq!(c.status.code(), r.status.code(), "status mismatch with extra argv");
}

/// stdin at immediate EOF (`/dev/null`) rather than a pipe that is written and
/// then closed.
#[test]
fn stdin_is_dev_null() {
    let c = Command::new(c_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run C binary");
    let r = Command::new(rust_bin())
        .stdin(Stdio::null())
        .output()
        .expect("run Rust binary");
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

/// stdin redirected from a regular file, so reads return in file-sized blocks
/// instead of pipe-sized ones.
#[test]
fn stdin_is_a_regular_file() {
    let dir = std::env::temp_dir();
    for (name, contents) in [
        ("driver_in_a", "1073741824".as_bytes().to_vec()),
        ("driver_in_b", b"   \n\n  -2147483648  \n".to_vec()),
        ("driver_in_c", vec![b'7'; 9000]),
    ] {
        let path = dir.join(format!("{name}_{}", std::process::id()));
        std::fs::write(&path, &contents).expect("write temp input file");
        let open = || std::fs::File::open(&path).expect("reopen temp input file");
        let c = Command::new(c_bin())
            .stdin(Stdio::from(open()))
            .output()
            .expect("run C binary");
        let r = Command::new(rust_bin())
            .stdin(Stdio::from(open()))
            .output()
            .expect("run Rust binary");
        assert_eq!(c.stdout, r.stdout, "stdout mismatch for file input {name}");
        assert_eq!(c.stderr, r.stderr, "stderr mismatch for file input {name}");
        assert_eq!(
            c.status.code(),
            r.status.code(),
            "status mismatch for file input {name}"
        );
        let _ = std::fs::remove_file(&path);
    }
}

/// Deterministic pseudo-random sweep (xorshift, so no external dependency) over
/// digit/sign/whitespace/junk soup and over the full byte range.
#[test]
fn randomized_sweep() {
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    const ALPHA: &[u8] = b"0123456789+- \t\n\r\x0b\x0cabcxX.,eE_";

    for i in 0..400 {
        let len = (next() % 15) as usize;
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHA[(next() % ALPHA.len() as u64) as usize])
            .collect();
        assert_same(&format!("random token soup #{i}"), &input);
    }

    for i in 0..200 {
        let len = (next() % 12) as usize;
        let input: Vec<u8> = (0..len).map(|_| (next() % 256) as u8).collect();
        assert_same(&format!("random raw bytes #{i}"), &input);
    }

    for i in 0..200 {
        // Random values around and beyond the int range.
        let v = (next() as i64) >> (next() % 40);
        assert_same(&format!("random integer #{i}"), v.to_string().as_bytes());
    }
}
