//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses, feed them identical stdin, and compare stdout, stderr and
//! exit status byte for byte.
//!
//! The Rust program is never linked as a library — it is driven exactly the way
//! a shell would drive it, because that is how the two are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary under test (built automatically by cargo).
const RUST_BIN: &str = env!("CARGO_BIN_EXE_driver");

fn workspace_root() -> PathBuf {
    // <root>/translation/Cargo.toml -> <root>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Build `c_src` with CMake (once per test binary) and return the C executable.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let root = workspace_root();
        let c_src = root.join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");

        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");

            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake ..` — is cmake installed?");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&cfg.stdout),
                String::from_utf8_lossy(&cfg.stderr)
            );

            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("failed to run `cmake --build .`");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }

        assert!(exe.exists(), "C executable missing at {}", exe.display());
        exe
    })
    .as_path()
}

/// Run `prog` with `input` on stdin, capturing stdout/stderr/status.
fn run_prog(prog: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", prog.display()));

    let mut stdin = child.stdin.take().expect("piped stdin");
    let data = input.to_vec();
    // Write on a helper thread: neither program consumes all of stdin, so a
    // large input could otherwise block us on a full pipe. Write errors
    // (EPIPE after the child exits) are expected and ignored.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
        let _ = stdin.flush();
    });

    let out = child.wait_with_output().expect("wait_with_output");
    let _ = writer.join();
    out
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Assert the two programs agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(name: &str, input: &[u8]) {
    let c = run_prog(c_bin(), input);
    let r = run_prog(Path::new(RUST_BIN), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for case `{name}` (input {:?})\n C: \"{}\"\n R: \"{}\"",
        show(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for case `{name}` (input {:?})\n C: \"{}\"\n R: \"{}\"",
        show(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status differs for case `{name}` (input {:?}): C={:?} R={:?}",
        show(input),
        c.status,
        r.status
    );
}

// ---------------------------------------------------------------------------
// Phase A — the built binaries are runnable and produce the reference output.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run_and_c_output_is_the_documented_reference() {
    let out = run_prog(c_bin(), b"3\n");
    assert!(out.status.success());
    let expected = "\
The house has 2 floors, 5 bedrooms, and 2.5 bathrooms
The house has 3 floors, 5 bedrooms, and 2.5 bathrooms
The house has 3 floors, 5 bedrooms, and 3.5 bathrooms
The house has 3 floors, 8 bedrooms, and 3.5 bathrooms
The house has 3 floors, 8 bedrooms, and 3.5 bathrooms
The house has 4 floors, 8 bedrooms, and 3.5 bathrooms
The house has 4 floors, 8 bedrooms, and 4.5 bathrooms
The house has 4 floors, 11 bedrooms, and 4.5 bathrooms
";
    assert_eq!(String::from_utf8_lossy(&out.stdout), expected);
    assert_same("reference: 3", b"3\n");
}

// ---------------------------------------------------------------------------
// Phase B — inputs the C actually branches on.
//
// main():        fgets (may fail) -> parse_val -> success path | error path
// parse_val():   endp != str  &&  errno == 0  &&  INT_MIN <= tmp <= INT_MAX
// run():         mutates the house; called twice on the SAME house
// ---------------------------------------------------------------------------

/// `fgets` returns NULL and leaves the zero-initialized buffer empty.
#[test]
fn empty_input_reaches_the_error_path() {
    assert_same("empty stdin", b"");
}

/// A single item: the smallest well-formed line.
#[test]
fn single_item() {
    assert_same("single digit", b"5\n");
    assert_same("single digit, no newline", b"1");
    assert_same("zero", b"0\n");
    assert_same("one", b"1\n");
    assert_same("negative one", b"-1\n");
}

/// `strtol` finds no digits -> `endp == str` -> error path.
#[test]
fn no_conversion_performed() {
    for (name, input) in [
        ("newline only", &b"\n"[..]),
        ("spaces only", b"   \n"),
        ("all whitespace kinds", b" \t\n"),
        ("alphabetic", b"abc\n"),
        ("plus sign only", b"+\n"),
        ("minus sign only", b"-\n"),
        ("plus then newline only", b"  +\n"),
        ("double minus", b"--5\n"),
        ("double plus", b"++5\n"),
        ("sign separated from digits", b" - 5\n"),
        ("leading dot", b".5\n"),
        ("underscore", b"_5\n"),
        ("comma", b",5\n"),
        ("NUL first byte", b"\x005\n"),
        ("empty first line then a number", b"\n5\n"),
        ("empty first line, no digits at all", b"\n\n"),
    ] {
        assert_same(name, input);
    }
}

/// Successful conversions, including `strtol`'s tolerance of leading
/// whitespace, an explicit sign and trailing garbage.
#[test]
fn successful_conversions_with_strtol_quirks() {
    for (name, input) in [
        ("leading spaces", &b"  42\n"[..]),
        ("leading tab", b"\t9\n"),
        ("mixed leading whitespace", b" \t\x0b\x0c\r 8\n"),
        ("explicit plus", b"+7\n"),
        ("explicit minus", b"-3\n"),
        ("negative zero", b"-0\n"),
        ("plus zero", b"+0\n"),
        ("leading zeros", b"007\n"),
        ("many leading zeros", b"00000000000000000000000000005\n"),
        ("trailing garbage", b"12abc\n"),
        ("base 10 stops at x", b"0x10\n"),
        ("exponent notation truncated", b"1e5\n"),
        ("decimal truncated at dot", b"4.9\n"),
        ("trailing spaces", b"42   \n"),
        ("CRLF keeps a trailing CR", b"5\r\n"),
        ("no newline at all", b"7"),
        ("embedded NUL after digits", b"5\x00abc\n"),
        ("second line ignored", b"5\n6\n"),
        ("digits then NUL then newline", b"6\x00\n"),
    ] {
        assert_same(name, input);
    }
}

/// The `tmp >= INT_MIN && tmp <= INT_MAX` range check, on both sides.
#[test]
fn int_range_boundaries() {
    for (name, input) in [
        ("INT_MAX - 1", &b"2147483646\n"[..]),
        ("INT_MAX", b"2147483647\n"),
        ("INT_MAX + 1", b"2147483648\n"),
        ("INT_MIN + 1", b"-2147483647\n"),
        ("INT_MIN", b"-2147483648\n"),
        ("INT_MIN - 1", b"-2147483649\n"),
        ("well past INT_MAX", b"3000000000\n"),
        ("well below INT_MIN", b"-3000000000\n"),
    ] {
        assert_same(name, input);
    }
}

/// `errno == ERANGE` from `strtol`, and the LONG boundaries just below it.
#[test]
fn long_range_and_erange() {
    for (name, input) in [
        ("LONG_MAX - 1", &b"9223372036854775806\n"[..]),
        ("LONG_MAX", b"9223372036854775807\n"),
        ("LONG_MAX + 1 (ERANGE)", b"9223372036854775808\n"),
        ("LONG_MIN", b"-9223372036854775808\n"),
        ("LONG_MIN - 1 (ERANGE)", b"-9223372036854775809\n"),
        ("far past LONG_MAX (ERANGE)", b"99999999999999999999999999\n"),
        ("far below LONG_MIN (ERANGE)", b"-99999999999999999999999999\n"),
        ("ERANGE with trailing garbage", b"99999999999999999999zz\n"),
    ] {
        assert_same(name, input);
    }
}

/// Signed overflow of `house->bedrooms += extra_bedrooms`, performed twice on
/// the same house exactly as the C does it.
#[test]
fn bedroom_accumulation_overflow() {
    for (name, input) in [
        ("INT_MAX overflows bedrooms twice", &b"2147483647\n"[..]),
        ("INT_MIN underflows bedrooms twice", b"-2147483648\n"),
        ("large positive", b"2147483640\n"),
        ("large negative", b"-2147483640\n"),
        ("half of INT_MAX", b"1073741824\n"),
    ] {
        assert_same(name, input);
    }
}

// ---------------------------------------------------------------------------
// Phase C — the buffer-length paths, and inputs no earlier case reaches.
//
// `char in[100]` + `fgets(in, sizeof(in), stdin)` reads at most 99 bytes and
// does NOT read across a newline.
// ---------------------------------------------------------------------------

#[test]
fn fgets_99_byte_boundary() {
    // 98 chars + '\n' == exactly the 99 bytes fgets will take.
    let mut a = vec![b'7'; 98];
    a.push(b'\n');
    assert_same("98 chars plus newline (exactly 99)", &a);

    // 99 chars + '\n': the newline is NOT read.
    let mut b = vec![b'8'; 99];
    b.push(b'\n');
    assert_same("99 chars plus newline (newline unread)", &b);

    // 100 chars: truncated at 99.
    assert_same("100 digits", &vec![b'9'; 100]);

    // A valid number whose digits are cut by the 99-byte limit: "1" * 120
    // truncates to 99 ones -> ERANGE -> error path.
    assert_same("120 ones truncated to 99", &vec![b'1'; 120]);

    // Value only becomes representable because of truncation.
    let mut c = b"1".to_vec();
    c.extend(std::iter::repeat(b'0').take(105));
    assert_same("1 followed by 105 zeros", &c);

    // Digits pushed past byte 99 by leading whitespace -> no conversion.
    let mut d = vec![b' '; 200];
    d.push(b'5');
    d.push(b'\n');
    assert_same("digits beyond the 99-byte window", &d);

    // Number that fits, padded with spaces up to and past the boundary.
    for pad in [96usize, 97, 98, 99, 100, 150] {
        let mut e = b"1".to_vec();
        e.extend(std::iter::repeat(b' ').take(pad));
        e.push(b'\n');
        assert_same(&format!("1 plus {pad} spaces"), &e);
    }

    // Sign lands on the last readable byte.
    let mut f = vec![b' '; 98];
    f.push(b'-');
    f.extend_from_slice(b"5\n");
    assert_same("sign at the 99-byte boundary", &f);

    // INT_MAX sitting right at the truncation boundary.
    let mut g = vec![b' '; 89];
    g.extend_from_slice(b"2147483647\n");
    assert_same("INT_MAX ending at the boundary", &g);
}

#[test]
fn fgets_does_not_read_across_newlines() {
    // Every one of these has a valid number on a later line only.
    assert_same("garbage line then number", b"abc\n5\n");
    assert_same("blank line then number", b"\n42\n");
    assert_same("number then garbage line", b"5\nabc\n");
    assert_same("three lines", b"1\n2\n3\n");
}

#[test]
fn embedded_nul_bytes_terminate_the_c_string() {
    assert_same("NUL only", b"\x00");
    assert_same("NUL then newline", b"\x00\n");
    assert_same("NUL between digits", b"1\x002\n");
    assert_same("spaces, NUL, digits", b"  \x0042\n");
    assert_same("sign, NUL, digits", b"-\x005\n");
    assert_same("NUL after full number", b"123\x00456\n");
}

#[test]
fn non_ascii_and_high_bytes() {
    assert_same("high bytes", b"\xff\xfe\n");
    assert_same("digit then high byte", b"5\xff\n");
    assert_same("utf8 minus sign", "\u{2212}5\n".as_bytes());
    assert_same("nbsp then digits", "\u{00a0}5\n".as_bytes());
}

/// Deterministic pseudo-random byte fuzz over the alphabet the parser cares
/// about, crossing the 99-byte buffer boundary.
#[test]
fn randomized_differential_fuzz() {
    const ALPHABET: &[u8] = b"0123456789+- \t\n\r\x0b\x0c\x00abcxX.eE,;9";
    const LENGTHS: [usize; 14] = [0, 1, 2, 3, 5, 10, 50, 97, 98, 99, 100, 101, 120, 200];

    // xorshift64* — reproducible without a dependency.
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545F4914F6CDD1D)
    };

    for i in 0..600 {
        let len = LENGTHS[(next() % LENGTHS.len() as u64) as usize];
        let input: Vec<u8> = (0..len)
            .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
            .collect();
        assert_same(&format!("fuzz #{i} (len {len})"), &input);
    }
}

/// Numeric strings around every interesting magnitude, with several suffixes.
#[test]
fn numeric_matrix() {
    let bodies: Vec<String> = [
        "0",
        "-0",
        "+0",
        "1",
        "-1",
        "2147483646",
        "2147483647",
        "2147483648",
        "-2147483647",
        "-2147483648",
        "-2147483649",
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "00000000000000000000000000005",
    ]
    .iter()
    .map(|s| s.to_string())
    .chain([
        "1".repeat(99),
        "9".repeat(99),
        format!("-{}", "9".repeat(98)),
    ])
    .collect();

    for body in &bodies {
        for suffix in ["", "\n", " \n", "x\n", "\r\n", "\n7\n"] {
            let input = format!("{body}{suffix}");
            assert_same(&format!("numeric {body:?}{suffix:?}"), input.as_bytes());
        }
    }
}

/// `fgets` fails outright: stdin is a directory (EISDIR). The zero-initialized
/// buffer means the error path is taken and the exit status is still 0.
#[test]
fn stdin_read_error() {
    use std::fs::File;

    let dir = File::open(workspace_root()).expect("open workspace root as a file");
    let dir2 = File::open(workspace_root()).expect("open workspace root as a file");

    let c = Command::new(c_bin())
        .stdin(Stdio::from(dir))
        .output()
        .expect("run C with a directory on stdin");
    let r = Command::new(RUST_BIN)
        .stdin(Stdio::from(dir2))
        .output()
        .expect("run Rust with a directory on stdin");

    assert_eq!(c.stdout, r.stdout, "stdout differs when stdin is a directory");
    assert_eq!(c.stderr, r.stderr, "stderr differs when stdin is a directory");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status differs when stdin is a directory"
    );
}

/// stdin at immediate EOF from /dev/null — the other way `fgets` returns NULL.
#[test]
fn stdin_dev_null() {
    use std::fs::File;

    let c = Command::new(c_bin())
        .stdin(Stdio::from(File::open("/dev/null").unwrap()))
        .output()
        .unwrap();
    let r = Command::new(RUST_BIN)
        .stdin(Stdio::from(File::open("/dev/null").unwrap()))
        .output()
        .unwrap();

    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
    assert_eq!(c.status.code(), r.status.code());
}

/// Nothing is written to stderr, and the exit status is always 0 — on both the
/// success and the error path.
#[test]
fn stderr_always_empty_and_exit_status_always_zero() {
    for input in [&b""[..], b"5\n", b"abc\n", b"9999999999999999999999\n"] {
        let c = run_prog(c_bin(), input);
        assert!(c.stderr.is_empty(), "C wrote to stderr for {:?}", show(input));
        assert_eq!(c.status.code(), Some(0));

        let r = run_prog(Path::new(RUST_BIN), input);
        assert!(r.stderr.is_empty(), "Rust wrote to stderr for {:?}", show(input));
        assert_eq!(r.status.code(), Some(0));
    }
}
