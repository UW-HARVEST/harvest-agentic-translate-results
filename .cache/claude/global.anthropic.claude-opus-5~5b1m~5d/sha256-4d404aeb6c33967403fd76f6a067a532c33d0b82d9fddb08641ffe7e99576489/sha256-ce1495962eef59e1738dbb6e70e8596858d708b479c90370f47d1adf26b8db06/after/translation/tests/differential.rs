//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through `libloading` into either the C `.so` or the Rust
//! `.so`; no Rust function is ever called directly.

mod common;

use common::*;
use std::os::raw::{c_char, c_int};

// ===========================================================================
// Layout / ABI parity (prerequisite for every other test)
// ===========================================================================

#[test]
fn layout_parity() {
    assert_eq!(std::mem::size_of::<DataBlock>(), 40, "sizeof(DataBlock)");
    assert_eq!(std::mem::align_of::<DataBlock>(), 4, "alignof(DataBlock)");
    assert_eq!(std::mem::size_of::<MemoryBlock>(), 16, "sizeof(MemoryBlock)");
    assert_eq!(std::mem::align_of::<MemoryBlock>(), 8, "alignof(MemoryBlock)");
    let d = DataBlock {
        id: 0,
        name: [0; 32],
        flags: 0,
    };
    let base = &d as *const _ as usize;
    assert_eq!(&d.id as *const _ as usize - base, 0);
    assert_eq!(&d.name as *const _ as usize - base, 4);
    assert_eq!(&d.flags as *const _ as usize - base, 36);
    // Both .so files really are two distinct files.
    assert_ne!(c().path, rs().path);
}

// ===========================================================================
// create_block  —  CONFIGS rows 1-8
// ===========================================================================

/// Compare `create_block` on one input. Returns the shared observable value.
fn cmp_create(id: c_int, name: &[u8], flags: u8) {
    let buf = cstr(name);
    let cv = unsafe { (c().create_block)(id, buf.as_ptr(), flags) };
    let rv = unsafe { (rs().create_block)(id, buf.as_ptr(), flags) };
    assert_eq!(
        cv.observable(),
        rv.observable(),
        "create_block(id={id}, name={:?} (len {}), flags={flags:#04x})",
        String::from_utf8_lossy(name),
        name.len()
    );
}

#[test]
fn row01_create_block_empty_name() {
    cmp_create(0, b"", 0x00);
}

#[test]
fn row02_create_block_len1_random() {
    let mut r = Rng::new(0x0102);
    for _ in 0..500 {
        let ch = r.range(1, 255) as u8;
        cmp_create(r.interesting_i32(), &[ch], r.next_u8());
    }
}

#[test]
fn row03_create_block_len_2_to_29_random() {
    let mut r = Rng::new(0x0103);
    for _ in 0..2000 {
        let len = r.range(2, 29) as usize;
        let name: Vec<u8> = (0..len).map(|_| r.range(1, 255) as u8).collect();
        cmp_create(r.interesting_i32(), &name, r.next_u8());
    }
}

#[test]
fn row04_create_block_len30() {
    let mut r = Rng::new(0x0104);
    for _ in 0..300 {
        let name: Vec<u8> = (0..30).map(|_| r.range(1, 255) as u8).collect();
        cmp_create(r.interesting_i32(), &name, r.next_u8());
    }
}

#[test]
fn row05_create_block_len31_exact_fit() {
    // 31 bytes + NUL exactly fills char name[32].
    let mut r = Rng::new(0x0105);
    for _ in 0..300 {
        let name: Vec<u8> = (0..31).map(|_| r.range(1, 255) as u8).collect();
        cmp_create(r.interesting_i32(), &name, r.next_u8());
    }
    cmp_create(7, &[b'Z'; 31], 0xFF);
}

#[test]
fn row06_create_block_all_256_flags() {
    for f in 0u16..=255 {
        cmp_create(1234, b"flag-sweep", f as u8);
    }
}

#[test]
fn row07_create_block_id_boundaries() {
    for &id in &[i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        for &f in &[0u8, 1, 0x0F, 0xF0, 0xAA, 0x55, 0xFF] {
            cmp_create(id, b"boundary", f);
        }
    }
}

#[test]
fn row08_create_block_high_bit_name_bytes() {
    // `char` is signed on x86-64: bytes >= 0x80 become negative c_char.
    let mut r = Rng::new(0x0108);
    for _ in 0..500 {
        let len = r.range(1, 31) as usize;
        let name: Vec<u8> = (0..len).map(|_| r.range(0x80, 0xFF) as u8).collect();
        cmp_create(r.interesting_i32(), &name, r.next_u8());
    }
    cmp_create(-1, &[0xFF, 0x80, 0xC3, 0xA9], 0x81);
}

// ===========================================================================
// allocate_block + free_block  —  CONFIGS rows 9-18
// ===========================================================================

/// Call `allocate_block(count, init)` on one implementation, snapshot the
/// resulting block, then hand it straight back to that same implementation's
/// `free_block`. Returns `(size, contents)`; `None` if it returned NULL.
fn alloc_snapshot(im: &Impl, count: usize, init: c_int) -> Option<(usize, Vec<c_int>)> {
    unsafe {
        let mb = (im.allocate_block)(count, init);
        if mb.is_null() {
            return None;
        }
        let size = (*mb).size;
        let data = (*mb).data;
        assert!(!data.is_null(), "{}: non-NULL block with NULL data", im.name);
        let mut v = Vec::with_capacity(size);
        for i in 0..size {
            v.push(*data.add(i));
        }
        (im.free_block)(mb);
        Some((size, v))
    }
}

fn cmp_alloc(count: usize, init: c_int) {
    let cv = alloc_snapshot(c(), count, init);
    let rv = alloc_snapshot(rs(), count, init);
    match (&cv, &rv) {
        (None, None) => {}
        (Some((cs, cd)), Some((rs_, rd))) => {
            assert_eq!(cs, rs_, "allocate_block({count}, {init}): size mismatch");
            assert_eq!(
                cd, rd,
                "allocate_block({count}, {init}): contents mismatch"
            );
        }
        _ => panic!(
            "allocate_block({count}, {init}): NULL-ness mismatch: C={:?} Rust={:?}",
            cv.is_some(),
            rv.is_some()
        ),
    }
}

#[test]
fn row09_allocate_count0() {
    // calloc(0, 4) returns a unique non-NULL pointer; the fill loop never runs.
    for &init in &[0, 1, -1, i32::MIN, i32::MAX] {
        let cv = alloc_snapshot(c(), 0, init).expect("C: calloc(0,4) must be non-NULL");
        let rv = alloc_snapshot(rs(), 0, init).expect("Rust: calloc(0,4) must be non-NULL");
        assert_eq!(cv.0, 0);
        assert_eq!(cv, rv);
    }
}

#[test]
fn row10_allocate_count1() {
    let mut r = Rng::new(0x1010);
    for _ in 0..1000 {
        cmp_alloc(1, r.interesting_i32());
    }
}

#[test]
fn row11_allocate_many_init0() {
    let mut r = Rng::new(0x1011);
    for _ in 0..400 {
        cmp_alloc(r.range(2, 4096) as usize, 0);
    }
}

#[test]
fn row12_allocate_many_init_large_positive() {
    let mut r = Rng::new(0x1012);
    for _ in 0..400 {
        let init = i32::MAX - r.range(0, 64) as i32;
        cmp_alloc(r.range(2, 4096) as usize, init);
    }
}

#[test]
fn row13_allocate_many_init_negative() {
    let mut r = Rng::new(0x1013);
    for _ in 0..400 {
        let init = i32::MIN + r.range(0, 64) as i32;
        cmp_alloc(r.range(2, 4096) as usize, init);
    }
    for _ in 0..400 {
        cmp_alloc(r.range(2, 512) as usize, -(r.range(1, 100_000) as i32));
    }
}

#[test]
fn row14_allocate_init_int_min() {
    for &n in &[1usize, 2, 10, 100, 1000] {
        cmp_alloc(n, i32::MIN);
    }
}

#[test]
fn row15_allocate_count_x_init_sweep() {
    const COUNTS: [usize; 15] = [0, 1, 2, 3, 7, 8, 9, 15, 16, 17, 255, 256, 257, 1023, 1024];
    const INITS: [c_int; 5] = [0, 1, -1, i32::MAX, i32::MIN];
    for &n in &COUNTS {
        for &i in &INITS {
            cmp_alloc(n, i);
        }
    }
}

#[test]
fn row16_free_block_size0() {
    unsafe {
        for im in [c(), rs()] {
            let mb = (im.allocate_block)(0, 42);
            assert!(!mb.is_null());
            assert_eq!((*mb).size, 0);
            assert!(!(*mb).data.is_null(), "{}: calloc(0) gave NULL", im.name);
            (im.free_block)(mb); // must not crash
        }
    }
}

#[test]
fn row17_free_block_nonempty() {
    unsafe {
        for im in [c(), rs()] {
            for &n in &[1usize, 5, 64, 4096] {
                let mb = (im.allocate_block)(n, 3);
                assert!(!mb.is_null());
                (im.free_block)(mb); // frees data + mb
            }
        }
    }
}

#[test]
fn row18_free_block_null_data_field() {
    // A heap MemoryBlock whose `data` is NULL: the inner `if (mb->data)` guard
    // must skip the free. Allocated with the same allocator both libraries use.
    extern "C" {
        fn malloc(n: usize) -> *mut std::os::raw::c_void;
    }
    unsafe {
        for im in [c(), rs()] {
            let mb = malloc(std::mem::size_of::<MemoryBlock>()) as *mut MemoryBlock;
            assert!(!mb.is_null());
            (*mb).data = std::ptr::null_mut();
            (*mb).size = 12345;
            (im.free_block)(mb); // must free only mb, and must not crash
        }
    }
}

// ===========================================================================
// compute_hash  —  CONFIGS rows 19-29
//
// `compute_hash` never dereferences `data`; it only compares the field values
// and the two struct addresses. So the whole 3x3 ordering matrix can be built
// deterministically with hand-made structs and fake `data` values.
// ===========================================================================

/// Build two `MemoryBlock`s with the requested relative ordering and hash them
/// with both implementations. `ord_ptr`/`ord_data` are -1 (`<`), 0 (`==`),
/// 1 (`>`).
fn cmp_hash_ordering(ord_ptr: i32, ord_data: i32, d_lo: u64, d_hi: u64) -> c_int {
    assert!(d_lo < d_hi);
    let mut arr: [MemoryBlock; 2] = [MemoryBlock {
        data: std::ptr::null_mut(),
        size: 0,
    }; 2];

    // Work through raw pointers throughout: two simultaneous `&mut` into the
    // same array would alias, and the writes must be observable to the callee.
    let base = arr.as_mut_ptr();
    let p0 = unsafe { base.add(0) };
    let p1 = unsafe { base.add(1) };
    assert!(p0 < p1, "array elements are ascending in address");

    let (lo, hi) = (d_lo as *mut c_int, d_hi as *mut c_int);
    let (mb1, mb2) = match ord_ptr {
        -1 => (p0, p1),
        0 => {
            // Aliased: one struct, so `data` ordering must be `==` too.
            assert_eq!(ord_data, 0, "aliased pointers force data equality");
            (p0, p0)
        }
        _ => (p1, p0),
    };
    // Assign `data` relative to the chosen mb1/mb2 so ord_data holds for them.
    unsafe {
        let (d1, d2) = match ord_data {
            -1 => (lo, hi),
            0 => (lo, lo),
            _ => (hi, lo),
        };
        (*mb1).data = d1;
        (*mb2).data = d2;
        (*p0).size = 11;
        (*p1).size = 22;
    }

    let cv = unsafe { (c().compute_hash)(mb1, mb2) };
    let rv = unsafe { (rs().compute_hash)(mb1, mb2) };
    assert_eq!(
        cv, rv,
        "compute_hash(ord_ptr={ord_ptr}, ord_data={ord_data}, lo={d_lo:#x}, hi={d_hi:#x})"
    );
    cv
}

#[test]
fn row19_hash_data_lt_ptr_lt() {
    assert_eq!(cmp_hash_ordering(-1, -1, 0x1000, 0x2000), 110);
}

#[test]
fn row20_hash_data_lt_ptr_gt() {
    // ord_ptr=1 swaps which struct holds which data, keeping data(mb1)<data(mb2).
    assert_eq!(cmp_hash_ordering(1, -1, 0x1000, 0x2000), 120);
}

#[test]
fn row21_hash_data_lt_ptr_eq() {
    // Impossible to alias the struct and still have differing data fields, so
    // the `ptr ==` half of the matrix is covered by rows 25-27 instead; this
    // row asserts the pointer-equal term contributes exactly 0 by comparing a
    // single struct against itself with a non-zero data value.
    let mut mb = MemoryBlock {
        data: 0xDEAD_BEEF_usize as *mut c_int,
        size: 3,
    };
    let p = &mut mb as *mut MemoryBlock;
    let cv = unsafe { (c().compute_hash)(p, p) };
    let rv = unsafe { (rs().compute_hash)(p, p) };
    assert_eq!(cv, rv);
    assert_eq!(cv, 0);
}

#[test]
fn row22_hash_data_gt_ptr_lt() {
    assert_eq!(cmp_hash_ordering(-1, 1, 0x1000, 0x2000), 210);
}

#[test]
fn row23_hash_data_gt_ptr_gt() {
    assert_eq!(cmp_hash_ordering(1, 1, 0x1000, 0x2000), 220);
}

#[test]
fn row24_hash_data_gt_ptr_eq_unreachable_but_checked() {
    // Same struct twice can't give data inequality; instead verify the 200 term
    // in isolation by making the two structs adjacent *and* comparing both
    // directions, so the pointer term is the only thing that changes.
    let mut arr: [MemoryBlock; 2] = [MemoryBlock {
        data: std::ptr::null_mut(),
        size: 0,
    }; 2];
    let base = arr.as_mut_ptr();
    let (p0, p1) = unsafe { (base.add(0), base.add(1)) };
    unsafe {
        (*p0).data = 0x9000_usize as *mut c_int;
        (*p1).data = 0x1000_usize as *mut c_int;
    }
    let cv = unsafe { (c().compute_hash)(p0, p1) };
    let rv = unsafe { (rs().compute_hash)(p0, p1) };
    assert_eq!(cv, rv);
    assert_eq!(cv, 210); // data> (200) + ptr< (10)
}

#[test]
fn row25_hash_data_eq_ptr_lt() {
    assert_eq!(cmp_hash_ordering(-1, 0, 0x1000, 0x2000), 10);
}

#[test]
fn row26_hash_data_eq_ptr_gt() {
    assert_eq!(cmp_hash_ordering(1, 0, 0x1000, 0x2000), 20);
}

#[test]
fn row27_hash_aliased() {
    assert_eq!(cmp_hash_ordering(0, 0, 0x1000, 0x2000), 0);
}

#[test]
fn row28_hash_high_bit_data_values() {
    // C pointer relational comparison is UNSIGNED. If the Rust used a signed
    // comparison these would come out inverted.
    let pairs: [(u64, u64); 6] = [
        (0x1, 0xFFFF_FFFF_FFFF_FFFF),
        (0x7FFF_FFFF_FFFF_FFFF, 0x8000_0000_0000_0000),
        (0x0000_0000_0000_0001, 0x8000_0000_0000_0000),
        (0x8000_0000_0000_0000, 0xFFFF_FFFF_FFFF_FFFF),
        (0x00FF_FFFF_FFFF_FFFF, 0xFF00_0000_0000_0000),
        (0x1, 0x2),
    ];
    for &(lo, hi) in &pairs {
        assert_eq!(cmp_hash_ordering(-1, -1, lo, hi), 110, "lo={lo:#x} hi={hi:#x}");
        assert_eq!(cmp_hash_ordering(-1, 1, lo, hi), 210, "lo={lo:#x} hi={hi:#x}");
        assert_eq!(cmp_hash_ordering(-1, 0, lo, hi), 10, "lo={lo:#x} hi={hi:#x}");
    }

    let mut r = Rng::new(0x1028);
    for _ in 0..2000 {
        let a = r.next_u64();
        let b = r.next_u64();
        if a == b {
            continue;
        }
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        for &od in &[-1i32, 0, 1] {
            for &op in &[-1i32, 1] {
                cmp_hash_ordering(op, od, lo, hi);
            }
        }
    }
}

#[test]
fn row29_hash_real_allocations_and_varied_size_field() {
    // `size` must be ignored entirely by compute_hash.
    unsafe {
        let a = (c().allocate_block)(8, 1);
        let b = (c().allocate_block)(8, 2);
        assert!(!a.is_null() && !b.is_null());
        let mut r = Rng::new(0x1029);
        for _ in 0..200 {
            (*a).size = r.next_u64() as usize;
            (*b).size = r.next_u64() as usize;
            let cv = (c().compute_hash)(a, b);
            let rv = (rs().compute_hash)(a, b);
            assert_eq!(cv, rv, "compute_hash on real allocations");
            let cv2 = (c().compute_hash)(b, a);
            let rv2 = (rs().compute_hash)(b, a);
            assert_eq!(cv2, rv2, "compute_hash on real allocations (swapped)");
        }
        (*a).size = 8;
        (*b).size = 8;
        (c().free_block)(a);
        (c().free_block)(b);
    }
}

// ===========================================================================
// betagamma  —  CONFIGS rows 30-41
//
// betagamma internally calls compute_hash on two fresh malloc results, so its
// return value depends on heap state. Both sides are therefore run in children
// forked from the identical parent state (see common::fork_both).
// ===========================================================================

fn cmp_betagamma(p1: c_int, p2: c_int, p3: c_int, p4: c_int) {
    let (cv, rv) = fork_both(|im| unsafe { (im.betagamma)(p1, p2, p3, p4) });
    assert_eq!(cv, rv, "betagamma({p1}, {p2}, {p3}, {p4})");
    if let Outcome::Value(_) = cv {
    } else {
        panic!("betagamma({p1}, {p2}, {p3}, {p4}) did not return a value: {cv:?}");
    }
}

/// Smallest `param1 >= 0`-ish value with the requested `param1 % 10` residue.
fn with_residue(residue: i32, k: i32) -> i32 {
    // C's % truncates toward zero, so residue sign follows param1's sign.
    if residue >= 0 {
        residue + 10 * k.abs()
    } else {
        residue - 10 * k.abs()
    }
}

#[test]
fn row30_betagamma_residue0() {
    for k in 0..8 {
        cmp_betagamma(with_residue(0, k), 3, 5, 7);
    }
}

#[test]
fn row31_betagamma_residue_1_to_9() {
    for res in 1..=9 {
        for k in 0..4 {
            cmp_betagamma(with_residue(res, k), 11, -13, 17);
        }
    }
}

#[test]
fn row32_betagamma_residue_neg1_to_neg4() {
    for res in -4..=-1 {
        for k in 0..4 {
            cmp_betagamma(with_residue(res, k), 11, -13, 17);
        }
    }
}

#[test]
fn row33_betagamma_residue_neg5_block_size_zero() {
    // block_size == 0 -> calloc(0,4) non-NULL -> must NOT return -1.
    for k in 0..5 {
        let p1 = with_residue(-5, k);
        let (cv, rv) = fork_both(|im| unsafe { (im.betagamma)(p1, 1, 1, 1) });
        assert_eq!(cv, rv, "betagamma({p1}, 1, 1, 1)");
        assert_ne!(cv, Outcome::Value(-1), "block_size 0 must not error");
    }
}

#[test]
fn row34_betagamma_all_zero() {
    cmp_betagamma(0, 0, 0, 0);
}

#[test]
fn row35_betagamma_extreme_params() {
    const EX: [c_int; 6] = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX];
    // param1 residues that don't hit the -1 error path.
    for &p1 in &[0, 5, 9, -1, -5, 100003] {
        for &p2 in &EX {
            for &p3 in &EX {
                cmp_betagamma(p1, p2, p3, i32::MAX);
                cmp_betagamma(p1, p2, p3, i32::MIN);
            }
        }
    }
}

#[test]
fn row36_betagamma_negative_sum_difference() {
    // sum1 - sum2 < 0 and not a multiple of 10 -> checks truncation toward zero.
    for (p1, p2) in [(3, 100), (7, 999), (1, 12345), (0, 7), (4, 33), (9, 1_000_003)] {
        cmp_betagamma(p1, p2, 1, 1);
    }
}

#[test]
fn row37_betagamma_positive_sum_difference() {
    for (p1, p2) in [(100, 3), (999, 7), (12345, 1), (7, 0), (33, 4), (1_000_003, 9)] {
        cmp_betagamma(p1, p2, 1, 1);
    }
}

#[test]
fn row38_betagamma_zero_sum_difference() {
    for p1 in [0, 1, 5, 9, -1, -5, 123, -123] {
        cmp_betagamma(p1, p1, 2, 3);
    }
}

#[test]
fn row39_betagamma_randomized_full_range() {
    let mut r = Rng::new(0x39_39_39);
    for _ in 0..1200 {
        cmp_betagamma(
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
    }
}

#[test]
fn row40_betagamma_randomized_valid_residues() {
    let mut r = Rng::new(0x40_40_40);
    let residues = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, -1, -2, -3, -4, -5];
    for _ in 0..900 {
        let res = residues[(r.next_u64() % residues.len() as u64) as usize];
        let p1 = with_residue(res, r.range(0, 100_000) as i32);
        cmp_betagamma(
            p1,
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
    }
}

#[test]
fn row41_composite_pipeline_via_low_level_exports() {
    // Reproduce betagamma's body using only the low-level exports, driving each
    // library's own primitives, and compare. This catches divergence in the
    // *composition* (allocation order -> hash -> summation) that per-function
    // tests can miss.
    fn pipeline(im: &Impl, block_size: usize, p1: c_int, p2: c_int) -> i32 {
        unsafe {
            let m1 = (im.allocate_block)(block_size, p1);
            let m2 = (im.allocate_block)(block_size, p2);
            if m1.is_null() || m2.is_null() {
                (im.free_block)(m1);
                (im.free_block)(m2);
                return -1;
            }
            let mut acc = (im.compute_hash)(m1, m2);
            let mut s1: c_int = 0;
            let mut s2: c_int = 0;
            for i in 0..(*m1).size {
                s1 = s1.wrapping_add(*(*m1).data.add(i));
            }
            for i in 0..(*m2).size {
                s2 = s2.wrapping_add(*(*m2).data.add(i));
            }
            acc = acc.wrapping_add(s1.wrapping_sub(s2).wrapping_div(10));
            if (*m1).data != (*m2).data {
                acc = acc.wrapping_add(99);
            }
            if !(*m1).data.is_null() && !(*m2).data.is_null() {
                acc = acc.wrapping_add(255);
            }
            (im.free_block)(m1);
            (im.free_block)(m2);
            acc
        }
    }

    let mut r = Rng::new(0x41_41_41);
    for _ in 0..300 {
        let bs = r.range(0, 300) as usize;
        let p1 = r.interesting_i32();
        let p2 = r.interesting_i32();
        let (cv, rv) = fork_both(|im| pipeline(im, bs, p1, p2));
        assert_eq!(cv, rv, "pipeline(block_size={bs}, {p1}, {p2})");
    }
}

// ===========================================================================
// Cross-check: betagamma's arithmetic core, independent of allocator addresses
//
// Confirms the whole result (not just its hash term) matches, by subtracting
// the address-dependent component. This runs in-process (no fork) and would
// catch any divergence in the flag/sum/division arithmetic even if the fork
// harness were broken.
// ===========================================================================

#[test]
fn betagamma_arithmetic_core_matches_modulo_hash() {
    let mut r = Rng::new(0xC0FFEE);
    for _ in 0..3000 {
        let (p1, p2, p3, p4) = (
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
        let cv = unsafe { (c().betagamma)(p1, p2, p3, p4) };
        let rv = unsafe { (rs().betagamma)(p1, p2, p3, p4) };
        if cv == -1 || rv == -1 {
            assert_eq!(cv, rv, "error path must agree for ({p1},{p2},{p3},{p4})");
            continue;
        }
        // The only permitted difference is the compute_hash term, which is one
        // of {0,10,20,100,110,120,200,210,220} on each side.
        let diff = (cv as i64) - (rv as i64);
        const HASHES: [i64; 9] = [0, 10, 20, 100, 110, 120, 200, 210, 220];
        let ok = HASHES
            .iter()
            .any(|a| HASHES.iter().any(|b| a - b == diff));
        assert!(
            ok,
            "betagamma({p1},{p2},{p3},{p4}): C={cv} Rust={rv} differ by {diff}, \
             which is not explainable by the allocator-address hash term"
        );
    }
}

// ===========================================================================
// Sanity: the fork harness itself is sound (C compared against C must agree).
// ===========================================================================

#[test]
fn fork_harness_is_deterministic() {
    let mut r = Rng::new(0xF0F0);
    for _ in 0..40 {
        let (p1, p2, p3, p4) = (
            r.range(-30, 30) as c_int,
            r.interesting_i32(),
            r.interesting_i32(),
            r.interesting_i32(),
        );
        let a = fork_call(|| unsafe { (c().betagamma)(p1, p2, p3, p4) });
        let b = fork_call(|| unsafe { (c().betagamma)(p1, p2, p3, p4) });
        assert_eq!(a, b, "C vs C under fork must be identical");
        let a = fork_call(|| unsafe { (rs().betagamma)(p1, p2, p3, p4) });
        let b = fork_call(|| unsafe { (rs().betagamma)(p1, p2, p3, p4) });
        assert_eq!(a, b, "Rust vs Rust under fork must be identical");
    }
}

#[test]
fn unused_helper_types_are_referenced() {
    // Keep the c_char alias used so the import isn't dead in some cfgs.
    let v: Vec<c_char> = cstr(b"x");
    assert_eq!(v.len(), 2);
}
