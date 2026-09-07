// Differential tests: load BOTH the C `.so` and the Rust `.so` through
// `libloading` and compare their observable behaviour byte-for-byte.
//
// Neither library is ever called directly as a Rust function -- every call goes
// through `dlsym` on a shared object, exactly as an external C consumer would,
// so the `#[no_mangle]` export wrappers are under test too.
//
// All five functions in this library return `void`; their entire observable
// effect is the bytes they write to `stdout`. We therefore capture `stdout` at
// the file-descriptor level (dup/dup2 around the call) so that we see the exact
// bytes glibc emitted for each library.

use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Run `f`, returning every byte it wrote to file descriptor 1.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    // fd 1 is process-global; serialise so parallel test threads cannot
    // interleave their captures.
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}.bin",
        std::process::id(),
        CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst)
    ));

    unsafe {
        // Flush anything already pending so it is not attributed to this call:
        // both the C stdio buffer and Rust's own `Stdout` buffer (the test
        // harness leaves a partial `test <name> ... ` line sitting in it).
        let _ = std::io::Write::flush(&mut std::io::stdout());
        fflush(std::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");

        {
            let file = File::create(&path).expect("create capture file");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");
        }

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

// ---------------------------------------------------------------------------
// library loading
// ---------------------------------------------------------------------------

type FnVoid = unsafe extern "C" fn();
type FnDriver = unsafe extern "C" fn(c_int);
type FnPrintLine = unsafe extern "C" fn(*const c_char);
type FnPrintHexCharLine = unsafe extern "C" fn(c_char);

/// One dlopen'd shared object plus typed accessors for its exports.
struct Lib {
    name: &'static str,
    lib: Library,
}

impl Lib {
    fn open(name: &'static str, path: &Path) -> Lib {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} ({}) failed: {e}", name, path.display()));
        Lib { name, lib }
    }

    fn sym<T: Copy>(&self, name: &[u8]) -> T {
        let s: Symbol<T> = unsafe { self.lib.get(name) }.unwrap_or_else(|e| {
            panic!(
                "{}: symbol `{}` not exported: {e}",
                self.name,
                String::from_utf8_lossy(name)
            )
        });
        *s
    }

    fn has(&self, name: &[u8]) -> bool {
        unsafe { self.lib.get::<*const c_void>(name) }.is_ok()
    }

    fn print_line(&self, s: *const c_char) {
        let f: FnPrintLine = self.sym(b"printLine\0");
        unsafe { f(s) }
    }
    fn print_hex_char_line(&self, c: c_char) {
        let f: FnPrintHexCharLine = self.sym(b"printHexCharLine\0");
        unsafe { f(c) }
    }
    fn bad(&self) {
        let f: FnVoid = self.sym(b"bad\0");
        unsafe { f() }
    }
    fn good(&self) {
        let f: FnVoid = self.sym(b"good\0");
        unsafe { f() }
    }
    fn driver(&self, use_good: c_int) {
        let f: FnDriver = self.sym(b"driver\0");
        unsafe { f(use_good) }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not built. Run:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn rust_so_path() -> PathBuf {
    // The test binary lives in <target>/<profile>/deps/, so the cdylib that
    // cargo just built for this package is one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let mut candidates = Vec::new();
    if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
        candidates.push(profile_dir.join("libdriver.so"));
    }
    candidates.push(manifest_dir().join("target/release/libdriver.so"));
    candidates.push(manifest_dir().join("target/debug/libdriver.so"));
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("Rust cdylib not found; tried: {candidates:?}");
}

static LIBS: OnceLock<(Lib, Lib)> = OnceLock::new();

fn libs() -> (&'static Lib, &'static Lib) {
    let (c, r) = LIBS.get_or_init(|| {
        (
            Lib::open("C", &c_so_path()),
            Lib::open("Rust", &rust_so_path()),
        )
    });
    (c, r)
}

// ---------------------------------------------------------------------------
// differential assertions
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    format!("{:?} ({} bytes)", String::from_utf8_lossy(b), b.len())
}

/// Run the same scenario against both libraries; require identical stdout.
fn diff<F: Fn(&Lib)>(label: &str, f: F) -> Vec<u8> {
    let (c, r) = libs();
    let out_c = capture(|| f(c));
    let out_r = capture(|| f(r));
    assert_eq!(
        out_c,
        out_r,
        "DIVERGENCE [{label}]\n  C   : {}\n  Rust: {}",
        show(&out_c),
        show(&out_r)
    );
    out_c
}

/// Same as `diff`, but also pins the absolute expected bytes so that a
/// "both produced nothing" false pass is impossible.
fn diff_expect<F: Fn(&Lib)>(label: &str, expected: &[u8], f: F) {
    let out = diff(label, f);
    assert_eq!(
        out,
        expected,
        "[{label}] both libraries agreed but on the WRONG bytes\n  got     : {}\n  expected: {}",
        show(&out),
        show(expected)
    );
}

// ---------------------------------------------------------------------------
// deterministic PRNG (fixed seed => reproducible runs)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    fn range(&mut self, lo: u64, hi_inclusive: u64) -> u64 {
        lo + self.below(hi_inclusive - lo + 1)
    }
    fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    fn nonzero_i32(&mut self) -> i32 {
        loop {
            let v = self.i32();
            if v != 0 {
                return v;
            }
        }
    }
    fn i8(&mut self) -> i8 {
        self.next_u64() as i8
    }
}

// Reference renderings of the C source's literal outputs.
const BAD_LINE: &[u8] = b"fffffffe\n"; // (char)(CHAR_MAX * 2) == -2 -> %02x on int
const G2B_LINE: &[u8] = b"04\n"; // (char)(2 * 2) == 4
const TOO_LARGE: &[u8] = b"data value is too large to perform arithmetic safely.\n";

fn good_bytes() -> Vec<u8> {
    let mut v = G2B_LINE.to_vec();
    v.extend_from_slice(TOO_LARGE);
    v
}

/// The exact bytes C's `printHexCharLine(v)` must produce: `v` is promoted to
/// `int` through varargs and rendered with `%02x`, i.e. reinterpreted as
/// `unsigned int`, so negatives widen to eight hex digits.
fn hex_line(v: i8) -> Vec<u8> {
    format!("{:02x}\n", (v as c_int) as u32).into_bytes()
}

// ===========================================================================
// Phase A / D -- symbol parity checked through dlsym
// ===========================================================================

/// Every symbol the C `.so` exports must be resolvable in the Rust `.so`.
#[test]
fn phase_d_symbol_parity_via_dlsym() {
    let (c, r) = libs();
    let exported: [&[u8]; 5] = [
        b"printLine\0",
        b"printHexCharLine\0",
        b"bad\0",
        b"good\0",
        b"driver\0",
    ];
    let mut missing = Vec::new();
    for name in exported {
        assert!(
            c.has(name),
            "test bug: C lib lacks {}",
            String::from_utf8_lossy(name)
        );
        if !r.has(name) {
            missing.push(String::from_utf8_lossy(name).to_string());
        }
    }
    assert!(missing.is_empty(), "Rust .so missing symbols: {missing:?}");

    // `goodG2B` / `goodB2G` are `static` in C: neither library may export them.
    for name in [b"goodG2B\0".as_slice(), b"goodB2G\0".as_slice()] {
        let n = String::from_utf8_lossy(name);
        assert!(!c.has(name), "unexpected: C exports static {n}");
        assert!(
            !r.has(name),
            "Rust exports {n}, but it is `static` in the C source"
        );
    }
}

// ===========================================================================
// Phase B -- valid-path differential tests, one per CONFIGS.md row
// ===========================================================================

// C1: printLine, randomized printable ASCII, length 1..64
#[test]
fn phase_b_c01_print_line_random_ascii() {
    let mut rng = Rng::new(0xC001);
    for _ in 0..512 {
        let len = rng.range(1, 64) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.range(0x20, 0x7e) as u8).collect();
        let cs = CString::new(bytes.clone()).unwrap();
        let mut expected = bytes.clone();
        expected.push(b'\n');
        diff_expect("C1", &expected, |lib| lib.print_line(cs.as_ptr()));
    }
}

// C2: printLine("")
#[test]
fn phase_b_c02_print_line_empty() {
    let cs = CString::new("").unwrap();
    diff_expect("C2", b"\n", |lib| lib.print_line(cs.as_ptr()));
}

// C3: printLine, every single non-NUL byte value as a 1-char string
#[test]
fn phase_b_c03_print_line_all_single_bytes() {
    for b in 1u8..=255 {
        let cs = CString::new(vec![b]).unwrap();
        let expected = [b, b'\n'];
        diff_expect(&format!("C3/{b:#04x}"), &expected, |lib| {
            lib.print_line(cs.as_ptr())
        });
    }
}

// C4: printLine, randomized bytes over the full 1..=255 range
#[test]
fn phase_b_c04_print_line_random_full_byte_range() {
    let mut rng = Rng::new(0xC004);
    for _ in 0..512 {
        let len = rng.range(1, 96) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.range(1, 255) as u8).collect();
        let cs = CString::new(bytes.clone()).unwrap();
        let mut expected = bytes.clone();
        expected.push(b'\n');
        diff_expect("C4", &expected, |lib| lib.print_line(cs.as_ptr()));
    }
}

// C5: printLine payloads that look like printf format strings
#[test]
fn phase_b_c05_print_line_format_specifiers() {
    let payloads: [&str; 8] = [
        "%s",
        "%n",
        "%d %i %u",
        "100%%",
        "%p %p %p %p",
        "%1000000d",
        "%s%s%s%s%s%s%s%s",
        "mixed %s literal %x tail",
    ];
    for p in payloads {
        let cs = CString::new(p).unwrap();
        let mut expected = p.as_bytes().to_vec();
        expected.push(b'\n');
        diff_expect(&format!("C5/{p}"), &expected, |lib| {
            lib.print_line(cs.as_ptr())
        });
    }
}

// C6: printLine with embedded control characters
#[test]
fn phase_b_c06_print_line_embedded_controls() {
    let payloads: [&[u8]; 5] = [
        b"a\nb",
        b"a\tb\tc",
        b"cr\rhere",
        b"\n\n\n",
        b"line1\nline2\nline3",
    ];
    for p in payloads {
        let cs = CString::new(p.to_vec()).unwrap();
        let mut expected = p.to_vec();
        expected.push(b'\n');
        diff_expect("C6", &expected, |lib| lib.print_line(cs.as_ptr()));
    }
}

// C7: printLine with strings longer than the stdio buffer
#[test]
fn phase_b_c07_print_line_long_strings() {
    let mut rng = Rng::new(0xC007);
    for _ in 0..24 {
        let len = rng.range(4000, 9000) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.range(0x21, 0x7e) as u8).collect();
        let cs = CString::new(bytes.clone()).unwrap();
        let mut expected = bytes.clone();
        expected.push(b'\n');
        diff_expect("C7", &expected, |lib| lib.print_line(cs.as_ptr()));
    }
}

// C8: printLine(NULL) -- see also E1
#[test]
fn phase_b_c08_print_line_null() {
    diff_expect("C8", b"", |lib| lib.print_line(std::ptr::null()));
}

// C9: printHexCharLine over all 256 char bit patterns
#[test]
fn phase_b_c09_print_hex_char_line_full_sweep() {
    for v in i8::MIN..=i8::MAX {
        diff_expect(&format!("C9/{v}"), &hex_line(v), |lib| {
            lib.print_hex_char_line(v as c_char)
        });
    }
}

// C10: printHexCharLine, randomized draws
#[test]
fn phase_b_c10_print_hex_char_line_random() {
    let mut rng = Rng::new(0xC010);
    for _ in 0..512 {
        let v = rng.i8();
        diff_expect(&format!("C10/{v}"), &hex_line(v), |lib| {
            lib.print_hex_char_line(v as c_char)
        });
    }
}

// C11: bad() once
#[test]
fn phase_b_c11_bad_single() {
    diff_expect("C11", BAD_LINE, |lib| lib.bad());
}

// C12: bad() repeated
#[test]
fn phase_b_c12_bad_repeated() {
    let mut rng = Rng::new(0xC012);
    for _ in 0..64 {
        let n = rng.range(1, 16) as usize;
        let expected: Vec<u8> = BAD_LINE.repeat(n);
        diff_expect(&format!("C12/n={n}"), &expected, |lib| {
            for _ in 0..n {
                lib.bad();
            }
        });
    }
}

// C13: good() once -- goodG2B then the goodB2G range-check rejection
#[test]
fn phase_b_c13_good_single() {
    diff_expect("C13", &good_bytes(), |lib| lib.good());
}

// C14: good() repeated
#[test]
fn phase_b_c14_good_repeated() {
    let mut rng = Rng::new(0xC014);
    let unit = good_bytes();
    for _ in 0..64 {
        let n = rng.range(1, 16) as usize;
        let expected: Vec<u8> = unit.repeat(n);
        diff_expect(&format!("C14/n={n}"), &expected, |lib| {
            for _ in 0..n {
                lib.good();
            }
        });
    }
}

// C15: driver(0) -> bad()
#[test]
fn phase_b_c15_driver_zero() {
    diff_expect("C15", BAD_LINE, |lib| lib.driver(0));
}

// C16: driver(1) -> good()
#[test]
fn phase_b_c16_driver_one() {
    diff_expect("C16", &good_bytes(), |lib| lib.driver(1));
}

// C17: driver(random non-zero) -> good()
#[test]
fn phase_b_c17_driver_random_nonzero() {
    let mut rng = Rng::new(0xC017);
    let expected = good_bytes();
    for _ in 0..1024 {
        let v = rng.nonzero_i32();
        diff_expect(&format!("C17/{v}"), &expected, |lib| lib.driver(v));
    }
}

// C18: driver with non-zero ints whose low byte may be zero
#[test]
fn phase_b_c18_driver_low_byte_zero() {
    let expected = good_bytes();
    for v in [
        256i32,
        0x1_0000,
        0x7FFF_FF00,
        -256,
        i32::MIN,
        i32::MAX,
        -1,
        2,
        0x0100_0000,
    ] {
        diff_expect(&format!("C18/{v}"), &expected, |lib| lib.driver(v));
    }
}

// C19: randomized sequence of mixed driver() modes in one capture
#[test]
fn phase_b_c19_driver_mixed_sequence() {
    let mut rng = Rng::new(0xC019);
    let good = good_bytes();
    for _ in 0..128 {
        let n = rng.range(1, 12) as usize;
        let args: Vec<i32> = (0..n)
            .map(|_| if rng.below(2) == 0 { 0 } else { rng.nonzero_i32() })
            .collect();
        let mut expected = Vec::new();
        for a in &args {
            expected.extend_from_slice(if *a == 0 { BAD_LINE } else { &good });
        }
        diff_expect(&format!("C19/{args:?}"), &expected, |lib| {
            for a in &args {
                lib.driver(*a);
            }
        });
    }
}

// C20: randomized interleaving of every entry point in one capture
#[test]
fn phase_b_c20_mixed_pipeline() {
    #[derive(Debug, Clone)]
    enum Op {
        Driver(i32),
        Good,
        Bad,
        PrintLine(Vec<u8>),
        PrintLineNull,
        PrintHex(i8),
    }

    let mut rng = Rng::new(0xC020);
    let good = good_bytes();

    for _ in 0..128 {
        let n = rng.range(1, 20) as usize;
        let mut ops = Vec::with_capacity(n);
        for _ in 0..n {
            ops.push(match rng.below(6) {
                0 => Op::Driver(if rng.below(2) == 0 { 0 } else { rng.nonzero_i32() }),
                1 => Op::Good,
                2 => Op::Bad,
                3 => {
                    let len = rng.range(0, 48) as usize;
                    Op::PrintLine((0..len).map(|_| rng.range(1, 255) as u8).collect())
                }
                4 => Op::PrintLineNull,
                _ => Op::PrintHex(rng.i8()),
            });
        }

        let mut expected = Vec::new();
        for op in &ops {
            match op {
                Op::Driver(0) => expected.extend_from_slice(BAD_LINE),
                Op::Driver(_) => expected.extend_from_slice(&good),
                Op::Good => expected.extend_from_slice(&good),
                Op::Bad => expected.extend_from_slice(BAD_LINE),
                Op::PrintLine(b) => {
                    expected.extend_from_slice(b);
                    expected.push(b'\n');
                }
                Op::PrintLineNull => {}
                Op::PrintHex(v) => expected.extend_from_slice(&hex_line(*v)),
            }
        }

        // Pre-build the CStrings so the closure can run twice unchanged.
        let strings: Vec<Option<CString>> = ops
            .iter()
            .map(|op| match op {
                Op::PrintLine(b) => Some(CString::new(b.clone()).unwrap()),
                _ => None,
            })
            .collect();

        diff_expect("C20", &expected, |lib| {
            for (op, s) in ops.iter().zip(strings.iter()) {
                match op {
                    Op::Driver(v) => lib.driver(*v),
                    Op::Good => lib.good(),
                    Op::Bad => lib.bad(),
                    Op::PrintLine(_) => lib.print_line(s.as_ref().unwrap().as_ptr()),
                    Op::PrintLineNull => lib.print_line(std::ptr::null()),
                    Op::PrintHex(v) => lib.print_hex_char_line(*v as c_char),
                }
            }
        });
    }
}

// ===========================================================================
// Phase C -- error / rejection paths, one per ERRORS.md row
// ===========================================================================

// E1: printLine(NULL) -> silent no-op, zero bytes
#[test]
fn phase_c_e01_print_line_null_is_silent() {
    diff_expect("E1", b"", |lib| lib.print_line(std::ptr::null()));
    // Also from a non-first position, to rule out latched state.
    let cs = CString::new("before").unwrap();
    diff_expect("E1/seq", b"before\n", |lib| {
        lib.print_line(cs.as_ptr());
        lib.print_line(std::ptr::null());
    });
}

// E2: printLine("") -> exactly one newline
#[test]
fn phase_c_e02_print_line_empty_is_one_newline() {
    let cs = CString::new("").unwrap();
    diff_expect("E2/empty", b"\n", |lib| lib.print_line(cs.as_ptr()));
}

// E3: format specifiers in the *argument* must not be interpreted
#[test]
fn phase_c_e03_print_line_no_format_injection() {
    for p in ["%s", "%n", "%d", "%%", "%s %n %d %%", "%99999999d"] {
        let cs = CString::new(p).unwrap();
        let mut expected = p.as_bytes().to_vec();
        expected.push(b'\n');
        diff_expect(&format!("E3/{p}"), &expected, |lib| {
            lib.print_line(cs.as_ptr())
        });
    }
}

// E4: negative char -> sign-extended 8-digit hex
#[test]
fn phase_c_e04_print_hex_negative_sign_extends() {
    let cases: [(i8, &[u8]); 5] = [
        (-1, b"ffffffff\n"),
        (-2, b"fffffffe\n"),
        (-128, b"ffffff80\n"),
        (-16, b"fffffff0\n"),
        (-127, b"ffffff81\n"),
    ];
    for (v, expected) in cases {
        diff_expect(&format!("E4/{v}"), expected, |lib| {
            lib.print_hex_char_line(v as c_char)
        });
    }
}

// E5: zero -> "00"
#[test]
fn phase_c_e05_print_hex_zero_pads() {
    diff_expect("E5", b"00\n", |lib| lib.print_hex_char_line(0));
    for (v, expected) in [(1i8, "01\n"), (9, "09\n"), (15, "0f\n"), (16, "10\n")] {
        diff_expect(&format!("E5/{v}"), expected.as_bytes(), |lib| {
            lib.print_hex_char_line(v as c_char)
        });
    }
}

// E6: exhaustive char domain
#[test]
fn phase_c_e06_print_hex_exhaustive_domain() {
    for v in i8::MIN..=i8::MAX {
        diff_expect(&format!("E6/{v}"), &hex_line(v), |lib| {
            lib.print_hex_char_line(v as c_char)
        });
    }
}

// E7: bad()'s `data > 0` false branch is unreachable and has no else
#[test]
fn phase_c_e07_bad_guard_has_no_else_branch() {
    // Exactly one line, always -- never zero lines, never two.
    diff_expect("E7", BAD_LINE, |lib| lib.bad());
    let out = diff("E7/lines", |lib| lib.bad());
    assert_eq!(
        out.iter().filter(|b| **b == b'\n').count(),
        1,
        "bad() must emit exactly one line"
    );
}

// E8: the signed overflow itself, reproduced not fixed
#[test]
fn phase_c_e08_bad_overflow_wraps_to_minus_two() {
    diff_expect("E8", b"fffffffe\n", |lib| lib.bad());
    // (char)(CHAR_MAX * 2) must be -2, i.e. NOT saturated to 127 and NOT 254.
    diff_expect("E8/ref", &hex_line(-2), |lib| lib.bad());
}

// E9: goodG2B's guard -- reachable only via good(); its line must be present
#[test]
fn phase_c_e09_good_g2b_guard_taken() {
    let out = diff("E9", |lib| lib.good());
    assert!(
        out.starts_with(G2B_LINE),
        "good() must begin with goodG2B's `04` line, got {}",
        show(&out)
    );
}

// E10: goodB2G's outer `data > 0` guard, no else
#[test]
fn phase_c_e10_good_b2g_outer_guard_taken() {
    let out = diff("E10", |lib| lib.good());
    assert_eq!(
        out.iter().filter(|b| **b == b'\n').count(),
        2,
        "good() must emit exactly two lines, got {}",
        show(&out)
    );
}

// E11: the explicit range check 127 < 63 is FALSE -> rejection message
#[test]
fn phase_c_e11_good_b2g_range_check_rejects() {
    let out = diff("E11", |lib| lib.good());
    assert!(
        out.ends_with(TOO_LARGE),
        "goodB2G must take the `else` rejection branch, got {}",
        show(&out)
    );
    // and must NOT have multiplied.
    assert!(
        !out.ends_with(&hex_line(-2)[..]),
        "goodB2G must not perform the multiply"
    );
}

// E12: the dead store `data = ' '` must have no effect
#[test]
fn phase_c_e12_good_b2g_dead_store_has_no_effect() {
    let out = diff("E12", |lib| lib.good());
    // ' ' == 32 would pass the range check and print "40\n"; CHAR_MAX must win.
    assert!(
        !out.contains(&b'4') || !out.windows(3).any(|w| w == b"40\n"),
        "the overwritten `data = ' '` value leaked: {}",
        show(&out)
    );
    assert_eq!(out, good_bytes(), "unexpected good() output: {}", show(&out));
}

// E13: driver(0) dispatches bad()
#[test]
fn phase_c_e13_driver_zero_dispatches_bad() {
    diff_expect("E13", BAD_LINE, |lib| lib.driver(0));
}

// E14: out-of-range "enum-like" ints across the FFI boundary
#[test]
fn phase_c_e14_driver_out_of_range_enum_values() {
    let expected = good_bytes();
    for v in [
        1i32,
        -1,
        2,
        42,
        256,
        0x1_0000,
        0x7FFF_FF00,
        i32::MAX,
        i32::MIN,
        -0x8000_0000i64 as i32,
        0x0000_FF00,
        0x0100_0000,
        i32::MIN + 1,
        i32::MAX - 1,
    ] {
        diff_expect(&format!("E14/{v}"), &expected, |lib| lib.driver(v));
    }
    // Only an exact zero selects bad().
    diff_expect("E14/zero", BAD_LINE, |lib| lib.driver(0));
}

// E15: statelessness -- a zero after a non-zero still selects bad()
#[test]
fn phase_c_e15_driver_is_stateless() {
    let mut expected = good_bytes();
    expected.extend_from_slice(BAD_LINE);
    expected.extend_from_slice(&good_bytes());
    expected.extend_from_slice(BAD_LINE);
    diff_expect("E15", &expected, |lib| {
        lib.driver(7);
        lib.driver(0);
        lib.driver(-3);
        lib.driver(0);
    });
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary boundaries (beyond the tables)
// ---------------------------------------------------------------------------

// A pointer that is non-NULL but points at an immediately-terminated buffer,
// and a pointer into the middle of a larger buffer (offset aliasing).
#[test]
fn phase_c_generic_pointer_shapes() {
    let buf = b"abcdef\0tail\0".to_vec();
    for off in 0..7usize {
        let p = unsafe { buf.as_ptr().add(off) } as *const c_char;
        let expected = {
            let mut v = buf[off..6].to_vec();
            v.push(b'\n');
            v
        };
        diff_expect(&format!("ptr/off={off}"), &expected, |lib| lib.print_line(p));
    }
    // Pointer to a lone NUL byte.
    let nul = [0u8];
    diff_expect("ptr/lone-nul", b"\n", |lib| {
        lib.print_line(nul.as_ptr() as *const c_char)
    });
}

// The `char` parameter passed with garbage in the upper register bits: C
// callees read only the low byte. Emulated by passing every i8, already done in
// E6; here we additionally confirm the two libs agree when the value arrives as
// a widened int (function pointer cast to take c_int).
#[test]
fn phase_c_generic_char_arg_widening() {
    let (c, r) = libs();
    for raw in [0x0000_0100i32, 0x0000_017f, -0x0000_0180, 0x7fff_ff00u32 as i32] {
        let f_c: unsafe extern "C" fn(c_int) = unsafe {
            std::mem::transmute::<FnPrintHexCharLine, unsafe extern "C" fn(c_int)>(
                c.sym(b"printHexCharLine\0"),
            )
        };
        let f_r: unsafe extern "C" fn(c_int) = unsafe {
            std::mem::transmute::<FnPrintHexCharLine, unsafe extern "C" fn(c_int)>(
                r.sym(b"printHexCharLine\0"),
            )
        };
        let out_c = capture(|| unsafe { f_c(raw) });
        let out_r = capture(|| unsafe { f_r(raw) });
        assert_eq!(
            out_c,
            out_r,
            "DIVERGENCE [widened char arg {raw:#x}]\n  C   : {}\n  Rust: {}",
            show(&out_c),
            show(&out_r)
        );
    }
}
