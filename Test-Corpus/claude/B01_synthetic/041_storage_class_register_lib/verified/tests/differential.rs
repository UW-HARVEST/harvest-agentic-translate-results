//! Differential tests: C `libdriver.so` vs. Rust `libdriver.so`.
//!
//! BOTH libraries are loaded with `libloading` and driven only through their
//! exported `driver` symbol, exactly as an external C consumer would — the Rust
//! implementation is never called directly, so the `#[no_mangle] extern "C"`
//! export wrapper is under test too.
//!
//! The only observable effect of `void driver(int x)` is the byte stream it
//! writes to libc `stdout` via `printf("%d\n", 2*x + 300)`. Every test therefore
//! redirects fd 1, runs the same call sequence against each library, and
//! compares the captured bytes byte-for-byte.

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::io::Write;
use std::os::raw::c_int;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits needed to capture fd 1
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut u8, count: usize) -> isize;
    /// `fflush(NULL)` flushes every open C stream, i.e. whatever `printf`
    /// buffered inside either shared library (both share one libc `stdout`).
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn _exit(code: c_int) -> !;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn kill(pid: c_int, sig: c_int) -> c_int;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

type DriverFn = unsafe extern "C" fn(c_int);

struct Libs {
    c: DriverFn,
    rs: DriverFn,
    // keep the handles alive for the whole process
    _c_lib: Library,
    _rs_lib: Library,
}

fn c_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

/// The Rust cdylib lives next to the test binary's parent directory
/// (`target/<profile>/libdriver.so`, test binary is in `target/<profile>/deps/`).
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe.parent().expect("deps dir").parent().expect("profile dir");
    dir.join("libdriver.so")
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rs_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path.display()
        );
        assert!(
            rs_path.exists(),
            "Rust shared library not found at {}. Build it with `cargo build`.",
            rs_path.display()
        );
        unsafe {
            // RTLD_LOCAL (libloading default) so the two identically named
            // `driver` symbols cannot shadow one another.
            let c_lib = Library::new(&c_path).expect("dlopen C lib");
            let rs_lib = Library::new(&rs_path).expect("dlopen Rust lib");
            let c_sym: Symbol<DriverFn> = c_lib.get(b"driver\0").expect("C `driver` symbol");
            let rs_sym: Symbol<DriverFn> = rs_lib.get(b"driver\0").expect("Rust `driver` symbol");
            let c = *c_sym;
            let rs = *rs_sym;
            Libs { c, rs, _c_lib: c_lib, _rs_lib: rs_lib }
        }
    })
}

// ---------------------------------------------------------------------------
// stdout capture (serialised: fd 1 is process-global)
// ---------------------------------------------------------------------------

fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn flush_everything() {
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

/// The library under test writes *only* lines matching `-?[0-9]+\n`. Anything
/// else in a capture came from a foreign writer to fd 1 — in practice libtest's
/// own progress lines ("test foo ... ok"), which it emits from another thread
/// when tests run in parallel. Such a capture is discarded and retried instead
/// of being compared (a contaminated capture would be a false divergence).
///
/// Run the suite with `-- --test-threads=1` to avoid the situation entirely.
fn is_clean(bytes: &[u8]) -> bool {
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'-' {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return false;
        }
        if i >= bytes.len() || bytes[i] != b'\n' {
            return false;
        }
        i += 1;
    }
    true
}

const CAPTURE_ATTEMPTS: usize = 64;

/// Redirect fd 1 to a temporary regular file, run `f`, return the bytes written.
fn capture_to_file<F: Fn()>(f: F) -> Vec<u8> {
    for attempt in 0..CAPTURE_ATTEMPTS {
        let out = capture_to_file_once(&f);
        if is_clean(&out) {
            return out;
        }
        if attempt + 1 == CAPTURE_ATTEMPTS {
            panic!(
                "could not obtain an uncontaminated stdout capture after {} attempts; \
                 re-run with `-- --test-threads=1`. Last capture: {:?}",
                CAPTURE_ATTEMPTS,
                String::from_utf8_lossy(&out[..out.len().min(200)])
            );
        }
    }
    unreachable!()
}

fn capture_to_file_once<F: Fn()>(f: &F) -> Vec<u8> {
    let _g = capture_lock();
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("driver_diff_{}_{}.out", std::process::id(), n));

    let file = std::fs::File::create(&path).expect("create capture file");
    flush_everything();
    let res = unsafe {
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
        // never leave fd 1 redirected, even if the closure unwinds
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f()));
        flush_everything();
        assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
        close(saved);
        res
    };
    drop(file);
    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    match res {
        Ok(()) => out,
        Err(p) => std::panic::resume_unwind(p),
    }
}

/// Redirect fd 1 to a pipe (different libc buffering decision than a file),
/// run `f`, return the bytes written. Output must stay below the pipe capacity.
fn capture_to_pipe<F: Fn()>(f: F) -> Vec<u8> {
    for attempt in 0..CAPTURE_ATTEMPTS {
        let out = capture_to_pipe_once(&f);
        if is_clean(&out) {
            return out;
        }
        if attempt + 1 == CAPTURE_ATTEMPTS {
            panic!(
                "could not obtain an uncontaminated pipe capture after {} attempts; \
                 re-run with `-- --test-threads=1`. Last capture: {:?}",
                CAPTURE_ATTEMPTS,
                String::from_utf8_lossy(&out[..out.len().min(200)])
            );
        }
    }
    unreachable!()
}

fn capture_to_pipe_once<F: Fn()>(f: &F) -> Vec<u8> {
    let _g = capture_lock();
    let mut fds = [0 as c_int; 2];
    let mut out = Vec::new();
    unsafe {
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() failed");
        flush_everything();
        let saved = dup(1);
        assert!(saved >= 0);
        assert!(dup2(fds[1], 1) >= 0);
        close(fds[1]);
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f()));
        flush_everything();
        assert!(dup2(saved, 1) >= 0);
        close(saved);

        let mut buf = vec![0u8; 1 << 16];
        loop {
            let n = read(fds[0], buf.as_mut_ptr(), buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(fds[0]);
        if let Err(p) = res {
            std::panic::resume_unwind(p);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// deterministic RNG (PCG-XSH-RR 64/32), fixed seed: reproducible inputs
// ---------------------------------------------------------------------------

struct Pcg32(u64);

impl Pcg32 {
    fn new(seed: u64) -> Self {
        let mut s = Pcg32(0);
        s.0 = seed.wrapping_add(0xa02b_dbf7_bb3c_0a7u64);
        s.next_u32();
        s
    }
    fn next_u32(&mut self) -> u32 {
        let old = self.0;
        self.0 = old
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }
    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// uniform in `[lo, hi]` inclusive, works across the full i32 range
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        let v = if span == 0 {
            self.next_u32() as u64
        } else {
            (self.next_u32() as u64) % span
        };
        (lo as i64 + v as i64) as i32
    }
}

fn rng() -> Pcg32 {
    Pcg32::new(0x5EED_1234_ABCD_0001)
}

// ---------------------------------------------------------------------------
// differential drivers
// ---------------------------------------------------------------------------

fn run_c(xs: &[i32]) -> Vec<u8> {
    let f = libs().c;
    capture_to_file(|| unsafe {
        for &x in xs {
            f(x as c_int);
        }
    })
}

fn run_rs(xs: &[i32]) -> Vec<u8> {
    let f = libs().rs;
    capture_to_file(|| unsafe {
        for &x in xs {
            f(x as c_int);
        }
    })
}

/// Compare the two libraries' stdout bytes for the whole call sequence, and on
/// divergence re-run value-by-value to name the first offending input.
fn assert_same(label: &str, xs: &[i32]) {
    let c_out = run_c(xs);
    let rs_out = run_rs(xs);
    if c_out == rs_out {
        return;
    }
    // localise
    let cf = libs().c;
    let rf = libs().rs;
    for &x in xs {
        let c1 = capture_to_file(|| unsafe { cf(x as c_int) });
        let r1 = capture_to_file(|| unsafe { rf(x as c_int) });
        if c1 != r1 {
            panic!(
                "[{}] divergence for x = {} ({:#010x}):\n  C   : {:?}\n  Rust: {:?}",
                label,
                x,
                x as u32,
                String::from_utf8_lossy(&c1),
                String::from_utf8_lossy(&r1)
            );
        }
    }
    panic!(
        "[{}] batch outputs differ although every single value matched \
         (buffering/interleaving divergence).\n  C   len={} Rust len={}\n  C   : {:?}\n  Rust: {:?}",
        label,
        c_out.len(),
        rs_out.len(),
        String::from_utf8_lossy(&c_out[..c_out.len().min(400)]),
        String::from_utf8_lossy(&rs_out[..rs_out.len().min(400)])
    );
}

/// Call `f(x)` in a forked child whose fd 1 has been closed, so the library's
/// `printf` fails. Returns the child's exit code (0 = returned normally), or a
/// negative marker if it died on a signal / had to be killed.
fn run_with_closed_stdout(f: DriverFn, x: i32) -> i32 {
    let _g = capture_lock();
    flush_everything();
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            close(1);
            f(x as c_int);
            _exit(0);
        }
        // bounded wait so a stuck child can never hang the suite
        let mut status: c_int = 0;
        let mut waited = 0;
        loop {
            let r = waitpid(pid, &mut status, 1 /* WNOHANG */);
            if r == pid {
                break;
            }
            if r < 0 {
                return -100;
            }
            if waited > 5000 {
                kill(pid, 9);
                let _ = waitpid(pid, &mut status, 0);
                return -101;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
            waited += 2;
        }
        if status & 0x7f == 0 {
            (status >> 8) & 0xff // WEXITSTATUS
        } else {
            -(status & 0x7f) // died on signal N
        }
    }
}

/// Independent model of the C semantics, used only to prove the capture harness
/// really observes the library's output (the C `.so` remains the ground truth).
fn expected(xs: &[i32]) -> Vec<u8> {
    let mut s = String::new();
    for &x in xs {
        s.push_str(&format!("{}\n", x.wrapping_mul(2).wrapping_add(300)));
    }
    s.into_bytes()
}

fn assert_same_and_modelled(label: &str, xs: &[i32]) {
    assert_same(label, xs);
    let c_out = run_c(xs);
    assert_eq!(
        c_out,
        expected(xs),
        "[{}] capture harness sanity check failed against the C library",
        label
    );
}

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

mod configs {
    use super::*;

    #[test] // C1
    fn c1_zero_calls_no_output() {
        assert_same("C1 zero calls", &[]);
        assert!(run_c(&[]).is_empty());
        assert!(run_rs(&[]).is_empty());
    }

    #[test] // C2
    fn c2_single_zero() {
        assert_same_and_modelled("C2 x=0", &[0]);
    }

    #[test] // C3
    fn c3_single_small_positive() {
        let mut r = rng();
        for _ in 0..200 {
            let x = r.range(1, 100);
            assert_same_and_modelled("C3 small positive", &[x]);
        }
    }

    #[test] // C4
    fn c4_single_small_negative() {
        let mut r = rng();
        for _ in 0..200 {
            let x = r.range(-100, -1);
            assert_same_and_modelled("C4 small negative", &[x]);
        }
    }

    #[test] // C5
    fn c5_small_positive_output_widths() {
        let xs: Vec<i32> = (-149..=-1).collect();
        assert_same_and_modelled("C5 output 1..3 digits", &xs);
    }

    #[test] // C6
    fn c6_output_zero_boundary() {
        assert_same_and_modelled("C6 output == 0", &[-150]);
        assert_same_and_modelled("C6 around output 0", &[-151, -150, -149]);
    }

    #[test] // C7
    fn c7_negative_output_no_wrap() {
        let mut r = rng();
        let mut xs = Vec::with_capacity(2000);
        for _ in 0..2000 {
            xs.push(r.range(i32::MIN / 2, -151));
        }
        assert_same_and_modelled("C7 negative output", &xs);
    }

    #[test] // C8
    fn c8_positive_no_wrap() {
        let mut r = rng();
        let mut xs = Vec::with_capacity(2000);
        for _ in 0..2000 {
            xs.push(r.range(1, i32::MAX / 2 - 200));
        }
        assert_same_and_modelled("C8 positive no wrap", &xs);
    }

    #[test] // C9
    fn c9_multiply_wraps_positive() {
        let mut r = rng();
        let mut xs = Vec::with_capacity(2000);
        for _ in 0..2000 {
            xs.push(r.range(i32::MAX / 2 + 1, i32::MAX));
        }
        assert_same_and_modelled("C9 2*x wraps (positive x)", &xs);
    }

    #[test] // C10
    fn c10_multiply_wraps_negative() {
        let mut r = rng();
        let mut xs = Vec::with_capacity(2000);
        for _ in 0..2000 {
            xs.push(r.range(i32::MIN, i32::MIN / 2 - 1));
        }
        assert_same_and_modelled("C10 2*x wraps (negative x)", &xs);
    }

    #[test] // C11
    fn c11_addition_overflow_edge() {
        let xs: Vec<i32> = (1_073_741_600..=1_073_741_900).collect();
        assert_same_and_modelled("C11 y+=300 overflow edge", &xs);
    }

    #[test] // C12
    fn c12_full_range_uniform() {
        let mut r = rng();
        let mut xs = Vec::with_capacity(5000);
        for _ in 0..5000 {
            xs.push(r.next_i32());
        }
        assert_same_and_modelled("C12 full i32 range", &xs);
    }

    #[test] // C13
    fn c13_output_width_classes() {
        // one x per printed decimal width, positive and negative results
        let mut xs = Vec::new();
        for w in 0..10u32 {
            let mag = 10i64.pow(w); // 1, 10, 100, ... 1e9
            for target in [mag, -mag, mag * 5 % 2_000_000_000] {
                // x such that 2x+300 == target (exactly when target is even)
                let x = ((target - 300) / 2) as i64;
                let x = x.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
                xs.push(x);
            }
        }
        xs.push(1_073_741_823); // widest positive result
        xs.push(-1_073_741_824); // widest negative result
        assert_same_and_modelled("C13 output width classes", &xs);
    }

    #[test] // C14
    fn c14_explicit_boundary_list() {
        let xs = [
            i32::MIN,
            i32::MIN + 1,
            -1_073_741_825,
            -1_073_741_824,
            -1_073_741_823,
            -151,
            -150,
            -149,
            -1,
            0,
            1,
            1_073_741_673,
            1_073_741_674,
            1_073_741_823,
            1_073_741_824,
            i32::MAX - 1,
            i32::MAX,
        ];
        assert_same_and_modelled("C14 boundaries", &xs);
        // and each one on its own, so no batching can mask a divergence
        for &x in &xs {
            assert_same_and_modelled("C14 boundary single", &[x]);
        }
    }

    #[test] // C15
    fn c15_exhaustive_contiguous_sweep() {
        let start = -100_000i32;
        let xs: Vec<i32> = (start..start + 200_000).collect();
        assert_same("C15 contiguous sweep", &xs);
    }

    #[test] // C16
    fn c16_repeated_identical_calls() {
        for x in [0, 7, -7, i32::MAX, i32::MIN] {
            let xs = vec![x; 1000];
            assert_same_and_modelled("C16 repeated", &xs);
        }
    }

    #[test] // C17
    fn c17_interleaved_c_and_rust() {
        let mut r = rng();
        let xs: Vec<i32> = (0..500).map(|_| r.next_i32()).collect();
        let cf = libs().c;
        let rf = libs().rs;

        // C, Rust, C, Rust ... in one stdout window
        let interleaved = capture_to_file(|| unsafe {
            for &x in &xs {
                cf(x as c_int);
                rf(x as c_int);
            }
        });
        // every value must appear twice in a row, identically
        let mut model = Vec::new();
        for &x in &xs {
            let line = format!("{}\n", x.wrapping_mul(2).wrapping_add(300));
            model.extend_from_slice(line.as_bytes());
            model.extend_from_slice(line.as_bytes());
        }
        assert_eq!(
            String::from_utf8_lossy(&interleaved),
            String::from_utf8_lossy(&model),
            "C17 interleaved C/Rust output diverges"
        );

        // reverse order too
        let interleaved2 = capture_to_file(|| unsafe {
            for &x in &xs {
                rf(x as c_int);
                cf(x as c_int);
            }
        });
        assert_eq!(interleaved, interleaved2, "C17 order-dependent divergence");
    }

    #[test] // C18
    fn c18_stdout_is_a_pipe() {
        let mut r = rng();
        let xs: Vec<i32> = (0..1000).map(|_| r.next_i32()).collect();
        let cf = libs().c;
        let rf = libs().rs;
        let c_out = capture_to_pipe(|| unsafe {
            for &x in &xs {
                cf(x as c_int);
            }
        });
        let rs_out = capture_to_pipe(|| unsafe {
            for &x in &xs {
                rf(x as c_int);
            }
        });
        assert_eq!(
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rs_out),
            "C18 pipe-buffered output diverges"
        );
        assert_eq!(c_out, expected(&xs), "C18 harness sanity check");
    }

    #[test] // C19
    fn c19_called_from_non_main_thread() {
        let mut r = rng();
        let xs: Vec<i32> = (0..1000).map(|_| r.next_i32()).collect();
        let cf = libs().c;
        let rf = libs().rs;

        let c_out = capture_to_file(|| {
            let xs_c = xs.clone();
            std::thread::spawn(move || unsafe {
                for &x in &xs_c {
                    cf(x as c_int);
                }
            })
            .join()
            .unwrap();
        });
        let rs_out = capture_to_file(|| {
            let xs_r = xs.clone();
            std::thread::spawn(move || unsafe {
                for &x in &xs_r {
                    rf(x as c_int);
                }
            })
            .join()
            .unwrap();
        });
        assert_eq!(c_out, rs_out, "C19 non-main-thread output diverges");
        assert_eq!(c_out, expected(&xs), "C19 harness sanity check");
    }

    #[test] // C20
    fn c20_large_batch_many_flushes() {
        let mut r = rng();
        let xs: Vec<i32> = (0..100_000).map(|_| r.next_i32()).collect();
        assert_same("C20 large batch", &xs);
    }

    #[test] // C21
    fn c21_strided_sweep_of_whole_i32_range() {
        // Systematic (not random) walk across the entire 32-bit input domain:
        // a prime stride visits every combination of the high 16 bits.
        const STRIDE: i64 = 65_521; // prime
        let mut xs = Vec::with_capacity(70_000);
        let mut v: i64 = i32::MIN as i64;
        while v <= i32::MAX as i64 {
            xs.push(v as i32);
            v += STRIDE;
        }
        assert_eq!(xs.len(), 65_552, "expected full-range stride coverage");
        assert_same("C21 strided full-range sweep", &xs);
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows
// ===========================================================================

mod errors {
    use super::*;

    #[test] // E1
    fn e1_no_error_paths_exist() {
        // The library has no error return, no sentinel and no rejection: the
        // only contract is "for every int, both libraries emit the same bytes".
        // Assert that on a dense set of adversarial values, including every
        // power-of-two boundary and its neighbours.
        let mut xs = Vec::new();
        for bit in 0..32u32 {
            let v = 1i64 << bit;
            for d in -2i64..=2 {
                let c = v + d;
                if c >= i32::MIN as i64 && c <= i32::MAX as i64 {
                    xs.push(c as i32);
                }
                let c = -v + d;
                if c >= i32::MIN as i64 && c <= i32::MAX as i64 {
                    xs.push(c as i32);
                }
            }
        }
        assert_same_and_modelled("E1 no rejection anywhere", &xs);
        for &x in &xs {
            assert_same_and_modelled("E1 single", &[x]);
        }
    }

    #[test] // E2
    fn e2_int_max() {
        let out = run_c(&[i32::MAX]);
        assert_eq!(out, b"298\n", "C behaviour for INT_MAX changed");
        assert_eq!(run_rs(&[i32::MAX]), out);
    }

    #[test] // E3
    fn e3_int_min() {
        let out = run_c(&[i32::MIN]);
        assert_eq!(out, b"300\n", "C behaviour for INT_MIN changed");
        assert_eq!(run_rs(&[i32::MIN]), out);
    }

    #[test] // E4
    fn e4_first_overflow_positive() {
        let x = 1_073_741_824; // INT_MAX/2 + 1
        let out = run_c(&[x]);
        assert_eq!(out, b"-2147483348\n");
        assert_eq!(run_rs(&[x]), out);
        // one step below is still in range
        assert_same_and_modelled("E4 step below", &[x - 1]);
    }

    #[test] // E5
    fn e5_first_overflow_negative() {
        let x = -1_073_741_825; // INT_MIN/2 - 1
        let out = run_c(&[x]);
        assert_eq!(out, b"-2147483350\n");
        assert_eq!(run_rs(&[x]), out);
        assert_same_and_modelled("E5 step above", &[x + 1]);
    }

    #[test] // E6
    fn e6_add_overflow() {
        let x = 1_073_741_674; // 2*x fits, 2*x + 300 does not
        let out = run_c(&[x]);
        assert_eq!(out, b"-2147483648\n");
        assert_eq!(run_rs(&[x]), out);
    }

    #[test] // E7
    fn e7_add_overflow_edge_sweep() {
        let xs: Vec<i32> = (1_073_741_673..=1_073_741_823).collect();
        assert_same_and_modelled("E7 add-overflow sweep", &xs);
        assert_eq!(run_c(&[1_073_741_673]), b"2147483646\n");
        assert_eq!(run_rs(&[1_073_741_673]), b"2147483646\n");
        assert_eq!(run_c(&[1_073_741_674]), b"-2147483648\n");
        assert_eq!(run_rs(&[1_073_741_674]), b"-2147483648\n");
    }

    #[test] // E8
    fn e8_all_bit_patterns_accepted() {
        // No enum exists, so the "out-of-range enum value" analogue is an int
        // bit pattern with no special meaning: check the sign-bit-only value,
        // all-ones, all-ones-but-sign, alternating patterns, etc.
        let xs: Vec<i32> = [
            0x0000_0000u32,
            0x8000_0000,
            0xFFFF_FFFF,
            0x7FFF_FFFF,
            0xAAAA_AAAA,
            0x5555_5555,
            0xDEAD_BEEF,
            0xCAFE_BABE,
            0xFFFF_0000,
            0x0000_FFFF,
            0x8000_0001,
            0x7FFF_FFFE,
        ]
        .iter()
        .map(|&v| v as i32)
        .collect();
        assert_same_and_modelled("E8 bit patterns", &xs);
        for &x in &xs {
            assert_same_and_modelled("E8 single", &[x]);
        }
    }

    #[test] // E9
    fn e9_upper_bits_truncated() {
        // Call through a 64-bit-argument signature: the C prologue is
        // `mov %edi,-0x14(%rbp)`, so the high half of the register is ignored
        // rather than rejected. The Rust export must truncate identically.
        type Driver64 = unsafe extern "C" fn(u64);
        let l = libs();
        let c64: Driver64 = unsafe { std::mem::transmute(l.c) };
        let r64: Driver64 = unsafe { std::mem::transmute(l.rs) };
        for raw in [
            0xDEAD_BEEF_0000_0005u64,
            0xFFFF_FFFF_0000_0000,
            0x0000_0001_FFFF_FFFF,
            0x1234_5678_8000_0000,
        ] {
            let c_out = capture_to_file(|| unsafe { c64(raw) });
            let r_out = capture_to_file(|| unsafe { r64(raw) });
            assert_eq!(
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&r_out),
                "E9 upper-bit handling diverges for {:#018x}",
                raw
            );
            let low = raw as u32 as i32;
            assert_eq!(c_out, expected(&[low]), "E9 C did not truncate as modelled");
        }
    }

    #[test] // E10
    fn e10_zero() {
        let out = run_c(&[0]);
        assert_eq!(out, b"300\n");
        assert_eq!(run_rs(&[0]), out);
    }

    #[test] // E11
    fn e11_write_error_stdout_closed() {
        // Environmental failure of the single I/O call the library makes:
        // fd 1 closed, so `printf` fails. The C ignores printf's return value
        // and returns normally; the Rust export must do exactly the same
        // (i.e. not panic / not abort). Run in a forked child so the poisoned
        // stdout stream cannot leak into the rest of the suite.
        for x in [0, 7, i32::MIN, i32::MAX] {
            let c_status = run_with_closed_stdout(libs().c, x);
            let r_status = run_with_closed_stdout(libs().rs, x);
            assert_eq!(
                c_status, 0,
                "C library did not return normally with stdout closed (x = {})",
                x
            );
            assert_eq!(
                r_status, c_status,
                "Rust library behaved differently from C with stdout closed (x = {})",
                x
            );
        }
    }
}

// ===========================================================================
// Phase D — symbol parity, checked from inside the test suite too
// ===========================================================================

mod symbols {
    use super::*;

    fn dynamic_defined(path: &std::path::Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", path.to_str().unwrap()])
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {}", path.display());
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    #[test]
    fn every_c_symbol_is_exported_by_rust() {
        let c_syms = dynamic_defined(&c_so_path());
        let rs_syms = dynamic_defined(&rust_so_path());
        assert!(c_syms.contains(&"driver".to_string()), "sanity: C exports driver");
        let missing: Vec<&String> = c_syms.iter().filter(|s| !rs_syms.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "symbols exported by the C .so but missing from the Rust .so: {:?}",
            missing
        );
    }

    #[test]
    fn rust_so_has_no_unresolved_symbols() {
        // dlopen already succeeded in `libs()`, which proves every undefined
        // symbol of the Rust .so resolves against libc/libgcc at load time.
        let _ = libs();
    }
}
