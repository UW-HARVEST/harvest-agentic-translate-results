// Differential tests: load BOTH the C `libdriver.so` and the Rust `libdriver.so`
// through `libloading` and compare their observable behaviour byte-for-byte.
//
// Neither library is ever called as a Rust function. Every call goes through
// `dlsym` on a `.so`, exactly as an external C consumer would, so the
// `#[unsafe(no_mangle)] extern "C"` export wrappers are themselves under test.
//
// Both `driver` and `printHexCharLine` return `void`; their entire observable
// output is what they write to `stdout` via libc `printf`. The harness below
// therefore captures file descriptor 1 around each call batch.
//
// This target is declared `harness = false` in Cargo.toml: fd-1 redirection is
// process-wide, so libtest's multi-threaded runner (which writes its own
// progress to fd 1) would land output inside the capture windows. `main` below
// runs every row sequentially instead.
//
// Environment overrides:
//   DRIVER_C_SO    — path to the C shared library
//   DRIVER_RUST_SO — path to the Rust cdylib (defaults to release, then debug)

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed to capture fd 1
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream. The test binary, the C
    /// `.so` and the Rust `.so` all resolve `printf`/`fflush` to the single glibc
    /// mapped into this process, so they share one `stdout` `FILE` and flushing
    /// here reliably drains what the loaded libraries wrote.
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FD: c_int = 1;

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

/// Walk up from the crate root to the shared working directory that holds both
/// `c_src/` and `translation/`.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate root must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        crate_root.join("target/release/libdriver.so"),
        crate_root.join("target/debug/libdriver.so"),
    ];
    candidates
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "Rust cdylib not found in any of {candidates:?}.\nBuild it with:\n  cd translation && cargo build --release"
            )
        })
}

/// The two libraries, loaded once for the whole run.
///
/// `libloading::Library::new` uses `RTLD_LOCAL`, so both `.so`s can export
/// `driver` / `printHexCharLine` without colliding: `dlsym` on a handle searches
/// that handle's own scope.
struct Libs {
    c: Library,
    rust: Library,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        // SAFETY: both paths point at plain C-ABI shared objects whose
        // initialisers do not run arbitrary code.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));
        Libs { c, rust }
    })
}

/// `void f(char)` as an external C consumer that has read `driver.h` would
/// declare it. On the SysV AMD64 ABI the argument travels in the low byte of
/// `%edi`, already narrowed by the caller.
type FnChar = unsafe extern "C" fn(c_char);

/// Deliberately widened to `void f(int)`. This is ABI-compatible with the `char`
/// declaration but leaves non-zero high bytes in the argument register — exactly
/// how an out-of-range value reaches a narrow C parameter, and the same reason a
/// C `enum` parameter accepts any `int`. Used by the Phase C out-of-range test.
type FnInt = unsafe extern "C" fn(c_int);

fn sym<'a, T>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    // SAFETY: the caller supplies a signature matching the C declaration, or an
    // ABI-compatible widening of it.
    unsafe { lib.get(name) }.unwrap_or_else(|e| panic!("dlsym {} failed: {e}", pretty(name)))
}

fn pretty(name: &[u8]) -> String {
    String::from_utf8_lossy(&name[..name.len() - 1]).to_string()
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Belt-and-braces guard on fd-1 redirection even though `main` is sequential.
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Run `body`, returning everything it wrote to file descriptor 1.
///
/// Redirects fd 1 to an unlinked scratch file for the duration of the call, so
/// output produced by `printf` inside either `.so` is collected verbatim,
/// including stream ordering across multiple calls.
fn capture<F: FnOnce()>(body: F) -> Vec<u8> {
    let guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let mut tmp = scratch_file();

    // Drain anything already buffered so it is not misattributed to `body`.
    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::stdout().flush();

    let saved = unsafe { dup(STDOUT_FD) };
    assert!(saved >= 0, "dup(1) failed");

    let tmp_fd = {
        use std::os::fd::AsRawFd;
        tmp.as_raw_fd()
    };
    assert!(
        unsafe { dup2(tmp_fd, STDOUT_FD) } >= 0,
        "dup2 onto fd 1 failed"
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));

    // Flush the libraries' buffered writes *before* putting fd 1 back.
    unsafe { fflush(std::ptr::null_mut()) };

    assert!(
        unsafe { dup2(saved, STDOUT_FD) } >= 0,
        "restoring fd 1 failed"
    );
    unsafe { close(saved) };

    drop(guard);
    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }

    let mut out = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek scratch file");
    tmp.read_to_end(&mut out).expect("read scratch file");
    out
}

fn scratch_file() -> std::fs::File {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("driver_diff_{}_{n}.out", std::process::id()));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create scratch capture file");
    // Unlink immediately: the fd keeps it alive and nothing is left behind even
    // if a row panics.
    let _ = std::fs::remove_file(&path);
    f
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Call `symbol` once per value in `values` on the C `.so`, capturing the whole
/// stream; then do the same on the Rust `.so`; assert the streams are identical.
///
/// Batching the whole value list into one capture also exercises stdout
/// buffering and ordering across a call sequence, not just a single call.
fn diff_char_seq(symbol: &[u8], values: &[u8], row: &str) {
    let l = libs();

    let run = |lib: &Library| -> Vec<u8> {
        let f: Symbol<FnChar> = sym(lib, symbol);
        capture(|| {
            for &v in values {
                unsafe { f(v as c_char) };
            }
        })
    };

    let c_out = run(&l.c);
    let r_out = run(&l.rust);
    assert_streams_eq(&c_out, &r_out, symbol, values, row);
}

/// Same, but through the intentionally widened `void f(int)` signature.
fn diff_int_seq(symbol: &[u8], values: &[i32], row: &str) {
    let l = libs();

    let run = |lib: &Library| -> Vec<u8> {
        let f: Symbol<FnInt> = sym(lib, symbol);
        capture(|| {
            for &v in values {
                unsafe { f(v as c_int) };
            }
        })
    };

    let c_out = run(&l.c);
    let r_out = run(&l.rust);

    if c_out != r_out {
        let c_lines: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
        let r_lines: Vec<&[u8]> = r_out.split(|&b| b == b'\n').collect();
        for (i, (cl, rl)) in c_lines.iter().zip(r_lines.iter()).enumerate() {
            if cl != rl {
                panic!(
                    "[{row}] {} diverged at call #{i} (widened input {:#010x}):\n  C   : {:?}\n  Rust: {:?}",
                    pretty(symbol),
                    values.get(i).copied().unwrap_or(0),
                    String::from_utf8_lossy(cl),
                    String::from_utf8_lossy(rl),
                );
            }
        }
        panic!(
            "[{row}] {} stream length differs: C {} bytes vs Rust {} bytes",
            pretty(symbol),
            c_out.len(),
            r_out.len()
        );
    }
    assert!(!c_out.is_empty(), "[{row}] C produced no output at all");
    assert_eq!(
        c_out.iter().filter(|&&b| b == b'\n').count(),
        values.len(),
        "[{row}] {} emitted the wrong number of lines",
        pretty(symbol)
    );
}

fn assert_streams_eq(c_out: &[u8], r_out: &[u8], symbol: &[u8], values: &[u8], row: &str) {
    let name = pretty(symbol);

    if c_out != r_out {
        let c_lines: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
        let r_lines: Vec<&[u8]> = r_out.split(|&b| b == b'\n').collect();
        for (i, (cl, rl)) in c_lines.iter().zip(r_lines.iter()).enumerate() {
            if cl != rl {
                panic!(
                    "[{row}] {name} diverged at call #{i} (input {:#04x}):\n  C   : {:?}\n  Rust: {:?}",
                    values.get(i).copied().unwrap_or(0),
                    String::from_utf8_lossy(cl),
                    String::from_utf8_lossy(rl),
                );
            }
        }
        panic!(
            "[{row}] {name} stream length differs: C {} bytes / {} lines vs Rust {} bytes / {} lines",
            c_out.len(),
            c_lines.len(),
            r_out.len(),
            r_lines.len()
        );
    }

    // Guard against a degenerate "both produced nothing" pass.
    assert!(
        !c_out.is_empty(),
        "[{row}] {name} produced no output at all for {} inputs — capture harness is broken",
        values.len()
    );
    assert_eq!(
        c_out.iter().filter(|&&b| b == b'\n').count(),
        values.len(),
        "[{row}] {name} emitted the wrong number of lines for {} inputs",
        values.len()
    );
}

// ---------------------------------------------------------------------------
// Seeded PRNG (fixed seed => reproducible)
// ---------------------------------------------------------------------------

const SEED: u64 = 0x2025_0901_DEAD_BEEF;

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        Rng(SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn in_range(&mut self, lo: u8, hi: u8) -> u8 {
        let span = (hi as u64 - lo as u64) + 1;
        lo + (self.next_u64() % span) as u8
    }
}

/// `n` randomized bytes drawn from `lo..=hi`, with both endpoints always
/// included so a randomized row can never miss its own boundaries.
fn random_in_range(salt: u64, lo: u8, hi: u8, n: usize) -> Vec<u8> {
    let mut rng = Rng::new(salt);
    let mut v = Vec::with_capacity(n + 2);
    v.push(lo);
    v.push(hi);
    for _ in 0..n {
        v.push(rng.in_range(lo, hi));
    }
    v
}

const N_RANDOM: usize = 400;

const ALL_256: [u8; 256] = {
    let mut a = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        a[i] = i as u8;
        i += 1;
    }
    a
};

const PRINT: &[u8] = b"printHexCharLine\0";
const DRIVER: &[u8] = b"driver\0";

// ===========================================================================
// Phase B — valid-path differential rows (one per CONFIGS.md row)
// ===========================================================================

/// C1: `0x00..=0x0F` — `%02x` zero-pads these to two digits.
fn configs_c1_print_low_nibble_padded() {
    diff_char_seq(PRINT, &random_in_range(1, 0x00, 0x0F, N_RANDOM), "C1");
}

/// C2: `0x10..=0x7F` — non-negative, exactly two digits, no padding.
fn configs_c2_print_positive_unpadded() {
    diff_char_seq(PRINT, &random_in_range(2, 0x10, 0x7F, N_RANDOM), "C2");
}

/// C3: `0x80..=0xFF` — negative as a signed `char`, so the variadic promotion
/// sign-extends and `%02x` prints eight digits.
fn configs_c3_print_negative_sign_extended() {
    diff_char_seq(PRINT, &random_in_range(3, 0x80, 0xFF, N_RANDOM), "C3");
}

/// C4: every sub-range boundary value for the low-level entry point.
fn configs_c4_print_boundaries() {
    diff_char_seq(
        PRINT,
        &[
            0x00, 0x01, 0x0E, 0x0F, 0x10, 0x7E, 0x7F, 0x80, 0x81, 0xFE, 0xFF,
        ],
        "C4",
    );
}

/// C5: exhaustive over the entire valid input domain.
fn configs_c5_print_exhaustive_all_256() {
    diff_char_seq(PRINT, &ALL_256, "C5");
}

/// C6: `driver` inputs whose `+1` result stays in the padded low-nibble range.
fn configs_c6_driver_low_nibble_padded() {
    diff_char_seq(DRIVER, &random_in_range(6, 0x00, 0x0E, N_RANDOM), "C6");
}

/// C7: `driver` inputs whose `+1` result lands in the unpadded positive range.
fn configs_c7_driver_positive_unpadded() {
    diff_char_seq(DRIVER, &random_in_range(7, 0x0F, 0x7E, N_RANDOM), "C7");
}

/// C8: `data + 1` pushes these into the negative sub-range — the boundary-shift
/// row that only exists because of the wrapper's arithmetic.
fn configs_c8_driver_negative_sign_extended() {
    diff_char_seq(DRIVER, &random_in_range(8, 0x7F, 0xFE, N_RANDOM), "C8");
}

/// C9: `0xFF` (== -1) + 1 truncates back to `0x00`.
fn configs_c9_driver_wraparound() {
    diff_char_seq(DRIVER, &[0xFF], "C9");
}

/// C10: exhaustive over the entire valid input domain, via the wrapper.
fn configs_c10_driver_exhaustive_all_256() {
    diff_char_seq(DRIVER, &ALL_256, "C10");
}

/// C11: the wrapper's arithmetic cannot hide behind a correct low-level
/// function. Assert `driver(v)` == `printHexCharLine(trunc(v+1))` on *each*
/// library, then assert the two libraries agree through the composed entry
/// point.
fn configs_c11_composition_identity() {
    let l = libs();
    let inputs = random_in_range(11, 0x00, 0xFF, N_RANDOM);
    let shifted: Vec<u8> = inputs.iter().map(|v| v.wrapping_add(1)).collect();

    for (label, lib) in [("C", &l.c), ("Rust", &l.rust)] {
        let via_driver = {
            let f: Symbol<FnChar> = sym(lib, DRIVER);
            capture(|| {
                for &v in &inputs {
                    unsafe { f(v as c_char) };
                }
            })
        };
        let via_low_level = {
            let f: Symbol<FnChar> = sym(lib, PRINT);
            capture(|| {
                for &v in &shifted {
                    unsafe { f(v as c_char) };
                }
            })
        };
        assert!(
            via_driver == via_low_level,
            "[C11] {label}: driver(v) != printHexCharLine(v+1) — the composed pipeline \
             diverges from its parts.\n  driver : {:?}\n  low-lvl: {:?}",
            String::from_utf8_lossy(&via_driver[..via_driver.len().min(120)]),
            String::from_utf8_lossy(&via_low_level[..via_low_level.len().min(120)]),
        );
        assert!(!via_driver.is_empty(), "[C11] {label}: no output captured");
    }

    diff_char_seq(DRIVER, &inputs, "C11");
}

/// C12: alternate the two entry points inside a single captured stream — checks
/// ordering and buffering of the composed pipeline, not one call in isolation.
fn configs_c12_interleaved_stream() {
    let l = libs();
    let mut rng = Rng::new(12);
    let plan: Vec<(bool, u8)> = (0..800)
        .map(|_| {
            let r = rng.next_u64();
            (r & 1 == 0, (r >> 8) as u8)
        })
        .collect();

    let run = |lib: &Library| -> Vec<u8> {
        let d: Symbol<FnChar> = sym(lib, DRIVER);
        let p: Symbol<FnChar> = sym(lib, PRINT);
        capture(|| {
            for &(use_driver, v) in &plan {
                if use_driver {
                    unsafe { d(v as c_char) };
                } else {
                    unsafe { p(v as c_char) };
                }
            }
        })
    };

    let c_out = run(&l.c);
    let r_out = run(&l.rust);
    assert!(
        c_out == r_out,
        "[C12] interleaved stream diverged\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out[..c_out.len().min(200)]),
        String::from_utf8_lossy(&r_out[..r_out.len().min(200)])
    );
    assert_eq!(
        c_out.iter().filter(|&&b| b == b'\n').count(),
        plan.len(),
        "[C12] wrong line count"
    );
}

// ===========================================================================
// Phase C — error-path differential rows (one per ERRORS.md row)
// ===========================================================================

/// ERRORS.md rows E1 and E2. The C source contains zero `return`s, zero
/// asserts, zero range/null checks and zero branches, so both functions are
/// total over the 256-value `char` domain. Prove that empirically instead of
/// trusting the grep: every input must yield exactly one non-empty all-hex line
/// from *both* libraries, and the two must agree byte-for-byte.
fn errors_e1_e2_no_rejection_path_exists() {
    let l = libs();

    for (row, symbol) in [("E1", PRINT), ("E2", DRIVER)] {
        for (label, lib) in [("C", &l.c), ("Rust", &l.rust)] {
            let f: Symbol<FnChar> = sym(lib, symbol);
            let out = capture(|| {
                for &v in ALL_256.iter() {
                    unsafe { f(v as c_char) };
                }
            });
            let lines: Vec<&[u8]> = out.split(|&b| b == b'\n').collect();
            // 256 lines plus the empty fragment after the final '\n'.
            assert_eq!(
                lines.len(),
                257,
                "[{row}] {label} rejected or dropped an input: got {} lines for 256 calls",
                lines.len().saturating_sub(1)
            );
            for (i, line) in lines[..256].iter().enumerate() {
                assert!(
                    !line.is_empty(),
                    "[{row}] {label} produced an empty line for input {i:#04x} — that would be a rejection path"
                );
                assert!(
                    line.iter().all(|b| b.is_ascii_hexdigit()),
                    "[{row}] {label} produced non-hex output {:?} for input {i:#04x}",
                    String::from_utf8_lossy(line)
                );
            }
        }
        diff_char_seq(symbol, &ALL_256, row);
    }
}

/// Generic C-API boundaries instantiated for the only parameter this API has:
/// signed-char max and one past it, unsigned-char max and one past it (i.e.
/// wraparound), plus each sub-range edge.
fn errors_boundary_values_one_past_range() {
    let boundaries: [u8; 9] = [
        0x00, // unsigned min == one past 0xFF
        0x01, 0x0F, 0x10, // zero-padding boundary
        0x7E, 0x7F, // signed char max
        0x80, // one past signed max => negative
        0x81, 0xFE,
    ];
    diff_char_seq(PRINT, &boundaries, "E-boundary/print");
    diff_char_seq(DRIVER, &boundaries, "E-boundary/driver");
    diff_char_seq(PRINT, &[0xFF], "E-boundary/print/0xFF");
    diff_char_seq(DRIVER, &[0xFF], "E-boundary/driver/0xFF");
}

/// `driver(0x7F)`: `data + 1` exceeds the signed-char range. The C computes it
/// in promoted `int` (so no UB) and truncates on assignment to `char`. Confirm
/// the Rust agrees, and that both really wrap rather than saturate.
fn errors_arithmetic_overflow_in_driver() {
    diff_char_seq(DRIVER, &[0x7E, 0x7F, 0x80, 0xFE, 0xFF], "E-overflow");

    let l = libs();
    let f: Symbol<FnChar> = sym(&l.c, DRIVER);
    let c_out = capture(|| unsafe { f(0x7F) });
    assert_eq!(
        String::from_utf8_lossy(&c_out).trim(),
        "ffffff80",
        "ground truth: C must wrap 0x7F+1 to -128 and sign-extend it"
    );
}

/// A C `char` parameter, like a C `enum` parameter, accepts any `int` at the ABI
/// level: the callee only sees an argument register. Values with non-zero high
/// bytes are therefore real inputs that cross the FFI boundary, and the Rust
/// export must narrow them exactly as GCC's `mov %al` does. This is the class of
/// bug happy-path tests miss.
fn errors_out_of_range_int_passed_as_char_arg() {
    let wide: [i32; 14] = [
        0x0000_0000,
        0x0000_0041,
        0x0000_0100, // low byte 0x00, high bits set
        0x0000_01FF, // low byte 0xFF
        0x0000_017F,
        0x0000_0180,
        0x1234_5678,
        0x7FFF_FFFF,
        -1,
        -128,
        -129,
        i32::MIN,
        0xDEAD_BEEFu32 as i32,
        0xFFFF_FF00u32 as i32,
    ];
    diff_int_seq(PRINT, &wide, "E-oob-int/print");
    diff_int_seq(DRIVER, &wide, "E-oob-int/driver");

    // Fuzz the widened signature so no single hand-picked value carries the row.
    let mut rng = Rng::new(99);
    let fuzz: Vec<i32> = (0..1000).map(|_| rng.next_u64() as i32).collect();
    diff_int_seq(PRINT, &fuzz, "E-oob-int/print/fuzz");
    diff_int_seq(DRIVER, &fuzz, "E-oob-int/driver/fuzz");
}

/// Neither function has state; assert that a long randomized sequence does not
/// drift between the libraries and that a repeated input yields the same line
/// every time.
fn errors_repeated_and_interleaved_calls_no_state_corruption() {
    let mut rng = Rng::new(77);
    let seq: Vec<u8> = (0..2000).map(|_| rng.next_u64() as u8).collect();
    diff_char_seq(PRINT, &seq, "E-state/print");
    diff_char_seq(DRIVER, &seq, "E-state/driver");

    let repeated = vec![0x80u8; 500];
    diff_char_seq(PRINT, &repeated, "E-state/repeat/print");
    diff_char_seq(DRIVER, &repeated, "E-state/repeat/driver");
}

// ===========================================================================
// Phase D — symbol parity, enforced from inside the suite
// ===========================================================================

fn defined_symbols(so: &Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// under the exact same name, and must be `dlsym`-resolvable in both.
fn phase_d_symbol_parity_c_so_vs_rust_so() {
    let c_syms = defined_symbols(&c_so_path());
    let r_syms = defined_symbols(&rust_so_path());

    assert!(
        !c_syms.is_empty(),
        "nm reported no symbols for the C .so — harness broken"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}",
        missing.len()
    );

    let l = libs();
    for symbol in [PRINT, DRIVER] {
        let _c: Symbol<FnChar> = sym(&l.c, symbol);
        let _r: Symbol<FnChar> = sym(&l.rust, symbol);
    }
}

/// The Rust `.so` must not import anything beyond libc / the unwinder / the
/// optional weak toolchain hooks present in every ELF DSO.
fn phase_d_rust_so_has_no_unresolved_non_libc_symbols() {
    let out = std::process::Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed");

    const LIBC_ALLOWED: &[&str] = &[
        "printf",
        "malloc",
        "free",
        "calloc",
        "realloc",
        "posix_memalign",
        "memcpy",
        "memmove",
        "memset",
        "bcmp",
        "strlen",
        "abort",
        "getenv",
        "getcwd",
        "readlink",
        "realpath",
        "open64",
        "read",
        "write",
        "writev",
        "close",
        "lseek64",
        "stat64",
        "fstat64",
        "statx",
        "mmap64",
        "munmap",
        "syscall",
        "gettid",
        "dl_iterate_phdr",
    ];

    let text = String::from_utf8_lossy(&out.stdout);
    let unresolved: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap_or(s))
        .filter(|s| {
            !(s.starts_with("_Unwind_")
                || s.starts_with("_ITM_")
                || s.starts_with("__cxa_")
                || s.starts_with("pthread_")
                || s.starts_with("__")
                || LIBC_ALLOWED.contains(s))
        })
        .collect();

    assert!(
        unresolved.is_empty(),
        "Rust .so has unresolved non-libc symbols: {unresolved:?}"
    );
}

// ---------------------------------------------------------------------------
// Harness self-check
// ---------------------------------------------------------------------------

/// If `capture` silently returned empty output, every differential assertion
/// would pass vacuously. Prove it captures, prove it can tell two outputs apart,
/// and pin the C's ground-truth sign-extension behaviour.
fn harness_capture_actually_captures_and_can_detect_a_difference() {
    let l = libs();
    let f: Symbol<FnChar> = sym(&l.c, PRINT);

    let a = capture(|| unsafe { f(0x41) });
    assert_eq!(
        String::from_utf8_lossy(&a),
        "41\n",
        "capture returned {a:?}"
    );

    let b = capture(|| unsafe { f(0x42) });
    assert_ne!(a, b, "capture must distinguish different inputs");

    let neg = capture(|| unsafe { f(0x80u8 as c_char) });
    assert_eq!(
        String::from_utf8_lossy(&neg).trim(),
        "ffffff80",
        "ground truth: C sign-extends a negative char before %02x"
    );

    let pad = capture(|| unsafe { f(0x05) });
    assert_eq!(
        String::from_utf8_lossy(&pad).trim(),
        "05",
        "ground truth: %02x zero-pads the low nibble range"
    );
}

// ===========================================================================
// Sequential runner (harness = false)
// ===========================================================================

type Row = (&'static str, fn());

const ROWS: &[Row] = &[
    // Harness self-check first: if capture is broken, nothing below means
    // anything.
    (
        "harness_capture_actually_captures_and_can_detect_a_difference",
        harness_capture_actually_captures_and_can_detect_a_difference,
    ),
    // Phase D symbol parity — cheap, and a missing symbol invalidates the rest.
    (
        "phase_d_symbol_parity_c_so_vs_rust_so",
        phase_d_symbol_parity_c_so_vs_rust_so,
    ),
    (
        "phase_d_rust_so_has_no_unresolved_non_libc_symbols",
        phase_d_rust_so_has_no_unresolved_non_libc_symbols,
    ),
    // Phase B — CONFIGS.md rows, lowest-level entry point first.
    (
        "configs_c1_print_low_nibble_padded",
        configs_c1_print_low_nibble_padded,
    ),
    (
        "configs_c2_print_positive_unpadded",
        configs_c2_print_positive_unpadded,
    ),
    (
        "configs_c3_print_negative_sign_extended",
        configs_c3_print_negative_sign_extended,
    ),
    ("configs_c4_print_boundaries", configs_c4_print_boundaries),
    (
        "configs_c5_print_exhaustive_all_256",
        configs_c5_print_exhaustive_all_256,
    ),
    (
        "configs_c6_driver_low_nibble_padded",
        configs_c6_driver_low_nibble_padded,
    ),
    (
        "configs_c7_driver_positive_unpadded",
        configs_c7_driver_positive_unpadded,
    ),
    (
        "configs_c8_driver_negative_sign_extended",
        configs_c8_driver_negative_sign_extended,
    ),
    ("configs_c9_driver_wraparound", configs_c9_driver_wraparound),
    (
        "configs_c10_driver_exhaustive_all_256",
        configs_c10_driver_exhaustive_all_256,
    ),
    (
        "configs_c11_composition_identity",
        configs_c11_composition_identity,
    ),
    (
        "configs_c12_interleaved_stream",
        configs_c12_interleaved_stream,
    ),
    // Phase C — ERRORS.md rows.
    (
        "errors_e1_e2_no_rejection_path_exists",
        errors_e1_e2_no_rejection_path_exists,
    ),
    (
        "errors_boundary_values_one_past_range",
        errors_boundary_values_one_past_range,
    ),
    (
        "errors_arithmetic_overflow_in_driver",
        errors_arithmetic_overflow_in_driver,
    ),
    (
        "errors_out_of_range_int_passed_as_char_arg",
        errors_out_of_range_int_passed_as_char_arg,
    ),
    (
        "errors_repeated_and_interleaved_calls_no_state_corruption",
        errors_repeated_and_interleaved_calls_no_state_corruption,
    ),
];

fn main() {
    // Accept an optional substring filter, like libtest.
    let filter: Option<String> = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with('-') && a != "--exact" && a != "--nocapture");

    let l = libs();
    eprintln!("C    .so: {}", c_so_path().display());
    eprintln!("Rust .so: {}", rust_so_path().display());
    let _ = l; // force load before the first capture window

    let mut passed = 0usize;
    let mut failed: Vec<&str> = Vec::new();
    let mut skipped = 0usize;

    for (name, f) in ROWS {
        if let Some(ref filt) = filter {
            if !name.contains(filt.as_str()) {
                skipped += 1;
                continue;
            }
        }
        // Silence the default hook so a failing row reports once, via us.
        let prev = std::panic::take_hook();
        let msg: std::sync::Arc<Mutex<String>> = std::sync::Arc::new(Mutex::new(String::new()));
        let msg2 = std::sync::Arc::clone(&msg);
        std::panic::set_hook(Box::new(move |info| {
            *msg2.lock().unwrap_or_else(|e| e.into_inner()) = format!("{info}");
        }));
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(*f));
        std::panic::set_hook(prev);

        match outcome {
            Ok(()) => {
                passed += 1;
                eprintln!("ok      {name}");
            }
            Err(_) => {
                failed.push(name);
                eprintln!("FAILED  {name}");
                eprintln!(
                    "{}",
                    msg.lock().unwrap_or_else(|e| e.into_inner()).as_str()
                );
            }
        }
    }

    eprintln!(
        "\ndifferential result: {} passed; {} failed; {} filtered out",
        passed,
        failed.len(),
        skipped
    );
    if !failed.is_empty() {
        eprintln!("failures:");
        for f in &failed {
            eprintln!("    {f}");
        }
        std::process::exit(1);
    }
}
