//! Phase B + Phase C for `bad`, `good` and `main` (CONFIGS.md rows 16-37 and
//! C1; ERRORS.md rows 9-18 and G7).
//!
//! These entry points are driven in a **child process** rather than in-process:
//!
//! * `main` consumes stdin, so it has to see a virgin stdin/stdout exactly as a
//!   real program start does (calling it twice in one process would reuse the
//!   already-filled stdio buffer of the first call and read the wrong bytes).
//! * `bad()` is the CWE-131 defect: it `alloca`s 10 bytes and writes 40. At
//!   `gcc -O1` the copy loop overwrites the *saved frame pointer* at `[rbp]`
//!   with zero, so the function returns with a corrupted `rbp`. That is genuine
//!   undefined behaviour in the C ground truth; it must be contained in a
//!   throw-away process instead of wrecking the test runner.
//!
//! The child is this very test binary re-executed with `DIFF_CHILD=1`; it
//! `dlopen`s whichever `.so` it is pointed at (C or Rust — never a direct Rust
//! call) and replays a small op script against its exported symbols.
//!
//! Uses `harness = false` so that `main()` can dispatch into the child role.

mod common;

use common::*;
use std::ffi::{c_int, CString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// op script
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Op {
    Bad,
    Good,
    Int(i32),
    Str(Vec<u8>),
    Null,
    Main,
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

fn encode_ops(ops: &[Op]) -> String {
    let mut s = String::new();
    for op in ops {
        match op {
            Op::Bad => s.push_str("bad\n"),
            Op::Good => s.push_str("good\n"),
            Op::Int(v) => s.push_str(&format!("int {v}\n")),
            Op::Str(b) => s.push_str(&format!("str {}\n", hex_encode(b))),
            Op::Null => s.push_str("null\n"),
            Op::Main => s.push_str("main\n"),
        }
    }
    s
}

fn decode_ops(text: &str) -> Vec<Op> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut it = l.splitn(2, ' ');
            let kind = it.next().unwrap();
            let arg = it.next().unwrap_or("");
            match kind {
                "bad" => Op::Bad,
                "good" => Op::Good,
                "int" => Op::Int(arg.parse().expect("int arg")),
                "str" => Op::Str(hex_decode(arg)),
                "null" => Op::Null,
                "main" => Op::Main,
                other => panic!("unknown op {other:?}"),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// child role
// ---------------------------------------------------------------------------

fn child_main() -> ! {
    let so = std::env::var("DIFF_SO").expect("DIFF_SO");
    let ops_path = std::env::var("DIFF_OPS").expect("DIFF_OPS");
    let ops = decode_ops(&std::fs::read_to_string(&ops_path).expect("read ops"));

    let lib = unsafe { libloading::Library::new(&so) }
        .unwrap_or_else(|e| panic!("dlopen {so}: {e}"));

    let mut last_code: c_int = 0;
    unsafe {
        let bad: libloading::Symbol<unsafe extern "C" fn()> = lib.get(b"bad\0").unwrap();
        let good: libloading::Symbol<unsafe extern "C" fn()> = lib.get(b"good\0").unwrap();
        let pil: libloading::Symbol<unsafe extern "C" fn(c_int)> =
            lib.get(b"printIntLine\0").unwrap();
        let pl: libloading::Symbol<unsafe extern "C" fn(*const std::ffi::c_char)> =
            lib.get(b"printLine\0").unwrap();
        let mainf: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
            lib.get(b"main\0").unwrap();

        for op in &ops {
            match op {
                Op::Bad => bad(),
                Op::Good => good(),
                Op::Int(v) => pil(*v),
                Op::Str(b) => {
                    let cs = CString::new(b.clone()).expect("NUL in str op");
                    pl(cs.as_ptr());
                }
                Op::Null => pl(std::ptr::null()),
                Op::Main => last_code = mainf(),
            }
        }
    }
    // `std::process::exit` runs libc `exit()`, which flushes the C stdio
    // buffers of the dlopened C object; the Rust cdylib's `stdout` is a
    // LineWriter that already flushed on each '\n'.
    std::io::stdout().flush().ok();
    std::process::exit(last_code as i32);
}

// ---------------------------------------------------------------------------
// parent helpers
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

fn run_child(so: &Path, ops: &[Op], stdin_bytes: &[u8]) -> Outcome {
    let ops_path = temp_path("ops");
    std::fs::write(&ops_path, encode_ops(ops)).expect("write ops");

    let mut child = Command::new(std::env::current_exe().expect("current_exe"))
        .env("DIFF_CHILD", "1")
        .env("DIFF_SO", so)
        .env("DIFF_OPS", &ops_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn child");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin_bytes)
        .ok();
    let out = child.wait_with_output().expect("wait child");
    let _ = std::fs::remove_file(&ops_path);
    outcome(out)
}

fn run_exe(exe: &Path, stdin_bytes: &[u8]) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin_bytes)
        .ok();
    outcome(child.wait_with_output().expect("wait"))
}

fn outcome(out: std::process::Output) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

#[track_caller]
fn assert_outcomes(row: &str, case: &str, c: &Outcome, r: &Outcome) {
    assert_same(row, &format!("{case} [stdout]"), &c.stdout, &r.stdout);
    assert!(
        c.code == r.code && c.signal == r.signal,
        "[{row}] {case}: exit status differs — C {{code: {:?}, signal: {:?}}} vs Rust {{code: {:?}, signal: {:?}}}\n  C stderr: {}\n  Rust stderr: {}",
        c.code,
        c.signal,
        r.code,
        r.signal,
        show(&c.stderr),
        show(&r.stderr)
    );
    assert!(
        c.stderr == r.stderr,
        "[{row}] {case}: stderr differs\n  C   : {}\n  Rust: {}",
        show(&c.stderr),
        show(&r.stderr)
    );
}

struct Ctx {
    c_so: PathBuf,
    rust_so: PathBuf,
}

impl Ctx {
    fn diff(&self, row: &str, case: &str, ops: &[Op], stdin_bytes: &[u8]) {
        let c = run_child(&self.c_so, ops, stdin_bytes);
        let r = run_child(&self.rust_so, ops, stdin_bytes);
        assert_outcomes(row, case, &c, &r);
    }

    /// Same as `diff` but also states what the C ground truth must be, so a
    /// silent "both broken the same way" cannot pass unnoticed.
    fn diff_expect(&self, row: &str, case: &str, ops: &[Op], stdin_bytes: &[u8], expect: &[u8]) {
        let c = run_child(&self.c_so, ops, stdin_bytes);
        assert_same(row, &format!("{case} [C vs contract]"), expect, &c.stdout);
        let r = run_child(&self.rust_so, ops, stdin_bytes);
        assert_outcomes(row, case, &c, &r);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows 16-19 / ERRORS rows 17-18 / G7 — bad() and good()
// ---------------------------------------------------------------------------

/// Row 16 + ERRORS row 17: `bad()` — `alloca(10)` under-allocation, 40-byte
/// copy, unchecked allocation. The C ground truth prints `0\n` and returns
/// normally (the out-of-bounds write does not surface as an error).
fn row16_bad_single_call(ctx: &Ctx) {
    ctx.diff_expect("row16/ERR17", "bad()", &[Op::Bad], b"", b"0\n");
}

/// Row 17 + ERRORS row 18: `good()` — correctly sized `alloca`, still unchecked.
fn row17_good_single_call(ctx: &Ctx) {
    ctx.diff_expect("row17/ERR18", "good()", &[Op::Good], b"", b"0\n");
}

/// Row 18 / G7: many repeated calls in one process.
fn row18_bad_and_good_repeated(ctx: &Ctx) {
    for n in [2usize, 5, 32, 256] {
        let bads: Vec<Op> = (0..n).map(|_| Op::Bad).collect();
        let expect: Vec<u8> = "0\n".repeat(n).into_bytes();
        ctx.diff_expect("row18/G7", &format!("bad() x {n}"), &bads, b"", &expect);

        let goods: Vec<Op> = (0..n).map(|_| Op::Good).collect();
        ctx.diff_expect("row18/G7", &format!("good() x {n}"), &goods, b"", &expect);

        let alt: Vec<Op> = (0..n)
            .map(|i| if i % 2 == 0 { Op::Bad } else { Op::Good })
            .collect();
        ctx.diff_expect(
            "row18/G7",
            &format!("alternating bad/good x {n}"),
            &alt,
            b"",
            &expect,
        );
    }
}

/// Row 19 / G7: randomized interleavings of all four printing entry points.
fn row19_interleaved_call_sequences(ctx: &Ctx) {
    let mut rng = Rng::new(SEED ^ 0x19);
    for seq in 0..8 {
        let ops: Vec<Op> = (0..200)
            .map(|_| match rng.below(5) {
                0 => Op::Bad,
                1 => Op::Good,
                2 => Op::Int(rng.next_i32()),
                3 => {
                    let len = rng.range_usize(0, 24);
                    Op::Str(rng.c_string_bytes(len))
                }
                _ => Op::Null,
            })
            .collect();
        ctx.diff(
            "row19/G7",
            &format!("interleaved sequence #{seq} (200 ops)"),
            &ops,
            b"",
        );
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows 20-36 / ERRORS rows 9-16 — main()
// ---------------------------------------------------------------------------

/// The stdin corpus lives in `common` so that the `main` rows, the executable
/// comparison (row 37) and the scanf probe all replay the identical inputs.
fn main_inputs() -> Vec<(&'static str, String, Vec<u8>)> {
    common::stdin_corpus()
}

fn rows20_to_36_main_via_dlsym(ctx: &Ctx) {
    for (row, case, input) in main_inputs() {
        // Both `good()` and `bad()` print `data[0]`, which is 0 in either
        // branch, so the C ground truth is `0\n` for every possible stdin.
        ctx.diff_expect(row, &format!("main <- {case}"), &[Op::Main], &input, b"0\n");
    }
}

/// Row 37 — the same corpus against the two compiled executables
/// (`add_executable(driver src/main.c)` vs `src/main.rs`).
fn row37_main_via_executables(_ctx: &Ctx) {
    let c_exe = c_exe_path();
    let r_exe = rust_exe_path();
    for (row, case, input) in main_inputs() {
        let c = run_exe(&c_exe, &input);
        let r = run_exe(&r_exe, &input);
        assert_same(
            "row37",
            &format!("{row} exe <- {case} [C vs contract]"),
            b"0\n",
            &c.stdout,
        );
        assert_outcomes("row37", &format!("{row} exe <- {case}"), &c, &r);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS row C1 — C optimisation-level sweep
// ---------------------------------------------------------------------------

#[allow(non_snake_case)]
fn rowC1_c_optimisation_level_sweep(_ctx: &Ctx) {
    let rust_so = rust_so_path();
    let mut rng = Rng::new(SEED ^ 0xC1);
    let ops: Vec<Op> = (0..40)
        .map(|_| match rng.below(5) {
            0 => Op::Bad,
            1 => Op::Good,
            2 => Op::Int(rng.next_i32()),
            3 => {
                let len = rng.range_usize(0, 20);
                Op::Str(rng.c_string_bytes(len))
            }
            _ => Op::Null,
        })
        .collect();
    let stdin_cases: [&[u8]; 6] = [b"", b"0", b"1", b"abc", b"99999999999999999999", b"4294967296"];

    for opt in ["", "-O0", "-O1", "-O2", "-Os"] {
        let c_so = build_c_so(opt);
        let tag = if opt.is_empty() { "cmake-default" } else { opt };

        let c = run_child(&c_so, &ops, b"");
        let r = run_child(&rust_so, &ops, b"");
        assert_outcomes("rowC1", &format!("gcc {tag}: 40 mixed ops"), &c, &r);

        for input in stdin_cases {
            let c = run_child(&c_so, &[Op::Main], input);
            let r = run_child(&rust_so, &[Op::Main], input);
            assert_same(
                "rowC1",
                &format!("gcc {tag}: main <- {:?} [C vs contract]", show(input)),
                b"0\n",
                &c.stdout,
            );
            assert_outcomes(
                "rowC1",
                &format!("gcc {tag}: main <- {:?}", show(input)),
                &c,
                &r,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS rows 38-40 — process termination under a hostile stdout
// ---------------------------------------------------------------------------
//
// These compare the *wait status* of the two executables, which is where the
// Rust runtime differs from a C program by default: `std` installs
// `SIGPIPE = SIG_IGN` before `main`, whereas c_src/src/main.c inherits the
// default disposition and is therefore killed by SIGPIPE when its stdout is a
// pipe with no reader. `src/main.rs` restores `SIG_DFL` to match.

use std::os::unix::io::FromRawFd;

fn run_exe_with_stdout(exe: &Path, stdin_bytes: &[u8], stdout: Stdio) -> Outcome {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(stdout)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin_bytes)
        .ok();
    outcome(child.wait_with_output().expect("wait"))
}

/// Row 38: stdout is a pipe whose reader is gone -> C dies with SIGPIPE (13).
fn row38_broken_stdout_pipe(_ctx: &Ctx) {
    let c_exe = c_exe_path();
    let r_exe = rust_exe_path();
    for input in [&b""[..], b"0", b"1", b"abc", b"99999999999999999999"] {
        let mk = || unsafe { Stdio::from_raw_fd(orphan_pipe_write_fd()) };
        let c = run_exe_with_stdout(&c_exe, input, mk());
        let r = run_exe_with_stdout(&r_exe, input, mk());
        assert!(
            c.signal == Some(13),
            "expected the C ground truth to be killed by SIGPIPE, got {c:?}"
        );
        assert_outcomes(
            "row38",
            &format!("broken stdout pipe, stdin={}", show(input)),
            &c,
            &r,
        );
    }
}

/// Row 39: stdout is `/dev/full` -> every write fails with ENOSPC. Neither
/// implementation checks `printf`'s return value, so both must still exit 0.
fn row39_stdout_enospc(_ctx: &Ctx) {
    let dev_full = Path::new("/dev/full");
    if !dev_full.exists() {
        println!("(skipped: no /dev/full) ");
        return;
    }
    let c_exe = c_exe_path();
    let r_exe = rust_exe_path();
    for input in [&b""[..], b"0", b"1"] {
        let open = || {
            Stdio::from(
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(dev_full)
                    .expect("open /dev/full"),
            )
        };
        let c = run_exe_with_stdout(&c_exe, input, open());
        let r = run_exe_with_stdout(&r_exe, input, open());
        assert_outcomes(
            "row39",
            &format!("stdout=/dev/full, stdin={}", show(input)),
            &c,
            &r,
        );
    }
}

/// Row 40: stdout discarded (`/dev/null`) -> no output, exit 0 for both.
fn row40_stdout_devnull(_ctx: &Ctx) {
    let c_exe = c_exe_path();
    let r_exe = rust_exe_path();
    for input in [&b""[..], b"0", b"1", b"-"] {
        let c = run_exe_with_stdout(&c_exe, input, Stdio::null());
        let r = run_exe_with_stdout(&r_exe, input, Stdio::null());
        assert_outcomes(
            "row40",
            &format!("stdout=/dev/null, stdin={}", show(input)),
            &c,
            &r,
        );
    }
}

/// Row 41: hostile / unusual *stdin* descriptors. `scanf` failing to read at
/// all is indistinguishable from EOF, so `x` keeps its `0` initializer and
/// `bad()` runs in every case.
fn row41_stdin_descriptor_shapes(_ctx: &Ctx) {
    use std::os::unix::process::CommandExt;

    let c_exe = c_exe_path();
    let r_exe = rust_exe_path();

    let run = |exe: &Path, setup: &dyn Fn(&mut Command)| -> Outcome {
        let mut cmd = Command::new(exe);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        setup(&mut cmd);
        outcome(cmd.output().expect("spawn"))
    };

    let cases: Vec<(&str, Box<dyn Fn(&mut Command)>)> = vec![
        (
            "stdin=/dev/null (immediate EOF)",
            Box::new(|c: &mut Command| {
                c.stdin(Stdio::null());
            }),
        ),
        (
            "stdin=/dev/zero (endless NUL bytes, not whitespace -> match failure)",
            Box::new(|c: &mut Command| {
                c.stdin(Stdio::from(
                    std::fs::File::open("/dev/zero").expect("open /dev/zero"),
                ));
            }),
        ),
        (
            "stdin closed (read fails with EBADF)",
            Box::new(|c: &mut Command| {
                unsafe {
                    c.pre_exec(|| {
                        close_fd(0);
                        Ok(())
                    })
                };
            }),
        ),
        (
            "stdin = write end of an orphan pipe (read fails with EBADF)",
            Box::new(|c: &mut Command| {
                let fd = orphan_pipe_write_fd();
                c.stdin(unsafe { Stdio::from_raw_fd(fd) });
            }),
        ),
    ];

    for (name, setup) in &cases {
        let c = run(&c_exe, setup.as_ref());
        let r = run(&r_exe, setup.as_ref());
        assert_same("row41", &format!("{name} [C vs contract]"), b"0\n", &c.stdout);
        assert_outcomes("row41", name, &c, &r);
    }
}

// ---------------------------------------------------------------------------
// mini harness (harness = false so that main() can become the child)
// ---------------------------------------------------------------------------

type TestFn = fn(&Ctx);

fn main() {
    if std::env::var_os("DIFF_CHILD").is_some() {
        child_main();
    }

    let ctx = Ctx {
        c_so: c_so_path(),
        rust_so: rust_so_path(),
    };

    let tests: &[(&str, TestFn)] = &[
        ("row16_bad_single_call", row16_bad_single_call),
        ("row17_good_single_call", row17_good_single_call),
        ("row18_bad_and_good_repeated", row18_bad_and_good_repeated),
        ("row19_interleaved_call_sequences", row19_interleaved_call_sequences),
        ("rows20_to_36_main_via_dlsym", rows20_to_36_main_via_dlsym),
        ("row37_main_via_executables", row37_main_via_executables),
        ("row38_broken_stdout_pipe", row38_broken_stdout_pipe),
        ("row39_stdout_enospc", row39_stdout_enospc),
        ("row40_stdout_devnull", row40_stdout_devnull),
        ("row41_stdin_descriptor_shapes", row41_stdin_descriptor_shapes),
        ("rowC1_c_optimisation_level_sweep", rowC1_c_optimisation_level_sweep),
    ];

    let filter: Option<String> = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .clone();

    println!("\nrunning {} tests", tests.len());
    let mut failed: Vec<(&str, String)> = Vec::new();
    let mut ran = 0usize;

    for (name, f) in tests {
        if let Some(ref pat) = filter {
            if !name.contains(pat.as_str()) {
                continue;
            }
        }
        ran += 1;
        print!("test {name} ... ");
        std::io::stdout().flush().ok();

        let prev = std::panic::take_hook();
        let store = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let store2 = store.clone();
        std::panic::set_hook(Box::new(move |info| {
            *store2.lock().unwrap() = format!("{info}");
        }));
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&ctx)));
        std::panic::set_hook(prev);

        match res {
            Ok(()) => println!("ok"),
            Err(_) => {
                println!("FAILED");
                failed.push((name, store.lock().unwrap().clone()));
            }
        }
    }

    if failed.is_empty() {
        println!("\ntest result: ok. {ran} passed; 0 failed\n");
    } else {
        println!("\nfailures:\n");
        for (name, msg) in &failed {
            println!("---- {name} ----\n{msg}\n");
        }
        println!(
            "test result: FAILED. {} passed; {} failed\n",
            ran - failed.len(),
            failed.len()
        );
        std::process::exit(101);
    }
}
