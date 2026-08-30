//! Differential tests: run the original C program and the Rust translation as
//! subprocesses on identical stdin, and require byte-identical stdout, stderr
//! and exit status.
//!
//! Nothing here calls the Rust code as a library; both programs are driven the
//! way a shell would drive them, because that is how they are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Path of the Rust binary under test (built by cargo for us).
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path of the C binary, building it with cmake if it is not there yet.
/// `c_src/` itself is never modified; only the ignored `c_src/build/` tree is
/// written to.
fn c_bin() -> PathBuf {
    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");
    let exe = build.join(if cfg!(windows) { "driver.exe" } else { "driver" });
    if exe.exists() {
        return exe;
    }

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
    assert!(exe.exists(), "C binary missing after build: {}", exe.display());
    exe
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
}

fn run(exe: &Path, stdin_bytes: &[u8], args: &[&str]) -> Run {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));

    {
        let mut sink = child.stdin.take().expect("stdin pipe");
        // The child may exit without draining stdin (e.g. huge inputs); a
        // broken pipe here is not a test failure.
        let _ = sink.write_all(stdin_bytes);
        let _ = sink.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// The core assertion: same stdin (and argv) => same stdout, stderr, status.
fn assert_same_with_args(label: &str, stdin_bytes: &[u8], args: &[&str]) {
    let c = run(&c_bin(), stdin_bytes, args);
    let r = run(&rust_bin(), stdin_bytes, args);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout mismatch for {label} (input {:?})\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(stdin_bytes),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr mismatch for {label} (input {:?})\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(stdin_bytes),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.code, r.code,
        "exit status mismatch for {label} (input {:?}): C={:?} Rust={:?}",
        show(stdin_bytes),
        c.code,
        r.code
    );
}

fn assert_same(label: &str, stdin_bytes: &[u8]) {
    assert_same_with_args(label, stdin_bytes, &[]);
}

fn assert_same_str(label: &str, input: &str) {
    assert_same(label, input.as_bytes());
}

// ---------------------------------------------------------------------------
// Sanity: both programs really do run, and they agree on the baseline case.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_exist_and_run() {
    let c = c_bin();
    let r = rust_bin();
    assert!(c.exists(), "C binary not built: {}", c.display());
    assert!(r.exists(), "Rust binary not built: {}", r.display());
    // driver(0, 0) == 0 | ~0 == -1
    let out = run(&r, b"0 0\n", &[]);
    assert_eq!(out.stdout, b"-1\n");
    assert_eq!(out.code, Some(0));
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C source branches on.
//
// main() is:   int x = 0, y = 0; scanf("%d",&x); scanf("%d",&y); driver(x,y);
// driver() is: printf("%d", x | ~y); puts("");
//
// So the branch structure lives entirely inside the two `scanf` calls: each
// one either converts a value or fails (matching failure / EOF) and leaves the
// variable at its initial 0. Return value is always 0; nothing goes to stderr.
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    // Both scanfs hit EOF immediately: x = y = 0.
    assert_same_str("empty", "");
}

#[test]
fn single_item_only() {
    // First scanf converts, second hits EOF and leaves y == 0.
    for s in ["5", "5\n", "0", "-1", "-1\n", "2147483647", "-2147483648"] {
        assert_same_str("single item", s);
    }
}

#[test]
fn two_items_space_separated() {
    for s in ["3 4", "3 4\n", "0 0", "1 2", "-7 9", "9 -7", "-1 -1"] {
        assert_same_str("two items", s);
    }
}

#[test]
fn scanf_reads_across_newlines() {
    // %d skips *any* leading whitespace, newlines included -- unlike fgets.
    for s in [
        "3\n4",
        "3\n4\n",
        "3\n\n\n4\n",
        "\n\n3\n4",
        "3\r\n4\r\n",
        "\t3\t\n \t4 \t\n",
        "3\u{b}4",
        "3\u{c}4",
    ] {
        assert_same_str("cross-newline", s);
    }
}

#[test]
fn whitespace_only_input() {
    for s in [" ", "   ", "\n", "\n\n\n", "\t\t", "\r\n", " \t\n\u{b}\u{c}\r "] {
        assert_same_str("whitespace only", s);
    }
}

#[test]
fn explicit_signs() {
    for s in [
        "+7 -8", "+7 +8", "-7 -8", "-0 -0", "+0 +0", "-0 0", "+2147483647 -2147483648",
    ] {
        assert_same_str("signs", s);
    }
}

#[test]
fn leading_zeros_are_decimal_not_octal() {
    // %d is base 10: "0012" is twelve, not ten.
    for s in ["007 0012", "0000 0000", "-007 +008", "00000000000000000009 -1"] {
        assert_same_str("leading zeros", s);
    }
}

// ---------------------------------------------------------------------------
// Error / matching-failure paths: scanf returns 0 or EOF, the variable keeps
// its initialiser, and the program still prints and exits 0.
// ---------------------------------------------------------------------------

#[test]
fn first_conversion_fails() {
    for s in ["abc", "abc def", "x", ".", "-", "+", "--5", "+-5", "e", "?", "/", ":"] {
        assert_same_str("first conversion fails", s);
    }
}

#[test]
fn second_conversion_fails() {
    // x converts, then the second %d hits a non-digit or EOF.
    for s in ["1 abc", "1 -", "1 +", "1 .", "1 --2", "1 x", "1 \n", "1  "] {
        assert_same_str("second conversion fails", s);
    }
}

#[test]
fn sign_then_eof_or_nondigit() {
    for s in ["-", "+", "-\n", "+\n", "- 5", "+ 5", "-x", "+x", "1 -", "1 +", "1 - 2"] {
        assert_same_str("sign then non-digit", s);
    }
}

#[test]
fn trailing_garbage_stops_conversion() {
    // The offending character stays in the stream, so the *second* %d sees it.
    for s in [
        "5abc", "5abc 6", "5x6", "5.6", "5,6", "5-6", "5+6", "12a34", "1e5 2", "0x10 2", "5e", "1_2",
    ] {
        assert_same_str("trailing garbage", s);
    }
}

#[test]
fn non_numeric_prefix_blocks_both_conversions() {
    for s in ["abc 1 2", "x 5 6", "# 1", "-- 1 2"] {
        assert_same_str("non-numeric prefix", s);
    }
}

// ---------------------------------------------------------------------------
// Integer range, overflow, truncation and signedness, exactly as C does it.
//
// glibc's %d converts with strtol semantics (saturating at LONG_MIN/LONG_MAX
// on overflow) and then truncates the `long` to `int` on assignment.
// Pairing a value with y == -1 makes `x | ~y` == `x | 0` == x, so these cases
// observe the converted value of x directly.
// ---------------------------------------------------------------------------

#[test]
fn int_boundaries() {
    for s in [
        "2147483647 2147483647",
        "-2147483648 -2147483648",
        "2147483647 -2147483648",
        "-2147483648 2147483647",
        "2147483647 0",
        "0 2147483647",
        "-2147483648 0",
        "0 -2147483648",
    ] {
        assert_same_str("int boundaries", s);
    }
}

#[test]
fn overflow_wraps_and_truncates_like_c() {
    let values = [
        "0",
        "1",
        "-1",
        "2147483647",  // INT_MAX
        "2147483648",  // INT_MAX + 1
        "2147483649",
        "-2147483648", // INT_MIN
        "-2147483649", // INT_MIN - 1
        "4294967295",  // UINT_MAX
        "4294967296",  // 2^32
        "4294967297",
        "-4294967296",
        "9223372036854775806",
        "9223372036854775807",  // LONG_MAX
        "9223372036854775808",  // LONG_MAX + 1 -> saturates
        "18446744073709551615", // 2^64 - 1
        "18446744073709551616", // 2^64
        "-9223372036854775807",
        "-9223372036854775808", // LONG_MIN
        "-9223372036854775809", // LONG_MIN - 1 -> saturates
        "99999999999999999999",
        "-99999999999999999999",
        "12345678901234567890",
        "340282366920938463463374607431768211456", // 2^128
    ];
    for v in values {
        // y = -1 exposes x verbatim ...
        assert_same_str("overflow x", &format!("{v} -1"));
        // ... and y = 0 / y = the same value exercise the other operand.
        assert_same_str("overflow y", &format!("0 {v}"));
        assert_same_str("overflow both", &format!("{v} {v}"));
    }
}

#[test]
fn very_long_digit_runs() {
    // Thousands of leading zeros: one enormous but valid conversion. This also
    // pushes the value across the reader's internal buffer boundaries.
    let mut s = "0".repeat(100_000);
    s.push_str("123 -1");
    assert_same("100k leading zeros", s.as_bytes());

    let s2 = format!("{} -1", "9".repeat(50_000));
    assert_same("50k nines", s2.as_bytes());

    let s3 = format!("-{} -1", "9".repeat(50_000));
    assert_same("50k nines negative", s3.as_bytes());
}

#[test]
fn buffer_boundary_whitespace_and_digits() {
    // Force the number, and the whitespace before it, to straddle 4 KiB reads.
    for pad in [4093, 4094, 4095, 4096, 4097, 8191, 8192, 9000] {
        let s = format!("{}12 34", " ".repeat(pad));
        assert_same(&format!("{pad} spaces"), s.as_bytes());
        let s = format!("{}\n56 78", "\n".repeat(pad));
        assert_same(&format!("{pad} newlines"), s.as_bytes());
        // A digit run that ends exactly at a buffer edge.
        let s = format!("{}7 -1", "0".repeat(pad));
        assert_same(&format!("{pad} zeros then digit"), s.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Extra items, binary bytes, argv, and the "maximum the code handles".
// ---------------------------------------------------------------------------

#[test]
fn extra_items_after_the_first_two_are_ignored() {
    for s in ["1 2 3", "1 2 3 4 5", "1\n2\n3\n4\n", "1 2 abc", "1 2 -"] {
        assert_same_str("extra items", s);
    }
    let many = (0..10_000).map(|_| "7").collect::<Vec<_>>().join(" ");
    assert_same("10k items", many.as_bytes());
}

#[test]
fn nul_and_high_bytes_in_input() {
    assert_same("leading NUL", b"\x005 6");
    assert_same("NUL between", b"5\x006");
    assert_same("NUL after digits", b"5 6\x00");
    assert_same("high bytes", b"\xff\xfe 5 6");
    assert_same("digits then high byte", b"5\xff 6");
    assert_same("all high bytes", b"\x80\x81\x82");
    assert_same("utf8 text", "héllo 5 6".as_bytes());
    assert_same("nbsp is not C whitespace", "\u{a0}5 6".as_bytes());
}

#[test]
fn argv_is_ignored() {
    // main() takes no parameters; extra argv must change nothing.
    assert_same_with_args("one arg", b"1 2\n", &["foo"]);
    assert_same_with_args("two args", b"1 2\n", &["foo", "bar"]);
    assert_same_with_args("flag-looking arg", b"", &["--help"]);
}

// ---------------------------------------------------------------------------
// scanf's single-character pushback. On a matching failure only the *last*
// character read is returned to the stream; a consumed '+'/'-' is lost. That
// changes what the *second* %d sees, so it is observable in the output.
// ---------------------------------------------------------------------------

#[test]
fn matching_failure_pushback_is_one_character() {
    for s in [
        "--5 -1",   // 1st fails on 2nd '-', which is pushed back -> y = -5
        "---5 -1",  // 1st fails, 2nd also fails on '-' -> x = y = 0
        "-+5 -1",   // '+' pushed back -> y = +5
        "-x 5",     // 'x' pushed back, blocks the 2nd conversion too
        "+x 5",
        "- -5",
        "+ -5",
        "--5",
        "5 --3",
        "5 - 3",
        "5x-3",
        "5-3",  // '-' pushed back -> y = -3
        "-5-3",
        "5 -3",
        "5+3",
        "1 2x",
    ] {
        assert_same_str("one-char pushback", s);
    }
}

// ---------------------------------------------------------------------------
// stdin and stdout error paths: unreadable stdin, and writes that fail.
// The C code checks neither scanf's nor printf's return value, so it must
// still exit 0 and print nothing to stderr.
// ---------------------------------------------------------------------------

#[test]
fn stdin_at_immediate_eof_from_dev_null() {
    let dev_null = || Stdio::from(std::fs::File::open("/dev/null").expect("open /dev/null"));
    let c = Command::new(c_bin())
        .stdin(dev_null())
        .output()
        .expect("run C with /dev/null stdin");
    let r = Command::new(rust_bin())
        .stdin(dev_null())
        .output()
        .expect("run Rust with /dev/null stdin");
    assert_eq!(c.stdout, r.stdout, "stdout differs on /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr differs on /dev/null stdin");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status differs on /dev/null stdin"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn stdout_write_failure_is_ignored() {
    // /dev/full accepts opens but fails every write with ENOSPC. The C program
    // ignores printf/puts failures, so it exits 0 with empty stderr; the Rust
    // translation must do the same instead of panicking on a write error.
    if std::fs::OpenOptions::new().write(true).open("/dev/full").is_err() {
        // /dev/full unavailable (e.g. restricted container): nothing to compare.
        return;
    }
    let full = || {
        Stdio::from(
            std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .expect("open /dev/full"),
        )
    };
    let spawn = |exe: PathBuf| {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(full())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn with /dev/full stdout");
        {
            let mut sink = child.stdin.take().unwrap();
            let _ = sink.write_all(b"1 2\n");
        }
        child.wait_with_output().expect("wait")
    };
    let c = spawn(c_bin());
    let r = spawn(rust_bin());
    assert_eq!(c.stderr, r.stderr, "stderr differs when stdout writes fail");
    assert_eq!(
        c.status.code(),
        r.status.code(),
        "exit status differs when stdout writes fail: C={:?} Rust={:?}",
        c.status.code(),
        r.status.code()
    );
}

// ---------------------------------------------------------------------------
// Exhaustive-ish sweep of the arithmetic in driver(): result = x | ~y.
// ---------------------------------------------------------------------------

#[test]
fn arithmetic_sweep_small_values() {
    for x in -4i64..=4 {
        for y in -4i64..=4 {
            assert_same_str("small sweep", &format!("{x} {y}\n"));
        }
    }
}

#[test]
fn arithmetic_sweep_bit_patterns() {
    let interesting: [i64; 14] = [
        0,
        1,
        -1,
        2,
        -2,
        255,
        256,
        -256,
        65535,
        65536,
        0x5555_5555,
        0x2aaa_aaaa,
        2147483647,
        -2147483648,
    ];
    for &x in &interesting {
        for &y in &interesting {
            assert_same_str("bit patterns", &format!("{x} {y}"));
        }
    }
}

#[test]
fn deterministic_pseudo_random_numeric_pairs() {
    // Small xorshift-ish LCG so the corpus is fixed across runs.
    let mut state: u64 = 0x1234_5678_9abc_def0;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..300 {
        let x = next() as i64 as i128 % 5_000_000_000;
        let y = next() as i64 as i128 % 5_000_000_000;
        let sep = match next() % 5 {
            0 => " ",
            1 => "\n",
            2 => "\t",
            3 => "  \n\t ",
            _ => "\r\n",
        };
        let input = format!("{x}{sep}{y}");
        assert_same_str("random numeric pair", &input);
    }
}

#[test]
fn deterministic_pseudo_random_junk() {
    // Random bytes drawn from the alphabet scanf actually distinguishes.
    const ALPHA: &[u8] = b" \t\n\r\x0b\x0c0123456789+-abcxXeE.,/:";
    let mut state: u64 = 0xdead_beef_cafe_1234;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for case in 0..300 {
        let len = (next() % 24) as usize;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(ALPHA[(next() % ALPHA.len() as u64) as usize]);
        }
        assert_same(&format!("random junk #{case}"), &buf);
    }
}

#[test]
fn deterministic_pseudo_random_raw_bytes() {
    let mut state: u64 = 0x0bad_f00d_1357_9bdf;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for case in 0..150 {
        let len = (next() % 16) as usize;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push((next() % 256) as u8);
        }
        assert_same(&format!("random raw bytes #{case}"), &buf);
    }
}
