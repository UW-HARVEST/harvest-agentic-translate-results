//! Phase B (valid paths) and Phase C (error paths) at the process boundary.
//!
//! The C executable (built from the unmodified `c_src/src/main.c`) and the Rust
//! executable this crate ships are spawned under identical conditions, and every
//! externally observable result is compared byte-for-byte: stdout bytes, stderr
//! bytes, exit code, terminating signal, and the shared stdin file offset.
//!
//! Covers CONFIGS.md rows C1–C15 and ERRORS.md rows E1, E3–E8, g2, g3.

mod common;

use common::*;
use std::ffi::CStr;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

/// Spawn with stdout and stderr on pipes and collect everything.
fn plain_run(exe: &Path) -> Outcome {
    Outcome::from_output(Command::new(exe).output().expect("spawn driver"))
}

// ===========================================================================
// Phase B — valid configurations
// ===========================================================================

/// C1 — baseline: stdout+stderr pipes, no args, inherited environment.
#[test]
fn b1_baseline_pipe() {
    for i in 0..5 {
        let o = assert_same_exe(&format!("baseline run {i}"), plain_run);
        assert_eq!(o.stdout, EXPECTED, "stdout bytes");
        assert!(o.stderr.is_empty(), "stderr must be empty, got {:?}", o.stderr);
        assert_eq!(o.code, Some(0), "exit code");
        assert_eq!(o.signal, None, "must not be signalled");
    }
}

/// C2 — stdout is a regular file: compare the file's bytes.
#[test]
fn b2_stdout_regular_file() {
    let (bytes, o) = assert_same_exe("stdout=regular file", |exe| {
        let path = tmp_path("outfile");
        let f = std::fs::File::create(&path).expect("create out file");
        let out = Command::new(exe)
            .stdout(stdio_from(f.into_raw_fd()))
            .stderr(Stdio::piped())
            .output()
            .expect("spawn");
        let bytes = std::fs::read(&path).expect("read out file");
        let _ = std::fs::remove_file(&path);
        (bytes, Outcome::from_output(out))
    });
    assert_eq!(bytes, EXPECTED);
    assert_eq!(o.code, Some(0));
    assert!(o.stderr.is_empty());
}

/// C3 — stdout is opened `O_APPEND` on a file that already holds randomized
/// content: the program's bytes must land exactly after the existing prefix.
#[test]
fn b3_stdout_append_prefixed() {
    let mut rng = Rng::new();
    for _ in 0..10 {
        let plen = rng.range(0, 300);
        let prefix = rng.bytes(plen);
        let p = prefix.clone();
        let bytes = assert_same_exe("stdout=O_APPEND file with prefix", |exe| {
            let path = tmp_path("appendfile");
            std::fs::write(&path, &p).expect("seed prefix");
            let f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .expect("open append");
            let st = Command::new(exe)
                .stdout(stdio_from(f.into_raw_fd()))
                .stderr(Stdio::piped())
                .output()
                .expect("spawn");
            assert_eq!(st.status.code(), Some(0));
            let bytes = std::fs::read(&path).expect("read back");
            let _ = std::fs::remove_file(&path);
            bytes
        });
        let mut want = prefix.clone();
        want.extend_from_slice(EXPECTED);
        assert_eq!(
            bytes,
            want,
            "append output wrong for prefix len {}",
            prefix.len()
        );
    }
}

/// C4 — stdout is `/dev/null`.
#[test]
fn b4_stdout_devnull() {
    let o = assert_same_exe("stdout=/dev/null", |exe| {
        Outcome::from_output(
            Command::new(exe)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .output()
                .expect("spawn"),
        )
    });
    assert_eq!(o.code, Some(0));
    assert!(o.stderr.is_empty());
}

/// C5 — stdout is a **tty**, which is the case where glibc switches `stdout`
/// from block- to line-buffered. The pty layer also applies `ONLCR`, so both
/// implementations are expected to yield `\r\n`; what matters is that they yield
/// the *same* thing.
#[test]
fn b5_stdout_tty() {
    let result = assert_same_exe("stdout=tty (pty)", |exe| {
        let master = unsafe { posix_openpt(O_RDWR | O_NOCTTY) };
        if master < 0 {
            return None; // no /dev/ptmx in this environment
        }
        unsafe {
            assert_eq!(grantpt(master), 0, "grantpt");
            assert_eq!(unlockpt(master), 0, "unlockpt");
        }
        let name = unsafe { CStr::from_ptr(ptsname(master)) }
            .to_str()
            .expect("ptsname")
            .to_owned();
        let slave = open_fd(&name, O_RDWR | O_NOCTTY, 0);
        assert!(slave >= 0, "open pty slave {name}");

        let mut child = Command::new(exe)
            .stdin(Stdio::null())
            .stdout(stdio_from(slave))
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");

        // Read from the master before reaping, so the pty buffer is still alive.
        let mut buf = Vec::new();
        let mut scratch = [0u8; 256];
        while buf.len() < EXPECTED.len() {
            let n = unsafe {
                read(
                    master,
                    scratch.as_mut_ptr() as *mut std::ffi::c_void,
                    scratch.len(),
                )
            };
            if n <= 0 {
                break;
            }
            buf.extend_from_slice(&scratch[..n as usize]);
        }
        let st = child.wait().expect("wait");
        let mut err = Vec::new();
        if let Some(mut s) = child.stderr.take() {
            let _ = s.read_to_end(&mut err);
        }
        unsafe { close(master) };
        Some((buf, Outcome::from_status(st, Vec::new(), err)))
    });

    match result {
        None => eprintln!("SKIP b5_stdout_tty: no pty available"),
        Some((buf, o)) => {
            assert_eq!(o.code, Some(0), "tty run exit code");
            assert!(
                buf == EXPECTED || buf == b"Hello World!\r\n",
                "unexpected tty bytes: {:?}",
                String::from_utf8_lossy(&buf)
            );
        }
    }
}

/// C6 — stdout is a pipe that is only drained *after* the child has exited.
/// This is the ordering in which a block-buffered C `stdout` flushes at exit.
#[test]
fn b6_pipe_read_after_exit() {
    let (bytes, o) = assert_same_exe("pipe drained after exit", |exe| {
        let mut child = Command::new(exe)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        let st = child.wait().expect("wait");
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .unwrap()
            .read_to_end(&mut bytes)
            .expect("read stdout");
        let mut err = Vec::new();
        child
            .stderr
            .take()
            .unwrap()
            .read_to_end(&mut err)
            .expect("read stderr");
        (bytes, Outcome::from_status(st, Vec::new(), err))
    });
    assert_eq!(bytes, EXPECTED);
    assert_eq!(o.code, Some(0));
    assert!(o.stderr.is_empty());
}

/// C7 — randomized argv: `int main()` takes no parameters, so no argument
/// vector may change the output.
#[test]
fn b7_random_argv() {
    let mut rng = Rng::new();
    for _ in 0..24 {
        let n = rng.below(64);
        let args: Vec<Vec<u8>> = (0..n)
            .map(|_| {
                let len = rng.below(40);
                rng.arg_bytes(len)
            })
            .collect();
        let a = args.clone();
        let o = assert_same_exe(&format!("{n} random argv entries"), |exe| {
            let mut cmd = Command::new(exe);
            for arg in &a {
                cmd.arg(std::ffi::OsStr::from_bytes(arg));
            }
            Outcome::from_output(cmd.output().expect("spawn"))
        });
        assert_eq!(o.stdout, EXPECTED, "argv must not affect stdout (n={n})");
        assert_eq!(o.code, Some(0));
        assert!(o.stderr.is_empty());
    }
}

/// C8 — randomized environment, including a completely empty one.
#[test]
fn b8_random_env() {
    let mut rng = Rng::new();
    for i in 0..24 {
        let n = rng.below(64);
        let vars: Vec<(Vec<u8>, Vec<u8>)> = (0..n)
            .map(|_| {
                let klen = rng.range(1, 20);
                let k = rng.arg_bytes(klen);
                let vlen = rng.below(40);
                let v = rng.arg_bytes(vlen);
                (k, v)
            })
            .collect();
        let cleared = i % 2 == 0;
        let vs = vars.clone();
        let o = assert_same_exe(&format!("{n} random env vars, cleared={cleared}"), |exe| {
            let mut cmd = Command::new(exe);
            if cleared {
                cmd.env_clear();
            }
            for (k, v) in &vs {
                cmd.env(
                    std::ffi::OsStr::from_bytes(k),
                    std::ffi::OsStr::from_bytes(v),
                );
            }
            Outcome::from_output(cmd.output().expect("spawn"))
        });
        assert_eq!(o.stdout, EXPECTED, "env must not affect stdout");
        assert_eq!(o.code, Some(0));
        assert!(o.stderr.is_empty());
    }
}

/// C10 — locale matrix. `printf` of a plain-ASCII literal must be
/// locale-independent; `tr_TR` is included because it has non-ASCII case rules.
#[test]
fn b9_locale_matrix() {
    for loc in [
        "C",
        "POSIX",
        "en_US.UTF-8",
        "tr_TR.UTF-8",
        "de_DE.ISO-8859-1",
        "not_a_locale.XYZ",
        "",
    ] {
        let o = assert_same_exe(&format!("LC_ALL={loc}"), |exe| {
            Outcome::from_output(
                Command::new(exe)
                    .env("LC_ALL", loc)
                    .env("LANG", loc)
                    .output()
                    .expect("spawn"),
            )
        });
        assert_eq!(o.stdout, EXPECTED, "locale {loc} changed the output");
        assert_eq!(o.code, Some(0));
    }
}

/// C11 — stdin is a regular file shared with us. The C program never reads
/// stdin, so the file offset must still be 0 afterwards.
#[test]
fn b10_stdin_offset_preserved() {
    let mut rng = Rng::new();
    for _ in 0..8 {
        let dlen = rng.range(1, 4096);
        let data = rng.bytes(dlen);
        let d = data.clone();
        let (offset, o) = assert_same_exe("stdin=file, offset must be untouched", |exe| {
            let path = tmp_path("stdin");
            std::fs::write(&path, &d).expect("write stdin file");
            let f = std::fs::File::open(&path).expect("open stdin file");
            // dup() so the child shares this file description: the seek offset
            // is then shared, and we can observe whether the child read it.
            let shared = unsafe { dup(f.as_raw_fd()) };
            assert!(shared >= 0);
            let out = Command::new(exe)
                .stdin(stdio_from(shared))
                .output()
                .expect("spawn");
            let off = unsafe { lseek(f.as_raw_fd(), 0, 1 /* SEEK_CUR */) };
            let _ = std::fs::remove_file(&path);
            (off, Outcome::from_output(out))
        });
        assert_eq!(
            offset, 0,
            "stdin offset moved: the program must not read stdin"
        );
        assert_eq!(o.stdout, EXPECTED);
        assert_eq!(o.code, Some(0));
    }
}

/// C12 — the other stdin shapes: a pipe holding unread data, `/dev/null`, and a
/// closed descriptor (row E8).
#[test]
fn b11_stdin_kinds() {
    let mut rng = Rng::new();
    let payload = rng.bytes(64);

    // (a) pipe with unread data
    let p = payload.clone();
    let o = assert_same_exe("stdin=pipe with unread data", |exe| {
        let (rfd, wfd) = make_pipe();
        let mut w = unsafe { std::fs::File::from(std::os::fd::OwnedFd::from_raw_fd(wfd)) };
        w.write_all(&p).expect("prime pipe");
        drop(w);
        Outcome::from_output(
            Command::new(exe)
                .stdin(stdio_from(rfd))
                .output()
                .expect("spawn"),
        )
    });
    assert_eq!(o.stdout, EXPECTED);
    assert_eq!(o.code, Some(0));

    // (b) /dev/null
    let o = assert_same_exe("stdin=/dev/null", |exe| {
        Outcome::from_output(
            Command::new(exe)
                .stdin(Stdio::null())
                .output()
                .expect("spawn"),
        )
    });
    assert_eq!(o.stdout, EXPECTED);
    assert_eq!(o.code, Some(0));
}

/// C13 — many concurrent invocations must all behave identically.
#[test]
fn b12_concurrent() {
    let all = assert_same_exe("32 concurrent invocations", |exe| {
        let children: Vec<_> = (0..32)
            .map(|_| {
                Command::new(exe)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("spawn")
            })
            .collect();
        children
            .into_iter()
            .map(|c| Outcome::from_output(c.wait_with_output().expect("wait")))
            .collect::<Vec<_>>()
    });
    assert_eq!(all.len(), 32);
    for (i, o) in all.iter().enumerate() {
        assert_eq!(o.stdout, EXPECTED, "concurrent child {i}");
        assert_eq!(o.code, Some(0), "concurrent child {i}");
        assert!(o.stderr.is_empty(), "concurrent child {i}");
    }
}

/// C14 — determinism across many sequential invocations.
#[test]
fn b13_repeat_determinism() {
    let mut seen = None;
    for i in 0..100 {
        let o = assert_same_exe(&format!("sequential run {i}"), plain_run);
        assert_eq!(o.stdout, EXPECTED);
        assert_eq!(o.code, Some(0));
        match &seen {
            None => seen = Some(o),
            Some(first) => assert_eq!(first, &o, "run {i} differs from run 0"),
        }
    }
}

// ===========================================================================
// Phase C — error paths (ERRORS.md)
// ===========================================================================

/// E1 — stdout is a pipe with **no reader at all**, so the write cannot succeed.
///
/// The C program runs with `SIGPIPE` at its default disposition and is therefore
/// *killed by signal 13* (wait status 141) rather than exiting. Deterministic:
/// the read end is closed before the child is even spawned, so there is no race.
///
/// This row caught a real translation bug — Rust's runtime sets `SIGPIPE` to
/// `SIG_IGN` before `main`, which made the Rust program exit 0 here.
#[test]
fn e1_sigpipe_killed() {
    for i in 0..5 {
        let o = assert_same_exe(&format!("stdout=pipe with no reader (iter {i})"), |exe| {
            let (rfd, wfd) = make_pipe();
            // No reader exists from this moment on.
            unsafe { close(rfd) };
            Outcome::from_output(
                Command::new(exe)
                    .stdout(stdio_from(wfd))
                    .stderr(Stdio::piped())
                    .output()
                    .expect("spawn"),
            )
        });
        assert_eq!(
            o.signal,
            Some(13),
            "must be killed by SIGPIPE exactly like the C program, got {o:?}"
        );
        assert_eq!(o.code, None, "a signalled process has no exit code");
        assert!(o.stderr.is_empty(), "nothing may be written to stderr");
    }
}

/// E3 — stdout is `/dev/full`: every write fails with `ENOSPC`. The C code
/// discards `printf`'s return value, so it still exits 0.
#[test]
fn e3_dev_full() {
    let o = assert_same_exe("stdout=/dev/full (ENOSPC)", |exe| {
        let fd = open_fd("/dev/full", O_WRONLY, 0);
        assert!(fd >= 0, "open /dev/full failed");
        Outcome::from_output(
            Command::new(exe)
                .stdout(stdio_from(fd))
                .stderr(Stdio::piped())
                .output()
                .expect("spawn"),
        )
    });
    assert_eq!(o.code, Some(0), "write errors are discarded -> exit 0");
    assert_eq!(o.signal, None);
    assert!(o.stderr.is_empty(), "no diagnostic may be printed");
}

/// E4 — file descriptor 1 is closed before entry: the write fails with `EBADF`
/// and is discarded.
#[test]
fn e4_closed_stdout() {
    let o = assert_same_exe("fd 1 closed (EBADF)", |exe| {
        let mut cmd = Command::new(exe);
        cmd.stdout(Stdio::null()).stderr(Stdio::piped());
        unsafe {
            cmd.pre_exec(|| {
                close(1);
                Ok(())
            });
        }
        Outcome::from_output(cmd.output().expect("spawn"))
    });
    assert_eq!(o.code, Some(0));
    assert_eq!(o.signal, None);
    assert!(o.stderr.is_empty());
}

/// E5 — both fd 1 and fd 2 are closed; only the exit status is observable.
#[test]
fn e5_closed_stdout_and_stderr() {
    let o = assert_same_exe("fd 1 and fd 2 both closed", |exe| {
        let mut cmd = Command::new(exe);
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        unsafe {
            cmd.pre_exec(|| {
                close(1);
                close(2);
                Ok(())
            });
        }
        Outcome::from_output(cmd.output().expect("spawn"))
    });
    assert_eq!(o.code, Some(0));
    assert_eq!(o.signal, None);
}

/// E6 — stdout is a **read-only** descriptor: `write` fails with `EBADF`, and
/// the target file must stay empty.
#[test]
fn e6_readonly_stdout() {
    let (len, o) = assert_same_exe("stdout=read-only fd", |exe| {
        let path = tmp_path("ro");
        std::fs::write(&path, b"").expect("create");
        let fd = open_fd(path.to_str().unwrap(), O_RDONLY, 0);
        assert!(fd >= 0, "open read-only failed");
        let out = Command::new(exe)
            .stdout(stdio_from(fd))
            .stderr(Stdio::piped())
            .output()
            .expect("spawn");
        let len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(u64::MAX);
        let _ = std::fs::remove_file(&path);
        (len, Outcome::from_output(out))
    });
    assert_eq!(len, 0, "nothing may reach a read-only fd");
    assert_eq!(o.code, Some(0));
    assert!(o.stderr.is_empty());
}

/// E7 — stdout is a **directory** descriptor: `write` fails (`EBADF`/`EISDIR`).
#[test]
fn e7_directory_stdout() {
    let o = assert_same_exe("stdout=directory fd", |exe| {
        let dir = std::env::temp_dir();
        let fd = open_fd(dir.to_str().unwrap(), O_RDONLY, 0);
        assert!(fd >= 0, "open directory failed");
        Outcome::from_output(
            Command::new(exe)
                .stdout(stdio_from(fd))
                .stderr(Stdio::piped())
                .output()
                .expect("spawn"),
        )
    });
    assert_eq!(o.code, Some(0));
    assert_eq!(o.signal, None);
    assert!(o.stderr.is_empty());
}

/// E8 — fd 0 closed. `main` never reads stdin, so this must NOT be an error:
/// full output, exit 0.
#[test]
fn e8_closed_stdin() {
    let o = assert_same_exe("fd 0 closed", |exe| {
        let mut cmd = Command::new(exe);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        unsafe {
            cmd.pre_exec(|| {
                close(0);
                Ok(())
            });
        }
        Outcome::from_output(cmd.output().expect("spawn"))
    });
    assert_eq!(o.stdout, EXPECTED, "closed stdin must not affect output");
    assert_eq!(o.code, Some(0));
    assert!(o.stderr.is_empty());
}

// ===========================================================================
// Generic boundaries (ERRORS.md "generic boundaries" section)
// ===========================================================================

/// g2 — the "zero / oversized length" analogue for `int main(void)`: argv counts
/// of 0, 1 and 4096, and one ~100 KiB argument.
#[test]
fn g2_argv_counts_and_huge_arg() {
    for n in [0usize, 1, 4096] {
        let o = assert_same_exe(&format!("argv count {n}"), |exe| {
            let mut cmd = Command::new(exe);
            for i in 0..n {
                cmd.arg(format!("a{i}"));
            }
            Outcome::from_output(cmd.output().expect("spawn"))
        });
        assert_eq!(o.stdout, EXPECTED, "argv count {n}");
        assert_eq!(o.code, Some(0));
    }

    // One very large argument (kept under MAX_ARG_STRLEN = 128 KiB).
    let big = "z".repeat(100_000);
    let o = assert_same_exe("single 100 KiB argument", |exe| {
        Outcome::from_output(Command::new(exe).arg(&big).output().expect("spawn"))
    });
    assert_eq!(o.stdout, EXPECTED);
    assert_eq!(o.code, Some(0));
}

/// g3 / C9 — `argv[0]` edge cases and environment-size edge cases.
#[test]
fn g3_arg0_and_env_edges() {
    // argv[0] = ""
    let o = assert_same_exe("argv[0] empty", |exe| {
        Outcome::from_output(Command::new(exe).arg0("").output().expect("spawn"))
    });
    assert_eq!(o.stdout, EXPECTED);
    assert_eq!(o.code, Some(0));

    // argv[0] = invalid UTF-8
    let o = assert_same_exe("argv[0] non-UTF-8", |exe| {
        Outcome::from_output(
            Command::new(exe)
                .arg0(std::ffi::OsStr::from_bytes(b"\xff\xfe\x80bad"))
                .output()
                .expect("spawn"),
        )
    });
    assert_eq!(o.stdout, EXPECTED);
    assert_eq!(o.code, Some(0));

    // Completely empty environment.
    let o = assert_same_exe("empty environment", |exe| {
        Outcome::from_output(Command::new(exe).env_clear().output().expect("spawn"))
    });
    assert_eq!(o.stdout, EXPECTED);
    assert_eq!(o.code, Some(0));

    // Oversized environment (~256 KiB spread over many variables).
    let o = assert_same_exe("oversized environment", |exe| {
        let mut cmd = Command::new(exe);
        let val = "v".repeat(4096);
        for i in 0..64 {
            cmd.env(format!("BIG_VAR_{i}"), &val);
        }
        Outcome::from_output(cmd.output().expect("spawn"))
    });
    assert_eq!(o.stdout, EXPECTED);
    assert_eq!(o.code, Some(0));

    // Non-UTF-8 environment value (no NUL: the kernel's envp is NUL-terminated,
    // so a NUL byte is rejected before either program ever runs).
    let o = assert_same_exe("non-UTF-8 env value", |exe| {
        Outcome::from_output(
            Command::new(exe)
                .env("WEIRD", std::ffi::OsStr::from_bytes(b"\xff\xfe\x80bad"))
                .output()
                .expect("spawn"),
        )
    });
    assert_eq!(o.stdout, EXPECTED);
    assert_eq!(o.code, Some(0));
}
