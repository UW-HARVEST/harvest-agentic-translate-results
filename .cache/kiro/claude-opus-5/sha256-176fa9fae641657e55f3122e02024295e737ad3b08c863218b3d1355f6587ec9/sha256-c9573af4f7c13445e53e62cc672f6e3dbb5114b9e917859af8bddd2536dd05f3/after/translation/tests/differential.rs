// Differential tests: load BOTH the C `libdriver.so` and the Rust
// `libdriver.so` with `libloading` and compare their observable behaviour
// through the FFI boundary.
//
// The Rust implementation is NEVER called directly — always via the loaded
// `.so`'s exported `driver` symbol, so the `#[no_mangle]` / `extern "C"`
// wrapper is under test too.
//
// `driver` returns `void`; its entire observable effect is the bytes it writes
// to `stdout` via C `printf`. So the differential oracle here is a `stdout`
// capture: fd 1 is redirected to a temp file around each call, `fflush(NULL)`
// drains the shared libc `FILE` buffer, and the resulting bytes are compared.

use libloading::Library;
use std::ffi::{c_int, c_void};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits needed to capture the C-side `stdout`
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open output streams, including the
    /// `stdout` buffer both `.so`s write through.
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Loading the two shared libraries
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Builds and returns the path to the Rust `cdylib`.
///
/// This must NOT just look for `target/{debug,release}/libdriver.so`: `cargo
/// test` does not build the `cdylib` artifact at all (an integration test
/// cannot link a `cdylib`, so Cargo skips it), and picking up a leftover
/// artifact would silently test a stale library. So build it explicitly, into
/// a dedicated target dir — a separate `--target-dir` has its own build lock,
/// so this does not deadlock against the outer `cargo test`.
fn rust_so_path() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let manifest = manifest_dir();
        let target_dir = manifest.join("target/ffi-so");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

        let out = std::process::Command::new(&cargo)
            .current_dir(&manifest)
            .args([
                "build",
                "--release",
                "--lib",
                "--target-dir",
                target_dir.to_str().unwrap(),
            ])
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("RUSTC_WORKSPACE_WRAPPER")
            .output()
            .expect("spawn cargo to build the Rust cdylib");
        assert!(
            out.status.success(),
            "failed to build the Rust cdylib:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );

        let so = target_dir.join("release/libdriver.so");
        assert!(so.exists(), "cargo build did not produce {so:?}");

        // Belt and braces: the artifact must be newer than the source it was
        // built from, so a stale `.so` can never be tested silently.
        let src = manifest.join("src/lib.rs");
        let mtime = |p: &std::path::Path| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .unwrap_or_else(|e| panic!("mtime of {p:?}: {e}"))
        };
        assert!(
            mtime(&so) >= mtime(&src),
            "{so:?} is older than {src:?} — stale artifact"
        );
        so
    })
}

fn c_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| unsafe { Library::new(c_so_path()).expect("dlopen C .so") })
}

fn rust_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| unsafe { Library::new(rust_so_path().clone()).expect("dlopen Rust .so") })
}

/// `void driver(int)` as exported by the C `.so`.
fn c_driver() -> extern "C" fn(c_int) {
    static F: OnceLock<usize> = OnceLock::new();
    let addr = *F.get_or_init(|| unsafe {
        let s = c_lib()
            .get::<extern "C" fn(c_int)>(b"driver\0")
            .expect("C .so exports `driver`");
        *s as usize
    });
    unsafe { std::mem::transmute::<usize, extern "C" fn(c_int)>(addr) }
}

/// `void driver(int)` as exported by the Rust `.so` (via `#[no_mangle]`).
fn rust_driver() -> extern "C" fn(c_int) {
    static F: OnceLock<usize> = OnceLock::new();
    let addr = *F.get_or_init(|| unsafe {
        let s = rust_lib()
            .get::<extern "C" fn(c_int)>(b"driver\0")
            .expect("Rust .so exports `driver`");
        *s as usize
    });
    unsafe { std::mem::transmute::<usize, extern "C" fn(c_int)>(addr) }
}

/// Same symbols, viewed through a deliberately *widened* prototype so a value
/// larger than `int` can be pushed into the argument register (CONFIGS row 13).
fn c_driver_wide() -> extern "C" fn(i64) {
    unsafe { std::mem::transmute::<extern "C" fn(c_int), extern "C" fn(i64)>(c_driver()) }
}
fn rust_driver_wide() -> extern "C" fn(i64) {
    unsafe { std::mem::transmute::<extern "C" fn(c_int), extern "C" fn(i64)>(rust_driver()) }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Runs `f` with fd 1 redirected to a temp file and returns everything written.
///
/// fd redirection is process-global, so this serialises on `CAPTURE_LOCK` and
/// the harness is pinned to a single thread (see `.cargo/config.toml`,
/// `RUST_TEST_THREADS=1`). Rust's own `stdout` buffer is flushed on both sides
/// of the redirect so libtest progress text can never land in the capture.
fn capture(f: impl FnOnce()) -> Vec<u8> {
    // fd redirection is process-global: serialise all captures.
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Drain both the Rust-side (libtest) and C-side stdio buffers first.
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver_diff_{}_{}.out",
        std::process::id(),
        TMP_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = std::fs::File::options()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("create capture file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "restore dup2 failed");
    unsafe { close(saved) };

    file.seek(SeekFrom::Start(0)).expect("rewind capture");
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).expect("read capture");
    drop(file);
    let _ = std::fs::remove_file(&path);

    // The library only ever emits lowercase hex digits and '\n'. Anything else
    // means foreign output (e.g. libtest progress text from another thread)
    // contaminated the capture, which would show up as a bogus "divergence".
    // Fail loudly and actionably instead.
    assert!(
        buf.iter().all(|b| b.is_ascii_hexdigit() || *b == b'\n'),
        "capture contaminated by foreign stdout output: {:?}\n\
         re-run with a single test thread: `cargo test -- --test-threads=1` \
         (or keep RUST_TEST_THREADS=1 from .cargo/config.toml)",
        String::from_utf8_lossy(&buf)
    );

    buf
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

// ---------------------------------------------------------------------------
// The core differential assertion
// ---------------------------------------------------------------------------

/// Calls C `driver(v)` and Rust `driver(v)` and asserts byte-identical output.
#[track_caller]
fn assert_same(v: i32) {
    let c = capture(|| c_driver()(v));
    let r = capture(|| rust_driver()(v));
    assert_eq!(
        c,
        r,
        "divergence for driver({v}) [0x{v:08x}]:\n  C   : {}\n  Rust: {}",
        show(&c),
        show(&r)
    );
    // Sanity: the output really is the 16-byte struct dump + newline. Guards
    // against a "both produced nothing" false pass.
    assert_eq!(
        c.len(),
        33,
        "unexpected output length for driver({v}): {}",
        show(&c)
    );
}

#[track_caller]
fn assert_same_wide(v: i64) {
    let c = capture(|| c_driver_wide()(v));
    let r = capture(|| rust_driver_wide()(v));
    assert_eq!(
        c,
        r,
        "divergence for wide driver(0x{v:016x}):\n  C   : {}\n  Rust: {}",
        show(&c),
        show(&r)
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
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
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `lo..=hi`.
    fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as i64
    }
}

const SEED: u64 = 0x5EED_1234;
/// Randomized inputs per property-style row.
const N: usize = 512;

// ===========================================================================
// Phase B — valid-path differential tests, one per CONFIGS.md row
// ===========================================================================

/// CONFIGS row 1 — `floors == 0`, the all-zero bit pattern / "empty" shape.
#[test]
fn configs_01_zero() {
    assert_same(0);
}

/// CONFIGS row 2 — `floors == 1`, the "one" shape.
#[test]
fn configs_02_one() {
    assert_same(1);
}

/// CONFIGS row 3 — randomized small positives `1..=255`.
#[test]
fn configs_03_small_positive() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..N {
        assert_same(rng.range_i64(1, 255) as i32);
    }
}

/// CONFIGS row 4 — randomized small negatives `-255..=-1`.
#[test]
fn configs_04_small_negative() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..N {
        assert_same(rng.range_i64(-255, -1) as i32);
    }
}

/// CONFIGS row 5 — randomized full-range positives `0..=INT_MAX`.
#[test]
fn configs_05_full_range_positive() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..N {
        assert_same(rng.range_i64(0, i32::MAX as i64) as i32);
    }
}

/// CONFIGS row 6 — randomized full-range negatives `INT_MIN..=-1`.
#[test]
fn configs_06_full_range_negative() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..N {
        assert_same(rng.range_i64(i32::MIN as i64, -1) as i32);
    }
}

/// CONFIGS row 7 — uniform over all 2^32 bit patterns reinterpreted as `int`.
#[test]
fn configs_07_arbitrary_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..N {
        assert_same(rng.next_u32() as i32);
    }
}

/// CONFIGS row 8 — byte/word boundary values and their ±1 neighbours.
#[test]
fn configs_08_boundaries() {
    let anchors: [i64; 17] = [
        0xff,
        0x100,
        0x7f,
        0x80,
        0xffff,
        0x10000,
        0x7fff,
        0x8000,
        0xffffff,
        0x100_0000,
        0x7fff_ffff,
        -0x8000_0000,
        -1,
        -256,
        -257,
        -65536,
        -65537,
    ];
    for a in anchors {
        for d in [-1i64, 0, 1] {
            let v = a.wrapping_add(d);
            // Stay inside `int`; wrap the way a C caller's `int` would.
            assert_same(v as i32);
        }
    }
}

/// CONFIGS row 9 — every single bit position of the dumped `floors` field.
#[test]
fn configs_09_powers_of_two() {
    for k in 0..=30 {
        let p: i32 = 1 << k;
        assert_same(p);
        assert_same(-p);
    }
    // The sign bit itself.
    assert_same(i32::MIN);
}

/// CONFIGS row 10 — repeated invocation of the same value through one handle;
/// output must be N identical lines (buffered-stdio shape).
#[test]
fn configs_10_repeated_invocation() {
    for v in [0i32, 7, -7, i32::MAX, i32::MIN] {
        const REPS: usize = 64;
        let c = capture(|| {
            let f = c_driver();
            for _ in 0..REPS {
                f(v);
            }
        });
        let r = capture(|| {
            let f = rust_driver();
            for _ in 0..REPS {
                f(v);
            }
        });
        assert_eq!(c, r, "repeated-call divergence for {v}");
        assert_eq!(c.len(), 33 * REPS, "unexpected repeated output length");
        let lines: Vec<&[u8]> = c.split(|&b| b == b'\n').filter(|s| !s.is_empty()).collect();
        assert_eq!(lines.len(), REPS);
        assert!(lines.iter().all(|l| *l == lines[0]));
    }
}

/// CONFIGS row 11 — a randomized *sequence* of different values through a
/// single `.so` handle, captured as one stream (catches leaked state).
#[test]
fn configs_11_sequenced_invocation() {
    let mut rng = Rng::new(SEED ^ 11);
    let seq: Vec<i32> = (0..256).map(|_| rng.next_u32() as i32).collect();

    let c = capture(|| {
        let f = c_driver();
        for &v in &seq {
            f(v);
        }
    });
    let r = capture(|| {
        let f = rust_driver();
        for &v in &seq {
            f(v);
        }
    });
    assert_eq!(c, r, "sequenced-call divergence");
    assert_eq!(c.len(), 33 * seq.len());
}

/// CONFIGS row 12 — interleaved C/Rust calls into the same redirected stdout.
#[test]
fn configs_12_interleaved() {
    let mut rng = Rng::new(SEED ^ 12);
    let seq: Vec<i32> = (0..128).map(|_| rng.next_u32() as i32).collect();

    let out = capture(|| {
        let cf = c_driver();
        let rf = rust_driver();
        for &v in &seq {
            cf(v);
            rf(v);
        }
    });
    let lines: Vec<&[u8]> = out.split(|&b| b == b'\n').filter(|s| !s.is_empty()).collect();
    assert_eq!(lines.len(), seq.len() * 2, "expected one line per call");
    for (i, pair) in lines.chunks(2).enumerate() {
        assert_eq!(
            pair[0],
            pair[1],
            "interleaved divergence at index {i} (value {}): C {} vs Rust {}",
            seq[i],
            show(pair[0]),
            show(pair[1])
        );
    }
}

/// CONFIGS row 13 — value delivered in a widened 64-bit argument register.
#[test]
fn configs_13_widened_argument_register() {
    let mut rng = Rng::new(SEED ^ 13);
    let mut vals: Vec<i64> = vec![
        0x0000_0000_8000_0000u64 as i64, // INT_MAX + 1 as a 64-bit value
        0x0000_0001_0000_0000u64 as i64, // pure upper-half garbage, low half 0
        0x0000_0001_dead_beefu64 as i64,
        0xffff_ffff_0000_0000u64 as i64,
        0x7fff_ffff_ffff_ffffu64 as i64,
        -1,
    ];
    for _ in 0..N {
        vals.push(rng.next_u64() as i64);
    }
    for v in vals {
        assert_same_wide(v);
    }
}

// ===========================================================================
// Phase C — error-path differential tests, one per ERRORS.md row
// ===========================================================================
//
// `driver` returns `void` and contains no rejection logic at all (see
// ERRORS.md for the mechanical derivation), so "same error/rejection" means:
// both `.so`s accept the input, neither traps, and both produce the identical
// byte stream. Each test asserts exactly that for its row's trigger.

/// ERRORS row 1 — `floors = 0` (zero-length analogue / all-zero pattern).
#[test]
fn error_surface_01_zero() {
    let c = capture(|| c_driver()(0));
    let r = capture(|| rust_driver()(0));
    assert_eq!(c, r);
    // The C accepts it: `floors` dumps as four zero bytes, then the fixed
    // `bedrooms = 3` and `bathrooms = 2.0` bytes. No error sentinel exists.
    assert_eq!(&c[..], b"00000000030000000000000000000040\n");
}

/// ERRORS row 2 — `INT_MAX`, top of the representable range.
#[test]
fn error_surface_02_int_max() {
    let c = capture(|| c_driver()(i32::MAX));
    let r = capture(|| rust_driver()(i32::MAX));
    assert_eq!(c, r);
    assert_eq!(&c[..], b"ffffff7f030000000000000000000040\n");
}

/// ERRORS row 3 — `INT_MIN`, bottom of the representable range.
#[test]
fn error_surface_03_int_min() {
    let c = capture(|| c_driver()(i32::MIN));
    let r = capture(|| rust_driver()(i32::MIN));
    assert_eq!(c, r);
    assert_eq!(&c[..], b"00000080030000000000000000000040\n");
}

/// ERRORS row 4 — `-1`, the classic error sentinel passed *in* as data.
#[test]
fn error_surface_04_minus_one() {
    let c = capture(|| c_driver()(-1));
    let r = capture(|| rust_driver()(-1));
    assert_eq!(c, r);
    assert_eq!(&c[..], b"ffffffff030000000000000000000040\n");
}

/// ERRORS row 5 — one step past `INT_MAX`, delivered as a 64-bit value.
/// Per the SysV AMD64 ABI the callee sees only the low 32 bits, so both must
/// behave exactly like `INT_MIN`, and must agree with each other.
#[test]
fn error_surface_05_one_past_int_max() {
    let v = 0x8000_0000i64;
    let c = capture(|| c_driver_wide()(v));
    let r = capture(|| rust_driver_wide()(v));
    assert_eq!(c, r);
    let min = capture(|| c_driver()(i32::MIN));
    assert_eq!(c, min, "low-32-bit truncation should match INT_MIN");
}

/// ERRORS row 6 — out-of-range "enum-like" integers across the FFI boundary.
/// The API declares no enum, so every bit pattern is a valid variant; both
/// implementations must accept and process them identically rather than
/// rejecting.
#[test]
fn error_surface_06_out_of_range_enum_values() {
    let odd: [i64; 10] = [
        0x7fff_ffff,
        0xdead_beefu32 as i64,
        0xcafe_babeu32 as i64,
        0xffff_ffffu32 as i64,
        0x8000_0000u32 as i64,
        -0x8000_0000,
        0x0f0f_0f0f,
        0xf0f0_f0f0u32 as i64,
        0x5555_5555,
        0xaaaa_aaaau32 as i64,
    ];
    for v in odd {
        // As a plain `int` (value with no "valid variant").
        assert_same(v as i32);
        // And widened, i.e. arriving with no valid variant *and* out of range.
        assert_same_wide(v);
    }
}

/// ERRORS rows 7 & 8 — structurally unreachable boundaries, asserted as such
/// so the omission stays explicit: the C `.so` exports exactly one symbol and
/// it takes no pointer and no length, so there is no null-pointer or
/// oversized-length path in either library.
#[test]
fn error_surface_07_08_unreachable_pointer_and_length_paths() {
    // No pointer/length-taking symbol exists to abuse. `print_hex` is `static`
    // in the C and private in the Rust, so neither is dynamically reachable.
    for name in [b"print_hex\0".as_slice(), b"house_t\0".as_slice()] {
        let in_c = unsafe { c_lib().get::<*const c_void>(name) }.is_ok();
        let in_rust = unsafe { rust_lib().get::<*const c_void>(name) }.is_ok();
        assert!(!in_c, "unexpected C export {:?}", show(name));
        assert_eq!(in_c, in_rust, "export parity for {:?}", show(name));
    }
}

// ===========================================================================
// Phase D — symbol parity, enforced as a test
// ===========================================================================

/// Every dynamic symbol the C `.so` defines must also be defined by the Rust
/// `.so`, with the exact same name. The diff must be empty.
#[test]
fn phase_d_symbol_parity() {
    fn defined(path: &std::path::Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", path.to_str().unwrap()])
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {path:?}");
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c = defined(&c_so_path());
    let r = defined(rust_so_path());
    assert!(!c.is_empty(), "C .so exported nothing — bad build?");
    assert!(c.contains(&"driver".to_string()));

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by C .so but missing from Rust .so: {missing:?}"
    );
}
