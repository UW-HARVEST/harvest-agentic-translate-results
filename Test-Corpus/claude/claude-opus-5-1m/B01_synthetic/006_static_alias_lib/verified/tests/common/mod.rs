// Shared differential-test harness for the StaticAlias library.
//
// Loads BOTH shared objects with libloading and calls them ONLY through their
// exported C symbols:
//   * C    : c_src/build/libStaticAlias.so
//   * Rust : target/<profile>/libStaticAlias.so   (the #[no_mangle] cdylib)
//
// No Rust function is ever called directly, so the `extern "C"` export wrappers
// are part of what is under test.
//
// ---------------------------------------------------------------------------
// The hidden static
// ---------------------------------------------------------------------------
// `static_alias` owns a function-local `static int inner = 1;`. It lives for the
// whole process, has no setter, and each .so has its own private copy. The
// harness therefore guarantees ONE invariant:
//
//     every operation issues the *identical* call sequence to both libraries
//
// so `C.inner == Rust.inner` holds at all times, regardless of the order the
// #[test]s happen to run in (they serialize on a global mutex). Both copies
// start at 1, so they start in agreement.
//
// `probe()` reads `inner` without mutating it, and `set_inner()` drives it to an
// arbitrary target -- both using nothing but the public API. See their comments.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, CString};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type StaticAliasFn = unsafe extern "C" fn(*mut c_int) -> *mut c_int;
pub type DriverFn = unsafe extern "C" fn(c_int, c_int);

// ---------------------------------------------------------------------------
// Observables
// ---------------------------------------------------------------------------

/// Which object the returned pointer aliases. Pointer *values* are not
/// comparable between the two libraries, but their aliasing *class* is, and it
/// is the primary observable of this API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cls {
    /// `ret == outer` — the else branch returned the caller's own pointer.
    Outer,
    /// `ret == &inner` — the then branch returned the internal static.
    Inner,
    /// Neither: a translation bug (e.g. returning a pointer to a fresh copy).
    Other,
}

/// Everything observable about one `static_alias` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Obs {
    /// `*ret`
    pub ret_val: c_int,
    /// the caller's buffer after the call (the else branch mutates it)
    pub buf_after: c_int,
    /// aliasing class of the returned pointer
    pub cls: Cls,
}

/// One step of a pointer-chained sequence (what `driver` does internally).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainStep {
    pub val: c_int,
    pub cls: Cls,
}

// ---------------------------------------------------------------------------
// Loaded library
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    pub static_alias: StaticAliasFn,
    pub driver: DriverFn,
    /// Address of this library's private `inner`, learned through the API.
    /// `usize` (not a pointer) so `Lib` stays `Send`.
    pub inner_addr: usize,
}

fn load_lib(name: &'static str, path: &PathBuf) -> Lib {
    assert!(
        path.exists(),
        "{} shared object not found at {}\n\
         (build the C lib with: cd c_src && mkdir -p build && cd build && \
          cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)",
        name,
        path.display()
    );
    // Leaked on purpose: the library must stay mapped for the whole process,
    // because its `inner` static is the state under test.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(path).unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()))
    }));

    let sa: Symbol<StaticAliasFn> = unsafe {
        lib.get(b"static_alias\0")
            .unwrap_or_else(|e| panic!("{name}: missing exported symbol `static_alias`: {e}"))
    };
    let dr: Symbol<DriverFn> = unsafe {
        lib.get(b"driver\0")
            .unwrap_or_else(|e| panic!("{name}: missing exported symbol `driver`: {e}"))
    };

    Lib {
        name,
        static_alias: *sa,
        driver: *dr,
        inner_addr: 0, // filled in by discover_inner_addr
    }
}

// ---------------------------------------------------------------------------
// Primitives on a single library
// ---------------------------------------------------------------------------

fn classify(lib: &Lib, ret: *mut c_int, outer: *mut c_int) -> Cls {
    if ret == outer {
        Cls::Outer
    } else if ret as usize == lib.inner_addr {
        Cls::Inner
    } else {
        Cls::Other
    }
}

/// One `static_alias` call on a caller-owned (distinct) buffer.
unsafe fn call_one(lib: &Lib, val: c_int) -> Obs {
    let mut buf: c_int = val;
    let p: *mut c_int = &mut buf;
    let ret = (lib.static_alias)(p);
    assert!(!ret.is_null(), "{}: static_alias returned NULL", lib.name);
    Obs {
        ret_val: *ret,
        buf_after: buf,
        cls: classify(lib, ret, p),
    }
}

/// One `static_alias` call with `outer == &inner` (the aliased case).
///
/// Here `outer == &inner`, so `ret == outer` and `ret == &inner` are the SAME
/// test; `classify` would ambiguously report `Outer`. Check `Inner` first, since
/// `*outer >= inner` is `inner >= inner` — always true — so the then branch is
/// always taken and `&inner` is the correct class.
unsafe fn call_aliased_one(lib: &Lib) -> Obs {
    let p = lib.inner_addr as *mut c_int;
    let before = *p;
    let ret = (lib.static_alias)(p);
    Obs {
        ret_val: *ret,
        buf_after: before, // reported for symmetry; `p` *is* inner here
        cls: if ret as usize == lib.inner_addr {
            Cls::Inner
        } else {
            Cls::Other
        },
    }
}

/// NON-MUTATING read of `inner`.
///
/// Passing `INT_MIN` takes the else branch whenever `inner > INT_MIN`, and the
/// else branch leaves `inner` alone while writing `INT_MIN + inner` into our own
/// buffer -- so `inner == buf - INT_MIN`.
///
/// If `inner` happens to be exactly `INT_MIN`, `INT_MIN >= INT_MIN` takes the
/// then branch instead, which sets `inner = INT_MIN + INT_MIN = 0`; that is
/// detectable from the returned pointer's class, and we report the new value.
unsafe fn probe_one(lib: &Lib) -> c_int {
    let mut buf: c_int = c_int::MIN;
    let p: *mut c_int = &mut buf;
    let ret = (lib.static_alias)(p);
    if ret == p {
        buf.wrapping_sub(c_int::MIN)
    } else {
        0
    }
}

/// Drive `inner` to an arbitrary target using only the public API.
///
/// 1. Aliased then-branch calls double `inner` (`inner += inner`); after at most
///    32 doublings any value has wrapped to 0.
/// 2. From 0: one call with `2^30` sets `inner = 2^30`, then one aliased call
///    wraps it to `INT_MIN`.
/// 3. From `INT_MIN`, `x >= INT_MIN` is true for every `x`, so a single call
///    with `target - INT_MIN` lands exactly on `target`.
unsafe fn set_inner_one(lib: &Lib, target: c_int) {
    let mut cur = probe_one(lib);
    if cur == target {
        return;
    }
    let ip = lib.inner_addr as *mut c_int;

    let mut guard = 0;
    while cur != 0 {
        (lib.static_alias)(ip);
        cur = cur.wrapping_add(cur);
        guard += 1;
        assert!(guard <= 40, "{}: doubling to 0 did not converge", lib.name);
    }
    if target == 0 {
        return;
    }
    let mut b: c_int = 1 << 30;
    (lib.static_alias)(&mut b as *mut c_int); // inner = 2^30
    (lib.static_alias)(ip); // inner = 2^31 -> INT_MIN
    let mut v: c_int = target.wrapping_sub(c_int::MIN);
    (lib.static_alias)(&mut v as *mut c_int); // inner = target
}

/// Replicates `driver`'s pointer chaining without the printing, so the
/// value/aliasing sequence can be compared independently of stdout.
unsafe fn chain_one(lib: &Lib, initial: c_int, steps: usize) -> Vec<ChainStep> {
    let mut buf: c_int = initial;
    let bp: *mut c_int = &mut buf;
    let mut p: *mut c_int = bp;
    let mut out = Vec::with_capacity(steps);
    for _ in 0..steps {
        p = (lib.static_alias)(p);
        out.push(ChainStep {
            val: *p,
            cls: classify(lib, p, bp),
        });
    }
    out
}

// ---------------------------------------------------------------------------
// stdout capture (for `driver`'s printf)
// ---------------------------------------------------------------------------

/// Run `f` with fd 1 redirected to a temp file and return the exact bytes it
/// wrote. Both .so's import the same libc `printf`, so both write through the
/// same `stdout` FILE; `fflush(NULL)` before and after keeps the buffered bytes
/// attributed to the right call.
///
/// fd 1 is process-wide, so this REQUIRES `--test-threads=1`: otherwise the test
/// runner's own progress output ("test foo ... ") can be flushed into the
/// redirect window by another thread and corrupt the capture. Both buffers
/// (libc's and Rust std's) are drained first, and the result is checked for
/// foreign bytes so that a violation fails loudly instead of silently.
unsafe fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom, Write};

    let _ = std::io::stdout().flush();
    libc::fflush(std::ptr::null_mut());

    let saved = libc::dup(1);
    assert!(saved >= 0, "dup(1) failed");

    let mut path = std::env::temp_dir();
    path.push(format!(
        "staticalias_cap_{}_{:p}.txt",
        std::process::id(),
        &saved as *const _
    ));
    let cpath = CString::new(path.to_str().unwrap()).unwrap();

    let fd = libc::open(
        cpath.as_ptr(),
        libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
        0o600,
    );
    assert!(fd >= 0, "open({}) failed", path.display());

    assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");

    f();

    libc::fflush(std::ptr::null_mut());
    libc::dup2(saved, 1);
    libc::close(saved);

    libc::lseek(fd, 0, libc::SEEK_SET);
    let mut file = <std::fs::File as std::os::fd::FromRawFd>::from_raw_fd(fd);
    let _ = file.seek(SeekFrom::Start(0));
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).expect("read capture file");
    drop(file); // closes fd
    let _ = std::fs::remove_file(&path);

    // `driver` can only ever emit `printf("%d\n", ...)`, i.e. bytes in
    // [0-9], '-' and '\n'. Anything else means a foreign writer got into the
    // redirect window.
    if let Some(bad) = buf
        .iter()
        .find(|b| !(b.is_ascii_digit() || **b == b'-' || **b == b'\n'))
    {
        panic!(
            "stdout capture was polluted by another writer (found byte {bad:?} = {:?}).\n\
             Run the differential suite single-threaded:\n\
             \x20   cargo test -- --test-threads=1\n\
             captured: {:?}",
            *bad as char,
            String::from_utf8_lossy(&buf)
        );
    }
    buf
}

// ---------------------------------------------------------------------------
// The dual harness
// ---------------------------------------------------------------------------

pub struct Harness {
    pub c: Lib,
    pub r: Lib,
}

fn target_dir() -> PathBuf {
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target dir")
        .to_path_buf()
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C `.so`. `STATICALIAS_C_SO` overrides it, so the same suite can
/// be re-run against a C library built at a different optimization level (the
/// overflow rows depend on signed-overflow behaviour, which is UB in C, so it is
/// worth confirming the ground truth is stable across -O levels).
fn c_so_path() -> PathBuf {
    match std::env::var("STATICALIAS_C_SO") {
        Ok(p) if !p.is_empty() => PathBuf::from(p),
        _ => manifest_dir().join("c_src/build/libStaticAlias.so"),
    }
}

fn build() -> Harness {
    let c_path = c_so_path();
    let r_path = target_dir().join("libStaticAlias.so");

    let mut h = Harness {
        c: load_lib("C", &c_path),
        r: load_lib("RUST", &r_path),
    };

    // Learn each library's `&inner`, and check the two agree on the initial
    // state (both must start at `inner == 1`).
    unsafe {
        let pc = probe_one(&h.c);
        let pr = probe_one(&h.r);
        assert_eq!(pc, 1, "C: initial inner should be 1");
        assert_eq!(pr, pc, "RUST: initial inner differs from C");

        // A then-branch call at the `>=` equality boundary reveals `&inner`.
        for lib in [&mut h.c, &mut h.r] {
            let mut buf: c_int = pc;
            let p: *mut c_int = &mut buf;
            let ret = (lib.static_alias)(p);
            assert_ne!(
                ret, p,
                "{}: expected the then branch to return &inner",
                lib.name
            );
            assert_eq!(*ret, pc.wrapping_add(pc), "{}: inner += *outer", lib.name);
            lib.inner_addr = ret as usize;
        }

        // &inner must be stable across calls.
        for lib in [&h.c, &h.r] {
            let cur = probe_one(lib);
            let mut buf: c_int = cur;
            let p: *mut c_int = &mut buf;
            let ret = (lib.static_alias)(p);
            assert_eq!(
                ret as usize, lib.inner_addr,
                "{}: &inner is not stable across calls",
                lib.name
            );
        }
    }
    h
}

/// Load ONE library in isolation, without the `inner_addr` discovery calls.
///
/// Used by the crash-path tests, which run in a throwaway child process and must
/// not perturb any state before faulting. `which` is "c" or "rust".
pub fn load_single(which: &str) -> Lib {
    match which {
        "c" => load_lib("C", &c_so_path()),
        "rust" => load_lib("RUST", &target_dir().join("libStaticAlias.so")),
        other => panic!("unknown library selector {other:?}"),
    }
}

static HARNESS: OnceLock<Mutex<Harness>> = OnceLock::new();

/// Acquire the harness. Serializes tests so that the shared `inner` state is
/// only ever advanced by matched C+Rust pairs.
pub fn harness() -> MutexGuard<'static, Harness> {
    match HARNESS.get_or_init(|| Mutex::new(build())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(), // a prior failing test poisoned it; keep going
    }
}

impl Harness {
    /// Non-mutating read of `inner`; asserts both libraries agree.
    pub fn probe(&mut self) -> c_int {
        let (pc, pr) = unsafe { (probe_one(&self.c), probe_one(&self.r)) };
        assert_eq!(pc, pr, "inner diverged: C={pc} RUST={pr}");
        pc
    }

    /// Drive both libraries' `inner` to `target`; asserts both landed there.
    ///
    /// `target == INT_MIN` is the one state that cannot be verified: every
    /// possible `*outer` satisfies `*outer >= INT_MIN`, so ANY observation takes
    /// the then branch and changes `inner`. For that target the construction is
    /// left unverified here — the caller's first `sa_np()` is the observation,
    /// and it is still fully differential (if only one library had reached
    /// `INT_MIN`, their results would disagree).
    pub fn set_inner(&mut self, target: c_int) {
        unsafe {
            set_inner_one(&self.c, target);
            set_inner_one(&self.r, target);
            if target != c_int::MIN {
                let pc = probe_one(&self.c);
                let pr = probe_one(&self.r);
                assert_eq!(pc, target, "C: set_inner({target}) landed on {pc}");
                assert_eq!(pr, target, "RUST: set_inner({target}) landed on {pr}");
            }
        }
    }

    /// `static_alias(&val)` on a distinct caller-owned buffer, both libraries.
    ///
    /// The pre-call probe goes through `self.probe()` so it hits BOTH libraries:
    /// probing only one would desync the two `inner` copies whenever
    /// `inner == INT_MIN` (where a probe takes the then branch and mutates).
    #[track_caller]
    pub fn sa(&mut self, val: c_int) -> Obs {
        let inner_before = self.probe();
        let (oc, or) = unsafe { (call_one(&self.c, val), call_one(&self.r, val)) };
        assert_eq!(
            oc, or,
            "static_alias(&{val}) diverged (inner was {inner_before})\n  C   = {oc:?}\n  RUST= {or:?}"
        );
        oc
    }

    /// Same, but without the pre-call probe (use when `inner == INT_MIN`, where
    /// probing would itself take the then branch and change `inner`).
    #[track_caller]
    pub fn sa_np(&mut self, val: c_int) -> Obs {
        let (oc, or) = unsafe { (call_one(&self.c, val), call_one(&self.r, val)) };
        assert_eq!(
            oc, or,
            "static_alias(&{val}) diverged\n  C   = {oc:?}\n  RUST= {or:?}"
        );
        oc
    }

    /// `static_alias(&inner)` — the aliased call, both libraries.
    #[track_caller]
    pub fn sa_aliased(&mut self) -> Obs {
        let (oc, or) = unsafe { (call_aliased_one(&self.c), call_aliased_one(&self.r)) };
        assert_eq!(
            oc, or,
            "aliased static_alias(&inner) diverged\n  C   = {oc:?}\n  RUST= {or:?}"
        );
        oc
    }

    /// Feed the returned pointer back in `steps` times, both libraries.
    #[track_caller]
    pub fn chain(&mut self, initial: c_int, steps: usize) -> Vec<ChainStep> {
        let (sc, sr) = unsafe {
            (
                chain_one(&self.c, initial, steps),
                chain_one(&self.r, initial, steps),
            )
        };
        if sc != sr {
            let at = sc
                .iter()
                .zip(sr.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            panic!(
                "chain(initial={initial}, steps={steps}) diverged at step {at}\n  C   = {:?}\n  RUST= {:?}",
                &sc[at..sc.len().min(at + 4)],
                &sr[at..sr.len().min(at + 4)]
            );
        }
        sc
    }

    /// `driver(initial, iterations)` on both libraries, comparing the exact
    /// bytes each wrote to stdout.
    #[track_caller]
    pub fn driver(&mut self, initial: c_int, iterations: c_int) -> Vec<u8> {
        let bc = unsafe { capture(|| (self.c.driver)(initial, iterations)) };
        let br = unsafe { capture(|| (self.r.driver)(initial, iterations)) };
        assert_eq!(
            bc,
            br,
            "driver({initial}, {iterations}) stdout differs\n  C   = {:?}\n  RUST= {:?}",
            String::from_utf8_lossy(&bc),
            String::from_utf8_lossy(&br)
        );
        bc
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5A17_A11A5;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform over the full 32-bit space (every bit pattern is a valid `int`).
    pub fn i32_any(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }
    /// Inclusive range.
    pub fn in_range(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    pub fn i32_in(&mut self, lo: c_int, hi: c_int) -> c_int {
        self.in_range(lo as i64, hi as i64) as c_int
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Independent model of the C semantics
// ---------------------------------------------------------------------------

/// Hand-derived model of `driver`'s inner loop, straight from the C:
///
/// ```c
/// int *running_sum = &initial_value;          //  starts at the caller's local
/// for (i = 0; i < iterations; i++) {
///   running_sum = static_alias(running_sum);  //  may switch to &inner
///   printf("%d\n", *running_sum);
/// }
/// ```
///
/// Returns the per-step (value, aliasing class) sequence and the final `inner`.
/// Once `running_sum == &inner`, `*outer >= inner` is `inner >= inner` — always
/// true — so the chain can never switch back to the caller's buffer.
///
/// This is a THIRD opinion: rows that use it must agree C == Rust == model.
pub fn model_chain(inner0: c_int, initial: c_int, steps: usize) -> (Vec<ChainStep>, c_int) {
    let mut inner = inner0;
    let mut buf = initial;
    let mut at_inner = false;
    let mut out = Vec::with_capacity(steps);
    for _ in 0..steps {
        let x = if at_inner { inner } else { buf };
        if x >= inner {
            inner = inner.wrapping_add(x); // inner += *outer
            at_inner = true;
            out.push(ChainStep {
                val: inner,
                cls: Cls::Inner,
            });
        } else {
            buf = x.wrapping_add(inner); // *outer += inner
            at_inner = false;
            out.push(ChainStep {
                val: buf,
                cls: Cls::Outer,
            });
        }
    }
    (out, inner)
}

/// The exact bytes `driver(initial, iterations)` must print, per the model.
pub fn model_driver_bytes(inner0: c_int, initial: c_int, iterations: c_int) -> Vec<u8> {
    let steps = if iterations > 0 { iterations as usize } else { 0 };
    let (seq, _) = model_chain(inner0, initial, steps);
    let vals: Vec<c_int> = seq.iter().map(|s| s.val).collect();
    expect_lines(&vals)
}

/// Formats what `printf("%d\n", v)` must produce, for byte-exact assertions
/// against the captured output.
pub fn expect_lines(vals: &[c_int]) -> Vec<u8> {
    let mut s = String::new();
    for v in vals {
        s.push_str(&format!("{v}\n"));
    }
    s.into_bytes()
}
