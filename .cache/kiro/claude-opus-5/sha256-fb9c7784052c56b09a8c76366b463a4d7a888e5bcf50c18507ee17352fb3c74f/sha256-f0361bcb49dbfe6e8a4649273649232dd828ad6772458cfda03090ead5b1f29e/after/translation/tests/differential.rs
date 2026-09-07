// Differential tests: load BOTH the C `.so` and the Rust `.so` with libloading
// and compare the bytes each writes to stdout for identical inputs.
//
// Nothing here calls the Rust implementation directly; every invocation goes
// through `libloading`'s lookup of the exported `driver` symbol, so the
// `#[no_mangle] extern "C"` wrapper is exercised exactly as an external C
// caller would exercise it.
//
// `harness = false` (see Cargo.toml): the suite redirects the process-wide
// fd 1 to capture each library's stdout, so it must be the only writer to
// fd 1 while a call is in flight. libtest's parallel progress output would
// otherwise land inside a capture. All diagnostics here go to stderr.

use std::ffi::c_char;
use std::ffi::c_int;
use std::io::Read;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture and for the locale rows
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

const LC_ALL: c_int = 6;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // Prefer the release artifact (the one whose symbols were diffed); fall
    // back to debug so `cargo test` works without a prior release build.
    let release = manifest_dir().join("target/release/libdriver.so");
    if release.exists() {
        return release;
    }
    manifest_dir().join("target/debug/libdriver.so")
}

struct Libs {
    c: Library,
    rust: Library,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(cp.exists(), "C .so not found at {cp:?} — build c_src first");
        assert!(
            rp.exists(),
            "Rust .so not found at {rp:?} — run `cargo build --release`"
        );
        eprintln!("  C    .so: {}", cp.display());
        eprintln!("  Rust .so: {}", rp.display());
        unsafe {
            Libs {
                c: Library::new(&cp).expect("load C .so"),
                rust: Library::new(&rp).expect("load Rust .so"),
            }
        }
    })
}

/// `void driver(char)` as declared in the header.
type DriverFn = unsafe extern "C" fn(c_char);
/// The same entry point viewed through a promoted `int` prototype — what a C
/// caller that passes an out-of-range value actually emits.
type DriverIntFn = unsafe extern "C" fn(c_int);

fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("C driver symbol") }
}
fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rust.get(b"driver\0").expect("Rust driver symbol") }
}
fn c_driver_int() -> Symbol<'static, DriverIntFn> {
    unsafe { libs().c.get(b"driver\0").expect("C driver symbol") }
}
fn rust_driver_int() -> Symbol<'static, DriverIntFn> {
    unsafe { libs().rust.get(b"driver\0").expect("Rust driver symbol") }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Runs `f` with fd 1 pointed at a temp file and returns the bytes written.
fn capture(f: impl FnOnce()) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!("driver_capture_{}.txt", std::process::id()));
    let bytes;
    unsafe {
        let file = std::fs::File::create(&path).expect("create capture file");
        // Flush anything already pending in libc's *and* Rust's buffers so it
        // lands on the real stdout rather than inside our capture.
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        drop(file);

        let mut buf = Vec::new();
        std::fs::File::open(&path)
            .expect("reopen capture file")
            .read_to_end(&mut buf)
            .expect("read capture file");
        bytes = buf;
    }
    let _ = std::fs::remove_file(&path);
    bytes
}

fn c_output(c: c_char) -> Vec<u8> {
    let f = c_driver();
    capture(|| unsafe { f(c) })
}

fn rust_output(c: c_char) -> Vec<u8> {
    let f = rust_driver();
    capture(|| unsafe { f(c) })
}

fn c_output_int(v: c_int) -> Vec<u8> {
    let f = c_driver_int();
    capture(|| unsafe { f(v) })
}

fn rust_output_int(v: c_int) -> Vec<u8> {
    let f = rust_driver_int();
    capture(|| unsafe { f(v) })
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// The 14 field labels `driver` prints, in order.
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

/// Sanity check that the capture really caught `driver`'s 14 fields and
/// nothing else (guards against the capture silently picking up a foreign
/// write to fd 1, which would make a byte-equality pass meaningless).
///
/// A line count is deliberately *not* asserted: `printf("%c", ...)` emits the
/// raw byte, so for c == '\n' the output legitimately contains 16 newlines.
fn assert_well_formed(bytes: &[u8], ctx: &str, what: &str) {
    let text = String::from_utf8_lossy(bytes).to_string();
    let mut pos = 0usize;
    for label in LABELS {
        match text[pos..].find(label) {
            Some(off) => pos += off + label.len(),
            None => panic!(
                "{ctx}: {what} output is missing the field {label:?} in order: {}",
                show(bytes)
            ),
        }
    }
}

/// Core assertion: identical stdout bytes for the same `char`.
fn assert_same(c: c_char, ctx: &str) {
    let got_c = c_output(c);
    let got_r = rust_output(c);
    assert!(
        !got_c.is_empty(),
        "{ctx}: C produced no output for c={c} (0x{:02x}) — capture broken",
        c as u8
    );
    assert_well_formed(&got_c, ctx, "C");
    assert_well_formed(&got_r, ctx, "Rust");
    assert_eq!(
        got_c,
        got_r,
        "{ctx}: divergence for c={c} (0x{:02x})\n  C   : {}\n  Rust: {}",
        c as u8,
        show(&got_c),
        show(&got_r)
    );
}

fn assert_same_int(v: c_int, ctx: &str) {
    let got_c = c_output_int(v);
    let got_r = rust_output_int(v);
    assert_eq!(
        got_c,
        got_r,
        "{ctx}: divergence for int arg {v} (0x{v:08x})\n  C   : {}\n  Rust: {}",
        show(&got_c),
        show(&got_r)
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Rng(0x5EED_C0DE_1234_5678)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    fn pick(&mut self, pool: &[u8]) -> c_char {
        pool[self.below(pool.len())] as c_char
    }
    fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
}

/// Every member of `pool` at least once, then `n` randomized draws from it.
fn fuzz_pool(pool: &[u8], n: usize, ctx: &str) {
    assert!(!pool.is_empty(), "{ctx}: empty pool");
    let mut rng = Rng::new();
    for &b in pool {
        assert_same(b as c_char, ctx);
    }
    for _ in 0..n {
        let v = rng.pick(pool);
        assert_same(v, ctx);
    }
}

fn range_pool(lo: u8, hi: u8) -> Vec<u8> {
    (lo..=hi).collect()
}

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

/// CONFIGS.md row 1 — exhaustive sweep over all 256 char bit patterns.
fn config_row_01_exhaustive_all_256() {
    for b in 0u16..=255 {
        assert_same(b as u8 as c_char, "row01/exhaustive");
    }
}

/// CONFIGS.md row 2 — positive control characters (cntrl bit only).
fn config_row_02_control_chars() {
    let mut pool: Vec<u8> = (0x01u8..=0x08).collect();
    pool.extend(0x0Eu8..=0x1F);
    fuzz_pool(&pool, 200, "row02/control");
}

/// CONFIGS.md row 3 — whitespace family, space/blank interaction.
fn config_row_03_whitespace() {
    let pool: Vec<u8> = vec![0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x20];
    fuzz_pool(&pool, 200, "row03/whitespace");
}

/// CONFIGS.md row 4 — decimal digits.
fn config_row_04_digits() {
    fuzz_pool(&range_pool(b'0', b'9'), 200, "row04/digits");
}

/// CONFIGS.md row 5 — hex letters, both cases.
fn config_row_05_hex_letters() {
    let mut pool = range_pool(b'a', b'f');
    pool.extend(range_pool(b'A', b'F'));
    fuzz_pool(&pool, 200, "row05/hex-letters");
}

/// CONFIGS.md row 6 — non-hex lowercase.
fn config_row_06_lowercase_non_hex() {
    fuzz_pool(&range_pool(b'g', b'z'), 200, "row06/lower-non-hex");
}

/// CONFIGS.md row 7 — non-hex uppercase.
fn config_row_07_uppercase_non_hex() {
    fuzz_pool(&range_pool(b'G', b'Z'), 200, "row07/upper-non-hex");
}

/// CONFIGS.md row 8 — every printable non-alphanumeric (punct/symbol).
fn config_row_08_punctuation() {
    let pool: Vec<u8> = (0x21u8..=0x7E)
        .filter(|b| !b.is_ascii_alphanumeric())
        .collect();
    assert_eq!(pool.len(), 32, "expected 32 ASCII punctuation characters");
    fuzz_pool(&pool, 300, "row08/punct");
}

/// CONFIGS.md row 9 — high-bit values, i.e. negative `char` table indices.
fn config_row_09_negative_chars() {
    fuzz_pool(&range_pool(0x80, 0xFF), 300, "row09/negative");
}

/// CONFIGS.md row 10 — uniform random over the whole char domain, many draws.
fn config_row_10_uniform_random_fuzz() {
    let mut rng = Rng::new();
    for _ in 0..4096 {
        let v = (rng.next_u64() & 0xFF) as u8 as c_char;
        assert_same(v, "row10/uniform-fuzz");
    }
}

/// CONFIGS.md row 11 — promoted out-of-range `int` arguments.
fn config_row_11_out_of_range_int_args() {
    for v in [
        256, 257, -129, -256, 1000, 0x1234, 0x100, 0xFF00, i32::MAX, i32::MIN, -1, 0, 65535,
    ] {
        assert_same_int(v, "row11/out-of-range-int");
    }
    let mut rng = Rng::new();
    for _ in 0..256 {
        assert_same_int(rng.i32(), "row11/out-of-range-int-fuzz");
    }
}

/// CONFIGS.md row 12 — baseline ambient locale, full sweep.
fn config_row_12_locale_baseline() {
    unsafe {
        setlocale(LC_ALL, c"C".as_ptr());
    }
    for b in 0u16..=255 {
        assert_same(b as u8 as c_char, "row12/locale-C");
    }
}

/// CONFIGS.md row 13 — ambient `LC_ALL=en_US.UTF-8`; `driver`'s own
/// `setlocale(LC_ALL,"C")` must override it identically in both libraries.
fn config_row_13_locale_env_utf8() {
    let prev = std::env::var("LC_ALL").ok();
    std::env::set_var("LC_ALL", "en_US.UTF-8");
    for b in 0u16..=255 {
        assert_same(b as u8 as c_char, "row13/locale-env-utf8");
    }
    match prev {
        Some(v) => std::env::set_var("LC_ALL", v),
        None => std::env::remove_var("LC_ALL"),
    }
    unsafe {
        setlocale(LC_ALL, c"C".as_ptr());
    }
}

/// CONFIGS.md row 14 — process locale actively switched to a multibyte locale
/// before the call (falls back gracefully if the locale is not installed).
fn config_row_14_locale_switched_multibyte() {
    let candidates = [c"en_US.UTF-8", c"C.UTF-8", c"en_US.utf8", c""];
    let mut switched: Option<&str> = None;
    for cand in candidates {
        unsafe {
            if !setlocale(LC_ALL, cand.as_ptr()).is_null() {
                switched = Some(cand.to_str().unwrap());
                break;
            }
        }
    }
    eprint!("(locale -> {switched:?}) ");
    for b in 0u16..=255 {
        assert_same(b as u8 as c_char, "row14/locale-multibyte");
    }
    unsafe {
        setlocale(LC_ALL, c"C".as_ptr());
    }
}

/// CONFIGS.md row 15 — interleaving and repetition: no state leaks between the
/// two libraries and `driver` is idempotent across calls.
fn config_row_15_interleave_and_repeat() {
    // Reverse call order: Rust first, then C.
    for b in 0u16..=255 {
        let c = b as u8 as c_char;
        let r = rust_output(c);
        let cc = c_output(c);
        assert_eq!(
            cc,
            r,
            "row15/reverse-order divergence at 0x{b:02x}\n  C   : {}\n  Rust: {}",
            show(&cc),
            show(&r)
        );
    }
    // Repetition on a fixed set of inputs.
    for c in [0i8, 9, 32, 65, 97, 127, -1, -128, 48] {
        let mut c_prev: Option<Vec<u8>> = None;
        let mut r_prev: Option<Vec<u8>> = None;
        for iter in 0..5 {
            let cc = c_output(c);
            let rr = rust_output(c);
            assert_eq!(cc, rr, "row15/repeat divergence c={c} iter={iter}");
            if let Some(p) = &c_prev {
                assert_eq!(p, &cc, "row15: C output not idempotent for c={c}");
            }
            if let Some(p) = &r_prev {
                assert_eq!(p, &rr, "row15: Rust output not idempotent for c={c}");
            }
            c_prev = Some(cc);
            r_prev = Some(rr);
        }
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows
// ===========================================================================

/// ERRORS.md row 1 — NUL.
fn error_row_01_nul() {
    assert_same(0, "err01/nul");
    let out = c_output(0);
    let text = String::from_utf8_lossy(&out).to_string();
    assert_eq!(
        text.lines().count(),
        14,
        "err01: expected 14 output lines, got {text:?}"
    );
    // glibc's macros yield the raw `_IScntrl` bit rather than a normalised 1.
    assert!(
        text.contains("control: 2"),
        "err01: expected the raw _IScntrl bit, got {text:?}"
    );
}

/// ERRORS.md row 2 — DEL (0x7F), top of the positive char range.
fn error_row_02_del_127() {
    assert_same(127, "err02/del");
    let text = String::from_utf8_lossy(&c_output(127)).to_string();
    assert!(
        text.contains("control: 2"),
        "err02: DEL should classify as control, got {text:?}"
    );
}

/// ERRORS.md row 3 — 0xFF, bit-identical to EOF, the ctype sentinel.
fn error_row_03_eof_minus_one() {
    assert_same(-1, "err03/eof");
    let out = c_output(-1);
    eprint!("(c=-1 last line: {:?}) ", {
        let t = String::from_utf8_lossy(&out).to_string();
        t.lines().last().map(|s| s.escape_debug().to_string())
    });
    assert_eq!(out, rust_output(-1), "err03: byte mismatch for c=-1");
}

/// ERRORS.md row 4 — most negative char, lowest legal ctype table index.
fn error_row_04_most_negative() {
    assert_same(-128, "err04/most-negative");
}

/// ERRORS.md row 5 — every negative char index.
fn error_row_05_all_negative_chars() {
    for v in -128i32..=-1 {
        assert_same(v as c_char, "err05/negative-sweep");
    }
}

/// ERRORS.md row 6 — out-of-range argument across the FFI boundary; the
/// analogue of an invalid enum value. Must truncate identically, not trap.
fn error_row_06_out_of_range_int_arg() {
    for v in [256, -129, 1000, 0x1234, 0x7F00, i32::MAX, i32::MIN, -32768] {
        assert_same_int(v, "err06/oor");
        let truncated = (v as u32 as u8) as c_char;
        assert_eq!(
            c_output_int(v),
            c_output(truncated),
            "err06: C did not truncate {v} to 0x{:02x}",
            truncated as u8
        );
        assert_eq!(
            rust_output_int(v),
            rust_output(truncated),
            "err06: Rust did not truncate {v} to 0x{:02x}",
            truncated as u8
        );
    }
}

/// ERRORS.md row 7 — space vs tab: the isspace/isblank/isprint disagreement.
fn error_row_07_space_vs_blank() {
    assert_same(b' ' as c_char, "err07/space");
    assert_same(b'\t' as c_char, "err07/tab");
    let sp = String::from_utf8_lossy(&c_output(b' ' as c_char)).to_string();
    let tab = String::from_utf8_lossy(&c_output(b'\t' as c_char)).to_string();
    assert_ne!(sp, tab, "err07: space and tab must not classify identically");
}

/// ERRORS.md row 8 — one step past each classification range.
fn error_row_08_one_past_each_class_boundary() {
    for c in [
        b'/', b':', b'@', b'[', b'`', b'{', b'g', b'G', b'0', b'9', b'A', b'Z', b'a', b'z', b'f',
        b'F',
    ] {
        assert_same(c as c_char, "err08/boundary");
    }
}

/// ERRORS.md row 9 — repeated invocation / setlocale re-entry, interleaved.
fn error_row_09_repeat_and_interleave() {
    let seq: [c_char; 8] = [0, -1, -128, 127, 32, 9, 65, 97];
    let mut c_first = Vec::new();
    let mut r_first = Vec::new();
    for &c in &seq {
        c_first.push(c_output(c));
        r_first.push(rust_output(c));
    }
    // Second pass in the opposite library order.
    for (i, &c) in seq.iter().enumerate() {
        let r = rust_output(c);
        let cc = c_output(c);
        assert_eq!(r_first[i], r, "err09: Rust drifted for c={c}");
        assert_eq!(c_first[i], cc, "err09: C drifted for c={c}");
        assert_eq!(cc, r, "err09: divergence for c={c}");
    }
}

// ===========================================================================
// Phase D — symbol parity, checked from inside the suite as well
// ===========================================================================

fn symbol_parity_driver_is_exported_by_both() {
    // Resolving the symbol through libloading proves it is a real dynamic
    // export in both objects (and that the Rust `#[no_mangle]` wrapper is the
    // thing under test, not an internal Rust function).
    let _c = c_driver();
    let _r = rust_driver();
    assert!(unsafe { libs().rust.get::<DriverFn>(b"driver\0") }.is_ok());
    assert!(unsafe { libs().c.get::<DriverFn>(b"driver\0") }.is_ok());
}

// ===========================================================================
// Runner
// ===========================================================================

fn main() {
    let rows: Vec<(&str, fn())> = vec![
        // Phase D first: a symbol regression should fail fast.
        (
            "phaseD:symbol_parity",
            symbol_parity_driver_is_exported_by_both as fn(),
        ),
        // Phase B — CONFIGS.md
        (
            "configs:row01_exhaustive_all_256",
            config_row_01_exhaustive_all_256,
        ),
        ("configs:row02_control_chars", config_row_02_control_chars),
        ("configs:row03_whitespace", config_row_03_whitespace),
        ("configs:row04_digits", config_row_04_digits),
        ("configs:row05_hex_letters", config_row_05_hex_letters),
        (
            "configs:row06_lowercase_non_hex",
            config_row_06_lowercase_non_hex,
        ),
        (
            "configs:row07_uppercase_non_hex",
            config_row_07_uppercase_non_hex,
        ),
        ("configs:row08_punctuation", config_row_08_punctuation),
        ("configs:row09_negative_chars", config_row_09_negative_chars),
        (
            "configs:row10_uniform_random_fuzz",
            config_row_10_uniform_random_fuzz,
        ),
        (
            "configs:row11_out_of_range_int_args",
            config_row_11_out_of_range_int_args,
        ),
        ("configs:row12_locale_baseline", config_row_12_locale_baseline),
        ("configs:row13_locale_env_utf8", config_row_13_locale_env_utf8),
        (
            "configs:row14_locale_switched_multibyte",
            config_row_14_locale_switched_multibyte,
        ),
        (
            "configs:row15_interleave_and_repeat",
            config_row_15_interleave_and_repeat,
        ),
        // Phase C — ERRORS.md
        ("errors:row01_nul", error_row_01_nul),
        ("errors:row02_del_127", error_row_02_del_127),
        ("errors:row03_eof_minus_one", error_row_03_eof_minus_one),
        ("errors:row04_most_negative", error_row_04_most_negative),
        (
            "errors:row05_all_negative_chars",
            error_row_05_all_negative_chars,
        ),
        (
            "errors:row06_out_of_range_int_arg",
            error_row_06_out_of_range_int_arg,
        ),
        ("errors:row07_space_vs_blank", error_row_07_space_vs_blank),
        (
            "errors:row08_one_past_each_class_boundary",
            error_row_08_one_past_each_class_boundary,
        ),
        (
            "errors:row09_repeat_and_interleave",
            error_row_09_repeat_and_interleave,
        ),
    ];

    // Optional substring filter, mirroring libtest's positional argument.
    let filter = std::env::args().nth(1).filter(|a| !a.starts_with("--"));

    eprintln!("running {} differential rows", rows.len());
    let _ = libs(); // load both .so up front so their paths are logged first

    let mut failed: Vec<&str> = Vec::new();
    let mut skipped = 0usize;
    for (name, f) in &rows {
        if let Some(pat) = &filter {
            if !name.contains(pat.as_str()) {
                skipped += 1;
                continue;
            }
        }
        eprint!("  {name} ... ");
        let _ = std::io::stderr().flush();
        match catch_unwind(AssertUnwindSafe(f)) {
            Ok(()) => eprintln!("ok"),
            Err(_) => {
                eprintln!("FAILED");
                failed.push(name);
            }
        }
    }

    eprintln!();
    if failed.is_empty() {
        eprintln!(
            "result: ok. {} passed; 0 failed; {skipped} filtered out",
            rows.len() - skipped
        );
    } else {
        eprintln!("failures:");
        for f in &failed {
            eprintln!("    {f}");
        }
        eprintln!(
            "result: FAILED. {} passed; {} failed; {skipped} filtered out",
            rows.len() - skipped - failed.len(),
            failed.len()
        );
        std::process::exit(1);
    }
}
