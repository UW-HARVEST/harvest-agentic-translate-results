// Shared differential-test harness.
//
// Both implementations are loaded as shared objects via `libloading` and driven
// only through their exported C symbols -- the Rust crate is NEVER called
// directly, so the `#[no_mangle]` / `extern "C"` wrappers are part of what is
// under test.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// FFI signatures (taken verbatim from c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub type FnSafeDoubleToInt = unsafe extern "C" fn(f64) -> i32;
pub type FnProcessWithFallthrough = unsafe extern "C" fn(i32, i32) -> i32;
pub type FnCopyDataBlock = unsafe extern "C" fn(*mut u8, *const u8);
pub type FnHandlePointerOperations = unsafe extern "C" fn(i32) -> i32;
pub type FnOverunder = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

/// `sizeof(DataBlock)` == 40, `_Alignof(DataBlock)` == 8 (verified against the
/// C compiler on this platform).
pub const DATABLOCK_SIZE: usize = 40;
pub const DATABLOCK_ALIGN: usize = 8;

/// Byte offsets inside `DataBlock`.
pub const OFF_ID: usize = 0;
pub const OFF_PAD1: usize = 4; // 4 padding bytes, copied by memcpy, unobservable via fields
pub const OFF_VALUE: usize = 8;
pub const OFF_LABEL: usize = 16;
pub const OFF_TAILPAD: usize = 36; // 4 tail-padding bytes

/// One loaded implementation.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub safe_double_to_int: FnSafeDoubleToInt,
    pub process_with_fallthrough: FnProcessWithFallthrough,
    pub copy_data_block: FnCopyDataBlock,
    pub handle_pointer_operations: FnHandlePointerOperations,
    pub overunder: FnOverunder,
    _lib: &'static Library,
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    let lib = unsafe { Library::new(&path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
    // Leaked on purpose: the function pointers below must stay valid for the
    // whole test-binary lifetime.
    let lib: &'static Library = Box::leak(Box::new(lib));

    macro_rules! sym {
        ($t:ty, $n:literal) => {{
            let s = unsafe { lib.get::<$t>(concat!($n, "\0").as_bytes()) }.unwrap_or_else(|e| {
                panic!("{} does not export `{}`: {e}", name, $n);
            });
            *s
        }};
    }

    Impl {
        name,
        path,
        safe_double_to_int: sym!(FnSafeDoubleToInt, "safe_double_to_int"),
        process_with_fallthrough: sym!(FnProcessWithFallthrough, "process_with_fallthrough"),
        copy_data_block: sym!(FnCopyDataBlock, "copy_data_block"),
        handle_pointer_operations: sym!(FnHandlePointerOperations, "handle_pointer_operations"),
        overunder: sym!(FnOverunder, "overunder"),
        _lib: lib,
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    // current_exe is <target>/<profile>/deps/<testbin>-<hash>; the cdylib lives
    // in <target>/<profile>/.
    let exe = std::env::current_exe().expect("current_exe");
    let mut dirs = Vec::new();
    if let Some(deps) = exe.parent() {
        dirs.push(deps.to_path_buf());
        if let Some(profile) = deps.parent() {
            dirs.push(profile.to_path_buf());
        }
    }
    dirs.push(manifest_dir().join("target/debug"));
    for d in &dirs {
        let p = d.join("liboverunder_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib `liboverunder_lib.so` not found in any of: {:?}. Run `cargo build` first.",
        dirs
    );
}

static C_IMPL: OnceLock<Impl> = OnceLock::new();
static RUST_IMPL: OnceLock<Impl> = OnceLock::new();

pub fn c_impl() -> &'static Impl {
    C_IMPL.get_or_init(|| load("C .so", c_so_path()))
}

pub fn rust_impl() -> &'static Impl {
    RUST_IMPL.get_or_init(|| load("Rust .so", rust_so_path()))
}

/// Both implementations, C first.
pub fn both() -> (&'static Impl, &'static Impl) {
    (c_impl(), rust_impl())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    /// A per-test stream derived from the global seed and a test-specific tag,
    /// so every row is reproducible independently of execution order.
    pub fn for_test(tag: &str) -> Self {
        let mut h: u64 = SEED;
        for b in tag.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100_0000_01B3);
        }
        Rng(h)
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    pub fn next_bool(&mut self) -> bool {
        self.next_u64() >> 63 == 1
    }
    /// Uniform in `[lo, hi]` (inclusive), works across the whole i32 range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64) as u64 + 1;
        let v = if span == 0 {
            self.next_u64()
        } else {
            self.next_u64() % span
        };
        (lo as i64 + v as i64) as i32
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Uniform in `[lo, hi)`.
    pub fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }
    /// An arbitrary 64-bit pattern reinterpreted as `f64` (covers NaNs, both
    /// infinities, subnormals and every exponent in one sweep).
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

// ---------------------------------------------------------------------------
// Differential helpers for the four non-printing leaf functions.
// ---------------------------------------------------------------------------

pub fn diff_safe_double_to_int(d: f64, ctx: &str) {
    let (c, r) = both();
    let cv = unsafe { (c.safe_double_to_int)(d) };
    let rv = unsafe { (r.safe_double_to_int)(d) };
    assert_eq!(
        cv, rv,
        "safe_double_to_int divergence [{ctx}]: d={d:?} (bits={:#018x}) C={cv} Rust={rv}",
        d.to_bits()
    );
}

pub fn diff_process(code: i32, base: i32, ctx: &str) -> i32 {
    let (c, r) = both();
    let cv = unsafe { (c.process_with_fallthrough)(code, base) };
    let rv = unsafe { (r.process_with_fallthrough)(code, base) };
    assert_eq!(
        cv, rv,
        "process_with_fallthrough divergence [{ctx}]: code={code} base={base} C={cv} Rust={rv}"
    );
    cv
}

pub fn diff_hpo(value: i32, ctx: &str) -> i32 {
    let (c, r) = both();
    let cv = unsafe { (c.handle_pointer_operations)(value) };
    let rv = unsafe { (r.handle_pointer_operations)(value) };
    assert_eq!(
        cv, rv,
        "handle_pointer_operations divergence [{ctx}]: value={value} C={cv} Rust={rv}"
    );
    cv
}

/// An 8-aligned heap arena, so `DataBlock*` arguments satisfy the C ABI.
pub struct Arena {
    ptr: *mut u8,
    layout: std::alloc::Layout,
    len: usize,
}

impl Arena {
    pub fn new(len: usize) -> Self {
        let layout = std::alloc::Layout::from_size_align(len, DATABLOCK_ALIGN).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };
        assert!(!ptr.is_null());
        unsafe { std::ptr::write_bytes(ptr, 0, len) };
        Arena { ptr, layout, len }
    }
    pub fn at(&self, off: usize) -> *mut u8 {
        assert!(off <= self.len);
        unsafe { self.ptr.add(off) }
    }
    pub fn read(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }.to_vec()
    }
    pub fn write(&self, off: usize, bytes: &[u8]) {
        assert!(off + bytes.len() <= self.len);
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.ptr.add(off), bytes.len()) };
    }
    pub fn fill(&self, byte: u8) {
        unsafe { std::ptr::write_bytes(self.ptr, byte, self.len) };
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, self.layout) };
    }
}

/// Copy `src_bytes` (40 bytes) through both `copy_data_block` implementations
/// into freshly-sentinel-filled destinations and require the whole destination
/// arenas -- padding included -- to be byte-identical.
pub fn diff_copy_data_block(src_bytes: &[u8], sentinel: u8, dest_off: usize, ctx: &str) {
    assert_eq!(src_bytes.len(), DATABLOCK_SIZE);
    let (c, r) = both();
    let arena_len = dest_off + DATABLOCK_SIZE + 56;

    let run = |f: FnCopyDataBlock| -> Vec<u8> {
        let dst = Arena::new(arena_len);
        dst.fill(sentinel);
        let src = Arena::new(DATABLOCK_SIZE);
        src.write(0, src_bytes);
        unsafe { f(dst.at(dest_off), src.at(0) as *const u8) };
        // Also return the source so an accidental write to the source is caught.
        let mut out = dst.read();
        out.extend_from_slice(&src.read());
        out
    };

    let cv = run(c.copy_data_block);
    let rv = run(r.copy_data_block);
    assert_eq!(
        cv, rv,
        "copy_data_block divergence [{ctx}]: dest_off={dest_off} sentinel={sentinel:#04x}\n  C   ={cv:02x?}\n  Rust={rv:02x?}"
    );
    // Sanity: the payload really did land where it should.
    assert_eq!(
        &cv[dest_off..dest_off + DATABLOCK_SIZE],
        src_bytes,
        "copy_data_block did not copy the payload [{ctx}]"
    );
}

// ---------------------------------------------------------------------------
// stdout capture -- `overunder` prints, so stdout must match byte-for-byte.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

/// fd 1 is process-global, so captures must be serialised.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

pub fn capture_lock() -> MutexGuard<'static, ()> {
    CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Run `f` with fd 1 redirected to a temporary file and return the bytes it
/// wrote. Uses libc `fflush(NULL)` so the C `.so`'s and the Rust `.so`'s shared
/// libc `stdout` buffer is drained at exactly the right moments.
pub fn capture_stdout<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    use std::os::unix::io::AsRawFd;

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("harvest_cap_{}_{}.bin", std::process::id(), n));

    // Drain Rust's own (independently buffered) stdout writer as well, so the
    // runner's progress lines cannot end up inside the capture file.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    let out;
    unsafe {
        // Drain anything already buffered so it is not attributed to us.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");

        {
            let file = std::fs::File::create(&path).expect("create capture file");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
        }

        out = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }

    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    (out, bytes)
}

/// Call `overunder` in both implementations and compare the return value *and*
/// the exact stdout bytes.
pub fn diff_overunder(a: i32, b: i32, c_arg: i32, d: i32, ctx: &str) -> i32 {
    let (c, r) = both();
    let _guard = capture_lock();

    let (cv, cout) = capture_stdout(|| unsafe { (c.overunder)(a, b, c_arg, d) });
    let (rv, rout) = capture_stdout(|| unsafe { (r.overunder)(a, b, c_arg, d) });

    assert_eq!(
        cv, rv,
        "overunder return divergence [{ctx}]: ({a},{b},{c_arg},{d}) C={cv} Rust={rv}"
    );
    if cout != rout {
        panic!(
            "overunder stdout divergence [{ctx}]: ({a},{b},{c_arg},{d})\n--- C ---\n{}\n--- Rust ---\n{}\n--- C bytes ---\n{:02x?}\n--- Rust bytes ---\n{:02x?}",
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout),
            cout,
            rout
        );
    }
    cv
}

/// Cheaper variant used for the high-iteration sweeps: still captures stdout
/// (so it is compared) but only formats a diagnostic on failure.
pub fn diff_overunder_quiet(a: i32, b: i32, c_arg: i32, d: i32) -> i32 {
    diff_overunder(a, b, c_arg, d, "sweep")
}

/// Interesting `i32` boundary values that the C code branches on.
pub fn i32_corners() -> Vec<i32> {
    vec![
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -1_431_655_766, // a * 1.5 < INT_MIN
        -1_431_655_765,
        -795_364_316, // b * 2.7 < INT_MIN
        -795_364_315,
        -46_342, // a*a overflows
        -46_341,
        -46_340,
        -32_768,
        -100,
        -7,
        -6,
        -5,
        -4,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        100,
        32_767,
        46_340,
        46_341,
        46_342,
        795_364_314,
        795_364_315, // b * 2.7 > INT_MAX
        1_073_741_823,
        1_073_741_824, // c * 2 overflows
        1_431_655_764,
        1_431_655_765, // a * 1.5 > INT_MAX
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
    ]
}

// ---------------------------------------------------------------------------
// Minimal single-threaded test runner for the `harness = false` binary.
// ---------------------------------------------------------------------------

pub struct Runner {
    pub filter: Option<String>,
    pub passed: usize,
    pub failed: Vec<String>,
    pub skipped: usize,
}

impl Runner {
    pub fn from_args() -> Self {
        // Ignore libtest-style flags cargo may forward; treat the first
        // non-flag argument as a substring filter.
        let filter = std::env::args()
            .skip(1)
            .find(|a| !a.starts_with('-'))
            .filter(|a| !a.is_empty());
        Runner {
            filter,
            passed: 0,
            failed: Vec::new(),
            skipped: 0,
        }
    }

    pub fn run(&mut self, name: &str, f: impl FnOnce() + std::panic::UnwindSafe) {
        if let Some(f) = &self.filter {
            if !name.contains(f.as_str()) {
                self.skipped += 1;
                return;
            }
        }
        print!("test {name} ... ");
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        match std::panic::catch_unwind(f) {
            Ok(()) => {
                println!("ok");
                self.passed += 1;
            }
            Err(_) => {
                println!("FAILED");
                self.failed.push(name.to_string());
            }
        }
    }

    pub fn finish(self) -> ! {
        println!();
        if self.failed.is_empty() {
            println!(
                "test result: ok. {} passed; 0 failed; {} filtered out",
                self.passed, self.skipped
            );
            std::process::exit(0);
        } else {
            println!("failures:");
            for f in &self.failed {
                println!("    {f}");
            }
            println!(
                "test result: FAILED. {} passed; {} failed; {} filtered out",
                self.passed,
                self.failed.len(),
                self.skipped
            );
            std::process::exit(1);
        }
    }
}
