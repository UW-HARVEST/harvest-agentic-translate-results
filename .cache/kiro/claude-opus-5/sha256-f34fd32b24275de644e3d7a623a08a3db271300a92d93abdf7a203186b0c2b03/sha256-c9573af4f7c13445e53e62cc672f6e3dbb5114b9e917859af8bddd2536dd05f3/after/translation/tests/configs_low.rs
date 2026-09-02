//! Phase B — CONFIGS.md rows 1..24 and 66: the lowest-level entry points
//! (`stbds_hash_bytes`, `stbds_hash_string`, `stbds_rand_seed`,
//! `stbds_arrgrowf`, `stbds_arrfreef`, `stbds_stralloc`, `stbds_strreset`).

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void, CString};

// ---------------------------------------------------------------------------
// rows 1..6 — stbds_hash_bytes
// ---------------------------------------------------------------------------

fn hash_bytes_case(c: &Api, r: &Api, buf: &mut [u8], len: usize, seed: usize) {
    let p = buf.as_mut_ptr() as *mut c_void;
    let hc = unsafe { (c.hash_bytes)(p, len, seed) };
    let hr = unsafe { (r.hash_bytes)(p, len, seed) };
    assert_eq!(
        hc, hr,
        "hash_bytes(len={len}, seed={seed:#x}) buf={:02x?}",
        &buf[..len.min(buf.len())]
    );
}

#[test]
fn row01_hash_bytes_len0() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 1);
    let mut buf = [0u8; 8];
    for _ in 0..2000 {
        let seed = rng.next_u64() as usize;
        hash_bytes_case(c, r, &mut buf, 0, seed);
    }
    for seed in [0usize, 1, usize::MAX, 0x31415926] {
        hash_bytes_case(c, r, &mut buf, 0, seed);
    }
}

#[test]
fn row02_hash_bytes_tail_1_to_7() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 2);
    for len in 1..=7usize {
        for _ in 0..3000 {
            let mut b = rng.bytes(len);
            let seed = rng.next_u64() as usize;
            hash_bytes_case(c, r, &mut b, len, seed);
        }
    }
}

#[test]
fn row03_hash_bytes_exact_words() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 3);
    for len in [8usize, 16, 24, 32, 64, 128] {
        for _ in 0..1500 {
            let mut b = rng.bytes(len);
            let seed = rng.next_u64() as usize;
            hash_bytes_case(c, r, &mut b, len, seed);
        }
    }
}

#[test]
fn row04_hash_bytes_words_plus_tail() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 4);
    for k in 1..=8usize {
        for rem in 1..=7usize {
            let len = 8 * k + rem;
            for _ in 0..400 {
                let mut b = rng.bytes(len);
                let seed = rng.next_u64() as usize;
                hash_bytes_case(c, r, &mut b, len, seed);
            }
        }
    }
}

#[test]
fn row05_hash_bytes_sign_extension_corners() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 5);
    for len in 1..=40usize {
        for pat in [0x00u8, 0xff, 0x80, 0x7f, 0x81] {
            let mut b = vec![pat; len];
            for seed in [0usize, 1, usize::MAX, 0x31415926, rng.next_u64() as usize] {
                hash_bytes_case(c, r, &mut b, len, seed);
            }
        }
        // every single byte position set to a high-bit value
        for pos in 0..len {
            let mut b = vec![0u8; len];
            b[pos] = 0xff;
            hash_bytes_case(c, r, &mut b, len, 0x31415926);
            b[pos] = 0x80;
            hash_bytes_case(c, r, &mut b, len, 0);
        }
    }
}

#[test]
fn row06_hash_bytes_large() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 6);
    for len in [256usize, 1024, 4096, 4097] {
        for _ in 0..60 {
            let mut b = rng.bytes(len);
            for seed in [0usize, usize::MAX, rng.next_u64() as usize] {
                hash_bytes_case(c, r, &mut b, len, seed);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 7..10 — stbds_hash_string
// ---------------------------------------------------------------------------

fn hash_string_case(c: &Api, r: &Api, s: &CString, seed: usize) {
    let p = s.as_ptr() as *mut c_char;
    let hc = unsafe { (c.hash_string)(p, seed) };
    let hr = unsafe { (r.hash_string)(p, seed) };
    assert_eq!(hc, hr, "hash_string({s:?}, {seed:#x})");
}

#[test]
fn row07_hash_string_empty() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 7);
    let e = CString::new("").unwrap();
    for _ in 0..3000 {
        hash_string_case(c, r, &e, rng.next_u64() as usize);
    }
    for seed in [0usize, 1, usize::MAX] {
        hash_string_case(c, r, &e, seed);
    }
}

#[test]
fn row08_hash_string_ascii() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 8);
    for len in 1..=64usize {
        for _ in 0..120 {
            let s = rng.ascii(len);
            let seed = rng.next_u64() as usize;
            hash_string_case(c, r, &s, seed);
        }
    }
}

#[test]
fn row09_hash_string_high_bytes() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 9);
    for len in 1..=64usize {
        for _ in 0..120 {
            let s = rng.highbytes(len);
            let seed = rng.next_u64() as usize;
            hash_string_case(c, r, &s, seed);
        }
        // all-0xFF and all-0x80
        for b in [0xffu8, 0x80] {
            let s = CString::new(vec![b; len]).unwrap();
            hash_string_case(c, r, &s, 0x31415926);
            hash_string_case(c, r, &s, 0);
        }
    }
}

#[test]
fn row10_hash_string_long() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 10);
    for len in [65usize, 128, 256, 4096] {
        for _ in 0..80 {
            let s = rng.highbytes(len);
            let seed = rng.next_u64() as usize;
            hash_string_case(c, r, &s, seed);
        }
    }
}

// ---------------------------------------------------------------------------
// row 11 — stbds_rand_seed drives the per-table seed progression
// ---------------------------------------------------------------------------

#[test]
fn row11_rand_seed_progression() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 11);
    let mut seeds: Vec<usize> = vec![0, 1, 0x31415926, usize::MAX, 2];
    for _ in 0..40 {
        seeds.push(rng.next_u64() as usize);
    }
    for s in seeds {
        unsafe {
            (c.rand_seed)(s);
            (r.rand_seed)(s);
            let mut cs = Vec::new();
            let mut rs = Vec::new();
            let mut cm = Vec::new();
            let mut rm = Vec::new();
            for _ in 0..8 {
                let a = (c.shmode_func)(16, SH_ARENA);
                let b = (r.shmode_func)(16, SH_ARENA);
                cm.push(a);
                rm.push(b);
                let ta = arr_table((a as *mut u8).sub(16) as *mut c_void);
                let tb = arr_table((b as *mut u8).sub(16) as *mut c_void);
                cs.push(std::ptr::read_unaligned(ta.add(hi::SEED) as *const usize));
                rs.push(std::ptr::read_unaligned(tb.add(hi::SEED) as *const usize));
            }
            assert_eq!(cs, rs, "table seed progression after rand_seed({s:#x})");
            for a in cm {
                hmfree(c, a, 16);
            }
            for b in rm {
                hmfree(r, b, 16);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 12..17, 66 — stbds_arrgrowf / stbds_arrfreef
// ---------------------------------------------------------------------------

#[test]
fn row12_arrgrowf_null_zero_zero() {
    let (c, r, _g) = both();
    for elemsize in [1usize, 4, 8, 12, 16, 40] {
        unsafe {
            let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(a.is_null(), "C arrgrowf(NULL,{elemsize},0,0) should be NULL");
            assert!(b.is_null(), "R arrgrowf(NULL,{elemsize},0,0) should be NULL");
            same(
                &format!("arrgrowf(NULL,{elemsize},0,0)"),
                &dump_arr(a, 0),
                &dump_arr(b, 0),
            );
        }
    }
}

#[test]
fn row13_arrgrowf_null_small_mincap() {
    let (c, r, _g) = both();
    for elemsize in [1usize, 4, 8, 12, 16, 40] {
        for min_cap in 1..=16usize {
            unsafe {
                let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                let b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                same(
                    &format!("arrgrowf(NULL,{elemsize},0,{min_cap})"),
                    &dump_arr(a, 0),
                    &dump_arr(b, 0),
                );
                (c.arrfreef)(a);
                (r.arrfreef)(b);
            }
        }
    }
}

#[test]
fn row14_arrgrowf_null_addlen() {
    let (c, r, _g) = both();
    for elemsize in [1usize, 4, 8, 16] {
        for addlen in 0..=64usize {
            unsafe {
                let a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, 0);
                let b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, 0);
                same(
                    &format!("arrgrowf(NULL,{elemsize},{addlen},0)"),
                    &dump_arr(a, 0),
                    &dump_arr(b, 0),
                );
                if !a.is_null() {
                    (c.arrfreef)(a);
                }
                if !b.is_null() {
                    (r.arrfreef)(b);
                }
            }
        }
    }
}

#[test]
fn row15_arrgrowf_noop_path() {
    let (c, r, _g) = both();
    unsafe {
        for elemsize in [4usize, 8, 16] {
            let mut a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let mut b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let ca = arr_capacity(a);
            assert_eq!(ca, arr_capacity(b));
            for min_cap in 0..=ca {
                let a2 = (c.arrgrowf)(a, elemsize, 0, min_cap);
                let b2 = (r.arrgrowf)(b, elemsize, 0, min_cap);
                assert_eq!(a2, a, "C no-op path must return the same pointer");
                assert_eq!(b2, b, "R no-op path must return the same pointer");
                a = a2;
                b = b2;
                same(
                    &format!("arrgrowf noop elemsize={elemsize} min_cap={min_cap}"),
                    &dump_arr(a, 0),
                    &dump_arr(b, 0),
                );
            }
            (c.arrfreef)(a);
            (r.arrfreef)(b);
        }
    }
}

#[test]
fn row16_arrgrowf_doubling_sequence() {
    let (c, r, _g) = both();
    unsafe {
        for elemsize in [1usize, 4, 8, 12, 16, 40] {
            let mut a: *mut c_void = std::ptr::null_mut();
            let mut b: *mut c_void = std::ptr::null_mut();
            let mut prev_cap = 0usize;
            for step in 0..200usize {
                // emulate `stbds_arrmaybegrow(a,1)` + `length++`
                let need = arr_length_or0(a) + 1;
                if a.is_null() || need > arr_capacity_or0(a) {
                    a = (c.arrgrowf)(a, elemsize, 1, 0);
                    b = (r.arrgrowf)(b, elemsize, 1, 0);
                }
                let cap = arr_capacity(a);
                assert_eq!(cap, arr_capacity(b), "capacity elemsize={elemsize}");
                // fill the newly available region with a position-derived pattern
                for byte in prev_cap * elemsize..cap * elemsize {
                    let v = ((byte * 31 + 7) & 0xff) as u8;
                    *(a as *mut u8).add(byte) = v;
                    *(b as *mut u8).add(byte) = v;
                }
                prev_cap = cap;
                *((a as *mut u8).sub(HDR_SIZE) as *mut usize) = need;
                *((b as *mut u8).sub(HDR_SIZE) as *mut usize) = need;
                same(
                    &format!("arrgrowf push elemsize={elemsize} step={step}"),
                    &dump_arr(a, cap * elemsize),
                    &dump_arr(b, cap * elemsize),
                );
            }
            (c.arrfreef)(a);
            (r.arrfreef)(b);
        }
    }
}

unsafe fn arr_length_or0(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        arr_length(a)
    }
}
unsafe fn arr_capacity_or0(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        arr_capacity(a)
    }
}

#[test]
fn row17_arrgrowf_exact_size_path() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 17);
    unsafe {
        for elemsize in [1usize, 8, 16] {
            for _ in 0..80 {
                let mut a = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
                let mut b = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
                for _ in 0..6 {
                    let cap = arr_capacity(a);
                    let min_cap = cap * 2 + 1 + rng.below(500);
                    a = (c.arrgrowf)(a, elemsize, 0, min_cap);
                    b = (r.arrgrowf)(b, elemsize, 0, min_cap);
                    same(
                        &format!("arrgrowf exact elemsize={elemsize} min_cap={min_cap}"),
                        &dump_arr(a, 0),
                        &dump_arr(b, 0),
                    );
                    let addlen = rng.below(400);
                    a = (c.arrgrowf)(a, elemsize, addlen, 0);
                    b = (r.arrgrowf)(b, elemsize, addlen, 0);
                    same(
                        &format!("arrgrowf addlen elemsize={elemsize} addlen={addlen}"),
                        &dump_arr(a, 0),
                        &dump_arr(b, 0),
                    );
                }
                (c.arrfreef)(a);
                (r.arrfreef)(b);
            }
        }
    }
}

#[test]
fn row66_arrgrowf_arrfreef_roundtrip() {
    let (c, r, _g) = both();
    unsafe {
        for _ in 0..50 {
            let mut a = (c.arrgrowf)(std::ptr::null_mut(), 16, 200, 0);
            let mut b = (r.arrgrowf)(std::ptr::null_mut(), 16, 200, 0);
            same("arrgrowf 200", &dump_arr(a, 0), &dump_arr(b, 0));
            a = (c.arrgrowf)(a, 16, 0, 1000);
            b = (r.arrgrowf)(b, 16, 0, 1000);
            same("arrgrowf 1000", &dump_arr(a, 0), &dump_arr(b, 0));
            (c.arrfreef)(a);
            (r.arrfreef)(b);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 18..24 — stbds_stralloc / stbds_strreset
// ---------------------------------------------------------------------------

/// Runs the same `stralloc` sequence on both libraries and compares the arena
/// state plus the string actually stored, after every call.
fn arena_sequence(c: &Api, r: &Api, what: &str, strings: &[CString]) {
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        for (i, s) in strings.iter().enumerate() {
            let pc = (c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
            assert!(!pc.is_null() && !pr.is_null());
            let sc = std::ffi::CStr::from_ptr(pc).to_bytes().to_vec();
            let sr = std::ffi::CStr::from_ptr(pr).to_bytes().to_vec();
            same(&format!("{what}: stored string #{i}"), &sc, &sr);
            assert_eq!(sc, s.as_bytes(), "{what}: stralloc #{i} lost the content");
            same(
                &format!("{what}: arena state after #{i}"),
                &dump_arena(&ca),
                &dump_arena(&ra),
            );
        }
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
        same(
            &format!("{what}: arena after strreset"),
            &dump_arena(&ca),
            &dump_arena(&ra),
        );
    }
}

#[test]
fn row18_stralloc_block_boundaries() {
    let (c, r, _g) = both();
    for len in [1usize, 2, 3, 510, 511, 512, 513, 514, 1023, 1024, 1025] {
        let s = CString::new(vec![b'x'; len - 1]).unwrap();
        arena_sequence(c, r, &format!("row18 len={len}"), &[s]);
    }
}

#[test]
fn row19_stralloc_many_random() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 19);
    for round in 0..20 {
        let n = 300;
        let strings: Vec<CString> = (0..n).map(|_| rng.ascii_len(1, 100)).collect();
        arena_sequence(c, r, &format!("row19 round={round}"), &strings);
    }
}

#[test]
fn row20_stralloc_first_oversized() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 20);
    for big in [513usize, 1000, 5000, 100_000] {
        let mut strings = vec![CString::new(vec![b'B'; big]).unwrap()];
        for _ in 0..30 {
            strings.push(rng.ascii_len(1, 80));
        }
        arena_sequence(c, r, &format!("row20 big={big}"), &strings);
    }
}

#[test]
fn row21_stralloc_oversized_after_head() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 21);
    for big in [600usize, 2000, 50_000] {
        let mut strings = vec![rng.ascii(10)]; // creates the 512-byte head block
        strings.push(CString::new(vec![b'C'; big]).unwrap());
        for _ in 0..40 {
            strings.push(rng.ascii_len(1, 60));
        }
        strings.push(CString::new(vec![b'D'; big]).unwrap());
        for _ in 0..40 {
            strings.push(rng.ascii_len(1, 60));
        }
        arena_sequence(c, r, &format!("row21 big={big}"), &strings);
    }
}

#[test]
fn row22_stralloc_block_saturation() {
    let (c, r, _g) = both();
    // Each string is sized just above the current block's capacity so that a
    // fresh block is allocated every time, driving `a->block` 0 -> 22.
    let mut strings = Vec::new();
    for b in 0..30u32 {
        let blocksize = 512usize << (b >> 1).min(11);
        strings.push(CString::new(vec![b'e'; blocksize.min(1 << 20)]).unwrap());
        strings.push(CString::new(vec![b'f'; 8]).unwrap());
    }
    arena_sequence(c, r, "row22 saturation", &strings);
}

#[test]
fn row22b_stralloc_block_field_sweep() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 122);
    // `a->block` only ever reaches 22 through the library itself (it stops
    // incrementing once 512 << (block>>1) reaches 1 MiB).  Sweep the whole
    // reachable range explicitly, with and without an existing head block.
    unsafe {
        for block in 0..=22u8 {
            for mode in [0u8, 1, 3, 255] {
                for len in [1usize, 8, 600, 5000] {
                    let s = CString::new(vec![b'q'; len]).unwrap();

                    let mut ca = StringArena::zeroed();
                    let mut ra = StringArena::zeroed();
                    ca.block = block;
                    ra.block = block;
                    ca.mode = mode;
                    ra.mode = mode;

                    let pc = (c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
                    let pr = (r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
                    same(
                        &format!("row22b b={block} m={mode} len={len} content"),
                        std::ffi::CStr::from_ptr(pc).to_bytes(),
                        std::ffi::CStr::from_ptr(pr).to_bytes(),
                    );
                    same(
                        &format!("row22b b={block} m={mode} len={len} arena"),
                        &dump_arena(&ca),
                        &dump_arena(&ra),
                    );
                    // a couple of follow-up allocations from the same arena
                    for k in 0..5 {
                        let s2 = rng.ascii_len(1, 200);
                        let pc2 = (c.stralloc)(&mut ca, s2.as_ptr() as *mut c_char);
                        let pr2 = (r.stralloc)(&mut ra, s2.as_ptr() as *mut c_char);
                        same(
                            &format!("row22b b={block} len={len} follow{k} content"),
                            std::ffi::CStr::from_ptr(pc2).to_bytes(),
                            std::ffi::CStr::from_ptr(pr2).to_bytes(),
                        );
                        same(
                            &format!("row22b b={block} len={len} follow{k} arena"),
                            &dump_arena(&ca),
                            &dump_arena(&ra),
                        );
                    }
                    (c.strreset)(&mut ca);
                    (r.strreset)(&mut ra);
                    same(
                        &format!("row22b b={block} len={len} reset"),
                        &dump_arena(&ca),
                        &dump_arena(&ra),
                    );
                }
            }
        }
    }
}

#[test]
fn row23_stralloc_reset_reuse() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 23);
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        for round in 0..30 {
            let strings: Vec<CString> = (0..80).map(|_| rng.ascii_len(1, 700)).collect();
            for (i, s) in strings.iter().enumerate() {
                let pc = (c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
                let pr = (r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
                let sc = std::ffi::CStr::from_ptr(pc).to_bytes().to_vec();
                let sr = std::ffi::CStr::from_ptr(pr).to_bytes().to_vec();
                same(&format!("row23 r{round} s{i}"), &sc, &sr);
                same(
                    &format!("row23 arena r{round} s{i}"),
                    &dump_arena(&ca),
                    &dump_arena(&ra),
                );
            }
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
            same(
                &format!("row23 arena reset r{round}"),
                &dump_arena(&ca),
                &dump_arena(&ra),
            );
        }
    }
}

#[test]
fn row24_strreset_idempotent() {
    let (c, r, _g) = both();
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        for i in 0..10 {
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
            same(
                &format!("row24 reset #{i}"),
                &dump_arena(&ca),
                &dump_arena(&ra),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// row 60 — strkey
// ---------------------------------------------------------------------------

#[test]
fn row60_strkey() {
    let (c, r, _g) = both();
    let mut rng = Rng::new(SEED ^ 60);
    let mut vals: Vec<c_int> = vec![0, 1, 9, 10, 99, 100, 12345, -1, -12345, i32::MIN, i32::MAX];
    for _ in 0..200 {
        vals.push(rng.next_u32() as i32);
    }
    unsafe {
        for n in vals {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            let sc = std::ffi::CStr::from_ptr(pc).to_bytes().to_vec();
            let sr = std::ffi::CStr::from_ptr(pr).to_bytes().to_vec();
            same(&format!("strkey({n})"), &sc, &sr);
        }
    }
}
