//! Differential harness shared by the Phase B / Phase C test modules.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and called
//! only through their exported `slice` symbol, so the `#[no_mangle]`/
//! `extern "C"` wrapper is part of what is under test.
//!
//! `slice` communicates through two channels:
//!   * its `int` return value, and
//!   * bytes written to `stdout` by libc `printf`/`puts`.
//!
//! Both libraries share the process's single libc `stdout`, so capturing is
//! done by temporarily `dup2`-ing fd 1 onto a temp file around each call and
//! `fflush(NULL)`-ing before and after. That means calls must be serialized:
//! every public helper takes `CAPTURE_LOCK`.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type SliceFn = unsafe extern "C" fn(*mut c_char, *mut c_int, *mut c_int) -> c_int;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Serializes fd-1 redirection across tests.
///
/// libtest's own progress lines ("test foo ... ok") are written to fd 1 by the
/// harness thread. If tests ran in parallel, those writes would land inside
/// another test's capture file and produce spurious diffs, so the whole suite
/// MUST run single-threaded. That is enforced here rather than left to chance.
pub fn capture_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    static CHECK: OnceLock<()> = OnceLock::new();
    CHECK.get_or_init(|| {
        let v = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
        assert_eq!(
            v, "1",
            "These differential tests redirect the process-wide stdout fd, so they \
             must run single-threaded.\nRun them via ./verify.sh, or with:\n  \
             RUST_TEST_THREADS=1 cargo test -- --test-threads=1"
        );
    });
    match L.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libString_Slice.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// The Rust `.so` produced by this crate.
///
/// IMPORTANT: `cargo test` does **not** rebuild a `cdylib` artifact — it only
/// builds the test harnesses. A stale `.so` would make the whole differential
/// suite pass vacuously, so the path is resolved explicitly and its mtime is
/// checked against the sources. Set `SLICE_RUST_SO` to pin a specific file
/// (`verify.sh` uses this to run the suite against both debug and release).
pub fn rust_so_path() -> PathBuf {
    let p = if let Ok(v) = std::env::var("SLICE_RUST_SO") {
        PathBuf::from(v)
    } else {
        let base = manifest_dir().join("target");
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        for profile in ["debug", "release"] {
            let c = base.join(profile).join("libString_Slice.so");
            if let Ok(t) = std::fs::metadata(&c).and_then(|m| m.modified()) {
                if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                    best = Some((t, c));
                }
            }
        }
        match best {
            Some((_, p)) => p,
            None => panic!(
                "Rust shared library not found under {}. Build it with `cargo build`.",
                base.display()
            ),
        }
    };

    assert!(
        p.exists(),
        "Rust shared library not found at {}. Run `cargo build` (cargo test alone does \
         NOT rebuild a cdylib).",
        p.display()
    );

    // Staleness guard.
    let so_t = std::fs::metadata(&p)
        .and_then(|m| m.modified())
        .expect("stat the Rust .so");
    for src in ["src/lib.rs", "Cargo.toml"] {
        let sp = manifest_dir().join(src);
        if let Ok(st) = std::fs::metadata(&sp).and_then(|m| m.modified()) {
            assert!(
                so_t >= st,
                "{} is STALE: it is older than {}. `cargo test` does not rebuild a \
                 cdylib — run `cargo build` (or ./verify.sh) first, otherwise this suite \
                 would test an old library and pass vacuously.",
                p.display(),
                sp.display()
            );
        }
    }
    p
}

pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c_slice: SliceFn,
    pub rust_slice: SliceFn,
}

/// Loads both `.so`s once and resolves `slice` from each.
pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("dlopen C .so");
        let r = Library::new(rust_so_path()).expect("dlopen Rust .so");
        let cs: Symbol<SliceFn> = c.get(b"slice\0").expect("C .so must export `slice`");
        let rs: Symbol<SliceFn> = r.get(b"slice\0").expect("Rust .so must export `slice`");
        let c_slice = *cs;
        let rust_slice = *rs;
        Libs {
            _c: c,
            _rust: r,
            c_slice,
            rust_slice,
        }
    })
}

/// Runs `f` with fd 1 redirected to a temp file and returns the bytes written.
fn with_captured_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "slice_diff_{}_{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    let fd = file.as_raw_fd();

    unsafe {
        fflush(std::ptr::null_mut());
    }
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(fd, 1) } >= 0, "dup2 onto fd 1 failed");

    let out = f();

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    drop(file);

    let mut buf = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut buf)
        .expect("read capture file");
    let _ = std::fs::remove_file(&path);
    (out, buf)
}

/// One observation of a `slice` call: return value + stdout bytes.
#[derive(PartialEq, Eq)]
pub struct Obs {
    pub ret: c_int,
    pub out: Vec<u8>,
}

impl std::fmt::Debug for Obs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Obs {{ ret: {}, out: {:?} (hex {}) }}",
            self.ret,
            String::from_utf8_lossy(&self.out),
            self.out
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join("")
        )
    }
}

/// Description of one `slice` invocation. `s`/`e` are `None` to pass a NULL
/// pointer, `Some(v)` to pass a pointer to `v`.
#[derive(Clone, Copy, Debug)]
pub struct Call<'a> {
    pub bytes: &'a [u8],
    pub s: Option<c_int>,
    pub e: Option<c_int>,
    /// Pass the *same* `int` object for both `start_ptr` and `stop_ptr`.
    pub alias: bool,
}

impl<'a> Call<'a> {
    pub fn new(bytes: &'a [u8], s: Option<c_int>, e: Option<c_int>) -> Self {
        Call {
            bytes,
            s,
            e,
            alias: false,
        }
    }
    pub fn aliased(bytes: &'a [u8], v: c_int) -> Self {
        Call {
            bytes,
            s: Some(v),
            e: Some(v),
            alias: true,
        }
    }
}

/// Invokes `f` for the given call description, capturing stdout.
///
/// Also asserts the callee treated all three arguments as read-only, which is
/// what the C does (`CONFIGS.md` rows 25 and 26).
fn invoke(f: SliceFn, call: &Call<'_>) -> Obs {
    let mut buf: Vec<u8> = Vec::with_capacity(call.bytes.len() + 1);
    buf.extend_from_slice(call.bytes);
    buf.push(0);
    let buf_before = buf.clone();

    let mut sv = call.s.unwrap_or(0);
    let mut ev = call.e.unwrap_or(0);
    let sv_before = sv;
    let ev_before = ev;

    let (ret, out) = with_captured_stdout(|| unsafe {
        if call.alias {
            let p: *mut c_int = &mut sv;
            f(buf.as_mut_ptr() as *mut c_char, p, p)
        } else {
            let sp: *mut c_int = if call.s.is_some() {
                &mut sv
            } else {
                std::ptr::null_mut()
            };
            let ep: *mut c_int = if call.e.is_some() {
                &mut ev
            } else {
                std::ptr::null_mut()
            };
            f(buf.as_mut_ptr() as *mut c_char, sp, ep)
        }
    });

    assert_eq!(buf, buf_before, "callee mutated the input string: {call:?}");
    if !call.alias {
        assert_eq!(sv, sv_before, "callee mutated *start_ptr: {call:?}");
        assert_eq!(ev, ev_before, "callee mutated *stop_ptr: {call:?}");
    }

    Obs { ret, out }
}

/// Calls C then Rust with the same input and asserts byte-identical results.
#[track_caller]
pub fn assert_same(row: &str, call: &Call<'_>) -> Obs {
    let l = libs();
    let _g = capture_lock();
    let c = invoke(l.c_slice, call);
    let r = invoke(l.rust_slice, call);
    assert_eq!(
        c,
        r,
        "[{row}] divergence for bytes(len={}) = {:?}, s={:?}, e={:?}, alias={}\n  C   : {:?}\n  Rust: {:?}",
        call.bytes.len(),
        String::from_utf8_lossy(call.bytes),
        call.s,
        call.e,
        call.alias,
        c,
        r
    );
    c
}

/// Same as [`assert_same`] but additionally asserts the observed return value.
#[track_caller]
pub fn assert_same_ret(row: &str, call: &Call<'_>, expect_ret: c_int) -> Obs {
    let o = assert_same(row, call);
    assert_eq!(
        o.ret, expect_ret,
        "[{row}] expected return {expect_ret}, got {o:?} for {call:?}"
    );
    o
}

/// Runs a whole sequence of calls under a single stdout capture, so that libc
/// buffering and message ordering across calls is compared too
/// (`CONFIGS.md` row 24).
#[track_caller]
pub fn assert_same_sequence(row: &str, calls: &[Call<'_>]) {
    let l = libs();
    let _g = capture_lock();

    let run = |f: SliceFn| -> (Vec<c_int>, Vec<u8>) {
        with_captured_stdout(|| {
            let mut rets = Vec::with_capacity(calls.len());
            for c in calls {
                let mut buf: Vec<u8> = Vec::with_capacity(c.bytes.len() + 1);
                buf.extend_from_slice(c.bytes);
                buf.push(0);
                let mut sv = c.s.unwrap_or(0);
                let mut ev = c.e.unwrap_or(0);
                unsafe {
                    let r = if c.alias {
                        let p: *mut c_int = &mut sv;
                        f(buf.as_mut_ptr() as *mut c_char, p, p)
                    } else {
                        let sp: *mut c_int = if c.s.is_some() {
                            &mut sv
                        } else {
                            std::ptr::null_mut()
                        };
                        let ep: *mut c_int = if c.e.is_some() {
                            &mut ev
                        } else {
                            std::ptr::null_mut()
                        };
                        f(buf.as_mut_ptr() as *mut c_char, sp, ep)
                    };
                    rets.push(r);
                }
            }
            rets
        })
    };

    let (cr, co) = run(l.c_slice);
    let (rr, ro) = run(l.rust_slice);
    assert_eq!(cr, rr, "[{row}] return-code sequence diverged");
    assert_eq!(
        String::from_utf8_lossy(&co),
        String::from_utf8_lossy(&ro),
        "[{row}] stdout byte stream diverged over the call sequence"
    );
    assert_eq!(co, ro, "[{row}] stdout byte stream diverged");
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) so every failure is reproducible.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5DEE_CE66_D15E_A5E5;

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
    /// Uniform-ish in `[0, n)`; `n == 0` yields 0.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }
    pub fn range_incl(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.below(hi - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
}

/// Byte-content generators — every alphabet the C's `%.*s` copy must survive.
/// No generator emits `0x00`, since that would truncate the C string.
#[derive(Clone, Copy, Debug)]
pub enum Alpha {
    /// Printable ASCII 0x20..=0x7E.
    PrintableAscii,
    /// High, non-UTF-8 bytes 0x80..=0xFF.
    HighBytes,
    /// Control bytes 0x01..=0x1F (includes `\n`, `\r`, `\t`).
    Control,
    /// Printable ASCII heavily seeded with printf format specifiers.
    FormatSpecifiers,
    /// Any non-zero byte 0x01..=0xFF.
    AnyNonZero,
}

impl Alpha {
    pub fn make(self, rng: &mut Rng, len: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(len);
        match self {
            Alpha::PrintableAscii => {
                for _ in 0..len {
                    v.push(0x20 + (rng.below(0x5F) as u8));
                }
            }
            Alpha::HighBytes => {
                for _ in 0..len {
                    v.push(0x80 + (rng.below(0x80) as u8));
                }
            }
            Alpha::Control => {
                for _ in 0..len {
                    v.push(0x01 + (rng.below(0x1F) as u8));
                }
            }
            Alpha::FormatSpecifiers => {
                const TOKENS: [&[u8]; 8] = [
                    b"%", b"%s", b"%n", b"%%", b"%d", b"%.*s", b"%p", b"%1000000d",
                ];
                while v.len() < len {
                    if rng.below(3) == 0 {
                        v.extend_from_slice(TOKENS[rng.below(8) as usize]);
                    } else {
                        v.push(0x21 + (rng.below(0x5E) as u8));
                    }
                }
                v.truncate(len);
                // Truncation can leave a trailing partial token; harmless, the
                // bytes are still data on both sides.
            }
            Alpha::AnyNonZero => {
                for _ in 0..len {
                    let mut b = rng.byte();
                    if b == 0 {
                        b = 1;
                    }
                    v.push(b);
                }
            }
        }
        debug_assert!(!v.contains(&0));
        v
    }
}
