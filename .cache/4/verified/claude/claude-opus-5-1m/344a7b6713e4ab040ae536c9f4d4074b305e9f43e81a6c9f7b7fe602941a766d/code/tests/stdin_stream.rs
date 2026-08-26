//! Phase B/C — the `stdin`/`stdout` *stream* behaviour that only shows up when
//! something else shares the descriptor with the program (rows C30–C32 of
//! CONFIGS.md and row E18 of ERRORS.md).
//!
//! `scanf` reads ahead into glibc's `stdin` buffer and, at exit, glibc seeks a
//! seekable descriptor back over whatever it did not consume.  A translation
//! that reads through a private buffer would silently eat the rest of the file,
//! so these tests compare, for the C and the Rust build:
//!
//!   * the file offset fd 0 is left at,
//!   * what a second reader on the same descriptor sees,
//!   * how many bytes are consumed from an unseekable pipe,
//!   * and the granularity at which concurrent callers may interleave.

mod common;

use common::*;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::raw::c_int;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::Path;
use std::process::{Command, Stdio};

extern "C" {
    fn alarm(seconds: u32) -> u32;
    fn signal(signum: c_int, handler: usize) -> usize;
    fn siginterrupt(sig: c_int, flag: c_int) -> c_int;
}

const SIGALRM: c_int = 14;

/// Run `exe` `runs` times with the *same* open file description on fd 0, and
/// report each run's stdout plus the file offset left behind.
fn runs_sharing_one_stdin(exe: &Path, input: &[u8], runs: usize) -> (Vec<String>, u64) {
    let path = scratch_file("shared-stdin");
    std::fs::write(&path, input).unwrap();
    let mut f = File::open(&path).unwrap();
    let mut outs = Vec::new();
    for _ in 0..runs {
        let dup = f.try_clone().unwrap(); // same file description => shared offset
        let out = Command::new(exe)
            .stdin(Stdio::from(dup))
            .stdout(Stdio::piped())
            .output()
            .unwrap();
        outs.push(as_text(&out.stdout));
    }
    let off = f.stream_position().unwrap();
    let _ = std::fs::remove_file(&path);
    (outs, off)
}

/// Run `exe` with a pipe on fd 0 that already holds `input`, and report the
/// program's stdout plus the bytes still queued in the pipe afterwards.
fn bytes_left_in_pipe(exe: &Path, input: &[u8]) -> (String, usize) {
    assert!(input.len() < 60_000);
    let (rd, wr) = make_pipe();
    let mut writer = unsafe { File::from_raw_fd(wr) };
    writer.write_all(input).unwrap();
    drop(writer); // close the write end so the program can see EOF
    let reader = unsafe { File::from_raw_fd(rd) };
    let child_stdin = reader.try_clone().unwrap();
    let out = Command::new(exe)
        .stdin(Stdio::from(child_stdin))
        .stdout(Stdio::piped())
        .output()
        .unwrap();
    let mut rest = Vec::new();
    let mut reader = reader;
    reader.read_to_end(&mut rest).unwrap();
    (as_text(&out.stdout), rest.len())
}

/// C30 — the read-ahead and the exit-time seek-back on a seekable fd 0.
#[test]
fn c30_stdin_read_ahead_and_offset_are_identical() {
    let mut inputs: Vec<Vec<u8>> = vec![
        b"1 2 3".to_vec(),
        b"1 2 3\n".to_vec(),
        b"1\n2\n3\n".to_vec(),
        b"12-34".to_vec(),
        b"12x34".to_vec(),
        b"-1 -2 -3".to_vec(),
        b"  \t\n 42   rest of the line\n".to_vec(),
        b"".to_vec(),
        b"   ".to_vec(),
        b"junk 7".to_vec(),
        b"0".to_vec(),
        b"2147483648 2147483648".to_vec(),
        b"9223372036854775808 5".to_vec(),
        b"\0 12".to_vec(),
    ];
    // Inputs that cross the stream buffer, so the read-ahead is truncated.
    for n in [4_000usize, 4_095, 4_096, 4_097, 8_192, 20_000] {
        let mut v = b"7 ".to_vec();
        v.extend(std::iter::repeat(b'A').take(n));
        v.push(b'\n');
        inputs.push(v);
        let mut w = vec![b' '; n];
        w.extend_from_slice(b"-99 tail\n");
        inputs.push(w);
    }
    let mut rng = Rng::new(0xC030);
    for _ in 0..40 {
        let tokens = 1 + rng.below(5) as usize;
        let mut s = String::new();
        for _ in 0..tokens {
            let sign = rng.pick(&["", "-", "+"]);
            let sep = rng.pick(&[" ", "\n", "x", "\t", ".", ""]);
            let len = 1 + rng.below(21) as usize;
            s.push_str(&format!("{sign}{}{sep}", rng.digits(len)));
        }
        inputs.push(s.into_bytes());
    }

    for input in &inputs {
        for runs in [1usize, 2, 3] {
            let c = runs_sharing_one_stdin(c_exe_path(), input, runs);
            let r = runs_sharing_one_stdin(rust_exe_path(), input, runs);
            assert_eq!(
                c,
                r,
                "stdin read-ahead/offset diverged after {runs} run(s) on {}",
                preview(input)
            );
        }
    }
}

/// C30 — a second reader on the same descriptor must see the same leftover
/// bytes (`{ ./driver; cat; } < file`).
#[test]
fn c30b_co_reader_sees_the_same_leftover_bytes() {
    let inputs: [&[u8]; 8] = [
        b"1\nHELLO\n",
        b"12 34 56\n",
        b"42rest",
        b"  7  tail\n",
        b"junk and more",
        b"",
        b"1",
        b"-9223372036854775809 trailing\n",
    ];
    for input in inputs {
        let mut outs = Vec::new();
        for exe in [c_exe_path(), rust_exe_path()] {
            let path = scratch_file("coread");
            std::fs::write(&path, input).unwrap();
            let f = File::open(&path).unwrap();
            let prog = Command::new(exe)
                .stdin(Stdio::from(f.try_clone().unwrap()))
                .stdout(Stdio::piped())
                .output()
                .unwrap();
            // Now read the rest of the shared description, like `cat` would.
            let mut rest = Vec::new();
            let mut f2 = f;
            f2.read_to_end(&mut rest).unwrap();
            let _ = std::fs::remove_file(&path);
            outs.push((as_text(&prog.stdout), as_text(&rest)));
        }
        assert_eq!(
            outs[0],
            outs[1],
            "a co-reader saw different bytes for {}",
            preview(input)
        );
    }
}

/// C30 — how much of an unseekable pipe the conversion consumes.
#[test]
fn c30c_pipe_consumption_is_identical() {
    let mut inputs: Vec<Vec<u8>> = vec![b"7 tail".to_vec(), b"1 2 3".to_vec()];
    for n in [100usize, 4_000, 4_095, 4_096, 4_097, 8_191, 8_192, 40_000] {
        let mut v = b"7".to_vec();
        v.extend(std::iter::repeat(b' ').take(n));
        v.extend_from_slice(b"TAIL");
        inputs.push(v);
        let mut w = vec![b'0'; n];
        w.extend_from_slice(b" 5 TAIL");
        inputs.push(w);
    }
    for input in &inputs {
        let c = bytes_left_in_pipe(c_exe_path(), input);
        let r = bytes_left_in_pipe(rust_exe_path(), input);
        assert_eq!(
            c,
            r,
            "pipe consumption diverged for {} (C left {} bytes, Rust {})",
            preview(input),
            c.1,
            r.1
        );
    }
}

/// C30 — the same, for the exported `main` called twice on one shared seekable
/// descriptor (no process exit in between, so no seek-back happens on either
/// side and the read-ahead has to line up by itself).
#[test]
fn c30d_repeated_main_on_a_shared_descriptor() {
    let p = pair();
    let inputs: [&[u8]; 6] = [b"1 2 3", b"12x34", b"7", b"", b"  8  ", b"1 2 3 4 5 6"];
    for input in inputs {
        for n in [2usize, 4] {
            let c = run_child(Stdin::File(input), Stdout::File, || {
                let mut rc = 0;
                for _ in 0..n {
                    rc |= unsafe { (p.c.main)() };
                }
                rc
            });
            let r = run_child(Stdin::File(input), Stdout::File, || {
                let mut rc = 0;
                for _ in 0..n {
                    rc |= unsafe { (p.rs.main)() };
                }
                rc
            });
            assert_eq!(
                (as_text(&c.out), c.status),
                (as_text(&r.out), r.status),
                "`main` x{n} on a shared descriptor for {}",
                preview(input)
            );
        }
    }
}

/// C31 — concurrency granularity: the C `print_hex` performs four `printf`
/// calls plus a `putchar`, each taking the stdout lock on its own, so records
/// from concurrent callers may interleave *within* a line.  The translation must
/// allow the same interleaving (and must of course lose no bytes).
#[test]
fn c31_concurrent_driver_calls_have_the_same_granularity() {
    const THREADS: usize = 4;
    const CALLS: usize = 300;
    const ROUNDS: usize = 3;
    let p = pair();
    let values: [i32; 4] = [0x1111_1111, 0x2222_2222, 0x3333_3333, 0x4444_4444];

    let mut torn = [0usize; 2]; // [C, Rust]
    for round in 0..ROUNDS {
        for (idx, driver) in [p.c.driver, p.rs.driver].into_iter().enumerate() {
            let run = capture_child(move || {
                let mut handles = Vec::new();
                for t in 0..THREADS {
                    let v = values[t];
                    handles.push(std::thread::spawn(move || {
                        for _ in 0..CALLS {
                            unsafe { driver(v) }
                        }
                    }));
                }
                for h in handles {
                    let _ = h.join();
                }
            });
            assert_eq!(run.status, Status::Exited(0));
            let text = as_text(&run.out);

            // No bytes lost or invented, whoever wins the races.
            assert_eq!(
                run.out.len(),
                THREADS * CALLS * 9,
                "round {round}, impl {idx}: wrong byte count"
            );
            let mut chars: Vec<char> = text.chars().collect();
            chars.sort_unstable();
            let mut expect: Vec<char> = Vec::new();
            for v in values {
                for _ in 0..CALLS * 8 {
                    expect.push(char::from_digit(((v >> 28) & 0xf) as u32, 16).unwrap());
                }
            }
            for _ in 0..THREADS * CALLS {
                expect.push('\n');
            }
            expect.sort_unstable();
            assert_eq!(chars, expect, "round {round}, impl {idx}: byte multiset");

            torn[idx] += text
                .lines()
                .filter(|l| {
                    !matches!(
                        *l,
                        "11111111" | "22222222" | "33333333" | "44444444"
                    )
                })
                .count();
        }
    }
    // If the C build interleaved at all, the translation must be able to as
    // well — a per-line lock would make it impossible.
    if torn[0] > 0 {
        assert!(
            torn[1] > 0,
            "C interleaved {} lines but Rust never did: the translation locks \
             stdout at a coarser granularity than the C code",
            torn[0]
        );
    }
    eprintln!("torn lines over {ROUNDS} rounds: C {} / Rust {}", torn[0], torn[1]);
}

/// E18 — a `read` interrupted by a signal whose handler is installed without
/// `SA_RESTART`: glibc's `_IO_new_file_underflow` treats the failed `read` as a
/// stream error and the conversion fails (leaving `x` at 0); it never retries.
#[test]
fn e18_interrupted_read_is_an_input_failure() {
    let p = pair();
    let mut results = Vec::new();
    for main in [p.c.main, p.rs.main] {
        let run = run_child(Stdin::SlowPipe(b"7\n"), Stdout::File, move || {
            // Interrupt the pending read after one second, with a handler that
            // does *not* restart the syscall.
            extern "C" fn on_alarm(_sig: c_int) {}
            unsafe {
                signal(SIGALRM, on_alarm as *const () as usize);
                siginterrupt(SIGALRM, 1);
                alarm(1);
                main()
            }
        });
        results.push((as_text(&run.out), run.status));
    }
    assert_eq!(
        results[0], results[1],
        "an interrupted stdin read diverged (C {:?} vs Rust {:?})",
        results[0], results[1]
    );
    assert_eq!(
        results[0].0, "00000000\n",
        "the interrupted conversion must fail and leave x == 0"
    );
}

/// E18 (b) — the same signal arriving while the handler *does* restart the
/// syscall: both must then read the value normally.
#[test]
fn e18b_restarted_read_still_converts() {
    let p = pair();
    let mut results = Vec::new();
    for main in [p.c.main, p.rs.main] {
        let run = run_child(Stdin::SlowPipe(b"7\n"), Stdout::File, move || {
            extern "C" fn on_alarm(_sig: c_int) {}
            unsafe {
                signal(SIGALRM, on_alarm as *const () as usize); // BSD semantics: SA_RESTART
                alarm(1);
                main()
            }
        });
        results.push((as_text(&run.out), run.status));
    }
    assert_eq!(
        results[0], results[1],
        "a restarted stdin read diverged (C {:?} vs Rust {:?})",
        results[0], results[1]
    );
    assert_eq!(results[0].0, "07000000\n");
}

/// C32 — fd 0 and fd 1 are the same read/write descriptor (a common shell
/// shape: `./driver <> file`), so the read-ahead and the output interact.
#[test]
fn c32_stdin_and_stdout_on_one_descriptor() {
    for input in [&b"1 2 3"[..], b"42", b"", b"   9  tail"] {
        let mut outs = Vec::new();
        for exe in [c_exe_path(), rust_exe_path()] {
            let path = scratch_file("rw");
            std::fs::write(&path, input).unwrap();
            let f = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .unwrap();
            let status = Command::new(exe)
                .stdin(Stdio::from(f.try_clone().unwrap()))
                .stdout(Stdio::from(f.try_clone().unwrap()))
                .status()
                .unwrap();
            let after = std::fs::read(&path).unwrap();
            let off = {
                let mut g = f;
                g.seek(SeekFrom::Current(0)).unwrap()
            };
            let _ = std::fs::remove_file(&path);
            outs.push((as_text(&after), status.code(), off));
        }
        assert_eq!(
            outs[0],
            outs[1],
            "stdin==stdout descriptor diverged for {}",
            preview(input)
        );
    }
}

/// Sanity: the helpers above really observe the C behaviour documented in
/// CONFIGS.md (guards against a helper that silently measures nothing).
#[test]
fn c30_helpers_observe_the_documented_c_behaviour() {
    // "1 2 3": the C build stops after `1`, hands ` ` back, and exits with the
    // descriptor positioned at offset 1.
    let (outs, off) = runs_sharing_one_stdin(c_exe_path(), b"1 2 3", 3);
    assert_eq!(outs, ["01000000\n", "02000000\n", "03000000\n"]);
    assert_eq!(off, 5, "the third run consumes the whole file");
    let (outs, off) = runs_sharing_one_stdin(c_exe_path(), b"1 2 3", 1);
    assert_eq!(outs, ["01000000\n"]);
    assert_eq!(off, 1, "glibc seeks back over the read-ahead at exit");

    // A pipe is not seekable, so the read-ahead (one buffer) is simply gone.
    let mut big = b"7".to_vec();
    big.extend(std::iter::repeat(b' ').take(5_000));
    big.extend_from_slice(b"TAIL");
    let (out, left) = bytes_left_in_pipe(c_exe_path(), &big);
    assert_eq!(out, "07000000\n");
    assert_eq!(
        left,
        big.len() - 4_096,
        "glibc reads one 4 KiB buffer from a pipe"
    );
    // And the raw fd numbers used by the helper are the ones we think they are.
    let f = File::open("/dev/null").unwrap();
    assert!(f.as_raw_fd() > 2);
}

/// C33 — stdout is a **character device** (a pseudo-terminal).  This is the only
/// case where glibc switches `stdout` from fully buffered to line buffered, so
/// it is the real "different buffering mode" configuration; the byte stream
/// (including the terminal's `\n` → `\r\n` translation) must be identical.
#[test]
fn c33_stdout_is_a_terminal() {
    let p = pair();

    // Several `driver` records, so a per-line flush is actually exercised.
    let c = run_child(Stdin::Inherit, Stdout::Tty, || unsafe {
        (p.c.driver)(1);
        (p.c.driver)(-1);
        (p.c.driver)(0x0a0b0c0d);
        0
    });
    let r = run_child(Stdin::Inherit, Stdout::Tty, || unsafe {
        (p.rs.driver)(1);
        (p.rs.driver)(-1);
        (p.rs.driver)(0x0a0b0c0d);
        0
    });
    assert_eq!(
        (as_text(&c.out), c.status),
        (as_text(&r.out), r.status),
        "`driver` with a tty stdout"
    );
    assert_eq!(
        as_text(&c.out),
        "01000000\r\nffffffff\r\n0d0c0b0a\r\n",
        "the terminal must translate each record's newline"
    );

    // And the whole program through the exported `main`.
    for input in [&b"42\n"[..], b"", b"junk", b"-2147483649"] {
        let c = run_child(Stdin::File(input), Stdout::Tty, || unsafe { (p.c.main)() });
        let r = run_child(Stdin::File(input), Stdout::Tty, || unsafe { (p.rs.main)() });
        assert_eq!(
            (as_text(&c.out), c.status),
            (as_text(&r.out), r.status),
            "`main` with a tty stdout for {}",
            preview(input)
        );
    }
}

/// C33 (b) — the same at the process level, driving the real programs under a
/// pty with `script(1)`.
#[test]
fn c33b_executables_under_a_pty() {
    if Command::new("script")
        .arg("--version")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        eprintln!("script(1) unavailable; skipping the pty process-level row");
        return;
    }
    for input in [&b"42"[..], b"", b"abc", b"1 2 3", b"2147483648", b"-9223372036854775809"] {
        let mut outs = Vec::new();
        for exe in [c_exe_path(), rust_exe_path()] {
            let path = scratch_file("pty-stdin");
            std::fs::write(&path, input).unwrap();
            let out = Command::new("script")
                .arg("-qec")
                .arg(format!("{} < {}", exe.display(), path.display()))
                .arg("/dev/null")
                .output()
                .unwrap();
            let _ = std::fs::remove_file(&path);
            outs.push((as_text(&out.stdout), out.status.code()));
        }
        assert_eq!(
            outs[0],
            outs[1],
            "the programs diverged under a pty for {}",
            preview(input)
        );
    }
}

/// C34 — glibc's **sticky EOF indicator**: `_IO_new_file_underflow` starts with
/// `if (_flags & _IO_EOF_SEEN) return EOF;` (C99 requires it), so once a
/// conversion has seen end of input no later conversion issues another `read` —
/// even if data has meanwhile become available, or the descriptor was rewound.
#[test]
fn c34_eof_indicator_is_sticky() {
    let p = pair();

    // (a) the file grows between two calls of the exported `main`
    for (initial, appended) in [
        (&b"42"[..], &b" 77"[..]),
        (b"", b"9"),
        (b"  ", b"5 6"),
        (b"1 2", b" 3"),
        (b"junk", b" 8"),
    ] {
        let mut outs = Vec::new();
        for main in [p.c.main, p.rs.main] {
            let path = scratch_file("grow");
            std::fs::write(&path, initial).unwrap();
            let f = File::open(&path).unwrap();
            let fd = f.as_raw_fd();
            let path2 = path.clone();
            let appended = appended.to_vec();
            let run = run_child(Stdin::Raw(fd), Stdout::File, move || {
                let a = unsafe { main() };
                // Append more data, then convert again.
                let mut g = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&path2)
                    .unwrap();
                g.write_all(&appended).unwrap();
                drop(g);
                a | unsafe { main() }
            });
            drop(f);
            let _ = std::fs::remove_file(&path);
            outs.push((as_text(&run.out), run.status));
        }
        assert_eq!(
            outs[0],
            outs[1],
            "a file that grows after EOF diverged (initial {}, appended {})",
            preview(initial),
            preview(appended)
        );
    }

    // (b) the descriptor is rewound between calls: C still reports EOF because
    //     the indicator is sticky, so the second conversion prints 0.
    for input in [&b"42"[..], b"1 2", b"7", b"junk"] {
        let mut outs = Vec::new();
        for main in [p.c.main, p.rs.main] {
            let path = scratch_file("rewind");
            std::fs::write(&path, input).unwrap();
            let f = File::open(&path).unwrap();
            let fd = f.as_raw_fd();
            let run = run_child(Stdin::Raw(fd), Stdout::File, move || {
                let mut rc = 0;
                // Drain the stream first, then rewind and convert twice more.
                for _ in 0..4 {
                    rc |= unsafe { main() };
                }
                let mut g = unsafe { <File as FromRawFd>::from_raw_fd(0) };
                let _ = g.seek(SeekFrom::Start(0));
                std::mem::forget(g);
                rc | unsafe { main() }
            });
            drop(f);
            let _ = std::fs::remove_file(&path);
            outs.push((as_text(&run.out), run.status));
        }
        assert_eq!(
            outs[0], outs[1],
            "a rewound descriptor after EOF diverged for {}",
            preview(input)
        );
    }
}

/// C35 — `scanf` holds the stream lock for the whole conversion
/// (`flockfile(stdin)`), so concurrent callers can never take alternate digits
/// of the same number.  Here every token is the same, so any value other than
/// that token proves a conversion was split.
#[test]
fn c35_concurrent_main_never_splits_a_number() {
    const TOKENS: usize = 240;
    let p = pair();
    let input: Vec<u8> = b"42 ".repeat(TOKENS);

    for threads in [2usize, 4, 8] {
        let calls = TOKENS / threads;
        let mut summaries = Vec::new();
        for main in [p.c.main, p.rs.main] {
            let run = run_child(Stdin::File(&input), Stdout::File, move || {
                let mut handles = Vec::new();
                for _ in 0..threads {
                    handles.push(std::thread::spawn(move || {
                        for _ in 0..calls {
                            unsafe { main() };
                        }
                    }));
                }
                for h in handles {
                    let _ = h.join();
                }
                0
            });
            assert_eq!(run.status, Status::Exited(0));
            let text = as_text(&run.out);
            // Records may interleave (C31), so classify characters, not lines:
            // "42" -> 0x2a -> "2a000000"; a split token would print 04.../02...
            let split_digits = text.matches('4').count();
            summaries.push((
                run.out.len(),
                text.matches('\n').count(),
                split_digits,
                text.matches('a').count(),
            ));
        }
        assert_eq!(
            summaries[0], summaries[1],
            "concurrent `main` ({threads} threads x {calls} calls) diverged: \
             (bytes, records, stray '4' digits, 'a' digits) C {:?} vs Rust {:?}",
            summaries[0], summaries[1]
        );
        assert_eq!(summaries[0].2, 0, "the C build never splits a token");
    }
}

/// C36 — re-entering the conversion from a signal handler while it blocks in
/// `read`: glibc's stream lock is recursive, so the inner conversion runs (and
/// consumes the data) and the outer one then reports end of input.  A
/// non-recursive lock would dead-lock here.
#[test]
fn c36_reentrant_main_from_a_signal_handler() {
    let p = pair();
    let mut outs = Vec::new();
    for (idx, main) in [p.c.main, p.rs.main].into_iter().enumerate() {
        REENTRANT_MAIN.store(main as *const () as usize, std::sync::atomic::Ordering::SeqCst);
        let run = run_child(Stdin::SlowPipe(b"7\n"), Stdout::File, move || {
            extern "C" fn on_alarm(_sig: c_int) {
                let f = REENTRANT_MAIN.load(std::sync::atomic::Ordering::SeqCst);
                if f != 0 {
                    let f: MainFn = unsafe { std::mem::transmute(f) };
                    unsafe { f() };
                }
            }
            unsafe {
                signal(SIGALRM, on_alarm as *const () as usize); // SA_RESTART
                alarm(1);
                main()
            }
        });
        assert_eq!(
            run.status,
            Status::Exited(0),
            "impl {idx}: re-entrant `main` must not hang or crash"
        );
        outs.push(as_text(&run.out));
    }
    assert_eq!(
        outs[0], outs[1],
        "re-entrant `main` diverged (C {:?} vs Rust {:?})",
        outs[0], outs[1]
    );
}

static REENTRANT_MAIN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// C37 — descriptor shapes for fd 0 that the stream code has to cope with: a
/// pre-positioned offset, `O_RDWR`, `O_APPEND`, a write-only fd, an empty pipe
/// with the writer still open... plus a *character device* stdin, whose
/// `st_blksize` (1024 for a pty) changes the stream buffer size.
#[test]
fn c37_fd0_descriptor_shapes() {
    let p = pair();

    // (a) fd 0 pre-positioned with lseek, and O_RDWR / O_APPEND variants.
    for offset in [0u64, 1, 2, 3] {
        for (read_write, append) in [(false, false), (true, false), (true, true)] {
            let mut outs = Vec::new();
            for main in [p.c.main, p.rs.main] {
                let path = scratch_file("shape");
                std::fs::write(&path, b"  1234xyz rest\n").unwrap();
                let mut opts = std::fs::OpenOptions::new();
                opts.read(true);
                if read_write {
                    opts.write(true);
                }
                if append {
                    opts.append(true);
                }
                let mut f = opts.open(&path).unwrap();
                f.seek(SeekFrom::Start(offset)).unwrap();
                let fd = f.as_raw_fd();
                let run = run_child(Stdin::Raw(fd), Stdout::File, move || unsafe { main() });
                let final_off = f.stream_position().unwrap();
                drop(f);
                let after = std::fs::read(&path).unwrap();
                let _ = std::fs::remove_file(&path);
                outs.push((as_text(&run.out), run.status, final_off, as_text(&after)));
            }
            assert_eq!(
                outs[0], outs[1],
                "fd 0 shape diverged (offset {offset}, rw {read_write}, append {append})"
            );
        }
    }

    // (b) a pty slave as stdin: st_blksize is 1024, not 4096, so the stream
    //     buffer — and therefore how much is consumed — must follow.  The lines
    //     are newline-terminated because a pty in canonical mode hands over a
    //     line at a time (an unterminated line would block *both* builds).
    let short_line = b"7 abcdefgh\n".to_vec();
    let long_line = {
        let mut v = b"7 ".to_vec();
        v.extend(std::iter::repeat(b'A').take(1_500));
        v.push(b'\n');
        v
    };
    for input in [&short_line[..], &long_line[..]] {
        let mut outs = Vec::new();
        for main in [p.c.main, p.rs.main] {
            let (master, slave) = make_pty();
            let mut w = unsafe { <File as FromRawFd>::from_raw_fd(master) };
            w.write_all(input).unwrap();
            let run = run_child(Stdin::Raw(slave), Stdout::File, move || unsafe { main() });
            // How much of the pty's input queue is left for the next reader?
            set_nonblocking(slave);
            let mut left = 0usize;
            let mut probe = [0u8; 4096];
            let mut sl = ManuallyDropFile::new(slave);
            while let Ok(n) = sl.get().read(&mut probe) {
                if n == 0 {
                    break;
                }
                left += n;
                if left > 8_000 {
                    break;
                }
            }
            drop(w);
            close_fd(slave);
            outs.push((as_text(&run.out), run.status, left));
        }
        assert_eq!(
            outs[0], outs[1],
            "pty stdin (st_blksize 1024) diverged for a {}-byte line",
            input.len()
        );
    }

    // (c) degenerate descriptors: write-only, /dev/null, /dev/zero (bounded),
    //     and an empty pipe whose writer is still open would block, so it is
    //     covered by E18 instead.
    for path in ["/dev/null", "/dev/zero"] {
        let mut outs = Vec::new();
        for main in [p.c.main, p.rs.main] {
            let f = File::open(path).unwrap();
            let fd = f.as_raw_fd();
            let run = run_child(Stdin::Raw(fd), Stdout::File, move || unsafe { main() });
            outs.push((as_text(&run.out), run.status));
        }
        assert_eq!(outs[0], outs[1], "fd 0 = {path} diverged");
    }
    {
        let mut outs = Vec::new();
        for main in [p.c.main, p.rs.main] {
            let path = scratch_file("wronly");
            std::fs::write(&path, b"42").unwrap();
            let f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
            let fd = f.as_raw_fd();
            let run = run_child(Stdin::Raw(fd), Stdout::File, move || unsafe { main() });
            drop(f);
            let _ = std::fs::remove_file(&path);
            outs.push((as_text(&run.out), run.status));
        }
        assert_eq!(outs[0], outs[1], "a write-only fd 0 diverged");
    }
}

/// A `File` view of a descriptor we do not own (so dropping it must not close it).
struct ManuallyDropFile(std::mem::ManuallyDrop<File>);

impl ManuallyDropFile {
    fn new(fd: c_int) -> Self {
        ManuallyDropFile(std::mem::ManuallyDrop::new(unsafe {
            <File as FromRawFd>::from_raw_fd(fd)
        }))
    }
    fn get(&mut self) -> &mut File {
        &mut self.0
    }
}
