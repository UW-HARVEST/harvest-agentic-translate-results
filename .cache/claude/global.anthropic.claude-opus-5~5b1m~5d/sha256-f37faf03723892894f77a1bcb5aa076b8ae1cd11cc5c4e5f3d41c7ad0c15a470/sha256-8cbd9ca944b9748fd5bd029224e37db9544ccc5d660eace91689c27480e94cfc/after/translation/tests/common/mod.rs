// Shared differential-test harness.
//
// Loads BOTH shared objects through `libloading` and calls every function only
// through its exported C symbol -- never by calling the Rust crate directly --
// so the `#[no_mangle]` / `extern "C"` wrappers are exercised too.
#![allow(dead_code)]

use std::ffi::{c_char, c_double, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// The C `DataBlock` layout (lib.c:33-37)
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub value: c_double,
    pub label: [c_char; 20],
}

/// `sizeof(DataBlock)` on the LP64 SysV ABI.
pub const DATABLOCK_SIZE: usize = 40;

impl DataBlock {
    pub fn zeroed() -> DataBlock {
        DataBlock { id: 0, value: 0.0, label: [0; 20] }
    }

    /// Build a block from a raw 40-byte image so that *padding* bytes are set
    /// too -- `copy_data_block` is a `memcpy` and must reproduce them verbatim.
    pub fn from_bytes(raw: &[u8; DATABLOCK_SIZE]) -> DataBlock {
        unsafe { std::ptr::read_unaligned(raw.as_ptr() as *const DataBlock) }
    }

    pub fn as_bytes(&self) -> [u8; DATABLOCK_SIZE] {
        let mut out = [0u8; DATABLOCK_SIZE];
        unsafe {
            std::ptr::copy_nonoverlapping(
                self as *const DataBlock as *const u8,
                out.as_mut_ptr(),
                DATABLOCK_SIZE,
            );
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    crate_root().parent().expect("crate has a parent dir").to_path_buf()
}

/// The C library's name is derived from the *parent directory* name by
/// `c_src/CMakeLists.txt`, so discover it instead of hard-coding it.
pub fn c_lib_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DIFFTEST_C_LIB") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DIFFTEST_C_LIB={} does not exist", p.display());
        return p;
    }
    let build = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
            {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}. Build the C library first:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display(),
        found
    );
    found.pop().unwrap()
}

pub fn rust_lib_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DIFFTEST_RUST_LIB") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DIFFTEST_RUST_LIB={} does not exist", p.display());
        return p;
    }
    for profile in ["release", "debug"] {
        let p = crate_root().join("target").join(profile).join("liboverunder_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("liboverunder_lib.so not found; run `cargo build --release --offline` first");
}

// ---------------------------------------------------------------------------
// The loaded pair
// ---------------------------------------------------------------------------
pub type FnSafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
pub type FnProcessWithFallthrough = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnCopyDataBlock = unsafe extern "C" fn(*mut DataBlock, *const DataBlock);
pub type FnHandlePointerOperations = unsafe extern "C" fn(c_int) -> c_int;
pub type FnOverunder = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    pub safe_double_to_int: FnSafeDoubleToInt,
    pub process_with_fallthrough: FnProcessWithFallthrough,
    pub copy_data_block: FnCopyDataBlock,
    pub handle_pointer_operations: FnHandlePointerOperations,
    pub overunder: FnOverunder,
}

fn load(name: &'static str, path: &Path) -> Lib {
    // Leak the Library: the symbols must outlive the whole test process.
    let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
        libloading::Library::new(path).unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
    }));

    macro_rules! sym {
        ($t:ty, $n:literal) => {{
            let s: libloading::Symbol<'static, $t> = unsafe {
                lib.get($n)
                    .unwrap_or_else(|e| panic!("{} is missing symbol {}: {e}", name, stringify!($n)))
            };
            *s
        }};
    }

    Lib {
        name,
        path: path.to_path_buf(),
        safe_double_to_int: sym!(FnSafeDoubleToInt, b"safe_double_to_int\0"),
        process_with_fallthrough: sym!(FnProcessWithFallthrough, b"process_with_fallthrough\0"),
        copy_data_block: sym!(FnCopyDataBlock, b"copy_data_block\0"),
        handle_pointer_operations: sym!(FnHandlePointerOperations, b"handle_pointer_operations\0"),
        overunder: sym!(FnOverunder, b"overunder\0"),
    }
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

pub fn libs() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: load("C", &c_lib_path()),
        rs: load("RUST", &rust_lib_path()),
    })
}

// ---------------------------------------------------------------------------
// stdout capture
//
// Both libraries write through the process-wide libc `stdout`, so redirect
// fd 1 into a temp file around each call and read it back. `fflush(NULL)`
// flushes every open stream, which is required because redirecting fd 1 to a
// regular file makes `stdout` fully buffered.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Redirecting fd 1 is inherently process-global, and libtest itself writes its
/// progress lines to fd 1 from other threads. Those bytes would land inside a
/// capture window and be mistaken for library output, so the suite must run
/// single-threaded. Detect the two ways that can be requested and fail loudly
/// otherwise instead of producing flaky "mismatches".
pub fn assert_serial() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        if std::env::var("RUST_TEST_THREADS").as_deref() == Ok("1") {
            return;
        }
        let args: Vec<String> = std::env::args().collect();
        let mut ok = false;
        for (i, a) in args.iter().enumerate() {
            if a == "--test-threads=1" {
                ok = true;
            }
            if a == "--test-threads" && args.get(i + 1).map(|s| s.as_str()) == Some("1") {
                ok = true;
            }
        }
        assert!(
            ok,
            "these differential tests redirect the process-wide fd 1 and must run \
             single-threaded.\nRun them as:\n  \
             RUST_TEST_THREADS=1 cargo test --offline --release\nor\n  \
             cargo test --offline --release -- --test-threads=1"
        );
    });
}

/// Run `f` with fd 1 redirected into a scratch file; return `f`'s value plus
/// every byte it wrote to stdout.
pub fn capture_stdout<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::unix::io::AsRawFd;

    assert_serial();
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let mut tmp = tempfile();

    // Push anything libtest has buffered in Rust's own LineWriter out to the
    // real fd 1 *before* we take fd 1 over, then flush every libc stream.
    let _ = std::io::stdout().flush();
    unsafe {
        fflush(std::ptr::null_mut());
    }
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(tmp.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let value = f();

    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
    }

    let mut bytes = Vec::new();
    tmp.seek(SeekFrom::Start(0)).unwrap();
    tmp.read_to_end(&mut bytes).unwrap();
    (value, bytes)
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate_root().join("target"));
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join(format!(
        "difftest-{}-{}-{}.out",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("open scratch file");
    // Unlink immediately; the fd keeps it alive.
    std::fs::remove_file(&path).ok();
    f
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) -- fixed seed, reproducible
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
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
    /// Uniform in `[lo, hi]` inclusive, works across the whole `i32` range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64) as u64 + 1;
        if span == 0 {
            return self.next_i32();
        }
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Uniform `f64` in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }
    /// A completely arbitrary `f64` -- includes NaN, inf and subnormals.
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
}

pub const SEED: u64 = 0x5EED_D1FF_0000_2025;

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------
pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Compare `overunder` across both libraries: return value AND stdout bytes.
pub fn cmp_overunder(row: &str, a: c_int, b: c_int, c: c_int, d: c_int) {
    let l = libs();
    let (rc, oc) = capture_stdout(|| unsafe { (l.c.overunder)(a, b, c, d) });
    let (rr, or) = capture_stdout(|| unsafe { (l.rs.overunder)(a, b, c, d) });
    assert_eq!(
        rc, rr,
        "[{row}] overunder({a},{b},{c},{d}) return mismatch: C={rc} RUST={rr}"
    );
    assert_eq!(
        oc,
        or,
        "[{row}] overunder({a},{b},{c},{d}) stdout mismatch:\n  C    = \"{}\"\n  RUST = \"{}\"",
        show(&oc),
        show(&or)
    );
}

/// Compare `overunder` return values only (for very high-volume fuzzing).
pub fn cmp_overunder_ret(row: &str, a: c_int, b: c_int, c: c_int, d: c_int) {
    let l = libs();
    let (rc, _) = capture_stdout(|| unsafe { (l.c.overunder)(a, b, c, d) });
    let (rr, _) = capture_stdout(|| unsafe { (l.rs.overunder)(a, b, c, d) });
    assert_eq!(
        rc, rr,
        "[{row}] overunder({a},{b},{c},{d}) return mismatch: C={rc} RUST={rr}"
    );
}

pub fn cmp_sdti(row: &str, d: f64) {
    let l = libs();
    let rc = unsafe { (l.c.safe_double_to_int)(d) };
    let rr = unsafe { (l.rs.safe_double_to_int)(d) };
    assert_eq!(
        rc, rr,
        "[{row}] safe_double_to_int({d:?} bits={:#018x}) C={rc} RUST={rr}",
        d.to_bits()
    );
}

pub fn cmp_pwf(row: &str, code: c_int, base: c_int) {
    let l = libs();
    let rc = unsafe { (l.c.process_with_fallthrough)(code, base) };
    let rr = unsafe { (l.rs.process_with_fallthrough)(code, base) };
    assert_eq!(
        rc, rr,
        "[{row}] process_with_fallthrough({code},{base}) C={rc} RUST={rr}"
    );
}

pub fn cmp_hpo(row: &str, v: c_int) {
    let l = libs();
    let rc = unsafe { (l.c.handle_pointer_operations)(v) };
    let rr = unsafe { (l.rs.handle_pointer_operations)(v) };
    assert_eq!(
        rc, rr,
        "[{row}] handle_pointer_operations({v}) C={rc} RUST={rr}"
    );
}

/// Copy `src` into a destination pre-filled with `fill` on both libraries and
/// compare all 40 raw bytes (including the ABI padding).
pub fn cmp_cdb(row: &str, src_bytes: &[u8; DATABLOCK_SIZE], fill: u8) {
    let l = libs();
    let src = DataBlock::from_bytes(src_bytes);

    let mut dc = DataBlock::from_bytes(&[fill; DATABLOCK_SIZE]);
    let mut dr = DataBlock::from_bytes(&[fill; DATABLOCK_SIZE]);

    unsafe {
        (l.c.copy_data_block)(&mut dc, &src);
        (l.rs.copy_data_block)(&mut dr, &src);
    }

    let bc = dc.as_bytes();
    let br = dr.as_bytes();
    assert_eq!(
        bc.as_slice(),
        br.as_slice(),
        "[{row}] copy_data_block byte mismatch\n  src  = {src_bytes:02x?}\n  C    = {bc:02x?}\n  RUST = {br:02x?}"
    );
    assert_eq!(
        bc.as_slice(),
        src_bytes.as_slice(),
        "[{row}] copy_data_block did not reproduce all {DATABLOCK_SIZE} source bytes (C side)"
    );
}
