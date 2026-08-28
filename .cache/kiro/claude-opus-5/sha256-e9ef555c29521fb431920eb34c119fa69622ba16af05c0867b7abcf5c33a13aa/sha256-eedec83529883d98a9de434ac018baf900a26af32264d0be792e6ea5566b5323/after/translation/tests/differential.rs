//! Differential tests: run the original C `driver` and the Rust `driver` as
//! subprocesses on identical stdin and require byte-identical stdout, stderr
//! and exit status.
//!
//! The Rust program is never linked as a library — it is driven exactly the way
//! a shell drives it, because that is how the translation is graded.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ==================== Locating / building the two binaries ====================

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The Rust binary for the profile `cargo test` was invoked with. Cargo builds
/// it before the test runs, so it always exists.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// The C binary, built with CMake on first use. `c_src/` itself is never
/// modified; only the untracked `c_src/build/` tree is created.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = manifest_dir()
            .parent()
            .expect("translation/ must have a parent")
            .join("c_src");
        assert!(
            c_src.join("CMakeLists.txt").is_file(),
            "expected {}/CMakeLists.txt",
            c_src.display()
        );

        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.is_file() {
            std::fs::create_dir_all(&build).expect("create c_src/build");

            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run `cmake ..` (is cmake installed?)");
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
                .expect("run `cmake --build .`");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(exe.is_file(), "C binary missing at {}", exe.display());
        exe
    })
}

// ==================== Running one program ====================

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "code={:?} signal={:?}\n  stdout={}\n  stderr={}",
            self.code,
            self.signal,
            summarize(&self.stdout),
            summarize(&self.stderr)
        )
    }
}

fn summarize(bytes: &[u8]) -> String {
    const LIMIT: usize = 400;
    if bytes.len() <= LIMIT {
        format!("{:?}", String::from_utf8_lossy(bytes))
    } else {
        format!(
            "{:?}... ({} bytes total)",
            String::from_utf8_lossy(&bytes[..LIMIT]),
            bytes.len()
        )
    }
}

/// Feed `input` to `exe` on stdin and collect everything it produced.
///
/// stdin is supplied from a real file so that neither large inputs nor large
/// outputs can deadlock against a pipe buffer.
fn run(exe: &Path, input: &[u8]) -> Outcome {
    use std::os::unix::process::ExitStatusExt;

    let dir = std::env::temp_dir().join(format!(
        "c2rust-diff-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("stdin.bin");
    {
        let mut f = std::fs::File::create(&path).expect("create stdin file");
        f.write_all(input).expect("write stdin file");
        f.flush().expect("flush stdin file");
    }
    let stdin = std::fs::File::open(&path).expect("open stdin file");

    let out = Command::new(exe)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", exe.display()));

    let _ = std::fs::remove_file(&path);

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Assert the C and Rust programs are indistinguishable on `input`.
#[track_caller]
fn same(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(rust_bin(), input);
    assert!(
        c == r,
        "differential mismatch [{label}]\n\
         input ({} bytes): {}\n\
         C  : {c:?}\n\
         Rust: {r:?}",
        input.len(),
        summarize(input),
    );
}

#[track_caller]
fn same_str(label: &str, input: &str) {
    same(label, input.as_bytes());
}

// ==================== Input builders ====================

/// `len v0 v1 ...` — one buffer as `read_buffer` expects it.
fn buf(vals: &[i64]) -> String {
    let mut s = vals.len().to_string();
    for v in vals {
        s.push(' ');
        s.push_str(&v.to_string());
    }
    s
}

/// Deterministic filler bytes.
fn fill(n: usize) -> Vec<i64> {
    (0..n).map(|i| ((i * 31 + 7) % 256) as i64).collect()
}

// ==================== Phase A: the binaries run at all ====================

#[test]
fn both_binaries_are_runnable() {
    // `<c_src>/build/driver` and `<target>/<profile>/driver`, driven as a shell
    // would: `driver < input`.
    assert!(c_bin().is_file());
    assert!(rust_bin().is_file());
    same("smoke", b"6 1 1 5");
}

// ==================== Phase B: every branch in the C ====================

// ---- reading the operation (main's first scanf) ----

#[test]
fn operation_unreadable() {
    same("empty", b"");
    same("newline only", b"\n");
    same("spaces only", b"   ");
    same("all whitespace", b" \t\n\r\x0b\x0c ");
    same("alpha", b"abc");
    same("punctuation", b"!!");
    same("lone minus", b"-");
    same("lone plus", b"+");
    same("minus then alpha", b"-x 1 0");
    same("double minus", b"--5 1 0");
    same("nul byte", b"\x00");
    same("nul before digits", b"\x006 1 1 5");
    same("hex-looking", b"0x10 2 0 0");
}

// ---- reading the buffer count (main's second scanf) ----

#[test]
fn buffer_count_unreadable() {
    same("op only", b"0");
    same("op then eof ws", b"0   \n");
    same("op then garbage", b"0 abc");
    same("op then minus", b"0 -");
}

#[test]
fn buffer_count_out_of_range() {
    for c in ["0", "-1", "-100", "101", "1000", "2147483647", "-2147483648"] {
        same_str(&format!("count {c}"), &format!("6 {c} 1 5"));
    }
}

#[test]
fn buffer_count_boundaries() {
    // 1 and 100 are accepted; 0 and 101 are not.
    same_str("count 1", &format!("6 1 {}", buf(&fill(3))));
    let hundred: Vec<String> = (0..100).map(|i| buf(&fill(i % 4))).collect();
    same_str("count 100", &format!("6 100 {}", hundred.join(" ")));
    same_str("count 101", &format!("6 101 {}", hundred.join(" ")));
}

// ---- read_buffer: length validation and byte reading ----

#[test]
fn buffer_length_unreadable() {
    same("no length", b"6 1");
    same("length garbage", b"6 1 xyz");
    same("second buffer length missing", b"6 2 1 5");
}

#[test]
fn buffer_length_out_of_range() {
    for l in ["-1", "-256", "257", "1000", "2147483647"] {
        same_str(&format!("len {l}"), &format!("6 1 {l}"));
    }
}

#[test]
fn buffer_length_boundaries() {
    same("len 0", b"6 1 0");
    same_str("len 1", &format!("6 1 {}", buf(&fill(1))));
    same_str("len 255", &format!("6 1 {}", buf(&fill(255))));
    same_str("len 256 (max)", &format!("6 1 {}", buf(&fill(256))));
    same_str("len 257 (over max)", &format!("6 1 257 {}", "1 ".repeat(257)));
}

#[test]
fn byte_read_failure() {
    same("no bytes at all", b"6 1 3");
    same("one byte short", b"6 1 3 1 2");
    same("byte is garbage", b"6 1 3 1 2 zz");
    same("fails on byte 0", b"6 1 2 q");
    same("fails in second buffer", b"6 2 2 1 2 2 9");
}

#[test]
fn byte_values_truncate_to_u8() {
    // `buf->data[i] = (uint8_t)byte` — a plain narrowing cast.
    same("256/257/-1", b"6 1 3 256 257 -1");
    same("multiples of 256", b"6 1 4 0 256 512 65536");
    same("2^32", b"6 1 1 4294967296");
    same("negatives", b"6 1 4 -1 -255 -256 -257");
    same("int extremes", b"6 1 4 2147483647 2147483648 -2147483648 -2147483649");
    same("saturating magnitudes", b"6 1 2 99999999999999999999999999 -99999999999999999999999999");
    // Printed via `%u` on a uint8_t, so 0..=255 only.
    same_str("all byte values", &format!("1 1 {}", buf(&(0..256).collect::<Vec<i64>>())));
}

// ---- OP_COPY (0) ----

#[test]
fn op_copy() {
    same_str("copy 2 buffers", &format!("0 2 {} {}", buf(&fill(3)), buf(&fill(5))));
    same("copy needs 2 buffers", b"0 1 3 1 2 3");
    same("copy with 1 empty buffer", b"0 1 0");
    same_str("copy empty first", &format!("0 2 0 {}", buf(&fill(4))));
    same_str("copy max first", &format!("0 2 {} 0", buf(&fill(256))));
    same_str("copy 3 buffers", &format!("0 3 {} {} {}", buf(&fill(2)), buf(&fill(3)), buf(&fill(4))));
}

// ---- OP_REVERSE (1) ----

#[test]
fn op_reverse() {
    same("reverse empty", b"1 1 0");
    same("reverse single byte", b"1 1 1 42");
    same("reverse two bytes", b"1 1 2 1 2");
    same("reverse odd", b"1 1 3 1 2 3");
    same_str("reverse max", &format!("1 1 {}", buf(&fill(256))));
    same_str("reverse many", &format!("1 3 {} {} {}", buf(&fill(0)), buf(&fill(1)), buf(&fill(7))));
}

// ---- OP_MERGE (2) ----

#[test]
fn op_merge() {
    same_str("merge basic", &format!("2 2 {} {}", buf(&fill(2)), buf(&fill(3))));
    same("merge needs 2 buffers", b"2 1 3 1 2 3");
    same("merge both empty", b"2 2 0 0");
    same_str("merge empty+full", &format!("2 2 0 {}", buf(&fill(9))));
    same_str("merge full+empty", &format!("2 2 {} 0", buf(&fill(9))));
    // sum == 256 is allowed, 257 is not.
    same_str("merge sum 255", &format!("2 2 {} {}", buf(&fill(128)), buf(&fill(127))));
    same_str("merge sum 256", &format!("2 2 {} {}", buf(&fill(128)), buf(&fill(128))));
    same_str("merge sum 257", &format!("2 2 {} {}", buf(&fill(129)), buf(&fill(128))));
    same_str("merge sum 512", &format!("2 2 {} {}", buf(&fill(256)), buf(&fill(256))));
    // Only buffers 0 and 1 participate, extra buffers are still read.
    same_str("merge ignores 3rd", &format!("2 3 {} {} {}", buf(&fill(2)), buf(&fill(2)), buf(&fill(200))));
}

// ---- OP_SPLIT (3) ----

#[test]
fn op_split() {
    same("split position missing", b"3 1 2 1 2");
    same("split position garbage", b"3 1 2 1 2 zz");
    same("split empty at 0", b"3 1 0 0");
    same("split at 0", b"3 1 3 1 2 3 0");
    same("split in middle", b"3 1 4 1 2 3 4 2");
    same("split at len", b"3 1 3 1 2 3 3");
    same("split past len", b"3 1 3 1 2 3 4");
    // `int` -> `size_t`: a negative position becomes a huge unsigned value and
    // is reported verbatim by `%zu`.
    same("split -1", b"3 1 3 1 2 3 -1");
    same("split -1000", b"3 1 3 1 2 3 -1000");
    same("split INT_MIN", b"3 1 3 1 2 3 -2147483648");
    same("split empty at 1", b"3 1 0 1");
    same_str("split max at 0", &format!("3 1 {} 0", buf(&fill(256))));
    same_str("split max at 128", &format!("3 1 {} 128", buf(&fill(256))));
    same_str("split max at 256", &format!("3 1 {} 256", buf(&fill(256))));
    same_str("split max at 257", &format!("3 1 {} 257", buf(&fill(256))));
    same_str("split uses buffer 0", &format!("3 2 {} {} 1", buf(&fill(3)), buf(&fill(200))));
}

// ---- OP_INTERLEAVE (4) ----

#[test]
fn op_interleave() {
    same_str("interleave equal", &format!("4 2 {} {}", buf(&fill(4)), buf(&fill(4))));
    same_str("interleave first longer", &format!("4 2 {} {}", buf(&fill(6)), buf(&fill(2))));
    same_str("interleave second longer", &format!("4 2 {} {}", buf(&fill(2)), buf(&fill(6))));
    same("interleave needs 2 buffers", b"4 1 2 1 2");
    same("interleave both empty", b"4 2 0 0");
    same_str("interleave empty+full", &format!("4 2 0 {}", buf(&fill(5))));
    same_str("interleave full+empty", &format!("4 2 {} 0", buf(&fill(5))));
    same_str("interleave sum 256", &format!("4 2 {} {}", buf(&fill(128)), buf(&fill(128))));
    same_str("interleave sum 257", &format!("4 2 {} {}", buf(&fill(129)), buf(&fill(128))));
    same_str("interleave lopsided 256", &format!("4 2 {} {}", buf(&fill(256)), buf(&fill(0))));
    same_str("interleave lopsided 257", &format!("4 2 {} {}", buf(&fill(256)), buf(&fill(1))));
}

// ---- OP_ROTATE (5) ----

#[test]
fn op_rotate() {
    same("rotate amount missing", b"5 1 2 1 2");
    same("rotate amount garbage", b"5 1 2 1 2 zz");
    same("rotate by 0", b"5 1 3 1 2 3 0");
    same("rotate empty buffer", b"5 1 0 3");
    same("rotate empty by 0", b"5 1 0 0");
    same("rotate by 1", b"5 1 4 1 2 3 4 1");
    same("rotate by len", b"5 1 4 1 2 3 4 4");
    same("rotate past len", b"5 1 4 1 2 3 4 7");
    same("rotate negative", b"5 1 4 1 2 3 4 -1");
    same("rotate negative past len", b"5 1 4 1 2 3 4 -7");
    same("rotate INT_MIN", b"5 1 3 1 2 3 -2147483648");
    same("rotate INT_MAX", b"5 1 3 1 2 3 2147483647");
    same("rotate len 1", b"5 1 1 9 5");
    same_str("rotate max buffer", &format!("5 1 {} 100", buf(&fill(256))));
    same_str("rotate many buffers", &format!("5 3 {} {} {} 2", buf(&fill(0)), buf(&fill(1)), buf(&fill(5))));
}

// ---- OP_CHECKSUM (6) ----

#[test]
fn op_checksum() {
    same("checksum empty", b"6 1 0");
    same("checksum single", b"6 1 1 1");
    // `sum = (sum << 3) ^ byte` on uint32_t: overflow wraps and bits fall off.
    same_str("checksum overflow wraps", &format!("6 1 {}", buf(&vec![255i64; 256])));
    same_str("checksum zeros", &format!("6 1 {}", buf(&vec![0i64; 256])));
    same_str("checksum max len", &format!("6 1 {}", buf(&fill(256))));
    same_str("checksum many", &format!("6 4 0 {} {} {}", buf(&fill(1)), buf(&fill(11)), buf(&fill(256))));
}

// ---- the default arm ----

#[test]
fn unknown_operation() {
    // Note the ordering: the operation is validated only *after* every buffer
    // has been read, so an unreadable buffer reports first.
    for op in ["7", "8", "100", "-1", "-2", "2147483647", "-2147483648"] {
        same_str(&format!("op {op}"), &format!("{op} 1 0"));
    }
    same("unknown op after bad buffer", b"7 1 999");
    same("unknown op after short buffer", b"7 1 3 1");
    same_str("unknown op reads all buffers", &format!("9 2 {} {}", buf(&fill(3)), buf(&fill(4))));
}

// ==================== Phase C: scanf semantics and overflow ====================

#[test]
fn scanf_crosses_newlines_and_whitespace() {
    same("newline separated", b"6\n1\n1\n5\n");
    same("tabs", b"6\t1\t1\t5");
    same("mixed whitespace", b"\n\t 6 \r\n 1 \n 1 \x0b 9 \x0c");
    same("no trailing newline", b"6 1 1 5");
    same("many trailing newlines", b"6 1 1 5\n\n\n");
    same("crlf", b"6\r\n1\r\n1\r\n5\r\n");
    same("leading whitespace flood", &[b" ".repeat(30000), b"6 1 1 5".to_vec()].concat());
    same(
        "interior whitespace flood",
        &[b"6".to_vec(), b"\n".repeat(20000), b"1 1 5".to_vec()].concat(),
    );
}

#[test]
fn scanf_sign_and_leading_zeros() {
    same("plus sign op", b"+6 1 1 7");
    same("plus sign byte", b"6 1 1 +7");
    same("leading zeros", b"6 1 1 00000000000000000007");
    same("zero", b"6 1 1 0");
    same("minus zero", b"6 1 1 -0");
    same("plus zero split", b"3 1 2 1 2 +0");
    same("300 leading zeros", &[b"6 1 1 ".to_vec(), b"0".repeat(300), b"5".to_vec()].concat());
}

#[test]
fn scanf_stops_at_first_non_digit() {
    same("digits then alpha", b"6 1 1 5abc");
    same("digits then punct", b"6 1 1 5,6");
    same("trailing garbage ignored", b"6 1 1 5 GARBAGE HERE");
    same("trailing garbage after reverse", b"1 2 2 1 2 2 3 4 EXTRA");
    same("digits then nul", b"6 1 1 5\x00");
}

#[test]
fn scanf_out_of_range_saturates_then_truncates() {
    // glibc converts through `long`, saturating at LONG_MAX / LONG_MIN, then
    // narrows to `int`.
    let cases = [
        "2147483647",
        "2147483648",
        "4294967295",
        "4294967296",
        "4294967302",
        "9223372036854775807",
        "9223372036854775808",
        "18446744073709551615",
        "18446744073709551616",
        "-2147483648",
        "-2147483649",
        "-4294967296",
        "-9223372036854775808",
        "-9223372036854775809",
        "-18446744073709551616",
    ];
    for v in cases {
        same_str(&format!("op {v}"), &format!("{v} 1 0"));
        same_str(&format!("count {v}"), &format!("6 {v} 1 5"));
        same_str(&format!("len {v}"), &format!("6 1 {v} 5"));
        same_str(&format!("byte {v}"), &format!("6 1 1 {v}"));
        same_str(&format!("split {v}"), &format!("3 1 3 1 2 3 {v}"));
        same_str(&format!("rotate {v}"), &format!("5 1 3 1 2 3 {v}"));
    }
    // Digit runs far longer than any integer type.
    let nines = "9".repeat(400);
    same_str("400 nines op", &format!("{nines} 1 0"));
    same_str("400 nines negative op", &format!("-{nines} 1 0"));
    same_str("400 nines byte", &format!("6 1 1 {nines}"));
    same_str("400 nines negative byte", &format!("6 1 1 -{nines}"));
    same_str("400 nines count", &format!("6 {nines} 1 5"));
    same_str("400 nines len", &format!("6 1 {nines} 5"));
    // Powers of two straddling every conversion boundary. Magnitudes are built
    // in `u128` and the sign is applied textually, so the test itself cannot
    // overflow while probing values well beyond `long`.
    for p in [7u32, 15, 16, 31, 32, 33, 52, 53, 63, 64, 65, 79, 80, 126, 127] {
        let base: u128 = 1u128 << p;
        for off in [-1i32, 0, 1] {
            let mag = match off {
                -1 => base - 1,
                1 => base + 1,
                _ => base,
            };
            same_str(&format!("op 2^{p}{off:+}"), &format!("{mag} 1 0"));
            same_str(&format!("byte 2^{p}{off:+}"), &format!("6 1 1 {mag}"));
            same_str(&format!("op -2^{p}{off:+}"), &format!("-{mag} 1 0"));
            same_str(&format!("byte -2^{p}{off:+}"), &format!("6 1 1 -{mag}"));
            same_str(&format!("count 2^{p}{off:+}"), &format!("6 {mag} 1 5"));
            same_str(&format!("len 2^{p}{off:+}"), &format!("6 1 {mag} 5"));
            same_str(&format!("split 2^{p}{off:+}"), &format!("3 1 3 1 2 3 {mag}"));
            same_str(&format!("rotate -2^{p}{off:+}"), &format!("5 1 3 1 2 3 -{mag}"));
        }
    }
}

#[test]
fn numeric_token_straddles_read_buffer_boundary() {
    // The Rust scanner refills an internal buffer; a token that spans the
    // refill point must still convert as one number.
    for off in 8180..8200usize {
        let pad = b" ".repeat(off);
        same("token at boundary", &[pad.clone(), b"123456 1 1 5".to_vec()].concat());
        same("token2 at boundary", &[pad, b"6 1 1 200".to_vec()].concat());
    }
    same(
        "9000-digit token",
        &[b"6 1 1 ".to_vec(), b"1".repeat(9000)].concat(),
    );
    same(
        "9000-digit negative token",
        &[b"-".to_vec(), b"1".repeat(9000), b" 1 0".to_vec()].concat(),
    );
}

#[test]
fn every_prefix_of_a_valid_input() {
    // Truncating at each byte walks the EOF path of every scanf in turn.
    for base in [
        &b"4 3 3 1 2 3 2 9 8 1 5 7"[..],
        &b"5 2 2 10 20 3 30 40 50 -1"[..],
        &b"3 1 4 11 22 33 44 2"[..],
        &b"0 2 2 1 2 2 3 4"[..],
        &b"6 2 1 250 1 251"[..],
    ] {
        for k in 0..=base.len() {
            same("prefix", &base[..k]);
        }
    }
}

#[test]
fn stdin_is_immediately_eof() {
    same("empty file", b"");
}

// ==================== Phase C: exhaustive domain sweeps ====================

#[test]
fn sweep_split_positions() {
    for len in [0usize, 1, 2, 3, 255, 256] {
        let b0 = buf(&fill(len));
        let mut positions: Vec<i64> = (-2..=(len as i64 + 2)).collect();
        positions.extend([-1000, 1000, i32::MIN as i64, i32::MAX as i64]);
        for p in positions {
            same_str(&format!("split len={len} pos={p}"), &format!("3 1 {b0} {p}"));
        }
    }
}

#[test]
fn sweep_rotate_positions() {
    for len in [0usize, 1, 2, 3, 255, 256] {
        let b0 = buf(&fill(len));
        let l = len as i64;
        let mut positions: Vec<i64> = (-l - 2..=l + 2).collect();
        positions.extend([100000, -100000, i32::MIN as i64, i32::MAX as i64]);
        for p in positions {
            same_str(&format!("rotate len={len} pos={p}"), &format!("5 1 {b0} {p}"));
        }
    }
}

#[test]
fn sweep_two_buffer_length_pairs() {
    // The merge/interleave cap depends only on `len1 + len2`, so each sum around
    // the 256-byte boundary is enumerated, sampling the decompositions rather
    // than taking all of them, plus a coarse grid over the whole domain.
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for sum in [0usize, 1, 2, 3, 254, 255, 256, 257, 258, 512] {
        let lo = sum.saturating_sub(256);
        let hi = sum.min(256);
        let mut a = lo;
        while a <= hi {
            pairs.push((a, sum - a));
            a += 16;
        }
        pairs.push((lo, sum - lo));
        pairs.push((hi, sum - hi));
    }
    for a in (0..=256).step_by(64) {
        for b in (0..=256).step_by(64) {
            pairs.push((a, b));
        }
    }
    pairs.sort_unstable();
    pairs.dedup();

    for (a, b) in pairs {
        let body = format!("{} {}", buf(&fill(a)), buf(&fill(b)));
        for op in [0, 1, 2, 4, 6] {
            same_str(&format!("op={op} lens={a}+{b}"), &format!("{op} 2 {body}"));
        }
        same_str(&format!("split lens={a}+{b}"), &format!("3 2 {body} {}", a / 2));
        same_str(&format!("rotate lens={a}+{b}"), &format!("5 2 {body} 3"));
    }
}

#[test]
fn sweep_every_buffer_length() {
    for len in 0..=256usize {
        let b0 = buf(&fill(len));
        same_str(&format!("reverse len={len}"), &format!("1 1 {b0}"));
        same_str(&format!("checksum len={len}"), &format!("6 1 {b0}"));
        let ff = buf(&vec![255i64; len]);
        same_str(&format!("reverse ff len={len}"), &format!("1 1 {ff}"));
        same_str(&format!("checksum ff len={len}"), &format!("6 1 {ff}"));
    }
}

#[test]
fn sweep_every_buffer_count_and_operation() {
    for n in 1..=100usize {
        let body: Vec<String> = (0..n).map(|i| buf(&fill(i % 5))).collect();
        let body = body.join(" ");
        for op in 0..=7 {
            let extra = if op == 3 || op == 5 { " 1" } else { "" };
            same_str(
                &format!("count={n} op={op}"),
                &format!("{op} {n} {body}{extra}"),
            );
        }
    }
}

#[test]
fn maximum_workload_for_every_operation() {
    // 100 buffers of 256 bytes: the largest input and output the C accepts.
    let bufs: Vec<String> = (0..100)
        .map(|i| buf(&(0..256).map(|j| ((i * 7 + j) % 256) as i64).collect::<Vec<i64>>()))
        .collect();
    let body = bufs.join(" ");
    for op in [0, 1, 2, 4, 6] {
        same_str(&format!("max op={op}"), &format!("{op} 100 {body}"));
    }
    same_str("max split", &format!("3 100 {body} 200"));
    same_str("max rotate", &format!("5 100 {body} 77"));
    same_str("max unknown op", &format!("42 100 {body}"));
}

// ==================== Phase C: process-level behavior ====================

#[test]
fn argv_is_ignored() {
    // `main` declares argc/argv but never touches them.
    for args in [vec!["foo"], vec!["--help"], vec!["-x", "y", "z"]] {
        let c = run_with_args(c_bin(), b"6 1 1 5", &args);
        let r = run_with_args(rust_bin(), b"6 1 1 5", &args);
        assert!(c == r, "argv mismatch for {args:?}\nC  : {c:?}\nRust: {r:?}");
    }
}

fn run_with_args(exe: &Path, input: &[u8], args: &[&str]) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input)
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

#[test]
fn stdin_closed_rather_than_empty() {
    // fd 0 unreadable: the first scanf fails just as it does at EOF.
    let c = run_with_stdin_closed(c_bin());
    let r = run_with_stdin_closed(rust_bin());
    assert!(c == r, "closed-stdin mismatch\nC  : {c:?}\nRust: {r:?}");
}

fn run_with_stdin_closed(exe: &Path) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    // `exe 0<&-` — the shell closes fd 0 before exec.
    let out = Command::new("bash")
        .arg("-c")
        .arg(format!("exec 0<&-; exec {}", shell_quote(exe)))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run bash");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

#[test]
fn stdout_reader_closes_early_yields_same_wait_status() {
    // The C program keeps SIGPIPE at its default disposition, so a reader that
    // stops early kills it with signal 13 (wait status 141). The Rust runtime
    // ignores SIGPIPE by default, which would instead surface as a panic, so the
    // translation has to restore the default handler.
    let input_path = std::env::temp_dir().join(format!("c2rust-sigpipe-{}.bin", std::process::id()));
    // ~100 KB of output, comfortably more than one pipe buffer, so the writer
    // is guaranteed to still be writing when the reader goes away.
    let bufs: Vec<String> = (0..100).map(|_| buf(&fill(256))).collect();
    std::fs::write(&input_path, format!("1 100 {}", bufs.join(" "))).expect("write input");

    let status_of = |exe: &Path| -> String {
        let out = Command::new("bash")
            .arg("-c")
            .arg(format!(
                "{} < {} 2>/dev/null | head -c 16 > /dev/null; echo ${{PIPESTATUS[0]}}",
                shell_quote(exe),
                shell_quote(&input_path)
            ))
            .output()
            .expect("run bash pipeline");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    let c = status_of(c_bin());
    let r = status_of(rust_bin());
    let _ = std::fs::remove_file(&input_path);
    assert_eq!(c, r, "broken-pipe wait status differs (C={c}, Rust={r})");
    assert_eq!(c, "141", "expected SIGPIPE termination, got {c}");
}

fn shell_quote(p: &Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', r"'\''"))
}
