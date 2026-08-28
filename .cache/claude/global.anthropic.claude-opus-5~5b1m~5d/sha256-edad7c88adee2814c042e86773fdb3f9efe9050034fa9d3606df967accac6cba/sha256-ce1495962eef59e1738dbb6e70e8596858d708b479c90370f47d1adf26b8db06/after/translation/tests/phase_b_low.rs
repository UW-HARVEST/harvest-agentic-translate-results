//! Phase B — valid-path differential tests for the LOWEST-LEVEL entry points.
//!
//! Covers `CONFIGS.md` rows 1-22:
//!   * `stbds_arrgrowf` / `stbds_arrfreef`            (rows 1-7)
//!   * `stbds_hash_bytes` / `stbds_hash_string`       (rows 8-13)
//!   * `stbds_rand_seed` seed evolution               (row 14)
//!   * `stbds_stralloc` / `stbds_strreset`            (rows 15-22)
//!
//! Every function is reached through `libloading` on both `.so`s.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

const SEED: u64 = 0xC0FF_EE00_1234_5678;

// ===========================================================================
// row 1 - arrgrowf(NULL, es, 0, min_cap<4) -> capacity clamped to 4
// ===========================================================================
#[test]
fn cfg01_arrgrowf_fresh_small_min_cap() {
    diff("cfg01", |lib, log| unsafe {
        for &es in &[1usize, 4, 8, 12, 16, 32, 64] {
            for &mc in &[1usize, 2, 3] {
                let a = (lib.arrgrowf)(std::ptr::null_mut(), es, 0, mc);
                log.usz("es", es);
                log.usz("mc", mc);
                snap_array(log, a, 0);
                (lib.arrfreef)(a);
            }
        }
    });
}

// ===========================================================================
// row 2 - arrgrowf(NULL, es, addlen, 0) -> capacity max(addlen,4)
// ===========================================================================
#[test]
fn cfg02_arrgrowf_fresh_random_addlen() {
    diff("cfg02", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 2);
        for &es in &[1usize, 8, 16, 40] {
            for _ in 0..64 {
                let addlen = rng.range(0, 4096);
                let a = (lib.arrgrowf)(std::ptr::null_mut(), es, addlen, 0);
                log.usz("es", es);
                log.usz("addlen", addlen);
                snap_array(log, a, 0);
                if !a.is_null() {
                    (lib.arrfreef)(a);
                }
            }
        }
    });
}

// ===========================================================================
// row 3 - arrgrowf on an existing array with min_cap <= cap -> identity
// ===========================================================================
#[test]
fn cfg03_arrgrowf_identity_when_capacity_suffices() {
    diff("cfg03", |lib, log| unsafe {
        for &es in &[1usize, 8, 24] {
            let a = (lib.arrgrowf)(std::ptr::null_mut(), es, 0, 32);
            (*header(a)).length = 5;
            for &(addlen, mc) in &[(0usize, 0usize), (0, 1), (0, 32), (5, 10), (27, 0), (1, 6)] {
                let b = (lib.arrgrowf)(a, es, addlen, mc);
                log.usz("es", es);
                log.usz("addlen", addlen);
                log.usz("mc", mc);
                log.flag("same_ptr", b == a);
                snap_array(log, b, 0);
            }
            (lib.arrfreef)(a);
        }
    });
}

// ===========================================================================
// rows 4+5 - the doubling branch and the exact-fit branch
// ===========================================================================
#[test]
fn cfg04_05_arrgrowf_doubling_and_exact_fit() {
    diff("cfg04_05", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 4);
        for &es in &[1usize, 8, 16] {
            for start_cap in [4usize, 5, 8, 16, 17, 100] {
                for _ in 0..24 {
                    let a = (lib.arrgrowf)(std::ptr::null_mut(), es, 0, start_cap);
                    let cap = (*header(a)).capacity;
                    (*header(a)).length = rng.below(cap + 1);
                    let len = (*header(a)).length;
                    // pick min_cap either inside (cap, 2*cap) or beyond 2*cap
                    let mc = if rng.next_u64() & 1 == 0 {
                        cap + 1 + rng.below(cap.max(1))
                    } else {
                        2 * cap + 1 + rng.below(64)
                    };
                    let addlen = rng.below(8);
                    let b = (lib.arrgrowf)(a, es, addlen, mc);
                    log.usz("es", es);
                    log.usz("start_cap", start_cap);
                    log.usz("len", len);
                    log.usz("mc", mc);
                    log.usz("addlen", addlen);
                    snap_array(log, b, 0);
                    (lib.arrfreef)(b);
                }
            }
        }
    });
}

// ===========================================================================
// row 6 - the arrput/arrmaybegrow append loop (full capacity growth sequence)
// ===========================================================================
#[test]
fn cfg06_arrgrowf_append_loop() {
    diff("cfg06", |lib, log| unsafe {
        for &es in &[1usize, 4, 8, 16] {
            let mut a: *mut c_void = std::ptr::null_mut();
            for i in 0..300u64 {
                // stbds_arrmaybegrow(a, 1)
                let need = if a.is_null() {
                    true
                } else {
                    (*header(a)).length + 1 > (*header(a)).capacity
                };
                if need {
                    a = (lib.arrgrowf)(a, es, 1, 0);
                }
                // (a)[header(a)->length++] = v
                let idx = (*header(a)).length;
                let e = (a as *mut u8).add(idx * es);
                let mut v = i.wrapping_mul(0x0101_0101_0101_0101);
                for k in 0..es {
                    *e.add(k) = (v & 0xff) as u8;
                    v = v.rotate_left(8);
                }
                (*header(a)).length = idx + 1;
                log.usz("es", es);
                log.usz("i", i as usize);
                snap_array(log, a, es * (idx + 1));
            }
            (lib.arrfreef)(a);
        }
    });
}

// ===========================================================================
// row 7 - grow, free, grow again
// ===========================================================================
#[test]
fn cfg07_arrgrowf_free_regrow() {
    diff("cfg07", |lib, log| unsafe {
        for &es in &[1usize, 8, 32] {
            for round in 0..8usize {
                let a = (lib.arrgrowf)(std::ptr::null_mut(), es, round, 4);
                log.usz("round", round);
                snap_array(log, a, 0);
                (lib.arrfreef)(a);
                let b = (lib.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
                snap_array(log, b, 0);
                (lib.arrfreef)(b);
            }
        }
    });
}

// ===========================================================================
// row 8 - hash_bytes over every length 0..=80 with random bytes
// ===========================================================================
#[test]
fn cfg08_hash_bytes_all_lengths_random() {
    diff("cfg08", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 8);
        for len in 0..=80usize {
            for _ in 0..24 {
                let mut b = rng.bytes(len.max(1));
                let h = (lib.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, 0x3141_5926);
                log.usz("len", len);
                log.blob("in", &b[..len]);
                log.usz("h", h);
            }
        }
    });
}

// ===========================================================================
// row 9 - hash_bytes with sign-extension-triggering byte patterns
// ===========================================================================
#[test]
fn cfg09_hash_bytes_patterns() {
    diff("cfg09", |lib, log| unsafe {
        let pats: Vec<Box<dyn Fn(usize) -> u8>> = vec![
            Box::new(|_| 0x00),
            Box::new(|_| 0xFF),
            Box::new(|_| 0x80),
            Box::new(|_| 0x7F),
            Box::new(|i| if i % 2 == 0 { 0x7F } else { 0x80 }),
            Box::new(|i| if i % 2 == 0 { 0x00 } else { 0xFF }),
            Box::new(|i| (0x80u8).wrapping_add(i as u8)),
            Box::new(|i| (i as u8).wrapping_mul(37)),
        ];
        for (pi, pat) in pats.iter().enumerate() {
            for len in 0..=80usize {
                let mut b: Vec<u8> = (0..len.max(1)).map(|i| pat(i)).collect();
                for &seed in &[0usize, 1, 0x3141_5926, usize::MAX] {
                    let h = (lib.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                    log.usz("pat", pi);
                    log.usz("len", len);
                    log.usz("seed", seed);
                    log.usz("h", h);
                }
            }
        }
    });
}

// ===========================================================================
// row 10 - hash_bytes seed sweep (including len == 0 with a null pointer)
// ===========================================================================
#[test]
fn cfg10_hash_bytes_seed_sweep() {
    diff("cfg10", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 10);
        let mut seeds: Vec<usize> = vec![0, 1, 2, 0x3141_5926, usize::MAX, usize::MAX - 1];
        for _ in 0..32 {
            seeds.push(rng.next_u64() as usize);
        }
        for &len in &[0usize, 1, 2, 7, 8, 9, 15, 16, 17, 63, 64, 65] {
            let mut b = rng.bytes(len.max(1));
            for &seed in &seeds {
                let h = (lib.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                log.usz("len", len);
                log.usz("seed", seed);
                log.usz("h", h);
            }
        }
        // len == 0 reads no bytes at all, so even a NULL pointer is valid input
        for &seed in &seeds {
            let h = (lib.hash_bytes)(std::ptr::null_mut(), 0, seed);
            log.usz("null0", h);
        }
    });
}

// ===========================================================================
// row 11 - hash_string over random printable strings, all lengths 0..=64
// ===========================================================================
#[test]
fn cfg11_hash_string_random_ascii() {
    diff("cfg11", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 11);
        for len in 0..=64usize {
            for _ in 0..16 {
                let mut s = rng.ascii(len);
                let h = (lib.hash_string)(s.as_mut_ptr() as *mut c_char, 0x3141_5926);
                log.usz("len", len);
                log.blob("in", &s);
                log.usz("h", h);
            }
        }
    });
}

// ===========================================================================
// row 12 - hash_string with high-bit bytes (the `(unsigned char)` cast)
// ===========================================================================
#[test]
fn cfg12_hash_string_high_bit_bytes() {
    diff("cfg12", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 12);
        for len in 1..=32usize {
            // all-0x80, all-0xFF, and random bytes in 0x80..=0xFF
            let cases: Vec<Vec<u8>> = vec![
                {
                    let mut v = vec![0x80u8; len];
                    v.push(0);
                    v
                },
                {
                    let mut v = vec![0xFFu8; len];
                    v.push(0);
                    v
                },
                {
                    let mut v: Vec<u8> =
                        (0..len).map(|_| 0x80 + (rng.next_u64() % 128) as u8).collect();
                    v.push(0);
                    v
                },
                rng.cstring(len),
            ];
            for (ci, mut s) in cases.into_iter().enumerate() {
                for &seed in &[0usize, 1, 0x3141_5926, usize::MAX] {
                    let h = (lib.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                    log.usz("case", ci);
                    log.usz("len", len);
                    log.usz("seed", seed);
                    log.blob("in", &s);
                    log.usz("h", h);
                }
            }
        }
    });
}

// ===========================================================================
// row 13 - hash_string seed sweep
// ===========================================================================
#[test]
fn cfg13_hash_string_seed_sweep() {
    diff("cfg13", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 13);
        let mut seeds: Vec<usize> = vec![0, 1, usize::MAX];
        for _ in 0..32 {
            seeds.push(rng.next_u64() as usize);
        }
        for &len in &[0usize, 1, 8, 9, 64] {
            let mut s = rng.ascii(len);
            for &seed in &seeds {
                let h = (lib.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                log.usz("len", len);
                log.usz("seed", seed);
                log.usz("h", h);
            }
        }
    });
}

// ===========================================================================
// row 14 - rand_seed + the seed self-advance visible via table->seed
// ===========================================================================
#[test]
fn cfg14_rand_seed_and_advance() {
    diff("cfg14", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 14);
        let mut seeds: Vec<usize> = vec![0, 1, 0x3141_5926, usize::MAX];
        for _ in 0..16 {
            seeds.push(rng.next_u64() as usize);
        }
        for &s in &seeds {
            (lib.rand_seed)(s);
            for _ in 0..16 {
                let t = (lib.shmode_func)(16, SH_NONE);
                snap_map(log, t, 16, KeyKind::Binary);
                hmfree(lib, t, 16);
            }
        }
    });
}

// ===========================================================================
// row 15 - stralloc: fresh arena, one string of a given length
// ===========================================================================
#[test]
fn cfg15_stralloc_single_string() {
    diff("cfg15", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 15);
        for &len in &[0usize, 1, 2, 10, 509, 510, 511, 512, 513, 1000, 4096] {
            let mut a = StringArena::zeroed();
            let mut s = rng.ascii(len);
            let p = (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            log.usz("len", len);
            snap_arena(log, &a);
            snap_stralloc_result(log, &a, p);
            (lib.strreset)(&mut a);
            snap_arena(log, &a);
        }
    });
}

// ===========================================================================
// row 16 - stralloc: many short strings (block walk + remaining exhaustion)
// ===========================================================================
#[test]
fn cfg16_stralloc_many_short() {
    diff("cfg16", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 16);
        let mut a = StringArena::zeroed();
        let mut strings: Vec<Vec<u8>> = (0..400).map(|_| { let n = rng.range(1, 60); rng.ascii(n) }).collect();
        for (i, s) in strings.iter_mut().enumerate() {
            let p = (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            log.usz("i", i);
            snap_arena(log, &a);
            snap_stralloc_result(log, &a, p);
        }
        (lib.strreset)(&mut a);
        snap_arena(log, &a);
    });
}

// ===========================================================================
// row 17 - stralloc: first string oversized -> storage==NULL splice
// ===========================================================================
#[test]
fn cfg17_stralloc_oversized_first() {
    diff("cfg17", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 17);
        for &len in &[512usize, 513, 700, 1024, 5000] {
            let mut a = StringArena::zeroed();
            let mut s = rng.ascii(len);
            let p = (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            log.usz("len", len);
            snap_arena(log, &a);
            snap_stralloc_result(log, &a, p);
            // a second string now that remaining == 0
            let mut s2 = rng.ascii(20);
            let p2 = (lib.stralloc)(&mut a, s2.as_mut_ptr() as *mut c_char);
            snap_arena(log, &a);
            snap_stralloc_result(log, &a, p2);
            (lib.strreset)(&mut a);
            snap_arena(log, &a);
        }
    });
}

// ===========================================================================
// row 18 - stralloc: short first, then oversized -> head->next splice
// ===========================================================================
#[test]
fn cfg18_stralloc_oversized_splice() {
    diff("cfg18", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 18);
        let mut a = StringArena::zeroed();
        for round in 0..12usize {
            // short string (creates / uses the head block)
            let n = rng.range(1, 40);
            let mut s = rng.ascii(n);
            let p = (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            log.usz("round", round);
            log.tag("short");
            snap_arena(log, &a);
            snap_stralloc_result(log, &a, p);
            // oversized string (spliced after the head, remaining untouched)
            let mut big = rng.ascii(2000 + round * 137);
            let q = (lib.stralloc)(&mut a, big.as_mut_ptr() as *mut c_char);
            log.tag("big");
            snap_arena(log, &a);
            snap_stralloc_result(log, &a, q);
        }
        (lib.strreset)(&mut a);
        snap_arena(log, &a);
    });
}

// ===========================================================================
// row 18b - stralloc: `len == blocksize` EXACTLY, on a NON-empty arena.
//
// This is the boundary of `if (len > blocksize)` (c_src/src/lib.c:893) and the
// only shape where `>` and `>=` differ observably:
//   * `>`  (the C) allocates a NORMAL block, so the new block becomes the head
//          (`sb->next = a->storage; a->storage = sb`), `remaining` becomes 0 and
//          the string is carved from the head at offset 8;
//   * `>=` would take the OVERSIZED path, which splices the block in as
//          `head->next` and leaves `remaining` at its old value.
// On an EMPTY arena the two are indistinguishable (both end up with
// storage = the new block and remaining = 0), which is why the arena must
// already have a head block.
// ===========================================================================
#[test]
fn cfg18b_stralloc_len_exactly_blocksize() {
    diff("cfg18b", |lib, log| unsafe {
        // After k short allocations the arena's `block` counter is k, so
        // blocksize == 512 << (k>>1). Probe len == blocksize, blocksize-1 and
        // blocksize+1 for each of the first few block sizes.
        for k in 0..6usize {
            for delta in [-1i64, 0, 1] {
                let mut a = StringArena::zeroed();
                // walk `block` up to k, each time filling the block completely
                // so that `remaining` is smaller than the next blocksize
                for j in 0..k {
                    let bs = 512usize << (j >> 1);
                    let mut fill = vec![b'f'; bs - 1]; // len == bs -> exhausts it
                    fill.push(0);
                    (lib.stralloc)(&mut a, fill.as_mut_ptr() as *mut c_char);
                }
                let blocksize = 512usize << (k >> 1);
                let want_len = (blocksize as i64 + delta) as usize; // strlen+1
                if want_len == 0 {
                    continue;
                }
                let mut s = vec![b'z'; want_len - 1];
                s.push(0);
                log.usz("k", k);
                log.isz("delta", delta as isize);
                log.usz("blocksize", blocksize);
                log.usz("len", want_len);
                log.tag("before");
                snap_arena(log, &a);
                let p = (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
                log.tag("after");
                snap_arena(log, &a);
                snap_stralloc_result(log, &a, p);
                // one more allocation, to expose which block is the head
                let mut t = vec![b'q'; 9];
                t.push(0);
                let q = (lib.stralloc)(&mut a, t.as_mut_ptr() as *mut c_char);
                snap_arena(log, &a);
                snap_stralloc_result(log, &a, q);
                (lib.strreset)(&mut a);
                snap_arena(log, &a);
            }
        }
    });
}

// ===========================================================================
// row 19 - stralloc: 200 interleaved short/oversized ops
// ===========================================================================
#[test]
fn cfg19_stralloc_interleaved() {
    diff("cfg19", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 19);
        let mut a = StringArena::zeroed();
        for i in 0..200usize {
            let len = if rng.below(4) == 0 {
                rng.range(600, 3000)
            } else {
                rng.range(0, 80)
            };
            let mut s = rng.ascii(len);
            let p = (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            log.usz("i", i);
            log.usz("len", len);
            snap_arena(log, &a);
            snap_stralloc_result(log, &a, p);
            if i == 120 {
                (lib.strreset)(&mut a);
                log.tag("reset");
                snap_arena(log, &a);
            }
        }
        (lib.strreset)(&mut a);
        snap_arena(log, &a);
    });
}

// ===========================================================================
// row 20 - stralloc: arena.block pre-set, `++block` saturation at 1<<20
// ===========================================================================
#[test]
fn cfg20_stralloc_block_saturation() {
    diff("cfg20", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 20);
        // 512 << (block>>1) crosses 1<<20 at block == 22
        for &blk in &[0u8, 1, 2, 3, 16, 18, 19, 20, 21, 22, 23, 24] {
            let mut a = StringArena::zeroed();
            a.block = blk;
            let mut s = rng.ascii(30);
            let p = (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            log.u8v("blk", blk);
            snap_arena(log, &a);
            snap_stralloc_result(log, &a, p);
            // one more allocation from the (possibly huge) block
            let mut s2 = rng.ascii(40);
            let p2 = (lib.stralloc)(&mut a, s2.as_mut_ptr() as *mut c_char);
            snap_arena(log, &a);
            snap_stralloc_result(log, &a, p2);
            (lib.strreset)(&mut a);
            snap_arena(log, &a);
        }
    });
}

// ===========================================================================
// row 21 - stralloc: block>>1 >= 64 -> the C shift is UB, x86 masks to 6 bits.
// Only values where the masked shift yields blocksize == 0 or a small block are
// used, so no absurd allocation is attempted (that would abort BOTH libraries
// identically but tell us nothing).
// ===========================================================================
#[test]
fn cfg21_stralloc_shift_count_overflow() {
    diff("cfg21", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 21);
        // (blk>>1)&63 >= 55  =>  512<<s wraps to 0
        // blk in 128..=131   =>  s in 0..=1  =>  512 / 1024
        for &blk in &[110u8, 112, 118, 126, 127, 128, 129, 130, 131, 238, 250, 254, 255] {
            for &len in &[1usize, 30, 600] {
                let mut a = StringArena::zeroed();
                a.block = blk;
                let mut s = rng.ascii(len);
                let p = (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
                log.u8v("blk", blk);
                log.usz("len", len);
                snap_arena(log, &a);
                snap_stralloc_result(log, &a, p);
                let mut s2 = rng.ascii(15);
                let p2 = (lib.stralloc)(&mut a, s2.as_mut_ptr() as *mut c_char);
                snap_arena(log, &a);
                snap_stralloc_result(log, &a, p2);
                (lib.strreset)(&mut a);
                snap_arena(log, &a);
            }
        }
    });
}

// ===========================================================================
// row 22 - strreset on every arena shape, and reuse after reset
// ===========================================================================
#[test]
fn cfg22_strreset_shapes() {
    diff("cfg22", |lib, log| unsafe {
        let mut rng = Rng::new(SEED ^ 22);

        // (a) empty arena
        let mut a = StringArena::zeroed();
        (lib.strreset)(&mut a);
        log.tag("empty");
        snap_arena(log, &a);

        // (b) one block
        let mut s = rng.ascii(10);
        (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
        (lib.strreset)(&mut a);
        log.tag("one_block");
        snap_arena(log, &a);

        // (c) many blocks
        for _ in 0..80 {
            let n = rng.range(1, 200);
            let mut s = rng.ascii(n);
            (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
        }
        snap_arena(log, &a);
        (lib.strreset)(&mut a);
        log.tag("many_blocks");
        snap_arena(log, &a);

        // (d) chain containing oversized-spliced blocks
        for i in 0..40 {
            let len = if i % 3 == 0 { 4000 } else { 20 };
            let mut s = rng.ascii(len);
            (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
        }
        snap_arena(log, &a);
        (lib.strreset)(&mut a);
        log.tag("spliced");
        snap_arena(log, &a);

        // (e) reuse after reset
        for i in 0..20 {
            let n = rng.range(1, 50);
            let mut s = rng.ascii(n);
            let p = (lib.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            log.usz("reuse", i);
            snap_arena(log, &a);
            snap_stralloc_result(log, &a, p);
        }
        (lib.strreset)(&mut a);
        snap_arena(log, &a);
    });
}
