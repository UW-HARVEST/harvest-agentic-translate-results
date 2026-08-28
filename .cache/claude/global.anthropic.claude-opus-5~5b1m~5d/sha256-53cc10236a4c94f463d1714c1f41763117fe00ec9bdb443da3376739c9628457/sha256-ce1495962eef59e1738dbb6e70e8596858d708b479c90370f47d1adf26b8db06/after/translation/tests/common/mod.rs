//! Shared differential-testing harness.
//!
//! Both the C library and the Rust library are loaded as shared objects with
//! `libloading` and every call goes through `dlsym`. The Rust functions are
//! *never* called directly, so the `#[no_mangle] extern "C"` wrappers are part
//! of what is under test.

#![allow(dead_code)]

use std::ffi::c_char;
use std::ffi::c_int;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::Library;
use libloading::Symbol;

// ---------------------------------------------------------------------------
// C signatures, as function-pointer types resolved via `dlsym`.
// ---------------------------------------------------------------------------

pub type FnConvertDoubleToInt = unsafe extern "C" fn(f64) -> c_int;
pub type FnFindValueInBuffer = unsafe extern "C" fn(*const c_char, usize, c_int) -> c_int;
pub type FnProcessNegation = unsafe extern "C" fn(c_int) -> c_int;
pub type FnCreateNumericBuffer = unsafe extern "C" fn(*mut c_char, c_int, c_int);
pub type FnCalculateWithDoubles = unsafe extern "C" fn(c_int, c_int, c_int) -> f64;
pub type FnDoubleneg = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// Every export of the library, resolved out of one `.so`.
pub struct Api {
    /// Kept alive so the resolved symbols stay valid.
    _lib: Library,
    pub label: &'static str,
    pub convert_double_to_int: FnConvertDoubleToInt,
    pub find_value_in_buffer: FnFindValueInBuffer,
    pub process_negation: FnProcessNegation,
    pub create_numeric_buffer: FnCreateNumericBuffer,
    pub calculate_with_doubles: FnCalculateWithDoubles,
    pub doubleneg: FnDoubleneg,
}

impl Api {
    fn load(path: &Path, label: &'static str) -> Api {
        // SAFETY: loading a library runs its initialisers; these are plain C /
        // Rust cdylibs with no unusual constructors.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));

        unsafe fn sym<T: Copy>(lib: &Library, name: &[u8], path: &Path) -> T {
            let s: Symbol<T> = lib
                .get(name)
                .unwrap_or_else(|e| {
                    panic!(
                        "dlsym({}) missing from {}: {e}",
                        String::from_utf8_lossy(name),
                        path.display()
                    )
                });
            *s
        }

        unsafe {
            Api {
                convert_double_to_int: sym(&lib, b"convert_double_to_int\0", path),
                find_value_in_buffer: sym(&lib, b"find_value_in_buffer\0", path),
                process_negation: sym(&lib, b"process_negation\0", path),
                create_numeric_buffer: sym(&lib, b"create_numeric_buffer\0", path),
                calculate_with_doubles: sym(&lib, b"calculate_with_doubles\0", path),
                doubleneg: sym(&lib, b"doubleneg\0", path),
                _lib: lib,
                label,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<project>.so` — the project name is derived from the parent
/// directory of `c_src` by `CMakeLists.txt`, so it is globbed rather than named.
pub fn c_so_path() -> PathBuf {
    let build_dir = manifest_dir().parent().unwrap().join("c_src").join("build");
    assert!(
        build_dir.is_dir(),
        "{} does not exist -- build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build_dir.display()
    );

    let mut hits: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("lib") && name.ends_with(".so") && p.is_file()
        })
        .collect();
    hits.sort();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one lib*.so in {}, found {hits:?}",
        build_dir.display()
    );
    hits.pop().unwrap()
}

/// `target/<profile>/libdoubleneg_lib.so`, discovered relative to the running
/// test executable so it always matches the profile `cargo test` used.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>
    let deps = exe.parent().expect("deps dir");
    let candidates = [
        deps.parent().map(|p| p.join("libdoubleneg_lib.so")),
        Some(deps.join("libdoubleneg_lib.so")),
    ];
    for c in candidates.into_iter().flatten() {
        if c.is_file() {
            return c;
        }
    }
    panic!(
        "libdoubleneg_lib.so not found next to {} -- run `cargo build` first",
        exe.display()
    );
}

/// Newest modification time under `dir` for files matching `ext`.
fn newest_mtime(dir: &Path, ext: &str) -> Option<std::time::SystemTime> {
    let mut newest = None;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == ext) {
                if let Ok(m) = entry.metadata().and_then(|m| m.modified()) {
                    if newest.is_none_or(|n| m > n) {
                        newest = Some(m);
                    }
                }
            }
        }
    }
    newest
}

/// `cargo test` does **not** rebuild a `cdylib` target: for
/// `crate-type = ["cdylib"]` it compiles `src/lib.rs` into a *test harness
/// binary*, leaving `target/<profile>/libdoubleneg_lib.so` untouched. Without
/// this guard the whole differential suite can silently validate a stale `.so`
/// and report success for code that was never built.
///
/// The same applies to the C side if `src/lib.c` changed since `cmake --build`.
fn assert_libraries_are_fresh(c_so: &Path, rust_so: &Path) {
    let manifest = manifest_dir();

    let rust_so_time = rust_so.metadata().and_then(|m| m.modified()).ok();
    let rust_src_time = newest_mtime(&manifest.join("src"), "rs");
    if let (Some(so), Some(src)) = (rust_so_time, rust_src_time) {
        assert!(
            so >= src,
            "STALE Rust .so: {} is older than the newest file in {}/src.\n\
             `cargo test` does not rebuild a cdylib -- run:\n    \
             cargo build --release   (or ./run_all_features.sh)",
            rust_so.display(),
            manifest.display()
        );
    }

    let c_root = manifest.parent().unwrap().join("c_src");
    let c_so_time = c_so.metadata().and_then(|m| m.modified()).ok();
    let c_src_time = newest_mtime(&c_root.join("src"), "c")
        .into_iter()
        .chain(newest_mtime(&c_root.join("include"), "h"))
        .max();
    if let (Some(so), Some(src)) = (c_so_time, c_src_time) {
        assert!(
            so >= src,
            "STALE C .so: {} is older than the C sources in {}.\n\
             Rebuild with:\n    cd c_src/build && cmake --build .",
            c_so.display(),
            c_root.display()
        );
    }
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();
static FRESHNESS: OnceLock<()> = OnceLock::new();

fn check_freshness_once() {
    FRESHNESS.get_or_init(|| assert_libraries_are_fresh(&c_so_path(), &rust_so_path()));
}

pub fn c_api() -> &'static Api {
    check_freshness_once();
    C_API.get_or_init(|| Api::load(&c_so_path(), "C"))
}

pub fn rust_api() -> &'static Api {
    check_freshness_once();
    RUST_API.get_or_init(|| Api::load(&rust_so_path(), "Rust"))
}

/// Convenience: `(c, rust)`.
pub fn both() -> (&'static Api, &'static Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- no external dev-dependency needed and the
// sequence is fixed, so failures are reproducible.
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

    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    /// Uniform in `0..n` (`n > 0`).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }

    /// Uniform in `[-scale, scale]`.
    pub fn next_f64_scaled(&mut self, scale: f64) -> f64 {
        let unit = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        (unit * 2.0 - 1.0) * scale
    }

    pub fn fill_bytes(&mut self, out: &mut [i8]) {
        for slot in out.iter_mut() {
            *slot = self.next_u8() as i8;
        }
    }
}

// ---------------------------------------------------------------------------
// Bit-exact float comparison (so NaN payloads and -0.0 are distinguished).
// ---------------------------------------------------------------------------

pub fn assert_f64_bits_eq(c: f64, rust: f64, ctx: impl std::fmt::Display) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{ctx}: C returned {c:?} (bits {:#018x}), Rust returned {rust:?} (bits {:#018x})",
        c.to_bits(),
        rust.to_bits()
    );
}

// ---------------------------------------------------------------------------
// stdout capture at the file-descriptor level.
//
// `doubleneg` produces most of its observable behaviour through `printf`, and
// both libraries print through the *process's* libc `stdout`. Comparing the
// bytes therefore requires redirecting fd 1 around each call.
// ---------------------------------------------------------------------------

mod libc_shim {
    use std::ffi::c_int;
    use std::ffi::c_void;

    extern "C" {
        pub fn dup(oldfd: c_int) -> c_int;
        pub fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
        pub fn close(fd: c_int) -> c_int;
        /// `fflush(NULL)` flushes *all* open output streams, which is exactly
        /// what is needed here: it reaches the `stdout` of whichever library
        /// just wrote to it without needing the `FILE*` symbol.
        pub fn fflush(stream: *mut c_void) -> c_int;
    }
}

/// Serialises fd-1 redirection, which is process-global state.
static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ---------------------------------------------------------------------------
// Minimal sequential test runner for the `harness = false` test binaries.
//
// libtest writes its own progress lines ("test foo ... ok") to the real fd 1 as
// each test finishes. Any test that redirects fd 1 therefore captures that noise
// when tests run in parallel, which corrupts the byte-for-byte comparison. The
// stdout-capturing binaries opt out of libtest entirely and run sequentially
// here, reporting progress on **stderr** so it can never pollute a capture.
// ---------------------------------------------------------------------------

pub fn run_sequentially(suite: &str, tests: &[(&str, fn())]) {
    let filter: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .collect();

    let selected: Vec<&(&str, fn())> = tests
        .iter()
        .filter(|(name, _)| filter.is_empty() || filter.iter().any(|f| name.contains(f.as_str())))
        .collect();

    eprintln!("\nrunning {} tests ({suite}, sequential)", selected.len());

    let mut failures: Vec<(String, String)> = Vec::new();
    for (name, f) in &selected {
        eprint!("test {name} ... ");
        let hook = std::panic::take_hook();
        // Silence the default hook; the payload is reported below instead.
        std::panic::set_hook(Box::new(|_| {}));
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        std::panic::set_hook(hook);

        match outcome {
            Ok(()) => eprintln!("ok"),
            Err(payload) => {
                let msg = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "<non-string panic payload>".to_string());
                eprintln!("FAILED");
                failures.push((name.to_string(), msg));
            }
        }
    }

    if failures.is_empty() {
        eprintln!(
            "\ntest result: ok. {} passed; 0 failed ({suite})\n",
            selected.len()
        );
        return;
    }

    eprintln!("\nfailures:");
    for (name, msg) in &failures {
        eprintln!("\n---- {name} ----\n{msg}");
    }
    eprintln!(
        "\ntest result: FAILED. {} passed; {} failed ({suite})\n",
        selected.len() - failures.len(),
        failures.len()
    );
    std::process::exit(1);
}

/// Runs `f` with fd 1 redirected into a temporary file and returns
/// `(f's value, the bytes written to fd 1)`.
pub fn capture_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::io::Read;
    use std::os::unix::io::AsRawFd;

    let guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "diffcap-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));

    let result;
    let mut bytes = Vec::new();
    unsafe {
        // Flush anything already pending so it lands on the *real* stdout.
        libc_shim::fflush(std::ptr::null_mut());
        let saved = libc_shim::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        {
            let file = std::fs::File::create(&path).expect("create capture file");
            assert!(libc_shim::dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
        } // `file` closed; fd 1 keeps the description alive.

        result = f();

        libc_shim::fflush(std::ptr::null_mut());
        assert!(libc_shim::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc_shim::close(saved);
    }

    std::fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut bytes)
        .expect("read capture file");
    let _ = std::fs::remove_file(&path);

    drop(guard);
    (result, bytes)
}

/// Calls `doubleneg` in both libraries and asserts the return value *and* the
/// full stdout byte stream match.
pub fn assert_doubleneg_matches(p1: c_int, p2: c_int, p3: c_int, p4: c_int) {
    let (c, rust) = both();

    let (c_ret, c_out) = capture_stdout(|| unsafe { (c.doubleneg)(p1, p2, p3, p4) });
    let (r_ret, r_out) = capture_stdout(|| unsafe { (rust.doubleneg)(p1, p2, p3, p4) });

    // Guard against a vacuous pass: `doubleneg` always prints > 500 bytes, so an
    // empty capture means the redirection broke rather than that the two agree.
    assert!(
        c_out.len() > 500,
        "capture failed: C doubleneg({p1},{p2},{p3},{p4}) produced only {} bytes",
        c_out.len()
    );
    assert!(
        r_out.len() > 500,
        "capture failed: Rust doubleneg({p1},{p2},{p3},{p4}) produced only {} bytes",
        r_out.len()
    );

    if c_out != r_out {
        let c_s = String::from_utf8_lossy(&c_out);
        let r_s = String::from_utf8_lossy(&r_out);
        let first_diff = c_s
            .lines()
            .zip(r_s.lines())
            .enumerate()
            .find(|(_, (a, b))| a != b)
            .map(|(i, (a, b))| format!("line {i}:\n  C   : {a:?}\n  Rust: {b:?}"))
            .unwrap_or_else(|| {
                format!(
                    "line counts differ: C {} vs Rust {}",
                    c_s.lines().count(),
                    r_s.lines().count()
                )
            });
        panic!("doubleneg({p1},{p2},{p3},{p4}) stdout differs:\n{first_diff}");
    }

    assert_eq!(
        c_ret, r_ret,
        "doubleneg({p1},{p2},{p3},{p4}) return value: C {c_ret} vs Rust {r_ret}"
    );
}

// ---------------------------------------------------------------------------
// Canary-guarded scratch buffers, so an out-of-bounds write by either library
// is detected instead of silently corrupting the heap.
// ---------------------------------------------------------------------------

pub const CANARY: i8 = 0x5A;
pub const PAD: usize = 32;

/// A `size`-byte writable region with `PAD` canary bytes on each side.
pub struct Guarded {
    storage: Vec<i8>,
    len: usize,
}

impl Guarded {
    pub fn new(len: usize) -> Guarded {
        Guarded {
            storage: vec![CANARY; len + 2 * PAD],
            len,
        }
    }

    pub fn ptr(&mut self) -> *mut c_char {
        // SAFETY: `PAD` is within the allocation by construction.
        unsafe { self.storage.as_mut_ptr().add(PAD) }
    }

    pub fn const_ptr(&self) -> *const c_char {
        unsafe { self.storage.as_ptr().add(PAD) }
    }

    pub fn body(&self) -> &[i8] {
        &self.storage[PAD..PAD + self.len]
    }

    pub fn set_body(&mut self, bytes: &[i8]) {
        assert_eq!(bytes.len(), self.len);
        self.storage[PAD..PAD + self.len].copy_from_slice(bytes);
    }

    pub fn check_canaries(&self, ctx: impl std::fmt::Display) {
        for (i, &b) in self.storage[..PAD].iter().enumerate() {
            assert_eq!(b, CANARY, "{ctx}: leading canary {i} clobbered");
        }
        for (i, &b) in self.storage[PAD + self.len..].iter().enumerate() {
            assert_eq!(b, CANARY, "{ctx}: trailing canary {i} clobbered");
        }
    }
}
