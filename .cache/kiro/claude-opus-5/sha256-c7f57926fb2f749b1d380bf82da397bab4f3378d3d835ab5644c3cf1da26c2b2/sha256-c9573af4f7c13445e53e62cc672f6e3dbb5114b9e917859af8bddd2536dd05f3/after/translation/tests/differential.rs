// Differential tests: C `libdriver.so` vs Rust `libdriver.so`.
//
// Both libraries are loaded through `libloading` and driven ONLY through their
// exported dynamic symbols, exactly as an external consumer would. The Rust
// functions are never called directly, so the `#[no_mangle] extern "C"` export
// wrapper is under test too.
//
// `driver` reports its result on stdout via `printf(3)`, so "compare the
// outputs" means: redirect fd 1 to a temp file, call one library, flush the
// shared libc stdout buffer, and read the bytes back. Comparison is
// byte-for-byte.

use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed to capture fd 1
// ---------------------------------------------------------------------------

extern "C" {
    fn fflush(stream: *mut core::ffi::c_void) -> core::ffi::c_int;
    fn dup(oldfd: core::ffi::c_int) -> core::ffi::c_int;
    fn dup2(oldfd: core::ffi::c_int, newfd: core::ffi::c_int) -> core::ffi::c_int;
    fn close(fd: core::ffi::c_int) -> core::ffi::c_int;
}

/// fd-1 redirection is process-global, and the cargo test harness runs test
/// functions on parallel threads. Every capture must hold this lock.
fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Run `f`, returning everything it wrote to file descriptor 1.
///
/// Two sources of contamination have to be handled, or unrelated bytes end up
/// attributed to the library under test:
///  * libc's `stdout` FILE buffer (what `printf` in both `.so`s writes into);
///  * Rust's own `std::io::stdout()` LineWriter buffer, which can be holding a
///    partial line (the test runner's `"name ... "` progress text has no
///    trailing newline, so it sits there until something flushes it).
/// Both are drained before the redirect and after the call.
fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{:p}.bin",
        std::process::id(),
        &guard as *const _
    ));
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("open capture temp file");

    /// Restores fd 1 even if `f` panics, so one failing assertion cannot
    /// swallow the rest of the run's output.
    struct Fd1Guard(core::ffi::c_int);
    impl Drop for Fd1Guard {
        fn drop(&mut self) {
            unsafe {
                let _ = std::io::Write::flush(&mut std::io::stdout());
                fflush(std::ptr::null_mut());
                dup2(self.0, 1);
                close(self.0);
            }
        }
    }

    let ret;
    {
        let _restore;
        unsafe {
            std::io::Write::flush(&mut std::io::stdout()).ok();
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            _restore = Fd1Guard(saved);
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");
        }
        ret = f();
        // `_restore` drops here: flush both buffers, then put fd 1 back.
    }

    let mut out = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("seek capture file");
    file.read_to_end(&mut out).expect("read capture file");
    drop(file);
    let _ = std::fs::remove_file(&path);

    drop(guard);
    (ret, out)
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = crate_root().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // Prefer the release artifact (what actually ships); fall back to the
    // debug artifact that `cargo test` itself produces.
    for rel in ["target/release/libdriver.so", "target/debug/libdriver.so"] {
        let p = crate_root().join(rel);
        if p.exists() {
            return p;
        }
    }
    panic!("Rust shared library not found; run `cargo build --release` in translation/");
}

struct Libs {
    c: Library,
    rust: Library,
}

fn libs() -> &'static Libs {
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(|| unsafe {
        Libs {
            c: Library::new(c_so_path()).expect("dlopen C libdriver.so"),
            rust: Library::new(rust_so_path()).expect("dlopen Rust libdriver.so"),
        }
    })
}

type DriverFn = unsafe extern "C" fn(core::ffi::c_int);

fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("C driver symbol") }
}

fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rust.get(b"driver\0").expect("Rust driver symbol") }
}

// ---------------------------------------------------------------------------
// The core differential primitive
// ---------------------------------------------------------------------------

/// Call `driver(x)` in both libraries and return (c_output, rust_output).
fn both(x: i32) -> (Vec<u8>, Vec<u8>) {
    let cd = c_driver();
    let rd = rust_driver();
    let ((), c_out) = capture(|| unsafe { cd(x) });
    let ((), r_out) = capture(|| unsafe { rd(x) });
    (c_out, r_out)
}

#[track_caller]
fn assert_same(x: i32, label: &str) {
    let (c_out, r_out) = both(x);
    assert_eq!(
        c_out,
        r_out,
        "[{label}] driver({x} / {x:#010x}) diverged\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
    // Sanity: the C library must actually have produced output, otherwise a
    // broken capture harness would make every comparison trivially "equal".
    assert!(
        !c_out.is_empty(),
        "[{label}] capture harness produced no C output for driver({x}) — \
         the comparison would be vacuous"
    );
}

#[track_caller]
fn assert_same_many(xs: impl IntoIterator<Item = i32>, label: &str) {
    let mut n = 0usize;
    for x in xs {
        assert_same(x, label);
        n += 1;
    }
    assert!(n > 0, "[{label}] exercised zero inputs");
}

/// Deterministic SplitMix64 — fixed seed, so every run uses the same inputs.
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
    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
}

const BOUNDARIES: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    0x7fff_ffffu32 as i32,
    0x8000_0000u32 as i32,
    0xffff_ffffu32 as i32,
    0x0000_ffffu32 as i32,
    0xffff_0000u32 as i32,
    0x00ff_00ffu32 as i32,
    0xff00_ff00u32 as i32,
    0x0f0f_0f0fu32 as i32,
    0xf0f0_f0f0u32 as i32,
    0x0102_0304u32 as i32,
    0x0403_0201u32 as i32,
    0x1000_0000u32 as i32,
    0x0000_000fu32 as i32,
];

// ===========================================================================
// PHASE B — valid-path differential tests, one per CONFIGS.md row
// ===========================================================================

/// C1 — no options exist; all-bytes-zero input.
fn cfg01_zero() {
    assert_same(0, "C1");
}

/// C2 — a single set bit walked through every bit of every byte.
fn cfg02_single_bit_set() {
    assert_same_many((0..32).map(|k| (1u32 << k) as i32), "C2");
}

/// C3 — a single cleared bit (complement of C2).
fn cfg03_single_bit_clear() {
    assert_same_many((0..32).map(|k| !(1u32 << k) as i32), "C3");
}

/// C4 — every one of the 256 byte values in byte 0, incl. all 16 values
/// below 0x10 that require `%02x`'s leading zero.
fn cfg04_every_byte_value_in_byte0() {
    assert_same_many((0u32..=0xff).map(|b| b as i32), "C4");
}

/// C5 — every byte value in every byte *position*, so a wrong index or stride
/// in the loop is visible.
fn cfg05_every_byte_value_in_every_position() {
    for shift in [8u32, 16, 24] {
        assert_same_many((0u32..=0xff).map(|b| (b << shift) as i32), "C5");
    }
}

/// C6 — randomized positive values.
fn cfg06_random_positive() {
    let mut rng = Rng::new(0xC0FFEE_1234_5678);
    assert_same_many(
        (0..4096).map(|_| (rng.next_u64() as u32 & 0x7fff_ffff) as i32),
        "C6",
    );
}

/// C7 — randomized negative values (sign bit set).
fn cfg07_random_negative() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_0001);
    assert_same_many(
        (0..4096).map(|_| ((rng.next_u64() as u32) | 0x8000_0000) as i32),
        "C7",
    );
}

/// C8 — unconstrained property test over all 2^32 bit patterns.
fn cfg08_random_full_range() {
    let mut rng = Rng::new(0x5EED_0000_0000_0042);
    assert_same_many((0..8192).map(|_| rng.next_i32()), "C8");
}

/// C9 — the hand-picked boundary values.
fn cfg09_boundaries() {
    assert_same_many(BOUNDARIES.iter().copied(), "C9");
}

/// C10 — byte-order sensitivity: feed `x` and its byte-swap. If the Rust loop
/// walked the object representation in the opposite direction, `driver(x)` in
/// Rust would equal `driver(bswap(x))` in C, which this catches.
fn cfg10_byte_order_pairs() {
    let mut rng = Rng::new(0xB5_0000_0000_AA55);
    for _ in 0..1024 {
        let x = rng.next_i32();
        let s = (x as u32).swap_bytes() as i32;
        assert_same(x, "C10");
        assert_same(s, "C10");

        // Asymmetric bit patterns must produce different output for x and its
        // byte-swap; otherwise the direction check above would be vacuous.
        if (x as u32).swap_bytes() != x as u32 {
            let (cx, _) = both(x);
            let (cs, _) = both(s);
            assert_ne!(
                cx, cs,
                "C10: byte-swapped inputs {x:#010x}/{s:#010x} produced identical output, \
                 so this row cannot detect a reversed traversal"
            );
        }
    }
}

/// C11 — call-count shape: zero calls, one call, then many consecutive calls
/// against the same loaded handle.
fn cfg11_zero_one_many_calls() {
    let cd = c_driver();
    let rd = rust_driver();

    // zero calls: both must emit nothing at all
    let ((), c0) = capture(|| {});
    let ((), r0) = capture(|| {});
    assert_eq!(c0, r0, "C11: empty capture mismatch");
    assert!(c0.is_empty(), "C11: empty capture was not empty");

    // one call
    assert_same(0x1234_5678, "C11/one");

    // many calls without reloading — catches any hidden per-library state
    let mut rng = Rng::new(0x11_2233_4455_6677);
    let xs: Vec<i32> = (0..256).map(|_| rng.next_i32()).collect();
    let ((), c_many) = capture(|| unsafe {
        for &x in &xs {
            cd(x);
        }
    });
    let ((), r_many) = capture(|| unsafe {
        for &x in &xs {
            rd(x);
        }
    });
    assert_eq!(c_many, r_many, "C11: 256 consecutive calls diverged");
    assert_eq!(
        c_many.len(),
        256 * 9,
        "C11: expected 9 bytes (8 hex + newline) per call"
    );
}

/// C12 — interleaving and ordering: alternate the two libraries inside one
/// captured stream, both C-first and Rust-first.
fn cfg12_interleaved_and_reversed_order() {
    let cd = c_driver();
    let rd = rust_driver();
    let mut rng = Rng::new(0x0FED_CBA9_8765_4321);

    for _ in 0..256 {
        let x = rng.next_i32();

        // C then Rust in one stream: the two halves must be identical.
        let ((), cr) = capture(|| unsafe {
            cd(x);
            rd(x);
        });
        // Rust then C in one stream.
        let ((), rc) = capture(|| unsafe {
            rd(x);
            cd(x);
        });
        assert_eq!(cr, rc, "C12: output depends on call order for {x:#010x}");

        let half = cr.len() / 2;
        assert_eq!(cr.len() % 2, 0, "C12: odd-length interleaved output");
        assert_eq!(
            &cr[..half],
            &cr[half..],
            "C12: C and Rust halves differ for {x:#010x}"
        );
    }
}

/// C13 — both `.so`s resident in one process, driven through the same fd 1.
/// This is the condition every other row already runs under; assert it
/// explicitly, including that the two handles are genuinely distinct objects.
fn cfg13_both_libraries_resident_same_process() {
    let cd = c_driver();
    let rd = rust_driver();
    let c_addr = *cd as usize;
    let r_addr = *rd as usize;
    assert_ne!(
        c_addr, r_addr,
        "C13: C and Rust `driver` resolved to the same address — only one \
         library is actually loaded, so no differential testing is happening"
    );
    assert_same(0x0BAD_F00Du32 as i32, "C13");
}

// ===========================================================================
// PHASE C — error-path differential tests, one per ERRORS.md row
// ===========================================================================

/// E1 — zero.
fn err_zero() {
    let (c, r) = both(0);
    assert_eq!(c, r, "E1");
    assert_eq!(c, b"00000000\n", "E1: unexpected C result");
}

/// E2 — INT_MAX, one below signed overflow.
fn err_int_max() {
    let (c, r) = both(i32::MAX);
    assert_eq!(c, r, "E2");
}

/// E3 — INT_MIN, the most negative value.
fn err_int_min() {
    let (c, r) = both(i32::MIN);
    assert_eq!(c, r, "E3");
}

/// E4 — all bits set.
fn err_minus_one() {
    let (c, r) = both(-1);
    assert_eq!(c, r, "E4");
    assert_eq!(c, b"ffffffff\n", "E4: unexpected C result");
}

/// E5 — one step past INT_MAX, i.e. the wrapped bit pattern 0x80000000.
/// `int` has no trap representation, so C must treat this exactly like E3.
fn err_one_past_int_max() {
    let wrapped = i32::MAX.wrapping_add(1);
    let (c, r) = both(wrapped);
    assert_eq!(c, r, "E5");
    let (c_min, _) = both(i32::MIN);
    assert_eq!(c, c_min, "E5: wrapped INT_MAX+1 should equal INT_MIN in C");
}

/// E6 — one step below INT_MIN, i.e. the wrapped bit pattern 0x7fffffff.
fn err_one_before_int_min() {
    let wrapped = i32::MIN.wrapping_sub(1);
    let (c, r) = both(wrapped);
    assert_eq!(c, r, "E6");
    let (c_max, _) = both(i32::MAX);
    assert_eq!(c, c_max, "E6: wrapped INT_MIN-1 should equal INT_MAX in C");
}

/// E7 — out-of-range enum-like values. A C enum parameter accepts any `int`,
/// so a caller can pass a value with no valid variant. This API has no enum
/// parameter, so the equivalent real input is an arbitrary sentinel `int`;
/// neither library may validate or reject it.
fn err_bogus_enum_like_values() {
    let bogus: &[i32] = &[
        0xdead_beefu32 as i32,
        -999_999,
        0x7f7f_7f7fu32 as i32,
        -1_000_000_000,
        1_000_000_000,
        i32::MIN,
        i32::MAX,
        0xcccc_ccccu32 as i32,
        0xaaaa_aaaau32 as i32,
        0x5555_5555u32 as i32,
        -128,
        -129,
        255,
        256,
        65_535,
        65_536,
    ];
    for &x in bogus {
        let (c, r) = both(x);
        assert_eq!(c, r, "E7: diverged on bogus value {x:#010x}");
        // No rejection channel exists: the C function must still emit a full
        // 8-hex-digit line rather than erroring out or emitting nothing.
        assert_eq!(
            c.len(),
            9,
            "E7: C rejected {x:#010x} instead of formatting it"
        );
    }
}

/// E8 — garbage in the upper 32 bits of the argument register. The C ABI says
/// only the low 32 bits of an `int` argument are significant; a caller holding
/// a wider value can leave the top half dirty. Both libraries must ignore it
/// identically.
fn err_dirty_upper_register_bits() {
    type WideFn = unsafe extern "C" fn(u64);
    let cd = c_driver();
    let rd = rust_driver();
    // Re-type the same loaded addresses as taking a 64-bit argument so the
    // upper half of the register is under our control.
    let c_wide: WideFn = unsafe { std::mem::transmute::<DriverFn, WideFn>(*cd) };
    let r_wide: WideFn = unsafe { std::mem::transmute::<DriverFn, WideFn>(*rd) };

    let mut rng = Rng::new(0xDEAD_0000_BEEF_0000);
    for _ in 0..256 {
        let low = rng.next_u64() as u32;
        let high = rng.next_u64() as u32;
        let wide = ((high as u64) << 32) | low as u64;

        let ((), c_out) = capture(|| unsafe { c_wide(wide) });
        let ((), r_out) = capture(|| unsafe { r_wide(wide) });
        assert_eq!(c_out, r_out, "E8: diverged on wide argument {wide:#018x}");

        // And both must match the clean 32-bit call.
        let (c_clean, _) = both(low as i32);
        assert_eq!(
            c_out, c_clean,
            "E8: C used the dirty upper bits of {wide:#018x}"
        );
        assert_eq!(
            r_out, c_clean,
            "E8: Rust used the dirty upper bits of {wide:#018x}"
        );
    }
}

/// E9 — repeated / back-to-back calls. There is no state in the C library to
/// corrupt; verify neither library has a hidden static buffer by checking that
/// a long run equals the concatenation of the individual calls.
fn err_repeated_calls_no_state() {
    let cd = c_driver();
    let rd = rust_driver();
    let mut rng = Rng::new(0x9999_8888_7777_6666);
    let xs: Vec<i32> = (0..128).map(|_| rng.next_i32()).collect();

    let ((), c_run) = capture(|| unsafe {
        for &x in &xs {
            cd(x);
        }
    });
    let ((), r_run) = capture(|| unsafe {
        for &x in &xs {
            rd(x);
        }
    });
    assert_eq!(c_run, r_run, "E9: repeated-call streams diverged");

    let mut expected = Vec::new();
    for &x in &xs {
        let ((), one) = capture(|| unsafe { cd(x) });
        expected.extend_from_slice(&one);
    }
    assert_eq!(
        c_run, expected,
        "E9: batched output differs from concatenated single calls"
    );
}

/// ERRORS.md note — null-pointer and zero/oversized-length inputs are not
/// expressible against this ABI, because the only pointer/length pair lives in
/// the `static` (non-exported) `print_hex`. Assert that inexpressibility rather
/// than silently skipping it.
fn err_no_pointer_or_length_params_reachable() {
    let l = libs();
    for name in [b"print_hex\0".as_ref(), b"driver_print_hex\0".as_ref()] {
        let c_sym = unsafe { l.c.get::<*const core::ffi::c_void>(name) };
        let r_sym = unsafe { l.rust.get::<*const core::ffi::c_void>(name) };
        assert!(
            c_sym.is_err(),
            "C .so unexpectedly exports {:?}; ERRORS.md must gain \
             null-pointer / length rows for it",
            String::from_utf8_lossy(name)
        );
        assert!(
            r_sym.is_err(),
            "Rust .so exports {:?} but the C .so does not — symbol parity broken",
            String::from_utf8_lossy(name)
        );
    }
}

// ===========================================================================
// PHASE D — symbol parity, enforced from inside the test suite
// ===========================================================================

/// Every symbol the C `.so` defines must also be defined by the Rust `.so`,
/// under the exact same name.
fn symbol_parity_defined() {
    fn defined(path: &PathBuf) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", path.to_str().unwrap()])
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {}", path.display());
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(str::to_string))
            .filter(|s| {
                // Toolchain/runtime-emitted symbols both objects get for free.
                !matches!(
                    s.as_str(),
                    "_ITM_deregisterTMCloneTable"
                        | "_ITM_registerTMCloneTable"
                        | "__gmon_start__"
                ) && !s.starts_with("__cxa_")
                    && !s.starts_with("_edata")
                    && !s.starts_with("_end")
                    && !s.starts_with("_fini")
                    && !s.starts_with("_init")
            })
            .collect()
    }

    let c = defined(&c_so_path());
    let r = defined(&rust_so_path());
    assert!(!c.is_empty(), "nm reported no defined symbols for the C .so");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}"
    );

    // Also flag anything the Rust .so exports that C does not, restricted to
    // names that look like library API rather than Rust runtime glue.
    let extra: Vec<&String> = r
        .iter()
        .filter(|s| !c.contains(s) && !s.starts_with('_') && !s.contains("rust"))
        .collect();
    assert!(
        extra.is_empty(),
        "Rust .so exports non-runtime symbols absent from the C .so: {extra:?}"
    );
}

/// The Rust `.so` must not carry unresolved non-libc dependencies: `dlopen`
/// with RTLD_NOW would fail if it did, and `libs()` already proves the lazy
/// case. Force the eager check.
fn no_unresolved_symbols_on_eager_load() {
    // libloading's `Library::new` uses RTLD_LAZY|RTLD_LOCAL. Ask for RTLD_NOW
    // so every undefined symbol must resolve at load time.
    const RTLD_NOW: i32 = 0x2;
    const RTLD_LOCAL: i32 = 0;
    for p in [c_so_path(), rust_so_path()] {
        let lib = unsafe {
            libloading::os::unix::Library::open(Some(&p), RTLD_NOW | RTLD_LOCAL)
        };
        assert!(
            lib.is_ok(),
            "RTLD_NOW load of {} failed (unresolved symbols): {:?}",
            p.display(),
            lib.err()
        );
    }
}

// ===========================================================================
// Sequential runner (`harness = false`)
// ===========================================================================
//
// The default libtest harness runs test functions on parallel threads and
// writes its own progress text to fd 1. Since `driver` reports its result on
// stdout, capturing it means redirecting fd 1 process-wide -- so any concurrent
// harness write lands inside a capture and is misread as a divergence. The
// cases here are therefore driven by this sequential runner, and all runner
// output goes to stderr so it can never contaminate a capture.

type Case = (&'static str, fn());

const CASES: &[Case] = &[
    // Phase B -- CONFIGS.md rows C1..C13
    ("cfg01_zero", cfg01_zero),
    ("cfg02_single_bit_set", cfg02_single_bit_set),
    ("cfg03_single_bit_clear", cfg03_single_bit_clear),
    ("cfg04_every_byte_value_in_byte0", cfg04_every_byte_value_in_byte0),
    ("cfg05_every_byte_value_in_every_position", cfg05_every_byte_value_in_every_position),
    ("cfg06_random_positive", cfg06_random_positive),
    ("cfg07_random_negative", cfg07_random_negative),
    ("cfg08_random_full_range", cfg08_random_full_range),
    ("cfg09_boundaries", cfg09_boundaries),
    ("cfg10_byte_order_pairs", cfg10_byte_order_pairs),
    ("cfg11_zero_one_many_calls", cfg11_zero_one_many_calls),
    ("cfg12_interleaved_and_reversed_order", cfg12_interleaved_and_reversed_order),
    ("cfg13_both_libraries_resident_same_process", cfg13_both_libraries_resident_same_process),
    // Phase C -- ERRORS.md rows E1..E9 + inexpressibility check
    ("err_zero", err_zero),
    ("err_int_max", err_int_max),
    ("err_int_min", err_int_min),
    ("err_minus_one", err_minus_one),
    ("err_one_past_int_max", err_one_past_int_max),
    ("err_one_before_int_min", err_one_before_int_min),
    ("err_bogus_enum_like_values", err_bogus_enum_like_values),
    ("err_dirty_upper_register_bits", err_dirty_upper_register_bits),
    ("err_repeated_calls_no_state", err_repeated_calls_no_state),
    ("err_no_pointer_or_length_params_reachable", err_no_pointer_or_length_params_reachable),
    // Phase D -- symbol parity
    ("symbol_parity_defined", symbol_parity_defined),
    ("no_unresolved_symbols_on_eager_load", no_unresolved_symbols_on_eager_load),
];

fn main() {
    // Accept a libtest-style substring filter so `cargo test -- <name>` works.
    let filters: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .collect();

    let selected: Vec<&Case> = CASES
        .iter()
        .filter(|(n, _)| filters.is_empty() || filters.iter().any(|f| n.contains(f.as_str())))
        .collect();

    eprintln!(
        "\nrunning {} differential cases (sequential)\n  C   : {}\n  Rust: {}\n",
        selected.len(),
        c_so_path().display(),
        rust_so_path().display()
    );

    let mut failed: Vec<(&str, String)> = Vec::new();
    for (name, f) in &selected {
        eprint!("test {name} ... ");
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        std::panic::set_hook(prev);
        match r {
            Ok(()) => eprintln!("ok"),
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "<non-string panic payload>".to_string()
                };
                eprintln!("FAILED");
                failed.push((name, msg));
            }
        }
    }

    eprintln!();
    if failed.is_empty() {
        eprintln!("test result: ok. {} passed; 0 failed\n", selected.len());
    } else {
        for (name, msg) in &failed {
            eprintln!("---- {name} ----\n{msg}\n");
        }
        eprintln!(
            "test result: FAILED. {} passed; {} failed\n",
            selected.len() - failed.len(),
            failed.len()
        );
        std::process::exit(1);
    }
}
