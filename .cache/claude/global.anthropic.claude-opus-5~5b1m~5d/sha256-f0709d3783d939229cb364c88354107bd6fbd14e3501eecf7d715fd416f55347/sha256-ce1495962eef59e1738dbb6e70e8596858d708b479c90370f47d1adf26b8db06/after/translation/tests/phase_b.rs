//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`.  Every row drives BOTH shared objects
//! through their exported `helloworld` symbol (never a Rust function directly)
//! and asserts the emitted bytes and return values match exactly, over many
//! seeded-random inputs.

mod common;

use common::*;
use std::ffi::c_int;

/// One scripted operation inside a captured run.
#[derive(Clone, Debug)]
enum Op {
    /// Call `helloworld` on the implementation under test.
    Call,
    /// The caller writes a token through the same `FILE*`.
    Stdio(String),
    /// The caller writes a token straight to fd 1.
    Raw(String),
}

fn expected(script: &[Op]) -> Vec<u8> {
    let mut out = Vec::new();
    for op in script {
        match op {
            Op::Call => out.extend_from_slice(LINE),
            Op::Stdio(s) | Op::Raw(s) => out.extend_from_slice(s.as_bytes()),
        }
    }
    out
}

fn play(script: &[Op], w: Which, ctx: &Ctx) -> Vec<c_int> {
    let mut rets = Vec::new();
    for op in script {
        match op {
            Op::Call => rets.push(unsafe { hello(w)() }),
            Op::Stdio(s) => ctx.stdio_write(s),
            Op::Raw(s) => ctx.raw_write(s),
        }
    }
    rets
}

// ---------------------------------------------------------------------------
// B1 — regular file, default buffering, single call
// ---------------------------------------------------------------------------
#[test]
fn b1_single_call_regular_file_default_buffering() {
    for i in 0..64 {
        let run = diff("B1", Sink::File, Buffering::Default, |w, _| unsafe {
            hello(w)()
        });
        assert_eq!(run.value, 0, "rep {i}: return value must be 0");
        assert_eq!(run.bytes, LINE, "rep {i}: bytes = {}", show(&run.bytes));
    }
}

// ---------------------------------------------------------------------------
// B2 — pipe sink
// ---------------------------------------------------------------------------
#[test]
fn b2_single_call_pipe_sink() {
    for i in 0..64 {
        let run = diff("B2", Sink::Pipe, Buffering::Default, |w, _| unsafe {
            hello(w)()
        });
        assert_eq!(run.value, 0, "rep {i}");
        assert_eq!(run.bytes, LINE, "rep {i}: bytes = {}", show(&run.bytes));
    }
}

// ---------------------------------------------------------------------------
// B3 — /dev/null: the write succeeds but the bytes are unobservable
// ---------------------------------------------------------------------------
#[test]
fn b3_dev_null_sink_return_value_only() {
    for i in 0..64 {
        let run = diff("B3", Sink::DevNull, Buffering::Default, |w, _| unsafe {
            hello(w)()
        });
        assert_eq!(run.value, 0, "rep {i}");
        assert!(run.bytes.is_empty());
        assert_eq!(run.ferror, 0, "rep {i}: writing to /dev/null must succeed");
    }
}

// ---------------------------------------------------------------------------
// B4 — N repeated calls, randomised N
// ---------------------------------------------------------------------------
#[test]
fn b4_repeated_calls_randomised_count() {
    let mut rng = Rng::new(0xB004);
    for _ in 0..48 {
        let n = rng.usize_range(1, 256);
        let run = diff("B4", Sink::File, Buffering::Default, |w, _| {
            (0..n).map(|_| unsafe { hello(w)() }).collect::<Vec<_>>()
        });
        assert!(run.value.iter().all(|&r| r == 0), "n={n}: all returns 0");
        assert_eq!(run.value.len(), n);
        assert_eq!(run.bytes, lines(n), "n={n}");
    }
}

// ---------------------------------------------------------------------------
// B5 — unbuffered stdout: each call becomes an immediate write(2)
// ---------------------------------------------------------------------------
#[test]
fn b5_unbuffered_stdout() {
    let mut rng = Rng::new(0xB005);
    for _ in 0..32 {
        let n = rng.usize_range(1, 64);
        let run = diff("B5", Sink::File, Buffering::NoneBuf, |w, ctx| {
            let mut rets = Vec::new();
            let mut mid = Vec::new();
            for i in 0..n {
                rets.push(unsafe { hello(w)() });
                if i == 0 {
                    // Proves the axis is real: with _IONBF the bytes are already
                    // in the file, before any fflush.
                    mid = ctx.snapshot();
                }
            }
            (rets, mid)
        });
        let (rets, mid) = &run.value;
        assert!(rets.iter().all(|&r| r == 0), "n={n}");
        assert_eq!(
            mid, &LINE.to_vec(),
            "n={n}: unbuffered mode did not take effect (mid-run snapshot = {})",
            show(mid)
        );
        assert_eq!(run.bytes, lines(n), "n={n}");
    }
}

// ---------------------------------------------------------------------------
// B6 — line-buffered stdout
// ---------------------------------------------------------------------------
#[test]
fn b6_line_buffered_stdout() {
    let mut rng = Rng::new(0xB006);
    for _ in 0..32 {
        let n = rng.usize_range(1, 64);
        let run = diff("B6", Sink::File, Buffering::Line, |w, ctx| {
            let mut rets = Vec::new();
            let mut mid = Vec::new();
            for i in 0..n {
                rets.push(unsafe { hello(w)() });
                if i == 0 {
                    mid = ctx.snapshot();
                }
            }
            (rets, mid)
        });
        let (rets, mid) = &run.value;
        assert!(rets.iter().all(|&r| r == 0), "n={n}");
        // The emitted text ends in '\n', so a line-buffered stream flushes it.
        assert_eq!(
            mid, &LINE.to_vec(),
            "n={n}: line buffering did not flush the newline (snapshot = {})",
            show(mid)
        );
        assert_eq!(run.bytes, lines(n), "n={n}");
    }
}

// ---------------------------------------------------------------------------
// B7 — tiny caller-supplied buffer: writes get split mid-line
// ---------------------------------------------------------------------------
#[test]
fn b7_tiny_fully_buffered_buffer() {
    let mut rng = Rng::new(0xB007);
    for _ in 0..32 {
        let sz = rng.usize_range(1, 8);
        let n = rng.usize_range(1, 64);
        let run = diff("B7", Sink::File, Buffering::FullTiny(sz), |w, _| {
            (0..n).map(|_| unsafe { hello(w)() }).collect::<Vec<_>>()
        });
        assert!(run.value.iter().all(|&r| r == 0), "sz={sz} n={n}");
        assert_eq!(run.bytes, lines(n), "sz={sz} n={n}");
    }
}

// ---------------------------------------------------------------------------
// B8 — randomised interleaving of C and Rust calls in ONE stream
// ---------------------------------------------------------------------------
#[test]
fn b8_interleaved_c_and_rust_calls() {
    let mut rng = Rng::new(0xB008);
    for round in 0..32 {
        let len = rng.usize_range(1, 64);
        let sched: Vec<Which> = (0..len)
            .map(|_| if rng.bool() { Which::C } else { Which::Rust })
            .collect();

        let run_with = |pick: &dyn Fn(usize) -> Which| {
            run_captured(Sink::File, Buffering::Default, |_| {
                (0..len)
                    .map(|i| unsafe { hello(pick(i))() })
                    .collect::<Vec<_>>()
            })
        };

        let mixed = run_with(&|i| sched[i]);
        let all_c = run_with(&|_| Which::C);
        let all_rust = run_with(&|_| Which::Rust);

        assert_eq!(
            show(&all_c.bytes),
            show(&all_rust.bytes),
            "round {round}: pure C and pure Rust streams differ"
        );
        assert_eq!(
            show(&mixed.bytes),
            show(&all_c.bytes),
            "round {round}: mixed stream differs from pure C (schedule {sched:?})"
        );
        assert_eq!(mixed.bytes, all_c.bytes);
        assert_eq!(mixed.bytes, all_rust.bytes);
        assert_eq!(mixed.bytes, lines(len));
        assert_eq!(mixed.value, all_c.value);
        assert_eq!(mixed.value, all_rust.value);
        assert!(mixed.value.iter().all(|&r| r == 0));
    }
}

// ---------------------------------------------------------------------------
// B9 — interleaved with the caller's own stdio writes
// ---------------------------------------------------------------------------
#[test]
fn b9_interleaved_with_caller_stdio_writes() {
    let mut rng = Rng::new(0xB009);
    for _ in 0..32 {
        let len = rng.usize_range(1, 24);
        let script: Vec<Op> = (0..len)
            .map(|_| {
                if rng.bool() {
                    Op::Call
                } else {
                    Op::Stdio(rng.token())
                }
            })
            .collect();
        let want = expected(&script);

        let run = diff("B9", Sink::File, Buffering::Default, |w, ctx| {
            play(&script, w, ctx)
        });
        assert!(run.value.iter().all(|&r| r == 0));
        assert_eq!(
            show(&run.bytes),
            show(&want),
            "script {script:?} produced the wrong interleaving"
        );
    }
}

// ---------------------------------------------------------------------------
// B10 — interleaved with the caller's raw write(2), stdout unbuffered
// ---------------------------------------------------------------------------
#[test]
fn b10_interleaved_with_raw_fd_writes_unbuffered() {
    let mut rng = Rng::new(0xB010);
    for _ in 0..32 {
        let len = rng.usize_range(1, 24);
        let script: Vec<Op> = (0..len)
            .map(|_| {
                if rng.bool() {
                    Op::Call
                } else {
                    Op::Raw(rng.token())
                }
            })
            .collect();
        let want = expected(&script);

        let run = diff("B10", Sink::File, Buffering::NoneBuf, |w, ctx| {
            play(&script, w, ctx)
        });
        assert!(run.value.iter().all(|&r| r == 0));
        assert_eq!(
            show(&run.bytes),
            show(&want),
            "script {script:?}: fd-level ordering differs"
        );
    }
}

// ---------------------------------------------------------------------------
// B11 — output must land at a non-zero file offset
// ---------------------------------------------------------------------------
#[test]
fn b11_non_zero_starting_offset() {
    let mut rng = Rng::new(0xB011);
    for _ in 0..32 {
        let prefix = rng.usize_range(0, 64);
        let n = rng.usize_range(1, 16);
        let run = diff(
            "B11",
            Sink::FileWithPrefix(prefix),
            Buffering::Default,
            |w, _| (0..n).map(|_| unsafe { hello(w)() }).collect::<Vec<_>>(),
        );
        let mut want = vec![b'.'; prefix];
        want.extend_from_slice(&lines(n));
        assert!(run.value.iter().all(|&r| r == 0));
        assert_eq!(run.bytes, want, "prefix={prefix} n={n}");
    }
}

// ---------------------------------------------------------------------------
// B12 — O_APPEND sink
// ---------------------------------------------------------------------------
#[test]
fn b12_append_mode_sink() {
    let mut rng = Rng::new(0xB012);
    for _ in 0..32 {
        let n = rng.usize_range(1, 32);
        let run = diff("B12", Sink::FileAppend, Buffering::Default, |w, _| {
            (0..n).map(|_| unsafe { hello(w)() }).collect::<Vec<_>>()
        });
        assert!(run.value.iter().all(|&r| r == 0));
        assert_eq!(run.bytes, lines(n), "n={n}");
    }
}

// ---------------------------------------------------------------------------
// B13 — concurrent calls: stdio locking must keep whole lines intact
// ---------------------------------------------------------------------------
#[test]
fn b13_concurrent_calls_from_many_threads() {
    let mut rng = Rng::new(0xB013);
    for _ in 0..8 {
        let threads = rng.usize_range(2, 8);
        let per_thread = rng.usize_range(1, 32);

        let mut results = Vec::new();
        for w in Which::BOTH {
            let run = run_captured(Sink::File, Buffering::Full, |_| {
                let f = hello(w);
                std::thread::scope(|s| {
                    let handles: Vec<_> = (0..threads)
                        .map(|_| {
                            s.spawn(move || (0..per_thread).map(|_| unsafe { f() }).collect::<Vec<_>>())
                        })
                        .collect();
                    handles
                        .into_iter()
                        .flat_map(|h| h.join().expect("worker thread panicked"))
                        .collect::<Vec<_>>()
                })
            });
            assert!(
                run.value.iter().all(|&r| r == 0),
                "{}: a concurrent call returned non-zero",
                w.name()
            );
            assert_eq!(run.value.len(), threads * per_thread);
            // Every 13-byte chunk must be a complete, untorn line.
            assert_eq!(
                run.bytes.len(),
                threads * per_thread * LINE.len(),
                "{}: wrong byte count (threads={threads} per_thread={per_thread})",
                w.name()
            );
            assert!(
                run.bytes.chunks(LINE.len()).all(|c| c == LINE),
                "{}: output contains a torn line",
                w.name()
            );
            results.push(run.bytes);
        }
        assert_eq!(
            results[0], results[1],
            "concurrent output differs (threads={threads} per_thread={per_thread})"
        );
    }
}

// ---------------------------------------------------------------------------
// B14 — idempotence: no hidden state, always 0
// ---------------------------------------------------------------------------
#[test]
fn b14_idempotence_over_many_calls() {
    const N: usize = 4096;
    let run = diff("B14", Sink::File, Buffering::Default, |w, _| {
        (0..N).map(|_| unsafe { hello(w)() }).collect::<Vec<_>>()
    });
    assert_eq!(run.value, vec![0; N]);
    assert_eq!(run.bytes.len(), N * LINE.len());
    assert_eq!(run.bytes, lines(N));
}

// ---------------------------------------------------------------------------
// B15 — happy path through the unprototyped extra-argument signature
// ---------------------------------------------------------------------------
#[test]
fn b15_extra_argument_call_signature_happy_path() {
    let mut rng = Rng::new(0xB015);
    let extremes = [0i32, -1, 1, i32::MIN, i32::MAX, 0x7f7f_7f7f, -0x8000_0000i64 as i32];
    for i in 0..48 {
        let args = if i < extremes.len() {
            let v = extremes[i];
            (v, v, v, v, v as f64)
        } else {
            (rng.i32(), rng.i32(), rng.i32(), rng.i32(), rng.i32() as f64)
        };
        let run = diff("B15", Sink::File, Buffering::Default, |w, _| unsafe {
            hello_extra_args(w)(args.0, args.1, args.2, args.3, args.4)
        });
        assert_eq!(run.value, 0, "args={args:?}");
        assert_eq!(run.bytes, LINE, "args={args:?}");
    }
}

// ---------------------------------------------------------------------------
// B16 — both .so's resident at once, strictly alternating
// ---------------------------------------------------------------------------
#[test]
fn b16_both_libraries_resident_alternating() {
    // Distinct code in the process: the two symbols must not be the same
    // address, otherwise one library shadowed the other and every "differential"
    // comparison would be vacuous.
    let c = hello(Which::C) as usize;
    let r = hello(Which::Rust) as usize;
    assert_ne!(
        c, r,
        "the C and Rust `helloworld` resolved to the same address — \
         the two .so's are not independently loaded"
    );

    const PAIRS: usize = 32;
    let run = run_captured(Sink::File, Buffering::Default, |_| {
        let mut rets = Vec::new();
        for _ in 0..PAIRS {
            rets.push(unsafe { hello(Which::C)() });
            rets.push(unsafe { hello(Which::Rust)() });
        }
        rets
    });
    assert_eq!(run.value, vec![0; PAIRS * 2]);
    assert_eq!(run.bytes, lines(PAIRS * 2));

    // And the same total, produced by each library on its own.
    for w in Which::BOTH {
        let solo = run_captured(Sink::File, Buffering::Default, |_| {
            (0..PAIRS * 2).map(|_| unsafe { hello(w)() }).collect::<Vec<_>>()
        });
        assert_eq!(
            solo.bytes, run.bytes,
            "{}-only stream differs from the alternating stream",
            w.name()
        );
    }
}

// ---------------------------------------------------------------------------
// B17 — a fresh dlsym before every call
// ---------------------------------------------------------------------------
#[test]
fn b17_fresh_dlsym_per_call() {
    const N: usize = 64;
    let run = diff("B17", Sink::File, Buffering::Default, |w, _| {
        let mut rets = Vec::new();
        let mut addrs = std::collections::BTreeSet::new();
        for _ in 0..N {
            let f = resolve(w); // dlsym every time
            addrs.insert(f as usize);
            rets.push(unsafe { f() });
        }
        // dlsym must be stable for a loaded library.
        assert_eq!(addrs.len(), 1, "{}: dlsym returned varying addresses", w.name());
        rets
    });
    assert_eq!(run.value, vec![0; N]);
    assert_eq!(run.bytes, lines(N));
}
