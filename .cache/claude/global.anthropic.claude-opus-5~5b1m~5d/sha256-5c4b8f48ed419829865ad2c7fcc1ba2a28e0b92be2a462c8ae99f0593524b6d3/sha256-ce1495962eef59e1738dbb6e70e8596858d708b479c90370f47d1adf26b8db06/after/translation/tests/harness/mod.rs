//! Differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls only their exported
//! `extern "C"` symbols — the Rust crate is never linked or called directly, so
//! the `#[no_mangle]` wrappers are part of what is under test.
//!
//! stdout is captured at the *file-descriptor* level (both libraries write via
//! libc `printf`/`puts`, so nothing else can observe it), which is a
//! process-global operation; every capture therefore takes a global mutex.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// house_t, mirrored exactly (repr(C), int/int/double)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct House {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: f64,
}

impl House {
    pub fn new(floors: i32, bedrooms: i32, bathrooms: f64) -> Self {
        House {
            floors,
            bedrooms,
            bathrooms,
        }
    }
    /// The initial state `driver` builds internally.
    pub fn driver_default() -> Self {
        House::new(2, 5, 2.5)
    }
    /// Compare including the exact IEEE-754 bit pattern of `bathrooms`, so a
    /// `-0.0` / `+0.0` or NaN-payload divergence cannot slip through.
    pub fn bitwise_eq(&self, other: &House) -> bool {
        self.floors == other.floors
            && self.bedrooms == other.bedrooms
            && self.bathrooms.to_bits() == other.bathrooms.to_bits()
    }
    pub fn dbg(&self) -> String {
        format!(
            "House {{ floors: {}, bedrooms: {}, bathrooms: {:?} (bits 0x{:016x}) }}",
            self.floors,
            self.bedrooms,
            self.bathrooms,
            self.bathrooms.to_bits()
        )
    }
}

// ---------------------------------------------------------------------------
// libc bits needed for fd-level stdout capture
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

type DriverFn = unsafe extern "C" fn(*const c_char);
type RunFn = unsafe extern "C" fn(*mut House, c_int);

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub driver: DriverFn,
    pub run: RunFn,
}

pub struct Libs {
    pub c: Impl,
    pub rs: Impl,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().parent().unwrap().join("c_src");
    for cand in [
        base.join("build/libdriver.so"),
        base.join("build/lib/libdriver.so"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "C shared library not found. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    )
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // `cargo test` writes the freshly built cdylib into `target/<prof>/deps/`
    // and only "uplifts" a hardlink to `target/<prof>/` during `cargo build`,
    // so the uplifted copy can be stale. Consider every candidate and pick the
    // NEWEST one, then assert it is at least as new as the source.
    let exe = std::env::current_exe().expect("current_exe");
    let exe_dir = exe.parent().unwrap().to_path_buf();
    let mut cands = vec![exe_dir.join("libdriver.so")];
    if exe_dir.ends_with("deps") {
        cands.push(exe_dir.parent().unwrap().join("libdriver.so"));
    }
    cands.push(manifest_dir().join("target/release/deps/libdriver.so"));
    cands.push(manifest_dir().join("target/release/libdriver.so"));
    cands.push(manifest_dir().join("target/debug/deps/libdriver.so"));
    cands.push(manifest_dir().join("target/debug/libdriver.so"));

    let newest = cands
        .iter()
        .filter(|p| p.exists())
        .max_by_key(|p| {
            std::fs::metadata(p)
                .and_then(|md| md.modified())
                .unwrap_or(std::time::UNIX_EPOCH)
        })
        .cloned();

    match newest {
        Some(p) => {
            assert_fresh(&p);
            p
        }
        None => panic!(
            "Rust cdylib not found; tried {cands:?}. Build it with `cargo build --release`."
        ),
    }
}

/// Guard against the trap that `cargo test` may not rebuild a `cdylib`-only
/// library: if `src/lib.rs` is newer than the `.so` we are about to dlopen, the
/// whole suite would silently validate stale machine code. Refuse to run.
fn assert_fresh(so: &PathBuf) {
    let src = manifest_dir().join("src/lib.rs");
    let m = |p: &PathBuf| std::fs::metadata(p).and_then(|md| md.modified()).ok();
    if let (Some(t_src), Some(t_so)) = (m(&src), m(so)) {
        assert!(
            t_so >= t_src,
            "STALE cdylib: {so:?} is older than {src:?}.\n\
             The tests would be validating out-of-date machine code.\n\
             Run `cargo build --release` (or use ./run_tests.sh) first."
        );
    }
}

fn load(name: &'static str, path: &PathBuf) -> Impl {
    unsafe {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({:?}): {e}", name, path));
        let driver: Symbol<DriverFn> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("{} is missing exported symbol `driver`: {e}", name));
        let run: Symbol<RunFn> = lib
            .get(b"run\0")
            .unwrap_or_else(|e| panic!("{} is missing exported symbol `run`: {e}", name));
        let driver = *driver;
        let run = *run;
        Impl {
            name,
            _lib: lib,
            driver,
            run,
        }
    }
}

/// Path of the C `.so` actually under test, as a string (for `nm` / `ldd`).
pub fn c_so_str() -> String {
    c_so_path().to_string_lossy().into_owned()
}

/// Path of the Rust `.so` actually under test, as a string (for `nm` / `ldd`).
pub fn rust_so_str() -> String {
    rust_so_path().to_string_lossy().into_owned()
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| Libs {
        c: load("C libdriver.so", &c_so_path()),
        rs: load("Rust libdriver.so", &rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Run `f` with fd 1 redirected into a fresh temp file and return everything
/// written to it. Serialised process-wide.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    // libtest's own progress output ("test foo ... ok") goes through Rust's
    // global, buffered stdout. Hold its lock for the whole window so libtest
    // cannot flush into our redirected fd, and drain whatever is pending first.
    let mut rust_stdout = std::io::stdout().lock();
    let _ = std::io::Write::flush(&mut rust_stdout);

    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    let out = {
        let file = std::fs::File::create(&path).expect("create temp capture file");
        use std::os::unix::io::AsRawFd;
        let tmp_fd = file.as_raw_fd();

        unsafe {
            // Flush anything already pending on the real stdout.
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(tmp_fd, 1) >= 0, "dup2 failed");

            f();

            // Flush *all* libc output streams so the bytes land in the file.
            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
            close(saved);
        }
        drop(file);
        std::fs::read(&path).expect("read temp capture file")
    };

    let _ = std::fs::remove_file(&path);
    drop(rust_stdout);
    drop(guard);
    out
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// NUL-terminate a raw byte slice (allows embedded NULs, unlike `CString`).
pub fn cbuf(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

/// Call `driver` in both implementations with the same NUL-terminated buffer
/// and assert the stdout bytes are identical. Returns the (shared) output.
pub fn diff_driver_raw(buf: &[u8], label: &str) -> Vec<u8> {
    let l = libs();
    let z = cbuf(buf);
    let p = z.as_ptr() as *const c_char;

    let c_out = capture_stdout(|| unsafe { (l.c.driver)(p) });
    let rs_out = capture_stdout(|| unsafe { (l.rs.driver)(p) });

    assert_eq!(
        c_out,
        rs_out,
        "driver() stdout mismatch [{label}]\n  input : \"{}\"\n  C     : \"{}\"\n  Rust  : \"{}\"",
        show(buf),
        show(&c_out),
        show(&rs_out)
    );
    c_out
}

pub fn diff_driver(s: &str, label: &str) -> Vec<u8> {
    diff_driver_raw(s.as_bytes(), label)
}

/// Call `run` in both implementations starting from the same `house` and
/// `extra_bedrooms`; assert both the stdout bytes and the mutated struct match.
pub fn diff_run(house: House, extra: c_int, label: &str) -> (Vec<u8>, House) {
    let l = libs();

    let mut hc = house;
    let mut hr = house;

    let c_out = capture_stdout(|| unsafe { (l.c.run)(&mut hc as *mut House, extra) });
    let rs_out = capture_stdout(|| unsafe { (l.rs.run)(&mut hr as *mut House, extra) });

    assert_eq!(
        c_out,
        rs_out,
        "run() stdout mismatch [{label}]\n  in    : {}\n  extra : {extra}\n  C     : \"{}\"\n  Rust  : \"{}\"",
        house.dbg(),
        show(&c_out),
        show(&rs_out)
    );
    assert!(
        hc.bitwise_eq(&hr),
        "run() out-struct mismatch [{label}]\n  in    : {}\n  extra : {extra}\n  C     : {}\n  Rust  : {}",
        house.dbg(),
        hc.dbg(),
        hr.dbg()
    );
    (c_out, hc)
}

/// Feed the *same* struct through `run` `n` times, comparing after every call.
/// This exercises the composed pipeline / accumulated state, not one call.
pub fn diff_run_sequence(start: House, extras: &[c_int], label: &str) {
    let l = libs();
    let mut hc = start;
    let mut hr = start;
    for (i, &extra) in extras.iter().enumerate() {
        let before_c = hc;
        let c_out = capture_stdout(|| unsafe { (l.c.run)(&mut hc as *mut House, extra) });
        let rs_out = capture_stdout(|| unsafe { (l.rs.run)(&mut hr as *mut House, extra) });
        assert_eq!(
            c_out,
            rs_out,
            "run() sequence stdout mismatch [{label}] at call {i}\n  before: {}\n  extra : {extra}\n  C   : \"{}\"\n  Rust: \"{}\"",
            before_c.dbg(),
            show(&c_out),
            show(&rs_out)
        );
        assert!(
            hc.bitwise_eq(&hr),
            "run() sequence struct mismatch [{label}] at call {i}\n  before: {}\n  extra : {extra}\n  C   : {}\n  Rust: {}",
            before_c.dbg(),
            hc.dbg(),
            hr.dbg()
        );
    }
}

pub const ERR_MSG: &[u8] = b"An error occurred\n";

/// Assert both implementations took the *rejecting* branch and produced exactly
/// the error sentinel — not merely "both failed somehow".
pub fn assert_rejected(buf: &[u8], label: &str) {
    let out = diff_driver_raw(buf, label);
    assert_eq!(
        out,
        ERR_MSG,
        "expected the rejection sentinel for [{label}] input \"{}\", got \"{}\"",
        show(buf),
        show(&out)
    );
}

/// Assert both implementations took the *accepting* branch: 8 `print_house`
/// lines (2 `run` calls x 4 prints) and no error sentinel. Also recompute the
/// expected text from the C algorithm to pin the values, not just C==Rust.
pub fn assert_accepted(buf: &[u8], x: i32, label: &str) {
    let out = diff_driver_raw(buf, label);
    assert_ne!(
        out,
        ERR_MSG,
        "expected acceptance for [{label}] input \"{}\"",
        show(buf)
    );
    let lines = out.split(|&b| b == b'\n').filter(|l| !l.is_empty()).count();
    assert_eq!(
        lines,
        8,
        "expected 8 print_house lines for [{label}] input \"{}\", got \"{}\"",
        show(buf),
        show(&out)
    );
    // Independent model of driver(): two run() passes over one house.
    let expected = model_driver(x);
    assert_eq!(
        out,
        expected,
        "driver() output disagrees with the C model for [{label}] input \"{}\" (x={x})\n  got     : \"{}\"\n  expected: \"{}\"",
        show(buf),
        show(&out),
        show(&expected)
    );
}

/// Faithful re-implementation of the C control flow, used as a third opinion.
pub fn model_driver(x: i32) -> Vec<u8> {
    let mut h = House::driver_default();
    let mut out = Vec::new();
    for _ in 0..2 {
        model_run(&mut h, x, &mut out);
    }
    out
}

pub fn model_run(h: &mut House, extra: i32, out: &mut Vec<u8>) {
    model_print(h, out);
    h.floors = h.floors.wrapping_add(1);
    model_print(h, out);
    h.bathrooms += 1.0;
    model_print(h, out);
    h.bedrooms = h.bedrooms.wrapping_add(extra);
    model_print(h, out);
}

fn model_print(h: &House, out: &mut Vec<u8>) {
    // Only used for finite, modestly-sized bathroom values (driver's 2.5/3.5/…),
    // where Rust's `{:.1}` and C's `%.1f` provably agree.
    out.extend_from_slice(
        format!(
            "The house has {} floors, {} bedrooms, and {:.1} bathrooms\n",
            h.floors, h.bedrooms, h.bathrooms
        )
        .as_bytes(),
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — reproducible property-style testing
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x5eed_1234_dead_beef;
    pub fn new() -> Self {
        Rng(Self::SEED)
    }
    pub fn with_seed(s: u64) -> Self {
        Rng(s)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (hi as i128 - lo as i128 + 1) as u128;
        (lo as i128 + (self.next_u64() as u128 % span) as i128) as i64
    }
    /// Arbitrary bit pattern reinterpreted as f64: reaches normals, subnormals,
    /// ±0, ±inf and NaNs with assorted payloads.
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A finite double spanning the whole exponent range.
    pub fn next_finite_f64(&mut self) -> f64 {
        loop {
            let mant = 1.0 + (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
            let exp = self.range_i64(-320, 308) as i32;
            let sign = if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
            let v = sign * mant * 10f64.powi(exp);
            if v.is_finite() {
                return v;
            }
        }
    }
    pub fn random_house(&mut self, finite_bathrooms: bool) -> House {
        House {
            floors: self.next_i32(),
            bedrooms: self.next_i32(),
            bathrooms: if finite_bathrooms {
                self.next_finite_f64()
            } else {
                self.next_f64_bits()
            },
        }
    }
}
