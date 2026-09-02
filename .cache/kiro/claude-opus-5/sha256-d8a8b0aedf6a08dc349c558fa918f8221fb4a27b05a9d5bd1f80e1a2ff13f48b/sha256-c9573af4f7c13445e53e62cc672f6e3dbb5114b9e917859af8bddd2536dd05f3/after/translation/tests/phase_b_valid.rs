//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through `libloading` into the two `.so` files. Rows C1–C13
//! drive the LOW-LEVEL entry points (`create_block`, `allocate_block`,
//! `free_block`, `compute_hash`) directly; rows C14–C20 drive `betagamma` and
//! the composed pipeline.

mod common;
use common::*;
use std::ffi::c_char;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ===========================================================================
// C1–C4 — create_block  (id × name shape × all 256 flag values)
// ===========================================================================

/// Compare `create_block` over a list of (id, name-bytes-without-NUL, flags).
/// Only the bytes the C actually writes are compared: `id`, `name` up to and
/// including the copied NUL, and `flags`. The 3 trailing padding bytes and the
/// `name[]` tail past the NUL are uninitialized in C (`DataBlock block;`) and
/// are therefore excluded — comparing them would be comparing stack garbage.
fn check_create_block(pair: &Pair, cases: &[(i32, Vec<u8>, u8)], what: &str) {
    for (id, name, flags) in cases {
        assert!(name.len() < 32, "test bug: name must fit in char[32]");
        let mut cstr = name.clone();
        cstr.push(0);
        unsafe {
            let bc = (pair.c.create_block)(*id, cstr.as_ptr() as *const c_char, *flags);
            let br = (pair.rs.create_block)(*id, cstr.as_ptr() as *const c_char, *flags);
            let (dc, dr) = (defined(&bc), defined(&br));
            assert_eq!(
                dc, dr,
                "{what}: create_block(id={id}, name={:?}, flags={flags:#010b}) diverged",
                String::from_utf8_lossy(name)
            );
            // Also pin the absolute expectation against the C semantics.
            assert_eq!(dc.id, *id, "{what}: id not copied verbatim");
            assert_eq!(dc.flags, *flags, "{what}: flags not copied verbatim");
            assert_eq!(&dc.name[..dc.name.len() - 1], &name[..], "{what}: name body");
        }
    }
}

#[test]
fn cfg_c1_create_block_empty_name() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 1);
    let mut cases = Vec::new();
    for flags in 0u16..=255 {
        cases.push((rng.interesting_i32(), Vec::new(), flags as u8));
    }
    for id in [0, 1, -1, i32::MIN, i32::MAX] {
        cases.push((id, Vec::new(), 0));
        cases.push((id, Vec::new(), 255));
    }
    check_create_block(&pair, &cases, "C1");
}

#[test]
fn cfg_c2_create_block_len1_name() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 2);
    let mut cases = Vec::new();
    for flags in 0u16..=255 {
        // every byte value 1..=255 as a 1-char name, cycled against every flag
        let ch = ((flags % 255) + 1) as u8;
        cases.push((rng.interesting_i32(), vec![ch], flags as u8));
    }
    check_create_block(&pair, &cases, "C2");
}

#[test]
fn cfg_c3_create_block_random_names() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 3);
    let mut cases = Vec::new();
    for _ in 0..4000 {
        let len = 2 + rng.below(29) as usize; // 2..=30
        let name: Vec<u8> = (0..len).map(|_| 1 + (rng.below(255) as u8)).collect();
        cases.push((rng.interesting_i32(), name, rng.next_u32() as u8));
    }
    check_create_block(&pair, &cases, "C3");
}

#[test]
fn cfg_c4_create_block_len31_name() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 4);
    let mut cases = Vec::new();
    for _ in 0..600 {
        // 31 bytes + NUL exactly fills char[32]: the largest non-overflowing name.
        let name: Vec<u8> = (0..31).map(|_| 1 + (rng.below(255) as u8)).collect();
        cases.push((rng.interesting_i32(), name, rng.next_u32() as u8));
    }
    cases.push((7, vec![b'Z'; 31], 0b1010_1010));
    cases.push((i32::MIN, vec![0xFF; 31], 0xFF));
    check_create_block(&pair, &cases, "C4");
}

// ===========================================================================
// C5–C8 — allocate_block / free_block
// ===========================================================================

/// Compare the full observable state of `allocate_block`: NULL-ness, `size`,
/// and every element of `data`. Addresses are deliberately NOT compared here
/// (those are covered under equalised heap state in C13/C19/C20).
fn check_allocate(pair: &Pair, cases: &[(usize, i32)], what: &str) {
    for &(count, init) in cases {
        unsafe {
            let mc = (pair.c.allocate_block)(count, init);
            let mr = (pair.rs.allocate_block)(count, init);
            assert_eq!(
                mc.is_null(),
                mr.is_null(),
                "{what}: allocate_block({count}, {init}) NULL-ness diverged \
                 (C null={}, Rust null={})",
                mc.is_null(),
                mr.is_null()
            );
            if mc.is_null() {
                continue;
            }
            assert_eq!(
                (*mc).size,
                (*mr).size,
                "{what}: allocate_block({count}, {init}).size diverged"
            );
            assert_eq!((*mc).size, count, "{what}: size must equal count");
            assert_eq!(
                (*mc).data.is_null(),
                (*mr).data.is_null(),
                "{what}: data NULL-ness diverged"
            );
            if !(*mc).data.is_null() && count > 0 {
                let a = std::slice::from_raw_parts((*mc).data, count);
                let b = std::slice::from_raw_parts((*mr).data, count);
                assert_eq!(
                    a, b,
                    "{what}: allocate_block({count}, {init}) contents diverged"
                );
                // Pin the C semantics: `init_value + i` in size_t, truncated to int.
                for (i, &v) in a.iter().enumerate() {
                    let expect = (init as isize as usize).wrapping_add(i) as u32 as i32;
                    assert_eq!(v, expect, "{what}: element {i} of ({count},{init})");
                }
            }
            (pair.c.free_block)(mc);
            (pair.rs.free_block)(mr);
        }
    }
}

#[test]
fn cfg_c5_allocate_zero_count() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 5);
    let mut cases = vec![(0usize, 0i32), (0, i32::MIN), (0, i32::MAX), (0, -1)];
    for _ in 0..500 {
        cases.push((0, rng.interesting_i32()));
    }
    check_allocate(&pair, &cases, "C5");
}

#[test]
fn cfg_c6_allocate_count_one() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 6);
    let mut cases = vec![(1usize, 0i32), (1, i32::MIN), (1, i32::MAX), (1, -1), (1, 1)];
    for _ in 0..2000 {
        cases.push((1, rng.interesting_i32()));
    }
    check_allocate(&pair, &cases, "C6");
}

#[test]
fn cfg_c7_allocate_small_counts() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 7);
    let mut cases = Vec::new();
    // 2..=14 covers every count `betagamma` can ever request, plus 2/3.
    for count in 2usize..=14 {
        for init in [0i32, -1, 1, i32::MIN, i32::MAX, i32::MAX - 7, i32::MIN + 3] {
            cases.push((count, init));
        }
        for _ in 0..200 {
            cases.push((count, rng.interesting_i32()));
        }
    }
    check_allocate(&pair, &cases, "C7");
}

#[test]
fn cfg_c8_allocate_large_count_wrapping() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 8);
    let mut cases = vec![
        // init_value chosen so `init_value + i` wraps INT_MAX -> INT_MIN mid-array
        (65536usize, i32::MAX - 3),
        (65536, i32::MAX),
        (65536, i32::MIN),
        (65536, 0),
        (1024, i32::MAX - 500),
        (100, i32::MAX - 50),
        (4096, -2048),
    ];
    for _ in 0..40 {
        cases.push((1 + rng.below(70000) as usize, rng.interesting_i32()));
    }
    check_allocate(&pair, &cases, "C8");
}

// ===========================================================================
// C9–C12 — compute_hash over synthesised MemoryBlocks
//
// These use STACK-allocated MemoryBlock structs with hand-picked `data`
// values, which lets us hit all 9 combinations of the two 3-way branches
// deterministically (impossible with real allocations, where the allocator
// picks the ordering). No dereference of `data` happens in compute_hash, so
// synthetic address values are legal inputs.
// ===========================================================================

/// Call `compute_hash` with fully synthesised operands. `data` values are used
/// only as integers by the C (compared, never dereferenced).
///
/// `struct_order` selects the SECOND branch's outcome independently of the
/// first: `Some(false)` => `mb1 < mb2`, `Some(true)` => `mb1 > mb2`,
/// `None` => `mb1 == mb2` (aliased, which forces `d1 == d2` too).
///
/// The two `MemoryBlock`s live on the stack, whose relative addresses we do not
/// control, so we determine the low/high local FIRST and only then write `d1`
/// into whichever one will be passed as argument 1. Doing it the other way
/// round would couple the two branches together.
fn hash_synth(pair: &Pair, d1: usize, d2: usize, struct_order: Option<bool>) -> (i32, i32) {
    let mut a = MemoryBlock {
        data: std::ptr::null_mut(),
        size: 0,
    };
    let mut b = MemoryBlock {
        data: std::ptr::null_mut(),
        size: 0,
    };

    let pa: *mut MemoryBlock = &mut a;
    let pb: *mut MemoryBlock = &mut b;
    let (lo, hi) = if (pa as usize) < (pb as usize) {
        (pa, pb)
    } else {
        (pb, pa)
    };

    let (arg1, arg2): (*mut MemoryBlock, *mut MemoryBlock) = match struct_order {
        Some(false) => (lo, hi), // mb1 < mb2
        Some(true) => (hi, lo),  // mb1 > mb2
        None => (lo, lo),        // mb1 == mb2
    };

    unsafe {
        (*arg1).data = d1 as *mut std::ffi::c_int;
        // For the aliased case arg1 == arg2, so this second store wins; callers
        // only use `None` with d1 == d2, keeping that consistent.
        (*arg2).data = d2 as *mut std::ffi::c_int;
        if struct_order.is_none() {
            debug_assert_eq!(d1, d2, "aliased operands cannot have distinct data");
        }
        (
            (pair.c.compute_hash)(arg1, arg2),
            (pair.rs.compute_hash)(arg1, arg2),
        )
    }
}


fn check_hash_case(pair: &Pair, d1: usize, d2: usize, mode: Option<bool>, what: &str) {
    let (hc, hr) = hash_synth(pair, d1, d2, mode);
    assert_eq!(
        hc, hr,
        "{what}: compute_hash(d1={d1:#x}, d2={d2:#x}, struct-order={mode:?}) diverged"
    );
    // Cross-check against the hand-derived C expectation for the data half.
    let data_part = if d1 < d2 {
        100
    } else if d1 > d2 {
        200
    } else {
        0
    };
    let struct_part = match mode {
        Some(false) => 10,
        Some(true) => 20,
        None => 0,
    };
    assert_eq!(
        hc,
        data_part + struct_part,
        "{what}: C hash disagrees with the C source's own formula"
    );
}

#[test]
fn cfg_c9_compute_hash_data_less() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 9);
    for mode in [Some(false), Some(true)] {
        check_hash_case(&pair, 0x1000, 0x2000, mode, "C9");
        check_hash_case(&pair, 0, 1, mode, "C9");
        check_hash_case(&pair, 0, usize::MAX, mode, "C9");
        check_hash_case(&pair, usize::MAX - 1, usize::MAX, mode, "C9");
        for _ in 0..500 {
            let x = rng.next_u64() as usize;
            let y = rng.next_u64() as usize;
            let (lo, hi) = (x.min(y), x.max(y));
            if lo != hi {
                check_hash_case(&pair, lo, hi, mode, "C9");
            }
        }
    }
    // d1 < d2 with p1 == p2 is unreachable (aliasing forces d1 == d2), so the
    // third struct-ordering is covered by C11 instead.
}

#[test]
fn cfg_c10_compute_hash_data_greater() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 10);
    for mode in [Some(false), Some(true)] {
        check_hash_case(&pair, 0x2000, 0x1000, mode, "C10");
        check_hash_case(&pair, 1, 0, mode, "C10");
        check_hash_case(&pair, usize::MAX, 0, mode, "C10");
        check_hash_case(&pair, usize::MAX, usize::MAX - 1, mode, "C10");
        for _ in 0..500 {
            let x = rng.next_u64() as usize;
            let y = rng.next_u64() as usize;
            let (lo, hi) = (x.min(y), x.max(y));
            if lo != hi {
                check_hash_case(&pair, hi, lo, mode, "C10");
            }
        }
    }
}

#[test]
fn cfg_c11_compute_hash_data_equal() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 11);
    for mode in [Some(false), Some(true), None] {
        // includes data == NULL on both sides (read, never dereferenced)
        for d in [0usize, 1, 0x1000, usize::MAX, usize::MAX / 2] {
            check_hash_case(&pair, d, d, mode, "C11");
        }
        for _ in 0..300 {
            let d = rng.next_u64() as usize;
            check_hash_case(&pair, d, d, mode, "C11");
        }
    }
}

#[test]
fn cfg_c12_compute_hash_random_addresses() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 12);
    // Unconstrained random addresses, deliberately including values whose
    // *signed* interpretation has the opposite ordering to their unsigned one.
    // C pointer relationals are unsigned, so this pins that the Rust used
    // `as usize` and not a signed comparison.
    let sign_traps: &[(usize, usize)] = &[
        (0x0000_0000_0000_0001, 0xFFFF_FFFF_FFFF_FFFF),
        (0xFFFF_FFFF_FFFF_FFFF, 0x0000_0000_0000_0001),
        (0x7FFF_FFFF_FFFF_FFFF, 0x8000_0000_0000_0000),
        (0x8000_0000_0000_0000, 0x7FFF_FFFF_FFFF_FFFF),
        (0x0000_0000_8000_0000, 0xFFFF_FFFF_0000_0000),
    ];
    for &(d1, d2) in sign_traps {
        for mode in [Some(false), Some(true)] {
            check_hash_case(&pair, d1, d2, mode, "C12/sign");
        }
    }
    for _ in 0..3000 {
        let d1 = rng.next_u64() as usize;
        let d2 = rng.next_u64() as usize;
        let mode = if rng.below(2) == 0 {
            Some(false)
        } else {
            Some(true)
        };
        check_hash_case(&pair, d1, d2, mode, "C12");
    }
}

#[test]
fn cfg_c13_compute_hash_real_allocations() {
    // Real allocations => address-sensitive => must be compared under
    // identical heap state, so this runs through the forked runner.
    let pair = load_pair();
    let counts: Vec<usize> = (0usize..=14).chain([64, 1024, 65536]).collect();
    let n = counts.len() * 2;
    let counts_for_child = counts.clone();
    let (c, rs) = dual_i32_batch(&pair, n, move |imp, out| unsafe {
        let mut k = 0;
        for &count in &counts_for_child {
            let m1 = (imp.allocate_block)(count, 11);
            let m2 = (imp.allocate_block)(count, 22);
            // both argument orders
            out[k] = (imp.compute_hash)(m1, m2);
            out[k + 1] = (imp.compute_hash)(m2, m1);
            k += 2;
            (imp.free_block)(m1);
            (imp.free_block)(m2);
        }
    });
    assert_i32_batches_eq("C13", &c, &rs, |i| {
        format!("count={} order={}", counts[i / 2], if i % 2 == 0 { "1,2" } else { "2,1" })
    });
    // Sanity: every result must be a legal (data-part + struct-part) sum.
    for &v in c.iter() {
        assert!(
            [0, 10, 20, 100, 110, 120, 200, 210, 220].contains(&v),
            "C13: impossible hash {v}"
        );
    }
}

// ===========================================================================
// C14–C20 — betagamma and the composed pipeline (all address-sensitive)
// ===========================================================================

fn check_betagamma(pair: &Pair, cases: Vec<[i32; 4]>, what: &'static str) {
    let n = cases.len();
    let batch = cases.clone();
    let (c, rs) = dual_i32_batch(pair, n, move |imp, out| unsafe {
        for (i, p) in batch.iter().enumerate() {
            out[i] = (imp.betagamma)(p[0], p[1], p[2], p[3]);
        }
    });
    assert_i32_batches_eq(what, &c, &rs, |i| {
        let p = cases[i];
        format!("betagamma({}, {}, {}, {})", p[0], p[1], p[2], p[3])
    });
}

#[test]
fn cfg_c14_betagamma_positive_residues() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 14);
    let mut cases = Vec::new();
    // every non-negative residue of param1 % 10 -> block_size 5..14
    for r in 0i32..10 {
        for mult in [0i32, 1, 2, 7, 100, 1000, 214748, 200_000_000] {
            let param1 = mult.wrapping_mul(10).wrapping_add(r);
            if param1 < 0 {
                continue;
            }
            for _ in 0..30 {
                cases.push([param1, rng.interesting_i32(), rng.interesting_i32(), rng.interesting_i32()]);
            }
            cases.push([param1, 0, 0, 0]);
            cases.push([param1, i32::MAX, i32::MIN, 0]);
        }
    }
    check_betagamma(&pair, cases, "C14");
}

#[test]
fn cfg_c15_betagamma_negative_valid_residues() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 15);
    let mut cases = Vec::new();
    // param1 < 0 with |param1| % 10 in 0..=5  =>  block_size 5,4,3,2,1,0 (valid)
    for r in 0i32..=5 {
        for mult in [1i32, 2, 3, 10, 1000, 21474836] {
            let param1 = -(mult.wrapping_mul(10).wrapping_add(r));
            for _ in 0..40 {
                cases.push([param1, rng.interesting_i32(), rng.interesting_i32(), rng.interesting_i32()]);
            }
            cases.push([param1, 0, 0, 0]);
            cases.push([param1, i32::MIN, i32::MAX, -1]);
        }
    }
    check_betagamma(&pair, cases, "C15");
}

#[test]
fn cfg_c16_betagamma_extreme_grid() {
    let pair = load_pair();
    let vals = [i32::MIN, -1, 0, 1, i32::MAX];
    let mut cases = Vec::new();
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    cases.push([a, b, c, d]);
                }
            }
        }
    }
    assert_eq!(cases.len(), 625);
    check_betagamma(&pair, cases, "C16");
}

#[test]
fn cfg_c17_betagamma_random_fullrange() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 17);
    let mut cases = Vec::with_capacity(20_000);
    for _ in 0..20_000 {
        cases.push([
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ]);
    }
    check_betagamma(&pair, cases, "C17");
}

#[test]
fn cfg_c18_betagamma_division_boundaries() {
    let pair = load_pair();
    let mut cases = Vec::new();
    // sum1 - sum2 == n*(param1 - param2) for block_size n, so sweep the
    // difference across every residue mod 10, both signs, at every block_size.
    for r in 0i32..10 {
        let param1 = 1000 + r; // block_size = r + 5
        for delta in -25i32..=25 {
            cases.push([param1, param1.wrapping_sub(delta), 0, 0]);
            cases.push([param1, param1.wrapping_sub(delta), 1, -1]);
        }
        // negative dividend: C truncates toward zero, not toward -inf
        for delta in [-9i32, -5, -1, 1, 5, 9, 10, -10, 11, -11] {
            cases.push([param1, param1.wrapping_sub(delta), 7, 3]);
        }
    }
    check_betagamma(&pair, cases, "C18");
}

#[test]
fn cfg_c19_manual_pipeline_matches_betagamma() {
    // Drive allocate_block -> compute_hash -> sum loops -> free_block through
    // the .so exports in exactly the order betagamma does, and check both that
    // C and Rust agree AND that the hand-composed pipeline reproduces the
    // library's own betagamma return value.
    let pair = load_pair();
    let params: Vec<[i32; 4]> = vec![
        [0, 0, 0, 0],
        [1, 2, 3, 4],
        [7, -3, 11, 5],
        [-5, 1, 1, 1],
        [-15, 7, -7, 2],
        [12345, -999, 7, 8],
        [9, i32::MAX, i32::MIN, 0],
        [1_000_003, -1_000_003, 5, -5],
        [-1000, 1000, 0, 0],
        [14, 14, 14, 14],
    ];
    let n = params.len() * 2;
    let batch = params.clone();
    let (c, rs) = dual_i32_batch(&pair, n, move |imp, out| unsafe {
        for (i, p) in batch.iter().enumerate() {
            let (p1, p2, p3, p4) = (p[0], p[1], p[2], p[3]);

            // --- replicate betagamma's flag loop with the C's fixed table
            let mut result: i32 = 0;
            for (id, flags) in [(1i32, 0xAAu8), (2, 0xCC), (3, 0xF0)] {
                let f = flags as i32;
                let mut fc: i32 = 0;
                if f & 0x0F != 0 {
                    fc = fc.wrapping_add(p1);
                }
                if f & 0xF0 != 0 {
                    fc = fc.wrapping_add(p2);
                }
                if f & 0xAA != 0 {
                    fc = fc.wrapping_add(p3);
                }
                if f & 0x55 != 0 {
                    fc = fc.wrapping_add(p4);
                }
                result = result.wrapping_add(fc.wrapping_mul(id));
            }

            // --- the low-level pipeline, via the .so exports
            let block_size = (p1.wrapping_rem(10).wrapping_add(5)) as isize as usize;
            let m1 = (imp.allocate_block)(block_size, p1);
            let m2 = (imp.allocate_block)(block_size, p2);
            let composed = if m1.is_null() || m2.is_null() {
                (imp.free_block)(m1);
                (imp.free_block)(m2);
                -1
            } else {
                result = result.wrapping_add((imp.compute_hash)(m1, m2));
                let mut s1: i32 = 0;
                let mut s2: i32 = 0;
                for k in 0..(*m1).size {
                    s1 = s1.wrapping_add(*(*m1).data.add(k));
                }
                for k in 0..(*m2).size {
                    s2 = s2.wrapping_add(*(*m2).data.add(k));
                }
                result = result.wrapping_add(s1.wrapping_sub(s2).wrapping_div(10));
                if (*m1).data != (*m2).data {
                    result = result.wrapping_add(99);
                }
                if !((*m1).data as usize == 0) && !((*m2).data as usize == 0) {
                    result = result.wrapping_add(255);
                }
                (imp.free_block)(m1);
                (imp.free_block)(m2);
                result
            };

            out[2 * i] = composed;
            out[2 * i + 1] = (imp.betagamma)(p1, p2, p3, p4);
        }
    });
    assert_i32_batches_eq("C19", &c, &rs, |i| {
        let p = params[i / 2];
        format!(
            "{} for ({}, {}, {}, {})",
            if i % 2 == 0 { "manual pipeline" } else { "betagamma" },
            p[0],
            p[1],
            p[2],
            p[3]
        )
    });
    // The hand-composed pipeline must reproduce betagamma exactly. (Heap state
    // differs slightly between the two calls, so only the -1 / non--1 outcome
    // and the non-address-dependent magnitude are compared strictly; the
    // address-dependent part can only differ by the hash term.)
    for i in 0..params.len() {
        let (manual, real) = (c[2 * i], c[2 * i + 1]);
        assert_eq!(
            manual == -1,
            real == -1,
            "C19: error-path disagreement for {:?}",
            params[i]
        );
        if manual != -1 {
            let diff = (manual as i64 - real as i64).abs();
            assert!(
                [0i64, 10, 20, 90, 100, 110, 120, 200, 210, 220].contains(&diff),
                "C19: manual pipeline {manual} vs betagamma {real} for {:?} \
                 differs by {diff}, which is not an explainable hash delta",
                params[i]
            );
        }
    }
}

#[test]
fn cfg_c20_interleaved_heap_churn() {
    // Exercise ALL FIVE entry points interleaved, so the allocator is in a
    // non-pristine, churned state when the address-sensitive comparisons run.
    // This is where a translation that allocated a different number/size of
    // chunks than the C would show up.
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 20);
    let script: Vec<[i32; 4]> = (0..400)
        .map(|_| {
            [
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            ]
        })
        .collect();
    let churn: Vec<usize> = (0..400).map(|_| rng.below(40) as usize).collect();

    let n = script.len();
    let s = script.clone();
    let ch = churn.clone();
    let (c, rs) = dual_i32_batch(&pair, n, move |imp, out| unsafe {
        let mut keep: [*mut MemoryBlock; 8] = [std::ptr::null_mut(); 8];
        for i in 0..s.len() {
            // churn: leave some blocks live across the betagamma call
            let slot = i % 8;
            if !keep[slot].is_null() {
                (imp.free_block)(keep[slot]);
            }
            keep[slot] = (imp.allocate_block)(ch[i], i as i32);

            // create_block writes only to the stack; call it for coverage
            let name = b"churn\0";
            let _ = (imp.create_block)(i as i32, name.as_ptr() as *const c_char, (i & 0xFF) as u8);

            let p = s[i];
            out[i] = (imp.betagamma)(p[0], p[1], p[2], p[3]);
        }
        for slot in 0..8 {
            (imp.free_block)(keep[slot]);
        }
    });
    assert_i32_batches_eq("C20", &c, &rs, |i| {
        let p = script[i];
        format!(
            "after churn count={} : betagamma({}, {}, {}, {})",
            churn[i], p[0], p[1], p[2], p[3]
        )
    });
}
