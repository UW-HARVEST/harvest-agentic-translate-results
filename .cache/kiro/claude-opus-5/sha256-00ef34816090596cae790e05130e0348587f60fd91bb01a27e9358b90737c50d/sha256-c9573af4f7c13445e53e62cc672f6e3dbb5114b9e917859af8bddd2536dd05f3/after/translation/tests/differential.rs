// Differential test harness: loads BOTH shared libraries through `libloading`
// and compares `smallestValue` across the FFI boundary.
//
// Neither implementation is ever called directly as Rust code. The Rust side is
// loaded from `target/<profile>/libSimpleList.so` exactly as an external C
// consumer would, so the `#[no_mangle] extern "C"` export wrapper is under test
// too.
//
// Rows referenced as C01..C24 come from CONFIGS.md (Phase B).
// Rows referenced as E1 / G1..G7 come from ERRORS.md (Phase C).

use std::ffi::c_int;
use std::path::PathBuf;
use std::ptr;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// ABI mirror of the C type (c_src/include/simplestruct.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

/// `int smallestValue (struct ListNode *date);`
type SmallestValueFn = unsafe extern "C" fn(*mut ListNode) -> c_int;

// ---------------------------------------------------------------------------
// Loading both .so files
// ---------------------------------------------------------------------------

struct Lib {
    _lib: libloading::Library,
    smallest_value: SmallestValueFn,
}

// The loaded libraries are immutable, stateless code pages: `smallestValue`
// touches no globals in either implementation (verified by inspection of
// c_src/src/simplestruct.c and translation/src/lib.rs), so sharing them across
// test threads is sound.
unsafe impl Sync for Lib {}
unsafe impl Send for Lib {}

impl Lib {
    fn open(path: &PathBuf) -> Lib {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        let sym: libloading::Symbol<SmallestValueFn> = unsafe { lib.get(b"smallestValue\0") }
            .unwrap_or_else(|e| panic!("`smallestValue` not exported by {}: {e}", path.display()));
        let smallest_value = *sym;
        Lib {
            _lib: lib,
            smallest_value,
        }
    }

    fn smallest_value(&self, head: *mut ListNode) -> c_int {
        unsafe { (self.smallest_value)(head) }
    }
}

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // -> working directory root
    p.push("c_src");
    p.push("build");
    p.push("libSimpleList.so");
    assert!(
        p.exists(),
        "C shared library not built at {}. Run:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Path to the Rust shared library, built fresh so it can never be stale.
///
/// `cargo test` does not build the `cdylib` lib target (nothing links it), so the
/// harness builds it itself into a dedicated target directory. A dedicated
/// directory is required: reusing the outer `target/` would deadlock on the
/// cargo build lock held by the running `cargo test`.
///
/// Override with `RUST_SO=/path/to/libSimpleList.so` to test a prebuilt artifact.
fn rust_so_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        if let Ok(p) = std::env::var("RUST_SO") {
            let p = PathBuf::from(p);
            assert!(p.exists(), "RUST_SO={} does not exist", p.display());
            return p;
        }

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let out = manifest.join("target").join("cdylib-under-test");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

        let status = std::process::Command::new(&cargo)
            .current_dir(&manifest)
            .env("CARGO_TARGET_DIR", &out)
            .args(["build", "--release", "--lib"])
            .args(feature_args())
            .status()
            .expect("spawn cargo to build the cdylib under test");
        assert!(status.success(), "building the Rust cdylib failed");

        let p = out.join("release").join("libSimpleList.so");
        assert!(
            p.exists(),
            "Rust cdylib not produced at {} despite a successful build",
            p.display()
        );
        p
    })
    .clone()
}

/// Propagate the feature selection of the running test binary to the cdylib
/// build, so `cargo test --no-default-features --features X` tests the `.so`
/// compiled with those same features.
fn feature_args() -> Vec<String> {
    let mut enabled: Vec<String> = std::env::vars()
        .filter_map(|(k, v)| {
            if v == "1" {
                k.strip_prefix("CARGO_FEATURE_")
                    .map(|f| f.to_lowercase().replace('_', "-"))
            } else {
                None
            }
        })
        .collect();
    enabled.sort();

    // The crate declares no [features]; if that ever changes, pin the build to
    // exactly the features this test binary was compiled with.
    if enabled.is_empty() {
        vec!["--no-default-features".to_string()]
    } else {
        vec![
            "--no-default-features".to_string(),
            "--features".to_string(),
            enabled.join(","),
        ]
    }
}

fn c_lib() -> &'static Lib {
    static L: OnceLock<Lib> = OnceLock::new();
    L.get_or_init(|| Lib::open(&c_so_path()))
}

fn rust_lib() -> &'static Lib {
    static L: OnceLock<Lib> = OnceLock::new();
    L.get_or_init(|| Lib::open(&rust_so_path()))
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed => reproducible property-style tests)
// ---------------------------------------------------------------------------

const SEED: u64 = 0x5EED_1EAF_D00D_F00D;

struct Rng(u64);

impl Rng {
    fn new(stream: u64) -> Rng {
        // Mix the per-row stream id into the global seed so each row gets a
        // distinct but fixed sequence.
        let mut s = SEED ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        if s == 0 {
            s = 0x1234_5678_9ABC_DEF0;
        }
        Rng(s)
    }

    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }

    /// Uniform in `0..n` (n > 0).
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// Uniform in `lo..=hi`.
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }

    /// Uniform in `lo..=hi` over i32, computed in i64 to avoid overflow.
    fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}

// ---------------------------------------------------------------------------
// List builders
// ---------------------------------------------------------------------------

/// Nodes in one contiguous allocation, linked in index order.
struct ContigList {
    nodes: Vec<ListNode>,
}

impl ContigList {
    fn new(values: &[i32]) -> ContigList {
        let mut nodes: Vec<ListNode> = values
            .iter()
            .map(|&v| ListNode {
                value: v as c_int,
                next: ptr::null_mut(),
            })
            .collect();
        // No further pushes, so `as_mut_ptr` stays valid for the whole lifetime.
        let base = nodes.as_mut_ptr();
        for i in 0..nodes.len().saturating_sub(1) {
            nodes[i].next = unsafe { base.add(i + 1) };
        }
        ContigList { nodes }
    }

    fn head(&mut self) -> *mut ListNode {
        if self.nodes.is_empty() {
            ptr::null_mut()
        } else {
            self.nodes.as_mut_ptr()
        }
    }

    /// Pointer to an interior node, i.e. the head of the suffix sub-list.
    fn at(&mut self, i: usize) -> *mut ListNode {
        assert!(i < self.nodes.len());
        unsafe { self.nodes.as_mut_ptr().add(i) }
    }
}

/// Each node in its own heap allocation, allocated in a scrambled order so the
/// logical list order does not match increasing memory addresses. Catches a
/// traversal that walks memory instead of following `next`.
struct ScatteredList {
    /// Kept alive for the duration of the test; the list is reached via `head`.
    #[allow(dead_code)]
    nodes: Vec<Box<ListNode>>,
    #[allow(dead_code)]
    decoys: Vec<Box<[u8; 48]>>,
    head: *mut ListNode,
}

impl ScatteredList {
    fn new(values: &[i32], rng: &mut Rng) -> ScatteredList {
        let n = values.len();
        let mut order: Vec<usize> = (0..n).collect();
        rng.shuffle(&mut order);

        let mut slots: Vec<Option<Box<ListNode>>> = (0..n).map(|_| None).collect();
        let mut decoys: Vec<Box<[u8; 48]>> = Vec::new();
        for &logical in &order {
            // Interleave junk allocations to further scatter the addresses.
            if rng.below(2) == 0 {
                decoys.push(Box::new([0u8; 48]));
            }
            slots[logical] = Some(Box::new(ListNode {
                value: values[logical] as c_int,
                next: ptr::null_mut(),
            }));
        }

        let mut nodes: Vec<Box<ListNode>> = slots.into_iter().map(|s| s.unwrap()).collect();
        let ptrs: Vec<*mut ListNode> = nodes
            .iter_mut()
            .map(|b| (&mut **b) as *mut ListNode)
            .collect();
        for i in 0..n.saturating_sub(1) {
            let (cur, next) = (ptrs[i], ptrs[i + 1]);
            unsafe { (*cur).next = next };
        }

        let head = if n == 0 { ptr::null_mut() } else { ptrs[0] };
        ScatteredList {
            nodes,
            decoys,
            head,
        }
    }

    fn head(&mut self) -> *mut ListNode {
        self.head
    }
}

// ---------------------------------------------------------------------------
// The differential assertion
// ---------------------------------------------------------------------------

/// Calls both `.so` exports on the same list and requires bit-identical `int`
/// results. `expected` is the independent oracle (the minimum computed here) and
/// is only checked for non-empty lists.
#[track_caller]
fn assert_same(row: &str, values: &[i32], head: *mut ListNode, expected: Option<i32>) {
    let c = c_lib().smallest_value(head);
    let r = rust_lib().smallest_value(head);

    assert_eq!(
        c.to_ne_bytes(),
        r.to_ne_bytes(),
        "{row}: C returned {c} but Rust returned {r} for list {}",
        preview(values)
    );

    if let Some(e) = expected {
        assert_eq!(
            c, e,
            "{row}: oracle disagrees with the C ground truth (C={c}, oracle={e}) for list {}",
            preview(values)
        );
    }
}

fn preview(values: &[i32]) -> String {
    if values.len() <= 24 {
        format!("{values:?}")
    } else {
        format!(
            "[{} ... ] (len {})",
            values[..24]
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            values.len()
        )
    }
}

/// Run one CONFIGS.md row: build a contiguous list from `values`, compare.
#[track_caller]
fn check_contig(row: &str, values: &[i32]) {
    let mut list = ContigList::new(values);
    let head = list.head();
    let expected = values.iter().copied().min();
    assert_same(row, values, head, expected);
}

// ===========================================================================
// Phase B — valid-path differential tests, one test per CONFIGS.md row.
// Every row is randomized with a fixed per-row seed stream.
// ===========================================================================

const ITERS: usize = 200;

/// C01 — len 1, random value over the full i32 range (the `while` guard at
/// simplestruct.c:29 is false on entry, so the loop body never executes).
#[test]
fn c01_len1_random_full_range() {
    let mut rng = Rng::new(1);
    for _ in 0..ITERS {
        check_contig("C01", &[rng.next_i32()]);
    }
    for v in [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        check_contig("C01/boundary", &[v]);
    }
}

/// C02 — len 2, minimum at the head: branch @31 never taken.
#[test]
fn c02_len2_min_at_head() {
    let mut rng = Rng::new(2);
    for _ in 0..ITERS {
        let lo = rng.i32_in(i32::MIN, i32::MAX - 1);
        let hi = rng.i32_in(lo.wrapping_add(1), i32::MAX);
        check_contig("C02", &[lo, hi]);
    }
}

/// C03 — len 2, minimum at the tail: branch @31 taken exactly once.
#[test]
fn c03_len2_min_at_tail() {
    let mut rng = Rng::new(3);
    for _ in 0..ITERS {
        let lo = rng.i32_in(i32::MIN, i32::MAX - 1);
        let hi = rng.i32_in(lo.wrapping_add(1), i32::MAX);
        check_contig("C03", &[hi, lo]);
    }
}

/// C04 — len 2, equal values: the strict `<` must NOT fire. Pins the operator
/// to `<` rather than `<=`.
#[test]
fn c04_len2_equal() {
    let mut rng = Rng::new(4);
    for _ in 0..ITERS {
        let v = rng.next_i32();
        check_contig("C04", &[v, v]);
    }
}

/// Builds `len` values with a unique minimum planted at `min_pos`.
fn unique_min_at(rng: &mut Rng, len: usize, min_pos: usize) -> Vec<i32> {
    loop {
        let mut vs: Vec<i32> = (0..len).map(|_| rng.i32_in(-1_000_000, 1_000_000)).collect();
        let m = *vs.iter().min().unwrap();
        vs[min_pos] = m - 1 - rng.below(1000) as i32;
        let mut sorted = vs.clone();
        sorted.sort_unstable();
        if sorted[0] != sorted[1] {
            return vs;
        }
    }
}

/// C05 — len 3..=8, unique minimum at the head.
#[test]
fn c05_unique_min_at_head() {
    let mut rng = Rng::new(5);
    for _ in 0..ITERS {
        let len = rng.range(3, 8);
        let vs = unique_min_at(&mut rng, len, 0);
        check_contig("C05", &vs);
    }
}

/// C06 — len 3..=8, unique minimum strictly in the middle.
#[test]
fn c06_unique_min_in_middle() {
    let mut rng = Rng::new(6);
    for _ in 0..ITERS {
        let len = rng.range(3, 8);
        let pos = rng.range(1, len - 2);
        let vs = unique_min_at(&mut rng, len, pos);
        check_contig("C06", &vs);
    }
}

/// C07 — len 3..=8, unique minimum at the tail: branch @31 fires on the final
/// iteration.
#[test]
fn c07_unique_min_at_tail() {
    let mut rng = Rng::new(7);
    for _ in 0..ITERS {
        let len = rng.range(3, 8);
        let vs = unique_min_at(&mut rng, len, len - 1);
        check_contig("C07", &vs);
    }
}

/// C08 — minimum duplicated at two distinct positions: the second occurrence
/// must not re-trigger the strict `<`.
#[test]
fn c08_duplicated_min() {
    let mut rng = Rng::new(8);
    for _ in 0..ITERS {
        let len = rng.range(3, 8);
        let mut vs = unique_min_at(&mut rng, len, 0);
        let m = vs[0];
        let mut i = rng.below(len);
        let mut j = rng.below(len);
        if i == j {
            j = (j + 1) % len;
        }
        if i > j {
            std::mem::swap(&mut i, &mut j);
        }
        vs[i] = m;
        vs[j] = m;
        check_contig("C08", &vs);
    }
}

/// C09 — len 3..=8, every element equal.
#[test]
fn c09_all_equal() {
    let mut rng = Rng::new(9);
    for _ in 0..ITERS {
        let len = rng.range(3, 8);
        let v = rng.next_i32();
        check_contig("C09", &vec![v; len]);
    }
}

/// C10 — len 3..=64 strictly ascending: branch @31 never fires after seeding.
#[test]
fn c10_strictly_ascending() {
    let mut rng = Rng::new(10);
    for _ in 0..ITERS {
        let len = rng.range(3, 64);
        let mut v = rng.i32_in(-1_000_000, 0) as i64;
        let mut vs = Vec::with_capacity(len);
        for _ in 0..len {
            vs.push(v as i32);
            v += 1 + rng.below(10_000) as i64;
        }
        check_contig("C10", &vs);
    }
}

/// C11 — len 3..=64 strictly descending: branch @31 fires every iteration.
#[test]
fn c11_strictly_descending() {
    let mut rng = Rng::new(11);
    for _ in 0..ITERS {
        let len = rng.range(3, 64);
        let mut v = rng.i32_in(0, 1_000_000) as i64;
        let mut vs = Vec::with_capacity(len);
        for _ in 0..len {
            vs.push(v as i32);
            v -= 1 + rng.below(10_000) as i64;
        }
        check_contig("C11", &vs);
    }
}

/// C12 — len 1..=64, all values non-negative.
#[test]
fn c12_all_non_negative() {
    let mut rng = Rng::new(12);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let vs: Vec<i32> = (0..len).map(|_| rng.i32_in(0, i32::MAX)).collect();
        check_contig("C12", &vs);
    }
}

/// C13 — len 1..=64, all values negative.
#[test]
fn c13_all_negative() {
    let mut rng = Rng::new(13);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let vs: Vec<i32> = (0..len).map(|_| rng.i32_in(i32::MIN, -1)).collect();
        check_contig("C13", &vs);
    }
}

/// C14 — len 1..=64, uniform over the full i32 range (mixed sign).
#[test]
fn c14_full_range_mixed_sign() {
    let mut rng = Rng::new(14);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let vs: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        check_contig("C14", &vs);
    }
}

/// C15 — values from {-1, 0, 1}: dense ties, and `-1` collides with the NULL
/// sentinel of ERRORS.md E1.
#[test]
fn c15_narrow_alphabet() {
    let mut rng = Rng::new(15);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let vs: Vec<i32> = (0..len).map(|_| [-1i32, 0, 1][rng.below(3)]).collect();
        check_contig("C15", &vs);
    }
}

/// C16 — `i32::MIN` planted at a random position.
#[test]
fn c16_int_min_planted() {
    let mut rng = Rng::new(16);
    for _ in 0..ITERS {
        let len = rng.range(2, 64);
        let mut vs: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        let pos = rng.below(len);
        vs[pos] = i32::MIN;
        check_contig("C16", &vs);
    }
}

/// C17 — all `i32::MAX` except one smaller value: `value < smallest` must be a
/// plain signed compare, not a subtraction that would overflow.
#[test]
fn c17_int_max_saturated() {
    let mut rng = Rng::new(17);
    for _ in 0..ITERS {
        let len = rng.range(2, 64);
        let mut vs: Vec<i32> = vec![i32::MAX; len];
        let pos = rng.below(len);
        vs[pos] = rng.next_i32();
        check_contig("C17", &vs);
    }
}

/// C18 — values drawn only from {i32::MIN, i32::MAX}: every comparison is
/// between the two extremes, the worst case for an overflow-prone compare.
#[test]
fn c18_extremes_only() {
    let mut rng = Rng::new(18);
    for _ in 0..ITERS {
        let len = rng.range(2, 64);
        let vs: Vec<i32> = (0..len)
            .map(|_| if rng.below(2) == 0 { i32::MIN } else { i32::MAX })
            .collect();
        check_contig("C18", &vs);
    }
}

/// C19 — len 1..=64, all zeros.
#[test]
fn c19_all_zero() {
    let mut rng = Rng::new(19);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        check_contig("C19", &vec![0i32; len]);
    }
}

/// C20 — exactly one `-1`, everything else strictly greater: the successful
/// result is `-1`, indistinguishable from the E1 NULL sentinel.
#[test]
fn c20_result_is_sentinel_value() {
    let mut rng = Rng::new(20);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let mut vs: Vec<i32> = (0..len).map(|_| rng.i32_in(0, i32::MAX)).collect();
        let pos = rng.below(len);
        vs[pos] = -1;

        let mut list = ContigList::new(&vs);
        let head = list.head();
        assert_same("C20", &vs, head, Some(-1));

        assert_eq!(c_lib().smallest_value(ptr::null_mut()), -1);
        assert_eq!(rust_lib().smallest_value(ptr::null_mut()), -1);
    }
}

/// C21 — long lists, 100..=1000 nodes.
#[test]
fn c21_long_lists() {
    let mut rng = Rng::new(21);
    for _ in 0..40 {
        let len = rng.range(100, 1000);
        let vs: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        check_contig("C21", &vs);
    }
}

/// C22 — oversized list, 100_000 nodes. The C traversal is an iterative loop;
/// a recursive Rust translation would overflow the stack here.
#[test]
fn c22_oversized_list() {
    let mut rng = Rng::new(22);
    for _ in 0..3 {
        let vs: Vec<i32> = (0..100_000).map(|_| rng.next_i32()).collect();
        check_contig("C22", &vs);
    }
    let mut vs: Vec<i32> = vec![0; 100_000];
    *vs.last_mut().unwrap() = i32::MIN;
    check_contig("C22/tail-min", &vs);
}

/// C23 — nodes in separate, scrambled heap allocations: traversal must follow
/// `next`, not ascending addresses.
#[test]
fn c23_scattered_allocations() {
    let mut rng = Rng::new(23);
    for _ in 0..ITERS {
        let len = rng.range(1, 64);
        let vs: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        let mut list = ScatteredList::new(&vs, &mut rng);
        let head = list.head();
        let expected = vs.iter().copied().min();
        assert_same("C23", &vs, head, expected);
    }
}

/// C24 — the caller passes an interior node. That is a valid null-terminated
/// list, so only the suffix is scanned.
#[test]
fn c24_interior_head_pointer() {
    let mut rng = Rng::new(24);
    for _ in 0..ITERS {
        let len = rng.range(3, 64);
        let vs: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        let start = rng.range(1, len - 1);

        let mut list = ContigList::new(&vs);
        let head = list.at(start);
        let expected = vs[start..].iter().copied().min();
        assert_same("C24", &vs[start..], head, expected);
    }
}

// ===========================================================================
// Phase C — error-path differential tests, one per ERRORS.md row.
// ===========================================================================

/// E1 / G1 — `head == NULL`: the `if (head)` test at simplestruct.c:27 is false
/// and control reaches `else return -1;`. Both libraries must return exactly
/// `-1` — the same sentinel, not merely "both failed".
#[test]
fn errors_e1_null_head_returns_minus_one() {
    let c = c_lib().smallest_value(ptr::null_mut());
    let r = rust_lib().smallest_value(ptr::null_mut());
    assert_eq!(c, -1, "C ground truth for NULL head must be -1, got {c}");
    assert_eq!(r, c, "Rust returned {r} for NULL head, C returned {c}");

    // Repeat: the function is stateless, so the sentinel must be stable across
    // calls and independent of any preceding successful call.
    let mut warmup = ContigList::new(&[7, 3, 9]);
    let h = warmup.head();
    assert_eq!(c_lib().smallest_value(h), rust_lib().smallest_value(h));
    for _ in 0..64 {
        assert_eq!(c_lib().smallest_value(ptr::null_mut()), -1);
        assert_eq!(rust_lib().smallest_value(ptr::null_mut()), -1);
    }
}

/// G2 — a zero-length list is not representable as anything other than the null
/// pointer, so "empty" and "NULL" are the same input. Asserted explicitly so the
/// equivalence is recorded rather than assumed.
#[test]
fn errors_g2_zero_length_is_null() {
    let mut empty = ContigList::new(&[]);
    let head = empty.head();
    assert!(head.is_null(), "an empty list must degenerate to NULL");
    assert_eq!(c_lib().smallest_value(head), -1);
    assert_eq!(rust_lib().smallest_value(head), -1);
    assert_eq!(
        c_lib().smallest_value(head),
        rust_lib().smallest_value(head)
    );
}

/// G3 — length 1: `head->next` is NULL so the loop body never runs and the head
/// value is returned verbatim, for every value in the domain.
#[test]
fn errors_g3_single_node_randomized() {
    let mut rng = Rng::new(103);
    for _ in 0..ITERS {
        let v = rng.next_i32();
        let mut list = ContigList::new(&[v]);
        let head = list.head();
        assert_same("G3", &[v], head, Some(v));
    }
    for v in [i32::MIN, -1, 0, i32::MAX] {
        let mut list = ContigList::new(&[v]);
        let head = list.head();
        assert_same("G3/boundary", &[v], head, Some(v));
    }
}

/// G4 — `-1` as a legitimate payload. The success result must be bit-identical
/// to the NULL-input error result: the C API genuinely cannot distinguish them,
/// and the Rust must reproduce that ambiguity rather than "fixing" it.
#[test]
fn errors_g4_minus_one_payload_aliases_sentinel() {
    let null_c = c_lib().smallest_value(ptr::null_mut());
    let null_r = rust_lib().smallest_value(ptr::null_mut());

    for vs in [
        vec![-1i32],
        vec![-1, -1],
        vec![0, -1],
        vec![-1, 0],
        vec![5, 5, -1, 5],
        vec![i32::MAX, -1],
    ] {
        let mut list = ContigList::new(&vs);
        let head = list.head();
        let c = c_lib().smallest_value(head);
        let r = rust_lib().smallest_value(head);
        assert_eq!(c, r, "G4: divergence on {vs:?} (C={c}, Rust={r})");
        assert_eq!(c, -1, "G4: C ground truth should be -1 for {vs:?}");
        assert_eq!(c, null_c);
        assert_eq!(r, null_r);
    }
}

/// G5 — the full `int` range is valid input; there is no range check in the C,
/// so the extremes must round-trip and the comparison must not overflow.
#[test]
fn errors_g5_int_extremes() {
    let extremes = [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 1,
        i32::MAX,
    ];
    // Every ordered pair of extremes, both orders.
    for &a in &extremes {
        for &b in &extremes {
            let vs = [a, b];
            let mut list = ContigList::new(&vs);
            let head = list.head();
            assert_same("G5/pair", &vs, head, Some(a.min(b)));
        }
    }
    // Every ordered triple.
    for &a in &extremes {
        for &b in &extremes {
            for &c in &extremes {
                let vs = [a, b, c];
                let mut list = ContigList::new(&vs);
                let head = list.head();
                assert_same("G5/triple", &vs, head, Some(a.min(b).min(c)));
            }
        }
    }
}

/// G6 — oversized length. No length limit or counter exists in the C, so a
/// large list is valid input, not an error; both must agree and neither may
/// overflow a counter or the stack.
#[test]
fn errors_g6_oversized_list() {
    let mut rng = Rng::new(106);
    let len = 250_000;
    let mut vs: Vec<i32> = (0..len).map(|_| rng.i32_in(-5, i32::MAX)).collect();
    // Minimum in the final node, so the whole chain must be walked.
    vs[len - 1] = i32::MIN;
    check_contig("G6", &vs);

    // Same size, minimum in the first node: branch @31 never fires across
    // 250k iterations.
    let mut vs2: Vec<i32> = (0..len).map(|_| rng.i32_in(0, i32::MAX)).collect();
    vs2[0] = i32::MIN;
    check_contig("G6/head-min", &vs2);
}

/// G7 — there is no enum and no integer mode/flag parameter anywhere in the
/// public API, so there is no out-of-range discriminant to pass across the FFI
/// boundary. This test records that fact mechanically against the header rather
/// than leaving the row silently unaddressed: the sole parameter is a pointer.
#[test]
fn errors_g7_no_enum_or_int_parameter_surface() {
    let header = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/simplestruct.h"),
    )
    .expect("read simplestruct.h");

    assert!(
        !header.contains("enum"),
        "the public header declares an enum; ERRORS.md row G7 must be replaced \
         with real out-of-range-discriminant tests"
    );
    assert!(
        header.contains("int smallestValue (struct ListNode *date);"),
        "the public API changed; regenerate ERRORS.md and CONFIGS.md"
    );
    // The only integer in the signature is the return value, so the only
    // scalar an attacker controls is `value` inside the nodes — exhausted by
    // G5 and by CONFIGS rows C16..C18.
}

/// Pointer-shaped abuse that is *defined* behavior: a self-consistent list
/// whose tail `next` is NULL is the only contract the C relies on. A NULL head
/// reached through a valid parent is still just the loop-termination condition.
#[test]
fn errors_null_next_terminates_identically() {
    let mut rng = Rng::new(107);
    for _ in 0..ITERS {
        let len = rng.range(1, 32);
        let vs: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        let mut list = ContigList::new(&vs);
        // Truncate the chain at a random node: the suffix becomes unreachable.
        let cut = rng.below(len);
        unsafe { (*list.at(cut)).next = ptr::null_mut() };
        let head = list.head();
        let expected = vs[..=cut].iter().copied().min();
        assert_same("truncated", &vs[..=cut], head, expected);
    }
}

// ===========================================================================
// Phase D — symbol parity asserted from inside the test suite.
// ===========================================================================

/// The defined, non-toolchain dynamic symbols of the two libraries must be the
/// same set, and the Rust library must have no undefined symbols outside
/// libc/the unwinder.
#[test]
fn symbol_parity() {
    fn defined_symbols(path: &PathBuf) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .arg("-D")
            .arg("--defined-only")
            .arg(path)
            .output()
            .expect("run nm");
        assert!(
            out.status.success(),
            "nm failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let (a, b) = (it.next(), it.next());
                match (a, b) {
                    // "<addr> T name"
                    (Some(_), Some(kind)) if kind.len() == 1 => {
                        it.next().map(|n| (kind.to_string(), n.to_string()))
                    }
                    // "w name" (weak, no address)
                    (Some(kind), Some(n)) if kind.len() == 1 => {
                        Some((kind.to_string(), n.to_string()))
                    }
                    _ => None,
                }
            })
            .filter(|(kind, name)| {
                // Drop toolchain-provided plumbing present in every .so.
                kind != "w"
                    && !name.starts_with("_ITM_")
                    && !name.starts_with("__cxa_")
                    && name != "__gmon_start__"
            })
            .map(|(_, name)| name)
            .collect();
        syms.sort();
        syms.dedup();
        syms
    }

    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());

    assert_eq!(c, vec!["smallestValue".to_string()], "C export set changed");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );
    assert_eq!(c, r, "export sets differ (C={c:?}, Rust={r:?})");
}

/// Guards the harness itself: the two libraries under comparison must be two
/// distinct files, one produced by cmake from `c_src/`, one produced by cargo
/// from `translation/src/`. Without this it would be possible to "pass" every
/// test by accidentally loading the same `.so` twice.
#[test]
fn harness_compares_two_distinct_libraries() {
    let c = c_so_path();
    let r = rust_so_path();
    assert_ne!(c, r, "both sides resolved to the same file: {}", c.display());
    assert!(c.exists() && r.exists());
    assert!(
        c.components().any(|p| p.as_os_str() == "c_src"),
        "the C side must come from c_src/, got {}",
        c.display()
    );
    // Both must actually export the symbol; Lib::open panics otherwise.
    let _ = c_lib();
    let _ = rust_lib();
}
