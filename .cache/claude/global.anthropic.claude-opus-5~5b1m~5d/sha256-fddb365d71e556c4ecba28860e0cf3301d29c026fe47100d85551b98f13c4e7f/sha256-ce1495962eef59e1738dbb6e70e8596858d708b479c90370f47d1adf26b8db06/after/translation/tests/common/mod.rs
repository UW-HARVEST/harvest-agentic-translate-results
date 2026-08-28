// Shared differential-test harness.
//
// Loads BOTH shared objects (the C one built by cmake and the Rust cdylib) with
// `libloading` and calls every entry point exclusively through `dlsym`, exactly
// like an external C consumer would. Nothing is ever called directly against
// the Rust crate, so the `#[unsafe(no_mangle)] extern "C"` wrappers are part of
// what is under test.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libc bits needed to capture the stdout the libraries print to.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// The same allocator both `.so`s use, so the test can hand a library a block it
/// is allowed to `free()` (needed to build a `ProcessState` with a NULL buffer).
pub mod libc {
    unsafe extern "C" {
        pub fn malloc(size: usize) -> *mut u8;
        pub fn free(ptr: *mut u8);
    }
}

// ---------------------------------------------------------------------------
// Signature aliases for the six exported symbols.
// ---------------------------------------------------------------------------
pub type FnCreateState = unsafe extern "C" fn(c_int, c_int) -> *mut u8;
pub type FnDestroyState = unsafe extern "C" fn(*mut u8);
pub type FnProcessBuffer = unsafe extern "C" fn(*mut u8, c_char) -> c_int;
pub type FnUpdateFlags = unsafe extern "C" fn(*mut u8, c_int);
pub type FnConfuseTypes = unsafe extern "C" fn(*mut u8, c_int) -> c_int;
pub type FnConfusion = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    pub create_state: FnCreateState,
    pub destroy_state: FnDestroyState,
    pub process_buffer: FnProcessBuffer,
    pub update_flags: FnUpdateFlags,
    pub confuse_types: FnConfuseTypes,
    pub confusion: FnConfusion,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        // The `Library` is intentionally leaked: the handle must stay valid for
        // the whole process lifetime because we hand out raw fn pointers.
        let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
        }));
        macro_rules! sym {
            ($t:ty, $n:literal) => {{
                let s: libloading::Symbol<'static, $t> = unsafe {
                    lib.get(concat!($n, "\0").as_bytes()).unwrap_or_else(|e| {
                        panic!("dlsym({}, {}) failed: {e}", path.display(), $n)
                    })
                };
                *s
            }};
        }
        Lib {
            name,
            create_state: sym!(FnCreateState, "create_state"),
            destroy_state: sym!(FnDestroyState, "destroy_state"),
            process_buffer: sym!(FnProcessBuffer, "process_buffer"),
            update_flags: sym!(FnUpdateFlags, "update_flags"),
            confuse_types: sym!(FnConfuseTypes, "confuse_types"),
            confusion: sym!(FnConfusion, "confusion"),
            path,
        }
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<project>.so`, located by scanning the cmake build dir.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB_PATH") {
        return PathBuf::from(p);
    }
    let build_dir = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = fs::read_dir(&build_dir) {
        for e in rd.flatten() {
            let p = e.path();
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") && p.is_file() {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no lib*.so under {}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

/// `target/{debug,release}/libconfusion_lib.so`.
///
/// The profile is chosen to match the profile this test binary was built with,
/// so `cargo test --release` really exercises the shipped release object.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB_PATH") {
        return PathBuf::from(p);
    }
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir: &Path = exe.parent().expect("deps dir");
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir = dir.parent().expect("profile dir");
    }
    let direct = dir.join("libconfusion_lib.so");
    // Deliberately strict: NO cross-profile fallback. `cargo test` does not
    // build a `cdylib`, so a fallback would silently test the *other* profile's
    // object and quietly invalidate the whole run.
    assert!(
        direct.is_file(),
        "{} does not exist.\nThe cdylib under test must be built for this profile first:\n  \
         cargo build{}\nor use ./run-diff-tests.sh, which does it for every configuration.",
        direct.display(),
        if cfg!(debug_assertions) { "" } else { " --release" }
    );
    direct
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static RUST_LIB: OnceLock<Lib> = OnceLock::new();

pub fn c_lib() -> &'static Lib {
    C_LIB.get_or_init(|| Lib::open("C", c_so_path()))
}
pub fn rust_lib() -> &'static Lib {
    RUST_LIB.get_or_init(|| Lib::open("RUST", rust_so_path()))
}

/// The two implementations, always in (C, Rust) order.
pub fn both() -> (&'static Lib, &'static Lib) {
    (c_lib(), rust_lib())
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------
static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(0);
/// fd 1 is process-wide, so only one capture may be in flight at a time.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Capture scratch files live under `target/`, which is guaranteed writable and
/// stable for the whole run (unlike `TMPDIR` in some sandboxes).
fn capture_dir() -> PathBuf {
    let d = manifest_dir().join("target").join("difftest-capture");
    let _ = fs::create_dir_all(&d);
    d
}

/// Capturing fd 1 is inherently process-wide; refuse to produce bogus results
/// if the harness was started with more than one test thread.
fn assert_single_threaded() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let v = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
        assert_eq!(
            v, "1",
            "the differential tests capture the process-wide fd 1 and must run \
             sequentially. Set RUST_TEST_THREADS=1 (translation/.cargo/config.toml \
             does this) or pass `-- --test-threads=1`."
        );
    });
}

/// Runs `f` with fd 1 redirected into a temp file and returns `(result, stdout)`.
///
/// Both libraries print through the *process's* glibc `stdout`, so flushing
/// before and after is what makes the captured bytes exact.
pub fn capture<T, F: FnOnce() -> T>(f: F) -> (T, Vec<u8>) {
    assert_single_threaded();
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let n = CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst);
    let mut path = capture_dir();
    path.push(format!("difftest-{}-{}.out", std::process::id(), n));

    // Push anything libtest has buffered in Rust's own line-buffered stdout out
    // to the *real* fd 1 before we hijack it, and flush every glibc FILE so the
    // capture window contains exactly what the library under test writes.
    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe { fflush(std::ptr::null_mut()) };

    let file = fs::File::create(&path).expect("create capture file");
    let file_fd = {
        use std::os::fd::AsRawFd;
        file.as_raw_fd()
    };

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file_fd, 1) } >= 0, "dup2 failed");

    let result = f();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };
    drop(file);

    let mut bytes = Vec::new();
    fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut bytes)
        .expect("read capture file");
    let _ = fs::remove_file(&path);

    (result, bytes)
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\n', "\\n")
}

// ---------------------------------------------------------------------------
// ProcessState introspection
//
// struct { PackedFlags flags; TypeConfusion data; char* buffer; int capacity; }
// offsets 0 / 4 / 8 / 16, sizeof 24, alignof 8  (verified against gcc).
// ---------------------------------------------------------------------------
pub const STATE_SIZE: usize = 24;
pub const OFF_FLAGS: usize = 0;
pub const OFF_DATA: usize = 4;
pub const OFF_BUFFER: usize = 8;
pub const OFF_CAPACITY: usize = 16;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StateSnapshot {
    pub flags: u32,
    pub data: u32,
    pub capacity: i32,
    pub buffer_null: bool,
    /// Bytes up to (excluding) the first NUL, or `None` when the buffer's
    /// contents are indeterminate (`capacity <= 0`) and must not be compared.
    pub buffer: Option<Vec<u8>>,
}

impl StateSnapshot {
    pub fn flag1(&self) -> u32 {
        self.flags & 1
    }
    pub fn flag2(&self) -> u32 {
        (self.flags >> 1) & 1
    }
    pub fn flag3(&self) -> u32 {
        (self.flags >> 2) & 1
    }
    pub fn counter(&self) -> u32 {
        (self.flags >> 3) & 0x1F
    }
    pub fn mode(&self) -> u32 {
        (self.flags >> 8) & 0x7
    }
    pub fn status(&self) -> u32 {
        (self.flags >> 11) & 0x1F
    }
    pub fn reserved(&self) -> u32 {
        self.flags >> 16
    }
}

/// Reads every *observable* field out of a `ProcessState` produced by either
/// library. The `buffer` pointer value itself is deliberately not compared
/// (heap addresses legitimately differ); its contents are.
pub unsafe fn snapshot(state: *mut u8) -> StateSnapshot {
    assert!(!state.is_null(), "snapshot of NULL state");
    let flags = unsafe { std::ptr::read_unaligned(state.add(OFF_FLAGS) as *const u32) };
    let data = unsafe { std::ptr::read_unaligned(state.add(OFF_DATA) as *const u32) };
    let bufp = unsafe { std::ptr::read_unaligned(state.add(OFF_BUFFER) as *const *mut u8) };
    let capacity = unsafe { std::ptr::read_unaligned(state.add(OFF_CAPACITY) as *const i32) };

    let buffer = if bufp.is_null() || capacity <= 0 {
        None
    } else {
        let cap = capacity as usize;
        let mut v = Vec::new();
        for i in 0..cap {
            let b = unsafe { *bufp.add(i) };
            if b == 0 {
                break;
            }
            v.push(b);
        }
        Some(v)
    };

    StateSnapshot {
        flags,
        data,
        capacity,
        buffer_null: bufp.is_null(),
        buffer,
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed => reproducible test inputs.
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
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
    /// An `i32` biased toward interesting shapes: boundaries, small magnitudes,
    /// and fully random bit patterns.
    pub fn interesting_i32(&mut self) -> i32 {
        match self.below(8) {
            0 => *self.pick(&BOUNDARY_I32),
            1 => *self.pick(&FLOAT_BITS),
            2 => (self.next_u64() % 64) as i32,
            3 => -((self.next_u64() % 64) as i32),
            4 => (self.next_u32() & 0xFFFF) as i32,
            _ => self.next_i32(),
        }
    }
}

pub const BOUNDARY_I32: [i32; 17] = [
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    4,
    -4,
    7,
    8,
    -8,
    9,
    10,
    -10,
    i32::MAX,
    i32::MIN,
];

/// `int` values whose bit pattern, read back as `float` by `confuse_types(1)`,
/// covers every IEEE-754 class plus the `cvttss2si` overflow boundary.
pub const FLOAT_BITS: [i32; 26] = [
    0x0000_0000u32 as i32, // +0.0
    0x8000_0000u32 as i32, // -0.0
    0x0000_0001u32 as i32, // smallest positive subnormal
    0x8000_0001u32 as i32, // smallest negative subnormal
    0x007F_FFFFu32 as i32, // largest subnormal
    0x0080_0000u32 as i32, // FLT_MIN
    0x3F80_0000u32 as i32, // 1.0
    0xBF80_0000u32 as i32, // -1.0
    0x4049_0FDBu32 as i32, // 3.14159274 (== 1078530011)
    0x7F7F_FFFFu32 as i32, // FLT_MAX
    0xFF7F_FFFFu32 as i32, // -FLT_MAX
    0x7F80_0000u32 as i32, // +Inf
    0xFF80_0000u32 as i32, // -Inf
    0x7FC0_0000u32 as i32, // qNaN
    0xFFC0_0000u32 as i32, // -qNaN
    0x7F80_0001u32 as i32, // sNaN
    0xFFBF_FFFFu32 as i32, // -sNaN
    0x4CBE_BC20u32 as i32, // 1.0e8  -> *100 overflows int32
    0x4B18_9680u32 as i32, // 1.0e7  -> *100 == 1e9, in range
    0x4BA7_D8C0u32 as i32, // 2.2e7  -> *100 == 2.2e9, just over INT_MAX
    0x4EFF_FFFFu32 as i32, // 2.14748352e9 (largest float < 2^31)
    0x4F00_0000u32 as i32, // 2.147483648e9 == 2^31 exactly -> INT_MIN
    0xCF00_0000u32 as i32, // -2^31 exactly -> representable
    0xCF00_0001u32 as i32, // just past -2^31 -> INT_MIN
    0x3F7F_FFFFu32 as i32, // 0.99999994
    0x4B18_967Fu32 as i32, // 9999999.0
];

// ---------------------------------------------------------------------------
// Differential driver shared by Phase B and Phase C.
// ---------------------------------------------------------------------------
pub struct Outcome {
    pub log: Vec<String>,
    pub stdout: Vec<u8>,
}

pub fn run(lib: &'static Lib, f: &dyn Fn(&'static Lib, &mut Vec<String>)) -> Outcome {
    let mut log: Vec<String> = Vec::new();
    let (_, stdout) = capture(|| f(lib, &mut log));
    Outcome { log, stdout }
}

/// Runs one scenario against both libraries and asserts total agreement on
/// every recorded observable **and** on the complete stdout byte stream.
#[track_caller]
pub fn diff(ctx: &str, f: &dyn Fn(&'static Lib, &mut Vec<String>)) {
    let (c, r) = both();
    let oc = run(c, f);
    let or = run(r, f);

    if oc.log != or.log {
        let first = oc
            .log
            .iter()
            .zip(or.log.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| oc.log.len().min(or.log.len()));
        panic!(
            "[{ctx}] observable-state divergence at entry #{first}\n  C    = {:?}\n  RUST = {:?}\n\
             full C    = {:#?}\nfull RUST = {:#?}",
            oc.log.get(first),
            or.log.get(first),
            oc.log,
            or.log
        );
    }
    assert_eq!(
        oc.stdout,
        or.stdout,
        "[{ctx}] stdout divergence\n  C    = \"{}\"\n  RUST = \"{}\"",
        show(&oc.stdout),
        show(&or.stdout)
    );
}

/// Same as [`diff`], but also hands back the (identical) C-side outcome so a
/// test can additionally assert the *absolute* expected value from the C source.
#[track_caller]
pub fn diff_and_get(ctx: &str, f: &dyn Fn(&'static Lib, &mut Vec<String>)) -> Outcome {
    diff(ctx, f);
    run(c_lib(), f)
}

pub fn log_state(log: &mut Vec<String>, tag: &str, state: *mut u8) {
    if state.is_null() {
        log.push(format!("{tag}: state=NULL"));
        return;
    }
    let s = unsafe { snapshot(state) };
    log.push(format!(
        "{tag}: flags=0x{:08x} f1={} f2={} f3={} counter={} mode={} status={} reserved={} \
         data=0x{:08x} capacity={} buf_null={} buf={:?}",
        s.flags,
        s.flag1(),
        s.flag2(),
        s.flag3(),
        s.counter(),
        s.mode(),
        s.status(),
        s.reserved(),
        s.data,
        s.capacity,
        s.buffer_null,
        s.buffer
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).to_string()),
    ));
}

/// Like [`log_state`] but never touches the `buffer` contents. Used where the
/// C leaves the buffer indeterminate (`capacity == 0`: `snprintf(buf, 0, ...)`
/// writes nothing at all, so the bytes are whatever `malloc` happened to hand
/// back and are NOT a defined part of the behaviour).
pub fn log_state_no_buffer(log: &mut Vec<String>, tag: &str, state: *mut u8) {
    if state.is_null() {
        log.push(format!("{tag}: state=NULL"));
        return;
    }
    let s = unsafe { snapshot_no_buffer(state) };
    log.push(format!(
        "{tag}: flags=0x{:08x} data=0x{:08x} capacity={} buf_null={}",
        s.flags, s.data, s.capacity, s.buffer_null
    ));
}

pub unsafe fn snapshot_no_buffer(state: *mut u8) -> StateSnapshot {
    assert!(!state.is_null());
    let flags = unsafe { std::ptr::read_unaligned(state.add(OFF_FLAGS) as *const u32) };
    let data = unsafe { std::ptr::read_unaligned(state.add(OFF_DATA) as *const u32) };
    let bufp = unsafe { std::ptr::read_unaligned(state.add(OFF_BUFFER) as *const *mut u8) };
    let capacity = unsafe { std::ptr::read_unaligned(state.add(OFF_CAPACITY) as *const i32) };
    StateSnapshot {
        flags,
        data,
        capacity,
        buffer_null: bufp.is_null(),
        buffer: None,
    }
}

/// Overwrites the `buffer` field of a `ProcessState` with NULL, releasing the
/// old block with the *same* allocator the library used. Lets the tests reach
/// the `state->buffer == NULL` branches (`lib.c:92`, `lib.c:100`) without ever
/// creating a dangling or foreign pointer.
pub unsafe fn null_out_buffer(state: *mut u8) {
    let bufp = unsafe { std::ptr::read_unaligned(state.add(OFF_BUFFER) as *const *mut u8) };
    if !bufp.is_null() {
        unsafe { libc::free(bufp) };
    }
    unsafe { std::ptr::write_unaligned(state.add(OFF_BUFFER) as *mut *mut u8, std::ptr::null_mut()) };
}
