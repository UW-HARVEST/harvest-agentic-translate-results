//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! The C library validates nothing (it has no parameters and no branches), so
//! these rows cover (a) the failure modes it really has — the discarded result
//! of `printf` — and (b) the generic FFI-boundary boundaries: null pointers,
//! zero/oversized lengths, values one past any plausible range and out-of-range
//! "enum" values.
//!
//! Every test asserts the *exact* value both libraries return (never merely
//! "both failed") plus the exact bytes each produced.
//!
//! Run with `-- --test-threads=1` (see `verify.sh`).

mod common;

use common::*;

/// Rows 1–4 share this shape: fd 1 is a destination on which `write(2)` fails,
/// `printf` therefore fails, and `helloworld` must still return 0 and the
/// process must still exit 0 — for both libraries.
///
/// Each destination is exercised in four buffering modes, because the mode
/// decides *when* the failure is observed:
///
/// * default (fully buffered on a file/char device/pipe): the `printf` call
///   itself succeeds and only the final `fflush` fails;
/// * `_IONBF`: `printf` performs the `write` and **returns a negative value**;
/// * `_IOLBF`: the trailing `\n` flushes inside the call, so `printf` fails too;
/// * `_IOFBF` with an 8-byte buffer (shorter than the 13-byte line): `printf`
///   must flush mid-call and fails.
///
/// The last three are what prove that C's *discarded* `printf` result is also
/// discarded by the translation — with only the default mode, a version that
/// returned `-1` on a failed `printf` would look identical.
fn assert_write_failure_returns_zero(row: &str, dest: Dest, ignore_sigpipe: bool) {
    let mut rng = Rng::new(0xE0_0000 ^ dest as u64);
    let modes: [Option<(std::os::raw::c_int, usize)>; 4] =
        [None, Some((IONBF, 0)), Some((IOLBF, 0)), Some((IOFBF, 8))];
    for mode in modes {
        for _ in 0..4 {
            let n = rng.range(1, 16) as usize;
            let steps: Vec<Step> = (0..n).map(|_| Step::hello()).collect();
            let mut opts = RunOpts::dest(dest);
            opts.setvbuf_first = mode;
            if ignore_sigpipe {
                opts = opts.ignoring_sigpipe();
            }
            let r = assert_same(row, &steps, &opts);
            assert_eq!(
                r.rets,
                vec![0; n],
                "[{row}] a failing printf changed the return value (buffering {:?})",
                mode
            );
            assert_eq!(
                r.exit, 0,
                "[{row}] a failing printf changed the exit status (buffering {:?})",
                mode
            );
            assert!(r.bytes.is_empty(), "[{row}] bytes escaped to a rejecting fd");
        }
    }
}

/// Row 1 — `printf` fails with `ENOSPC` (`/dev/full`).
#[test]
fn err01_helloworld_enospc_dev_full() {
    assert_write_failure_returns_zero("err01", Dest::DevFull, false);
}

/// Row 2 — `printf` fails with `EBADF` (fd 1 closed).
#[test]
fn err02_helloworld_ebadf_closed_fd1() {
    assert_write_failure_returns_zero("err02", Dest::Closed, false);
}

/// Row 3 — `printf` fails with `EBADF` (fd 1 is an `O_RDONLY` description).
#[test]
fn err03_helloworld_ebadf_readonly_fd1() {
    assert_write_failure_returns_zero("err03", Dest::ReadOnly, false);
}

/// Row 4 — `printf` fails with `EPIPE` (pipe with no reader). `SIGPIPE` is set
/// to `SIG_IGN` so the failure surfaces as an error instead of a signal; both
/// libraries must survive and return 0.
#[test]
fn err04_helloworld_epipe_broken_pipe() {
    assert_write_failure_returns_zero("err04", Dest::BrokenPipe, true);
}

/// Row 16 (library level) — the same broken pipe with `SIGPIPE` at its **default**
/// disposition, i.e. what a C program starts with: the process must be killed by
/// signal 13 in both cases, at the same point (before any call could return).
#[test]
fn err16_broken_pipe_default_sigpipe_kills_process() {
    for n in [1usize, 2, 5] {
        let steps: Vec<Step> = (0..n).map(|_| Step::hello()).collect();
        let opts = RunOpts::dest(Dest::BrokenPipe).default_sigpipe();
        let c = run(&steps, [c_fns(), c_fns()], &opts);
        let r = run(&steps, [rust_fns(), rust_fns()], &opts);
        assert_eq!(c.exit, -13, "err16: C was expected to die from SIGPIPE, got {}", c.exit);
        assert_eq!(r.exit, c.exit, "err16: exit status differs (C {} vs Rust {})", c.exit, r.exit);
        assert_eq!(r.rets, c.rets, "err16: return values differ");
    }
}

/// Row 17 — whole program with stdout on a pipe that has no reader.
///
/// This is where a Rust translation silently diverges: the Rust runtime installs
/// `SIG_IGN` for `SIGPIPE` before `main`, so an unmodified binary would exit 0
/// where `c_src`'s `driver` is killed by signal 13. `src/main.rs` restores the C
/// disposition; this test is the regression guard.
#[test]
fn err17_program_broken_pipe_status_matches() {
    for _ in 0..4 {
        let mut results = Vec::new();
        for exe in [c_exe_path(), rust_exe_path()] {
            results.push(run_program_with_broken_stdout(&exe));
        }
        assert_eq!(
            results[0], -13,
            "err17: the C program was expected to die from SIGPIPE, got {}",
            results[0]
        );
        assert_eq!(
            results[1], results[0],
            "err17: program exit status differs on a broken stdout pipe (C {} vs Rust {})",
            results[0], results[1]
        );
    }
}

/// Spawns `exe` with fd 1 on a pipe whose read end is already closed and returns
/// its `WEXITSTATUS`, or `-signal` if it was killed.
fn run_program_with_broken_stdout(exe: &std::path::Path) -> i32 {
    use std::os::fd::FromRawFd;
    unsafe {
        let mut fds = [-1 as std::os::raw::c_int; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() failed");
        close(fds[0]); // no reader at all
        let w = std::fs::File::from_raw_fd(fds[1]);
        let status = std::process::Command::new(exe)
            .stdout(std::process::Stdio::from(w))
            .status()
            .expect("spawn driver");
        match (status.code(), std::os::unix::process::ExitStatusExt::signal(&status)) {
            (Some(c), _) => c,
            (None, Some(s)) => -s,
            _ => panic!("neither code nor signal"),
        }
    }
}

/// Row 5 — retry after a failed call. glibc leaves the stream's error indicator
/// set, so later calls fail too; the return value must stay 0 regardless, and
/// the sequence of return values must match exactly.
#[test]
fn err05_helloworld_repeat_after_failure() {
    for dest in [Dest::DevFull, Dest::ReadOnly, Dest::Closed] {
        // No clearerr in between: the error state is deliberately sticky.
        let mut steps = Vec::new();
        for _ in 0..12 {
            steps.push(Step::hello());
            steps.push(Step::Flush);
        }
        for mode in [None, Some((IONBF, 0)), Some((IOFBF, 8))] {
            let mut opts = RunOpts::dest(dest);
            opts.setvbuf_first = mode;
            let r = assert_same("err05", &steps, &opts);
            assert_eq!(
                r.rets,
                vec![0; 12],
                "err05: sticky error state changed a return value (buffering {:?})",
                mode
            );
            assert_eq!(r.exit, 0);
        }
    }
}

/// Row 6 — the same failures through `main`, whose return value *is*
/// `helloworld`'s and becomes the process exit status.
#[test]
fn err06_main_write_failures_propagate_zero() {
    for dest in [Dest::DevFull, Dest::ReadOnly, Dest::Closed, Dest::BrokenPipe] {
        let steps = [Step::main_(), Step::main_(), Step::main_()];
        for mode in [None, Some((IONBF, 0)), Some((IOLBF, 0)), Some((IOFBF, 8))] {
            let mut opts = RunOpts::dest(dest).ignoring_sigpipe();
            opts.setvbuf_first = mode;
            let r = assert_same("err06", &steps, &opts);
            assert_eq!(
                r.rets,
                vec![0; 3],
                "err06: main propagated a non-zero value (buffering {:?})",
                mode
            );
            assert_eq!(r.exit, 0, "err06: exit status changed on write failure ({:?})", mode);
        }
    }
}

/// Row 7 — extra arguments across the FFI boundary (legal for the unprototyped
/// `int helloworld();`): they must be ignored, output unchanged, return 0.
#[test]
fn err07_extra_args_ignored() {
    let mut rng = Rng::new(0x0707_E4E4);
    for _ in 0..24 {
        let shapes = [
            ArgShape::Int(rng.next_u64() as c_int_alias),
            ArgShape::TwoInts(rng.next_u64() as c_int_alias, rng.next_u64() as c_int_alias),
            ArgShape::Size(rng.next_u64() as usize),
            ArgShape::SixInts([
                rng.next_u64() as c_int_alias,
                rng.next_u64() as c_int_alias,
                rng.next_u64() as c_int_alias,
                rng.next_u64() as c_int_alias,
                rng.next_u64() as c_int_alias,
                rng.next_u64() as c_int_alias,
            ]),
            ArgShape::Mixed {
                i: rng.next_u64() as c_int_alias,
                // A pointer value that must never be dereferenced.
                p: rng.next_u64() as usize,
                d: f64::from_bits(rng.next_u64()),
                u: rng.next_u64(),
                f: f32::from_bits(rng.next_u64() as u32),
                extra: [
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                    rng.next_u64() as i64,
                ],
            },
        ];
        let steps: Vec<Step> = shapes
            .iter()
            .cloned()
            .map(|a| Step::call_with(Entry::Hello, a))
            .collect();
        assert_same_and_expect(
            "err07",
            &steps,
            &RunOpts::default(),
            &hello_repeated(steps.len()),
            steps.len(),
        );
    }
}

/// Row 8 — `main` called with the conventional `(argc, argv)` shape, including
/// `argc = -1` (out of range) and `argv = NULL`.
#[test]
fn err08_main_extra_args_and_null_argv() {
    let mut rng = Rng::new(0x0808_E4E4);
    let mut steps = vec![
        Step::call_with(Entry::Main, ArgShape::IntPtr(0, 0)),
        Step::call_with(Entry::Main, ArgShape::IntPtr(-1, 0)),
        Step::call_with(Entry::Main, ArgShape::IntPtr(c_int_alias::MIN, 0)),
        Step::call_with(Entry::Main, ArgShape::IntPtr(c_int_alias::MAX, usize::MAX)),
        Step::call_with(Entry::Main, ArgShape::IntPtr(1, 1)),
    ];
    for _ in 0..8 {
        steps.push(Step::call_with(
            Entry::Main,
            ArgShape::IntPtr(rng.next_u64() as c_int_alias, rng.next_u64() as usize),
        ));
    }
    let n = steps.len();
    assert_same_and_expect("err08", &steps, &RunOpts::default(), &hello_repeated(n), n);
}

/// Row 9 — null and garbage pointer arguments. Neither function takes a
/// pointer, so none may ever be dereferenced: no segfault, return 0.
#[test]
fn err09_null_and_garbage_pointer_args() {
    let mut ptrs: Vec<usize> = vec![0, 1, 8, usize::MAX, usize::MAX - 7, 0xdead_beef, 0xffff_8000_0000_0000];
    let mut rng = Rng::new(0x0909_E4E4);
    for _ in 0..8 {
        ptrs.push(rng.next_u64() as usize);
    }
    let mut steps = Vec::new();
    for p in ptrs {
        steps.push(Step::call_with(Entry::Hello, ArgShape::Ptr(p)));
        steps.push(Step::call_with(Entry::Main, ArgShape::Ptr(p)));
    }
    let n = steps.len();
    assert_same_and_expect("err09", &steps, &RunOpts::default(), &hello_repeated(n), n);
}

/// Row 10 — out-of-range "enum" values. The C API declares no enum, so the
/// generic case is covered: an `int` with no valid variant, including the
/// extremes and one step past them.
#[test]
fn err10_out_of_range_enum_values() {
    let values: [c_int_alias; 12] = [
        c_int_alias::MIN,
        c_int_alias::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        255,
        256,
        0x7fff_fffe,
        c_int_alias::MAX,
        0xdead_beefu32 as c_int_alias,
    ];
    let mut steps = Vec::new();
    for v in values {
        steps.push(Step::call_with(Entry::Hello, ArgShape::Int(v)));
        steps.push(Step::call_with(Entry::Main, ArgShape::Int(v)));
    }
    let n = steps.len();
    assert_same_and_expect("err10", &steps, &RunOpts::default(), &hello_repeated(n), n);
}

/// Row 11 — zero and oversized length arguments.
#[test]
fn err11_zero_and_oversized_lengths() {
    let lens: [usize; 8] = [
        0,
        1,
        usize::MAX,
        usize::MAX - 1,
        isize::MAX as usize,
        isize::MAX as usize + 1, // isize::MIN reinterpreted
        1 << 62,
        u32::MAX as usize,
    ];
    let mut steps = Vec::new();
    for l in lens {
        steps.push(Step::call_with(Entry::Hello, ArgShape::Size(l)));
        steps.push(Step::call_with(Entry::Main, ArgShape::Size(l)));
    }
    let n = steps.len();
    assert_same_and_expect("err11", &steps, &RunOpts::default(), &hello_repeated(n), n);
}

/// Row 12 — the return value read through a `long`-returning signature: the C
/// `int` half must be 0 for both. (The upper 32 bits are ABI-undefined for a
/// function returning `int`, so they are deliberately not compared.)
#[test]
fn err12_return_value_is_c_int_zero() {
    let steps: Vec<Step> = (0..8)
        .map(|i| {
            if i % 2 == 0 {
                Step::call_with(Entry::Hello, ArgShape::RetLong)
            } else {
                Step::call_with(Entry::Main, ArgShape::RetLong)
            }
        })
        .collect();
    let n = steps.len();
    assert_same_and_expect("err12", &steps, &RunOpts::default(), &hello_repeated(n), n);
    // Same through the declared signature, on a destination where the write fails.
    let plain: Vec<Step> = (0..4).map(|_| Step::hello()).collect();
    let r = assert_same("err12b", &plain, &RunOpts::dest(Dest::DevFull));
    assert_eq!(r.rets, vec![0; 4]);
}

/// Row 13 — concurrent calls while the stream is in an error state: no partial
/// lines, no failure return, identical behaviour for both libraries.
#[test]
fn err13_concurrent_calls_no_partial_lines() {
    let mut rng = Rng::new(0x1313_E4E4);
    for dest in [Dest::DevFull, Dest::Closed, Dest::ReadOnly] {
        let threads = rng.range(2, 6) as usize;
        let per_thread = rng.range(1, 8) as usize;
        let opts = RunOpts::dest(dest);
        let c = run_threaded(c_fns(), threads, per_thread, &opts);
        let r = run_threaded(rust_fns(), threads, per_thread, &opts);
        assert_eq!(c.bytes, r.bytes, "err13: bytes differ ({:?})", dest);
        // The exit code counts calls that returned non-zero: must be 0.
        assert_eq!(c.exit, 0, "err13: C returned non-zero under concurrency ({:?})", dest);
        assert_eq!(r.exit, c.exit, "err13: exit status differs ({:?})", dest);
    }
}

/// Row 14 — the same error paths after `dlclose` + `dlopen`.
#[test]
fn err14_reload_library() {
    let cpath = c_lib_path();
    let rpath = rust_lib_path();
    for dest in [Dest::DevFull, Dest::Closed, Dest::BrokenPipe] {
        for _round in 0..2 {
            let (clib, cf) = open_lib(&cpath);
            let (rlib, rf) = open_lib(&rpath);
            let steps = [Step::hello(), Step::main_(), Step::hello()];
            let opts = RunOpts::dest(dest).ignoring_sigpipe();
            let c = run(&steps, [cf, cf], &opts);
            let r = run(&steps, [rf, rf], &opts);
            assert_eq!(c.rets, vec![0; 3], "err14: C returned non-zero after reload");
            assert_eq!(r.rets, c.rets, "err14: return values differ after reload ({:?})", dest);
            assert_eq!(r.bytes, c.bytes, "err14: bytes differ after reload ({:?})", dest);
            assert_eq!(r.exit, c.exit, "err14: exit status differs after reload ({:?})", dest);
            drop(clib);
            drop(rlib);
        }
    }
}

/// Generic boundary: a symbol that does *not* exist must be absent from both
/// libraries, so a consumer gets the same `dlsym` failure either way. This
/// guards against the Rust side exporting extra look-alike names or a stub.
#[test]
fn err15_absent_symbols_are_absent_in_both() {
    let cpath = c_lib_path();
    let rpath = rust_lib_path();
    for name in [
        "helloworld_",
        "_helloworld",
        "HelloWorld",
        "hello_world",
        "sillymain",
        "driver_main",
    ] {
        let mut sym = name.as_bytes().to_vec();
        sym.push(0);
        unsafe {
            let clib = libloading::Library::new(&cpath).expect("dlopen C");
            let rlib = libloading::Library::new(&rpath).expect("dlopen Rust");
            let c_found = clib.get::<CFn>(&sym).is_ok();
            let r_found = rlib.get::<CFn>(&sym).is_ok();
            assert!(!c_found, "err15: C unexpectedly exports {name}");
            assert_eq!(c_found, r_found, "err15: only one library exports {name}");
        }
    }
}

use std::os::raw::c_int as c_int_alias;
