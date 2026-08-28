//! Differential tests: run the original C program and the Rust translation as
//! *subprocesses*, feed them identical stdin, and require byte-identical
//! stdout, byte-identical stderr and an identical exit status (including death
//! by signal).
//!
//! Nothing here links against the Rust code as a library - the binary is driven
//! exactly the way a shell would drive it, because that is how the two programs
//! are compared.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// locating / building the two executables
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // translation/ -> repository root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Build `c_src` with CMake (once per test binary) and return the executable.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
        let build_dir = c_src.join("build");
        std::fs::create_dir_all(&build_dir).expect("cannot create c_src/build");

        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build_dir)
            .output()
            .expect("failed to run `cmake ..` - is cmake installed?");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );

        let build = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .output()
            .expect("failed to run `cmake --build .`");
        assert!(
            build.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );

        let exe = build_dir.join("driver");
        assert!(exe.is_file(), "expected C executable at {}", exe.display());
        exe
    })
}

/// The Rust executable produced by this crate (`[[bin]] name = "driver"`).
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

// ---------------------------------------------------------------------------
// running and comparing
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

fn describe_status(s: &Result<i32, i32>) -> String {
    match s {
        Ok(c) => format!("exited with code {c}"),
        Err(sig) => format!("killed by signal {sig}"),
    }
}

/// Some inputs make both programs die by SIGSEGV. Writing a core dump for each
/// of those costs about half a second, so core dumps are switched off in the
/// children. This is applied identically to the C and the Rust process and
/// changes nothing that is compared: stdout, stderr and the wait status
/// (`killed by signal 11`) are unaffected - only the core file is not written.
#[cfg(unix)]
fn disable_core_dumps(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;

    const RLIMIT_CORE: i32 = 4;
    #[repr(C)]
    struct RLimit {
        rlim_cur: u64,
        rlim_max: u64,
    }
    extern "C" {
        fn setrlimit(resource: i32, rlim: *const RLimit) -> i32;
    }

    unsafe {
        cmd.pre_exec(|| {
            let zero = RLimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            // A failure here is not fatal; it only means cores are still written.
            setrlimit(RLIMIT_CORE, &zero);
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn disable_core_dumps(_cmd: &mut Command) {}

fn exec(bin: &Path, input: &[u8]) -> Run {
    let mut cmd = Command::new(bin);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    disable_core_dumps(&mut cmd);

    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // The child may exit before consuming everything (e.g. a bad `flags`);
        // a broken pipe here is expected and must not fail the test.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("failed to wait for {}: {e}", bin.display()));

    let status = match out.status.code() {
        Some(code) => Ok(code),
        None => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                Err(out.status.signal().expect("no code and no signal"))
            }
            #[cfg(not(unix))]
            {
                panic!("process ended without an exit code");
            }
        }
    };

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

fn render(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    if text.len() <= 600 {
        text.escape_debug().to_string()
    } else {
        let head: String = text.chars().take(300).collect();
        let tail: String = {
            let all: Vec<char> = text.chars().collect();
            all[all.len().saturating_sub(120)..].iter().collect()
        };
        format!(
            "{} ...<{} bytes total>... {}",
            head.escape_debug(),
            bytes.len(),
            tail.escape_debug()
        )
    }
}

/// First differing byte offset of two buffers, if any.
fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    a.iter().zip(b.iter()).position(|(x, y)| x != y).or({
        if a.len() == b.len() {
            None
        } else {
            Some(a.len().min(b.len()))
        }
    })
}

/// The whole point of the suite: stdout, stderr and exit status must all match.
#[track_caller]
fn assert_identical(label: &str, input: &[u8]) {
    let c = exec(c_binary(), input);
    let r = exec(rust_binary(), input);

    let mut problems = Vec::new();

    if c.stdout != r.stdout {
        problems.push(format!(
            "stdout differs (first difference at byte {:?})\n     C: {}\n  rust: {}",
            first_diff(&c.stdout, &r.stdout),
            render(&c.stdout),
            render(&r.stdout)
        ));
    }
    if c.stderr != r.stderr {
        problems.push(format!(
            "stderr differs (first difference at byte {:?})\n     C: {}\n  rust: {}",
            first_diff(&c.stderr, &r.stderr),
            render(&c.stderr),
            render(&r.stderr)
        ));
    }
    if c.status != r.status {
        problems.push(format!(
            "exit status differs\n     C: {}\n  rust: {}",
            describe_status(&c.status),
            describe_status(&r.status)
        ));
    }

    assert!(
        problems.is_empty(),
        "[{label}] mismatch for input {}\n{}",
        render(input),
        problems.join("\n")
    );
}

#[track_caller]
fn check(label: &str, input: &str) {
    assert_identical(label, input.as_bytes());
}

/// Format a whole test vector.
fn vector(flags: u32, param1: i64, param2: i64, values: &[u8]) -> String {
    let mut s = format!("{} {} {} {}", flags, param1, param2, values.len());
    for v in values {
        s.push(' ');
        s.push_str(&v.to_string());
    }
    s.push('\n');
    s
}

// ---------------------------------------------------------------------------
// Phase B/C: the input classes the C program actually branches on
// ---------------------------------------------------------------------------

// --- `scanf("%u", &flags) != 1` -------------------------------------------

#[test]
fn flags_read_failure() {
    check("empty stdin (EOF)", "");
    check("single space", " ");
    check("single newline", "\n");
    check("only whitespace of every kind", " \t\n\u{b}\u{c}\r ");
    check("non numeric", "abc");
    check("lone minus", "-");
    check("lone plus", "+");
    check("double sign", "--1 0 0 0");
    check("sign then letter", "-x 0 0 0");
    check("dot", ".5 0 0 0");
}

// --- `scanf("%d", &param1) != 1` ------------------------------------------

#[test]
fn param1_read_failure() {
    check("flags only", "1");
    check("flags then EOF after newline", "1\n");
    check("flags then junk", "1 x");
    // `%u` stops at 'x', so the leftover "x10" breaks the *next* conversion.
    check("hex literal is not consumed by %u", "0x10 0 0 0");
    check("flags then sign only", "7 -");
}

// --- `scanf("%d", &param2) != 1` ------------------------------------------

#[test]
fn param2_read_failure() {
    check("two fields only", "1 2");
    check("two fields then junk", "1 2 zzz");
    check("two fields then sign", "1 2 +");
}

// --- `scanf("%zu", &length) != 1` -----------------------------------------

#[test]
fn length_read_failure() {
    check("three fields only", "1 2 3");
    check("three fields then junk", "1 2 3 q");
    check("three fields then newline", "1 2 3\n");
}

// --- `length > 256` -------------------------------------------------------

#[test]
fn length_exceeds_maximum() {
    check("257", "0 0 0 257");
    check("258 with data", "0 0 0 258 1 2 3");
    check("1000", "0 0 0 1000");
    check("u32 max", "0 0 0 4294967295");
    check("u64 max", "0 0 0 18446744073709551615");
    // `%zu` on a negative literal wraps like `strtoul` does.
    check("negative one wraps to SIZE_MAX", "0 0 0 -1");
    check("negative 256 wraps", "0 0 0 -256");
    // Saturation at ULONG_MAX for absurdly long digit strings.
    check("overlong digits saturate", "0 0 0 99999999999999999999999999999999");
}

#[test]
fn length_at_and_below_maximum_is_accepted() {
    check("exactly 256 is allowed", &vector(0, 0, 0, &vec![7u8; 256]));
    check("255", &vector(0, 0, 0, &vec![9u8; 255]));
    check("minus zero is zero", "0 0 0 -0");
}

// --- `scanf("%u", &byte) != 1` (per index) --------------------------------

#[test]
fn byte_read_failure() {
    check("no bytes at all -> byte 0", "0 0 0 5");
    check("truncated after 3 -> byte 3", "0 0 0 5 1 2 3");
    check("junk at byte 0", "0 0 0 1 x");
    check("junk at byte 2", "0 0 0 4 1 2 x 4");
    check("truncated at last byte", "0 0 0 256 ".to_string().as_str());
    let mut short = String::from("0 0 0 256");
    for i in 0..255 {
        short.push(' ');
        short.push_str(&(i % 251).to_string());
    }
    check("255 of 256 bytes -> byte 255", &short);
}

// --- `length == 0` (process_buffer early return) --------------------------

#[test]
fn zero_length() {
    check("no flags", "0 0 0 0");
    check("all flags set", "31 5 1 0");
    check("all bits set", "4294967295 -7 3 0");
    check("trailing data ignored", "0 0 0 0 1 2 3 4");
}

// --- single element -------------------------------------------------------

#[test]
fn single_element() {
    for flags in 0u32..32 {
        check(
            "single element, every flag combination",
            &vector(flags, 3, 1, &[42]),
        );
        check("single element, param1 = 1", &vector(flags, 1, 0, &[42]));
        check("single element, param1 = 0", &vector(flags, 0, 0, &[42]));
    }
}

// --- rotate (flags & 0x01) ------------------------------------------------

#[test]
fn rotate_branches() {
    let data: Vec<u8> = (1..=8).collect();
    // offset == 0 -> rotate_buffer is never called
    check("offset 0", &vector(1, 0, 0, &data));
    check("offset == length", &vector(1, 8, 0, &data));
    check("offset == -length", &vector(1, -8, 0, &data));
    check("offset multiple of length", &vector(1, 24, 0, &data));
    // small-offset branch: offset < len / 2
    check("offset 1", &vector(1, 1, 0, &data));
    check("offset 3", &vector(1, 3, 0, &data));
    // exactly len/2 -> large-offset branch
    check("offset 4 (== len/2)", &vector(1, 4, 0, &data));
    // large-offset branch
    check("offset 5", &vector(1, 5, 0, &data));
    check("offset 7", &vector(1, 7, 0, &data));
    // negative offsets normalise by += len
    check("offset -1", &vector(1, -1, 0, &data));
    check("offset -3", &vector(1, -3, 0, &data));
    check("offset -5", &vector(1, -5, 0, &data));
    // two elements: len/2 == 1 so offset 1 takes the large branch
    check("two elements offset 1", &vector(1, 1, 0, &[10, 20]));
    check("two elements offset -1", &vector(1, -1, 0, &[10, 20]));
    // three elements, both branches
    check("three elements offset 1", &vector(1, 1, 0, &[10, 20, 30]));
    check("three elements offset 2", &vector(1, 2, 0, &[10, 20, 30]));
    // length 1: param1 % 1 == 0, so rotation is skipped entirely
    check("length 1", &vector(1, 5, 0, &[99]));
    // extreme parameters
    check("INT_MAX offset", &vector(1, 2147483647, 0, &data));
    check("INT_MIN offset", &vector(1, -2147483648, 0, &data));
    check("full buffer rotate", &vector(1, 100, 0, &(0..=255).collect::<Vec<u8>>()));
    check(
        "full buffer rotate small offset",
        &vector(1, 3, 0, &(0..=255).collect::<Vec<u8>>()),
    );
}

// --- compact runs (flags & 0x02) ------------------------------------------

#[test]
fn compact_runs_branches() {
    // threshold = 3 (default, because param1 <= 0 or > 255)
    check(
        "default threshold, mixed runs",
        &vector(2, 0, 0, &[1, 1, 1, 2, 2, 3, 3, 3, 3, 4]),
    );
    check(
        "default threshold via param1 = 256",
        &vector(2, 256, 0, &[1, 1, 1, 2, 2, 3, 3, 3, 3, 4]),
    );
    check(
        "default threshold via negative param1",
        &vector(2, -5, 0, &[5, 5, 5, 5, 6]),
    );
    // no run reaches the threshold -> pure "keep as-is" path
    check("all runs shorter than threshold", &vector(2, 4, 0, &[1, 2, 3, 4, 5]));
    // threshold 255 / 256-boundary
    check("threshold 255 unreachable", &vector(2, 255, 0, &vec![8u8; 100]));
    check(
        "threshold 255 reached exactly",
        &vector(2, 255, 0, &vec![8u8; 255]),
    );
    // run longer than 255 -> the `run_len > 255` cap
    check("run of 256 caps at 255", &vector(2, 1, 0, &vec![7u8; 256]));
    check(
        "run of 256 caps at 255, default threshold",
        &vector(2, 0, 0, &vec![7u8; 256]),
    );
    // threshold 2: runs of exactly 2 compact to the same size
    check("threshold 2", &vector(2, 2, 0, &[1, 1, 2, 2, 3, 4, 4, 4]));
    // last run reaches the end -> `read + run_len < len` is false
    check("trailing run", &vector(2, 3, 0, &[1, 2, 3, 3, 3, 3]));
    check("single run covering all", &vector(2, 3, 0, &vec![4u8; 10]));
    // threshold 1: every run compacts, the length *grows*
    check("threshold 1, tiny", &vector(2, 1, 0, &[1]));
    check("threshold 1, two distinct", &vector(2, 1, 0, &[1, 2]));
    check(
        "threshold 1, 128 distinct (grows to exactly 256)",
        &vector(2, 1, 0, &(0..128).collect::<Vec<u8>>()),
    );
}

// --- remove duplicates (flags & 0x04) -------------------------------------

#[test]
fn remove_duplicates_branches() {
    let data = [1u8, 2, 1, 3, 2, 4, 1, 5];
    check("preserve order (param2 != 0)", &vector(4, 0, 1, &data));
    check("preserve order (param2 negative)", &vector(4, 0, -1, &data));
    check("do not preserve order (param2 == 0)", &vector(4, 0, 0, &data));
    // len <= 1 early return
    check("one element, preserve", &vector(4, 0, 1, &[7]));
    check("one element, no preserve", &vector(4, 0, 0, &[7]));
    // no duplicates at all -> `write == i` throughout
    check("all distinct, preserve", &vector(4, 0, 1, &[1, 2, 3, 4]));
    check("all distinct, no preserve", &vector(4, 0, 0, &[1, 2, 3, 4]));
    // everything identical
    check("all identical, preserve", &vector(4, 0, 1, &vec![9u8; 16]));
    check("all identical, no preserve", &vector(4, 0, 0, &vec![9u8; 16]));
    // full 256 distinct values, and 256 values from a tiny alphabet
    check(
        "256 distinct, no preserve",
        &vector(4, 0, 0, &(0..=255).collect::<Vec<u8>>()),
    );
    check(
        "256 values from 3 symbols, preserve",
        &vector(4, 0, 1, &(0..256).map(|i| (i % 3) as u8).collect::<Vec<u8>>()),
    );
    check(
        "256 values from 3 symbols, no preserve",
        &vector(4, 0, 0, &(0..256).map(|i| (i % 3) as u8).collect::<Vec<u8>>()),
    );
}

// --- interleave halves (flags & 0x08) -------------------------------------

#[test]
fn interleave_branches() {
    // new_len >= 2 guard
    check("length 1 -> guard blocks interleave", &vector(8, 0, 0, &[5]));
    check("length 2 (even)", &vector(8, 0, 0, &[1, 2]));
    check("length 3 (odd)", &vector(8, 0, 0, &[1, 2, 3]));
    check("length 8 (even)", &vector(8, 0, 0, &(1..=8).collect::<Vec<u8>>()));
    check("length 7 (odd)", &vector(8, 0, 0, &(1..=7).collect::<Vec<u8>>()));
    check(
        "length 255 (odd, max-ish)",
        &vector(8, 0, 0, &(0..255).collect::<Vec<u8>>()),
    );
    check(
        "length 256 (even, max)",
        &vector(8, 0, 0, &(0..=255).collect::<Vec<u8>>()),
    );
    // guard blocked by a compaction that leaves fewer than 2 elements
    check("compact to 1 then interleave", &vector(10, 300, 0, &vec![3u8; 4]));
}

// --- reverse segments (flags & 0x10) --------------------------------------

#[test]
fn reverse_segments_branches() {
    let data: Vec<u8> = (1..=10).collect();
    // new_len >= 4 guard
    check("length 3 -> guard blocks reverse", &vector(16, 2, 0, &[1, 2, 3]));
    check("length 4 boundary", &vector(16, 2, 0, &[1, 2, 3, 4]));
    // seg_size == 1 -> `seg_size <= 1` early return
    check("seg_size 1", &vector(16, 1, 0, &data));
    // seg_size default 4 when param1 <= 0
    check("seg_size default (param1 0)", &vector(16, 0, 0, &data));
    check("seg_size default (param1 negative)", &vector(16, -9, 0, &data));
    // exact multiple -> remainder == 0
    check("seg_size 5 divides 10", &vector(16, 5, 0, &data));
    check("seg_size 2 divides 10", &vector(16, 2, 0, &data));
    // remainder > 1 and remainder == 1
    check("seg_size 3 -> remainder 1", &vector(16, 3, 0, &data));
    check("seg_size 4 -> remainder 2", &vector(16, 4, 0, &data));
    check("seg_size 6 -> remainder 4", &vector(16, 6, 0, &data));
    // seg_size == new_len
    check("seg_size == length", &vector(16, 10, 0, &data));
    // seg_size > new_len -> the `seg_size <= new_len` guard skips everything
    check("seg_size 11 > length", &vector(16, 11, 0, &data));
    check("seg_size huge", &vector(16, 2147483647, 0, &data));
    check(
        "seg_size 7 over full buffer",
        &vector(16, 7, 0, &(0..=255).collect::<Vec<u8>>()),
    );
}

// --- flag combinations ----------------------------------------------------

#[test]
fn every_flag_combination() {
    let data: Vec<u8> = (0..40).map(|i| ((i * 7) % 11) as u8).collect();
    let runs: Vec<u8> = vec![1, 1, 1, 2, 2, 3, 3, 3, 3, 4, 5, 5, 6, 6, 6, 7];
    for flags in 0u32..32 {
        check("flags sweep, param1 3", &vector(flags, 3, 1, &data));
        check("flags sweep on runs, param1 3", &vector(flags, 3, 0, &runs));
        check("flags sweep on runs, param1 2", &vector(flags, 2, 1, &runs));
    }
}

#[test]
fn high_flag_bits_are_ignored() {
    let data: Vec<u8> = (1..=9).collect();
    // Only bits 0..4 are examined; everything above must be inert.
    check("only high bits set", &vector(0xFFFF_FFE0, 3, 1, &data));
    check("high bits plus bit 0", &vector(0xFFFF_FFE1, 3, 1, &data));
    check("high bits plus all low bits", &vector(0xFFFF_FFFF, 3, 1, &data));
    check("bit 5 only", &vector(32, 3, 1, &data));
    // `%u` accepts a negative literal and wraps it.
    check("negative flags wrap", "-1 3 1 9 1 2 3 4 5 6 7 8 9");
    check("flags above u32 truncate", "4294967297 3 1 9 1 2 3 4 5 6 7 8 9");
}

// --- numeric conversion edge cases ---------------------------------------

#[test]
fn numeric_conversion_edges() {
    check("param1 INT_MAX", "3 2147483647 0 6 1 1 1 2 2 2");
    check("param1 INT_MAX+1 saturates", "3 2147483648 0 6 1 1 1 2 2 2");
    check("param1 INT_MIN", "3 -2147483648 0 6 1 1 1 2 2 2");
    check("param1 INT_MIN-1 saturates", "3 -2147483649 0 6 1 1 1 2 2 2");
    check("param1 LONG_MAX", "3 9223372036854775807 0 6 1 1 1 2 2 2");
    check(
        "param1 beyond LONG_MAX saturates",
        "3 99999999999999999999999 0 6 1 1 1 2 2 2",
    );
    check(
        "param1 beyond LONG_MIN saturates",
        "3 -99999999999999999999999 0 6 1 1 1 2 2 2",
    );
    check("param2 huge", "4 0 9223372036854775807 5 1 1 2 2 3");
    check("explicit plus signs", "+2 +3 +0 +6 +1 +1 +1 +2 +2 +2");
    check("leading zeros", "0002 0003 0000 0006 001 001 001 002 002 002");
    check("byte values wrap mod 256", "0 0 0 4 256 257 511 4294967296");
    check("byte value negative wraps", "0 0 0 4 -1 -2 -256 -257");
    check("byte value huge saturates", "0 0 0 2 99999999999999999999999 5");
}

// --- whitespace handling: scanf crosses newlines -------------------------

#[test]
fn whitespace_and_layout() {
    check("newline separated", "2\n3\n0\n6\n1\n1\n1\n2\n2\n2\n");
    check("tab separated", "2\t3\t0\t6\t1\t1\t1\t2\t2\t2");
    check("crlf separated", "2\r\n3\r\n0\r\n6\r\n1\r\n1\r\n1\r\n2\r\n2\r\n2\r\n");
    check("vertical tab and form feed", "2\u{b}3\u{c}0 6 1 1 1 2 2 2");
    check("leading whitespace", "\n\n   \t 2 3 0 6 1 1 1 2 2 2");
    check("lots of internal whitespace", "2    3\n\n\n0 \t 6 1  1   1 2 2 2");
    check("no trailing newline", "2 3 0 6 1 1 1 2 2 2");
    check("trailing whitespace", "2 3 0 6 1 1 1 2 2 2   \n\n");
    check("trailing garbage is never read", "2 3 0 6 1 1 1 2 2 2 garbage");
    check("garbage glued to last byte", "2 3 0 6 1 1 1 2 2 2xyz");
    check("NUL byte after the input", "2 3 0 6 1 1 1 2 2 2\u{0}999");
}

// --- guards re-evaluated after the length changed ------------------------

#[test]
fn length_guards_after_the_length_changed() {
    // Dedup collapses everything to one element, so both the `new_len >= 2`
    // and the `new_len >= 4` guards block the later stages.
    check(
        "dedup to 1 blocks interleave and reverse",
        &vector(0x1C, 3, 0, &vec![5u8; 32]),
    );
    check(
        "dedup to 1 blocks interleave and reverse (preserve)",
        &vector(0x1C, 3, 1, &vec![5u8; 32]),
    );
    // Compaction collapses 32 equal bytes to 2, so interleave runs but reverse
    // is blocked by `new_len >= 4`.
    check(
        "compact to 2: interleave runs, reverse blocked",
        &vector(0x1A, 3, 0, &vec![5u8; 32]),
    );
    // Compacts to 3 -> reverse still blocked, interleave sees an odd length.
    check(
        "compact to 3: odd interleave, reverse blocked",
        &vector(0x1A, 3, 0, &[5, 5, 5, 5, 5, 5, 9]),
    );
    // Compacts to 4 -> reverse runs, but `seg_size <= new_len` fails for 5.
    check(
        "compact to 4 with seg_size 5 -> reverse skipped",
        &vector(0x12, 5, 0, &vec![5u8; 5]),
    );
    // Dedup shrinks below seg_size.
    check(
        "dedup below seg_size -> reverse skipped",
        &vector(0x14, 8, 1, &(0..40).map(|i| (i % 5) as u8).collect::<Vec<u8>>()),
    );
    // All three length-changing/consuming stages at once on run-rich data.
    check(
        "compact then dedup then interleave then reverse",
        &vector(0x1E, 3, 1, &(0..64).map(|i| (i / 3 % 4) as u8).collect::<Vec<u8>>()),
    );
}

// --- more malformed byte streams -----------------------------------------

#[test]
fn malformed_byte_tokens() {
    // `%u` consumes the leading "0" and stops at 'x'; the *next* conversion
    // then fails on 'x'.
    check("hex byte splits the token", "0 0 0 3 1 0x1F 3");
    check("embedded NUL stops the scan", "2 3 0 6 1 1\u{0} 1 2 2 2");
    check("float byte splits the token", "0 0 0 3 1 2.5 3");
    check("comma separated fails", "0 0 0 3 1,2,3");
    check("sign only as a byte", "0 0 0 3 1 - 3");
    check("letter as first byte", "0 0 0 3 a 2 3");
    check("length with leading zeros", "0 0 0 000000000003 1 2 3");
    check("length as +3", "0 0 0 +3 1 2 3");
}

// ---------------------------------------------------------------------------
// The out-of-bounds region: `compact_runs` can grow past `uint8_t buffer[256]`
// ---------------------------------------------------------------------------

/// `2 * runs` stays <= 280, so nothing observable is disturbed yet.
#[test]
fn overflow_below_the_aliased_locals() {
    for len in [129usize, 130, 132, 135, 138, 139, 140] {
        let data: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        check("grows past 256 but below 280", &vector(2, 1, 0, &data));
    }
}

/// The compacted data reaches the locals that live right after the array, so
/// the printed bytes stop being the compacted data.
#[test]
fn overflow_into_aliased_locals() {
    for len in 141usize..=156 {
        let data: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        check("overwrites new_length / loop counter", &vector(2, 1, 0, &data));
    }
    // A different alphabet, so the aliased bytes cannot match by accident.
    for len in [141usize, 145, 150, 156] {
        let data: Vec<u8> = (0..len).map(|i| if i % 2 == 0 { 200 } else { 100 }).collect();
        check("aliased locals, two symbols", &vector(2, 1, 0, &data));
    }
}

/// Far enough and the return address is clobbered: the C dies on `ret`.
#[test]
fn overflow_clobbers_the_return_address() {
    for len in [157usize, 158, 160, 175, 200, 231, 255, 256] {
        let data: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        check("return address clobbered", &vector(2, 1, 0, &data));
    }
    let alternating: Vec<u8> = (0..256).map(|i| (i % 2) as u8).collect();
    check("alternating 0/1 over 256", &vector(2, 1, 0, &alternating));
    check(
        "alternating plus every other flag",
        &vector(0x1F, 1, 1, &alternating),
    );
}

/// The length *transiently* exceeds the frame even though the final length is
/// small, because a long run at the end shrinks it again.
#[test]
fn transient_overflow_still_clobbers_the_return_address() {
    for (singles, run) in [(200usize, 56usize), (160, 96), (130, 126), (100, 156)] {
        let mut data: Vec<u8> = (0..singles).map(|i| ((i % 250) + 1) as u8).collect();
        data.extend(std::iter::repeat(0u8).take(run));
        check("transient overflow", &vector(2, 1, 0, &data));
    }
}

/// The same growth combined with every other operation.
#[test]
fn overflow_combined_with_other_flags() {
    let data: Vec<u8> = (0..150).map(|i| (i % 251) as u8).collect();
    for flags in 0u32..32 {
        check("growth plus flag sweep", &vector(flags | 2, 1, 0, &data));
        check("growth plus flag sweep, preserve", &vector(flags | 2, 1, 1, &data));
    }
}

// ---------------------------------------------------------------------------
// Broad deterministic sweep
// ---------------------------------------------------------------------------

/// Small deterministic PRNG so the sweep is reproducible without dependencies.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[test]
fn deterministic_random_sweep() {
    let mut rng = Lcg(0x5EED_1234_ABCD_0001);
    let param1_pool: [i64; 14] = [0, 1, 2, 3, 4, 5, 7, 8, 255, 256, -1, -3, -256, 2147483647];
    let param2_pool: [i64; 4] = [0, 1, -1, 5];

    for _ in 0..320 {
        let flags = (rng.below(32) as u32) | if rng.below(4) == 0 { 0xFFFF_FFE0 } else { 0 };
        let param1 = param1_pool[rng.below(param1_pool.len() as u64) as usize];
        let param2 = param2_pool[rng.below(param2_pool.len() as u64) as usize];
        let len = match rng.below(6) {
            0 => rng.below(6),
            1 => 255 + rng.below(2),
            2 => 120 + rng.below(140),
            _ => rng.below(257),
        } as usize;
        let alphabet = [2u64, 3, 5, 17, 256][rng.below(5) as usize];
        let data: Vec<u8> = (0..len).map(|_| rng.below(alphabet) as u8).collect();
        check("deterministic sweep", &vector(flags, param1, param2, &data));
    }
}

/*
 * The three sweeps below deliberately live in separate `#[test]` functions so
 * the test harness runs them concurrently: every crashing input costs ~0.3s of
 * wall time because the kernel hands SIGSEGV terminations to a core-dump
 * helper, and these sweeps are mostly crashing inputs.
 */

/// Exhaustive length sweep with `threshold == 1`, straddling every boundary:
/// no overflow, overflow into padding, overflow onto the aliased locals, and
/// overflow onto the return address.
#[test]
fn overflow_length_sweep() {
    for len in 100usize..=256 {
        let data: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        check("length sweep, threshold 1", &vector(2, 1, 0, &data));
    }
}

/// Singles followed by one long run: the length grows past the frame and then
/// shrinks again, so the crash is driven by the transient maximum.
#[test]
fn overflow_transient_boundary_sweep() {
    for singles in (60usize..=200).step_by(4) {
        let run = 256 - singles;
        let mut data: Vec<u8> = (0..singles).map(|i| ((i % 250) + 1) as u8).collect();
        data.extend(std::iter::repeat(0u8).take(run));
        check("transient overflow boundary", &vector(2, 1, 0, &data));
    }
}

/// Randomised sweep concentrated on the `threshold == 1` growth case, which is
/// the only way to write past `buffer[256]`.
#[test]
fn overflow_random_sweep() {
    let mut rng = Lcg(0xC0FF_EE00_1234_5678);
    for _ in 0..160 {
        // bit 1 always set (compact runs) and param1 == 1 (threshold 1)
        let flags = (rng.below(32) as u32) | 0x02;
        let param2 = [0i64, 1, -1][rng.below(3) as usize];
        let len = (120 + rng.below(137)) as usize;
        let alphabet = [2u64, 3, 5, 250, 256][rng.below(5) as usize];
        let data: Vec<u8> = (0..len).map(|_| rng.below(alphabet) as u8).collect();
        check("overflow sweep", &vector(flags, 1, param2, &data));
    }
}

/// Every flag combination against a handful of shapes at the maximum length.
#[test]
fn maximum_length_sweep() {
    let shapes: Vec<Vec<u8>> = vec![
        (0..=255u8).collect(),                                   // all distinct
        vec![0u8; 256],                                          // one huge run
        (0..256).map(|i| (i % 2) as u8).collect(),               // 256 runs
        (0..256).map(|i| (i / 4) as u8).collect(),               // runs of 4
        (0..256).map(|i| (i / 3 % 5) as u8).collect(),           // runs of 3
    ];
    for flags in 0u32..32 {
        for shape in &shapes {
            check("max length, flag sweep, param1 3", &vector(flags, 3, 1, shape));
            check("max length, flag sweep, param1 4", &vector(flags, 4, 0, shape));
        }
    }
}
