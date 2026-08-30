//! Differential tests: run the reference C program and the Rust translation as
//! subprocesses on identical stdin and require identical stdout, stderr and
//! exit status.
//!
//! Nothing here links against the translation as a library; both sides are
//! driven exactly the way a shell drives them, because that is how the two
//! programs are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// locating / building the two programs
// ---------------------------------------------------------------------------

/// `translation/`
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The directory that holds both `c_src/` and `translation/`.
fn workspace_dir() -> PathBuf {
    crate_dir().parent().expect("crate has a parent").to_path_buf()
}

/// The Rust program: `cargo` builds it for us and hands us the path.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// The C program, built out of tree with CMake exactly as documented in
/// `c_src/CMakeLists.txt`.  Built once per test binary.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_dir().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run cmake (is cmake installed?)");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}",
                String::from_utf8_lossy(&cfg.stderr)
            );
            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run cmake --build");
            assert!(
                bld.status.success(),
                "cmake --build failed:\n{}",
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(exe.exists(), "reference binary missing at {}", exe.display());
        exe
    })
}

// ---------------------------------------------------------------------------
// running them
// ---------------------------------------------------------------------------

/// stdout, stderr, exit code, terminating signal.
#[derive(PartialEq, Eq, Clone)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout={:?} stderr={:?} code={:?} signal={:?}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            self.code,
            self.signal
        )
    }
}

impl From<Output> for Run {
    fn from(o: Output) -> Self {
        #[cfg(unix)]
        let signal = {
            use std::os::unix::process::ExitStatusExt;
            o.status.signal()
        };
        #[cfg(not(unix))]
        let signal = None;
        Run {
            stdout: o.stdout,
            stderr: o.stderr,
            code: o.status.code(),
            signal,
        }
    }
}

fn feed(bin: &Path, stdin_data: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    {
        let mut si = child.stdin.take().expect("stdin pipe");
        // The C program may exit before reading everything (e.g. a length
        // error), which closes the pipe: a broken pipe here is not a failure.
        let _ = si.write_all(stdin_data);
        let _ = si.flush();
    }
    Run::from(child.wait_with_output().expect("collect output"))
}

/// Runs both programs on `stdin_data` and asserts stdout, stderr and exit
/// status all match byte for byte.
///
/// The reference program is run three times first.  Some inputs make the C code
/// read uninitialised stack bytes that ASLR randomises, and for those inputs the
/// C program has no single answer to match; this catches such a case as an
/// explicit failure instead of letting it flake.
#[track_caller]
fn assert_same(name: &str, stdin_data: &[u8]) {
    dump(name, stdin_data);

    let c = feed(c_bin(), stdin_data);
    for _ in 0..2 {
        let again = feed(c_bin(), stdin_data);
        assert!(
            again == c,
            "[{name}] the reference program is not deterministic on this input, \
             so it cannot be used as an oracle here\n  input: {:?}\n  run A: {c:?}\n  run B: {again:?}",
            show(stdin_data)
        );
    }

    let r = feed(rust_bin(), stdin_data);
    assert!(
        r.stdout == c.stdout,
        "[{name}] stdout differs\n  input: {:?}\n  C:    {:?}\n  Rust: {:?}",
        show(stdin_data),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert!(
        r.stderr == c.stderr,
        "[{name}] stderr differs\n  input: {:?}\n  C:    {:?}\n  Rust: {:?}",
        show(stdin_data),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert!(
        r.code == c.code && r.signal == c.signal,
        "[{name}] exit status differs\n  input: {:?}\n  C:    code={:?} signal={:?}\n  Rust: code={:?} signal={:?}",
        show(stdin_data),
        c.code,
        c.signal,
        r.code,
        r.signal
    );
}

/// Appends `name` and `stdin_data` to the file named by `$DIFF_DUMP_INPUTS`,
/// if that variable is set.  Used to re-check the whole enumerated corpus for
/// reference-program determinism with far more repetitions than a test run can
/// afford; see `translation/ERRORS.md`.
fn dump(name: &str, stdin_data: &[u8]) {
    let Ok(path) = std::env::var("DIFF_DUMP_INPUTS") else {
        return;
    };
    use std::sync::Mutex;
    static LOCK: Mutex<()> = Mutex::new(());
    let _g = LOCK.lock().unwrap();
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open DIFF_DUMP_INPUTS");
    let header = format!("{}\t{}\n", name.len(), stdin_data.len());
    f.write_all(header.as_bytes()).unwrap();
    f.write_all(name.as_bytes()).unwrap();
    f.write_all(stdin_data).unwrap();
}

/// Abbreviated rendering of an input, so a failure on a 1024-byte buffer stays
/// readable.
fn show(data: &[u8]) -> String {
    let s = String::from_utf8_lossy(data);
    if s.len() <= 120 {
        s.into_owned()
    } else {
        format!("{}... ({} bytes)", &s[..120], data.len())
    }
}

// ---------------------------------------------------------------------------
// input builders
// ---------------------------------------------------------------------------

/// Builds a well-formed stdin: operation, flags, input buffer, reference buffer.
fn input(op: i64, flags: u64, ib: &[u8], rb: &[u8]) -> Vec<u8> {
    let mut s = format!("{op} {flags} {}", ib.len());
    for b in ib {
        s.push_str(&format!(" {b}"));
    }
    s.push_str(&format!(" {}", rb.len()));
    for b in rb {
        s.push_str(&format!(" {b}"));
    }
    s.push('\n');
    s.into_bytes()
}

/// A C string: the bytes plus the NUL terminator, so the buffer the C code sees
/// really is terminated inside `input_len`.
fn z(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

/// The bytes without a terminator, the way a caller who trusts `input_len`
/// would pass them.
fn raw(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

// ===========================================================================
// Phase B / C - main.c: the read-and-validate paths
// ===========================================================================

#[test]
fn operation_read_failures() {
    // `scanf("%d", &operation) != 1`
    assert_same("empty stdin", b"");
    assert_same("whitespace only", b"   \t\n\n  ");
    assert_same("not a number", b"hello\n");
    assert_same("lone sign", b"+");
    assert_same("lone minus", b"-\n");
    assert_same("double sign", b"--3 0 0 0");
    assert_same("float", b"1e3 0 0 0");
}

#[test]
fn flags_read_failures() {
    // `scanf("%u", &flags) != 1`
    assert_same("operation only", b"0");
    assert_same("operation then newline", b"0\n");
    assert_same("operation then junk", b"0 x");
}

#[test]
fn input_length_read_failures() {
    // `scanf("%zu", &input_len) != 1`
    assert_same("no input length", b"0 0");
    assert_same("junk input length", b"0 0 abc");
}

#[test]
fn input_length_bounds() {
    // `input_len > MAX_BUFFER_SIZE`
    assert_same("input len 1025", b"0 0 1025 0");
    assert_same("input len 2048", b"0 0 2048 0");
    // `%zu` runs through strtoul, so a negative literal wraps to ULONG_MAX and
    // the message prints that value.
    assert_same("input len -1", b"0 0 -1");
    assert_same("input len -1024", b"0 0 -1024");
    assert_same("input len 2^32", b"0 0 4294967296 0");
    assert_same("input len overflow", b"0 0 99999999999999999999 0");
    // 1024 itself is accepted.
    let full = vec![b'A'; 1024];
    assert_same("input len exactly 1024", &input(0, 0, &full, &[]));
}

#[test]
fn input_byte_read_failures() {
    // `scanf("%u", &byte) != 1` - the message carries the byte index.
    assert_same("byte 0 missing", b"0 0 5");
    assert_same("byte 0 junk", b"0 0 5 zz");
    assert_same("byte 3 missing", b"0 0 5 65 66 67");
    assert_same("byte 9 missing", b"0 0 10 65 66 67 68 69 70 71 72 73");
    assert_same("byte is negative sign only", b"0 0 2 65 -");
}

#[test]
fn reference_length_read_failures() {
    assert_same("no ref length", b"0 0 3 65 66 67");
    assert_same("junk ref length", b"0 0 3 65 66 67 zz");
    assert_same("no ref length, empty input", b"0 0 0");
}

#[test]
fn reference_length_bounds() {
    assert_same("ref len 1025", b"0 0 0 1025");
    assert_same("ref len -1", b"0 0 0 -1");
    assert_same("ref len 2^32", b"0 0 0 4294967296");
    let full = vec![b'A'; 1024];
    assert_same("ref len exactly 1024", &input(0, 0, &full, &full));
}

#[test]
fn reference_byte_read_failures() {
    assert_same("ref byte 0 missing", b"0 0 1 65 4");
    assert_same("ref byte 2 missing", b"0 0 1 65 4 66 67");
}

#[test]
fn scanf_reads_across_newlines_and_whitespace() {
    // `scanf` skips any run of whitespace, newlines included, so the layout of
    // the input makes no difference.
    assert_same("one per line", b"0\n0\n4\n86\n65\n76\n0\n0\n");
    assert_same("crlf", b"0\r\n0\r\n0\r\n0\r\n");
    assert_same("vertical tab and form feed", b"\x0b0\x0c0\x0b0\x0c0");
    assert_same("leading whitespace", b"\n\n\t   0 0 0 0");
    assert_same("no trailing newline", b"0 0 0 0");
    assert_same("trailing junk is never read", b"0 0 0 0 and then some words");
    assert_same("explicit plus signs", b"+3 +0 +0 +0");
}

#[test]
fn scanf_integer_conversion_edges() {
    // `%d` goes through strtol: it saturates at LONG_MAX/LONG_MIN and the
    // result is then truncated into an `int`.
    assert_same("op INT_MAX", b"2147483647 0 0 0");
    assert_same("op INT_MAX+1", b"2147483648 0 0 0");
    assert_same("op INT_MIN", b"-2147483648 0 0 0");
    assert_same("op INT_MIN-1", b"-2147483649 0 0 0");
    assert_same("op huge", b"99999999999999999999 0 0 0");
    assert_same("op huge negative", b"-99999999999999999999 0 0 0");
    assert_same("op 2^32", b"4294967296 0 0 0");
    assert_same("op 2^32-1", b"4294967295 0 0 0");
    // `%u` goes through strtoul: negatives wrap, huge values saturate.
    assert_same("flags UINT_MAX", b"0 4294967295 0 0");
    assert_same("flags 2^32", b"0 4294967296 0 0");
    assert_same("flags -1", b"0 -1 0 0");
    assert_same("flags -5", b"0 -5 0 0");
    assert_same("flags huge", b"0 18446744073709551617 0 0");
    // strtoul saturates at ULONG_MAX *before* the sign is applied, so a
    // negative magnitude that does not fit does not come back as a small
    // number.
    assert_same("len -(2^64)", b"0 0 -18446744073709551616 0");
    assert_same("len -(2^64+1)", b"0 0 -18446744073709551617 0");
    assert_same("len 40 digit negative", b"0 0 -9999999999999999999999999999999999999999 0");
    assert_same("len 40 digit positive", b"0 0 9999999999999999999999999999999999999999 0");
    assert_same("len 100 digits", &{
        let mut v = b"0 0 ".to_vec();
        v.extend(std::iter::repeat(b'9').take(100));
        v.extend_from_slice(b" 0");
        v
    });
    assert_same("flags -(2^64)", b"0 -18446744073709551616 0 0");
    assert_same("op -(2^64)", b"-18446744073709551616 0 0 0");
    assert_same("op ULONG_MAX", b"18446744073709551615 0 0 0");
    assert_same("op -ULONG_MAX", b"-18446744073709551615 0 0 0");
    assert_same("op LONG_MAX", b"9223372036854775807 0 0 0");
    assert_same("op LONG_MAX+1", b"9223372036854775808 0 0 0");
    assert_same("op LONG_MIN", b"-9223372036854775808 0 0 0");
    assert_same("op LONG_MIN-1", b"-9223372036854775809 0 0 0");
    assert_same("byte -(2^64)", b"0 0 1 -18446744073709551616 0");
    // A '-' in the middle of a run of digits ends the token.
    assert_same("digits then minus", b"12-34 0 0 0");
    assert_same("zero then minus", b"0 0 0- 0");
    // Leading zeros are decimal, and "0x41" stops at the 'x'.
    assert_same("byte with leading zeros", b"0 0 1 0041 0");
    assert_same("byte written as hex", b"0 0 1 0x41 0");
    assert_same("many leading zeros", b"0 0 000000000000000000000000000001 0 1 0");
}

#[test]
fn byte_values_are_truncated_to_char() {
    // `input_buffer[i] = (char)byte` - the high bits of the `unsigned int` are
    // simply dropped.
    for v in ["0", "1", "65", "127", "128", "255", "256", "300", "511", "65601",
              "4294967295", "4294967296"] {
        let data = format!("0 0 2 {v} 0 2 {v} 0\n");
        assert_same(&format!("byte value {v}"), data.as_bytes());
    }
}

// ===========================================================================
// Phase B / C - lib.c: process_strings dispatch
// ===========================================================================

#[test]
fn operation_dispatch_including_default() {
    for op in [0i64, 1, 2, 3, 4] {
        assert_same(
            &format!("operation {op}"),
            &input(op, 0, &z("ABC"), &z("ABC")),
        );
    }
    // `default: return -3`
    for op in [5i64, 6, 99, -1, -3, 2147483647, -2147483648] {
        assert_same(
            &format!("operation {op} hits default"),
            &input(op, 0, &z("ABC"), &z("ABC")),
        );
    }
}

// ---------------------------------------------------------------------------
// operation 0 - validate_token
// ---------------------------------------------------------------------------

#[test]
fn validate_token_paths() {
    // strcmp(token, expected) == 0
    assert_same("token equals reference", &input(0, 0, &z("ABC"), &z("ABC")));
    assert_same("token equals empty reference", &input(0, 0, &z(""), &z("")));
    // the two hard-coded variations
    assert_same("token VALID", &input(0, 0, &z("VALID"), &z("zzz")));
    assert_same("token OK", &input(0, 0, &z("OK"), &z("zzz")));
    // near misses
    assert_same("token valid lowercase", &input(0, 0, &z("valid"), &z("zzz")));
    assert_same("token VALIDX", &input(0, 0, &z("VALIDX"), &z("zzz")));
    assert_same("token OKX", &input(0, 0, &z("OKX"), &z("zzz")));
    assert_same("token mismatch", &input(0, 0, &z("ABC"), &z("ABD")));
    assert_same("token is a prefix of reference", &input(0, 0, &z("AB"), &z("ABC")));
    assert_same("reference is a prefix of token", &input(0, 0, &z("ABC"), &z("AB")));
    // embedded NUL: strcmp stops there
    assert_same("embedded NUL", &input(0, 0, &[65, 0, 66, 0], &[65, 0, 67, 0]));
    // high-bit bytes: strcmp compares as unsigned char
    assert_same("high bytes equal", &input(0, 0, &[200, 201, 0], &[200, 201, 0]));
    assert_same("high vs low byte", &input(0, 0, &[200, 0], &[65, 0]));
}

// ---------------------------------------------------------------------------
// operation 1 - parse_command
// ---------------------------------------------------------------------------

#[test]
fn parse_command_every_command() {
    for (i, cmd) in ["START", "STOP", "PAUSE", "RESUME", "RESET"].iter().enumerate() {
        // terminated inside input_len: strncmp matches, buffer[cmd_len] == '\0'
        assert_same(
            &format!("command {cmd} index {i}"),
            &input(1, 0, &z(cmd), &[]),
        );
        // buffer[cmd_len] == ' ' also counts as an exact match
        assert_same(
            &format!("command {cmd} followed by space"),
            &input(1, 0, &z(&format!("{cmd} tail")), &[]),
        );
        // followed by something else: neither branch matches
        assert_same(
            &format!("command {cmd} followed by X"),
            &input(1, 0, &z(&format!("{cmd}X")), &[]),
        );
    }
}

#[test]
fn parse_command_admin_and_misses() {
    assert_same("ADMIN returns 99", &input(1, 0, &z("ADMIN"), &[]));
    assert_same("ADMIN with space", &input(1, 0, &z("ADMIN X"), &[]));
    assert_same("ADMINX", &input(1, 0, &z("ADMINX"), &[]));
    assert_same("no match", &input(1, 0, &z("NOPE"), &[]));
    assert_same("empty command", &input(1, 0, &z(""), &[]));
    // `buf_size >= cmd_len` guard: too short for START but long enough for others
    assert_same("buf shorter than every command", &input(1, 0, &z("ST"), &[]));
    assert_same("buf size 1", &input(1, 0, &[0], &[]));
    assert_same("buf size 4 STOP", &input(1, 0, &z("STOP"), &[]));
    // STOP is a prefix of nothing else; START vs STOP share "ST"
    assert_same("STA", &input(1, 0, &z("STA"), &[]));
    // reference is ignored entirely by operation 1
    assert_same("reference ignored", &input(1, 0, &z("START"), &z("ZZZZZZ")));
}

#[test]
fn parse_command_full_buffer() {
    // input_len == 1024, no terminator anywhere: `strlen` walks out of the
    // array into `ref_len`.
    let full = vec![b'A'; 1024];
    assert_same("parse_command full buffer", &input(1, 0, &full, &[]));
    let mut start_full = vec![b'A'; 1024];
    start_full[..5].copy_from_slice(b"START");
    assert_same("parse_command START then filler", &input(1, 0, &start_full, &[]));
    // "START" with a space at index 5 inside a full buffer
    let mut start_space = vec![b'A'; 1024];
    start_space[..6].copy_from_slice(b"START ");
    assert_same("parse_command START space filler", &input(1, 0, &start_space, &[]));
}

// ---------------------------------------------------------------------------
// operation 2 - compare_prefix
// ---------------------------------------------------------------------------

#[test]
fn compare_prefix_non_exact() {
    // flags bit 0 clear -> strncmp(str, prefix, strlen(prefix))
    assert_same("prefix match", &input(2, 0, &z("ABCDEF"), &z("ABC")));
    assert_same("prefix equal", &input(2, 0, &z("ABC"), &z("ABC")));
    assert_same("prefix mismatch", &input(2, 0, &z("ABCDEF"), &z("XYZ")));
    assert_same("prefix longer than str", &input(2, 0, &z("AB"), &z("ABC")));
    assert_same("empty prefix", &input(2, 0, &z("ABC"), &z("")));
    assert_same("empty both", &input(2, 0, &z(""), &z("")));
    assert_same("flags 2 also clears bit 0", &input(2, 2, &z("ABCDEF"), &z("ABC")));
    assert_same("flags 0xfffffffe", &input(2, 0xffff_fffe, &z("ABCDEF"), &z("ABC")));
}

#[test]
fn compare_prefix_exact_and_variations() {
    // flags bit 0 set -> strcmp, then the five constructed suffixes
    assert_same("exact match", &input(2, 1, &z("ABC"), &z("ABC")));
    for (i, suffix) in ["_v1", "_v2", "_old", "_new", "_tmp"].iter().enumerate() {
        assert_same(
            &format!("exact variation {i} {suffix}"),
            &input(2, 1, &z(&format!("ABC{suffix}")), &z("ABC")),
        );
    }
    assert_same("exact no variation", &input(2, 1, &z("ABC_zzz"), &z("ABC")));
    assert_same("exact empty prefix", &input(2, 1, &z("_v1"), &z("")));
    assert_same("exact empty both", &input(2, 1, &z(""), &z("")));
    assert_same("flags 3 sets bit 0", &input(2, 3, &z("ABC_v1"), &z("ABC")));
    assert_same("flags 0xffffffff", &input(2, 0xffff_ffff, &z("ABC_v1"), &z("ABC")));
}

#[test]
fn compare_prefix_strncpy_strncat_boundary() {
    // `strncpy(expected, prefix, 63)` truncates at 63 bytes and
    // `strncat(expected, variations[i], 63 - strlen(expected))` then has less
    // and less room, so long prefixes truncate the suffix away.
    for n in [58usize, 59, 60, 61, 62, 63, 64, 100] {
        let prefix = "A".repeat(n);
        assert_same(
            &format!("prefix len {n} exact"),
            &input(2, 1, &z(&prefix), &z(&prefix)),
        );
        assert_same(
            &format!("prefix len {n} plus _v1"),
            &input(2, 1, &z(&format!("{prefix}_v1")), &z(&prefix)),
        );
        assert_same(
            &format!("prefix len {n} plus _old"),
            &input(2, 1, &z(&format!("{prefix}_old")), &z(&prefix)),
        );
        // the truncated-to-63 form of the prefix itself
        let truncated: String = prefix.chars().take(63).collect();
        assert_same(
            &format!("prefix len {n} truncated to 63"),
            &input(2, 1, &z(&truncated), &z(&prefix)),
        );
    }
}

#[test]
fn compare_prefix_full_buffers() {
    let full = vec![b'A'; 1024];
    assert_same("compare_prefix full both non exact", &input(2, 0, &full, &full));
    assert_same("compare_prefix full both exact", &input(2, 1, &full, &full));
}

// ---------------------------------------------------------------------------
// operation 3 - find_delimiter
// ---------------------------------------------------------------------------

#[test]
fn find_delimiter_paths() {
    // `len == 0` short circuit
    assert_same("len 0", &input(3, 0, &[], &z(":")));
    assert_same("len 0 empty ref", &input(3, 0, &[], &[]));
    // delimiter taken from reference[0]
    assert_same("colon at 1", &input(3, 0, &z("A:B"), &z(":")));
    assert_same("colon at 0", &input(3, 0, &z(":AB"), &z(":")));
    assert_same("pipe at 2", &input(3, 0, &z("AB|C"), &z("|")));
    assert_same("delimiter absent", &input(3, 0, &z("ABC"), &z("|")));
    // reference[0] is used even when more bytes follow
    assert_same("multi byte reference", &input(3, 0, &z("A#B"), &z("#$%")));
    // the NUL inside the buffer breaks the scan before the delimiter
    assert_same("NUL before delimiter", &input(3, 0, &[65, 0, 58, 0], &z(":")));
    // scanning a NUL as the delimiter itself: the equality test wins over break
    assert_same("delimiter is NUL", &input(3, 0, &[65, 66, 0], &[0]));
    // the two hard-coded special cases
    assert_same("NONE with pipe", &input(3, 0, &z("NONE"), &z("|")));
    assert_same("EMPTY with colon", &input(3, 0, &z("EMPTY"), &z(":")));
    // ...and their near misses
    assert_same("NONE with colon", &input(3, 0, &z("NONE"), &z(":")));
    assert_same("EMPTY with pipe", &input(3, 0, &z("EMPTY"), &z("|")));
    assert_same("NONEX with pipe", &input(3, 0, &z("NONEX"), &z("|")));
    assert_same("EMPTYX with colon", &input(3, 0, &z("EMPTYX"), &z(":")));
    // a pipe inside NONE would be found by the loop first
    assert_same("NONE| with pipe", &input(3, 0, &z("NONE|"), &z("|")));
    // delimiter at the last scanned byte
    assert_same("delimiter at len-1", &input(3, 0, &raw("AB:"), &z(":")));
    // high-bit delimiter
    assert_same("high byte delimiter", &input(3, 0, &[65, 200, 0], &[200, 0]));
    // full buffer, delimiter never present
    let full = vec![b'A'; 1024];
    assert_same("find_delimiter full buffer", &input(3, 0, &full, &z("|")));
    let mut full_delim = vec![b'A'; 1024];
    full_delim[1023] = b'|';
    assert_same("find_delimiter at 1023", &input(3, 0, &full_delim, &z("|")));
}

// ---------------------------------------------------------------------------
// operation 4 - match_pattern, case sensitive (flags bit 1 set)
// ---------------------------------------------------------------------------

#[test]
fn match_pattern_case_sensitive_paths() {
    // exact match
    assert_same("cs exact", &input(4, 2, &z("ABC"), &z("ABC")));
    // the three constructed wildcard forms
    assert_same("cs wildcard both", &input(4, 2, &z("*ABC*"), &z("ABC")));
    assert_same("cs wildcard trailing", &input(4, 2, &z("ABC*"), &z("ABC")));
    assert_same("cs wildcard leading", &input(4, 2, &z("*ABC"), &z("ABC")));
    // containment: 10 + position
    assert_same("cs contains at 0", &input(4, 2, &z("ABCDE"), &z("ABC")));
    assert_same("cs contains at 2", &input(4, 2, &z("XYABCDE"), &z("ABC")));
    assert_same("cs contains at end", &input(4, 2, &z("XYABC"), &z("ABC")));
    assert_same("cs no containment", &input(4, 2, &z("XYZ"), &z("ABC")));
    // case matters here
    assert_same("cs case differs", &input(4, 2, &z("abc"), &z("ABC")));
    // empty pattern: strncmp with n == 0 succeeds immediately
    assert_same("cs empty pattern", &input(4, 2, &z("ABC"), &z("")));
    assert_same("cs empty text and pattern", &input(4, 2, &z(""), &z("")));
    // single byte
    assert_same("cs single byte", &input(4, 2, &z("A"), &z("A")));
    assert_same("cs single byte miss", &input(4, 2, &z("B"), &z("A")));
    // flags bit 1 is what selects this branch
    assert_same("cs flags 3", &input(4, 3, &z("ABCDE"), &z("ABC")));
    assert_same("cs flags 0xffffffff", &input(4, 0xffff_ffff, &z("ABCDE"), &z("ABC")));
}

#[test]
fn match_pattern_case_sensitive_snprintf_truncation() {
    // `snprintf(dst, 64, "*%s*", pattern)` keeps only 63 characters, so a long
    // pattern produces a truncated wildcard string.
    for n in [58usize, 60, 61, 62, 63, 70] {
        let pat = "A".repeat(n);
        let both = format!("*{pat}*");
        let truncated: String = both.chars().take(63).collect();
        assert_same(
            &format!("cs wildcard truncated at pattern len {n}"),
            &input(4, 2, &z(&truncated), &z(&pat)),
        );
        let trailing: String = format!("{pat}*").chars().take(63).collect();
        assert_same(
            &format!("cs trailing wildcard truncated at pattern len {n}"),
            &input(4, 2, &z(&trailing), &z(&pat)),
        );
    }
}

#[test]
fn match_pattern_case_sensitive_pattern_longer_than_text() {
    // `for (size_t i = 0; i <= text_len - pattern_len; i++)` - the subtraction
    // is unsigned, so a pattern longer than the text wraps the bound around and
    // the scan runs off the stack.  The reference program dies from SIGSEGV
    // before printing anything, and so must the translation.
    assert_same("cs pattern longer", &input(4, 2, &z("ABC"), &z("ABCDEFGHIJ")));
    assert_same("cs pattern longer by one", &input(4, 2, &z("AB"), &z("ABC")));
    assert_same("cs empty text long pattern", &input(4, 2, &z(""), &z("ABCDEFGHIJ")));
    let full = vec![1u8; 1024];
    assert_same("cs full buffers", &input(4, 2, &full, &full));
}

// ---------------------------------------------------------------------------
// operation 4 - match_pattern, case insensitive (flags bit 1 clear)
// ---------------------------------------------------------------------------

#[test]
fn match_pattern_case_insensitive_paths() {
    // exact strcmp first
    assert_same("ci exact", &input(4, 0, &z("ABC"), &z("ABC")));
    // differing lengths -> strncmp prefix check -> 5
    assert_same("ci prefix longer text", &input(4, 0, &z("ABCDE"), &z("ABC")));
    assert_same("ci prefix mismatch", &input(4, 0, &z("XYZDE"), &z("ABC")));
    assert_same("ci pattern longer", &input(4, 0, &z("ABC"), &z("ABCDE")));
    // equal lengths -> manual lowercase compare -> 6
    assert_same("ci same length lower", &input(4, 0, &z("abc"), &z("ABC")));
    assert_same("ci same length upper", &input(4, 0, &z("ABC"), &z("abc")));
    assert_same("ci same length mixed", &input(4, 0, &z("aBc"), &z("AbC")));
    assert_same("ci same length differs", &input(4, 0, &z("abd"), &z("ABC")));
    // only A-Z is folded: the +32 rule does not touch digits or punctuation
    assert_same("ci digits", &input(4, 0, &z("123"), &z("123")));
    assert_same("ci at sign vs backtick", &input(4, 0, &z("@"), &z("`")));
    assert_same("ci bracket vs brace", &input(4, 0, &z("["), &z("{")));
    // high-bit bytes must not wrap into ASCII
    assert_same("ci high bytes", &input(4, 0, &[200, 0], &[232, 0]));
    assert_same("ci high bytes equal", &input(4, 0, &[200, 0], &[200, 0]));
    // empty pattern with non-empty text: lengths differ, strncmp(n=0) succeeds
    assert_same("ci empty pattern", &input(4, 0, &z("ABC"), &z("")));
    assert_same("ci empty both", &input(4, 0, &z(""), &z("")));
    // flags bit 0 does not select this branch
    assert_same("ci flags 1", &input(4, 1, &z("abc"), &z("ABC")));
    assert_same("ci flags 0xfffffffd", &input(4, 0xffff_fffd, &z("abc"), &z("ABC")));
    // full buffers
    let full = vec![b'A'; 1024];
    assert_same("ci full buffers", &input(4, 0, &full, &full));
}

// ===========================================================================
// Phase C - buffers the driver leaves unterminated
// ===========================================================================

#[test]
fn unterminated_buffers() {
    // `main` never NUL-terminates, so these inputs make `lib.c` read past
    // `input_len` / `ref_len`.  Only residue bytes whose value is stable from
    // run to run are exercised here; the ones ASLR randomises are listed in
    // `translation/ERRORS.md` as outside the comparable domain.
    assert_same("raw ABC vs raw ABC", &input(0, 0, &raw("ABC"), &raw("ABC")));
    assert_same("raw VALID", &input(0, 0, &raw("VALID"), &raw("zzz")));
    assert_same("raw START", &input(1, 0, &raw("START"), &[]));
    assert_same("raw ADMIN", &input(1, 0, &raw("ADMIN"), &[]));
    assert_same("raw EMPTY with colon", &input(3, 0, &raw("EMPTY"), &raw(":")));
    assert_same("raw input, terminated prefix", &input(2, 0, &raw("ABCDEF"), &z("ABC")));
    assert_same("raw exact", &input(2, 1, &raw("ABC"), &raw("ABC")));
    assert_same("raw 1023 both", &input(0, 0, &vec![b'A'; 1023], &vec![b'A'; 1023]));
}

#[test]
fn full_buffer_overruns_into_the_frame() {
    // Both buffers full and free of NULs: `strlen` leaves the arrays entirely.
    // `ref_buffer` sits directly below `input_buffer`, and past `input_buffer`
    // come `ref_len` and `input_len`, whose upper bytes are always zero.
    let full_a = vec![b'A'; 1024];
    let full_1 = vec![1u8; 1024];
    for op in [0i64, 1, 2, 3, 4] {
        for flags in [0u64, 1, 2, 3] {
            assert_same(
                &format!("full A op {op} flags {flags}"),
                &input(op, flags, &full_a, &full_a),
            );
        }
    }
    assert_same("full A vs empty ref", &input(0, 0, &full_a, &[]));
    assert_same("full 1 vs full 1 op 0", &input(0, 0, &full_1, &full_1));
    assert_same("full 1 vs full 1 op 2", &input(2, 0, &full_1, &full_1));
    // 1023 bytes then a terminator, and 1024 bytes ending in a terminator
    let mut term = vec![b'A'; 1024];
    term[1023] = 0;
    assert_same("1024 with terminator", &input(0, 0, &term, &term));
    assert_same("1023 unterminated", &input(0, 0, &vec![b'A'; 1023], &vec![b'A'; 1023]));
}
