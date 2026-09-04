// Shared differential-testing harness.
//
// Both libraries are loaded through `libloading` -- the Rust implementation is
// NEVER called directly, always through the exported C ABI symbols of
// `libdriver.so`, exactly as an external consumer would.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need for byte-exact stdout capture. Declared by hand so the
// harness needs no dependency other than `libloading`.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

// ---------------------------------------------------------------------------
// Function signatures under test (from c_src/include/driver.h and
// c_src/src/driver.c).
// ---------------------------------------------------------------------------
pub type FmaArrayFn = unsafe extern "C" fn(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
);
pub type DriverFn = unsafe extern "C" fn(data: *const c_int, len: c_int);

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub fma_array: FmaArrayFn,
    pub driver: DriverFn,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({:?}): {e}", name, path));
        // SAFETY: symbols are looked up by the exact names `nm -D` reports and
        // the pointers are kept alive by the `Library` stored alongside them.
        let fma_array: FmaArrayFn = unsafe {
            let s: Symbol<FmaArrayFn> = lib
                .get(b"fma_array\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `fma_array`: {e}"));
            *s.into_raw()
        };
        let driver: DriverFn = unsafe {
            let s: Symbol<DriverFn> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `driver`: {e}"));
            *s.into_raw()
        };
        Impl { name, _lib: lib, fma_array, driver }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}.\nBuild it with:\n  cd c_src && mkdir -p build && \
         cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // The cdylib is produced in the same profile as the test binary. Locate it
    // relative to the test executable itself so it works for dev, release and
    // any custom profile.
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>
    let mut dir = exe.parent().unwrap().to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let cand = dir.join("libdriver.so");
    if cand.exists() {
        return cand;
    }
    for profile in ["debug", "release"] {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found (looked in {cand:?} and target/{{debug,release}}).\n\
         Run `cargo build` first: this crate's only crate-type is `cdylib`, and `cargo test` \
         does NOT emit the cdylib artifact."
    );
}

/// Guard against silently testing a STALE `.so`.
///
/// The crate's only `crate-type` is `cdylib`. `cargo test` compiles `src/lib.rs`
/// into a unit-test binary but does **not** re-emit `target/<profile>/libdriver.so`
/// -- so `cargo test` alone will happily re-run the whole differential suite
/// against an old shared object and report success for a broken translation.
/// (Verified: mutating `src/lib.rs` and running only `cargo test` left every test
/// passing.) Refuse to run if the `.so` is older than any Rust source file.
fn assert_so_fresh(so: &PathBuf) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let entries = match std::fs::read_dir(&src_dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().map(|x| x != "rs").unwrap_or(true) {
            continue;
        }
        if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
            assert!(
                t <= so_mtime,
                "STALE ARTIFACT: {p:?} is newer than {so:?}.\n\
                 This crate's only crate-type is `cdylib`, and `cargo test` does NOT rebuild \
                 the cdylib, so the suite would be comparing against an OUT-OF-DATE Rust .so.\n\
                 Run `cargo build` (or `cargo build --release`) before `cargo test`, or just \
                 use ./run_all.sh which does both."
            );
        }
    }
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    assert_single_threaded();
    PAIR.get_or_init(|| {
        let rs = rust_so_path();
        assert_so_fresh(&rs);
        Pair { c: Impl::load("C", &c_so_path()), rs: Impl::load("Rust", &rs) }
    })
}

/// The library under test writes to `stdout` via C `printf`, so capturing its
/// output means redirecting file descriptor 1, which is process-global. libtest
/// also writes its progress lines ("test foo ... ok") to fd 1 from the runner
/// thread; if tests run concurrently those bytes land inside a capture and
/// corrupt the comparison. Refuse to run unless the harness is single-threaded.
///
/// `.cargo/config.toml` sets `RUST_TEST_THREADS = "1"` so plain `cargo test`
/// just works; this check catches the case where the test binary is invoked
/// directly.
fn assert_single_threaded() {
    let args: Vec<String> = std::env::args().collect();
    let cli_one = args.windows(2).any(|w| w[0] == "--test-threads" && w[1] == "1")
        || args.iter().any(|a| a == "--test-threads=1");
    let env_one = std::env::var("RUST_TEST_THREADS").ok().as_deref() == Some("1");
    assert!(
        cli_one || env_one,
        "these differential tests redirect the process-global fd 1 and must run \
         single-threaded.\nRun them as:  RUST_TEST_THREADS=1 cargo test    (or pass \
         `-- --test-threads=1`).\nNote: translation/.cargo/config.toml already sets \
         RUST_TEST_THREADS=1 for `cargo test`."
    );
}

// ---------------------------------------------------------------------------
// stdout capture
//
// fd 1 is process-global, so every capture must be serialized. Tests also take
// this lock so that a multi-capture sequence (e.g. C then Rust) is atomic.
// ---------------------------------------------------------------------------
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

pub fn stdout_guard() -> MutexGuard<'static, ()> {
    STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Run `f` with file descriptor 1 redirected into a temporary file and return
/// the exact bytes it wrote. `stdout` is flushed before and after so nothing
/// from a neighbouring capture leaks in or out.
pub fn capture_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = format!("{dir}/driver_difftest_capture_{}.bin\0", std::process::id());

    unsafe {
        // Flush anything already pending on the real stdout (both libc's and
        // Rust's own buffer) so it is not misattributed to this capture.
        fflush(std::ptr::null_mut());
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(path.as_ptr() as *const c_char, O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "open({path}) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        let r = f();

        // The library under test uses C `printf`, which is fully buffered when
        // stdout is a file: flush before reading the bytes back.
        fflush(std::ptr::null_mut());

        assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
        close(saved);

        lseek(fd, 0, 0 /* SEEK_SET */);
        let mut out = Vec::new();
        let mut buf = [0u8; 65536];
        loop {
            let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(fd);
        (r, out)
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seed, reproducible.
// ---------------------------------------------------------------------------
pub struct Rng(pub u64);

impl Rng {
    pub const DEFAULT_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_add(Self::DEFAULT_SEED))
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` inclusive, over i64 then narrowed.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// Value-shape generators (Axis D of CONFIGS.md)
// ---------------------------------------------------------------------------
pub const EXTREMES: &[i32] = &[i32::MIN, i32::MIN + 1, -46341, -46340, -65536, -65535, -2, -1, 0, 1, 2, 65535, 65536, 46340, 46341, i32::MAX - 1, i32::MAX];
pub const BOUNDARY: &[i32] = &[46340, 46341, -46340, -46341, 65535, 65536, -65535, -65536, 46339, -46339];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    /// V0
    Zeros,
    /// V1
    Ones,
    /// V2
    SmallPos,
    /// V3
    SmallNeg,
    /// V4
    MixedSmall,
    /// V5 -- |x| <= 46340, product still representable
    SafeMag,
    /// V6 -- values right at the first-overflow boundary
    Boundary,
    /// V7 -- INT_MAX / INT_MIN / +-1 / 0, cycled so every position sees each
    Extremes,
    /// V8 -- uniform over the whole i32 range
    FullRandom,
    /// V9 -- random draws from the extreme pool
    ExtremePool,
}

pub const ALL_SHAPES: &[Shape] = &[
    Shape::Zeros,
    Shape::Ones,
    Shape::SmallPos,
    Shape::SmallNeg,
    Shape::MixedSmall,
    Shape::SafeMag,
    Shape::Boundary,
    Shape::Extremes,
    Shape::FullRandom,
    Shape::ExtremePool,
];

pub fn gen_vals(shape: Shape, len: usize, rng: &mut Rng) -> Vec<c_int> {
    let mut v = Vec::with_capacity(len);
    // For `Extremes` rotate the starting offset so, across draws, every extreme
    // lands in every position.
    let rot = if len == 0 { 0 } else { (rng.next_u64() % EXTREMES.len() as u64) as usize };
    for i in 0..len {
        v.push(match shape {
            Shape::Zeros => 0,
            Shape::Ones => 1,
            Shape::SmallPos => rng.range_i32(1, 9),
            Shape::SmallNeg => rng.range_i32(-9, -1),
            Shape::MixedSmall => rng.range_i32(-9, 9),
            Shape::SafeMag => rng.range_i32(-46340, 46340),
            Shape::Boundary => rng.pick(BOUNDARY),
            Shape::Extremes => EXTREMES[(i + rot) % EXTREMES.len()],
            Shape::FullRandom => rng.next_i32(),
            Shape::ExtremePool => rng.pick(EXTREMES),
        });
    }
    v
}

/// `len` values used everywhere (Axis C of CONFIGS.md): empty / one / many /
/// power-of-two boundaries.
pub const LENS: &[usize] = &[0, 1, 2, 3, 4, 5, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 1000];

/// Number of independent random draws per (row, len) pair.
pub const DRAWS: usize = 8;

// ---------------------------------------------------------------------------
// Aliasing configurations (Axis B of CONFIGS.md)
// ---------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Alias {
    /// A0: out, mul1, mul2, add all distinct
    Distinct,
    /// A1: out == mul1
    OutIsMul1,
    /// A2: out == mul2
    OutIsMul2,
    /// A3: out == add
    OutIsAdd,
    /// A4: out == mul1 == mul2 == add (what `inner` does)
    AllSame,
    /// A5: mul1 == mul2 (squaring), out and add distinct
    Mul1IsMul2,
    /// A6: mul1 == mul2 == add, out distinct
    InputsAllSame,
    /// A7: out == mul1 == mul2, add distinct
    OutMul1Mul2,
    /// A8: out == mul1 and mul2 == add
    TwoPairs,
}

pub const ALL_ALIASES: &[Alias] = &[
    Alias::Distinct,
    Alias::OutIsMul1,
    Alias::OutIsMul2,
    Alias::OutIsAdd,
    Alias::AllSame,
    Alias::Mul1IsMul2,
    Alias::InputsAllSame,
    Alias::OutMul1Mul2,
    Alias::TwoPairs,
];

/// How many independent buffers an aliasing configuration needs, and which
/// buffer index each of the four parameters points at:
/// `(nbufs, [out, mul1, mul2, add])`.
pub fn alias_layout(a: Alias) -> (usize, [usize; 4]) {
    match a {
        Alias::Distinct => (4, [0, 1, 2, 3]),
        Alias::OutIsMul1 => (3, [0, 0, 1, 2]),
        Alias::OutIsMul2 => (3, [0, 1, 0, 2]),
        Alias::OutIsAdd => (3, [0, 1, 2, 0]),
        Alias::AllSame => (1, [0, 0, 0, 0]),
        Alias::Mul1IsMul2 => (3, [0, 1, 1, 2]),
        Alias::InputsAllSame => (2, [0, 1, 1, 1]),
        Alias::OutMul1Mul2 => (2, [0, 0, 0, 1]),
        Alias::TwoPairs => (2, [0, 0, 1, 1]),
    }
}

/// Run `fma_array` in aliasing configuration `a` on both implementations and
/// assert every buffer is byte-identical afterwards.
///
/// `bufs` is the set of independent starting buffers (already generated); it is
/// cloned separately for each implementation.
pub fn diff_fma_alias(p: &Pair, a: Alias, bufs: &[Vec<c_int>], len: c_int, ctx: &str) {
    let (nbufs, map) = alias_layout(a);
    assert_eq!(nbufs, bufs.len(), "{ctx}: wrong buffer count for {a:?}");

    let run = |imp: &Impl| -> Vec<Vec<c_int>> {
        let mut local: Vec<Vec<c_int>> = bufs.to_vec();
        // Raw pointers into `local`, taken before any call so aliasing is real.
        let ptrs: Vec<*mut c_int> = local.iter_mut().map(|b| b.as_mut_ptr()).collect();
        unsafe {
            (imp.fma_array)(
                ptrs[map[0]],
                ptrs[map[1]] as *const c_int,
                ptrs[map[2]] as *const c_int,
                ptrs[map[3]] as *const c_int,
                len,
            );
        }
        local
    };

    let got_c = run(&p.c);
    let got_rs = run(&p.rs);
    assert_bufs_eq(&got_c, &got_rs, bufs, a, len, ctx);
}

fn assert_bufs_eq(
    got_c: &[Vec<c_int>],
    got_rs: &[Vec<c_int>],
    input: &[Vec<c_int>],
    a: Alias,
    len: c_int,
    ctx: &str,
) {
    for (i, (bc, br)) in got_c.iter().zip(got_rs.iter()).enumerate() {
        if bc != br {
            let bad = bc.iter().zip(br.iter()).position(|(x, y)| x != y);
            panic!(
                "DIVERGENCE {ctx}\n  alias={a:?} len={len} buffer #{i}\n  \
                 first differing index: {bad:?}\n  input   = {:?}\n  C   out = {:?}\n  Rust out= {:?}",
                trunc(&input[i]),
                trunc(bc),
                trunc(br)
            );
        }
    }
}

pub fn trunc(v: &[c_int]) -> String {
    if v.len() <= 24 {
        format!("{v:?}")
    } else {
        format!("{:?}.. ({} elems)", &v[..24], v.len())
    }
}

/// Run `driver` on both implementations and assert the captured stdout bytes
/// are identical. Also asserts neither implementation modified the caller's
/// input buffer (the C parameter is `const int *`).
pub fn diff_driver(p: &Pair, data: &[c_int], len: c_int, ctx: &str) -> Vec<u8> {
    let _g = stdout_guard();

    let mut c_in = data.to_vec();
    let mut rs_in = data.to_vec();

    let (_, out_c) = capture_stdout(|| unsafe { (p.c.driver)(c_in.as_mut_ptr(), len) });
    let (_, out_rs) = capture_stdout(|| unsafe { (p.rs.driver)(rs_in.as_mut_ptr(), len) });

    if out_c != out_rs {
        panic!(
            "DIVERGENCE {ctx}\n  len={len} data={}\n  C    stdout ({} bytes) = {:?}\n  \
             Rust stdout ({} bytes) = {:?}",
            trunc(data),
            out_c.len(),
            String::from_utf8_lossy(&out_c[..out_c.len().min(400)]),
            out_rs.len(),
            String::from_utf8_lossy(&out_rs[..out_rs.len().min(400)]),
        );
    }
    assert_eq!(c_in, data, "{ctx}: C driver modified the caller's `const` input buffer");
    assert_eq!(rs_in, data, "{ctx}: Rust driver modified the caller's input buffer");
    out_c
}
