//! Shared differential-test harness.
//!
//! Both the C library and the Rust library are loaded as shared objects with
//! `libloading` and driven **only** through their exported `wcscat` symbol, so
//! the `#[no_mangle] extern "C"` wrapper is part of what is under test.
//!
//! `wcscat` collides with glibc's 2-argument `wcscat`, so both handles are opened
//! with `RTLD_LOCAL` (libloading's default) and looked up per handle; `dlsym` on a
//! handle searches that object first, which was confirmed with `dladdr`.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// `wchar_t` on the C build target (`x86_64-unknown-linux-gnu`, gcc): signed 32-bit.
pub type WcharT = i32;

pub type WcscatFn = unsafe extern "C" fn(*mut WcharT, usize, *const WcharT) -> i32;

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_so_in(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") {
            let name = p.file_name()?.to_str()?.to_string();
            if name.starts_with("lib") {
                return Some(p);
            }
        }
    }
    None
}

fn c_so_path() -> PathBuf {
    let root = workspace_root();
    let c_src = root.join("c_src");
    let build = c_src.join("build");

    if let Some(p) = find_so_in(&build) {
        return p;
    }

    // Build it on demand so the test suite is self-contained.
    std::fs::create_dir_all(&build).expect("create c_src/build");
    let cfg = Command::new("cmake")
        .current_dir(&build)
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .output()
        .expect("run cmake configure");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&cfg.stdout),
        String::from_utf8_lossy(&cfg.stderr)
    );
    let bld = Command::new("cmake")
        .current_dir(&build)
        .arg("--build")
        .arg(".")
        .output()
        .expect("run cmake build");
    assert!(
        bld.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&bld.stdout),
        String::from_utf8_lossy(&bld.stderr)
    );

    find_so_in(&build).expect("C .so produced by cmake --build")
}

/// Locate the Rust `cdylib`.
///
/// `cargo test` does **not** build the `cdylib` target (it only builds the test
/// harnesses), so the harness builds it explicitly into a *separate*
/// `CARGO_TARGET_DIR` — a separate dir avoids fighting the outer `cargo test`
/// for the target-directory lock.
///
/// Set `WCSCAT_RUST_SO=/path/to/libwcscat_lib.so` to test a specific artifact
/// (used by `run_all_configs.sh` to exercise several profiles / feature combos).
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("WCSCAT_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(
            p.exists(),
            "WCSCAT_RUST_SO points at a missing file: {}",
            p.display()
        );
        return p;
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let harness_target = manifest.join("target").join("harness-rs");
    let profile_arg = "--release";
    let profile_dir = "release";

    let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(&manifest)
        .arg("build")
        .arg("--lib")
        .arg(profile_arg)
        .env("CARGO_TARGET_DIR", &harness_target)
        // Do not inherit the outer test run's RUSTFLAGS-ish env that could
        // change the artifact name.
        .env_remove("CARGO_BUILD_TARGET_DIR")
        .output()
        .expect("run cargo build --lib to produce the cdylib");
    assert!(
        out.status.success(),
        "cargo build --lib failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let dir = harness_target.join(profile_dir);
    for name in ["libwcscat_lib.so", "libtranslation.so"] {
        let cand = dir.join(name);
        if cand.exists() {
            return cand;
        }
    }
    if let Some(p) = find_so_in(&dir) {
        return p;
    }
    panic!(
        "cargo build --lib succeeded but no .so was found in {}",
        dir.display()
    );
}

pub struct Libs {
    _c_lib: libloading::Library,
    _rs_lib: libloading::Library,
    pub c: WcscatFn,
    pub rs: WcscatFn,
    pub c_path: PathBuf,
    pub rs_path: PathBuf,
}

// The two function pointers are plain `extern "C" fn`s into leaf code with no
// shared mutable state, so sharing the struct across threads is fine.
unsafe impl Sync for Libs {}
unsafe impl Send for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rs_path = rust_so_path();
        unsafe {
            let c_lib = libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rs_lib = libloading::Library::new(&rs_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display()));

            let c_sym: libloading::Symbol<WcscatFn> = c_lib
                .get(b"wcscat\0")
                .expect("C .so must export `wcscat`");
            let rs_sym: libloading::Symbol<WcscatFn> = rs_lib
                .get(b"wcscat\0")
                .expect("Rust .so must export `wcscat`");

            let c = *c_sym;
            let rs = *rs_sym;
            Libs {
                _c_lib: c_lib,
                _rs_lib: rs_lib,
                c,
                rs,
                c_path,
                rs_path,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Case description
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Src {
    /// Pass `NULL` for `src`.
    Null,
    /// Pass a pointer to a separate allocation with these contents.
    Buf(Vec<WcharT>),
    /// Pass `dst.add(offset)` — deliberate aliasing.
    AliasDst(usize),
}

/// One differential test case.
///
/// `dst` is the **entire backing allocation** handed to the library. `num_elem`
/// is what is passed as the `numElem` argument and may be smaller than, equal to
/// or larger than `dst.len()`; the elements of `dst` beyond `num_elem` act as a
/// guard region that the C never touches, and the comparison covers all of them.
#[derive(Clone, Debug)]
pub struct Case {
    pub name: String,
    /// `None` => pass `NULL` for `dst`.
    pub dst: Option<Vec<WcharT>>,
    pub num_elem: usize,
    pub src: Src,
}

impl Case {
    pub fn new(name: impl Into<String>, dst: Vec<WcharT>, num_elem: usize, src: Src) -> Self {
        Case {
            name: name.into(),
            dst: Some(dst),
            num_elem,
            src,
        }
    }
    pub fn null_dst(name: impl Into<String>, num_elem: usize, src: Src) -> Self {
        Case {
            name: name.into(),
            dst: None,
            num_elem,
            src,
        }
    }
}

/// Everything observable after a call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub ret: i32,
    /// Final contents of the whole `dst` allocation (`None` if `dst` was NULL).
    pub dst: Option<Vec<WcharT>>,
    /// Final contents of the separate `src` allocation, to prove `src` is not
    /// written through (`None` for `Src::Null` / `Src::AliasDst`).
    pub src: Option<Vec<WcharT>>,
}

fn invoke(f: WcscatFn, case: &Case) -> Outcome {
    match &case.dst {
        None => {
            // dst == NULL: still honour the src spec.
            match &case.src {
                Src::Null => {
                    let ret = unsafe { f(std::ptr::null_mut(), case.num_elem, std::ptr::null()) };
                    Outcome {
                        ret,
                        dst: None,
                        src: None,
                    }
                }
                Src::Buf(s) => {
                    let mut sv = s.clone();
                    let ret = unsafe { f(std::ptr::null_mut(), case.num_elem, sv.as_ptr()) };
                    Outcome {
                        ret,
                        dst: None,
                        src: Some(std::mem::take(&mut sv)),
                    }
                }
                Src::AliasDst(_) => panic!("AliasDst is meaningless with a NULL dst"),
            }
        }
        Some(d) => {
            let mut dv = d.clone();
            match &case.src {
                Src::Null => {
                    let ret = unsafe { f(dv.as_mut_ptr(), case.num_elem, std::ptr::null()) };
                    Outcome {
                        ret,
                        dst: Some(dv),
                        src: None,
                    }
                }
                Src::Buf(s) => {
                    let mut sv = s.clone();
                    let ret = unsafe { f(dv.as_mut_ptr(), case.num_elem, sv.as_ptr()) };
                    Outcome {
                        ret,
                        dst: Some(dv),
                        src: Some(std::mem::take(&mut sv)),
                    }
                }
                Src::AliasDst(off) => {
                    assert!(
                        *off < dv.len(),
                        "aliasing offset {off} must stay inside the {} element allocation",
                        dv.len()
                    );
                    let base = dv.as_mut_ptr();
                    let ret = unsafe { f(base, case.num_elem, base.add(*off) as *const WcharT) };
                    Outcome {
                        ret,
                        dst: Some(dv),
                        src: None,
                    }
                }
            }
        }
    }
}

fn fmt_slice(v: &[WcharT]) -> String {
    let mut s = String::from("[");
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        if *x == 0 {
            s.push_str("__");
        } else {
            s.push_str(&format!("{:#x}", *x as u32));
        }
    }
    s.push(']');
    s
}

/// Run one case against both `.so`s and assert byte-identical observable results.
/// Returns the (shared) outcome so callers can additionally assert absolute facts.
pub fn check(case: &Case) -> Outcome {
    let l = libs();
    let c_out = invoke(l.c, case);
    let rs_out = invoke(l.rs, case);

    if c_out != rs_out {
        let mut msg = format!(
            "DIVERGENCE in case `{}`\n  num_elem = {} ({:#x})\n  src spec = {:?}\n",
            case.name, case.num_elem, case.num_elem, case.src
        );
        if let Some(d) = &case.dst {
            msg += &format!("  dst  in  = {}\n", fmt_slice(d));
        } else {
            msg += "  dst  in  = NULL\n";
        }
        if let Src::Buf(s) = &case.src {
            msg += &format!("  src  in  = {}\n", fmt_slice(s));
        }
        msg += &format!("  C  ret  = {}\n  RS ret  = {}\n", c_out.ret, rs_out.ret);
        match (&c_out.dst, &rs_out.dst) {
            (Some(a), Some(b)) => {
                msg += &format!("  C  dst  = {}\n  RS dst  = {}\n", fmt_slice(a), fmt_slice(b));
                if a.len() == b.len() {
                    let diffs: Vec<usize> =
                        (0..a.len()).filter(|&i| a[i] != b[i]).take(16).collect();
                    msg += &format!("  first differing dst indices: {diffs:?}\n");
                }
            }
            _ => {}
        }
        match (&c_out.src, &rs_out.src) {
            (Some(a), Some(b)) if a != b => {
                msg += &format!("  C  src  = {}\n  RS src  = {}\n", fmt_slice(a), fmt_slice(b));
            }
            _ => {}
        }
        panic!("{msg}");
    }

    // The return code domain of the C is exactly {0, 22, 34}.
    assert!(
        matches!(c_out.ret, 0 | 22 | 34),
        "case `{}`: unexpected return code {} from the C library",
        case.name,
        c_out.ret
    );

    c_out
}

/// Run a batch and report how many cases ran (for the row-coverage output).
pub fn check_all(cases: &[Case]) -> usize {
    for c in cases {
        check(c);
    }
    cases.len()
}

/// One step of a multi-call sequence on the *same* buffer.
pub struct Step {
    pub num_elem: usize,
    pub src: Src,
}

impl Step {
    pub fn new(num_elem: usize, src: Src) -> Self {
        Step { num_elem, src }
    }
}

/// Drive a whole *sequence* of calls against one persistent buffer in each
/// library, comparing the return code and the entire buffer after EVERY step.
/// This is how a real consumer uses `wcscat` (append, append, append …) and it
/// catches divergences that only show up in the composed pipeline.
///
/// Returns the per-step return codes.
pub fn check_sequence(name: &str, dst_init: &[WcharT], steps: &[Step]) -> Vec<i32> {
    let l = libs();
    let mut c_dst = dst_init.to_vec();
    let mut rs_dst = dst_init.to_vec();
    let mut rets = Vec::with_capacity(steps.len());

    for (i, st) in steps.iter().enumerate() {
        let (c_ret, rs_ret) = match &st.src {
            Src::Null => unsafe {
                (
                    (l.c)(c_dst.as_mut_ptr(), st.num_elem, std::ptr::null()),
                    (l.rs)(rs_dst.as_mut_ptr(), st.num_elem, std::ptr::null()),
                )
            },
            Src::Buf(s) => {
                let cs = s.clone();
                let rs = s.clone();
                let out = unsafe {
                    (
                        (l.c)(c_dst.as_mut_ptr(), st.num_elem, cs.as_ptr()),
                        (l.rs)(rs_dst.as_mut_ptr(), st.num_elem, rs.as_ptr()),
                    )
                };
                assert_eq!(cs, rs, "`{name}` step {i}: src buffers diverged");
                assert_eq!(&cs, s, "`{name}` step {i}: src must not be modified");
                out
            }
            Src::AliasDst(off) => {
                assert!(*off < c_dst.len());
                unsafe {
                    let cb = c_dst.as_mut_ptr();
                    let rb = rs_dst.as_mut_ptr();
                    (
                        (l.c)(cb, st.num_elem, cb.add(*off) as *const WcharT),
                        (l.rs)(rb, st.num_elem, rb.add(*off) as *const WcharT),
                    )
                }
            }
        };
        assert_eq!(
            c_ret, rs_ret,
            "`{name}` step {i}: return codes diverged (C={c_ret}, RS={rs_ret})\n  \
             C  dst = {}\n  RS dst = {}",
            fmt_slice(&c_dst),
            fmt_slice(&rs_dst)
        );
        assert_eq!(
            c_dst,
            rs_dst,
            "`{name}` step {i}: buffers diverged after ret {c_ret}\n  C  dst = {}\n  RS dst = {}",
            fmt_slice(&c_dst),
            fmt_slice(&rs_dst)
        );
        assert!(
            matches!(c_ret, 0 | 22 | 34),
            "`{name}` step {i}: illegal return code {c_ret}"
        );
        rets.push(c_ret);
    }
    rets
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed, no external dependency
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0DE_1234_5678;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform-ish in `[0, n)`; `n == 0` yields 0.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// A non-zero `wchar_t`, biased towards nasty values.
    pub fn nonzero_wchar(&mut self) -> WcharT {
        const NASTY: [i32; 12] = [
            i32::MIN,
            i32::MIN + 1,
            -1,
            -2,
            i32::MAX,
            i32::MAX - 1,
            0x41424344,
            0xD800u32 as i32,
            0xDFFFu32 as i32,
            0x0011_0000,
            0x7FFF_FFFE,
            1,
        ];
        if self.below(4) == 0 {
            NASTY[self.below(NASTY.len())]
        } else {
            let mut v = self.next_u32() as i32;
            if v == 0 {
                v = 1;
            }
            v
        }
    }

    pub fn fill_nonzero(&mut self, n: usize) -> Vec<WcharT> {
        (0..n).map(|_| self.nonzero_wchar()).collect()
    }

    /// `make_src` with a random length in `[0, n)`.
    pub fn rand_src_below(&mut self, n: usize) -> Vec<WcharT> {
        let l = self.below(n);
        make_src(self, l)
    }

    /// `make_src` with a random length in `[lo, hi]`.
    pub fn rand_src_range(&mut self, lo: usize, hi: usize) -> Vec<WcharT> {
        let l = self.range(lo, hi);
        make_src(self, l)
    }
}

// ---------------------------------------------------------------------------
// Buffer builders
// ---------------------------------------------------------------------------

/// Sentinel used for the guard region so any stray write is visible.
pub const GUARD: WcharT = 0x6775_4152 /* "guAR" */;

/// Build a `dst` allocation of `alloc` elements whose window `[0, num_elem)`
/// holds `k` non-zero chars then a `0`, with the rest of the window and the
/// guard tail filled with distinctive non-zero junk.
///
/// `k == None` means "no terminator inside the window" (unterminated / full).
pub fn make_dst(rng: &mut Rng, alloc: usize, num_elem: usize, k: Option<usize>) -> Vec<WcharT> {
    let mut v: Vec<WcharT> = (0..alloc)
        .map(|i| {
            if i >= num_elem {
                GUARD ^ (i as i32)
            } else {
                rng.nonzero_wchar()
            }
        })
        .collect();
    if let Some(k) = k {
        assert!(k < num_elem, "terminator index {k} must be < num_elem {num_elem}");
        assert!(k < alloc, "terminator index {k} must be < alloc {alloc}");
        v[k] = 0;
        // Junk after the terminator (still inside the window) must be preserved.
        for i in (k + 1)..num_elem.min(alloc) {
            if v[i] == 0 {
                v[i] = 0x0BAD_F00D;
            }
        }
    }
    v
}

/// A NUL-terminated `src` buffer of `len` non-zero chars (plus the terminator),
/// followed by junk to prove nothing past the terminator is read as payload.
pub fn make_src(rng: &mut Rng, len: usize) -> Vec<WcharT> {
    let mut v = rng.fill_nonzero(len);
    v.push(0);
    let tail = rng.below(4);
    for _ in 0..tail {
        v.push(rng.nonzero_wchar());
    }
    v
}
