//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//! Every call crosses the FFI boundary into BOTH the C `.so` and the Rust `.so`.

mod common;
use common::oracle::{self, Op};
use common::*;
use std::ffi::c_int;

const SEED: u64 = 0xC0FFEE_1234_5678;

// ---------------------------------------------------------------------------
// C1..C5 — create_block: name shapes and the full `flags` domain
// ---------------------------------------------------------------------------

#[test]
fn cfg_c1_create_block_empty_name() {
    let mut rng = Rng::new(SEED ^ 1);
    for i in 0..500 {
        diff_create_block(&format!("C1[{}]", i), rng.i32(), b"", rng.u8());
    }
    // Explicit boundary ids too.
    for id in [0, 1, -1, i32::MIN, i32::MAX] {
        for flags in [0u8, 1, 0x7F, 0x80, 0xFF] {
            diff_create_block("C1/boundary", id, b"", flags);
        }
    }
}

#[test]
fn cfg_c2_create_block_len1() {
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..500 {
        let ch = [rng.u8() | 1]; // any non-NUL byte
        diff_create_block(&format!("C2[{}]", i), rng.i32(), &ch, rng.u8());
    }
    for id in [i32::MIN, i32::MAX, 0] {
        diff_create_block("C2/boundary", id, b"A", 0xAA);
    }
}

#[test]
fn cfg_c3_create_block_mid_len() {
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..2000 {
        let len = rng.range_usize(2, 30);
        let mut name = Vec::with_capacity(len);
        for _ in 0..len {
            name.push(rng.u8() | 1); // non-NUL so strlen == len
        }
        diff_create_block(&format!("C3[{}] len={}", i, len), rng.i32(), &name, rng.u8());
    }
}

#[test]
fn cfg_c4_create_block_len31() {
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..500 {
        let mut name = [0u8; 31];
        for b in name.iter_mut() {
            *b = rng.u8() | 1;
        }
        // strcpy writes 31 bytes + NUL == exactly the whole array.
        diff_create_block(&format!("C4[{}]", i), rng.i32(), &name, rng.u8());
    }
}

#[test]
fn cfg_c5_create_block_all_flags() {
    for f in 0u16..=255 {
        diff_create_block(&format!("C5 flags={}", f), 42, b"Block_Alpha", f as u8);
    }
}

// ---------------------------------------------------------------------------
// C6..C10 — allocate_block / free_block: count and init_value shapes
// ---------------------------------------------------------------------------

#[test]
fn cfg_c6_allocate_zero_count() {
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..200 {
        let snap = diff_allocate(&format!("C6[{}]", i), 0, rng.i32())
            .expect("calloc(0,4) returns a unique non-NULL pointer");
        assert_eq!(snap.0, 0, "size must be 0");
        assert!(!snap.2, "data must be non-NULL for count == 0");
        assert!(snap.1.is_empty());
    }
}

#[test]
fn cfg_c7_allocate_one() {
    let mut rng = Rng::new(SEED ^ 7);
    for init in [0, 1, -1, 7, -7, i32::MIN, i32::MAX, i32::MAX - 1] {
        let s = diff_allocate("C7/boundary", 1, init).expect("non-NULL");
        assert_eq!(s.1, vec![init]);
    }
    for i in 0..1000 {
        diff_allocate(&format!("C7[{}]", i), 1, rng.i32());
    }
}

#[test]
fn cfg_c8_allocate_many() {
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..1500 {
        let count = rng.range_usize(2, 64);
        diff_allocate(&format!("C8[{}] count={}", i, count), count, rng.i32());
    }
    // Deliberate wrap-around inits with many elements.
    for init in [i32::MAX - 3, i32::MIN + 3, -1, -64, 0] {
        for count in [2usize, 3, 5, 14, 63, 64] {
            diff_allocate("C8/boundary", count, init);
        }
    }
}

#[test]
fn cfg_c9_allocate_large() {
    for count in [4096usize, 65536, 1 << 20] {
        for init in [0, -1, i32::MAX, i32::MIN] {
            let s = diff_allocate(&format!("C9 count={} init={}", count, init), count, init)
                .expect("allocatable");
            assert_eq!(s.0, count);
            assert_eq!(s.1.len(), count);
        }
    }
}

#[test]
fn cfg_c10_allocate_wrap_at_int_max() {
    // init_value + i is computed in size_t and truncated to int, so the
    // sequence wraps from INT_MAX to INT_MIN.
    let s = diff_allocate("C10", 8, i32::MAX - 2).expect("non-NULL");
    assert_eq!(
        s.1,
        vec![
            i32::MAX - 2,
            i32::MAX - 1,
            i32::MAX,
            i32::MIN,
            i32::MIN + 1,
            i32::MIN + 2,
            i32::MIN + 3,
            i32::MIN + 4
        ]
    );
    // Negative init crossing zero.
    let s = diff_allocate("C10/neg", 6, -3).expect("non-NULL");
    assert_eq!(s.1, vec![-3, -2, -1, 0, 1, 2]);
}

// ---------------------------------------------------------------------------
// C11..C14 — compute_hash: all 9 pointer-ordering combinations, with the
// operand addresses chosen by the TEST (so the comparison is deterministic and
// independent of the allocator).
// ---------------------------------------------------------------------------

#[test]
fn cfg_c11_compute_hash_9_orderings() {
    let mut arena: Vec<MemoryBlock> = vec![
        MemoryBlock {
            data: std::ptr::null_mut(),
            size: 0
        };
        2
    ];
    let lo_ptr = arena.as_mut_ptr();
    let hi_ptr = unsafe { arena.as_mut_ptr().add(1) };
    assert!(lo_ptr < hi_ptr);

    let mut ints: Vec<c_int> = vec![0; 4];
    let d_lo = ints.as_mut_ptr();
    let d_hi = unsafe { ints.as_mut_ptr().add(2) };
    assert!(d_lo < d_hi);

    let data_cases: [(*mut c_int, *mut c_int); 3] = [(d_lo, d_hi), (d_hi, d_lo), (d_lo, d_lo)];
    let struct_cases: [(bool, bool, c_int); 3] = [
        (false, true, 10), // mb1 = lo, mb2 = hi  => mb1 < mb2
        (true, false, 20), // mb1 = hi, mb2 = lo  => mb1 > mb2
        (false, false, 0), // mb1 == mb2
    ];

    for (d1, d2) in data_cases {
        for (s1_hi, s2_hi, sh) in struct_cases {
            let a = if s1_hi { hi_ptr } else { lo_ptr };
            let b = if s2_hi { hi_ptr } else { lo_ptr };
            unsafe {
                // When a == b the single struct carries one data pointer, so
                // the data comparison degenerates to "equal" (contributing 0).
                (*a).data = d1;
                (*b).data = d2;
                let expect_data = if (*a).data < (*b).data {
                    100
                } else if (*a).data > (*b).data {
                    200
                } else {
                    0
                };
                diff_compute_hash(
                    &format!("C11 a_hi={} b_hi={}", s1_hi, s2_hi),
                    a,
                    b,
                    Some(expect_data + sh),
                );
            }
        }
    }
}

#[test]
fn cfg_c12_c13_c14_compute_hash_randomized_orderings() {
    // Randomised addresses from a larger arena so the < / > / == branches on
    // BOTH pointers are hit in every combination, many times.
    let mut rng = Rng::new(SEED ^ 11);
    let mut arena: Vec<MemoryBlock> = vec![
        MemoryBlock {
            data: std::ptr::null_mut(),
            size: 0
        };
        16
    ];
    let mut ints: Vec<c_int> = vec![0; 16];
    let base = arena.as_mut_ptr();
    let ibase = ints.as_mut_ptr();

    for i in 0..3000 {
        let ia = rng.range_usize(0, 15);
        let ib = rng.range_usize(0, 15);
        let da = rng.range_usize(0, 15);
        let db = rng.range_usize(0, 15);
        unsafe {
            let a = base.add(ia);
            let b = base.add(ib);
            (*a).data = ibase.add(da);
            (*a).size = rng.range_usize(0, 99);
            (*b).data = ibase.add(db);
            (*b).size = rng.range_usize(0, 99);

            let dh = if (*a).data < (*b).data {
                100
            } else if (*a).data > (*b).data {
                200
            } else {
                0
            };
            let sh = if a < b {
                10
            } else if a > b {
                20
            } else {
                0
            };
            diff_compute_hash(&format!("C12-14[{}]", i), a, b, Some(dh + sh));
        }
    }
}

// ---------------------------------------------------------------------------
// C15..C28 — rows whose result depends on heap ADDRESSES.
//
// `compute_hash` compares `mb1->data` vs `mb2->data` and `mb1` vs `mb2`, so
// `betagamma`'s return value is a function of the allocator's history as well
// as of its arguments.  These rows are therefore compared OUT OF PROCESS: the
// test binary re-executes itself twice, loading only the C `.so` in one child
// and only the Rust `.so` in the other, so both libraries observe an identical,
// pristine heap.  (In-process, whichever library ran second would pop the
// chunks the first just freed off glibc's LIFO tcache and see the opposite
// pointer ordering — an artefact of the harness, not of the translation.)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c15_compute_hash_real_heap() {
    let mut rng = Rng::new(SEED ^ 15);
    let mut ops = Vec::new();
    for _ in 0..400 {
        ops.push(Op::Pipeline {
            count: rng.range_usize(0, 32),
            init1: rng.i32(),
            init2: rng.i32(),
        });
    }
    for count in [0usize, 1, 2, 5, 14, 64, 4096] {
        ops.push(Op::Pipeline {
            count,
            init1: 1,
            init2: 2,
        });
        ops.push(Op::Pipeline {
            count,
            init1: i32::MAX,
            init2: i32::MIN,
        });
    }
    oracle::assert_same("c15", &ops);
}

#[test]
fn cfg_c16_betagamma_residue_sweep() {
    let mut rng = Rng::new(SEED ^ 16);
    let mut ops = Vec::new();
    // Every residue of param1 % 10 in {-9..9} => block_size in {-4..14}.
    for r in -9i32..=9 {
        let mut hits = 0;
        for k in 0..60 {
            let p1 = r + 10 * k * if r < 0 { -1 } else { 1 };
            if p1 % 10 != r {
                continue;
            }
            hits += 1;
            ops.push(Op::Betagamma(p1, rng.i32(), rng.i32(), rng.i32()));
            ops.push(Op::Betagamma(
                p1,
                rng.range_i32(-40, 40),
                rng.range_i32(-40, 40),
                rng.range_i32(-40, 40),
            ));
        }
        assert!(hits > 0, "residue {} was never exercised", r);
    }
    oracle::assert_same("c16", &ops);

    // The rows that must take the error path do so identically (C19).
    // NOTE: block_size = (p1 % 10) + 5, so the error path needs block_size < 0,
    // i.e. p1 % 10 in {-9..-6}.  p1 % 10 == -5 gives block_size == 0, and
    // calloc(0, 4) SUCCEEDS — that residue is a valid row (C18), not an error.
    let l = libs();
    for r in -9i32..=-6 {
        for k in 0..5 {
            let p1 = r - 10 * k;
            assert_eq!(p1 % 10, r);
            unsafe {
                assert_eq!((l.c.betagamma)(p1, 1, 2, 3), -1, "C: p1={}", p1);
                assert_eq!((l.rs.betagamma)(p1, 1, 2, 3), -1, "Rust: p1={}", p1);
            }
        }
    }
}

#[test]
fn cfg_c20_betagamma_corner_vectors() {
    let vals = [
        0i32,
        1,
        -1,
        2,
        -2,
        10,
        -10,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    let mut ops = vec![Op::Betagamma(0, 0, 0, 0)];
    for &a in &vals {
        for &b in &vals {
            ops.push(Op::Betagamma(a, b, 0, 0));
            ops.push(Op::Betagamma(a, 0, b, 0));
            ops.push(Op::Betagamma(a, 0, 0, b));
            ops.push(Op::Betagamma(0, a, b, 0));
            ops.push(Op::Betagamma(0, 0, a, b));
        }
    }
    oracle::assert_same("c20", &ops);
}

#[test]
fn cfg_c23_betagamma_negative_division() {
    // sum1 - sum2 negative and not a multiple of 10 => C truncates toward zero.
    let mut rng = Rng::new(SEED ^ 23);
    let mut ops = Vec::new();
    for _ in 0..1500 {
        let p1 = rng.range_i32(0, 9) + 10 * rng.range_i32(0, 5); // block_size 5..14
        let p2 = p1 + rng.range_i32(1, 97); // sum2 > sum1 => negative numerator
        ops.push(Op::Betagamma(
            p1,
            p2,
            rng.range_i32(-20, 20),
            rng.range_i32(-20, 20),
        ));
    }
    for d in 1..=19 {
        ops.push(Op::Betagamma(5, 5 + d, 0, 0));
        ops.push(Op::Betagamma(5, 5 - d, 0, 0));
    }
    oracle::assert_same("c23", &ops);
}

#[test]
fn cfg_c24_betagamma_equal_params() {
    // param1 == param2 => identical element sums => (sum1-sum2)/10 == 0.
    let mut ops = Vec::new();
    for p in [0i32, 1, -1, 5, -5, 9, -9, 1234, -1234, i32::MAX, i32::MIN] {
        ops.push(Op::Betagamma(p, p, p, p));
        ops.push(Op::Betagamma(p, p, 0, 0));
    }
    oracle::assert_same("c24", &ops);
}

#[test]
fn cfg_c25_betagamma_overflow_sums() {
    // Large magnitudes so sum1/sum2 and result overflow inside the C loops.
    let big = [
        1 << 30,
        -(1 << 30),
        1 << 24,
        -(1 << 24),
        i32::MAX,
        i32::MIN,
        i32::MAX / 3,
        i32::MIN / 3,
    ];
    let mut ops = Vec::new();
    for &a in &big {
        for &b in &big {
            for &c in &big {
                ops.push(Op::Betagamma(a, b, c, 0));
                ops.push(Op::Betagamma(a, b, 0, c));
            }
        }
    }
    oracle::assert_same("c25", &ops);
}

#[test]
fn cfg_c26_betagamma_random_full_range() {
    let mut rng = Rng::new(SEED ^ 26);
    let ops: Vec<Op> = (0..4000)
        .map(|_| Op::Betagamma(rng.i32(), rng.i32(), rng.i32(), rng.i32()))
        .collect();
    oracle::assert_same("c26", &ops);
}

#[test]
fn cfg_c27_betagamma_random_small() {
    let mut rng = Rng::new(SEED ^ 27);
    let mut ops: Vec<Op> = (0..4000)
        .map(|_| {
            Op::Betagamma(
                rng.range_i32(-50, 50),
                rng.range_i32(-50, 50),
                rng.range_i32(-50, 50),
                rng.range_i32(-50, 50),
            )
        })
        .collect();
    // Exhaustive over the small square that drives every flags mask.
    for a in -12i32..=12 {
        for b in -3i32..=3 {
            ops.push(Op::Betagamma(a, b, -b, b));
        }
    }
    oracle::assert_same("c27", &ops);
}

#[test]
fn cfg_c28_manual_pipeline() {
    // Re-drive the exact low-level pipeline `betagamma` composes — allocate,
    // hash, sum, free — and then the one-shot wrapper, in the same child, so
    // the composed path and the wrapper are both covered end to end.
    let mut rng = Rng::new(SEED ^ 28);
    let mut ops = Vec::new();
    for _ in 0..500 {
        let p1 = rng.range_i32(-1000, 1000);
        let p2 = rng.range_i32(-1000, 1000);
        let bs = (p1 % 10) + 5;
        if bs < 0 {
            continue;
        }
        ops.push(Op::Pipeline {
            count: bs as usize,
            init1: p1,
            init2: p2,
        });
        ops.push(Op::Betagamma(
            p1,
            p2,
            rng.range_i32(-9, 9),
            rng.range_i32(-9, 9),
        ));
    }
    oracle::assert_same("c28", &ops);
}

#[test]
fn cfg_c28b_hash_matrix() {
    // Many live blocks at once: the full n x n compute_hash matrix must be
    // identical, which pins down the whole allocation ORDER, not just one pair.
    let mut ops = Vec::new();
    for n in [2usize, 3, 5, 8] {
        for count in [0usize, 1, 5, 14, 40] {
            ops.push(Op::HashMatrix { n, count, init: 1 });
        }
    }
    oracle::assert_same("c28b", &ops);
}

#[test]
fn cfg_c9b_allocate_out_of_process() {
    // allocate_block re-verified out of process too (element values are
    // address-independent, but this also pins `size` and data null-ness).
    let mut rng = Rng::new(SEED ^ 91);
    let mut ops = Vec::new();
    for _ in 0..300 {
        ops.push(Op::Allocate {
            count: rng.range_usize(0, 40),
            init: rng.i32(),
        });
    }
    for &init in &[0i32, -1, i32::MAX, i32::MIN, i32::MAX - 5] {
        for &count in &[0usize, 1, 2, 7, 14, 33] {
            ops.push(Op::Allocate { count, init });
        }
    }
    oracle::assert_same("c9b", &ops);
}

/// Child process entry point used by `common::oracle`.
/// A no-op when the oracle env vars are absent (i.e. during a normal run).
#[test]
fn zz_oracle_child() {
    common::oracle::maybe_run_as_child();
}

// ---------------------------------------------------------------------------
// C29..C30 — multi-block / ABI (address-independent, safe in one process)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c29_multi_block_pairwise() {
    // Many live blocks at once; element-wise identity, self-hash == 0, and
    // hashes over MIXED operands (the same two addresses handed to both
    // libraries) which must agree exactly because they are pure pointer
    // comparisons.
    let l = libs();
    let mut rng = Rng::new(SEED ^ 29);
    const N: usize = 8;
    for round in 0..50 {
        unsafe {
            let mut cv = [std::ptr::null_mut(); N];
            let mut rv = [std::ptr::null_mut(); N];
            let counts: Vec<usize> = (0..N).map(|_| rng.range_usize(0, 20)).collect();
            let inits: Vec<c_int> = (0..N).map(|_| rng.i32()).collect();
            for k in 0..N {
                cv[k] = (l.c.allocate_block)(counts[k], inits[k]);
                rv[k] = (l.rs.allocate_block)(counts[k], inits[k]);
                assert!(!cv[k].is_null() && !rv[k].is_null());
                for j in 0..counts[k] {
                    assert_eq!(
                        *(*cv[k]).data.add(j),
                        *(*rv[k]).data.add(j),
                        "C29 round {} block {} elem {}",
                        round,
                        k,
                        j
                    );
                }
                assert_eq!((*cv[k]).size, (*rv[k]).size, "C29 size");
            }
            for a in 0..N {
                for b in 0..N {
                    if a == b {
                        assert_eq!((l.c.compute_hash)(cv[a], cv[a]), 0, "C29 self-hash C");
                        assert_eq!((l.rs.compute_hash)(rv[a], rv[a]), 0, "C29 self-hash Rust");
                    }
                    // Identical operands => identical results, guaranteed.
                    let x = (l.c.compute_hash)(cv[a], rv[b]);
                    let y = (l.rs.compute_hash)(cv[a], rv[b]);
                    assert_eq!(x, y, "C29 mixed operands round {} {} {}", round, a, b);
                    let x = (l.c.compute_hash)(cv[a], cv[b]);
                    let y = (l.rs.compute_hash)(cv[a], cv[b]);
                    assert_eq!(x, y, "C29 C-owned operands round {} {} {}", round, a, b);
                    let x = (l.c.compute_hash)(rv[a], rv[b]);
                    let y = (l.rs.compute_hash)(rv[a], rv[b]);
                    assert_eq!(x, y, "C29 Rust-owned operands round {} {} {}", round, a, b);
                }
            }
            for k in (0..N).rev() {
                (l.c.free_block)(cv[k]);
                (l.rs.free_block)(rv[k]);
            }
        }
    }
}

#[test]
fn cfg_c30_abi_layout() {
    assert_eq!(std::mem::size_of::<DataBlock>(), 40, "DataBlock size");
    assert_eq!(std::mem::align_of::<DataBlock>(), 4, "DataBlock align");
    assert_eq!(std::mem::size_of::<MemoryBlock>(), 16, "MemoryBlock size");
    assert_eq!(std::mem::align_of::<MemoryBlock>(), 8, "MemoryBlock align");

    // sret return path: both libs must fill id/name/flags at the same offsets.
    let l = libs();
    unsafe {
        let c = (l.c.create_block)(0x11223344, b"Layout\0".as_ptr(), 0x5A);
        let r = (l.rs.create_block)(0x11223344, b"Layout\0".as_ptr(), 0x5A);
        assert_datablock_eq("C30", &c, &r, 7);
        assert_eq!(c.id, 0x11223344);
        assert_eq!(&c.name[..7], b"Layout\0");
        assert_eq!(c.flags, 0x5A);

        // MemoryBlock read back through the FFI at the same offsets.
        let cm = (l.c.allocate_block)(3, 7);
        let rm = (l.rs.allocate_block)(3, 7);
        assert_eq!((*cm).size, 3);
        assert_eq!((*rm).size, 3);
        assert_eq!(
            [*(*cm).data, *(*cm).data.add(1), *(*cm).data.add(2)],
            [7, 8, 9]
        );
        assert_eq!(
            [*(*rm).data, *(*rm).data.add(1), *(*rm).data.add(2)],
            [7, 8, 9]
        );
        (l.c.free_block)(cm);
        (l.rs.free_block)(rm);
    }
}
