//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading`; the Rust
//! crate is never linked directly, so every call goes through the `#[no_mangle]`
//! export wrappers exactly as an external C consumer would see them.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------- libc pieces

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn free(p: *mut c_void);
}

// ------------------------------------------------------------- function types

pub type CounterFn = unsafe extern "C" fn(c_int) -> c_int;
pub type IsStringEmptyFn = unsafe extern "C" fn(*const c_char) -> c_int;
pub type FindCharFn = unsafe extern "C" fn(*const c_char, usize, c_char) -> *mut c_char;
pub type CreateBufferFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;
pub type ValidateFn = unsafe extern "C" fn(c_int) -> c_int;
pub type ApplyOperationFn = unsafe extern "C" fn(Option<CounterFn>, c_int) -> c_int;
pub type CharInBufFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C one or the Rust one).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub increment_counter: CounterFn,
    pub decrement_counter: CounterFn,
    pub multiply_counter: CounterFn,
    pub reset_counter: CounterFn,
    pub is_string_empty: IsStringEmptyFn,
    pub find_char_in_buffer: FindCharFn,
    pub create_buffer: CreateBufferFn,
    pub validate_uint16_range: ValidateFn,
    pub apply_operation: ApplyOperationFn,
    pub charinbuf: CharInBufFn,
}

fn sym<T: Copy>(lib: &'static Library, name: &str) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(format!("{name}\0").as_bytes())
            .unwrap_or_else(|e| panic!("missing symbol `{name}`: {e}"));
        *s
    }
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(&path).unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()))
        }));
        Impl {
            name,
            increment_counter: sym(lib, "increment_counter"),
            decrement_counter: sym(lib, "decrement_counter"),
            multiply_counter: sym(lib, "multiply_counter"),
            reset_counter: sym(lib, "reset_counter"),
            is_string_empty: sym(lib, "is_string_empty"),
            find_char_in_buffer: sym(lib, "find_char_in_buffer"),
            create_buffer: sym(lib, "create_buffer"),
            validate_uint16_range: sym(lib, "validate_uint16_range"),
            apply_operation: sym(lib, "apply_operation"),
            charinbuf: sym(lib, "charinbuf"),
            path,
        }
    }

    /// The four counter entry points, in a stable order, as raw pointers so they
    /// can be handed back to *this* library's `apply_operation`.
    pub fn ops(&self) -> [(&'static str, CounterFn); 4] {
        [
            ("reset", self.reset_counter),
            ("increment", self.increment_counter),
            ("decrement", self.decrement_counter),
            ("multiply", self.multiply_counter),
        ]
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate root has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("{} not built: {e}", build.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "so"))
        .collect();
    found.sort();
    found
        .pop()
        .unwrap_or_else(|| panic!("no .so under {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join("libcharinbuf_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libcharinbuf_lib.so not built; run `cargo build --release` first");
}

/// Load both implementations (once per test process).
pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", find_c_so()),
        rs: Impl::load("Rust", find_rust_so()),
    })
}

/// Serialise tests: both libraries keep a process-global `counter`, and stdout
/// capture rewires fd 1 for the whole process.
pub fn guard() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ------------------------------------------------------------ stdout capture

/// Run `f` with the process' file descriptor 1 redirected to a temporary file,
/// then return `f`'s value together with the exact bytes it wrote.
///
/// `fflush(NULL)` is issued on both sides of the redirect because the C stdout
/// stream is fully buffered once it points at a file.
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "charinbuf-cap-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));

    let _ = std::io::stdout().flush();
    let file = std::fs::File::create(&tmp).expect("create capture file");

    unsafe { fflush(std::ptr::null_mut()) };
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let out = f();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };
    drop(file);

    let mut bytes = Vec::new();
    std::fs::File::open(&tmp)
        .expect("reopen capture file")
        .read_to_end(&mut bytes)
        .expect("read capture file");
    let _ = std::fs::remove_file(&tmp);
    (out, bytes)
}

/// `charinbuf` on both libraries with identical arguments; asserts the return
/// value *and* the stdout bytes match.
pub fn diff_charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int, ctx: &str) {
    let p = pair();
    let (rc, oc) = capture(|| unsafe { (p.c.charinbuf)(mode, value, opt1, opt2) });
    let (rr, or) = capture(|| unsafe { (p.rs.charinbuf)(mode, value, opt1, opt2) });
    // Sanity: every switch arm prints at least a banner line, so an empty
    // capture would mean the redirect harness is broken and the byte comparison
    // below would be vacuous.
    assert!(
        !oc.is_empty(),
        "[{ctx}] stdout capture produced nothing for C charinbuf({mode},..); \
         the fd-1 redirect is broken"
    );
    assert!(
        oc.ends_with(b"\n"),
        "[{ctx}] C stdout was truncated (no trailing newline): {:?}",
        String::from_utf8_lossy(&oc)
    );
    assert_eq!(
        rc, rr,
        "[{ctx}] charinbuf({mode},{value},{opt1},{opt2}) return: C={rc} Rust={rr}\n\
         C stdout: {:?}\nRust stdout: {:?}",
        String::from_utf8_lossy(&oc),
        String::from_utf8_lossy(&or)
    );
    assert_eq!(
        oc,
        or,
        "[{ctx}] charinbuf({mode},{value},{opt1},{opt2}) stdout differs\n\
         C   ({} bytes): {:?}\nRust ({} bytes): {:?}",
        oc.len(),
        String::from_utf8_lossy(&oc),
        or.len(),
        String::from_utf8_lossy(&or)
    );
}

// ----------------------------------------------------------------------- misc

/// Copy `bytes` into a heap `Vec<c_char>` and append a NUL, returning it so the
/// caller controls the lifetime.
pub fn cstring(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}

/// Read a NUL-terminated string out of a pointer the library returned.
pub unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0isize;
    loop {
        let b = unsafe { *p.offset(i) } as u8;
        if b == 0 {
            break;
        }
        out.push(b);
        i += 1;
    }
    out
}

pub unsafe fn libc_free(p: *mut c_char) {
    unsafe { free(p as *mut c_void) }
}

// ------------------------------------------------------------------- test RNG

/// SplitMix64 — deterministic, no external crate, fixed seed per test.
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
    pub fn i32_full(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    /// A value biased towards interesting boundaries.
    pub fn interesting_i32(&mut self) -> i32 {
        const EDGES: [i32; 14] = [
            i32::MIN,
            i32::MIN + 1,
            -65537,
            -65536,
            -65535,
            -2,
            -1,
            0,
            1,
            2,
            65534,
            65535,
            65536,
            i32::MAX,
        ];
        match self.below(3) {
            0 => EDGES[self.below(EDGES.len() as u64) as usize],
            1 => self.range_i32(-70000, 70000),
            _ => self.i32_full(),
        }
    }
}
