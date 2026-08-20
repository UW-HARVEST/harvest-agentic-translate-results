//! End-to-end differential tests of the two *executables* (CONFIGS.md rows
//! 31-33 and the executable half of ERRORS.md row 10).
//!
//! The shared-object tests cover the exported functions; these cover the
//! program as a whole: process startup, stdin handling, stdout flushing at
//! exit, exit status and signal disposition.

mod common;

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::FromRawFd;
use std::os::raw::c_int;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use common::{c_exe_path, escape, locale_available, rust_exe_path, target_dir, Rng, SEED};

struct Out {
    stdout: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Out {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Out {{ code: {:?}, signal: {:?}, stdout ({}): {} }}",
            self.code,
            self.signal,
            self.stdout.len(),
            escape(&self.stdout)
        )
    }
}

fn tmp_path(tag: &str) -> PathBuf {
    let d = target_dir().join("ittmp");
    fs::create_dir_all(&d).unwrap();
    d.join(format!("e2e.{tag}.{}", std::process::id()))
}

fn stdin_file(bytes: &[u8]) -> File {
    let p = tmp_path("stdin");
    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&p)
        .unwrap();
    f.write_all(bytes).unwrap();
    f.flush().unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    let _ = fs::remove_file(&p);
    f
}

/// Runs `prog` with the given stdin bytes delivered through a regular file.
fn run_file_stdin(prog: &Path, input: &[u8], env: &[(&str, &str)]) -> Out {
    let f = stdin_file(input);
    let out = Command::new(prog)
        .stdin(Stdio::from(f))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .envs(env.iter().copied())
        .output()
        .expect("spawn");
    Out {
        stdout: out.stdout,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Runs `prog` with stdin delivered through a pipe (non-seekable).
fn run_pipe_stdin(prog: &Path, input: &[u8]) -> Out {
    let mut child = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().unwrap();
    let data = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
        // dropping closes the pipe -> EOF
    });
    let out = child.wait_with_output().expect("wait");
    writer.join().unwrap();
    Out {
        stdout: out.stdout,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Runs `prog` with stdout connected to a pipe whose read end is already
/// closed, so the first write fails with EPIPE.
fn run_broken_stdout(prog: &Path, input: &[u8]) -> Out {
    let mut fds = [0 as c_int; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    unsafe { libc::close(fds[0]) };
    let wr = unsafe { File::from_raw_fd(fds[1]) };
    let f = stdin_file(input);
    let status = Command::new(prog)
        .stdin(Stdio::from(f))
        .stdout(Stdio::from(wr))
        .stderr(Stdio::null())
        .status()
        .expect("spawn");
    Out {
        stdout: Vec::new(),
        code: status.code(),
        signal: status.signal(),
    }
}

fn assert_same(label: &str, c: Out, r: Out) -> Out {
    assert_eq!(c.stdout, r.stdout, "{label}: stdout differs\nC:    {c:?}\nRust: {r:?}");
    assert_eq!(c.code, r.code, "{label}: exit code differs\nC: {c:?}\nRust: {r:?}");
    assert_eq!(
        c.signal, r.signal,
        "{label}: termination signal differs\nC: {c:?}\nRust: {r:?}"
    );
    c
}

#[test]
fn e2e_all_bytes_and_eof() {
    let (cx, rx) = (c_exe_path(), rust_exe_path());
    for b in 0..=255u8 {
        let label = format!("e2e/file 0x{b:02x}");
        let out = assert_same(
            &label,
            run_file_stdin(&cx, &[b], &[]),
            run_file_stdin(&rx, &[b], &[]),
        );
        assert_eq!(out.code, Some(0));
        assert!(!out.stdout.is_empty());
    }
    // EOF (empty stdin), both stdin shapes.
    let out = assert_same(
        "e2e/eof-file",
        run_file_stdin(&cx, &[], &[]),
        run_file_stdin(&rx, &[], &[]),
    );
    assert_eq!(out.code, Some(0));
    assert_same(
        "e2e/eof-pipe",
        run_pipe_stdin(&cx, &[]),
        run_pipe_stdin(&rx, &[]),
    );
    // /dev/null as stdin.
    for prog in [&cx, &rx] {
        assert!(prog.is_file(), "{} missing", prog.display());
    }
    let c = Command::new(&cx)
        .stdin(Stdio::from(File::open("/dev/null").unwrap()))
        .output()
        .unwrap();
    let r = Command::new(&rx)
        .stdin(Stdio::from(File::open("/dev/null").unwrap()))
        .output()
        .unwrap();
    assert_eq!(c.stdout, r.stdout, "e2e//dev/null");
    assert_eq!(c.status.code(), r.status.code());
}

#[test]
fn e2e_random_multibyte_stdin_pipe_stdout() {
    let (cx, rx) = (c_exe_path(), rust_exe_path());
    let mut rng = Rng::new(SEED ^ 0xE2E0);
    for i in 0..48 {
        let len = 1 + rng.below(9000);
        let data = rng.bytes(len);
        assert_same(
            &format!("e2e/random file len {len} @{i}"),
            run_file_stdin(&cx, &data, &[]),
            run_file_stdin(&rx, &data, &[]),
        );
        let short = &data[..1 + rng.below(std::cmp::min(len, 512))];
        assert_same(
            &format!("e2e/random pipe len {} @{i}", short.len()),
            run_pipe_stdin(&cx, short),
            run_pipe_stdin(&rx, short),
        );
    }
}

#[test]
fn e2e_env_locale_variants() {
    // The program calls setlocale(LC_ALL, "C"), so the environment's locale
    // must not change anything.
    let (cx, rx) = (c_exe_path(), rust_exe_path());
    let mut rng = Rng::new(SEED ^ 0xE2E1);
    for loc in ["C", "POSIX", "C.utf8", "en_US.utf8", "en_US.iso88591", "de_DE.iso88591"] {
        if !locale_available(loc) {
            continue;
        }
        for _ in 0..8 {
            let b = rng.byte();
            let env = [("LC_ALL", loc), ("LANG", loc)];
            let out = assert_same(
                &format!("e2e/{loc} 0x{b:02x}"),
                run_file_stdin(&cx, &[b], &env),
                run_file_stdin(&rx, &[b], &env),
            );
            // Must equal the plain-environment output as well.
            let plain = run_file_stdin(&cx, &[b], &[]);
            assert_eq!(
                out.stdout, plain.stdout,
                "LC_ALL={loc} changed the output for 0x{b:02x}"
            );
        }
    }
}

#[test]
fn e2e_broken_pipe_kills_with_sigpipe() {
    // ERRORS.md row 10: writing to a closed pipe must terminate the process
    // with SIGPIPE (status 141), because a C program starts with the default
    // disposition.
    let (cx, rx) = (c_exe_path(), rust_exe_path());
    for i in 0..8 {
        let c = run_broken_stdout(&cx, b"A");
        let r = run_broken_stdout(&rx, b"A");
        assert_eq!(c.signal, Some(13), "C run {i}: {c:?}");
        assert_eq!(r.signal, Some(13), "Rust run {i}: {r:?}");
        assert_same(&format!("e2e/broken-pipe @{i}"), c, r);
    }
}

#[test]
fn e2e_stdout_closed() {
    // ERRORS.md row 8 at the executable level: fd 1 closed -> no output, but a
    // successful exit status.
    let (cx, rx) = (c_exe_path(), rust_exe_path());
    for prog in [&cx, &rx] {
        let f = stdin_file(b"A");
        let status = Command::new(prog)
            .stdin(Stdio::from(f))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(0), "{}", prog.display());
    }
}

#[test]
fn e2e_stdin_is_a_directory() {
    let (cx, rx) = (c_exe_path(), rust_exe_path());
    let dir = File::open(common::manifest_dir()).unwrap();
    let c = Command::new(&cx)
        .stdin(Stdio::from(dir))
        .output()
        .expect("spawn");
    let dir = File::open(common::manifest_dir()).unwrap();
    let r = Command::new(&rx)
        .stdin(Stdio::from(dir))
        .output()
        .expect("spawn");
    assert_eq!(c.stdout, r.stdout, "stdin = directory");
    assert_eq!(c.status.code(), r.status.code());
    // Identical to the EOF output.
    let eof = run_file_stdin(&cx, &[], &[]);
    assert_eq!(c.stdout, eof.stdout);
}

#[test]
fn e2e_no_stderr_output() {
    let (cx, rx) = (c_exe_path(), rust_exe_path());
    let mut rng = Rng::new(SEED ^ 0xE2E2);
    for _ in 0..16 {
        let b = rng.byte();
        let f = stdin_file(&[b]);
        let c = Command::new(&cx).stdin(Stdio::from(f)).output().unwrap();
        let f = stdin_file(&[b]);
        let r = Command::new(&rx).stdin(Stdio::from(f)).output().unwrap();
        assert_eq!(c.stderr, r.stderr, "stderr differs for 0x{b:02x}");
        assert!(c.stderr.is_empty());
    }
}

// Silence "unused" warnings for helpers imported for readability.
#[allow(dead_code)]
fn _unused(p: &Path) -> Option<Vec<u8>> {
    let mut v = Vec::new();
    File::open(p).ok()?.read_to_end(&mut v).ok()?;
    Some(v)
}
