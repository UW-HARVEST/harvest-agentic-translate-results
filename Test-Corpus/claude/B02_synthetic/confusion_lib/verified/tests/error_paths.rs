// Phase C — error-path differential tests.
//
// One test per row of ERRORS.md. Every test constructs the exact invalid input
// / condition, calls BOTH shared objects through their exported C symbols, and
// asserts they return the same error code / sentinel AND print the same bytes.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

// ===========================================================================
// helpers
// ===========================================================================

fn diff_create_state(ctx: &str, initial_val: c_int, capacity: c_int) {
    let (c, r) = impls();

    let (cptr, cout) = capture(|| unsafe { (c.create_state)(initial_val, capacity) });
    let csnap = unsafe { snapshot(cptr) };
    unsafe { (c.destroy_state)(cptr) };

    let (rptr, rout) = capture(|| unsafe { (r.create_state)(initial_val, capacity) });
    let rsnap = unsafe { snapshot(rptr) };
    unsafe { (r.destroy_state)(rptr) };

    assert_same(ctx, (cptr.is_null(), cout), (rptr.is_null(), rout));
    assert_eq!(csnap, rsnap, "[{ctx}] ProcessState differs");
}

/// A state whose `buffer` has been released and nulled out (the C code's
/// `state->buffer == NULL` branch is otherwise unreachable through the public
/// API, since `create_state` never returns a state with a NULL buffer).
struct NullBufferState {
    c_state: *mut ProcessState,
    r_state: *mut ProcessState,
}

impl NullBufferState {
    fn new() -> NullBufferState {
        let (c, r) = impls();
        let (c_state, _) = capture(|| unsafe { (c.create_state)(0, 64) });
        let (r_state, _) = capture(|| unsafe { (r.create_state)(0, 64) });
        assert!(!c_state.is_null() && !r_state.is_null());
        unsafe {
            // Same allocator on both sides (glibc `malloc`/`free`).
            free((*c_state).buffer as *mut c_void);
            (*c_state).buffer = std::ptr::null_mut();
            free((*r_state).buffer as *mut c_void);
            (*r_state).buffer = std::ptr::null_mut();
        }
        NullBufferState { c_state, r_state }
    }
}

impl Drop for NullBufferState {
    fn drop(&mut self) {
        let (c, r) = impls();
        let _ = capture(|| unsafe { (c.destroy_state)(self.c_state) });
        let _ = capture(|| unsafe { (r.destroy_state)(self.r_state) });
    }
}

struct Pair {
    c_state: *mut ProcessState,
    r_state: *mut ProcessState,
}

impl Pair {
    fn new(initial_val: c_int, capacity: c_int) -> Pair {
        let (c, r) = impls();
        let (c_state, _) = capture(|| unsafe { (c.create_state)(initial_val, capacity) });
        let (r_state, _) = capture(|| unsafe { (r.create_state)(initial_val, capacity) });
        assert!(!c_state.is_null() && !r_state.is_null());
        Pair { c_state, r_state }
    }
    fn assert_states_equal(&self, ctx: &str) {
        let cs = unsafe { snapshot(self.c_state) };
        let rs = unsafe { snapshot(self.r_state) };
        assert_eq!(cs, rs, "[{ctx}] ProcessState differs");
    }
}

impl Drop for Pair {
    fn drop(&mut self) {
        let (c, r) = impls();
        let _ = capture(|| unsafe { (c.destroy_state)(self.c_state) });
        let _ = capture(|| unsafe { (r.destroy_state)(self.r_state) });
    }
}

// ===========================================================================
// ERRORS.md row 1 & 15 — the `malloc` failure paths.
//
// Re-executes THIS test binary with a tight RLIMIT_AS, exhausts the heap in the
// child and then calls the entry point, so `malloc(sizeof(ProcessState))`
// genuinely returns NULL. The child encodes the outcome in its exit status and
// writes the library's stdout to a file; parent compares C vs Rust.
// ===========================================================================

const CHILD_ENV_IMPL: &str = "CONFUSION_OOM_IMPL";
const CHILD_ENV_MODE: &str = "CONFUSION_OOM_MODE";
const CHILD_ENV_OUT: &str = "CONFUSION_OOM_OUT";

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn open(path: *const c_char, flags: c_int, mode: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `malloc(size)`, leaking the result, reporting whether it failed.
///
/// `black_box` is essential: with optimisations enabled LLVM happily deletes a
/// `malloc` whose result is only null-checked and folds the check to "non-null",
/// which would make the heap-exhaustion loop below a no-op.
#[inline(never)]
fn malloc_failed(size: usize) -> bool {
    let p = std::hint::black_box(unsafe { malloc(std::hint::black_box(size)) });
    std::hint::black_box(p).is_null()
}

/// Hidden helper "test": a no-op unless the harness re-executed us with
/// `CONFUSION_OOM_IMPL` set. When it is set, this exhausts the heap and exits
/// with a status that encodes the observed error sentinel.
#[test]
fn zz_oom_child_helper() {
    let which = match std::env::var(CHILD_ENV_IMPL) {
        Ok(v) => v,
        Err(_) => return, // normal test run: nothing to do
    };
    let mode = std::env::var(CHILD_ENV_MODE).unwrap();
    let out = std::env::var(CHILD_ENV_OUT).unwrap();

    // Load exactly one implementation (keeps the address-space budget equal for
    // both children).
    let im = if which == "c" {
        Impl::load_one("C", &c_so_path())
    } else {
        Impl::load_one("Rust", &rust_so_path())
    };

    // Redirect fd 1 into the capture file and force glibc to allocate the
    // `stdout` buffer *now*, while the heap is still healthy, so the library's
    // error message can still be printed once the heap is exhausted.
    let path = std::ffi::CString::new(out).unwrap();
    const O_WRONLY: c_int = 1;
    const O_CREAT: c_int = 0o100;
    const O_TRUNC: c_int = 0o1000;
    let fd = unsafe { open(path.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644) };
    assert!(fd >= 0, "child could not open capture file");
    assert!(unsafe { dup2(fd, 1) } >= 0);
    // Warm up stdio (identical prefix in both children) so that glibc has
    // already allocated the `stdout` buffer before the heap is exhausted.
    let warm = b"warmup\n\0";
    unsafe { printf(warm.as_ptr() as *const c_char) };
    unsafe { fflush(std::ptr::null_mut()) };

    // Exhaust the heap: coarse blocks first, then finer and finer, so that even
    // a 24-byte request (`sizeof(ProcessState)`) must fail afterwards. The
    // pointers are deliberately leaked — the whole point is to keep the memory
    // — and nothing is recorded, because any bookkeeping would itself need to
    // allocate.
    for &sz in &[1 << 20usize, 1 << 16, 1 << 12, 256, 64, 32, 24, 16, 8] {
        let mut guard = 0u64;
        while !malloc_failed(sz) {
            guard += 1;
            if guard > 4_000_000 {
                break;
            }
        }
    }
    // Final assurance: keep asking for exactly `sizeof(ProcessState)` bytes
    // until that fails too.
    let mut guard = 0u64;
    while !malloc_failed(24) {
        guard += 1;
        if guard > 4_000_000 {
            break;
        }
    }
    let exhausted = malloc_failed(24);

    // NOTE: the "expected" codes are deliberately non-zero so that a child that
    // never reached this point (e.g. because the environment variables were not
    // seen and libtest exited 0 on its own) can never be mistaken for a pass.
    let code = if !exhausted {
        90 // could not exhaust the heap
    } else if mode == "create_state" {
        let s = unsafe { (im.create_state)(7, 128) };
        if s.is_null() {
            42 // expected: NULL sentinel
        } else {
            91
        }
    } else {
        let v = unsafe { (im.confusion)(7, 2, 3, 4) };
        if v == -1 {
            42 // expected: -1 sentinel
        } else {
            92
        }
    };

    unsafe { fflush(std::ptr::null_mut()) };
    std::process::exit(code);
}

fn vm_size_bytes() -> u64 {
    let s = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: u64 = s
        .split_whitespace()
        .next()
        .and_then(|v| v.parse().ok())
        .unwrap_or(65536);
    pages * 4096
}

/// Runs the OOM child for one implementation, returns `(exit_code, stdout)`.
fn run_oom_child(which: &str, mode: &str) -> (i32, Vec<u8>) {
    use std::os::unix::process::CommandExt;

    #[repr(C)]
    struct Rlimit {
        cur: u64,
        max: u64,
    }
    extern "C" {
        fn setrlimit(resource: c_int, rlim: *const Rlimit) -> c_int;
    }
    const RLIMIT_AS: c_int = 9;

    let limit = vm_size_bytes() + (48 << 20);
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let out_path = std::path::PathBuf::from(dir).join(format!(
        "confusion_oom_{}_{}_{}.txt",
        which,
        mode,
        std::process::id()
    ));

    let exe = std::env::current_exe().unwrap();
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--exact")
        .arg("zz_oom_child_helper")
        .arg("--test-threads=1")
        .arg("--nocapture")
        .env(CHILD_ENV_IMPL, which)
        .env(CHILD_ENV_MODE, mode)
        .env(CHILD_ENV_OUT, &out_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    unsafe {
        cmd.pre_exec(move || {
            let rl = Rlimit {
                cur: limit,
                max: limit,
            };
            if setrlimit(RLIMIT_AS, &rl) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let status = cmd.status().expect("spawn OOM child");
    let out = std::fs::read(&out_path).unwrap_or_default();
    let _ = std::fs::remove_file(&out_path);
    (status.code().unwrap_or(-1), out)
}

/// Expected child exit code when the entry point returned its documented
/// error sentinel under heap exhaustion.
const OOM_SENTINEL_OK: i32 = 42;

#[test]
fn err_01_create_state_state_malloc_fails() {
    let (cc, cout) = run_oom_child("c", "create_state");
    let (rc, rout) = run_oom_child("rust", "create_state");
    assert_ne!(
        cc, 90,
        "could not exhaust the heap in the child; test is inconclusive"
    );
    assert_eq!(
        cc, OOM_SENTINEL_OK,
        "C create_state did not return NULL on heap exhaustion (code {cc}, stdout {:?})",
        String::from_utf8_lossy(&cout)
    );
    // The child really got as far as printing the C error message.
    assert_eq!(
        String::from_utf8_lossy(&cout),
        "warmup\nError: Failed to allocate memory for state\n",
        "C child stdout"
    );
    assert_same("row1 create_state OOM", (cc, cout), (rc, rout));
}

#[test]
fn err_15_confusion_create_state_null() {
    let (cc, cout) = run_oom_child("c", "confusion");
    let (rc, rout) = run_oom_child("rust", "confusion");
    assert_ne!(cc, 90, "could not exhaust the heap; test inconclusive");
    assert_eq!(
        cc, OOM_SENTINEL_OK,
        "C confusion did not return -1 on heap exhaustion (code {cc}, stdout {:?})",
        String::from_utf8_lossy(&cout)
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        "warmup\nDebug: param1 = 7\nDebug: param2 = 2\nDebug: param3 = 3\n\
         Debug: param4 = 4\nError: Failed to allocate memory for state\n",
        "C child stdout"
    );
    assert_same("row15 confusion OOM", (cc, cout), (rc, rout));
}

// ===========================================================================
// Row 2 — negative capacity => malloc((size_t)negative) fails
// ===========================================================================

#[test]
fn err_02_create_state_negative_capacity() {
    for &cap in &[-1i32, -2, -8, -128, -1000000, i32::MIN, i32::MIN + 1] {
        for &v in &[0i32, 42, -42, i32::MIN, i32::MAX] {
            diff_create_state(&format!("row2 cap={cap} v={v}"), v, cap);
        }
    }
    // The returned pointer must really be NULL on the C side (sentinel check).
    let (c, _r) = impls();
    let (p, out) = capture(|| unsafe { (c.create_state)(1, -1) });
    assert!(p.is_null(), "C create_state(1, -1) should return NULL");
    assert_eq!(
        String::from_utf8_lossy(&out),
        "Error: Failed to allocate buffer\n"
    );
}

// ===========================================================================
// Row 3 — positive-but-unsatisfiable capacity
// ===========================================================================

#[test]
fn err_03_create_state_huge_capacity() {
    for &cap in &[i32::MAX, i32::MAX - 1, 0x4000_0000, 0x2000_0000] {
        diff_create_state(&format!("row3 cap={cap}"), 12345, cap);
    }
}

// ===========================================================================
// Row 4 — destroy_state(NULL)
// ===========================================================================

#[test]
fn err_04_destroy_state_null() {
    let (c, r) = impls();
    let cv = capture(|| unsafe { (c.destroy_state)(std::ptr::null_mut()) });
    let rv = capture(|| unsafe { (r.destroy_state)(std::ptr::null_mut()) });
    assert!(cv.1.is_empty(), "C must print nothing for destroy_state(NULL)");
    assert!(rv.1.is_empty(), "Rust must print nothing for destroy_state(NULL)");
    assert_same("row4 destroy_state(NULL)", cv, rv);
}

// ===========================================================================
// Row 5 — destroy_state on a state whose buffer is NULL
// ===========================================================================

#[test]
fn err_05_destroy_state_null_buffer() {
    let (c, r) = impls();
    let s = NullBufferState::new();
    let cv = capture(|| unsafe { (c.destroy_state)(s.c_state) });
    let rv = capture(|| unsafe { (r.destroy_state)(s.r_state) });
    assert!(cv.1.is_empty() && rv.1.is_empty(), "no output expected");
    assert_same("row5 destroy_state(buffer=NULL)", cv, rv);
    // Both states are now freed; neutralise the destructor.
    std::mem::forget(s);
}

// ===========================================================================
// Row 6 — process_buffer(NULL, target)
// ===========================================================================

#[test]
fn err_06_process_buffer_null_state() {
    let (c, r) = impls();
    for t in [0u8, 1, b'0', b'a', 0x7F, 0x80, 0xFF] {
        let cv = capture(|| unsafe { (c.process_buffer)(std::ptr::null_mut(), t as c_char) });
        let rv = capture(|| unsafe { (r.process_buffer)(std::ptr::null_mut(), t as c_char) });
        assert_eq!(cv.0, -1, "C must return -1");
        assert_eq!(
            String::from_utf8_lossy(&cv.1),
            "Error: Null pointer in process_buffer\n"
        );
        assert_same(&format!("row6 target={t}"), cv, rv);
    }
}

// ===========================================================================
// Row 7 — process_buffer on a state with buffer == NULL
// ===========================================================================

#[test]
fn err_07_process_buffer_null_buffer() {
    let (c, r) = impls();
    let s = NullBufferState::new();
    for t in [0u8, b'a', 0xFF] {
        let cv = capture(|| unsafe { (c.process_buffer)(s.c_state, t as c_char) });
        let rv = capture(|| unsafe { (r.process_buffer)(s.r_state, t as c_char) });
        assert_eq!(cv.0, -1, "C must return -1");
        assert_eq!(
            String::from_utf8_lossy(&cv.1),
            "Error: Null pointer in process_buffer\n"
        );
        assert_same(&format!("row7 target={t}"), cv, rv);
    }
}

// ===========================================================================
// Row 8 — empty buffer (strlen == 0)
// ===========================================================================

#[test]
fn err_08_process_buffer_empty() {
    let (c, r) = impls();
    for t in [0u8, b'a', b'S', 0xFF] {
        let p = Pair::new(0, 64);
        unsafe {
            set_buffer(p.c_state, b"");
            set_buffer(p.r_state, b"");
        }
        let cv = capture(|| unsafe { (c.process_buffer)(p.c_state, t as c_char) });
        let rv = capture(|| unsafe { (r.process_buffer)(p.r_state, t as c_char) });
        assert_eq!(cv.0, 0, "C must return 0 for an empty buffer");
        assert!(cv.1.is_empty(), "C must print nothing for an empty buffer");
        assert_same(&format!("row8 target={t}"), cv, rv);
        p.assert_states_equal("row8");
    }
}

// ===========================================================================
// Row 9 — memchr finds nothing
// ===========================================================================

#[test]
fn err_09_process_buffer_no_match() {
    let (c, r) = impls();
    let p = Pair::new(0, 64);
    unsafe {
        set_buffer(p.c_state, b"abcdef");
        set_buffer(p.r_state, b"abcdef");
    }
    for t in [b'z', b'A', b'0', 0x80u8, 0xFF] {
        let cv = capture(|| unsafe { (c.process_buffer)(p.c_state, t as c_char) });
        let rv = capture(|| unsafe { (r.process_buffer)(p.r_state, t as c_char) });
        assert_eq!(cv.0, 0, "C must return 0 when the target is absent");
        assert!(cv.1.is_empty());
        assert_same(&format!("row9 target={t}"), cv, rv);
    }
    // Partial match then break: two 'a's then no more.
    unsafe {
        set_buffer(p.c_state, b"aXaXXXX");
        set_buffer(p.r_state, b"aXaXXXX");
    }
    let cv = capture(|| unsafe { (c.process_buffer)(p.c_state, b'a' as c_char) });
    let rv = capture(|| unsafe { (r.process_buffer)(p.r_state, b'a' as c_char) });
    assert_eq!(cv.0, 2);
    assert_same("row9 break-after-2", cv, rv);
    p.assert_states_equal("row9");
}

// ===========================================================================
// Row 10 — target == '\0' can never be found inside strlen() bytes
// ===========================================================================

#[test]
fn err_10_process_buffer_nul_target() {
    let (c, r) = impls();
    for &content in &[&b""[..], &b"a"[..], &b"State:0:Mode:3"[..], &b"\x01\x02\xff"[..]] {
        let p = Pair::new(0, 64);
        unsafe {
            set_buffer(p.c_state, content);
            set_buffer(p.r_state, content);
        }
        let cv = capture(|| unsafe { (c.process_buffer)(p.c_state, 0) });
        let rv = capture(|| unsafe { (r.process_buffer)(p.r_state, 0) });
        assert_eq!(cv.0, 0, "C must return 0 for target '\\0'");
        assert!(cv.1.is_empty());
        assert_same("row10 nul target", cv, rv);
        p.assert_states_equal("row10");
    }
}

// ===========================================================================
// Row 11 — update_flags(NULL, param)
// ===========================================================================

#[test]
fn err_11_update_flags_null() {
    let (c, r) = impls();
    for &param in &[0i32, 1, 7, 63, -1, i32::MIN, i32::MAX] {
        let cv = capture(|| unsafe { (c.update_flags)(std::ptr::null_mut(), param) });
        let rv = capture(|| unsafe { (r.update_flags)(std::ptr::null_mut(), param) });
        assert!(cv.1.is_empty(), "C must print nothing for a NULL state");
        assert_same(&format!("row11 param={param}"), cv, rv);
    }
}

// ===========================================================================
// Row 12 — confuse_types(NULL, operation)
// ===========================================================================

#[test]
fn err_12_confuse_types_null() {
    let (c, r) = impls();
    for &op in &[0i32, 1, 2, 3, 4, -1, i32::MIN, i32::MAX] {
        let cv = capture(|| unsafe { (c.confuse_types)(std::ptr::null_mut(), op) });
        let rv = capture(|| unsafe { (r.confuse_types)(std::ptr::null_mut(), op) });
        assert_eq!(cv.0, 0, "C must return 0 for a NULL state");
        assert!(cv.1.is_empty(), "C must print nothing for a NULL state");
        assert_same(&format!("row12 op={op}"), cv, rv);
    }
}

// ===========================================================================
// Row 13 — operation outside the switch (out-of-range "enum" values)
// ===========================================================================

#[test]
fn err_13_confuse_types_out_of_range_operation() {
    let (c, r) = impls();
    let ops: Vec<c_int> = vec![
        -1,
        -2,
        -3,
        -4,
        4,
        5,
        6,
        7,
        8,
        100,
        255,
        256,
        65536,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        0x7FFF_FFFE,
        -0x8000_0000,
    ];
    let payloads = [0u32, 0xFFFF_FFFF, 0x7FC0_0000, 1078530011, 0x8080_8080];
    for &op in &ops {
        for &payload in &payloads {
            let p = Pair::new(0, 64);
            unsafe {
                (*p.c_state).data = payload;
                (*p.r_state).data = payload;
            }
            let ctx = format!("row13 op={op} payload=0x{payload:08X}");
            let cv = capture(|| unsafe { (c.confuse_types)(p.c_state, op) });
            let rv = capture(|| unsafe { (r.confuse_types)(p.r_state, op) });
            assert_eq!(cv.0, 0, "[{ctx}] C must return 0");
            assert!(cv.1.is_empty(), "[{ctx}] C must print nothing");
            // The union must be untouched.
            assert_eq!(unsafe { (*p.c_state).data }, payload, "[{ctx}]");
            assert_same(&ctx, cv, rv);
            p.assert_states_equal(&ctx);
        }
    }
}

// ===========================================================================
// Row 14 — (int) cast of an out-of-range / NaN float => INT_MIN (cvttss2si)
// ===========================================================================

#[test]
fn err_14_confuse_types_float_cast_out_of_range() {
    let (c, r) = impls();
    // Every one of these has |x * 100| > INT_MAX, or is NaN/Inf.
    let cases: Vec<(u32, &str)> = vec![
        (f32::NAN.to_bits(), "nan"),
        (0x7FC0_0000, "quiet nan"),
        (0xFFC0_0000, "-quiet nan"),
        (0x7F80_0001, "signalling nan"),
        (0xFF80_0001, "-signalling nan"),
        (f32::INFINITY.to_bits(), "+inf"),
        (f32::NEG_INFINITY.to_bits(), "-inf"),
        (f32::MAX.to_bits(), "FLT_MAX"),
        (f32::MIN.to_bits(), "-FLT_MAX"),
        (1.0e30f32.to_bits(), "1e30"),
        ((-1.0e30f32).to_bits(), "-1e30"),
        (21474836.5f32.to_bits(), "just over INT_MAX/100"),
        ((-21474836.5f32).to_bits(), "just under INT_MIN/100"),
        (0x4F00_0000, "2^31"),
        (0xCF00_0000, "-2^31"),
    ];
    for (payload, name) in cases {
        let p = Pair::new(0, 64);
        unsafe {
            (*p.c_state).data = payload;
            (*p.r_state).data = payload;
        }
        let ctx = format!("row14 {name} payload=0x{payload:08X}");
        let cv = capture(|| unsafe { (c.confuse_types)(p.c_state, 1) });
        let rv = capture(|| unsafe { (r.confuse_types)(p.r_state, 1) });
        assert_eq!(
            cv.0,
            i32::MIN,
            "[{ctx}] C's (int) cast should yield the integer-indefinite value"
        );
        assert_same(&ctx, cv, rv);
        p.assert_states_equal(&ctx);
    }
}

// ===========================================================================
// Row 16 — negative `char` target across the FFI boundary
// ===========================================================================

#[test]
fn err_16_process_buffer_high_bit_target() {
    let (c, r) = impls();
    let content: &[u8] = b"\x80\xff\x7f\x81\xc3\xa9\x80\xff";
    for t in 0x80u8..=0xFF {
        let p = Pair::new(0, 64);
        unsafe {
            set_buffer(p.c_state, content);
            set_buffer(p.r_state, content);
        }
        let ctx = format!("row16 target=0x{t:02X} (as char {})", t as i8);
        let cv = capture(|| unsafe { (c.process_buffer)(p.c_state, t as i8 as c_char) });
        let rv = capture(|| unsafe { (r.process_buffer)(p.r_state, t as i8 as c_char) });
        let expect = content.iter().filter(|&&b| b == t).count() as c_int;
        assert_eq!(cv.0, expect, "[{ctx}] C count");
        assert_same(&ctx, cv, rv);
    }
}

// ===========================================================================
// Row 17 — capacity == 0
// ===========================================================================

#[test]
fn err_17_create_state_zero_capacity() {
    for &v in &[0i32, 1, -1, i32::MIN, i32::MAX] {
        diff_create_state(&format!("row17 v={v} cap=0"), v, 0);
    }
    // malloc(0) must return non-NULL, so create_state must succeed.
    let (c, r) = impls();
    let (cp, cout) = capture(|| unsafe { (c.create_state)(5, 0) });
    let (rp, rout) = capture(|| unsafe { (r.create_state)(5, 0) });
    assert!(!cp.is_null(), "C create_state(_, 0) should succeed");
    assert!(!rp.is_null(), "Rust create_state(_, 0) should succeed");
    assert_eq!(unsafe { (*cp).capacity }, 0);
    assert_eq!(unsafe { (*rp).capacity }, 0);
    assert!(!unsafe { (*cp).buffer }.is_null());
    assert!(!unsafe { (*rp).buffer }.is_null());
    assert_eq!(unsafe { (*cp).flags }, unsafe { (*rp).flags });
    assert_eq!(unsafe { (*cp).data }, unsafe { (*rp).data });
    assert_eq!(cout, rout);
    let _ = capture(|| unsafe { (c.destroy_state)(cp) });
    let _ = capture(|| unsafe { (r.destroy_state)(rp) });
}

// ===========================================================================
// Row 18 — snprintf truncation at tiny capacities
// ===========================================================================

#[test]
fn err_18_create_state_truncating_capacity() {
    for cap in 1i32..=30 {
        for &v in &[0i32, 7, 42, -7, 123456, -123456, i32::MIN, i32::MAX] {
            diff_create_state(&format!("row18 cap={cap} v={v}"), v, cap);
        }
    }
    // Exact expected truncation on the C side, for one case.
    let (c, _r) = impls();
    let (p, _) = capture(|| unsafe { (c.create_state)(0, 8) });
    assert!(!p.is_null());
    let s = unsafe { std::ffi::CStr::from_ptr((*p).buffer) };
    assert_eq!(s.to_bytes(), b"State:0", "snprintf must truncate and NUL-terminate");
    let _ = capture(|| unsafe { (c.destroy_state)(p) });
}

// ===========================================================================
// Row 19 — confusion with negative param3 / param4
// ===========================================================================

#[test]
fn err_19_confusion_negative_params() {
    let (c, r) = impls();
    for c3 in -25i32..=0 {
        for c4 in -8i32..=0 {
            let ctx = format!("row19 (7,5,{c3},{c4})");
            let cv = capture(|| unsafe { (c.confusion)(7, 5, c3, c4) });
            let rv = capture(|| unsafe { (r.confusion)(7, 5, c3, c4) });
            assert_same(&ctx, cv, rv);
        }
    }
}

// ===========================================================================
// Row 20 — INT_MIN / INT_MAX in every parameter
// ===========================================================================

#[test]
fn err_20_confusion_extreme_params() {
    let (c, r) = impls();
    let vals = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &a in &vals {
        for &b in &vals {
            let ctx = format!("row20 ({a},{b},{},{})", i32::MIN, i32::MAX);
            let cv = capture(|| unsafe { (c.confusion)(a, b, i32::MIN, i32::MAX) });
            let rv = capture(|| unsafe { (r.confusion)(a, b, i32::MIN, i32::MAX) });
            assert_same(&ctx, cv, rv);
            let ctx = format!("row20b ({a},{b},{},{})", i32::MAX, i32::MIN);
            let cv = capture(|| unsafe { (c.confusion)(a, b, i32::MAX, i32::MIN) });
            let rv = capture(|| unsafe { (r.confusion)(a, b, i32::MAX, i32::MIN) });
            assert_same(&ctx, cv, rv);
        }
    }
    for &cc in &vals {
        for &d in &vals {
            let ctx = format!("row20c ({},{},{cc},{d})", i32::MIN, i32::MIN);
            let cv = capture(|| unsafe { (c.confusion)(i32::MIN, i32::MIN, cc, d) });
            let rv = capture(|| unsafe { (r.confusion)(i32::MIN, i32::MIN, cc, d) });
            assert_same(&ctx, cv, rv);
        }
    }
}
