// Differential test harness: loads BOTH the C `.so` and the Rust `.so` through
// `libloading` and compares their observable behaviour byte-for-byte.
//
// No Rust function is ever called directly. Every call goes through a symbol
// looked up in a dynamically loaded shared object, exactly as an external C
// consumer would do it, so the `#[no_mangle]` / `extern "C"` export wrappers
// are under test too.
//
// WHY EVERY CALL RUNS IN A SUBPROCESS
// -----------------------------------
// Both `driver` and `printLine` return `void` and have no error return: the
// only observable effects of this library are the bytes it writes to stdout
// with C `printf`, and — for the memory-unsafe negative-`data` path — the
// signal that kills the process. Observing stdout means capturing file
// descriptor 1, which is process-global; an in-process `dup2` capture races
// with Rust's own libtest harness, which writes its progress lines to fd 1
// from another thread and corrupts the capture. And the negative-`data` rows
// abort the process outright, so they cannot run in-process at all.
//
// So the harness re-execs this very test binary as a child, points the child's
// fd 1 at a private file, and has the child `dlopen` ONE of the two libraries
// and replay a script of operations. The parent then compares the two
// children's stdout bytes and their termination status. One spawn per
// (row, implementation), so a whole row of hundreds of randomized inputs costs
// two spawns.

#![allow(non_snake_case)]

use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use libloading::{Library, Symbol};

use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open output streams, including the C
    /// `stdout` FILE* that the loaded library writes through. fd 1 is a regular
    /// file in the child, so the stream is fully buffered and this is mandatory.
    fn fflush(stream: *mut c_void) -> c_int;
    fn _exit(status: c_int) -> !;
}

// ---------------------------------------------------------------------------
// Library location
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared library built by CMake.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("../c_src/build/libdriver.so")
}

/// Path to the Rust `cdylib`. Overridable with `RUST_SO` so the same suite can
/// be run against both the debug and the release artifact.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let debug = manifest_dir().join("target/debug/libdriver.so");
    if debug.exists() {
        return debug;
    }
    manifest_dir().join("target/release/libdriver.so")
}

/// Which of the two implementations a run should go to.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Impl {
    C,
    Rust,
}

impl Impl {
    fn path(self) -> PathBuf {
        match self {
            Impl::C => c_so_path(),
            Impl::Rust => rust_so_path(),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Impl::C => "C",
            Impl::Rust => "Rust",
        }
    }
}

type DriverFn = unsafe extern "C" fn(c_int);
type PrintLineFn = unsafe extern "C" fn(*const c_char);

// ---------------------------------------------------------------------------
// Operation scripts
// ---------------------------------------------------------------------------

/// One call into the library under test.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Op {
    /// `driver(data)`
    Driver(c_int),
    /// `printLine(ptr)`; the payload includes its terminating NUL, so payloads
    /// with embedded NULs are expressible.
    PrintLine(Vec<u8>),
    /// `printLine(NULL)`
    PrintLineNull,
}

impl Op {
    fn describe(&self) -> String {
        match self {
            Op::Driver(v) => format!("driver({v})"),
            Op::PrintLineNull => "printLine(NULL)".to_string(),
            Op::PrintLine(p) => {
                let body = &p[..p.len().saturating_sub(1)];
                format!("printLine({} payload bytes: {})", body.len(), hex(body))
            }
        }
    }
    fn encode(&self) -> String {
        match self {
            Op::Driver(v) => format!("D {v}"),
            Op::PrintLineNull => "N".to_string(),
            Op::PrintLine(p) => {
                let h: String = p.iter().map(|b| format!("{b:02x}")).collect();
                format!("P {h}")
            }
        }
    }
    fn decode(line: &str) -> Op {
        let (tag, rest) = match line.split_once(' ') {
            Some((t, r)) => (t, r),
            None => (line, ""),
        };
        match tag {
            "D" => Op::Driver(rest.trim().parse().expect("decode driver arg")),
            "N" => Op::PrintLineNull,
            "P" => {
                let bytes: Vec<u8> = rest
                    .trim()
                    .as_bytes()
                    .chunks(2)
                    .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
                    .collect();
                Op::PrintLine(bytes)
            }
            other => panic!("unknown op tag {other:?}"),
        }
    }
}

fn hex(b: &[u8]) -> String {
    if b.len() > 48 {
        let head: String = b[..24].iter().map(|x| format!("{x:02x}")).collect();
        let tail: String = b[b.len() - 24..].iter().map(|x| format!("{x:02x}")).collect();
        format!("{head}..[{} bytes]..{tail}", b.len())
    } else {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }
}

/// Compact description of the first difference between two byte strings, so
/// failures stay readable even for 64 KiB payloads.
fn first_diff(a: &[u8], b: &[u8]) -> String {
    if a == b {
        return "identical".to_string();
    }
    let n = a.len().min(b.len());
    match (0..n).find(|&i| a[i] != b[i]) {
        Some(i) => format!(
            "len C={} Rust={}; first differing byte at index {i}: C={:#04x} Rust={:#04x}",
            a.len(),
            b.len(),
            a[i],
            b[i]
        ),
        None => format!(
            "common prefix of {n} bytes equal, lengths differ: C={} Rust={}",
            a.len(),
            b.len()
        ),
    }
}

// ---------------------------------------------------------------------------
// Child process
// ---------------------------------------------------------------------------

const ENV_LIB: &str = "DRIVER_DIFFTEST_CHILD_LIB";
const ENV_SCRIPT: &str = "DRIVER_DIFFTEST_CHILD_SCRIPT";
const ENV_OUT: &str = "DRIVER_DIFFTEST_CHILD_OUT";

/// The designated child entry point. When the control env vars are absent this
/// is a no-op that simply passes, so a normal run pays nothing for it.
#[test]
fn zz_child_worker() {
    let (lib, script, out) = match (
        std::env::var(ENV_LIB),
        std::env::var(ENV_SCRIPT),
        std::env::var(ENV_OUT),
    ) {
        (Ok(a), Ok(b), Ok(c)) => (a, b, c),
        _ => return, // normal test run: nothing to do
    };

    let text = std::fs::read_to_string(&script).expect("child read script");
    let ops: Vec<Op> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(Op::decode)
        .collect();

    let file = std::fs::File::create(&out).expect("child create out file");

    // SAFETY: fd manipulation followed by calls into the library under test
    // through symbols resolved by `dlsym`.
    unsafe {
        let _ = std::io::stdout().flush();
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "child dup2 onto fd 1");

        let l = Library::new(&lib).unwrap_or_else(|e| panic!("child dlopen {lib}: {e}"));
        let driver: Symbol<DriverFn> = l.get(b"driver\0").expect("child dlsym driver");
        let print_line: Symbol<PrintLineFn> =
            l.get(b"printLine\0").expect("child dlsym printLine");

        for op in &ops {
            match op {
                Op::Driver(v) => driver(*v),
                Op::PrintLineNull => print_line(std::ptr::null()),
                Op::PrintLine(p) => print_line(p.as_ptr() as *const c_char),
            }
        }

        fflush(std::ptr::null_mut());
        // `_exit` keeps any atexit / harness output out of the capture file.
        _exit(0);
    }
}

// ---------------------------------------------------------------------------
// Parent side: run a script against one implementation
// ---------------------------------------------------------------------------

fn unique_tmp_path(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver_difftest_{}_{}_{}.tmp",
        std::process::id(),
        tag,
        n
    ))
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    /// `Some(sig)` if the run was killed by a signal.
    signal: Option<i32>,
    /// Exit code if it terminated normally.
    code: Option<i32>,
    stdout: Vec<u8>,
}

impl Outcome {
    fn summary(&self) -> String {
        format!(
            "signal={:?} code={:?} stdout={} bytes: {}",
            self.signal,
            self.code,
            self.stdout.len(),
            hex(&self.stdout)
        )
    }
}

fn run_ops(which: Impl, ops: &[Op]) -> Outcome {
    let lib = which.path();
    assert!(
        lib.exists(),
        "{} shared library not found at {lib:?}. Build the C side with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         Build the Rust side with:\n  cd translation && cargo build",
        which.name()
    );

    let script_path = unique_tmp_path("script");
    let out_path = unique_tmp_path("out");
    let mut script = String::new();
    for op in ops {
        script.push_str(&op.encode());
        script.push('\n');
    }
    std::fs::write(&script_path, script).expect("write script");
    // Create the file up front so a child that dies before writing still yields
    // a readable, empty capture.
    std::fs::write(&out_path, b"").expect("create out file");

    let exe = std::env::current_exe().expect("current_exe");
    let status = std::process::Command::new(exe)
        .args(["--exact", "zz_child_worker", "--test-threads=1"])
        .env(ENV_LIB, &lib)
        .env(ENV_SCRIPT, &script_path)
        .env(ENV_OUT, &out_path)
        // Any libtest chatter in the child goes to /dev/null; only the library's
        // own printf output reaches `out_path`, via the child's dup2.
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("spawn child");

    let stdout = std::fs::read(&out_path).unwrap_or_default();
    let _ = std::fs::remove_file(&script_path);
    let _ = std::fs::remove_file(&out_path);

    Outcome {
        signal: status.signal(),
        code: status.code(),
        stdout,
    }
}

/// Run `ops` against the C `.so` and the Rust `.so` and require identical
/// stdout bytes AND identical termination status. On divergence, narrow down to
/// the single operation responsible before reporting.
fn diff_ops(ops: &[Op], label: &str) {
    let c = run_ops(Impl::C, ops);
    let r = run_ops(Impl::Rust, ops);
    if c == r {
        return;
    }

    // Narrow: find the first single op that diverges on its own.
    if ops.len() > 1 {
        for (i, op) in ops.iter().enumerate() {
            let one = [op.clone()];
            let cc = run_ops(Impl::C, &one);
            let rr = run_ops(Impl::Rust, &one);
            if cc != rr {
                panic!(
                    "{label}: divergence isolated to op #{i} = {}\n  C   : {}\n  Rust: {}\n  {}",
                    op.describe(),
                    cc.summary(),
                    rr.summary(),
                    first_diff(&cc.stdout, &rr.stdout)
                );
            }
        }
        panic!(
            "{label}: every op agrees in isolation, but the SEQUENCE diverges \
             (an ordering / cross-call state difference)\n  C   : {}\n  Rust: {}\n  {}",
            c.summary(),
            r.summary(),
            first_diff(&c.stdout, &r.stdout)
        );
    }

    panic!(
        "{label}: divergence on {}\n  C   : {}\n  Rust: {}\n  {}",
        ops[0].describe(),
        c.summary(),
        r.summary(),
        first_diff(&c.stdout, &r.stdout)
    );
}

fn diff_driver(data: c_int) {
    diff_ops(&[Op::Driver(data)], &format!("driver({data})"));
}

fn diff_print_line(payload: &[u8], label: &str) {
    assert_eq!(
        payload.last(),
        Some(&0u8),
        "test payload must be NUL-terminated"
    );
    diff_ops(&[Op::PrintLine(payload.to_vec())], label);
}

/// The C ground truth for a script, used where a row asserts the exact expected
/// bytes and not merely C/Rust equality (so a mutual-but-wrong result fails).
fn c_ground_truth(ops: &[Op]) -> Outcome {
    run_ops(Impl::C, ops)
}

// ---------------------------------------------------------------------------
// Fixed-seed PRNG (SplitMix64) for property-style inputs.
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
    /// Uniform-ish in `[lo, hi]` inclusive.
    fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as i64
    }
    fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        self.range_i64(lo as i64, hi as i64) as usize
    }
    fn byte(&mut self, lo: u8, hi: u8) -> u8 {
        self.range_i64(lo as i64, hi as i64) as u8
    }
}

/// The one seed used by every randomized row, so runs are reproducible.
const SEED: u64 = 0x243F_6A88_85A3_08D3;

/// Build a NUL-terminated random payload of `len` body bytes drawn from
/// `[lo, hi]` (kept NUL-free when `lo >= 1`).
fn rand_payload(rng: &mut Rng, len: usize, lo: u8, hi: u8) -> Vec<u8> {
    let mut v: Vec<u8> = (0..len).map(|_| rng.byte(lo, hi)).collect();
    v.push(0);
    v
}

// ===========================================================================
// PHASE B — valid-path differential tests, one per CONFIGS.md row.
// ===========================================================================

/// CONFIGS.md row 1 — `printLine` with the empty string.
#[test]
fn cfg_01_printline_empty() {
    diff_print_line(b"\0", "cfg_01 empty string");
    // Exact C shape: `printf("%s\n", "")` emits a lone newline.
    assert_eq!(
        c_ground_truth(&[Op::PrintLine(b"\0".to_vec())]).stdout,
        b"\n",
        "C ground truth for printLine(\"\") is a lone newline"
    );
}

/// CONFIGS.md row 2 — `printLine` with every single-byte payload `0x01..=0xFF`
/// (exhaustive; the space is small enough to enumerate).
#[test]
fn cfg_02_printline_single_byte_all() {
    let ops: Vec<Op> = (1u8..=255).map(|b| Op::PrintLine(vec![b, 0])).collect();
    diff_ops(&ops, "cfg_02 every single-byte payload 0x01..=0xff");
}

/// CONFIGS.md row 3 — `printLine` with many bytes: randomized length and
/// randomized NUL-free payload, so the whole payload reaches stdout.
#[test]
fn cfg_03_printline_random_many() {
    let mut rng = Rng::new(SEED ^ 0x03);
    let ops: Vec<Op> = (0..400)
        .map(|_| {
            let len = rng.range_usize(2, 512);
            Op::PrintLine(rand_payload(&mut rng, len, 1, 255))
        })
        .collect();
    diff_ops(&ops, "cfg_03 400 randomized multi-byte payloads");
}

/// CONFIGS.md row 4 — `printf` format directives appearing in the *argument*.
/// The C format string is the fixed `"%s\n"`, so these must be emitted
/// literally rather than interpreted.
#[test]
fn cfg_04_printline_format_directives() {
    let cases: &[&[u8]] = &[
        b"%s\0",
        b"%d\0",
        b"%n\0",
        b"%%\0",
        b"%p\0",
        b"%s%s%s%s\0",
        b"100%% done\0",
        b"%1$s %2$d\0",
        b"%.999999f\0",
        b"a%zub%hhnc\0",
        b"%\0",
    ];
    let ops: Vec<Op> = cases.iter().map(|c| Op::PrintLine(c.to_vec())).collect();
    diff_ops(&ops, "cfg_04 format directives as data");
    // Exact C shape: the directives must survive verbatim.
    assert_eq!(
        c_ground_truth(&[Op::PrintLine(b"%s %d %n\0".to_vec())]).stdout,
        b"%s %d %n\n",
        "the fixed \"%s\\n\" format must emit argument directives literally"
    );
}

/// CONFIGS.md row 5 — payload with an embedded NUL: `%s` must stop there and
/// drop the tail.
#[test]
fn cfg_05_printline_embedded_nul() {
    let fixed: &[&[u8]] = &[
        b"head\0tail\0",
        b"\0dropped entirely\0",
        b"a\0\0\0b\0",
    ];
    let mut ops: Vec<Op> = fixed.iter().map(|c| Op::PrintLine(c.to_vec())).collect();

    let mut rng = Rng::new(SEED ^ 0x05);
    for _ in 0..150 {
        let head = rng.range_usize(0, 32);
        let tail = rng.range_usize(1, 32);
        let mut buf: Vec<u8> = (0..head).map(|_| rng.byte(1, 255)).collect();
        buf.push(0);
        buf.extend((0..tail).map(|_| rng.byte(1, 255)));
        buf.push(0);
        ops.push(Op::PrintLine(buf));
    }
    diff_ops(&ops, "cfg_05 embedded NUL payloads");
    // Exact C shape: everything past the first NUL is dropped.
    assert_eq!(
        c_ground_truth(&[Op::PrintLine(b"head\0tail\0".to_vec())]).stdout,
        b"head\n",
        "%s must stop at the first NUL"
    );
}

/// CONFIGS.md row 6 — high / non-ASCII bytes and embedded control characters.
#[test]
fn cfg_06_printline_high_bytes_and_newlines() {
    let fixed: &[&[u8]] = &[
        b"\x80\xff\xfe\xc3\xa9\0",
        b"line1\nline2\nline3\0",
        b"tab\there\rcarriage\0",
        b"\x1b[31mansi\x1b[0m\0",
    ];
    let mut ops: Vec<Op> = fixed.iter().map(|c| Op::PrintLine(c.to_vec())).collect();

    let mut rng = Rng::new(SEED ^ 0x06);
    for _ in 0..300 {
        let len = rng.range_usize(1, 256);
        let mut buf: Vec<u8> = (0..len)
            .map(|_| match rng.range_i64(0, 2) {
                0 => rng.byte(0x80, 0xff),
                1 => [b'\n', b'\t', b'\r', 0x0b, 0x0c][rng.range_usize(0, 4)],
                _ => rng.byte(0x20, 0x7e),
            })
            .collect();
        buf.push(0);
        ops.push(Op::PrintLine(buf));
    }
    diff_ops(&ops, "cfg_06 high bytes and control characters");
}

/// CONFIGS.md row 7 — long payloads that cross the libc stdout buffer size.
#[test]
fn cfg_07_printline_long() {
    for len in [1024usize, 4095, 4096, 4097, 65536] {
        let mut rng = Rng::new(SEED ^ 0x07 ^ len as u64);
        let p = rand_payload(&mut rng, len, 1, 255);
        diff_print_line(&p, &format!("cfg_07 long payload len={len}"));
        // Exact C shape: the payload plus exactly one trailing newline.
        let out = c_ground_truth(&[Op::PrintLine(p.clone())]).stdout;
        assert_eq!(out.len(), len + 1, "C output for a {len}-byte payload");
        assert_eq!(&out[..len], &p[..len]);
        assert_eq!(out[len], b'\n');
    }
}

/// CONFIGS.md row 8 — many `printLine` calls in ONE process, so the accumulated
/// stdio state is compared too.
#[test]
fn cfg_08_printline_repeated_calls() {
    let mut rng = Rng::new(SEED ^ 0x08);
    for round in 0..8 {
        let ops: Vec<Op> = (0..64)
            .map(|_| {
                let len = rng.range_usize(0, 64);
                Op::PrintLine(rand_payload(&mut rng, len, 1, 255))
            })
            .collect();
        diff_ops(&ops, &format!("cfg_08 64 printLine calls, round {round}"));
    }
}

/// CONFIGS.md row 9 — `driver(0)`: guard taken, `strncpy` length 0.
#[test]
fn cfg_09_driver_zero() {
    diff_driver(0);
}

/// CONFIGS.md row 10 — `driver(1)`: minimal non-empty copy.
#[test]
fn cfg_10_driver_one() {
    diff_driver(1);
    assert_eq!(
        c_ground_truth(&[Op::Driver(1)]).stdout,
        b"A\n",
        "C ground truth for driver(1)"
    );
}

/// CONFIGS.md row 11 — randomized interior of the accepted band. `data` drives
/// both the copy length and the `dest[data]` store index, so behaviour is
/// value-dependent and a single hand-picked value would be inadequate.
#[test]
fn cfg_11_driver_interior_randomized() {
    let mut rng = Rng::new(SEED ^ 0x11);
    let ops: Vec<Op> = (0..500)
        .map(|_| Op::Driver(rng.range_i64(2, 98) as c_int))
        .collect();
    diff_ops(&ops, "cfg_11 500 randomized data in [2,98]");
}

/// CONFIGS.md row 12 — `driver(98)`: last value strictly below
/// `strlen(source) == 99`.
#[test]
fn cfg_12_driver_98() {
    diff_driver(98);
}

/// CONFIGS.md row 13 — `driver(99)` = `strlen(source)`: `strncpy` copies the
/// full source and contributes no NUL of its own, so termination comes only
/// from the explicit `dest[99] = '\0'`.
#[test]
fn cfg_13_driver_99() {
    diff_driver(99);
}

/// CONFIGS.md row 14 — the ENTIRE accepted band `[0, 99]`, exhaustively, with
/// the exact C shape asserted per value so a mutual-but-wrong result fails.
#[test]
fn cfg_14_driver_accepted_band_exhaustive() {
    let ops: Vec<Op> = (0..=99).map(Op::Driver).collect();
    diff_ops(&ops, "cfg_14 exhaustive data in [0,99]");

    let out = c_ground_truth(&ops).stdout;
    let mut want = Vec::new();
    for v in 0..=99usize {
        want.extend(std::iter::repeat(b'A').take(v));
        want.push(b'\n');
    }
    assert_eq!(
        out, want,
        "C output over the whole accepted band must be `data` 'A's then a newline, per value"
    );
}

/// CONFIGS.md row 15 — the rejected band `data >= 100`, randomized across the
/// whole range up to `INT_MAX` plus the fixed endpoints.
#[test]
fn cfg_15_driver_rejected_band_randomized() {
    let mut ops: Vec<Op> = [
        100,
        101,
        102,
        127,
        128,
        255,
        256,
        65535,
        65536,
        i32::MAX - 1,
        i32::MAX,
    ]
    .iter()
    .map(|v| Op::Driver(*v))
    .collect();
    let mut rng = Rng::new(SEED ^ 0x15);
    for _ in 0..500 {
        ops.push(Op::Driver(rng.range_i64(100, i32::MAX as i64) as c_int));
    }
    let n = ops.len();
    diff_ops(&ops, "cfg_15 randomized data in [100, INT_MAX]");

    // Exact C shape: every rejected value prints exactly one newline.
    let out = c_ground_truth(&ops).stdout;
    assert_eq!(
        out,
        vec![b'\n'; n],
        "each rejected `data` must print exactly one newline"
    );
}

/// CONFIGS.md row 16 — the two entry points interleaved in ONE process: the
/// composed `driver` pipeline and the low-level `printLine` primitive must
/// compose identically, and `driver` must leave no state behind between calls.
#[test]
fn cfg_16_driver_printline_interleaved() {
    let mut rng = Rng::new(SEED ^ 0x16);
    for round in 0..10 {
        let ops: Vec<Op> = (0..40)
            .map(|_| {
                if rng.range_i64(0, 1) == 0 {
                    // Draw from the accepted band and the rejected band alike.
                    let v = if rng.range_i64(0, 3) == 0 {
                        rng.range_i64(100, i32::MAX as i64)
                    } else {
                        rng.range_i64(0, 99)
                    };
                    Op::Driver(v as c_int)
                } else {
                    let len = rng.range_usize(0, 120);
                    Op::PrintLine(rand_payload(&mut rng, len, 1, 255))
                }
            })
            .collect();
        diff_ops(&ops, &format!("cfg_16 interleaved round {round}"));
    }
}

// ===========================================================================
// PHASE C — error-path differential tests, one per ERRORS.md row.
// ===========================================================================

/// ERRORS.md row 1 — `printLine(NULL)` fails the `if(line != NULL)` check.
/// Both implementations must reject it identically: zero bytes of output and a
/// normal return.
#[test]
fn err_01_printline_null() {
    diff_ops(&[Op::PrintLineNull], "err_01 printLine(NULL)");

    let c = c_ground_truth(&[Op::PrintLineNull]);
    assert!(
        c.stdout.is_empty(),
        "C ground truth for printLine(NULL) is no output at all, got {}",
        hex(&c.stdout)
    );
    assert_eq!(c.signal, None, "printLine(NULL) must not crash in C");

    // Repeat, and follow with a valid call, to confirm the rejection is not
    // order-dependent and leaves no state a later call would reveal.
    let mut ops: Vec<Op> = std::iter::repeat(Op::PrintLineNull).take(50).collect();
    ops.push(Op::PrintLine(b"after\0".to_vec()));
    ops.push(Op::PrintLineNull);
    ops.push(Op::Driver(5));
    diff_ops(&ops, "err_01 repeated NULL rejections mixed with valid calls");
    assert_eq!(
        c_ground_truth(&ops).stdout,
        b"after\nAAAAA\n",
        "50 NULL rejections must contribute nothing to stdout"
    );
}

/// ERRORS.md row 2 — `data == 100`, the first value that fails `data < 100`.
/// One step past the accepted range: the copy is skipped and the untouched
/// empty `dest` is printed as a lone newline.
#[test]
fn err_02_driver_at_upper_bound() {
    diff_driver(100);
    let c = c_ground_truth(&[Op::Driver(100)]);
    assert_eq!(c.stdout, b"\n", "C ground truth for driver(100)");
    assert_eq!(c.signal, None, "driver(100) must not crash in C");
}

/// ERRORS.md row 3 — `data == 101`, one further past the bound.
#[test]
fn err_03_driver_past_upper_bound() {
    diff_driver(101);
    assert_eq!(c_ground_truth(&[Op::Driver(101)]).stdout, b"\n");
}

/// ERRORS.md row 4 — `data == INT_MAX`, maximal oversized length.
#[test]
fn err_04_driver_int_max() {
    diff_driver(i32::MAX);
    let c = c_ground_truth(&[Op::Driver(i32::MAX)]);
    assert_eq!(c.stdout, b"\n", "C ground truth for driver(INT_MAX)");
    assert_eq!(
        c.signal, None,
        "the guard rejects INT_MAX before any copy, so C must not crash"
    );
}

/// ERRORS.md row 5 — `data == 99`, the last value that passes the guard: the
/// accepted side of the only bound in the library.
#[test]
fn err_05_driver_last_accepted() {
    diff_driver(99);
    let c = c_ground_truth(&[Op::Driver(99)]);
    assert_eq!(c.stdout.len(), 100, "driver(99) is 99 'A' then a newline");
    assert!(c.stdout[..99].iter().all(|&b| b == b'A') && c.stdout[99] == b'\n');
    // The two sides of the bound must stay distinguishable in both impls.
    diff_ops(
        &[Op::Driver(99), Op::Driver(100)],
        "err_05 both sides of the data<100 bound",
    );
}

/// ERRORS.md row 6 — zero length: `strncpy(dest, source, 0)` copies nothing.
#[test]
fn err_06_driver_zero_length() {
    diff_driver(0);
    assert_eq!(
        c_ground_truth(&[Op::Driver(0)]).stdout,
        b"\n",
        "C ground truth for driver(0)"
    );
}

/// ERRORS.md row 7 — the accepted band `[0, 99]` exhaustively, asserting the
/// exact C shape as well as C/Rust equality.
#[test]
fn err_07_driver_accepted_band_exhaustive() {
    for v in 0..=99i32 {
        let ops = [Op::Driver(v)];
        let c = run_ops(Impl::C, &ops);
        let r = run_ops(Impl::Rust, &ops);
        assert_eq!(
            c,
            r,
            "driver({v}) diverges\n  C   : {}\n  Rust: {}\n  {}",
            c.summary(),
            r.summary(),
            first_diff(&c.stdout, &r.stdout)
        );
        let mut want = vec![b'A'; v as usize];
        want.push(b'\n');
        assert_eq!(c.stdout, want, "driver({v}) does not match the derived C shape");
        assert_eq!(c.signal, None, "driver({v}) must not crash");
    }
}

/// ERRORS.md row 8 — `data == -1`. The C never checks the lower bound, so this
/// passes `data < 100` and then hands `(size_t)(-1)` to `strncpy` while storing
/// `dest[-1]`. The C's real behaviour is a fatal SIGSEGV; the Rust must fail
/// with the SAME signal and the same (empty) stdout.
#[test]
fn err_08_driver_negative_one() {
    assert_fatal_and_identical(-1);
}

/// ERRORS.md row 9 — `data == -2`.
#[test]
fn err_09_driver_negative_two() {
    assert_fatal_and_identical(-2);
}

/// ERRORS.md row 10 — `data == -100`, magnitude past the buffer size.
#[test]
fn err_10_driver_negative_hundred() {
    assert_fatal_and_identical(-100);
}

/// ERRORS.md row 11 — `data == INT_MIN`, the extreme negative.
#[test]
fn err_11_driver_int_min() {
    assert_fatal_and_identical(i32::MIN);
}

/// Additional negative values, to show the fatal outcome is uniform across the
/// whole unchecked lower range rather than specific to the four table rows.
#[test]
fn err_12_driver_negative_sweep() {
    let mut rng = Rng::new(SEED ^ 0x12);
    let mut vals: Vec<c_int> = vec![-3, -50, -99, -101, -1000, i32::MIN + 1];
    for _ in 0..6 {
        vals.push(rng.range_i64(i32::MIN as i64, -1) as c_int);
    }
    for v in vals {
        assert_fatal_and_identical(v);
    }
}

/// Shared assertion for the unchecked-lower-bound rows: C and Rust must agree
/// on the exact fatal signal and on stdout, and the C's outcome must really be
/// the fatal signal (not a quiet success we merely matched).
fn assert_fatal_and_identical(data: c_int) {
    const SIGSEGV: i32 = 11;
    let ops = [Op::Driver(data)];
    let c = run_ops(Impl::C, &ops);
    let r = run_ops(Impl::Rust, &ops);
    assert_eq!(
        c,
        r,
        "driver({data}) subprocess outcome mismatch\n  C   : {}\n  Rust: {}",
        c.summary(),
        r.summary()
    );
    assert_eq!(
        c.signal,
        Some(SIGSEGV),
        "the C ground truth for driver({data}) is death by SIGSEGV; got {}",
        c.summary()
    );
    assert!(
        c.stdout.is_empty(),
        "C crashes before any output for driver({data}); got {}",
        hex(&c.stdout)
    );
}

// ===========================================================================
// PHASE D — symbol parity, asserted from inside the test suite as well.
// ===========================================================================

/// Every symbol the C `.so` exports must be resolvable in the Rust `.so` under
/// the exact same name.
#[test]
fn sym_01_exported_symbol_parity() {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(c_so_path())
        .output()
        .expect("run nm on the C .so");
    assert!(out.status.success(), "nm failed on the C .so");
    let text = String::from_utf8_lossy(&out.stdout);

    let mut c_syms: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Global text/data symbols only; skip ld/glibc bookkeeping.
            if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "i") {
                return None;
            }
            if name.starts_with('_') {
                return None;
            }
            Some(name.to_string())
        })
        .collect();
    c_syms.sort();
    c_syms.dedup();

    assert!(
        !c_syms.is_empty(),
        "parsed no exported symbols from the C .so; nm output was:\n{text}"
    );
    assert!(
        c_syms.iter().any(|s| s == "driver") && c_syms.iter().any(|s| s == "printLine"),
        "expected both `driver` and `printLine` among the C exports, got {c_syms:?}"
    );

    let rp = rust_so_path();
    // SAFETY: loading a well-formed shared object; symbols are only resolved.
    let rust = unsafe { Library::new(&rp) }.unwrap_or_else(|e| panic!("dlopen {rp:?}: {e}"));
    let mut missing = Vec::new();
    for s in &c_syms {
        let mut name = s.clone().into_bytes();
        name.push(0);
        let found = unsafe { rust.get::<*const c_void>(&name) }.is_ok();
        if !found {
            missing.push(s.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "the Rust .so does not export these C symbols: {missing:?} (C exports {c_syms:?})"
    );
}

/// Both symbols must be reachable through `dlsym` on the Rust `.so` and be
/// callable with the C ABI — i.e. the `#[no_mangle]` export wrappers work.
#[test]
fn sym_02_rust_exports_are_callable_via_dlsym() {
    diff_ops(
        &[
            Op::PrintLine(b"callable via dlsym\0".to_vec()),
            Op::Driver(7),
            Op::PrintLineNull,
        ],
        "sym_02 both exports callable through dlsym",
    );
}
