//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! The C `helloworld` has no parameters and discards `printf`'s result, so every
//! row is an environmental / FFI-boundary failure mode. In each case both
//! implementations must return the SAME value (`0`) and must not crash.

mod common;

use common::*;
use std::os::raw::c_int;

// --- row 1: /dev/full (every write fails with ENOSPC) -----------------------
#[test]
fn err_01_dev_full() {
    let _g = fd_lock();
    let fc = helloworld_of(Impl::C);
    let fr = helloworld_of(Impl::Rust);
    for unbuffered in [false, true] {
        let (c_rets, c_flush) = with_stdout_to("/dev/full", unbuffered, || vec![unsafe { fc() }]);
        let (r_rets, r_flush) = with_stdout_to("/dev/full", unbuffered, || vec![unsafe { fr() }]);
        assert_eq!(
            c_rets, r_rets,
            "row1 (unbuffered={unbuffered}): return values differ on ENOSPC"
        );
        assert_eq!(c_rets, vec![0 as c_int], "row1: C must still return 0");
        assert_eq!(
            c_flush, r_flush,
            "row1 (unbuffered={unbuffered}): fflush results differ"
        );
    }
}

// --- row 2: fd 1 closed ----------------------------------------------------
#[test]
fn err_02_closed_fd1() {
    let _g = fd_lock();

    fn with_fd1_closed(f: HelloFn) -> (c_int, c_int) {
        unsafe {
            let saved = libc::dup(1);
            assert!(saved >= 0);
            libc::fflush(libc_stdout());
            libc::setvbuf(libc_stdout(), std::ptr::null_mut(), libc::_IONBF, 0);
            libc::close(1);
            let ret = f();
            let fr = libc::fflush(libc_stdout());
            libc::dup2(saved, 1);
            libc::close(saved);
            libc::setvbuf(libc_stdout(), leaked_buf(4096), libc::_IOFBF, 4096);
            libc::clearerr(libc_stdout());
            (ret, fr)
        }
    }

    let (cr, cf) = with_fd1_closed(helloworld_of(Impl::C));
    let (rr, rf) = with_fd1_closed(helloworld_of(Impl::Rust));
    assert_eq!(cr, rr, "row2: return values differ with fd 1 closed");
    assert_eq!(cr, 0 as c_int, "row2: C must still return 0 (EBADF ignored)");
    assert_eq!(cf, rf, "row2: fflush results differ");
}

// --- row 3: stdout is a read-only fd (writes fail with EBADF) --------------
#[test]
fn err_03_readonly_fd() {
    let _g = fd_lock();

    fn with_readonly_stdout(f: HelloFn) -> (c_int, c_int) {
        unsafe {
            let path = std::ffi::CString::new("/dev/zero").unwrap();
            let fd = libc::open(path.as_ptr(), libc::O_RDONLY);
            assert!(fd >= 0);
            let saved = libc::dup(1);
            libc::fflush(libc_stdout());
            libc::setvbuf(libc_stdout(), std::ptr::null_mut(), libc::_IONBF, 0);
            libc::dup2(fd, 1);
            let ret = f();
            let fr = libc::fflush(libc_stdout());
            libc::dup2(saved, 1);
            libc::close(saved);
            libc::close(fd);
            libc::setvbuf(libc_stdout(), leaked_buf(4096), libc::_IOFBF, 4096);
            libc::clearerr(libc_stdout());
            (ret, fr)
        }
    }

    let (cr, cf) = with_readonly_stdout(helloworld_of(Impl::C));
    let (rr, rf) = with_readonly_stdout(helloworld_of(Impl::Rust));
    assert_eq!(cr, rr, "row3: return values differ on read-only stdout");
    assert_eq!(cr, 0 as c_int, "row3: C must still return 0");
    assert_eq!(cf, rf, "row3: fflush results differ");
}

// --- row 4: pipe with the read end closed (EPIPE / SIGPIPE) ---------------
#[test]
fn err_04_broken_pipe() {
    let _g = fd_lock();

    fn with_broken_pipe(f: HelloFn) -> (c_int, c_int) {
        unsafe {
            // Rust's runtime already sets SIGPIPE to SIG_IGN; make it explicit so
            // both implementations see the same disposition.
            libc::signal(libc::SIGPIPE, libc::SIG_IGN);
            let mut fds = [0 as c_int; 2];
            assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
            libc::close(fds[0]); // reader gone
            let saved = libc::dup(1);
            libc::fflush(libc_stdout());
            libc::setvbuf(libc_stdout(), std::ptr::null_mut(), libc::_IONBF, 0);
            libc::dup2(fds[1], 1);
            let ret = f();
            let fr = libc::fflush(libc_stdout());
            libc::dup2(saved, 1);
            libc::close(saved);
            libc::close(fds[1]);
            libc::setvbuf(libc_stdout(), leaked_buf(4096), libc::_IOFBF, 4096);
            libc::clearerr(libc_stdout());
            (ret, fr)
        }
    }

    let (cr, cf) = with_broken_pipe(helloworld_of(Impl::C));
    let (rr, rf) = with_broken_pipe(helloworld_of(Impl::Rust));
    assert_eq!(cr, rr, "row4: return values differ on EPIPE");
    assert_eq!(cr, 0 as c_int, "row4: C must still return 0");
    assert_eq!(cf, rf, "row4: fflush results differ");
}

// --- row 5: unbuffered stream onto a failing target -----------------------
#[test]
fn err_05_unbuffered_failing_target() {
    let _g = fd_lock();
    let fc = helloworld_of(Impl::C);
    let fr = helloworld_of(Impl::Rust);
    // several calls: the error sticks on the FILE, both must behave the same
    let (c_rets, c_flush) = with_stdout_to("/dev/full", true, || {
        (0..5).map(|_| unsafe { fc() }).collect()
    });
    let (r_rets, r_flush) = with_stdout_to("/dev/full", true, || {
        (0..5).map(|_| unsafe { fr() }).collect()
    });
    unsafe { libc::clearerr(libc_stdout()) };
    assert_eq!(c_rets, r_rets, "row5: return values differ");
    assert_eq!(c_rets, vec![0 as c_int; 5], "row5: C returns 0 for every call");
    assert_eq!(c_flush, r_flush, "row5: fflush results differ");
}

// --- row 6: extra arguments through an unprototyped-style pointer ---------
#[test]
fn err_06_extra_args_unprototyped() {
    let _g = fd_lock();
    type Hello6 = unsafe extern "C" fn(c_int, u64, *const u8, c_int, c_int, c_int) -> c_int;
    let fc: Hello6 = unsafe { std::mem::transmute(helloworld_of(Impl::C)) };
    let fr: Hello6 = unsafe { std::mem::transmute(helloworld_of(Impl::Rust)) };
    let mut rng = Rng::new(SEED ^ 0x06);
    for _ in 0..8 {
        // includes a null pointer argument and out-of-range-ish int values
        let a = rng.next_u64() as c_int;
        let b = rng.next_u64();
        let c = capture_stdout("e06c", Buffering::Default, Flush::Stdout, &[], false, || {
            vec![unsafe { fc(a, b, std::ptr::null(), c_int::MIN, c_int::MAX, 0) }]
        });
        let r = capture_stdout("e06r", Buffering::Default, Flush::Stdout, &[], false, || {
            vec![unsafe { fr(a, b, std::ptr::null(), c_int::MIN, c_int::MAX, 0) }]
        });
        assert_eq!(c.rets, r.rets, "row6: return values differ with extra args");
        assert_eq!(c.bytes, r.bytes, "row6: output differs with extra args");
        assert_eq!(c.rets, vec![0 as c_int]);
        assert_eq!(c.bytes, EXPECTED_LINE);
    }
}

// --- row 7: return-register width reinterpretation ------------------------
#[test]
fn err_07_return_register_width() {
    let _g = fd_lock();
    type HelloI64 = unsafe extern "C" fn() -> i64;
    type HelloI16 = unsafe extern "C" fn() -> i16;
    let c64: HelloI64 = unsafe { std::mem::transmute(helloworld_of(Impl::C)) };
    let r64: HelloI64 = unsafe { std::mem::transmute(helloworld_of(Impl::Rust)) };
    let c16: HelloI16 = unsafe { std::mem::transmute(helloworld_of(Impl::C)) };
    let r16: HelloI16 = unsafe { std::mem::transmute(helloworld_of(Impl::Rust)) };

    let mut wide = (0u32, 0u32, 0i16, 0i16);
    with_stdout_to("/dev/null", false, || {
        unsafe {
            wide = (
                c64() as u64 as u32,
                r64() as u64 as u32,
                c16(),
                r16(),
            );
        }
        Vec::new()
    });
    let (c_lo, r_lo, c_s, r_s) = wide;
    // The C ABI only defines the low 32 bits for an `int` return.
    assert_eq!(c_lo, r_lo, "row7: low 32 bits of return register differ");
    assert_eq!(c_lo, 0, "row7: C returns 0");
    assert_eq!(c_s, r_s, "row7: narrowed return values differ");
    assert_eq!(c_s, 0);
    // silence the stray output produced above
    unsafe { libc::fflush(libc_stdout()) };
}

// --- row 8: baseline, the only admissible input --------------------------
#[test]
fn err_08_baseline_no_args() {
    let _g = fd_lock();
    let fc = helloworld_of(Impl::C);
    let fr = helloworld_of(Impl::Rust);
    let c = capture_stdout("e08c", Buffering::Default, Flush::Stdout, &[], false, || {
        vec![unsafe { fc() }]
    });
    let r = capture_stdout("e08r", Buffering::Default, Flush::Stdout, &[], false, || {
        vec![unsafe { fr() }]
    });
    assert_eq!(c.rets, r.rets, "row8");
    assert_eq!(c.bytes, r.bytes, "row8");
    assert_eq!(c.rets, vec![0 as c_int]);
    assert_eq!(c.bytes, EXPECTED_LINE);
}

// --- row 9: 100 000 calls (state / drift probe) ---------------------------
#[test]
fn err_09_many_calls() {
    let _g = fd_lock();
    let n = 100_000usize;
    let fc = helloworld_of(Impl::C);
    let fr = helloworld_of(Impl::Rust);
    let c = capture_stdout("e09c", Buffering::Default, Flush::Stdout, &[], false, || {
        (0..n).map(|_| unsafe { fc() }).collect()
    });
    let r = capture_stdout("e09r", Buffering::Default, Flush::Stdout, &[], false, || {
        (0..n).map(|_| unsafe { fr() }).collect()
    });
    assert_eq!(c.bytes.len(), n * EXPECTED_LINE.len(), "row9: C byte count");
    assert_eq!(c.bytes, r.bytes, "row9: bytes differ after {n} calls");
    assert_eq!(c.rets, r.rets, "row9: return values differ");
    assert!(r.rets.iter().all(|&v| v == 0), "row9: every call must return 0");
}

// --- row 10: concurrent calls --------------------------------------------
#[test]
fn err_10_threaded() {
    let _g = fd_lock();
    let per_thread = [17usize, 5, 31, 2, 9, 24, 1, 13];
    let total: usize = per_thread.iter().sum();

    fn threaded(which: Impl, per_thread: &[usize], tag: &str) -> Capture {
        let f = helloworld_of(which);
        capture_stdout(tag, Buffering::Default, Flush::All, &[], false, || {
            let hs: Vec<_> = per_thread
                .iter()
                .map(|&n| std::thread::spawn(move || (0..n).map(|_| unsafe { f() }).sum::<c_int>()))
                .collect();
            let mut v: Vec<c_int> = hs.into_iter().map(|h| h.join().unwrap()).collect();
            v.sort_unstable();
            v
        })
    }

    let c = threaded(Impl::C, &per_thread, "e10c");
    let r = threaded(Impl::Rust, &per_thread, "e10r");
    assert_eq!(c.bytes, repeat_line(total), "row10: C output torn or short");
    assert_eq!(r.bytes, c.bytes, "row10: Rust output differs under concurrency");
    assert_eq!(c.rets, r.rets, "row10: summed return values differ");
    assert!(c.rets.iter().all(|&v| v == 0));
}

// --- row 11: freopen redirection ----------------------------------------
#[test]
fn err_11_freopen() {
    let _g = fd_lock();

    fn via_freopen(f: HelloFn, tag: &str) -> (Vec<u8>, c_int) {
        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
        let path = format!("{dir}/hello_freopen_{tag}_{}.out", std::process::id());
        let cpath = std::ffi::CString::new(path.clone()).unwrap();
        let mode = std::ffi::CString::new("w").unwrap();
        unsafe {
            let saved = libc::dup(1);
            libc::fflush(libc_stdout());
            let fp = libc::freopen(cpath.as_ptr(), mode.as_ptr(), libc_stdout());
            assert!(!fp.is_null(), "freopen failed");
            let ret = f();
            libc::fflush(libc_stdout());
            // restore stdout onto the saved terminal/pipe fd
            let dn = std::ffi::CString::new("/dev/null").unwrap();
            libc::freopen(dn.as_ptr(), mode.as_ptr(), libc_stdout());
            libc::dup2(saved, 1);
            libc::close(saved);
            let bytes = std::fs::read(&path).unwrap_or_default();
            let _ = std::fs::remove_file(&path);
            (bytes, ret)
        }
    }

    let (cb, cr) = via_freopen(helloworld_of(Impl::C), "c");
    let (rb, rr) = via_freopen(helloworld_of(Impl::Rust), "r");
    assert_eq!(cb, rb, "row11: freopen'd output differs");
    assert_eq!(cr, rr, "row11: return values differ");
    assert_eq!(cb, EXPECTED_LINE, "row11: output must follow the new target");
    assert_eq!(cr, 0 as c_int);
}
