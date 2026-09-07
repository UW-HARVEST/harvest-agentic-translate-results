//! Phase B — low-level differential tests.
//!
//! CONFIGS.md rows 1-11, 38-44: the hash primitives, the raw array grower,
//! `stbds_hmput_default`, the string arena, `strkey` and `arr_del`.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

// ===========================================================================
// Rows 1-5 — stbds_hash_bytes
// ===========================================================================

fn cmp_hash_bytes(p: &Pair, data: &[u8], seed: usize, ctx: &str) {
    let mut buf = data.to_vec();
    if buf.is_empty() {
        buf.push(0); // keep the pointer valid; len=0 means it is never read
    }
    let ptr = buf.as_mut_ptr() as *mut c_void;
    let len = data.len();
    let cv = unsafe { (p.c.hash_bytes)(ptr, len, seed) };
    let rv = unsafe { (p.rs.hash_bytes)(ptr, len, seed) };
    assert_eq!(
        cv, rv,
        "hash_bytes divergence [{ctx}] data={data:02x?} seed={seed:#x}: C={cv:#x} Rust={rv:#x}"
    );
}

const SEEDS: &[usize] = &[
    0,
    1,
    2,
    0x3141_5926,
    usize::MAX,
    usize::MAX - 1,
    0x8000_0000_0000_0000,
    0xdead_beef_cafe_babe,
];

#[test]
fn cfg_01_hash_bytes_tail_lengths() {
    let p = pair();
    let mut rng = Rng::new(0x01);
    // len 0 plus every switch-case 1..=7
    for len in 0..=7usize {
        for &seed in SEEDS {
            cmp_hash_bytes(&p, &vec![0u8; len], seed, "zeros");
            cmp_hash_bytes(&p, &vec![0xffu8; len], seed, "ones");
            for _ in 0..200 {
                let d = rng.bytes(len);
                cmp_hash_bytes(&p, &d, seed, "random");
            }
        }
    }
}

#[test]
fn cfg_02_hash_bytes_whole_blocks() {
    let p = pair();
    let mut rng = Rng::new(0x02);
    for len in [8usize, 16, 24, 32, 64, 128, 256] {
        for &seed in SEEDS {
            cmp_hash_bytes(&p, &vec![0u8; len], seed, "zeros");
            cmp_hash_bytes(&p, &vec![0xffu8; len], seed, "ones");
            for _ in 0..100 {
                let d = rng.bytes(len);
                cmp_hash_bytes(&p, &d, seed, "random");
            }
        }
    }
}

#[test]
fn cfg_03_hash_bytes_block_plus_tail() {
    let p = pair();
    let mut rng = Rng::new(0x03);
    for len in 9..=64usize {
        for &seed in SEEDS {
            for _ in 0..40 {
                let d = rng.bytes(len);
                cmp_hash_bytes(&p, &d, seed, "random");
            }
        }
    }
    // A few very long ones too.
    for _ in 0..200 {
        let len = 65 + rng.below(4000);
        let d = rng.bytes(len);
        cmp_hash_bytes(&p, &d, rng.next_u64() as usize, "long");
    }
}

#[test]
fn cfg_04_hash_bytes_sign_extension() {
    // The C computes `d[0] | (d[1]<<8) | (d[2]<<16) | (d[3]<<24)` as an `int`;
    // when d[3] >= 0x80 the value is negative and sign-extends on the widening
    // conversion to size_t. Same for bytes 4..7 and for the tail `case 4`.
    let p = pair();
    let mut rng = Rng::new(0x04);
    for &hi3 in &[0x00u8, 0x7f, 0x80, 0xff] {
        for &hi7 in &[0x00u8, 0x7f, 0x80, 0xff] {
            for &seed in SEEDS {
                // Whole-block path.
                for _ in 0..50 {
                    let mut d = rng.bytes(8);
                    d[3] = hi3;
                    d[7] = hi7;
                    cmp_hash_bytes(&p, &d, seed, "block sign-ext");
                    // 16 bytes, both blocks.
                    let mut d16 = rng.bytes(16);
                    d16[3] = hi3;
                    d16[7] = hi7;
                    d16[11] = hi3;
                    d16[15] = hi7;
                    cmp_hash_bytes(&p, &d16, seed, "2 blocks sign-ext");
                }
                // Tail path with len 4..7 (case 4 does the int shift).
                for len in 4..=7usize {
                    for _ in 0..50 {
                        let mut d = rng.bytes(len);
                        d[3] = hi3;
                        cmp_hash_bytes(&p, &d, seed, "tail sign-ext");
                    }
                }
                // Block + tail where both sign-extend.
                for len in 12..=15usize {
                    let mut d = rng.bytes(len);
                    d[3] = hi3;
                    d[7] = hi7;
                    d[11] = hi3;
                    cmp_hash_bytes(&p, &d, seed, "block+tail sign-ext");
                }
            }
        }
    }
}

#[test]
fn cfg_05_hash_bytes_seed_space() {
    let p = pair();
    let mut rng = Rng::new(0x05);
    for _ in 0..4000 {
        let len = rng.below(40);
        let d = rng.bytes(len);
        let seed = rng.next_u64() as usize;
        cmp_hash_bytes(&p, &d, seed, "random seed");
    }
    for &seed in SEEDS {
        for len in 0..40usize {
            cmp_hash_bytes(&p, &vec![0x80u8; len], seed, "0x80 fill");
        }
    }
}

// ===========================================================================
// Row 6 — stbds_hash_string
// ===========================================================================

fn cmp_hash_string(p: &Pair, s: &[u8], seed: usize, ctx: &str) {
    let mut buf = s.to_vec();
    assert_eq!(*buf.last().unwrap(), 0, "must be NUL terminated");
    let ptr = buf.as_mut_ptr() as *mut c_char;
    let cv = unsafe { (p.c.hash_string)(ptr, seed) };
    let rv = unsafe { (p.rs.hash_string)(ptr, seed) };
    assert_eq!(
        cv, rv,
        "hash_string divergence [{ctx}] s={s:02x?} seed={seed:#x}: C={cv:#x} Rust={rv:#x}"
    );
}

#[test]
fn cfg_06_hash_string() {
    let p = pair();
    let mut rng = Rng::new(0x06);
    for &seed in SEEDS {
        cmp_hash_string(&p, b"\0", seed, "empty");
        cmp_hash_string(&p, b"a\0", seed, "1 char");
        cmp_hash_string(&p, b"\x7f\0", seed, "0x7f");
        cmp_hash_string(&p, b"\x80\0", seed, "0x80 (unsigned char cast)");
        cmp_hash_string(&p, b"\xff\0", seed, "0xff");
        cmp_hash_string(&p, b"\xff\xff\xff\xff\xff\xff\xff\xff\0", seed, "8x0xff");
        cmp_hash_string(&p, b"test_0\0", seed, "strkey-like");
        cmp_hash_string(
            &p,
            b"the quick brown fox jumps over the lazy dog\0",
            seed,
            "long ascii",
        );
        for len in 0..40usize {
            for _ in 0..20 {
                let s = rng.cstring(len);
                cmp_hash_string(&p, &s, seed, "random");
            }
        }
    }
    // Random seeds
    for _ in 0..3000 {
        let len = rng.below(64);
        let s = rng.cstring(len);
        cmp_hash_string(&p, &s, rng.next_u64() as usize, "random seed");
    }
}

// ===========================================================================
// Row 7 / 44 — stbds_rand_seed and the per-table seed LCG
// ===========================================================================

#[test]
fn cfg_07_44_rand_seed_sequence() {
    let p = pair();
    // The raw hash functions take the seed explicitly, so rand_seed must not
    // change their output.
    let mut rng = Rng::new(0x07);
    for &g in &[0usize, 1, 12345, usize::MAX] {
        p.reseed(g);
        for _ in 0..200 {
            let n = rng.below(32);
        let d = rng.bytes(n);
            cmp_hash_bytes(&p, &d, 0xabcd, "after rand_seed");
        }
    }

    // The per-table seed *is* taken from the global and then advanced by
    // seed = seed*a + b. Chain many fresh tables and compare each seed.
    for &g in &[0usize, 1, 0x3141_5926, usize::MAX, 0xdead_beef] {
        p.reseed(g);
        let elemsize = 8usize;
        let mut cseeds = Vec::new();
        let mut rseeds = Vec::new();
        for i in 0..32u64 {
            let mut key = i.to_le_bytes();
            let kp = key.as_mut_ptr() as *mut c_void;
            let ch = unsafe { (p.c.hmput_key)(std::ptr::null_mut(), elemsize, kp, 8, HM_BINARY) };
            let rh = unsafe { (p.rs.hmput_key)(std::ptr::null_mut(), elemsize, kp, 8, HM_BINARY) };
            cseeds.push(unsafe { table_snapshot(ch, elemsize) }.unwrap().seed);
            rseeds.push(unsafe { table_snapshot(rh, elemsize) }.unwrap().seed);
            unsafe {
                (p.c.hmfree_func)(hash_to_arr(ch, elemsize), elemsize);
                (p.rs.hmfree_func)(hash_to_arr(rh, elemsize), elemsize);
            }
        }
        // The two libraries advance their own global; because they were reseeded
        // to the same value and each created the same number of tables, the
        // sequences must be identical.
        assert_eq!(cseeds, rseeds, "table seed sequence for global seed {g:#x}");
        // And it must actually be an LCG (not constant).
        assert!(cseeds.windows(2).any(|w| w[0] != w[1]));
    }
}

// ===========================================================================
// Rows 8-10 — stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

#[test]
fn cfg_08_arrgrowf_from_null() {
    let p = pair();
    for &elemsize in &[0usize, 1, 2, 3, 4, 8, 16, 20, 64] {
        for &addlen in &[0usize, 1, 2, 3, 4, 5, 7, 8, 100] {
            for &min_cap in &[0usize, 1, 2, 3, 4, 5, 7, 8, 100] {
                let ca =
                    unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
                let ra =
                    unsafe { (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
                let ctx = format!("elemsize={elemsize} addlen={addlen} min_cap={min_cap}");
                assert_eq!(
                    ca.is_null(),
                    ra.is_null(),
                    "null-ness differs [{ctx}] C={ca:?} Rust={ra:?}"
                );
                if ca.is_null() {
                    // Row 1 of ERRORS.md: addlen==0 && min_cap==0 -> NULL.
                    assert_eq!((addlen, min_cap), (0, 0), "unexpected NULL [{ctx}]");
                    continue;
                }
                let ch = unsafe { hdr_snapshot(ca) };
                let rh = unsafe { hdr_snapshot(ra) };
                diffeq!(ctx, ch.clone(), rh);
                assert_eq!(ch.length, 0, "[{ctx}]");
                assert_eq!(ch.temp, 0, "[{ctx}]");
                assert!(!ch.has_table, "[{ctx}]");
                unsafe {
                    (p.c.arrfreef)(ca);
                    (p.rs.arrfreef)(ra);
                }
            }
        }
    }
}

#[test]
fn cfg_09_arrgrowf_growth_chain() {
    let p = pair();
    let mut rng = Rng::new(0x09);
    for &elemsize in &[1usize, 4, 8, 16, 20] {
        for trial in 0..60 {
            let mut ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0) };
            let mut ra = unsafe { (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0) };
            for step in 0..40 {
                // Randomly bump the length like arrput would, then grow.
                let addlen = rng.below(9);
                let min_cap = if rng.next_u64() & 1 == 0 { 0 } else { rng.below(40) };
                let bump = rng.below(3);
                let (prev_len, prev_cap) = unsafe {
                    let chp = (ca as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader;
                    let rhp = (ra as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader;
                    let newlen = ((*chp).length + bump).min((*chp).capacity);
                    (*chp).length = newlen;
                    (*rhp).length = newlen;
                    (newlen, (*chp).capacity)
                };
                let nca = unsafe { (p.c.arrgrowf)(ca, elemsize, addlen, min_cap) };
                let nra = unsafe { (p.rs.arrgrowf)(ra, elemsize, addlen, min_cap) };
                let ctx = format!(
                    "elemsize={elemsize} trial={trial} step={step} addlen={addlen} min_cap={min_cap} len={prev_len} cap={prev_cap}"
                );
                // The C returns `a` unchanged iff max(len+addlen, min_cap) <= cap
                // (line 286). `realloc` is free to return the same address on
                // the grow path, so identity alone cannot distinguish the two —
                // predict the branch and require both libs to honour it.
                let early = prev_len.wrapping_add(addlen).max(min_cap) <= prev_cap;
                if early {
                    assert_eq!(nca, ca, "C should have early-returned [{ctx}]");
                    assert_eq!(nra, ra, "Rust should have early-returned [{ctx}]");
                }
                ca = nca;
                ra = nra;
                diffeq!(ctx, unsafe { hdr_snapshot(ca) }, unsafe {
                    hdr_snapshot(ra)
                });
            }
            unsafe {
                (p.c.arrfreef)(ca);
                (p.rs.arrfreef)(ra);
            }
        }
    }
}

#[test]
fn cfg_10_arrgrowf_preserves_data() {
    let p = pair();
    let mut rng = Rng::new(0x0a);
    let elemsize = 4usize;
    for _ in 0..50 {
        let n = 1 + rng.below(500);
        let vals: Vec<u32> = (0..n).map(|_| rng.next_u32()).collect();
        let mut ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0) };
        let mut ra = unsafe { (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0) };
        for (i, &v) in vals.iter().enumerate() {
            unsafe {
                let chp = (ca as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader;
                let rhp = (ra as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader;
                if (*chp).length + 1 > (*chp).capacity {
                    ca = (p.c.arrgrowf)(ca, elemsize, 1, 0);
                }
                if (*rhp).length + 1 > (*rhp).capacity {
                    ra = (p.rs.arrgrowf)(ra, elemsize, 1, 0);
                }
                let chp = (ca as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader;
                let rhp = (ra as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader;
                *(ca as *mut u32).add(i) = v;
                *(ra as *mut u32).add(i) = v;
                (*chp).length = i + 1;
                (*rhp).length = i + 1;
            }
        }
        diffeq!("after fill", unsafe { hdr_snapshot(ca) }, unsafe {
            hdr_snapshot(ra)
        });
        diffeq!("bytes", unsafe { arr_bytes(ca, elemsize) }, unsafe {
            arr_bytes(ra, elemsize)
        });
        unsafe {
            (p.c.arrfreef)(ca);
            (p.rs.arrfreef)(ra);
        }
    }
}

// ===========================================================================
// Row 11 — stbds_hmput_default
// ===========================================================================

#[test]
fn cfg_11_hmput_default() {
    let p = pair();
    for &elemsize in &[4usize, 8, 16, 20, 64] {
        // (a) a == NULL
        let ch = unsafe { (p.c.hmput_default)(std::ptr::null_mut(), elemsize) };
        let rh = unsafe { (p.rs.hmput_default)(std::ptr::null_mut(), elemsize) };
        let ctx = format!("null a, elemsize={elemsize}");
        diffeq!(
            ctx.clone(),
            unsafe { hash_hdr_snapshot(ch, elemsize) },
            unsafe { hash_hdr_snapshot(rh, elemsize) }
        );
        diffeq!(ctx.clone(), unsafe { hash_elems(ch, elemsize) }, unsafe {
            hash_elems(rh, elemsize)
        });
        assert_eq!(unsafe { hash_hdr_snapshot(ch, elemsize) }.length, 1);

        // (b) a != NULL with length > 0 -> no-op, same pointer
        let ch2 = unsafe { (p.c.hmput_default)(ch, elemsize) };
        let rh2 = unsafe { (p.rs.hmput_default)(rh, elemsize) };
        assert_eq!(ch2, ch, "C hmput_default should be a no-op");
        assert_eq!(rh2, rh, "Rust hmput_default should be a no-op");

        // (c) a != NULL with length == 0
        unsafe {
            let cap = hash_to_arr(ch, elemsize);
            let rap = hash_to_arr(rh, elemsize);
            ((cap as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader).as_mut().unwrap().length = 0;
            ((rap as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader).as_mut().unwrap().length = 0;
        }
        let ch3 = unsafe { (p.c.hmput_default)(ch, elemsize) };
        let rh3 = unsafe { (p.rs.hmput_default)(rh, elemsize) };
        let ctx = format!("zero-length a, elemsize={elemsize}");
        diffeq!(
            ctx.clone(),
            unsafe { hash_hdr_snapshot(ch3, elemsize) },
            unsafe { hash_hdr_snapshot(rh3, elemsize) }
        );
        diffeq!(ctx, unsafe { hash_elems(ch3, elemsize) }, unsafe {
            hash_elems(rh3, elemsize)
        });

        unsafe {
            (p.c.hmfree_func)(hash_to_arr(ch3, elemsize), elemsize);
            (p.rs.hmfree_func)(hash_to_arr(rh3, elemsize), elemsize);
        }
    }
}

// ===========================================================================
// Rows 38-41 — stbds_stralloc / stbds_strreset
// ===========================================================================

/// Snapshot of an arena that is comparable across libraries: the block chain
/// lengths and the offset of `p` inside its block.
unsafe fn arena_chain_len(a: *const StringArena) -> usize {
    unsafe {
        let mut n = 0usize;
        let mut x = (*a).storage as *const *const c_void;
        while !x.is_null() {
            n += 1;
            x = *x as *const *const c_void;
            if n > 100000 {
                panic!("arena chain loop");
            }
        }
        n
    }
}

#[test]
fn cfg_38_stralloc_boundaries() {
    let p = pair();
    for len in [0usize, 1, 2, 100, 502, 503, 504, 511, 512, 513, 1000, 5000, 100000] {
        let mut s: Vec<u8> = vec![b'x'; len];
        s.push(0);
        let sp = s.as_mut_ptr() as *mut c_char;

        let mut ca = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
        let mut ra = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
        let cp = unsafe { (p.c.stralloc)(&mut ca, sp) };
        let rp = unsafe { (p.rs.stralloc)(&mut ra, sp) };
        let ctx = format!("fresh arena len={len}");
        diffeq!(ctx.clone(), unsafe { cstr(cp) }, unsafe { cstr(rp) });
        assert_eq!(unsafe { cstr(cp) }, s[..len], "[{ctx}] content");
        diffeq!(
            format!("{ctx} remaining"),
            ca.remaining,
            ra.remaining
        );
        diffeq!(format!("{ctx} block"), ca.block, ra.block);
        diffeq!(
            format!("{ctx} storage null"),
            ca.storage.is_null(),
            ra.storage.is_null()
        );
        // offset of the returned pointer inside its block
        let coff = (cp as usize).wrapping_sub(ca.storage as usize);
        let roff = (rp as usize).wrapping_sub(ra.storage as usize);
        diffeq!(format!("{ctx} offset-in-block"), coff, roff);

        unsafe {
            (p.c.strreset)(&mut ca);
            (p.rs.strreset)(&mut ra);
        }
        assert_eq!(ca.remaining, 0);
        assert_eq!(ra.remaining, 0);
        assert!(ca.storage.is_null() && ra.storage.is_null());
    }
}

#[test]
fn cfg_39_stralloc_block_growth() {
    let p = pair();
    let mut rng = Rng::new(0x27);
    let mut ca = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
    let mut ra = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
    // Many random small strings: drives `block` upward. Each new block doubles
    // every 2 increments (512 << (block>>1)), saturating at 1 MiB.
    for i in 0..4000 {
        let len = rng.below(300);
        let mut s: Vec<u8> = (0..len).map(|_| b'a' + (rng.below(26) as u8)).collect();
        s.push(0);
        let sp = s.as_mut_ptr() as *mut c_char;
        let cp = unsafe { (p.c.stralloc)(&mut ca, sp) };
        let rp = unsafe { (p.rs.stralloc)(&mut ra, sp) };
        let ctx = format!("i={i} len={len}");
        diffeq!(format!("{ctx} content"), unsafe { cstr(cp) }, unsafe {
            cstr(rp)
        });
        diffeq!(format!("{ctx} remaining"), ca.remaining, ra.remaining);
        diffeq!(format!("{ctx} block"), ca.block, ra.block);
        let coff = (cp as usize).wrapping_sub(ca.storage as usize);
        let roff = (rp as usize).wrapping_sub(ra.storage as usize);
        diffeq!(format!("{ctx} offset"), coff, roff);
        diffeq!(
            format!("{ctx} chain"),
            unsafe { arena_chain_len(&ca) },
            unsafe { arena_chain_len(&ra) }
        );
    }
    // block should have grown well past 0
    assert!(ca.block > 4, "block did not grow: {}", ca.block);
    assert_eq!(ca.block, ra.block);
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.rs.strreset)(&mut ra);
    }
}

#[test]
fn cfg_39b_stralloc_block_saturation() {
    // Force a->block up to its saturation point (block == 22, blocksize 1 MiB)
    // by asking for strings that always exceed `remaining`.
    let p = pair();
    let mut ca = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
    let mut ra = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
    for i in 0..30 {
        // A string just longer than `remaining` forces the grow path each time.
        let len = ca.remaining + 1;
        assert_eq!(ca.remaining, ra.remaining);
        let mut s: Vec<u8> = vec![b'z'; len.saturating_sub(1)];
        s.push(0);
        let sp = s.as_mut_ptr() as *mut c_char;
        let cp = unsafe { (p.c.stralloc)(&mut ca, sp) };
        let rp = unsafe { (p.rs.stralloc)(&mut ra, sp) };
        let ctx = format!("saturation i={i} len={len}");
        diffeq!(format!("{ctx} content"), unsafe { cstr(cp) }, unsafe {
            cstr(rp)
        });
        diffeq!(format!("{ctx} remaining"), ca.remaining, ra.remaining);
        diffeq!(format!("{ctx} block"), ca.block, ra.block);
        diffeq!(
            format!("{ctx} chain"),
            unsafe { arena_chain_len(&ca) },
            unsafe { arena_chain_len(&ra) }
        );
    }
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.rs.strreset)(&mut ra);
    }
}

#[test]
fn cfg_40_stralloc_oversize_splice() {
    let p = pair();
    // (a) oversize on a *fresh* arena -> a->storage = sb, a->remaining = 0
    {
        let mut ca = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
        let mut ra = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
        let mut s: Vec<u8> = vec![b'q'; 4096];
        s.push(0);
        let sp = s.as_mut_ptr() as *mut c_char;
        let cp = unsafe { (p.c.stralloc)(&mut ca, sp) };
        let rp = unsafe { (p.rs.stralloc)(&mut ra, sp) };
        diffeq!("oversize fresh content", unsafe { cstr(cp) }, unsafe {
            cstr(rp)
        });
        diffeq!("oversize fresh remaining", ca.remaining, ra.remaining);
        diffeq!("oversize fresh block", ca.block, ra.block);
        assert_eq!(ca.remaining, 0);
        // returned pointer is the block's storage field (offset 8)
        assert_eq!((cp as usize) - (ca.storage as usize), 8);
        assert_eq!((rp as usize) - (ra.storage as usize), 8);
        unsafe {
            (p.c.strreset)(&mut ca);
            (p.rs.strreset)(&mut ra);
        }
    }
    // (b) oversize *after* the arena already has storage -> splice after head
    {
        let mut ca = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
        let mut ra = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
        let mut small = b"hello\0".to_vec();
        let smp = small.as_mut_ptr() as *mut c_char;
        unsafe {
            (p.c.stralloc)(&mut ca, smp);
            (p.rs.stralloc)(&mut ra, smp);
        }
        assert!(!ca.storage.is_null() && !ra.storage.is_null());
        for round in 0..4 {
            let mut s: Vec<u8> = vec![b'Q'; 3_000_000];
            s.push(0);
            let sp = s.as_mut_ptr() as *mut c_char;
            let cp = unsafe { (p.c.stralloc)(&mut ca, sp) };
            let rp = unsafe { (p.rs.stralloc)(&mut ra, sp) };
            let ctx = format!("oversize splice round={round}");
            diffeq!(format!("{ctx} len"), unsafe { cstr(cp) }.len(), unsafe {
                cstr(rp)
            }
            .len());
            diffeq!(format!("{ctx} remaining"), ca.remaining, ra.remaining);
            diffeq!(format!("{ctx} block"), ca.block, ra.block);
            diffeq!(
                format!("{ctx} chain"),
                unsafe { arena_chain_len(&ca) },
                unsafe { arena_chain_len(&ra) }
            );
        }
        unsafe {
            (p.c.strreset)(&mut ca);
            (p.rs.strreset)(&mut ra);
        }
        assert!(ca.storage.is_null() && ra.storage.is_null());
        assert_eq!(ca.block, 0);
        assert_eq!(ra.block, 0);
    }
}

#[test]
fn cfg_41_strreset_shapes() {
    let p = pair();
    let mut rng = Rng::new(0x29);
    // (a) empty
    let mut ca = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 7 };
    let mut ra = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 7 };
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.rs.strreset)(&mut ra);
    }
    assert_eq!(ca, ra);
    assert_eq!(ca.mode, 0, "strreset must zero the whole struct");

    // (b) 1 block, (c) many blocks, (d) with a spliced oversize block
    for nblocks in [1usize, 2, 5, 30] {
        let mut ca =
            StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 3 };
        let mut ra =
            StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 3 };
        for _ in 0..nblocks {
            // force a new block every time
            let len = ca.remaining + 1;
            let mut s: Vec<u8> = vec![b'k'; len.saturating_sub(1)];
            s.push(0);
            let sp = s.as_mut_ptr() as *mut c_char;
            unsafe {
                (p.c.stralloc)(&mut ca, sp);
                (p.rs.stralloc)(&mut ra, sp);
            }
        }
        // a few normal ones
        for _ in 0..10 {
            let n = rng.below(40);
            let mut s = rng.cstring(n);
            let sp = s.as_mut_ptr() as *mut c_char;
            unsafe {
                (p.c.stralloc)(&mut ca, sp);
                (p.rs.stralloc)(&mut ra, sp);
            }
        }
        diffeq!(
            format!("nblocks={nblocks} chain"),
            unsafe { arena_chain_len(&ca) },
            unsafe { arena_chain_len(&ra) }
        );
        unsafe {
            (p.c.strreset)(&mut ca);
            (p.rs.strreset)(&mut ra);
        }
        assert_eq!(ca, ra);
        assert!(ca.storage.is_null());
        assert_eq!(ca.remaining, 0);
        assert_eq!(ca.block, 0);
        assert_eq!(ca.mode, 0);
    }
}

// ===========================================================================
// Row 42 — strkey
// ===========================================================================

#[test]
fn cfg_42_strkey() {
    let p = pair();
    let mut rng = Rng::new(0x2a);
    let mut cases: Vec<c_int> = vec![
        0,
        1,
        -1,
        9,
        10,
        -9,
        -10,
        99,
        100,
        999,
        1000,
        -1000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    for _ in 0..2000 {
        cases.push(rng.next_u32() as i32);
    }
    for n in cases {
        let cp = unsafe { (p.c.strkey)(n) };
        let cs = unsafe { cstr(cp) };
        let rp = unsafe { (p.rs.strkey)(n) };
        let rs = unsafe { cstr(rp) };
        assert_eq!(
            cs,
            rs,
            "strkey({n}): C={:?} Rust={:?}",
            String::from_utf8_lossy(&cs),
            String::from_utf8_lossy(&rs)
        );
        assert_eq!(cs, format!("test_{n}").into_bytes(), "strkey({n}) content");
    }
}

// ===========================================================================
// Row 43 — arr_del
// ===========================================================================

#[test]
fn cfg_43_arr_del() {
    let p = pair();
    let mut rng = Rng::new(0x2b);
    let mut cases: Vec<c_int> = vec![0, 1, -1, 2, 3, 4, i32::MAX, i32::MIN];
    for _ in 0..500 {
        cases.push(rng.next_u32() as i32);
    }
    for n in cases {
        // `arr_del` is void and has no observable output; both must simply
        // complete (no abort from a live assert, no allocator corruption).
        unsafe {
            (p.c.arr_del)(n);
            (p.rs.arr_del)(n);
        }
    }
}

// ===========================================================================
// Row 46 (layout) — the harness's struct mirrors must match the C's ABI
// ===========================================================================

/// The whole suite reads the C library's `stbds_array_header`,
/// `stbds_hash_index`, `stbds_hash_bucket` and `stbds_string_arena` through the
/// `#[repr(C)]` mirrors in `tests/common/mod.rs`. If any size or offset were
/// wrong, every comparison would be meaningless, so pin them down explicitly
/// and cross-check the two that the C exposes indirectly.
#[test]
fn cfg_46_struct_layout() {
    use std::mem::{align_of, size_of};

    // sizeof(stbds_array_header) == 4 * 8
    assert_eq!(size_of::<ArrayHeader>(), 32);
    assert_eq!(align_of::<ArrayHeader>(), 8);
    assert_eq!(HDR_SIZE, 32);
    // sizeof(stbds_hash_bucket) == size_t[8] + ptrdiff_t[8]
    assert_eq!(size_of::<HashBucket>(), 128);
    // struct stbds_string_arena { void*; size_t; unsigned char; unsigned char; }
    assert_eq!(size_of::<StringArena>(), 24);
    assert_eq!(align_of::<StringArena>(), 8);
    // stbds_hash_index: char* + 8 * size_t + stbds_string_arena + ptr
    assert_eq!(size_of::<HashIndex>(), 8 + 8 * 8 + 24 + 8);

    let p = pair();

    // Cross-check HDR_SIZE against the C: `stbds_arrgrowf` returns
    // `realloc(...) + sizeof(header)`, and the header's `capacity` field sits at
    // offset 8. Reading it back through the mirror must give the value the C
    // computed (4, from the `min_cap < 4` bump).
    for &elemsize in &[1usize, 4, 8, 16] {
        let ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0) };
        let ra = unsafe { (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0) };
        let ch = unsafe { header(ca) };
        assert_eq!(ch.length, 0);
        assert_eq!(ch.capacity, 4, "capacity must be at header offset 8");
        assert_eq!(ch.temp, 0, "temp must be at header offset 24");
        assert!(ch.hash_table.is_null(), "hash_table must be at offset 16");
        dq(
            format!("layout elemsize={elemsize}"),
            unsafe { hdr_snapshot(ca) },
            unsafe { hdr_snapshot(ra) },
        );
        unsafe {
            (p.c.arrfreef)(ca);
            (p.rs.arrfreef)(ra);
        }
    }

    // Cross-check the stbds_hash_index layout: `stbds_shmode_func` sets
    // `string.mode`, `slot_count`, and 64-byte-aligns `storage`.
    for &mode in &[0i32, 1, 2, 3] {
        let elemsize = 16usize;
        let ch = unsafe { (p.c.shmode_func)(elemsize, mode) };
        let rh = unsafe { (p.rs.shmode_func)(elemsize, mode) };
        for (name, h) in [("C", ch), ("Rust", rh)] {
            let hdr = unsafe { header(hash_to_arr(h, elemsize)) };
            let t = hdr.hash_table as *const HashIndex;
            let ti = unsafe { *t };
            assert_eq!(ti.slot_count, 8, "{name}: slot_count offset");
            assert_eq!(ti.slot_count_log2, 3, "{name}: slot_count_log2 offset");
            assert_eq!(ti.used_count, 0, "{name}: used_count offset");
            assert_eq!(ti.used_count_threshold, 6, "{name}: 8 - 8/4");
            assert_eq!(ti.tombstone_count_threshold, 1, "{name}: 8/8 + 8/16");
            assert_eq!(
                ti.used_count_shrink_threshold, 0,
                "{name}: forced to 0 when slot_count <= 8"
            );
            assert_eq!(ti.string.mode, mode as u8, "{name}: string.mode offset");
            assert_eq!(
                ti.storage as usize % 64,
                0,
                "{name}: storage must be 64-byte aligned (STBDS_ALIGN_FWD)"
            );
            assert!(
                ti.storage as usize >= t as usize + size_of::<HashIndex>(),
                "{name}: storage must start past the header"
            );
        }
        dq(
            format!("shmode layout mode={mode}"),
            unsafe { table_snapshot(ch, elemsize) },
            unsafe { table_snapshot(rh, elemsize) },
        );
        unsafe {
            (p.c.hmfree_func)(hash_to_arr(ch, elemsize), elemsize);
            (p.rs.hmfree_func)(hash_to_arr(rh, elemsize), elemsize);
        }
    }

    // Cross-check sizeof(stbds_string_block): `stbds_stralloc` returns
    // `sb->storage`, which is at offset 8 (after `next`).
    let mut ca = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
    let mut ra = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
    let mut big: Vec<u8> = vec![b'z'; 4096];
    big.push(0);
    let cp = unsafe { (p.c.stralloc)(&mut ca, big.as_mut_ptr() as *mut c_char) };
    let rp = unsafe { (p.rs.stralloc)(&mut ra, big.as_mut_ptr() as *mut c_char) };
    assert_eq!((cp as usize) - (ca.storage as usize), 8, "C: storage offset");
    assert_eq!((rp as usize) - (ra.storage as usize), 8, "Rust: storage offset");
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.rs.strreset)(&mut ra);
    }
}

/// Allocation *sufficiency*. Every block either library allocates must be big
/// enough for what it then writes into it — an undersized request is a silent
/// heap overflow that the functional comparisons cannot see.
///
/// NOTE: this deliberately checks sufficiency rather than *equality* of
/// `malloc_usable_size`. glibc may serve a request from a larger free chunk
/// without splitting it, so the usable size is not a function of the requested
/// size, and the two libraries' allocation histories differ (they interleave
/// with the test harness's own allocations). Equality would be flaky; the
/// bounds below are exact.
#[test]
fn cfg_46b_allocation_sufficiency() {
    let p = pair();
    p.reseed_default();

    // stbds_arrgrowf: elemsize*capacity + sizeof(stbds_array_header)
    for &elemsize in &[1usize, 3, 4, 8, 16, 20, 64, 100] {
        for &min_cap in &[1usize, 2, 4, 5, 8, 9, 100, 1000] {
            let ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap) };
            let ra = unsafe { (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap) };
            let ctx = format!("arrgrowf elemsize={elemsize} min_cap={min_cap}");
            dq(
                format!("{ctx} / header"),
                unsafe { hdr_snapshot(ca) },
                unsafe { hdr_snapshot(ra) },
            );
            for (name, a) in [("C", ca), ("Rust", ra)] {
                let base = stbds_header_ptr(a);
                let need = HDR_SIZE + elemsize * unsafe { header(a) }.capacity;
                let got = unsafe { usable_size(base) }.unwrap();
                assert!(
                    got >= need,
                    "{name} [{ctx}]: allocated {got} usable bytes but the array needs {need}"
                );
            }
            unsafe {
                (p.c.arrfreef)(ca);
                (p.rs.arrfreef)(ra);
            }
        }
    }

    // stbds_make_hash_index: the 64-byte-aligned `storage` plus
    // (slot_count>>3) buckets must fit inside the block.
    let elemsize = 16usize;
    let keysize = 4usize;
    let mut ch = std::ptr::null_mut();
    let mut rh = std::ptr::null_mut();
    let mut slot_counts = std::collections::BTreeSet::new();
    for i in 0..2000u32 {
        let mut k = i.to_le_bytes();
        ch = unsafe {
            (p.c.hmput_key)(ch, elemsize, k.as_mut_ptr() as *mut c_void, keysize, HM_BINARY)
        };
        rh = unsafe {
            (p.rs.hmput_key)(rh, elemsize, k.as_mut_ptr() as *mut c_void, keysize, HM_BINARY)
        };
        let cs = unsafe { table_snapshot(ch, elemsize) }.unwrap();
        dq(format!("table i={i}"), Some(cs.clone()), unsafe {
            table_snapshot(rh, elemsize)
        });
        slot_counts.insert(cs.slot_count);
        for (name, h) in [("C", ch), ("Rust", rh)] {
            let hdr = unsafe { header(hash_to_arr(h, elemsize)) };
            let t = hdr.hash_table;
            let ti = unsafe { *(t as *const HashIndex) };
            let end = ti.storage as usize + (ti.slot_count >> BUCKET_SHIFT) * 128;
            let got = unsafe { usable_size(t) }.unwrap();
            assert!(
                end <= t as usize + got,
                "{name}: hash index block of {got} usable bytes ends at {:#x} but the \
                 {} buckets end at {end:#x} (slot_count={})",
                t as usize + got,
                ti.slot_count >> BUCKET_SHIFT,
                ti.slot_count
            );
            // the array itself
            let a = hash_to_arr(h, elemsize);
            let need = HDR_SIZE + elemsize * unsafe { header(a) }.capacity;
            let got_a = unsafe { usable_size(stbds_header_ptr(a)) }.unwrap();
            assert!(got_a >= need, "{name}: array block too small ({got_a} < {need})");
        }
    }
    assert!(
        slot_counts.len() >= 9,
        "expected many table sizes, saw {slot_counts:?}"
    );
    unsafe {
        (p.c.hmfree_func)(hash_to_arr(ch, elemsize), elemsize);
        (p.rs.hmfree_func)(hash_to_arr(rh, elemsize), elemsize);
    }

    // stbds_strdup (SH_STRDUP) and stbds_stralloc (SH_ARENA)
    for sh in [SH_STRDUP, SH_ARENA] {
        let mut cm_h = unsafe { (p.c.shmode_func)(elemsize, sh) };
        let mut rm_h = unsafe { (p.rs.shmode_func)(elemsize, sh) };
        for i in 0..400usize {
            let len = 1 + (i * 37) % 900;
            let mut key: Vec<u8> = vec![b'a' + (i % 26) as u8; len];
            key.push(0);
            let kp = key.as_mut_ptr() as *mut c_char;
            cm_h = unsafe { (p.c.hmput_key)(cm_h, elemsize, kp as *mut c_void, 8, HM_STRING) };
            rm_h = unsafe { (p.rs.hmput_key)(rm_h, elemsize, kp as *mut c_void, 8, HM_STRING) };
            let ct = unsafe { header(hash_to_arr(cm_h, elemsize)) };
            let rt = unsafe { header(hash_to_arr(rm_h, elemsize)) };
            dq(format!("sh={sh} i={i} temp"), ct.temp, rt.temp);
            let ckp = unsafe {
                std::ptr::read_unaligned(
                    (cm_h as *mut u8).add(elemsize * ct.temp as usize) as *const *mut c_void,
                )
            };
            let rkp = unsafe {
                std::ptr::read_unaligned(
                    (rm_h as *mut u8).add(elemsize * rt.temp as usize) as *const *mut c_void,
                )
            };
            // The stored key must read back byte-identically in both.
            dq(
                format!("sh={sh} i={i} stored key"),
                unsafe { cstr(ckp as *const c_char) },
                unsafe { cstr(rkp as *const c_char) },
            );
            assert_eq!(unsafe { cstr(ckp as *const c_char) }, key[..len]);
            if sh == SH_STRDUP {
                for (name, p2) in [("C", ckp), ("Rust", rkp)] {
                    let got = unsafe { usable_size(p2) }.unwrap();
                    assert!(
                        got >= len + 1,
                        "{name}: strdup block of {got} bytes cannot hold {} (i={i})",
                        len + 1
                    );
                }
            } else {
                let cti = unsafe { *(ct.hash_table as *const HashIndex) };
                let rti = unsafe { *(rt.hash_table as *const HashIndex) };
                dq(
                    format!("sh=ARENA i={i} remaining/block"),
                    (cti.string.remaining, cti.string.block),
                    (rti.string.remaining, rti.string.block),
                );
                // The returned pointer plus the string must lie inside its block.
                for (name, blk, kptr) in [
                    ("C", cti.string.storage as usize, ckp as usize),
                    ("Rust", rti.string.storage as usize, rkp as usize),
                ] {
                    // The key may live in the head block or in a spliced
                    // oversize block; only check when it is in the head one.
                    let got = unsafe { usable_size(blk as *mut c_void) }.unwrap();
                    if kptr >= blk && kptr < blk + got {
                        assert!(
                            kptr + len + 1 <= blk + got,
                            "{name}: arena string at {kptr:#x}+{} overruns block \
                             [{blk:#x}, {:#x}) (i={i})",
                            len + 1,
                            blk + got
                        );
                    }
                }
            }
        }
        unsafe {
            (p.c.hmfree_func)(hash_to_arr(cm_h, elemsize), elemsize);
            (p.rs.hmfree_func)(hash_to_arr(rm_h, elemsize), elemsize);
        }
    }
}

/// `stbds_header(a)` as a raw `*mut c_void` (the pointer glibc actually owns).
fn stbds_header_ptr(a: *mut c_void) -> *mut c_void {
    (a as *mut u8).wrapping_sub(HDR_SIZE) as *mut c_void
}
