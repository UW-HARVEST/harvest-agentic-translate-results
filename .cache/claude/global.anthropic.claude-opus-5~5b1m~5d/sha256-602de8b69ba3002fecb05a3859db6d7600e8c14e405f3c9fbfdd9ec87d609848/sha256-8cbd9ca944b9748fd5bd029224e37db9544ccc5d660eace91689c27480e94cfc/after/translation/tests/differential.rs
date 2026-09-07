//! Differential tests: C `libSimpleList.so` vs. Rust `libSimpleList.so`.
//!
//! BOTH libraries are loaded via `libloading` and called only through their
//! exported `smallestValue` symbol, so the Rust `#[no_mangle] extern "C"`
//! wrapper is exercised exactly as an external C consumer would exercise it.
//!
//! Phase B rows come from `CONFIGS.md`, Phase C rows from `ERRORS.md`.

use libloading::{Library, Symbol};
use std::path::PathBuf;

/// Must match `struct ListNode` in `c_src/include/simplestruct.h`:
/// ```c
/// struct ListNode { int value; struct ListNode* next; };
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ListNode {
    value: i32,
    next: *mut ListNode,
}

type SmallestValueFn = unsafe extern "C" fn(*mut ListNode) -> i32;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    crate_root()
        .parent()
        .expect("crate root has a parent")
        .join("c_src/build/libSimpleList.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib under test. Built with `cargo build --release`.
    let release = crate_root().join("target/release/libSimpleList.so");
    if release.exists() {
        return release;
    }
    crate_root().join("target/debug/libSimpleList.so")
}

/// Holds both libraries alive for the duration of a test.
struct Pair {
    c: Library,
    r: Library,
}

impl Pair {
    fn load() -> Pair {
        let cp = c_lib_path();
        let rp = rust_lib_path();
        assert!(
            cp.exists(),
            "C shared library missing at {cp:?}; build it with \
             `cd c_src && mkdir -p build && cd build && cmake .. \
             -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`"
        );
        assert!(
            rp.exists(),
            "Rust shared library missing at {rp:?}; build it with \
             `cd translation && cargo build --release`"
        );
        unsafe {
            Pair {
                c: Library::new(&cp).expect("load C .so"),
                r: Library::new(&rp).expect("load Rust .so"),
            }
        }
    }

    fn funcs(&self) -> (Symbol<'_, SmallestValueFn>, Symbol<'_, SmallestValueFn>) {
        unsafe {
            (
                self.c.get(b"smallestValue\0").expect("C smallestValue"),
                self.r.get(b"smallestValue\0").expect("Rust smallestValue"),
            )
        }
    }
}

/// Deterministic PRNG (xorshift64*) so every row is reproducible.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[0, n)`.
    fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    fn in_range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        // inclusive lo, inclusive hi
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

/// A list stored as one contiguous `Vec<ListNode>` and linked head -> tail.
/// Kept in a `Box` so node addresses are stable while the C/Rust calls run.
struct ContiguousList {
    nodes: Vec<ListNode>,
}

impl ContiguousList {
    fn new(values: &[i32]) -> ContiguousList {
        let mut nodes: Vec<ListNode> = values
            .iter()
            .map(|&value| ListNode {
                value,
                next: std::ptr::null_mut(),
            })
            .collect();
        // Link after the Vec's buffer is final so addresses do not move.
        for i in 0..nodes.len().saturating_sub(1) {
            let next: *mut ListNode = &mut nodes[i + 1];
            nodes[i].next = next;
        }
        ContiguousList { nodes }
    }

    fn head(&mut self) -> *mut ListNode {
        if self.nodes.is_empty() {
            std::ptr::null_mut()
        } else {
            &mut self.nodes[0]
        }
    }

    fn node_at(&mut self, idx: usize) -> *mut ListNode {
        &mut self.nodes[idx]
    }

    fn values(&self) -> Vec<i32> {
        self.nodes.iter().map(|n| n.value).collect()
    }
}

/// A list whose nodes are individually heap-allocated (addresses in arbitrary
/// order), to prove traversal follows `next` rather than memory layout.
struct BoxedList {
    // Kept alive; order in this Vec is the *allocation* order, not list order.
    boxes: Vec<Box<ListNode>>,
    head: *mut ListNode,
}

impl BoxedList {
    /// `values` is the list order; `link_order` is a permutation of indices
    /// giving the allocation order.
    fn new(values: &[i32], link_order: &[usize]) -> BoxedList {
        assert_eq!(values.len(), link_order.len());
        // Allocate in `link_order` order, remembering where each list position
        // landed.
        let mut boxes: Vec<Box<ListNode>> = Vec::with_capacity(values.len());
        let mut ptr_of_pos: Vec<*mut ListNode> = vec![std::ptr::null_mut(); values.len()];
        for &pos in link_order {
            let mut b = Box::new(ListNode {
                value: values[pos],
                next: std::ptr::null_mut(),
            });
            ptr_of_pos[pos] = &mut *b as *mut ListNode;
            boxes.push(b);
        }
        for pos in 0..values.len().saturating_sub(1) {
            unsafe {
                (*ptr_of_pos[pos]).next = ptr_of_pos[pos + 1];
            }
        }
        let head = if values.is_empty() {
            std::ptr::null_mut()
        } else {
            ptr_of_pos[0]
        };
        BoxedList { boxes, head }
    }

    fn head(&self) -> *mut ListNode {
        self.head
    }
}

impl Drop for BoxedList {
    fn drop(&mut self) {
        self.boxes.clear();
    }
}

/// The oracle: what the C code computes, restated independently.
/// Used only as a cross-check that the *test data* is what we think it is;
/// the authoritative assertion is always C-vs-Rust equality.
fn expected(values: &[i32]) -> i32 {
    match values.first() {
        None => -1,
        Some(&first) => {
            let mut smallest = first;
            for &v in &values[1..] {
                if v < smallest {
                    smallest = v;
                }
            }
            smallest
        }
    }
}

/// Core differential assertion for a contiguous list of `values`.
#[track_caller]
fn assert_same(pair: &Pair, values: &[i32]) {
    let (cf, rf) = pair.funcs();
    let mut cl = ContiguousList::new(values);
    let head = cl.head();
    let c_out = unsafe { cf(head) };
    let r_out = unsafe { rf(head) };
    assert_eq!(
        c_out, r_out,
        "C/Rust divergence: C={c_out} Rust={r_out} for values={:?} (len {})",
        &values[..values.len().min(24)],
        values.len()
    );
    assert_eq!(
        c_out,
        expected(values),
        "C disagreed with the restated oracle for values={:?}",
        &values[..values.len().min(24)]
    );
    // The C function does not mutate the list; verify the Rust call did not
    // either (list is shared between both calls, so any mutation would show).
    assert_eq!(cl.values(), values, "list contents were mutated");
}

/// Differential assertion given an explicit head pointer.
#[track_caller]
fn assert_same_ptr(pair: &Pair, head: *mut ListNode, label: &str) {
    let (cf, rf) = pair.funcs();
    let c_out = unsafe { cf(head) };
    let r_out = unsafe { rf(head) };
    assert_eq!(c_out, r_out, "C/Rust divergence ({label}): C={c_out} Rust={r_out}");
}

// ---------------------------------------------------------------------------
// Phase B — CONFIGS.md rows
// ---------------------------------------------------------------------------

/// CONFIGS row 1 — NULL head.
#[test]
fn cfg_row01_null() {
    let pair = Pair::load();
    assert_same_ptr(&pair, std::ptr::null_mut(), "null head");
    let (cf, rf) = pair.funcs();
    // Repeated to catch any hidden state.
    for _ in 0..64 {
        let c = unsafe { cf(std::ptr::null_mut()) };
        let r = unsafe { rf(std::ptr::null_mut()) };
        assert_eq!(c, -1);
        assert_eq!(c, r);
    }
}

/// CONFIGS row 2 — length 1, randomized over the full `i32` range.
#[test]
fn cfg_row02_len1_random() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0201);
    for _ in 0..2000 {
        assert_same(&pair, &[rng.next_i32()]);
    }
    // Hand-picked edges too.
    for v in [i32::MIN, -2, -1, 0, 1, 2, i32::MAX] {
        assert_same(&pair, &[v]);
    }
}

/// CONFIGS row 3 — length 2, randomized (min at head and min at tail).
#[test]
fn cfg_row03_len2_random() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0302);
    for _ in 0..2000 {
        assert_same(&pair, &[rng.next_i32(), rng.next_i32()]);
    }
    for a in [i32::MIN, -1, 0, 1, i32::MAX] {
        for b in [i32::MIN, -1, 0, 1, i32::MAX] {
            assert_same(&pair, &[a, b]);
        }
    }
}

/// CONFIGS row 4 — length 3..=32, randomized mixed-sign values.
#[test]
fn cfg_row04_small_random() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0403);
    for _ in 0..1500 {
        let len = 3 + rng.below(30);
        let values: Vec<i32> = (0..len).map(|_| rng.in_range_i32(-1000, 1000)).collect();
        assert_same(&pair, &values);
    }
}

/// CONFIGS row 5 — length 33..=512, full-range values.
#[test]
fn cfg_row05_medium_random() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0504);
    for _ in 0..300 {
        let len = 33 + rng.below(480);
        let values: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        assert_same(&pair, &values);
    }
}

/// CONFIGS row 6 — minimum forced at the HEAD (the `if` body never executes).
#[test]
fn cfg_row06_min_at_head() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0605);
    for _ in 0..1000 {
        let len = 1 + rng.below(64);
        let min = rng.in_range_i32(-500_000, 0);
        let mut values: Vec<i32> = vec![min];
        for _ in 1..len {
            values.push(rng.in_range_i32(min + 1, i32::MAX));
        }
        assert_same(&pair, &values);
    }
}

/// CONFIGS row 7 — minimum forced at the TAIL (`if` taken on the last iter).
#[test]
fn cfg_row07_min_at_tail() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0706);
    for _ in 0..1000 {
        let len = 2 + rng.below(64);
        let min = rng.in_range_i32(-500_000, 0);
        let mut values: Vec<i32> = (0..len - 1)
            .map(|_| rng.in_range_i32(min + 1, i32::MAX))
            .collect();
        values.push(min);
        assert_same(&pair, &values);
    }
}

/// CONFIGS row 8 — minimum at a random INTERIOR position.
#[test]
fn cfg_row08_min_interior() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0807);
    for _ in 0..1000 {
        let len = 3 + rng.below(64);
        let min = rng.in_range_i32(-500_000, 0);
        let mut values: Vec<i32> = (0..len).map(|_| rng.in_range_i32(min + 1, i32::MAX)).collect();
        let pos = 1 + rng.below(len - 2);
        values[pos] = min;
        assert_same(&pair, &values);
    }
}

/// CONFIGS row 9 — duplicate minima (`<` vs `<=` distinction).
#[test]
fn cfg_row09_duplicate_minima() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0908);
    for _ in 0..1000 {
        let len = 3 + rng.below(32);
        let min = rng.in_range_i32(-100_000, 100_000);
        let mut values: Vec<i32> = (0..len)
            .map(|_| rng.in_range_i32(min.saturating_add(1), i32::MAX))
            .collect();
        // Plant the same minimum at two or three distinct positions.
        let dupes = 2 + rng.below(2);
        for _ in 0..dupes {
            let pos = rng.below(len);
            values[pos] = min;
        }
        assert_same(&pair, &values);
    }
}

/// CONFIGS row 10 — all nodes identical.
#[test]
fn cfg_row10_all_equal() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0a09);
    for _ in 0..600 {
        let len = 1 + rng.below(80);
        let v = rng.next_i32();
        assert_same(&pair, &vec![v; len]);
    }
    for v in [i32::MIN, -1, 0, i32::MAX] {
        assert_same(&pair, &vec![v; 17]);
    }
}

/// CONFIGS row 11 — strictly ascending (`if` never taken).
#[test]
fn cfg_row11_ascending() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0b0a);
    for _ in 0..600 {
        let len = 2 + rng.below(80);
        let mut v = rng.in_range_i32(-1_000_000, 0);
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(v);
            v = v.saturating_add(rng.in_range_i32(1, 1000));
        }
        assert_same(&pair, &values);
    }
}

/// CONFIGS row 12 — strictly descending (`if` taken every iteration).
#[test]
fn cfg_row12_descending() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0c0b);
    for _ in 0..600 {
        let len = 2 + rng.below(80);
        let mut v = rng.in_range_i32(0, 1_000_000);
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(v);
            v = v.saturating_sub(rng.in_range_i32(1, 1000));
        }
        assert_same(&pair, &values);
    }
}

/// CONFIGS row 13 — all positive.
#[test]
fn cfg_row13_all_positive() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0d0c);
    for _ in 0..800 {
        let len = 1 + rng.below(64);
        let values: Vec<i32> = (0..len).map(|_| rng.in_range_i32(1, i32::MAX)).collect();
        assert_same(&pair, &values);
    }
}

/// CONFIGS row 14 — all negative.
#[test]
fn cfg_row14_all_negative() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0e0d);
    for _ in 0..800 {
        let len = 1 + rng.below(64);
        let values: Vec<i32> = (0..len).map(|_| rng.in_range_i32(i32::MIN, -1)).collect();
        assert_same(&pair, &values);
    }
}

/// CONFIGS row 15 — values restricted to `{-1, 0, 1}`: the result can equal the
/// NULL sentinel `-1` while being a perfectly valid answer.
#[test]
fn cfg_row15_sentinel_adjacent() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0f0e);
    for _ in 0..2000 {
        let len = 1 + rng.below(32);
        let values: Vec<i32> = (0..len).map(|_| rng.in_range_i32(-1, 1)).collect();
        assert_same(&pair, &values);
    }
}

/// CONFIGS row 16 — boundary values mixed randomly.
#[test]
fn cfg_row16_boundary_values() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x100f);
    const POOL: [i32; 8] = [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
    ];
    for _ in 0..2000 {
        let len = 1 + rng.below(24);
        let values: Vec<i32> = (0..len).map(|_| POOL[rng.below(POOL.len())]).collect();
        assert_same(&pair, &values);
    }
}

/// CONFIGS row 17 — `INT_MIN` placed at every position of a fixed-length list.
#[test]
fn cfg_row17_int_min_each_position() {
    let pair = Pair::load();
    for len in 1..=40usize {
        for pos in 0..len {
            let mut values: Vec<i32> = (0..len).map(|i| (i as i32) * 7 - 3).collect();
            values[pos] = i32::MIN;
            assert_same(&pair, &values);
        }
    }
}

/// CONFIGS row 18 — all `INT_MAX` (largest possible minimum).
#[test]
fn cfg_row18_all_int_max() {
    let pair = Pair::load();
    for len in [1usize, 2, 3, 17, 64, 257] {
        assert_same(&pair, &vec![i32::MAX; len]);
    }
}

/// CONFIGS row 19 — individually heap-allocated nodes in shuffled address
/// order: traversal must follow `next`, never memory layout.
#[test]
fn cfg_row19_shuffled_addresses() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1110);
    for _ in 0..400 {
        let len = 1 + rng.below(64);
        let values: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        // Fisher-Yates shuffle of the allocation order.
        let mut order: Vec<usize> = (0..len).collect();
        for i in (1..len).rev() {
            let j = rng.below(i + 1);
            order.swap(i, j);
        }
        let bl = BoxedList::new(&values, &order);
        let (cf, rf) = pair.funcs();
        let c_out = unsafe { cf(bl.head()) };
        let r_out = unsafe { rf(bl.head()) };
        assert_eq!(c_out, r_out, "shuffled-address divergence for {values:?}");
        assert_eq!(c_out, expected(&values), "C vs oracle for {values:?}");
    }
}

/// CONFIGS row 20 — long list (10_000 nodes).
#[test]
fn cfg_row20_long_list() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1211);
    for _ in 0..5 {
        let values: Vec<i32> = (0..10_000).map(|_| rng.next_i32()).collect();
        assert_same(&pair, &values);
    }
    // Minimum at the very last node of a long list.
    let mut values: Vec<i32> = (0..10_000).map(|_| rng.in_range_i32(0, i32::MAX)).collect();
    *values.last_mut().unwrap() = i32::MIN;
    assert_same(&pair, &values);
    // Minimum at the very first node of a long list.
    let mut values: Vec<i32> = (0..10_000).map(|_| rng.in_range_i32(0, i32::MAX)).collect();
    values[0] = i32::MIN;
    assert_same(&pair, &values);
}

/// CONFIGS row 21 — start traversal from an interior node (sub-list).
#[test]
fn cfg_row21_start_midlist() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1312);
    let (cf, rf) = pair.funcs();
    for _ in 0..500 {
        let len = 2 + rng.below(48);
        let values: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        let mut cl = ContiguousList::new(&values);
        let start = rng.below(len);
        let head = cl.node_at(start);
        let c_out = unsafe { cf(head) };
        let r_out = unsafe { rf(head) };
        assert_eq!(c_out, r_out, "sub-list divergence at start={start} for {values:?}");
        assert_eq!(c_out, expected(&values[start..]), "C vs oracle for sub-list");
    }
}

/// CONFIGS row 22 — repeated calls are pure and leave the list untouched.
#[test]
fn cfg_row22_idempotent_and_nonmutating() {
    let pair = Pair::load();
    let (cf, rf) = pair.funcs();
    let mut rng = Rng::new(0x1413);
    for _ in 0..200 {
        let len = 1 + rng.below(40);
        let values: Vec<i32> = (0..len).map(|_| rng.next_i32()).collect();
        let mut cl = ContiguousList::new(&values);
        let head = cl.head();
        let mut outs = Vec::new();
        for _ in 0..8 {
            outs.push(unsafe { cf(head) });
            outs.push(unsafe { rf(head) });
        }
        assert!(
            outs.windows(2).all(|w| w[0] == w[1]),
            "non-idempotent results {outs:?} for {values:?}"
        );
        assert_eq!(cl.values(), values, "list mutated by the calls");
    }
}

// ---------------------------------------------------------------------------
// Phase C — ERRORS.md rows
// ---------------------------------------------------------------------------

/// ERRORS row 1 / G1 / G3 — `head == NULL` must return exactly `-1` in both.
#[test]
fn err_row1_null_head() {
    let pair = Pair::load();
    let (cf, rf) = pair.funcs();
    let c_out = unsafe { cf(std::ptr::null_mut()) };
    let r_out = unsafe { rf(std::ptr::null_mut()) };
    assert_eq!(c_out, -1, "C must return the -1 sentinel for NULL");
    assert_eq!(r_out, -1, "Rust must return the -1 sentinel for NULL");
    assert_eq!(c_out, r_out);
}

/// ERRORS row G2 — sentinel collision: a single node holding `-1` returns the
/// same value as the NULL error, and both libs must agree.
#[test]
fn err_sentinel_collision_minus_one() {
    let pair = Pair::load();
    let (cf, rf) = pair.funcs();

    let mut one = ContiguousList::new(&[-1]);
    let h = one.head();
    assert_eq!(unsafe { cf(h) }, -1);
    assert_eq!(unsafe { rf(h) }, -1);

    // -1 as the minimum of a longer list.
    for values in [
        vec![-1, 0, 1],
        vec![5, -1, 5],
        vec![7, 7, -1],
        vec![-1, -1, -1],
        vec![0, 3, -1, 9],
    ] {
        assert_same(&pair, &values);
        assert_eq!(expected(&values), -1);
    }
}

/// ERRORS rows G4 / G5 — extreme `int` values, including `INT_MIN` next to `-1`
/// (guards against signed-comparison or sentinel confusion).
#[test]
fn err_extreme_int_values() {
    let pair = Pair::load();

    assert_same(&pair, &[i32::MIN]);
    assert_same(&pair, &[i32::MAX]);
    assert_same(&pair, &[i32::MIN, i32::MAX]);
    assert_same(&pair, &[i32::MAX, i32::MIN]);
    assert_same(&pair, &[i32::MIN, -1]);
    assert_same(&pair, &[-1, i32::MIN]);
    assert_same(&pair, &[i32::MIN + 1, i32::MIN, -1, 0, i32::MAX]);
    assert_same(&pair, &[0, i32::MIN, 0]);

    // INT_MIN must win over -1 in every arrangement of a 4-element list.
    let base = [i32::MIN, -1, 0, i32::MAX];
    let mut perm = base;
    // All 24 permutations by simple index enumeration.
    let idx = [
        [0, 1, 2, 3],
        [0, 1, 3, 2],
        [0, 2, 1, 3],
        [0, 2, 3, 1],
        [0, 3, 1, 2],
        [0, 3, 2, 1],
        [1, 0, 2, 3],
        [1, 0, 3, 2],
        [1, 2, 0, 3],
        [1, 2, 3, 0],
        [1, 3, 0, 2],
        [1, 3, 2, 0],
        [2, 0, 1, 3],
        [2, 0, 3, 1],
        [2, 1, 0, 3],
        [2, 1, 3, 0],
        [2, 3, 0, 1],
        [2, 3, 1, 0],
        [3, 0, 1, 2],
        [3, 0, 2, 1],
        [3, 1, 0, 2],
        [3, 1, 2, 0],
        [3, 2, 0, 1],
        [3, 2, 1, 0],
    ];
    for p in idx {
        for (slot, &src) in p.iter().enumerate() {
            perm[slot] = base[src];
        }
        assert_same(&pair, &perm);
        assert_eq!(expected(&perm), i32::MIN);
    }
}

/// ERRORS row G6 — deep traversal: a loop in C vs. anything recursive in Rust
/// would diverge (stack overflow) here.
#[test]
fn err_long_list_no_stack_divergence() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x2001);
    for len in [1000usize, 10_000, 100_000] {
        let mut values: Vec<i32> = (0..len).map(|_| rng.in_range_i32(-5, i32::MAX)).collect();
        values[len / 2] = i32::MIN;
        assert_same(&pair, &values);
    }
}

/// Extra: a wide random sweep over all shapes at once, as a catch-all.
#[test]
fn sweep_random_all_shapes() {
    let pair = Pair::load();
    let mut rng = Rng::new(0xDEAD_BEEF);
    for _ in 0..4000 {
        let len = rng.below(65); // includes 0 -> NULL head
        let mode = rng.below(5);
        let values: Vec<i32> = (0..len)
            .map(|_| match mode {
                0 => rng.next_i32(),
                1 => rng.in_range_i32(-3, 3),
                2 => rng.in_range_i32(i32::MIN, i32::MIN + 5),
                3 => rng.in_range_i32(i32::MAX - 5, i32::MAX),
                _ => rng.in_range_i32(-1, 1),
            })
            .collect();
        if values.is_empty() {
            assert_same_ptr(&pair, std::ptr::null_mut(), "sweep null");
        } else {
            assert_same(&pair, &values);
        }
    }
}
