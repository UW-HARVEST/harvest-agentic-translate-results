//! Differential test harness: loads BOTH the C `libdriver.so` and the Rust
//! `libdriver.so` through `libloading` and compares their stdout byte-for-byte.
//!
//! The Rust functions are NEVER called directly — always through the `.so`
//! exports, so the `#[no_mangle]` wrappers are under test too.

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn unlink(path: *const c_char) -> c_int;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

/// Runs `f` with fd 1 redirected into a temp file and returns the raw bytes
/// written. Flushes the C stdio buffer (`fflush(NULL)`) before restoring, so
/// output produced by `printf`/`puts` inside the loaded libraries is captured
/// regardless of buffering mode.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    // Unique-ish path per call.
    let path = format!(
        "/tmp/driver_diff_{}_{}.out\0",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    );
    unsafe {
        // Make sure nothing already buffered leaks into our capture.
        fflush(std::ptr::null_mut());
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let tmp = open(path.as_ptr() as *const c_char, O_RDWR | O_CREAT | O_TRUNC, 0o600);
        assert!(tmp >= 0, "failed to open temp capture file {path}");
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp, 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        let _ = std::io::Write::flush(&mut std::io::stdout());

        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);

        // Read everything back.
        lseek(tmp, 0, 0);
        let mut out = Vec::new();
        let mut buf = vec![0u8; 1 << 16];
        loop {
            let n = read(tmp, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(tmp);
        unlink(path.as_ptr() as *const c_char);
        out
    }
}

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// ---------------------------------------------------------------------------
// library loading
// ---------------------------------------------------------------------------

type FnPrintLine = unsafe extern "C" fn(*const c_char);
type FnVoid = unsafe extern "C" fn();
type FnDriver = unsafe extern "C" fn(c_int);

/// One loaded implementation (C or Rust) with its four exported symbols
/// resolved by name — exactly as an external consumer would.
struct Impl {
    name: &'static str,
    _lib: Library,
    print_line: FnPrintLine,
    bad: FnVoid,
    good: FnVoid,
    driver: FnDriver,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        assert!(path.exists(), "{} shared object not found at {:?}", name, path);
        unsafe {
            let lib = Library::new(&path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
            let print_line: Symbol<FnPrintLine> = lib
                .get(b"printLine\0")
                .unwrap_or_else(|e| panic!("{name}: printLine: {e}"));
            let bad: Symbol<FnVoid> =
                lib.get(b"bad\0").unwrap_or_else(|e| panic!("{name}: bad: {e}"));
            let good: Symbol<FnVoid> =
                lib.get(b"good\0").unwrap_or_else(|e| panic!("{name}: good: {e}"));
            let driver: Symbol<FnDriver> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: driver: {e}"));
            let (print_line, bad, good, driver) = (*print_line, *bad, *good, *driver);
            Impl { name, _lib: lib, print_line, bad, good, driver }
        }
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn c_impl() -> &'static Impl {
    static ONCE: std::sync::OnceLock<Impl> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| Impl::load("C", repo_root().join("c_src/build/libdriver.so")))
}

fn rust_impl() -> &'static Impl {
    static ONCE: std::sync::OnceLock<Impl> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        // Lets the Phase D sweep point the harness at a specific profile's
        // cdylib (e.g. the dev-profile build) instead of the release one.
        if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
            return Impl::load("Rust", PathBuf::from(p));
        }
        let base = repo_root().join("translation/target");
        for p in ["release/libdriver.so", "debug/libdriver.so"] {
            let cand = base.join(p);
            if cand.exists() {
                return Impl::load("Rust", cand);
            }
        }
        panic!("Rust libdriver.so not found under {base:?}; run `cargo build --release`");
    })
}

/// Serializes fd-1 redirection across the (multi-threaded) test runner.
fn lock() -> std::sync::MutexGuard<'static, ()> {
    static M: std::sync::Mutex<()> = std::sync::Mutex::new(());
    M.lock().unwrap_or_else(|e| e.into_inner())
}

/// Runs `op` against both implementations and asserts byte-identical stdout.
fn differential<F>(label: &str, op: F)
where
    F: Fn(&Impl),
{
    let _g = lock();
    let c = capture(|| op(c_impl()));
    let r = capture(|| op(rust_impl()));
    if c != r {
        panic!(
            "DIVERGENCE [{label}]\n  C   ({} bytes): {:?}\n  Rust({} bytes): {:?}",
            c.len(),
            String::from_utf8_lossy(&c[..c.len().min(400)]),
            r.len(),
            String::from_utf8_lossy(&r[..r.len().min(400)]),
        );
    }
}

// ---------------------------------------------------------------------------
// deterministic PRNG (fixed seed => reproducible)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// A non-NUL byte, 0x01..=0xFF.
    fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }
    fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
}

/// Builds a NUL-terminated buffer from `bytes` (which must not contain NUL).
fn cstring(bytes: &[u8]) -> Vec<u8> {
    assert!(!bytes.contains(&0));
    let mut v = Vec::with_capacity(bytes.len() + 1);
    v.extend_from_slice(bytes);
    v.push(0);
    v
}

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

/// C1 — every single non-NUL byte value.
#[test]
fn cfg_c1_print_line_single_bytes() {
    for b in 1u8..=255 {
        let s = cstring(&[b]);
        differential(&format!("C1 byte {b:#04x}"), |i| unsafe {
            (i.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// C2 — randomized plain ASCII, length 2..=64.
#[test]
fn cfg_c2_print_line_random_ascii() {
    let mut rng = Rng::new(0xC2);
    for case in 0..512 {
        let len = 2 + rng.below(63);
        let bytes: Vec<u8> = (0..len).map(|_| b' ' + rng.below(95) as u8).collect();
        let s = cstring(&bytes);
        differential(&format!("C2 case {case}"), |i| unsafe {
            (i.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// C3 — randomized arbitrary non-NUL bytes (covers invalid UTF-8).
#[test]
fn cfg_c3_print_line_random_bytes() {
    let mut rng = Rng::new(0xC3);
    for case in 0..512 {
        let len = 1 + rng.below(512);
        let bytes: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let s = cstring(&bytes);
        differential(&format!("C3 case {case}"), |i| unsafe {
            (i.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// C4 — randomized content salted with printf conversion specifiers.
#[test]
fn cfg_c4_print_line_random_with_percent() {
    const SPECS: [&[u8]; 8] = [b"%s", b"%d", b"%n", b"%p", b"%%", b"%1000d", b"%.*s", b"%hhn"];
    let mut rng = Rng::new(0xC4);
    for case in 0..384 {
        let mut bytes = Vec::new();
        let chunks = 1 + rng.below(8);
        for _ in 0..chunks {
            bytes.extend_from_slice(SPECS[rng.below(SPECS.len())]);
            let filler = rng.below(8);
            for _ in 0..filler {
                bytes.push(b'a' + rng.below(26) as u8);
            }
        }
        let s = cstring(&bytes);
        differential(&format!("C4 case {case}"), |i| unsafe {
            (i.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// C5 — embedded control bytes, including newlines and CR.
#[test]
fn cfg_c5_print_line_embedded_control_bytes() {
    let mut rng = Rng::new(0xC5);
    for case in 0..256 {
        let len = 1 + rng.below(64);
        let bytes: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                0 => b'\n',
                1 => b'\r',
                2 => b'\t',
                _ => (1 + rng.below(31)) as u8,
            })
            .collect();
        let s = cstring(&bytes);
        differential(&format!("C5 case {case}"), |i| unsafe {
            (i.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// C6 — lengths straddling stdio buffer and page boundaries.
#[test]
fn cfg_c6_print_line_buffer_boundaries() {
    let mut rng = Rng::new(0xC6);
    for len in [4095usize, 4096, 4097, 8191, 8192, 8193, 65535, 65536, 65537, 1 << 20] {
        let bytes: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let s = cstring(&bytes);
        differential(&format!("C6 len {len}"), |i| unsafe {
            (i.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// C7 — NULL argument.
#[test]
fn cfg_c7_print_line_null() {
    differential("C7 null", |i| unsafe { (i.print_line)(std::ptr::null()) });
}

/// C8 — single `good()` call (static-storage helper).
#[test]
fn cfg_c8_good_single() {
    differential("C8 good", |i| unsafe { (i.good)() });
    // Also pin the absolute expectation so a "both print nothing" false pass
    // cannot hide here.
    let _g = lock();
    let out = capture(|| unsafe { (c_impl().good)() });
    assert_eq!(out, b"helperGood1 string\n", "C reference output changed");
}

/// C9 — repeated `good()` calls, N = 1..=64.
#[test]
fn cfg_c9_good_repeated() {
    for n in 1..=64usize {
        differential(&format!("C9 n={n}"), |i| unsafe {
            for _ in 0..n {
                (i.good)()
            }
        });
    }
}

/// C10 — single `bad()` call (automatic-storage helper, CWE-562).
#[test]
fn cfg_c10_bad_single() {
    differential("C10 bad", |i| unsafe { (i.bad)() });
}

/// C11 — repeated `bad()` calls, N = 1..=64 (dirty stack between iterations).
#[test]
fn cfg_c11_bad_repeated() {
    for n in 1..=64usize {
        differential(&format!("C11 n={n}"), |i| unsafe {
            for _ in 0..n {
                (i.bad)()
            }
        });
    }
}

/// C12 — `driver` with assorted truthy flags.
#[test]
fn cfg_c12_driver_truthy() {
    for v in [1i32, 2, -1, 42, i32::MAX, i32::MIN, 0x100, -0x100, 0x7FFF_FFFE] {
        differential(&format!("C12 useGood={v}"), |i| unsafe { (i.driver)(v) });
    }
}

/// C13 — `driver(0)`.
#[test]
fn cfg_c13_driver_falsy() {
    differential("C13 useGood=0", |i| unsafe { (i.driver)(0) });
}

/// C14 — randomized full-range i32 flags, zero mixed in.
#[test]
fn cfg_c14_driver_random_i32() {
    let mut rng = Rng::new(0xC14);
    for case in 0..512 {
        let v = if case % 7 == 0 { 0 } else { rng.i32() };
        differential(&format!("C14 case {case} useGood={v}"), |i| unsafe { (i.driver)(v) });
    }
}

/// C15 — randomized interleaving of every entry point, whole-session compare.
#[test]
fn cfg_c15_interleaved_pipeline() {
    for run in 0..8u64 {
        let mut rng = Rng::new(0xC150 + run);
        // Pre-generate the op script so both implementations execute the
        // identical sequence with identical inputs.
        enum Op {
            Driver(i32),
            Good,
            Bad,
            Print(Vec<u8>),
            PrintNull,
        }
        let mut ops = Vec::new();
        for _ in 0..512 {
            ops.push(match rng.below(5) {
                0 => Op::Driver(if rng.below(3) == 0 { 0 } else { rng.i32() }),
                1 => Op::Good,
                2 => Op::Bad,
                3 => {
                    let len = 1 + rng.below(96);
                    Op::Print(cstring(&(0..len).map(|_| rng.nonzero_byte()).collect::<Vec<_>>()))
                }
                _ => Op::PrintNull,
            });
        }
        differential(&format!("C15 run {run}"), |i| unsafe {
            for op in &ops {
                match op {
                    Op::Driver(v) => (i.driver)(*v),
                    Op::Good => (i.good)(),
                    Op::Bad => (i.bad)(),
                    Op::Print(s) => (i.print_line)(s.as_ptr() as *const c_char),
                    Op::PrintNull => (i.print_line)(std::ptr::null()),
                }
            }
        });
    }
}

/// C16 — `printLine` immediately after the helpers ran (dirty stack frame),
/// including feeding back the exact bytes `good()` emits.
#[test]
fn cfg_c16_print_line_after_helpers() {
    let echo = cstring(b"helperGood1 string");
    let echo_bad = cstring(b"helperBad string");
    differential("C16 good-then-print", |i| unsafe {
        (i.good)();
        (i.print_line)(echo.as_ptr() as *const c_char);
        (i.bad)();
        (i.print_line)(echo_bad.as_ptr() as *const c_char);
        (i.bad)();
        (i.print_line)(std::ptr::null());
        (i.good)();
    });
    // Deep recursion-ish stack churn before bad(), to make any dangling read
    // observable if it existed.
    differential("C16 dirty-stack-then-bad", |i| unsafe {
        let filler = cstring(&[b'Z'; 4096]);
        (i.print_line)(filler.as_ptr() as *const c_char);
        (i.bad)();
    });
}

// ===========================================================================
// Phase C — ERRORS.md rows
// ===========================================================================

/// E1 — `printLine(NULL)`: the null guard. Must produce zero bytes in both.
#[test]
fn err_e1_print_line_null() {
    differential("E1", |i| unsafe { (i.print_line)(std::ptr::null()) });
    let _g = lock();
    assert_eq!(
        capture(|| unsafe { (c_impl().print_line)(std::ptr::null()) }),
        Vec::<u8>::new(),
        "C printLine(NULL) unexpectedly produced output"
    );
    assert_eq!(
        capture(|| unsafe { (rust_impl().print_line)(std::ptr::null()) }),
        Vec::<u8>::new(),
        "Rust printLine(NULL) unexpectedly produced output"
    );
}

/// E2 — empty string: passes the guard, prints exactly "\n".
#[test]
fn err_e2_print_line_empty() {
    let s = cstring(b"");
    differential("E2", |i| unsafe { (i.print_line)(s.as_ptr() as *const c_char) });
    let _g = lock();
    assert_eq!(capture(|| unsafe { (c_impl().print_line)(s.as_ptr() as *const c_char) }), b"\n");
    assert_eq!(capture(|| unsafe { (rust_impl().print_line)(s.as_ptr() as *const c_char) }), b"\n");
}

/// E3 — format specifiers must be echoed, never interpreted.
#[test]
fn err_e3_print_line_format_specifiers() {
    for pat in [
        &b"%s"[..],
        b"%n",
        b"%p",
        b"%d",
        b"%%",
        b"%s %n %p %d %%",
        b"%999999999d",
        b"%.2000f",
        b"%hhn%hhn%hhn",
        b"AAAA%08x.%08x.%08x.%08x",
    ] {
        let s = cstring(pat);
        differential(&format!("E3 {:?}", String::from_utf8_lossy(pat)), |i| unsafe {
            (i.print_line)(s.as_ptr() as *const c_char)
        });
        // The bytes must survive verbatim: no format expansion.
        let _g = lock();
        let mut expect = pat.to_vec();
        expect.push(b'\n');
        assert_eq!(
            capture(|| unsafe { (c_impl().print_line)(s.as_ptr() as *const c_char) }),
            expect,
            "C interpreted the argument as a format string"
        );
        assert_eq!(
            capture(|| unsafe { (rust_impl().print_line)(s.as_ptr() as *const c_char) }),
            expect,
            "Rust interpreted the argument as a format string"
        );
    }
}

/// E4 — `bad()`: CWE-562. GCC materialises NULL, so nothing is printed.
/// Asserted as an exact sentinel (empty output) on both sides, not merely
/// "both did something".
#[test]
fn err_e4_bad_returns_dangling_is_null() {
    differential("E4", |i| unsafe { (i.bad)() });
    let _g = lock();
    let c = capture(|| unsafe { (c_impl().bad)() });
    let r = capture(|| unsafe { (rust_impl().bad)() });
    assert_eq!(c, Vec::<u8>::new(), "C bad() produced output: {:?}", String::from_utf8_lossy(&c));
    assert_eq!(r, Vec::<u8>::new(), "Rust bad() produced output: {:?}", String::from_utf8_lossy(&r));
}

/// E5 — `driver(0)` selects the defective `bad()` path.
#[test]
fn err_e5_driver_zero_selects_bad() {
    differential("E5", |i| unsafe { (i.driver)(0) });
    let _g = lock();
    let via_driver = capture(|| unsafe { (c_impl().driver)(0) });
    let direct = capture(|| unsafe { (c_impl().bad)() });
    assert_eq!(via_driver, direct, "C driver(0) did not behave like bad()");
    let r_driver = capture(|| unsafe { (rust_impl().driver)(0) });
    let r_bad = capture(|| unsafe { (rust_impl().bad)() });
    assert_eq!(r_driver, r_bad, "Rust driver(0) did not behave like bad()");
    assert_eq!(via_driver, r_driver);
}

/// E6 — out-of-range "enum" ints across the FFI boundary. Any non-zero int is
/// truthy; only 0 is falsy. No clamping or rejection in either impl.
#[test]
fn err_e6_driver_out_of_range_enum_values() {
    let mut vals: Vec<i32> = vec![
        -1,
        2,
        3,
        4,
        0x100,
        0xFFFF,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        -0x8000,
        1 << 30,
        -(1 << 30),
    ];
    let mut rng = Rng::new(0xE6);
    while vals.len() < 256 {
        let v = rng.i32();
        if v != 0 {
            vals.push(v);
        }
    }
    let good_out = {
        let _g = lock();
        capture(|| unsafe { (c_impl().good)() })
    };
    for v in vals {
        differential(&format!("E6 useGood={v}"), |i| unsafe { (i.driver)(v) });
        let _g = lock();
        assert_eq!(
            capture(|| unsafe { (rust_impl().driver)(v) }),
            good_out,
            "Rust driver({v}) did not take the good() branch"
        );
    }
}

/// E7 — oversized input: no length limit, no truncation.
#[test]
fn err_e7_print_line_oversized() {
    for len in [1usize << 20, (1 << 22) + 1] {
        let mut bytes = vec![b'q'; len];
        // Vary content so a length-only comparison can't pass accidentally.
        let mut rng = Rng::new(0xE7 ^ len as u64);
        for i in (0..len).step_by(997) {
            bytes[i] = rng.nonzero_byte();
        }
        let s = cstring(&bytes);
        differential(&format!("E7 len {len}"), |i| unsafe {
            (i.print_line)(s.as_ptr() as *const c_char)
        });
        let _g = lock();
        let out = capture(|| unsafe { (c_impl().print_line)(s.as_ptr() as *const c_char) });
        assert_eq!(out.len(), len + 1, "C truncated a {len}-byte string");
    }
}

/// E8 — non-UTF-8 byte sequences.
#[test]
fn err_e8_print_line_invalid_utf8() {
    let cases: Vec<Vec<u8>> = vec![
        vec![0xFF],
        vec![0xFE, 0xFF],
        vec![0x80],                   // lone continuation
        vec![0xC0, 0x80],             // overlong NUL encoding (no real NUL byte)
        vec![0xED, 0xA0, 0x80],       // UTF-16 surrogate
        vec![0xF4, 0x90, 0x80, 0x80], // > U+10FFFF
        vec![0xC2],                   // truncated 2-byte seq
        vec![0xE2, 0x82],             // truncated 3-byte seq
        (0x80u8..=0xFF).collect(),
        (0x01u8..=0x7F).collect(),
    ];
    for (n, bytes) in cases.iter().enumerate() {
        let s = cstring(bytes);
        differential(&format!("E8 case {n}"), |i| unsafe {
            (i.print_line)(s.as_ptr() as *const c_char)
        });
        let _g = lock();
        let mut expect = bytes.clone();
        expect.push(b'\n');
        assert_eq!(
            capture(|| unsafe { (rust_impl().print_line)(s.as_ptr() as *const c_char) }),
            expect,
            "Rust mangled non-UTF-8 bytes (case {n})"
        );
    }
    // Randomized invalid-UTF-8 fuzz.
    let mut rng = Rng::new(0xE8);
    for case in 0..256 {
        let len = 1 + rng.below(64);
        let bytes: Vec<u8> = (0..len).map(|_| (0x80 + rng.below(128)) as u8).collect();
        let s = cstring(&bytes);
        differential(&format!("E8 fuzz {case}"), |i| unsafe {
            (i.print_line)(s.as_ptr() as *const c_char)
        });
    }
}

/// E9 — NUL byte sitting at the very end of an allocation: proves neither impl
/// reads past the terminator.
#[test]
fn err_e9_print_line_pointer_at_boundary() {
    // A page-sized, page-aligned buffer whose last byte is the NUL.
    let page = 4096usize;
    let layout = std::alloc::Layout::from_size_align(page, page).unwrap();
    unsafe {
        let p = std::alloc::alloc(layout);
        assert!(!p.is_null());
        std::ptr::write_bytes(p, b'x', page - 1);
        *p.add(page - 1) = 0;
        differential("E9 full page", |i| (i.print_line)(p as *const c_char));

        // NUL as the only byte, at the end of the page.
        let last = p.add(page - 1);
        *last = 0;
        differential("E9 empty at page end", |i| (i.print_line)(last as *const c_char));

        std::alloc::dealloc(p, layout);
    }
}
