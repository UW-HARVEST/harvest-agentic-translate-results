//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with identical stdin, and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! The Rust code is NEVER called as a library here — only the built binary is
//! driven, exactly the way a shell would drive it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Repository root (the directory holding `c_src/` and `translation/`).
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test (built by cargo for this test run).
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary, building it with CMake on first use.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("failed to spawn cmake (is cmake installed?)");
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
                .expect("failed to spawn cmake --build");
            assert!(
                bld.status.success(),
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(
            exe.exists(),
            "C binary missing after build: {}",
            exe.display()
        );
        exe
    })
}

/// Run `program` with `stdin_bytes` piped to its standard input.
fn run(program: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));
    {
        let mut sin = child.stdin.take().expect("piped stdin");
        // The child may legitimately stop reading; a write error is not a test
        // failure by itself (both programs are treated the same way).
        let _ = sin.write_all(stdin_bytes);
        let _ = sin.flush();
    }
    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait on {}: {e}", program.display()))
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Assert that both binaries agree on stdout, stderr and exit status.
#[track_caller]
fn assert_same(label: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(rust_bin(), stdin_bytes);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout mismatch\n  input : \"{}\"\n  C     : \"{}\"\n  Rust  : \"{}\"",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr mismatch\n  input : \"{}\"\n  C     : \"{}\"\n  Rust  : \"{}\"",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "[{label}] exit code mismatch\n  input : \"{}\"\n  C     : {:?}\n  Rust  : {:?}",
        show(stdin_bytes),
        c.status,
        r.status
    );
    assert_eq!(
        c.status.success(),
        r.status.success(),
        "[{label}] exit success mismatch: C={:?} Rust={:?}",
        c.status,
        r.status
    );
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"1\n");
    let r = run(rust_bin(), b"1\n");
    assert!(!c.stdout.is_empty(), "C produced no stdout");
    assert!(!r.stdout.is_empty(), "Rust produced no stdout");
    assert_eq!(c.status.code(), Some(0));
    assert_eq!(r.status.code(), Some(0));
}

/// Pin the exact expected text so a regression in *both* directions is caught.
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

#[test]
fn golden_error_output() {
    let c = run(c_bin(), b"nope\n");
    let r = run(rust_bin(), b"nope\n");
    assert_eq!(String::from_utf8_lossy(&c.stdout), "An error occurred\n");
    assert_eq!(String::from_utf8_lossy(&r.stdout), "An error occurred\n");
    assert_eq!(c.status.code(), Some(0));
    assert_eq!(r.status.code(), Some(0));
}

// ---------------------------------------------------------------------------
// Phase B: the input classes main() branches on.
// ---------------------------------------------------------------------------

/// `fgets` returns NULL / leaves `in` as "" -> parse_val fails -> error path.
#[test]
fn empty_input_takes_error_path() {
    assert_same("empty stdin (EOF immediately)", b"");
}

#[test]
fn newline_only_takes_error_path() {
    // strtol skips the '\n' as whitespace, then finds no digits.
    assert_same("bare newline", b"\n");
    assert_same("CRLF", b"\r\n");
    assert_same("whitespace only", b"  \t \n");
    assert_same("all C isspace chars", b" \t\n\x0b\x0c\r");
}

#[test]
fn single_valid_value() {
    assert_same("0", b"0\n");
    assert_same("1", b"1\n");
    assert_same("-0", b"-0\n");
    assert_same("no trailing newline", b"3");
    assert_same("single digit, EOF", b"7");
}

#[test]
fn signs_and_leading_whitespace() {
    assert_same("negative", b"-7\n");
    assert_same("explicit plus", b"+7\n");
    assert_same("leading spaces", b"   12\n");
    assert_same("leading tab", b"\t12\n");
    assert_same("leading newline then digits", b"\n12\n");
    assert_same("mixed leading whitespace", b" \t\r\n \x0b\x0c-12\n");
}

/// `endp != str` is the only "did we convert anything" check, so trailing
/// garbage is accepted while leading garbage is rejected.
#[test]
fn trailing_garbage_is_accepted() {
    assert_same("digits then letters", b"12abc\n");
    assert_same("digits then space then digits", b"12 34\n");
    assert_same("decimal point", b"1.9\n");
    assert_same("digits then punctuation", b"5!!!\n");
    assert_same("octal-looking", b"010\n");
    assert_same("hex-looking (base 10 stops at x)", b"0x10\n");
    assert_same("comma separated", b"7,8,9\n");
}

#[test]
fn leading_garbage_is_rejected() {
    assert_same("letters", b"abc\n");
    assert_same("lone minus", b"-\n");
    assert_same("lone plus", b"+\n");
    assert_same("sign then space then digits", b"- 5\n");
    assert_same("sign then letters", b"-abc\n");
    assert_same("leading dot", b".5\n");
    assert_same("double sign", b"--5\n");
    assert_same("plus minus", b"+-5\n");
    assert_same("underscore", b"_5\n");
    assert_same("hash", b"#5\n");
}

// ---------------------------------------------------------------------------
// Phase B/C: the numeric range checks (INT_MIN/INT_MAX and strtol ERANGE).
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    assert_same("INT_MAX", b"2147483647\n");
    assert_same("INT_MAX - 1", b"2147483646\n");
    assert_same("INT_MIN", b"-2147483648\n");
    assert_same("INT_MIN + 1", b"-2147483647\n");
}

/// In range for `long` but outside `[INT_MIN, INT_MAX]` -> the range check in
/// parse_val fails even though strtol succeeded.
#[test]
fn outside_int_range_but_inside_long_range() {
    assert_same("INT_MAX + 1", b"2147483648\n");
    assert_same("INT_MIN - 1", b"-2147483649\n");
    assert_same("4294967296", b"4294967296\n");
    assert_same("-4294967296", b"-4294967296\n");
    assert_same("LONG_MAX", b"9223372036854775807\n");
    assert_same("LONG_MIN", b"-9223372036854775808\n");
}

/// strtol sets ERANGE -> `errno == 0` fails -> error path.
#[test]
fn strtol_erange() {
    assert_same("LONG_MAX + 1", b"9223372036854775808\n");
    assert_same("LONG_MIN - 1", b"-9223372036854775809\n");
    assert_same("huge positive", b"999999999999999999999999999999\n");
    assert_same("huge negative", b"-999999999999999999999999999999\n");
    assert_same("many zeros then digits", b"000000000000000000000000000001\n");
    assert_same("padded INT_MAX", b"0000000000002147483647\n");
    assert_same("padded INT_MAX + 1", b"0000000000002147483648\n");
}

/// Signed `int` overflow in `bedrooms += extra_bedrooms`, as the C performs it.
#[test]
fn bedroom_addition_overflow() {
    assert_same("INT_MAX bedrooms overflow", b"2147483647\n");
    assert_same("INT_MAX - 5", b"2147483642\n");
    assert_same("INT_MIN bedrooms underflow", b"-2147483648\n");
    assert_same("large negative", b"-2000000000\n");
    assert_same("large positive", b"2000000000\n");
}

// ---------------------------------------------------------------------------
// Phase C: fgets buffer limits, embedded NULs, and multi-line input.
// ---------------------------------------------------------------------------

/// `fgets(in, 100, stdin)` reads at most 99 bytes and does NOT read across the
/// first newline (unlike scanf).
#[test]
fn fgets_stops_at_first_newline() {
    assert_same("two lines, first valid", b"5\n9\n");
    assert_same("two lines, first invalid", b"abc\n9\n");
    assert_same("second line never read", b"1\nthis is never read at all\n");
    assert_same("empty first line", b"\n5\n");
}

#[test]
fn fgets_buffer_boundary() {
    // 98 digits + newline: fits entirely (99 bytes).
    let mut v = vec![b'1'; 98];
    v.push(b'\n');
    assert_same("98 digits + newline", &v);

    // 99 digits: exactly fills the buffer, newline not consumed.
    assert_same("99 digits", &vec![b'1'; 99]);

    // 100 digits: only the first 99 are seen.
    assert_same("100 digits", &vec![b'1'; 100]);

    // 200 digits: truncation at 99.
    assert_same("200 digits", &vec![b'9'; 200]);

    // 99 spaces then digits: the digits fall outside the buffer -> error path.
    let mut v = vec![b' '; 99];
    v.extend_from_slice(b"42\n");
    assert_same("99 spaces then 42", &v);

    // 98 spaces then digits: exactly one digit makes it in.
    let mut v = vec![b' '; 98];
    v.extend_from_slice(b"42\n");
    assert_same("98 spaces then 42", &v);

    // 97 spaces then "-1": sign and one digit fit.
    let mut v = vec![b' '; 97];
    v.extend_from_slice(b"-12\n");
    assert_same("97 spaces then -12", &v);

    // A sign as the very last byte in the buffer -> no digits -> error path.
    let mut v = vec![b' '; 98];
    v.extend_from_slice(b"-5\n");
    assert_same("sign at last buffer byte", &v);
}

/// The buffer is zero-initialized, so an embedded NUL terminates the C string.
#[test]
fn embedded_nul_bytes() {
    assert_same("NUL first", b"\x005\n");
    assert_same("NUL after digits", b"5\x00abc\n");
    assert_same("NUL only", b"\x00");
    assert_same("NUL then newline", b"\x00\n");
    assert_same("digits, NUL, more digits", b"12\x0034\n");
    assert_same("space, NUL, digits", b" \x009\n");
}

#[test]
fn non_ascii_and_binary_input() {
    assert_same("utf8 text", "héllo\n".as_bytes());
    assert_same("utf8 after digits", "5é\n".as_bytes());
    assert_same("high bytes", b"\xff\xfe\xfd\n");
    assert_same("high bytes after digits", b"9\xff\xfe\n");
    assert_same("0x80 leading", b"\x80\x815\n");
    assert_same("del char", b"\x7f5\n");
    assert_same("bell", b"\x075\n");
}

// ---------------------------------------------------------------------------
// Phase C: broad sweep + deterministic pseudo-random fuzz.
// ---------------------------------------------------------------------------

#[test]
fn sweep_many_decimal_values() {
    let values: &[i64] = &[
        -100, -99, -50, -17, -10, -9, -2, -1, 0, 1, 2, 3, 4, 5, 9, 10, 17, 42, 99, 100, 1000,
        65535, 65536, 100000, 1000000, 16777216, 1073741823, 1073741824, 2147483645, 2147483646,
        2147483647, -2147483646, -2147483647, -2147483648, 2147483649, -2147483650, 4294967295,
        -4294967295, 9223372036854775807, -9223372036854775807,
    ];
    for v in values {
        assert_same(&format!("value {v}"), format!("{v}\n").as_bytes());
        assert_same(&format!("value {v} no newline"), format!("{v}").as_bytes());
        assert_same(&format!("value {v} padded"), format!("  {v}  \n").as_bytes());
    }
}

#[test]
fn fuzz_deterministic() {
    // xorshift64* so the corpus is reproducible without extra dependencies.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        state
    };

    const POOLS: [&[u8]; 5] = [
        b"0123456789",
        b"0123456789 +-\t\n",
        b"0123456789+-\x00 \n",
        b" \t\n\x0b\x0c\r+-9",
        b"0123456789abcdefx.,\xff\x00\n -+",
    ];

    for i in 0..600u32 {
        let pool = POOLS[(i as usize) % POOLS.len()];
        let len = (next() % 150) as usize;
        let mut input = Vec::with_capacity(len);
        for _ in 0..len {
            input.push(pool[(next() % pool.len() as u64) as usize]);
        }
        assert_same(&format!("fuzz #{i}"), &input);
    }
}

// ---------------------------------------------------------------------------
// Phase C: stdin that is not a pipe with data.
// ---------------------------------------------------------------------------

#[test]
fn stdin_is_empty_dev_null() {
    // Equivalent to `./driver < /dev/null`.
    let null = std::fs::File::open("/dev/null").expect("open /dev/null");
    let c = Command::new(c_bin())
        .stdin(Stdio::from(null))
        .output()
        .expect("run C with /dev/null stdin");
    let null = std::fs::File::open("/dev/null").expect("open /dev/null");
    let r = Command::new(rust_bin())
        .stdin(Stdio::from(null))
        .output()
        .expect("run Rust with /dev/null stdin");
    assert_eq!(c.stdout, r.stdout, "stdout mismatch with /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch with /dev/null stdin");
    assert_eq!(c.status.code(), r.status.code());
    assert_eq!(String::from_utf8_lossy(&c.stdout), "An error occurred\n");
}

/// Command-line arguments are ignored by `main()` (it takes no parameters).
#[test]
fn arguments_are_ignored() {
    for args in [
        vec![],
        vec!["--help".to_string()],
        vec!["1".to_string(), "2".to_string()],
    ] {
        let mut c = Command::new(c_bin());
        let mut r = Command::new(rust_bin());
        c.args(&args);
        r.args(&args);
        let feed = |cmd: &mut Command| -> Output {
            let mut ch = cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn");
            ch.stdin.take().unwrap().write_all(b"4\n").unwrap();
            ch.wait_with_output().expect("wait")
        };
        let co = feed(&mut c);
        let ro = feed(&mut r);
        assert_eq!(co.stdout, ro.stdout, "stdout mismatch with args {args:?}");
        assert_eq!(co.stderr, ro.stderr, "stderr mismatch with args {args:?}");
        assert_eq!(co.status.code(), ro.status.code());
    }
}
