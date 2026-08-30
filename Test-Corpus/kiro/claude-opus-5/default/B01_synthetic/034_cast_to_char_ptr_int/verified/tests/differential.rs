// Differential integration tests: run the C binary and the Rust binary as
// subprocesses, feed both the same bytes on stdin, and require byte-identical
// stdout, byte-identical stderr and an identical exit status.
//
// The Rust code is NEVER called as a library here; only the built executable is
// driven, exactly the way the graded comparison drives it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Path to the Rust executable under test, supplied by Cargo.
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

/// Locate (building if necessary) the C reference executable.
fn c_bin() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_src = manifest
            .parent()
            .expect("translation/ must have a parent directory")
            .join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to spawn cmake (is cmake installed?)");
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
                .expect("failed to spawn cmake --build");
            assert!(
                compile.status.success(),
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            );
        }
        assert!(exe.exists(), "C reference binary missing at {}", exe.display());
        exe
    })
    .as_path()
}

/// Spawn `program`, write `input` to its stdin, and collect its full output.
fn run(program: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let owned = input.to_vec();
        // Write on a helper thread so a program that never drains stdin cannot
        // deadlock the test against the pipe buffer.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&owned);
            let _ = stdin.flush();
        });
    }

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to collect output of {}: {e}", program.display()))
}

fn describe(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s.push('"');
    s
}

/// Core assertion: stdout, stderr and exit status must all agree.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(Path::new(RUST_BIN), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {})\n  C   : {}\n  Rust: {}",
        describe(input),
        describe(&c.stdout),
        describe(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {})\n  C   : {}\n  Rust: {}",
        describe(input),
        describe(&c.stderr),
        describe(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status mismatch for {label} (input {}): C={:?} Rust={:?}",
        describe(input),
        c.status,
        r.status
    );
    assert_eq!(
        c.status.success(),
        r.status.success(),
        "exit success mismatch for {label} (input {})",
        describe(input)
    );
}

#[track_caller]
fn check_all(cases: &[(&str, &[u8])]) {
    for (label, input) in cases {
        assert_same(label, input);
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and run.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"1\n");
    let r = run(Path::new(RUST_BIN), b"1\n");
    assert_eq!(c.stdout, b"01000000\n".to_vec(), "C reference output changed");
    assert_eq!(r.stdout, c.stdout);
    assert_eq!(c.status.code(), Some(0));
    assert_eq!(r.status.code(), Some(0));
}

// ---------------------------------------------------------------------------
// Phase B: the input classes main() branches on.
//
// main() is `int x = 0; scanf("%d", &x); driver(x);` so the branch structure
// lives entirely inside scanf's %d directive:
//   * input failure at EOF before any non-whitespace  -> x untouched (0)
//   * matching failure (no digits after optional sign) -> x untouched (0)
//   * successful conversion                           -> strtol value, narrowed to int
// print_hex then always writes sizeof(int)==4 bytes as %02x plus one newline,
// so stdout is always exactly 9 bytes and the exit status is always 0.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_leaves_x_zero() {
    // EOF before any conversion: scanf returns EOF and never touches x.
    assert_same("empty stdin", b"");
}

#[test]
fn single_item_happy_path() {
    check_all(&[
        ("single zero", b"0"),
        ("single zero with newline", b"0\n"),
        ("single small value", b"42"),
        ("single small value with newline", b"42\n"),
        ("single value, no trailing newline", b"7"),
        ("negative one", b"-1"),
        ("explicit plus", b"+7"),
        ("negative zero", b"-0"),
        ("positive zero", b"+0"),
    ]);
}

#[test]
fn whitespace_is_skipped_across_newlines() {
    // %d skips leading whitespace, and that skip crosses newlines (unlike fgets).
    check_all(&[
        ("spaces then value", b"    5"),
        ("newlines then value", b"\n\n\n5\n"),
        ("mixed whitespace then value", b" \t\n \r\n\x0b\x0c 5\n"),
        ("crlf around value", b"\r\n7\r\n"),
        ("vertical tab and form feed", b"\x0b\x0c9"),
        ("whitespace only", b"  \n\t\n "),
        ("single newline only", b"\n"),
        ("single space only", b" "),
        ("tab only", b"\t"),
        ("many newlines only", b"\n\n\n\n\n\n\n\n"),
    ]);
}

#[test]
fn only_the_first_conversion_is_consumed() {
    // scanf is called once, so trailing tokens are irrelevant to the output.
    check_all(&[
        ("two values separated by space", b"1 2"),
        ("two values separated by newline", b"1\n2\n"),
        ("value then letters", b"12abc"),
        ("value then punctuation", b"12,34"),
        ("value then sign", b"12-34"),
        ("many trailing tokens", b"3 4 5 6 7 8 9\n"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase B/C: the error / matching-failure paths.
// ---------------------------------------------------------------------------

#[test]
fn matching_failure_leaves_x_zero() {
    check_all(&[
        ("letters", b"abc"),
        ("single letter", b"a"),
        ("sign then EOF", b"-"),
        ("plus then EOF", b"+"),
        ("sign then newline", b"-\n"),
        ("sign then letter", b"-a"),
        ("sign then space then digit", b"- 5"),
        ("double minus", b"--5"),
        ("plus then minus", b"+-5"),
        ("minus then plus", b"-+5"),
        ("leading dot", b".5"),
        ("leading e", b"e5"),
        ("leading NUL byte", b"\x005"),
        ("tilde", b"~1"),
        ("utf8 lead byte", b"\xc3\xa95"),
        ("all high bytes", b"\xff\xfe\xfd"),
        ("underscore", b"_1"),
        ("hash", b"#1"),
        ("whitespace then sign then EOF", b"   +"),
        ("whitespace then letter", b"   z"),
    ]);
}

#[test]
fn hex_and_octal_prefixes_are_not_special_for_percent_d() {
    // %d is decimal only: "0x10" converts the leading "0" and stops at 'x'.
    check_all(&[
        ("0x prefix", b"0x10"),
        ("0X prefix", b"0X10"),
        ("leading zeros are decimal, not octal", b"010"),
        ("0b prefix", b"0b101"),
        ("bare zero then letter", b"0z"),
    ]);
}

// ---------------------------------------------------------------------------
// Phase C: boundaries, truncation, overflow and signedness.
//
// glibc converts the digit run with strtol (saturating at LONG_MIN/LONG_MAX)
// and then narrows the long to int for %d, so values above INT_MAX wrap.
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries_and_narrowing() {
    check_all(&[
        ("INT_MAX", b"2147483647"),
        ("INT_MAX + 1 wraps to INT_MIN", b"2147483648"),
        ("INT_MIN", b"-2147483648"),
        ("INT_MIN - 1 wraps to INT_MAX", b"-2147483649"),
        ("UINT_MAX becomes -1", b"4294967295"),
        ("2^32 truncates to 0", b"4294967296"),
        ("2^32 + 1 truncates to 1", b"4294967297"),
        ("2^32 - 2", b"4294967294"),
        ("negative 2^32", b"-4294967296"),
        ("negative 2^32 - 1", b"-4294967297"),
        ("2^31 + 5", b"2147483653"),
        ("2^33", b"8589934592"),
    ]);
}

#[test]
fn long_range_saturation() {
    check_all(&[
        ("LONG_MAX", b"9223372036854775807"),
        ("LONG_MAX + 1 saturates", b"9223372036854775808"),
        ("LONG_MAX + 2 saturates", b"9223372036854775809"),
        ("LONG_MIN", b"-9223372036854775808"),
        ("LONG_MIN - 1 saturates", b"-9223372036854775809"),
        ("LONG_MIN - 2 saturates", b"-9223372036854775810"),
        ("far above LONG_MAX", b"99999999999999999999999999999"),
        ("far below LONG_MIN", b"-99999999999999999999999999999"),
        ("saturated then trailing junk", b"123456789012345678901234567890abc"),
    ]);
}

#[test]
fn leading_zeros_do_not_change_magnitude() {
    check_all(&[
        ("many leading zeros then 5", b"0000000000000000000000005"),
        ("many leading zeros then 42", b"000000000000000000000000000000000042"),
        ("leading zeros then a saturating value", b"00009223372036854775808"),
        ("leading zeros with sign", b"-000000000000000000000042"),
        ("only zeros", b"00000000000000000000000000"),
    ]);
}

/// Every conversion result is exactly 4 little-endian bytes plus a newline;
/// this pins the print_hex formatting (two lowercase hex digits, no separator,
/// one trailing newline, nothing on stderr, exit 0).
#[test]
fn output_shape_is_nine_bytes_and_exit_zero() {
    for input in [
        &b""[..], b"0", b"1", b"-1", b"abc", b"2147483648", b"-9223372036854775809",
    ] {
        let c = run(c_bin(), input);
        assert_eq!(
            c.stdout.len(),
            9,
            "C stdout for {} was {} bytes",
            describe(input),
            c.stdout.len()
        );
        assert_eq!(c.stdout[8], b'\n');
        assert!(c.stderr.is_empty());
        assert_eq!(c.status.code(), Some(0));
        assert_same("output shape", input);
    }
}

// ---------------------------------------------------------------------------
// Phase C: stdin buffering boundaries. The Rust reader refills a 4096-byte
// buffer, so exercise inputs whose whitespace run, sign and digit run all land
// on and across that boundary. glibc's own buffer is a different size, which is
// exactly why these must be compared rather than reasoned about.
// ---------------------------------------------------------------------------

#[test]
fn buffer_boundary_inputs() {
    for pad in [4094usize, 4095, 4096, 4097, 8190, 8191, 8192, 8193] {
        let mut input = vec![b' '; pad];
        input.extend_from_slice(b"77");
        assert_same(&format!("{pad} spaces then 77"), &input);

        let mut input = vec![b' '; pad];
        input.extend_from_slice(b"-1234567890");
        assert_same(&format!("{pad} spaces then negative value"), &input);

        let mut input = vec![b'\n'; pad];
        input.extend_from_slice(b"9");
        assert_same(&format!("{pad} newlines then 9"), &input);
    }
}

#[test]
fn very_long_digit_runs() {
    // Digit runs far longer than any buffer, both saturating and not.
    let mut ones = vec![b'1'; 5000];
    assert_same("5000 ones", &ones);
    ones.insert(0, b'-');
    assert_same("minus then 5000 ones", &ones);

    let mut zeros = vec![b'0'; 5000];
    zeros.push(b'3');
    assert_same("5000 zeros then 3", &zeros);

    let mut zeros = vec![b'0'; 4095];
    zeros.extend_from_slice(b"2147483648");
    assert_same("4095 zeros then INT_MAX+1", &zeros);

    let mut huge = vec![b'9'; 100_000];
    assert_same("100000 nines", &huge);
    huge.insert(0, b'-');
    assert_same("minus then 100000 nines", &huge);
}

// ---------------------------------------------------------------------------
// Phase C: exhaustive-ish sweeps. These are cheap and catch anything the
// hand-written classes above missed.
// ---------------------------------------------------------------------------

#[test]
fn sweep_every_single_byte_input() {
    // One byte of input, for all 256 possible bytes: covers every first-character
    // dispatch inside %d (whitespace, sign, digit, everything else) plus the
    // immediately-following EOF.
    for b in 0u16..=255 {
        let byte = b as u8;
        assert_same(&format!("single byte {byte:#04x}"), &[byte]);
    }
}

#[test]
fn sweep_two_byte_inputs_over_significant_bytes() {
    // Pairs drawn from the bytes %d treats specially, to cover sign-then-X,
    // digit-then-X and whitespace-then-X transitions including EOF handling.
    let significant: &[u8] = b" \t\n\r\x0b\x0c+-0123456789.eExX\0\xff";
    for &a in significant {
        for &b in significant {
            assert_same(&format!("pair {a:#04x} {b:#04x}"), &[a, b]);
        }
    }
}

#[test]
fn sweep_decimal_values_around_every_power_of_two() {
    // Values straddling each power of two up to 2^70 exercise truncation to int
    // and saturation to long, in both signs.
    for shift in 0..71u32 {
        let base: i128 = 1i128 << shift;
        for delta in [-2i128, -1, 0, 1, 2] {
            let v = base + delta;
            assert_same(&format!("value {v}"), v.to_string().as_bytes());
            let n = -v;
            assert_same(&format!("value {n}"), n.to_string().as_bytes());
        }
    }
}

#[test]
fn sweep_small_contiguous_range() {
    for v in -300i32..=300 {
        assert_same(&format!("small value {v}"), v.to_string().as_bytes());
    }
}

/// Deterministic pseudo-random differential sweep over structured and raw-byte
/// inputs. Uses a fixed seed so failures are reproducible.
#[test]
fn randomized_differential_sweep() {
    // xorshift64*, so the test has no external dependencies.
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
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
        fn pick(&mut self, s: &[u8]) -> u8 {
            s[self.below(s.len() as u64) as usize]
        }
    }

    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let alphabet: &[u8] = b" \t\n\r\x0b\x0c+-0123456789aAxX.eE_\0\x7f\xff";

    for i in 0..1500 {
        let input: Vec<u8> = match rng.below(5) {
            // Raw random bytes, short.
            0 => (0..rng.below(12)).map(|_| rng.next() as u8).collect(),
            // Random string over the significant alphabet.
            1 => (0..rng.below(30)).map(|_| rng.pick(alphabet)).collect(),
            // Optional whitespace, optional sign, random digit run.
            2 => {
                let mut v = vec![b' '; rng.below(5) as usize];
                match rng.below(3) {
                    0 => v.push(b'+'),
                    1 => v.push(b'-'),
                    _ => {}
                }
                for _ in 0..rng.below(26) {
                    v.push(rng.pick(b"0123456789"));
                }
                v
            }
            // A plain decimal number spanning well past 64 bits.
            3 => {
                let mag = (u128::from(rng.next()) << 6) | u128::from(rng.below(64));
                let mut s = mag.to_string();
                if rng.below(2) == 0 {
                    s.insert(0, '-');
                }
                s.into_bytes()
            }
            // Padding that lands near a buffer refill, then a number.
            _ => {
                let pad = 4080 + rng.below(40) as usize;
                let mut v = vec![rng.pick(b" \t\n"); pad];
                if rng.below(2) == 0 {
                    v.push(b'-');
                }
                for _ in 0..(1 + rng.below(25)) {
                    v.push(rng.pick(b"0123456789"));
                }
                v
            }
        };
        assert_same(&format!("random case {i}"), &input);
    }
}
