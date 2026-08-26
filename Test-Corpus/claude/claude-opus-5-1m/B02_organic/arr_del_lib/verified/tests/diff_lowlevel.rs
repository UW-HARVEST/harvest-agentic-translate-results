//! Phase B — differential tests for the LOW-LEVEL entry points:
//! `stbds_hash_bytes`, `stbds_hash_string`, `stbds_rand_seed`,
//! `stbds_arrgrowf`, `stbds_arrfreef`, `stbds_stralloc`, `stbds_strreset`,
//! `stbds_hmput_default`, `stbds_shmode_func`, `strkey`, `arr_del`.
//!
//! Covers CONFIGS.md rows 1–36, 71, 72.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr::null_mut;

// ===========================================================================
// A. stbds_hash_bytes  (CONFIGS rows 1–7)
// ===========================================================================

fn hb(seed_for_rng: u64, lens: &[usize], seeds: &[usize], mangle: Option<(usize, u8)>) {
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(seed_for_rng);
    for &len in lens {
        for _rep in 0..24 {
            let mut buf = rng.bytes(len.max(1) + 8);
            if let Some((idx, or_mask)) = mangle {
                if idx < buf.len() {
                    buf[idx] |= or_mask;
                }
            }
            for &seed in seeds {
                let cv = unsafe { (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
                let rv = unsafe { (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
                assert_eq!(
                    cv, rv,
                    "hash_bytes(len={len}, seed={seed:#x}, buf={:02x?}) C={cv:#x} RUST={rv:#x}",
                    &buf[..len.min(buf.len())]
                );
            }
        }
    }
}

const SEEDS: &[usize] = &[0, 1, 2, usize::MAX, usize::MAX - 1, 0x3141_5926, 0xDEAD_BEEF_CAFE_F00D];

#[test]
fn cfg01_hash_bytes_zero_len() {
    let (_g, c, r) = scenario(0x3141_5926);
    for &seed in SEEDS {
        // p == NULL, len == 0 : never dereferenced
        let cv = unsafe { (c.hash_bytes)(null_mut(), 0, seed) };
        let rv = unsafe { (r.hash_bytes)(null_mut(), 0, seed) };
        assert_eq!(cv, rv, "hash_bytes(NULL,0,{seed:#x})");
        // p valid, len == 0
        let mut b = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let cv = unsafe { (c.hash_bytes)(b.as_mut_ptr() as *mut c_void, 0, seed) };
        let rv = unsafe { (r.hash_bytes)(b.as_mut_ptr() as *mut c_void, 0, seed) };
        assert_eq!(cv, rv, "hash_bytes(buf,0,{seed:#x})");
    }
}

#[test]
fn cfg02_hash_bytes_tail_1_to_7() {
    hb(1, &[1, 2, 3, 4, 5, 6, 7], SEEDS, None);
}

#[test]
fn cfg03_hash_bytes_tail_sign_extension() {
    // `case 4: data |= (d[3] << 24);` is an *int* expression: a byte >= 0x80 at
    // index 3 makes it negative and the widening to size_t sign-extends.
    hb(2, &[4, 5, 6, 7], SEEDS, Some((3, 0x80)));
    // `case 7: data |= ((size_t) d[6] << 24) << 24;`
    hb(3, &[7], SEEDS, Some((6, 0xF0)));
    // all-0xFF tails
    let (_g, c, r) = scenario(0x3141_5926);
    for len in 0..=8usize {
        let mut buf = vec![0xFFu8; 16];
        for &seed in SEEDS {
            let cv = unsafe { (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
            let rv = unsafe { (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
            assert_eq!(cv, rv, "hash_bytes(0xFF*{len},{seed:#x})");
        }
    }
}

#[test]
fn cfg04_hash_bytes_exact_multiples() {
    hb(4, &[8, 16, 24, 32, 64, 128], SEEDS, None);
}

#[test]
fn cfg05_hash_bytes_block_loop_and_tails() {
    let lens: Vec<usize> = (9..=64).collect();
    hb(5, &lens, &[0, 0x3141_5926, usize::MAX], None);
}

#[test]
fn cfg06_hash_bytes_block_sign_extension() {
    // high bit set at index 3 and 7 inside the first 8-byte block
    hb(6, &[8, 9, 15, 16, 17, 23, 24], SEEDS, Some((3, 0x80)));
    hb(7, &[8, 9, 15, 16, 17, 23, 24], SEEDS, Some((7, 0x80)));
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(8);
    for len in 8..=40usize {
        for _ in 0..8 {
            let mut buf = rng.bytes(len + 8);
            for b in buf.iter_mut() {
                *b |= 0x80; // every byte has its high bit set
            }
            for &seed in SEEDS {
                let cv = unsafe { (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
                let rv = unsafe { (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
                assert_eq!(cv, rv, "hash_bytes high-bit len={len} seed={seed:#x}");
            }
        }
    }
}

#[test]
fn cfg07_hash_bytes_seed_matrix() {
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(9);
    for len in [0usize, 1, 7, 8, 15, 32] {
        let mut buf = rng.bytes(len + 8);
        for _ in 0..64 {
            let seed = rng.next_u64() as usize;
            let cv = unsafe { (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
            let rv = unsafe { (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
            assert_eq!(cv, rv, "hash_bytes random seed len={len}");
        }
    }
}

// ===========================================================================
// B. stbds_hash_string / stbds_rand_seed  (CONFIGS rows 8–12)
// ===========================================================================

fn hs(strings: &[Vec<u8>]) {
    let (_g, c, r) = scenario(0x3141_5926);
    for s in strings {
        let mut s = s.clone();
        assert_eq!(*s.last().unwrap(), 0, "test string must be NUL-terminated");
        for &seed in SEEDS {
            let cv = unsafe { (c.hash_string)(s.as_mut_ptr() as *mut c_char, seed) };
            let rv = unsafe { (r.hash_string)(s.as_mut_ptr() as *mut c_char, seed) };
            assert_eq!(cv, rv, "hash_string({:?}, {seed:#x})", &s[..s.len() - 1]);
        }
    }
}

#[test]
fn cfg08_hash_string_empty() {
    hs(&[vec![0u8]]);
}

#[test]
fn cfg09_hash_string_lengths() {
    let mut rng = Rng::new(10);
    let mut v = Vec::new();
    for n in [1usize, 2, 3, 7, 8, 9, 15, 16, 17, 31, 63, 100, 257] {
        for _ in 0..8 {
            v.push(rng.cstring(n));
        }
    }
    hs(&v);
}

#[test]
fn cfg10_hash_string_high_bytes() {
    let mut rng = Rng::new(11);
    let mut v = Vec::new();
    for n in [1usize, 2, 5, 8, 16, 33] {
        for _ in 0..8 {
            let mut s: Vec<u8> = (0..n).map(|_| 0x80 | (rng.next_u8() & 0x7F)).collect();
            s.push(0);
            v.push(s);
        }
    }
    // 0xFF-only and 0x80-only
    v.push(vec![0xFF, 0xFF, 0xFF, 0]);
    v.push(vec![0x80, 0x80, 0]);
    hs(&v);
}

#[test]
fn cfg11_hash_string_random_seeds() {
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(12);
    for _ in 0..300 {
        let n = rng.below(40);
        let mut s = rng.cstring(n);
        let seed = rng.next_u64() as usize;
        let cv = unsafe { (c.hash_string)(s.as_mut_ptr() as *mut c_char, seed) };
        let rv = unsafe { (r.hash_string)(s.as_mut_ptr() as *mut c_char, seed) };
        assert_eq!(cv, rv, "hash_string random");
    }
}

#[test]
fn cfg12_rand_seed_and_lcg_advance() {
    // stbds_rand_seed sets the global seed; every fresh hash index consumes it
    // and advances it with the embedded LCG. Creating several maps in a row
    // exposes the whole sequence.
    for &seed in &[0usize, 1, 2, usize::MAX, 0x3141_5926, 0x1234_5678_9ABC_DEF0] {
        let (_g, c, r) = scenario(seed);
        let mut cd = Vec::new();
        let mut rd = Vec::new();
        let mut cm: Vec<*mut u8> = Vec::new();
        let mut rm: Vec<*mut u8> = Vec::new();
        for i in 0..6 {
            let ct = unsafe { (c.shmode_func)(16, (i % 4) as c_int) } as *mut u8;
            let rt = unsafe { (r.shmode_func)(16, (i % 4) as c_int) } as *mut u8;
            cd.extend_from_slice(&unsafe { dump_map(ct, 16, KeyKind::Raw) });
            rd.extend_from_slice(&unsafe { dump_map(rt, 16, KeyKind::Raw) });
            cm.push(ct);
            rm.push(rt);
        }
        same(&format!("rand_seed({seed:#x}) sequence"), &cd, &rd);
        for (i, t) in cm.iter().enumerate() {
            unsafe { (c.hmfree_func)(t.sub(16) as *mut c_void, 16) };
            unsafe { (r.hmfree_func)(rm[i].sub(16) as *mut c_void, 16) };
        }
    }
}

// ===========================================================================
// C. stbds_arrgrowf / stbds_arrfreef / arr_del  (CONFIGS rows 13–21)
// ===========================================================================

/// Run a schedule of `(addlen, min_cap)` calls, mimicking what `arraddn` does
/// to `length` afterwards, and serialise every intermediate state.
unsafe fn arr_schedule(api: &Api, elemsize: usize, ops: &[(usize, usize)]) -> Vec<u8> {
    let mut p: *mut u8 = null_mut();
    let mut out: Vec<u8> = Vec::new();
    for &(addlen, min_cap) in ops {
        let (before_len, before_cap) = if p.is_null() {
            (0usize, 0usize)
        } else {
            let h = p.sub(HEADER_SIZE) as *const ArrayHeader;
            ((*h).length, (*h).capacity)
        };
        // lib.c:280-287 — when the request already fits, `a` is returned as-is
        // and nothing is (re)allocated. That is the only case in which the
        // returned address is required to be the input address; otherwise the
        // address is up to realloc() and must not be compared.
        let effective = min_cap.max(before_len.wrapping_add(addlen));
        let no_grow = effective <= before_cap;
        let np = (api.arrgrowf)(p as *mut c_void, elemsize, addlen, min_cap) as *mut u8;
        if no_grow {
            assert!(
                np == p,
                "[{}] arrgrowf(elemsize={elemsize}, addlen={addlen}, min_cap={min_cap}) must return its input unchanged when min_cap <= cap ({before_cap})",
                api.name
            );
        }
        out.push(no_grow as u8);
        p = np;
        if p.is_null() {
            out.push(0xEE);
            continue;
        }
        let h = p.sub(HEADER_SIZE) as *mut ArrayHeader;
        // `stbds_arraddn`: length += addlen (never beyond capacity here)
        let newlen = (before_len + addlen).min((*h).capacity);
        (*h).length = newlen;
        for i in before_len..newlen {
            let e = p.add(i * elemsize);
            for j in 0..elemsize {
                *e.add(j) = (i as u8).wrapping_mul(31).wrapping_add((j as u8).wrapping_mul(7)) ^ 0x5A;
            }
        }
        out.extend_from_slice(&dump_array(p, elemsize, KeyKind::Raw));
    }
    if !p.is_null() {
        (api.arrfreef)(p as *mut c_void);
    }
    out
}

fn arr_case(name: &str, elemsize: usize, ops: &[(usize, usize)]) {
    let (_g, c, r) = scenario(0x3141_5926);
    let cd = unsafe { arr_schedule(c, elemsize, ops) };
    let rd = unsafe { arr_schedule(r, elemsize, ops) };
    same(&format!("{name} (elemsize={elemsize}, ops={ops:?})"), &cd, &rd);
}

const ELEMSIZES: &[usize] = &[1, 2, 3, 4, 8, 12, 16, 17, 64];

#[test]
fn cfg13_arrgrowf_min_cap_below_4() {
    for &es in ELEMSIZES {
        for mc in 1..=3usize {
            arr_case("min_cap<4 bumped to 4", es, &[(0, mc)]);
        }
    }
}

#[test]
fn cfg14_arrgrowf_min_cap_used_verbatim() {
    for &es in ELEMSIZES {
        for mc in [4usize, 5, 17, 1000] {
            arr_case("min_cap used as-is", es, &[(0, mc)]);
        }
    }
}

#[test]
fn cfg15_arrgrowf_addlen_drives_min_len() {
    for &es in ELEMSIZES {
        for al in [1usize, 3, 7, 64] {
            arr_case("addlen drives min_len", es, &[(al, 0)]);
        }
    }
}

#[test]
fn cfg16_arrgrowf_no_growth_needed() {
    for &es in ELEMSIZES {
        // grow to 4, then ask for <= 4 several times: must return unchanged
        arr_case("no growth needed", es, &[(0, 4), (0, 0), (0, 1), (0, 4), (0, 3)]);
    }
}

#[test]
fn cfg17_arrgrowf_doubling_wins() {
    for &es in ELEMSIZES {
        // cap becomes 8; then ask for 9..16 -> 2*cap == 16 wins
        arr_case("doubling wins", es, &[(0, 8), (0, 9), (0, 17), (0, 20)]);
    }
}

#[test]
fn cfg18_arrgrowf_min_cap_wins() {
    for &es in ELEMSIZES {
        arr_case("min_cap > 2*cap", es, &[(0, 4), (0, 100), (0, 1000)]);
    }
}

#[test]
fn cfg19_arrgrowf_random_schedules() {
    let mut rng = Rng::new(20);
    for &es in ELEMSIZES {
        for _case in 0..12 {
            let n = 1 + rng.below(10);
            let ops: Vec<(usize, usize)> = (0..n)
                .map(|_| (rng.below(9), rng.below(40)))
                .collect();
            arr_case("random grow schedule", es, &ops);
        }
    }
}

#[test]
fn cfg20_arrgrowf_long_push_sequence() {
    // the `arrput` pattern: repeatedly add one element
    for &es in &[1usize, 4, 16, 17] {
        let ops: Vec<(usize, usize)> = (0..200).map(|_| (1usize, 0usize)).collect();
        arr_case("200 x arrput", es, &ops);
    }
}

#[test]
fn cfg21_arr_del_all_inputs() {
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(21);
    let mut vals: Vec<c_int> = vec![0, 1, -1, 2, 3, 4, i32::MIN, i32::MAX, -2147483647];
    for _ in 0..64 {
        vals.push(rng.next_u32() as c_int);
    }
    for v in vals {
        // arr_del returns void and frees everything it allocates; the
        // differential property is "both complete without crashing or
        // corrupting the heap" -- verified by running them interleaved.
        unsafe { (c.arr_del)(v) };
        unsafe { (r.arr_del)(v) };
    }
}

// ===========================================================================
// D. stbds_stralloc / stbds_strreset  (CONFIGS rows 22–29)
// ===========================================================================

unsafe fn arena_schedule(api: &Api, strings: &[Vec<u8>], forged_block: Option<u8>) -> Vec<u8> {
    let mut a = Arena::zeroed();
    if let Some(b) = forged_block {
        a.block = b;
    }
    let mut out: Vec<u8> = Vec::new();
    for s in strings {
        let mut s = s.clone();
        let p = (api.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
        out.push(stralloc_class(&a, p));
        out.push(1);
        out.extend_from_slice(&cstr_to_vec(p));
        out.push(0);
        out.extend_from_slice(&dump_arena(&a));
    }
    (api.strreset)(&mut a);
    out.extend_from_slice(&dump_arena(&a));
    out
}

fn arena_case(name: &str, strings: &[Vec<u8>], forged_block: Option<u8>) {
    let (_g, c, r) = scenario(0x3141_5926);
    let cd = unsafe { arena_schedule(c, strings, forged_block) };
    let rd = unsafe { arena_schedule(r, strings, forged_block) };
    same(&format!("{name} (block={forged_block:?}, n={})", strings.len()), &cd, &rd);
}

#[test]
fn cfg22_stralloc_first_block() {
    let mut rng = Rng::new(30);
    for n in [0usize, 1, 2, 7, 8, 100, 400, 510, 511] {
        arena_case("first block", &[rng.cstring(n)], None);
    }
}

#[test]
fn cfg23_stralloc_carved_from_same_block() {
    let mut rng = Rng::new(31);
    let strings: Vec<Vec<u8>> = (0..20)
        .map(|_| {
            let n = 1 + rng.below(20);
            rng.cstring(n)
        })
        .collect();
    arena_case("carved from one block", &strings, None);
}

#[test]
fn cfg24_stralloc_oversize_on_empty_arena() {
    let mut rng = Rng::new(32);
    // len > blocksize(512) on a *fresh* arena -> dedicated block becomes head
    for n in [512usize, 513, 1000, 4096] {
        arena_case("oversize, empty arena", &[rng.cstring(n)], None);
    }
}

#[test]
fn cfg25_stralloc_oversize_on_nonempty_arena() {
    let mut rng = Rng::new(33);
    // first a small string (creates a 512 block, block -> 1), then an
    // over-sized one (blocksize is 512 again for block==1) -> spliced behind
    for n in [512usize, 700, 5000] {
        arena_case(
            "oversize, non-empty arena",
            &[rng.cstring(10), rng.cstring(n), rng.cstring(5)],
            None,
        );
    }
}

#[test]
fn cfg26_stralloc_block_growth() {
    let mut rng = Rng::new(34);
    // strings of ~500 bytes force a new block almost every time, walking
    // a->block up through the 512<<(block>>1) progression
    let strings: Vec<Vec<u8>> = (0..40)
        .map(|_| {
            let n = 450 + rng.below(80);
            rng.cstring(n)
        })
        .collect();
    arena_case("block growth", &strings, None);
}

#[test]
fn cfg27_stralloc_forged_block_counter() {
    let mut rng = Rng::new(35);
    // Values chosen so that `512 << (block>>1)` is either small or wraps to 0;
    // values in between would request multi-gigabyte blocks.
    for b in [0u8, 1, 2, 3, 4, 5, 20, 21, 22, 23, 24, 110, 111, 126, 127, 128, 129, 254, 255] {
        arena_case("forged block counter", &[rng.cstring(20)], Some(b));
        arena_case("forged block counter (long)", &[rng.cstring(600)], Some(b));
        arena_case(
            "forged block counter (seq)",
            &[rng.cstring(5), rng.cstring(5), rng.cstring(700)],
            Some(b),
        );
    }
}

#[test]
fn cfg28_stralloc_empty_strings_exhaust_block() {
    // len == 1 each time: exactly 512 fit in the first block, the 513th
    // allocates a new one
    let strings: Vec<Vec<u8>> = (0..1100).map(|_| vec![0u8]).collect();
    arena_case("empty strings", &strings, None);
}

#[test]
fn cfg29_strreset_various_chain_lengths() {
    let mut rng = Rng::new(36);
    // 0 blocks
    arena_case("strreset 0 blocks", &[], None);
    // 1..N blocks, mixed normal + oversized
    for n in 1..=6usize {
        let mut strings = Vec::new();
        for i in 0..n {
            strings.push(rng.cstring(if i % 2 == 0 { 20 } else { 900 }));
        }
        arena_case("strreset mixed chain", &strings, None);
    }
}

// ===========================================================================
// E. stbds_hmput_default / stbds_shmode_func  (CONFIGS rows 31–36)
// ===========================================================================

#[test]
fn cfg31_hmput_default_from_null() {
    let (_g, c, r) = scenario(0x3141_5926);
    for es in [8usize, 12, 16, 20, 32, 1, 7] {
        let ct = unsafe { (c.hmput_default)(null_mut(), es) } as *mut u8;
        let rt = unsafe { (r.hmput_default)(null_mut(), es) } as *mut u8;
        same(
            &format!("hmput_default(NULL,{es})"),
            &unsafe { dump_map(ct, es, KeyKind::Raw) },
            &unsafe { dump_map(rt, es, KeyKind::Raw) },
        );
        unsafe { (c.hmfree_func)(ct.sub(es) as *mut c_void, es) };
        unsafe { (r.hmfree_func)(rt.sub(es) as *mut c_void, es) };
    }
}

#[test]
fn cfg32_hmput_default_idempotent() {
    let (_g, c, r) = scenario(0x3141_5926);
    let es = 16usize;
    let mut ct = unsafe { (c.hmput_default)(null_mut(), es) } as *mut u8;
    let mut rt = unsafe { (r.hmput_default)(null_mut(), es) } as *mut u8;
    for _ in 0..5 {
        ct = unsafe { (c.hmput_default)(ct as *mut c_void, es) } as *mut u8;
        rt = unsafe { (r.hmput_default)(rt as *mut c_void, es) } as *mut u8;
        same(
            "hmput_default idempotent",
            &unsafe { dump_map(ct, es, KeyKind::Raw) },
            &unsafe { dump_map(rt, es, KeyKind::Raw) },
        );
    }
    unsafe { (c.hmfree_func)(ct.sub(es) as *mut c_void, es) };
    unsafe { (r.hmfree_func)(rt.sub(es) as *mut c_void, es) };
}

#[test]
fn cfg33_hmput_default_on_zero_length_array() {
    let (_g, c, r) = scenario(0x3141_5926);
    let es = 16usize;
    // an array made by arrgrowf has length 0 -> hmput_default re-inits it
    let cbase = unsafe { (c.arrgrowf)(null_mut(), es, 0, 4) } as *mut u8;
    let rbase = unsafe { (r.arrgrowf)(null_mut(), es, 0, 4) } as *mut u8;
    let ct = unsafe { (c.hmput_default)(cbase.add(es) as *mut c_void, es) } as *mut u8;
    let rt = unsafe { (r.hmput_default)(rbase.add(es) as *mut c_void, es) } as *mut u8;
    same(
        "hmput_default on length==0",
        &unsafe { dump_map(ct, es, KeyKind::Raw) },
        &unsafe { dump_map(rt, es, KeyKind::Raw) },
    );
    unsafe { (c.hmfree_func)(ct.sub(es) as *mut c_void, es) };
    unsafe { (r.hmfree_func)(rt.sub(es) as *mut c_void, es) };
}

#[test]
fn cfg34_hmdefault_idiom_after_puts() {
    // the `hmdefault` idiom: put keys, then set a default at t[-1], then look
    // up a missing key -> index -1 -> t[-1] is read
    let (_g, c, r) = scenario(0x3141_5926);
    let es = 16usize;
    let mut cm = Map::new(c, es, KeyKind::Raw);
    let mut rm = Map::new(r, es, KeyKind::Raw);
    let mut rng = Rng::new(40);
    for i in 0..10u64 {
        let mut k = (i * 7 + 3).to_le_bytes();
        unsafe { cm.put(k.as_mut_ptr(), 8, STBDS_HM_BINARY, rng.next_u8()) };
        let mut k2 = (i * 7 + 3).to_le_bytes();
        unsafe { rm.put(k2.as_mut_ptr(), 8, STBDS_HM_BINARY, 0) };
    }
    // deterministic fill so the dumps are comparable
    let mut rng = Rng::new(40);
    for i in 0..10i64 {
        let p = rng.next_u8();
        unsafe { cm.fill_value(i as isize, 8, p) };
        unsafe { rm.fill_value(i as isize, 8, p) };
    }
    unsafe {
        cm.put_default();
        rm.put_default();
        // write the "default value" into t[-1]
        for j in 8..es {
            *cm.elem(-1).add(j) = 0xAB;
            *rm.elem(-1).add(j) = 0xAB;
        }
        let mut miss = 0xFFFF_FFFF_u64.to_le_bytes();
        let ci = cm.get(miss.as_mut_ptr(), 8, STBDS_HM_BINARY);
        let mut miss2 = 0xFFFF_FFFF_u64.to_le_bytes();
        let ri = rm.get(miss2.as_mut_ptr(), 8, STBDS_HM_BINARY);
        assert_eq!(ci, ri, "missing key index");
        assert_eq!(ci, -1, "missing key must be -1");
        same("hmdefault idiom", &cm.dump(), &rm.dump());
        cm.free();
        rm.free();
    }
}

#[test]
fn cfg35_shmode_func_valid_modes() {
    for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for es in [8usize, 16, 24, 32] {
            let (_g, c, r) = scenario(0x3141_5926);
            let ct = unsafe { (c.shmode_func)(es, mode) } as *mut u8;
            let rt = unsafe { (r.shmode_func)(es, mode) } as *mut u8;
            same(
                &format!("shmode_func({es},{mode})"),
                &unsafe { dump_map(ct, es, KeyKind::Raw) },
                &unsafe { dump_map(rt, es, KeyKind::Raw) },
            );
            unsafe { (c.hmfree_func)(ct.sub(es) as *mut c_void, es) };
            unsafe { (r.hmfree_func)(rt.sub(es) as *mut c_void, es) };
        }
    }
}

#[test]
fn cfg36_shmode_func_out_of_range_modes() {
    // C enums accept any int; the value is stored as (unsigned char) mode.
    for mode in [4i32, 5, 255, 256, 257, 512, -1, -2, i32::MIN, i32::MAX, 0x1_0001] {
        let (_g, c, r) = scenario(0x3141_5926);
        let es = 16usize;
        let ct = unsafe { (c.shmode_func)(es, mode) } as *mut u8;
        let rt = unsafe { (r.shmode_func)(es, mode) } as *mut u8;
        same(
            &format!("shmode_func(16,{mode})"),
            &unsafe { dump_map(ct, es, KeyKind::Raw) },
            &unsafe { dump_map(rt, es, KeyKind::Raw) },
        );
        // and the stored byte must be the truncation
        unsafe {
            let ht = (*(ct.sub(es).sub(HEADER_SIZE) as *const ArrayHeader)).hash_table
                as *const HashIndex;
            assert_eq!((*ht).string.mode, mode as u8, "truncated mode byte");
        }
        unsafe { (c.hmfree_func)(ct.sub(es) as *mut c_void, es) };
        unsafe { (r.hmfree_func)(rt.sub(es) as *mut c_void, es) };
    }
}

// ===========================================================================
// H. strkey  (CONFIGS row 71)
// ===========================================================================

#[test]
fn cfg71_strkey() {
    // The whole 256-byte static buffer is compared, so the "leftover bytes from
    // a previous longer string" behaviour of sprintf() is covered too. This is
    // the only test that touches `strkey`, so both buffers stay in lock-step.
    let (_g, c, r) = scenario(0x3141_5926);
    let mut rng = Rng::new(71);
    let mut vals: Vec<c_int> = vec![
        0, 1, 2, 9, 10, 11, 99, 100, 101, 999, 1000, -1, -2, -9, -10, -99, -100, 123456789,
        -123456789, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1,
    ];
    for _ in 0..200 {
        vals.push(rng.next_u32() as c_int);
    }
    for v in vals {
        let cp = unsafe { (c.strkey)(v) };
        let rp = unsafe { (r.strkey)(v) };
        let cb = unsafe { std::slice::from_raw_parts(cp as *const u8, 256) };
        let rb = unsafe { std::slice::from_raw_parts(rp as *const u8, 256) };
        assert_eq!(
            cb, rb,
            "strkey({v}): C={:?} RUST={:?}",
            String::from_utf8_lossy(&cb[..32]),
            String::from_utf8_lossy(&rb[..32])
        );
        // sanity: it really is "test_%d"
        let expect = format!("test_{v}");
        let got = unsafe { cstr_to_vec(cp) };
        assert_eq!(String::from_utf8_lossy(&got), expect);
    }
}
