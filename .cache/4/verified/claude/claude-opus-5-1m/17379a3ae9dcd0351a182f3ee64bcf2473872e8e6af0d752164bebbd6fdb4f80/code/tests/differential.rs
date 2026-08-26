//! Differential test suite: C `libdriver.so` vs. Rust `libdriver.so`.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! driven **only** through their exported `extern "C"` symbols, exactly as an
//! external consumer would.  No Rust function is ever called directly, so the
//! `#[unsafe(no_mangle)]` export wrappers are part of what is under test.
//!
//! * Phase B rows -> `phase_b_*` tests, gated on `CONFIGS.md`
//! * Phase C rows -> `phase_c_*` tests, gated on `ERRORS.md`
//! * Phase D      -> `phase_d_symbol_parity`
//!
//! The C library writes to stdout with libc `printf`, so `driver` is compared by
//! capturing file descriptor 1 (`dup2`) around each call and diffing the raw
//! bytes.  Inputs that make the C code fault (NULL pointers, `c == '\0'`,
//! unterminated buffers) are run in a re-exec'd child process so that the
//! termination signal of the C `.so` can be compared with that of the Rust
//! `.so`.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need (declared by hand so the crate keeps zero dependencies
// beyond libloading).
// ---------------------------------------------------------------------------

extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        off: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
}

const PROT_NONE: c_int = 0;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_PRIVATE: c_int = 0x02;
const MAP_ANONYMOUS: c_int = 0x20;
const SIGSEGV: i32 = 11;
const SIGBUS: i32 = 7;

// ---------------------------------------------------------------------------
// Exported ABI under test
// ---------------------------------------------------------------------------

/// `int foo(const char *in, char c)`
type FooFn = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
/// The same symbol, but called through a prototype whose second parameter is a
/// full `int`.  C `char` parameters only define the low 8 bits of the argument
/// register, so this is how an out-of-range value actually reaches the callee.
type FooIntFn = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
/// `void driver(const char *in)`
type DriverFn = unsafe extern "C" fn(*const c_char);

/// Serializes every test that touches the process-wide stdout descriptor.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn lock_stdout() -> std::sync::MutexGuard<'static, ()> {
    STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libdriver.so`, built with CMake on first use if absent.
fn c_so_path() -> PathBuf {
    let build_dir = manifest_dir().join("c_src/build");
    let so = build_dir.join("libdriver.so");
    if !so.exists() {
        std::fs::create_dir_all(&build_dir).expect("create c_src/build");
        let cfg = std::process::Command::new("cmake")
            .current_dir(&build_dir)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .status()
            .expect("run cmake (configure)");
        assert!(cfg.success(), "cmake configure failed");
        let bld = std::process::Command::new("cmake")
            .current_dir(&build_dir)
            .args(["--build", "."])
            .status()
            .expect("run cmake (build)");
        assert!(bld.success(), "cmake build failed");
    }
    assert!(so.exists(), "missing C shared object at {}", so.display());
    so
}

/// The Rust `cdylib` that cargo just built next to this test binary
/// (`target/<profile>/libdriver.so`).
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/differential-<hash>  ->  .../target/<profile>
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("test binary should live in target/<profile>/deps");
    let so = profile_dir.join("libdriver.so");
    assert!(
        so.exists(),
        "missing Rust shared object at {} (run `cargo build` first)",
        so.display()
    );
    assert_so_is_fresh(&so);
    so
}

/// Guard against a silently stale `.so`.
///
/// `cargo test` does **not** rebuild a `cdylib`-only lib target: integration
/// tests do not link against it, so cargo has no reason to. Without this check
/// the whole suite happily measures whatever `libdriver.so` was left in
/// `target/<profile>/` by an earlier `cargo build`, and edits to `src/` appear
/// to change nothing. Always run `cargo build` (same profile) before
/// `cargo test`; `./run_tests.sh` does that.
fn assert_so_is_fresh(so: &Path) {
    fn walk(dir: &Path, newest: &mut std::time::SystemTime, which: &mut PathBuf) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, newest, which);
            } else if p.extension().is_some_and(|x| x == "rs") {
                if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                    if m > *newest {
                        *newest = m;
                        *which = p;
                    }
                }
            }
        }
    }
    let so_time = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let mut src_time = std::time::SystemTime::UNIX_EPOCH;
    let mut culprit = PathBuf::new();
    walk(&manifest_dir().join("src"), &mut src_time, &mut culprit);
    if let Ok(m) = std::fs::metadata(manifest_dir().join("Cargo.toml")).and_then(|m| m.modified()) {
        if m > src_time {
            src_time = m;
            culprit = manifest_dir().join("Cargo.toml");
        }
    }
    assert!(
        so_time >= src_time,
        "STALE Rust shared object: {} is older than {}.\n\
         `cargo test` does not rebuild a cdylib-only lib target -- run \
         `cargo build` (same profile, same features) first, or use ./run_tests.sh.",
        so.display(),
        culprit.display()
    );
}

/// One loaded implementation, addressed exclusively through its dynamic symbols.
struct Impl {
    name: &'static str,
    path: PathBuf,
    lib: Library,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Self {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {} ({}): {e}", path.display(), name));
        // Both symbols must resolve in both objects (Phase A/D requirement).
        unsafe {
            let _: Symbol<FooFn> = lib.get(b"foo\0").expect("symbol `foo`");
            let _: Symbol<DriverFn> = lib.get(b"driver\0").expect("symbol `driver`");
        }
        Impl { name, path, lib }
    }

    fn foo(&self, s: *const c_char, c: c_char) -> c_int {
        unsafe {
            let f: Symbol<FooFn> = self.lib.get(b"foo\0").unwrap();
            f(s, c)
        }
    }

    fn foo_int(&self, s: *const c_char, c: c_int) -> c_int {
        unsafe {
            let f: Symbol<FooIntFn> = self.lib.get(b"foo\0").unwrap();
            f(s, c)
        }
    }

    /// Calls `driver(in)` leaving fd 1 exactly as the caller arranged it (used
    /// by the crash-parity child, which has already redirected stdout itself).
    fn driver_raw(&self, s: *const c_char) {
        unsafe {
            let d: Symbol<DriverFn> = self.lib.get(b"driver\0").unwrap();
            d(s);
        }
    }

    /// Calls `driver(in)` and returns the exact bytes it wrote to fd 1.
    fn driver_stdout(&self, s: *const c_char) -> Vec<u8> {
        let _guard = lock_stdout();
        let tmp = std::env::temp_dir().join(format!(
            "driver-capture-{}-{}.txt",
            std::process::id(),
            self.name
        ));
        let bytes = {
            let file = std::fs::File::create(&tmp).expect("create capture file");
            let fd = {
                use std::os::fd::AsRawFd;
                file.as_raw_fd()
            };
            // Flush anything already pending so it is not attributed to us.
            std::io::stdout().flush().ok();
            unsafe { fflush(std::ptr::null_mut()) };
            let saved = unsafe { dup(1) };
            assert!(saved >= 0, "dup(1) failed");
            assert!(unsafe { dup2(fd, 1) } >= 0, "dup2 onto stdout failed");

            unsafe {
                let d: Symbol<DriverFn> = self.lib.get(b"driver\0").unwrap();
                d(s);
                // libc stdout is fully buffered when redirected to a file.
                fflush(std::ptr::null_mut());
            }

            assert!(unsafe { dup2(saved, 1) } >= 0, "restore stdout failed");
            unsafe { close(saved) };
            std::fs::read(&tmp).expect("read capture file")
        };
        let _ = std::fs::remove_file(&tmp);
        bytes
    }
}

/// The pair of implementations, loaded once per test process.
struct Pair {
    c: Impl,
    rust: Impl,
}

fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let c = Impl::load("c", c_so_path());
        let rust = Impl::load("rust", rust_so_path());
        assert_ne!(
            std::fs::canonicalize(&c.path).unwrap(),
            std::fs::canonicalize(&rust.path).unwrap(),
            "the same shared object was loaded twice; the comparison would be vacuous"
        );
        Pair { c, rust }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed, reproducible runs
// ---------------------------------------------------------------------------

const SEED: u64 = 0x5EED_1234_ABCD_0001;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform-ish value in `0..n`.
    fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    /// Inclusive range.
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    fn byte_from(&mut self, alphabet: &[u8]) -> u8 {
        alphabet[self.below(alphabet.len())]
    }
    fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Oracles and assertion helpers
// ---------------------------------------------------------------------------

/// What the C is specified to compute: occurrences of `needle` strictly before
/// the first NUL.  Only valid for `needle != 0` (see `ERRORS.md` row 3).
fn ref_count(buf: &[u8], needle: u8) -> c_int {
    assert_ne!(needle, 0);
    buf.iter()
        .take_while(|&&b| b != 0)
        .filter(|&&b| b == needle)
        .count() as c_int
}

fn nul_terminated(body: &[u8]) -> Vec<u8> {
    let mut v = body.to_vec();
    v.push(0);
    v
}

fn show(buf: &[u8]) -> String {
    let shown: Vec<u8> = buf.iter().copied().take(96).collect();
    format!(
        "len={} bytes={}{}",
        buf.len(),
        shown
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(""),
        if buf.len() > 96 { "..." } else { "" }
    )
}

/// Differential check of `foo` for one input, plus an independent oracle check.
fn check_foo(row: &str, buf: &[u8], needle: u8) {
    let p = pair();
    let ptr = buf.as_ptr() as *const c_char;
    let got_c = p.c.foo(ptr, needle as i8 as c_char);
    let got_r = p.rust.foo(ptr, needle as i8 as c_char);
    assert_eq!(
        got_c, got_r,
        "[{row}] foo divergence: C={got_c} Rust={got_r} needle={needle:#04x} input {}",
        show(buf)
    );
    let expect = ref_count(buf, needle);
    assert_eq!(
        got_c, expect,
        "[{row}] oracle mismatch (C={got_c}, expected {expect}); needle={needle:#04x} input {}",
        show(buf)
    );
}

/// Differential check of `foo` called through the `int`-typed prototype.
fn check_foo_int(row: &str, buf: &[u8], needle: c_int) {
    let p = pair();
    let ptr = buf.as_ptr() as *const c_char;
    let got_c = p.c.foo_int(ptr, needle);
    let got_r = p.rust.foo_int(ptr, needle);
    assert_eq!(
        got_c, got_r,
        "[{row}] foo(int) divergence: C={got_c} Rust={got_r} needle={needle:#010x} input {}",
        show(buf)
    );
}

/// Differential check of `driver` for one input, plus an oracle on the bytes.
fn check_driver(row: &str, buf: &[u8]) {
    let p = pair();
    let ptr = buf.as_ptr() as *const c_char;
    let out_c = p.c.driver_stdout(ptr);
    let out_r = p.rust.driver_stdout(ptr);
    assert_eq!(
        out_c,
        out_r,
        "[{row}] driver stdout divergence:\n  C   = {:?}\n  Rust= {:?}\n  input {}",
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_r),
        show(buf)
    );
    let expect = format!(
        "A: {}\nx: {}\n",
        ref_count(buf, b'A'),
        ref_count(buf, b'x')
    );
    assert_eq!(
        String::from_utf8_lossy(&out_c),
        expect,
        "[{row}] driver oracle mismatch; input {}",
        show(buf)
    );
}

// ===========================================================================
// PHASE B — valid-path differential tests (one test per CONFIGS.md row group)
// ===========================================================================

/// Rows 1-3: empty haystack, len-1 match, len-1 non-match.
#[test]
fn phase_b_rows_1_2_3_tiny_haystacks() {
    let mut rng = Rng::new(SEED ^ 1);
    check_foo("row1", &nul_terminated(b""), b'A');
    check_foo("row2", &nul_terminated(b"A"), b'A');
    for _ in 0..256 {
        // any single byte that is neither NUL nor the needle
        let mut b = rng.range(1, 255) as u8;
        if b == b'A' {
            b = b'B';
        }
        check_foo("row3", &nul_terminated(&[b]), b'A');
    }
}

/// Row 4: dense two-letter alphabet, every match/miss pattern, len 2..64.
#[test]
fn phase_b_row_4_dense_two_letter_alphabet() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..3000 {
        let len = rng.range(2, 64);
        let body: Vec<u8> = (0..len).map(|_| rng.byte_from(b"AB")).collect();
        check_foo("row4", &nul_terminated(&body), b'A');
    }
    // exhaustive over all 2^8 patterns of length 8, so no pattern is left to chance
    for mask in 0u32..256 {
        let body: Vec<u8> = (0..8)
            .map(|i| if mask >> i & 1 == 1 { b'A' } else { b'B' })
            .collect();
        check_foo("row4-exhaustive", &nul_terminated(&body), b'A');
    }
}

/// Rows 5-6: single match pinned at the first / last index.
#[test]
fn phase_b_rows_5_6_match_at_edges() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..500 {
        let len = rng.range(1, 64);
        let mut body: Vec<u8> = (0..len).map(|_| rng.byte_from(b"BCDeF9 ")).collect();
        body[0] = b'A';
        check_foo("row5", &nul_terminated(&body), b'A');

        let mut body2: Vec<u8> = (0..len).map(|_| rng.byte_from(b"BCDeF9 ")).collect();
        *body2.last_mut().unwrap() = b'A';
        check_foo("row6", &nul_terminated(&body2), b'A');
    }
}

/// Row 7: adjacent runs of the needle (exercises the `s++` after a hit).
#[test]
fn phase_b_row_7_adjacent_runs() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..1000 {
        let mut body = Vec::new();
        let runs = rng.range(1, 8);
        for _ in 0..runs {
            for _ in 0..rng.range(1, 9) {
                body.push(b'A');
            }
            for _ in 0..rng.below(4) {
                body.push(b'B');
            }
        }
        check_foo("row7", &nul_terminated(&body), b'A');
    }
    // pure runs, including the degenerate all-needle string
    for len in 1..=64 {
        check_foo("row7-pure", &nul_terminated(&vec![b'A'; len]), b'A');
    }
}

/// Row 8: guaranteed zero occurrences.
#[test]
fn phase_b_row_8_zero_occurrences() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..1000 {
        let len = rng.range(1, 64);
        let body: Vec<u8> = (0..len)
            .map(|_| {
                let mut b = rng.range(1, 255) as u8;
                if b == b'A' {
                    b = b'Z';
                }
                b
            })
            .collect();
        check_foo("row8", &nul_terminated(&body), b'A');
    }
}

/// Row 9: the other needle `driver` hard-codes, random printable haystacks.
#[test]
fn phase_b_row_9_needle_x_printable() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..2000 {
        let len = rng.below(257);
        let body: Vec<u8> = (0..len).map(|_| rng.range(0x20, 0x7e) as u8).collect();
        check_foo("row9", &nul_terminated(&body), b'x');
    }
}

/// Row 10: random ASCII needle vs. random ASCII haystack.
#[test]
fn phase_b_row_10_random_ascii() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..3000 {
        let needle = rng.range(0x01, 0x7f) as u8;
        let len = rng.below(257);
        let body: Vec<u8> = (0..len).map(|_| rng.range(0x01, 0x7f) as u8).collect();
        check_foo("row10", &nul_terminated(&body), needle);
    }
    // every possible non-zero ASCII needle at least once
    for needle in 0x01u8..=0x7f {
        let body: Vec<u8> = (0x01u8..=0x7f).collect();
        check_foo("row10-all-needles", &nul_terminated(&body), needle);
    }
}

/// Rows 11-12: full byte range, i.e. needles/bytes with the high bit set, where
/// `char` is *signed* on x86-64 and `strchr` compares as `unsigned char`.
#[test]
fn phase_b_rows_11_12_full_byte_range_signedness() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..3000 {
        let needle = rng.range(0x01, 0xff) as u8;
        let len = rng.below(257);
        let body: Vec<u8> = (0..len).map(|_| rng.range(0x01, 0xff) as u8).collect();
        check_foo("row11", &nul_terminated(&body), needle);
    }
    // row 12: high-bit needle guaranteed present among other high-bit bytes
    for _ in 0..1000 {
        let needle = rng.range(0x80, 0xff) as u8;
        let len = rng.range(1, 64);
        let mut body: Vec<u8> = (0..len).map(|_| rng.range(0x80, 0xff) as u8).collect();
        body[rng.below(len)] = needle;
        check_foo("row12", &nul_terminated(&body), needle);
    }
    // every possible non-zero needle at least once against a full-range haystack
    let full: Vec<u8> = (0x01u8..=0xff).collect();
    for needle in 0x01u8..=0xff {
        check_foo("row11-all-needles", &nul_terminated(&full), needle);
    }
}

/// Rows 13-14: large haystacks; counts with 4 and 6 decimal digits.
#[test]
fn phase_b_rows_13_14_large_haystacks() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..20 {
        let len = rng.range(4096, 8192);
        let body: Vec<u8> = (0..len).map(|_| rng.byte_from(b"AB")).collect();
        check_foo("row13", &nul_terminated(&body), b'A');
    }
    let len = 1024 * 1024;
    let body: Vec<u8> = (0..len).map(|_| rng.byte_from(b"AB")).collect();
    check_foo("row14", &nul_terminated(&body), b'A');
    // and a 1 MiB all-needle buffer: the maximum possible count for that size
    check_foo("row14-max", &nul_terminated(&vec![b'A'; len]), b'A');
}

/// Row 15: embedded NUL — matches after it must be invisible.
#[test]
fn phase_b_row_15_embedded_nul() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..1000 {
        let head: Vec<u8> = (0..rng.below(32)).map(|_| rng.byte_from(b"AB")).collect();
        let tail: Vec<u8> = (0..rng.range(1, 32)).map(|_| rng.byte_from(b"AB")).collect();
        let mut buf = head.clone();
        buf.push(0);
        buf.extend_from_slice(&tail);
        buf.push(0);
        check_foo("row15", &buf, b'A');
        check_foo("row15", &buf, b'x');
    }
}

/// Row 16: `c` delivered as a full `int` with garbage in the upper 24 bits.
/// Values whose low byte is `0x00` are excluded here because they select the
/// fatal `c == '\0'` path; they are covered by `phase_c_int_arg_low_byte_zero`.
#[test]
fn phase_b_row_16_out_of_range_int_argument() {
    let mut rng = Rng::new(SEED ^ 16);
    let buf = nul_terminated(b"AAxxA\x41\x78\xff\x01 A");
    let fixed: [c_int; 12] = [
        0x141,
        0x1_0041,
        0x7fff_ff41,
        -1,
        -191, // 0xFFFF_FF41 as i32
        i32::MAX,
        0xff,
        0x100 | b'x' as c_int,
        -256 | b'A' as c_int,
        0x2A41,
        i32::MIN + 1,
        0x0BAD_BE41u32 as c_int,
    ];
    for v in fixed {
        check_foo_int("row16-fixed", &buf, v);
    }
    for _ in 0..2000 {
        let low = rng.range(1, 255) as c_int; // never 0
        let high = (rng.next_u64() as u32 & 0xffff_ff00) as c_int;
        let v = high | low;
        let len = rng.below(129);
        let body: Vec<u8> = (0..len).map(|_| rng.range(0x01, 0xff) as u8).collect();
        let hay = nul_terminated(&body);
        check_foo_int("row16-random", &hay, v);
        // and the truncation must agree with the plain `char` prototype
        let p = pair();
        let via_int = p.rust.foo_int(hay.as_ptr() as *const c_char, v);
        let via_char = p.c.foo(hay.as_ptr() as *const c_char, low as u8 as i8);
        assert_eq!(
            via_int, via_char,
            "row16: int-arg {v:#010x} did not truncate to {low:#04x}"
        );
    }
}

/// Row 17: interior (`in + k`) pointers.
#[test]
fn phase_b_row_17_interior_pointer() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..1000 {
        let len = rng.range(1, 96);
        let body: Vec<u8> = (0..len).map(|_| rng.byte_from(b"AxBy")).collect();
        let buf = nul_terminated(&body);
        let k = rng.below(buf.len()); // may point directly at the NUL
        check_foo("row17", &buf[k..], b'A');
        check_foo("row17", &buf[k..], b'x');
    }
}

/// Rows 18-20: `driver` with (0,0), (n,0) and (0,n) counts.
#[test]
fn phase_b_rows_18_19_20_driver_single_class() {
    let mut rng = Rng::new(SEED ^ 18);
    check_driver("row18", &nul_terminated(b""));
    for _ in 0..40 {
        let n = rng.range(1, 300);
        check_driver("row19", &nul_terminated(&vec![b'A'; n]));
        let m = rng.range(1, 300);
        check_driver("row20", &nul_terminated(&vec![b'x'; m]));
    }
    // digit-width boundaries for both lines
    for n in [1usize, 9, 10, 99, 100, 999, 1000] {
        check_driver("row19-width", &nul_terminated(&vec![b'A'; n]));
        check_driver("row20-width", &nul_terminated(&vec![b'x'; n]));
    }
}

/// Row 21: both counts non-zero; case variants must be ignored.
#[test]
fn phase_b_row_21_driver_mixed_case_variants() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..120 {
        let len = rng.range(1, 200);
        let body: Vec<u8> = (0..len).map(|_| rng.byte_from(b"AxaXB")).collect();
        check_driver("row21", &nul_terminated(&body));
    }
}

/// Row 22: random full-byte-range input.
#[test]
fn phase_b_row_22_driver_full_byte_range() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..120 {
        let len = rng.below(513);
        let body: Vec<u8> = (0..len).map(|_| rng.range(0x01, 0xff) as u8).collect();
        check_driver("row22", &nul_terminated(&body));
    }
}

/// Row 23: `printf`-hostile bytes in the *input* (the format is a fixed literal).
#[test]
fn phase_b_row_23_driver_format_hostile_input() {
    let mut rng = Rng::new(SEED ^ 23);
    for fixed in [
        &b"%s"[..],
        b"%n",
        b"%d %d %d",
        b"%A%x%%",
        b"A%nx",
        b"\\n\nA\tx\r%p",
        b"%999999999d",
        b"x%sA%nA",
    ] {
        check_driver("row23-fixed", &nul_terminated(fixed));
    }
    for _ in 0..120 {
        let len = rng.range(1, 128);
        let body: Vec<u8> = (0..len)
            .map(|_| rng.byte_from(b"%sndpxA\n\\\"\t "))
            .collect();
        check_driver("row23-random", &nul_terminated(&body));
    }
}

/// Row 24: wide, asymmetric counts (5-6 digits on one line, fewer on the other).
#[test]
fn phase_b_row_24_driver_wide_counts() {
    let mut rng = Rng::new(SEED ^ 24);
    for (na, nx) in [
        (100_000usize, 7usize),
        (7, 100_000),
        (12_345, 999),
        (99_999, 100_000),
        (1, 654_321),
    ] {
        let mut body = vec![b'A'; na];
        body.extend(std::iter::repeat(b'x').take(nx));
        check_driver("row24", &nul_terminated(&body));
    }
    for _ in 0..10 {
        let na = rng.range(10_000, 200_000);
        let nx = rng.range(1, 20_000);
        let mut body = vec![b'A'; na];
        body.extend(std::iter::repeat(b'x').take(nx));
        check_driver("row24-random", &nul_terminated(&body));
    }
}

/// Row 25: `driver` with an embedded NUL.
#[test]
fn phase_b_row_25_driver_embedded_nul() {
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..120 {
        let head: Vec<u8> = (0..rng.below(64)).map(|_| rng.byte_from(b"AxB")).collect();
        let tail: Vec<u8> = (0..rng.range(1, 64)).map(|_| rng.byte_from(b"AxB")).collect();
        let mut buf = head;
        buf.push(0);
        buf.extend_from_slice(&tail);
        buf.push(0);
        check_driver("row25", &buf);
    }
}

/// Row 26: the composed pipeline — the integers `driver` prints must be exactly
/// what `foo` returns, both within one `.so` and across the two `.so`s.
#[test]
fn phase_b_row_26_pipeline_consistency() {
    let mut rng = Rng::new(SEED ^ 26);
    let p = pair();
    for _ in 0..150 {
        let len = rng.below(300);
        let body: Vec<u8> = (0..len).map(|_| rng.byte_from(b"AxaXB\x01\xff")).collect();
        let buf = nul_terminated(&body);
        let ptr = buf.as_ptr() as *const c_char;

        for (name, imp) in [("c", &p.c), ("rust", &p.rust)] {
            let out = imp.driver_stdout(ptr);
            let text = String::from_utf8(out).expect("driver output is ASCII");
            let mut lines = text.lines();
            let a: c_int = lines
                .next()
                .and_then(|l| l.strip_prefix("A: "))
                .expect("first line shape")
                .parse()
                .expect("first count");
            let x: c_int = lines
                .next()
                .and_then(|l| l.strip_prefix("x: "))
                .expect("second line shape")
                .parse()
                .expect("second count");
            assert_eq!(lines.next(), None, "row26: exactly two lines expected");
            // same object, and cross-object
            assert_eq!(a, imp.foo(ptr, b'A' as c_char), "row26 {name}: A line vs foo");
            assert_eq!(x, imp.foo(ptr, b'x' as c_char), "row26 {name}: x line vs foo");
            assert_eq!(a, p.c.foo(ptr, b'A' as c_char), "row26 {name}: A line vs C foo");
            assert_eq!(x, p.rust.foo(ptr, b'x' as c_char), "row26 {name}: x vs Rust foo");
        }
    }
}

/// Row 27: statelessness / order independence, C->Rust->C interleaved.
#[test]
fn phase_b_row_27_statelessness_and_interleaving() {
    let mut rng = Rng::new(SEED ^ 27);
    let p = pair();
    for _ in 0..150 {
        let len = rng.below(200);
        let body: Vec<u8> = (0..len).map(|_| rng.byte_from(b"AxB")).collect();
        let buf = nul_terminated(&body);
        let ptr = buf.as_ptr() as *const c_char;
        let mut results = Vec::new();
        for _ in 0..3 {
            results.push(p.c.foo(ptr, b'A' as c_char));
            results.push(p.rust.foo(ptr, b'A' as c_char));
            results.push(p.c.foo(ptr, b'x' as c_char));
            results.push(p.rust.foo(ptr, b'x' as c_char));
        }
        for chunk in results.chunks(4) {
            assert_eq!(chunk[0], chunk[1], "row27: foo('A') C vs Rust");
            assert_eq!(chunk[2], chunk[3], "row27: foo('x') C vs Rust");
        }
        assert!(
            results.chunks(4).all(|c| c == &results[0..4]),
            "row27: repeated calls are not stateless: {results:?}"
        );
        let o1 = p.c.driver_stdout(ptr);
        let o2 = p.rust.driver_stdout(ptr);
        let o3 = p.c.driver_stdout(ptr);
        let o4 = p.rust.driver_stdout(ptr);
        assert_eq!(o1, o2, "row27: driver C vs Rust");
        assert_eq!(o1, o3, "row27: driver C is not idempotent");
        assert_eq!(o2, o4, "row27: driver Rust is not idempotent");
    }
}

// ===========================================================================
// PHASE C — error-path differential tests (one per ERRORS.md row)
// ===========================================================================
//
// The C library validates nothing, so most of its error surface is a memory
// fault.  A fault cannot be observed in-process, so each faulting row is run in
// a re-exec'd child of this very test binary: the parent compares the child's
// termination signal, exit code, captured stdout and (when it survives) the
// returned value between the C `.so` and the Rust `.so`.

const CASE_ENV: &str = "DIFF_CRASH_CASE";
const OUT_ENV: &str = "DIFF_CRASH_STDOUT";
const RET_ENV: &str = "DIFF_CRASH_RET";
const CHILD_TEST: &str = "phase_c_crash_child_helper";
const PAGE: usize = 4096;

/// Copies `payload` so that its last byte sits at the very end of a readable
/// page whose successor page is `PROT_NONE`.  Any read past the payload
/// therefore faults immediately and deterministically, instead of wandering
/// through whatever the heap happens to contain.
fn guarded(payload: &[u8]) -> *const c_char {
    assert!(payload.len() <= PAGE);
    let base = unsafe {
        mmap(
            std::ptr::null_mut(),
            2 * PAGE,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert!(base as isize > 0, "mmap failed");
    let guard = unsafe { (base as *mut u8).add(PAGE) };
    assert_eq!(
        unsafe { mprotect(guard as *mut c_void, PAGE, PROT_NONE) },
        0,
        "mprotect failed"
    );
    let start = unsafe { (base as *mut u8).add(PAGE - payload.len()) };
    unsafe { std::ptr::copy_nonoverlapping(payload.as_ptr(), start, payload.len()) };
    start as *const c_char
}

/// Outcome of one child run.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    signal: Option<i32>,
    code: Option<i32>,
    stdout: Vec<u8>,
    ret: Option<String>,
}

fn run_case(lib: &str, scenario: &str) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let tag = format!("{}-{}-{}", std::process::id(), lib, scenario);
    let out_path = std::env::temp_dir().join(format!("diffcrash-out-{tag}"));
    let ret_path = std::env::temp_dir().join(format!("diffcrash-ret-{tag}"));
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&ret_path);

    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([CHILD_TEST, "--exact", "--nocapture", "--test-threads=1"])
        .env(CASE_ENV, format!("{lib}:{scenario}"))
        .env(OUT_ENV, &out_path)
        .env(RET_ENV, &ret_path)
        .env("RUST_BACKTRACE", "0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("spawn crash-case child");

    let outcome = Outcome {
        signal: status.signal(),
        code: status.code(),
        stdout: std::fs::read(&out_path).unwrap_or_default(),
        ret: std::fs::read_to_string(&ret_path).ok(),
    };
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&ret_path);
    outcome
}

/// Runs `scenario` against both `.so`s and asserts identical outcomes.
fn check_case(row: &str, scenario: &str) -> Outcome {
    let c = run_case("c", scenario);
    let r = run_case("rust", scenario);
    assert_eq!(
        c, r,
        "[{row}] scenario `{scenario}` diverged:\n  C   = {c:?}\n  Rust= {r:?}"
    );
    c
}

/// Asserts both implementations died from the *same* memory fault (and really
/// did fault, rather than merely "failing somehow" such as a Rust panic).
fn check_fatal(row: &str, scenario: &str, expect_stdout: &[u8]) {
    let o = check_case(row, scenario);
    assert!(
        o.signal == Some(SIGSEGV) || o.signal == Some(SIGBUS),
        "[{row}] scenario `{scenario}`: expected a memory fault, got {o:?}"
    );
    assert_eq!(
        o.ret, None,
        "[{row}] scenario `{scenario}`: the call returned instead of faulting"
    );
    assert_eq!(
        o.stdout, expect_stdout,
        "[{row}] scenario `{scenario}`: unexpected bytes on stdout: {:?}",
        String::from_utf8_lossy(&o.stdout)
    );
}

/// Asserts both implementations returned normally with the same value.
fn check_survives(row: &str, scenario: &str, expect_ret: &str) {
    let o = check_case(row, scenario);
    assert_eq!(
        o.signal, None,
        "[{row}] scenario `{scenario}`: unexpected fatal signal: {o:?}"
    );
    assert_eq!(
        o.code,
        Some(0),
        "[{row}] scenario `{scenario}`: non-zero exit: {o:?}"
    );
    assert_eq!(
        o.ret.as_deref(),
        Some(expect_ret),
        "[{row}] scenario `{scenario}`: wrong return value: {o:?}"
    );
}

/// The child half of the crash harness.  A no-op unless `DIFF_CRASH_CASE` is
/// set, so it is inert during a normal `cargo test` run.
#[test]
fn phase_c_crash_child_helper() {
    let Ok(case) = std::env::var(CASE_ENV) else {
        return;
    };
    let (which, scenario) = case.split_once(':').expect("CASE format lib:scenario");
    let imp = match which {
        "c" => Impl::load("c", c_so_path()),
        "rust" => Impl::load("rust", rust_so_path()),
        other => panic!("unknown implementation `{other}`"),
    };

    // Redirect fd 1 into the file the parent will read, so that only bytes the
    // library itself emits are attributed to it.
    let out_path = std::env::var(OUT_ENV).expect(OUT_ENV);
    let file = std::fs::File::create(&out_path).expect("create child stdout file");
    let saved = {
        use std::os::fd::AsRawFd;
        std::io::stdout().flush().ok();
        unsafe { fflush(std::ptr::null_mut()) };
        let saved = unsafe { dup(1) };
        assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0);
        saved
    };

    // Any of these may kill the process; that is the point.
    let ret: Option<c_int> = match scenario {
        // ERRORS.md rows 1-2: NULL input pointer.
        "null_a" => Some(imp.foo(std::ptr::null(), b'A' as c_char)),
        "null_nul" => Some(imp.foo(std::ptr::null(), 0)),
        // row 3: c == '\0' -> strchr never returns NULL -> runs off the object.
        "nul_needle" => Some(imp.foo(guarded(b"hello\0"), 0)),
        "nul_needle_empty" => Some(imp.foo(guarded(b"\0"), 0)),
        // rows 4-5: buffer without a terminator, needle present / absent.
        "unterm_present" => Some(imp.foo(guarded(&[b'A'; 64]), b'A' as c_char)),
        "unterm_absent" => Some(imp.foo(guarded(&[b'B'; 64]), b'A' as c_char)),
        // row 6: driver(NULL) must fault before printing anything at all.
        "driver_null" => {
            imp.driver_raw(std::ptr::null());
            None
        }
        // row 9 boundary: an out-of-range `int` whose low byte is 0x00 selects
        // the fatal `c == '\0'` path just like a real NUL would.
        "int_nul_needle" => Some(imp.foo_int(guarded(b"hello\0"), 0x100)),
        "int_min_needle" => Some(imp.foo_int(guarded(b"hi\0"), i32::MIN)),
        // Positive controls: prove the harness reports survival correctly, and
        // that a guarded buffer is not intrinsically fatal.
        "ok_empty" => Some(imp.foo(guarded(b"\0"), b'A' as c_char)),
        "ok_count" => Some(imp.foo(guarded(b"AxAA\0"), b'A' as c_char)),
        "ok_at_nul" => {
            let p = guarded(b"AAA\0");
            Some(imp.foo(unsafe { p.add(3) }, b'A' as c_char))
        }
        "ok_driver" => {
            imp.driver_raw(guarded(b"AxxA\0"));
            None
        }
        other => panic!("unknown scenario `{other}`"),
    };

    // Survived: flush the library's output, put stdout back so the test
    // harness's own chatter is not mistaken for library output, and report.
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0);
    unsafe { close(saved) };
    if let Some(v) = ret {
        std::fs::write(std::env::var(RET_ENV).unwrap(), format!("{v}")).unwrap();
    } else {
        std::fs::write(std::env::var(RET_ENV).unwrap(), "void").unwrap();
    }
}

/// ERRORS.md rows 1 and 2: `foo(NULL, ...)`.
#[test]
fn phase_c_rows_1_2_null_input_pointer() {
    check_fatal("row1", "null_a", b"");
    check_fatal("row2", "null_nul", b"");
}

/// ERRORS.md row 3: `c == '\0'` never terminates the loop.
#[test]
fn phase_c_row_3_nul_needle_runs_off_the_object() {
    check_fatal("row3", "nul_needle", b"");
    check_fatal("row3", "nul_needle_empty", b"");
}

/// ERRORS.md rows 4 and 5: unterminated buffer, needle present / absent.
#[test]
fn phase_c_rows_4_5_unterminated_buffer() {
    check_fatal("row4", "unterm_present", b"");
    check_fatal("row5", "unterm_absent", b"");
}

/// ERRORS.md row 6: `driver(NULL)` faults with no output whatsoever.
#[test]
fn phase_c_row_6_driver_null() {
    check_fatal("row6", "driver_null", b"");
}

/// ERRORS.md rows 7 and 8: zero length, and a pointer aimed at a terminator.
#[test]
fn phase_c_rows_7_8_zero_length_and_one_past_end() {
    check_survives("row7", "ok_empty", "0");
    check_survives("row8", "ok_at_nul", "0");
    // in-process equivalents, driven directly through both `.so`s
    check_foo("row7", &nul_terminated(b""), b'A');
    let buf = nul_terminated(b"AAA");
    check_foo("row8", &buf[3..], b'A');
    check_foo("row8", &buf[buf.len() - 1..], b'x');
    check_driver("row7", &nul_terminated(b""));
    check_driver("row8", &buf[3..]);
}

/// ERRORS.md row 9: out-of-range `int` in the `char` slot, including the two
/// values whose low byte is `0x00` (which are fatal in *both* implementations).
#[test]
fn phase_c_row_9_out_of_range_int_argument() {
    check_fatal("row9", "int_nul_needle", b"");
    check_fatal("row9", "int_min_needle", b"");
    // non-fatal out-of-range values: identical results, equal to the low byte
    let buf = nul_terminated(b"AAxxA\x41\x78\xff\x01 A");
    for v in [
        0x141,
        0x1_0041,
        0x7fff_ff41,
        -1,
        i32::MAX,
        0x100 | b'x' as c_int,
        0xdead_be41u32 as c_int,
        i32::MIN + 1,
    ] {
        check_foo_int("row9", &buf, v);
    }
}

/// ERRORS.md row 10: high-bit (negative `char`) needles.
#[test]
fn phase_c_row_10_negative_char_needle() {
    let mut rng = Rng::new(SEED ^ 110);
    for needle in 0x80u8..=0xff {
        let body: Vec<u8> = (0x80u8..=0xff).collect();
        check_foo("row10", &nul_terminated(&body), needle);
        check_foo_int("row10", &nul_terminated(&body), needle as i8 as c_int);
        check_foo_int("row10", &nul_terminated(&body), needle as c_int);
    }
    for _ in 0..500 {
        let needle = rng.range(0x80, 0xff) as u8;
        let len = rng.range(1, 40);
        let body: Vec<u8> = (0..len)
            .map(|_| {
                if rng.bool() {
                    needle
                } else {
                    rng.range(0x01, 0xff) as u8
                }
            })
            .collect();
        check_foo("row10", &nul_terminated(&body), needle);
    }
}

/// ERRORS.md row 11: no length parameter and no cap — a 1 MiB haystack with
/// ~524 288 matches is accepted without complaint.
#[test]
fn phase_c_row_11_oversized_unchecked_length() {
    let mut rng = Rng::new(SEED ^ 111);
    let len = 1024 * 1024;
    let body: Vec<u8> = (0..len).map(|_| rng.byte_from(b"AB")).collect();
    check_foo("row11", &nul_terminated(&body), b'A');
    check_foo("row11", &nul_terminated(&vec![b'A'; len]), b'A');
}

/// ERRORS.md row 12: counts wider than any literal in the format string.
#[test]
fn phase_c_row_12_unbounded_printf_width() {
    let mut body = vec![b'A'; 123_456];
    body.extend(std::iter::repeat(b'x').take(7));
    check_driver("row12", &nul_terminated(&body));
}

/// ERRORS.md row 13: matches hidden behind an embedded NUL are not counted.
#[test]
fn phase_c_row_13_matches_hidden_behind_embedded_nul() {
    let mut buf = b"Ax".to_vec();
    buf.push(0);
    buf.extend_from_slice(b"AAAAxxxx");
    buf.push(0);
    check_foo("row13", &buf, b'A');
    check_foo("row13", &buf, b'x');
    check_driver("row13", &buf);
    // NUL as the very first byte hides everything
    let mut buf2 = vec![0u8];
    buf2.extend_from_slice(b"AAxx");
    buf2.push(0);
    check_foo("row13", &buf2, b'A');
    check_driver("row13", &buf2);
}

/// The harness itself must be trustworthy: a scenario that does *not* fault has
/// to be reported as surviving with the right value and the right stdout bytes.
#[test]
fn phase_c_harness_positive_controls() {
    check_survives("control", "ok_count", "3"); // "AxAA" holds three A
    let o = check_case("control", "ok_driver");
    assert_eq!(o.signal, None, "control ok_driver should not fault: {o:?}");
    assert_eq!(o.stdout, b"A: 2\nx: 2\n", "control ok_driver stdout");
}

// ===========================================================================
// PHASE D — symbol parity
// ===========================================================================

fn defined_dynamic_symbols(so: &Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(|s| s.to_string())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// under the exact same name.  The diff must be empty.
#[test]
fn phase_d_symbol_parity() {
    let c = defined_dynamic_symbols(&c_so_path());
    let r = defined_dynamic_symbols(&rust_so_path());
    assert_eq!(
        c,
        vec!["driver".to_string(), "foo".to_string()],
        "the C `.so` exports an unexpected symbol set; the tables need updating"
    );
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C `.so` but missing from the Rust `.so`: {missing:?}\n\
         C   = {c:?}\nRust= {r:?}"
    );
    for want in ["foo", "driver"] {
        assert!(r.contains(&want.to_string()), "Rust `.so` lacks `{want}`");
    }
}

/// The Rust `.so` must not have unresolved (non-libc) dependencies.
#[test]
fn phase_d_no_undefined_symbols() {
    let out = std::process::Command::new("ldd")
        .args(["-r"])
        .arg(rust_so_path())
        .output()
        .expect("run ldd");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
        .collect();
    assert!(bad.is_empty(), "unresolved symbols in the Rust `.so`: {bad:#?}");
}
