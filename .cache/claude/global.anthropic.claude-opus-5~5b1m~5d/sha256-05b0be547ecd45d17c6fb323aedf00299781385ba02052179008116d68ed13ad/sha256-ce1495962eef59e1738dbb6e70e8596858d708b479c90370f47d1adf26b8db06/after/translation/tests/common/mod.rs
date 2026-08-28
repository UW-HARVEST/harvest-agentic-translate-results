//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as *shared objects* through `libloading` and
//! called only through their exported `bin2hex` symbol — the Rust crate is never
//! linked or called directly, so the `#[no_mangle] extern "C"` wrapper is part
//! of what gets tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Exact ABI of `char *bin2hex(char *, size_t, const uint8_t *, size_t)`.
pub type Bin2Hex =
    unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;

pub struct Impls {
    pub c: Bin2Hex,
    pub rust: Bin2Hex,
    // Keep the libraries alive for the whole process.
    _c_lib: Library,
    _rust_lib: Library,
}

// `Library` and plain `fn` pointers are both `Send + Sync`.
unsafe impl Send for Impls {}
unsafe impl Sync for Impls {}

static IMPLS: OnceLock<Impls> = OnceLock::new();

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn work_root() -> PathBuf {
    crate_root().parent().expect("crate has a parent dir").to_path_buf()
}

fn find_so(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut found: Vec<PathBuf> = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name()?.to_string_lossy().to_string();
        if name.starts_with(prefix) && name.ends_with(".so") {
            found.push(p);
        }
    }
    found.sort();
    found.into_iter().next()
}

/// `<work>/c_src/build/lib*.so` — the CMake target name is derived from the
/// parent directory name, so it is discovered rather than hard-coded.
fn c_so_path() -> PathBuf {
    let build = work_root().join("c_src").join("build");
    find_so(&build, "lib").unwrap_or_else(|| {
        panic!(
            "no C shared library found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// The Rust `cdylib` for the profile the tests were built with.
///
/// This is deliberately **strict**: it only accepts
/// `target/<same profile as this test binary>/libbin2hex_lib.so`. `cargo test`
/// does not build a `cdylib` on its own, and silently falling back to another
/// profile's `.so` would mean the "dev" run was really re-testing the release
/// artifact. `BIN2HEX_RUST_SO` overrides the path if needed.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("BIN2HEX_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "BIN2HEX_RUST_SO={} does not exist", p.display());
        return p;
    }

    let exe = std::env::current_exe().expect("current_exe");
    // target/<profile>/deps/<test>-<hash>  ->  target/<profile>
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("test binary lives in target/<profile>/deps/")
        .to_path_buf();
    let profile = profile_dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let direct = profile_dir.join("libbin2hex_lib.so");
    if direct.exists() {
        return direct;
    }
    if let Some(p) = find_so(&profile_dir, "libbin2hex_lib") {
        return p;
    }
    panic!(
        "the Rust cdylib for this profile is missing: {}\n\
         `cargo test` does not build a cdylib. Build it first with:\n    \
         cargo build{}\n\
         (or point BIN2HEX_RUST_SO at an explicit .so)",
        direct.display(),
        if profile == "release" { " --release" } else { "" }
    );
}

fn load(path: &Path) -> (Library, Bin2Hex) {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
    let f: Bin2Hex = unsafe {
        let sym: Symbol<Bin2Hex> = lib
            .get(b"bin2hex\0")
            .unwrap_or_else(|e| panic!("dlsym(bin2hex) in {} failed: {e}", path.display()));
        *sym
    };
    (lib, f)
}

/// Loads both `.so`s once per process (before any `fork()`, so children inherit
/// them) and returns the two `bin2hex` entry points.
pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| {
        let (c_lib, c) = load(&c_so_path());
        let (rust_lib, rust) = load(&rust_so_path());
        Impls { c, rust, _c_lib: c_lib, _rust_lib: rust_lib }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible property-style tests)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform-ish value in `0..n` (`n > 0`).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

// ---------------------------------------------------------------------------
// In-process differential driver (for inputs that must NOT crash)
// ---------------------------------------------------------------------------

/// Canary byte used to prove that neither implementation writes outside
/// `bin_len * 2 + 1` bytes.
pub const CANARY: u8 = 0xA5;

/// Runs both implementations on freshly canary-filled destination buffers of
/// `dst_len` bytes and asserts that the resulting buffers and returned pointers
/// agree exactly.
///
/// `hex_off` is the offset of the `hex` argument inside the destination buffer.
pub fn diff_disjoint(
    ctx: &str,
    bin: &[u8],
    bin_len: usize,
    hex_maxlen: usize,
    dst_len: usize,
    hex_off: usize,
) {
    let f = impls();

    let mut c_buf = vec![CANARY; dst_len];
    let mut r_buf = vec![CANARY; dst_len];

    let c_hex = unsafe { c_buf.as_mut_ptr().add(hex_off) } as *mut c_char;
    let r_hex = unsafe { r_buf.as_mut_ptr().add(hex_off) } as *mut c_char;

    let c_ret = unsafe { (f.c)(c_hex, hex_maxlen, bin.as_ptr(), bin_len) };
    let r_ret = unsafe { (f.rust)(r_hex, hex_maxlen, bin.as_ptr(), bin_len) };

    assert_eq!(c_ret, c_hex, "{ctx}: C did not return its `hex` argument");
    assert_eq!(r_ret, r_hex, "{ctx}: Rust did not return its `hex` argument");

    assert_buffers_eq(ctx, &c_buf, &r_buf);

    // The C contract writes exactly bin_len*2+1 bytes starting at hex_off.
    let end = hex_off + bin_len * 2 + 1;
    for i in 0..dst_len {
        if i < hex_off || i >= end {
            assert_eq!(
                c_buf[i], CANARY,
                "{ctx}: C clobbered canary byte at {i} (window {hex_off}..{end})"
            );
            assert_eq!(
                r_buf[i], CANARY,
                "{ctx}: Rust clobbered canary byte at {i} (window {hex_off}..{end})"
            );
        }
    }
    assert_eq!(c_buf[end - 1], 0, "{ctx}: C did not NUL-terminate at {}", end - 1);
}

pub fn assert_buffers_eq(ctx: &str, c_buf: &[u8], r_buf: &[u8]) {
    assert_eq!(c_buf.len(), r_buf.len(), "{ctx}: buffer length mismatch");
    if c_buf != r_buf {
        let at = c_buf.iter().zip(r_buf.iter()).position(|(a, b)| a != b).unwrap();
        let lo = at.saturating_sub(8);
        let hi = (at + 8).min(c_buf.len());
        panic!(
            "{ctx}: output differs at byte {at}: C=0x{:02x} Rust=0x{:02x}\n  C   [{lo}..{hi}] = {:02x?}\n  Rust[{lo}..{hi}] = {:02x?}",
            c_buf[at], r_buf[at], &c_buf[lo..hi], &r_buf[lo..hi]
        );
    }
}

/// Runs both implementations on a buffer where `hex` and `bin` overlap.
///
/// `arena_len` bytes are canary-filled, then `seed_data` is copied in at
/// `bin_off`; `hex` points at `hex_off`. Because the C code reads `bin[i]` and
/// then immediately writes `hex[2i]`/`hex[2i+1]`, an overlap makes the
/// read/write interleaving observable, which is exactly what this compares.
pub fn diff_overlapping(
    ctx: &str,
    arena_len: usize,
    seed_data: &[u8],
    bin_off: usize,
    bin_len: usize,
    hex_off: usize,
    hex_maxlen: usize,
) {
    let f = impls();

    let mut c_buf = vec![CANARY; arena_len];
    let mut r_buf = vec![CANARY; arena_len];
    c_buf[bin_off..bin_off + seed_data.len()].copy_from_slice(seed_data);
    r_buf[bin_off..bin_off + seed_data.len()].copy_from_slice(seed_data);

    let c_ret = unsafe {
        (f.c)(
            c_buf.as_mut_ptr().add(hex_off) as *mut c_char,
            hex_maxlen,
            c_buf.as_ptr().add(bin_off),
            bin_len,
        )
    };
    let r_ret = unsafe {
        (f.rust)(
            r_buf.as_mut_ptr().add(hex_off) as *mut c_char,
            hex_maxlen,
            r_buf.as_ptr().add(bin_off),
            bin_len,
        )
    };

    assert_eq!(c_ret as usize, unsafe { c_buf.as_ptr().add(hex_off) } as usize, "{ctx}: C return");
    assert_eq!(r_ret as usize, unsafe { r_buf.as_ptr().add(hex_off) } as usize, "{ctx}: Rust return");
    assert_buffers_eq(ctx, &c_buf, &r_buf);
}

// ---------------------------------------------------------------------------
// Raw libc bindings used for the crash-path (Phase C) tests
// ---------------------------------------------------------------------------

pub mod sys {
    use std::ffi::c_void;

    #[repr(C)]
    pub struct RLimit {
        pub rlim_cur: u64,
        pub rlim_max: u64,
    }

    pub const RLIMIT_CORE: i32 = 4;
    /// `PR_SET_DUMPABLE` — setting it to 0 stops the kernel from handing the
    /// dying child to the `core_pattern` helper (`systemd-coredump` here),
    /// which otherwise costs ~65 ms per fatal signal.
    pub const PR_SET_DUMPABLE: i32 = 4;
    pub const WNOHANG: i32 = 1;
    pub const SIGKILL: i32 = 9;
    pub const SIGABRT: i32 = 6;
    pub const SIGSEGV: i32 = 11;
    pub const SIGBUS: i32 = 7;

    pub const PROT_NONE: i32 = 0;
    pub const PROT_READ: i32 = 1;
    pub const PROT_WRITE: i32 = 2;
    pub const MAP_SHARED: i32 = 0x01;
    pub const MAP_PRIVATE: i32 = 0x02;
    pub const MAP_ANONYMOUS: i32 = 0x20;
    pub const MAP_FAILED: isize = -1;

    unsafe extern "C" {
        pub fn fork() -> i32;
        pub fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
        pub fn kill(pid: i32, sig: i32) -> i32;
        pub fn _exit(code: i32) -> !;
        pub fn setrlimit(resource: i32, rlim: *const RLimit) -> i32;
        pub fn prctl(option: i32, a2: u64, a3: u64, a4: u64, a5: u64) -> i32;
        pub fn mmap(
            addr: *mut c_void,
            len: usize,
            prot: i32,
            flags: i32,
            fd: i32,
            off: i64,
        ) -> *mut c_void;
        pub fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
        pub fn munmap(addr: *mut c_void, len: usize) -> i32;
        pub fn sysconf(name: i32) -> i64;
    }

    pub fn page_size() -> usize {
        // _SC_PAGESIZE == 30 on Linux
        let v = unsafe { sysconf(30) };
        if v > 0 { v as usize } else { 4096 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
    TimedOut,
}

impl Outcome {
    pub fn describe(self) -> String {
        match self {
            Outcome::Exited(c) => format!("exited({c})"),
            Outcome::Signaled(s) => {
                let name = match s {
                    sys::SIGABRT => " SIGABRT",
                    sys::SIGSEGV => " SIGSEGV",
                    sys::SIGBUS => " SIGBUS",
                    sys::SIGKILL => " SIGKILL",
                    _ => "",
                };
                format!("signaled({s}{name})")
            }
            Outcome::TimedOut => "timed out".to_string(),
        }
    }
}

/// Forks, runs `body` in the child, and reports how the child terminated.
///
/// The child disables core dumps and calls `_exit` (never `exit`) so no atexit
/// handler of the test harness runs twice. `body` must only do async-signal-safe
/// work — here it is a single call into the loaded `bin2hex`.
pub fn run_in_child<F: FnOnce() -> i32>(body: F) -> Outcome {
    // Make sure both libraries are resolved before forking.
    let _ = impls();
    // Flush so the child does not duplicate buffered output.
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    let pid = unsafe { sys::fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        let rl = sys::RLimit { rlim_cur: 0, rlim_max: 0 };
        unsafe { sys::setrlimit(sys::RLIMIT_CORE, &rl) };
        // Do not involve the core-dump helper; this changes nothing about the
        // wait status (still SIGABRT / SIGSEGV) but makes each case ~400x faster.
        unsafe { sys::prctl(sys::PR_SET_DUMPABLE, 0, 0, 0, 0) };
        let code = body();
        unsafe { sys::_exit(code) };
    }
    wait_for(pid, std::time::Duration::from_secs(20))
}

fn decode(status: i32) -> Outcome {
    let low = status & 0x7f;
    if low == 0 {
        Outcome::Exited((status >> 8) & 0xff)
    } else if low == 0x7f {
        // stopped; treat as timeout-ish, should not happen here
        Outcome::TimedOut
    } else {
        Outcome::Signaled(low)
    }
}

/// Blocking `waitpid` plus a watchdog thread, so a wedged child cannot hang the
/// suite while the common (immediate `abort()`/`SIGSEGV`) case costs no polling
/// latency at all.
fn wait_for(pid: i32, limit: std::time::Duration) -> Outcome {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let killed = Arc::new(AtomicBool::new(false));
    // Condvar (not sleep-polling) so the fast path — an immediate abort() —
    // costs nothing beyond the fork itself.
    let gate: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)> =
        Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let watchdog = {
        let gate = Arc::clone(&gate);
        let killed = Arc::clone(&killed);
        std::thread::spawn(move || {
            let (lock, cv) = &*gate;
            let mut done = lock.lock().unwrap();
            while !*done {
                let (g, timeout) = cv.wait_timeout(done, limit).unwrap();
                done = g;
                if timeout.timed_out() {
                    break;
                }
            }
            if !*done {
                killed.store(true, Ordering::SeqCst);
                unsafe { sys::kill(pid, sys::SIGKILL) };
            }
        })
    };

    let mut status: i32 = 0;
    let r = unsafe { sys::waitpid(pid, &mut status, 0) };
    {
        let (lock, cv) = &*gate;
        *lock.lock().unwrap() = true;
        cv.notify_all();
    }
    let _ = watchdog.join();

    if r != pid {
        return Outcome::TimedOut;
    }
    let out = decode(status);
    if killed.load(Ordering::SeqCst) && out == Outcome::Signaled(sys::SIGKILL) {
        return Outcome::TimedOut;
    }
    out
}

/// Calls one implementation with raw arguments inside a forked child and
/// reports the child's termination status. Exit code `0` means the call
/// returned and gave back its `hex` argument; `1` means it returned a different
/// pointer.
pub fn call_in_child(
    f: Bin2Hex,
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> Outcome {
    run_in_child(move || {
        let ret = unsafe { f(hex, hex_maxlen, bin, bin_len) };
        if ret == hex { 0 } else { 1 }
    })
}

/// Asserts that both implementations terminate identically for the given raw
/// arguments. Used for every `ERRORS.md` row.
pub fn diff_outcome(
    ctx: &str,
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> Outcome {
    let f = impls();
    let c = call_in_child(f.c, hex, hex_maxlen, bin, bin_len);
    let r = call_in_child(f.rust, hex, hex_maxlen, bin, bin_len);
    assert_eq!(
        c,
        r,
        "{ctx}: C {} but Rust {}",
        c.describe(),
        r.describe()
    );
    assert_ne!(c, Outcome::TimedOut, "{ctx}: both implementations timed out");
    c
}

// ---------------------------------------------------------------------------
// Guard-page-terminated buffers
// ---------------------------------------------------------------------------

/// `usable` writable bytes immediately followed by one `PROT_NONE` page, so a
/// single byte written past the end raises `SIGSEGV` deterministically.
pub struct Guarded {
    base: *mut u8,
    total: usize,
    pub usable: usize,
    pub shared: bool,
}

impl Guarded {
    /// `usable_pages` writable pages plus a trailing guard page. When `shared`
    /// is true the writable pages are `MAP_SHARED`, so a forked child's writes
    /// are visible to the parent (used to compare the bytes produced before a
    /// fault).
    pub fn new(usable_pages: usize, shared: bool) -> Guarded {
        let ps = sys::page_size();
        let usable = usable_pages * ps;
        let total = usable + ps;
        let flags = sys::MAP_ANONYMOUS | if shared { sys::MAP_SHARED } else { sys::MAP_PRIVATE };
        let base = unsafe {
            sys::mmap(
                std::ptr::null_mut(),
                total,
                sys::PROT_READ | sys::PROT_WRITE,
                flags,
                -1,
                0,
            )
        };
        assert_ne!(base as isize, sys::MAP_FAILED, "mmap failed");
        let guard = unsafe { (base as *mut u8).add(usable) };
        let rc = unsafe { sys::mprotect(guard as *mut std::ffi::c_void, ps, sys::PROT_NONE) };
        assert_eq!(rc, 0, "mprotect(PROT_NONE) failed");
        Guarded { base: base as *mut u8, total, usable, shared }
    }

    pub fn ptr(&self) -> *mut u8 {
        self.base
    }
    pub fn at(&self, off: usize) -> *mut u8 {
        assert!(off <= self.usable);
        unsafe { self.base.add(off) }
    }
    pub fn fill(&self, byte: u8) {
        unsafe { std::ptr::write_bytes(self.base, byte, self.usable) };
    }
    pub fn copy_in(&self, off: usize, data: &[u8]) {
        assert!(off + data.len() <= self.usable);
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), self.base.add(off), data.len()) };
    }
    pub fn snapshot(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.base, self.usable) }.to_vec()
    }
}

impl Drop for Guarded {
    fn drop(&mut self) {
        unsafe { sys::munmap(self.base as *mut std::ffi::c_void, self.total) };
    }
}
