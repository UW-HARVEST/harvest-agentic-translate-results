// Differential test: load BOTH the C shared library and the Rust `cdylib` with
// `libloading` and compare every exported symbol's behaviour through the FFI
// boundary. No Rust function is ever called directly -- everything goes through
// `libdriver.so`'s `#[no_mangle]` exports, exactly as an external C consumer
// would see them.
//
// The artifacts for the *current* feature configuration are produced by
// `scripts/build_artifacts.sh` into
//     artifacts/<op>_<repeat>/libcdriver.so   (gcc -shared, c_src/src/mdcore.c)
//     artifacts/<op>_<repeat>/libdriver.so    (cargo build, src/lib.rs)
//     artifacts/<op>_<repeat>/cbin/driver     (cmake, the C executable)
//     artifacts/<op>_<repeat>/rbin/driver     (cargo build, the Rust executable)
//
// Run with `--test-threads=1`: the stdout comparison temporarily redirects
// file descriptor 1, which is process-global.

#![allow(clippy::needless_range_loop)]

use std::ffi::{c_char, c_int, CStr};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Build-configuration mirror.
//
// Exactly the same feature precedence as `src/mdmacros.rs`, so that the test
// binary and the `.so` it loads always agree on which C build to compare with.
// ---------------------------------------------------------------------------

const OP_NAME: &str = if cfg!(feature = "mul") {
    "mul"
} else if cfg!(feature = "sub") {
    "sub"
} else {
    "add"
};

const REPEAT: c_int = if cfg!(feature = "0") {
    0
} else if cfg!(feature = "1") {
    1
} else if cfg!(feature = "2") {
    2
} else if cfg!(feature = "3") {
    3
} else if cfg!(feature = "4") {
    4
} else if cfg!(feature = "6") {
    6
} else if cfg!(feature = "7") {
    7
} else {
    5
};

/// C: `INIT_FOR(OP)` -- `INIT_mul` is 1, `INIT_add`/`INIT_sub` are 0.
const INIT: c_int = if cfg!(feature = "mul") { 1 } else { 0 };

// ---------------------------------------------------------------------------
// FFI signatures of the eight exported symbols.
// ---------------------------------------------------------------------------

type BinFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
type UnFn = unsafe extern "C" fn(c_int) -> c_int;

/// `int (*G_OP)(int,int)` -- a writable `.data` object holding a function pointer.
type GOpSlot = *mut Option<BinFn>;
/// `const char *G_OP_NAME` -- a writable `.data` object holding a `char*`.
type GNameSlot = *mut *const c_char;

// ---------------------------------------------------------------------------
// Artifact locations / library loading.
// ---------------------------------------------------------------------------

fn crate_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Where the four artifacts for the current configuration live.
///
/// By default this is derived from the *features this test binary was compiled
/// with*, so the C build it is compared against always matches (`-DOP=<OP_NAME>
/// -DREPEAT=<REPEAT>`). `MD_ARTIFACT_DIR` overrides it, which is how
/// `scripts/run_all.sh` checks the alternative feature spellings
/// (`repeat_N` aliases, and "cache variable not set" fallbacks) against the
/// C build they must agree with.
fn artifact_dir() -> PathBuf {
    if let Ok(d) = std::env::var("MD_ARTIFACT_DIR") {
        return PathBuf::from(d);
    }
    crate_root()
        .join("artifacts")
        .join(format!("{OP_NAME}_{REPEAT}"))
}

/// Build the four artifacts for this configuration if they are not there yet, so
/// that a bare `cargo test --features <op>,<n>` works on a clean tree.
///
/// The C flags reproduce `c_src/CMakeLists.txt` exactly: it overwrites
/// `CMAKE_C_FLAGS` with just `-DOP=${OP} -DREPEAT=${REPEAT}` and sets no build
/// type, so there is no optimisation flag.
fn ensure_artifacts() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = artifact_dir();
        let explicit = std::env::var_os("MD_ARTIFACT_DIR").is_some();
        let complete = ["libcdriver.so", "libdriver.so", "cbin/driver", "rbin/driver"]
            .iter()
            .all(|p| dir.join(p).exists());
        if complete || explicit {
            assert!(
                complete,
                "MD_ARTIFACT_DIR={} is missing one of libcdriver.so, libdriver.so, \
                 cbin/driver, rbin/driver",
                dir.display()
            );
            return dir;
        }

        let root = crate_root();
        let core = root.join("c_src/src/mdcore.c");
        let main_c = root.join("c_src/src/mdmain.c");
        std::fs::create_dir_all(dir.join("cbin")).expect("mkdir cbin");
        std::fs::create_dir_all(dir.join("rbin")).expect("mkdir rbin");
        let dop = format!("-DOP={OP_NAME}");
        let drep = format!("-DREPEAT={REPEAT}");

        let ok = Command::new("gcc")
            .args(["-fPIC", "-shared", &dop, &drep, "-o"])
            .arg(dir.join("libcdriver.so"))
            .arg(&core)
            .status()
            .expect("run gcc")
            .success();
        assert!(ok, "failed to build the C shared library");

        let ok = Command::new("gcc")
            .args([&dop, &drep, "-o"])
            .arg(dir.join("cbin/driver"))
            .args([&core, &main_c])
            .status()
            .expect("run gcc")
            .success();
        assert!(ok, "failed to build the C executable");

        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let scratch = std::env::temp_dir().join(format!("md_selfbuild_{OP_NAME}_{REPEAT}"));
        let ok = Command::new(&cargo)
            .args([
                "build",
                "--offline",
                "--quiet",
                "--no-default-features",
                "--features",
            ])
            .arg(format!("{OP_NAME},{REPEAT}"))
            .current_dir(root)
            .env("CARGO_TARGET_DIR", &scratch)
            .status()
            .expect("run cargo build")
            .success();
        assert!(ok, "failed to build the Rust cdylib/executable");
        std::fs::copy(scratch.join("debug/libdriver.so"), dir.join("libdriver.so"))
            .expect("copy libdriver.so");
        std::fs::copy(scratch.join("debug/driver"), dir.join("rbin/driver"))
            .expect("copy driver");
        dir
    })
}

pub struct Pair {
    pub c: Library,
    pub r: Library,
}

fn libs() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let dir = ensure_artifacts().clone();
        let cpath = dir.join("libcdriver.so");
        let rpath = dir.join("libdriver.so");
        for p in [&cpath, &rpath] {
            assert!(
                p.exists(),
                "missing artifact {}\nrun: bash scripts/build_artifacts.sh \"{},{}\"",
                p.display(),
                OP_NAME,
                REPEAT
            );
        }
        // SAFETY: both paths are plain ELF shared objects built from this repo.
        unsafe {
            Pair {
                c: Library::new(&cpath).expect("load C .so"),
                r: Library::new(&rpath).expect("load Rust .so"),
            }
        }
    })
}

/// Resolve a function symbol from a library, asserting it exists.
fn bin_fn<'l>(lib: &'l Library, name: &str) -> Symbol<'l, BinFn> {
    let mut sym = name.as_bytes().to_vec();
    sym.push(0);
    // SAFETY: the symbol's C signature is `int(int,int)`.
    unsafe { lib.get(&sym) }.unwrap_or_else(|e| panic!("symbol {name}: {e}"))
}

fn un_fn<'l>(lib: &'l Library, name: &str) -> Symbol<'l, UnFn> {
    let mut sym = name.as_bytes().to_vec();
    sym.push(0);
    // SAFETY: the symbol's C signature is `int(int)`.
    unsafe { lib.get(&sym) }.unwrap_or_else(|e| panic!("symbol {name}: {e}"))
}

fn g_op_slot<'l>(lib: &'l Library) -> Symbol<'l, GOpSlot> {
    // SAFETY: `G_OP` is an 8-byte data object holding a function pointer.
    unsafe { lib.get(b"G_OP\0") }.expect("symbol G_OP")
}

fn g_name_slot<'l>(lib: &'l Library) -> Symbol<'l, GNameSlot> {
    // SAFETY: `G_OP_NAME` is an 8-byte data object holding a `const char*`.
    unsafe { lib.get(b"G_OP_NAME\0") }.expect("symbol G_OP_NAME")
}

// ---------------------------------------------------------------------------
// stdout capture (fd 1 redirection -- works for the C library's `printf`, which
// uses this process's libc `stdout`, and for the Rust `cdylib`'s own `std`,
// whose `LineWriter` flushes on every '\n').
// ---------------------------------------------------------------------------

/// One reusable scratch file, so a test that captures thousands of calls does
/// not create thousands of temporary files.
fn capture_file() -> &'static Mutex<File> {
    static L: OnceLock<Mutex<File>> = OnceLock::new();
    L.get_or_init(|| {
        let path = std::env::temp_dir().join(format!(
            "md_capture_{}_{}_{}.txt",
            std::process::id(),
            OP_NAME,
            REPEAT
        ));
        let f = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("open capture file");
        // Unlink immediately: the fd keeps it alive, nothing is left behind.
        let _ = std::fs::remove_file(&path);
        Mutex::new(f)
    })
}

fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let mut file = capture_file().lock().unwrap();

    // SAFETY: plain fd juggling; `saved` is restored before returning.
    unsafe {
        std::io::stdout().flush().ok();
        libc::fflush(std::ptr::null_mut());
        file.set_len(0).expect("truncate");
        file.seek(SeekFrom::Start(0)).expect("rewind");

        let saved = libc::dup(libc::STDOUT_FILENO);
        assert!(saved >= 0, "dup(1) failed");
        assert!(
            libc::dup2(file.as_raw_fd(), libc::STDOUT_FILENO) >= 0,
            "dup2 failed"
        );
        let out = f();
        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, libc::STDOUT_FILENO) >= 0, "restore failed");
        libc::close(saved);

        let mut buf = Vec::new();
        file.seek(SeekFrom::Start(0)).expect("seek");
        file.read_to_end(&mut buf).expect("read capture");
        (out, buf)
    }
}

// ---------------------------------------------------------------------------
// Comparison helpers.
// ---------------------------------------------------------------------------

fn cfg_tag() -> String {
    format!("[OP={OP_NAME} REPEAT={REPEAT}]")
}

/// Byte-for-byte stdout comparison with a readable first-difference message.
#[track_caller]
fn assert_stdout_eq(ctx: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    let cl: Vec<&[u8]> = c.split(|&b| b == b'\n').collect();
    let rl: Vec<&[u8]> = r.split(|&b| b == b'\n').collect();
    for i in 0..cl.len().max(rl.len()) {
        let a = cl.get(i).copied().unwrap_or(b"<missing>");
        let b = rl.get(i).copied().unwrap_or(b"<missing>");
        if a != b {
            panic!(
                "{} {}: stdout differs at line {}\n  C   : {:?}\n  Rust: {:?}\n  (C total {} bytes, Rust total {} bytes)",
                cfg_tag(),
                ctx,
                i,
                String::from_utf8_lossy(a),
                String::from_utf8_lossy(b),
                c.len(),
                r.len()
            );
        }
    }
    panic!(
        "{} {}: stdout differs only in trailing bytes ({} vs {})",
        cfg_tag(),
        ctx,
        c.len(),
        r.len()
    );
}

#[track_caller]
fn assert_rets_eq<T: std::fmt::Debug + PartialEq + Copy>(
    ctx: &str,
    labels: &[String],
    c: &[T],
    r: &[T],
) {
    assert_eq!(
        c.len(),
        r.len(),
        "{} {}: different number of results",
        cfg_tag(),
        ctx
    );
    for i in 0..c.len() {
        assert!(
            c[i] == r[i],
            "{} {}: return value mismatch for {}\n  C   : {:?}\n  Rust: {:?}",
            cfg_tag(),
            ctx,
            labels[i],
            c[i],
            r[i]
        );
    }
}

/// Drive a `int(int,int)` export over `pairs` in both libraries and compare the
/// full return-value vector *and* the whole captured stdout stream.
fn diff_bin_batch(name: &str, pairs: &[(c_int, c_int)]) {
    let l = libs();
    let cf = *bin_fn(&l.c, name);
    let rf = *bin_fn(&l.r, name);
    let (cv, co) = capture(|| {
        pairs
            .iter()
            .map(|&(a, b)| unsafe { cf(a, b) })
            .collect::<Vec<c_int>>()
    });
    let (rv, ro) = capture(|| {
        pairs
            .iter()
            .map(|&(a, b)| unsafe { rf(a, b) })
            .collect::<Vec<c_int>>()
    });
    let labels: Vec<String> = pairs
        .iter()
        .map(|&(a, b)| format!("{name}({a}, {b})"))
        .collect();
    assert_rets_eq(name, &labels, &cv, &rv);
    assert_stdout_eq(name, &co, &ro);
}

/// Drive a `int(int)` export over `ns` in both libraries and compare.
fn diff_un_batch(name: &str, ns: &[c_int]) {
    let l = libs();
    let cf = *un_fn(&l.c, name);
    let rf = *un_fn(&l.r, name);
    let (cv, co) = capture(|| ns.iter().map(|&n| unsafe { cf(n) }).collect::<Vec<c_int>>());
    let (rv, ro) = capture(|| ns.iter().map(|&n| unsafe { rf(n) }).collect::<Vec<c_int>>());
    let labels: Vec<String> = ns.iter().map(|&n| format!("{name}({n})")).collect();
    assert_rets_eq(name, &labels, &cv, &rv);
    assert_stdout_eq(name, &co, &ro);
}

fn edge_pairs() -> Vec<(c_int, c_int)> {
    let mut v = Vec::with_capacity(EDGES.len() * EDGES.len());
    for &a in EDGES.iter() {
        for &b in EDGES.iter() {
            v.push((a, b));
        }
    }
    v
}

fn random_pairs(seed: u64, n: usize) -> Vec<(c_int, c_int)> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| (rng.next_i32(), rng.next_i32()))
        .collect()
}

// ---------------------------------------------------------------------------
// Executable comparison helpers (the `main` / `mdmain.c` surface).
// ---------------------------------------------------------------------------

struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
}

fn run_exe(which: &str, args: &[&str]) -> Run {
    // Both executables are named `driver` and are invoked as `./driver` from
    // their own directory so that `argv[0]` is byte-identical.
    let dir = ensure_artifacts().join(which);
    let out = Command::new("./driver")
        .args(args)
        .current_dir(&dir)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}/driver: {e}", dir.display()));
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

#[track_caller]
fn diff_exe(args: &[&str]) {
    let c = run_exe("cbin", args);
    let r = run_exe("rbin", args);
    let ctx = format!("driver {args:?}");
    assert_stdout_eq(&ctx, &c.stdout, &r.stdout);
    assert_stdout_eq(&format!("{ctx} (stderr)"), &c.stderr, &r.stderr);
    assert_eq!(
        c.code,
        r.code,
        "{} {}: exit status differs",
        cfg_tag(),
        ctx
    );
}


// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) -- fixed seed per test for reproducibility.
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn next_i32(&mut self) -> c_int {
        (self.next_u64() >> 32) as u32 as c_int
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// 41 distinct edge values: identities, small magnitudes, `sqrt(INT_MAX)`
/// neighbours (overflow boundary for `op_mul`), powers of two and the extremes.
const EDGES: [c_int; 41] = [
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    4,
    -4,
    5,
    -5,
    6,
    -6,
    7,
    -7,
    8,
    -8,
    10,
    -10,
    100,
    -100,
    255,
    -255,
    256,
    -256,
    1000,
    -1000,
    46340,
    46341,
    -46340,
    -46341,
    65535,
    65536,
    -65536,
    1_073_741_824,
    -1_073_741_824,
    536_870_912,
    i32::MAX,
    i32::MAX - 1,
    i32::MIN,
    i32::MIN + 1,
];

// ===========================================================================
// PHASE B -- valid-path differential tests (one per CONFIGS.md row)
// ===========================================================================

/// CONFIGS row 4 -- `op_add`, randomized full-range `i32` pairs.
#[test]
fn b01_op_add_randomized() {
    diff_bin_batch("op_add", &random_pairs(0x0B01, 4096));
}

/// CONFIGS row 5 -- `op_sub`, randomized full-range `i32` pairs.
#[test]
fn b02_op_sub_randomized() {
    diff_bin_batch("op_sub", &random_pairs(0x0B02, 4096));
}

/// CONFIGS row 6 -- `op_mul`, randomized full-range `i32` pairs (overflow is
/// pervasive here: the C is UB but gcc at `-O0` wraps, so the Rust must wrap).
#[test]
fn b03_op_mul_randomized() {
    diff_bin_batch("op_mul", &random_pairs(0x0B03, 4096));
}

/// CONFIGS rows 1-3 / ERRORS row 19 -- the full 41x41 edge-value cross product
/// against all three low-level operation exports (independent of `OP`: all three
/// are always compiled in, `mdcore.c:28-30`).
#[test]
fn b04_op_fns_edge_cross_product() {
    let pairs = edge_pairs();
    assert_eq!(pairs.len(), 1681);
    for name in ["op_add", "op_sub", "op_mul"] {
        diff_bin_batch(name, &pairs);
    }
}

/// CONFIGS row 7 -- `helper_call` over the 41x41 edge cross product. Compares
/// the returned `r + acc` and the exact `helper.call=%d helper.acc=%d\n` line,
/// which is where the build-time `REPEAT` shows up.
#[test]
fn b05_helper_call_edge_cross_product() {
    diff_bin_batch("helper_call", &edge_pairs());
}

/// CONFIGS row 8 -- `helper_call`, randomized full-range pairs.
#[test]
fn b06_helper_call_randomized() {
    diff_bin_batch("helper_call", &random_pairs(0x0B06, 2048));
}

/// CONFIGS row 9 -- `helper_ptr` over the 41x41 edge cross product.
#[test]
fn b07_helper_ptr_edge_cross_product() {
    diff_bin_batch("helper_ptr", &edge_pairs());
}

/// CONFIGS row 10 -- `helper_ptr`, randomized full-range pairs.
#[test]
fn b08_helper_ptr_randomized() {
    diff_bin_batch("helper_ptr", &random_pairs(0x0B08, 2048));
}

/// CONFIGS rows 11-17 -- every distinct `case` of `DISPATCH_REP`
/// (`mdmacros.h:84-90`), one row per `case`, driven in both directions and
/// repeated so ordering effects would show up.
#[test]
fn b09_use_generated_each_switch_case() {
    for n in 0..=6 {
        diff_un_batch("use_generated", &[n]);
    }
    // all cases in one captured stream, forwards and backwards
    diff_un_batch("use_generated", &[0, 1, 2, 3, 4, 5, 6]);
    diff_un_batch("use_generated", &[6, 5, 4, 3, 2, 1, 0]);
}

/// CONFIGS row 18 -- `use_generated` with `n` drawn uniformly from the whole
/// `int` range, which mixes in-`switch` and `default:` shapes.
#[test]
fn b10_use_generated_randomized_full_range() {
    let mut rng = Rng::new(0x0B10);
    let ns: Vec<c_int> = (0..4096).map(|_| rng.next_i32()).collect();
    diff_un_batch("use_generated", &ns);
    // plus a run biased towards the interesting neighbourhood of the switch
    let mut rng2 = Rng::new(0x0B10_0002);
    let near: Vec<c_int> = (0..2048)
        .map(|_| (rng2.below(41) as i64 - 20) as c_int)
        .collect();
    diff_un_batch("use_generated", &near);
}

/// CONFIGS row 19 -- `use_generated(REPEAT)`, i.e. exactly the call `main`
/// makes. Pins the `REPEAT == 7` asymmetry: `helper_call` unrolls seven steps
/// via `REP7`, yet `DISPATCH_REP` has no `case 7:` so this returns `INIT`.
#[test]
fn b11_use_generated_at_repeat() {
    diff_un_batch("use_generated", &[REPEAT]);

    let l = libs();
    let cf = *un_fn(&l.c, "use_generated");
    let (cv, _) = capture(|| unsafe { cf(REPEAT) });
    if REPEAT == 7 {
        assert_eq!(
            cv, INIT,
            "{} use_generated(7) must fall through DISPATCH_REP's default:",
            cfg_tag()
        );
    }
}

/// CONFIGS row 20 -- `G_OP` is `OP_FN(OP)`; the stored pointer must be the
/// address of that library's own `op_<OP>`.
#[test]
fn b12_g_op_points_at_selected_op() {
    let l = libs();
    for (tag, lib) in [("C", &l.c), ("Rust", &l.r)] {
        let slot: GOpSlot = *g_op_slot(lib);
        let stored = unsafe { *slot }.expect("G_OP must not be NULL");
        let expected = *bin_fn(lib, &format!("op_{OP_NAME}"));
        assert_eq!(
            stored as usize, expected as usize,
            "{} {tag}: G_OP does not point at op_{OP_NAME}",
            cfg_tag()
        );
        // and it must NOT be one of the other two
        for other in ["add", "sub", "mul"] {
            if other == OP_NAME {
                continue;
            }
            let o = *bin_fn(lib, &format!("op_{other}"));
            assert_ne!(
                stored as usize, o as usize,
                "{} {tag}: G_OP wrongly points at op_{other}",
                cfg_tag()
            );
        }
    }
}

/// CONFIGS row 21 -- call through the `G_OP` function pointer read out of each
/// `.so` (edge cross product + randomized pairs).
#[test]
fn b13_g_op_call_through() {
    let l = libs();
    let cslot: GOpSlot = *g_op_slot(&l.c);
    let rslot: GOpSlot = *g_op_slot(&l.r);
    let cf = unsafe { *cslot }.expect("C G_OP");
    let rf = unsafe { *rslot }.expect("Rust G_OP");

    let mut pairs = edge_pairs();
    pairs.extend(random_pairs(0x0B13, 2048));

    let (cv, co) = capture(|| {
        pairs
            .iter()
            .map(|&(a, b)| unsafe { cf(a, b) })
            .collect::<Vec<c_int>>()
    });
    let (rv, ro) = capture(|| {
        pairs
            .iter()
            .map(|&(a, b)| unsafe { rf(a, b) })
            .collect::<Vec<c_int>>()
    });
    let labels: Vec<String> = pairs
        .iter()
        .map(|&(a, b)| format!("G_OP({a}, {b})"))
        .collect();
    assert_rets_eq("G_OP call-through", &labels, &cv, &rv);
    assert_stdout_eq("G_OP call-through", &co, &ro);
}

/// CONFIGS row 22 -- `G_OP` is a *writable* `.data` object in C (`int
/// (*G_OP)(int,int)` is not const). Store each of the three operations into it,
/// dispatch through it, and restore. Proves the Rust object is writable too and
/// that both dispatch identically.
#[test]
fn b14_g_op_writable_then_call_through() {
    let l = libs();
    let cslot: GOpSlot = *g_op_slot(&l.c);
    let rslot: GOpSlot = *g_op_slot(&l.r);
    let corig = unsafe { *cslot };
    let rorig = unsafe { *rslot };

    let pairs = {
        let mut p = edge_pairs();
        p.extend(random_pairs(0x0B14, 512));
        p
    };

    for target in ["op_add", "op_sub", "op_mul"] {
        let cf = *bin_fn(&l.c, target);
        let rf = *bin_fn(&l.r, target);
        unsafe {
            *cslot = Some(cf);
            *rslot = Some(rf);
        }
        let cnow = unsafe { *cslot }.expect("C G_OP after store");
        let rnow = unsafe { *rslot }.expect("Rust G_OP after store");
        assert_eq!(
            cnow as usize, cf as usize,
            "{} C G_OP did not accept the store of {target}",
            cfg_tag()
        );
        assert_eq!(
            rnow as usize, rf as usize,
            "{} Rust G_OP did not accept the store of {target} (is it in .data?)",
            cfg_tag()
        );
        let (cv, co) = capture(|| {
            pairs
                .iter()
                .map(|&(a, b)| unsafe { cnow(a, b) })
                .collect::<Vec<c_int>>()
        });
        let (rv, ro) = capture(|| {
            pairs
                .iter()
                .map(|&(a, b)| unsafe { rnow(a, b) })
                .collect::<Vec<c_int>>()
        });
        let labels: Vec<String> = pairs
            .iter()
            .map(|&(a, b)| format!("G_OP:={target} ({a}, {b})"))
            .collect();
        assert_rets_eq("G_OP store", &labels, &cv, &rv);
        assert_stdout_eq("G_OP store", &co, &ro);
    }

    unsafe {
        *cslot = corig;
        *rslot = rorig;
    }
    // `helper_*` must still behave after the round trip.
    diff_bin_batch("helper_call", &[(3, 4), (-1, 1)]);
}

/// CONFIGS row 23 -- `G_OP_NAME` is `STR(OP)`; compare the NUL-terminated bytes.
#[test]
fn b15_g_op_name_bytes() {
    let l = libs();
    let cslot: GNameSlot = *g_name_slot(&l.c);
    let rslot: GNameSlot = *g_name_slot(&l.r);
    let cp = unsafe { *cslot };
    let rp = unsafe { *rslot };
    assert!(!cp.is_null(), "C G_OP_NAME is NULL");
    assert!(!rp.is_null(), "Rust G_OP_NAME is NULL");
    let cs = unsafe { CStr::from_ptr(cp) };
    let rs = unsafe { CStr::from_ptr(rp) };
    assert_eq!(
        cs.to_bytes_with_nul(),
        rs.to_bytes_with_nul(),
        "{} G_OP_NAME bytes differ",
        cfg_tag()
    );
    assert_eq!(
        cs.to_bytes(),
        OP_NAME.as_bytes(),
        "{} C G_OP_NAME is not the selected OP",
        cfg_tag()
    );
}

/// Replay `mdmain.c`'s body using nothing but this library's exported symbols,
/// and rebuild the exact two summary lines `main` prints.
///
/// `helper_call` returns `r + acc` where `r == OP_FN(a,b) == G_OP(a,b)`, so the
/// `RUN_LOOP(OP, acc, REPEAT)` accumulator (which has no export of its own) is
/// recovered as `helper_call(a,b) - G_OP(a,b)`.
fn pipeline_stream(lib: &Library, a: c_int, b: c_int) -> (Vec<u8>, c_int) {
    let gslot: GOpSlot = *g_op_slot(lib);
    let gop = unsafe { *gslot }.expect("G_OP");
    let nslot: GNameSlot = *g_name_slot(lib);
    let name = unsafe { CStr::from_ptr(*nslot) }.to_bytes().to_vec();
    let hc = *bin_fn(lib, "helper_call");
    let hp = *bin_fn(lib, "helper_ptr");
    let ug = *un_fn(lib, "use_generated");

    let ((r_call, x1, x2, x3, g), mut out) = capture(|| unsafe {
        let r_call = gop(a, b);
        let x1 = hc(a, b);
        let x2 = hp(a, b);
        let x3 = ug(REPEAT);
        let g = gop(a, b);
        (r_call, x1, x2, x3, g)
    });
    let acc = x1.wrapping_sub(r_call);
    out.extend_from_slice(
        format!(
            "op={} call={} acc={} g.call={}\n",
            String::from_utf8_lossy(&name),
            r_call,
            acc,
            g
        )
        .as_bytes(),
    );
    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);
    out.extend_from_slice(format!("summary={summary}\n").as_bytes());
    (out, summary)
}

/// CONFIGS row 24 -- the composed pipeline through the `.so` exports, compared
/// C-vs-Rust *and* against the byte stream the real C executable prints.
#[test]
fn b16_composed_pipeline_like_main() {
    let l = libs();
    let mut inputs: Vec<(c_int, c_int)> = edge_pairs();
    inputs.extend(random_pairs(0x0B16, 1024));

    for &(a, b) in inputs.iter() {
        let (cout, csum) = pipeline_stream(&l.c, a, b);
        let (rout, rsum) = pipeline_stream(&l.r, a, b);
        assert_stdout_eq(&format!("pipeline({a}, {b})"), &cout, &rout);
        assert_eq!(csum, rsum, "{} pipeline({a},{b}) summary", cfg_tag());
    }

    // The reconstruction must reproduce the executable's stdout exactly.
    for &(a, b) in EDGES.iter().zip(EDGES.iter().rev()).map(|(x, y)| (*x, *y)).collect::<Vec<_>>().iter() {
        let (rout, _) = pipeline_stream(&l.r, a, b);
        let exe = run_exe("cbin", &[&a.to_string(), &b.to_string()]);
        assert_stdout_eq(
            &format!("pipeline vs C executable ({a}, {b})"),
            &exe.stdout,
            &rout,
        );
    }
}

/// CONFIGS row 25 -- randomized *sequences* of interleaved calls captured as one
/// stdout stream, to catch output-ordering / buffering divergence.
#[test]
fn b17_interleaved_call_sequences() {
    let l = libs();
    let mut rng = Rng::new(0x0B17);

    for _ in 0..256 {
        let len = 1 + rng.below(24) as usize;
        let script: Vec<(u64, c_int, c_int)> = (0..len)
            .map(|_| (rng.below(7), rng.next_i32(), rng.next_i32()))
            .collect();

        let run = |lib: &Library| -> (Vec<c_int>, Vec<u8>) {
            let gslot: GOpSlot = *g_op_slot(lib);
            let gop = unsafe { *gslot }.expect("G_OP");
            let hc = *bin_fn(lib, "helper_call");
            let hp = *bin_fn(lib, "helper_ptr");
            let ug = *un_fn(lib, "use_generated");
            let oa = *bin_fn(lib, "op_add");
            let os = *bin_fn(lib, "op_sub");
            let om = *bin_fn(lib, "op_mul");
            capture(|| {
                script
                    .iter()
                    .map(|&(k, a, b)| unsafe {
                        match k {
                            0 => hc(a, b),
                            1 => hp(a, b),
                            2 => ug(a),
                            3 => oa(a, b),
                            4 => os(a, b),
                            5 => om(a, b),
                            _ => gop(a, b),
                        }
                    })
                    .collect::<Vec<c_int>>()
            })
        };

        let (cv, co) = run(&l.c);
        let (rv, ro) = run(&l.r);
        let labels: Vec<String> = script
            .iter()
            .map(|&(k, a, b)| format!("kind{k}({a}, {b})"))
            .collect();
        assert_rets_eq("interleaved", &labels, &cv, &rv);
        assert_stdout_eq("interleaved", &co, &ro);
    }
}

/// CONFIGS row 26 -- `main` with randomized decimal argument pairs.
#[test]
fn b18_main_randomized_decimal_args() {
    let mut rng = Rng::new(0x0B18);
    for _ in 0..512 {
        let a = rng.next_i32().to_string();
        let b = rng.next_i32().to_string();
        diff_exe(&[&a, &b]);
    }
}

/// CONFIGS row 27 -- `main` with edge-value decimal arguments.
#[test]
fn b19_main_edge_decimal_args() {
    let mut extra: Vec<String> = EDGES.iter().map(|v| v.to_string()).collect();
    extra.extend(
        [
            "2147483648",
            "-2147483649",
            "4294967295",
            "4294967296",
            "-4294967296",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    for i in 0..extra.len() {
        let a = &extra[i];
        let b = &extra[extra.len() - 1 - i];
        diff_exe(&[a, b]);
        diff_exe(&[b, a]);
    }
}

/// CONFIGS row 28 -- `main` with every `atoi` input *shape*.
#[test]
fn b20_main_atoi_input_shapes() {
    let shapes = [
        "0", "+0", "-0", "00", "007", "-007", "+7", " 12", "\t12", "\n12", "  -34  ",
        "\u{b}12", "\u{c}12", "\r12", "\u{b}\u{c}\r \t\n-9", "12\t", "12 ",
        "12abc", "abc", "abc12", "", " ", "-", "+", "--5", "++5", "3.9", "-3.9", "1e5",
        "0x10", "0X10", "010", "1 2", "2147483647", "-2147483648", "2147483648",
        "-2147483649", "9223372036854775807", "9223372036854775808",
        "-9223372036854775808", "-9223372036854775809",
        "99999999999999999999999999999999999999", "-99999999999999999999999999999999999999",
        "1_000", "0b101", "\u{7f}5", "5\u{7f}",
    ];
    for a in shapes.iter() {
        for b in shapes.iter() {
            diff_exe(&[a, b]);
        }
    }
}

// ===========================================================================
// PHASE C -- error-path differential tests (one per ERRORS.md row)
// ===========================================================================

fn read_all_fd(fd: c_int) -> Vec<u8> {
    use std::os::unix::io::FromRawFd;
    let mut f = unsafe { File::from_raw_fd(fd) };
    let mut v = Vec::new();
    f.read_to_end(&mut v).expect("read pipe");
    // `f` closes `fd` on drop.
    v
}

/// `execve` the given executable with a *completely empty* `argv`, so the
/// program starts with `argc == 0` (`argv[0]` is not a valid string). This is
/// the only way to reach `mdmain.c:30`'s `%s` with no program name.
fn run_exe_argc0(which: &str) -> Run {
    let prog = ensure_artifacts().join(which).join("driver");
    let cprog = std::ffi::CString::new(prog.to_str().unwrap()).unwrap();
    // SAFETY: the child only performs async-signal-safe calls before `execve`.
    unsafe {
        let mut so = [0 as c_int; 2];
        let mut se = [0 as c_int; 2];
        assert_eq!(libc::pipe(so.as_mut_ptr()), 0, "pipe");
        assert_eq!(libc::pipe(se.as_mut_ptr()), 0, "pipe");
        let pid = libc::fork();
        assert!(pid >= 0, "fork");
        if pid == 0 {
            libc::close(so[0]);
            libc::close(se[0]);
            libc::dup2(so[1], libc::STDOUT_FILENO);
            libc::dup2(se[1], libc::STDERR_FILENO);
            libc::close(so[1]);
            libc::close(se[1]);
            let argv: [*const c_char; 1] = [std::ptr::null()];
            let envp: [*const c_char; 1] = [std::ptr::null()];
            libc::execve(cprog.as_ptr(), argv.as_ptr(), envp.as_ptr());
            libc::_exit(127);
        }
        libc::close(so[1]);
        libc::close(se[1]);
        let stdout = read_all_fd(so[0]);
        let stderr = read_all_fd(se[0]);
        let mut status: c_int = 0;
        libc::waitpid(pid, &mut status, 0);
        let code = if libc::WIFEXITED(status) {
            Some(libc::WEXITSTATUS(status))
        } else {
            None
        };
        Run {
            stdout,
            stderr,
            code,
        }
    }
}

/// ERRORS row 1 -- `argc == 1`: usage on stderr, empty stdout, exit status 2.
#[test]
fn c01_argc_one() {
    let c = run_exe("cbin", &[]);
    let r = run_exe("rbin", &[]);
    assert_eq!(c.code, Some(2), "{} C must exit 2", cfg_tag());
    assert_eq!(r.code, Some(2), "{} Rust must exit 2", cfg_tag());
    assert!(c.stdout.is_empty(), "{} C stdout must be empty", cfg_tag());
    assert_stdout_eq("argc==1 stdout", &c.stdout, &r.stdout);
    assert_stdout_eq("argc==1 stderr", &c.stderr, &r.stderr);
    assert_eq!(c.stderr, b"usage: ./driver A B\n", "unexpected C usage text");
}

/// ERRORS row 2 -- `argc == 2`: same rejection, including an empty-string arg.
#[test]
fn c02_argc_two() {
    for only in ["1", "", " ", "abc", "-2147483648", "2147483648"] {
        let c = run_exe("cbin", &[only]);
        let r = run_exe("rbin", &[only]);
        assert_eq!(c.code, Some(2), "{} C argc==2 must exit 2", cfg_tag());
        assert_eq!(r.code, Some(2), "{} Rust argc==2 must exit 2", cfg_tag());
        assert_stdout_eq("argc==2 stdout", &c.stdout, &r.stdout);
        assert_stdout_eq("argc==2 stderr", &c.stderr, &r.stderr);
        assert_eq!(c.stderr, b"usage: ./driver A B\n");
    }
}

/// ERRORS row 3 -- `argc == 0` via `execve` with an empty `argv`.
#[test]
fn c03_argc_zero_via_execve() {
    let c = run_exe_argc0("cbin");
    let r = run_exe_argc0("rbin");
    assert_eq!(c.code, Some(2), "{} C argc==0 must exit 2", cfg_tag());
    assert_eq!(r.code, Some(2), "{} Rust argc==0 must exit 2", cfg_tag());
    assert_stdout_eq("argc==0 stdout", &c.stdout, &r.stdout);
    assert_stdout_eq("argc==0 stderr", &c.stderr, &r.stderr);
    assert!(
        c.stderr.starts_with(b"usage: ") && c.stderr.ends_with(b" A B\n"),
        "{} unexpected C usage text {:?}",
        cfg_tag(),
        String::from_utf8_lossy(&c.stderr)
    );
}

/// Assert `use_generated(n)` falls through `DISPATCH_REP`'s `default:` in both
/// implementations: identical stdout/return, and the return value is `INIT`.
#[track_caller]
fn assert_use_generated_default(ns: &[c_int]) {
    diff_un_batch("use_generated", ns);
    let l = libs();
    let cf = *un_fn(&l.c, "use_generated");
    let rf = *un_fn(&l.r, "use_generated");
    for &n in ns {
        let (cv, co) = capture(|| unsafe { cf(n) });
        let (rv, ro) = capture(|| unsafe { rf(n) });
        assert_eq!(
            cv, INIT,
            "{} C use_generated({n}) must return INIT_FOR(OP)={INIT}",
            cfg_tag()
        );
        assert_eq!(rv, INIT, "{} Rust use_generated({n}) must return {INIT}", cfg_tag());
        let want = format!("gen.acc={INIT}\n").into_bytes();
        assert_eq!(co, want, "{} C stdout for n={n}", cfg_tag());
        assert_eq!(ro, want, "{} Rust stdout for n={n}", cfg_tag());
    }
}

/// ERRORS row 4 -- `n == 7`, one past the last `case` of `DISPATCH_REP`.
#[test]
fn c04_use_generated_n_7() {
    assert_use_generated_default(&[7]);
}

/// ERRORS row 5 -- `n` well above the last `case`.
#[test]
fn c05_use_generated_n_above_switch() {
    assert_use_generated_default(&[8, 9, 10, 100, 255, 256, 1000, 65536, 1 << 30]);
}

/// ERRORS row 6 -- `n == INT_MAX`.
#[test]
fn c06_use_generated_n_int_max() {
    assert_use_generated_default(&[i32::MAX, i32::MAX - 1]);
}

/// ERRORS row 7 -- `n == -1`, one step below the first `case`.
#[test]
fn c07_use_generated_n_negative() {
    assert_use_generated_default(&[-1, -2, -7, -100, -65536]);
}

/// ERRORS row 8 -- `n == INT_MIN`.
#[test]
fn c08_use_generated_n_int_min() {
    assert_use_generated_default(&[i32::MIN, i32::MIN + 1]);
}

/// ERRORS row 9 -- 4000 randomized `n` outside `0..=6`.
#[test]
fn c09_use_generated_random_out_of_range() {
    let mut rng = Rng::new(0x0C09);
    let mut ns = Vec::with_capacity(4000);
    while ns.len() < 4000 {
        let n = rng.next_i32();
        if !(0..=6).contains(&n) {
            ns.push(n);
        }
    }
    diff_un_batch("use_generated", &ns);
    // and every one of them must yield INIT
    let l = libs();
    let cf = *un_fn(&l.c, "use_generated");
    let (cv, _) = capture(|| ns.iter().map(|&n| unsafe { cf(n) }).collect::<Vec<c_int>>());
    assert!(
        cv.iter().all(|&v| v == INIT),
        "{} some out-of-range n did not return INIT",
        cfg_tag()
    );
}

/// ERRORS row 10 -- `n == 0` reaches `case 0:` → `REP0`, an empty unrolled body,
/// which is observationally identical to the `default:` path.
#[test]
fn c10_use_generated_n_zero_equals_default() {
    diff_un_batch("use_generated", &[0]);
    let l = libs();
    for (tag, lib) in [("C", &l.c), ("Rust", &l.r)] {
        let f = *un_fn(lib, "use_generated");
        let (v0, o0) = capture(|| unsafe { f(0) });
        let (v7, o7) = capture(|| unsafe { f(7) });
        assert_eq!(
            (v0, &o0),
            (v7, &o7),
            "{} {tag}: use_generated(0) and use_generated(7) must be indistinguishable",
            cfg_tag()
        );
        assert_eq!(v0, INIT, "{} {tag}: use_generated(0) must be INIT", cfg_tag());
    }
}

/// ERRORS row 11 -- `atoi` on input that is not a number at all → 0, exit 0.
#[test]
fn c11_atoi_non_numeric() {
    let bad = ["abc", "-", "+", "", " ", "x1", ".", "/", ":", "\t", "\n", "e5", "NaN"];
    for a in bad.iter() {
        for b in bad.iter() {
            diff_exe(&[a, b]);
        }
        diff_exe(&[a, "0"]);
        diff_exe(&["0", a]);
        // ... and it really must behave like the argument were 0
        let bad_run = run_exe("cbin", &[a, "0"]);
        let zero_run = run_exe("cbin", &["0", "0"]);
        assert_eq!(
            bad_run.stdout, zero_run.stdout,
            "{} C atoi({a:?}) is expected to be 0",
            cfg_tag()
        );
        assert_eq!(bad_run.code, Some(0));
    }
}

/// ERRORS row 12 -- `atoi` stops at the first non-digit.
#[test]
fn c12_atoi_partial_numeric() {
    let cases = [
        ("12abc", "12"),
        ("3.9", "3"),
        ("1e5", "1"),
        ("0x10", "0"),
        ("007z", "7"),
        ("-5x", "-5"),
        ("+5x", "5"),
        ("1 2", "1"),
        ("1_000", "1"),
        ("0b101", "0"),
    ];
    for (raw, equiv) in cases.iter() {
        diff_exe(&[raw, "1"]);
        diff_exe(&["1", raw]);
        let a = run_exe("cbin", &[raw, "1"]);
        let b = run_exe("cbin", &[equiv, "1"]);
        assert_eq!(
            a.stdout, b.stdout,
            "{} C atoi({raw:?}) is expected to equal atoi({equiv:?})",
            cfg_tag()
        );
        assert_eq!(a.code, Some(0));
    }
}

/// ERRORS row 13 -- values outside `int` but inside `long` → `(int)` truncation.
#[test]
fn c13_atoi_int_overflow() {
    let cases = [
        ("2147483648", "-2147483648"),
        ("-2147483649", "2147483647"),
        ("4294967296", "0"),
        ("4294967295", "-1"),
        ("-4294967296", "0"),
        ("8589934592", "0"),
    ];
    for (raw, equiv) in cases.iter() {
        diff_exe(&[raw, "3"]);
        diff_exe(&["3", raw]);
        let a = run_exe("cbin", &[raw, "3"]);
        let b = run_exe("cbin", &[equiv, "3"]);
        assert_eq!(
            a.stdout, b.stdout,
            "{} C atoi({raw:?}) is expected to truncate to {equiv}",
            cfg_tag()
        );
        assert_eq!(a.code, Some(0));
    }
}

/// ERRORS row 14 -- values outside `long` → `strtol` saturation then truncation.
#[test]
fn c14_atoi_long_overflow() {
    let cases = [
        ("9223372036854775808", "-1"),   // LONG_MAX  -> (int)-1
        ("99999999999999999999", "-1"),  // saturates to LONG_MAX
        ("-9223372036854775809", "0"),   // LONG_MIN  -> (int)0
        ("-99999999999999999999", "0"),
        ("9223372036854775807", "-1"),   // LONG_MAX exactly
        ("-9223372036854775808", "0"),   // LONG_MIN exactly
        (
            "123456789012345678901234567890123456789012345678901234567890",
            "-1",
        ),
    ];
    for (raw, equiv) in cases.iter() {
        diff_exe(&[raw, "4"]);
        diff_exe(&["4", raw]);
        let a = run_exe("cbin", &[raw, "4"]);
        let b = run_exe("cbin", &[equiv, "4"]);
        assert_eq!(
            a.stdout, b.stdout,
            "{} C atoi({raw:?}) is expected to end up as {equiv}",
            cfg_tag()
        );
        assert_eq!(a.code, Some(0));
    }
}

/// ERRORS row 15 / CONFIGS row 29 -- `argc > 3`: surplus arguments are ignored.
#[test]
fn c15_extra_args_ignored() {
    diff_exe(&["3", "4", "5"]);
    diff_exe(&["3", "4", "5", "6"]);
    diff_exe(&["3", "4", "ignored", "", "-1"]);
    let three = run_exe("cbin", &["3", "4"]);
    let more = run_exe("cbin", &["3", "4", "5", "6"]);
    assert_eq!(
        three.stdout, more.stdout,
        "{} C must ignore argv[3..]",
        cfg_tag()
    );
    assert_eq!(more.code, Some(0));
}

/// ERRORS row 16 -- build-time rejection of `REPEAT` outside `0..=7`.
/// C: `CHOOSE_REP(8)` pastes `REP8`, which was never defined, so the
/// translation unit does not compile. Rust: there is no Cargo feature `8`, so
/// `cargo` refuses the build.
#[test]
fn c16_build_time_repeat_out_of_range() {
    for r in ["8", "9", "42", "100"] {
        let out = Command::new("gcc")
            .args(["-fsyntax-only", "-DOP=add"])
            .arg(format!("-DREPEAT={r}"))
            .arg(crate_root().join("c_src/src/mdcore.c"))
            .output()
            .expect("run gcc");
        assert!(
            !out.status.success(),
            "C unexpectedly accepted -DREPEAT={r}"
        );
        let msg = String::from_utf8_lossy(&out.stderr);
        assert!(
            msg.contains(&format!("REP{r}")) || msg.contains("undeclared"),
            "unexpected gcc diagnostic for -DREPEAT={r}: {msg}"
        );

        assert!(
            !cargo_accepts_feature(r),
            "cargo unexpectedly accepted --features {r}"
        );
    }
    // ...while every value the C accepts is also a Cargo feature.
    for r in ["0", "1", "2", "3", "4", "5", "6", "7"] {
        let out = Command::new("gcc")
            .args(["-fsyntax-only", "-DOP=add"])
            .arg(format!("-DREPEAT={r}"))
            .arg(crate_root().join("c_src/src/mdcore.c"))
            .output()
            .expect("run gcc");
        assert!(out.status.success(), "C rejected the valid -DREPEAT={r}");
        assert!(
            feature_declared(r) && feature_declared(&format!("repeat_{r}")),
            "Cargo.toml is missing the feature {r}"
        );
    }
}

/// ERRORS row 17 -- build-time rejection of an `OP` outside `{add,sub,mul}`.
#[test]
fn c17_build_time_bad_op() {
    for op in ["div", "mod", "xor", "foo"] {
        let out = Command::new("gcc")
            .args(["-fsyntax-only"])
            .arg(format!("-DOP={op}"))
            .args(["-DREPEAT=5"])
            .arg(crate_root().join("c_src/src/mdcore.c"))
            .output()
            .expect("run gcc");
        assert!(!out.status.success(), "C unexpectedly accepted -DOP={op}");
        let msg = String::from_utf8_lossy(&out.stderr);
        assert!(
            msg.contains(&format!("INIT_{op}")) || msg.contains("undeclared"),
            "unexpected gcc diagnostic for -DOP={op}: {msg}"
        );
        assert!(
            !cargo_accepts_feature(op),
            "cargo unexpectedly accepted --features {op}"
        );
    }
    for op in ["add", "sub", "mul"] {
        assert!(
            feature_declared(op),
            "Cargo.toml is missing the feature {op}"
        );
    }
}

/// Does `Cargo.toml` declare `name` in `[features]`?
fn feature_declared(name: &str) -> bool {
    let toml = std::fs::read_to_string(crate_root().join("Cargo.toml")).expect("Cargo.toml");
    let feats = toml
        .split("[features]")
        .nth(1)
        .expect("[features] section")
        .split("\n[")
        .next()
        .unwrap();
    feats.lines().any(|l| {
        let l = l.trim();
        let key = l.split('=').next().unwrap_or("").trim().trim_matches('"');
        key == name
    })
}

/// Run `cargo check` for a single feature in a scratch target dir (so the outer
/// `cargo test`'s lock is untouched) and report whether it was accepted.
fn cargo_accepts_feature(name: &str) -> bool {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let tgt = std::env::temp_dir().join(format!("md_featcheck_{}", std::process::id()));
    Command::new(cargo)
        .args(["check", "--offline", "--quiet", "--no-default-features", "--features", name])
        .current_dir(crate_root())
        .env("CARGO_TARGET_DIR", &tgt)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// ERRORS row 19 -- the `op_*` exports contain no checks at all: signed overflow
/// wraps rather than being rejected, in both implementations.
#[test]
fn c19_op_overflow_no_rejection() {
    let l = libs();
    let overflow_pairs: Vec<(c_int, c_int)> = vec![
        (i32::MAX, 1),
        (1, i32::MAX),
        (i32::MAX, i32::MAX),
        (i32::MIN, -1),
        (-1, i32::MIN),
        (i32::MIN, i32::MIN),
        (i32::MIN, 1),
        (46341, 46341),
        (-46341, 46341),
        (65536, 65536),
        (1 << 30, 4),
        (i32::MAX, -1),
        (i32::MIN, -2147483647),
    ];
    for (name, model) in [
        ("op_add", 0u8),
        ("op_sub", 1u8),
        ("op_mul", 2u8),
    ] {
        let cf = *bin_fn(&l.c, name);
        let rf = *bin_fn(&l.r, name);
        for &(a, b) in overflow_pairs.iter() {
            let cv = unsafe { cf(a, b) };
            let rv = unsafe { rf(a, b) };
            let want = match model {
                0 => a.wrapping_add(b),
                1 => a.wrapping_sub(b),
                _ => a.wrapping_mul(b),
            };
            assert_eq!(cv, rv, "{} {name}({a},{b})", cfg_tag());
            assert_eq!(
                cv, want,
                "{} C {name}({a},{b}) is expected to wrap two's-complement",
                cfg_tag()
            );
        }
    }
}

/// ERRORS row 20 -- `helper_call`'s `r + acc` overflow is likewise unchecked.
#[test]
fn c20_helper_call_return_overflow() {
    let extreme: Vec<(c_int, c_int)> = vec![
        (i32::MAX, 0),
        (i32::MAX, i32::MAX),
        (i32::MIN, 0),
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX),
        (i32::MAX - 20, 0),
        (i32::MIN + 20, 0),
        (46341, 46341),
        (65536, 65536),
    ];
    diff_bin_batch("helper_call", &extreme);
    diff_bin_batch("helper_ptr", &extreme);
}

// ERRORS row 18 -- storing NULL into `G_OP` and calling through it is UB and
// crashes both implementations, so it is deliberately not executed here; the
// checkable half (that `G_OP` is writable in both) is
// `b14_g_op_writable_then_call_through`.

/// Byte-exact `argv` handling: a program path and arguments need not be valid
/// UTF-8. C copies `argv[0]` verbatim into the usage message and `atoi` reads
/// raw bytes, so a lossy `String` conversion anywhere would diverge.
///
/// (Generic FFI-boundary robustness, beyond the ERRORS.md rows.)
#[test]
fn c21_non_utf8_argv() {
    use std::ffi::{OsStr, OsString};
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let weird: Vec<OsString> = vec![
        OsString::from_vec(vec![0xff, b'1', b'2']),
        OsString::from_vec(vec![b'1', 0xff, b'2']),
        OsString::from_vec(vec![b'-', b'5', 0x80]),
        OsString::from_vec(vec![0xc3]),
        OsString::from_vec(vec![0xed, 0xa0, 0x80]),
    ];

    // (a) non-UTF-8 A / B arguments
    for a in weird.iter() {
        for b in weird.iter() {
            let mut runs = Vec::new();
            for which in ["cbin", "rbin"] {
                let dir = ensure_artifacts().join(which);
                let out = Command::new("./driver")
                    .arg(a)
                    .arg(b)
                    .current_dir(&dir)
                    .output()
                    .expect("spawn");
                runs.push(out);
            }
            let ctx = format!("non-utf8 args {:?} {:?}", a.as_bytes(), b.as_bytes());
            assert_stdout_eq(&ctx, &runs[0].stdout, &runs[1].stdout);
            assert_stdout_eq(&ctx, &runs[0].stderr, &runs[1].stderr);
            assert_eq!(runs[0].status.code(), runs[1].status.code(), "{ctx}");
        }
    }

    // (b) non-UTF-8 argv[0]: hard-link the executable under an invalid-UTF-8
    //     name and trigger the `argc < 3` usage path, which prints argv[0].
    let linkname = OsString::from_vec(vec![b'.', b'/', b'd', b'r', 0xff, 0xfe, b'v']);
    let mut outs = Vec::new();
    for which in ["cbin", "rbin"] {
        let dir = ensure_artifacts().join(which);
        let target = dir.join(OsStr::from_bytes(&linkname.as_bytes()[2..]));
        let _ = std::fs::remove_file(&target);
        std::fs::hard_link(dir.join("driver"), &target).expect("hard_link");
        for args in [vec![], vec!["1"]] {
            let out = Command::new(&linkname)
                .args(&args)
                .current_dir(&dir)
                .output()
                .expect("spawn non-utf8 argv0");
            outs.push((args.len(), out));
        }
        let _ = std::fs::remove_file(&target);
    }
    let half = outs.len() / 2;
    for i in 0..half {
        let (n, c) = &outs[i];
        let (_, r) = &outs[i + half];
        let ctx = format!("non-utf8 argv[0], {} extra arg(s)", n);
        assert_stdout_eq(&ctx, &c.stdout, &r.stdout);
        assert_stdout_eq(&format!("{ctx} (stderr)"), &c.stderr, &r.stderr);
        assert_eq!(c.status.code(), r.status.code(), "{ctx}");
        assert_eq!(
            c.stderr,
            [b"usage: ".as_slice(), linkname.as_bytes(), b" A B\n"].concat(),
            "{} C usage message must contain argv[0] verbatim",
            cfg_tag()
        );
    }
}

/// stdout write failures: `mdcore.c`/`mdmain.c` discard `printf`'s return value,
/// so an unwritable stdout must not change the result or the exit status.
/// (Generic FFI/OS-boundary robustness, beyond the ERRORS.md rows.)
#[test]
fn c22_unwritable_stdout() {
    use std::fs::OpenOptions;

    // (a) the executables, with stdout on /dev/full (every write fails ENOSPC)
    //     and with stdout on /dev/null.
    for sink in ["/dev/full", "/dev/null"] {
        let mut res = Vec::new();
        for which in ["cbin", "rbin"] {
            let dir = ensure_artifacts().join(which);
            let f = OpenOptions::new().write(true).open(sink).expect(sink);
            let out = Command::new("./driver")
                .args(["3", "4"])
                .current_dir(&dir)
                .stdout(f)
                .output()
                .expect("spawn");
            res.push(out);
        }
        let ctx = format!("stdout={sink}");
        assert_eq!(
            res[0].status.code(),
            res[1].status.code(),
            "{} {ctx}: exit status differs (C={:?} Rust={:?})",
            cfg_tag(),
            res[0].status.code(),
            res[1].status.code()
        );
        assert_stdout_eq(&format!("{ctx} (stderr)"), &res[0].stderr, &res[1].stderr);
    }

    // (b) the shared libraries, with fd 1 pointing at /dev/full.
    let l = libs();
    let run_on_full = |lib: &Library| -> Vec<c_int> {
        let hc = *bin_fn(lib, "helper_call");
        let hp = *bin_fn(lib, "helper_ptr");
        let ug = *un_fn(lib, "use_generated");
        let f = OpenOptions::new().write(true).open("/dev/full").unwrap();
        // SAFETY: fd 1 is restored before returning.
        unsafe {
            std::io::stdout().flush().ok();
            libc::fflush(std::ptr::null_mut());
            let saved = libc::dup(libc::STDOUT_FILENO);
            libc::dup2(f.as_raw_fd(), libc::STDOUT_FILENO);
            let v = vec![hc(3, 4), hp(3, 4), ug(REPEAT), hc(i32::MIN, 7)];
            libc::fflush(std::ptr::null_mut());
            libc::dup2(saved, libc::STDOUT_FILENO);
            libc::close(saved);
            v
        }
    };
    let cv = run_on_full(&l.c);
    let rv = run_on_full(&l.r);
    assert_eq!(
        cv, rv,
        "{} return values must be unaffected by a failing stdout",
        cfg_tag()
    );
}
