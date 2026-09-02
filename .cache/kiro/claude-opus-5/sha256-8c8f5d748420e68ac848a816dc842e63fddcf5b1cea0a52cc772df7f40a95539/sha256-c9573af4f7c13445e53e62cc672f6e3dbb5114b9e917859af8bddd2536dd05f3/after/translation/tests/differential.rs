//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and compares their observable behaviour byte-for-byte.
//!
//! Neither implementation is ever called directly as a Rust function — every
//! call goes through `dlsym` on a `dlopen`ed object, exactly as an external
//! consumer would, so the `#[no_mangle]` / `extern "C"` export wrappers are
//! themselves under test.
//!
//! The library's entire observable output is bytes written to `stdout`, so the
//! harness captures file descriptor 1 around each call.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs (not part of the library under test)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Flush every stdio stream in the process. Both the C `.so` and the Rust `.so`
/// write through the *same* glibc `stdout`, so this is what makes captured
/// output complete and comparable.
fn flush_all() {
    unsafe {
        fflush(ptr::null_mut());
    }
}

// ---------------------------------------------------------------------------
// The two implementations under test
// ---------------------------------------------------------------------------

type FnPrintLine = unsafe extern "C" fn(*const c_char);
type FnVoid = unsafe extern "C" fn();

/// The four exported entry points of one implementation, as raw code pointers.
#[derive(Clone, Copy)]
struct Api {
    which: &'static str,
    printLine: FnPrintLine,
    bad: FnVoid,
    good: FnVoid,
    driver: FnVoid,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(explicit) = std::env::var("RUST_DRIVER_SO") {
        let p = PathBuf::from(explicit);
        assert!(p.exists(), "RUST_DRIVER_SO points at a missing file: {}", p.display());
        return p;
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let release = manifest.join("target/release/libdriver.so");
    let debug = manifest.join("target/debug/libdriver.so");
    let chosen = if release.exists() {
        release
    } else if debug.exists() {
        debug
    } else {
        panic!(
            "Rust cdylib not found. Run `cargo build --release` in translation/ first \
             (looked for target/release/libdriver.so and target/debug/libdriver.so)."
        )
    };

    // Guard against silently verifying a stale artifact.
    let src = manifest.join("src/lib.rs");
    if let (Ok(so_m), Ok(src_m)) = (
        fs::metadata(&chosen).and_then(|m| m.modified()),
        fs::metadata(&src).and_then(|m| m.modified()),
    ) {
        assert!(
            so_m >= src_m,
            "{} is OLDER than src/lib.rs — rebuild with `cargo build --release` \
             before running the differential tests.",
            chosen.display()
        );
    }
    chosen
}

fn load(path: &Path, which: &'static str) -> Api {
    // Leaked so the Symbol-derived function pointers stay valid for the whole
    // process; the object must never be unloaded while we hold code pointers.
    let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
        libloading::Library::new(path).unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
    }));
    unsafe {
        let get_pl = |n: &[u8]| -> FnPrintLine {
            *lib.get::<FnPrintLine>(n)
                .unwrap_or_else(|e| panic!("{which}: dlsym({}) failed: {e}", String::from_utf8_lossy(n)))
        };
        let get_v = |n: &[u8]| -> FnVoid {
            *lib.get::<FnVoid>(n)
                .unwrap_or_else(|e| panic!("{which}: dlsym({}) failed: {e}", String::from_utf8_lossy(n)))
        };
        Api {
            which,
            printLine: get_pl(b"printLine\0"),
            bad: get_v(b"bad\0"),
            good: get_v(b"good\0"),
            driver: get_v(b"driver\0"),
        }
    }
}

static APIS: OnceLock<(Api, Api)> = OnceLock::new();

/// `(c, rust)`
fn apis() -> (Api, Api) {
    *APIS.get_or_init(|| (load(&c_so_path(), "C"), load(&rust_so_path(), "Rust")))
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 1 is process-global state, so captures must not overlap.
static FD_LOCK: Mutex<()> = Mutex::new(());

fn fd_lock() -> MutexGuard<'static, ()> {
    match FD_LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn tmp_path() -> PathBuf {
    let n = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("driver_diff_{}_{}.out", std::process::id(), n))
}

/// Run `f` with fd 1 redirected to a regular file (glibc: fully buffered) and
/// return every byte it produced.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _g = fd_lock();
    let path = tmp_path();
    let data = {
        let file = fs::File::create(&path).expect("create temp capture file");
        flush_all(); // don't let pre-existing buffered bytes leak into the capture
        let saved = unsafe { dup(1) };
        assert!(saved >= 0, "dup(1) failed");
        assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 onto fd 1 failed");

        f();

        flush_all(); // push the library's buffered bytes into the file
        assert!(unsafe { dup2(saved, 1) } >= 0, "restoring fd 1 failed");
        unsafe { close(saved) };
        drop(file);
        fs::read(&path).expect("read back temp capture file")
    };
    let _ = fs::remove_file(&path);
    data
}

/// Run `f` with fd 1 redirected to a pipe (glibc's other buffering regime).
/// Output must stay well under the pipe capacity (64 KiB) to avoid blocking.
fn capture_via_pipe<F: FnOnce()>(f: F) -> Vec<u8> {
    let _g = fd_lock();
    let mut fds = [0 as c_int; 2];
    assert!(unsafe { pipe(fds.as_mut_ptr()) } == 0, "pipe() failed");
    let (rd, wr) = (fds[0], fds[1]);

    flush_all();
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(wr, 1) } >= 0, "dup2 onto fd 1 failed");

    f();

    flush_all();
    assert!(unsafe { dup2(saved, 1) } >= 0, "restoring fd 1 failed");
    unsafe {
        close(saved);
        close(wr); // signal EOF to the read end
    }
    let mut out = Vec::new();
    let mut r = unsafe { fs::File::from_raw_fd(rd) };
    r.read_to_end(&mut out).expect("read pipe");
    drop(r);
    out
}

// ---------------------------------------------------------------------------
// comparison
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    const LIMIT: usize = 160;
    let mut s = String::new();
    for &c in b.iter().take(LIMIT) {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    if b.len() > LIMIT {
        s.push_str(&format!("... ({} bytes total)", b.len()));
    }
    s
}

fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    let n = a.len().min(b.len());
    (0..n).find(|&i| a[i] != b[i]).or(if a.len() == b.len() { None } else { Some(n) })
}

/// Core differential assertion: run the same closure against each `.so` in turn
/// and require identical stdout bytes. Returns the (shared) output.
fn assert_same(case: &str, run: impl Fn(&Api)) -> Vec<u8> {
    assert_same_with(case, cap_file, run)
}

fn assert_same_with<C>(case: &str, cap: C, run: impl Fn(&Api)) -> Vec<u8>
where
    C: Fn(&mut dyn FnMut()) -> Vec<u8>,
{
    let (c, r) = apis();
    let c_out = cap(&mut || run(&c));
    let r_out = cap(&mut || run(&r));
    if c_out != r_out {
        let at = first_diff(&c_out, &r_out);
        panic!(
            "DIVERGENCE [{case}]\n  first difference at byte offset: {at:?}\n  \
             {} ({} bytes): {}\n  {} ({} bytes): {}",
            c.which,
            c_out.len(),
            show(&c_out),
            r.which,
            r_out.len(),
            show(&r_out)
        );
    }
    c_out
}

// Adapters so `assert_same_with` can take either capture strategy.
#[allow(clippy::type_complexity)]
fn cap_file(f: &mut dyn FnMut()) -> Vec<u8> {
    capture(f)
}
fn cap_pipe(f: &mut dyn FnMut()) -> Vec<u8> {
    capture_via_pipe(f)
}

/// `printLine(payload)` where `payload` must contain a NUL terminator
/// somewhere (not necessarily at the end — some tests deliberately place guard
/// bytes after the terminator).
fn call_print_line(api: &Api, nul_terminated: &[u8]) {
    debug_assert!(
        nul_terminated.contains(&0),
        "payload must contain a NUL terminator"
    );
    unsafe { (api.printLine)(nul_terminated.as_ptr() as *const c_char) }
}

/// Differential check of `printLine` on one payload (given without its NUL).
fn diff_print_line(case: &str, payload: &[u8]) -> Vec<u8> {
    assert!(!payload.contains(&0), "payload must not contain an interior NUL");
    let mut buf = payload.to_vec();
    buf.push(0);
    assert_same(case, |api| call_print_line(api, &buf))
}

// ---------------------------------------------------------------------------
// deterministic RNG (SplitMix64) — fixed seeds keep failures reproducible
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `lo..=hi`.
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }
    /// Any byte except NUL (NUL would terminate the string early).
    fn nonzero_byte(&mut self) -> u8 {
        (self.next_u64() % 255) as u8 + 1
    }
    fn ascii_printable(&mut self) -> u8 {
        0x20 + (self.next_u64() % 95) as u8
    }
    fn bytes(&mut self, len: usize, mut make: impl FnMut(&mut Self) -> u8) -> Vec<u8> {
        (0..len).map(|_| make(self)).collect()
    }
}

// ===========================================================================
// Sanity gate — must hold or every other comparison is meaningless
// ===========================================================================

/// Both files are named `libdriver.so`; the C one even carries
/// `SONAME=libdriver.so`. If the loader aliased the second `dlopen` onto the
/// first object, both "implementations" would be the same code and every
/// differential test below would pass vacuously. Prove they are distinct.
#[test]
fn test_00_both_libraries_are_distinct_objects() {
    let (c, r) = apis();
    let ca = c.driver as usize;
    let ra = r.driver as usize;
    assert_ne!(
        ca, ra,
        "C and Rust `driver` resolved to the SAME address ({ca:#x}) — the dynamic loader \
         aliased the two libdriver.so objects, so the differential suite would be vacuous."
    );
    for (n, a, b) in [
        ("printLine", c.printLine as usize, r.printLine as usize),
        ("bad", c.bad as usize, r.bad as usize),
        ("good", c.good as usize, r.good as usize),
    ] {
        assert_ne!(a, b, "C and Rust `{n}` resolved to the same address");
    }
    eprintln!("C   .so: {}", c_so_path().display());
    eprintln!("Rust.so: {}", rust_so_path().display());
}

/// The harness itself must be able to observe output; a capture that always
/// returned empty would make every comparison pass.
#[test]
fn test_00_capture_harness_actually_observes_output() {
    let (c, _) = apis();
    let out = capture(|| unsafe { (c.bad)() });
    assert_eq!(out, b"bad()\n", "stdout capture harness is broken");
    let out = capture(|| {});
    assert!(out.is_empty(), "capture of a no-op should be empty");
}

// ===========================================================================
// PHASE B — valid-path differential tests, one per CONFIGS.md row
// ===========================================================================

/// CONFIGS row 1 — randomized printable ASCII, length 1..=64.
#[test]
fn test_cfg_01_random_printable_ascii() {
    let mut rng = Rng::new(0x0000_0001_C0FF_EE01);
    for i in 0..256 {
        let len = rng.range(1, 64);
        let s = rng.bytes(len, |r| r.ascii_printable());
        diff_print_line(&format!("cfg01/random printable #{i} len={len}"), &s);
    }
}

/// CONFIGS row 2 — every single-byte string, 0x01..=0xFF exhaustively.
#[test]
fn test_cfg_02_every_single_byte_value() {
    for b in 1u8..=255 {
        let out = diff_print_line(&format!("cfg02/single byte {b:#04x}"), &[b]);
        assert_eq!(out, vec![b, b'\n'], "cfg02: unexpected bytes for {b:#04x}");
    }
}

/// CONFIGS row 3 — arbitrary non-NUL bytes incl. invalid UTF-8.
#[test]
fn test_cfg_03_random_arbitrary_bytes_including_invalid_utf8() {
    let mut rng = Rng::new(0x0000_0003_5EED_0003);
    let mut saw_invalid_utf8 = false;
    for i in 0..256 {
        let len = rng.range(1, 128);
        let s = rng.bytes(len, |r| r.nonzero_byte());
        if std::str::from_utf8(&s).is_err() {
            saw_invalid_utf8 = true;
        }
        diff_print_line(&format!("cfg03/random bytes #{i} len={len}"), &s);
    }
    assert!(
        saw_invalid_utf8,
        "cfg03 should have generated invalid UTF-8 (that is the point of the row)"
    );

    // Hand-picked invalid UTF-8 sequences, to be sure rather than lucky.
    for (name, s) in [
        ("lone continuation", vec![0x80]),
        ("truncated 2-byte", vec![0xC3]),
        ("truncated 3-byte", vec![0xE2, 0x82]),
        ("truncated 4-byte", vec![0xF0, 0x9F, 0x92]),
        ("overlong", vec![0xC0, 0xAF]),
        ("surrogate", vec![0xED, 0xA0, 0x80]),
        ("0xFF 0xFE", vec![0xFF, 0xFE]),
        ("all high bytes", (0x80u8..=0xFF).collect()),
        ("valid utf8 mix", "héllo — 世界 🌍".as_bytes().to_vec()),
    ] {
        diff_print_line(&format!("cfg03/{name}"), &s);
    }
}

/// CONFIGS rows 4,5,6 — the stdio buffer boundary, exactly.
#[test]
fn test_cfg_04_05_06_stdio_buffer_boundary_lengths() {
    const BUFSIZ: usize = 4096;
    for len in [BUFSIZ - 1, BUFSIZ, BUFSIZ + 1] {
        // Non-uniform content so a length/offset bug cannot hide behind a
        // repeated byte.
        let mut rng = Rng::new(0x0000_0004_B0DE_0000 + len as u64);
        let s = rng.bytes(len, |r| r.ascii_printable());
        let out = diff_print_line(&format!("cfg04-06/exact len {len}"), &s);
        assert_eq!(out.len(), len + 1, "expected payload + newline");
        assert_eq!(out[len], b'\n');
    }
}

/// CONFIGS row 7 — randomized lengths sweeping across the buffer boundary.
#[test]
fn test_cfg_07_random_lengths_across_buffer_boundary() {
    let mut rng = Rng::new(0x0000_0007_5EED_0007);
    for i in 0..64 {
        let len = rng.range(4000, 4200);
        let s = rng.bytes(len, |r| r.nonzero_byte());
        diff_print_line(&format!("cfg07/sweep #{i} len={len}"), &s);
    }
}

/// CONFIGS row 8 — 64 KiB payload, many buffer flushes.
#[test]
fn test_cfg_08_64kib_payload() {
    let mut rng = Rng::new(0x0000_0008_5EED_0008);
    let s = rng.bytes(64 * 1024, |r| r.nonzero_byte());
    let out = diff_print_line("cfg08/64KiB", &s);
    assert_eq!(out.len(), 64 * 1024 + 1);
}

/// CONFIGS row 9 — 1 MiB payload (the "oversized length" boundary analogue).
#[test]
fn test_cfg_09_1mib_payload() {
    let mut rng = Rng::new(0x0000_0009_5EED_0009);
    let s = rng.bytes(1024 * 1024, |r| r.nonzero_byte());
    let out = diff_print_line("cfg09/1MiB", &s);
    assert_eq!(out.len(), 1024 * 1024 + 1);
}

/// CONFIGS row 10 — content the `printf` path could mishandle.
///
/// The C is `printf("%s\n", line)`: `line` is an *argument*, never the format.
/// Any translation that fed `line` to a formatter would diverge (or crash on
/// `%n`) here.
#[test]
fn test_cfg_10_content_special_cases() {
    let cases: &[(&str, &[u8])] = &[
        ("percent s", b"%s"),
        ("percent d", b"%d"),
        ("percent n", b"%n"),
        ("percent p", b"%p"),
        ("percent percent", b"%%"),
        ("percent lone trailing", b"abc%"),
        ("percent wide", b"%1000000d"),
        ("percent star", b"%*d"),
        ("percent dollar", b"%1$s"),
        ("many percent n", b"%n%n%n%n%n%n%n%n"),
        ("format soup", b"%s%d%n%%%p%x%hhn%lln"),
        ("brace", b"{}"),
        ("rust format", b"{0} {name} {{}}"),
        ("embedded newline", b"a\nb\nc"),
        ("leading newline", b"\nleading"),
        ("trailing newline", b"trailing\n"),
        ("only newlines", b"\n\n\n"),
        ("tab and cr", b"a\tb\rc"),
        ("backslashes", b"a\\b\\\\c\\"),
        ("escape and bell", b"\x1b[31mred\x07"),
        ("del and high", b"\x7f\x80\xff"),
        ("whitespace only", b"   \t  "),
    ];
    for (name, payload) in cases {
        let out = diff_print_line(&format!("cfg10/{name}"), payload);
        let mut want = payload.to_vec();
        want.push(b'\n');
        assert_eq!(out, want, "cfg10/{name}: bytes were not emitted verbatim");
    }

    // Randomized `%`-heavy payloads.
    let mut rng = Rng::new(0x0000_0010_5EED_0010);
    let alphabet: &[u8] = b"%sdnxp*$0123456789hl \n\t\\{}";
    for i in 0..256 {
        let len = rng.range(1, 48);
        let s = rng.bytes(len, |r| {
            let k = (r.next_u64() % alphabet.len() as u64) as usize;
            alphabet[k]
        });
        diff_print_line(&format!("cfg10/random format-ish #{i}"), &s);
    }

    // Nothing past the NUL terminator may be read or written.
    let buf: Vec<u8> = b"payload\0GUARD-MUST-NOT-APPEAR".to_vec();
    let out = assert_same("cfg10/guard byte after terminator", |api| {
        call_print_line(api, &buf)
    });
    assert_eq!(out, b"payload\n", "read past the NUL terminator");
    assert!(!out.windows(5).any(|w| w == b"GUARD"));
}

/// CONFIGS row 11 — `bad()`, the only configuration it has.
#[test]
fn test_cfg_11_bad() {
    let out = assert_same("cfg11/bad", |api| unsafe { (api.bad)() });
    assert_eq!(out, b"bad()\n");
}

/// CONFIGS row 12 — `good()` calls its helper; `bad()` deliberately does not.
///
/// The C's asymmetry (`helperBad` is defined but never called) is behaviour to
/// reproduce, not a typo to tidy up.
#[test]
fn test_cfg_12_good_calls_its_helper_and_bad_does_not() {
    let good_out = assert_same("cfg12/good", |api| unsafe { (api.good)() });
    assert_eq!(good_out, b"good()\nhelperGood()\n");

    let bad_out = assert_same("cfg12/bad has no helper line", |api| unsafe { (api.bad)() });
    assert_eq!(bad_out, b"bad()\n");
    assert!(
        !bad_out.windows(9).any(|w| w == b"helperBad"),
        "bad() must NOT emit helperBad() — the C never calls it"
    );
}

/// CONFIGS row 13 — `driver()` end to end, exact sequence.
#[test]
fn test_cfg_13_driver_end_to_end() {
    let out = assert_same("cfg13/driver", |api| unsafe { (api.driver)() });
    assert_eq!(
        out,
        b"Calling good()...\ngood()\nhelperGood()\nFinished good()\nCalling bad()...\nbad()\nFinished bad()\n",
        "driver() emitted the wrong sequence"
    );
}

enum Op {
    PrintLine(Vec<u8>),
    PrintNull,
    Bad,
    Good,
    Driver,
}

fn run_ops(api: &Api, ops: &[Op]) {
    for op in ops {
        unsafe {
            match op {
                Op::PrintLine(b) => (api.printLine)(b.as_ptr() as *const c_char),
                Op::PrintNull => (api.printLine)(ptr::null()),
                Op::Bad => (api.bad)(),
                Op::Good => (api.good)(),
                Op::Driver => (api.driver)(),
            }
        }
    }
}

/// CONFIGS row 14 — randomized interleavings of all four entry points against
/// one continuous output stream. Catches ordering / buffering / residual-state
/// bugs that per-function tests cannot see.
#[test]
fn test_cfg_14_randomized_interleaved_call_sequences() {
    let mut rng = Rng::new(0x0000_0014_5EED_0014);
    for seq in 0..128 {
        let n = rng.range(1, 40);
        // Pre-generate the program so both implementations execute it identically.
        let mut ops = Vec::with_capacity(n);
        for _ in 0..n {
            ops.push(match rng.next_u64() % 5 {
                0 => {
                    let len = rng.range(0, 32);
                    let mut b = rng.bytes(len, |r| r.nonzero_byte());
                    b.push(0);
                    Op::PrintLine(b)
                }
                1 => Op::PrintNull,
                2 => Op::Bad,
                3 => Op::Good,
                _ => Op::Driver,
            });
        }
        assert_same(&format!("cfg14/sequence #{seq} len={n}"), |api| {
            run_ops(api, &ops)
        });
    }
}

/// CONFIGS row 15 — no residual state: N calls == N copies of one call.
#[test]
fn test_cfg_15_driver_is_repeatable_with_no_residual_state() {
    let one = assert_same("cfg15/driver x1", |api| unsafe { (api.driver)() });
    let many = assert_same("cfg15/driver x50", |api| {
        for _ in 0..50 {
            unsafe { (api.driver)() }
        }
    });
    let mut expect = Vec::new();
    for _ in 0..50 {
        expect.extend_from_slice(&one);
    }
    assert_eq!(many, expect, "driver() is not stateless across repeated calls");
}

/// CONFIGS row 16 — output fd is a pipe rather than a regular file.
#[test]
fn test_cfg_16_output_to_pipe() {
    let out = assert_same_with("cfg16/driver via pipe", cap_pipe, |api| unsafe {
        (api.driver)()
    });
    assert_eq!(
        out,
        b"Calling good()...\ngood()\nhelperGood()\nFinished good()\nCalling bad()...\nbad()\nFinished bad()\n"
    );

    let mut rng = Rng::new(0x0000_0016_5EED_0016);
    for i in 0..32 {
        let len = rng.range(0, 200);
        let mut b = rng.bytes(len, |r| r.nonzero_byte());
        b.push(0);
        assert_same_with(&format!("cfg16/printLine via pipe #{i}"), cap_pipe, |api| {
            call_print_line(api, &b)
        });
    }

    // Interleaving through a pipe too — buffering regime plus composition.
    let ops = vec![
        Op::Good,
        Op::PrintNull,
        Op::Bad,
        Op::Driver,
        Op::PrintLine(b"tail\0".to_vec()),
    ];
    assert_same_with("cfg16/interleaved via pipe", cap_pipe, |api| {
        run_ops(api, &ops)
    });
}

// ===========================================================================
// PHASE C — error-path differential tests, one per ERRORS.md row
// ===========================================================================
//
// Note on what "same error" means for this library: every function returns
// `void` and the C never sets `errno`, returns a sentinel, or aborts. The
// library's sole rejection channel is *suppression of output*. So the
// same-error assertion here is necessarily: both implementations return
// normally AND both produce the identical (empty, for row 1) byte stream. A
// test that only checked "both failed somehow" would be meaningless because
// neither can signal failure at all.

/// ERRORS row 1 — `printLine(NULL)`: the null check fires, nothing is printed,
/// the call returns normally.
#[test]
fn test_err_01_null_pointer() {
    let out = assert_same("err01/printLine(NULL)", |api| unsafe {
        (api.printLine)(ptr::null())
    });
    assert!(
        out.is_empty(),
        "printLine(NULL) must emit ZERO bytes, got {:?}",
        show(&out)
    );

    // Repeated, and surrounded by valid calls, so a divergence cannot hide in
    // "the whole capture was empty anyway".
    let out = assert_same("err01/NULL between valid calls", |api| unsafe {
        call_print_line(api, b"before\0");
        (api.printLine)(ptr::null());
        (api.printLine)(ptr::null());
        (api.printLine)(ptr::null());
        call_print_line(api, b"after\0");
    });
    assert_eq!(
        out, b"before\nafter\n",
        "NULL must contribute nothing at all, not even a bare newline"
    );
}

/// ERRORS row 2 — empty string is ACCEPTED, not rejected: the guard tests the
/// pointer, not emptiness. Distinguishes `line != NULL` from `line && *line`.
#[test]
fn test_err_02_empty_string() {
    let out = diff_print_line("err02/empty string", b"");
    assert_eq!(
        out, b"\n",
        "printLine(\"\") must emit exactly one newline — the C checks the POINTER, not emptiness"
    );

    let out = assert_same("err02/empty vs null side by side", |api| unsafe {
        call_print_line(api, b"\0"); // empty string -> "\n"
        (api.printLine)(ptr::null()); // null        -> nothing
        call_print_line(api, b"\0"); // empty string -> "\n"
    });
    assert_eq!(out, b"\n\n", "empty string and NULL must be treated differently");
}

/// ERRORS row 3 — conversion specifiers in the payload are emitted verbatim and
/// never interpreted, because `line` is the argument to a fixed `"%s\n"`
/// format. `%n` in particular would be a write-what-where primitive if the
/// payload were ever used as a format string, so this row is also the security
/// check.
#[test]
fn test_err_03_format_specifiers_not_interpreted() {
    for payload in [
        &b"%n"[..],
        &b"%s"[..],
        &b"%99999999d"[..],
        &b"%s%s%s%s%s%s%s%s%s%s"[..],
        &b"%n%n%n%n%n%n%n%n%n%n"[..],
        &b"AAAA%08x.%08x.%08x.%08x.%n"[..],
    ] {
        let out = diff_print_line("err03/format specifier", payload);
        let mut want = payload.to_vec();
        want.push(b'\n');
        assert_eq!(
            out, want,
            "payload {:?} was INTERPRETED as a format string instead of emitted verbatim",
            show(payload)
        );
    }
}

/// ERRORS row 4 — stdout unwritable: `printf` fails, the C discards its return
/// value, so `printLine` still returns normally and nothing is reported. The
/// Rust must not panic or abort either.
///
/// Implemented by pointing fd 1 at `/dev/full` (writes fail with ENOSPC) and by
/// closing fd 1 outright, in a *forked child* — an unwritable fd 1 poisons the
/// process-wide stdio error flag, so it must not leak into other tests.
#[test]
fn test_err_04_stdout_closed_is_silently_ignored() {
    // Both variants are exercised in a child process; we compare the child's
    // exit status between C and Rust. A panic/abort in Rust would change it.
    for mode in ["devfull", "closed"] {
        let mut statuses = Vec::new();
        for which in ["c", "rust"] {
            let exe = std::env::current_exe().expect("current_exe");
            let st = std::process::Command::new(&exe)
                .args(["--exact", "helper_unwritable_stdout", "--nocapture", "--ignored"])
                .env("DRIVER_HELPER_MODE", mode)
                .env("DRIVER_HELPER_WHICH", which)
                .env("RUST_DRIVER_SO", rust_so_path())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .expect("spawn helper");
            statuses.push((which, st.code(), st.success()));
        }
        let (_, c_code, c_ok) = statuses[0];
        let (_, r_code, r_ok) = statuses[1];
        assert_eq!(
            (c_code, c_ok),
            (r_code, r_ok),
            "err04/{mode}: C exited {c_code:?} but Rust exited {r_code:?} — an unwritable \
             stdout must be swallowed identically (the C discards printf's return value)"
        );
        assert!(
            c_ok,
            "err04/{mode}: sanity — the C helper itself should exit 0 (failure is ignored, not fatal)"
        );
    }
}

/// Child-process body for `test_err_04...`. Ignored so it never runs on its own.
#[test]
#[ignore]
fn helper_unwritable_stdout() {
    let mode = std::env::var("DRIVER_HELPER_MODE").unwrap_or_default();
    let which = std::env::var("DRIVER_HELPER_WHICH").unwrap_or_default();
    if mode.is_empty() {
        return; // not invoked as a helper
    }
    let (c, r) = apis();
    let api = if which == "c" { c } else { r };

    unsafe {
        fflush(ptr::null_mut());
        match mode.as_str() {
            "devfull" => {
                let f = fs::OpenOptions::new()
                    .write(true)
                    .open("/dev/full")
                    .expect("open /dev/full");
                assert!(dup2(f.as_raw_fd(), 1) >= 0);
            }
            "closed" => {
                assert!(close(1) == 0 || true);
            }
            other => panic!("unknown helper mode {other}"),
        }

        // Every entry point, on a stdout that cannot be written.
        (api.printLine)(b"unwritable\0".as_ptr() as *const c_char);
        (api.printLine)(ptr::null());
        (api.bad)();
        (api.good)();
        (api.driver)();
        fflush(ptr::null_mut());
    }
    // Reaching here without panic/abort is the assertion.
    std::process::exit(0);
}

/// ERRORS row 5 — vacuous, and documented as such: there is no `enum` or
/// integer parameter anywhere in this API, so no out-of-range enum value can be
/// passed across the FFI boundary. This test pins that fact down structurally
/// by calling every argument-less entry point through a `void(void)` pointer
/// type, and by re-deriving the parameter count from the C header.
#[test]
fn test_err_05_no_enum_or_integer_input_exists() {
    let hdr = fs::read_to_string(workspace_root().join("c_src/include/driver.h")).expect("read driver.h");
    let src = fs::read_to_string(workspace_root().join("c_src/src/driver.c")).expect("read driver.c");
    assert!(
        !hdr.contains("enum") && !src.contains("enum"),
        "the C source gained an enum — ERRORS.md row 5 is no longer vacuous and needs real tests"
    );
    assert!(
        !src.contains("switch"),
        "the C source gained a switch — ERRORS.md row 5 needs real tests"
    );
    // The only parameter in the whole library is printLine's `const char *`.
    assert_eq!(
        src.matches("const char *").count(),
        1,
        "parameter surface of the C changed; re-derive ERRORS.md"
    );

    // The argument-less entry points are genuinely `void(void)`: calling them
    // through a zero-arg fn pointer is well-defined for both .so's.
    let out = assert_same("err05/void(void) entry points", |api| unsafe {
        (api.bad)();
        (api.good)();
        (api.driver)();
    });
    assert!(!out.is_empty());
}

/// ERRORS row 6 — the argument-less functions have no rejection path at all:
/// under every calling pattern they unconditionally emit their fixed output.
#[test]
fn test_err_06_argless_functions_have_no_rejection_path() {
    for (name, f) in [
        ("bad", 0usize),
        ("good", 1),
        ("driver", 2),
    ] {
        let out = assert_same(&format!("err06/{name} repeated"), |api| unsafe {
            let g: FnVoid = match f {
                0 => api.bad,
                1 => api.good,
                _ => api.driver,
            };
            for _ in 0..10 {
                g();
            }
        });
        assert!(
            !out.is_empty(),
            "err06/{name}: an argument-less function must always produce its output"
        );
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary probes (beyond the ERRORS.md table, per the checklist)
// ---------------------------------------------------------------------------

/// Misaligned / unusual-but-valid `const char *` values, and a pointer to a
/// string sitting at the very end of a heap allocation (so any read past the
/// NUL would be a real out-of-bounds access, catchable under a sanitizer).
#[test]
fn test_err_07_pointer_shapes() {
    // Deliberately misaligned start (odd address inside a larger buffer).
    let backing: Vec<u8> = {
        let mut v = vec![0xAAu8; 1];
        v.extend_from_slice(b"misaligned payload\0");
        v
    };
    let out = assert_same("err07/misaligned pointer", |api| unsafe {
        (api.printLine)(backing[1..].as_ptr() as *const c_char)
    });
    assert_eq!(out, b"misaligned payload\n");

    // String whose NUL is the last byte of its allocation.
    for len in [0usize, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 63, 64, 4095, 4096, 4097] {
        let mut rng = Rng::new(0xE07_0000 + len as u64);
        let mut buf: Vec<u8> = rng.bytes(len, |r| r.nonzero_byte());
        buf.push(0);
        buf.shrink_to_fit();
        let out = assert_same(&format!("err07/tight allocation len={len}"), |api| {
            call_print_line(api, &buf)
        });
        assert_eq!(out.len(), len + 1);
    }

    // A `printLine` call whose payload is the maximum single byte value, right
    // at a NUL that terminates immediately after.
    let out = diff_print_line("err07/0xff then nul", &[0xFF]);
    assert_eq!(out, vec![0xFF, b'\n']);
}

/// NULL passed to `printLine` in every position of a randomized call sequence —
/// the interaction of the error path with the valid path.
#[test]
fn test_err_08_null_interleaved_randomized() {
    let mut rng = Rng::new(0xE08_5EED_0008);
    for seq in 0..128 {
        let n = rng.range(1, 24);
        let mut ops = Vec::with_capacity(n);
        for _ in 0..n {
            // Bias heavily toward NULL so the error path dominates.
            ops.push(if rng.next_u64() % 2 == 0 {
                Op::PrintNull
            } else {
                match rng.next_u64() % 4 {
                    0 => {
                        let len = rng.range(0, 16);
                        let mut b = rng.bytes(len, |r| r.nonzero_byte());
                        b.push(0);
                        Op::PrintLine(b)
                    }
                    1 => Op::Bad,
                    2 => Op::Good,
                    _ => Op::Driver,
                }
            });
        }
        assert_same(&format!("err08/null-heavy sequence #{seq}"), |api| {
            run_ops(api, &ops)
        });
    }
}
