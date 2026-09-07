//! Shared differential-test harness.
//!
//! Both the C reference `.so` and the Rust `.so` are loaded with `libloading`
//! and driven exclusively through their exported symbols, so the
//! `#[no_mangle] extern "C"` wrappers are part of what is under test.
//!
//! `cp_inflate` is run inside a **forked child** because the C library, when
//! built without `-DNDEBUG` (which is what a plain `cmake ..` does), calls
//! `__assert_fail()` on malformed streams and dies with `SIGABRT`; and because
//! in `Release` some malformed streams make it scribble outside its own stack
//! frame.  Isolating the call lets us tell "returned an error" apart from "the
//! C library self-destructed", and keeps one bad case from taking the whole
//! test run down.

#![allow(dead_code)]

mod deflate;
pub use deflate::*;

use std::ffi::{c_char, c_int, c_void, OsStr};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub type InflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;
pub type UnfilterFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8) -> c_int;

// ---------------------------------------------------------------------------
// Locating the shared objects
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn first_so_in(dir: &Path) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension() == Some(OsStr::new("so")))
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// The C `.so` built exactly as the task describes (`cmake ..` with no build
/// type => `assert()` is **live**).
pub fn c_so_asserts() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_ASSERTS") {
        return PathBuf::from(p);
    }
    let d = repo_root().join("c_src/build");
    first_so_in(&d).unwrap_or_else(|| {
        panic!(
            "no .so in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            d.display()
        )
    })
}

/// The same C sources built with `-DCMAKE_BUILD_TYPE=Release` (`NDEBUG`, so no
/// `assert()`s).  This is the configuration the Rust translation targets.
pub fn c_so_release() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_RELEASE") {
        return PathBuf::from(p);
    }
    let d = repo_root().join("cbuild_release");
    first_so_in(&d).unwrap_or_else(|| {
        panic!(
            "no .so in {}; build it with:\n  cmake -S c_src -B cbuild_release \
             -DCMAKE_BUILD_TYPE=Release -DCMAKE_POSITION_INDEPENDENT_CODE=ON && \
             cmake --build cbuild_release",
            d.display()
        )
    })
}

pub fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let td = repo_root().join("translation/target");
    for prof in ["release", "debug"] {
        let p = td.join(prof).join("libunfilter_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libunfilter_lib.so not found under {}; build it with `cargo build --release`",
        td.display()
    )
}

// ---------------------------------------------------------------------------
// Loaded library handle
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: &'static libloading::Library,
    pub inflate: InflateFn,
    pub unfilter: UnfilterFn,
    pub error_reason: *mut *const c_char,
    pub fixed_table: *mut u8,
    pub permutation_order: *mut u8,
    pub len_extra_bits: *mut u8,
    pub len_base: *mut u32,
    pub dist_extra_bits: *mut u8,
    pub dist_base: *mut u32,
}

unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        unsafe {
            let lib: &'static libloading::Library = Box::leak(Box::new(
                libloading::Library::new(&path)
                    .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display())),
            ));
            macro_rules! sym {
                ($t:ty, $n:expr) => {{
                    let s: libloading::Symbol<$t> = lib
                        .get($n)
                        .unwrap_or_else(|e| panic!("{} missing {:?}: {e}", path.display(), $n));
                    *s
                }};
            }
            macro_rules! data {
                ($t:ty, $n:expr) => {{
                    let s: libloading::Symbol<*mut $t> = lib
                        .get($n)
                        .unwrap_or_else(|e| panic!("{} missing {:?}: {e}", path.display(), $n));
                    s.into_raw().into_raw() as *mut $t
                }};
            }
            Lib {
                name,
                _lib: lib,
                inflate: sym!(InflateFn, b"cp_inflate\0"),
                unfilter: sym!(UnfilterFn, b"unfilter\0"),
                error_reason: data!(*const c_char, b"cp_error_reason\0"),
                fixed_table: data!(u8, b"cp_fixed_table\0"),
                permutation_order: data!(u8, b"cp_permutation_order\0"),
                len_extra_bits: data!(u8, b"cp_len_extra_bits\0"),
                len_base: data!(u32, b"cp_len_base\0"),
                dist_extra_bits: data!(u8, b"cp_dist_extra_bits\0"),
                dist_base: data!(u32, b"cp_dist_base\0"),
                path,
            }
        }
    }

    pub fn err_string(&self) -> Option<Vec<u8>> {
        unsafe {
            let p = *self.error_reason;
            if p.is_null() {
                None
            } else {
                Some(std::ffi::CStr::from_ptr(p).to_bytes().to_vec())
            }
        }
    }
}

pub struct Pair {
    pub c: &'static Lib,
    pub rust: &'static Lib,
}

/// C built with live `assert()`s (the task's build command) + Rust.
pub fn pair_asserts() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Box::leak(Box::new(Lib::open("C(assert)", c_so_asserts()))),
        rust: Box::leak(Box::new(Lib::open("Rust", rust_so()))),
    })
}

/// C built with `NDEBUG` + Rust.
pub fn pair_release() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Box::leak(Box::new(Lib::open("C(release)", c_so_release()))),
        rust: Box::leak(Box::new(Lib::open("Rust", rust_so()))),
    })
}

// ---------------------------------------------------------------------------
// unfilter: called in-process (no assert()s and no allocation on that path)
// ---------------------------------------------------------------------------

pub const FILL: u8 = 0xAA;

/// Runs `unfilter` on both libraries over identical copies of `raw` and
/// asserts the return value *and* the whole buffer match byte for byte.
#[track_caller]
pub fn diff_unfilter(pair: &Pair, label: &str, w: i32, h: i32, bpp: i32, raw: &[u8]) {
    diff_unfilter_at(pair, label, w, h, bpp, raw, 0)
}

/// Same, but `raw + offset` is handed to `unfilter` (so tests whose
/// `len = w*bpp` is negative can let the C code walk *backwards* without
/// leaving the buffer).  The whole buffer is still compared.
#[track_caller]
pub fn diff_unfilter_at(
    pair: &Pair,
    label: &str,
    w: i32,
    h: i32,
    bpp: i32,
    raw: &[u8],
    offset: usize,
) {
    let mut a = raw.to_vec();
    let mut b = raw.to_vec();
    let (ra, ea) = unsafe {
        *pair.c.error_reason = std::ptr::null();
        let r = (pair.c.unfilter)(w, h, bpp, a.as_mut_ptr().add(offset));
        (r, pair.c.err_string())
    };
    let (rb, eb) = unsafe {
        *pair.rust.error_reason = std::ptr::null();
        let r = (pair.rust.unfilter)(w, h, bpp, b.as_mut_ptr().add(offset));
        (r, pair.rust.err_string())
    };
    assert_eq!(
        ra, rb,
        "[{label}] unfilter(w={w},h={h},bpp={bpp}) return value: {}={ra} {}={rb}",
        pair.c.name, pair.rust.name
    );
    assert_eq!(ea, eb, "[{label}] cp_error_reason diverged");
    if a != b {
        let i = a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap();
        panic!(
            "[{label}] unfilter(w={w},h={h},bpp={bpp}) buffer diverged at byte {i}: \
             C=0x{:02x} Rust=0x{:02x}\n  in : {:02x?}\n  C  : {:02x?}\n  Rust: {:02x?}",
            a[i],
            b[i],
            &raw[i.saturating_sub(8)..(i + 8).min(raw.len())],
            &a[i.saturating_sub(8)..(i + 8).min(a.len())],
            &b[i.saturating_sub(8)..(i + 8).min(b.len())],
        );
    }
}

// ---------------------------------------------------------------------------
// cp_inflate: fork-isolated batch runner
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Case {
    pub label: String,
    pub input: Vec<u8>,
    /// `in_bytes` actually handed to the callee (may lie about `input.len()`).
    pub in_bytes: i32,
    /// `out_bytes` actually handed to the callee (may be 0 or negative).
    pub out_bytes: i32,
    /// bytes really mapped for the out buffer (>= `max(out_bytes,0)`).
    pub out_cap: usize,
    /// how far past a 4-aligned address the `in` pointer is placed (0..=3).
    pub misalign: usize,
    /// pass a NULL `in` pointer instead of the arena slot.
    pub null_in: bool,
    /// pass a NULL `out` pointer instead of the arena slot.
    pub null_out: bool,
}

impl Case {
    pub fn new(label: impl Into<String>, input: Vec<u8>, out_bytes: i32) -> Case {
        let in_bytes = input.len() as i32;
        Case {
            label: label.into(),
            input,
            in_bytes,
            out_bytes,
            out_cap: out_bytes.max(0) as usize + 32,
            misalign: 0,
            null_in: false,
            null_out: false,
        }
    }
    pub fn misaligned(mut self, m: usize) -> Case {
        self.misalign = m;
        self
    }
    pub fn in_bytes(mut self, n: i32) -> Case {
        self.in_bytes = n;
        self
    }
    pub fn out_cap(mut self, n: usize) -> Case {
        self.out_cap = n;
        self
    }
    pub fn null_in(mut self) -> Case {
        self.null_in = true;
        self
    }
    pub fn null_out(mut self) -> Case {
        self.null_out = true;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseResult {
    /// false => the callee died (signal / non-zero exit) before finishing.
    pub completed: bool,
    /// raw `waitpid` status of the child that was running this case (only
    /// meaningful when `completed == false`).
    pub status: i32,
    pub ret: i32,
    pub err: Option<Vec<u8>>,
    pub out: Vec<u8>,
}

impl CaseResult {
    pub fn err_str(&self) -> String {
        match &self.err {
            None => "(null)".to_string(),
            Some(v) => String::from_utf8_lossy(v).into_owned(),
        }
    }
    pub fn signal(&self) -> Option<i32> {
        if self.completed {
            return None;
        }
        if libc::WIFSIGNALED(self.status) {
            Some(libc::WTERMSIG(self.status))
        } else {
            None
        }
    }
    pub fn describe(&self) -> String {
        if self.completed {
            format!("ret={} err={}", self.ret, self.err_str())
        } else {
            match self.signal() {
                Some(s) => format!("killed by signal {s}"),
                None => format!("exited abnormally, status={:#x}", self.status),
            }
        }
    }
}

const ERRBUF: usize = 512;
const HDR: usize = 64; // per-case header: done, ret, err_len, padding

fn align_up(x: usize, a: usize) -> usize {
    (x + a - 1) & !(a - 1)
}

struct Arena {
    base: *mut u8,
    size: usize,
    /// per case: (hdr_off, in_off, out_off)
    slots: Vec<(usize, usize, usize)>,
}

impl Arena {
    fn build(cases: &[Case]) -> Arena {
        let page = 4096usize;
        let mut off = page;
        let mut slots = Vec::with_capacity(cases.len());
        for c in cases {
            let hdr = off;
            off = align_up(off + HDR + ERRBUF, 16);
            // 4-align, then add the requested misalignment
            let inb = align_up(off, 16) + c.misalign;
            off = align_up(inb + c.input.len() + 16, 16);
            let outb = align_up(off, 16);
            off = align_up(outb + c.out_cap + 16, 16);
            slots.push((hdr, inb, outb));
        }
        let size = align_up(off + page, page);
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(base, libc::MAP_FAILED, "mmap({size}) failed");
        let base = base as *mut u8;
        unsafe {
            for (i, c) in cases.iter().enumerate() {
                let (hdr, inb, outb) = slots[i];
                std::ptr::write_bytes(base.add(hdr), 0, HDR + ERRBUF);
                std::ptr::copy_nonoverlapping(c.input.as_ptr(), base.add(inb), c.input.len());
                std::ptr::write_bytes(base.add(outb), FILL, c.out_cap);
            }
        }
        Arena { base, size, slots }
    }

    #[inline]
    unsafe fn done(&self, i: usize) -> *mut i32 {
        self.base.add(self.slots[i].0) as *mut i32
    }
    #[inline]
    unsafe fn ret(&self, i: usize) -> *mut i32 {
        self.base.add(self.slots[i].0 + 4) as *mut i32
    }
    #[inline]
    unsafe fn errlen(&self, i: usize) -> *mut i32 {
        self.base.add(self.slots[i].0 + 8) as *mut i32
    }
    #[inline]
    unsafe fn errbuf(&self, i: usize) -> *mut u8 {
        self.base.add(self.slots[i].0 + HDR)
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.base as *mut c_void, self.size);
        }
    }
}

fn fork_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Runs every case through `lib`'s `cp_inflate`, each in a forked child.  A
/// single child processes as many cases as it survives; when it dies the case
/// it was on is marked `completed: false` and a fresh child resumes after it.
pub fn run_inflate_batch(lib: &Lib, cases: &[Case]) -> Vec<CaseResult> {
    if cases.is_empty() {
        return Vec::new();
    }
    let arena = Arena::build(cases);
    let _guard = fork_lock().lock().unwrap();

    let mut start = 0usize;
    let mut statuses = vec![0i32; cases.len()];
    while start < cases.len() {
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // ---- child ----
            unsafe {
                libc::alarm(30); // backstop against a wedged child
                for i in start..cases.len() {
                    let c = &cases[i];
                    let (_, inb, outb) = arena.slots[i];
                    let inp = if c.null_in {
                        std::ptr::null_mut()
                    } else {
                        arena.base.add(inb) as *mut c_void
                    };
                    let outp = if c.null_out {
                        std::ptr::null_mut()
                    } else {
                        arena.base.add(outb) as *mut c_void
                    };
                    *lib.error_reason = std::ptr::null();
                    let r = (lib.inflate)(inp, c.in_bytes, outp, c.out_bytes);
                    *arena.ret(i) = r;
                    let e = *lib.error_reason;
                    if e.is_null() {
                        *arena.errlen(i) = -1;
                    } else {
                        let mut n = 0usize;
                        while n < ERRBUF - 1 && *e.add(n) != 0 {
                            n += 1;
                        }
                        std::ptr::copy_nonoverlapping(e as *const u8, arena.errbuf(i), n);
                        *arena.errlen(i) = n as i32;
                    }
                    // The parent only reads these after waitpid(), which is a
                    // full synchronisation point, so plain stores suffice.
                    *arena.done(i) = 1;
                }
                libc::_exit(0);
            }
        }
        // ---- parent ----
        let mut status = 0i32;
        unsafe { libc::waitpid(pid, &mut status, 0) };
        let mut next = None;
        for i in start..cases.len() {
            if unsafe { *arena.done(i) } == 0 {
                next = Some(i);
                break;
            }
        }
        match next {
            None => break,
            Some(i) => {
                statuses[i] = status;
                start = i + 1;
            }
        }
    }

    (0..cases.len())
        .map(|i| unsafe {
            let done = *arena.done(i) != 0;
            let el = *arena.errlen(i);
            CaseResult {
                completed: done,
                status: statuses[i],
                ret: *arena.ret(i),
                err: if el < 0 {
                    None
                } else {
                    Some(std::slice::from_raw_parts(arena.errbuf(i), el as usize).to_vec())
                },
                out: std::slice::from_raw_parts(arena.base.add(arena.slots[i].2), cases[i].out_cap)
                    .to_vec(),
            }
        })
        .collect()
}

pub struct BatchReport {
    pub compared: usize,
    /// cases where the C library killed itself (assert / OOB) and no
    /// comparison is possible.
    pub c_died: Vec<(String, Option<i32>)>,
    /// the C results, indexed like the input `cases` slice.
    pub c: Vec<CaseResult>,
    /// the Rust results, indexed like the input `cases` slice.
    pub rust: Vec<CaseResult>,
}

impl BatchReport {
    /// Asserts that every case decoded successfully (`ret == 1`, no error) and
    /// produced exactly `expected`, with the untouched tail of the buffer still
    /// holding the `FILL` pattern.  This keeps a "both sides fail identically"
    /// result from silently passing as a happy-path row.
    #[track_caller]
    pub fn assert_all_decoded(&self, cases: &[Case], expected: &[Vec<u8>]) {
        assert!(
            self.c_died.is_empty(),
            "C library died on cases that should be valid: {:?}",
            self.c_died
        );
        for (i, c) in cases.iter().enumerate() {
            let r = &self.c[i];
            assert_eq!(
                r.ret, 1,
                "[{}] expected cp_inflate == 1, got {} (err={:?})",
                c.label,
                r.ret,
                r.err_str()
            );
            assert_eq!(r.err, None, "[{}] unexpected cp_error_reason", c.label);
            let e = &expected[i];
            assert_eq!(
                &r.out[..e.len()],
                &e[..],
                "[{}] decoded payload differs from the expected plaintext",
                c.label
            );
            assert!(
                r.out[e.len()..].iter().all(|&b| b == FILL),
                "[{}] cp_inflate wrote past the expected {} bytes",
                c.label,
                e.len()
            );
        }
    }
}

/// Runs `cases` through both libraries and asserts full agreement on every
/// case that the C library survives.
#[track_caller]
pub fn diff_inflate_batch(pair: &Pair, cases: &[Case]) -> BatchReport {
    let rc = run_inflate_batch(pair.c, cases);
    let rr = run_inflate_batch(pair.rust, cases);
    let mut rep = BatchReport {
        compared: 0,
        c_died: Vec::new(),
        c: rc.clone(),
        rust: rr.clone(),
    };
    for (i, c) in cases.iter().enumerate() {
        let (a, b) = (&rc[i], &rr[i]);
        if !a.completed {
            rep.c_died.push((c.label.clone(), a.signal()));
            continue;
        }
        assert!(
            b.completed,
            "[{}] C survived ({}) but Rust died ({})",
            c.label,
            a.describe(),
            b.describe()
        );
        assert_eq!(
            a.ret, b.ret,
            "[{}] cp_inflate return value: C={} Rust={} (C err={:?}, Rust err={:?})",
            c.label,
            a.ret,
            b.ret,
            a.err_str(),
            b.err_str()
        );
        assert_eq!(
            a.err,
            b.err,
            "[{}] cp_error_reason: C={:?} Rust={:?}",
            c.label,
            a.err_str(),
            b.err_str()
        );
        if a.out != b.out {
            let j = a
                .out
                .iter()
                .zip(b.out.iter())
                .position(|(x, y)| x != y)
                .unwrap();
            let lo = j.saturating_sub(8);
            let hi = (j + 8).min(a.out.len());
            panic!(
                "[{}] out buffer diverged at byte {j} of {} (ret={}, err={:?})\n  \
                 C   : {:02x?}\n  Rust: {:02x?}",
                c.label,
                a.out.len(),
                a.ret,
                a.err_str(),
                &a.out[lo..hi],
                &b.out[lo..hi]
            );
        }
        rep.compared += 1;
    }
    rep
}
