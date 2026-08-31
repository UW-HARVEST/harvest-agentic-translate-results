//! Differential tests: run the C binary and the Rust binary as subprocesses,
//! feed both the same bytes on stdin, and require byte-identical stdout,
//! byte-identical stderr, and an identical exit status.
//!
//! The Rust program is never loaded as a library; it is executed exactly the
//! way a shell would execute it, because that is how it is graded.
//!
//! ---------------------------------------------------------------------------
//! Branch inventory of c_src/src/main.c (what the inputs below must cover)
//! ---------------------------------------------------------------------------
//!
//!   char s1[100] = "", s2[100] = "";
//!   fgets(s1, sizeof(s1), stdin);
//!   fgets(s2, sizeof(s1), stdin);
//!   s1[strlen(s1)-1] = '\0';
//!   s2[strlen(s2)-1] = '\0';
//!   printf("%zu\n", strcspn(s1, s2));
//!
//! There is no explicit `if` in the program, but the library calls branch on
//! the input, and each branch is an input class:
//!
//!   fgets #1 / #2, each independently:
//!     (a) immediate EOF        -> returns NULL, buffer left as ""
//!     (b) stops on a newline   -> newline retained in the buffer
//!     (c) fills size-1 == 99 bytes without seeing a newline -> truncation,
//!         and the remainder of that line is left for the *next* fgets
//!     (d) EOF after >=1 byte with no newline -> partial line, no newline
//!
//!   s[strlen(s)-1] = '\0', each independently:
//!     (e) strlen > 0  -> deletes the final byte, which is the newline in the
//!         common case but is a *data* byte for a truncated or newline-less
//!         line (the program's built-in quirk; must be reproduced)
//!     (f) strlen == 0 -> index -1, an out-of-bounds store. Verified against
//!         the compiled binary: s1 lives at rsp+0x00 and s2 at rsp+0x70, so
//!         the stores land at rsp-0x01 and rsp+0x6f, i.e. below the frame and
//!         in the 0x64..0x6f padding gap. Neither byte belongs to s1 or s2,
//!         so the store cannot influence the printed result.
//!
//!   strcspn(s1, s2):
//!     (g) a rejected byte at index 0            -> 0
//!     (h) a rejected byte in the middle         -> that index
//!     (i) a rejected byte only at the last index
//!     (j) no rejected byte at all               -> strlen(s1)
//!     (k) empty reject set s2                   -> strlen(s1)
//!     (l) empty s1                              -> 0
//!
//!   NUL bytes: fgets copies them into the buffer, but strlen/strcspn stop at
//!     the first one, so a NUL truncates the effective string.
//!     (m) NUL inside s1, (n) NUL inside s2, (o) NUL as the very first byte.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating and building the two programs
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary under test, as built by cargo. Driven as a
/// subprocess, never linked against.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary, building it with CMake on first use if needed.
/// `c_src/` is only read from and configured out-of-tree into `c_src/build`;
/// no file under `c_src/` is modified.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let bin = build.join("driver");
        if bin.exists() {
            return bin;
        }

        std::fs::create_dir_all(&build).expect("create c_src/build");

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

        assert!(bin.exists(), "C binary missing after build: {}", bin.display());
        bin
    })
}

// ---------------------------------------------------------------------------
// Running and comparing
// ---------------------------------------------------------------------------

fn run(bin: &Path, stdin_bytes: &[u8]) -> Output {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    // Write on a worker thread so a program that never drains stdin (this one
    // reads at most ~200 bytes) cannot deadlock us on a large input.
    let mut sink = child.stdin.take().expect("piped stdin");
    let data = stdin_bytes.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = sink.write_all(&data);
        let _ = sink.flush();
        // dropping `sink` closes the pipe, producing EOF for the child
    });

    let out = child.wait_with_output().expect("wait_with_output");
    let _ = writer.join();
    out
}

/// Render bytes readably so a failure message pinpoints the offending input.
fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            0 => s.push_str("\\0"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    s
}

/// The core assertion: stdout, stderr and exit status must all match.
fn assert_same(desc: &str, stdin_bytes: &[u8]) {
    let c = run(c_bin(), stdin_bytes);
    let r = run(rust_bin(), stdin_bytes);

    let ctx = || {
        format!(
            "case: {desc}\n\
             stdin ({} bytes): \"{}\"\n\
             C  stdout: \"{}\"\n\
             Rs stdout: \"{}\"\n\
             C  stderr: \"{}\"\n\
             Rs stderr: \"{}\"\n\
             C  status: {:?} (code {:?})\n\
             Rs status: {:?} (code {:?})",
            stdin_bytes.len(),
            show(stdin_bytes),
            show(&c.stdout),
            show(&r.stdout),
            show(&c.stderr),
            show(&r.stderr),
            c.status,
            c.status.code(),
            r.status,
            r.status.code(),
        )
    };

    assert_eq!(c.stdout, r.stdout, "stdout mismatch\n{}", ctx());
    assert_eq!(c.stderr, r.stderr, "stderr mismatch\n{}", ctx());
    assert_eq!(c.status.code(), r.status.code(), "exit code mismatch\n{}", ctx());
    assert_eq!(
        c.status.success(),
        r.status.success(),
        "exit status mismatch\n{}",
        ctx()
    );
}

fn many(cases: &[(&str, Vec<u8>)]) {
    for (desc, input) in cases {
        assert_same(desc, input);
    }
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist, run, and agree on a trivial input
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_run() {
    let input = b"abcdef\ncd\n";
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);
    assert_eq!(c.stdout, b"2\n", "C reference output changed unexpectedly");
    assert_eq!(r.stdout, c.stdout);
    assert_eq!(r.stderr, c.stderr);
    assert_eq!(r.status.code(), c.status.code());
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program branches on
// ---------------------------------------------------------------------------

/// (a) immediate EOF on fgets #1 and #2; (f) both out-of-bounds chops;
/// (l) empty s1; (k) empty reject set.
#[test]
fn empty_and_near_empty_input() {
    many(&[
        ("completely empty input (both fgets return NULL)", b"".to_vec()),
        ("single newline only (s1 becomes \"\", fgets #2 NULL)", b"\n".to_vec()),
        ("two newlines (both strings become empty)", b"\n\n".to_vec()),
        ("three newlines (third line never read)", b"\n\n\n".to_vec()),
        ("one NUL byte only", b"\0".to_vec()),
        ("NUL then newline", b"\0\n".to_vec()),
        ("newline then NUL", b"\n\0".to_vec()),
    ]);
}

/// (d) EOF with no trailing newline, and (e) the chop eating a data byte.
#[test]
fn single_line_no_trailing_newline() {
    many(&[
        ("one char, no newline (chop removes the only char)", b"a".to_vec()),
        ("two chars, no newline", b"ab".to_vec()),
        ("three chars, no newline", b"abc".to_vec()),
        ("one line with newline, no second line", b"abc\n".to_vec()),
        ("s1 present, s2 absent, s1 chopped to \"ab\"", b"abc".to_vec()),
    ]);
}

/// (g)(h)(i)(j)(k)(l): every outcome shape of strcspn.
#[test]
fn strcspn_match_positions() {
    many(&[
        ("no rejected byte -> full length", b"abcdef\nxyz\n".to_vec()),
        ("rejected byte at index 0", b"abcdef\na\n".to_vec()),
        ("rejected byte in the middle", b"abcdef\ncd\n".to_vec()),
        ("rejected byte at the last index of s1", b"abcdef\nf\n".to_vec()),
        ("every byte of s1 rejected", b"abcdef\nfedcba\n".to_vec()),
        ("empty reject set s2 -> strlen(s1)", b"abcdef\n\n".to_vec()),
        ("empty s1 -> 0", b"\nabcdef\n".to_vec()),
        ("both empty -> 0", b"\n\n".to_vec()),
        ("s2 chopped so its last char stops rejecting", b"abcdef\ncf\n".to_vec()),
        ("single char s1 matching", b"a\na\n".to_vec()),
        ("single char s1 not matching", b"a\nb\n".to_vec()),
        ("repeated bytes in reject set", b"abcdef\ncccc\n".to_vec()),
        ("s2 has no newline at EOF (last byte chopped)", b"abcdef\ncd".to_vec()),
        ("s2 single char, no newline -> reject set empties", b"abcdef\nc".to_vec()),
    ]);
}

/// (m)(n)(o): NUL bytes truncate the effective strings.
#[test]
fn embedded_nul_bytes() {
    many(&[
        ("NUL inside s1 truncates it", b"a\0b\ncd\n".to_vec()),
        ("NUL inside s2 empties the reject set", b"abc\na\0b\n".to_vec()),
        ("NUL first in s1", b"\0abc\ncd\n".to_vec()),
        ("NUL first in s2", b"abc\n\0cd\n".to_vec()),
        ("NUL in both", b"a\0b\nc\0d\n".to_vec()),
        ("NUL just before the newline of s1", b"abc\0\ncd\n".to_vec()),
        ("NUL is the reject byte", b"abc\n\0\n".to_vec()),
        ("only NULs", b"\0\0\0\n\0\0\n".to_vec()),
    ]);
}

/// Full byte-value sweep: each of the 256 values as the sole content of s1,
/// and again as the sole content of s2. Covers `char` signedness in strcspn
/// and any high-byte / control-byte handling.
#[test]
fn every_byte_value() {
    for v in 0u16..=255 {
        let b = v as u8;
        let mut a = vec![b];
        a.extend_from_slice(b"\nabc\n");
        assert_same(&format!("byte 0x{b:02x} as s1"), &a);

        let mut c = b"abc\n".to_vec();
        c.push(b);
        c.push(b'\n');
        assert_same(&format!("byte 0x{b:02x} as s2"), &c);

        // the same byte on both sides, so it is both content and reject set
        let mut d = vec![b, b, b, b'\n', b, b'\n'];
        d.push(b'\n');
        assert_same(&format!("byte 0x{b:02x} on both sides"), &d);
    }
}

// ---------------------------------------------------------------------------
// Phase C: the boundary paths, i.e. (c) fgets truncation at size-1 == 99
// ---------------------------------------------------------------------------

/// The maximum the code handles. `fgets(s, 100, ...)` accepts at most 99
/// bytes, so lengths 98/99/100/101 straddle every interesting boundary, and a
/// line longer than 99 bytes spills its remainder into the *second* fgets.
#[test]
fn buffer_length_boundaries() {
    for n in [0usize, 1, 2, 96, 97, 98, 99, 100, 101, 102, 197, 198, 199, 200, 201, 260] {
        let body = vec![b'a'; n];

        for (tag, tail) in [
            ("+NL +\"z\"NL", b"\nz\n".to_vec()),
            ("+NL only", b"\n".to_vec()),
            ("no NL at all", b"".to_vec()),
            ("+NL +empty line", b"\n\n".to_vec()),
            ("+NL +\"a\"NL (reject matches)", b"\na\n".to_vec()),
            ("+NL +long second line", {
                let mut t = b"\n".to_vec();
                t.extend_from_slice(&vec![b'b'; 150]);
                t.push(b'\n');
                t
            }),
        ] {
            let mut input = body.clone();
            input.extend_from_slice(&tail);
            assert_same(&format!("s1 len={n} {tag}"), &input);
        }
    }
}

/// A single long line split across both fgets calls: s2 is fed the tail of
/// s1's own line, which is the program's most counter-intuitive behavior.
#[test]
fn long_line_spills_into_second_fgets() {
    let cases: Vec<(String, Vec<u8>)> = vec![
        ("99 a's then 50 b's, one line".to_string(), {
            let mut v = vec![b'a'; 99];
            v.extend_from_slice(&vec![b'b'; 50]);
            v.push(b'\n');
            v
        }),
        ("99 a's then a single 'a' tail".to_string(), {
            let mut v = vec![b'a'; 100];
            v.push(b'\n');
            v
        }),
        ("exactly 99 bytes then EOF".to_string(), vec![b'a'; 99]),
        ("exactly 100 bytes then EOF".to_string(), vec![b'a'; 100]),
        ("exactly 198 bytes then EOF".to_string(), vec![b'a'; 198]),
        ("exactly 199 bytes then EOF".to_string(), vec![b'a'; 199]),
        ("300 bytes, no newline".to_string(), vec![b'a'; 300]),
        ("98 a's + newline as byte 99".to_string(), {
            let mut v = vec![b'a'; 98];
            v.push(b'\n');
            v.extend_from_slice(b"a\n");
            v
        }),
        ("newline as byte 100 (one past the fgets limit)".to_string(), {
            let mut v = vec![b'a'; 99];
            v.push(b'\n');
            v.extend_from_slice(b"a\n");
            v
        }),
    ];
    for (desc, input) in &cases {
        assert_same(desc, input);
    }
}

/// A rejected byte placed at every index of a 99-, 100- and 101-byte first
/// line, so the match position sweeps across the truncation point and across
/// the byte that the blind chop deletes.
#[test]
fn reject_byte_at_every_index_near_boundary() {
    for n in [98usize, 99, 100, 101] {
        for pos in 0..n {
            let mut s1 = vec![b'a'; n];
            s1[pos] = b'X';
            let mut input = s1;
            input.push(b'\n');
            input.extend_from_slice(b"X\n");
            assert_same(&format!("X at index {pos} of {n}-byte s1"), &input);
        }
    }
}

/// The reject set itself at the length boundary: s2 of 98/99/100/101 bytes,
/// where the distinguishing byte sits at, just before, or past the point
/// where fgets truncates and the chop deletes.
#[test]
fn reject_set_at_length_boundary() {
    for n in [98usize, 99, 100, 101] {
        for pos in [0usize, 1, n / 2, n - 3, n - 2, n - 1] {
            let mut s2 = vec![b'z'; n];
            s2[pos] = b'X';
            let mut input = b"aaaXaaa\n".to_vec();
            input.extend_from_slice(&s2);
            input.push(b'\n');
            assert_same(&format!("X at index {pos} of {n}-byte s2"), &input);
        }
    }
}

/// Newlines and NULs interleaved at the boundary, plus CRLF input, since the
/// chop is not newline-aware.
#[test]
fn boundary_with_nul_and_crlf() {
    many(&[
        ("CRLF line endings (CR survives the chop)", b"abcdef\r\ncd\r\n".to_vec()),
        ("CRLF, CR is the reject byte", b"abcdef\r\n\r\n".to_vec()),
        ("lone CR, no LF", b"abcdef\rcd\r".to_vec()),
        ("NUL at index 98 of a 99-byte line", {
            let mut v = vec![b'a'; 99];
            v[98] = 0;
            v.extend_from_slice(b"\na\n");
            v
        }),
        ("NUL at index 98 of a 100-byte line", {
            let mut v = vec![b'a'; 100];
            v[98] = 0;
            v.extend_from_slice(b"\na\n");
            v
        }),
        ("NUL at index 99 of a 100-byte line (past truncation)", {
            let mut v = vec![b'a'; 100];
            v[99] = 0;
            v.extend_from_slice(b"\na\n");
            v
        }),
        ("NUL at index 0 of a 99-byte line", {
            let mut v = vec![b'a'; 99];
            v[0] = 0;
            v.extend_from_slice(b"\na\n");
            v
        }),
        ("99-byte line of NULs", {
            let mut v = vec![0u8; 99];
            v.extend_from_slice(b"\na\n");
            v
        }),
    ]);
}

/// Trailing content after the two lines must be ignored, and a third line
/// must never be read.
#[test]
fn trailing_input_is_ignored() {
    many(&[
        ("three short lines", b"abc\ndef\nghi\n".to_vec()),
        ("many lines", b"abcdef\ncd\nxxx\nyyy\nzzz\n".to_vec()),
        ("huge trailing blob after two lines", {
            let mut v = b"abcdef\ncd\n".to_vec();
            v.extend_from_slice(&vec![b'q'; 10_000]);
            v
        }),
        ("binary garbage after two lines", {
            let mut v = b"abcdef\ncd\n".to_vec();
            v.extend((0u16..=255).map(|b| b as u8));
            v
        }),
    ]);
}

/// argv is ignored by `int main()`, so extra arguments must change nothing.
#[test]
fn extra_arguments_are_ignored() {
    let input: &[u8] = b"abcdef\ncd\n";
    let mk = |bin: &Path| {
        let mut child = Command::new(bin)
            .args(["one", "two", "three"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn with args");
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(input)
            .expect("write stdin");
        child.wait_with_output().expect("wait")
    };
    let c = mk(c_bin());
    let r = mk(rust_bin());
    assert_eq!(c.stdout, r.stdout, "stdout mismatch with extra argv");
    assert_eq!(c.stderr, r.stderr, "stderr mismatch with extra argv");
    assert_eq!(c.status.code(), r.status.code(), "exit code mismatch with extra argv");
}

// ---------------------------------------------------------------------------
// Randomized differential fuzzing
// ---------------------------------------------------------------------------

/// Deterministic xorshift64* so failures are reproducible without adding a
/// dependency on the `rand` crate.
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

/// Broad randomized sweep over lengths that straddle the 99/198 boundaries,
/// using alphabets rich in the bytes the program treats specially: newline,
/// NUL, CR, and high bytes.
#[test]
fn randomized_differential_fuzz() {
    let alphabets: [&[u8]; 6] = [
        b"ab\n",
        b"abc",
        b"aX\n\x00",
        b"\n",
        b"aX\n\x00\r\xff\x80",
        b"abcdefX\n\x00\x01\x7f\x80\xfe\xff",
    ];

    let mut rng = Rng(0x1234_5678_9abc_def1);
    for i in 0..1200 {
        let alpha = alphabets[rng.below(alphabets.len())];
        // Bias lengths toward the boundaries the code branches on.
        let len = match rng.below(4) {
            0 => rng.below(8),
            1 => 90 + rng.below(20),
            2 => 190 + rng.below(20),
            _ => rng.below(260),
        };
        let input: Vec<u8> = (0..len).map(|_| alpha[rng.below(alpha.len())]).collect();
        assert_same(&format!("fuzz #{i} (len {len})"), &input);
    }
}
