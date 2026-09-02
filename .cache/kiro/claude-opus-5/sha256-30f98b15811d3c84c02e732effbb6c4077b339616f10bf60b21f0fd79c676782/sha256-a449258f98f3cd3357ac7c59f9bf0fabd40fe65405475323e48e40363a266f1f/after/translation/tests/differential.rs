//! Differential tests: run the C `driver` and the Rust `driver` as
//! subprocesses on the same stdin and require byte-identical stdout, stderr
//! and identical exit status.
//!
//! The Rust code is NEVER called as a library here; only the built binary is
//! driven, exactly the way a shell would drive it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path to the Rust binary produced by cargo for this crate.
fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn repo_root() -> PathBuf {
    // translation/ -> workspace root
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Path to the C binary, building it with cmake on first use if needed.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = repo_root().join("c_src");
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
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&cfg.stdout),
                String::from_utf8_lossy(&cfg.stderr)
            );
            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run cmake --build");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
    .as_path()
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

fn invoke(bin: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(input)
        .expect("write stdin");
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("wait for child");

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;

    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal,
    }
}

fn show(bytes: &[u8]) -> String {
    // Escaped, byte-exact rendering so failures are readable for any input.
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// The single assertion used by every case: stdout, stderr and exit status.
fn assert_same(name: &str, input: &[u8]) {
    let c = invoke(c_bin(), input);
    let r = invoke(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{name}] stdout differs\n  input : {}\n  C     : {}\n  Rust  : {}",
        show(input),
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{name}] stderr differs\n  input : {}\n  C     : {}\n  Rust  : {}",
        show(input),
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{name}] exit status differs\n  input : {}\n  C     : code={:?} signal={:?}\n  Rust  : code={:?} signal={:?}",
        show(input),
        c.code,
        c.signal,
        r.code,
        r.signal
    );
}

// ---------------------------------------------------------------------------
// Input classes the C source actually branches on.
//
// main():      fgets(in, 100, stdin); parse_val(in, &x) ? run()x2 : error
// parse_val(): strtol(str, &endp, 10); endp != str && errno == 0
//              && tmp >= INT_MIN && tmp <= INT_MAX
// ---------------------------------------------------------------------------

// -- parse_val() succeeds: the run()/run() path -----------------------------

#[test]
fn accepts_single_digit() {
    assert_same("single_digit", b"1\n");
}

#[test]
fn accepts_zero() {
    assert_same("zero", b"0\n");
}

#[test]
fn accepts_negative_zero() {
    assert_same("negative_zero", b"-0\n");
}

#[test]
fn accepts_plain_number() {
    assert_same("plain_number", b"42\n");
}

#[test]
fn accepts_negative_number() {
    assert_same("negative_number", b"-1\n");
}

#[test]
fn accepts_explicit_plus_sign() {
    assert_same("plus_sign", b"+3\n");
}

#[test]
fn accepts_leading_zeros() {
    // strtol base 10: no octal interpretation.
    assert_same("leading_zeros", b"000000042\n");
}

#[test]
fn accepts_leading_whitespace_forms() {
    // strtol skips isspace(): space, \t, \v, \f, \r, \n
    assert_same("lead_space", b"   7\n");
    assert_same("lead_tab", b"\t7\n");
    assert_same("lead_vtab", b"\x0b7\n");
    assert_same("lead_formfeed", b"\x0c7\n");
    assert_same("lead_cr", b"\r7\n");
    assert_same("lead_mixed", b" \t\r\x0b\x0c-42\n");
}

#[test]
fn accepts_no_trailing_newline() {
    // fgets stops at EOF without a '\n'.
    assert_same("no_newline", b"5");
    assert_same("no_newline_int_min", b"-2147483648");
}

#[test]
fn accepts_trailing_garbage_after_digits() {
    // parse_val never checks *endp, so trailing junk is accepted.
    assert_same("digits_then_alpha", b"42abc\n");
    assert_same("digits_then_space_digits", b"1 2\n");
    assert_same("digits_then_dot", b"2.5\n");
    assert_same("digits_then_e", b"1e5\n");
    assert_same("digits_then_underscore", b"1_000\n");
    assert_same("hex_prefix_base10", b"0x10\n");
    assert_same("digits_then_crlf", b"42\r\n");
    assert_same("digits_then_high_byte", b"7\xff\n");
    assert_same("negative_padded", b"  -42  \n");
}

#[test]
fn accepts_int_boundaries() {
    assert_same("int_max", b"2147483647\n");
    assert_same("int_min", b"-2147483648\n");
    assert_same("int_max_minus_one", b"2147483646\n");
    assert_same("int_min_plus_one", b"-2147483647\n");
}

#[test]
fn bedrooms_overflow_wraps_like_c() {
    // 5 + x and 5 + 2x overflow signed int for these; must match the C.
    assert_same("ovf_int_max", b"2147483647\n");
    assert_same("ovf_half", b"1073741824\n");
    assert_same("ovf_quarter", b"2147483640\n");
    assert_same("ovf_neg_int_min", b"-2147483648\n");
    assert_same("ovf_neg_large", b"-2147483647\n");
    assert_same("ovf_neg_half", b"-1073741824\n");
}

#[test]
fn fgets_reads_only_the_first_line() {
    assert_same("two_lines", b"42\n99\n");
    assert_same("many_lines", b"7\nabc\n-1\n");
}

#[test]
fn embedded_nul_terminates_the_c_string() {
    // The C buffer is NUL-terminated, so strtol stops at an embedded NUL.
    assert_same("nul_mid", b"4\x002\n");
    assert_same("nul_then_junk", b"12\x00abc\n");
}

// -- fgets buffer limit: char in[100], so at most 99 bytes are read ----------

#[test]
fn fgets_truncates_at_99_bytes() {
    let mut v = vec![b'0'; 97];
    v.extend_from_slice(b"42\n");
    assert_same("digits_97_zeros_then_42", &v); // 99 digits, fits exactly

    let mut v = vec![b'1'; 99];
    v.push(b'\n');
    assert_same("digits_exactly_99", &v); // overflow -> ERANGE -> error

    let mut v = vec![b'1'; 100];
    v.push(b'\n');
    assert_same("digits_100", &v); // truncated to 99 -> ERANGE -> error

    let mut v = vec![b'0'; 98];
    v.push(b'5');
    v.extend(std::iter::repeat(b'9').take(20));
    v.push(b'\n');
    assert_same("truncated_mid_number", &v); // reads "0..05" == 5

    let mut v = vec![b'0'; 99];
    v.push(b'\n');
    assert_same("ninety_nine_zeros", &v);

    // A short valid number followed by 200 bytes of junk on the same line.
    let mut v = b"8".to_vec();
    v.extend(std::iter::repeat(b'z').take(200));
    v.push(b'\n');
    assert_same("valid_then_long_junk", &v);
}

// -- parse_val() fails: the "An error occurred" path ------------------------

#[test]
fn rejects_empty_input() {
    // fgets returns NULL; in[] keeps its "" initializer.
    assert_same("empty", b"");
}

#[test]
fn rejects_newline_only() {
    assert_same("newline_only", b"\n");
}

#[test]
fn rejects_whitespace_only() {
    assert_same("spaces_only", b"   \n");
    assert_same("tabs_cr_only", b"\t\r\n");
    assert_same("all_space_kinds", b" \t\x0b\x0c\r\n");
    assert_same("whitespace_no_newline", b"    ");
}

#[test]
fn rejects_empty_first_line() {
    assert_same("empty_first_line", b"\n42\n");
}

#[test]
fn rejects_leading_nul() {
    assert_same("nul_first", b"\x0042\n");
}

#[test]
fn rejects_non_numeric() {
    // endp == str -> no conversion
    assert_same("alpha", b"abc\n");
    assert_same("dot", b".\n");
    assert_same("just_plus", b"+\n");
    assert_same("just_minus", b"-\n");
    assert_same("double_sign", b"--5\n");
    assert_same("plus_minus", b"+-5\n");
    assert_same("space_after_sign", b"- 5\n");
    assert_same("hex_only", b"0x\n"); // parses "0", so this one SUCCEEDS
    assert_same("x_first", b"x10\n");
    assert_same("comma", b",5\n");
    assert_same("high_bytes", b"\xff\xfe\n");
}

#[test]
fn rejects_out_of_int_range_but_in_long_range() {
    // errno == 0, but tmp outside [INT_MIN, INT_MAX]
    assert_same("int_max_plus_1", b"2147483648\n");
    assert_same("int_min_minus_1", b"-2147483649\n");
    assert_same("three_billion", b"3000000000\n");
    assert_same("neg_three_billion", b"-3000000000\n");
    assert_same("long_max", b"9223372036854775807\n");
    assert_same("long_min", b"-9223372036854775808\n");
}

#[test]
fn rejects_erange_overflow() {
    // strtol sets ERANGE -> errno != 0
    assert_same("long_max_plus_1", b"9223372036854775808\n");
    assert_same("long_min_minus_1", b"-9223372036854775809\n");
    assert_same("forty_nines", &[b"9".repeat(40), b"\n".to_vec()].concat());
    assert_same("neg_forty_nines", &[b"-".to_vec(), b"9".repeat(40), b"\n".to_vec()].concat());
    assert_same(
        "erange_with_trailing_junk",
        b"99999999999999999999999999999999 hello\n",
    );
}

#[test]
fn rejects_long_non_numeric_line() {
    let mut v = vec![b'a'; 120];
    v.push(b'\n');
    assert_same("120_letters", &v);
}

#[test]
fn handles_arbitrary_binary_bytes() {
    let mut v: Vec<u8> = (1u8..60).collect();
    v.push(b'\n');
    assert_same("byte_range_1_59", &v);

    let v: Vec<u8> = (0u8..=255).collect();
    assert_same("all_256_bytes", &v);
}

// -- broad sweep over numeric magnitudes -----------------------------------

#[test]
fn sweeps_powers_of_two_and_boundaries() {
    let mut vals: Vec<i128> = vec![0, -1, 1];
    for bits in 0..=70 {
        let p: i128 = 1i128 << bits;
        vals.push(p - 1);
        vals.push(p);
        vals.push(p + 1);
        vals.push(-(p - 1));
        vals.push(-p);
        vals.push(-(p + 1));
    }
    for v in vals {
        let s = format!("{v}\n");
        assert_same(&format!("sweep_{v}"), s.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Output-stream failure classes.
//
// The C code discards every `printf` return value and returns 0 regardless, so
// a failing stdout must not change the exit status. It also runs with the
// default SIGPIPE disposition, unlike a stock Rust program.
// ---------------------------------------------------------------------------

/// Runs `bin` with stdin fed from `input` and stdout pointed at `/dev/full`,
/// so every write fails with ENOSPC.
#[cfg(unix)]
fn invoke_stdout_enospc(bin: &Path, input: &[u8]) -> Outcome {
    use std::fs::OpenOptions;
    let dev_full = OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("open /dev/full");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(dev_full))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(input)
        .expect("write stdin");
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("wait");
    use std::os::unix::process::ExitStatusExt;
    Outcome {
        stdout: Vec::new(), // went to /dev/full
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

#[cfg(unix)]
#[test]
fn stdout_write_error_is_ignored_like_c() {
    if !Path::new("/dev/full").exists() {
        return;
    }
    for input in [&b"42\n"[..], &b"abc\n"[..], &b"-2147483648\n"[..], &b""[..]] {
        let c = invoke_stdout_enospc(c_bin(), input);
        let r = invoke_stdout_enospc(&rust_bin(), input);
        assert_eq!(
            c.stderr,
            r.stderr,
            "/dev/full stderr differs for {}\n  C   : {}\n  Rust: {}",
            show(input),
            show(&c.stderr),
            show(&r.stderr)
        );
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "/dev/full exit status differs for {}\n  C   : code={:?} signal={:?}\n  Rust: code={:?} signal={:?}",
            show(input),
            c.code,
            c.signal,
            r.code,
            r.signal
        );
    }
}

/// Runs `bin` with stdout connected to a pipe whose read end is closed before
/// any output is produced. The child is blocked reading stdin until after the
/// close, so the EPIPE is deterministic.
#[cfg(unix)]
fn invoke_stdout_closed_pipe(bin: &Path, input: &[u8]) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    // Close the read end first; the child cannot have written yet because it
    // is still waiting on stdin.
    drop(child.stdout.take());
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(input)
        .expect("write stdin");
    drop(child.stdin.take());
    let mut stderr = Vec::new();
    if let Some(mut e) = child.stderr.take() {
        use std::io::Read as _;
        let _ = e.read_to_end(&mut stderr);
    }
    let status = child.wait().expect("wait");
    Outcome {
        stdout: Vec::new(),
        stderr,
        code: status.code(),
        signal: status.signal(),
    }
}

#[cfg(unix)]
#[test]
fn sigpipe_disposition_matches_c() {
    for input in [&b"42\n"[..], &b"abc\n"[..], &b"2147483647\n"[..]] {
        let c = invoke_stdout_closed_pipe(c_bin(), input);
        let r = invoke_stdout_closed_pipe(&rust_bin(), input);
        assert_eq!(
            c.stderr,
            r.stderr,
            "closed-pipe stderr differs for {}\n  C   : {}\n  Rust: {}",
            show(input),
            show(&c.stderr),
            show(&r.stderr)
        );
        assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "closed-pipe exit status differs for {}\n  C   : code={:?} signal={:?}\n  Rust: code={:?} signal={:?}",
            show(input),
            c.code,
            c.signal,
            r.code,
            r.signal
        );
    }
}

/// Runs `bin` with file descriptor 0 closed outright, so `fgets` fails with
/// EBADF rather than seeing EOF.
#[cfg(unix)]
fn invoke_stdin_closed(bin: &Path) -> Outcome {
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    let mut cmd = Command::new(bin);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Safety: only closes fd 0 in the forked child, between fork and exec.
    unsafe {
        cmd.pre_exec(|| {
            extern "C" {
                fn close(fd: i32) -> i32;
            }
            close(0);
            Ok(())
        });
    }
    let out = cmd.output().expect("run with fd 0 closed");
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

#[cfg(unix)]
#[test]
fn closed_stdin_matches_c() {
    let c = invoke_stdin_closed(c_bin());
    let r = invoke_stdin_closed(&rust_bin());
    assert_eq!(
        c.stdout,
        r.stdout,
        "closed-stdin stdout differs\n  C   : {}\n  Rust: {}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(c.stderr, r.stderr, "closed-stdin stderr differs");
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "closed-stdin exit status differs"
    );
}

#[cfg(unix)]
#[test]
fn stdin_from_directory_matches_c() {
    // read(2) on a directory fd fails with EISDIR, so fgets fails.
    use std::fs::File;
    let dir = File::open(repo_root()).expect("open repo root as file");
    let run = |bin: &Path, f: File| -> Outcome {
        use std::os::unix::process::ExitStatusExt;
        let out = Command::new(bin)
            .stdin(Stdio::from(f))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run with directory stdin");
        Outcome {
            stdout: out.stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        }
    };
    let c = run(c_bin(), dir);
    let dir2 = File::open(repo_root()).expect("reopen repo root");
    let r = run(&rust_bin(), dir2);
    assert_eq!(
        c.stdout,
        r.stdout,
        "dir-stdin stdout differs\n  C   : {}\n  Rust: {}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(c.stderr, r.stderr, "dir-stdin stderr differs");
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "dir-stdin exit status differs"
    );
}

#[test]
fn sweeps_deterministic_pseudorandom_inputs() {
    // Small xorshift PRNG so the corpus is reproducible without dev-deps.
    let mut state: u64 = 0x9e3779b97f4a7c15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    const ALPHA: &[u8] = b"0123456789+- \t\n\r\x0b\x0cabcxX.eE\x00\xff_";

    for i in 0..600u32 {
        let input: Vec<u8> = match i % 3 {
            0 => {
                let len = (next() % 13) as usize;
                (0..len).map(|_| ALPHA[(next() % ALPHA.len() as u64) as usize]).collect()
            }
            1 => {
                let n = next() as i64;
                format!("{n}\n").into_bytes()
            }
            _ => {
                let len = 90 + (next() % 21) as usize;
                (0..len).map(|_| (next() % 256) as u8).collect()
            }
        };
        assert_same(&format!("random_{i}"), &input);
    }
}
