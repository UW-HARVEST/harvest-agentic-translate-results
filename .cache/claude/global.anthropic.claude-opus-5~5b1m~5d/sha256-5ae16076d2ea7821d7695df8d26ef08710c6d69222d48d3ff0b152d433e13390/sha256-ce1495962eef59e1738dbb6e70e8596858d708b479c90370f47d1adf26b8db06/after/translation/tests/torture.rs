//! High-volume differential torture tests.
//!
//! Phases B and C establish that each `CONFIGS.md` / `ERRORS.md` row matches.
//! This file exists to try much harder to *break* the equivalence: exhaustive
//! length/offset sweeps for the pure hash functions, alignment sweeps,
//! `size_t`-overflow corners of `stbds_arrgrowf`, exhaustive block-boundary
//! sweeps for the string arena, and very long randomized hash-map op sequences
//! cross-checked after every single operation.

mod common;

use common::*;
use core::ffi::{c_char, c_void};
use std::collections::HashSet;

const SEEDS: [usize; 6] = [
    0,
    1,
    0x3141_5926,
    0xdead_beef_dead_beef,
    0x8000_0000_0000_0000,
    usize::MAX,
];

// ---------------------------------------------------------------------------
// stbds_hash_bytes — exhaustive length sweep x byte-position sweep
// ---------------------------------------------------------------------------
#[test]
fn torture_hash_bytes_length_and_position_sweep() {
    let p = libs();
    let mut rng = Rng::new(0xB17E);
    const MAXLEN: usize = 264;
    let mut buf = vec![0u8; MAXLEN + 16];

    for len in 0..=MAXLEN {
        // all-zero and all-0xFF
        for fill in [0x00u8, 0xFF, 0x80, 0x7F] {
            buf[..MAXLEN + 16].fill(fill);
            for &seed in SEEDS.iter() {
                let c = unsafe { (p.c.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed) };
                let r = unsafe { (p.r.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed) };
                diff_eq!(c, r, "fill={fill:#x} len={len} seed={seed:#x}");
            }
        }
        // one 0xFF at every position (exercises the int-promotion sign extension
        // at every byte lane of every 8-byte word and of the tail)
        for pos in 0..len.min(80) {
            buf[..MAXLEN + 16].fill(0x01);
            buf[pos] = 0xFF;
            for &seed in [SEEDS[2], SEEDS[0]].iter() {
                let c = unsafe { (p.c.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed) };
                let r = unsafe { (p.r.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed) };
                diff_eq!(c, r, "ff@{pos} len={len} seed={seed:#x}");
            }
        }
        // random contents
        for _ in 0..12 {
            let b = rng.bytes(MAXLEN + 16);
            buf.copy_from_slice(&b);
            let seed = rng.next_u64() as usize;
            let c = unsafe { (p.c.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed) };
            let r = unsafe { (p.r.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed) };
            diff_eq!(c, r, "rand len={len} seed={seed:#x}");
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_hash_bytes — alignment sweep (the C reads byte-by-byte, so every
// misaligned start must give the same answer as an aligned copy)
// ---------------------------------------------------------------------------
#[test]
fn torture_hash_bytes_alignment() {
    let p = libs();
    let mut rng = Rng::new(0xA716);
    let base = rng.bytes(512);
    for off in 0..64usize {
        for len in [0usize, 1, 3, 7, 8, 9, 15, 16, 17, 31, 33, 64, 100, 255] {
            if off + len > base.len() {
                continue;
            }
            let ptr = unsafe { base.as_ptr().add(off) } as *mut c_void;
            for &seed in SEEDS.iter() {
                let c = unsafe { (p.c.hash_bytes)(ptr, len, seed) };
                let r = unsafe { (p.r.hash_bytes)(ptr, len, seed) };
                diff_eq!(c, r, "off={off} len={len} seed={seed:#x}");
                // and against an aligned copy of the same bytes
                let copy: Vec<u8> = base[off..off + len].to_vec();
                let cp = if len == 0 {
                    std::ptr::null_mut()
                } else {
                    copy.as_ptr() as *mut c_void
                };
                let c2 = unsafe { (p.c.hash_bytes)(cp, len, seed) };
                let r2 = unsafe { (p.r.hash_bytes)(cp, len, seed) };
                diff_eq!(c2, r2, "copy off={off} len={len}");
                assert_eq!(c, c2, "C must not depend on alignment");
                assert_eq!(r, r2, "RUST must not depend on alignment");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_hash_bytes / hash_string — 200k random cases
// ---------------------------------------------------------------------------
#[test]
fn torture_hash_random_bulk() {
    let p = libs();
    let mut rng = Rng::new(0x1234_5678);
    for _ in 0..120_000 {
        let len = rng.below(96);
        let buf = rng.bytes(len.max(1));
        let seed = rng.next_u64() as usize;
        let ptr = if len == 0 {
            std::ptr::null_mut()
        } else {
            buf.as_ptr() as *mut c_void
        };
        let c = unsafe { (p.c.hash_bytes)(ptr, len, seed) };
        let r = unsafe { (p.r.hash_bytes)(ptr, len, seed) };
        diff_eq!(c, r, "bulk hash_bytes len={len} seed={seed:#x}");
    }
    for _ in 0..120_000 {
        let n = rng.below(96);
        let mut s = rng.cstr_body(n, true);
        s.push(0);
        let seed = rng.next_u64() as usize;
        let c = unsafe { (p.c.hash_string)(s.as_ptr() as *mut c_char, seed) };
        let r = unsafe { (p.r.hash_string)(s.as_ptr() as *mut c_char, seed) };
        diff_eq!(c, r, "bulk hash_string len={n} seed={seed:#x}");
    }
}

// ---------------------------------------------------------------------------
// stbds_hash_string — exhaustive 1-2 byte strings and every byte in position k
// ---------------------------------------------------------------------------
#[test]
fn torture_hash_string_exhaustive_short() {
    let p = libs();
    // every 1-byte string
    for b in 1u16..=255 {
        let s = [b as u8, 0];
        for &seed in SEEDS.iter() {
            let c = unsafe { (p.c.hash_string)(s.as_ptr() as *mut c_char, seed) };
            let r = unsafe { (p.r.hash_string)(s.as_ptr() as *mut c_char, seed) };
            diff_eq!(c, r, "1-byte {b:#x} seed={seed:#x}");
        }
    }
    // every 2-byte string
    for a in 1u16..=255 {
        for b in 1u16..=255 {
            let s = [a as u8, b as u8, 0];
            let seed = SEEDS[2];
            let c = unsafe { (p.c.hash_string)(s.as_ptr() as *mut c_char, seed) };
            let r = unsafe { (p.r.hash_string)(s.as_ptr() as *mut c_char, seed) };
            diff_eq!(c, r, "2-byte {a:#x},{b:#x}");
        }
    }
    // one high byte at every position of a 40-byte string
    for pos in 0..40usize {
        for hi in [0x80u8, 0xFF, 0xC3] {
            let mut s = vec![b'k'; 41];
            s[pos] = hi;
            s[40] = 0;
            for &seed in SEEDS.iter() {
                let c = unsafe { (p.c.hash_string)(s.as_ptr() as *mut c_char, seed) };
                let r = unsafe { (p.r.hash_string)(s.as_ptr() as *mut c_char, seed) };
                diff_eq!(c, r, "hi {hi:#x}@{pos} seed={seed:#x}");
            }
        }
    }
    // long runs of the same byte, every length 0..600
    for len in 0..=600usize {
        let mut s = vec![0xFFu8; len + 1];
        s[len] = 0;
        let c = unsafe { (p.c.hash_string)(s.as_ptr() as *mut c_char, SEEDS[5]) };
        let r = unsafe { (p.r.hash_string)(s.as_ptr() as *mut c_char, SEEDS[5]) };
        diff_eq!(c, r, "0xff*{len}");
    }
}

/// Exact model of `stbds_arrgrowf`'s capacity arithmetic (`lib.c:280-301`),
/// including every `size_t` wrap-around.
fn model_cap(len: usize, cap: usize, addlen: usize, min_cap_in: usize) -> usize {
    let mut min_cap = min_cap_in;
    let min_len = len.wrapping_add(addlen);
    if min_len > min_cap {
        min_cap = min_len;
    }
    if min_cap <= cap {
        return cap; // early return, header untouched
    }
    if min_cap < cap.wrapping_mul(2) {
        min_cap = cap.wrapping_mul(2);
    } else if min_cap < 4 {
        min_cap = 4;
    }
    min_cap
}

// ---------------------------------------------------------------------------
// stbds_arrgrowf — the `2 * arrcap(a)` size_t-overflow corner
// ---------------------------------------------------------------------------
#[test]
fn torture_arrgrowf_capacity_overflow() {
    let p = libs();
    // 16 * (k * 2^60 + 30) + 32 wraps to 512 bytes for any k in 1..16, so we can
    // build arrays whose *capacity* field is astronomically large while the real
    // allocation stays 512 bytes.
    for k in 1u32..16 {
        let cap0 = ((k as usize) << 60) + 30;
        let c = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), 16, 0, cap0) };
        let r = unsafe { (p.r.arrgrowf)(std::ptr::null_mut(), 16, 0, cap0) };
        let cs = unsafe { snap_hdr(c) };
        diff_eq!(cs.clone(), unsafe { snap_hdr(r) }, "k={k} initial");
        assert_eq!(cs.capacity, cap0);

        // now ask for more; `2 * cap0` overflows for k >= 8.  `realloc` may move
        // the block, so always follow the returned pointer.
        let mut cc = c;
        let mut rr = r;
        let mut prev_cap = cap0;
        for k2 in (k + 1)..16 {
            let cap1 = ((k2 as usize) << 60) + 30;
            cc = unsafe { (p.c.arrgrowf)(cc, 16, 0, cap1) };
            rr = unsafe { (p.r.arrgrowf)(rr, 16, 0, cap1) };
            diff_eq!(
                unsafe { snap_hdr(cc) },
                unsafe { snap_hdr(rr) },
                "k={k} -> k2={k2} (2*cap wraps to {})",
                cap0.wrapping_mul(2)
            );
            let want = model_cap(0, prev_cap, 0, cap1);
            assert_eq!(
                unsafe { snap_hdr(cc) }.capacity,
                want,
                "k={k} k2={k2}: prev_cap={prev_cap} min_cap={cap1}"
            );
            prev_cap = want;
        }
        // and a request that is <= the current capacity must be a pure no-op
        let c3 = unsafe { (p.c.arrgrowf)(cc, 16, 0, 1) };
        let r3 = unsafe { (p.r.arrgrowf)(rr, 16, 0, 1) };
        assert_eq!(c3, cc, "C: must not realloc");
        assert_eq!(r3, rr, "RUST: must not realloc");
        diff_eq!(unsafe { snap_hdr(c3) }, unsafe { snap_hdr(r3) }, "k={k} noop");
        unsafe {
            (p.c.arrfreef)(cc);
            (p.r.arrfreef)(rr);
        }
    }

    // exhaustive small (addlen, min_cap, starting-cap) cube
    for start in 0..=12usize {
        for addlen in 0..=12usize {
            for min_cap in 0..=12usize {
                let mut ca = Arr::new(&p.c, 4);
                let mut ra = Arr::new(&p.r, 4);
                if start > 0 {
                    ca.grow(0, start);
                    ra.grow(0, start);
                }
                let cap_before = ca.cap();
                let len_before = ca.len() as usize;
                ca.grow(addlen, min_cap);
                ra.grow(addlen, min_cap);
                diff_eq!(
                    unsafe { snap_hdr(ca.a) },
                    unsafe { snap_hdr(ra.a) },
                    "cube start={start} addlen={addlen} min_cap={min_cap}"
                );
                let want = model_cap(len_before, cap_before, addlen, min_cap);
                assert_eq!(
                    ca.cap(),
                    want,
                    "cube model: start={start} addlen={addlen} min_cap={min_cap}"
                );
                ca.free();
                ra.free();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_stralloc — exhaustive length sweep around every block boundary
// ---------------------------------------------------------------------------
fn arena_res(lib: &Lib, a: *mut CStringArena, body: &[u8]) -> (ArenaSnap, Vec<u8>, bool) {
    let mut v = body.to_vec();
    v.push(0);
    unsafe {
        let ptr = (lib.stralloc)(a, v.as_ptr() as *mut c_char);
        let snap = snap_arena(a);
        let in_block = {
            let st = (*a).storage;
            if st.is_null() {
                false
            } else {
                let base = (&raw const (*st).storage) as *const u8;
                ptr as *const u8 == base.add((*a).remaining)
            }
        };
        (snap, read_cstr(ptr), in_block)
    }
}

#[test]
fn torture_stralloc_boundary_sweep() {
    let p = libs();
    // For each first-string length 0..1100, then a second string of every
    // interesting length, compare the full arena state.
    for first in 0..=1100usize {
        let mut ca = CStringArena::zeroed();
        let mut ra = CStringArena::zeroed();
        let c = arena_res(&p.c, &mut ca, &vec![b'a'; first]);
        let r = arena_res(&p.r, &mut ra, &vec![b'a'; first]);
        diff_eq!(c.clone(), r, "first={first}");
        let rem = c.0.remaining;
        // second string exactly at / around the remaining boundary
        for second in [
            0usize,
            1,
            rem.saturating_sub(2),
            rem.saturating_sub(1),
            rem,
            rem + 1,
            rem + 2,
            511,
            512,
            513,
            1023,
            1024,
            1025,
        ] {
            let mut ca2 = ca;
            let mut ra2 = ra;
            let c2 = arena_res(&p.c, &mut ca2, &vec![b'b'; second]);
            let r2 = arena_res(&p.r, &mut ra2, &vec![b'b'; second]);
            diff_eq!(c2.clone(), r2, "first={first} second={second}");
            // put the (possibly grown) chains back so strreset frees everything
            ca = ca2;
            ra = ra2;
        }
        unsafe {
            (p.c.strreset)(&mut ca);
            (p.r.strreset)(&mut ra);
        }
        diff_eq!(
            unsafe { snap_arena(&ca) },
            unsafe { snap_arena(&ra) },
            "first={first} reset"
        );
    }
}

#[test]
fn torture_stralloc_long_sequences() {
    let p = libs();
    let mut rng = Rng::new(0xA2E4);
    for round in 0..12 {
        let mut ca = CStringArena::zeroed();
        let mut ra = CStringArena::zeroed();
        // bias the distribution so both the normal and oversized paths fire a lot
        for i in 0..1500 {
            let n = match rng.below(10) {
                0 => rng.range(500, 700),
                1 => rng.range(1000, 1300),
                2 => 0,
                _ => rng.below(60),
            };
            let full = rng.below(2) == 0;
            let body = rng.cstr_body(n, full);
            let c = arena_res(&p.c, &mut ca, &body);
            let r = arena_res(&p.r, &mut ra, &body);
            diff_eq!(c.clone(), r, "round={round} i={i} n={n}");
            assert_eq!(c.1, body, "arena must return the string verbatim");
        }
        unsafe {
            (p.c.strreset)(&mut ca);
            (p.r.strreset)(&mut ra);
        }
        diff_eq!(
            unsafe { snap_arena(&ca) },
            unsafe { snap_arena(&ra) },
            "round={round} reset"
        );
    }
}

// ---------------------------------------------------------------------------
// Hash map — very long op sequences, cross-checked after EVERY operation
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Shape {
    elemsize: usize,
    keysize: usize,
}

const SHAPES: [Shape; 9] = [
    Shape { elemsize: 1, keysize: 1 },
    Shape { elemsize: 2, keysize: 1 },
    Shape { elemsize: 4, keysize: 4 },
    Shape { elemsize: 8, keysize: 4 },
    Shape { elemsize: 8, keysize: 8 },
    Shape { elemsize: 16, keysize: 8 },
    Shape { elemsize: 20, keysize: 8 },
    Shape { elemsize: 32, keysize: 16 },
    Shape { elemsize: 3, keysize: 2 },
];

#[test]
fn torture_binary_map_long_sequences() {
    let p = libs();
    for (si, sh) in SHAPES.iter().enumerate() {
        for (seedi, &seed) in SEEDS.iter().enumerate() {
            let mut rng = Rng::new((si as u64) * 7919 + seedi as u64 * 104_729);
            let spec = Spec::bytes(sh.elemsize, sh.keysize);
            reset_seed(&p, seed);
            let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
            let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
            // small key pool so collisions/tombstones/rehashes are frequent
            let pool_bits = if sh.keysize == 1 { 6 } else { 9 };
            for step in 0..2500usize {
                let mut k = vec![0u8; sh.keysize];
                let idx = rng.below(1 << pool_bits);
                for (j, b) in k.iter_mut().enumerate() {
                    *b = ((idx >> (j * 3)) & 0xFF) as u8;
                }
                let v = rng.bytes(sh.elemsize - sh.keysize);
                match rng.below(16) {
                    0..=6 => {
                        let a = cm.hmput(&k, &v);
                        let b = rm.hmput(&k, &v);
                        diff_eq!(a, b, "s{si} seed{seedi} step{step} put");
                    }
                    7..=11 => {
                        let a = cm.hmdel(k.as_ptr() as *mut c_void, 0);
                        let b = rm.hmdel(k.as_ptr() as *mut c_void, 0);
                        diff_eq!(a, b, "s{si} seed{seedi} step{step} del");
                    }
                    12..=13 => {
                        let a = cm.hmgeti(&k);
                        let b = rm.hmgeti(&k);
                        diff_eq!(a, b, "s{si} seed{seedi} step{step} get");
                    }
                    14 => {
                        let a = cm.hmgeti_ts(k.as_ptr() as *mut c_void);
                        let b = rm.hmgeti_ts(k.as_ptr() as *mut c_void);
                        diff_eq!(a, b, "s{si} seed{seedi} step{step} get_ts");
                    }
                    _ => {
                        cm.hmdefault(&v);
                        rm.hmdefault(&v);
                    }
                }
                diff_eq!(
                    cm.snap(),
                    rm.snap(),
                    "s{si}(e{} k{}) seed{seedi} step{step}",
                    sh.elemsize,
                    sh.keysize
                );
            }
            cm.hmfree();
            rm.hmfree();
        }
    }
    reset_seed(&p, DEFAULT_SEED);
}

#[test]
fn torture_string_map_long_sequences() {
    let p = libs();
    let sh_modes = [
        (STBDS_SH_DEFAULT, false),
        (STBDS_SH_STRDUP, true),
        (STBDS_SH_ARENA, true),
    ];
    for (mi, &(sh, owned)) in sh_modes.iter().enumerate() {
        for elemsize in [16usize, 24, 32] {
            for (seedi, &seed) in SEEDS.iter().enumerate() {
                let mut rng = Rng::new(
                    (mi as u64) * 31_337 + elemsize as u64 * 977 + seedi as u64 * 7,
                );
                let spec = if owned {
                    Spec::ptr(elemsize)
                } else {
                    Spec::bytes(elemsize, 8)
                };
                // key pool: many prefixes/suffixes so strcmp is stressed
                let mut keys = Keys::new();
                let mut pool: Vec<*mut c_char> = Vec::new();
                let mut seen = HashSet::new();
                while pool.len() < 128 {
                    let n = rng.range(0, 12);
                    let body = rng.cstr_body(n, false);
                    if seen.insert(body.clone()) {
                        pool.push(keys.add(&body));
                    }
                }
                reset_seed(&p, seed);
                let mut cm = Map::new_shmode(&p.c, spec, STBDS_HM_STRING, sh);
                let mut rm = Map::new_shmode(&p.r, spec, STBDS_HM_STRING, sh);
                for step in 0..1500usize {
                    let k = pool[rng.below(pool.len())];
                    let v = rng.bytes(elemsize - 8);
                    match rng.below(16) {
                        0..=5 => {
                            let a = cm.shput(k, &v);
                            let b = rm.shput(k, &v);
                            diff_eq!(a, b, "m{mi} e{elemsize} seed{seedi} step{step} shput");
                        }
                        6..=8 => {
                            // `stbds_shputs` overwrites the whole element and then
                            // restores `.key` from `stbds_temp_key`.  For a
                            // duplicate found in the probe loop's *wrap-around*
                            // half-scan the C never refreshes `temp_key`
                            // (ERRORS.md E20), so `shputs` would store a stale
                            // pointer and corrupt the C's own `hmdel_key`
                            // invariant (`lib.c:849`).  Only use it for keys that
                            // are not already present, which is the sound contract.
                            let present_c = cm.shgeti(k);
                            let present_r = rm.shgeti(k);
                            diff_eq!(present_c, present_r, "m{mi} step{step} shputs probe");
                            if present_c < 0 {
                                let mut whole = vec![0u8; elemsize];
                                whole[8..].copy_from_slice(&v);
                                let a = cm.shputs(k, &whole);
                                let b = rm.shputs(k, &whole);
                                diff_eq!(a, b, "m{mi} e{elemsize} seed{seedi} step{step} shputs");
                            }
                        }
                        9..=12 => {
                            let a = cm.hmdel(k as *mut c_void, 0);
                            let b = rm.hmdel(k as *mut c_void, 0);
                            diff_eq!(a, b, "m{mi} e{elemsize} seed{seedi} step{step} shdel");
                        }
                        13..=14 => {
                            let a = cm.shgeti(k);
                            let b = rm.shgeti(k);
                            diff_eq!(a, b, "m{mi} e{elemsize} seed{seedi} step{step} shgeti");
                        }
                        _ => {
                            let a = cm.hmgeti_ts(k as *mut c_void);
                            let b = rm.hmgeti_ts(k as *mut c_void);
                            diff_eq!(a, b, "m{mi} e{elemsize} seed{seedi} step{step} ts");
                        }
                    }
                    diff_eq!(
                        cm.snap(),
                        rm.snap(),
                        "m{mi}(sh={sh}) e{elemsize} seed{seedi} step{step}"
                    );
                    // NOTE: `stbds_hash_index::temp_key` is deliberately NOT
                    // compared here.  `stbds_make_hash_index` never initialises
                    // it, so after any grow/shrink/tombstone rebuild it holds
                    // uninitialised heap bytes until the next string-mode
                    // insert writes it.  Comparing it across the two libraries
                    // would be comparing two different allocators' garbage.
                    // The well-defined cases (immediately after a put on a map
                    // that has never been rebuilt) are covered by
                    // `hashmap_string::cfg_c52_sh_default_dup_temp_key` and
                    // `errors::err_e20_hmput_dup_wrap`.
                }
                cm.hmfree();
                rm.hmfree();
            }
        }
    }
    reset_seed(&p, DEFAULT_SEED);
}

// ---------------------------------------------------------------------------
// Interleave raw array ops with hash-map ops on the SAME allocation
// ---------------------------------------------------------------------------
#[test]
fn torture_mixed_array_and_map_ops() {
    let p = libs();
    let mut rng = Rng::new(0x4D_1978);
    let spec = Spec::bytes(16, 8);
    for round in 0..10 {
        reset_seed(&p, SEEDS[round % SEEDS.len()]);
        let mut cm = Map::new(&p.c, spec, STBDS_HM_BINARY);
        let mut rm = Map::new(&p.r, spec, STBDS_HM_BINARY);
        for step in 0..600usize {
            match rng.below(8) {
                0 => {
                    // raw arrsetcap on the map's array
                    if !cm.t.is_null() {
                        let n = rng.below(300);
                        unsafe {
                            let ca = (p.c.arrgrowf)(cm.raw(), spec.elemsize, 0, n);
                            let ra = (p.r.arrgrowf)(rm.raw(), spec.elemsize, 0, n);
                            cm.t = (ca as *mut u8).add(spec.elemsize) as *mut c_void;
                            rm.t = (ra as *mut u8).add(spec.elemsize) as *mut c_void;
                        }
                    }
                }
                1..=4 => {
                    let k = (rng.below(400) as u64).to_ne_bytes();
                    let v = rng.bytes(8);
                    let a = cm.hmput(&k, &v);
                    let b = rm.hmput(&k, &v);
                    diff_eq!(a, b, "r{round} s{step} put");
                }
                5..=6 => {
                    let k = (rng.below(400) as u64).to_ne_bytes();
                    let a = cm.hmdel(k.as_ptr() as *mut c_void, 0);
                    let b = rm.hmdel(k.as_ptr() as *mut c_void, 0);
                    diff_eq!(a, b, "r{round} s{step} del");
                }
                _ => {
                    let k = (rng.below(400) as u64).to_ne_bytes();
                    let a = cm.hmgeti(&k);
                    let b = rm.hmgeti(&k);
                    diff_eq!(a, b, "r{round} s{step} get");
                }
            }
            diff_eq!(cm.snap(), rm.snap(), "r{round} s{step}");
        }
        cm.hmfree();
        rm.hmfree();
    }
    reset_seed(&p, DEFAULT_SEED);
}

// ---------------------------------------------------------------------------
// arr_push over a wide range of `num` (exercises arrgrowf's growth curve and
// arrfreef in a tight loop)
// ---------------------------------------------------------------------------
#[test]
fn torture_arr_push_sweep() {
    let p = libs();
    for num in 0..400i32 {
        unsafe {
            (p.c.arr_push)(num);
            (p.r.arr_push)(num);
        }
    }
    for num in [1000i32, 2000, 4000, 8000] {
        unsafe {
            (p.c.arr_push)(num);
            (p.r.arr_push)(num);
        }
    }
}

// ---------------------------------------------------------------------------
// strkey over a dense range, comparing the whole static buffer each time
// ---------------------------------------------------------------------------
#[test]
fn torture_strkey_sweep() {
    let p = libs();
    let mut vals: Vec<i32> = (-2000..2000).collect();
    let mut rng = Rng::new(0x5_3E7A);
    for _ in 0..3000 {
        vals.push(rng.next_u32() as i32);
    }
    for n in vals {
        unsafe {
            let cp = (p.c.strkey)(n);
            let rp = (p.r.strkey)(n);
            let cb = core::slice::from_raw_parts(cp as *const u8, 256).to_vec();
            let rb = core::slice::from_raw_parts(rp as *const u8, 256).to_vec();
            diff_eq!(cb, rb, "strkey({n})");
        }
    }
}
