//! End-to-end comparison of the two `main` implementations: the C `driver`
//! executable built from the unmodified `c_src` and the Rust `driver` binary,
//! fed identical stdin (`CONFIGS.md` rows C41-C50, `ERRORS.md` rows E36-E42).
//!
//! This is the same code path as the `main` symbol of the two shared libraries
//! (`src/ffi.rs::main` calls `driver::run`, exactly like `src/main.rs`).

mod common;

use common::*;
use std::io::Write;
use std::process::{Command, Stdio};

fn run_binary(bin: &std::path::Path, stdin: &[u8]) -> (Vec<u8>, Vec<u8>, Option<i32>) {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {}", bin.display(), e));
    {
        let mut si = child.stdin.take().unwrap();
        let data = stdin.to_vec();
        std::thread::spawn(move || {
            let _ = si.write_all(&data);
        });
    }
    let out = child.wait_with_output().expect("wait");
    (out.stdout, out.stderr, out.status.code())
}

/// stdout and stderr captured separately.
fn diff_driver(stdin: &[u8]) -> Vec<u8> {
    ensure_c_artifacts();
    ensure_rust_driver();
    let (co, ce, cs) = run_binary(&c_driver_path(), stdin);
    let (ro, re, rs) = run_binary(&rust_driver_path(), stdin);
    assert_eq!(show(&co), show(&ro), "stdout differs for stdin {}", show(stdin));
    assert_eq!(show(&ce), show(&re), "stderr differs for stdin {}", show(stdin));
    assert_eq!(cs, rs, "exit status differs for stdin {}", show(stdin));
    co
}

/// stdout and stderr merged into one file, which is what a shell `>file 2>&1`
/// does: this also compares the *flush order* of the two streams.
fn diff_driver_merged(stdin: &[u8]) -> Vec<u8> {
    ensure_c_artifacts();
    ensure_rust_driver();

    let run = |bin: &std::path::Path, tag: &str| -> Vec<u8> {
        let path = std::env::temp_dir().join(format!(
            "ta_merged_{}_{}_{}.txt",
            std::process::id(),
            tag,
            CAPTURE_SEQ_PUB.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let file = std::fs::File::create(&path).expect("create merged file");
        let file2 = file.try_clone().expect("clone fd");
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(file))
            .stderr(Stdio::from(file2))
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {}: {}", bin.display(), e));
        {
            let mut si = child.stdin.take().unwrap();
            let data = stdin.to_vec();
            std::thread::spawn(move || {
                let _ = si.write_all(&data);
            });
        }
        let st = child.wait().expect("wait");
        assert_eq!(st.code(), Some(0), "{} exited with {:?}", tag, st);
        let out = std::fs::read(&path).expect("read merged file");
        let _ = std::fs::remove_file(&path);
        out
    };

    let c = run(&c_driver_path(), "c");
    let r = run(&rust_driver_path(), "rust");
    assert_eq!(
        show(&c),
        show(&r),
        "merged stdout+stderr differs for stdin {}",
        show(stdin)
    );
    c
}

static CAPTURE_SEQ_PUB: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// ---------------------------------------------------------------------------
// stdout attached to a terminal: C switches to line buffering, and the
// translation has to do the same (cio::Out checks `is_terminal`)
// ---------------------------------------------------------------------------

extern "C" {
    fn openpty(
        amaster: *mut std::ffi::c_int,
        aslave: *mut std::ffi::c_int,
        name: *mut std::ffi::c_char,
        termp: *const std::ffi::c_void,
        winp: *const std::ffi::c_void,
    ) -> std::ffi::c_int;
    fn read(fd: std::ffi::c_int, buf: *mut std::ffi::c_void, n: usize) -> isize;
    fn close(fd: std::ffi::c_int) -> std::ffi::c_int;
    fn dup(fd: std::ffi::c_int) -> std::ffi::c_int;
}

/// Runs `bin` with stdout **and** stderr on the same pseudo terminal and returns
/// everything the terminal saw.
fn run_on_pty(bin: &std::path::Path, stdin: &[u8]) -> (Vec<u8>, Option<i32>) {
    use std::os::fd::FromRawFd;

    let mut master: std::ffi::c_int = -1;
    let mut slave: std::ffi::c_int = -1;
    let rc = unsafe {
        openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    assert_eq!(rc, 0, "openpty failed");

    let out_fd = unsafe { dup(slave) };
    let err_fd = unsafe { dup(slave) };
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(unsafe { Stdio::from_raw_fd(out_fd) })
        .stderr(unsafe { Stdio::from_raw_fd(err_fd) })
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {}", bin.display(), e));
    unsafe { close(slave) };

    {
        let mut si = child.stdin.take().unwrap();
        let data = stdin.to_vec();
        std::thread::spawn(move || {
            let _ = si.write_all(&data);
        });
    }

    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { read(master, buf.as_mut_ptr() as *mut std::ffi::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
    let st = child.wait().expect("wait");
    unsafe { close(master) };
    (out, st.code())
}

/// stdout and stderr merged into the *same pipe* (`prog 2>&1 | cat`).  glibc
/// picks its buffer size from `st_blksize`, which differs between a pipe and a
/// regular file, so this is a different configuration from `c50_merged_streams`.
fn diff_driver_merged_pipe(stdin: &[u8]) -> Vec<u8> {
    ensure_c_artifacts();
    ensure_rust_driver();

    let run = |bin: &std::path::Path| -> Vec<u8> {
        let (mut reader, writer) = std::io::pipe().expect("pipe");
        let writer2 = writer.try_clone().expect("clone pipe writer");
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(writer))
            .stderr(Stdio::from(writer2))
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {}: {}", bin.display(), e));
        {
            let mut si = child.stdin.take().unwrap();
            let data = stdin.to_vec();
            std::thread::spawn(move || {
                let _ = si.write_all(&data);
            });
        }
        let collector = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = std::io::Read::read_to_end(&mut reader, &mut buf);
            buf
        });
        let st = child.wait().expect("wait");
        assert_eq!(st.code(), Some(0));
        collector.join().expect("collector")
    };

    let c = run(&c_driver_path());
    let r = run(&rust_driver_path());
    assert_eq!(
        show(&c),
        show(&r),
        "merged pipe output differs for stdin {}",
        show(stdin)
    );
    c
}

#[test]
fn c50d_merged_pipe() {
    let dir = std::env::temp_dir();
    let big = dir.join(format!("ta_e2e_pipe_big_{}", std::process::id()));
    std::fs::write(&big, vec![b'x'; 9000]).unwrap();
    let big = big.to_string_lossy().into_owned();

    // 60 stderr messages, then >4096 bytes of stdout, then more stderr
    let mut stdin: Vec<u8> = Vec::new();
    for i in 0..60 {
        stdin.extend_from_slice(format!("2\n/missing/file/{}\n", i).as_bytes());
    }
    stdin.extend_from_slice(b"1\n");
    for i in 0..300 {
        stdin.extend_from_slice(format!("id{} ", i).as_bytes());
    }
    stdin.extend_from_slice(b"\n\n3\n");
    stdin.extend_from_slice(format!("2\n{}\n", big).as_bytes());
    stdin.extend_from_slice(b"4\n7\n");

    let out = diff_driver_merged_pipe(&stdin);
    assert!(out.len() > 8192, "expected several buffer flushes, got {}", out.len());

    diff_driver_merged_pipe(b"7\n");
    diff_driver_merged_pipe(b"2\n/missing\n7\n");
    let _ = std::fs::remove_file(&big);
}

/// A reader that goes away early: the C build dies from `SIGPIPE`, so the
/// translation has to as well (the Rust runtime ignores `SIGPIPE` by default).
#[test]
fn c50c_stdout_closed_early() {
    ensure_c_artifacts();
    ensure_rust_driver();

    let mut stdin: Vec<u8> = Vec::new();
    for _ in 0..300 {
        stdin.extend_from_slice(b"1\nsome text to analyze here\n\n3\n4\n");
    }
    stdin.extend_from_slice(b"7\n");

    let run = |bin: &std::path::Path| -> (Option<i32>, Option<i32>) {
        use std::os::unix::process::ExitStatusExt;
        // `head -c 64` exits as soon as it has its 64 bytes, closing the pipe
        let mut head = Command::new("head")
            .args(["-c", "64"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn head");
        let sink = head.stdin.take().unwrap();
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::from(sink))
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {}: {}", bin.display(), e));
        {
            let mut si = child.stdin.take().unwrap();
            let data = stdin.to_vec();
            std::thread::spawn(move || {
                let _ = si.write_all(&data);
            });
        }
        let st = child.wait().expect("wait");
        let _ = head.wait();
        (st.code(), st.signal())
    };

    let c = run(&c_driver_path());
    let r = run(&rust_driver_path());
    assert_eq!(
        c, r,
        "exit status/signal differ when stdout is closed early (C {:?}, Rust {:?})",
        c, r
    );
}

// -- fully interactive: stdin, stdout and stderr all on one terminal ---------

const NCCS: usize = 32;

#[repr(C)]
#[derive(Clone, Copy)]
struct Termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_line: u8,
    c_cc: [u8; NCCS],
    c_ispeed: u32,
    c_ospeed: u32,
}

extern "C" {
    fn tcgetattr(fd: std::ffi::c_int, t: *mut Termios) -> std::ffi::c_int;
    fn tcsetattr(fd: std::ffi::c_int, act: std::ffi::c_int, t: *const Termios) -> std::ffi::c_int;
    fn write(fd: std::ffi::c_int, buf: *const std::ffi::c_void, n: usize) -> isize;
}

/// Runs `bin` with stdin **and** stdout/stderr on the same pseudo terminal, so
/// that the C `stdin` stream is line buffered too - which makes glibc flush the
/// line-buffered `stdout` before every read (the unflushed `"Choice: "` prompt).
fn run_interactive_pty(bin: &std::path::Path, stdin: &[u8]) -> (Vec<u8>, Option<i32>) {
    use std::os::fd::FromRawFd;

    let mut master: std::ffi::c_int = -1;
    let mut slave: std::ffi::c_int = -1;
    assert_eq!(
        unsafe {
            openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
            )
        },
        0,
        "openpty failed"
    );

    // switch the terminal echo off so that only the program's output is seen
    let mut t = Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_line: 0,
        c_cc: [0; NCCS],
        c_ispeed: 0,
        c_ospeed: 0,
    };
    assert_eq!(unsafe { tcgetattr(slave, &mut t) }, 0, "tcgetattr failed");
    const ECHO: u32 = 0o10;
    t.c_lflag &= !ECHO;
    assert_eq!(unsafe { tcsetattr(slave, 0, &t) }, 0, "tcsetattr failed");

    let in_fd = unsafe { dup(slave) };
    let out_fd = unsafe { dup(slave) };
    let err_fd = unsafe { dup(slave) };
    let mut child = Command::new(bin)
        .stdin(unsafe { Stdio::from_raw_fd(in_fd) })
        .stdout(unsafe { Stdio::from_raw_fd(out_fd) })
        .stderr(unsafe { Stdio::from_raw_fd(err_fd) })
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {}", bin.display(), e));
    unsafe { close(slave) };

    let data = stdin.to_vec();
    let writer = std::thread::spawn(move || {
        unsafe {
            write(master, data.as_ptr() as *const std::ffi::c_void, data.len());
        }
    });

    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { read(master, buf.as_mut_ptr() as *mut std::ffi::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
        if out.ends_with(b"Goodbye!\r\n") {
            break;
        }
    }
    let _ = writer.join();
    let st = child.wait().expect("wait");
    unsafe { close(master) };
    (out, st.code())
}

#[test]
fn c50e_interactive_terminal() {
    ensure_c_artifacts();
    ensure_rust_driver();
    let cases: Vec<Vec<u8>> = vec![
        b"7\n".to_vec(),
        b"2\n/missing-file-xyz\n7\n".to_vec(),
        b"2\n/missing1\n2\n/missing2\n3\n7\n".to_vec(),
        b"1\nint a = 1; // c\n\n3\n4\n5\na\n7\n".to_vec(),
        b"x\n0\n8\n7\n".to_vec(),
        b"6\nint x;\n\n7\n".to_vec(),
    ];
    for stdin in &cases {
        let (co, cs) = run_interactive_pty(&c_driver_path(), stdin);
        let (ro, rs) = run_interactive_pty(&rust_driver_path(), stdin);
        assert_eq!(
            show(&co),
            show(&ro),
            "interactive terminal output differs for stdin {}",
            show(stdin)
        );
        assert_eq!(cs, rs, "exit status differs for stdin {}", show(stdin));
        assert!(co.len() > 100);
    }
}

#[test]
fn c50b_tty_line_buffering() {
    ensure_c_artifacts();
    ensure_rust_driver();
    let missing_file_script = b"2\n/definitely/missing\n3\n7\n".to_vec();
    let cases: Vec<Vec<u8>> = vec![
        b"7\n".to_vec(),
        b"".to_vec(),
        b"1\nint a = 1; // c\n\n3\n4\n7\n".to_vec(),
        missing_file_script,
        b"6\nint x;\n\n5\nx\n5\n\n7\n".to_vec(),
        b"x\n0\n8\n99\n7\n".to_vec(),
        b"1\na b c\n\n1\nd e f\n\n3\n4\n7\n".to_vec(),
    ];
    for stdin in &cases {
        let (co, cs) = run_on_pty(&c_driver_path(), stdin);
        let (ro, rs) = run_on_pty(&rust_driver_path(), stdin);
        assert_eq!(
            show(&co),
            show(&ro),
            "terminal output differs for stdin {}",
            show(stdin)
        );
        assert_eq!(cs, rs, "exit status differs for stdin {}", show(stdin));
        assert!(!co.is_empty() || stdin.is_empty());
    }
}

// ---------------------------------------------------------------------------
// C41, E36, E37, E38, E42: the menu loop itself
// ---------------------------------------------------------------------------

#[test]
fn c41_menu_choices_and_invalid_input() {
    let cases: Vec<Vec<u8>> = vec![
        b"7\n".to_vec(),
        b"".to_vec(),
        b"\n".to_vec(),
        b"\n\n\n7\n".to_vec(),
        b"x\n".to_vec(),
        b"x\n7\n".to_vec(),
        b" \n7\n".to_vec(),
        b"+\n7\n".to_vec(),
        b"-\n7\n".to_vec(),
        b".\n7\n".to_vec(),
        b"abc\n7\n".to_vec(),
        b"0\n".to_vec(),
        b"8\n".to_vec(),
        b"-1\n".to_vec(),
        b"99\n".to_vec(),
        b"0\n8\n-1\n7\n".to_vec(),
        b"99999999999999999999\n7\n".to_vec(),
        b"-99999999999999999999\n7\n".to_vec(),
        b"2147483647\n7\n".to_vec(),
        b"2147483648\n7\n".to_vec(),
        b"4294967296\n7\n".to_vec(),
        b"4294967297\n7\n".to_vec(),
        b"  3  \n7\n".to_vec(),
        b"3junk\n7\n".to_vec(),
        b"+4\n7\n".to_vec(),
        b"007\n".to_vec(),
        b"7x\n".to_vec(),
        b"7".to_vec(),
        b"3".to_vec(),
        b"\t7\n".to_vec(),
        b"0x7\n7\n".to_vec(),
        b"\r\n7\n".to_vec(),
        b"\x0b7\n".to_vec(),
        b"\x0c7\n".to_vec(),
        b"0000000000000000007\n".to_vec(),
        b"  -0  \n7\n".to_vec(),
        b"1 2 3\n\n7\n".to_vec(),
        b"7\r\n".to_vec(),
        b"\xff\n7\n".to_vec(),
        b"3\x00 4\n7\n".to_vec(),
        // E42: a menu line longer than the 256-byte fgets buffer
        {
            let mut v = vec![b'1'; 300];
            v.push(b'\n');
            v.extend_from_slice(b"\n7\n");
            v
        },
        {
            let mut v = vec![b' '; 300];
            v.extend_from_slice(b"7\n");
            v
        },
    ];
    for stdin in &cases {
        diff_driver(stdin);
    }
}

// ---------------------------------------------------------------------------
// C42, C43, C47, E40: choice 1 (analyze), 3 (distribution), 4 (score)
// ---------------------------------------------------------------------------

#[test]
fn c42_analyze_then_report() {
    let cases: Vec<Vec<u8>> = vec![
        b"1\n\n7\n".to_vec(),
        b"1\nint a = 1;\n\n7\n".to_vec(),
        b"1\nint a = 1;\n\n3\n4\n7\n".to_vec(),
        b"1\nif (a >= 1) { return \"s\"; } // c\n/* m */\n\n3\n4\n7\n".to_vec(),
        b"1\nint a = 1;\n".to_vec(), // EOF right after the text
        b"1\n".to_vec(),             // EOF while reading the text
        b"1\nno trailing newline".to_vec(),
        b"3\n4\n7\n".to_vec(), // C47: report before any analysis
        b"4\n3\n4\n7\n".to_vec(),
    ];
    for stdin in &cases {
        diff_driver(stdin);
    }
}

#[test]
fn c43_repeated_analysis() {
    let mut stdin: Vec<u8> = Vec::new();
    for i in 0..6 {
        stdin.extend_from_slice(b"1\n");
        stdin.extend_from_slice(format!("int v{} = {}; // c{}\n", i, i, i).as_bytes());
        stdin.extend_from_slice(format!("v{} += v{};\n", i, i).as_bytes());
        stdin.extend_from_slice(b"\n");
        stdin.extend_from_slice(b"3\n4\n");
    }
    stdin.extend_from_slice(b"5\nv1\n5\n\n7\n");
    diff_driver(&stdin);
}

#[test]
fn c49_oversized_payloads() {
    // E40/E41: more than MAX_INPUT_SIZE (4096) bytes for choice 1
    let mut stdin: Vec<u8> = b"1\n".to_vec();
    let mut n = 0;
    while n < 6000 {
        stdin.extend_from_slice(b"alpha beta gamma delta epsilon zeta eta theta\n");
        n += 45;
    }
    stdin.extend_from_slice(b"\n3\n4\n7\n");
    diff_driver(&stdin);

    // a single 5000-byte line (split by fgets into 256-byte chunks)
    let mut stdin: Vec<u8> = b"1\n".to_vec();
    stdin.extend_from_slice(&vec![b'q'; 5000]);
    stdin.extend_from_slice(b"\n\n3\n4\n7\n");
    diff_driver(&stdin);

    // choice 6 with more than 4096 bytes
    let mut stdin: Vec<u8> = b"6\n".to_vec();
    while stdin.len() < 5000 {
        stdin.extend_from_slice(b"tok1 tok2 tok3 tok4 tok5 tok6 tok7 tok8\n");
    }
    stdin.extend_from_slice(b"\n7\n");
    diff_driver(&stdin);
}

// ---------------------------------------------------------------------------
// C44, E30-E32, E39: choice 2 (load from file)
// ---------------------------------------------------------------------------

#[test]
fn c44_load_from_file() {
    let dir = std::env::temp_dir();
    let mk = |name: &str, bytes: &[u8]| -> String {
        let p = dir.join(format!("ta_e2e_{}_{}", std::process::id(), name));
        std::fs::write(&p, bytes).unwrap();
        p.to_string_lossy().into_owned()
    };

    let empty = mk("empty", b"");
    let small = mk("small", b"int main(void) { return 0; } // c\n");
    let n8191 = mk("n8191", &vec![b'a'; 8191]);
    let n8192 = mk("n8192", &vec![b'b'; 8192]);
    let n8193 = mk("n8193", &vec![b'c'; 8193]);
    let withnul = mk("withnul", b"abc\0def ghi\n");
    let subdir = dir.join(format!("ta_e2e_dir_{}", std::process::id()));
    std::fs::create_dir_all(&subdir).unwrap();

    let cases: Vec<Vec<u8>> = vec![
        format!("2\n{}\n7\n", empty).into_bytes(),
        format!("2\n{}\n3\n4\n7\n", small).into_bytes(),
        format!("2\n{}\n7\n", n8191).into_bytes(),
        format!("2\n{}\n7\n", n8192).into_bytes(),
        format!("2\n{}\n7\n", n8193).into_bytes(),
        format!("2\n{}\n7\n", withnul).into_bytes(),
        format!("2\n{}\n7\n", subdir.to_string_lossy()).into_bytes(),
        b"2\n/definitely/missing/file\n7\n".to_vec(),
        b"2\n\n7\n".to_vec(),   // empty filename
        b"2\n7\n".to_vec(),     // "7" read as the filename
        b"2\n".to_vec(),        // E39: EOF at the filename prompt
        b"2\n/dev/null\n7\n".to_vec(),
        b"2\n/proc/version\n7\n".to_vec(),
        format!("2\n{}\n2\n{}\n3\n4\n7\n", small, small).into_bytes(),
    ];
    for stdin in &cases {
        diff_driver(stdin);
    }

    let _ = std::fs::remove_dir(&subdir);
    for f in [empty, small, n8191, n8192, n8193, withnul] {
        let _ = std::fs::remove_file(f);
    }
}

// ---------------------------------------------------------------------------
// C45, E39: choice 5 (find pattern)
// ---------------------------------------------------------------------------

#[test]
fn c45_find_pattern() {
    let cases: Vec<Vec<u8>> = vec![
        b"5\nabc\n7\n".to_vec(),                       // before any analysis
        b"1\naa bb aa cc\n\n5\naa\n7\n".to_vec(),
        b"1\naa bb aa cc\n\n5\n\n7\n".to_vec(),        // empty pattern
        b"1\naa bb aa cc\n\n5\nzz\n7\n".to_vec(),
        b"1\nint a; // int\n\n5\nint\n5\n//\n7\n".to_vec(),
        b"1\n\"quoted\" 'x'\n\n5\n\"\n5\n'\n7\n".to_vec(),
        b"1\na\\b\n\n5\n\\\n7\n".to_vec(),
        b"5\n".to_vec(), // E39: EOF at the pattern prompt
        b"1\nab\n\n5\nab\n5\nab\n5\nab\n7\n".to_vec(),
    ];
    for stdin in &cases {
        diff_driver(stdin);
    }
}

// ---------------------------------------------------------------------------
// C46, E35: choice 6 (interactive tokenizer)
// ---------------------------------------------------------------------------

#[test]
fn c46_interactive_tokenizer() {
    let mut many = b"6\n".to_vec();
    for i in 0..120 {
        many.extend_from_slice(format!("t{} ", i).as_bytes());
    }
    many.extend_from_slice(b"\n\n7\n");

    let cases: Vec<Vec<u8>> = vec![
        b"6\n\n7\n".to_vec(),
        b"6\nint a = 1;\n\n7\n".to_vec(),
        b"6\na\nb\nc\n\n7\n".to_vec(),
        b"6\nint a = 1;\n".to_vec(),
        b"6\n".to_vec(),
        many,
        b"6\n/* unterminated\n\n7\n".to_vec(),
        b"6\n\"unterminated\n\n7\n".to_vec(),
        b"6\n\xff\x80 caf\xc3\xa9\n\n7\n".to_vec(),
        b"6\nfirst\n\n6\nsecond\n\n7\n".to_vec(),
        b"6\n\n6\n\n6\n\n7\n".to_vec(),
    ];
    for stdin in &cases {
        diff_driver(stdin);
    }
}

// ---------------------------------------------------------------------------
// C48: randomized menu scripts
// ---------------------------------------------------------------------------

#[test]
fn c48_random_menu_scripts() {
    let dir = std::env::temp_dir();
    let file = dir.join(format!("ta_e2e_rand_{}", std::process::id()));
    std::fs::write(&file, b"int rand_file = 1; // c\n\"s\"\n").unwrap();
    let file = file.to_string_lossy().into_owned();

    let mut rng = Rng::new(0xC48);
    for _ in 0..60 {
        let mut stdin: Vec<u8> = Vec::new();
        let ops = rng.range(1, 12);
        for _ in 0..ops {
            match rng.below(9) {
                0 => {
                    stdin.extend_from_slice(b"1\n");
                    let lines = rng.below(4);
                    for _ in 0..lines {
                        stdin.extend_from_slice(&random_source(&mut rng, 12));
                        stdin.push(b'\n');
                    }
                    stdin.push(b'\n');
                }
                1 => {
                    stdin.extend_from_slice(b"2\n");
                    if rng.chance(3) {
                        stdin.extend_from_slice(b"/no/such/file\n");
                    } else {
                        stdin.extend_from_slice(file.as_bytes());
                        stdin.push(b'\n');
                    }
                }
                2 => stdin.extend_from_slice(b"3\n"),
                3 => stdin.extend_from_slice(b"4\n"),
                4 => {
                    stdin.extend_from_slice(b"5\n");
                    let pat = random_soup(&mut rng, 4);
                    let pat: Vec<u8> = pat.into_iter().filter(|&b| b != b'\n').collect();
                    stdin.extend_from_slice(&pat);
                    stdin.push(b'\n');
                }
                5 => {
                    stdin.extend_from_slice(b"6\n");
                    let lines = rng.below(3);
                    for _ in 0..lines {
                        stdin.extend_from_slice(&random_source(&mut rng, 10));
                        stdin.push(b'\n');
                    }
                    stdin.push(b'\n');
                }
                6 => stdin.extend_from_slice(b"\n"),
                7 => {
                    let junk = random_soup(&mut rng, 6);
                    let junk: Vec<u8> = junk.into_iter().filter(|&b| b != b'\n').collect();
                    stdin.extend_from_slice(&junk);
                    stdin.push(b'\n');
                }
                _ => {
                    let n = rng.next_u64() as i64 % 1000 - 500;
                    stdin.extend_from_slice(format!("{}\n", n).as_bytes());
                }
            }
        }
        if rng.chance(2) {
            stdin.extend_from_slice(b"7\n");
        }
        diff_driver(&stdin);
    }
    let _ = std::fs::remove_file(&file);
}

// ---------------------------------------------------------------------------
// C50: stdout and stderr merged into a single file (flush ordering)
// ---------------------------------------------------------------------------

#[test]
fn c50_merged_streams() {
    let dir = std::env::temp_dir();
    let big = dir.join(format!("ta_e2e_big_{}", std::process::id()));
    std::fs::write(&big, vec![b'x'; 9000]).unwrap();
    let big = big.to_string_lossy().into_owned();

    let mut long_analysis: Vec<u8> = b"1\n".to_vec();
    for i in 0..200 {
        long_analysis.extend_from_slice(format!("word{} op{} = {};\n", i, i, i).as_bytes());
    }
    long_analysis.extend_from_slice(b"\n3\n4\n");
    long_analysis.extend_from_slice(format!("2\n{}\n", big).as_bytes());
    long_analysis.extend_from_slice(b"2\n/missing/file\n3\n7\n");

    // many stderr writes spread over far more than one stdio buffer of stdout
    let mut many_errors: Vec<u8> = Vec::new();
    for i in 0..60 {
        many_errors.extend_from_slice(format!("2\n/missing/file/{}\n", i).as_bytes());
    }
    many_errors.extend_from_slice(b"7\n");

    // a big analysis (>4096 bytes of stdout) followed by a stderr write
    let mut big_then_error: Vec<u8> = b"1\n".to_vec();
    for i in 0..300 {
        big_then_error.extend_from_slice(format!("id{} ", i).as_bytes());
    }
    big_then_error.extend_from_slice(b"\n\n3\n");
    big_then_error.extend_from_slice(format!("2\n{}\n", big).as_bytes());
    big_then_error.extend_from_slice(b"3\n4\n7\n");

    let cases: Vec<Vec<u8>> = vec![
        b"7\n".to_vec(),
        b"2\n/missing/file\n7\n".to_vec(),
        format!("2\n{}\n7\n", big).into_bytes(),
        format!("2\n/missing\n2\n{}\n2\n/missing2\n7\n", big).into_bytes(),
        long_analysis,
        many_errors,
        big_then_error,
    ];
    for stdin in &cases {
        diff_driver_merged(stdin);
    }
    let _ = std::fs::remove_file(&big);
}
