//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported C symbols — the Rust crate is never linked
//! or called directly, so the `#[no_mangle]` / `extern "C"` wrappers are under
//! test as well.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

pub type HashBytesFn = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type SiphashFn = unsafe extern "C" fn(c_int);

/// One loaded implementation ("C" or "RUST").
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub hash_bytes: HashBytesFn,
    pub siphash: SiphashFn,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libtranslated_rust.so`
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

/// The Rust `cdylib`. Prefers the profile the test binary itself was built with
/// (`target/<profile>/libsiphash_lib.so`), so `cargo test --release` exercises
/// the release object. Overridable with `RUST_SO=...`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // current_exe = target/<profile>/deps/<testname>-<hash>
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
            let cand = profile_dir.join("libsiphash_lib.so");
            if cand.exists() {
                return cand;
            }
        }
    }
    for profile in ["debug", "release"] {
        let cand = manifest_dir()
            .join("target")
            .join(profile)
            .join("libsiphash_lib.so");
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "could not locate libsiphash_lib.so; run `cargo build` first or set RUST_SO=<path>"
    );
}

fn load_one(name: &'static str, path: PathBuf) -> Impl {
    assert!(
        path.exists(),
        "{name} shared object not found at {}\n\
         (build the C lib with: cd c_src && mkdir -p build && cd build && \
          cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)",
        path.display()
    );
    // Leak the Library so the extracted function pointers are valid for the whole
    // process lifetime (and so the object is never unloaded mid-test).
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(&path).unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
    }));

    let hash_bytes: Symbol<HashBytesFn> = unsafe {
        lib.get(b"stbds_hash_bytes\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol stbds_hash_bytes: {e}"))
    };
    let siphash: Symbol<SiphashFn> = unsafe {
        lib.get(b"siphash\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol siphash: {e}"))
    };

    Impl {
        name,
        path,
        hash_bytes: *hash_bytes,
        siphash: *siphash,
    }
}

/// `[C, RUST]`, loaded once per test binary.
pub fn impls() -> &'static (Impl, Impl) {
    static IMPLS: OnceLock<(Impl, Impl)> = OnceLock::new();
    IMPLS.get_or_init(|| {
        (
            load_one("C", c_so_path()),
            load_one("RUST", rust_so_path()),
        )
    })
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

/// Calls `stbds_hash_bytes` in both objects and asserts bit-identical results.
/// Also asserts neither implementation mutates the input buffer.
#[track_caller]
pub fn diff_hash(buf: &mut [u8], len: usize, seed: usize, ctx: &str) -> usize {
    let (c, r) = impls();
    let before = buf.to_vec();

    let cv = unsafe { (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
    assert_eq!(
        &before[..],
        &buf[..],
        "C stbds_hash_bytes mutated the input buffer [{ctx}]"
    );

    let rv = unsafe { (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
    assert_eq!(
        &before[..],
        &buf[..],
        "RUST stbds_hash_bytes mutated the input buffer [{ctx}]"
    );

    assert_eq!(
        cv, rv,
        "stbds_hash_bytes MISMATCH [{ctx}]\n  len={len} seed={seed:#018x}\n  \
         C   = {cv:#018x}\n  RUST= {rv:#018x}\n  buf[..{}]={:02x?}",
        len.min(before.len()),
        &before[..len.min(before.len())]
    );
    cv
}

/// Same as [`diff_hash`] but with a raw pointer (for unaligned / null cases).
#[track_caller]
pub fn diff_hash_raw(p: *mut c_void, len: usize, seed: usize, ctx: &str) -> usize {
    let (c, r) = impls();
    let cv = unsafe { (c.hash_bytes)(p, len, seed) };
    let rv = unsafe { (r.hash_bytes)(p, len, seed) };
    assert_eq!(
        cv, rv,
        "stbds_hash_bytes MISMATCH [{ctx}]\n  p={p:p} len={len} seed={seed:#018x}\n  \
         C   = {cv:#018x}\n  RUST= {rv:#018x}"
    );
    cv
}

// ---------------------------------------------------------------------------
// stdout capture (for `siphash`, which prints instead of returning)
// ---------------------------------------------------------------------------

/// fd 1 is process-global, so captures must be serialized.
fn stdout_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Redirects fd 1 to a temp file, runs `f`, flushes every C stream, restores
/// fd 1 and returns the captured bytes.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom, Write};

    let _guard = stdout_lock().lock().unwrap_or_else(|e| e.into_inner());

    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "siphash_capture_{}_{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp)
        .expect("create capture temp file");

    unsafe {
        // Flush anything already pending so it does not land in our capture.
        let _ = std::io::stdout().flush();
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = {
            use std::os::unix::io::AsRawFd;
            file.as_raw_fd()
        };
        assert!(libc::dup2(fd, 1) >= 0, "dup2 -> 1 failed");

        f();

        // The library uses C `printf`; the stream is fully buffered when fd 1 is
        // a file, so it must be flushed before fd 1 is restored.
        libc::fflush(std::ptr::null_mut());
        let _ = std::io::stdout().flush();

        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);
    }

    let mut out = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("seek capture file");
    file.read_to_end(&mut out).expect("read capture file");
    drop(file);
    let _ = std::fs::remove_file(&tmp);
    out
}

/// Runs `siphash(init)` in both objects, capturing stdout, and asserts the
/// emitted bytes are identical.
#[track_caller]
pub fn diff_siphash(init: c_int) -> Vec<u8> {
    let (c, r) = impls();
    let cout = capture_stdout(|| unsafe { (c.siphash)(init) });
    let rout = capture_stdout(|| unsafe { (r.siphash)(init) });

    assert!(
        !cout.is_empty(),
        "C siphash({init}) produced no output — stdout capture is broken"
    );
    if cout != rout {
        let cs = String::from_utf8_lossy(&cout);
        let rs = String::from_utf8_lossy(&rout);
        let first_diff = cout
            .iter()
            .zip(rout.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(cout.len().min(rout.len()));
        panic!(
            "siphash({init}) stdout MISMATCH at byte {first_diff}\n\
             --- C ({} bytes) ---\n{cs}\n--- RUST ({} bytes) ---\n{rs}",
            cout.len(),
            rout.len()
        );
    }
    cout
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seeds, reproducible runs
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    pub fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
    }
    /// Uniform-ish in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

/// The interesting seeds: extremes plus a deterministic random spread.
pub fn seed_corpus(rng: &mut Rng, extra_random: usize) -> Vec<usize> {
    let mut v = vec![
        0usize,
        1,
        2,
        usize::MAX,
        usize::MAX - 1,
        1usize << 63,
        1usize << 62,
        0x5555_5555_5555_5555,
        0xAAAA_AAAA_AAAA_AAAA,
        0xFFFF_FFFF,
        0xFFFF_FFFF_0000_0000,
    ];
    for _ in 0..extra_random {
        v.push(rng.next_usize());
    }
    v
}
