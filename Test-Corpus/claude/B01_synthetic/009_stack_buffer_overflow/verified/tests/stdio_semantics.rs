//! Differential tests for the C-stdio behaviors that a `std::io`-based
//! translation gets wrong.
//!
//! Every test here corresponds to a divergence that was found and fixed. They are
//! separated out because they need special setups (a pty, a closed pipe, a
//! seekable file, a C consumer of the shared object) rather than the plain
//! stdin/stdout pipe the other suites use.

mod common;

use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::Path;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// SIGPIPE disposition
// ---------------------------------------------------------------------------

/// A C program inherits SIGPIPE at its default disposition, so writing to a
/// stdout whose reader is gone kills it with signal 13. Rust's runtime sets
/// SIGPIPE to `SIG_IGN` before `main`, which would silently swallow the write and
/// exit 0 instead.
#[test]
fn sigpipe_kills_both_programs() {
    common::ensure_built();
    for _ in 0..6 {
        let mut statuses = Vec::new();
        for exe in [common::c_exe(), common::rust_exe()] {
            // Create a pipe and immediately close the read end.
            let mut fds = [0i32; 2];
            assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe()");
            let (r, w) = (fds[0], fds[1]);
            unsafe { libc::close(r) };
            let wfile = unsafe { std::fs::File::from_raw_fd(w) };

            let mut child = Command::new(&exe)
                .stdin(Stdio::piped())
                .stdout(Stdio::from(wfile))
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn");
            {
                let si = child.stdin.as_mut().unwrap();
                let _ = si.write_all(b"7\n3\n");
            }
            let st = child.wait().expect("wait");
            use std::os::unix::process::ExitStatusExt;
            statuses.push((st.code(), st.signal()));
        }
        assert_eq!(
            statuses[0], statuses[1],
            "SIGPIPE handling diverged: C={:?} Rust={:?}",
            statuses[0], statuses[1]
        );
        assert_eq!(
            statuses[0].1,
            Some(libc::SIGPIPE),
            "expected C to die from SIGPIPE, got {:?}",
            statuses[0]
        );
    }
}

// ---------------------------------------------------------------------------
// Terminal vs. pipe buffering
// ---------------------------------------------------------------------------

/// Runs `exe` with stdout on a pseudo-terminal and returns
/// `(signal_or_code, bytes_written)`.
fn run_on_pty(exe: &Path, stdin: &[u8]) -> (Option<i32>, Option<i32>, usize) {
    let mut primary = 0i32;
    let mut replica = 0i32;
    assert_eq!(
        unsafe {
            libc::openpty(
                &mut primary,
                &mut replica,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
            )
        },
        0,
        "openpty()"
    );
    let replica_file = unsafe { std::fs::File::from_raw_fd(replica) };
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(replica_file))
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    {
        let si = child.stdin.as_mut().unwrap();
        let _ = si.write_all(stdin);
    }
    let st = child.wait().expect("wait");

    // Drain whatever reached the terminal, without blocking.
    let flags = unsafe { libc::fcntl(primary, libc::F_GETFL) };
    unsafe { libc::fcntl(primary, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    let mut total = 0usize;
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { libc::read(primary, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        total += n as usize;
    }
    unsafe { libc::close(primary) };
    use std::os::unix::process::ExitStatusExt;
    (st.code(), st.signal(), total)
}

/// On a terminal C line-buffers stdout, so output written before the
/// out-of-bounds write kills the process *survives*. How much survives depends on
/// which return address was clobbered, which makes this a sharp test of the crash
/// *timing*, not just of its occurrence:
///
/// * index 16/17 (bad's saved rbp) and 26/27 (main's return address) — the fault
///   lands as `main` returns, so the whole 167-byte transcript is out.
/// * index 18/19 (bad's own return address) — the fault lands at `bad`'s `ret`,
///   after the ten values but before `"Finished bad()"`: 151 bytes.
/// * a far index — the store itself faults before the ten values: 121 bytes.
#[test]
fn tty_line_buffering_and_crash_timing_match() {
    common::ensure_built();
    for k in [5i64, 9, 10, 15, 16, 17, 18, 19, 20, 26, 27, 1_000_000] {
        let stdin = format!("-1\n{k}\n").into_bytes();
        let c = run_on_pty(&common::c_exe(), &stdin);
        let r = run_on_pty(&common::rust_exe(), &stdin);
        assert_eq!(
            c, r,
            "tty behavior diverged at index {k}: C=(code,sig,bytes)={c:?} Rust={r:?}"
        );
    }
}

/// The same indices produce *no* output when stdout is a pipe, because the block
/// buffer dies with the process. Both sides must agree on that too, which is what
/// makes the pair of tests meaningful: the difference between them is exactly
/// C's buffering-mode switch.
#[test]
fn pipe_buffering_discards_output_on_crash() {
    for k in [16i64, 17, 18, 19, 26, 27] {
        let stdin = format!("-1\n{k}\n").into_bytes();
        let (c, r) = common::exe::both(&stdin);
        assert_eq!(c, r, "pipe behavior diverged at index {k}");
        assert!(
            c.stdout.is_empty(),
            "on a pipe the buffered output must be lost at index {k}, got {c:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Seekable stdin
// ---------------------------------------------------------------------------

/// glibc repositions a seekable input stream to the logical read offset when the
/// process exits, so a later reader of the same descriptor sees the unread
/// remainder. Rust's `io::Stdin` would have consumed the whole file into its own
/// buffer and left the descriptor at EOF.
#[test]
fn seekable_stdin_is_left_at_the_logical_offset() {
    common::ensure_built();
    let dir = std::env::temp_dir();
    let path = dir.join(format!("stdin-seek-{}.txt", std::process::id()));
    let content = b"7\nrest-line-1\nAAA\nBBB\nCCC\n";
    std::fs::write(&path, content).expect("write temp input");

    let mut offsets = Vec::new();
    let mut tails = Vec::new();
    for exe in [common::c_exe(), common::rust_exe()] {
        let f = std::fs::File::open(&path).expect("open");
        let fd = f.as_raw_fd();
        let st = Command::new(&exe)
            .stdin(unsafe { Stdio::from_raw_fd(libc::dup(fd)) })
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("run");
        assert!(st.success(), "{} did not exit cleanly", exe.display());
        // The child had a dup of our fd, so the shared offset is observable here
        // only if we re-open; instead read the remainder through a fresh handle
        // positioned where the child left the *shared* description.
        drop(f);

        // Repeat, this time sharing the descriptor with a shell so the offset is
        // directly visible to a following reader.
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "{} >/dev/null 2>&1; cat",
                exe.to_str().unwrap()
            ))
            .stdin(Stdio::from(std::fs::File::open(&path).expect("open")))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("run sh");
        tails.push(out.stdout);

        // And measure the raw offset the implementation leaves behind.
        let mut f2 = std::fs::File::open(&path).expect("open");
        let child = Command::new(&exe)
            .stdin(unsafe { Stdio::from_raw_fd(libc::dup(f2.as_raw_fd())) })
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        let _ = child.wait_with_output();
        let _ = f2.seek(SeekFrom::Current(0));
        let mut rest = Vec::new();
        let _ = f2.read_to_end(&mut rest);
        offsets.push(rest.len());
    }

    let _ = std::fs::remove_file(&path);
    assert_eq!(
        tails[0], tails[1],
        "the remainder left on a shared stdin diverged:\n  C    = {:?}\n  Rust = {:?}",
        String::from_utf8_lossy(&tails[0]),
        String::from_utf8_lossy(&tails[1])
    );
    assert_eq!(
        tails[0], b"AAA\nBBB\nCCC\n",
        "C should leave exactly the three unread lines, got {:?}",
        String::from_utf8_lossy(&tails[0])
    );
}

// ---------------------------------------------------------------------------
// Shared FILE state with a C consumer of the shared object
// ---------------------------------------------------------------------------

/// Compiles the C consumer used by the shared-object tests, once.
fn consumer() -> std::path::PathBuf {
    use std::sync::OnceLock;
    static P: OnceLock<std::path::PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        common::ensure_built();
        let dir = std::env::temp_dir().join("driver-consumer");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let src = dir.join("consumer.c");
        std::fs::write(
            &src,
            br#"
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
int main(int argc, char**argv){
  void*h=dlopen(argv[1], RTLD_NOW); if(!h){fprintf(stderr,"%s\n",dlerror());return 99;}
  const char* mode=argv[2];
  void(*pl)(const char*)=dlsym(h,"printLine");
  void(*pil)(int)=dlsym(h,"printIntLine");
  void(*bad_)(void)=dlsym(h,"bad");
  void(*good_)(void)=dlsym(h,"good");
  if(!strcmp(mode,"interleave")){
    printf("A-from-caller\n"); pl("B-from-lib");
    printf("C-from-caller\n"); pil(42); printf("D-from-caller\n");
  } else if(!strcmp(mode,"sharedstdin")){
    char b[8]; if(fgets(b,8,stdin)){ b[strcspn(b,"\n")]=0; printf("caller-read=[%s]\n",b); }
    good_(); bad_();
  }
  return 0;
}
"#,
        )
        .expect("write consumer.c");
        let exe = dir.join("consumer");
        let ok = Command::new("gcc")
            .args(["-O0", "-o"])
            .arg(&exe)
            .arg(&src)
            .arg("-ldl")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "failed to build the C consumer");
        exe
    })
    .clone()
}

fn run_consumer(lib: &Path, mode: &str, stdin: &[u8]) -> (Option<i32>, Vec<u8>) {
    let mut child = Command::new(consumer())
        .arg(lib)
        .arg(mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn consumer");
    {
        let si = child.stdin.as_mut().unwrap();
        let _ = si.write_all(stdin);
    }
    let out = child.wait_with_output().expect("wait");
    (out.status.code(), out.stdout)
}

/// A C consumer's own `printf` output shares the process `stdout` buffer with
/// `printLine`, so everything appears in call order. A private `BufWriter` inside
/// the library would reorder it.
#[test]
fn consumer_printf_interleaves_with_printline() {
    let c = run_consumer(&common::c_lib(), "interleave", b"");
    let r = run_consumer(&common::rust_lib(), "interleave", b"");
    assert_eq!(
        c,
        r,
        "output ordering diverged:\n  C    = {:?}\n  Rust = {:?}",
        String::from_utf8_lossy(&c.1),
        String::from_utf8_lossy(&r.1)
    );
    assert_eq!(
        c.1,
        b"A-from-caller\nB-from-lib\nC-from-caller\n42\nD-from-caller\n",
        "unexpected C ordering"
    );
}

/// A C consumer that reads `stdin` itself shares glibc's stream position with
/// `good()` and `bad()`, so all three reads cooperate. Reading through Rust's own
/// `io::Stdin` would make the library see EOF.
#[test]
fn consumer_fgets_shares_stdin_with_the_library() {
    let c = run_consumer(&common::c_lib(), "sharedstdin", b"1\n2\n3\n");
    let r = run_consumer(&common::rust_lib(), "sharedstdin", b"1\n2\n3\n");
    assert_eq!(
        c,
        r,
        "shared stdin diverged:\n  C    = {:?}\n  Rust = {:?}",
        String::from_utf8_lossy(&c.1),
        String::from_utf8_lossy(&r.1)
    );
    let text = String::from_utf8_lossy(&c.1);
    assert!(
        text.starts_with("caller-read=[1]\n"),
        "the consumer should have read the first line, got {text:?}"
    );
    assert!(
        !text.contains("fgets() failed."),
        "the library should have seen lines 2 and 3, got {text:?}"
    );
}

/// The exported `bad()` across every caller-independent index class.
#[test]
fn exported_bad_matches_across_index_classes() {
    common::ensure_built();
    for k in [0i64, 1, 5, 9, 10, 11, 14, 15, 16, 17, 18, 19, 20, 25, 26, 27, 50, 51, 100, 200] {
        let stdin = format!("{k}\n").into_bytes();
        let c = run_consumer(&common::c_lib(), "sharedstdin", &stdin);
        let r = run_consumer(&common::rust_lib(), "sharedstdin", &stdin);
        // `sharedstdin` has the consumer read the line first, so good()/bad() see
        // EOF; that is fine -- the point is that both libraries agree.
        assert_eq!(c, r, "exported bad() diverged at index {k}");
    }
}
