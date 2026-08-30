//! Shared differential-testing harness.
//!
//! Both libraries are loaded as shared objects with `libloading` and called
//! only through their exported C symbols — the Rust implementation is never
//! called directly, so the `#[no_mangle] extern "C"` wrappers are under test
//! too.
//!
//! Because the library's only observable effect is `printf` to `stdout`, the
//! harness temporarily redirects file descriptor 1 to a temporary file around
//! a batch of calls and compares the resulting bytes.

#![allow(dead_code)]

use std::ffi::{c_int, c_uint, c_void};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open stdio stream, which is what we need
    /// because the C `.so` and the Rust `.so` share this process's libc
    /// `stdout` FILE object.
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// exported signatures
// ---------------------------------------------------------------------------

/// `void driver(unsigned int x, unsigned int y, bool b, int z);`
pub type DriverFn = unsafe extern "C" fn(c_uint, c_uint, u8, c_int);

/// Same symbol, but with the `_Bool` argument declared as a full 32-bit `int`.
/// Used to probe what the callee does with the ABI-irrelevant upper 24 bits of
/// the third argument register.
pub type DriverWideBoolFn = unsafe extern "C" fn(c_uint, c_uint, c_uint, c_int);

/// `void print_foo(const foo_t *foo);` — the pointer is passed as a raw byte
/// pointer so the test can hand over arbitrary (even misaligned) memory.
pub type PrintFooFn = unsafe extern "C" fn(*const u8);

/// The two implementations under test.
pub struct Impls {
    pub c: Library,
    pub rust: Library,
}

impl Impls {
    pub fn driver(&self, which: Which) -> Symbol<'_, DriverFn> {
        unsafe { self.lib(which).get(b"driver\0").expect("driver symbol") }
    }
    pub fn driver_wide(&self, which: Which) -> Symbol<'_, DriverWideBoolFn> {
        unsafe { self.lib(which).get(b"driver\0").expect("driver symbol") }
    }
    pub fn print_foo(&self, which: Which) -> Symbol<'_, PrintFooFn> {
        unsafe { self.lib(which).get(b"print_foo\0").expect("print_foo symbol") }
    }
    pub fn lib(&self, which: Which) -> &Library {
        match which {
            Which::C => &self.c,
            Which::Rust => &self.rust,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Which {
    C,
    Rust,
}

// ---------------------------------------------------------------------------
// library discovery / loading
// ---------------------------------------------------------------------------

pub fn workspace_root() -> PathBuf {
    // .../<root>/translation  ->  .../<root>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

pub fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

/// The Rust `cdylib` produced for the profile the current test binary was
/// built with (`target/<profile>/libdriver.so`).
pub fn rust_lib_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // target/<profile>/deps/<test>-<hash>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf();
    let direct = profile_dir.join("libdriver.so");
    if direct.exists() {
        return direct;
    }
    for p in ["target/debug/libdriver.so", "target/release/libdriver.so"] {
        let cand = Path::new(env!("CARGO_MANIFEST_DIR")).join(p);
        if cand.exists() {
            return cand;
        }
    }
    panic!("could not locate the Rust cdylib (looked for {direct:?})");
}

fn ensure_c_lib() {
    let so = c_lib_path();
    if so.exists() {
        return;
    }
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let ok = std::process::Command::new("cmake")
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .current_dir(&build)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && std::process::Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    assert!(ok && so.exists(), "failed to build the C shared library at {so:?}");
}

static IMPLS: OnceLock<Impls> = OnceLock::new();

pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| {
        ensure_c_lib();
        let c = unsafe { Library::new(c_lib_path()) }.expect("load C .so");
        let rust = unsafe { Library::new(rust_lib_path()) }.expect("load Rust .so");
        Impls { c, rust }
    })
}

// ---------------------------------------------------------------------------
// stdout capture (process-global, therefore serialized)
// ---------------------------------------------------------------------------

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// minimal sequential test runner (`harness = false`)
// ---------------------------------------------------------------------------

/// Runs every test sequentially in the current thread and exits with a non-zero
/// status if any of them failed. Sequential execution is required: the tests
/// redirect fd 1 process-wide.
pub fn run_tests(tests: &[(&str, fn())]) -> ! {
    use std::io::Write;
    let filter: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();
    let mut failed: Vec<&str> = Vec::new();
    let mut ran = 0usize;
    for (name, f) in tests {
        if !filter.is_empty() && !filter.iter().any(|p| name.contains(p.as_str())) {
            continue;
        }
        ran += 1;
        print!("test {name} ... ");
        let _ = std::io::stdout().flush();
        match std::panic::catch_unwind(*f) {
            Ok(()) => println!("ok"),
            Err(_) => {
                println!("FAILED");
                failed.push(name);
            }
        }
        let _ = std::io::stdout().flush();
    }
    println!(
        "\nresult: {}. {} passed; {} failed (of {ran} run)",
        if failed.is_empty() { "ok" } else { "FAILED" },
        ran - failed.len(),
        failed.len()
    );
    if !failed.is_empty() {
        println!("failures:");
        for f in &failed {
            println!("    {f}");
        }
    }
    let _ = std::io::stdout().flush();
    std::process::exit(if failed.is_empty() { 0 } else { 1 });
}

pub fn stdout_guard() -> MutexGuard<'static, ()> {
    STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` with file descriptor 1 redirected to a temporary file and returns
/// everything that was written to it.
///
/// The caller must already hold [`stdout_guard`].
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let path = dir.join(format!(
        "cdiff-{}-{:?}.out",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");

    // Drain anything still sitting in Rust's own line-buffered stdout so it
    // cannot leak into the capture file.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    let saved = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
        saved
    };

    f();

    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    drop(file);

    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    out
}

/// Captures the output of the same batch of calls from both implementations
/// and asserts byte equality, reporting the first differing line.
///
/// `run` receives the implementation selector and must perform the calls.
pub fn assert_same<F>(label: &str, cases: &[String], mut run: F)
where
    F: FnMut(Which),
{
    let _g = stdout_guard();
    let c_out = capture(|| run(Which::C));
    let r_out = capture(|| run(Which::Rust));
    drop(_g);

    if c_out == r_out {
        return;
    }

    let c_lines: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
    let r_lines: Vec<&[u8]> = r_out.split(|&b| b == b'\n').collect();
    for i in 0..c_lines.len().max(r_lines.len()) {
        let cl = c_lines.get(i).copied().unwrap_or(b"<missing>");
        let rl = r_lines.get(i).copied().unwrap_or(b"<missing>");
        if cl != rl {
            let case = cases.get(i).map(String::as_str).unwrap_or("<unknown case>");
            panic!(
                "[{label}] divergence on line {i} (case: {case})\n  C   : {:?}\n  Rust: {:?}\n\
                 total bytes: C={} Rust={}",
                String::from_utf8_lossy(cl),
                String::from_utf8_lossy(rl),
                c_out.len(),
                r_out.len()
            );
        }
    }
    panic!("[{label}] outputs differ but no differing line found (C={} B, Rust={} B)", c_out.len(), r_out.len());
}

// ---------------------------------------------------------------------------
// deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// A 32-bit value biased towards "interesting" bit patterns.
    pub fn interesting_u32(&mut self) -> u32 {
        const SPECIAL: [u32; 12] = [
            0,
            1,
            2,
            3,
            4,
            7,
            8,
            0xFF,
            0x1_0000,
            0x8000_0000,
            0x7FFF_FFFF,
            0xFFFF_FFFF,
        ];
        match self.below(3) {
            0 => SPECIAL[self.below(SPECIAL.len() as u32) as usize],
            1 => self.next_u32() & ((1u32 << (1 + self.below(31))) - 1),
            _ => self.next_u32(),
        }
    }
}

/// The `foo_t` byte image gcc produces for `{.x = x, .y = y, .b = b, .z = z}`
/// (SysV x86-64: bit-field storage byte at offset 0, `int z` at offset 4).
pub fn foo_bytes(x: u32, y: u32, b: u8, z: i32, padding: u8) -> [u8; 8] {
    let bits = ((x as u8) & 0x3) | (((y as u8) & 0x7) << 2) | ((b & 0x1) << 5);
    let zz = z.to_le_bytes();
    [
        bits | (padding & 0xC0),
        padding,
        padding,
        padding,
        zz[0],
        zz[1],
        zz[2],
        zz[3],
    ]
}
