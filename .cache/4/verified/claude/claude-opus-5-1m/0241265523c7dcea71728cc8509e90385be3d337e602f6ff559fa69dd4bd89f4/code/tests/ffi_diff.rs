//! Phase B/C differential tests for the `printLine`, `bad` and `good` exports.
//!
//! Both the C shared object and the Rust `cdylib` are loaded with
//! `libloading::Library` (i.e. `dlopen`) and driven through their exported
//! C-ABI symbols only — the Rust implementation is never called directly, so
//! the `#[no_mangle] extern "C"` wrappers are themselves under test.
//!
//! Standard output is captured by temporarily pointing file descriptor 1 at a
//! file, which works for both objects: the C object writes through glibc's
//! `stdout` (flushed with `fflush(NULL)`) and the Rust object writes through
//! the `std::io::stdout()` belonging to its own statically linked `std` (which
//! its export wrappers flush before returning).

mod common;

use common::{assert_same_bytes, capture_fd1, c_so, rust_so, show, Rng, SEED};
use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

struct Objects {
    c: Library,
    rust: Library,
}

// Both handles are only ever used through `&` and `dlsym` is thread safe.
unsafe impl Sync for Objects {}
unsafe impl Send for Objects {}

fn objects() -> &'static Objects {
    static O: OnceLock<Objects> = OnceLock::new();
    O.get_or_init(|| unsafe {
        let c = Library::new(c_so()).expect("dlopen the C shared object");
        let rust = Library::new(rust_so()).expect("dlopen the Rust cdylib");
        Objects { c, rust }
    })
}

// ---------------------------------------------------------------------------
// Thin wrappers around the exported symbols
// ---------------------------------------------------------------------------

fn print_line(lib: &Library, arg: Option<&CString>) -> Vec<u8> {
    let f: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { lib.get(b"printLine\0") }.expect("printLine is exported");
    let ptr = match arg {
        Some(s) => s.as_ptr(),
        None => std::ptr::null(),
    };
    capture_fd1(|| unsafe { f(ptr) })
}

fn call_void(lib: &Library, symbol: &str) -> Vec<u8> {
    let name = format!("{symbol}\0");
    let f: Symbol<unsafe extern "C" fn()> =
        unsafe { lib.get(name.as_bytes()) }.expect("symbol is exported");
    capture_fd1(|| unsafe { f() })
}

fn call_sequence(lib: &Library, symbols: &[&str]) -> Vec<u8> {
    let mut fns: Vec<Symbol<unsafe extern "C" fn()>> = Vec::new();
    for s in symbols {
        let name = format!("{s}\0");
        fns.push(unsafe { lib.get(name.as_bytes()) }.expect("symbol is exported"));
    }
    capture_fd1(|| {
        for f in &fns {
            unsafe { f() }
        }
    })
}

/// Drive `printLine` in both objects with the same bytes and compare.
#[track_caller]
fn diff_print_line(label: &str, bytes: &[u8]) {
    let arg = CString::new(bytes).expect("no interior NUL");
    let o = objects();
    let c_out = print_line(&o.c, Some(&arg));
    let r_out = print_line(&o.rust, Some(&arg));
    assert_same_bytes(label, bytes, &c_out, &r_out);
    // Sanity: `printf("%s\n", s)` (lowered by gcc to `puts`) must emit exactly
    // the bytes plus one line feed.
    let mut expected = bytes.to_vec();
    expected.push(b'\n');
    assert_eq!(
        c_out,
        expected,
        "C reference produced something other than \"<bytes>\\n\" for {label}: \"{}\"",
        show(&c_out)
    );
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 1 — printLine with the empty string
// ---------------------------------------------------------------------------

fn cfg_01_print_line_empty() {
    diff_print_line("empty string", b"");
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 2 — printLine with every single ASCII byte
// ---------------------------------------------------------------------------

fn cfg_02_print_line_single_ascii() {
    for b in 0x01u8..=0x7f {
        diff_print_line(&format!("single byte {b:#04x}"), &[b]);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 3 — random printable-ASCII strings
// ---------------------------------------------------------------------------

fn cfg_03_print_line_random_ascii() {
    let alphabet: Vec<u8> = (0x20u8..=0x7e).collect();
    let mut rng = Rng::new(SEED ^ 0x03);
    for i in 0..256 {
        let len = rng.range(1, 64) as usize;
        let s = rng.bytes(len, &alphabet);
        diff_print_line(&format!("random ascii #{i} (len {len})"), &s);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 4 — random arbitrary (non-UTF-8) byte strings
// ---------------------------------------------------------------------------

fn cfg_04_print_line_random_bytes() {
    let alphabet: Vec<u8> = (0x01u8..=0xff).collect();
    let mut rng = Rng::new(SEED ^ 0x04);
    for i in 0..256 {
        let len = rng.range(1, 256) as usize;
        let s = rng.bytes(len, &alphabet);
        diff_print_line(&format!("random bytes #{i} (len {len})"), &s);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 5 — embedded control characters
// ---------------------------------------------------------------------------

fn cfg_05_print_line_embedded_control() {
    let controls: [u8; 5] = [b'\n', b'\r', b'\t', 0x0b, 0x0c];
    let mut rng = Rng::new(SEED ^ 0x05);
    for i in 0..128 {
        let len = rng.range(1, 40) as usize;
        let mut s: Vec<u8> = Vec::with_capacity(len);
        for _ in 0..len {
            if rng.below(3) == 0 {
                s.push(*rng.pick(&controls));
            } else {
                s.push(rng.range(0x41, 0x5a) as u8);
            }
        }
        diff_print_line(&format!("embedded control #{i}"), &s);
    }
    // Deterministic shapes too.
    diff_print_line("only newline", b"\n");
    diff_print_line("leading newline", b"\nabc");
    diff_print_line("trailing newline", b"abc\n");
    diff_print_line("many newlines", b"a\nb\nc\n\n\n");
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 6 — length boundaries
// ---------------------------------------------------------------------------

fn cfg_06_print_line_length_boundaries() {
    let lengths = [
        1usize, 2, 7, 8, 15, 16, 17, 31, 32, 63, 64, 127, 128, 255, 256, 511, 512, 1023, 1024,
        1025, 2047, 2048, 4095, 4096, 4097, 8191, 8192, 8193, 65535, 65536, 65537, 1_048_576,
    ];
    let mut rng = Rng::new(SEED ^ 0x06);
    let alphabet: Vec<u8> = (0x21u8..=0x7e).collect();
    for len in lengths {
        let s = rng.bytes(len, &alphabet);
        diff_print_line(&format!("length {len}"), &s);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 7 / ERRORS.md row 1 — printLine(NULL)
// ---------------------------------------------------------------------------

fn cfg_07_print_line_null() {
    let o = objects();
    let c_out = print_line(&o.c, None);
    let r_out = print_line(&o.rust, None);
    assert_same_bytes("printLine(NULL)", b"<null>", &c_out, &r_out);
    assert!(
        c_out.is_empty(),
        "the C null branch must produce no output, got \"{}\"",
        show(&c_out)
    );
}

fn err_01_print_line_null() {
    // Same rejection, asserted repeatedly to make sure it is not order- or
    // state-dependent (e.g. a stale buffer flushed on a later call).
    let o = objects();
    for _ in 0..8 {
        let c_out = print_line(&o.c, None);
        let r_out = print_line(&o.rust, None);
        assert_same_bytes("printLine(NULL) repeated", b"<null>", &c_out, &r_out);
        assert!(c_out.is_empty());
        assert!(r_out.is_empty());
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md generic rows G2..G6
// ---------------------------------------------------------------------------

fn err_g2_print_line_empty() {
    let arg = CString::new("").unwrap();
    let o = objects();
    let c_out = print_line(&o.c, Some(&arg));
    let r_out = print_line(&o.rust, Some(&arg));
    assert_same_bytes("printLine(\"\")", b"", &c_out, &r_out);
    assert_eq!(c_out, b"\n", "C emits just the line feed for an empty string");
}

fn err_g3_print_line_oversized() {
    for len in [1_048_576usize, 2_097_153] {
        let s = vec![b'Z'; len];
        diff_print_line(&format!("oversized {len}"), &s);
    }
}

fn err_g4_print_line_non_utf8() {
    // Byte sequences that are definitively invalid UTF-8; a Rust wrapper that
    // validated its input would reject or replace these.
    let cases: Vec<Vec<u8>> = vec![
        vec![0x80],
        vec![0xff],
        vec![0xc3],             // truncated 2-byte sequence
        vec![0xe2, 0x28, 0xa1], // invalid continuation
        vec![0xf0, 0x9f],       // truncated 4-byte sequence
        vec![0xed, 0xa0, 0x80], // UTF-16 surrogate encoded as UTF-8
        vec![0xfe, 0xff],
        (0x80u8..=0xff).collect(),
        (0x01u8..=0xff).collect(),
    ];
    for (i, c) in cases.iter().enumerate() {
        diff_print_line(&format!("non-utf8 #{i}"), c);
    }
}

fn err_g5_print_line_embedded_newline() {
    diff_print_line("embedded newline", b"line1\nline2");
    diff_print_line("embedded crlf", b"line1\r\nline2");
    diff_print_line("newline only", b"\n");
    diff_print_line("double newline", b"\n\n");
}

fn err_g6_print_line_every_byte() {
    for b in 0x01u8..=0xff {
        diff_print_line(&format!("byte {b:#04x}"), &[b]);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 8, 9, 10 — bad() / good()
// ---------------------------------------------------------------------------

fn cfg_08_bad() {
    let o = objects();
    let c_out = call_void(&o.c, "bad");
    let r_out = call_void(&o.rust, "bad");
    assert_same_bytes("bad()", b"", &c_out, &r_out);
}

fn cfg_09_good() {
    let o = objects();
    let c_out = call_void(&o.c, "good");
    let r_out = call_void(&o.rust, "good");
    assert_same_bytes("good()", b"", &c_out, &r_out);
    assert_eq!(
        c_out, b"helperGood1 string\n",
        "unexpected C reference output for good()"
    );
}

fn cfg_10_bad_good_interleaved() {
    let o = objects();
    let mut rng = Rng::new(SEED ^ 0x0a);
    for i in 0..64 {
        let n = rng.range(1, 12) as usize;
        let seq: Vec<&str> = (0..n)
            .map(|_| if rng.bool() { "good" } else { "bad" })
            .collect();
        let c_out = call_sequence(&o.c, &seq);
        let r_out = call_sequence(&o.rust, &seq);
        assert_same_bytes(
            &format!("sequence #{i} {seq:?}"),
            seq.join(",").as_bytes(),
            &c_out,
            &r_out,
        );
    }
    // A long deterministic run, to prove the `static` buffer is reusable.
    let seq: Vec<&str> = vec!["good"; 200];
    let c_out = call_sequence(&o.c, &seq);
    let r_out = call_sequence(&o.rust, &seq);
    assert_same_bytes("200x good", b"200x good", &c_out, &r_out);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — bad() always takes the null branch
// ---------------------------------------------------------------------------

fn err_02_bad_prints_nothing() {
    let o = objects();
    for _ in 0..16 {
        let c_out = call_void(&o.c, "bad");
        let r_out = call_void(&o.rust, "bad");
        assert_same_bytes("bad() repeated", b"", &c_out, &r_out);
        assert!(
            c_out.is_empty(),
            "gcc lowers helperBad() to `return NULL`, so bad() must print nothing; got \"{}\"",
            show(&c_out)
        );
        assert!(r_out.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Sequential entry point (`harness = false`)
// ---------------------------------------------------------------------------

fn main() {
    let cases: &[(&str, fn())] = &[
        ("cfg_01_print_line_empty", cfg_01_print_line_empty),
        ("cfg_02_print_line_single_ascii", cfg_02_print_line_single_ascii),
        ("cfg_03_print_line_random_ascii", cfg_03_print_line_random_ascii),
        ("cfg_04_print_line_random_bytes", cfg_04_print_line_random_bytes),
        ("cfg_05_print_line_embedded_control", cfg_05_print_line_embedded_control),
        ("cfg_06_print_line_length_boundaries", cfg_06_print_line_length_boundaries),
        ("cfg_07_print_line_null", cfg_07_print_line_null),
        ("err_01_print_line_null", err_01_print_line_null),
        ("err_g2_print_line_empty", err_g2_print_line_empty),
        ("err_g3_print_line_oversized", err_g3_print_line_oversized),
        ("err_g4_print_line_non_utf8", err_g4_print_line_non_utf8),
        ("err_g5_print_line_embedded_newline", err_g5_print_line_embedded_newline),
        ("err_g6_print_line_every_byte", err_g6_print_line_every_byte),
        ("cfg_08_bad", cfg_08_bad),
        ("cfg_09_good", cfg_09_good),
        ("cfg_10_bad_good_interleaved", cfg_10_bad_good_interleaved),
        ("err_02_bad_prints_nothing", err_02_bad_prints_nothing),
    ];
    common::run_cases(cases);
}
