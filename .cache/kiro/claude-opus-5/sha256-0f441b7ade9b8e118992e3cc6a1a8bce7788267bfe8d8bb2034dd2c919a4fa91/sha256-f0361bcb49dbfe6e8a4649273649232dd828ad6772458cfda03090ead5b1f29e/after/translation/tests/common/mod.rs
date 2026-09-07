//! Shared differential-test harness.
//!
//! Both libraries are loaded as *shared objects* through `libloading` and driven
//! only through their exported symbols (`long_exec`, `perform_expensive_operations`,
//! `array`).  Nothing in this crate is ever called directly, so the
//! `#[no_mangle] extern "C"` wrappers and the exported `array` object are part of
//! what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_uint};
use std::path::PathBuf;

/// `#define ARRAY_SIZE (256 * 1024)`
pub const ARRAY_SIZE: usize = 256 * 1024;
/// `#define ITERATIONS 2000`
pub const ITERATIONS: usize = 2000;
/// Kernel applications per `perform_expensive_operations()` call.
pub const INNER: usize = 100;

/// One loaded implementation, addressed purely through `dlsym`.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    // `lib` must outlive the raw pointers/fn pointers taken out of it.
    lib: Library,
    long_exec: usize,
    peo: usize,
    array: *mut c_int,
}

impl Impl {
    pub fn open(name: &'static str, path: PathBuf) -> Impl {
        assert!(
            path.exists(),
            "{name}: shared object {} does not exist -- build it first",
            path.display()
        );
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("{name}: dlopen({}) failed: {e}", path.display()));
            let long_exec: Symbol<unsafe extern "C" fn(c_uint)> = lib
                .get(b"long_exec\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym(long_exec) failed: {e}"));
            let peo: Symbol<unsafe extern "C" fn()> = lib
                .get(b"perform_expensive_operations\0")
                .unwrap_or_else(|e| {
                    panic!("{name}: dlsym(perform_expensive_operations) failed: {e}")
                });
            let array: Symbol<*mut c_int> = lib
                .get(b"array\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym(array) failed: {e}"));

            let long_exec_addr = long_exec.into_raw().into_raw() as usize;
            let peo_addr = peo.into_raw().into_raw() as usize;
            let array_ptr = array.into_raw().into_raw() as *mut c_int;

            Impl {
                name,
                path,
                lib,
                long_exec: long_exec_addr,
                peo: peo_addr,
                array: array_ptr,
            }
        }
    }

    /// `void long_exec(unsigned int seed)`
    pub fn long_exec(&self, seed: c_uint) {
        let f: unsafe extern "C" fn(c_uint) = unsafe { std::mem::transmute(self.long_exec) };
        unsafe { f(seed) }
    }

    /// `void perform_expensive_operations(void)`
    pub fn peo(&self) {
        let f: unsafe extern "C" fn() = unsafe { std::mem::transmute(self.peo) };
        unsafe { f() }
    }

    /// Call `perform_expensive_operations()` `k` times.
    pub fn peo_times(&self, k: usize) {
        for _ in 0..k {
            self.peo();
        }
    }

    /// The exported `int array[262144]` object, as a slice.
    pub fn array(&self) -> &[c_int] {
        unsafe { std::slice::from_raw_parts(self.array, ARRAY_SIZE) }
    }

    pub fn array_mut(&self) -> &mut [c_int] {
        unsafe { std::slice::from_raw_parts_mut(self.array, ARRAY_SIZE) }
    }

    pub fn set_array(&self, vals: &[c_int]) {
        assert_eq!(vals.len(), ARRAY_SIZE);
        self.array_mut().copy_from_slice(vals);
    }

    pub fn xor(&self) -> c_int {
        self.array().iter().fold(0, |a, &b| a ^ b)
    }

    /// Keep the handle alive explicitly (silences unused-field lints).
    pub fn lib(&self) -> &Library {
        &self.lib
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("LONG_C_SO") {
        return PathBuf::from(p);
    }
    workspace_root().join("c_src/build/liblong.so")
}

/// Path to the Rust `cdylib` under test.
///
/// Derived from the running test executable (`target/<profile>/deps/<test>`) so
/// that the `.so` always matches the profile *and the feature set* the test
/// binary was built with -- otherwise `cargo test --features …` would silently
/// keep exercising a stale default-feature `.so`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("LONG_RUST_SO") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        // .../target/<profile>/deps/<test>-<hash>  ->  .../target/<profile>/liblong.so
        if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
            let cand = profile_dir.join("liblong.so");
            if cand.exists() {
                return cand;
            }
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rel = manifest.join("target/release/liblong.so");
    if rel.exists() {
        return rel;
    }
    manifest.join("target/debug/liblong.so")
}

/// Serialises the things that are genuinely process-global: the libc PRNG state
/// (`srand`/`rand`, shared by both libraries) and fd 1 during stdout capture.
pub static GLOBAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct Pair {
    pub c: Impl,
    pub r: Impl,
    tmp: Vec<PathBuf>,
}

impl Drop for Pair {
    fn drop(&mut self) {
        for p in &self.tmp {
            let _ = std::fs::remove_file(p);
        }
    }
}

impl Pair {
    pub fn split(&self) -> (&Impl, &Impl) {
        (&self.c, &self.r)
    }
}

fn unique_copy(src: &std::path::Path, tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dst = std::env::temp_dir().join(format!(
        "long_diff_{}_{}_{}_{}.so",
        std::process::id(),
        tag,
        n,
        src.file_name().unwrap().to_string_lossy()
    ));
    std::fs::copy(src, &dst)
        .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
    dst
}

/// Load the C implementation and the Rust implementation, both as `.so`s.
///
/// Each library is opened from a *private copy* on disk so that `dlopen`'s
/// reference counting cannot hand two tests the same `array` object: every
/// `load_pair()` therefore starts from a genuinely pristine, zero-filled `.bss`,
/// exactly as a fresh process would see it.
pub fn load_pair() -> Pair {
    let cs = c_so_path();
    let rs = rust_so_path();
    assert!(cs.exists(), "missing C .so at {}", cs.display());
    assert!(rs.exists(), "missing Rust .so at {}", rs.display());
    let ctmp = unique_copy(&cs, "c");
    let rtmp = unique_copy(&rs, "rust");
    Pair {
        c: Impl::open("C", ctmp.clone()),
        r: Impl::open("Rust", rtmp.clone()),
        tmp: vec![ctmp, rtmp],
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG for property-style rows (fixed seed => reproducible).
// ---------------------------------------------------------------------------

/// SplitMix64.  Independent of libc `rand`, so filling test arrays never
/// perturbs the PRNG state that `long_exec` depends on.
pub struct Rng(pub u64);

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
    pub fn next_i32(&mut self) -> c_int {
        (self.next_u64() >> 32) as u32 as c_int
    }
    /// Value in the same domain glibc `rand()` produces: `0 ..= RAND_MAX`.
    pub fn next_rand_like(&mut self) -> c_int {
        (self.next_u64() >> 33) as u32 as c_int & 0x7FFF_FFFF
    }
    pub fn fill_full_range(&mut self, buf: &mut [c_int]) {
        for slot in buf.iter_mut() {
            *slot = self.next_i32();
        }
    }
    pub fn fill_rand_like(&mut self, buf: &mut [c_int]) {
        for slot in buf.iter_mut() {
            *slot = self.next_rand_like();
        }
    }
}

// ---------------------------------------------------------------------------
// stdout capture (both libraries `printf` into the *same* libc stdout).
// ---------------------------------------------------------------------------

extern "C" {
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

/// Run `f`, returning everything it wrote to fd 1 via libc stdio.
///
/// `libtest`'s own progress lines (`test <name> ... ok`) also go to fd 1, and
/// with `--test-threads > 1` another thread can emit one while fd 1 is
/// redirected here.  Those lines are stripped; the payload this library emits is
/// only ever `printf("%d\n", …)`.  Run stdout-capturing tests with
/// `--test-threads=1` to avoid the interleaving entirely.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let tmp = std::env::temp_dir().join(format!(
        "long_cap_{}_{}_{}.txt",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let cpath = std::ffi::CString::new(tmp.to_str().unwrap()).unwrap();
    unsafe {
        use std::io::Write;
        // Flush both stdio layers so nothing pending is misattributed.
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o644 as c_int);
        assert!(fd >= 0, "open({}) failed", tmp.display());
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        f();

        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        close(fd);
    }
    let bytes = std::fs::read(&tmp).unwrap_or_default();
    let _ = std::fs::remove_file(&tmp);
    strip_harness_noise(&bytes)
}

fn strip_harness_noise(raw: &[u8]) -> Vec<u8> {
    let s = String::from_utf8_lossy(raw).to_string();
    if !s.contains("test ") && !s.contains("running ") {
        return raw.to_vec();
    }
    let mut out = String::new();
    for line in s.split_inclusive('\n') {
        let t = line.trim_end_matches('\n');
        let noise = t.starts_with("test ")
            || t.starts_with("running ")
            || t.starts_with("test result:")
            || t.starts_with("failures")
            || t.starts_with("---- ")
            || t.starts_with("note: ")
            || t.starts_with("     Running")
            || t.starts_with("thread '");
        if !noise {
            out.push_str(line);
        }
    }
    out.into_bytes()
}

/// Byte-for-byte comparison of two 1 MiB arrays with a useful failure message.
pub fn assert_arrays_eq(ctx: &str, c: &[c_int], r: &[c_int]) {
    assert_eq!(c.len(), r.len(), "{ctx}: length mismatch");
    if c == r {
        return;
    }
    let mut diffs = 0usize;
    let mut first = None;
    for i in 0..c.len() {
        if c[i] != r[i] {
            diffs += 1;
            if first.is_none() {
                first = Some(i);
            }
        }
    }
    let i = first.unwrap();
    panic!(
        "{ctx}: {diffs}/{} elements differ; first at index {i}: C={} Rust={}",
        c.len(),
        c[i],
        r[i]
    );
}

/// Reference stdout captured from the **C `.so`** for a given seed.
///
/// One full `long_exec` run of the C library takes ~8 minutes (2000 x 262144 x 100
/// kernel applications), so the seed sweep in `CONFIGS.md` rows 20-33 is driven
/// from recorded C output rather than 60 live 8-minute runs.  Every file was
/// produced by `dlopen`ing **the very C `.so` under test** and calling its
/// exported `long_exec`; see `tools/gen_c_refs.sh`.  The live in-process
/// C-`.so`-vs-Rust-`.so` comparison is `long_exec_diff.rs`'s `#[ignore]`d tests.
pub fn c_reference_dir() -> PathBuf {
    if let Ok(p) = std::env::var("LONG_REF_DIR") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/cref")
}

/// seed -> the full 262144-element `array` object the C `.so` left behind after
/// `long_exec(seed)`, read from `tests/cref/arr_<seed>.bin` (raw little-endian
/// `int32`, 1048576 bytes).  Recorded by `tools/gen_c_refs.sh --with-arrays`.
pub fn load_c_array_dumps() -> Vec<(c_uint, Vec<c_int>)> {
    let mut out = Vec::new();
    let dir = c_reference_dir();
    let rd = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return out,
    };
    let mut entries: Vec<(c_uint, PathBuf)> = Vec::new();
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if let Some(s) = name
            .strip_prefix("arr_")
            .and_then(|s| s.strip_suffix(".bin"))
        {
            if let Ok(seed) = s.parse::<u32>() {
                entries.push((seed, e.path()));
            }
        }
    }
    entries.sort();
    for (seed, path) in entries {
        let bytes = std::fs::read(&path).unwrap_or_default();
        assert_eq!(
            bytes.len(),
            ARRAY_SIZE * 4,
            "{}: expected {} bytes, got {}",
            path.display(),
            ARRAY_SIZE * 4,
            bytes.len()
        );
        let vals: Vec<c_int> = bytes
            .chunks_exact(4)
            .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        out.push((seed, vals));
    }
    out
}
/// seed -> the exact decimal the C `.so` printed (no trailing newline).
pub fn load_c_references() -> std::collections::BTreeMap<u32, String> {
    let mut out = std::collections::BTreeMap::new();
    let dir = c_reference_dir();
    let rd = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return out,
    };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let seed = match name.strip_prefix("c_").and_then(|s| s.strip_suffix(".txt")) {
            Some(s) => match s.parse::<u32>() {
                Ok(v) => v,
                Err(_) => continue,
            },
            None => continue,
        };
        let body = std::fs::read_to_string(e.path()).unwrap_or_default();
        let body = body.trim_end().to_string();
        if !body.is_empty() {
            out.insert(seed, body);
        }
    }
    out
}
