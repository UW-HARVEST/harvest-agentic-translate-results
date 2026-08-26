//! Process-level axes that `std::process::Command` cannot express:
//!
//!   * ERRORS.md E05  — `argc == 0` (raw `execve` with an empty argv)
//!   * ERRORS.md E34/E35 — stdout / stderr closed (fd 1 or 2 not open at all)
//!   * ERRORS.md E36/E37 — stdout / stderr is a pipe with **no reader**: the C
//!     process is killed by SIGPIPE, so the Rust one must be too
//!   * CONFIGS.md C26–C30 — arg counts, stdio destinations, environment/locale,
//!     non-UTF-8 argv
//!
//! Both binaries are always driven identically and compared byte-for-byte.

mod common;
use common::*;

use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Command, Stdio};

// ------------------------------------------------------------------ helpers

fn pipe2() -> (i32, i32) {
    let mut fds = [0i32; 2];
    let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
    assert_eq!(rc, 0, "pipe() failed");
    (fds[0], fds[1])
}

/// Run `bin` through a raw `fork` + `execve` so that argv/envp can be
/// completely empty (`argc == 0`), which `Command` cannot do.
fn run_execve(bin: &Path, argv: &[&str], envp: &[&str]) -> Out {
    let cpath = CString::new(bin.as_os_str().as_encoded_bytes()).unwrap();
    let cargv: Vec<CString> = argv.iter().map(|s| CString::new(*s).unwrap()).collect();
    let cenvp: Vec<CString> = envp.iter().map(|s| CString::new(*s).unwrap()).collect();
    let mut pargv: Vec<*const libc::c_char> = cargv.iter().map(|c| c.as_ptr()).collect();
    pargv.push(std::ptr::null());
    let mut penvp: Vec<*const libc::c_char> = cenvp.iter().map(|c| c.as_ptr()).collect();
    penvp.push(std::ptr::null());

    let (o_r, o_w) = pipe2();
    let (e_r, e_w) = pipe2();

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        // child: no allocation, no locks -- just dup2 + execve
        unsafe {
            libc::close(o_r);
            libc::close(e_r);
            libc::dup2(o_w, 1);
            libc::dup2(e_w, 2);
            if o_w > 2 {
                libc::close(o_w);
            }
            if e_w > 2 {
                libc::close(e_w);
            }
            let devnull = libc::open(b"/dev/null\0".as_ptr() as *const libc::c_char, libc::O_RDONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 0);
                if devnull > 2 {
                    libc::close(devnull);
                }
            }
            libc::execve(cpath.as_ptr(), pargv.as_ptr(), penvp.as_ptr());
            libc::_exit(127);
        }
    }
    unsafe {
        libc::close(o_w);
        libc::close(e_w);
    }
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    unsafe { File::from_raw_fd(o_r) }.read_to_end(&mut stdout).unwrap();
    unsafe { File::from_raw_fd(e_r) }.read_to_end(&mut stderr).unwrap();
    let mut status = 0i32;
    unsafe { libc::waitpid(pid, &mut status, 0) };
    let (code, signal) = if libc::WIFSIGNALED(status) {
        (None, Some(libc::WTERMSIG(status)))
    } else {
        (Some(libc::WEXITSTATUS(status)), None)
    };
    Out {
        code,
        signal,
        stdout,
        stderr,
    }
}

/// Run `bin` with fd 1 (or 2) *closed* rather than redirected.
fn run_with_closed_fd(bin: &Path, args: &[&str], close_fd: i32) -> Out {
    let mut cmd = Command::new(bin);
    cmd.arg0(ARGV0);
    cmd.args(args);
    cmd.stdin(Stdio::null());
    if close_fd == 1 {
        cmd.stdout(Stdio::null()).stderr(Stdio::piped());
    } else {
        cmd.stdout(Stdio::piped()).stderr(Stdio::null());
    }
    unsafe {
        cmd.pre_exec(move || {
            // std sets up the stdio fds before running pre_exec callbacks.
            libc::close(close_fd);
            Ok(())
        });
    }
    let o = cmd.spawn().unwrap().wait_with_output().unwrap();
    Out {
        code: o.status.code(),
        signal: o.status.signal(),
        stdout: o.stdout,
        stderr: o.stderr,
    }
}

/// Run `bin` with fd 1 (or 2) connected to a pipe whose read end is already
/// closed => writing must raise SIGPIPE.
fn run_with_dead_pipe(bin: &Path, args: &[&str], pipe_fd: i32) -> Out {
    let (r, w) = pipe2();
    unsafe { libc::close(r) }; // no reader, ever
    let w_owned = unsafe { OwnedFd::from_raw_fd(w) };

    let mut cmd = Command::new(bin);
    cmd.arg0(ARGV0);
    cmd.args(args);
    cmd.stdin(Stdio::null());
    if pipe_fd == 1 {
        cmd.stdout(Stdio::from(w_owned)).stderr(Stdio::piped());
    } else {
        cmd.stderr(Stdio::from(w_owned)).stdout(Stdio::piped());
    }
    let o = cmd.spawn().unwrap().wait_with_output().unwrap();
    Out {
        code: o.status.code(),
        signal: o.status.signal(),
        stdout: o.stdout,
        stderr: o.stderr,
    }
}

fn same(row: &str, c: Out, r: Out) -> Out {
    assert_eq!(c, r, "[{row}] DIVERGENCE\n  C    : {c:?}\n  RUST : {r:?}");
    c
}

// ------------------------------------------------------------------ E05 / C26

#[test]
fn e05_argc_zero_empty_argv() {
    // execve(path, {NULL}, {NULL}).  Linux >= 5.18 rewrites an empty argv to a
    // single empty string, so argc == 1 and `%s` prints nothing; the
    // argv[0]==NULL -> "(null)" branch is unreachable on this kernel.  Either
    // way both binaries must agree.
    let c = run_execve(&c_bin(), &[], &[]);
    let r = run_execve(&rust_bin(), &[], &[]);
    let o = same("E05", c, r);
    assert_eq!(o.code, Some(1), "{o:?}");
    assert_eq!(stderr_str(&o), "Usage:  base exponent\n");
    assert!(o.stdout.is_empty());
}

#[test]
fn c26_arg_counts_via_raw_execve() {
    // argc 1..5 through raw execve (no Command involvement), empty environment
    for n in 1..=5usize {
        let mut argv = vec![ARGV0];
        for _ in 1..n {
            argv.push("2");
        }
        let c = run_execve(&c_bin(), &argv, &[]);
        let r = run_execve(&rust_bin(), &argv, &[]);
        let o = same("C26", c, r);
        if n == 3 {
            assert_eq!(o.code, Some(0), "argc=3 -> {o:?}");
            assert_eq!(stdout_str(&o), "Result: 4.00\n");
        } else {
            assert_eq!(o.code, Some(1), "argc={n} -> {o:?}");
            assert_eq!(stderr_str(&o), "Usage: driver base exponent\n");
        }
    }
}

// ------------------------------------------------------------------ E34 / E35

#[test]
fn e34_stdout_closed_on_success_path() {
    // printf() fails with EBADF; the C ignores the return value and exits 0.
    let c = run_with_closed_fd(&c_bin(), &["2", "10"], 1);
    let r = run_with_closed_fd(&rust_bin(), &["2", "10"], 1);
    let o = same("E34", c, r);
    assert_eq!(o.code, Some(0), "{o:?}");
    assert!(o.stderr.is_empty(), "{o:?}");
}

#[test]
fn e35_stderr_closed_on_error_paths() {
    for args in [
        vec!["abc", "2"],
        vec!["1e400", "2"],
        vec!["2"],
        vec!["-2", "0.5"],
        vec!["10", "400"],
    ] {
        let c = run_with_closed_fd(&c_bin(), &args, 2);
        let r = run_with_closed_fd(&rust_bin(), &args, 2);
        let o = same("E35", c, r);
        assert_eq!(o.code, Some(1), "args={args:?} -> {o:?}");
        assert!(o.stdout.is_empty(), "{o:?}");
    }
}

// ------------------------------------------------------------------ E36 / E37 / C28

#[test]
fn e36_sigpipe_on_stdout_success_path() {
    // The C program keeps the default SIGPIPE disposition and is KILLED by
    // signal 13.  Rust's runtime installs SIG_IGN before main, so the
    // translation has to restore SIG_DFL to match.
    for args in [vec!["2", "10"], vec!["1e300", "1"], vec!["nan", "3"]] {
        let c = run_with_dead_pipe(&c_bin(), &args, 1);
        let r = run_with_dead_pipe(&rust_bin(), &args, 1);
        let o = same("E36+C28", c, r);
        assert_eq!(o.signal, Some(libc::SIGPIPE), "args={args:?} -> {o:?}");
        assert_eq!(o.code, None);
    }
}

#[test]
fn e37_sigpipe_on_stderr_error_path() {
    for args in [
        vec!["abc", "2"],
        vec!["1e400", "2"],
        vec!["-2", "0.5"],
        vec!["10", "400"],
        vec!["2"],
    ] {
        let c = run_with_dead_pipe(&c_bin(), &args, 2);
        let r = run_with_dead_pipe(&rust_bin(), &args, 2);
        let o = same("E37+C28", c, r);
        assert_eq!(o.signal, Some(libc::SIGPIPE), "args={args:?} -> {o:?}");
    }
}

#[test]
fn e36b_no_sigpipe_when_nothing_is_written_to_that_stream() {
    // error path with a dead stdout pipe: nothing is written to stdout, so the
    // process must exit(1) normally instead of dying.
    let c = run_with_dead_pipe(&c_bin(), &["abc", "2"], 1);
    let r = run_with_dead_pipe(&rust_bin(), &["abc", "2"], 1);
    let o = same("E36b+C28", c, r);
    assert_eq!(o.code, Some(1), "{o:?}");
    assert_eq!(o.signal, None);
    assert_eq!(stderr_str(&o), "Invalid numeric input for base: 'abc'\n");

    // success path with a dead stderr pipe: nothing is written to stderr.
    let c = run_with_dead_pipe(&c_bin(), &["2", "10"], 2);
    let r = run_with_dead_pipe(&rust_bin(), &["2", "10"], 2);
    let o = same("E36b+C28", c, r);
    assert_eq!(o.code, Some(0), "{o:?}");
    assert_eq!(stdout_str(&o), "Result: 1024.00\n");
}

// ------------------------------------------------------------------ C27

#[test]
fn c27_stdout_to_file_and_devnull() {
    let dir = std::env::temp_dir();
    for (i, args) in [vec!["2", "10"], vec!["abc", "2"], vec!["-2", "0.5"]]
        .into_iter()
        .enumerate()
    {
        let mut got = Vec::new();
        for (which, bin) in [("c", c_bin()), ("r", rust_bin())] {
            let out = dir.join(format!("diff_{which}_{i}.out"));
            let err = dir.join(format!("diff_{which}_{i}.err"));
            let st = Command::new(&bin)
                .arg0(ARGV0)
                .args(&args)
                .stdin(Stdio::null())
                .stdout(File::create(&out).unwrap())
                .stderr(File::create(&err).unwrap())
                .status()
                .unwrap();
            got.push((
                st.code(),
                st.signal(),
                std::fs::read(&out).unwrap(),
                std::fs::read(&err).unwrap(),
            ));
            let _ = std::fs::remove_file(&out);
            let _ = std::fs::remove_file(&err);
        }
        assert_eq!(got[0], got[1], "[C27] file-redirect divergence {args:?}");

        // /dev/null
        let mut st2 = Vec::new();
        for bin in [c_bin(), rust_bin()] {
            let st = Command::new(&bin)
                .arg0(ARGV0)
                .args(&args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();
            st2.push((st.code(), st.signal()));
        }
        assert_eq!(st2[0], st2[1], "[C27] /dev/null divergence {args:?}");
    }
}

// ------------------------------------------------------------------ C29

#[test]
fn c29_locale_and_environment_have_no_effect() {
    // The C never calls setlocale(), so it stays in the "C" locale and the
    // decimal point is always '.' no matter what LC_* says.
    let envs: Vec<Vec<(&str, &str)>> = vec![
        vec![],
        vec![("LC_ALL", "de_DE.UTF-8")],
        vec![("LC_NUMERIC", "de_DE.UTF-8")],
        vec![("LC_ALL", "de_DE.UTF-8"), ("LANG", "de_DE.UTF-8")],
        vec![("LC_ALL", "C")],
        vec![("LC_ALL", "fr_FR.UTF-8"), ("LC_NUMERIC", "fr_FR.UTF-8")],
        vec![("LC_ALL", "nonexistent.locale.XYZ")],
    ];
    let cases = [
        ("2.5", "3"),      // success, fractional output
        ("0.125", "1"),    // tie rounding
        ("-2", "0.5"),     // EDOM message
        ("10", "400"),     // ERANGE message
        ("1e400", "2"),    // strtod ERANGE message
        ("abc", "2"),      // invalid input message
        ("1,5", "2"),      // comma is never a decimal separator
        ("1e300", "1"),    // 300-digit output
    ];
    for env in &envs {
        for (b, e) in cases {
            let o = assert_same_raw_env("C29", &[b.as_bytes(), e.as_bytes()], Some(env));
            // The decimal separator must always be '.', never ',': i.e. no
            // "<digit>,<digit>" sequence may appear.  (A plain ", " does appear
            // between the two pow arguments, which is fine.)
            let all = [o.stdout.clone(), o.stderr.clone()].concat();
            if b != "1,5" {
                let comma_decimal = all.windows(3).any(|w| {
                    w[0].is_ascii_digit() && w[1] == b',' && w[2].is_ascii_digit()
                });
                assert!(
                    !comma_decimal,
                    "[C29] locale decimal comma leaked into output with env={env:?}: {o:?}"
                );
            }
        }
    }
}

// ------------------------------------------------------------------ C30

#[test]
fn c30_non_utf8_and_metacharacter_arguments() {
    let raw: Vec<&[u8]> = vec![
        b"\xff",
        b"\x80\x81",
        b"\xc3",
        b"\xed\xa0\x80",     // UTF-16 surrogate encoded as UTF-8
        b"\xf4\x90\x80\x80", // above U+10FFFF
        b"%s",
        b"%n",
        b"%.2f",
        b"\\",
        b"\"",
        b"'",
        b"2\xff",
        b"\xff2",
        b"1.5\xc2\xa0", // NBSP is NOT whitespace for strtod
    ];
    for a in &raw {
        // as base
        assert_same_raw("C30", &[a, b"2"]);
        // as exponent
        assert_same_raw("C30", &[b"2", a]);
        // as argv[0] in the usage path
        let c = run_raw(&c_bin(), &String::from_utf8_lossy(a), &[], None);
        let r = run_raw(&rust_bin(), &String::from_utf8_lossy(a), &[], None);
        same("C30", c, r);
    }
}
