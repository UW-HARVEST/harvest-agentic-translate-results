//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are loaded from their `.so` via `libloading` and called
//! only through the exported `helloworld` symbol.

mod common;

use common::*;
use std::os::raw::c_int;

/// Runs `n` calls against `which` inside a captured stdout region.
fn run_n(which: Impl, n: usize, tag: &str, buffering: Buffering, flush: Flush) -> Capture {
    let f = helloworld_of(which);
    capture_stdout(tag, buffering, flush, &[], false, || {
        (0..n).map(|_| unsafe { f() }).collect()
    })
}

fn assert_same(c: &Capture, r: &Capture, ctx: &str) {
    assert_eq!(
        c.bytes, r.bytes,
        "{ctx}: stdout bytes differ\n  C   = {:?}\n  Rust= {:?}",
        String::from_utf8_lossy(&c.bytes),
        String::from_utf8_lossy(&r.bytes)
    );
    assert_eq!(c.rets, r.rets, "{ctx}: return values differ");
}

// --- row 1 -----------------------------------------------------------------
#[test]
fn cfg_01_single_call_file_default_buffering() {
    let _g = fd_lock();
    let c = run_n(Impl::C, 1, "c01c", Buffering::Default, Flush::Stdout);
    let r = run_n(Impl::Rust, 1, "c01r", Buffering::Default, Flush::Stdout);
    assert_same(&c, &r, "row1");
    assert_eq!(c.bytes, EXPECTED_LINE, "row1: C must emit exactly one line");
    assert_eq!(c.rets, vec![0 as c_int]);
}

// --- row 2 -----------------------------------------------------------------
#[test]
fn cfg_02_unbuffered() {
    let _g = fd_lock();
    let c = run_n(Impl::C, 1, "c02c", Buffering::None_, Flush::Stdout);
    let r = run_n(Impl::Rust, 1, "c02r", Buffering::None_, Flush::Stdout);
    assert_same(&c, &r, "row2");
    assert_eq!(c.bytes, EXPECTED_LINE);
}

// --- row 3 -----------------------------------------------------------------
#[test]
fn cfg_03_line_buffered_own_buffer() {
    let _g = fd_lock();
    let c = run_n(Impl::C, 1, "c03c", Buffering::LineOwnBuf(64), Flush::Stdout);
    let r = run_n(Impl::Rust, 1, "c03r", Buffering::LineOwnBuf(64), Flush::Stdout);
    assert_same(&c, &r, "row3");
    assert_eq!(c.bytes, EXPECTED_LINE);
}

// --- row 4 -----------------------------------------------------------------
#[test]
fn cfg_04_tiny_full_buffer_spans_multiple_writes() {
    let _g = fd_lock();
    // 4-byte buffer < 13-byte payload -> several write(2)s per call
    let c = run_n(Impl::C, 3, "c04c", Buffering::FullOwnBuf(4), Flush::Stdout);
    let r = run_n(Impl::Rust, 3, "c04r", Buffering::FullOwnBuf(4), Flush::Stdout);
    assert_same(&c, &r, "row4");
    assert_eq!(c.bytes, repeat_line(3));
}

// --- row 5 -----------------------------------------------------------------
#[test]
fn cfg_05_stdout_is_a_pipe() {
    let _g = fd_lock();

    fn via_pipe(which: Impl, n: usize) -> (Vec<u8>, Vec<c_int>) {
        let f = helloworld_of(which);
        let mut fds = [0 as c_int; 2];
        unsafe {
            assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
            let saved = libc::dup(1);
            libc::fflush(libc_stdout());
            libc::dup2(fds[1], 1);
            let rets: Vec<c_int> = (0..n).map(|_| f()).collect();
            libc::fflush(libc_stdout());
            libc::dup2(saved, 1);
            libc::close(saved);
            libc::close(fds[1]); // EOF for the reader
            let mut out = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let k = libc::read(fds[0], buf.as_mut_ptr() as *mut libc::c_void, buf.len());
                if k <= 0 {
                    break;
                }
                out.extend_from_slice(&buf[..k as usize]);
            }
            libc::close(fds[0]);
            (out, rets)
        }
    }

    let (cb, cr) = via_pipe(Impl::C, 2);
    let (rb, rr) = via_pipe(Impl::Rust, 2);
    assert_eq!(cb, rb, "row5: pipe bytes differ");
    assert_eq!(cr, rr, "row5: return values differ");
    assert_eq!(cb, repeat_line(2));
}

// --- row 6 -----------------------------------------------------------------
#[test]
fn cfg_06_dev_null() {
    let _g = fd_lock();
    let fc = helloworld_of(Impl::C);
    let fr = helloworld_of(Impl::Rust);
    let (c_rets, c_flush) = with_stdout_to("/dev/null", false, || vec![unsafe { fc() }]);
    let (r_rets, r_flush) = with_stdout_to("/dev/null", false, || vec![unsafe { fr() }]);
    assert_eq!(c_rets, r_rets, "row6: return values differ");
    assert_eq!(c_flush, r_flush, "row6: fflush results differ");
    assert_eq!(c_rets, vec![0 as c_int]);
}

// --- row 7 -----------------------------------------------------------------
#[test]
fn cfg_07_append_mode_with_existing_content() {
    let _g = fd_lock();
    let pad = b"PREEXISTING-CONTENT\n";
    let fc = helloworld_of(Impl::C);
    let fr = helloworld_of(Impl::Rust);
    let c = capture_stdout("c07c", Buffering::Default, Flush::Stdout, pad, true, || {
        vec![unsafe { fc() }]
    });
    let r = capture_stdout("c07r", Buffering::Default, Flush::Stdout, pad, true, || {
        vec![unsafe { fr() }]
    });
    assert_same(&c, &r, "row7");
    assert_eq!(c.bytes, EXPECTED_LINE);
}

// --- row 8 -----------------------------------------------------------------
#[test]
fn cfg_08_randomized_nonzero_initial_offset() {
    let _g = fd_lock();
    let mut rng = Rng::new(SEED ^ 0x08);
    for iter in 0..8 {
        let padlen = rng.range(0, 4096) as usize;
        let pad = vec![b'.'; padlen];
        let fc = helloworld_of(Impl::C);
        let fr = helloworld_of(Impl::Rust);
        let c = capture_stdout("c08c", Buffering::Default, Flush::Stdout, &pad, false, || {
            vec![unsafe { fc() }]
        });
        let r = capture_stdout("c08r", Buffering::Default, Flush::Stdout, &pad, false, || {
            vec![unsafe { fr() }]
        });
        assert_same(&c, &r, &format!("row8 iter{iter} pad={padlen}"));
        assert_eq!(c.bytes, EXPECTED_LINE, "row8 iter{iter}");
    }
}

// --- row 9 -----------------------------------------------------------------
#[test]
fn cfg_09_randomized_call_counts() {
    let _g = fd_lock();
    let mut rng = Rng::new(SEED ^ 0x09);
    for iter in 0..24 {
        let n = rng.range(1, 64) as usize;
        let buffering = match rng.range(0, 4) {
            0 => Buffering::Default,
            1 => Buffering::FullDefaultBuf,
            2 => Buffering::FullOwnBuf(rng.range(1, 128) as usize),
            3 => Buffering::LineOwnBuf(rng.range(1, 128) as usize),
            _ => Buffering::None_,
        };
        let flush = if rng.bool() { Flush::Stdout } else { Flush::All };
        let c = run_n(Impl::C, n, "c09c", buffering, flush);
        let r = run_n(Impl::Rust, n, "c09r", buffering, flush);
        assert_same(&c, &r, &format!("row9 iter{iter} n={n} buf={buffering:?}"));
        assert_eq!(c.bytes, repeat_line(n), "row9 iter{iter} n={n}");
        assert!(c.rets.iter().all(|&v| v == 0));
    }
}

// --- row 10 ----------------------------------------------------------------
#[test]
fn cfg_10_zero_calls() {
    let _g = fd_lock();
    let c = run_n(Impl::C, 0, "c10c", Buffering::Default, Flush::All);
    let r = run_n(Impl::Rust, 0, "c10r", Buffering::Default, Flush::All);
    assert_same(&c, &r, "row10");
    assert!(c.bytes.is_empty(), "row10: no calls must produce no bytes");
}

// --- row 11 ----------------------------------------------------------------
#[test]
fn cfg_11_randomly_interleaved_c_and_rust() {
    let _g = fd_lock();
    let mut rng = Rng::new(SEED ^ 0x11);
    for iter in 0..16 {
        let n = rng.range(1, 64) as usize;
        let schedule: Vec<Impl> = (0..n)
            .map(|_| if rng.bool() { Impl::C } else { Impl::Rust })
            .collect();
        let fc = helloworld_of(Impl::C);
        let fr = helloworld_of(Impl::Rust);
        let cap = capture_stdout("c11", Buffering::Default, Flush::Stdout, &[], false, || {
            schedule
                .iter()
                .map(|w| unsafe {
                    match w {
                        Impl::C => fc(),
                        Impl::Rust => fr(),
                    }
                })
                .collect()
        });
        assert_eq!(
            cap.bytes,
            repeat_line(n),
            "row11 iter{iter}: interleaved C/Rust output must be indistinguishable (n={n})"
        );
        assert!(cap.rets.iter().all(|&v| v == 0), "row11 iter{iter}");
    }
}

// --- row 12 ----------------------------------------------------------------
#[test]
fn cfg_12_ordering_c_then_rust_vs_rust_then_c() {
    let _g = fd_lock();
    let fc = helloworld_of(Impl::C);
    let fr = helloworld_of(Impl::Rust);
    let n = 7usize;
    let a = capture_stdout("c12a", Buffering::Default, Flush::Stdout, &[], false, || {
        let mut v: Vec<c_int> = (0..n).map(|_| unsafe { fc() }).collect();
        v.extend((0..n).map(|_| unsafe { fr() }));
        v
    });
    let b = capture_stdout("c12b", Buffering::Default, Flush::Stdout, &[], false, || {
        let mut v: Vec<c_int> = (0..n).map(|_| unsafe { fr() }).collect();
        v.extend((0..n).map(|_| unsafe { fc() }));
        v
    });
    assert_eq!(a.bytes, b.bytes, "row12: ordering must not matter");
    assert_eq!(a.rets, b.rets, "row12");
    assert_eq!(a.bytes, repeat_line(2 * n));
}

// --- row 13 ----------------------------------------------------------------
#[test]
fn cfg_13_fresh_dlopen_per_call() {
    let _g = fd_lock();

    fn fresh_calls(path: std::path::PathBuf, n: usize) -> Capture {
        capture_stdout("c13", Buffering::Default, Flush::Stdout, &[], false, || {
            (0..n)
                .map(|_| unsafe {
                    let lib = libloading::Library::new(&path).expect("dlopen");
                    let f: libloading::Symbol<HelloFn> = lib.get(b"helloworld\0").unwrap();
                    let v = f();
                    drop(lib);
                    v
                })
                .collect()
        })
    }

    let cp = std::env::var("C_HELLO_SO").map(std::path::PathBuf::from).unwrap_or_else(|_| {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libhello.so")
    });
    let rp = rust_so_path();
    let c = fresh_calls(cp, 4);
    let r = fresh_calls(rp, 4);
    assert_eq!(c.bytes, r.bytes, "row13: bytes differ after fresh dlopen per call");
    assert_eq!(c.rets, r.rets, "row13");
    assert_eq!(c.bytes, repeat_line(4));
}

fn rust_so_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("RUST_HELLO_SO") {
        return std::path::PathBuf::from(p);
    }
    let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let mut best: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for profile in ["debug", "release"] {
        let p = base.join(profile).join("libhello.so");
        if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            if best.as_ref().map_or(true, |(bt, _)| t > *bt) {
                best = Some((t, p));
            }
        }
    }
    best.expect("Rust cdylib not built").1
}

// --- row 14 ----------------------------------------------------------------
#[test]
fn cfg_14_both_libs_loaded_distinct_addresses() {
    let c = helloworld_of(Impl::C) as usize;
    let r = helloworld_of(Impl::Rust) as usize;
    assert_ne!(
        c, r,
        "row14: dlsym returned the same address for both libraries — the Rust \
         implementation is being masked by the C one, tests would be vacuous"
    );
}

// --- row 15 ----------------------------------------------------------------
#[test]
fn cfg_15_bulk_ten_thousand_calls() {
    let _g = fd_lock();
    let n = 10_000usize;
    let c = run_n(Impl::C, n, "c15c", Buffering::Default, Flush::Stdout);
    let r = run_n(Impl::Rust, n, "c15r", Buffering::Default, Flush::Stdout);
    assert_eq!(c.bytes.len(), n * EXPECTED_LINE.len(), "row15: C byte count");
    assert_eq!(c.bytes, r.bytes, "row15: bulk bytes differ");
    assert_eq!(c.rets, r.rets, "row15");
    assert_eq!(c.bytes, repeat_line(n));
}

// --- row 16 ----------------------------------------------------------------
#[test]
fn cfg_16_called_with_extra_args_unprototyped_style() {
    let _g = fd_lock();
    // `int helloworld();` has no prototype in C, so a caller may legally pass
    // extra arguments; on SysV x86-64 they land in unused registers.
    type Hello3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
    let fc: Hello3 = unsafe { std::mem::transmute(helloworld_of(Impl::C)) };
    let fr: Hello3 = unsafe { std::mem::transmute(helloworld_of(Impl::Rust)) };
    let mut rng = Rng::new(SEED ^ 0x16);
    for _ in 0..8 {
        let (a, b, d) = (
            rng.next_u64() as c_int,
            rng.next_u64() as c_int,
            rng.next_u64() as c_int,
        );
        let c = capture_stdout("c16c", Buffering::Default, Flush::Stdout, &[], false, || {
            vec![unsafe { fc(a, b, d) }]
        });
        let r = capture_stdout("c16r", Buffering::Default, Flush::Stdout, &[], false, || {
            vec![unsafe { fr(a, b, d) }]
        });
        assert_same(&c, &r, &format!("row16 args=({a},{b},{d})"));
        assert_eq!(c.bytes, EXPECTED_LINE);
        assert_eq!(c.rets, vec![0 as c_int]);
    }
}

// --- row 17 ----------------------------------------------------------------
#[test]
fn cfg_17_flush_all_streams() {
    let _g = fd_lock();
    let c = run_n(Impl::C, 3, "c17c", Buffering::FullDefaultBuf, Flush::All);
    let r = run_n(Impl::Rust, 3, "c17r", Buffering::FullDefaultBuf, Flush::All);
    assert_same(&c, &r, "row17");
    assert_eq!(c.flush_ret, r.flush_ret, "row17: fflush(NULL) result differs");
    assert_eq!(c.bytes, repeat_line(3));
}

// --- row 18 ----------------------------------------------------------------
#[test]
fn cfg_18_multithreaded_shared_stdout() {
    let _g = fd_lock();
    let mut rng = Rng::new(SEED ^ 0x18);
    let per_thread: Vec<usize> = (0..8).map(|_| rng.range(1, 32) as usize).collect();
    let total: usize = per_thread.iter().sum();

    fn threaded(which: Impl, per_thread: &[usize], tag: &str) -> Capture {
        let f = helloworld_of(which);
        capture_stdout(tag, Buffering::Default, Flush::All, &[], false, || {
            let mut handles = Vec::new();
            for &n in per_thread {
                handles.push(std::thread::spawn(move || {
                    (0..n).map(|_| unsafe { f() }).collect::<Vec<c_int>>()
                }));
            }
            let mut rets = Vec::new();
            for h in handles {
                rets.extend(h.join().unwrap());
            }
            rets.sort_unstable();
            rets
        })
    }

    let c = threaded(Impl::C, &per_thread, "c18c");
    let r = threaded(Impl::Rust, &per_thread, "c18r");
    assert_eq!(c.bytes.len(), total * EXPECTED_LINE.len(), "row18: C byte count");
    assert_eq!(r.bytes.len(), c.bytes.len(), "row18: byte counts differ");
    // stdout is locked per-printf, so no line may be torn
    assert_eq!(c.bytes, repeat_line(total), "row18: C output torn?");
    assert_eq!(r.bytes, c.bytes, "row18: Rust output differs");
    assert_eq!(c.rets, r.rets, "row18: return values differ");
}
