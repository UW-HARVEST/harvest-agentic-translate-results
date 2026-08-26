// Differential test: the C `.so` vs the Rust `.so`, both loaded with `libloading`.
//
// Rust code is NEVER called directly from this test: every Rust call goes through
// a symbol resolved out of `target/<profile>/libdriver.so`, exactly as an external
// C consumer would, so the `#[no_mangle]`/`extern "C"` export wrappers are under
// test too.
//
// The whole library returns `void` everywhere; its only observable effect is the
// byte stream it writes to the C runtime's `stdout`. So each row captures `stdout`
// (dup2 onto a temp file + `fflush(NULL)`) around the C call and around the Rust
// call, and asserts the two buffers are identical byte-for-byte.
//
// `harness = false`: fd-1 redirection is process-global, so the rows must run
// strictly sequentially. libtest's own progress lines also go to fd 1 and would
// otherwise leak into a capture taken by a parallel test thread.

use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;

use libloading::Library;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// deterministic PRNG (xorshift64*), so every "randomized" row is reproducible
// ---------------------------------------------------------------------------

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
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// uniform-ish in `[lo, hi]`
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        debug_assert!(lo <= hi);
        lo + self.next_u64() % (hi - lo + 1)
    }
    fn byte_excluding_nul(&mut self) -> u8 {
        (self.range(1, 255)) as u8
    }
    fn printable_ascii(&mut self) -> u8 {
        (self.range(0x20, 0x7E)) as u8
    }
    /// random index in `0..len`
    fn idx(&mut self, len: usize) -> usize {
        (self.next_u64() % len as u64) as usize
    }
}

// ---------------------------------------------------------------------------
// the library surface, resolved dynamically from a `.so`
// ---------------------------------------------------------------------------

type FnVoid = unsafe extern "C" fn();
type FnLine = unsafe extern "C" fn(*const c_char);
type FnInt = unsafe extern "C" fn(c_int);
/// Same symbol as `print_int_line`, but declared 64-bit wide on purpose, to probe
/// what happens when a caller passes a dirty-upper-bits argument across the FFI
/// boundary to a function whose C prototype says `int` (ERRORS.md row 12).
type FnInt64 = unsafe extern "C" fn(i64);

struct Api {
    name: &'static str,
    path: PathBuf,
    _lib: Library,
    print_line: FnLine,
    print_int_line: FnInt,
    print_int_line_64: FnInt64,
    bad: FnVoid,
    good: FnVoid,
    driver: FnVoid,
}

impl Api {
    fn load(name: &'static str, path: &Path) -> Api {
        assert!(
            path.exists(),
            "{} shared library not found at {}\n\
             build it first:\n  \
             C:    cd c_src && mkdir -p build && cd build && cmake .. \
             -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  \
             Rust: cargo build --offline",
            name,
            path.display()
        );
        unsafe {
            let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
            let sym = |n: &[u8]| -> usize {
                let s: libloading::Symbol<'_, *const c_void> = lib
                    .get(n)
                    .unwrap_or_else(|e| panic!("{} missing symbol {:?}: {e}", name, String::from_utf8_lossy(n)));
                *s as usize
            };
            let print_line: FnLine = std::mem::transmute(sym(b"printLine\0".as_slice()));
            let pil = sym(b"printIntLine\0".as_slice());
            let print_int_line: FnInt = std::mem::transmute(pil);
            let print_int_line_64: FnInt64 = std::mem::transmute(pil);
            let bad: FnVoid = std::mem::transmute(sym(b"bad\0".as_slice()));
            let good: FnVoid = std::mem::transmute(sym(b"good\0".as_slice()));
            let driver: FnVoid = std::mem::transmute(sym(b"driver\0".as_slice()));
            Api {
                name,
                path: path.to_path_buf(),
                _lib: lib,
                print_line,
                print_int_line,
                print_int_line_64,
                bad,
                good,
                driver,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// one replayable call sequence, so C and Rust are driven with identical input
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Op {
    /// `printLine(ptr)` with a NUL terminator appended to these bytes.
    Line(Vec<u8>),
    /// `printLine(NULL)`
    LineNull,
    /// `printIntLine(v)`
    Int(i32),
    /// `printIntLine(v)` called through a 64-bit-wide prototype
    Int64(i64),
    Good,
    Bad,
    Driver,
}

unsafe fn run_ops(api: &Api, ops: &[Op]) {
    for op in ops {
        match op {
            Op::Line(bytes) => {
                let mut buf = bytes.clone();
                buf.push(0);
                (api.print_line)(buf.as_ptr() as *const c_char);
            }
            Op::LineNull => (api.print_line)(std::ptr::null()),
            Op::Int(v) => (api.print_int_line)(*v),
            Op::Int64(v) => (api.print_int_line_64)(*v),
            Op::Good => (api.good)(),
            Op::Bad => (api.bad)(),
            Op::Driver => (api.driver)(),
        }
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn tmp_capture_path(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-diff-{}-{}-{}.out",
        std::process::id(),
        tag,
        n
    ))
}

/// Redirect fd 1 to a fresh temp file, run `f`, flush all C streams, restore fd 1
/// and return the bytes that were written.
fn capture<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let path = tmp_capture_path(tag);
    let file = fs::File::create(&path).expect("create capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };
    drop(file);

    let data = fs::read(&path).expect("read capture file");
    let _ = fs::remove_file(&path);
    data
}

fn escape(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b.iter().take(400) {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7E => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    if b.len() > 400 {
        s.push_str(&format!("...(+{} bytes)", b.len() - 400));
    }
    s
}

// ---------------------------------------------------------------------------
// the differential assertion
// ---------------------------------------------------------------------------

struct Case<'a> {
    c: &'a Api,
    r: &'a Api,
    failures: Vec<String>,
    checks: u64,
}

impl<'a> Case<'a> {
    /// Drive both `.so`s with the same op sequence and require identical stdout.
    fn diff(&mut self, what: &str, ops: &[Op]) -> Vec<u8> {
        let c_out = capture("c", || unsafe { run_ops(self.c, ops) });
        let r_out = capture("r", || unsafe { run_ops(self.r, ops) });
        self.checks += 1;
        if c_out != r_out {
            let at = c_out
                .iter()
                .zip(r_out.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(c_out.len().min(r_out.len()));
            self.failures.push(format!(
                "  DIVERGENCE [{what}]\n    ops: {:?}\n    first diff at byte {at}\n    \
                 C   ({} bytes): \"{}\"\n    Rust({} bytes): \"{}\"",
                OpsBrief(ops),
                c_out.len(),
                escape(&c_out),
                r_out.len(),
                escape(&r_out),
            ));
        }
        c_out
    }

    /// Extra absolute check on any comparable value.
    fn expect_val<T: PartialEq + std::fmt::Debug>(&mut self, what: &str, got: T, expect: T) {
        self.checks += 1;
        if got != expect {
            self.failures.push(format!(
                "  C-BEHAVIOUR CHECK FAILED [{what}]\n    expected {expect:?}\n    got      {got:?}"
            ));
        }
    }

    /// Extra absolute check: the C output itself must equal `expect`.
    fn expect_c(&mut self, what: &str, got: &[u8], expect: &[u8]) {
        self.checks += 1;
        if got != expect {
            self.failures.push(format!(
                "  C-BEHAVIOUR CHECK FAILED [{what}]\n    expected \"{}\"\n    got      \"{}\"",
                escape(expect),
                escape(got)
            ));
        }
    }
}

/// Compact `Debug` for op lists so huge payloads don't flood the report.
struct OpsBrief<'a>(&'a [Op]);

impl std::fmt::Debug for OpsBrief<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, op) in self.0.iter().take(8).enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            match op {
                Op::Line(b) if b.len() > 48 => write!(f, "Line(<{} bytes>)", b.len())?,
                Op::Line(b) => write!(f, "Line(\"{}\")", escape(b))?,
                other => write!(f, "{other:?}")?,
            }
        }
        if self.0.len() > 8 {
            write!(f, ", ...(+{} ops)", self.0.len() - 8)?;
        }
        write!(f, "]")
    }
}

// ---------------------------------------------------------------------------
// input generators
// ---------------------------------------------------------------------------

fn gen(rng: &mut Rng, len: usize, mut f: impl FnMut(&mut Rng) -> u8) -> Vec<u8> {
    (0..len).map(|_| f(rng)).collect()
}

/// Batch many single-arg calls into one capture: fewer fd flips, and it also
/// exercises the shared stdout buffer across consecutive calls.
fn batched(ops: Vec<Op>, chunk: usize) -> Vec<Vec<Op>> {
    ops.chunks(chunk).map(|c| c.to_vec()).collect()
}

// ---------------------------------------------------------------------------
// PHASE B — CONFIGS.md rows 1..22 (valid inputs)
// ---------------------------------------------------------------------------

fn phase_b(cs: &mut Case) {
    // row 1 — printLine(NULL): the false side of the library's only branch
    let out = cs.diff("cfg_01_print_line_null", &[Op::LineNull]);
    cs.expect_c("cfg_01 C writes nothing for NULL", &out, b"");

    // row 2 — length 0
    let out = cs.diff("cfg_02_print_line_empty", &[Op::Line(vec![])]);
    cs.expect_c("cfg_02 C prints bare newline", &out, b"\n");

    // row 3 — length 1, randomized printable ASCII (seed 0x1234_5678, 256 draws)
    let mut rng = Rng::new(0x1234_5678);
    let ops: Vec<Op> = (0..256)
        .map(|_| Op::Line(gen(&mut rng, 1, |r| r.printable_ascii())))
        .collect();
    for (i, b) in batched(ops, 32).iter().enumerate() {
        cs.diff(&format!("cfg_03_print_line_len1_random[{i}]"), b);
    }

    // row 4 — length 2..=255, randomized printable ASCII (seed 0xA5A5_0001, 512 draws)
    let mut rng = Rng::new(0xA5A5_0001);
    let ops: Vec<Op> = (0..512)
        .map(|_| {
            let n = rng.range(2, 255) as usize;
            Op::Line(gen(&mut rng, n, |r| r.printable_ascii()))
        })
        .collect();
    for (i, b) in batched(ops, 32).iter().enumerate() {
        cs.diff(&format!("cfg_04_print_line_short_random_ascii[{i}]"), b);
    }

    // row 5 — length 1..=255 over the full non-NUL byte domain 0x01..=0xFF,
    //         i.e. invalid UTF-8 included (seed 0xDEAD_BEEF, 512 draws)
    let mut rng = Rng::new(0xDEAD_BEEF);
    let ops: Vec<Op> = (0..512)
        .map(|_| {
            let n = rng.range(1, 255) as usize;
            Op::Line(gen(&mut rng, n, |r| r.byte_excluding_nul()))
        })
        .collect();
    for (i, b) in batched(ops, 32).iter().enumerate() {
        cs.diff(&format!("cfg_05_print_line_random_bytes[{i}]"), b);
    }

    // row 6 — payloads made of printf format specifiers (seed 0x0F0F_1111, 256 draws)
    const SPECS: [&[u8]; 12] = [
        b"%s", b"%d", b"%n", b"%p", b"%%", b"%1000000d", b"%.*s", b"%hhn", b"%99999999s",
        b"%c", b"%x", b"%lld",
    ];
    let mut rng = Rng::new(0x0F0F_1111);
    let ops: Vec<Op> = (0..256)
        .map(|_| {
            let n = rng.range(1, 8);
            let mut v = Vec::new();
            for _ in 0..n {
                let k = rng.idx(SPECS.len());
                v.extend_from_slice(SPECS[k]);
                if rng.next_u32() % 2 == 0 {
                    v.push(rng.printable_ascii());
                }
            }
            Op::Line(v)
        })
        .collect();
    for (i, b) in batched(ops, 32).iter().enumerate() {
        cs.diff(&format!("cfg_06_print_line_format_specifiers[{i}]"), b);
    }

    // row 7 — embedded whitespace / control characters (seed 0xC0FF_EE01, 256 draws)
    const WS: [u8; 6] = [b'\n', b'\r', b'\t', 0x0b, 0x0c, b' '];
    let mut rng = Rng::new(0xC0FF_EE01);
    let ops: Vec<Op> = (0..256)
        .map(|_| {
            let n = rng.range(1, 64) as usize;
            Op::Line(gen(&mut rng, n, |r| {
                if r.next_u32() % 3 == 0 {
                    WS[r.idx(WS.len())]
                } else {
                    r.printable_ascii()
                }
            }))
        })
        .collect();
    for (i, b) in batched(ops, 32).iter().enumerate() {
        cs.diff(&format!("cfg_07_print_line_whitespace[{i}]"), b);
    }

    // row 8 — embedded NUL at a randomized position (seed 0x5EED_0008, 256 draws)
    let mut rng = Rng::new(0x5EED_0008);
    let ops: Vec<Op> = (0..256)
        .map(|_| {
            let n = rng.range(1, 64) as usize;
            let mut v = gen(&mut rng, n, |r| r.byte_excluding_nul());
            let at = rng.range(0, n as u64 - 1) as usize;
            v[at] = 0;
            Op::Line(v)
        })
        .collect();
    for (i, b) in batched(ops, 32).iter().enumerate() {
        cs.diff(&format!("cfg_08_print_line_embedded_nul[{i}]"), b);
    }

    // row 9 — lengths straddling stdio buffer boundaries (seed 0xB00B_0009)
    let mut rng = Rng::new(0xB00B_0009);
    for len in [
        1023usize, 1024, 1025, 4095, 4096, 4097, 8191, 8192, 8193, 65535, 65536, 65537,
    ] {
        let v = gen(&mut rng, len, |r| r.byte_excluding_nul());
        cs.diff(
            &format!("cfg_09_print_line_buffer_boundaries[len={len}]"),
            &[Op::Line(v)],
        );
    }

    // row 10 — oversized: 1 MiB payload
    let mut rng = Rng::new(0x1010_000A);
    let v = gen(&mut rng, 1 << 20, |r| r.printable_ascii());
    let out = cs.diff("cfg_10_print_line_oversized", &[Op::Line(v.clone())]);
    cs.expect_val("cfg_10 C writes 1 MiB + newline", out.len(), v.len() + 1);
    cs.expect_c("cfg_10 payload verbatim", &out[..v.len()], &v);

    // row 11 — printIntLine(0)
    let out = cs.diff("cfg_11_print_int_line_zero", &[Op::Int(0)]);
    cs.expect_c("cfg_11 C prints 0", &out, b"0\n");

    // row 12 — smallest magnitudes, both signs
    let out = cs.diff(
        "cfg_12_print_int_line_small",
        &[Op::Int(1), Op::Int(-1), Op::Int(2), Op::Int(-2)],
    );
    cs.expect_c("cfg_12 C output", &out, b"1\n-1\n2\n-2\n");

    // row 13 — every decimal-width boundary: +-10^k and +-(10^k - 1), k = 1..9
    let mut ops = Vec::new();
    let mut p: i64 = 10;
    for _ in 1..=9 {
        ops.push(Op::Int(p as i32));
        ops.push(Op::Int(-(p as i32)));
        ops.push(Op::Int((p - 1) as i32));
        ops.push(Op::Int(-((p - 1) as i32)));
        p *= 10;
    }
    cs.diff("cfg_13_print_int_line_decimal_boundaries", &ops);

    // row 14 — extremes
    let out = cs.diff(
        "cfg_14_print_int_line_extremes",
        &[
            Op::Int(i32::MIN),
            Op::Int(i32::MAX),
            Op::Int(i32::MIN + 1),
            Op::Int(i32::MAX - 1),
        ],
    );
    cs.expect_c(
        "cfg_14 C output",
        &out,
        b"-2147483648\n2147483647\n-2147483647\n2147483646\n",
    );

    // row 15 — randomized full-range i32 (seed 0x1357_9BDF, 4096 draws)
    let mut rng = Rng::new(0x1357_9BDF);
    let ops: Vec<Op> = (0..4096).map(|_| Op::Int(rng.next_u32() as i32)).collect();
    for (i, b) in batched(ops, 64).iter().enumerate() {
        cs.diff(&format!("cfg_15_print_int_line_random_full_range[{i}]"), b);
    }

    // row 16 — randomized sign-restricted sub-ranges + all +-2^k (seed 0x2468_ACE0)
    let mut rng = Rng::new(0x2468_ACE0);
    let mut ops: Vec<Op> = Vec::new();
    for k in 0..31 {
        ops.push(Op::Int(1i32 << k));
        ops.push(Op::Int(-(1i32 << k)));
    }
    ops.push(Op::Int(i32::MIN)); // -2^31
    for _ in 0..512 {
        ops.push(Op::Int((rng.next_u32() >> 1) as i32)); // positive only
        ops.push(Op::Int(-((rng.next_u32() >> 1) as i32))); // negative only
    }
    for (i, b) in batched(ops, 64).iter().enumerate() {
        cs.diff(&format!("cfg_16_print_int_line_random_signed_ranges[{i}]"), b);
    }

    // row 17 — good()
    let out = cs.diff("cfg_17_good_single", &[Op::Good]);
    cs.expect_c("cfg_17 C output", &out, b"0\n2\n");

    // row 18 — bad(): the sum is computed and discarded, so 0 twice
    let out = cs.diff("cfg_18_bad_single", &[Op::Bad]);
    cs.expect_c("cfg_18 C output", &out, b"0\n0\n");

    // row 19 — driver(): the whole composed pipeline
    let out = cs.diff("cfg_19_driver_single", &[Op::Driver]);
    cs.expect_c(
        "cfg_19 C output",
        &out,
        b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n",
    );

    // row 20 — repeated invocation: no hidden accumulated state
    for (name, op) in [("good", Op::Good), ("bad", Op::Bad), ("driver", Op::Driver)] {
        let ops: Vec<Op> = (0..64).map(|_| op.clone()).collect();
        let out = cs.diff(&format!("cfg_20_no_arg_fns_repeated[{name}]"), &ops);
        let one = capture("c", || unsafe { run_ops(cs.c, &[op.clone()]) });
        let expect: Vec<u8> = one.repeat(64);
        cs.expect_c(&format!("cfg_20 {name} x64 == 64x once"), &out, &expect);
    }

    // row 21 — randomized interleavings of all 5 entry points (seed 0xFEED_FACE)
    let mut rng = Rng::new(0xFEED_FACE);
    for i in 0..64 {
        let n = rng.range(1, 32);
        let ops: Vec<Op> = (0..n)
            .map(|_| match rng.next_u32() % 6 {
                0 => Op::Driver,
                1 => Op::Good,
                2 => Op::Bad,
                3 => Op::LineNull,
                4 => {
                    let l = rng.range(0, 40) as usize;
                    Op::Line(gen(&mut rng, l, |r| r.byte_excluding_nul()))
                }
                _ => Op::Int(rng.next_u32() as i32),
            })
            .collect();
        cs.diff(&format!("cfg_21_random_interleaved_sequences[{i}]"), &ops);
    }

    // row 22 — printLine x printIntLine cross product, no flush in between
    //          (seed 0x0BAD_C0DE, 512 draws)
    let mut rng = Rng::new(0x0BAD_C0DE);
    let ops: Vec<Op> = (0..512)
        .flat_map(|_| {
            let l = rng.range(0, 64) as usize;
            let s = gen(&mut rng, l, |r| r.byte_excluding_nul());
            let v = rng.next_u32() as i32;
            if rng.next_u32() % 2 == 0 {
                vec![Op::Line(s), Op::Int(v)]
            } else {
                vec![Op::Int(v), Op::Line(s)]
            }
        })
        .collect();
    for (i, b) in batched(ops, 64).iter().enumerate() {
        cs.diff(&format!("cfg_22_low_level_cross_product[{i}]"), b);
    }
}

// ---------------------------------------------------------------------------
// PHASE C — ERRORS.md rows 1..13 (invalid / boundary inputs)
// ---------------------------------------------------------------------------

fn phase_c(cs: &mut Case) {
    // row 1 — the library's one and only explicit rejection: line == NULL
    let out = cs.diff("err_01_print_line_null", &[Op::LineNull]);
    cs.expect_c("err_01 NULL => zero bytes", &out, b"");
    // and NULL mixed into a stream must not disturb neighbouring output
    let out = cs.diff(
        "err_01_print_line_null_interleaved",
        &[
            Op::LineNull,
            Op::Line(b"x".to_vec()),
            Op::LineNull,
            Op::Int(7),
            Op::LineNull,
        ],
    );
    cs.expect_c("err_01 interleaved NULLs", &out, b"x\n7\n");

    // row 2 — one step past NULL: a valid pointer to a zero-length string
    let out = cs.diff("err_02_print_line_empty", &[Op::Line(vec![])]);
    cs.expect_c("err_02 empty => \\n", &out, b"\n");

    // row 3 — first byte is NUL, more bytes follow
    let out = cs.diff(
        "err_03_print_line_embedded_nul_first",
        &[Op::Line(vec![0, b'h', b'i', 0xff])],
    );
    cs.expect_c("err_03 truncates at first NUL", &out, b"\n");

    // row 4 — NUL in the middle
    let out = cs.diff(
        "err_04_print_line_embedded_nul_mid",
        &[Op::Line(b"abc\0def".to_vec())],
    );
    cs.expect_c("err_04 truncates mid-string", &out, b"abc\n");

    // row 5 — format-string hazard: specifiers are data, never a format
    let payloads: [&[u8]; 8] = [
        b"%s",
        b"%d %d %d %d %d %d %d %d",
        b"%n",
        b"%p",
        b"%%",
        b"%1000000d",
        b"AAAA%08x.%08x.%08x.%08x.%n",
        b"%s%s%s%s%s%s%s%s%s%s",
    ];
    for (i, p) in payloads.iter().enumerate() {
        let out = cs.diff(
            &format!("err_05_print_line_format_specifiers[{i}]"),
            &[Op::Line(p.to_vec())],
        );
        let mut expect = p.to_vec();
        expect.push(b'\n');
        cs.expect_c("err_05 printed literally", &out, &expect);
    }

    // row 6 — oversized: 1 MiB, must not truncate
    let big = vec![b'Z'; 1 << 20];
    let out = cs.diff("err_06_print_line_oversized", &[Op::Line(big.clone())]);
    cs.expect_val("err_06 no truncation", out.len(), big.len() + 1);

    // row 7 — non-ASCII / invalid UTF-8 byte sequences
    let bad_utf8: [&[u8]; 6] = [
        &[0x80],
        &[0xff, 0xfe],
        &[0xc3],                   // truncated 2-byte seq
        &[0xed, 0xa0, 0x80],       // UTF-8-encoded surrogate D800
        &[0xf4, 0x90, 0x80, 0x80], // > U+10FFFF
        &[0xc0, 0x80],             // overlong NUL
    ];
    for (i, p) in bad_utf8.iter().enumerate() {
        let out = cs.diff(
            &format!("err_07_print_line_invalid_utf8[{i}]"),
            &[Op::Line(p.to_vec())],
        );
        let mut expect = p.to_vec();
        expect.push(b'\n');
        cs.expect_c("err_07 bytes verbatim", &out, &expect);
    }

    // row 8 — exhaustive single-byte sweep 0x01..=0xFF
    let ops: Vec<Op> = (1u8..=255).map(|b| Op::Line(vec![b])).collect();
    for (i, b) in batched(ops, 64).iter().enumerate() {
        cs.diff(&format!("err_08_print_line_all_byte_values[{i}]"), b);
    }

    // row 9 — INT_MIN
    let out = cs.diff("err_09_print_int_line_int_min", &[Op::Int(i32::MIN)]);
    cs.expect_c("err_09", &out, b"-2147483648\n");

    // row 10 — INT_MAX
    let out = cs.diff("err_10_print_int_line_int_max", &[Op::Int(i32::MAX)]);
    cs.expect_c("err_10", &out, b"2147483647\n");

    // row 11 — one step past each boundary, i.e. the wrapped bit patterns, plus
    //          unsigned values reinterpreted as int
    let out = cs.diff(
        "err_11_print_int_line_wraparound",
        &[
            Op::Int(i32::MAX.wrapping_add(1)),  // INT_MAX + 1  => INT_MIN
            Op::Int(i32::MIN.wrapping_sub(1)),  // INT_MIN - 1  => INT_MAX
            Op::Int(0u32 as i32),
            Op::Int(0xFFFF_FFFFu32 as i32),
            Op::Int(0x8000_0000u32 as i32),
            Op::Int(0x7FFF_FFFFu32 as i32),
        ],
    );
    cs.expect_c(
        "err_11",
        &out,
        b"-2147483648\n2147483647\n0\n-1\n-2147483648\n2147483647\n",
    );

    // row 12 — 64-bit-wide argument handed to an `int` prototype (dirty upper bits)
    for v in [
        0x0000_0001_0000_0000i64,
        0x7FFF_FFFF_0000_0002u64 as i64,
        -1i64,
        0xDEAD_BEEF_FFFF_FFFFu64 as i64,
        0x0000_0000_8000_0000i64,
    ] {
        cs.diff(
            &format!("err_12_print_int_line_dirty_upper_bits[{v:#x}]"),
            &[Op::Int64(v)],
        );
    }

    // row 13 — the no-argument functions have no invalid input; the only misuse is
    //          repeated / interleaved invocation, which must stay stateless
    for (name, op) in [("good", Op::Good), ("bad", Op::Bad), ("driver", Op::Driver)] {
        let first = cs.diff(
            &format!("err_13_no_arg_fns_have_no_error_path[{name}]#1"),
            std::slice::from_ref(&op),
        );
        for i in 2..=5 {
            let again = cs.diff(
                &format!("err_13_no_arg_fns_have_no_error_path[{name}]#{i}"),
                std::slice::from_ref(&op),
            );
            cs.expect_c(&format!("err_13 {name} invocation {i} identical"), &again, &first);
        }
    }
}

// ---------------------------------------------------------------------------
// PHASE D — symbol parity
// ---------------------------------------------------------------------------

fn exported_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.as_slice() {
                [_addr, kind, name] if *kind == "T" || *kind == "t" => Some((*name).to_string()),
                _ => None,
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

fn phase_d(cs: &mut Case) {
    let c_syms = exported_symbols(&cs.c.path);
    let r_syms = exported_symbols(&cs.r.path);

    // The C library's 5 functions must all be there.
    cs.expect_val(
        "phase_d C exports the expected 5 functions",
        c_syms.clone(),
        vec![
            "bad".to_string(),
            "driver".to_string(),
            "good".to_string(),
            "printIntLine".to_string(),
            "printLine".to_string(),
        ],
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    cs.expect_val(
        "phase_d symbols missing from the Rust .so",
        format!("{missing:?}"),
        "[]".to_string(),
    );

    // sanity: each symbol must actually be callable through the Rust .so, which the
    // rest of the suite already exercises; here we just re-resolve them by name.
    for s in &c_syms {
        let mut name = s.clone().into_bytes();
        name.push(0);
        unsafe {
            let got: Result<libloading::Symbol<'_, *const c_void>, _> = cs.r._lib.get(&name);
            cs.expect_val(
                &format!("phase_d dlsym({s}) on Rust .so"),
                got.is_ok(),
                true,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// runner
// ---------------------------------------------------------------------------

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so = root.join("c_src/build/libdriver.so");
    // The Rust library is loaded as a *shared object*, never linked: this exercises
    // the `#[no_mangle]` exports exactly as an external C consumer would.
    let r_so = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(if cfg!(debug_assertions) { "debug" } else { "release" })
        .join("libdriver.so");
    let r_so = if r_so.exists() {
        r_so
    } else {
        // fall back to whichever profile dir has it (e.g. when run via `cargo test --release`)
        let alt = root.join("target/debug/libdriver.so");
        if alt.exists() { alt } else { r_so }
    };

    let c = Api::load("C", &c_so);
    let r = Api::load("Rust", &r_so);
    eprintln!("{:<4} .so: {}", c.name, c.path.display());
    eprintln!("{:<4} .so: {}", r.name, r.path.display());
    eprintln!("features: {}", FEATURES);

    let mut cs = Case { c: &c, r: &r, failures: Vec::new(), checks: 0 };

    eprintln!("== Phase B: CONFIGS.md rows (valid paths) ==");
    phase_b(&mut cs);
    let after_b = cs.failures.len();
    eprintln!("   {} checks, {} failures", cs.checks, after_b);

    eprintln!("== Phase C: ERRORS.md rows (error paths) ==");
    phase_c(&mut cs);
    eprintln!("   {} failures in phase C", cs.failures.len() - after_b);

    eprintln!("== Phase D: symbol parity ==");
    phase_d(&mut cs);

    eprintln!("\n{} total checks, {} failures", cs.checks, cs.failures.len());
    if !cs.failures.is_empty() {
        for f in &cs.failures {
            eprintln!("{f}");
        }
        panic!("{} differential check(s) failed", cs.failures.len());
    }
    eprintln!("ALL DIFFERENTIAL CHECKS PASSED");
}

/// Records which cargo features the binary was built with, so the report shows it.
const FEATURES: &str = "<none: the crate declares no [features]; single configuration>";
