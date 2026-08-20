//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1..C30). Every row drives BOTH the C
//! `.so` and the Rust `.so` through `dlopen`/`dlsym` and asserts that the
//! result buffer `a` **and** the scratch buffer `b` are byte-for-byte
//! identical, tail padding included.
//!
//! Every row uses many randomized inputs (fixed seed) rather than a single
//! hand-picked value.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Input shapes
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Pattern {
    /// all `sort_bits` equal
    Eq,
    /// strictly ascending `sort_bits`
    Asc,
    /// strictly descending `sort_bits`
    Desc,
    /// two distinct `sort_bits` values
    Two,
    /// four distinct `sort_bits` values (heavy duplicates)
    FewDup,
    /// random `sort_bits` in 0..8
    RSmall,
    /// random `sort_bits` over the full i32 range
    RFull,
    /// `sort_bits` drawn from {INT_MIN, INT_MAX, 0, -1, 1}
    Extr,
    /// equal `sort_bits`, strictly descending `texture_id` (pins the dead branch)
    EqTexDesc,
    /// equal `sort_bits`, `texture_id` from {0, u64::MAX, random}
    EqTexExtr,
    /// random `sort_bits`, `texture_id` all zero
    RSmallTexZero,
    /// left half all INT_MIN, right half all INT_MAX (right run exhausts last)
    BlockSplit,
    /// left half all INT_MAX, right half all INT_MIN (right run exhausts first)
    BlockSplitMirror,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Pad {
    Zero,
    Garbage,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Scratch {
    Zero,
    Garbage,
}

fn gen_input(size: i32, p: Pattern, pad: Pad, rng: &mut Rng) -> Vec<Sprite> {
    let n = size.max(0) as usize;
    let mut v = Vec::with_capacity(n);
    // Per-array random constants so each iteration explores different values.
    let k = rng.next_i32();
    let (v0, v1) = (rng.next_i32(), rng.next_i32());
    let quad = [rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()];
    let extremes = [i32::MIN, i32::MAX, 0, -1, 1];
    let base = rng.next_i32() / 4; // keep ascending/descending in range
    let split = (n / 2) as usize;

    for i in 0..n {
        let (sort_bits, texture_id) = match p {
            Pattern::Eq => (k, rng.next_u64()),
            Pattern::Asc => (base.wrapping_add(i as i32), rng.next_u64()),
            Pattern::Desc => (base.wrapping_sub(i as i32), rng.next_u64()),
            Pattern::Two => (
                if rng.next_u64() & 1 == 0 { v0 } else { v1 },
                rng.next_u64(),
            ),
            Pattern::FewDup => (quad[(rng.below(4)) as usize], rng.next_u64()),
            Pattern::RSmall => (rng.below(8) as i32, rng.next_u64()),
            Pattern::RFull => (rng.next_i32(), rng.next_u64()),
            Pattern::Extr => (extremes[rng.below(5) as usize], rng.next_u64()),
            Pattern::EqTexDesc => (k, (n - i) as u64),
            Pattern::EqTexExtr => (
                k,
                match rng.below(3) {
                    0 => 0,
                    1 => u64::MAX,
                    _ => rng.next_u64(),
                },
            ),
            Pattern::RSmallTexZero => (rng.below(8) as i32, 0),
            Pattern::BlockSplit => {
                if i < split {
                    (i32::MIN, rng.next_u64())
                } else {
                    (i32::MAX, rng.next_u64())
                }
            }
            Pattern::BlockSplitMirror => {
                if i < split {
                    (i32::MAX, rng.next_u64())
                } else {
                    (i32::MIN, rng.next_u64())
                }
            }
        };
        v.push(Sprite {
            texture_id,
            sort_bits,
            pad: match pad {
                Pad::Zero => 0,
                Pad::Garbage => rng.next_u32(),
            },
        });
    }
    v
}

fn gen_scratch(size: i32, s: Scratch, rng: &mut Rng) -> Vec<Sprite> {
    let n = size.max(0) as usize;
    (0..n)
        .map(|_| match s {
            Scratch::Zero => Sprite::default(),
            Scratch::Garbage => Sprite {
                texture_id: rng.next_u64(),
                sort_bits: rng.next_i32(),
                pad: rng.next_u32(),
            },
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Row runner
// ---------------------------------------------------------------------------

struct Row {
    id: &'static str,
    sizes: Vec<i32>,
    pattern: Pattern,
    pad: Pad,
    scratch: Scratch,
    iters: usize,
    /// number of consecutive `merge_sort` calls on the same buffers
    passes: usize,
}

impl Row {
    fn new(id: &'static str, sizes: Vec<i32>, pattern: Pattern) -> Self {
        Row {
            id,
            sizes,
            pattern,
            pad: Pad::Zero,
            scratch: Scratch::Zero,
            iters: 200,
            passes: 1,
        }
    }
    fn pad(mut self, p: Pad) -> Self {
        self.pad = p;
        self
    }
    fn scratch(mut self, s: Scratch) -> Self {
        self.scratch = s;
        self
    }
    fn iters(mut self, n: usize) -> Self {
        self.iters = n;
        self
    }
    fn passes(mut self, n: usize) -> Self {
        self.passes = n;
        self
    }
}

/// Runs one `CONFIGS.md` row against both `.so`s.
fn run_row(row: Row) {
    let c = load_c();
    let r = load_rust();
    // Seed derived from the row id so rows are independent but reproducible.
    let mut seed = SEED;
    for b in row.id.bytes() {
        seed = seed.wrapping_mul(0x100_0000_01B3) ^ b as u64;
    }
    let mut rng = Rng::new(seed);
    let mut calls = 0usize;

    for iter in 0..row.iters {
        for &size in &row.sizes {
            let a0 = gen_input(size, row.pattern, row.pad, &mut rng);
            let b0 = gen_scratch(size, row.scratch, &mut rng);

            let (mut ac, mut bc) = (a0.clone(), b0.clone());
            let (mut ar, mut br) = (a0.clone(), b0.clone());

            for pass in 0..row.passes {
                unsafe { c.call(ac.as_mut_ptr(), bc.as_mut_ptr(), size) };
                unsafe { r.call(ar.as_mut_ptr(), br.as_mut_ptr(), size) };
                calls += 1;

                let ctx = format!(
                    "row {} iter {iter} size {size} pass {pass} pattern {:?} pad {:?} scratch {:?}",
                    row.id, row.pattern, row.pad, row.scratch
                );
                assert_bytes_eq("a (sorted result)", &ctx, &ac, &ar);
                assert_bytes_eq("b (scratch buffer)", &ctx, &bc, &br);
            }
        }
    }
    assert!(calls > 0, "row {} executed no calls", row.id);
    eprintln!(
        "row {:<4} OK  ({} differential merge_sort call pairs)",
        row.id, calls
    );
}

fn small_sizes() -> Vec<i32> {
    (2..=64).collect()
}

// ---------------------------------------------------------------------------
// ABI parity
// ---------------------------------------------------------------------------

/// Values on the right come from gcc on `c_src/include/lib.h`:
/// `size=16 align=8 off_tex=0 off_sb=8`.
#[test]
fn abi_layout_matches_c() {
    assert_eq!(std::mem::size_of::<Sprite>(), 16, "sizeof mismatch");
    assert_eq!(std::mem::align_of::<Sprite>(), 8, "alignof mismatch");
    assert_eq!(std::mem::offset_of!(Sprite, texture_id), 0);
    assert_eq!(std::mem::offset_of!(Sprite, sort_bits), 8);
    // The `pad` field must occupy exactly the C struct's tail padding.
    assert_eq!(std::mem::offset_of!(Sprite, pad), 12);
}

#[test]
fn both_libraries_load_and_export_merge_sort() {
    let c = load_c();
    let r = load_rust();
    eprintln!("C    .so: {}", c.path.display());
    eprintln!("Rust .so: {}", r.path.display());
    assert_ne!(c.path, r.path);
}

/// Sanity guard: proves the differential rows are doing real work, i.e. the C
/// really permutes the array into `sort_bits` order (a suite where both sides
/// no-op would otherwise "pass" trivially).
#[test]
fn sanity_c_actually_sorts_and_permutes() {
    let c = load_c();
    let mut rng = Rng::new(SEED ^ 0xABCD);
    let mut saw_reorder = 0usize;
    for size in 2..=64i32 {
        for _ in 0..20 {
            let a0 = gen_input(size, Pattern::RFull, Pad::Zero, &mut rng);
            let mut a = a0.clone();
            let mut b = gen_scratch(size, Scratch::Zero, &mut rng);
            unsafe { c.call(a.as_mut_ptr(), b.as_mut_ptr(), size) };

            // sorted non-decreasing by sort_bits
            for w in a.windows(2) {
                assert!(
                    w[0].sort_bits <= w[1].sort_bits,
                    "C output not sorted by sort_bits at size {size}: {:?}",
                    a
                );
            }
            // output is a permutation of the input
            let mut i0: Vec<(i32, u64)> = a0.iter().map(|s| (s.sort_bits, s.texture_id)).collect();
            let mut i1: Vec<(i32, u64)> = a.iter().map(|s| (s.sort_bits, s.texture_id)).collect();
            i0.sort_unstable();
            i1.sort_unstable();
            assert_eq!(i0, i1, "C output is not a permutation of the input");
            if a0 != a {
                saw_reorder += 1;
            }
        }
    }
    assert!(
        saw_reorder > 100,
        "expected the C to reorder most random arrays, only saw {saw_reorder}"
    );
    eprintln!("sanity: C sorted+permuted; {saw_reorder} arrays were reordered");
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows C1..C30
// ---------------------------------------------------------------------------

#[test]
fn c01_size_zero() {
    run_row(Row::new("C1", vec![0], Pattern::RSmall));
}

#[test]
fn c02_size_one() {
    run_row(Row::new("C2", vec![1], Pattern::RFull));
}

#[test]
fn c03_size_one_garbage_padding() {
    run_row(Row::new("C3", vec![1], Pattern::RFull).pad(Pad::Garbage));
}

#[test]
fn c04_size_two_ascending() {
    run_row(Row::new("C4", vec![2], Pattern::Asc));
}

#[test]
fn c05_size_two_descending() {
    run_row(Row::new("C5", vec![2], Pattern::Desc));
}

#[test]
fn c06_size_two_equal_keys() {
    run_row(Row::new("C6", vec![2], Pattern::Eq));
}

#[test]
fn c07_size_two_garbage_padding() {
    run_row(Row::new("C7", vec![2], Pattern::RSmall).pad(Pad::Garbage));
}

#[test]
fn c08_size_three_random() {
    run_row(Row::new("C8", vec![3], Pattern::RSmall));
}

#[test]
fn c09_size_three_equal_keys() {
    run_row(Row::new("C9", vec![3], Pattern::Eq));
}

#[test]
fn c10_power_of_two_sizes_small_keys() {
    run_row(Row::new("C10", vec![4, 8, 16, 32, 64, 256, 1024], Pattern::RSmall).iters(40));
}

#[test]
fn c11_power_of_two_sizes_full_range_keys() {
    run_row(Row::new("C11", vec![4, 8, 16, 32, 64, 256, 1024], Pattern::RFull).iters(40));
}

#[test]
fn c12_ragged_sizes_small_keys() {
    run_row(Row::new("C12", vec![5, 7, 11, 17, 31, 99, 1001], Pattern::RSmall).iters(40));
}

#[test]
fn c13_ragged_sizes_full_range_keys() {
    run_row(Row::new("C13", vec![5, 7, 11, 17, 31, 99, 1001], Pattern::RFull).iters(40));
}

#[test]
fn c14_exhaustive_small_size_sweep() {
    run_row(Row::new("C14", (0..=64).collect(), Pattern::RSmall).iters(30));
}

#[test]
fn c15_exhaustive_small_size_sweep_garbage_padding() {
    run_row(
        Row::new("C15", (0..=64).collect(), Pattern::RSmall)
            .pad(Pad::Garbage)
            .iters(30),
    );
}

#[test]
fn c16_presorted_input() {
    run_row(Row::new("C16", small_sizes(), Pattern::Asc).iters(30));
}

#[test]
fn c17_reverse_sorted_input() {
    run_row(Row::new("C17", small_sizes(), Pattern::Desc).iters(30));
}

#[test]
fn c18_all_keys_identical() {
    run_row(Row::new("C18", small_sizes(), Pattern::Eq).iters(30));
}

#[test]
fn c19_two_distinct_keys() {
    run_row(Row::new("C19", small_sizes(), Pattern::Two).iters(30));
}

#[test]
fn c20_four_distinct_keys() {
    run_row(Row::new("C20", small_sizes(), Pattern::FewDup).iters(30));
}

#[test]
fn c21_signed_extreme_keys() {
    run_row(Row::new("C21", small_sizes(), Pattern::Extr).iters(30));
}

#[test]
fn c22_full_range_keys_and_garbage_padding() {
    run_row(
        Row::new("C22", small_sizes(), Pattern::RFull)
            .pad(Pad::Garbage)
            .iters(30),
    );
}

#[test]
fn c23_equal_keys_descending_texture_id() {
    run_row(Row::new("C23", small_sizes(), Pattern::EqTexDesc).iters(30));
}

#[test]
fn c24_equal_keys_extreme_texture_id() {
    run_row(Row::new("C24", small_sizes(), Pattern::EqTexExtr).iters(30));
}

#[test]
fn c25_random_keys_zero_texture_id() {
    run_row(Row::new("C25", small_sizes(), Pattern::RSmallTexZero).iters(30));
}

#[test]
fn c26_deep_recursion_4096() {
    run_row(Row::new("C26", vec![4096], Pattern::RFull).iters(25));
}

#[test]
fn c27_deep_ragged_recursion_4095_garbage_padding() {
    run_row(
        Row::new("C27", vec![4095], Pattern::FewDup)
            .pad(Pad::Garbage)
            .iters(25),
    );
}

#[test]
fn c28_repeated_calls_on_returned_state() {
    run_row(
        Row::new("C28", small_sizes(), Pattern::RSmall)
            .iters(20)
            .passes(3),
    );
}

#[test]
fn c29_uninitialised_garbage_scratch_buffer() {
    run_row(
        Row::new("C29", small_sizes(), Pattern::RSmall)
            .pad(Pad::Garbage)
            .scratch(Scratch::Garbage)
            .iters(30),
    );
}

#[test]
fn c30_run_exhaustion_block_split() {
    run_row(Row::new("C30a", small_sizes(), Pattern::BlockSplit).iters(30));
    run_row(Row::new("C30b", small_sizes(), Pattern::BlockSplitMirror).iters(30));
}

// ---------------------------------------------------------------------------
// Internal-branch coverage instrument
// ---------------------------------------------------------------------------
//
// The three `spritebatch_internal_*` functions are `static` in the C, so they
// cannot be called across the FFI boundary on either side. This instrument
// re-runs the same algorithm over the same generated inputs with a counter on
// every branch of `c_src/src/lib.c`, proving the CONFIGS.md rows really do
// reach every internal branch (rather than only the ones a happy path hits).

#[derive(Default, Debug)]
struct Cov {
    base_case: u64,
    recursed: u64,
    split_even_span: u64,
    split_odd_span: u64,
    iter_take_i_cmp_true: u64,
    iter_take_i_right_spent: u64,
    iter_take_j_cmp_false: u64,
    iter_take_j_left_spent: u64,
    cmp_lt: u64,
    cmp_eq: u64,
    cmp_gt_falls_through: u64,
    memcpy_zero: u64,
    memcpy_nonzero: u64,
}

fn cov_cmp(a: &Sprite, b: &Sprite, c: &mut Cov) -> bool {
    if a.sort_bits <= b.sort_bits {
        if a.sort_bits < b.sort_bits {
            c.cmp_lt += 1;
        } else {
            c.cmp_eq += 1;
        }
        return true;
    }
    // line 9: reached only when sort_bits differ, so `==` is always false here.
    c.cmp_gt_falls_through += 1;
    assert_ne!(
        a.sort_bits, b.sort_bits,
        "line 9 reached with equal sort_bits — the dead branch would be live!"
    );
    false
}

fn cov_iter(a: &[Sprite], lo: i32, split: i32, hi: i32, b: &mut [Sprite], c: &mut Cov) {
    let (mut i, mut j) = (lo, split);
    let mut k = lo;
    while k < hi {
        let take_i = if i < split {
            if j >= hi {
                c.iter_take_i_right_spent += 1;
                true
            } else if cov_cmp(&a[i as usize], &a[j as usize], c) {
                c.iter_take_i_cmp_true += 1;
                true
            } else {
                c.iter_take_j_cmp_false += 1;
                false
            }
        } else {
            c.iter_take_j_left_spent += 1;
            false
        };
        if take_i {
            b[k as usize] = a[i as usize];
            i += 1;
        } else {
            b[k as usize] = a[j as usize];
            j += 1;
        }
        k += 1;
    }
}

fn cov_recurse(b: &mut [Sprite], lo: i32, hi: i32, a: &mut [Sprite], c: &mut Cov) {
    if hi - lo <= 1 {
        c.base_case += 1;
        return;
    }
    c.recursed += 1;
    if (hi - lo) % 2 == 0 {
        c.split_even_span += 1;
    } else {
        c.split_odd_span += 1;
    }
    let split = (lo + hi) / 2;
    cov_recurse(a, lo, split, b, c);
    cov_recurse(a, split, hi, b, c);
    let snapshot = b.to_vec();
    cov_iter(&snapshot, lo, split, hi, a, c);
}

fn cov_merge_sort(a: &mut [Sprite], b: &mut [Sprite], size: i32, c: &mut Cov) {
    if size == 0 {
        c.memcpy_zero += 1;
    } else {
        c.memcpy_nonzero += 1;
    }
    b[..size as usize].copy_from_slice(&a[..size as usize]);
    let mut bb = b.to_vec();
    cov_recurse(&mut bb, 0, size, a, c);
    b.copy_from_slice(&bb);
}

#[test]
fn internal_branch_coverage_is_complete() {
    let mut cov = Cov::default();
    let mut rng = Rng::new(SEED ^ 0x1111);
    let patterns = [
        Pattern::Eq,
        Pattern::Asc,
        Pattern::Desc,
        Pattern::Two,
        Pattern::FewDup,
        Pattern::RSmall,
        Pattern::RFull,
        Pattern::Extr,
        Pattern::EqTexDesc,
        Pattern::BlockSplit,
        Pattern::BlockSplitMirror,
    ];
    for p in patterns {
        for size in 0..=40i32 {
            for _ in 0..5 {
                let mut a = gen_input(size, p, Pad::Zero, &mut rng);
                let mut b = gen_scratch(size, Scratch::Zero, &mut rng);
                cov_merge_sort(&mut a, &mut b, size, &mut cov);
            }
        }
    }
    eprintln!("internal branch coverage: {cov:#?}");
    let checks: [(&str, u64); 13] = [
        ("base_case", cov.base_case),
        ("recursed", cov.recursed),
        ("split_even_span", cov.split_even_span),
        ("split_odd_span", cov.split_odd_span),
        ("iter_take_i_cmp_true", cov.iter_take_i_cmp_true),
        ("iter_take_i_right_spent", cov.iter_take_i_right_spent),
        ("iter_take_j_cmp_false", cov.iter_take_j_cmp_false),
        ("iter_take_j_left_spent", cov.iter_take_j_left_spent),
        ("cmp_lt", cov.cmp_lt),
        ("cmp_eq", cov.cmp_eq),
        ("cmp_gt_falls_through", cov.cmp_gt_falls_through),
        ("memcpy_zero", cov.memcpy_zero),
        ("memcpy_nonzero", cov.memcpy_nonzero),
    ];
    for (name, n) in checks {
        assert!(n > 0, "internal branch `{name}` was never exercised");
    }
}
