//! Differential tests: run the original C binary and the Rust binary as
//! subprocesses on identical stdin, and require byte-identical stdout,
//! byte-identical stderr and an identical exit status.
//!
//! The Rust code is never called as a library here; only the built executable
//! is driven, exactly the way a shell would drive it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The Rust executable under test, as built by cargo for this integration test.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Repository root (parent of the `translation/` crate).
fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

/// Path to the C executable, building it with cmake if it is not there yet.
fn c_bin() -> PathBuf {
    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");
    if bin.exists() {
        return bin;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");
    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("run cmake (is cmake installed?)");
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
        .expect("run cmake --build");
    assert!(
        compile.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );
    assert!(bin.exists(), "C binary missing after build: {}", bin.display());
    bin
}

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` if killed by a signal.
    code: Option<i32>,
    /// `Some(sig)` if the process was killed by a signal (Unix only).
    signal: Option<i32>,
}

#[cfg(unix)]
fn signal_of(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn signal_of(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

/// Run `program` with `args`, feeding `stdin_bytes` on stdin.
fn run(program: &Path, args: &[&str], stdin_bytes: &[u8]) -> Run {
    run_chunked(program, args, &[stdin_bytes], false)
}

/// Run `program`, writing stdin in the given chunks, optionally pausing between
/// them so the child sees several short reads instead of one big one.
///
/// stdin is written from a helper thread so that inputs larger than a pipe
/// buffer cannot deadlock against the child's own output.
fn run_chunked(program: &Path, args: &[&str], chunks: &[&[u8]], pause: bool) -> Run {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    let mut sink = child.stdin.take().expect("piped stdin");
    let payload: Vec<Vec<u8>> = chunks.iter().map(|c| c.to_vec()).collect();
    let writer = std::thread::spawn(move || {
        for chunk in payload {
            // A broken pipe is fine: the program is free to stop reading early.
            if sink.write_all(&chunk).is_err() {
                break;
            }
            let _ = sink.flush();
            if pause {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        drop(sink);
    });

    let out = child.wait_with_output().expect("wait for child");
    writer.join().expect("stdin writer thread");

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: signal_of(&out.status),
    }
}

/// Run `program`, letting the caller configure the child's stdio itself.
/// Defaults: stdin from /dev/null, stdout and stderr captured.
fn run_configured(program: &Path, configure: &dyn Fn(&mut Command)) -> Run {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure(&mut cmd);
    let child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));
    let out = child.wait_with_output().expect("wait for child");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: signal_of(&out.status),
    }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn compare(label: &str, c: &Run, r: &Run) {
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{label}] stdout differs\n  C   : {:?}\n  Rust: {:?}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{label}] stderr differs\n  C   : {:?}\n  Rust: {:?}",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        c.code, r.code,
        "[{label}] exit code differs: C {:?} vs Rust {:?}",
        c.code, r.code
    );
    assert_eq!(
        c.signal, r.signal,
        "[{label}] terminating signal differs: C {:?} vs Rust {:?}",
        c.signal, r.signal
    );
}

/// Compare C and Rust on one input: stdout, stderr and exit status.
fn assert_same_with_args(label: &str, args: &[&str], input: &[u8]) {
    let c = run(&c_bin(), args, input);
    let r = run(&rust_bin(), args, input);
    compare(label, &c, &r);
}

/// Parse the program's output, `"A: <n>\nx: <n>\n"`, into its two counts.
fn counts(out: &[u8]) -> Option<(i64, i64)> {
    let text = std::str::from_utf8(out).ok()?;
    let mut lines = text.lines();
    let a = lines.next()?.strip_prefix("A: ")?.parse().ok()?;
    let x = lines.next()?.strip_prefix("x: ")?.parse().ok()?;
    if lines.next().is_some() || !text.ends_with('\n') {
        return None;
    }
    Some((a, x))
}

/// Reference C behavior for an input that fills the 1000-byte buffer completely.
///
/// Such an input leaves the C buffer without a NUL terminator, so `strchr`
/// reads past its end (undefined behavior, see ERRORS.md). Whatever the stack
/// holds there is usually 0, in which case the string ends at the buffer edge,
/// but a few percent of runs find a stray byte there and report a higher count.
/// The C is therefore not a deterministic function of its input for this class,
/// so the reference is the output it produces in the majority of runs.
///
/// Deviating runs are still checked: they must have the same shape and counts
/// no lower than the reference, which is the only thing an out-of-bounds read
/// past the end of the input can do. Anything else fails the test.
fn c_reference(label: &str, runs: usize, mut once: impl FnMut() -> Run) -> Run {
    let samples: Vec<Run> = (0..runs).map(|_| once()).collect();

    let mut tally: Vec<(Vec<u8>, usize)> = Vec::new();
    for s in &samples {
        match tally.iter_mut().find(|(out, _)| *out == s.stdout) {
            Some((_, n)) => *n += 1,
            None => tally.push((s.stdout.clone(), 1)),
        }
    }
    tally.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    let modal_stdout = tally[0].0.clone();
    let reference = samples
        .iter()
        .find(|s| s.stdout == modal_stdout)
        .expect("modal sample exists");

    let (ref_a, ref_x) = counts(&modal_stdout).unwrap_or_else(|| {
        panic!(
            "[{label}] C stdout is not in the expected format: {:?}",
            show(&modal_stdout)
        )
    });

    for s in &samples {
        assert_eq!(
            s.stderr, reference.stderr,
            "[{label}] C stderr varies between runs"
        );
        assert_eq!(
            (s.code, s.signal),
            (reference.code, reference.signal),
            "[{label}] C exit status varies between runs"
        );
        if s.stdout == modal_stdout {
            continue;
        }
        let (a, x) = counts(&s.stdout).unwrap_or_else(|| {
            panic!(
                "[{label}] C stdout is not in the expected format: {:?}",
                show(&s.stdout)
            )
        });
        assert!(
            a >= ref_a && x >= ref_x,
            "[{label}] a C run reported fewer matches than the reference \
             ({a},{x}) < ({ref_a},{ref_x}); that is not explained by the \
             out-of-bounds read past the full buffer"
        );
        eprintln!(
            "[{label}] note: this C run read past the unterminated buffer: \
             {:?} instead of {:?} (see ERRORS.md)",
            show(&s.stdout),
            show(&modal_stdout)
        );
    }

    Run {
        stdout: modal_stdout,
        stderr: reference.stderr.clone(),
        code: reference.code,
        signal: reference.signal,
    }
}

const FULL_BUFFER_SAMPLES: usize = 9;

/// Differential check for an input that leaves the C buffer unterminated.
fn assert_same_full_buffer(label: &str, input: &[u8]) {
    let c = c_reference(label, FULL_BUFFER_SAMPLES, || run(&c_bin(), &[], input));
    let r = run(&rust_bin(), &[], input);
    compare(label, &c, &r);
}

fn assert_same(label: &str, input: &[u8]) {
    assert_same_with_args(label, &[], input);
}

// ---------------------------------------------------------------------------
// Phase B: the input classes the C program branches on.
//
// The C reads at most 1000 raw bytes with fread into a zero-initialized
// buffer, then counts 'A' and 'x' in the resulting C string. The branch points
// are therefore: how much input arrives (none / some / buffer-full / more than
// the buffer), whether a NUL byte truncates the string, and whether the
// counted bytes are present, absent or only near-misses in case.
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    assert_same("empty", b"");
}

#[test]
fn single_a() {
    assert_same("single 'A'", b"A");
}

#[test]
fn single_x() {
    assert_same("single 'x'", b"x");
}

#[test]
fn single_unrelated_byte() {
    assert_same("single 'q'", b"q");
}

#[test]
fn only_a_no_x() {
    assert_same("only A", b"AAAAA");
}

#[test]
fn only_x_no_a() {
    assert_same("only x", b"xxxxx");
}

#[test]
fn both_interleaved() {
    assert_same("interleaved", b"AxAxAxxxAA");
}

#[test]
fn adjacent_matches() {
    // Exercises the `s++` step after a hit: consecutive matches must all count.
    assert_same("adjacent AA / xx", b"AAAAxxxxAAxx");
}

#[test]
fn match_at_first_and_last_byte() {
    assert_same("edges", b"Aqqqqx");
}

#[test]
fn wrong_case_only() {
    // 'a' and 'X' must not be counted: strchr is case sensitive.
    assert_same("wrong case", b"aaaXXXaX");
}

#[test]
fn newlines_are_not_special() {
    // fread does not stop at a newline, unlike fgets/scanf.
    assert_same("newlines", b"A\nx\nAA\n\n\nxx\n");
}

#[test]
fn crlf_input() {
    assert_same("crlf", b"A\r\nx\r\nAx\r\n");
}

#[test]
fn whitespace_only() {
    assert_same("whitespace", b"   \t\n  \n");
}

#[test]
fn trailing_newline_only() {
    assert_same("newline only", b"\n");
}

// --- NUL handling: the buffer is zero-initialized, so the first NUL byte ends
// --- the string that foo() walks; everything after it is invisible.

#[test]
fn nul_first_byte_hides_everything() {
    assert_same("leading NUL", b"\0AAAAxxxx");
}

#[test]
fn nul_in_the_middle_truncates() {
    assert_same("embedded NUL", b"AAx\0AAAAxxxx");
}

#[test]
fn nul_last_byte() {
    assert_same("trailing NUL", b"AAxx\0");
}

#[test]
fn many_nuls() {
    assert_same("many NULs", b"Ax\0\0\0Ax\0Ax");
}

// --- Non-ASCII / signedness: bytes above 0x7F must not be mistaken for the
// --- targets and must not trip up Rust's UTF-8 expectations.

#[test]
fn high_bytes() {
    assert_same("high bytes", &[0xFF, 0xFE, 0x80, b'A', b'x', b'A', 0xC3, 0x28]);
}

#[test]
fn all_byte_values_except_nul() {
    // Every byte 0x01..=0xFF once, so exactly one 'A' and one 'x'.
    let input: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    assert_same("bytes 0x01..0xFF", &input);
}

#[test]
fn utf8_text() {
    assert_same("utf8", "héllo Ax — Ünïcøde x".as_bytes());
}

// --- Length boundaries around the 1000-byte fread limit.
//
// Inputs whose first 1000 bytes contain no NUL fill the C buffer completely and
// leave it unterminated, so the C reads past it. Those go through
// `assert_same_full_buffer`, which establishes the C's majority behavior first;
// see the comment on `c_reference` and ERRORS.md. Everything at 999 bytes or
// below, or with a NUL inside the first 1000 bytes, is compared strictly.

#[test]
fn length_999_full_of_a() {
    // Largest input that still leaves the zero-initialized terminator intact.
    let input = vec![b'A'; 999];
    assert_same("999 x 'A'", &input);
}

#[test]
fn length_999_mixed() {
    let input: Vec<u8> = (0..999)
        .map(|i| match i % 3 {
            0 => b'A',
            1 => b'x',
            _ => b'.',
        })
        .collect();
    assert_same("999 mixed", &input);
}

#[test]
fn length_1000_fills_the_buffer() {
    // fread fills the whole buffer, consuming the zero terminator slot.
    let input = vec![b'A'; 1000];
    assert_same_full_buffer("1000 x 'A'", &input);
}

#[test]
fn length_1000_mixed() {
    let mut input = vec![b'A'; 600];
    input.extend(std::iter::repeat(b'x').take(400));
    assert_same_full_buffer("1000 mixed", &input);
}

#[test]
fn length_1000_ending_in_nul_is_terminated() {
    // The 1000th byte is a NUL, so the buffer is still a well-formed C string
    // and the comparison can be strict.
    let mut input = vec![b'A'; 600];
    input.extend(std::iter::repeat(b'x').take(399));
    input.push(0);
    assert_same("1000 bytes ending in NUL", &input);
}

#[test]
fn length_1001_drops_the_last_byte() {
    let mut input = vec![b'A'; 1000];
    input.push(b'x');
    assert_same_full_buffer("1001 bytes", &input);
}

#[test]
fn longer_than_buffer_is_truncated() {
    // Only the first 1000 bytes may be counted: 600 'A' then 400 'x'.
    let mut input = vec![b'A'; 600];
    input.extend(std::iter::repeat(b'x').take(900));
    assert_same_full_buffer("1500 bytes", &input);
}

#[test]
fn much_longer_than_buffer() {
    let input: Vec<u8> = (0..100_000)
        .map(|i| if i % 2 == 0 { b'A' } else { b'x' })
        .collect();
    assert_same_full_buffer("100000 bytes", &input);
}

#[test]
fn matches_only_beyond_the_buffer_are_invisible() {
    let mut input = vec![b'.'; 1000];
    input.extend_from_slice(b"AAAAxxxx");
    assert_same_full_buffer("matches past 1000", &input);
}

// --- Arguments are ignored by C's `int main()`.

#[test]
fn extra_arguments_are_ignored() {
    assert_same_with_args("args", &["ignored", "--also", "-1"], b"AxA");
}

// --- A sweep, so no single off-by-one in the counting or the buffer edge can
// --- slip through unnoticed.

#[test]
fn length_sweep_around_the_boundary() {
    for n in [0usize, 1, 2, 3, 997, 998, 999, 1000, 1001, 1002, 1003] {
        let input: Vec<u8> = (0..n)
            .map(|i| match i % 4 {
                0 => b'A',
                1 => b'x',
                2 => b'A',
                _ => b'z',
            })
            .collect();
        let label = format!("sweep n={n}");
        if n >= 1000 {
            assert_same_full_buffer(&label, &input);
        } else {
            assert_same(&label, &input);
        }
    }
}

#[test]
fn nul_position_sweep() {
    for pos in [0usize, 1, 500, 998, 999] {
        let mut input = vec![b'A'; 1000];
        input[pos] = 0;
        // 'x' bytes only after the NUL, so they must never be counted.
        for slot in input.iter_mut().skip(pos + 1).step_by(7) {
            *slot = b'x';
        }
        assert_same(&format!("NUL at {pos}"), &input);
    }
}

// ---------------------------------------------------------------------------
// Phase C: paths reached through how the streams are wired up rather than
// through the bytes themselves. `fread` loops until the buffer is full or the
// stream ends, and it silently tolerates a read error; `printf` failures are
// silently ignored too. All of that is observable from outside the program.
// ---------------------------------------------------------------------------

/// Several short reads must accumulate: a single `read` returning early must
/// not end the input, or the counts would be too low.
#[test]
fn stdin_delivered_in_small_slow_chunks() {
    let a = vec![b'A'; 25];
    let x = vec![b'x'; 25];
    let mut chunks: Vec<&[u8]> = Vec::new();
    for i in 0..40 {
        chunks.push(if i % 2 == 0 { &a } else { &x });
    }
    // 1000 bytes in 25-byte pieces: this fills the buffer, so take the C's
    // majority behavior as the reference (see `c_reference`).
    let c = c_reference("slow chunked stdin", 3, || {
        run_chunked(&c_bin(), &[], &chunks, true)
    });
    let r = run_chunked(&rust_bin(), &[], &chunks, true);
    assert_eq!(
        show(&c.stdout),
        "A: 500\nx: 500\n",
        "the C did not read all 1000 bytes"
    );
    compare("slow chunked stdin", &c, &r);
}

/// The stream ends after one short chunk: both must stop with what they have.
#[test]
fn stdin_ends_after_one_short_chunk() {
    let chunk = b"AAAxx".to_vec();
    let c = run_chunked(&c_bin(), &[], &[&chunk], true);
    let r = run_chunked(&rust_bin(), &[], &[&chunk], true);
    compare("single short chunk", &c, &r);
}

/// stdin from /dev/null: an immediate end of file, no bytes at all.
#[test]
fn stdin_is_dev_null() {
    let cfg = |cmd: &mut Command| {
        cmd.stdin(Stdio::null());
    };
    compare(
        "stdin=/dev/null",
        &run_configured(&c_bin(), &cfg),
        &run_configured(&rust_bin(), &cfg),
    );
}

/// stdin from a regular file rather than a pipe (a different `fread` path,
/// since the whole request can be satisfied at once).
#[test]
fn stdin_is_a_regular_file() {
    let path = std::env::temp_dir().join(format!("driver_stdin_{}", std::process::id()));
    let mut body = vec![b'A'; 700];
    body.extend(std::iter::repeat(b'x').take(500));
    std::fs::write(&path, &body).expect("write temp stdin file");

    let open = |cmd: &mut Command| {
        let f = std::fs::File::open(&path).expect("open temp stdin file");
        cmd.stdin(Stdio::from(f));
    };
    // 1200 bytes, so the buffer ends up full and unterminated.
    let c = c_reference("stdin=regular file", FULL_BUFFER_SAMPLES, || {
        run_configured(&c_bin(), &open)
    });
    let r = run_configured(&rust_bin(), &open);
    let _ = std::fs::remove_file(&path);
    compare("stdin=regular file", &c, &r);
}

/// An empty regular file on stdin (as opposed to an empty pipe).
#[test]
fn stdin_is_an_empty_regular_file() {
    let path = std::env::temp_dir().join(format!("driver_empty_{}", std::process::id()));
    std::fs::write(&path, b"").expect("write empty temp file");
    let open = |cmd: &mut Command| {
        let f = std::fs::File::open(&path).expect("open empty temp file");
        cmd.stdin(Stdio::from(f));
    };
    let c = run_configured(&c_bin(), &open);
    let r = run_configured(&rust_bin(), &open);
    let _ = std::fs::remove_file(&path);
    compare("stdin=empty file", &c, &r);
}

/// stdin closed outright: `fread` fails instead of hitting end of file. The C
/// ignores the failure and the buffer stays zeroed, so the counts are 0.
#[cfg(unix)]
#[test]
fn stdin_is_closed() {
    use std::os::unix::process::CommandExt;
    extern "C" {
        fn close(fd: i32) -> i32;
    }
    let cfg = |cmd: &mut Command| {
        unsafe {
            cmd.pre_exec(|| {
                close(0);
                Ok(())
            });
        }
    };
    let c = run_configured(&c_bin(), &cfg);
    let r = run_configured(&rust_bin(), &cfg);
    assert_eq!(show(&c.stdout), "A: 0\nx: 0\n");
    compare("stdin closed", &c, &r);
}

/// stdout that always fails to accept data (/dev/full). Both must ignore the
/// write error and still exit 0, producing nothing on stderr.
#[cfg(target_os = "linux")]
#[test]
fn stdout_write_fails_on_dev_full() {
    let cfg = |cmd: &mut Command| {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("open /dev/full");
        cmd.stdout(Stdio::from(f));
    };
    let c = run_configured(&c_bin(), &cfg);
    let r = run_configured(&rust_bin(), &cfg);
    compare("stdout=/dev/full", &c, &r);
}

/// stdout is a pipe with no reader. The C process is killed by SIGPIPE; the
/// Rust process must be killed the same way rather than exiting 0.
#[cfg(unix)]
#[test]
fn stdout_is_a_pipe_with_no_reader() {
    fn run_into_dead_pipe(program: &Path) -> Run {
        // `true` gives us a pipe whose read end is closed as soon as it exits,
        // while we keep the write end.
        let mut reader = Command::new("true")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn `true`");
        let write_end = reader.stdin.take().expect("piped stdin of `true`");
        reader.wait().expect("wait for `true`");
        let write_end: std::os::unix::io::OwnedFd = write_end.into();

        run_configured(program, &|cmd: &mut Command| {
            let dup = write_end.try_clone().expect("clone pipe write end");
            cmd.stdout(Stdio::from(dup));
        })
    }

    let c = run_into_dead_pipe(&c_bin());
    let r = run_into_dead_pipe(&rust_bin());
    assert_eq!(c.signal, Some(13), "expected the C program to die from SIGPIPE");
    compare("stdout=closed pipe", &c, &r);
}
