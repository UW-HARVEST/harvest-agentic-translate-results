//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call on both sides goes through `dlopen`/`dlsym`. Randomized rows use
//! a fixed-seed SplitMix64 PRNG so failures are reproducible.

mod harness;

use harness::{assert_alloc_diff, assert_pair_eq, load, measure_pair, Rng};
use std::os::raw::{c_char, c_int};

/// Iterations per randomized row.
const N: usize = 4000;
/// Iterations for rows whose body performs heap traffic (kept lower: each
/// iteration does 4 mallocs + 4 frees across the two libraries).
const N_HEAP: usize = 1500;

// ===========================================================================
// Rows 1–5: apply_bitmask
// ===========================================================================

fn bitmask_row(op_fixed: Option<c_int>, seed: u64, label: &str) {
    let p = load();
    let mut rng = Rng::new(seed);
    for i in 0..N {
        let value = rng.i32_mixed();
        let op = match op_fixed {
            Some(o) => o,
            None => rng.i32_mixed(),
        };
        let c = unsafe { (p.c.apply_bitmask)(value, op) };
        let r = unsafe { (p.rust.apply_bitmask)(value, op) };
        assert_eq!(
            c, r,
            "[{label}] iter {i}: apply_bitmask(value={value}, operation={op}) C={c} Rust={r}"
        );
    }
}

#[test]
fn row01_apply_bitmask_op0() {
    bitmask_row(Some(0), 0x1001, "row01 op=0 (&0xF0)");
}

#[test]
fn row02_apply_bitmask_op1() {
    bitmask_row(Some(1), 0x1002, "row02 op=1 (&0x0F)");
}

#[test]
fn row03_apply_bitmask_op2() {
    bitmask_row(Some(2), 0x1003, "row03 op=2 (|0xAA)");
}

#[test]
fn row04_apply_bitmask_op3() {
    bitmask_row(Some(3), 0x1004, "row04 op=3 (^0x55)");
}

#[test]
fn row05_apply_bitmask_full_cross_product() {
    bitmask_row(None, 0x1005, "row05 random op x random value");
}

// ===========================================================================
// Rows 6–7: init_matrix
// ===========================================================================

/// Layout: 4 guard ints, then the 3x4 matrix, then 4 guard ints. `init_matrix`
/// must write exactly the 12 middle cells and leave the guards untouched.
fn init_matrix_once(f: harness::FnInitMatrix, poison: c_int) -> (Vec<c_int>, Vec<c_int>) {
    let mut buf: Vec<c_int> = vec![poison; 4 + 12 + 4];
    unsafe {
        let mid = buf.as_mut_ptr().add(4) as *mut [c_int; 4];
        f(mid);
    }
    let guards = [&buf[0..4], &buf[16..20]].concat();
    let cells = buf[4..16].to_vec();
    (cells, guards)
}

#[test]
fn row06_init_matrix_fresh_buffer_with_guards() {
    let p = load();
    let mut rng = Rng::new(0x2001);
    for i in 0..N {
        let poison = rng.i32_mixed();
        let (cc, cg) = init_matrix_once(p.c.init_matrix, poison);
        let (rc, rg) = init_matrix_once(p.rust.init_matrix, poison);
        assert_eq!(cc, rc, "[row06] iter {i}: matrix cells differ (poison={poison})");
        assert_eq!(cg, rg, "[row06] iter {i}: guard cells differ (poison={poison})");
        assert_eq!(
            cg,
            vec![poison; 8],
            "[row06] iter {i}: C wrote outside the 3x4 matrix"
        );
    }
}

#[test]
fn row07_init_matrix_idempotent() {
    let p = load();
    let mut rng = Rng::new(0x2002);
    for i in 0..200 {
        let poison = rng.i32_mixed();
        let mut cbuf: Vec<c_int> = vec![poison; 12];
        let mut rbuf: Vec<c_int> = vec![poison; 12];
        unsafe {
            for _ in 0..2 {
                (p.c.init_matrix)(cbuf.as_mut_ptr() as *mut [c_int; 4]);
                (p.rust.init_matrix)(rbuf.as_mut_ptr() as *mut [c_int; 4]);
            }
        }
        assert_eq!(cbuf, rbuf, "[row07] iter {i}: double init_matrix differs");
    }
}

// ===========================================================================
// Rows 8–12: process_string
// ===========================================================================

fn ps(f: harness::FnProcessString, bytes: &[u8]) -> c_int {
    // Caller guarantees a trailing NUL.
    unsafe { f(bytes.as_ptr() as *const c_char) }
}

fn ps_both(p: &harness::Pair, bytes: &[u8], label: &str, i: usize) -> c_int {
    let c = ps(p.c.process_string, bytes);
    let r = ps(p.rust.process_string, bytes);
    assert_eq!(c, r, "[{label}] iter {i}: process_string({bytes:?}) C={c} Rust={r}");
    c
}

#[test]
fn row08_process_string_empty() {
    let p = load();
    let got = ps_both(&p, b"\0", "row08 empty", 0);
    assert_eq!(got, 0, "[row08] empty string must return 0");
}

#[test]
fn row09_process_string_len1() {
    let p = load();
    // Every possible single non-NUL byte, including the high-bit (negative
    // `char`) half — row 16 of ERRORS.md overlaps here.
    for b in 1u16..=255 {
        let buf = [b as u8, 0];
        let got = ps_both(&p, &buf, "row09 len1", b as usize);
        assert_eq!(got, 1, "[row09] byte {b:#04x} must give strlen 1");
    }
}

#[test]
fn row10_process_string_random_nonzero_bytes() {
    let p = load();
    let mut rng = Rng::new(0x3001);
    for i in 0..N {
        let len = rng.range(2, 64) as usize;
        let mut buf: Vec<u8> = (0..len).map(|_| rng.range(1, 255) as u8).collect();
        buf.push(0);
        let got = ps_both(&p, &buf, "row10 random", i);
        assert_eq!(got, len as c_int, "[row10] iter {i}: expected strlen {len}");
    }
}

#[test]
fn row11_process_string_embedded_nul() {
    let p = load();
    let mut rng = Rng::new(0x3002);
    for i in 0..N {
        let prefix = rng.range(1, 32) as usize;
        let suffix = rng.range(1, 32) as usize;
        let mut buf: Vec<u8> = (0..prefix).map(|_| rng.range(1, 255) as u8).collect();
        buf.push(0);
        buf.extend((0..suffix).map(|_| rng.range(1, 255) as u8));
        buf.push(0);
        let got = ps_both(&p, &buf, "row11 embedded NUL", i);
        assert_eq!(got, prefix as c_int, "[row11] iter {i}: strlen must stop at first NUL");
    }
}

#[test]
fn row12_process_string_long() {
    let p = load();
    let mut rng = Rng::new(0x3003);
    for i in 0..50 {
        let mut buf: Vec<u8> = (0..512).map(|_| rng.range(1, 255) as u8).collect();
        buf.push(0);
        let got = ps_both(&p, &buf, "row12 long", i);
        assert_eq!(got, 512);
    }
}

// ===========================================================================
// Rows 13–19: shift_array
// ===========================================================================

/// Run `shift_array` on both sides over an identical buffer with guard padding
/// on each end, and compare the full buffer (guards included) byte-for-byte.
fn shift_both(
    p: &harness::Pair,
    contents: &[c_int],
    size: c_int,
    positions: c_int,
    label: &str,
    i: usize,
) {
    const PAD: usize = 4;
    let guard: c_int = 0x5A5A_5A5A;
    let build = || {
        let mut v = vec![guard; PAD];
        v.extend_from_slice(contents);
        v.extend(std::iter::repeat(guard).take(PAD));
        v
    };
    let mut cbuf = build();
    let mut rbuf = build();
    unsafe {
        (p.c.shift_array)(cbuf.as_mut_ptr().add(PAD), size, positions);
        (p.rust.shift_array)(rbuf.as_mut_ptr().add(PAD), size, positions);
    }
    assert_eq!(
        cbuf, rbuf,
        "[{label}] iter {i}: shift_array(size={size}, positions={positions}) \
         contents={contents:?}\n  C   ={cbuf:?}\n  Rust={rbuf:?}"
    );
    assert_eq!(
        &cbuf[0..PAD],
        &vec![guard; PAD][..],
        "[{label}] iter {i}: C underflowed the buffer"
    );
    assert_eq!(
        &cbuf[PAD + contents.len()..],
        &vec![guard; PAD][..],
        "[{label}] iter {i}: C overflowed the buffer"
    );
}

fn rand_contents(rng: &mut Rng, n: usize) -> Vec<c_int> {
    (0..n).map(|_| rng.i32_mixed()).collect()
}

#[test]
fn row13_shift_array_positions_one() {
    let p = load();
    let mut rng = Rng::new(0x4001);
    for i in 0..N {
        let size = rng.range(1, 16);
        let contents = rand_contents(&mut rng, size as usize);
        shift_both(&p, &contents, size, 1, "row13 positions=1", i);
    }
}

#[test]
fn row14_shift_array_mid_range() {
    let p = load();
    let mut rng = Rng::new(0x4002);
    for i in 0..N {
        let size = rng.range(2, 16);
        let positions = rng.range(1, size - 1);
        let contents = rand_contents(&mut rng, size as usize);
        shift_both(&p, &contents, size, positions, "row14 mid range", i);
    }
}

#[test]
fn row15_shift_array_positions_size_minus_one() {
    let p = load();
    let mut rng = Rng::new(0x4003);
    for i in 0..N {
        let size = rng.range(2, 16);
        let contents = rand_contents(&mut rng, size as usize);
        shift_both(&p, &contents, size, size - 1, "row15 positions=size-1", i);
    }
}

#[test]
fn row16_shift_array_positions_nonpositive_noop() {
    let p = load();
    let mut rng = Rng::new(0x4004);
    for i in 0..N {
        let size = rng.range(1, 16);
        let positions = if i % 2 == 0 { 0 } else { -rng.range(1, 32) };
        let contents = rand_contents(&mut rng, size as usize);
        let before = contents.clone();
        shift_both(&p, &contents, size, positions, "row16 positions<=0", i);
        assert_eq!(before, contents, "[row16] test harness must not mutate its input");
    }
}

#[test]
fn row17_shift_array_positions_ge_size_noop() {
    let p = load();
    let mut rng = Rng::new(0x4005);
    for i in 0..N {
        let size = rng.range(1, 16);
        let positions = if i % 2 == 0 { size } else { size + rng.range(1, 32) };
        let contents = rand_contents(&mut rng, size as usize);
        shift_both(&p, &contents, size, positions, "row17 positions>=size", i);
    }
}

#[test]
fn row18_shift_array_degenerate_sizes() {
    let p = load();
    let mut rng = Rng::new(0x4006);
    for i in 0..N {
        // A real buffer always exists; only the `size` argument is degenerate.
        let real = rng.range(1, 8) as usize;
        let contents = rand_contents(&mut rng, real);
        let size = match i % 3 {
            0 => 1,
            1 => 0,
            _ => -rng.range(1, 32),
        };
        let positions = rng.i32_small(8);
        shift_both(&p, &contents, size, positions, "row18 degenerate size", i);
    }
}

#[test]
fn row19_shift_array_half_overlap() {
    let p = load();
    let mut rng = Rng::new(0x4007);
    for i in 0..N {
        // size=16, positions=8: destination and source overlap exactly, the
        // case a forward `memcpy` would corrupt but `memmove` handles.
        let contents = rand_contents(&mut rng, 16);
        shift_both(&p, &contents, 16, 8, "row19 half overlap", i);
        // And every other overlap ratio for size 16.
        for pos in 1..16 {
            shift_both(&p, &contents, 16, pos, "row19 all overlaps", i);
        }
    }
}

// ===========================================================================
// Rows 20–22: compare_allocations (phase-neutral pairs)
// ===========================================================================

fn cmp_alloc_row(seed: u64, label: &str, pick: fn(&mut Rng) -> (c_int, c_int)) {
    let p = load();
    let mut rng = Rng::new(seed);
    for i in 0..N_HEAP {
        let (v1, v2) = pick(&mut rng);
        let ctx = format!("{label} iter {i}: val1={v1} val2={v2}");
        // Strict, ORDERED C-vs-Rust differential (adjacent, heap-locked).
        assert_alloc_diff(&ctx, || unsafe { (p.c.compare_allocations)(v1, v2) }, || unsafe { (p.rust.compare_allocations)(v1, v2) });
        // Stable phase-independent C measurement for the C-vs-C checks below.
        let c = measure_pair(&ctx, || unsafe { (p.c.compare_allocations)(v1, v2) });
        // Structural check on the C's own contract: 1 or 2 (never 3, since two
        // live allocations cannot share an address), plus 10 iff val1 > 0.
        let bonus = if v1 > 0 { 10 } else { 0 };
        for got in c {
            assert!(
                got == 1 + bonus || got == 2 + bonus,
                "[{label}] iter {i}: unexpected C result {got} for val1={v1}"
            );
        }
    }
}

#[test]
fn row20_compare_allocations_val1_positive() {
    cmp_alloc_row(0x5001, "row20 val1>0", |r| (r.range(1, i32::MAX), r.i32_mixed()));
}

#[test]
fn row21_compare_allocations_val1_nonpositive() {
    cmp_alloc_row(0x5002, "row21 val1<=0", |r| {
        let v1 = if r.next_u64() % 2 == 0 { 0 } else { r.range(i32::MIN, 0) };
        (v1, r.i32_mixed())
    });
}

#[test]
fn row22_compare_allocations_full_range() {
    cmp_alloc_row(0x5003, "row22 full range", |r| (r.i32_mixed(), r.i32_mixed()));
}

// ===========================================================================
// Rows 23–33: arity4
// ===========================================================================

fn a4_both(p: &harness::Pair, q: [c_int; 4], label: &str, i: usize) {
    assert_alloc_diff(
        &format!("{label} iter {i}: arity4{q:?}"),
        || unsafe { (p.c.arity4)(q[0], q[1], q[2], q[3]) },
        || unsafe { (p.rust.arity4)(q[0], q[1], q[2], q[3]) },
    );
}

/// Pick `p1` so that `p1 % 4 == residue` (C `%` keeps the dividend's sign, so a
/// negative residue requires a negative `p1`).
fn p1_with_residue(rng: &mut Rng, residue: c_int) -> c_int {
    loop {
        let base = rng.i32_mixed();
        // Avoid INT_MIN edge distortion when adjusting; just filter.
        if base % 4 == residue {
            return base;
        }
        let adj = base.wrapping_sub(base % 4).wrapping_add(residue);
        if adj % 4 == residue && (adj >= 0) == (residue >= 0 || residue == 0) {
            return adj;
        }
    }
}

fn a4_residue_row(residue: c_int, seed: u64, label: &str) {
    let p = load();
    let mut rng = Rng::new(seed);
    for i in 0..N_HEAP {
        let p1 = p1_with_residue(&mut rng, residue);
        assert_eq!(p1 % 4, residue, "[{label}] residue setup failed for p1={p1}");
        a4_both(&p, [p1, rng.i32_mixed(), 0, 0], label, i);
    }
}

#[test]
fn row23_arity4_residue0_no_rescale_no_add() {
    a4_residue_row(0, 0x6000, "row23 p1%4==0, p3=0, p4=0");
}

#[test]
fn row24_arity4_residue1_no_rescale_no_add() {
    a4_residue_row(1, 0x6001, "row24 p1%4==1, p3=0, p4=0");
}

#[test]
fn row25_arity4_residue2_no_rescale_no_add() {
    a4_residue_row(2, 0x6002, "row25 p1%4==2, p3=0, p4=0");
}

#[test]
fn row26_arity4_residue3_no_rescale_no_add() {
    a4_residue_row(3, 0x6003, "row26 p1%4==3, p3=0, p4=0");
}

#[test]
fn row27_arity4_negative_residues_hit_default_arm() {
    let p = load();
    let mut rng = Rng::new(0x6004);
    let mut seen = [false; 3];
    for i in 0..N_HEAP {
        let p1 = -rng.range(1, i32::MAX);
        let res = p1 % 4;
        if (-3..=-1).contains(&res) {
            seen[(-res - 1) as usize] = true;
        }
        a4_both(&p, [p1, rng.i32_mixed(), 0, 0], "row27 negative residue", i);
    }
    assert_eq!(seen, [true; 3], "[row27] did not cover residues -1, -2, -3");
}

#[test]
fn row28_arity4_rescale_positive_p3() {
    let p = load();
    let mut rng = Rng::new(0x6005);
    for i in 0..N_HEAP {
        let p3 = rng.range(1, i32::MAX);
        a4_both(&p, [rng.i32_mixed(), rng.i32_mixed(), p3, 0], "row28 p3>0", i);
    }
}

#[test]
fn row29_arity4_rescale_negative_p3() {
    let p = load();
    let mut rng = Rng::new(0x6006);
    for i in 0..N_HEAP {
        let p3 = rng.range(i32::MIN, -1);
        a4_both(&p, [rng.i32_mixed(), rng.i32_mixed(), p3, 0], "row29 p3<0", i);
    }
}

#[test]
fn row30_arity4_both_branches() {
    let p = load();
    let mut rng = Rng::new(0x6007);
    for i in 0..N_HEAP {
        let mut p3 = rng.i32_mixed();
        if p3 == 0 {
            p3 = 7;
        }
        let mut p4 = rng.i32_mixed();
        if p4 == 0 {
            p4 = -13;
        }
        a4_both(&p, [rng.i32_mixed(), rng.i32_mixed(), p3, p4], "row30 p3!=0 p4!=0", i);
    }
}

#[test]
fn row31_arity4_only_p4_branch() {
    let p = load();
    let mut rng = Rng::new(0x6008);
    for i in 0..N_HEAP {
        let mut p4 = rng.i32_mixed();
        if p4 == 0 {
            p4 = i32::MIN;
        }
        a4_both(&p, [rng.i32_mixed(), rng.i32_mixed(), 0, p4], "row31 p3=0 p4!=0", i);
    }
}

#[test]
fn row32_arity4_full_cross_product_random() {
    let p = load();
    let mut rng = Rng::new(0x6009);
    for i in 0..N_HEAP * 3 {
        let q = [rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed()];
        a4_both(&p, q, "row32 all-random", i);
    }
}

#[test]
fn row33_arity4_boundary_magnitudes() {
    let p = load();
    const B: [c_int; 21] = [
        0, 1, -1, 2, -2, 3, -3, 4, -4, 99, -99, 100, -100, 101, -101, i32::MAX, i32::MIN,
        i32::MAX - 1, i32::MIN + 1, 1 << 30, -(1 << 30),
    ];
    // Full 4-way cross-product would be 194k heap-touching iterations; sample
    // it deterministically instead: exhaustive over (p1, p3) -- the two params
    // that select branches -- crossed with a rotating choice of (p2, p4).
    let mut i = 0usize;
    for (ai, &p1) in B.iter().enumerate() {
        for (ci, &p3) in B.iter().enumerate() {
            for k in 0..B.len() {
                let p2 = B[(ai + k) % B.len()];
                let p4 = B[(ci + k * 3) % B.len()];
                a4_both(&p, [p1, p2, p3, p4], "row33 boundaries", i);
                i += 1;
            }
        }
    }
    assert_eq!(i, B.len() * B.len() * B.len(), "[row33] unexpected iteration count");
}

// ===========================================================================
// Rows 34–35: arity2 / arity3
// ===========================================================================

#[test]
fn row34_arity2() {
    let p = load();
    let mut rng = Rng::new(0x7001);
    for i in 0..N_HEAP {
        let (a, b) = (rng.i32_mixed(), rng.i32_mixed());
        let ctx = format!("row34 iter {i}: arity2({a},{b})");
        // Strict, ORDERED C-vs-Rust differential (adjacent, heap-locked).
        assert_alloc_diff(&ctx, || unsafe { (p.c.arity2)(a, b) }, || unsafe { (p.rust.arity2)(a, b) });
        // Stable phase-independent C measurement for the C-vs-C checks below.
        let c = measure_pair(&ctx, || unsafe { (p.c.arity2)(a, b) });
        // arity2 must be exactly arity4(a,b,0,0) -- verified on the C side.
        let c4 = measure_pair("phase_b.rs:554 c4", || unsafe { (p.c.arity4)(a, b, 0, 0) });
        assert_pair_eq(&format!("row34 iter {i}: C arity2 vs C arity4({a},{b},0,0)"), c, c4);
    }
}

#[test]
fn row35_arity3() {
    let p = load();
    let mut rng = Rng::new(0x7002);
    for i in 0..N_HEAP {
        let a = rng.i32_mixed();
        let b = rng.i32_mixed();
        // Alternate p3 == 0 (guard skipped) and p3 != 0.
        let cc = if i % 3 == 0 { 0 } else { rng.i32_mixed() };
        let ctx = format!("row35 iter {i}: arity3({a},{b},{cc})");
        // Strict, ORDERED C-vs-Rust differential (adjacent, heap-locked).
        assert_alloc_diff(&ctx, || unsafe { (p.c.arity3)(a, b, cc) }, || unsafe { (p.rust.arity3)(a, b, cc) });
        // Stable phase-independent C measurement for the C-vs-C checks below.
        let c = measure_pair(&ctx, || unsafe { (p.c.arity3)(a, b, cc) });
        let c4 = measure_pair("phase_b.rs:573 c4", || unsafe { (p.c.arity4)(a, b, cc, 0) });
        assert_pair_eq(&format!("row35 iter {i}: C arity3 vs C arity4({a},{b},{cc},0)"), c, c4);
    }
}

// ===========================================================================
// Rows 36–42: arity dispatch
// ===========================================================================

fn arity_both(p: &harness::Pair, len: c_int, params: &[c_int], label: &str, i: usize) {
    let ptr = params.as_ptr();
    assert_alloc_diff(
        &format!("{label} iter {i}: arity(len={len}, params={params:?})"),
        || unsafe { (p.c.arity)(len, ptr) },
        || unsafe { (p.rust.arity)(len, ptr) },
    );
}

#[test]
fn row36_arity_len2_exact_buffer() {
    let p = load();
    let mut rng = Rng::new(0x8001);
    for i in 0..N_HEAP {
        // Buffer sized EXACTLY 2: an over-read past params[1] would be caught
        // by the guard page eventually, and any behavioural over-read shows as
        // a divergence from arity2.
        let params = [rng.i32_mixed(), rng.i32_mixed()];
        arity_both(&p, 2, &params, "row36 len=2", i);
        assert_alloc_diff(
            &format!("row36 iter {i}: C arity(2) vs C arity2"),
            || unsafe { (p.c.arity2)(params[0], params[1]) },
            || unsafe { (p.c.arity)(2, params.as_ptr()) },
        );
    }
}

#[test]
fn row37_arity_len3_exact_buffer() {
    let p = load();
    let mut rng = Rng::new(0x8002);
    for i in 0..N_HEAP {
        let params = [rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed()];
        arity_both(&p, 3, &params, "row37 len=3", i);
        assert_alloc_diff(
            &format!("row37 iter {i}: C arity(3) vs C arity3"),
            || unsafe { (p.c.arity3)(params[0], params[1], params[2]) },
            || unsafe { (p.c.arity)(3, params.as_ptr()) },
        );
    }
}

#[test]
fn row38_arity_len4() {
    let p = load();
    let mut rng = Rng::new(0x8003);
    for i in 0..N_HEAP {
        let params = [rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed()];
        arity_both(&p, 4, &params, "row38 len=4", i);
        assert_alloc_diff(
            &format!("row38 iter {i}: C arity(4) vs C arity4"),
            || unsafe { (p.c.arity4)(params[0], params[1], params[2], params[3]) },
            || unsafe { (p.c.arity)(4, params.as_ptr()) },
        );
    }
}

#[test]
fn row39_arity_len_5_to_255_still_arity4() {
    let p = load();
    let mut rng = Rng::new(0x8004);
    for len in 5..=255 {
        let params: Vec<c_int> = (0..len as usize).map(|_| rng.i32_mixed()).collect();
        arity_both(&p, len, &params, "row39 len 5..255", len as usize);
        // Reads only the first four.
        assert_alloc_diff(
            &format!("row39 len={len}: C arity vs C arity4 on first 4"),
            || unsafe { (p.c.arity4)(params[0], params[1], params[2], params[3]) },
            || unsafe { (p.c.arity)(len, params.as_ptr()) },
        );
    }
}

#[test]
fn row40_arity_len_above_255_truncates() {
    let p = load();
    let mut rng = Rng::new(0x8005);
    for i in 0..300 {
        let params = [rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed()];
        for (len, equiv) in [(258, 2), (259, 3), (260, 4), (512 + 2, 2), (65536 + 3, 3)] {
            arity_both(&p, len, &params, "row40 len>255", i);
            let want = measure_pair("phase_b.rs:663 want", || unsafe { (p.c.arity)(equiv, params.as_ptr()) });
            let got = measure_pair("phase_b.rs:664 got", || unsafe { (p.c.arity)(len, params.as_ptr()) });
            assert_pair_eq(
                &format!("row40 iter {i}: C arity({len}) must equal C arity({equiv})"),
                want,
                got,
            );
        }
    }
}

#[test]
fn row41_arity_negative_len_low_byte_unsigned() {
    let p = load();
    let mut rng = Rng::new(0x8006);
    let params: Vec<c_int> = (0..4).map(|_| rng.i32_mixed()).collect();
    // -1 -> 0xFF = 255 -> arity4 ; -254 -> 0x02 = 2 -> arity2
    for (len, equiv) in [(-1, 255), (-2, 254), (-254, 2), (-253, 3), (-252, 4), (-256, 0)] {
        arity_both(&p, len, &params, "row41 negative len", len as usize);
        let want = measure_pair("phase_b.rs:682 want", || unsafe { (p.c.arity)(equiv, params.as_ptr()) });
        let got = measure_pair("phase_b.rs:683 got", || unsafe { (p.c.arity)(len, params.as_ptr()) });
        assert_pair_eq(
            &format!("row41: C arity({len}) must equal C arity({equiv}) via low byte"),
            want,
            got,
        );
    }
}

#[test]
fn row42_arity_composed_pipeline_random_len_and_params() {
    let p = load();
    let mut rng = Rng::new(0x8007);
    for i in 0..N_HEAP * 2 {
        // Random `len` across the whole i32 range plus a dense sweep of the
        // interesting low values, with a params buffer always at least 4 long
        // so the >=4 branch is well-defined.
        let len = match i % 4 {
            0 => rng.range(0, 8),
            1 => rng.range(0, 300),
            2 => rng.i32_full(),
            _ => rng.range(-300, 300),
        };
        let params: Vec<c_int> = (0..8).map(|_| rng.i32_mixed()).collect();
        arity_both(&p, len, &params, "row42 composed", i);
    }
}

// ===========================================================================
// Row 43: staged cross-check of the composed pipeline
// ===========================================================================

/// Rebuild `arity4`'s value from the C library's OWN low-level exports, then
/// require that both `arity4` implementations equal that model for one of the
/// two possible heap phases. This validates the composition, not just the
/// endpoints: a Rust bug in the middle of the pipeline that happens to cancel
/// out at the end would still show up as a model mismatch.
fn model_arity4(p: &harness::Pair, q: [c_int; 4], alloc_result: c_int) -> c_int {
    let mut result: c_int = 0;

    let len1 = ps(p.c.process_string, b"Hello\0");
    let len2 = ps(p.c.process_string, b"\0");
    result = result.wrapping_add(len1.wrapping_add(len2));

    let mut values = [q[0], q[1], q[2], q[3]];
    unsafe { (p.c.shift_array)(values.as_mut_ptr(), 4, 1) };
    for v in values {
        result = result.wrapping_add(v);
    }

    result = unsafe { (p.c.apply_bitmask)(result, q[0] % 4) };

    let mut matrix = [0 as c_int; 12];
    unsafe { (p.c.init_matrix)(matrix.as_mut_ptr() as *mut [c_int; 4]) };
    result = result.wrapping_add(matrix[0].wrapping_add(matrix[11]));

    result = result.wrapping_add(alloc_result);

    if q[2] != 0 {
        result = result.wrapping_mul(q[2]).wrapping_div(100);
    }
    if q[3] != 0 {
        result = result.wrapping_add(q[3]);
    }
    result
}

#[test]
fn row43_staged_crosscheck_against_c_primitives() {
    let p = load();
    let mut rng = Rng::new(0x9001);
    for i in 0..N_HEAP {
        let q = [rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed(), rng.i32_mixed()];
        let bonus = if q[0] > 0 { 10 } else { 0 };
        let want: Vec<c_int> = [1, 2].iter().map(|ph| model_arity4(&p, q, ph + bonus)).collect();

        let ctx = format!("row43 iter {i}: arity4{q:?}");
        // Strict, ORDERED C-vs-Rust differential (adjacent, heap-locked).
        assert_alloc_diff(&ctx, || unsafe { (p.c.arity4)(q[0], q[1], q[2], q[3]) }, || unsafe { (p.rust.arity4)(q[0], q[1], q[2], q[3]) });
        // Stable phase-independent C measurement for the C-vs-C checks below.
        let c = measure_pair(&ctx, || unsafe { (p.c.arity4)(q[0], q[1], q[2], q[3]) });

        for got in c {
            assert!(
                want.contains(&got),
                "[row43] iter {i}: arity4{q:?} returned {got}, model (built from the C's \
                 own primitives) allows {want:?}"
            );
        }
        // Both heap phases are visited by the two-call pair, so the pair as a
        // set must be exactly the model set.
        let mut cs = c.to_vec();
        cs.sort();
        let mut ws = want.clone();
        ws.sort();
        assert_eq!(
            cs, ws,
            "[row43] iter {i}: arity4{q:?} pair {c:?} does not cover both modelled phases {want:?}"
        );
    }
}
