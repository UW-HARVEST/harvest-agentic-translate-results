//! Phase B — valid-path differential tests for the low-level entry points.
//!
//! Covers CONFIGS.md rows C1–C20 and C54–C63.

mod harness;

use harness::*;
use std::ffi::{c_char, c_int, c_void};

// ===========================================================================
// C1–C6: stbds_hash_bytes
// ===========================================================================

fn hash_bytes_case(b: &Both, buf: &mut [u8], len: usize, seed: usize, ctx: &str) {
    let p = if buf.is_empty() {
        std::ptr::null_mut()
    } else {
        buf.as_mut_ptr() as *mut c_void
    };
    unsafe {
        let c = (b.c.hash_bytes)(p, len, seed);
        let r = (b.r.hash_bytes)(p, len, seed);
        diff_eq!(ctx, c, r);
    }
}

const SEEDS: [usize; 5] = [0, 1, 0x3141_5926, usize::MAX, 0x9E37_79B9_7F4A_7C15];

#[test]
fn c1_hash_bytes_len_zero() {
    let b = libs();
    // NULL pointer, len 0 — the C reads nothing.
    for &s in &SEEDS {
        unsafe {
            let c = (b.c.hash_bytes)(std::ptr::null_mut(), 0, s);
            let r = (b.r.hash_bytes)(std::ptr::null_mut(), 0, s);
            diff_eq!(format!("C1 null len=0 seed={s:#x}"), c, r);
        }
    }
    // Valid pointer, len 0.
    let mut rng = Rng::new(TEST_SEED);
    for _ in 0..64 {
        let mut buf = rng.bytes(16);
        for &s in &SEEDS {
            hash_bytes_case(&b, &mut buf, 0, s, "C1 valid ptr len=0");
        }
    }
}

#[test]
fn c2_hash_bytes_tail_only() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 2);
    for len in 1..8usize {
        for iter in 0..400 {
            let mut buf = rng.bytes(len.max(1));
            for &s in &SEEDS {
                hash_bytes_case(
                    &b,
                    &mut buf,
                    len,
                    s,
                    &format!("C2 len={len} iter={iter} seed={s:#x}"),
                );
            }
        }
    }
}

#[test]
fn c3_hash_bytes_whole_blocks() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 3);
    for len in [8usize, 16, 24, 32, 64, 128] {
        for iter in 0..300 {
            let mut buf = rng.bytes(len);
            for &s in &SEEDS {
                hash_bytes_case(&b, &mut buf, len, s, &format!("C3 len={len} iter={iter}"));
            }
        }
    }
}

#[test]
fn c4_hash_bytes_blocks_plus_tail() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 4);
    for len in 9..32usize {
        for iter in 0..200 {
            let mut buf = rng.bytes(len);
            for &s in &SEEDS {
                hash_bytes_case(&b, &mut buf, len, s, &format!("C4 len={len} iter={iter}"));
            }
        }
    }
}

#[test]
fn c5_hash_bytes_large() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 5);
    for _ in 0..300 {
        let len = 64 + rng.below(4096 - 64);
        let mut buf = rng.bytes(len);
        for &s in &SEEDS {
            hash_bytes_case(&b, &mut buf, len, s, &format!("C5 len={len}"));
        }
    }
}

#[test]
fn c6_hash_bytes_extreme_patterns() {
    let b = libs();
    for len in 0..40usize {
        for pat in [0x00u8, 0xFF, 0x80, 0x7F, 0x01] {
            let mut buf = vec![pat; len.max(1)];
            for &s in &SEEDS {
                hash_bytes_case(
                    &b,
                    &mut buf,
                    len,
                    s,
                    &format!("C6 len={len} pat={pat:#x} seed={s:#x}"),
                );
            }
        }
        // A buffer where only the sign bit of byte 3 / 7 is set (exercises the
        // sign-extending `d[3] << 24` int shift).
        for hot in 0..len.min(16) {
            let mut buf = vec![0u8; len.max(1)];
            buf[hot] = 0x80;
            for &s in &SEEDS {
                hash_bytes_case(
                    &b,
                    &mut buf,
                    len,
                    s,
                    &format!("C6 len={len} hot={hot} seed={s:#x}"),
                );
            }
        }
    }
}

// ===========================================================================
// C7–C9: stbds_hash_string
// ===========================================================================

fn hash_string_case(b: &Both, s: &mut Vec<u8>, seed: usize, ctx: &str) {
    unsafe {
        let p = s.as_mut_ptr() as *mut c_char;
        let c = (b.c.hash_string)(p, seed);
        let r = (b.r.hash_string)(p, seed);
        diff_eq!(ctx, c, r);
    }
}

#[test]
fn c7_hash_string_ascii() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 7);
    for fixed in [&b""[..], &b"a"[..], &b"ab"[..], &b"test_0"[..]] {
        let mut s = fixed.to_vec();
        s.push(0);
        for &sd in &SEEDS {
            hash_string_case(&b, &mut s, sd, &format!("C7 fixed={fixed:?} seed={sd:#x}"));
        }
    }
    for len in 0..40usize {
        for _ in 0..60 {
            let mut s = rng.ascii(len);
            for &sd in &SEEDS {
                hash_string_case(&b, &mut s, sd, &format!("C7 len={len}"));
            }
        }
    }
}

#[test]
fn c8_hash_string_high_bit() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 8);
    for len in 1..32usize {
        for _ in 0..80 {
            let mut s = rng.latin1(len);
            for &sd in &SEEDS {
                hash_string_case(&b, &mut s, sd, &format!("C8 len={len}"));
            }
        }
        // All bytes 0x80..0xFF only.
        let mut s: Vec<u8> = (0..len).map(|i| 0x80 + (i as u8 % 0x80)).collect();
        s.push(0);
        for &sd in &SEEDS {
            hash_string_case(&b, &mut s, sd, &format!("C8 highonly len={len}"));
        }
    }
}

#[test]
fn c9_hash_string_long() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 9);
    for _ in 0..200 {
        let len = 128 + rng.below(1024 - 128);
        let mut s = rng.latin1(len);
        for &sd in &SEEDS {
            hash_string_case(&b, &mut s, sd, &format!("C9 len={len}"));
        }
    }
}

// ===========================================================================
// C10: stbds_rand_seed + the global seed advance in stbds_make_hash_index
// ===========================================================================

#[test]
fn c10_rand_seed_and_global_advance() {
    let b = libs();
    for &sd in &SEEDS {
        reset_seed(&b, sd);
        // Eight consecutive fresh tables: each takes the current global seed and
        // then advances it via `seed = seed * a + b`.
        for i in 0..8 {
            unsafe {
                let cm = (b.c.shmode_func)(16, STBDS_SH_STRDUP);
                let rm = (b.r.shmode_func)(16, STBDS_SH_STRDUP);
                let cs = snap_map(cm, 16, KeyRepr::Ptr, 8, 8);
                let rs = snap_map(rm, 16, KeyRepr::Ptr, 8, 8);
                diff_eq!(format!("C10 seed={sd:#x} table#{i}"), cs, rs);
                (b.c.hmfree_func)((cm as *mut u8).wrapping_sub(16) as *mut c_void, 16);
                (b.r.hmfree_func)((rm as *mut u8).wrapping_sub(16) as *mut c_void, 16);
            }
        }
    }
    reset_seed(&b, 0x3141_5926);
}

// ===========================================================================
// C11–C17: stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

unsafe fn free_arr(api: &Api, a: *mut c_void) {
    if (a as usize) >= 4096 {
        unsafe { (api.arrfreef)(a) };
    }
}

#[test]
fn c11_arrgrowf_degenerate_zero() {
    let b = libs();
    for &elemsize in &[0usize, 1, 4, 8] {
        unsafe {
            let c = (b.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let r = (b.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            diff_eq!(
                format!("C11 elemsize={elemsize} nullness"),
                c.is_null(),
                r.is_null()
            );
            diff_eq!(
                format!("C11 elemsize={elemsize} value"),
                c as usize,
                r as usize
            );
            free_arr(&b.c, c);
            free_arr(&b.r, r);
        }
    }
}

#[test]
fn c12_arrgrowf_from_null_cross_product() {
    let b = libs();
    for elemsize in [0usize, 1, 3, 4, 8, 12, 16] {
        for addlen in 0..9usize {
            for min_cap in 0..9usize {
                unsafe {
                    let c = (b.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let r = (b.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let ctx = format!("C12 es={elemsize} addlen={addlen} min_cap={min_cap}");
                    diff_eq!(&ctx, snap_arr(c), snap_arr(r));
                    free_arr(&b.c, c);
                    free_arr(&b.r, r);
                }
            }
        }
    }
}

#[test]
fn c13_c14_c15_arrgrowf_existing() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 13);
    for elemsize in [1usize, 4, 8, 12, 16] {
        for _ in 0..200 {
            unsafe {
                // Establish an array with a known capacity.
                let init_cap = 1 + rng.below(64);
                let mut c = (b.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, init_cap);
                let mut r = (b.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, init_cap);
                diff_eq!("C13 setup", snap_arr(c), snap_arr(r));

                // Give it a nonzero length so `min_len` is interesting.
                let len = rng.below(init_cap);
                (*((c as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader)).length = len;
                (*((r as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader)).length = len;

                let cap = (*((c as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader)).capacity;

                // C13 early-out, C14 doubling, C15 exact min_cap.
                for (tag, addlen, min_cap) in [
                    ("C13", 0usize, 0usize),
                    ("C13", 0, cap),
                    ("C13", 0, cap.saturating_sub(1)),
                    ("C14", 1, cap + 1),
                    ("C14", 0, cap + 1),
                    ("C14", 0, 2 * cap - 1),
                    ("C15", 0, 2 * cap),
                    ("C15", 0, 4 * cap + 7),
                    ("C15", 3 * cap, 0),
                ] {
                    let cc = (b.c.arrgrowf)(c, elemsize, addlen, min_cap);
                    let rr = (b.r.arrgrowf)(r, elemsize, addlen, min_cap);
                    let ctx =
                        format!("{tag} es={elemsize} cap={cap} len={len} addlen={addlen} min_cap={min_cap}");
                    diff_eq!(&ctx, snap_arr(cc), snap_arr(rr));
                    // NOTE: whether `realloc` returned a *different* address is
                    // an allocator artifact, not library semantics, so it is
                    // deliberately not compared here.
                    c = cc;
                    r = rr;
                }
                free_arr(&b.c, c);
                free_arr(&b.r, r);
            }
        }
    }
}

#[test]
fn c16_arrgrowf_growth_chain() {
    let b = libs();
    for elemsize in [1usize, 4, 8, 16] {
        unsafe {
            let mut c: *mut c_void = std::ptr::null_mut();
            let mut r: *mut c_void = std::ptr::null_mut();
            for step in 0..40 {
                c = (b.c.arrgrowf)(c, elemsize, 1, 0);
                r = (b.r.arrgrowf)(r, elemsize, 1, 0);
                diff_eq!(
                    format!("C16 es={elemsize} step={step}"),
                    snap_arr(c),
                    snap_arr(r)
                );
                // Simulate arrput: bump the length so the next grow is required.
                (*((c as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader)).length += 1;
                (*((r as *mut u8).wrapping_sub(HEADER_SIZE) as *mut ArrHeader)).length += 1;
            }
            free_arr(&b.c, c);
            free_arr(&b.r, r);
        }
    }
}

#[test]
fn c17_arrgrowf_then_arrfreef() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 17);
    for _ in 0..200 {
        let elemsize = 1 + rng.below(32);
        let min_cap = 1 + rng.below(256);
        unsafe {
            let c = (b.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
            let r = (b.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
            diff_eq!("C17", snap_arr(c), snap_arr(r));
            (b.c.arrfreef)(c);
            (b.r.arrfreef)(r);
        }
    }
}

// ===========================================================================
// C18–C20: stbds_hmput_default
// ===========================================================================

#[test]
fn c18_hmput_default_from_null() {
    let b = libs();
    for elemsize in [1usize, 4, 8, 12, 16, 24, 32] {
        unsafe {
            let c = (b.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let r = (b.r.hmput_default)(std::ptr::null_mut(), elemsize);
            diff_eq!(
                format!("C18 es={elemsize}"),
                snap_map(c, elemsize, KeyRepr::Bytes, elemsize, 0),
                snap_map(r, elemsize, KeyRepr::Bytes, elemsize, 0)
            );
            (b.c.hmfree_func)((c as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
            (b.r.hmfree_func)((r as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn c19_hmput_default_twice() {
    let b = libs();
    for elemsize in [1usize, 4, 8, 16, 24] {
        unsafe {
            let mut c = (b.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let mut r = (b.r.hmput_default)(std::ptr::null_mut(), elemsize);
            // Write a recognisable default value so we can see whether the
            // second call re-zeroes it (the C does not).
            for k in 0..elemsize {
                *(c as *mut u8).wrapping_sub(elemsize).wrapping_add(k) = 0xAB;
                *(r as *mut u8).wrapping_sub(elemsize).wrapping_add(k) = 0xAB;
            }
            for round in 0..4 {
                c = (b.c.hmput_default)(c, elemsize);
                r = (b.r.hmput_default)(r, elemsize);
                diff_eq!(
                    format!("C19 es={elemsize} round={round}"),
                    snap_map(c, elemsize, KeyRepr::Bytes, elemsize, 0),
                    snap_map(r, elemsize, KeyRepr::Bytes, elemsize, 0)
                );
            }
            (b.c.hmfree_func)((c as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
            (b.r.hmfree_func)((r as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn c20_hmput_default_after_length_reset() {
    let b = libs();
    for elemsize in [4usize, 8, 16] {
        unsafe {
            let mut c = (b.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let mut r = (b.r.hmput_default)(std::ptr::null_mut(), elemsize);
            // Force the `length == 0` re-grow branch.
            let ch = (c as *mut u8)
                .wrapping_sub(elemsize)
                .wrapping_sub(HEADER_SIZE) as *mut ArrHeader;
            let rh = (r as *mut u8)
                .wrapping_sub(elemsize)
                .wrapping_sub(HEADER_SIZE) as *mut ArrHeader;
            (*ch).length = 0;
            (*rh).length = 0;
            c = (b.c.hmput_default)(c, elemsize);
            r = (b.r.hmput_default)(r, elemsize);
            diff_eq!(
                format!("C20 es={elemsize}"),
                snap_map(c, elemsize, KeyRepr::Bytes, elemsize, 0),
                snap_map(r, elemsize, KeyRepr::Bytes, elemsize, 0)
            );
            (b.c.hmfree_func)((c as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
            (b.r.hmfree_func)((r as *mut u8).wrapping_sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ===========================================================================
// C54–C61: stbds_stralloc / stbds_strreset
// ===========================================================================

/// Where a `stbds_stralloc` return value sits, expressed structurally so the
/// two libraries' distinct heap addresses stay comparable.
#[derive(PartialEq, Eq, Debug)]
enum RetLoc {
    /// `a->storage->storage + a->remaining` (the normal in-block allocation)
    HeadPlusRemaining,
    /// `a->storage->storage` (oversize block became the new head)
    HeadStorage,
    /// `a->storage->next->storage` (oversize block spliced after the head)
    SecondStorage,
    Other,
}

unsafe fn classify(a: &StringArena, ret: *mut c_char) -> RetLoc {
    unsafe {
        if a.storage.is_null() {
            return RetLoc::Other;
        }
        let head = std::ptr::addr_of_mut!((*a.storage).storage) as *mut c_char;
        if ret == head.wrapping_add(a.remaining) {
            return RetLoc::HeadPlusRemaining;
        }
        if ret == head {
            return RetLoc::HeadStorage;
        }
        let nxt = (*a.storage).next;
        if !nxt.is_null() && ret == std::ptr::addr_of_mut!((*nxt).storage) as *mut c_char {
            return RetLoc::SecondStorage;
        }
        RetLoc::Other
    }
}

#[derive(PartialEq, Eq, Debug)]
struct StrallocObs {
    content: Vec<u8>,
    loc: RetLoc,
    remaining: usize,
    block: u8,
    mode: u8,
    chain_len: usize,
}

unsafe fn stralloc_obs(api: &Api, a: *mut StringArena, s: &mut Vec<u8>) -> StrallocObs {
    unsafe {
        let ret = (api.stralloc)(a, s.as_mut_ptr() as *mut c_char);
        let (remaining, block, mode, chain_len) = snap_arena(&*a);
        StrallocObs {
            content: read_cstr(ret),
            loc: classify(&*a, ret),
            remaining,
            block,
            mode,
            chain_len,
        }
    }
}

#[test]
fn c54_stralloc_fresh_arena_varied_len() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 54);
    for len in (0..600usize).step_by(1) {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        let mut s = rng.ascii(len);
        unsafe {
            let co = stralloc_obs(&b.c, &mut ca, &mut s);
            let ro = stralloc_obs(&b.r, &mut ra, &mut s);
            diff_eq!(format!("C54 len={len}"), co, ro);
            (b.c.strreset)(&mut ca);
            (b.r.strreset)(&mut ra);
        }
    }
}

#[test]
fn c55_stralloc_many_allocations() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 55);
    for trial in 0..40 {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        let mut kept_c: Vec<*mut c_char> = Vec::new();
        let mut kept_r: Vec<*mut c_char> = Vec::new();
        let mut contents: Vec<Vec<u8>> = Vec::new();
        for step in 0..300 {
            // Mix short strings with occasional huge ones that force oversize blocks.
            let len = if step % 37 == 36 {
                600 + rng.below(4000)
            } else {
                rng.below(80)
            };
            let mut s = rng.ascii(len);
            unsafe {
                let cret = (b.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
                let rret = (b.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
                diff_eq!(
                    format!("C55 trial={trial} step={step} len={len} content"),
                    read_cstr(cret),
                    read_cstr(rret)
                );
                diff_eq!(
                    format!("C55 trial={trial} step={step} len={len} loc"),
                    classify(&ca, cret),
                    classify(&ra, rret)
                );
                diff_eq!(
                    format!("C55 trial={trial} step={step} len={len} arena"),
                    snap_arena(&ca),
                    snap_arena(&ra)
                );
                kept_c.push(cret);
                kept_r.push(rret);
                let mut c = s.clone();
                c.pop();
                contents.push(c);
            }
        }
        // Every previously returned pointer must still hold its string on both sides.
        unsafe {
            for (i, want) in contents.iter().enumerate() {
                diff_eq!(
                    format!("C55 trial={trial} retained #{i}"),
                    read_cstr(kept_c[i]),
                    read_cstr(kept_r[i])
                );
                assert_eq!(&read_cstr(kept_c[i]), want, "C55 C content drift at #{i}");
            }
            (b.c.strreset)(&mut ca);
            (b.r.strreset)(&mut ra);
            diff_eq!(
                format!("C55 trial={trial} after reset"),
                snap_arena(&ca),
                snap_arena(&ra)
            );
        }
    }
}

#[test]
fn c56_stralloc_fast_path() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 56);
    for _ in 0..100 {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        unsafe {
            // First allocation creates a 512-byte block.
            let mut seed_s = rng.ascii(4);
            let _ = stralloc_obs(&b.c, &mut ca, &mut seed_s);
            let _ = stralloc_obs(&b.r, &mut ra, &mut seed_s);
            // Subsequent small allocations must not create new blocks.
            for step in 0..20 {
                let n = rng.below(16);
                let mut s = rng.ascii(n);
                let co = stralloc_obs(&b.c, &mut ca, &mut s);
                let ro = stralloc_obs(&b.r, &mut ra, &mut s);
                diff_eq!(format!("C56 step={step}"), co, ro);
            }
            (b.c.strreset)(&mut ca);
            (b.r.strreset)(&mut ra);
        }
    }
}

#[test]
fn c57_stralloc_oversize_null_storage() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 57);
    for len in [512usize, 513, 600, 1000, 4096, 100_000] {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        let mut s = rng.ascii(len);
        unsafe {
            let co = stralloc_obs(&b.c, &mut ca, &mut s);
            let ro = stralloc_obs(&b.r, &mut ra, &mut s);
            diff_eq!(format!("C57 len={len}"), co, ro);
            (b.c.strreset)(&mut ca);
            (b.r.strreset)(&mut ra);
        }
    }
}

#[test]
fn c58_stralloc_oversize_with_existing_storage() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 58);
    for big in [513usize, 700, 5000, 50_000] {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        unsafe {
            let mut small = rng.ascii(8);
            let _ = stralloc_obs(&b.c, &mut ca, &mut small);
            let _ = stralloc_obs(&b.r, &mut ra, &mut small);
            let mut s = rng.ascii(big);
            let co = stralloc_obs(&b.c, &mut ca, &mut s);
            let ro = stralloc_obs(&b.r, &mut ra, &mut s);
            diff_eq!(format!("C58 big={big}"), co, ro);
            // The head block must still be usable afterwards.
            let mut after = rng.ascii(4);
            let co2 = stralloc_obs(&b.c, &mut ca, &mut after);
            let ro2 = stralloc_obs(&b.r, &mut ra, &mut after);
            diff_eq!(format!("C58 big={big} follow-up"), co2, ro2);
            (b.c.strreset)(&mut ca);
            (b.r.strreset)(&mut ra);
        }
    }
}

#[test]
fn c59_stralloc_block_in_range() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 59);
    for block in [0u8, 1, 2, 3, 4, 5, 20, 21, 22, 23] {
        for len in [1usize, 8, 200, 511, 512, 513, 2000, 100_000] {
            let mut ca = StringArena::zeroed();
            let mut ra = StringArena::zeroed();
            ca.block = block;
            ra.block = block;
            let mut s = rng.ascii(len - 1);
            unsafe {
                let co = stralloc_obs(&b.c, &mut ca, &mut s);
                let ro = stralloc_obs(&b.r, &mut ra, &mut s);
                diff_eq!(format!("C59 block={block} len={len}"), co, ro);
                (b.c.strreset)(&mut ca);
                (b.r.strreset)(&mut ra);
            }
        }
    }
}

#[test]
fn c60_stralloc_block_out_of_range() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 60);
    // `a->block` is caller-visible state; the C computes
    // `(size_t)512u << (block>>1)` in size_t, so the shift count reaches 127.
    // On x86-64 `shl` masks the count by 63, which is what both sides must do.
    //
    // Block values whose masked shift lands in 21..=54 make `blocksize` a
    // multi-gigabyte-to-exabyte figure that the C then tries to `realloc`;
    // that fails and BOTH libraries dereference NULL (identical crash, not a
    // divergence), so those values are excluded — they are not observable.
    for block in [
        24u8, 25, 26, 27, 28, 29, 30, 31, // shift 12..15 -> 2MiB..16MiB blocks
        110, 111, 120, 127, // shift 55..63 -> 512<<shift wraps to 0
        128, 129, 130, 131, 140, 141, 150, 151, // shift 64..75 -> masked 0..11
        240, 241, 254, 255, // shift 120..127 -> masked 56..63 -> wraps to 0
    ] {
        for len in [1usize, 16, 600] {
            let mut ca = StringArena::zeroed();
            let mut ra = StringArena::zeroed();
            ca.block = block;
            ra.block = block;
            let mut s = rng.ascii(len - 1);
            unsafe {
                let co = stralloc_obs(&b.c, &mut ca, &mut s);
                let ro = stralloc_obs(&b.r, &mut ra, &mut s);
                diff_eq!(format!("C60 block={block} len={len}"), co, ro);
                (b.c.strreset)(&mut ca);
                (b.r.strreset)(&mut ra);
            }
        }
    }
}

#[test]
fn c61_strreset_chain_lengths() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 61);
    for nblocks in 0..9usize {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        unsafe {
            for _ in 0..nblocks {
                // Each oversize allocation adds a block.
                let mut s = rng.ascii(2000);
                let _ = (b.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char);
                let _ = (b.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char);
            }
            diff_eq!(
                format!("C61 nblocks={nblocks} before"),
                snap_arena(&ca),
                snap_arena(&ra)
            );
            (b.c.strreset)(&mut ca);
            (b.r.strreset)(&mut ra);
            diff_eq!(
                format!("C61 nblocks={nblocks} after"),
                snap_arena(&ca),
                snap_arena(&ra)
            );
            assert_eq!(snap_arena(&ca), (0, 0, 0, 0), "C61 arena not zeroed");
        }
    }
}

// ===========================================================================
// C62–C63: strkey / arr_del
// ===========================================================================

#[test]
fn c62_strkey() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 62);
    let mut cases: Vec<c_int> = vec![0, 1, -1, 12345, c_int::MAX, c_int::MIN, 99, -99];
    for _ in 0..200 {
        cases.push(rng.next_u32() as c_int);
    }
    for n in cases {
        unsafe {
            let c = read_cstr((b.c.strkey)(n));
            let r = read_cstr((b.r.strkey)(n));
            diff_eq!(format!("C62 n={n}"), c, r);
        }
    }
    // The returned pointer must be the same static buffer on every call.
    unsafe {
        let c1 = (b.c.strkey)(1);
        let c2 = (b.c.strkey)(2);
        let r1 = (b.r.strkey)(1);
        let r2 = (b.r.strkey)(2);
        diff_eq!("C62 stable buffer", c1 == c2, r1 == r2);
    }
}

#[test]
fn c63_arr_del() {
    let b = libs();
    let mut rng = Rng::new(TEST_SEED ^ 63);
    let mut cases: Vec<c_int> = vec![0, 1, -1, 2, 3, 4, c_int::MAX, c_int::MIN];
    for _ in 0..200 {
        cases.push(rng.next_u32() as c_int);
    }
    for n in cases {
        unsafe {
            (b.c.arr_del)(n);
            (b.r.arr_del)(n);
        }
    }
}
