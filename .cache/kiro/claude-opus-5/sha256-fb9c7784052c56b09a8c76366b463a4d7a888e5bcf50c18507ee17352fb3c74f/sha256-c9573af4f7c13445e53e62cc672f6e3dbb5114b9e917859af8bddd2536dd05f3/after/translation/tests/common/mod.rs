//! Shared differential-test harness.
//!
//! Both the C `libdriver.so` and the Rust `libdriver.so` are loaded with
//! `libloading` and driven **only** through their exported `driver` symbol, so
//! the `#[no_mangle] extern "C"` wrapper is under test just like the C ABI is.
//!
//! `driver` has no return value: its entire observable behaviour is what it
//! writes to `stdout` via libc `printf`. Both shared objects resolve `printf`
//! against the *same* glibc in this process, hence the same `stdout` FILE. We
//! capture by temporarily `dup2`-ing fd 1 onto a scratch file, with
//! `fflush(NULL)` on both sides of the call so nothing leaks between captures.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CString};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits we need for capture + locale manipulation
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn newlocale(mask: c_int, locale: *const c_char, base: *mut c_void) -> *mut c_void;
    fn uselocale(newloc: *mut c_void) -> *mut c_void;
    fn freelocale(loc: *mut c_void);
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    static mut stdout: *mut c_void;
}

/// glibc `<stdio.h>` buffering modes.
pub const IOFBF: c_int = 0;
pub const IOLBF: c_int = 1;
pub const IONBF: c_int = 2;

/// Set libc `stdout`'s buffering mode, flushing first so the change is legal.
pub fn set_stdout_buffering(mode: c_int) {
    unsafe {
        fflush(stdout);
        let rc = setvbuf(stdout, std::ptr::null_mut(), mode, 0);
        assert_eq!(rc, 0, "setvbuf(mode={mode}) failed");
    }
}

pub const LC_ALL: c_int = 6;
/// glibc's `LC_ALL_MASK`: every `1 << __LC_*` bit **except** bit 6 (`__LC_ALL`
/// itself), i.e. `0x1fbf`. Passing `0x1fff` makes `newlocale` fail with EINVAL.
pub const LC_ALL_MASK: c_int = 0x1fbf;
/// glibc's `LC_GLOBAL_LOCALE` == `(locale_t) -1L`.
pub fn lc_global_locale() -> *mut c_void {
    usize::MAX as *mut c_void
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub rust: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn locate(env_key: &str, candidates: &[PathBuf], what: &str) -> PathBuf {
    if let Ok(p) = std::env::var(env_key) {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "{env_key}={} does not exist", p.display());
        return p;
    }
    for c in candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "could not locate the {what} shared object; tried {:?}. \
         Build it first (see README / build_and_test.sh) or set ${env_key}.",
        candidates
    );
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let md = manifest_dir();
        let c_path = locate(
            "DRIVER_C_SO",
            &[
                md.join("../c_src/build/libdriver.so"),
                md.join("../c_src/build/lib/libdriver.so"),
            ],
            "C",
        );
        let rust_path = locate(
            "DRIVER_RUST_SO",
            &[
                md.join("target/release/libdriver.so"),
                md.join("target/debug/libdriver.so"),
            ],
            "Rust",
        );
        // SAFETY: both objects are plain C-ABI libraries with no init side
        // effects beyond the usual .init_array.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
        Libs {
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

pub type DriverChar = unsafe extern "C" fn(c_char);
/// The very same exported symbol, but declared as taking a full-width `int`.
///
/// This is how an out-of-range value gets across the FFI boundary in practice:
/// the x86-64 SysV ABI hands sub-`int` arguments over in a 32-bit register slot
/// and the callee may not rely on the upper bits. Calling through this type
/// lets us hand `driver` a value that is *not* representable in a `char` and
/// check that C and Rust narrow it identically.
pub type DriverInt = unsafe extern "C" fn(c_int);

pub fn c_driver() -> Symbol<'static, DriverChar> {
    unsafe { libs().c.get(b"driver\0").expect("C .so exports `driver`") }
}
pub fn rust_driver() -> Symbol<'static, DriverChar> {
    unsafe {
        libs()
            .rust
            .get(b"driver\0")
            .expect("Rust .so exports `driver`")
    }
}
pub fn c_driver_int() -> Symbol<'static, DriverInt> {
    unsafe { libs().c.get(b"driver\0").unwrap() }
}
pub fn rust_driver_int() -> Symbol<'static, DriverInt> {
    unsafe { libs().rust.get(b"driver\0").unwrap() }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 1 is process-wide state, so captures must not overlap. `cargo test` runs
/// tests on parallel threads by default, hence the global lock.
///
/// The lock is necessary but not sufficient: libtest itself writes its
/// `test foo ... ok` progress lines to fd 1 through a `LineWriter`, so a
/// *different* test thread can flush into our redirected fd. Always run these
/// suites with `--test-threads=1` (`build_and_test.sh` does). `capture`
/// additionally drains Rust's own stdout buffer before redirecting, and
/// `check_shape` below catches any residual contamination instead of silently
/// reporting it as a C/Rust divergence.
static FD_LOCK: Mutex<()> = Mutex::new(());
static SEQ: AtomicU64 = AtomicU64::new(0);

pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = std::env::temp_dir().join(format!(
        "driver_cap_{}_{}.bin",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let file = std::fs::File::create(&path).expect("create scratch file");
    let scratch = file.as_raw_fd();

    let saved = unsafe {
        // Drain whatever the harness itself has buffered so it does not land
        // in our capture: Rust's std buffer first, then all libc FILE streams.
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
        }
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(scratch, 1) >= 0, "dup2 onto fd 1 failed");
        saved
    };

    // Run with fd 1 pointing at the scratch file. Any panic here would leave
    // fd 1 redirected, so restore it before letting the unwind continue.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore fd 1 failed");
        close(saved);
    }
    drop(file);
    let out = std::fs::read(&path).expect("read scratch file");
    let _ = std::fs::remove_file(&path);
    drop(guard);

    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }
    out
}

/// Like [`capture`], but reports the scratch file's contents **twice**: once
/// before any `fflush`, and once after. `driver` itself never calls `fflush`, so
/// the first snapshot shows how much of the output the callee pushed out on its
/// own — a property both implementations must share, and one a translation built
/// on Rust's `std::io` (a separate buffer) would get wrong.
pub fn capture_two_stage<F: FnOnce()>(f: F) -> (Vec<u8>, Vec<u8>) {
    let guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = std::env::temp_dir().join(format!(
        "driver_cap2_{}_{}.bin",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let file = std::fs::File::create(&path).expect("create scratch file");
    let scratch = file.as_raw_fd();

    let saved = unsafe {
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
        }
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0);
        assert!(dup2(scratch, 1) >= 0);
        saved
    };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    // Snapshot before flushing, while fd 1 is still the scratch file.
    let before = std::fs::read(&path).unwrap_or_default();
    unsafe {
        // Still redirected, so the drain lands in the scratch file, not on the
        // real stdout.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0);
        close(saved);
    }
    let after = std::fs::read(&path).expect("read scratch file");
    drop(file);
    let _ = std::fs::remove_file(&path);
    drop(guard);

    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }
    (before, after)
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

pub fn render(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            0x20..=0x7E => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// Structural invariant of `driver`'s output, held by *both* implementations:
/// exactly the 14 `printf` lines of `driver.c`, in order, and nothing else.
/// The first twelve are `%d` (a decimal ctype mask), the last two are `%c` (one
/// arbitrary byte, possibly NUL or a newline).
///
/// A capture that breaks this parse means libtest's own progress output leaked
/// into the redirected fd, which would otherwise masquerade as a C/Rust
/// divergence. This is a harness-integrity check, not a leniency: it never
/// relaxes the byte-for-byte comparison that follows.
fn check_shape(who: &str, bytes: &[u8], ctx: &str) {
    const LABELS: [&str; 14] = [
        "alphanumeric: ",
        "alphabetic: ",
        "lowercase: ",
        "uppercase: ",
        "digit: ",
        "hexadecimal: ",
        "control: ",
        "graphical: ",
        "space: ",
        "blank: ",
        "printing: ",
        "punctuation: ",
        "to lower: ",
        "to upper: ",
    ];
    let bad = |why: String| -> ! {
        panic!(
            "{who} capture for {ctx} is not `driver`'s 14-line output: {why}\n  {}\n\
             If this looks like libtest progress text spliced in, re-run with \
             `--test-threads=1`.",
            render(bytes)
        )
    };
    let mut p = 0usize;
    for (i, lab) in LABELS.iter().enumerate() {
        if !bytes[p..].starts_with(lab.as_bytes()) {
            bad(format!("expected label {lab:?} at offset {p}"));
        }
        p += lab.len();
        if i < 12 {
            let start = p;
            while p < bytes.len() && bytes[p].is_ascii_digit() {
                p += 1;
            }
            if p == start {
                bad(format!("expected decimal digits after {lab:?}"));
            }
        } else {
            // `%c` of an int: exactly one byte, any value.
            if p >= bytes.len() {
                bad(format!("truncated after {lab:?}"));
            }
            p += 1;
        }
        if bytes.get(p) != Some(&b'\n') {
            bad(format!("expected newline ending the {lab:?} line"));
        }
        p += 1;
    }
    if p != bytes.len() {
        bad(format!(
            "{} trailing byte(s) after the 14th line",
            bytes.len() - p
        ));
    }
}

/// Call both `.so`s with the same `char` and require byte-identical stdout.
#[track_caller]
pub fn diff_char(c: c_char, ctx: &str) {
    let cd = c_driver();
    let rd = rust_driver();
    let c_out = capture(|| unsafe { cd(c) });
    let r_out = capture(|| unsafe { rd(c) });
    check_shape("C", &c_out, ctx);
    check_shape("Rust", &r_out, ctx);
    assert_eq!(
        c_out,
        r_out,
        "\ndivergence for {ctx} (char {c} = 0x{:02x}):\n  C   : {}\n  Rust: {}\n",
        c as u8,
        render(&c_out),
        render(&r_out)
    );
}

/// Call both `.so`s through the `int`-typed view of `driver`.
#[track_caller]
pub fn diff_int(v: c_int, ctx: &str) {
    let cd = c_driver_int();
    let rd = rust_driver_int();
    let c_out = capture(|| unsafe { cd(v) });
    let r_out = capture(|| unsafe { rd(v) });
    check_shape("C", &c_out, ctx);
    check_shape("Rust", &r_out, ctx);
    assert_eq!(
        c_out,
        r_out,
        "\ndivergence for {ctx} (int {v} = 0x{v:08x}):\n  C   : {}\n  Rust: {}\n",
        render(&c_out),
        render(&r_out)
    );
}

/// Every `char` bit pattern, driven through the `char` entry point.
pub fn diff_all_chars(ctx: &str) {
    for v in 0u16..=255 {
        diff_char(v as u8 as c_char, ctx);
    }
}

/// Like [`diff_char`], but re-runs `prepare` immediately before **each** side.
///
/// This matters whenever the state under test is something `driver` itself
/// mutates. `driver` calls `setlocale(LC_ALL, "C")`, so in a plain
/// `diff_char` the C call — which runs first — has already reset the global
/// locale by the time the Rust call happens, and the Rust side is never
/// actually observed under the foreign locale. That ordering artefact hides
/// any bug in the Rust's own `setlocale`. Re-preparing per side removes it.
#[track_caller]
pub fn diff_char_prepared(c: c_char, ctx: &str, prepare: &dyn Fn()) {
    let cd = c_driver();
    let rd = rust_driver();
    prepare();
    let c_out = capture(|| unsafe { cd(c) });
    prepare();
    let r_out = capture(|| unsafe { rd(c) });
    check_shape("C", &c_out, ctx);
    check_shape("Rust", &r_out, ctx);
    assert_eq!(
        c_out,
        r_out,
        "\ndivergence for {ctx} (char {c} = 0x{:02x}):\n  C   : {}\n  Rust: {}\n",
        c as u8,
        render(&c_out),
        render(&r_out)
    );
}

/// [`diff_int`] with the same per-side preparation as [`diff_char_prepared`].
#[track_caller]
pub fn diff_int_prepared(v: c_int, ctx: &str, prepare: &dyn Fn()) {
    let cd = c_driver_int();
    let rd = rust_driver_int();
    prepare();
    let c_out = capture(|| unsafe { cd(v) });
    prepare();
    let r_out = capture(|| unsafe { rd(v) });
    check_shape("C", &c_out, ctx);
    check_shape("Rust", &r_out, ctx);
    assert_eq!(
        c_out,
        r_out,
        "\ndivergence for {ctx} (int {v} = 0x{v:08x}):\n  C   : {}\n  Rust: {}\n",
        render(&c_out),
        render(&r_out)
    );
}

/// All 256 `char` values, re-preparing state before each side of each pair.
pub fn diff_all_chars_prepared(ctx: &str, prepare: &dyn Fn()) {
    for v in 0u16..=255 {
        diff_char_prepared(v as u8 as c_char, ctx, prepare);
    }
}

// ---------------------------------------------------------------------------
// Locale helpers
// ---------------------------------------------------------------------------

/// Set the process-global locale, returning `false` if it is unavailable here.
pub fn set_global_locale(name: &str) -> bool {
    let cs = CString::new(name).unwrap();
    unsafe { !setlocale(LC_ALL, cs.as_ptr()).is_null() }
}

pub fn reset_global_locale() {
    assert!(set_global_locale("C"), "the \"C\" locale must always exist");
}

/// Query the process-global locale without changing it (`setlocale(cat, NULL)`).
pub fn query_global_locale(category: c_int) -> String {
    unsafe {
        let p = setlocale(category, std::ptr::null());
        assert!(!p.is_null(), "setlocale query returned NULL");
        std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

/// `LC_CTYPE` from glibc's <locale.h>.
pub const LC_CTYPE: c_int = 0;

/// Install a thread-local locale. Returns the handle plus the previous locale,
/// or `None` if the named locale is unavailable.
pub fn push_thread_locale(name: &str) -> Option<(*mut c_void, *mut c_void)> {
    let cs = CString::new(name).unwrap();
    unsafe {
        let loc = newlocale(LC_ALL_MASK, cs.as_ptr(), std::ptr::null_mut());
        if loc.is_null() {
            return None;
        }
        let prev = uselocale(loc);
        Some((loc, prev))
    }
}

pub fn pop_thread_locale(handle: (*mut c_void, *mut c_void)) {
    unsafe {
        let (loc, prev) = handle;
        let prev = if prev.is_null() {
            lc_global_locale()
        } else {
            prev
        };
        uselocale(prev);
        freelocale(loc);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible property-style testing)
// ---------------------------------------------------------------------------

/// SplitMix64. Self-contained so no dev-dependency beyond `libloading`.
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
    /// Uniform in `lo..=hi`.
    pub fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u8
    }
}

/// The seed every property-style test uses, so failures reproduce exactly.
pub const SEED: u64 = 0x5EED_1234_ABCD_0001;
/// Randomized samples drawn per `CONFIGS.md` row.
pub const SAMPLES: usize = 64;

/// Draw `SAMPLES` values uniformly from an inclusive byte range and diff each.
pub fn diff_random_in_range(lo: u8, hi: u8, seed_tweak: u64, ctx: &str) {
    let mut rng = Rng::new(SEED ^ seed_tweak);
    // Always pin the two endpoints, then sample the interior.
    diff_char(lo as c_char, &format!("{ctx} [lo endpoint]"));
    diff_char(hi as c_char, &format!("{ctx} [hi endpoint]"));
    for i in 0..SAMPLES {
        let v = rng.range_u8(lo, hi);
        diff_char(v as c_char, &format!("{ctx} [sample {i}]"));
    }
}
