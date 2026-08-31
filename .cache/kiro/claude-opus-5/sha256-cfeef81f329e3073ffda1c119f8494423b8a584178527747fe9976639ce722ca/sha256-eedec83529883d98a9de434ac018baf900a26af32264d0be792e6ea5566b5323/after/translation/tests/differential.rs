// Differential test harness: runs the C reference binary and the Rust binary as
// subprocesses on identical stdin and requires byte-identical stdout, stderr and
// exit status.
//
// The Rust code is NEVER called as a library here; both programs are driven the
// way a shell would drive them, because that is how the translation is graded.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary produced by this crate.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

/// Path to the C reference binary, building it with CMake on first use if it is
/// not already present.  A comparison against a program that did not build
/// measures nothing, so a build failure aborts the test run loudly.
fn c_bin() -> &'static Path {
    static C: OnceLock<PathBuf> = OnceLock::new();
    C.get_or_init(|| {
        let root = workspace_root();
        let c_src = root.join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if exe.exists() {
            return exe;
        }
        std::fs::create_dir_all(&build).expect("cannot create c_src/build");
        let cfg = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("failed to run `cmake` -- is CMake installed?");
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
        assert!(exe.exists(), "C binary missing after build: {}", exe.display());
        exe
    })
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Option<i32>,
}

fn run(exe: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", exe.display()));

    // Write on a helper thread: some inputs are large enough to fill the pipe
    // buffer, and the child may exit without draining it.
    let owned = input.to_vec();
    let mut stdin = child.stdin.take().expect("stdin");
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&owned);
        let _ = stdin.flush();
        drop(stdin);
    });

    let out = child.wait_with_output().expect("wait_with_output");
    let _ = writer.join();

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

fn show_input(b: &[u8]) -> String {
    if b.len() <= 160 {
        show(b)
    } else {
        format!(
            "{}...[{} bytes total]...{}",
            show(&b[..80]),
            b.len(),
            show(&b[b.len() - 40..])
        )
    }
}

/// Compare all three observable channels for one input.
#[track_caller]
fn assert_same(input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for input \"{}\"\n  C   : \"{}\"\n  Rust: \"{}\"",
        show_input(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for input \"{}\"\n  C   : \"{}\"\n  Rust: \"{}\"",
        show_input(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.status,
        r.status,
        "exit status differs for input \"{}\": C={:?} Rust={:?}",
        show_input(input),
        c.status,
        r.status
    );
}

fn all(cases: &[&str]) {
    for c in cases {
        assert_same(c.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both programs exist, run, and agree on the trivial input.
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let c = run(c_bin(), b"1");
    let r = run(rust_bin(), b"1");
    assert_eq!(c.status, Some(0), "C binary did not exit 0");
    assert_eq!(r.status, Some(0), "Rust binary did not exit 0");
    // The C program always prints 8 hex digits plus a newline.
    assert_eq!(c.stdout, b"0000803f\n".to_vec(), "unexpected C reference output");
    assert_eq!(c.stdout, r.stdout);
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program branches on.
//
// `main` is `scanf("%f", &x)` over an `x` pre-initialised to 0.0f, followed by
// `driver`/`print_hex`.  The branch structure that is observable is therefore
// the whole `%f` grammar: whether the conversion matches at all, and which
// value it produces.
// ---------------------------------------------------------------------------

/// scanf reaches EOF before any conversion: `x` keeps its initial 0.0f.
#[test]
fn empty_and_whitespace_only_input() {
    assert_same(b"");
    all(&[
        " ", "\n", "\t", "\r", "\u{b}", "\u{c}", "  \n\n  ", "\t \r\n\u{b}\u{c}",
    ]);
}

/// A single item: the plain happy path, and `scanf` skipping leading
/// whitespace/newlines to find it.
#[test]
fn single_value_and_leading_whitespace() {
    all(&[
        "0", "1", "2", "-1", "+1", "1.5", "-1.5", "42",
        " 1", "\t1", "\n1", "\n\n\n3.5", "  \r\n 42", "\u{b}\u{c}1", "   -2.75   ",
    ]);
}

/// scanf reads across newlines, unlike fgets: whitespace before the number is
/// skipped no matter how many lines it spans, and only the first item is read.
#[test]
fn reads_across_newlines_and_stops_after_first_item() {
    all(&["1 2", "1\n2", "\n\n\n\n\n\n1.25\n9999\n", "1\n\n\n", "3.5 abc", "3.5abc"]);
}

/// Sign handling, including negative zero (sign bit must survive).
#[test]
fn signs_and_negative_zero() {
    all(&[
        "-0", "+0", "-0.0", "+0.0", "-0e0", "-0.000e10", "-0x0p0", "+0x0p0",
        "-0.00000000000000000000000000000000000000000000000000001",
        "0.00000000000000000000000000000000000000000000000000001",
    ]);
}

/// Matching failure: the first non-whitespace character cannot start a float,
/// so scanf returns 0 and never touches `x`.
#[test]
fn matching_failure_leaves_x_untouched() {
    all(&[
        "abc", "xyz", "z", "q1", "-abc", "+abc", "-", "+", "--5", "+-5", "- 5", "+ 5",
        ".", "-.", "+.", "..", ".e5", "e5", "E5", "+e5", "-e5", "p1", "%", "*", "/",
        "0b101", "0o17", "\u{80}\u{81}", "\u{20ac}1",
    ]);
}

/// The decimal grammar's optional pieces: leading/trailing point, no integer
/// part, no fractional part.
#[test]
fn decimal_forms() {
    all(&[
        ".5", "-.5", "+.5", "5.", "-5.", "0.5", "1.", "1.0", "0.0", "1.2.3", ".5.5", "5..",
        "000000000000001", "00000000000000000.5", "0000.0000",
    ]);
}

/// Decimal exponents, including the ones with no digits after the marker (the
/// marker is then not part of the number).
#[test]
fn decimal_exponents() {
    all(&[
        "1e5", "1E5", "1e+5", "1e-5", "1e0", "1e-0", "1.5e3", ".5e3", "5.e3", "1.e5",
        "1e", "1e+", "1e-", "1ex", "1e5x", "1e5.5", "1e5e5", "1e+", "2e", "0e", "0e+9",
        "1e007", "1e-007", "1e0000000000000000005",
    ]);
}

/// Hexadecimal floats, including every way `0x` can fail to be followed by a
/// significand.
#[test]
fn hex_forms() {
    all(&[
        "0x1", "0X1", "0x1p3", "0X1P3", "0x1p+3", "0x1p-3", "0x1.8p1", "0x.8p1", "0x.5",
        "0x10", "0xA", "0xa", "0xabcdef", "0xABCDEF", "0x1.8", "0x0p0", "0x0.0p0",
        "-0x1p3", "+0x1p3",
        // no significand / no exponent digits / not a hex digit at all
        "0x", "0X", "0x.", "0X.", "0x.p1", "0xp1", "0x0p", "0x1p", "0x1p+", "0x1p-",
        "0xg", "0xz", "0x1z", "0x1.z", "0x_",
    ]);
}

/// `inf` / `infinity`, every prefix and every case pattern.  A partially
/// matched `infinity` suffix is a matching failure in glibc's scanf.
#[test]
fn infinity_words() {
    all(&[
        "inf", "INF", "Inf", "iNf", "infinity", "INFINITY", "InFiNiTy", "-inf", "+inf",
        "-infinity", "+INFINITY", "inf.5", "infx", "infinityx", "inf inf",
        // prefixes that never complete a word
        "i", "I", "in", "IN", "infi", "INFI", "infin", "infini", "infinit", "iNfInI",
    ]);
}

/// `nan`, with and without the parenthesised n-char-sequence.
#[test]
fn nan_words() {
    all(&[
        "nan", "NAN", "nAn", "NaN", "-nan", "+nan", "nanx", "nan-3", "nan nan",
        "nan()", "nan(0)", "nan(123)", "nan(abc_123)", "nan(ABC)", "nan(abc)def",
        "nan(", "nan(abc", "nan(a b)", "nan(!)", "nan(_)",
        // prefixes
        "n", "N", "na", "NA", "nA",
    ]);
}

/// Overflow to infinity, underflow to zero, and the subnormal range.
#[test]
fn overflow_underflow_and_subnormals() {
    all(&[
        // overflow
        "1e39", "1e400", "-1e400", "3.4028236e38", "340282360000000000000000000000000000000",
        "0x1p128", "0x1.ffffffp127", "1e999999999999999999999",
        // largest finite
        "3.4028235e38", "0x1.fffffep127", "340282350000000000000000000000000000000",
        // smallest normal / subnormals
        "1.1754944e-38", "1.1754943508222875e-38", "5.877471754111438e-39",
        "1e-38", "1e-39", "1e-40", "1e-44", "1e-45", "1.4e-45", "0x1p-149",
        // underflow to zero (and the half-way case that rounds up to the
        // smallest subnormal)
        "0.7e-45", "0.5e-45", "1e-46", "1e-100", "1e-400", "1e-999999999999999999999",
        "0x1p-150", "0x1.0000001p-150", "0x1p-151", "7.006492321624085e-46",
        "7.0064923e-46", "3.5032461e-46",
    ]);
}

/// Round-to-nearest-even at the binary32 tie points.
#[test]
fn rounding_ties() {
    all(&[
        "16777216", "16777217", "16777218", "16777219", "16777220",
        "1.0000000596046448", "1.00000005960464477539062500",
        "1.000000059604644775390625", "1.0000000596046447753906250000000001",
        "0x1.0000001p0", "0x1.0000003p0", "0x1.00000010000001p0",
        "8388609.5", "8388610.5",
    ]);
}

// ---------------------------------------------------------------------------
// Phase C: input classes no earlier test reaches.
// ---------------------------------------------------------------------------

/// The maximum the code handles: inputs far longer than any fixed buffer, and
/// digit strings far wider than binary32's significand.  These exercise the
/// arbitrary-precision paths and the sticky-bit handling.
#[test]
fn very_long_inputs() {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    let mut push = |s: String| cases.push(s.into_bytes());

    for n in [1usize, 10, 100, 1000, 5000, 100_000] {
        push("0".repeat(n));
        push(format!("{}1", "0".repeat(n)));
        push(format!("1{}", "0".repeat(n)));
        push(format!("0.{}1", "0".repeat(n)));
        push(format!("1{}e-{}", "0".repeat(n), n));
        push("9".repeat(n));
        push(format!("0.{}", "9".repeat(n)));
        push(format!("{} 5", " ".repeat(n)));
        push(format!("{}2.5", "\n".repeat(n)));
        push(format!("0x{}1p0", "0".repeat(n)));
        push(format!("0x0.{}1p0", "0".repeat(n)));
        push(format!("1e{}5", "0".repeat(n)));
    }
    // Hex significands wider than the 28-digit window kept internally.
    for n in [1usize, 8, 24, 28, 29, 32, 64, 200, 5000] {
        let d = "f".repeat(n);
        for p in ["", "p0", "p-149", "p-150", "p-160", "p127", "p128"] {
            push(format!("0x{d}{p}"));
            push(format!("0x.{d}{p}"));
            push(format!("-0x{d}.{d}{p}"));
        }
        let d = format!("8{}", "0".repeat(n.saturating_sub(1)));
        push(format!("0x{d}p-{}", 4 * n));
        push(format!("0x{d}1p-{}", 4 * n));
    }
    push(format!("0x{}p-400000", "f".repeat(100_000)));
    push(format!("1e-{}", "9".repeat(40)));
    push(format!("1e{}", "9".repeat(40)));

    for c in &cases {
        assert_same(c);
    }
}

/// Bytes that are not text at all, plus embedded NULs.
#[test]
fn binary_and_nul_bytes() {
    let cases: Vec<Vec<u8>> = vec![
        vec![0u8],
        vec![0u8; 4096],
        b"\x001".to_vec(),
        b"1\x00".to_vec(),
        b"1\x002".to_vec(),
        b"\x00\x00 2.5".to_vec(),
        b"\xff\xfe\xfd".to_vec(),
        (0u8..=255).collect(),
        (0u8..=255).rev().collect(),
        b" \xff1".to_vec(),
        b"\x1b[0m1".to_vec(),
    ];
    for c in &cases {
        assert_same(c);
    }
}

/// Every binary32 exponent, printed as an exact decimal expansion and as a C99
/// hex float, round-tripped through both programs.
#[test]
fn every_exponent_round_trips() {
    for be in 0u32..255 {
        for frac in [0u32, 1, 0x40_0000, 0x7f_ffff] {
            let bits = (be << 23) | frac;
            let f = f32::from_bits(bits);
            // Exact decimal expansion: f32 -> f64 is lossless, and {:.80e}
            // prints the value exactly for every binary32.
            assert_same(format!("{:.80e}", f as f64).as_bytes());
            assert_same(format!("{}", f as f64).as_bytes());
        }
    }
}

/// A deterministic pseudo-random sweep: junk strings, decimal numbers and hex
/// floats.  Seeded, so a failure is reproducible.
#[test]
fn randomized_sweep() {
    // xorshift64*, so the suite needs no external crates.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
        fn pick(&mut self, s: &[u8]) -> u8 {
            s[self.below(s.len())]
        }
    }

    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);

    // Junk over an alphabet of every character the %f grammar reacts to.
    const ALPHA: &[u8] = b"0123456789.-+eExXpPabcdefABCDEFnNiIfFtTyY() \t\n\r";
    for _ in 0..1500 {
        let n = 1 + rng.below(14);
        let s: Vec<u8> = (0..n).map(|_| rng.pick(ALPHA)).collect();
        assert_same(&s);
    }

    // Decimal numbers.
    for _ in 0..1200 {
        let mut s = String::new();
        match rng.below(4) {
            0 => s.push('-'),
            1 => s.push('+'),
            _ => {}
        }
        for _ in 0..rng.below(7) {
            s.push(rng.pick(b"0123456789") as char);
        }
        if rng.below(2) == 0 {
            s.push('.');
            for _ in 0..rng.below(30) {
                s.push(rng.pick(b"0123456789") as char);
            }
        }
        if rng.below(10) < 7 {
            s.push(rng.pick(b"eE") as char);
            match rng.below(3) {
                0 => s.push('-'),
                1 => s.push('+'),
                _ => {}
            }
            s.push_str(&rng.below(401).to_string());
        }
        assert_same(s.as_bytes());
    }

    // Hex floats.
    for _ in 0..1200 {
        let mut s = String::new();
        if rng.below(3) == 0 {
            s.push('-');
        }
        s.push('0');
        s.push(rng.pick(b"xX") as char);
        for _ in 0..rng.below(9) {
            s.push(rng.pick(b"0123456789abcdefABCDEF") as char);
        }
        if rng.below(2) == 0 {
            s.push('.');
            for _ in 0..rng.below(20) {
                s.push(rng.pick(b"0123456789abcdefABCDEF") as char);
            }
        }
        if rng.below(10) < 7 {
            s.push(rng.pick(b"pP") as char);
            match rng.below(3) {
                0 => s.push('-'),
                1 => s.push('+'),
                _ => {}
            }
            s.push_str(&rng.below(201).to_string());
        }
        assert_same(s.as_bytes());
    }

    // Random bit patterns, fed back in every textual form.
    for _ in 0..600 {
        let bits = rng.next() as u32;
        let f = f32::from_bits(bits) as f64;
        if f.is_nan() {
            continue;
        }
        assert_same(format!("{f}").as_bytes());
        assert_same(format!("{f:.60e}").as_bytes());
    }
}

/// Exhaustive over all 3-character inputs drawn from the characters the grammar
/// reacts to.  This is the cheapest way to cover prefix/pushback behaviour that
/// hand-written cases miss.
#[test]
fn exhaustive_short_inputs() {
    const ALPHA: &[u8] = b"01.xXpPnN(aAiIfFtTyY-+e \n";
    let mut buf = [0u8; 3];
    for &a in ALPHA {
        buf[0] = a;
        assert_same(&buf[..1]);
        for &b in ALPHA {
            buf[1] = b;
            assert_same(&buf[..2]);
            for &c in ALPHA {
                buf[2] = c;
                assert_same(&buf[..3]);
            }
        }
    }
}
