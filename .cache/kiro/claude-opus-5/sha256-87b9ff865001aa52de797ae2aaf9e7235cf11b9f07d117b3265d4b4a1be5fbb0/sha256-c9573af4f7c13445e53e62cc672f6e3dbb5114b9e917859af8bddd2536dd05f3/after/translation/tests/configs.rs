//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every call on both sides goes through
//! `dlsym` on the respective `.so`; nothing is called as a Rust function.

mod common;

use common::*;
use std::ffi::{c_double, c_int, c_void};

/// Runs `body` once against C and once against Rust under an identical
/// capture configuration and asserts the captured bytes and any returned
/// value match exactly.
fn differential<T: std::fmt::Debug + PartialEq>(
    label: &str,
    sink: Sink,
    buffering: Buffering,
    mut body: impl FnMut(&Pair, Impl) -> T,
) {
    let pair = load_pair();
    let (c_ret, c_out) = capture(sink, buffering, || body(&pair, Impl::C));
    let (r_ret, r_out) = capture(sink, buffering, || body(&pair, Impl::Rust));
    assert_eq!(
        c_ret, r_ret,
        "{label}: returned values differ (C={c_ret:?}, Rust={r_ret:?})"
    );
    assert_same_bytes(label, &c_out, &r_out);
}

/// Convenience: call `helloworld` `n` times through the `.so` and collect the
/// return values.
fn call_n(pair: &Pair, which: Impl, n: usize) -> Vec<c_int> {
    let f = pair.helloworld(which);
    (0..n).map(|_| unsafe { f() }).collect()
}

// ---------------------------------------------------------------------- C1
#[test]
fn c1_single_call_return_value() {
    let pair = load_pair();
    // Captured so the message does not pollute the test log; the assertion
    // here is about the return value.
    let (c_ret, _) = capture(Sink::File, Buffering::Default, || unsafe {
        pair.helloworld(Impl::C)()
    });
    let (r_ret, _) = capture(Sink::File, Buffering::Default, || unsafe {
        pair.helloworld(Impl::Rust)()
    });
    assert_same_ret("C1", c_ret, r_ret);
    // The C body is `return 0;`, unconditionally.
    assert_eq!(c_ret, 0, "C1: the C implementation must return 0");
}

// ---------------------------------------------------------------------- C2
#[test]
fn c2_single_call_stdout_to_regular_file() {
    differential("C2", Sink::File, Buffering::Default, |p, w| {
        call_n(p, w, 1)
    });
    // Also pin the absolute expected bytes, so a translation that changed the
    // message in *both* directions could not pass by symmetry alone.
    let pair = load_pair();
    for which in [Impl::C, Impl::Rust] {
        let (_, out) = capture(Sink::File, Buffering::Default, || call_n(&pair, which, 1));
        assert_eq!(
            out,
            EXPECTED_LINE,
            "C2/{}: expected {:?}, got {:?}",
            which.name(),
            show(EXPECTED_LINE),
            show(&out)
        );
    }
}

// ---------------------------------------------------------------------- C3
#[test]
fn c3_zero_calls_produces_no_output() {
    differential("C3", Sink::File, Buffering::Default, |p, w| {
        call_n(p, w, 0)
    });
    let pair = load_pair();
    for which in [Impl::C, Impl::Rust] {
        let (rets, out) = capture(Sink::File, Buffering::Default, || call_n(&pair, which, 0));
        assert!(rets.is_empty());
        assert!(
            out.is_empty(),
            "C3/{}: zero calls must emit nothing, got {:?}",
            which.name(),
            show(&out)
        );
    }
}

// ---------------------------------------------------------------------- C4
#[test]
fn c4_many_calls_randomized_counts() {
    let mut rng = Rng::new(SEED ^ 0xC4);
    for iter in 0..40 {
        let n = rng.range(1, 64) as usize;
        differential(
            &format!("C4[iter={iter},n={n}]"),
            Sink::File,
            Buffering::Default,
            |p, w| call_n(p, w, n),
        );
        // Statelessness: exactly n repetitions of the line, nothing more.
        let pair = load_pair();
        let (rets, out) = capture(Sink::File, Buffering::Default, || call_n(&pair, Impl::Rust, n));
        assert!(rets.iter().all(|&r| r == 0), "C4: all returns must be 0");
        assert_eq!(
            out.len(),
            EXPECTED_LINE.len() * n,
            "C4[n={n}]: expected {n} lines, got {:?}",
            show(&out)
        );
    }
}

// ------------------------------------------------------------------ C5/C6/C7
#[test]
fn c5_fully_buffered_stdout() {
    let mut rng = Rng::new(SEED ^ 0xC5);
    for iter in 0..12 {
        let cap = rng.range(2, 4096) as usize;
        let n = rng.range(1, 24) as usize;
        differential(
            &format!("C5[iter={iter},bufcap={cap},n={n}]"),
            Sink::File,
            Buffering::Full(cap),
            |p, w| call_n(p, w, n),
        );
    }
}

#[test]
fn c6_line_buffered_stdout() {
    let mut rng = Rng::new(SEED ^ 0xC6);
    for iter in 0..12 {
        let cap = rng.range(2, 4096) as usize;
        let n = rng.range(1, 24) as usize;
        differential(
            &format!("C6[iter={iter},bufcap={cap},n={n}]"),
            Sink::File,
            Buffering::Line(cap),
            |p, w| call_n(p, w, n),
        );
    }
}

#[test]
fn c7_unbuffered_stdout() {
    let mut rng = Rng::new(SEED ^ 0xC7);
    for iter in 0..12 {
        let n = rng.range(1, 24) as usize;
        differential(
            &format!("C7[iter={iter},n={n}]"),
            Sink::File,
            Buffering::None,
            |p, w| call_n(p, w, n),
        );
    }
}

// ---------------------------------------------------------------------- C8
#[test]
fn c8_destination_is_a_pipe() {
    let mut rng = Rng::new(SEED ^ 0xC8);
    for iter in 0..12 {
        let n = rng.range(1, 32) as usize;
        differential(
            &format!("C8[iter={iter},n={n}]"),
            Sink::Pipe,
            Buffering::Default,
            |p, w| call_n(p, w, n),
        );
    }
    // A pipe large enough to force multiple write(2) flushes.
    differential("C8[large]", Sink::Pipe, Buffering::None, |p, w| {
        call_n(p, w, 512)
    });
}

// ---------------------------------------------------------------------- C9
#[test]
fn c9_interleaved_with_caller_side_printf() {
    let mut rng = Rng::new(SEED ^ 0xC9);
    for iter in 0..20 {
        let n = rng.range(1, 12) as usize;
        // Pre-generate the payloads so both sides see identical caller output.
        let pre: Vec<Vec<u8>> = (0..n).map(|_| { let l = rng_len(&mut rng); rng.ascii(l) }).collect();
        let post: Vec<Vec<u8>> = (0..n).map(|_| { let l = rng_len(&mut rng); rng.ascii(l) }).collect();
        let buffering = match iter % 3 {
            0 => Buffering::Default,
            1 => Buffering::Full(64),
            _ => Buffering::None,
        };
        differential(
            &format!("C9[iter={iter},n={n},{buffering:?}]"),
            Sink::File,
            buffering,
            |p, w| {
                let f = p.helloworld(w);
                let mut rets = Vec::new();
                for i in 0..n {
                    printf_bytes(&pre[i]);
                    rets.push(unsafe { f() });
                    printf_bytes(&post[i]);
                }
                rets
            },
        );
    }
}

// --------------------------------------------------------------------- C10
#[test]
fn c10_interleaved_with_raw_write_bypassing_the_file_buffer() {
    // Raw write(2) goes straight to the fd. If an implementation buffered its
    // output somewhere other than libc's `stdout`, the relative order of the
    // marker and the message would differ between C and Rust.
    let mut rng = Rng::new(SEED ^ 0xCA);
    for iter in 0..20 {
        let n = rng.range(1, 8) as usize;
        let marks: Vec<Vec<u8>> = (0..n).map(|_| { let l = rng_len(&mut rng); rng.ascii(l) }).collect();
        let buffering = if iter % 2 == 0 {
            Buffering::None
        } else {
            Buffering::Line(128)
        };
        differential(
            &format!("C10[iter={iter},n={n},{buffering:?}]"),
            Sink::File,
            buffering,
            |p, w| {
                let f = p.helloworld(w);
                let mut rets = Vec::new();
                for i in 0..n {
                    unsafe {
                        // Flush first so the ordering is well defined for both.
                        libc::fflush(std::ptr::null_mut());
                        libc::write(1, marks[i].as_ptr() as *const c_void, marks[i].len());
                    }
                    rets.push(unsafe { f() });
                    unsafe { libc::fflush(std::ptr::null_mut()) };
                }
                rets
            },
        );
    }
}

// --------------------------------------------------------------------- C11
#[test]
fn c11_c_and_rust_alternating_into_the_same_stream() {
    let mut rng = Rng::new(SEED ^ 0xCB);
    let pair = load_pair();
    for iter in 0..20 {
        let n = rng.range(2, 40) as usize;
        // A random schedule of which implementation writes each line.
        let schedule: Vec<Impl> = (0..n)
            .map(|_| if rng.bool() { Impl::C } else { Impl::Rust })
            .collect();
        let (rets, out) = capture(Sink::File, Buffering::Default, || {
            let c = pair.helloworld(Impl::C);
            let r = pair.helloworld(Impl::Rust);
            schedule
                .iter()
                .map(|&w| unsafe {
                    match w {
                        Impl::C => c(),
                        Impl::Rust => r(),
                    }
                })
                .collect::<Vec<_>>()
        });
        assert!(
            rets.iter().all(|&x| x == 0),
            "C11[iter={iter}]: every call must return 0, got {rets:?}"
        );
        // Every line must be indistinguishable regardless of which
        // implementation produced it.
        let expected: Vec<u8> = EXPECTED_LINE.repeat(n);
        assert_same_bytes(
            &format!("C11[iter={iter},n={n},schedule={schedule:?}]"),
            &expected,
            &out,
        );
    }
}

// --------------------------------------------------------------------- C12
#[test]
fn c12_concurrent_calls_from_many_threads() {
    let mut rng = Rng::new(SEED ^ 0xCC);
    for iter in 0..8 {
        let threads = rng.range(2, 8) as usize;
        let per = rng.range(1, 16) as usize;
        let total = threads * per;

        for which in [Impl::C, Impl::Rust] {
            let pair = load_pair();
            // Raw address so it can cross the thread boundary.
            let addr = pair.helloworld_addr(which) as usize;
            let (rets, out) = capture(Sink::File, Buffering::Default, || {
                let handles: Vec<_> = (0..threads)
                    .map(|_| {
                        std::thread::spawn(move || {
                            let f: unsafe extern "C" fn() -> c_int =
                                unsafe { std::mem::transmute(addr) };
                            (0..per).map(|_| unsafe { f() }).collect::<Vec<_>>()
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .flat_map(|h| h.join().expect("worker thread"))
                    .collect::<Vec<_>>()
            });
            assert_eq!(rets.len(), total);
            assert!(
                rets.iter().all(|&r| r == 0),
                "C12/{}[iter={iter}]: all returns must be 0",
                which.name()
            );
            // libc holds the stream lock per call, so lines stay intact; only
            // their order is unspecified. Compare as a multiset of lines.
            assert_eq!(
                out.len(),
                EXPECTED_LINE.len() * total,
                "C12/{}[iter={iter},threads={threads},per={per}]: expected {total} lines, got {:?}",
                which.name(),
                show(&out)
            );
            let lines: Vec<&[u8]> = out
                .split_inclusive(|&b| b == b'\n')
                .collect();
            assert_eq!(lines.len(), total);
            assert!(
                lines.iter().all(|l| *l == EXPECTED_LINE),
                "C12/{}[iter={iter}]: a line was torn or altered: {:?}",
                which.name(),
                show(&out)
            );
        }
    }
}

// --------------------------------------------------------------------- C13
#[test]
fn c13_declared_arity_call() {
    // Same as C2 but stated explicitly as its own row: the pointer type used
    // matches the header's declared `int helloworld()`.
    differential("C13", Sink::File, Buffering::Default, |p, w| {
        let f: unsafe extern "C" fn() -> c_int = unsafe { std::mem::transmute(p.helloworld_addr(w)) };
        unsafe { f() }
    });
}

// --------------------------------------------------------------------- C14
#[test]
fn c14_called_through_wider_argument_lists() {
    // `int helloworld();` is an *unprototyped* declaration, so a conforming C
    // caller may pass arguments; the SysV AMD64 callee simply ignores them.
    let mut rng = Rng::new(SEED ^ 0xCE);
    for iter in 0..15 {
        let a = rng.next_u64() as c_int;
        let b = rng.next_u64() as c_int;
        let c = rng.next_u64() as c_int;
        let d = rng.next_u64() as c_int;
        let e = rng.next_u64() as c_int;
        let g = rng.next_u64() as c_int;
        differential(
            &format!("C14[iter={iter}]"),
            Sink::File,
            Buffering::Default,
            |p, w| {
                let addr = p.helloworld_addr(w);
                unsafe {
                    let f1: unsafe extern "C" fn(c_int) -> c_int = std::mem::transmute(addr);
                    let f3: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int =
                        std::mem::transmute(addr);
                    let f6: unsafe extern "C" fn(
                        c_int,
                        c_int,
                        c_int,
                        c_int,
                        c_int,
                        c_int,
                    ) -> c_int = std::mem::transmute(addr);
                    vec![f1(a), f3(a, b, c), f6(a, b, c, d, e, g)]
                }
            },
        );
    }
}

// --------------------------------------------------------------------- C15
#[test]
fn c15_called_through_sse_argument_registers() {
    let mut rng = Rng::new(SEED ^ 0xCF);
    for iter in 0..10 {
        let x = f64::from_bits(rng.next_u64());
        let y = (rng.next_u64() % 1000) as c_double;
        differential(
            &format!("C15[iter={iter}]"),
            Sink::File,
            Buffering::Default,
            |p, w| {
                let addr = p.helloworld_addr(w);
                unsafe {
                    let fd2: unsafe extern "C" fn(c_double, c_double) -> c_int =
                        std::mem::transmute(addr);
                    let fmix: unsafe extern "C" fn(c_int, c_double, c_int) -> c_int =
                        std::mem::transmute(addr);
                    vec![fd2(x, y), fmix(1, y, 2)]
                }
            },
        );
    }
}

// --------------------------------------------------------------------- C16
#[test]
fn c16_repeated_dlopen_dlclose() {
    // No per-load state: each fresh load must behave exactly like the first,
    // and loading must not itself emit anything (no constructors).
    for iter in 0..10 {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        let (_, c_out) = capture(Sink::File, Buffering::Default, || {
            let lib = open(&c_path);
            let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                unsafe { lib.get(b"helloworld\0") }.unwrap();
            let r = unsafe { f() };
            drop(lib); // dlclose
            r
        });
        let (_, r_out) = capture(Sink::File, Buffering::Default, || {
            let lib = open(&r_path);
            let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                unsafe { lib.get(b"helloworld\0") }.unwrap();
            let r = unsafe { f() };
            drop(lib);
            r
        });
        assert_same_bytes(&format!("C16[iter={iter}]"), &c_out, &r_out);
        assert_eq!(c_out, EXPECTED_LINE);
    }

    // Loading (and unloading) alone must be silent for both.
    let (_, c_quiet) = capture(Sink::File, Buffering::Default, || {
        drop(open(&c_so_path()));
    });
    let (_, r_quiet) = capture(Sink::File, Buffering::Default, || {
        drop(open(&rust_so_path()));
    });
    assert_same_bytes("C16[load-is-silent]", &c_quiet, &r_quiet);
    assert!(
        c_quiet.is_empty() && r_quiet.is_empty(),
        "C16: dlopen/dlclose must not write to stdout (C={:?}, Rust={:?})",
        show(&c_quiet),
        show(&r_quiet)
    );
}

// --------------------------------------------------------------------- C17
#[test]
fn c17_both_libraries_resident_simultaneously() {
    // Both `.so`s export the same symbol name. `RTLD_LOCAL` must keep them
    // distinct: each handle has to resolve to its own definition, so the two
    // addresses differ while the behaviour matches.
    let pair = load_pair();
    let ca = pair.helloworld_addr(Impl::C);
    let ra = pair.helloworld_addr(Impl::Rust);
    assert_ne!(
        ca, ra,
        "C17: both handles resolved to the same address — the test would be \
         comparing one implementation with itself"
    );
    let (c_ret, c_out) = capture(Sink::File, Buffering::Default, || unsafe {
        pair.helloworld(Impl::C)()
    });
    let (r_ret, r_out) = capture(Sink::File, Buffering::Default, || unsafe {
        pair.helloworld(Impl::Rust)()
    });
    assert_same_ret("C17", c_ret, r_ret);
    assert_same_bytes("C17", &c_out, &r_out);
}

// ---------------------------------------------------------------- utilities

fn rng_len(rng: &mut Rng) -> usize {
    rng.range(0, 24) as usize
}

/// Emit `bytes` through libc `printf` (the same stream the library uses).
fn printf_bytes(bytes: &[u8]) {
    let mut z = Vec::with_capacity(bytes.len() + 1);
    z.extend_from_slice(bytes);
    z.push(0);
    unsafe {
        libc::printf(
            b"%s\0".as_ptr() as *const std::ffi::c_char,
            z.as_ptr() as *const std::ffi::c_char,
        );
    }
}
