//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded through `libloading` (`dlopen`) and every call goes
//! through the exported `extern "C"` symbols, exactly as an external consumer
//! would call them. No Rust function is ever called directly.

#![allow(dead_code)]

use libloading::Library;
use std::cell::Cell;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type StaticAliasFn = unsafe extern "C" fn(*mut c_int) -> *mut c_int;
pub type DriverFn = unsafe extern "C" fn(c_int, c_int);

const C_SO_DEFAULT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../c_src/build/libStaticAlias.so");
const RUST_SO_DEFAULT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/target/release/libStaticAlias.so");

/// Path to the C `.so` under test (`STATICALIAS_C_SO` overrides the default).
pub fn c_so() -> &'static str {
    static P: OnceLock<String> = OnceLock::new();
    P.get_or_init(|| {
        std::env::var("STATICALIAS_C_SO").unwrap_or_else(|_| C_SO_DEFAULT.to_string())
    })
}

/// Path to the Rust `.so` under test (`STATICALIAS_RUST_SO` overrides the
/// default). Used to run the whole differential suite against more than one Rust
/// build configuration, e.g. the debug profile, which has overflow checks on.
pub fn rust_so() -> &'static str {
    static P: OnceLock<String> = OnceLock::new();
    P.get_or_init(|| {
        std::env::var("STATICALIAS_RUST_SO").unwrap_or_else(|_| RUST_SO_DEFAULT.to_string())
    })
}

/// The `static int inner` inside each `.so` is process-global, and `dup2` on fd 1
/// (used for stdout capture) is process-global too. Every row therefore takes
/// this gate so rows never run concurrently, no matter how the test harness is
/// invoked.
pub fn gate() -> MutexGuard<'static, ()> {
    static G: OnceLock<Mutex<()>> = OnceLock::new();
    match G.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Observation of one `static_alias` call
// ---------------------------------------------------------------------------

/// Everything observable about a single `static_alias` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Obs {
    /// Returned pointer is bit-identical to the `outer` argument (axis P2).
    pub ret_is_arg: bool,
    /// Returned pointer is the library-internal `&inner` (axis P1).
    pub ret_is_internal: bool,
    /// `*ret` after the call.
    pub ret_val: i32,
    /// The caller-visible object behind `outer` after the call.
    pub arg_val: i32,
}

// ---------------------------------------------------------------------------
// A freshly `dlopen`ed library
// ---------------------------------------------------------------------------

pub struct Lib {
    _lib: Library,
    pub sa: StaticAliasFn,
    pub dr: DriverFn,
    /// Address of the library's `inner`, learned the first time it is returned.
    internal: Cell<*mut c_int>,
    pub name: &'static str,
}

impl Lib {
    pub fn open(path: &str, name: &'static str) -> Lib {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({path}) failed: {e}"));
        let sa: StaticAliasFn = unsafe {
            *lib.get(b"static_alias\0")
                .expect("symbol `static_alias` missing")
        };
        let dr: DriverFn = unsafe { *lib.get(b"driver\0").expect("symbol `driver` missing") };
        Lib {
            _lib: lib,
            sa,
            dr,
            internal: Cell::new(std::ptr::null_mut()),
            name,
        }
    }

    /// Raw call; records the internal `&inner` address when it is revealed.
    /// Returns the observation together with the raw returned pointer.
    pub fn call_raw2(&self, arg: *mut c_int) -> (Obs, *mut c_int) {
        let ret = unsafe { (self.sa)(arg) };
        assert!(!ret.is_null(), "{}: static_alias returned NULL", self.name);
        let ret_is_arg = ret == arg;
        if !ret_is_arg {
            // `ret` must be `&inner`; it has to be stable across the library's
            // whole lifetime.
            let known = self.internal.get();
            if known.is_null() {
                self.internal.set(ret);
            } else {
                assert_eq!(known, ret, "{}: &inner moved between calls", self.name);
            }
        }
        let ret_is_internal = ret == self.internal.get();
        let obs = Obs {
            ret_is_arg,
            ret_is_internal,
            // `read_unaligned` so the *harness* never trips Rust's debug
            // alignment check when deliberately testing misaligned arguments.
            ret_val: unsafe { ret.read_unaligned() },
            arg_val: unsafe { arg.read_unaligned() },
        };
        (obs, ret)
    }

    pub fn call_raw(&self, arg: *mut c_int) -> Obs {
        self.call_raw2(arg).0
    }

    /// `static_alias(&mut v)` on a caller-owned object (axis A1).
    pub fn call_local(&self, v: i32) -> Obs {
        let mut local: c_int = v;
        self.call_raw(&mut local)
    }

    /// `static_alias(&inner)` — the aliased case (axis A2). Requires that the
    /// internal address has already been revealed by an earlier then-branch.
    pub fn call_aliased(&self) -> Obs {
        let p = self.internal.get();
        assert!(!p.is_null(), "{}: &inner not yet known", self.name);
        self.call_raw(p)
    }

    /// Non-destructive read-back of `inner` (when `inner != INT_MIN`).
    ///
    /// `INT_MIN >= inner` is false for every `inner > INT_MIN`, so this takes the
    /// else-branch, leaves `inner` untouched, and sets the local to
    /// `INT_MIN + inner`. In the single `inner == INT_MIN` case it takes the
    /// then-branch instead; the returned arithmetic below is still correct and,
    /// because the probe is applied identically to both libraries, they stay in
    /// lockstep either way.
    pub fn probe_inner(&self) -> Obs {
        let o = self.call_local(i32::MIN);
        if !o.ret_is_arg {
            // The only way `INT_MIN >= inner` holds is `inner == INT_MIN`, and in
            // that case the probe clobbered `inner` (to 0). Restore it exactly, so
            // `probe_inner` is non-destructive for every possible state. The same
            // restore is applied to both libraries.
            let p = self.internal.get();
            assert!(!p.is_null());
            unsafe { p.write(Self::inner_from_probe(o)) };
        }
        o
    }

    /// Decode `probe_inner`'s observation into the value `inner` had.
    pub fn inner_from_probe(o: Obs) -> i32 {
        o.ret_val.wrapping_sub(i32::MIN)
    }

    /// The C `driver` pattern expressed at the *low level*: take the address of a
    /// local, then feed the returned pointer straight back in `n` times — one
    /// `static_alias` call per step, exactly as `driver` does.
    pub fn feedback(&self, initial: i32, n: usize) -> Vec<Obs> {
        let mut local: c_int = initial;
        let mut cur: *mut c_int = &mut local;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let (o, ret) = self.call_raw2(cur);
            out.push(o);
            cur = ret;
        }
        out
    }

    /// A fresh caller-owned local per call (axis A1), driven by `vals`.
    pub fn seq_fresh_locals(&self, vals: &[i32]) -> Vec<Obs> {
        vals.iter().map(|&v| self.call_local(v)).collect()
    }

    /// Drive `inner` to a chosen value by repeatedly taking the then-branch.
    /// Returns the observations so they can be compared too.
    pub fn drive_inner_to(&self, steps: &[i32]) -> Vec<Obs> {
        steps.iter().map(|&v| self.call_local(v)).collect()
    }

    pub fn driver(&self, initial: c_int, iterations: c_int) {
        unsafe { (self.dr)(initial, iterations) }
    }

    /// The library's `&inner`, once revealed by a then-branch call.
    pub fn internal_ptr(&self) -> *mut c_int {
        self.internal.get()
    }
}

// ---------------------------------------------------------------------------
// A matched, freshly loaded C+Rust pair
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

impl Pair {
    /// `dlopen` both libraries fresh. Because the previous `Pair` has been
    /// dropped (`dlclose`), the data segment is re-initialised and
    /// `inner == 1` again — asserted here so a row can never silently start from
    /// inherited state.
    pub fn fresh() -> Pair {
        let p = Pair {
            c: Lib::open(c_so(), "C"),
            r: Lib::open(rust_so(), "Rust"),
        };
        let (oc, or) = (p.c.probe_inner(), p.r.probe_inner());
        assert_eq!(oc, or, "fresh-state probe already diverges");
        assert_eq!(Lib::inner_from_probe(oc), 1, "C library not freshly loaded");
        assert_eq!(Lib::inner_from_probe(or), 1, "Rust library not freshly loaded");
        p
    }

    /// Same call on both; assert the full observation matches.
    pub fn both<F: Fn(&Lib) -> Vec<Obs>>(&self, ctx: &str, f: F) -> Vec<Obs> {
        let a = f(&self.c);
        let b = f(&self.r);
        assert_eq!(a.len(), b.len(), "{ctx}: observation count differs");
        for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
            assert_eq!(x, y, "{ctx}: divergence at call {i}\n  C   ={x:?}\n  Rust={y:?}");
        }
        // Post-state must agree as well.
        let (pc, pr) = (self.c.probe_inner(), self.r.probe_inner());
        assert_eq!(
            pc, pr,
            "{ctx}: post-call `inner` diverges (C inner={}, Rust inner={})",
            Lib::inner_from_probe(pc),
            Lib::inner_from_probe(pr)
        );
        a
    }

    /// Run `driver` on both and compare stdout byte-for-byte plus post-state.
    pub fn both_driver(&self, ctx: &str, initial: c_int, iterations: c_int) {
        let out_c = capture_stdout(|| self.c.driver(initial, iterations));
        let out_r = capture_stdout(|| self.r.driver(initial, iterations));
        assert_eq!(
            out_c,
            out_r,
            "{ctx}: driver({initial}, {iterations}) stdout differs\n  C   ={:?}\n  Rust={:?}",
            String::from_utf8_lossy(&out_c),
            String::from_utf8_lossy(&out_r)
        );
        let (pc, pr) = (self.c.probe_inner(), self.r.probe_inner());
        assert_eq!(
            pc, pr,
            "{ctx}: driver({initial}, {iterations}) left different `inner` (C={}, Rust={})",
            Lib::inner_from_probe(pc),
            Lib::inner_from_probe(pr)
        );
    }
}

// ---------------------------------------------------------------------------
// stdout capture (fd-level, so libc `printf` from either `.so` is caught)
// ---------------------------------------------------------------------------

pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let mut tmpl: [u8; 26] = *b"/tmp/salias_cap_XXXXXX\0\0\0\0";
        let fd = libc::mkstemp(tmpl.as_mut_ptr() as *mut c_char);
        assert!(fd >= 0, "mkstemp failed");
        libc::unlink(tmpl.as_ptr() as *const c_char);

        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");
        f();
        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);

        libc::lseek(fd, 0, libc::SEEK_SET);
        let mut buf = Vec::new();
        let mut chunk = [0u8; 65536];
        loop {
            let n = libc::read(fd, chunk.as_mut_ptr() as *mut c_void, chunk.len());
            if n <= 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n as usize]);
        }
        libc::close(fd);
        buf
    }
}

// ---------------------------------------------------------------------------
// Crash-comparison: run a call in a forked child, report how it terminated
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Signal(i32),
    Exit(i32),
}

/// The libraries are `dlopen`ed by the *parent* before forking, so the child only
/// makes an already-resolved call — no loader work happens after `fork`.
pub fn run_in_child<F: FnOnce()>(f: F) -> Outcome {
    run_in_child_capturing_stderr(f).0
}

/// As `run_in_child`, but also returns whatever the child wrote to stderr. Used to
/// tell a real hardware fault apart from Rust's *optional* debug-build UB
/// precondition assertions, which print a diagnostic and `abort()`.
pub fn run_in_child_capturing_stderr<F: FnOnce()>(f: F) -> (Outcome, Vec<u8>) {
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let mut fds = [0 as c_int; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe failed");
        let (rd, wr) = (fds[0], fds[1]);

        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            libc::close(rd);
            libc::dup2(wr, 2);
            libc::close(wr);
            f();
            libc::_exit(0);
        }
        libc::close(wr);

        let mut err = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let n = libc::read(rd, chunk.as_mut_ptr() as *mut c_void, chunk.len());
            if n <= 0 {
                break;
            }
            err.extend_from_slice(&chunk[..n as usize]);
        }
        libc::close(rd);

        let mut status: c_int = 0;
        libc::waitpid(pid, &mut status, 0);
        let outcome = if libc::WIFSIGNALED(status) {
            Outcome::Signal(libc::WTERMSIG(status))
        } else {
            Outcome::Exit(libc::WEXITSTATUS(status))
        };
        (outcome, err)
    }
}

/// True if the child died because a Rust *debug-build* optional UB precondition
/// assertion fired (rather than because the hardware faulted). These checks exist
/// only when `debug-assertions`/`ub-checks` are on, and `core` documents them as
/// optional and not to be relied upon; the release cdylib — the artifact that
/// corresponds to the C `.so` — has none of them.
pub fn is_rust_ub_check_abort(outcome: Outcome, stderr: &[u8]) -> bool {
    let text = String::from_utf8_lossy(stderr);
    outcome == Outcome::Signal(libc::SIGABRT)
        && (text.contains("unsafe precondition") || text.contains("Undefined Behavior check"))
}

/// Assert that C and Rust agree on how an invalid-pointer call terminates.
///
/// Requires the *exact* same signal, except when the Rust side demonstrably died
/// from an optional debug-build UB precondition assertion — which is proven from
/// the child's own stderr, not assumed.
pub fn assert_same_fault(ctx: &str, c: (Outcome, Vec<u8>), r: (Outcome, Vec<u8>)) {
    let (oc, ec) = c;
    let (or, er) = r;
    assert!(
        ec.is_empty(),
        "{ctx}: the C library wrote to stderr, which it never does: {}",
        String::from_utf8_lossy(&ec)
    );
    if oc == or {
        return;
    }
    assert!(
        is_rust_ub_check_abort(or, &er),
        "{ctx}: fault outcome differs and the Rust side did not die from an \
         optional debug-build UB check: C={oc:?} Rust={or:?}\nRust stderr: {}",
        String::from_utf8_lossy(&er)
    );
    eprintln!(
        "{ctx}: note — C={oc:?}, Rust={or:?} via a debug-build optional UB \
         precondition assertion (absent from the release cdylib)"
    );
}

// ---------------------------------------------------------------------------
// Fixed-seed PRNG (xorshift64*) — reproducible property-style inputs
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5A71C_A11A5;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform over the full `i32` range.
    pub fn i32_any(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform inclusive range over `i64`, then narrowed.
    pub fn in_range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        if span == 0 {
            return self.next_u64() as i64;
        }
        lo + (self.next_u64() % span) as i64
    }
    pub fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        self.in_range(lo as i64, hi as i64) as i32
    }
}

/// How many randomized inputs each `CONFIGS.md` row uses.
pub const N: usize = 200;
