// Shared differential-test harness.
//
// Both implementations are loaded as shared objects with `libloading` and driven
// exclusively through `dlsym`'d symbols. Nothing in this harness calls a Rust
// function from the crate directly, so the `#[unsafe(no_mangle)]` export
// wrappers, the C ABI of every parameter, and the linker symbol names are all
// part of what is under test.
//
//   C    .so : c_src/build/libtranslated_rust.so
//   Rust .so : target/<profile>/libcharinbuf_lib.so
//
// `charinbuf` communicates through `stdout` as well as its return value, so
// every comparison covers the captured stdout bytes too.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs (fd juggling + freeing returned buffers).
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

/// `free` for pointers handed back by `create_buffer`. Both `.so`s allocate with
/// the process `malloc`, so the process `free` is the correct counterpart.
pub unsafe fn libc_free(p: *mut c_char) {
    unsafe { free(p.cast()) }
}

pub unsafe fn libc_strlen(p: *const c_char) -> usize {
    unsafe { strlen(p) }
}

// ---------------------------------------------------------------------------
// The exported ABI, as seen by an external caller.
// ---------------------------------------------------------------------------

pub type Op = extern "C" fn(c_int) -> c_int;

pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,

    pub charinbuf: extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
    pub increment_counter: extern "C" fn(c_int) -> c_int,
    pub decrement_counter: extern "C" fn(c_int) -> c_int,
    pub multiply_counter: extern "C" fn(c_int) -> c_int,
    pub reset_counter: extern "C" fn(c_int) -> c_int,
    pub validate_uint16_range: extern "C" fn(c_int) -> c_int,
    pub is_string_empty: unsafe extern "C" fn(*const c_char) -> c_int,
    pub find_char_in_buffer: unsafe extern "C" fn(*const c_char, usize, c_char) -> *mut c_char,
    pub create_buffer: unsafe extern "C" fn(*const c_char) -> *mut c_char,
    // `operation_func` is passed as a bare pointer so NULL and foreign callbacks
    // are both expressible.
    pub apply_operation: unsafe extern "C" fn(*const c_void, c_int) -> c_int,

    // Raw addresses of this library's own counter operations, for feeding back
    // into its own `apply_operation`.
    pub p_increment: *const c_void,
    pub p_decrement: *const c_void,
    pub p_multiply: *const c_void,
    pub p_reset: *const c_void,

    _lib: libloading::Library,
}

// The pointers above are plain code addresses in a permanently-loaded library.
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: libloading::Symbol<$ty> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("dlsym({}) failed: {e}", $name));
        *s
    }};
}

impl Api {
    fn load(name: &'static str, path: PathBuf) -> Api {
        let lib = unsafe { libloading::Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));

        let increment = sym!(lib, "increment_counter", extern "C" fn(c_int) -> c_int);
        let decrement = sym!(lib, "decrement_counter", extern "C" fn(c_int) -> c_int);
        let multiply = sym!(lib, "multiply_counter", extern "C" fn(c_int) -> c_int);
        let reset = sym!(lib, "reset_counter", extern "C" fn(c_int) -> c_int);

        Api {
            name,
            path,
            charinbuf: sym!(
                lib,
                "charinbuf",
                extern "C" fn(c_int, c_int, c_int, c_int) -> c_int
            ),
            increment_counter: increment,
            decrement_counter: decrement,
            multiply_counter: multiply,
            reset_counter: reset,
            validate_uint16_range: sym!(
                lib,
                "validate_uint16_range",
                extern "C" fn(c_int) -> c_int
            ),
            is_string_empty: sym!(
                lib,
                "is_string_empty",
                unsafe extern "C" fn(*const c_char) -> c_int
            ),
            find_char_in_buffer: sym!(
                lib,
                "find_char_in_buffer",
                unsafe extern "C" fn(*const c_char, usize, c_char) -> *mut c_char
            ),
            create_buffer: sym!(
                lib,
                "create_buffer",
                unsafe extern "C" fn(*const c_char) -> *mut c_char
            ),
            apply_operation: sym!(
                lib,
                "apply_operation",
                unsafe extern "C" fn(*const c_void, c_int) -> c_int
            ),
            p_increment: increment as *const c_void,
            p_decrement: decrement as *const c_void,
            p_multiply: multiply as *const c_void,
            p_reset: reset as *const c_void,
            _lib: lib,
        }
    }

    /// Puts this library's hidden `static counter` into a known state.
    pub fn normalize_counter(&self) {
        (self.reset_counter)(0);
    }

    /// Reads the hidden counter without disturbing it (`counter += 0`).
    pub fn peek_counter(&self) -> c_int {
        (self.increment_counter)(0)
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects.
// ---------------------------------------------------------------------------

/// Newest mtime among the files matching `ext` under `dir` (recursively).
fn newest_source_mtime(dir: &std::path::Path, ext: &str) -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(t) = newest_source_mtime(&p, ext) {
                newest = Some(newest.map_or(t, |n: std::time::SystemTime| n.max(t)));
            }
        } else if p.extension().and_then(|s| s.to_str()) == Some(ext) {
            if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                newest = Some(newest.map_or(t, |n: std::time::SystemTime| n.max(t)));
            }
        }
    }
    newest
}

/// Guards against the single most dangerous failure mode of this harness:
/// silently testing a **stale** shared object, which makes every test pass
/// vacuously.
///
/// `cargo test --test <name>` does *not* rebuild the `cdylib`, because no Rust
/// target can depend on a `cdylib` — so a source edit is invisible to
/// `cargo test` unless `cargo build` is run first. This check turns that silent
/// false-pass into a loud failure.
fn assert_fresh(so: &std::path::Path, src_dir: &std::path::Path, ext: &str, rebuild: &str) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    if let Some(src_mtime) = newest_source_mtime(src_dir, ext) {
        assert!(
            so_mtime >= src_mtime,
            "STALE SHARED OBJECT — refusing to run a vacuous test.\n  \
             {} is older than the newest source in {}\n  \
             rebuild with: {}",
            so.display(),
            src_dir.display(),
            rebuild
        );
    }
}

fn c_so_path() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let p = root.join("c_src/build/libtranslated_rust.so");
    assert!(
        p.is_file(),
        "C shared object not built at {}\n\
         build it with: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    assert_fresh(
        &p,
        &root.join("c_src"),
        "c",
        "cd c_src/build && cmake --build .",
    );
    p
}

fn rust_so_path() -> PathBuf {
    // target/<profile>/deps/<test-exe>  ->  target/<profile>/libcharinbuf_lib.so
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    for _ in 0..3 {
        let cand = dir.join("libcharinbuf_lib.so");
        if cand.is_file() {
            assert_fresh(
                &cand,
                &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src"),
                "rs",
                "cargo build   (cargo test alone does NOT rebuild a cdylib)",
            );
            return cand;
        }
        if !dir.pop() {
            break;
        }
    }
    panic!(
        "Rust cdylib libcharinbuf_lib.so not found near {}\n\
         build it with: cargo build",
        exe.display()
    );
}

static LIBS: OnceLock<(Api, Api)> = OnceLock::new();

/// `(c, rust)` — both loaded exactly once for the lifetime of the process, so
/// the hidden `static counter` inside each behaves like it would for a real
/// long-running consumer.
pub fn apis() -> &'static (Api, Api) {
    LIBS.get_or_init(|| {
        (
            Api::load("C", c_so_path()),
            Api::load("Rust", rust_so_path()),
        )
    })
}

// ---------------------------------------------------------------------------
// Serialization.
//
// Needed for two independent reasons: the stdout capture below rewires the
// process-wide fd 1, and both libraries carry mutable `static` state that
// concurrent tests would interleave.
// ---------------------------------------------------------------------------

static GATE: Mutex<()> = Mutex::new(());

pub fn gate() -> MutexGuard<'static, ()> {
    GATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---------------------------------------------------------------------------
// stdout capture.
// ---------------------------------------------------------------------------

/// Runs `f` with fd 1 redirected to a temporary file and returns `f`'s value
/// together with the exact bytes the callee wrote to `stdout`.
///
/// `fflush(NULL)` before and after drains the *C* streams — the same `stdout`
/// FILE both `.so`s print through — without touching Rust's own `LineWriter`
/// buffer, so libtest's own progress output cannot bleed into the capture.
pub fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _ = std::io::stdout().flush();

    let mut tmp = tempfile();
    let tmp_fd = tmp.as_raw_fd();

    let (value, bytes) = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp_fd, 1) >= 0, "dup2 failed");

        let value = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);

        tmp.seek(SeekFrom::Start(0)).expect("seek");
        let mut bytes = Vec::new();
        tmp.read_to_end(&mut bytes).expect("read capture");
        (value, bytes)
    };

    (value, bytes)
}

fn tempfile() -> std::fs::File {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "charinbuf_diff_{}_{}.out",
        std::process::id(),
        n
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("open temp capture file");
    // Unlink immediately; the open fd keeps it alive.
    let _ = std::fs::remove_file(&path);
    f
}

// ---------------------------------------------------------------------------
// Differential assertion helpers.
// ---------------------------------------------------------------------------

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Asserts that a `charinbuf`-style call returns the same value AND writes the
/// same stdout bytes from both libraries.
#[track_caller]
pub fn assert_same_call(ctx: &str, c_out: (c_int, Vec<u8>), r_out: (c_int, Vec<u8>)) {
    assert_eq!(
        c_out.0, r_out.0,
        "return value mismatch for {ctx}\n  C    = {}\n  Rust = {}\n  C stdout    = \"{}\"\n  Rust stdout = \"{}\"",
        c_out.0,
        r_out.0,
        show(&c_out.1),
        show(&r_out.1)
    );
    assert_eq!(
        c_out.1,
        r_out.1,
        "stdout mismatch for {ctx}\n  C    = \"{}\"\n  Rust = \"{}\"",
        show(&c_out.1),
        show(&r_out.1)
    );
}

/// Calls `charinbuf` on both libraries with the same arguments and compares
/// return value and stdout.
#[track_caller]
pub fn diff_charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) {
    let (c, r) = apis();
    let ctx = format!("charinbuf(mode={mode}, value={value}, opt1={opt1}, opt2={opt2})");
    let c_out = capture(|| (c.charinbuf)(mode, value, opt1, opt2));
    let r_out = capture(|| (r.charinbuf)(mode, value, opt1, opt2));
    assert_same_call(&ctx, c_out, r_out);
}

/// Same, but also compares the hidden counter left behind afterwards.
#[track_caller]
pub fn diff_charinbuf_with_state(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) {
    let (c, r) = apis();
    diff_charinbuf(mode, value, opt1, opt2);
    let cs = c.peek_counter();
    let rs = r.peek_counter();
    assert_eq!(
        cs, rs,
        "residual counter mismatch after charinbuf({mode}, {value}, {opt1}, {opt2})"
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds, reproducible runs, no extra
// dependency.
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

    /// Uniform over the whole `int` domain.
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn in_range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    /// A mix of "interesting" and uniform `int`s: boundary values appear often
    /// enough to matter, but the full domain is still covered.
    pub fn interesting_i32(&mut self) -> i32 {
        const SPECIAL: [i32; 14] = [
            0,
            1,
            -1,
            2,
            -2,
            5,
            65535,
            65536,
            65534,
            -65535,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
        ];
        if self.next_u64() % 3 == 0 {
            SPECIAL[self.below(SPECIAL.len())]
        } else {
            self.next_i32()
        }
    }
}

// ---------------------------------------------------------------------------
// Differential helpers for the nine lower-level entry points.
//
// None of these nine C functions writes to stdout; `silence.rs` pins that
// separately, so these helpers compare return values only and stay fast enough
// for large randomized loops.
// ---------------------------------------------------------------------------

/// `validate_uint16_range`
#[track_caller]
pub fn diff_validate(value: c_int) {
    let (c, r) = apis();
    let cv = (c.validate_uint16_range)(value);
    let rv = (r.validate_uint16_range)(value);
    assert_eq!(
        cv, rv,
        "validate_uint16_range({value}): C={cv} Rust={rv}"
    );
}

/// `is_string_empty`. `s` must contain its own NUL terminator.
#[track_caller]
pub fn diff_is_string_empty(s: &[u8]) {
    assert!(s.contains(&0), "test string needs a NUL terminator");
    let (c, r) = apis();
    let p = s.as_ptr().cast::<c_char>();
    let cv = unsafe { (c.is_string_empty)(p) };
    let rv = unsafe { (r.is_string_empty)(p) };
    assert_eq!(
        cv,
        rv,
        "is_string_empty({}): C={cv} Rust={rv}",
        show(&s[..s.iter().position(|&b| b == 0).unwrap()])
    );
}

/// `is_string_empty` with an arbitrary (possibly NULL) pointer.
#[track_caller]
pub fn diff_is_string_empty_raw(p: *const c_char, ctx: &str) {
    let (c, r) = apis();
    let cv = unsafe { (c.is_string_empty)(p) };
    let rv = unsafe { (r.is_string_empty)(p) };
    assert_eq!(cv, rv, "is_string_empty({ctx}): C={cv} Rust={rv}");
}

/// `find_char_in_buffer`. Both libraries search the *same* caller-owned buffer,
/// so the returned pointers are comparable as offsets — a null-vs-null check
/// alone would miss a wrong-position bug.
#[track_caller]
pub fn diff_find_char(buf: &[u8], size: usize, target: u8) {
    let (c, r) = apis();
    let p = buf.as_ptr().cast::<c_char>();
    let t = target as c_char;

    let cp = unsafe { (c.find_char_in_buffer)(p, size, t) };
    let rp = unsafe { (r.find_char_in_buffer)(p, size, t) };

    let off = |q: *mut c_char| -> Option<isize> {
        if q.is_null() {
            None
        } else {
            Some(unsafe { q.offset_from(p) })
        }
    };
    let (co, ro) = (off(cp), off(rp));
    assert_eq!(
        co,
        ro,
        "find_char_in_buffer(buf={}, size={size}, target=0x{target:02x}): C={co:?} Rust={ro:?}",
        show(buf)
    );

    // Sanity-check against the expected answer, so a bug present in *both*
    // (e.g. a harness mistake) still shows up.
    let expected = buf[..size.min(buf.len())]
        .iter()
        .position(|&b| b == target)
        .map(|i| i as isize);
    if size <= buf.len() {
        assert_eq!(
            co, expected,
            "both libraries disagree with memchr semantics for target=0x{target:02x} size={size}"
        );
    }
}

/// `find_char_in_buffer` with an arbitrary (possibly NULL) pointer / huge size.
#[track_caller]
pub fn diff_find_char_raw(p: *const c_char, size: usize, target: u8, ctx: &str) {
    let (c, r) = apis();
    let t = target as c_char;
    let cp = unsafe { (c.find_char_in_buffer)(p, size, t) };
    let rp = unsafe { (r.find_char_in_buffer)(p, size, t) };
    assert_eq!(
        cp.is_null(),
        rp.is_null(),
        "find_char_in_buffer({ctx}): C={cp:?} Rust={rp:?}"
    );
    if !cp.is_null() {
        assert_eq!(cp, rp, "find_char_in_buffer({ctx}) pointer mismatch");
    }
}

/// `create_buffer`. Compares NULL-ness, the duplicated bytes, `strlen` of the
/// result, and that the result is a distinct allocation from the input; frees
/// both results with the process `free`.
#[track_caller]
pub fn diff_create_buffer(s: &[u8]) {
    assert!(s.contains(&0), "test string needs a NUL terminator");
    let (c, r) = apis();
    let p = s.as_ptr().cast::<c_char>();

    let cb = unsafe { (c.create_buffer)(p) };
    let rb = unsafe { (r.create_buffer)(p) };

    assert_eq!(
        cb.is_null(),
        rb.is_null(),
        "create_buffer NULL-ness differs: C={cb:?} Rust={rb:?}"
    );

    if !cb.is_null() {
        let expected_len = s.iter().position(|&b| b == 0).unwrap();
        let cl = unsafe { libc_strlen(cb) };
        let rl = unsafe { libc_strlen(rb) };
        assert_eq!(cl, rl, "create_buffer strlen differs: C={cl} Rust={rl}");
        assert_eq!(cl, expected_len, "create_buffer strlen wrong in both");

        // Compare the copied bytes including the terminator.
        let cbytes = unsafe { std::slice::from_raw_parts(cb.cast::<u8>(), cl + 1) };
        let rbytes = unsafe { std::slice::from_raw_parts(rb.cast::<u8>(), rl + 1) };
        assert_eq!(
            cbytes,
            rbytes,
            "create_buffer contents differ: C=\"{}\" Rust=\"{}\"",
            show(cbytes),
            show(rbytes)
        );
        assert_eq!(cbytes, &s[..expected_len + 1], "create_buffer copy wrong in both");
        assert_eq!(cbytes[cl], 0, "missing NUL terminator");

        assert_ne!(cb.cast_const(), p, "C returned the input pointer");
        assert_ne!(rb.cast_const(), p, "Rust returned the input pointer");

        unsafe {
            libc_free(cb);
            libc_free(rb);
        }
    }
}

/// `apply_operation` with a raw callback pointer.
#[track_caller]
pub fn diff_apply_operation_raw(
    c_op: *const c_void,
    r_op: *const c_void,
    value: c_int,
    ctx: &str,
) {
    let (c, r) = apis();
    let cv = unsafe { (c.apply_operation)(c_op, value) };
    let rv = unsafe { (r.apply_operation)(r_op, value) };
    assert_eq!(
        cv, rv,
        "apply_operation({ctx}, {value}): C={cv} Rust={rv}"
    );
}
