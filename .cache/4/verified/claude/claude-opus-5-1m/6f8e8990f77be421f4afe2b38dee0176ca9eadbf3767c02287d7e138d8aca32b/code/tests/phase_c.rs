//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! The C library has no rejection path (see `ERRORS.md`): `helloworld` is
//! total and always returns `0`. Each row below constructs the exact hostile
//! condition from the table, runs it against BOTH `.so`s through `dlsym`, and
//! asserts they agree on the *same* observable outcome — return value, `errno`
//! left behind by the failed `write(2)`, the `FILE *stdout` error flag, and the
//! bytes (not) emitted. "Both failed somehow" is never accepted.

mod harness;

use harness::*;
use std::ffi::{c_char, c_int, c_void};
use std::path::Path;

/// Everything observable about one call.
#[derive(Debug, PartialEq, Eq)]
struct Obs {
    ret: i64,
    errno: c_int,
    ferror: bool,
    bytes: Vec<u8>,
}

fn cmp(tag: &str, c: &Obs, r: &Obs) {
    if c != r {
        panic!(
            "{tag}: C and Rust disagree\n  C   : ret={} errno={} ferror={} bytes={:?}\n  Rust: ret={} errno={} ferror={} bytes={:?}",
            c.ret,
            c.errno,
            c.ferror,
            String::from_utf8_lossy(&c.bytes),
            r.ret,
            r.errno,
            r.ferror,
            String::from_utf8_lossy(&r.bytes),
        );
    }
}

/// Leaves libc `stdout` in a pristine, fully-buffered, error-free state.
fn reset_stdout() {
    clear_stdout_error();
    apply_buf(BufCfg::Default);
}

// --- E1: fd 1 closed → write(2) fails EBADF ---------------------------------

fn observe_closed_stdout(addr: usize) -> Obs {
    reset_stdout();
    apply_buf(BufCfg::NoBuf); // no buffering: the write happens inside the call
    let ret;
    let e;
    let fe;
    {
        let _r = Redirect::close_stdout();
        ret = unsafe { call0_long(addr) };
        e = errno();
        fe = stdout_has_error();
    }
    reset_stdout();
    Obs {
        ret,
        errno: e,
        ferror: fe,
        bytes: Vec::new(),
    }
}

fn e1_closed_stdout_fd() {
    let _g = serial();
    let (c, r) = addrs();
    for _ in 0..4 {
        let oc = observe_closed_stdout(c);
        let or = observe_closed_stdout(r);
        cmp("E1 close(1) → EBADF", &oc, &or);
        assert_eq!(oc.ret, 0, "E1: C must still return 0");
        assert_eq!(oc.errno, libc::EBADF, "E1: C errno from the failed write");
        assert!(oc.ferror, "E1: C must leave stdout's error flag set");
    }
}

// --- E2: fd 1 is a read-only descriptor → EBADF -----------------------------

fn observe_readonly_stdout(addr: usize) -> Obs {
    reset_stdout();
    let path = std::env::temp_dir().join(format!("hello-ro-{}.bin", std::process::id()));
    std::fs::write(&path, b"").expect("create ro file");
    let fd = open_fd(&path, libc::O_RDONLY, 0);
    let (ret, e, fe);
    {
        let _r = Redirect::to_fd(fd);
        apply_buf(BufCfg::NoBuf);
        ret = unsafe { call0_long(addr) };
        e = errno();
        fe = stdout_has_error();
    }
    reset_stdout();
    unsafe { libc::close(fd) };
    let bytes = std::fs::read(&path).expect("read ro file");
    let _ = std::fs::remove_file(&path);
    Obs {
        ret,
        errno: e,
        ferror: fe,
        bytes,
    }
}

fn e2_readonly_stdout_fd() {
    let _g = serial();
    let (c, r) = addrs();
    for _ in 0..4 {
        let oc = observe_readonly_stdout(c);
        let or = observe_readonly_stdout(r);
        cmp("E2 O_RDONLY fd 1 → EBADF", &oc, &or);
        assert_eq!(oc.ret, 0, "E2: C must still return 0");
        assert_eq!(oc.errno, libc::EBADF, "E2: C errno");
        assert!(oc.bytes.is_empty(), "E2: nothing may be written");
    }
}

// --- E3: device full → ENOSPC ----------------------------------------------

fn observe_device(addr: usize, dev: &str, buf: BufCfg) -> Obs {
    reset_stdout();
    let fd = open_fd(Path::new(dev), libc::O_WRONLY, 0);
    let (ret, e, fe);
    {
        let _r = Redirect::to_fd(fd);
        apply_buf(buf);
        ret = unsafe { call0_long(addr) };
        e = errno();
        fe = stdout_has_error();
    }
    reset_stdout();
    unsafe { libc::close(fd) };
    Obs {
        ret,
        errno: e,
        ferror: fe,
        bytes: Vec::new(),
    }
}

fn e3_device_full_enospc() {
    let _g = serial();
    let (c, r) = addrs();
    for buf in [BufCfg::NoBuf, BufCfg::Full(1), BufCfg::Line(13)] {
        let oc = observe_device(c, "/dev/full", buf);
        let or = observe_device(r, "/dev/full", buf);
        cmp(&format!("E3 /dev/full → ENOSPC buf={buf:?}"), &oc, &or);
        assert_eq!(oc.ret, 0, "E3: C must still return 0");
        assert_eq!(oc.errno, libc::ENOSPC, "E3: C errno");
        assert!(oc.ferror, "E3: C leaves the error flag set");
    }
}

// --- E4: closed pipe → EPIPE -----------------------------------------------

fn observe_closed_pipe(addr: usize) -> Obs {
    reset_stdout();
    let mut fds = [0 as c_int; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe()");
    let (rfd, wfd) = (fds[0], fds[1]);
    assert_eq!(unsafe { libc::close(rfd) }, 0, "close(read end)");
    let (ret, e, fe);
    {
        let _r = Redirect::to_fd(wfd);
        apply_buf(BufCfg::NoBuf);
        ret = unsafe { call0_long(addr) };
        e = errno();
        fe = stdout_has_error();
    }
    reset_stdout();
    unsafe { libc::close(wfd) };
    Obs {
        ret,
        errno: e,
        ferror: fe,
        bytes: Vec::new(),
    }
}

fn e4_closed_pipe_epipe() {
    let _g = serial();
    let (c, r) = addrs();
    // Make the EPIPE observable instead of fatal (Rust's runtime already does
    // this for the whole process; be explicit and restore afterwards).
    let prev = unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN) };
    for _ in 0..4 {
        let oc = observe_closed_pipe(c);
        let or = observe_closed_pipe(r);
        cmp("E4 closed pipe → EPIPE", &oc, &or);
        assert_eq!(oc.ret, 0, "E4: C must still return 0");
        assert_eq!(oc.errno, libc::EPIPE, "E4: C errno");
        assert!(oc.ferror, "E4: C leaves the error flag set");
    }
    unsafe { libc::signal(libc::SIGPIPE, prev) };
}

// --- E5 / E6 / E7: hostile argument shapes through the K&R declaration ------

type F1 = unsafe extern "C" fn(c_int) -> i64;
type F2 = unsafe extern "C" fn(c_int, c_int) -> i64;
type F3 = unsafe extern "C" fn(c_int, c_int, c_int) -> i64;
type F4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> i64;
type F5 = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int) -> i64;
type F6 = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int) -> i64;

unsafe fn call_ints(addr: usize, a: &[c_int]) -> i64 {
    unsafe {
        match a.len() {
            0 => call0_long(addr),
            1 => std::mem::transmute::<usize, F1>(addr)(a[0]),
            2 => std::mem::transmute::<usize, F2>(addr)(a[0], a[1]),
            3 => std::mem::transmute::<usize, F3>(addr)(a[0], a[1], a[2]),
            4 => std::mem::transmute::<usize, F4>(addr)(a[0], a[1], a[2], a[3]),
            5 => std::mem::transmute::<usize, F5>(addr)(a[0], a[1], a[2], a[3], a[4]),
            6 => std::mem::transmute::<usize, F6>(addr)(a[0], a[1], a[2], a[3], a[4], a[5]),
            _ => unreachable!(),
        }
    }
}

fn observe_with(addr: usize, buf: BufCfg, body: impl FnOnce(usize) -> i64) -> Obs {
    reset_stdout();
    let (bytes, (ret, e, fe)) = capture_file(buf, || {
        let ret = body(addr);
        (ret, errno(), stdout_has_error())
    });
    reset_stdout();
    Obs {
        ret,
        errno: e,
        ferror: fe,
        bytes,
    }
}

fn e5_extra_garbage_int_arguments() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 0xE5);
    for arity in 1..=6usize {
        for _ in 0..6 {
            let args: Vec<c_int> = (0..arity).map(|_| rng.i32()).collect();
            let a2 = args.clone();
            let oc = observe_with(c, BufCfg::NoBuf, |ad| unsafe { call_ints(ad, &args) });
            let or = observe_with(r, BufCfg::NoBuf, |ad| unsafe { call_ints(ad, &a2) });
            let tag = format!("E5 arity={arity} args={args:?}");
            cmp(&tag, &oc, &or);
            assert_eq!(oc.ret, 0, "{tag}: C return");
            assert_eq!(oc.bytes, expected(1), "{tag}: C output");
            assert!(!oc.ferror, "{tag}: no error expected here");
        }
    }
}

fn e6_out_of_range_and_pointer_garbage() {
    let _g = serial();
    let (c, r) = addrs();
    // Values that name no valid variant of any enum, plus pointer-shaped junk.
    let ints: [c_int; 8] = [
        i32::MIN,
        -1,
        0,
        1,
        i32::MAX,
        i32::MAX - 1,
        0x4000_0000,
        -0x4000_0000,
    ];
    for &v in &ints {
        let a = vec![v; 6];
        let b = a.clone();
        let oc = observe_with(c, BufCfg::NoBuf, |ad| unsafe { call_ints(ad, &a) });
        let or = observe_with(r, BufCfg::NoBuf, |ad| unsafe { call_ints(ad, &b) });
        let tag = format!("E6 out-of-range int {v}");
        cmp(&tag, &oc, &or);
        assert_eq!(oc.ret, 0, "{tag}: C return");
        assert_eq!(oc.bytes, expected(1), "{tag}: C output");
    }

    type P3 = unsafe extern "C" fn(*const c_void, *const c_void, *const c_char) -> i64;
    let live = b"caller-owned bytes\0".to_vec();
    let ptrs: [(*const c_void, *const c_void); 4] = [
        (std::ptr::null(), std::ptr::null()),
        (live.as_ptr() as *const c_void, std::ptr::null()),
        (0xDEAD_BEEFusize as *const c_void, 1usize as *const c_void),
        (
            usize::MAX as *const c_void,
            0xFFFF_FFFF_FFFF_F000usize as *const c_void,
        ),
    ];
    for (p0, p1) in ptrs {
        let p2 = live.as_ptr() as *const c_char;
        let oc = observe_with(c, BufCfg::NoBuf, |ad| unsafe {
            std::mem::transmute::<usize, P3>(ad)(p0, p1, p2)
        });
        let or = observe_with(r, BufCfg::NoBuf, |ad| unsafe {
            std::mem::transmute::<usize, P3>(ad)(p0, p1, p2)
        });
        let tag = format!("E6 pointer garbage {p0:?} {p1:?}");
        cmp(&tag, &oc, &or);
        assert_eq!(oc.ret, 0, "{tag}: C return");
        assert_eq!(oc.bytes, expected(1), "{tag}: C output");
    }
    assert_eq!(&live[..6], b"caller", "E6: caller buffer clobbered");
}

fn e7_float_arguments_and_al_set() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 0xE7);
    type Fv = unsafe extern "C" fn(c_int, ...) -> i64;
    type Ff = unsafe extern "C" fn(f64, f64, f64, f64, f64, f64, f64, f64) -> i64;
    for _ in 0..8 {
        let f: Vec<f64> = (0..8).map(|_| rng.f64()).collect();
        let i0 = rng.i32();
        let oc = observe_with(c, BufCfg::NoBuf, |ad| unsafe {
            std::mem::transmute::<usize, Ff>(ad)(f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7])
        });
        let or = observe_with(r, BufCfg::NoBuf, |ad| unsafe {
            std::mem::transmute::<usize, Ff>(ad)(f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7])
        });
        cmp(&format!("E7 8 doubles {f:?}"), &oc, &or);
        assert_eq!(oc.bytes, expected(1), "E7: C output");
        assert_eq!(oc.ret, 0, "E7: C return");

        // Variadic call site: the ABI requires %al = number of vector regs used.
        let oc = observe_with(c, BufCfg::NoBuf, |ad| unsafe {
            std::mem::transmute::<usize, Fv>(ad)(i0, f[0], f[1], f[2], f[3])
        });
        let or = observe_with(r, BufCfg::NoBuf, |ad| unsafe {
            std::mem::transmute::<usize, Fv>(ad)(i0, f[0], f[1], f[2], f[3])
        });
        cmp(&format!("E7 variadic %al set {f:?}"), &oc, &or);
        assert_eq!(oc.bytes, expected(1), "E7: C output (variadic)");
    }
}

// --- E8: full 64-bit return -------------------------------------------------

fn e8_return_upper_bits() {
    let _g = serial();
    let (c, r) = addrs();
    for _ in 0..8 {
        let oc = observe_with(c, BufCfg::NoBuf, |ad| unsafe { call0_long(ad) });
        let or = observe_with(r, BufCfg::NoBuf, |ad| unsafe { call0_long(ad) });
        cmp("E8 i64 return", &oc, &or);
        assert_eq!(oc.ret, 0i64, "E8: C must zero the whole of %rax");
        assert_eq!(oc.ret as i32, 0, "E8: low 32 bits");
        assert_eq!(oc.ret >> 32, 0, "E8: upper 32 bits");
    }
}

// --- E9: 1-byte stdio buffer ------------------------------------------------

fn e9_one_byte_buffer() {
    let _g = serial();
    let (c, r) = addrs();
    for size in [1usize, 2, 3] {
        let oc = observe_with(c, BufCfg::Full(size), |ad| unsafe { call0_long(ad) });
        let or = observe_with(r, BufCfg::Full(size), |ad| unsafe { call0_long(ad) });
        cmp(&format!("E9 _IOFBF size={size}"), &oc, &or);
        assert_eq!(oc.bytes, expected(1), "E9: C output with size={size}");
        assert_eq!(oc.ret, 0, "E9: C return");
    }
}

// --- E10: stdout already in its error state ---------------------------------

/// Makes a real write fail so `ferror(stdout)` is set, and leaves it set.
fn poison_stdout() {
    clear_stdout_error();
    let fd = open_fd(Path::new("/dev/full"), libc::O_WRONLY, 0);
    {
        let _r = Redirect::to_fd(fd);
        apply_buf(BufCfg::NoBuf);
        caller_write_ignore_result(b"poison\n");
    }
    unsafe { libc::close(fd) };
    assert!(
        stdout_has_error(),
        "E10 setup: expected ferror(stdout) to be set"
    );
}

fn caller_write_ignore_result(bytes: &[u8]) {
    unsafe {
        libc::fwrite(bytes.as_ptr() as *const c_void, 1, bytes.len(), c_stdout());
    }
}

fn observe_with_sticky_error(addr: usize) -> Obs {
    poison_stdout();
    // NOTE: no clearerr() here — the sticky error flag is the condition.
    let (bytes, (ret, e, fe)) = capture_file_keep_error(BufCfg::NoBuf, || {
        let ret = unsafe { call0_long(addr) };
        (ret, errno(), stdout_has_error())
    });
    reset_stdout();
    Obs {
        ret,
        errno: e,
        ferror: fe,
        bytes,
    }
}

/// `capture_file`, but nothing clears libc's error flag before the body runs.
fn capture_file_keep_error<T>(buf: BufCfg, body: impl FnOnce() -> T) -> (Vec<u8>, T) {
    assert!(stdout_has_error(), "expected a sticky error before capture");
    let out = capture_file(buf, body);
    out
}

fn e10_sticky_stdout_error_flag() {
    let _g = serial();
    let (c, r) = addrs();
    for _ in 0..4 {
        let oc = observe_with_sticky_error(c);
        let or = observe_with_sticky_error(r);
        cmp("E10 sticky ferror(stdout)", &oc, &or);
        assert_eq!(oc.ret, 0, "E10: C return");
    }
}

// --- E11: fresh dlopen, first-ever call, repeated load/unload ---------------

fn e11_no_state_across_load_unload() {
    let _g = serial();
    let mut rng = Rng::new(SEED ^ 0xE11);
    let cycles = rng.range(3, 12) as usize;
    for i in 0..cycles {
        let n = rng.range(1, 5) as usize;
        let (cb, cr) = capture_file(BufCfg::NoBuf, || {
            let l = open_lib(&c_so_path());
            let a = hello_addr(&l);
            let v: Vec<i64> = (0..n).map(|_| unsafe { call0_long(a) }).collect();
            drop(l);
            v
        });
        let (rb, rr) = capture_file(BufCfg::NoBuf, || {
            let l = open_lib(&rust_so_path());
            let a = hello_addr(&l);
            let v: Vec<i64> = (0..n).map(|_| unsafe { call0_long(a) }).collect();
            drop(l);
            v
        });
        let tag = format!("E11 cycle={i} n={n}");
        assert_same_bytes(&tag, &cb, &rb);
        assert_same_rets(&tag, &cr, &rr);
        assert_eq!(cb, expected(n), "{tag}: C output");
        assert_eq!(cr, vec![0i64; n], "{tag}: C returns");
    }
    reset_stdout();
}

// --- E12: concurrent calls, including onto a failing destination ------------

fn threaded_rets(addr: usize, threads: usize, per: usize) -> Vec<i64> {
    let mut all = Vec::new();
    std::thread::scope(|s| {
        let hs: Vec<_> = (0..threads)
            .map(|_| s.spawn(move || (0..per).map(|_| unsafe { call0_long(addr) }).collect::<Vec<_>>()))
            .collect();
        for h in hs {
            all.extend(h.join().expect("worker panicked"));
        }
    });
    all
}

fn e12_concurrent_calls() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 0xE12);
    for _ in 0..4 {
        let t = rng.range(2, 8) as usize;
        let k = rng.range(1, 16) as usize;

        // (a) working destination
        let (cb, cr) = capture_file(BufCfg::NoBuf, || threaded_rets(c, t, k));
        let (rb, rr) = capture_file(BufCfg::NoBuf, || threaded_rets(r, t, k));
        let tag = format!("E12 threads={t} per={k} file");
        assert_same_bytes(&tag, &cb, &rb);
        assert_same_rets(&tag, &cr, &rr);
        assert_eq!(cr, vec![0i64; t * k], "{tag}: C returns");
        for (i, line) in cb.split_inclusive(|&b| b == b'\n').enumerate() {
            assert_eq!(line, HELLO_LINE, "{tag}: torn line {i}");
        }

        // (b) failing destination: every concurrent call must still return 0
        let cr = with_stdout_device("/dev/full", BufCfg::NoBuf, || threaded_rets(c, t, k));
        let rr = with_stdout_device("/dev/full", BufCfg::NoBuf, || threaded_rets(r, t, k));
        let tag = format!("E12 threads={t} per={k} /dev/full");
        assert_same_rets(&tag, &cr, &rr);
        assert_eq!(cr, vec![0i64; t * k], "{tag}: C returns under ENOSPC");
        reset_stdout();
    }
}

// --- E13: symbols that do not exist must be rejected by both ---------------

fn e13_unknown_symbols_rejected() {
    let _g = serial();
    let cl = open_lib(&c_so_path());
    let rl = open_lib(&rust_so_path());
    for name in [
        &b"helloworld_v2\0"[..],
        &b"hello_world\0"[..],
        &b"HelloWorld\0"[..],
        &b"helloworld2\0"[..],
        &b"hello\0"[..],
        &b"\0"[..],
    ] {
        let c_ok = unsafe { cl.get::<unsafe extern "C" fn() -> c_int>(name) }.is_ok();
        let r_ok = unsafe { rl.get::<unsafe extern "C" fn() -> c_int>(name) }.is_ok();
        assert_eq!(
            c_ok,
            r_ok,
            "E13 dlsym({:?}): C={} Rust={}",
            String::from_utf8_lossy(name),
            c_ok,
            r_ok
        );
        assert!(
            !c_ok,
            "E13 dlsym({:?}) unexpectedly resolved in the C .so",
            String::from_utf8_lossy(name)
        );
    }
    // ...while the one real symbol resolves in both.
    assert!(unsafe { cl.get::<unsafe extern "C" fn() -> c_int>(b"helloworld\0") }.is_ok());
    assert!(unsafe { rl.get::<unsafe extern "C" fn() -> c_int>(b"helloworld\0") }.is_ok());
}

// --- the single #[test] entry point -----------------------------------------

#[test]
fn phase_c_every_errors_row() {
    let mut rows = Rows::new("Phase C — ERRORS.md");
    rows.row("E1  fd 1 closed → write EBADF, must still return 0", e1_closed_stdout_fd);
    rows.row("E2  fd 1 read-only → write EBADF, must still return 0", e2_readonly_stdout_fd);
    rows.row("E3  /dev/full → write ENOSPC, must still return 0", e3_device_full_enospc);
    rows.row("E4  closed pipe → write EPIPE, must still return 0", e4_closed_pipe_epipe);
    rows.row("E5  extra garbage int args (K&R declaration)", e5_extra_garbage_int_arguments);
    rows.row("E6  out-of-range ints + NULL/garbage pointers", e6_out_of_range_and_pointer_garbage);
    rows.row("E7  float args in xmm0..7 and %al set (varargs)", e7_float_arguments_and_al_set);
    rows.row("E8  whole %rax must be 0 (i64 return)", e8_return_upper_bits);
    rows.row("E9  1..3 byte stdio buffer", e9_one_byte_buffer);
    rows.row("E10 sticky ferror(stdout) not checked/cleared", e10_sticky_stdout_error_flag);
    rows.row("E11 fresh dlopen / first call / load-unload cycles", e11_no_state_across_load_unload);
    rows.row("E12 concurrent calls, working and failing stdout", e12_concurrent_calls);
    rows.row("E13 unknown symbol names rejected by both .so's", e13_unknown_symbols_rejected);
    rows.finish();
}
