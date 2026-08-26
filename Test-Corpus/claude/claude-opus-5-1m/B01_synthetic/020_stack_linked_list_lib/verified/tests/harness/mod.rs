//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called only through their exported `smallestValue` symbol. The Rust code is
//! NEVER called directly as a Rust function -- it is always reached through the
//! `#[no_mangle] extern "C"` export in the `.so`, exactly as a C consumer would,
//! so the export wrapper and its ABI are part of what is under test.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// Mirrors `struct ListNode { int value; struct ListNode* next; };`
#[repr(C)]
pub struct CListNode {
    pub value: c_int,
    pub next: *mut CListNode,
}

/// ABI of `int smallestValue(struct ListNode *)`.
pub type SmallestFn = unsafe extern "C" fn(*mut CListNode) -> c_int;

pub struct Impl {
    name: &'static str,
    // Field order matters: `f` is only valid while `_lib` is loaded, and struct
    // fields drop in declaration order, so keep the library last.
    f: SmallestFn,
    _lib: Library,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} ({}) failed: {e}", path.display(), name));
            let f = {
                let sym: Symbol<SmallestFn> = lib.get(b"smallestValue").unwrap_or_else(|e| {
                    panic!("dlsym smallestValue in {} ({}) failed: {e}", path.display(), name)
                });
                *sym
            };
            Impl { name, f, _lib: lib }
        }
    }

    /// Call the library's exported `smallestValue` across the FFI boundary.
    pub fn smallest_value(&self, head: *mut CListNode) -> c_int {
        unsafe { (self.f)(head) }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

pub struct Impls {
    pub c: Impl,
    pub rust: Impl,
}

// `libloading::Library` is Send + Sync on unix and a bare `extern "C" fn` is
// Copy + Send + Sync, so the pair can live in a process-wide OnceLock.
unsafe impl Send for Impls {}
unsafe impl Sync for Impls {}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn newest_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .filter(|p| p.is_file())
        .max_by_key(|p| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        })
        .cloned()
}

/// Locate `libSimpleList.so` built from `c_src/` (building it if necessary).
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("SIMPLELIST_C_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir();
    let candidates = vec![
        root.join("c_src/build/libSimpleList.so"),
        root.join("target/c-build/libSimpleList.so"),
    ];
    if let Some(p) = newest_existing(&candidates) {
        return p;
    }

    // Fall back to configuring/building out of tree so nothing in c_src/ is touched.
    let build_dir = root.join("target/c-build");
    std::fs::create_dir_all(&build_dir).expect("create target/c-build");
    let cfg = Command::new("cmake")
        .arg("-S")
        .arg(root.join("c_src"))
        .arg("-B")
        .arg(&build_dir)
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
        .arg("--build")
        .arg(&build_dir)
        .output()
        .expect("run cmake build");
    assert!(
        bld.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&bld.stdout),
        String::from_utf8_lossy(&bld.stderr)
    );
    let out = build_dir.join("libSimpleList.so");
    assert!(out.is_file(), "cmake produced no {}", out.display());
    out
}

/// Locate the Rust cdylib (building it if necessary).
///
/// `cargo test` does not emit the cdylib artifact, so if no prebuilt `.so` is
/// present we build one into a *separate* target dir. Using a separate
/// `--target-dir` avoids contending on the build lock held by the outer
/// `cargo test` invocation.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("SIMPLELIST_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir();
    let candidates = vec![
        root.join("target/release/libSimpleList.so"),
        root.join("target/debug/libSimpleList.so"),
        root.join("target/diff-so/release/libSimpleList.so"),
    ];
    if let Some(p) = newest_existing(&candidates) {
        return p;
    }

    let target_dir = root.join("target/diff-so");
    let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(&root)
        .args([
            "build",
            "--offline",
            "--release",
            "--no-default-features",
            "--target-dir",
        ])
        .arg(&target_dir)
        .output()
        .expect("run cargo build for the cdylib");
    assert!(
        out.status.success(),
        "cargo build of cdylib failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let so = target_dir.join("release/libSimpleList.so");
    assert!(so.is_file(), "cargo produced no {}", so.display());
    so
}

static IMPLS: OnceLock<Impls> = OnceLock::new();

/// The two implementations under differential test.
pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| {
        let c = c_so_path();
        let r = rust_so_path();
        eprintln!("[harness] C    .so: {}", c.display());
        eprintln!("[harness] Rust .so: {}", r.display());
        Impls {
            c: Impl::load("C", &c),
            rust: Impl::load("Rust", &r),
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) -- fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234;

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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform over the whole `i32` bit-pattern space.
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }

    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    /// Uniform in `lo..=hi` for lengths.
    pub fn len_in(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
}

// ---------------------------------------------------------------------------
// List construction
// ---------------------------------------------------------------------------

/// Owns a linked list of `ListNode`s with stable addresses.
pub struct List {
    /// Boxed nodes; allocation order is *not* necessarily link order.
    nodes: Vec<Box<CListNode>>,
    head: *mut CListNode,
    /// Logical values in link order.
    pub values: Vec<i32>,
}

impl List {
    /// Build a list whose nodes are allocated in link order.
    pub fn new(values: &[i32]) -> List {
        List::with_order(values, &(0..values.len()).collect::<Vec<_>>())
    }

    /// Build a list where `alloc_order[k]` is the logical index of the k-th
    /// allocated node, letting `next` order differ from address order.
    pub fn with_order(values: &[i32], alloc_order: &[usize]) -> List {
        assert_eq!(values.len(), alloc_order.len());
        let n = values.len();
        let mut nodes: Vec<Box<CListNode>> = Vec::with_capacity(n);
        // slot[i] = pointer to the node holding logical index i
        let mut slot: Vec<*mut CListNode> = vec![std::ptr::null_mut(); n];
        for &logical in alloc_order {
            let mut b = Box::new(CListNode {
                value: values[logical],
                next: std::ptr::null_mut(),
            });
            slot[logical] = b.as_mut() as *mut CListNode;
            nodes.push(b);
        }
        for i in 0..n {
            let next = if i + 1 < n { slot[i + 1] } else { std::ptr::null_mut() };
            unsafe { (*slot[i]).next = next };
        }
        let head = if n == 0 { std::ptr::null_mut() } else { slot[0] };
        List { nodes, head, values: values.to_vec() }
    }

    pub fn head(&self) -> *mut CListNode {
        self.head
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Read the values back by walking the `next` chain (used to prove the
    /// callee did not mutate the caller's nodes).
    pub fn walk_values(&self) -> Vec<i32> {
        let mut out = Vec::new();
        let mut p = self.head;
        while !p.is_null() {
            unsafe {
                out.push((*p).value);
                p = (*p).next;
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Differential assertion
// ---------------------------------------------------------------------------

fn describe(values: &[i32]) -> String {
    if values.len() <= 24 {
        format!("{values:?}")
    } else {
        format!(
            "len={} head={:?} ... tail={:?}",
            values.len(),
            &values[..8],
            &values[values.len() - 8..]
        )
    }
}

/// Call both `.so`s with `head` and require bit-identical `int` results.
pub fn assert_same(ctx: &str, head: *mut CListNode, values: &[i32]) -> c_int {
    let im = impls();
    let c = im.c.smallest_value(head);
    let r = im.rust.smallest_value(head);
    assert_eq!(
        c, r,
        "[{ctx}] DIVERGENCE: C returned {c} (0x{c:08x}) but Rust returned {r} (0x{r:08x}) \
         for input {}",
        describe(values)
    );
    assert_eq!(
        c.to_ne_bytes(),
        r.to_ne_bytes(),
        "[{ctx}] byte-level divergence for input {}",
        describe(values)
    );
    c
}

/// `assert_same` plus a cross-check against the independently computed
/// expectation, which catches harness bugs (e.g. a mis-linked list).
pub fn assert_same_expect(ctx: &str, list: &List, expected: c_int) -> c_int {
    let got = assert_same(ctx, list.head(), &list.values);
    assert_eq!(
        got, expected,
        "[{ctx}] both impls agreed on {got} but the expected value is {expected} for input {}",
        describe(&list.values)
    );
    got
}

/// Expected result of the C function for a list of `values`:
/// `-1` when empty, otherwise the minimum.
pub fn expected(values: &[i32]) -> c_int {
    match values.iter().copied().min() {
        None => -1,
        Some(m) => m,
    }
}
