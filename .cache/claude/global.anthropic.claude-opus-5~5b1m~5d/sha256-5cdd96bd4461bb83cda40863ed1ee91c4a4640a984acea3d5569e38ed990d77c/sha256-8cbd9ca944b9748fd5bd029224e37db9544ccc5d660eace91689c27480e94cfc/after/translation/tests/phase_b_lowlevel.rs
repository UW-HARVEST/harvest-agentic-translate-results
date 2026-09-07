//! Phase B — differential tests for the LOW-LEVEL entry points.
//!
//! CONFIGS.md rows 1-9 and 33-38.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// row 1/2/3: stbds_arrgrowf capacity rules
// ---------------------------------------------------------------------------

#[test]
fn row01_arrgrowf_fresh_allocation_matrix() {
    let (c, r) = apis();
    for &elemsize in &[1usize, 2, 4, 8, 16, 24, 32] {
        for &addlen in &[0usize, 1, 3, 17] {
            for &min_cap in &[0usize, 1, 4, 5, 100] {
                unsafe {
                    let pc = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let pr = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let ctx = format!("arrgrowf(NULL,{elemsize},{addlen},{min_cap})");
                    // `addlen == 0 && min_cap == 0` is the "already satisfied"
                    // early-out: the C code returns the NULL it was given.
                    assert_same(&format!("{ctx} nullness"), &pc.is_null(), &pr.is_null());
                    if pc.is_null() {
                        continue;
                    }
                    let sc = snap_arr(pc, elemsize, 0);
                    let sr = snap_arr(pr, elemsize, 0);
                    assert_same(&ctx, &sc, &sr);
                    (c.arrfreef)(pc);
                    (r.arrfreef)(pr);
                }
            }
        }
    }
}

#[test]
fn row02_arrgrowf_request_already_satisfied_returns_same_pointer() {
    let (c, r) = apis();
    let elemsize = 4usize;
    unsafe {
        let pc = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
        let pr = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
        for &(addlen, min_cap) in &[(0usize, 0usize), (0, 1), (0, 10), (5, 0), (5, 3)] {
            let qc = (c.arrgrowf)(pc, elemsize, addlen, min_cap);
            let qr = (r.arrgrowf)(pr, elemsize, addlen, min_cap);
            assert_same(
                &format!("no-op grow identity ({addlen},{min_cap})"),
                &(qc == pc),
                &(qr == pr),
            );
            assert_same(
                &format!("no-op grow state ({addlen},{min_cap})"),
                &snap_arr(qc, elemsize, 0),
                &snap_arr(qr, elemsize, 0),
            );
        }
        (c.arrfreef)(pc);
        (r.arrfreef)(pr);
    }
}

#[test]
fn row03_arrgrowf_doubling_vs_explicit() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x1111);
    for elemsize in [1usize, 4, 8, 16] {
        unsafe {
            let mut pc = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
            let mut pr = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
            for step in 0..40 {
                let addlen = rng.below(9);
                let min_cap = rng.below(64);
                // keep a plausible length so `arrlen+addlen` matters
                (*((pc as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length =
                    rng.below(4).min(4);
                (*((pr as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length =
                    (*((pc as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)).length;
                pc = (c.arrgrowf)(pc, elemsize, addlen, min_cap);
                pr = (r.arrgrowf)(pr, elemsize, addlen, min_cap);
                assert_same(
                    &format!("grow es={elemsize} step={step} addlen={addlen} min_cap={min_cap}"),
                    &snap_arr(pc, elemsize, 0),
                    &snap_arr(pr, elemsize, 0),
                );
            }
            (c.arrfreef)(pc);
            (r.arrfreef)(pr);
        }
    }
}

/// Reproduces the `arrput` macro on top of `arrgrowf` and compares every step.
unsafe fn arrput_u32(api: &Api, a: &mut *mut c_void, v: u32) {
    let elemsize = 4usize;
    let grow = {
        if a.is_null() {
            true
        } else {
            let h = &*((*a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader);
            h.length + 1 > h.capacity
        }
    };
    if grow {
        *a = (api.arrgrowf)(*a, elemsize, 1, 0);
    }
    let h = (*a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader;
    let n = (*h).length;
    *((*a as *mut u32).add(n)) = v;
    (*h).length = n + 1;
}

#[test]
fn row04_arrgrowf_append_chain() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x2222);
    unsafe {
        let mut ac: *mut c_void = std::ptr::null_mut();
        let mut ar: *mut c_void = std::ptr::null_mut();
        for i in 0..300u32 {
            let v = rng.next_u32();
            arrput_u32(c, &mut ac, v);
            arrput_u32(r, &mut ar, v);
            assert_same(
                &format!("arrput #{i}"),
                &snap_arr(ac, 4, (i + 1) as usize),
                &snap_arr(ar, 4, (i + 1) as usize),
            );
        }
        (c.arrfreef)(ac);
        (r.arrfreef)(ar);
    }
}

// ---------------------------------------------------------------------------
// rows 5-7: stbds_hash_bytes
// ---------------------------------------------------------------------------

fn hash_bytes_cmp(ctx: &str, data: &mut [u8], len: usize, seed: usize) {
    let (c, r) = apis();
    unsafe {
        let hc = (c.hash_bytes)(data.as_mut_ptr() as *mut c_void, len, seed);
        let hr = (r.hash_bytes)(data.as_mut_ptr() as *mut c_void, len, seed);
        assert_same(ctx, &hc, &hr);
    }
}

#[test]
fn row05_hash_bytes_all_tail_lengths() {
    let mut rng = Rng::new(0x3333);
    for len in 0..=24usize {
        for iter in 0..200 {
            let mut data = rng.bytes(len.max(1) + 8);
            let seed = rng.next_u64() as usize;
            hash_bytes_cmp(&format!("hash_bytes len={len} it={iter}"), &mut data, len, seed);
        }
    }
}

#[test]
fn row06_hash_bytes_high_bit_bytes() {
    let mut rng = Rng::new(0x4444);
    for len in 0..=24usize {
        for iter in 0..100 {
            let mut data = rng.bytes(len.max(1) + 8);
            // force the sign bit of every 4th byte (the `d[3]<<24` / `d[7]<<24` paths)
            for (i, b) in data.iter_mut().enumerate() {
                if i % 4 == 3 {
                    *b |= 0x80;
                }
            }
            let seed = rng.next_u64() as usize;
            hash_bytes_cmp(&format!("hash_bytes hi len={len} it={iter}"), &mut data, len, seed);
        }
        // and the all-0xff / all-0x80 extremes
        let mut ff = vec![0xffu8; len + 8];
        hash_bytes_cmp(&format!("hash_bytes 0xff len={len}"), &mut ff, len, 0);
        let mut hi = vec![0x80u8; len + 8];
        hash_bytes_cmp(&format!("hash_bytes 0x80 len={len}"), &mut hi, len, usize::MAX);
    }
}

#[test]
fn row07_hash_bytes_large_and_extreme_seeds() {
    let mut rng = Rng::new(0x5555);
    for &len in &[63usize, 64, 65, 127, 128, 1024, 4096] {
        let mut data = rng.bytes(len + 8);
        for &seed in &[0usize, 1, usize::MAX, 0x31415926, 0x8000_0000_0000_0000] {
            hash_bytes_cmp(&format!("hash_bytes big len={len} seed={seed:#x}"), &mut data, len, seed);
        }
        for _ in 0..20 {
            let seed = rng.next_u64() as usize;
            hash_bytes_cmp(&format!("hash_bytes big len={len} rnd"), &mut data, len, seed);
        }
    }
}

// ---------------------------------------------------------------------------
// row 8: stbds_hash_string
// ---------------------------------------------------------------------------

#[test]
fn row08_hash_string() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x6666);
    let mut cases: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"a\0".to_vec(),
        b"test_0\0".to_vec(),
        b"the quick brown fox jumps over the lazy dog\0".to_vec(),
        vec![0x80, 0xff, 0x7f, 0x01, 0],
        vec![0xff; 65].into_iter().chain([0u8]).collect(),
    ];
    for len in 0..=64usize {
        for _ in 0..8 {
            cases.push(rng.cstring(len));
        }
    }
    for case in cases.iter_mut() {
        for &seed in &[0usize, 1, usize::MAX, 0x31415926, 0xdead_beef_cafe_babe] {
            unsafe {
                let hc = (c.hash_string)(case.as_mut_ptr() as *mut c_char, seed);
                let hr = (r.hash_string)(case.as_mut_ptr() as *mut c_char, seed);
                assert_same(
                    &format!("hash_string len={} seed={seed:#x}", case.len() - 1),
                    &hc,
                    &hr,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 9: stbds_rand_seed drives the per-table seed and its advance
// ---------------------------------------------------------------------------

#[test]
fn row09_rand_seed_and_seed_advance() {
    let (c, r) = apis();
    for &seed in &[0usize, 1, 0x31415926, usize::MAX, 0x1234_5678_9abc_def0] {
        unsafe {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            // create several independent tables: each consumes and advances the global
            let mut seeds_c = Vec::new();
            let mut seeds_r = Vec::new();
            let mut keep_c = Vec::new();
            let mut keep_r = Vec::new();
            for _ in 0..8 {
                let ac = (c.shmode_func)(16, SH_DEFAULT);
                let ar = (r.shmode_func)(16, SH_DEFAULT);
                let sc = snap_index((ac as *mut u8).sub(16) as *mut c_void).unwrap();
                let sr = snap_index((ar as *mut u8).sub(16) as *mut c_void).unwrap();
                seeds_c.push(sc.scalars[6]);
                seeds_r.push(sr.scalars[6]);
                keep_c.push(ac);
                keep_r.push(ar);
            }
            assert_same(&format!("seed chain from {seed:#x}"), &seeds_c, &seeds_r);
            for (i, p) in keep_c.iter().enumerate() {
                (c.hmfree_func)((*p as *mut u8).sub(16) as *mut c_void, 16);
                (r.hmfree_func)((keep_r[i] as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 33-36: stbds_stralloc / stbds_strreset
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct ArenaSnap {
    remaining: usize,
    block: u8,
    mode: u8,
    nblocks: usize,
    /// returned pointer is `head.storage + remaining` (normal sub-allocation)
    p_in_head: bool,
    /// returned pointer is exactly `head.storage` (over-sized empty-arena path)
    p_is_head_storage: bool,
    content: Vec<u8>,
}

unsafe fn stralloc_step(api: &Api, arena: *mut StringArena, s: &mut [u8]) -> ArenaSnap {
    let p = (api.stralloc)(arena, s.as_mut_ptr() as *mut c_char);
    let a = &*arena;
    let head = a.storage;
    let head_storage = if head.is_null() {
        std::ptr::null_mut()
    } else {
        (*head).storage.as_ptr() as *mut c_char
    };
    let mut nblocks = 0usize;
    let mut b = head;
    while !b.is_null() {
        nblocks += 1;
        b = (*b).next;
        assert!(nblocks < 100_000);
    }
    ArenaSnap {
        remaining: a.remaining,
        block: a.block,
        mode: a.mode,
        nblocks,
        p_in_head: !head_storage.is_null() && p == head_storage.add(a.remaining as usize),
        p_is_head_storage: p == head_storage,
        content: read_cstr(p),
    }
}

#[test]
fn row33_stralloc_first_block_and_oversized() {
    let (c, r) = apis();
    for &len in &[1usize, 2, 10, 100, 510, 511, 512, 513, 1000, 5000] {
        unsafe {
            let mut ac = StringArena::default();
            let mut ar = StringArena::default();
            let mut s: Vec<u8> = vec![b'x'; len];
            s.push(0);
            let sc = stralloc_step(c, &mut ac, &mut s);
            let sr = stralloc_step(r, &mut ar, &mut s);
            assert_same(&format!("stralloc fresh len={len}"), &sc, &sr);
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
            assert_same(
                &format!("arena after reset len={len}"),
                &(ac.remaining, ac.block, ac.mode, ac.storage.is_null()),
                &(ar.remaining, ar.block, ar.mode, ar.storage.is_null()),
            );
        }
    }
}

#[test]
fn row34_stralloc_block_growth_saturation() {
    let (c, r) = apis();
    unsafe {
        let mut ac = StringArena::default();
        let mut ar = StringArena::default();
        // ~4 MiB of 400-byte strings: forces block 0 -> saturation at 1<<20
        for i in 0..12000usize {
            let mut s: Vec<u8> = format!("{:0400}", i).into_bytes();
            s.push(0);
            let sc = stralloc_step(c, &mut ac, &mut s);
            let sr = stralloc_step(r, &mut ar, &mut s);
            if sc != sr {
                assert_same(&format!("stralloc growth #{i}"), &sc, &sr);
            }
        }
        assert!(ac.block >= 11, "block counter did not saturate: {}", ac.block);
        assert_same("arena after growth", &(ac.remaining, ac.block), &(ar.remaining, ar.block));
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
        assert!(ac.storage.is_null() && ar.storage.is_null());
    }
}

#[test]
fn row35_stralloc_interleaved_small_and_huge() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x7777);
    unsafe {
        let mut ac = StringArena::default();
        let mut ar = StringArena::default();
        for i in 0..600usize {
            let len = if i % 7 == 3 {
                600_000 + rng.below(1000)
            } else {
                1 + rng.below(300)
            };
            let mut s: Vec<u8> = vec![b'a' + (i % 26) as u8; len];
            s.push(0);
            let sc = stralloc_step(c, &mut ac, &mut s);
            let sr = stralloc_step(r, &mut ar, &mut s);
            if sc != sr {
                assert_same(&format!("stralloc mixed #{i} len={len}"), &sc, &sr);
            }
        }
        assert_same("arena mixed final", &(ac.remaining, ac.block), &(ar.remaining, ar.block));
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }
}

#[test]
fn row36_strreset_zero_one_many_blocks() {
    let (c, r) = apis();
    for nstrings in [0usize, 1, 5, 50] {
        unsafe {
            let mut ac = StringArena::default();
            let mut ar = StringArena::default();
            for i in 0..nstrings {
                let mut s: Vec<u8> = vec![b'z'; 200 + i];
                s.push(0);
                (c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
                (r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            }
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
            assert_same(
                &format!("strreset n={nstrings}"),
                &(ac.remaining, ac.block, ac.mode, ac.storage.is_null()),
                &(ar.remaining, ar.block, ar.mode, ar.storage.is_null()),
            );
            assert_eq!((ac.remaining, ac.block, ac.mode), (0, 0, 0));
        }
    }
}

// ---------------------------------------------------------------------------
// row 37: strkey
// ---------------------------------------------------------------------------

#[test]
fn row37_strkey() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x8888);
    let mut cases: Vec<c_int> = vec![
        0,
        1,
        9,
        10,
        99,
        100,
        12345,
        -1,
        -9,
        -10,
        -12345,
        c_int::MAX,
        c_int::MIN,
    ];
    for _ in 0..500 {
        cases.push(rng.next_u32() as c_int);
    }
    for n in cases {
        unsafe {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            assert_same(&format!("strkey({n})"), &read_cstr(pc), &read_cstr(pr));
        }
    }
    // repeated calls reuse the same static buffer -> the pointer must be stable
    unsafe {
        let p1 = (c.strkey)(1);
        let p2 = (c.strkey)(2);
        let q1 = (r.strkey)(1);
        let q2 = (r.strkey)(2);
        assert_same("strkey buffer reuse", &(p1 == p2), &(q1 == q2));
        assert_same("strkey after reuse", &read_cstr(p2), &read_cstr(q2));
    }
}

// ---------------------------------------------------------------------------
// row 38: arr_ins
// ---------------------------------------------------------------------------

#[test]
fn row38_arr_ins() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x9999);
    let mut cases: Vec<c_int> = vec![0, 1, 2, 3, 4, 5, -1, c_int::MAX, c_int::MIN];
    for _ in 0..200 {
        cases.push(rng.next_u32() as c_int);
    }
    for n in cases {
        unsafe {
            (c.arr_ins)(n);
            (r.arr_ins)(n);
        }
    }
}
