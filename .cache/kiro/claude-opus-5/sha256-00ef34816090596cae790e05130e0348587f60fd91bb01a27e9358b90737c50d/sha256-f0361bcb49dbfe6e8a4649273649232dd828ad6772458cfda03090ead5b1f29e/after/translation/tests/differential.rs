//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` via
//! `libloading` and compares `smallestValue` results through the FFI boundary.
//!
//! Neither implementation is ever called directly as a Rust function — both go
//! through `dlopen`/`dlsym`, exactly as an external C consumer would, so the
//! `#[no_mangle] extern "C"` export wrapper is under test too.

use std::ffi::c_int;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// The C ABI type, mirrored from c_src/include/simplestruct.h
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

impl ListNode {
    pub fn new(value: c_int) -> Self {
        ListNode {
            value,
            next: std::ptr::null_mut(),
        }
    }
}

pub type SmallestValueFn = unsafe extern "C" fn(*mut ListNode) -> c_int;

// ---------------------------------------------------------------------------
// Locating and loading the two shared libraries
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let p = workspace_root().join("c_src/build/libSimpleList.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Build and locate the Rust cdylib.
///
/// IMPORTANT: `cargo test` does **not** rebuild a `crate-type = ["cdylib"]`
/// artifact (it only builds the lib-test rlib), so simply pointing at
/// `target/debug/libSimpleList.so` can silently load a STALE library and make
/// every differential test vacuously pass. We therefore invoke
/// `cargo build --lib` ourselves into a dedicated target directory (a separate
/// dir avoids contending on the outer cargo's `target/` lock) and load that
/// artifact, so the `.so` under test always matches the current `src/lib.rs`.
fn rust_so_path() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest.join("target/ffi-so");
    let profile_dir = if cfg!(debug_assertions) {
        target_dir.join("debug")
    } else {
        target_dir.join("release")
    };
    let so = profile_dir.join("libSimpleList.so");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(manifest)
        .arg("build")
        .arg("--lib")
        .arg("--target-dir")
        .arg(&target_dir);
    if !cfg!(debug_assertions) {
        cmd.arg("--release");
    }
    // Don't let the outer cargo's environment redirect the nested build.
    cmd.env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .env_remove("RUSTC_WRAPPER");

    let out = cmd.output().expect("failed to spawn `cargo build --lib`");
    assert!(
        out.status.success(),
        "nested `cargo build --lib` failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        so.is_file(),
        "Rust cdylib not produced at {}\ncargo stderr:\n{}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );

    // Belt-and-braces staleness check: the .so must be at least as new as the
    // newest Rust source input.
    let so_mtime = std::fs::metadata(&so)
        .and_then(|m| m.modified())
        .expect("stat cdylib");
    for src in ["src/lib.rs", "Cargo.toml"] {
        let p = manifest.join(src);
        if let Ok(m) = std::fs::metadata(&p).and_then(|md| md.modified()) {
            assert!(
                so_mtime >= m,
                "STALE Rust .so: {} is newer than {}",
                p.display(),
                so.display()
            );
        }
    }

    so
}

/// Both libraries, kept alive for the process lifetime.
struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    c: SmallestValueFn,
    rust: SmallestValueFn,
}

impl Impls {
    fn load() -> Self {
        // SAFETY: loading two plain C-ABI libraries with no constructors that
        // run arbitrary code beyond libc/libstd init.
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("dlopen C libSimpleList.so");
            let rust_lib = Library::new(rust_so_path()).expect("dlopen Rust libSimpleList.so");

            let c_sym: Symbol<SmallestValueFn> = c_lib
                .get(b"smallestValue\0")
                .expect("dlsym smallestValue in C .so");
            let rust_sym: Symbol<SmallestValueFn> = rust_lib
                .get(b"smallestValue\0")
                .expect("dlsym smallestValue in Rust .so");

            let c = *c_sym;
            let rust = *rust_sym;

            Impls {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }
}

fn impls() -> &'static Impls {
    use std::sync::OnceLock;
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(Impls::load)
}

// `Impls` holds raw fn pointers + Library; sharing across threads is fine here
// because the C function is pure and reentrant (no globals, no allocation).
unsafe impl Sync for Impls {}
unsafe impl Send for Impls {}

// ---------------------------------------------------------------------------
// List construction helpers (three distinct memory layouts)
// ---------------------------------------------------------------------------

/// Storage backing a chain; keeps nodes alive while the head pointer is used.
/// The payloads are intentionally never read — they exist only to own the
/// memory the `head` pointer walks.
#[allow(dead_code)]
enum Storage {
    Contig(Vec<ListNode>),
    Boxes(Vec<Box<ListNode>>),
}

struct Chain {
    #[allow(dead_code)]
    _storage: Storage,
    head: *mut ListNode,
}

impl Chain {
    fn head(&self) -> *mut ListNode {
        self.head
    }
}

/// Contiguous `Vec`, chained in ascending address order (values in order).
fn chain_ascending(values: &[c_int]) -> Chain {
    let mut nodes: Vec<ListNode> = values.iter().map(|&v| ListNode::new(v)).collect();
    if nodes.is_empty() {
        return Chain {
            _storage: Storage::Contig(nodes),
            head: std::ptr::null_mut(),
        };
    }
    let base = nodes.as_mut_ptr();
    for i in 0..nodes.len() - 1 {
        // SAFETY: i+1 is in bounds.
        nodes[i].next = unsafe { base.add(i + 1) };
    }
    let head = base;
    Chain {
        _storage: Storage::Contig(nodes),
        head,
    }
}

/// Contiguous `Vec`, but chained from the LAST slot to the FIRST, so traversal
/// order runs against increasing addresses. `values[k]` is still the k-th node
/// visited.
fn chain_descending(values: &[c_int]) -> Chain {
    let n = values.len();
    let mut nodes: Vec<ListNode> = vec![ListNode::new(0); n];
    if n == 0 {
        return Chain {
            _storage: Storage::Contig(nodes),
            head: std::ptr::null_mut(),
        };
    }
    // node at slot (n-1-k) is the k-th visited node.
    for (k, &v) in values.iter().enumerate() {
        nodes[n - 1 - k].value = v;
    }
    let base = nodes.as_mut_ptr();
    for k in 0..n - 1 {
        let cur = n - 1 - k;
        let nxt = n - 1 - (k + 1);
        // SAFETY: both indices are in bounds.
        nodes[cur].next = unsafe { base.add(nxt) };
    }
    let head = unsafe { base.add(n - 1) };
    Chain {
        _storage: Storage::Contig(nodes),
        head,
    }
}

/// Individually heap-allocated nodes, chained in an order determined by the
/// PRNG so that traversal order is unrelated to allocation/address order.
fn chain_shuffled_boxes(values: &[c_int], rng: &mut Rng) -> Chain {
    let n = values.len();
    if n == 0 {
        return Chain {
            _storage: Storage::Boxes(Vec::new()),
            head: std::ptr::null_mut(),
        };
    }

    // Allocate n boxes, then decide a random permutation `order` such that
    // order[k] is the box index used for the k-th visited node.
    let mut boxes: Vec<Box<ListNode>> = (0..n).map(|_| Box::new(ListNode::new(0))).collect();
    let mut order: Vec<usize> = (0..n).collect();
    // Fisher-Yates with the seeded PRNG.
    for i in (1..n).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }

    for (k, &v) in values.iter().enumerate() {
        boxes[order[k]].value = v;
    }
    for k in 0..n - 1 {
        let next_ptr: *mut ListNode = &mut *boxes[order[k + 1]] as *mut ListNode;
        boxes[order[k]].next = next_ptr;
    }
    let head: *mut ListNode = &mut *boxes[order[0]] as *mut ListNode;
    Chain {
        _storage: Storage::Boxes(boxes),
        head,
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed, SplitMix64) — reproducible property tests
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
    fn i32(&mut self) -> c_int {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[lo, hi]`.
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        debug_assert!(lo <= hi);
        lo + self.next_u64() % (hi - lo + 1)
    }
    fn i32_in(&mut self, lo: i32, hi: i32) -> c_int {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

/// Number of randomized inputs per configuration row.
const ITERS: usize = 400;

// ---------------------------------------------------------------------------
// The core differential assertion
// ---------------------------------------------------------------------------

/// Calls BOTH `.so` exports on the same head pointer and asserts the returned
/// bytes are identical. Also re-runs in the opposite order to prove neither
/// implementation mutated the list.
#[track_caller]
fn assert_same(head: *mut ListNode, ctx: &str) -> c_int {
    let im = impls();
    // SAFETY: `head` is NULL or a valid null-terminated ListNode chain that
    // outlives these calls (owned by the caller's `Chain`).
    let c_out = unsafe { (im.c)(head) };
    let rust_out = unsafe { (im.rust)(head) };
    assert_eq!(
        c_out.to_ne_bytes(),
        rust_out.to_ne_bytes(),
        "divergence [{ctx}]: C returned {c_out}, Rust returned {rust_out}"
    );

    // Reverse call order: catches any hidden mutation of the input chain.
    let rust_out2 = unsafe { (im.rust)(head) };
    let c_out2 = unsafe { (im.c)(head) };
    assert_eq!(c_out2, c_out, "C not idempotent [{ctx}]");
    assert_eq!(rust_out2, rust_out, "Rust not idempotent [{ctx}]");

    c_out
}

/// Convenience: build an ascending-layout chain from `values` and diff it.
#[track_caller]
fn diff_values(values: &[c_int], ctx: &str) -> c_int {
    let chain = chain_ascending(values);
    assert_same(chain.head(), ctx)
}

/// Independent oracle: the C contract is "minimum, or -1 for an empty list".
#[track_caller]
fn assert_matches_oracle(values: &[c_int], got: c_int, ctx: &str) {
    let expected = values.iter().copied().min().unwrap_or(-1);
    assert_eq!(expected, got, "oracle mismatch [{ctx}] for {values:?}");
}

// ===========================================================================
// PHASE B — valid-path differential tests, one per CONFIGS.md row
// ===========================================================================

#[test]
fn cfg_c1_single_node() {
    let mut rng = Rng::new(0xC001);
    for i in 0..ITERS {
        let v = rng.i32();
        let got = diff_values(&[v], &format!("C1 iter={i} v={v}"));
        assert_matches_oracle(&[v], got, "C1");
    }
}

#[test]
fn cfg_c2_len2_min_at_head() {
    let mut rng = Rng::new(0xC002);
    for i in 0..ITERS {
        let a = rng.i32_in(i32::MIN, i32::MAX - 1);
        let b = rng.i32_in(a.saturating_add(1), i32::MAX);
        let vals = [a, b];
        let got = diff_values(&vals, &format!("C2 iter={i} {vals:?}"));
        assert_matches_oracle(&vals, got, "C2");
        assert_eq!(got, a);
    }
}

#[test]
fn cfg_c3_len2_min_at_tail() {
    let mut rng = Rng::new(0xC003);
    for i in 0..ITERS {
        let a = rng.i32_in(i32::MIN + 1, i32::MAX);
        let b = rng.i32_in(i32::MIN, a - 1);
        let vals = [a, b];
        let got = diff_values(&vals, &format!("C3 iter={i} {vals:?}"));
        assert_matches_oracle(&vals, got, "C3");
        assert_eq!(got, b);
    }
}

#[test]
fn cfg_c4_len2_equal() {
    let mut rng = Rng::new(0xC004);
    for i in 0..ITERS {
        let v = rng.i32();
        let vals = [v, v];
        let got = diff_values(&vals, &format!("C4 iter={i} v={v}"));
        assert_matches_oracle(&vals, got, "C4");
    }
}

#[test]
fn cfg_c5_len3_min_interior() {
    let mut rng = Rng::new(0xC005);
    for i in 0..ITERS {
        let m = rng.i32_in(i32::MIN, i32::MAX - 1);
        let a = rng.i32_in(m + 1, i32::MAX);
        let c = rng.i32_in(m + 1, i32::MAX);
        let vals = [a, m, c];
        let got = diff_values(&vals, &format!("C5 iter={i} {vals:?}"));
        assert_matches_oracle(&vals, got, "C5");
        assert_eq!(got, m);
    }
}

#[test]
fn cfg_c6_min_at_head_various_len() {
    let mut rng = Rng::new(0xC006);
    for i in 0..ITERS {
        let n = rng.range(3, 8) as usize;
        let m = rng.i32_in(i32::MIN, i32::MAX - 1);
        let mut vals = vec![m];
        for _ in 1..n {
            vals.push(rng.i32_in(m + 1, i32::MAX));
        }
        let got = diff_values(&vals, &format!("C6 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C6");
        assert_eq!(got, m);
    }
}

#[test]
fn cfg_c7_min_at_tail_various_len() {
    let mut rng = Rng::new(0xC007);
    for i in 0..ITERS {
        let n = rng.range(3, 8) as usize;
        let m = rng.i32_in(i32::MIN, i32::MAX - 1);
        let mut vals: Vec<c_int> = (0..n - 1).map(|_| rng.i32_in(m + 1, i32::MAX)).collect();
        vals.push(m);
        let got = diff_values(&vals, &format!("C7 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C7");
        assert_eq!(got, m);
    }
}

#[test]
fn cfg_c8_min_random_interior() {
    let mut rng = Rng::new(0xC008);
    for i in 0..ITERS {
        let n = rng.range(3, 64) as usize;
        let m = rng.i32_in(i32::MIN, i32::MAX - 1);
        let idx = rng.range(1, n as u64 - 2) as usize;
        let mut vals: Vec<c_int> = (0..n).map(|_| rng.i32_in(m + 1, i32::MAX)).collect();
        vals[idx] = m;
        let got = diff_values(&vals, &format!("C8 iter={i} n={n} idx={idx}"));
        assert_matches_oracle(&vals, got, "C8");
        assert_eq!(got, m);
    }
}

#[test]
fn cfg_c9_duplicate_minima() {
    let mut rng = Rng::new(0xC009);
    for i in 0..ITERS {
        let n = rng.range(2, 32) as usize;
        let m = rng.i32_in(i32::MIN, i32::MAX - 1);
        let mut vals: Vec<c_int> = (0..n).map(|_| rng.i32_in(m + 1, i32::MAX)).collect();
        // Sprinkle the same minimum at 2..=n random indices.
        let dups = rng.range(2, n as u64) as usize;
        for _ in 0..dups {
            let idx = rng.range(0, n as u64 - 1) as usize;
            vals[idx] = m;
        }
        let got = diff_values(&vals, &format!("C9 iter={i} n={n} dups={dups}"));
        assert_matches_oracle(&vals, got, "C9");
        assert_eq!(got, m);
    }
}

#[test]
fn cfg_c10_all_equal() {
    let mut rng = Rng::new(0xC010);
    for i in 0..ITERS {
        let n = rng.range(1, 32) as usize;
        let v = rng.i32();
        let vals = vec![v; n];
        let got = diff_values(&vals, &format!("C10 iter={i} n={n} v={v}"));
        assert_matches_oracle(&vals, got, "C10");
    }
}

#[test]
fn cfg_c11_monotone_increasing() {
    let mut rng = Rng::new(0xC011);
    for i in 0..ITERS {
        let n = rng.range(1, 64) as usize;
        let mut v = rng.i32_in(i32::MIN, 0);
        let mut vals = Vec::with_capacity(n);
        for _ in 0..n {
            vals.push(v);
            v = v.saturating_add(rng.i32_in(1, 1_000_000));
        }
        let got = diff_values(&vals, &format!("C11 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C11");
    }
}

#[test]
fn cfg_c12_monotone_decreasing() {
    let mut rng = Rng::new(0xC012);
    for i in 0..ITERS {
        let n = rng.range(1, 64) as usize;
        let mut v = rng.i32_in(0, i32::MAX);
        let mut vals = Vec::with_capacity(n);
        for _ in 0..n {
            vals.push(v);
            v = v.saturating_sub(rng.i32_in(1, 1_000_000));
        }
        let got = diff_values(&vals, &format!("C12 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C12");
    }
}

#[test]
fn cfg_c13_all_positive() {
    let mut rng = Rng::new(0xC013);
    for i in 0..ITERS {
        let n = rng.range(1, 64) as usize;
        let vals: Vec<c_int> = (0..n).map(|_| rng.i32_in(1, i32::MAX)).collect();
        let got = diff_values(&vals, &format!("C13 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C13");
    }
}

#[test]
fn cfg_c14_all_negative() {
    let mut rng = Rng::new(0xC014);
    for i in 0..ITERS {
        let n = rng.range(1, 64) as usize;
        let vals: Vec<c_int> = (0..n).map(|_| rng.i32_in(i32::MIN, -1)).collect();
        let got = diff_values(&vals, &format!("C14 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C14");
    }
}

#[test]
fn cfg_c15_mixed_sign() {
    let mut rng = Rng::new(0xC015);
    for i in 0..ITERS {
        let n = rng.range(1, 64) as usize;
        let vals: Vec<c_int> = (0..n).map(|_| rng.i32_in(-1000, 1000)).collect();
        let got = diff_values(&vals, &format!("C15 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C15");
    }
}

#[test]
fn cfg_c16_all_zero() {
    let mut rng = Rng::new(0xC016);
    for i in 0..ITERS {
        let n = rng.range(1, 32) as usize;
        let vals = vec![0; n];
        let got = diff_values(&vals, &format!("C16 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C16");
        assert_eq!(got, 0);
    }
}

#[test]
fn cfg_c17_full_random_i32() {
    let mut rng = Rng::new(0xC017);
    for i in 0..ITERS {
        let n = rng.range(1, 128) as usize;
        let vals: Vec<c_int> = (0..n).map(|_| rng.i32()).collect();
        let got = diff_values(&vals, &format!("C17 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C17");
    }
}

#[test]
fn cfg_c18_boundary_value_pool() {
    const POOL: [c_int; 7] = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let mut rng = Rng::new(0xC018);
    for i in 0..ITERS {
        let n = rng.range(1, 24) as usize;
        let vals: Vec<c_int> = (0..n)
            .map(|_| POOL[rng.range(0, POOL.len() as u64 - 1) as usize])
            .collect();
        let got = diff_values(&vals, &format!("C18 iter={i} {vals:?}"));
        assert_matches_oracle(&vals, got, "C18");
    }
}

#[test]
fn cfg_c19_int_min_random_index() {
    let mut rng = Rng::new(0xC019);
    for i in 0..ITERS {
        let n = rng.range(1, 48) as usize;
        let idx = rng.range(0, n as u64 - 1) as usize;
        let mut vals: Vec<c_int> = (0..n).map(|_| rng.i32_in(i32::MIN + 1, i32::MAX)).collect();
        vals[idx] = i32::MIN;
        let got = diff_values(&vals, &format!("C19 iter={i} n={n} idx={idx}"));
        assert_matches_oracle(&vals, got, "C19");
        assert_eq!(got, i32::MIN);
    }
}

#[test]
fn cfg_c20_all_int_max_but_one() {
    let mut rng = Rng::new(0xC020);
    for i in 0..ITERS {
        let n = rng.range(2, 48) as usize;
        let idx = rng.range(0, n as u64 - 1) as usize;
        let m = rng.i32_in(i32::MIN, i32::MAX - 1);
        let mut vals = vec![i32::MAX; n];
        vals[idx] = m;
        let got = diff_values(&vals, &format!("C20 iter={i} n={n} idx={idx} m={m}"));
        assert_matches_oracle(&vals, got, "C20");
        assert_eq!(got, m);
    }
}

#[test]
fn cfg_c21_layout_ascending() {
    let mut rng = Rng::new(0xC021);
    for i in 0..ITERS {
        let n = rng.range(1, 40) as usize;
        let vals: Vec<c_int> = (0..n).map(|_| rng.i32()).collect();
        let chain = chain_ascending(&vals);
        let got = assert_same(chain.head(), &format!("C21 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C21");
    }
}

#[test]
fn cfg_c22_layout_descending() {
    let mut rng = Rng::new(0xC022);
    for i in 0..ITERS {
        let n = rng.range(1, 40) as usize;
        let vals: Vec<c_int> = (0..n).map(|_| rng.i32()).collect();
        let chain = chain_descending(&vals);
        let got = assert_same(chain.head(), &format!("C22 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C22");
    }
}

#[test]
fn cfg_c23_layout_shuffled_boxes() {
    let mut rng = Rng::new(0xC023);
    for i in 0..ITERS {
        let n = rng.range(1, 40) as usize;
        let vals: Vec<c_int> = (0..n).map(|_| rng.i32()).collect();
        let chain = chain_shuffled_boxes(&vals, &mut rng);
        let got = assert_same(chain.head(), &format!("C23 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C23");
    }
}

#[test]
fn cfg_c24_long_list() {
    let mut rng = Rng::new(0xC024);
    let n = 100_000usize;
    for i in 0..3 {
        let vals: Vec<c_int> = (0..n).map(|_| rng.i32()).collect();
        let got = diff_values(&vals, &format!("C24 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C24");
    }
    // Also: minimum at the very last node of a long list.
    let mut vals: Vec<c_int> = (0..n).map(|_| rng.i32_in(0, i32::MAX)).collect();
    *vals.last_mut().unwrap() = i32::MIN;
    let got = diff_values(&vals, "C24 min-at-tail");
    assert_eq!(got, i32::MIN);
}

#[test]
fn cfg_c25_no_mutation_of_input() {
    let mut rng = Rng::new(0xC025);
    for i in 0..ITERS {
        let n = rng.range(1, 32) as usize;
        let vals: Vec<c_int> = (0..n).map(|_| rng.i32()).collect();
        let chain = chain_ascending(&vals);
        let head = chain.head();

        // `assert_same` already calls C, Rust, Rust, C. Now verify the chain
        // itself is bit-identical to what we built.
        let got = assert_same(head, &format!("C25 iter={i} n={n}"));
        assert_matches_oracle(&vals, got, "C25");

        let mut cur = head;
        let mut k = 0usize;
        while !cur.is_null() {
            // SAFETY: cur is a live node from `chain`.
            let node = unsafe { &*cur };
            assert_eq!(node.value, vals[k], "C25: node {k} value was mutated");
            cur = node.next;
            k += 1;
        }
        assert_eq!(k, n, "C25: chain length changed");
    }
}

// ===========================================================================
// PHASE C — error-path differential tests, one per ERRORS.md row
// ===========================================================================

/// E1 / G1: `head == NULL` — the only rejection branch in the C source.
#[test]
fn err_e1_null_head() {
    let im = impls();
    // SAFETY: NULL is explicitly handled by both implementations.
    let c_out = unsafe { (im.c)(std::ptr::null_mut()) };
    let rust_out = unsafe { (im.rust)(std::ptr::null_mut()) };
    assert_eq!(c_out, -1, "C must return the -1 sentinel for NULL");
    assert_eq!(
        rust_out, c_out,
        "NULL sentinel divergence: C={c_out}, Rust={rust_out}"
    );

    // Repeat many times: the sentinel must be stable, not incidental.
    for i in 0..1000 {
        let c2 = unsafe { (im.c)(std::ptr::null_mut()) };
        let r2 = unsafe { (im.rust)(std::ptr::null_mut()) };
        assert_eq!((c2, r2), (-1, -1), "NULL sentinel unstable at iter {i}");
    }
}

/// G2: a zero-length list is only representable as NULL (no count parameter).
#[test]
fn err_g2_zero_length_is_null() {
    let chain = chain_ascending(&[]);
    assert!(chain.head().is_null(), "empty chain must be a NULL head");
    let got = assert_same(chain.head(), "G2 zero-length");
    assert_eq!(got, -1);

    // Same for the other two layout builders.
    assert_eq!(assert_same(chain_descending(&[]).head(), "G2 desc"), -1);
    let mut rng = Rng::new(0x62);
    assert_eq!(
        assert_same(chain_shuffled_boxes(&[], &mut rng).head(), "G2 boxes"),
        -1
    );
}

/// G3: `-1` is a legitimate value that collides with the NULL sentinel.
/// The C code does NOT treat it as an error; Rust must agree.
#[test]
fn err_g3_minus_one_sentinel_collision() {
    let im = impls();
    let null_result = unsafe { (im.c)(std::ptr::null_mut()) };

    // Single node holding -1.
    let got = diff_values(&[-1], "G3 single -1");
    assert_eq!(got, -1);
    assert_eq!(got, null_result, "the -1/NULL ambiguity is preserved");

    // -1 as the minimum among larger values, at many positions/lengths.
    let mut rng = Rng::new(0x63);
    for i in 0..ITERS {
        let n = rng.range(1, 32) as usize;
        let idx = rng.range(0, n as u64 - 1) as usize;
        let mut vals: Vec<c_int> = (0..n).map(|_| rng.i32_in(0, i32::MAX)).collect();
        vals[idx] = -1;
        let got = diff_values(&vals, &format!("G3 iter={i} n={n} idx={idx}"));
        assert_eq!(got, -1);
        assert_matches_oracle(&vals, got, "G3");
    }

    // And a list whose minimum is -2: proves -1 is not special-cased.
    let got = diff_values(&[-1, -2, -1], "G3 min -2");
    assert_eq!(got, -2);
}

/// G4: extreme `int` values — one step past every practical range boundary.
#[test]
fn err_g4_extreme_int_values() {
    let cases: &[&[c_int]] = &[
        &[i32::MIN],
        &[i32::MAX],
        &[i32::MIN, i32::MAX],
        &[i32::MAX, i32::MIN],
        &[i32::MIN, i32::MIN],
        &[i32::MAX, i32::MAX],
        &[i32::MIN + 1, i32::MIN],
        &[i32::MIN, i32::MIN + 1],
        &[i32::MAX - 1, i32::MAX],
        &[i32::MAX, i32::MAX - 1],
        &[0, i32::MIN, 0],
        &[0, i32::MAX, 0],
        &[i32::MIN, 0, i32::MAX],
        &[i32::MAX, 0, i32::MIN],
        &[-1, i32::MIN],
        &[i32::MIN, -1],
    ];
    for (i, vals) in cases.iter().enumerate() {
        let got = diff_values(vals, &format!("G4 case={i} {vals:?}"));
        assert_matches_oracle(vals, got, "G4");
    }
}

/// G5: extremes specifically at head / interior / tail of longer lists.
#[test]
fn err_g5_extremes_by_position() {
    let mut rng = Rng::new(0x65);
    for extreme in [i32::MIN, i32::MAX] {
        for n in 1..=12usize {
            for idx in 0..n {
                let mut vals: Vec<c_int> = (0..n).map(|_| rng.i32_in(-100, 100)).collect();
                vals[idx] = extreme;
                let got =
                    diff_values(&vals, &format!("G5 extreme={extreme} n={n} idx={idx}"));
                assert_matches_oracle(&vals, got, "G5");
            }
        }
    }
}

/// G6: oversized length — the C code caps nothing.
#[test]
fn err_g6_oversized_length() {
    let mut rng = Rng::new(0x66);
    let n = 1_000_000usize;
    let vals: Vec<c_int> = (0..n).map(|_| rng.i32_in(-1_000_000, i32::MAX)).collect();
    let got = diff_values(&vals, "G6 1M nodes");
    assert_matches_oracle(&vals, got, "G6");

    // Extreme value buried deep in the middle of a huge list.
    let mut vals2: Vec<c_int> = vec![7; n];
    vals2[n / 2] = i32::MIN;
    let got2 = diff_values(&vals2, "G6 1M nodes, INT_MIN mid");
    assert_eq!(got2, i32::MIN);
}

/// G7 (documented N/A): there is no enum / mode / flag parameter, so an
/// out-of-range enum value cannot be constructed. The closest analogue is an
/// arbitrary bit pattern in the `int` field, which no "valid variant" restricts;
/// this test drives every byte pattern class through the field.
#[test]
fn err_g7_arbitrary_bit_patterns_in_int_field() {
    let patterns: [u32; 12] = [
        0x0000_0000,
        0xFFFF_FFFF,
        0x8000_0000,
        0x7FFF_FFFF,
        0xAAAA_AAAA,
        0x5555_5555,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0x0000_00FF,
        0xFF00_0000,
        0x0000_0001,
        0xFFFF_FFFE,
    ];
    // Every ordered pair, plus every triple built from the pattern list.
    for (i, &a) in patterns.iter().enumerate() {
        for (j, &b) in patterns.iter().enumerate() {
            let vals = [a as i32, b as i32];
            let got = diff_values(&vals, &format!("G7 pair {i},{j}"));
            assert_matches_oracle(&vals, got, "G7 pair");

            let vals3 = [b as i32, a as i32, b as i32];
            let got3 = diff_values(&vals3, &format!("G7 triple {i},{j}"));
            assert_matches_oracle(&vals3, got3, "G7 triple");
        }
    }
}

// ===========================================================================
// PHASE D — export-surface check performed through dlsym itself
// ===========================================================================

/// The Rust `.so` must export the C `.so`'s complete public symbol set under
/// the exact same names. Verified here at runtime via `dlsym` (the `nm -D` diff
/// is recorded in SYMBOLS.md).
#[test]
fn symbols_rust_so_exports_every_c_symbol() {
    // The C public ABI, from `nm -D --defined-only` on the C .so.
    const C_PUBLIC_SYMBOLS: &[&[u8]] = &[b"smallestValue\0"];

    // SAFETY: plain C-ABI libraries.
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("dlopen C .so");
        let rust_lib = Library::new(rust_so_path()).expect("dlopen Rust .so");

        for sym in C_PUBLIC_SYMBOLS {
            let name = String::from_utf8_lossy(&sym[..sym.len() - 1]).to_string();
            c_lib
                .get::<SmallestValueFn>(sym)
                .unwrap_or_else(|e| panic!("C .so missing {name}: {e}"));
            rust_lib.get::<SmallestValueFn>(sym).unwrap_or_else(|e| {
                panic!("Rust .so does NOT export {name} (symbol parity failure): {e}")
            });
        }
    }
}
