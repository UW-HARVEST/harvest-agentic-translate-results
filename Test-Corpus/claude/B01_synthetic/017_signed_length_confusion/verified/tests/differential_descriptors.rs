//! Differential tests for behavior visible through the *descriptors* the
//! program is handed, rather than through its stdout bytes:
//!
//! * what happens when writing to stdout fails (no pipe reader → `SIGPIPE`,
//!   closed descriptor → `EBADF`, full device → `ENOSPC`);
//! * where a shared stdin descriptor's file offset is left when the program
//!   exits, and how much of a pipe it consumes.
//!
//! Both are real, externally observable behaviors of the C program that plain
//! stdout comparison cannot see, and both are places a Rust translation
//! naturally diverges (Rust's runtime ignores `SIGPIPE`, and `StdinLock` uses
//! an 8 KiB buffer and never rewinds).
//!
//! Covers `CONFIGS.md` rows C29–C33 and `ERRORS.md` rows E15–E18.

mod common;

use common::{c_exe, rust_exe, Diff, Outcome};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::{FromRawFd, OwnedFd};
use std::path::Path;
use std::process::{Command, Stdio};

extern "C" {
    fn pipe2(fds: *mut i32, flags: i32) -> i32;
    fn close(fd: i32) -> i32;
}

/// `O_CLOEXEC` is essential here: these tests run on several threads at once,
/// and a plain `pipe(2)` would leak its ends into children spawned by *other*
/// threads between `pipe` and `drop(r)`. An unrelated child holding the read
/// end keeps the pipe readable, so the expected `SIGPIPE` never arrives and the
/// test flakes. `Stdio::from` still works, because the descriptor is `dup2`ed
/// onto the child's stdio (which clears `FD_CLOEXEC` on the copy).
fn make_pipe() -> (OwnedFd, OwnedFd) {
    const O_CLOEXEC: i32 = 0o2_000_000;
    let mut fds = [0i32; 2];
    assert_eq!(
        unsafe { pipe2(fds.as_mut_ptr(), O_CLOEXEC) },
        0,
        "pipe2(2) failed"
    );
    unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) }
}

fn outcome(status: std::process::ExitStatus, stdout: Vec<u8>, stderr: Vec<u8>) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    Outcome {
        stdout,
        stderr,
        code: status.code(),
        signal: status.signal(),
    }
}

// ---------------------------------------------------------------------------
// Failing stdout
// ---------------------------------------------------------------------------

/// Run `exe` with stdout on a pipe that has **no reader at all**, so the
/// exit-time flush cannot succeed.
fn run_no_pipe_reader(exe: &Path, input: &[u8]) -> Outcome {
    let (r, w) = make_pipe();
    drop(r); // close the read end before the child can write
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(w))
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let _ = child.stdin.take().unwrap().write_all(input);
    let out = child.wait_with_output().expect("wait");
    outcome(out.status, Vec::new(), out.stderr)
}

/// C29/E15: a write to a pipe with no reader raises `SIGPIPE`. A C program
/// inherits the default disposition and dies (status 141); Rust's runtime sets
/// `SIG_IGN` before `main`, so the translation must restore `SIG_DFL`.
#[test]
fn c29_sigpipe_no_reader() {
    let mut d = Diff::new("C29,E15 SIGPIPE (stdout pipe with no reader)");
    for input in [
        &b"50\n"[..],
        &b"0\n"[..],
        &b"99\n"[..],
        &b"100\n"[..],
        &b"abc\n"[..],
        &b"1\n"[..],
    ] {
        let c = run_no_pipe_reader(&c_exe(), input);
        let r = run_no_pipe_reader(&rust_exe(), input);
        d.check(&format!("no-reader stdin={:?}", String::from_utf8_lossy(input)), &c, &r);
    }
    // Sanity: the C side really is dying from SIGPIPE, not exiting cleanly.
    let c = run_no_pipe_reader(&c_exe(), b"50\n");
    assert_eq!(
        c.signal,
        Some(13),
        "expected the C program to die from SIGPIPE, got {}",
        c.describe()
    );
    d.finish();
}

/// C30/E16: descriptor 1 closed outright — the write fails with `EBADF`, which
/// the C code never checks, so the program still exits 0.
#[test]
fn c30_stdout_closed() {
    let mut d = Diff::new("C30,E16 stdout closed (EBADF)");
    let run = |exe: &Path, input: &[u8]| -> Outcome {
        let mut cmd = Command::new(exe);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        unsafe {
            use std::os::unix::process::CommandExt;
            cmd.pre_exec(|| {
                close(1);
                Ok(())
            });
        }
        let mut child = cmd.spawn().expect("spawn");
        let _ = child.stdin.take().unwrap().write_all(input);
        let out = child.wait_with_output().expect("wait");
        outcome(out.status, Vec::new(), out.stderr)
    };
    for input in [&b"50\n"[..], &b"0\n"[..], &b"100\n"[..], &b""[..]] {
        let c = run(&c_exe(), input);
        let r = run(&rust_exe(), input);
        d.check(
            &format!("closed-stdout stdin={:?}", String::from_utf8_lossy(input)),
            &c,
            &r,
        );
    }
    d.finish();
}

/// C31/E17: `/dev/full` — every write fails with `ENOSPC`, again unchecked.
#[test]
fn c31_stdout_dev_full() {
    if !Path::new("/dev/full").exists() {
        eprintln!("skipping C31: /dev/full not present");
        return;
    }
    let mut d = Diff::new("C31,E17 stdout on /dev/full (ENOSPC)");
    let run = |exe: &Path, input: &[u8]| -> Outcome {
        let f = File::options()
            .write(true)
            .open("/dev/full")
            .expect("open /dev/full");
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(f))
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        let _ = child.stdin.take().unwrap().write_all(input);
        let out = child.wait_with_output().expect("wait");
        outcome(out.status, Vec::new(), out.stderr)
    };
    for input in [&b"50\n"[..], &b"99\n"[..], &b"0\n"[..], &b""[..]] {
        let c = run(&c_exe(), input);
        let r = run(&rust_exe(), input);
        d.check(
            &format!("/dev/full stdin={:?}", String::from_utf8_lossy(input)),
            &c,
            &r,
        );
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Where stdin is left
// ---------------------------------------------------------------------------

/// Result of running with a shared, seekable stdin.
#[derive(Debug, PartialEq, Eq)]
struct SeekResult {
    outcome: Outcome,
    offset: u64,
}

fn run_with_file_stdin(exe: &Path, contents: &[u8], start_at: u64, tag: &str) -> SeekResult {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("c_build");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("desc_stdin_{}_{tag}.bin", std::process::id()));
    std::fs::write(&path, contents).unwrap();

    let mut f = File::open(&path).unwrap();
    if start_at > 0 {
        f.seek(SeekFrom::Start(start_at)).unwrap();
    }
    // The child gets a dup of this descriptor, so they share one file offset.
    let child_end = f.try_clone().unwrap();
    let out = Command::new(exe)
        .stdin(Stdio::from(child_end))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn");
    let offset = f.stream_position().unwrap();
    drop(f);
    let _ = std::fs::remove_file(&path);
    SeekResult {
        outcome: outcome(out.status, out.stdout, out.stderr),
        offset,
    }
}

/// C32/E18: glibc buffers a whole `st_blksize` block, then `exit`'s stream
/// cleanup rewinds a seekable stdin to the stream's *logical* position — so a
/// process sharing the descriptor sees only the bytes actually consumed. When
/// the program dies from a signal instead, that rewind never happens and the
/// raw block offset remains.
#[test]
fn c32_seekable_stdin_offset() {
    let mut d = Diff::new("C32,E18 seekable stdin offset at exit");
    let filler: Vec<u8> = (0..4000u32)
        .flat_map(|i| format!("line{i:05}\n").into_bytes())
        .collect();

    let mut cases: Vec<(String, Vec<u8>, u64)> = Vec::new();
    for first in ["50\n", "0\n", "100\n", "-1\n", "abc\n", "7", "\n"] {
        // Long file: more than one block, so buffering is observable.
        let mut big = first.as_bytes().to_vec();
        big.extend_from_slice(&filler);
        cases.push((format!("long file, first line {first:?}"), big, 0));
        // Short file: less than one block.
        cases.push((
            format!("short file, first line {first:?}"),
            first.as_bytes().to_vec(),
            0,
        ));
    }
    // Empty file: fgets fails, then the program faults.
    cases.push(("empty file".to_string(), Vec::new(), 0));
    // A non-zero starting offset must be honored: the rewind is to
    // start + consumed, not to consumed.
    let mut pre = b"XXXXX50\n".to_vec();
    pre.extend_from_slice(&filler);
    cases.push(("long file, pre-seeked to 5".to_string(), pre, 5));
    let mut pre2 = b"ZZ-1\n".to_vec();
    pre2.extend_from_slice(&filler);
    cases.push(("long file, pre-seeked to 2, faults".to_string(), pre2, 2));

    for (i, (label, contents, start)) in cases.iter().enumerate() {
        let c = run_with_file_stdin(&c_exe(), contents, *start, &format!("c{i}"));
        let r = run_with_file_stdin(&rust_exe(), contents, *start, &format!("r{i}"));
        d.check(
            &format!("{label} [C offset={} Rust offset={}]", c.offset, r.offset),
            &c.outcome,
            &r.outcome,
        );
        assert_eq!(
            c.offset, r.offset,
            "stdin offset diverged for {label}: C left {} but Rust left {}",
            c.offset, r.offset
        );
    }

    // Sanity: the behavior being pinned is real — a long file read to
    // completion must rewind to the logical position, not stay at the block.
    let mut big = b"50\n".to_vec();
    big.extend_from_slice(&filler);
    let c = run_with_file_stdin(&c_exe(), &big, 0, "sanity");
    assert_eq!(c.offset, 3, "C should rewind a seekable stdin to 3 bytes");
    d.finish();
}

/// C33: a **pipe** cannot be rewound, so glibc's block read really does consume
/// `st_blksize` bytes of it; anything reading the pipe afterwards sees the rest.
#[test]
fn c33_pipe_stdin_consumption() {
    let mut d = Diff::new("C33 pipe stdin consumption");

    let run = |exe: &Path, payload: &[u8]| -> (Outcome, usize) {
        let (r, w) = make_pipe();
        // Payload must fit the pipe buffer so the write cannot block.
        assert!(payload.len() < 60_000);
        let mut wf = File::from(w);
        wf.write_all(payload).unwrap();
        drop(wf);

        let child_end = r.try_clone().unwrap();
        let out = Command::new(exe)
            .stdin(Stdio::from(child_end))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("spawn");

        let mut rest = Vec::new();
        File::from(r).read_to_end(&mut rest).unwrap();
        (
            outcome(out.status, out.stdout, out.stderr),
            payload.len() - rest.len(),
        )
    };

    let filler: Vec<u8> = (0..3000u32)
        .flat_map(|i| format!("line{i:05}\n").into_bytes())
        .collect();
    for first in ["50\n", "0\n", "-1\n", "100\n", "abc\n"] {
        let mut payload = first.as_bytes().to_vec();
        payload.extend_from_slice(&filler);
        let (co, cn) = run(&c_exe(), &payload);
        let (ro, rn) = run(&rust_exe(), &payload);
        d.check(
            &format!("pipe first={first:?} [C consumed={cn} Rust consumed={rn}]"),
            &co,
            &ro,
        );
        assert_eq!(
            cn, rn,
            "pipe consumption diverged for {first:?}: C consumed {cn}, Rust consumed {rn}"
        );
    }
    // Short payload: everything is consumed, nothing is left over.
    for first in ["50\n", "-1\n"] {
        let (co, cn) = run(&c_exe(), first.as_bytes());
        let (ro, rn) = run(&rust_exe(), first.as_bytes());
        d.check(&format!("short pipe {first:?} [consumed {cn}/{rn}]"), &co, &ro);
        assert_eq!(cn, rn);
    }
    d.finish();
}
