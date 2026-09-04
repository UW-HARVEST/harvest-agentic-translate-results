//! Shared differential-test harness.
//!
//! Both the C shared object and the Rust shared object are loaded with
//! `libloading` and driven *only* through their exported symbols
//! (`long_exec`, `perform_expensive_operations`, `array`).  No Rust function is
//! ever called directly, so the `#[no_mangle]` export wrappers and the ABI of
//! the exported `array` object are part of what is under test.

#![allow(dead_code)]

use std::ffi::{c_int, c_uint, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// `#define ARRAY_SIZE (256 * 1024)`
pub const ARRAY_LEN: usize = 256 * 1024;
/// Byte size of the exported `array` object.
pub const ARRAY_BYTES: usize = ARRAY_LEN * 4;
/// `#define ITERATIONS 2000` times the 100-step inner loop.
pub const FULL_N: u32 = 2000 * 100;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open stdio stream, including the `stdout`
    /// that both shared objects share through glibc.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// One loaded shared object, reduced to its three exported symbols.
pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    long_exec: unsafe extern "C" fn(c_uint),
    peo: unsafe extern "C" fn(),
    /// Address of the exported `int array[ARRAY_SIZE]` object.
    array: *mut c_int,
    /// Variadic-ish views of the same two functions, used to prove that
    /// mismatched prototypes behave the same on both sides.
    long_exec_u64: unsafe extern "C" fn(u64),
    peo_with_arg: unsafe extern "C" fn(c_int),
}

impl Lib {
    /// `void long_exec(unsigned int seed)`
    pub fn long_exec(&self, seed: c_uint) {
        unsafe { (self.long_exec)(seed) }
    }

    /// `long_exec` reached through a prototype that passes a 64-bit value, so
    /// only the low 32 bits are the `unsigned int` parameter.
    pub fn long_exec_u64(&self, raw: u64) {
        unsafe { (self.long_exec_u64)(raw) }
    }

    /// `void perform_expensive_operations()`
    pub fn peo(&self) {
        unsafe { (self.peo)() }
    }

    /// `perform_expensive_operations` reached through a prototype with an
    /// argument.  A C `void f()` declaration accepts any argument list, so this
    /// is a real call a consumer can make; the argument must be ignored.
    pub fn peo_with_arg(&self, arg: c_int) {
        unsafe { (self.peo_with_arg)(arg) }
    }

    pub fn array_ptr(&self) -> *mut c_int {
        self.array
    }

    pub fn read_array(&self) -> Vec<c_int> {
        unsafe { std::slice::from_raw_parts(self.array, ARRAY_LEN).to_vec() }
    }

    pub fn read_array_bytes(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.array as *const u8, ARRAY_BYTES).to_vec() }
    }

    pub fn write_array(&self, src: &[c_int]) {
        assert_eq!(src.len(), ARRAY_LEN);
        unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), self.array, ARRAY_LEN) }
    }
}

fn load(path: PathBuf, name: &'static str) -> Lib {
    unsafe {
        // Leaked so the symbols stay valid for the whole test binary.
        let lib: &'static libloading::Library = Box::leak(Box::new(
            libloading::Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen {} ({}): {e}", path.display(), name)),
        ));

        let long_exec = *lib
            .get::<unsafe extern "C" fn(c_uint)>(b"long_exec\0")
            .expect("long_exec not exported");
        let long_exec_u64 = *lib
            .get::<unsafe extern "C" fn(u64)>(b"long_exec\0")
            .expect("long_exec not exported");
        let peo = *lib
            .get::<unsafe extern "C" fn()>(b"perform_expensive_operations\0")
            .expect("perform_expensive_operations not exported");
        let peo_with_arg = *lib
            .get::<unsafe extern "C" fn(c_int)>(b"perform_expensive_operations\0")
            .expect("perform_expensive_operations not exported");

        // `array` is a data symbol: the symbol's own address is the object.
        let array_sym = lib
            .get::<*mut c_int>(b"array\0")
            .expect("array not exported");
        let array = array_sym.into_raw().into_raw() as *mut c_int;
        assert!(!array.is_null(), "{name}: array resolved to NULL");

        Lib {
            name,
            path,
            long_exec,
            peo,
            array,
            long_exec_u64,
            peo_with_arg,
        }
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    match std::env::var_os("C_LIB") {
        Some(p) => PathBuf::from(p),
        None => crate_root().join("../c_src/build/liblong.so"),
    }
}

fn rust_lib_path() -> PathBuf {
    if let Some(p) = std::env::var_os("RUST_LIB") {
        return PathBuf::from(p);
    }
    // Prefer the artifact built by the very `cargo test` invocation that is
    // running, so the active feature combination is the one under test.
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let here = profile_dir.join("liblong.so");
    if here.exists() {
        return here;
    }
    crate_root().join("target/release/liblong.so")
}

/// Both libraries share the process-wide `array` objects, glibc's `rand` state
/// and file descriptor 1, so every differential test must run exclusively.
static LOCK: Mutex<()> = Mutex::new(());

pub struct Libs {
    pub c: &'static Lib,
    pub rs: &'static Lib,
    _guard: MutexGuard<'static, ()>,
}

/// Acquire the two libraries.  Loaded once per test binary, handed out under a
/// mutex so tests never interleave their use of the shared globals.
pub fn libs() -> Libs {
    static C: OnceLock<Lib> = OnceLock::new();
    static R: OnceLock<Lib> = OnceLock::new();
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let c = C.get_or_init(|| load(c_lib_path(), "C"));
    let rs = R.get_or_init(|| load(rust_lib_path(), "Rust"));
    Libs {
        c,
        rs,
        _guard: guard,
    }
}

// SAFETY: the raw pointers in `Lib` point at process-lifetime `.bss` objects in
// the loaded shared objects; access is serialised by `LOCK`.
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

/// Run `f` with file descriptor 1 redirected to a temporary file and return the
/// bytes it wrote.  Used to capture the `printf("%d\n", ...)` from either
/// library exactly as a shell would see it.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "long_diff_{}_{}_{}.out",
        std::process::id(),
        id,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let bytes = {
        let file = std::fs::File::create(&path).expect("create capture file");
        let fd = {
            use std::os::unix::io::AsRawFd;
            file.as_raw_fd()
        };
        unsafe {
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(fd, 1) >= 0, "dup2 failed");
            f();
            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
            close(saved);
        }
        drop(file);
        std::fs::read(&path).expect("read capture file")
    };
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Deterministic SplitMix64, so every "random" configuration is reproducible.
pub struct Rng(pub u64);

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
    pub fn next_i32(&mut self) -> c_int {
        (self.next_u64() >> 32) as u32 as c_int
    }
    /// A value in `[0, 2^31)`, i.e. exactly the range glibc `rand()` produces.
    pub fn next_rand_range(&mut self) -> c_int {
        ((self.next_u64() >> 33) as u32 & 0x7fff_ffff) as c_int
    }
    pub fn fill(&mut self, buf: &mut [c_int]) {
        for slot in buf.iter_mut() {
            *slot = self.next_i32();
        }
    }
}

/// Compare two arrays element-wise, reporting the first divergence with enough
/// context to identify the offending input value.
pub fn assert_arrays_eq(label: &str, input: &[c_int], c: &[c_int], rs: &[c_int]) {
    assert_eq!(c.len(), rs.len(), "{label}: length mismatch");
    if c == rs {
        return;
    }
    let mut shown = 0;
    let mut msg = format!("{label}: array divergence\n");
    for i in 0..c.len() {
        if c[i] != rs[i] {
            if shown < 8 {
                msg += &format!(
                    "  [{i}] input={} C={} Rust={}\n",
                    input.get(i).copied().unwrap_or(0),
                    c[i],
                    rs[i]
                );
            }
            shown += 1;
        }
    }
    msg += &format!("  {shown} of {} elements differ", c.len());
    panic!("{msg}");
}

/// Drive one library: install `input` into its `array`, call
/// `perform_expensive_operations()` `k` times, return the resulting array.
pub fn run_peo(lib: &Lib, input: &[c_int], k: u32) -> Vec<c_int> {
    lib.write_array(input);
    for _ in 0..k {
        lib.peo();
    }
    lib.read_array()
}

/// The core differential check for the low-level entry point: same input, same
/// number of composed passes, byte-identical `array` afterwards.
pub fn diff_peo(label: &str, input: &[c_int], k: u32) {
    let l = libs();
    let c = run_peo(l.c, input, k);
    let rs = run_peo(l.rs, input, k);
    assert_arrays_eq(&format!("{label} (k={k})"), input, &c, &rs);
    // Byte-level equality of the exported objects, not just element equality.
    assert_eq!(
        l.c.read_array_bytes(),
        l.rs.read_array_bytes(),
        "{label} (k={k}): raw byte images of `array` differ"
    );
}
