//! Phase B -- valid-path differential tests.
//!
//! One test per group of rows in `CONFIGS.md`. Every call goes through
//! `libloading` into the two `.so`s; no Rust function is called directly.
//!
//! `betagamma`'s return value embeds `compute_hash`, which reads raw heap
//! addresses, so its *exact* value depends on allocator state and can only be
//! compared between two libraries running from an identical heap. The exact
//! byte-for-byte comparison for `betagamma` therefore lives in `tests/driver.rs`
//! (fresh process per `.so`, identical call sequence). What is checked here
//! in-process is the address-*independent* part, isolated by the fact that the
//! only address-dependent term is `compute_hash`'s contribution, which for two
//! distinct heap blocks is one of {110, 120, 210, 220}.

mod common;

use common::*;
use std::ffi::c_char;

/// Hash values `compute_hash` can return when called from `betagamma`, where
/// `mem1`/`mem2` and their `data` arrays are always four distinct allocations:
/// data ordering contributes 100 or 200, struct ordering 10 or 20.
const BETAGAMMA_HASHES: [i32; 4] = [110, 120, 210, 220];

fn hash_deltas() -> Vec<i32> {
    let mut v = Vec::new();
    for a in BETAGAMMA_HASHES {
        for b in BETAGAMMA_HASHES {
            v.push(a - b);
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// Assert two `betagamma` results differ only by the address-dependent
/// `compute_hash` term.
fn assert_betagamma_consistent(cv: i32, rv: i32, args: (i32, i32, i32, i32)) {
    let deltas = hash_deltas();
    let d = cv.wrapping_sub(rv);
    assert!(
        deltas.contains(&d),
        "betagamma{args:?}: C={cv} Rust={rv} differ by {d}, which is not an \
         allocator-ordering difference (allowed: {deltas:?}). This is a real \
         divergence in the address-independent arithmetic."
    );
}

fn cstr(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}

// ===========================================================================
// CONFIGS.md rows 1-5 -- create_block
// ===========================================================================

fn check_create_block(c: &Api, r: &Api, id: i32, name: &[u8], flags: u8) {
    assert!(name.len() <= 31, "row 17 of ERRORS.md: >31 is UB, not tested");
    assert!(!name.contains(&0));
    let s = cstr(name);
    unsafe {
        let cb = (c.create_block)(id, s.as_ptr(), flags);
        let rb = (r.create_block)(id, s.as_ptr(), flags);
        assert_eq!(
            defined_bytes(&cb, name.len()),
            defined_bytes(&rb, name.len()),
            "create_block(id={id}, name={:?}, flags={flags:#010b})",
            String::from_utf8_lossy(name)
        );
        assert_eq!(cb.id, rb.id);
        assert_eq!(cb.flags, rb.flags);
    }
}

#[test]
fn row01_create_block_empty_name_no_flags() {
    let (c, r) = load_both();
    check_create_block(&c, &r, 0, b"", 0x00);
}

#[test]
fn row02_create_block_len1_all_flags_negative_id() {
    let (c, r) = load_both();
    check_create_block(&c, &r, -1, b"X", 0xFF);
    check_create_block(&c, &r, i32::MIN, b"Z", 0xFF);
}

#[test]
fn row03_create_block_literal_block_shapes() {
    let (c, r) = load_both();
    // Exactly the (name, flags) triples betagamma hard-codes.
    check_create_block(&c, &r, 1, b"Block_Alpha", 0b1010_1010);
    check_create_block(&c, &r, 2, b"Block_Beta", 0b1100_1100);
    check_create_block(&c, &r, 3, b"Block_Gamma", 0b1111_0000);
    check_create_block(&c, &r, 99, b"Special", 0b1111_1111);
    check_create_block(&c, &r, 99, b"Modified", 0b1111_1111);
}

#[test]
fn row04_create_block_name_exactly_fills_array() {
    let (c, r) = load_both();
    // 31 chars + NUL == sizeof(name); the last in-bounds length.
    let name: Vec<u8> = (0..31u8).map(|i| b'a' + (i % 26)).collect();
    assert_eq!(name.len(), 31);
    check_create_block(&c, &r, 7, &name, 0b0101_0101);
}

#[test]
fn row05_create_block_randomised_cross_product() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0x05);
    // Full flags sweep x randomised name length x interesting ids.
    for flags in 0u8..=255 {
        for _ in 0..4 {
            let len = rng.below(32) as usize;
            let name: Vec<u8> = (0..len).map(|_| 1 + rng.below(255) as u8).collect();
            let id = rng.interesting_i32();
            check_create_block(&c, &r, id, &name, flags);
        }
    }
}

// ===========================================================================
// CONFIGS.md rows 6-13 -- allocate_block / free_block
// ===========================================================================

fn check_allocate(c: &Api, r: &Api, count: usize, init: i32) {
    unsafe {
        let cm = (c.allocate_block)(count, init);
        let cv = read_block(cm);
        if !cm.is_null() {
            (c.free_block)(cm);
        }

        let rm = (r.allocate_block)(count, init);
        let rv = read_block(rm);
        if !rm.is_null() {
            (r.free_block)(rm);
        }

        match (&cv, &rv) {
            (None, None) => {}
            (Some((cs, cd)), Some((rs, rd))) => {
                assert_eq!(cs, rs, "allocate_block({count}, {init}).size");
                assert_eq!(
                    cd, rd,
                    "allocate_block({count}, {init}) contents diverge (len {cs})"
                );
            }
            _ => panic!(
                "allocate_block({count}, {init}): C null={} Rust null={}",
                cm.is_null(),
                rm.is_null()
            ),
        }
    }
}

#[test]
fn row06_allocate_zero_count() {
    let (c, r) = load_both();
    // calloc(0, 4) returns a unique non-NULL pointer; loop body never runs.
    unsafe {
        let cm = (c.allocate_block)(0, 12345);
        let rm = (r.allocate_block)(0, 12345);
        assert!(!cm.is_null(), "C: calloc(0,4) unexpectedly failed");
        assert_eq!(cm.is_null(), rm.is_null());
        assert_eq!((*cm).size, 0);
        assert_eq!((*rm).size, 0);
        assert!(!(*cm).data.is_null());
        assert!(!(*rm).data.is_null());
        (c.free_block)(cm);
        (r.free_block)(rm);
    }
    check_allocate(&c, &r, 0, 0);
    check_allocate(&c, &r, 0, i32::MIN);
    check_allocate(&c, &r, 0, i32::MAX);
}

#[test]
fn row07_allocate_single_element() {
    let (c, r) = load_both();
    for init in [0i32, 1, -1, i32::MAX, i32::MIN, 42, -42] {
        check_allocate(&c, &r, 1, init);
    }
}

#[test]
fn row08_allocate_betagamma_sizes() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0x08);
    // 5..=14 is exactly the range betagamma requests via (param1 % 10) + 5.
    for count in 0..=14usize {
        for _ in 0..64 {
            check_allocate(&c, &r, count, rng.interesting_i32());
        }
    }
}

#[test]
fn row09_allocate_large() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0x09);
    for count in [1000usize, 4095, 4096, 65536] {
        for _ in 0..4 {
            check_allocate(&c, &r, count, rng.interesting_i32());
        }
    }
}

#[test]
fn row10_allocate_large_init_int_max_wraps_midarray() {
    let (c, r) = load_both();
    // init_value + i is computed in size_t then truncated to int, so the ramp
    // wraps from INT_MAX to INT_MIN partway through the array.
    for count in [1usize, 2, 14, 1000, 65536] {
        check_allocate(&c, &r, count, i32::MAX);
    }
}

#[test]
fn row11_allocate_large_init_int_min() {
    let (c, r) = load_both();
    for count in [1usize, 2, 14, 1000, 65536] {
        check_allocate(&c, &r, count, i32::MIN);
    }
}

#[test]
fn row12_allocate_init_near_int_max_partial_wrap() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0x12);
    for _ in 0..200 {
        let count = 1 + rng.below(2048) as usize;
        // Land init so that only the tail of the array wraps.
        let off = rng.below(count as u64) as i32;
        check_allocate(&c, &r, count, i32::MAX - off);
        check_allocate(&c, &r, count, i32::MIN + off);
    }
}

#[test]
fn row13_free_block_with_null_data() {
    let (c, r) = load_both();
    unsafe {
        // Build the struct with the library's own allocator, release the inner
        // array through the library, then clear `data` so free_block takes the
        // `if (mb->data)`-false branch. Exercised for both libraries.
        for (api, tag) in [(&c, "C"), (&r, "Rust")] {
            let mb = (api.allocate_block)(4, 1);
            assert!(!mb.is_null(), "{tag}: allocate_block failed");
            // free the inner array via free_block on a shallow copy, then null it
            let inner = MemoryBlock {
                data: (*mb).data,
                size: (*mb).size,
            };
            let holder = (api.allocate_block)(0, 0);
            assert!(!holder.is_null());
            // Release `inner.data` exactly once by handing it to free_block in a
            // heap struct we own, then null out the original.
            (*holder).data = inner.data;
            (api.free_block)(holder); // frees inner.data and holder
            (*mb).data = std::ptr::null_mut();
            (api.free_block)(mb); // must skip the inner free and not crash
        }
    }
}

// ===========================================================================
// CONFIGS.md rows 14-24 -- compute_hash
// ===========================================================================

/// Call `compute_hash` in both libraries with caller-supplied structs. This is
/// fully deterministic: the test chooses every pointer value that is compared.
fn check_hash(c: &Api, r: &Api, lo: MemoryBlock, hi: MemoryBlock, swap: bool, label: &str) -> i32 {
    // A 2-element array guarantees &arr[0] < &arr[1].
    let mut arr = [lo, hi];
    let p0: *mut MemoryBlock = &mut arr[0];
    let p1: *mut MemoryBlock = &mut arr[1];
    assert!(p0 < p1);
    let (a, b) = if swap { (p1, p0) } else { (p0, p1) };
    unsafe {
        let cv = (c.compute_hash)(a, b);
        let rv = (r.compute_hash)(a, b);
        assert_eq!(cv, rv, "compute_hash {label}: C={cv} Rust={rv}");
        cv
    }
}

fn mb(data: usize, size: usize) -> MemoryBlock {
    MemoryBlock {
        data: data as *mut i32,
        size,
    }
}

#[test]
fn rows14to22_compute_hash_full_ordering_matrix() {
    let (c, r) = load_both();

    // row 14: data <, mb <  -> 100 + 10
    assert_eq!(
        check_hash(&c, &r, mb(0x1000, 1), mb(0x2000, 1), false, "d< m<"),
        110
    );
    // row 15: data <, mb >  -> 100 + 20
    assert_eq!(
        check_hash(&c, &r, mb(0x2000, 1), mb(0x1000, 1), true, "d< m>"),
        120
    );
    // row 17: data >, mb <  -> 200 + 10
    assert_eq!(
        check_hash(&c, &r, mb(0x2000, 1), mb(0x1000, 1), false, "d> m<"),
        210
    );
    // row 18: data >, mb >  -> 200 + 20
    assert_eq!(
        check_hash(&c, &r, mb(0x1000, 1), mb(0x2000, 1), true, "d> m>"),
        220
    );
    // row 20: data ==, mb < -> 0 + 10
    assert_eq!(
        check_hash(&c, &r, mb(0x3000, 1), mb(0x3000, 9), false, "d== m<"),
        10
    );
    // row 21: data ==, mb > -> 0 + 20
    assert_eq!(
        check_hash(&c, &r, mb(0x3000, 1), mb(0x3000, 9), true, "d== m>"),
        20
    );
    // row 22: data ==, mb == -> 0. Requires aliasing the same struct.
    let mut only = mb(0x4000, 3);
    let p: *mut MemoryBlock = &mut only;
    unsafe {
        let cv = (c.compute_hash)(p, p);
        let rv = (r.compute_hash)(p, p);
        assert_eq!(cv, 0, "C compute_hash(p, p)");
        assert_eq!(rv, 0, "Rust compute_hash(p, p)");
    }

    // rows 16 & 19 are unreachable by construction: mb1 == mb2 forces
    // mb1->data == mb2->data, so `data <`/`data >` cannot coexist with `mb ==`.
}

#[test]
fn row23_compute_hash_randomised_pointer_values() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0x23);
    for _ in 0..4000 {
        // Include values above 2^63 so that a signed pointer comparison in the
        // translation would produce the wrong branch, and 0 (NULL) in `data`.
        let pick = |rng: &mut Rng| -> usize {
            match rng.below(6) {
                0 => 0,
                1 => usize::MAX,
                2 => 1usize << 63,
                3 => (1usize << 63) | rng.next_u64() as usize >> 1,
                4 => rng.below(4096) as usize,
                _ => rng.next_u64() as usize,
            }
        };
        let d1 = pick(&mut rng);
        let d2 = if rng.below(4) == 0 { d1 } else { pick(&mut rng) };
        let s1 = rng.next_u64() as usize;
        let s2 = rng.next_u64() as usize;
        let swap = rng.below(2) == 1;
        let h = check_hash(&c, &r, mb(d1, s1), mb(d2, s2), swap, "randomised");
        // Independent cross-check of the expected value from the C source.
        let (a_d, b_d, mb_rel) = if swap { (d2, d1, 20) } else { (d1, d2, 10) };
        let expect = (if a_d < b_d {
            100
        } else if a_d > b_d {
            200
        } else {
            0
        }) + mb_rel;
        assert_eq!(h, expect, "d1={d1:#x} d2={d2:#x} swap={swap}");
    }
}

#[test]
fn row24_compute_hash_on_real_allocations() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0x24);
    unsafe {
        for _ in 0..200 {
            let n = rng.below(20) as usize;
            // Two real blocks per library; compare in both argument orders.
            let c1 = (c.allocate_block)(n, rng.next_i32());
            let c2 = (c.allocate_block)(n, rng.next_i32());
            let r1 = (r.allocate_block)(n, rng.next_i32());
            let r2 = (r.allocate_block)(n, rng.next_i32());
            assert!(!c1.is_null() && !c2.is_null() && !r1.is_null() && !r2.is_null());

            // Feed *the same* structs to both libraries so the comparison is
            // deterministic, using the C-allocated pair and then the Rust pair.
            for (a, b) in [(c1, c2), (c2, c1), (r1, r2), (r2, r1)] {
                let cv = (c.compute_hash)(a, b);
                let rv = (r.compute_hash)(a, b);
                assert_eq!(cv, rv, "compute_hash on real allocations");
            }

            (c.free_block)(c1);
            (c.free_block)(c2);
            (r.free_block)(r1);
            (r.free_block)(r2);
        }
    }
}

// ===========================================================================
// CONFIGS.md rows 25-32 -- betagamma valid inputs (address-independent part)
// ===========================================================================

#[test]
fn row25_betagamma_all_nonnegative_residues() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0x25);
    for base in 0..10i32 {
        for k in 0..8i32 {
            let p1 = base + k * 10;
            for _ in 0..8 {
                let args = (
                    p1,
                    rng.below(1000) as i32,
                    rng.below(1000) as i32,
                    rng.below(1000) as i32,
                );
                unsafe {
                    let cv = (c.betagamma)(args.0, args.1, args.2, args.3);
                    let rv = (r.betagamma)(args.0, args.1, args.2, args.3);
                    assert_betagamma_consistent(cv, rv, args);
                }
            }
        }
    }
}

#[test]
fn row26_betagamma_block_size_zero() {
    let (c, r) = load_both();
    // param1 % 10 == -5 -> block_size == 0; both allocations still succeed.
    for p1 in [-5i32, -15, -25, -35, -105, -1005] {
        for &p in &[0i32, 1, -1, 12345, -12345, i32::MAX, i32::MIN] {
            unsafe {
                let cv = (c.betagamma)(p1, p, p, p);
                let rv = (r.betagamma)(p1, p, p, p);
                assert_ne!(cv, -1, "block_size 0 must not error (C), p1={p1}");
                assert_ne!(rv, -1, "block_size 0 must not error (Rust), p1={p1}");
                assert_betagamma_consistent(cv, rv, (p1, p, p, p));
            }
        }
    }
}

#[test]
fn row27_betagamma_small_negative_residues() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0x27);
    // param1 % 10 in {-1,-2,-3,-4} -> block_size 4,3,2,1
    for p1 in [-1i32, -2, -3, -4, -11, -12, -13, -14, -101, -1004] {
        for _ in 0..16 {
            let args = (
                p1,
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            unsafe {
                let cv = (c.betagamma)(args.0, args.1, args.2, args.3);
                let rv = (r.betagamma)(args.0, args.1, args.2, args.3);
                assert_ne!(cv, -1, "block_size 1..4 must not error, p1={p1}");
                assert_betagamma_consistent(cv, rv, args);
            }
        }
    }
}

#[test]
fn row28_betagamma_all_zero() {
    let (c, r) = load_both();
    unsafe {
        let cv = (c.betagamma)(0, 0, 0, 0);
        let rv = (r.betagamma)(0, 0, 0, 0);
        assert_betagamma_consistent(cv, rv, (0, 0, 0, 0));
    }
}

#[test]
fn row29_betagamma_all_negative_truncating_division() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0x29);
    for _ in 0..400 {
        // param1 residue kept out of the erroring set.
        let p1 = -((rng.below(10_000) as i32) * 10 + 5);
        let args = (
            p1,
            -(rng.below(10_000) as i32),
            -(rng.below(10_000) as i32),
            -(rng.below(10_000) as i32),
        );
        unsafe {
            let cv = (c.betagamma)(args.0, args.1, args.2, args.3);
            let rv = (r.betagamma)(args.0, args.1, args.2, args.3);
            assert_betagamma_consistent(cv, rv, args);
        }
    }
}

#[test]
fn row30_betagamma_full_range_random() {
    let (c, r) = load_both();
    let mut rng = Rng::new(SEED ^ 0x30);
    let mut errors = 0usize;
    let mut ok = 0usize;
    for _ in 0..3000 {
        let args = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        unsafe {
            let cv = (c.betagamma)(args.0, args.1, args.2, args.3);
            let rv = (r.betagamma)(args.0, args.1, args.2, args.3);
            // The -1 error path must agree exactly (deterministic).
            let res = args.0 % 10;
            if res + 5 < 0 {
                assert_eq!(cv, -1, "C must error for param1={} ", args.0);
                assert_eq!(rv, -1, "Rust must error for param1={}", args.0);
                errors += 1;
            } else {
                assert_betagamma_consistent(cv, rv, args);
                ok += 1;
            }
        }
    }
    assert!(errors > 100, "expected the erroring residues to be sampled");
    assert!(ok > 100);
}

#[test]
fn row31_betagamma_signed_overflow() {
    let (c, r) = load_both();
    let extremes = [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 0, 1, -1];
    // param1 must avoid the erroring residues to reach the arithmetic.
    let p1s = [0i32, 1, 2, 3, 4, 5, 6, 7, 8, 9, -5, -1, 2147483640];
    for &p1 in &p1s {
        for &p2 in &extremes {
            for &p3 in &extremes {
                for &p4 in &extremes {
                    unsafe {
                        let cv = (c.betagamma)(p1, p2, p3, p4);
                        let rv = (r.betagamma)(p1, p2, p3, p4);
                        assert_betagamma_consistent(cv, rv, (p1, p2, p3, p4));
                    }
                }
            }
        }
    }
}

#[test]
fn row32_betagamma_division_boundaries() {
    let (c, r) = load_both();
    // Sweep param2 around param1 so sum1-sum2 straddles 0 and multiples of 10,
    // where C's truncate-toward-zero division differs from flooring.
    for p1 in [0i32, 3, 7, 9, -5, -1] {
        for d in -60i32..=60 {
            let p2 = p1.wrapping_add(d);
            unsafe {
                let cv = (c.betagamma)(p1, p2, 0, 0);
                let rv = (r.betagamma)(p1, p2, 0, 0);
                assert_betagamma_consistent(cv, rv, (p1, p2, 0, 0));
            }
        }
    }
}
