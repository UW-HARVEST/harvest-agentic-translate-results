//! Phase B — differential tests for the leaf-level entry points:
//! `stbds_hash_bytes`, `stbds_hash_string`, `stbds_rand_seed`, `strkey`,
//! `stbds_arrgrowf`, `stbds_arrfreef`, `stbds_stralloc`, `stbds_strreset`,
//! `sh_puts`.
//!
//! Covers CONFIGS.md rows C1–C14, C48–C53 and ERRORS.md rows
//! E22–E32, E44, E45.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

// ===========================================================================
// C1–C7  stbds_hash_bytes
// ===========================================================================

/// C1: `len == 0` with `p == NULL` — `p` must never be dereferenced (E30).
#[test]
fn c1_hash_bytes_len0_null() {
    let s = session(0x31415926);
    for seed in [0usize, 1, 2, 0x31415926, usize::MAX, usize::MAX / 3] {
        unsafe {
            let c = (s.c().hash_bytes)(std::ptr::null_mut(), 0, seed);
            let r = (s.r().hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(c, r, "hash_bytes(NULL,0,{seed:#x})");
        }
    }
}

/// C2/C3/C4/C5/C6/C7 + E31: every tail length, every residue, sign-extension
/// bytes, and randomized seeds.
#[test]
fn c2_c7_hash_bytes_all_lengths_and_seeds() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xC0FFEE_1234);

    // Every length 0..=136 (covers len<8, len==8, all residues, many loops)
    for len in 0usize..=136 {
        for trial in 0..24 {
            let mut buf = rng.bytes(len.max(1));
            // C6: force the sign-extension bytes on some trials.
            if trial % 3 == 0 {
                if len > 3 {
                    buf[3] |= 0x80;
                }
                if len > 7 {
                    buf[7] |= 0x80;
                }
            }
            if trial % 5 == 0 {
                for b in buf.iter_mut() {
                    *b = 0xFF;
                }
            }
            if trial % 7 == 0 {
                for b in buf.iter_mut() {
                    *b = 0x00;
                }
            }
            let seed = match trial {
                0 => 0usize,
                1 => 1,
                2 => usize::MAX,
                3 => 1 << 63,
                _ => rng.next_u64() as usize,
            };
            unsafe {
                let p = buf.as_mut_ptr() as *mut c_void;
                let c = (s.c().hash_bytes)(p, len, seed);
                let r = (s.r().hash_bytes)(p, len, seed);
                assert_eq!(c, r, "hash_bytes(len={len}, seed={seed:#x}, buf={buf:02x?})");
            }
        }
    }

    // C5: larger buffers.
    for len in [137usize, 255, 256, 257, 511, 1000, 4096] {
        for _ in 0..8 {
            let mut buf = rng.bytes(len);
            let seed = rng.next_u64() as usize;
            unsafe {
                let p = buf.as_mut_ptr() as *mut c_void;
                assert_eq!(
                    (s.c().hash_bytes)(p, len, seed),
                    (s.r().hash_bytes)(p, len, seed),
                    "hash_bytes(len={len})"
                );
            }
        }
    }
}

// ===========================================================================
// C8  stbds_hash_string  (+ E32 empty string)
// ===========================================================================

#[test]
fn c8_hash_string() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xBEEF_0001);

    let mut fixed: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"a\0".to_vec(),
        b"ab\0".to_vec(),
        b"test_0\0".to_vec(),
        b"test_1000000\0".to_vec(),
        vec![0x80, 0xFF, 0x7F, 0x01, 0],
        vec![0xFF; 64].into_iter().chain([0]).collect(),
    ];
    for len in [0usize, 1, 2, 3, 7, 8, 9, 15, 16, 31, 63, 64, 200, 1000] {
        for _ in 0..8 {
            fixed.push(rng.cstring(len));
        }
    }

    for mut sbytes in fixed {
        for seed in [
            0usize,
            1,
            0x31415926,
            usize::MAX,
            rng.next_u64() as usize,
            rng.next_u64() as usize,
        ] {
            unsafe {
                let p = sbytes.as_mut_ptr() as *mut c_char;
                let c = (s.c().hash_string)(p, seed);
                let r = (s.r().hash_string)(p, seed);
                assert_eq!(c, r, "hash_string(len={}, seed={seed:#x})", sbytes.len() - 1);
            }
        }
    }
}

// ===========================================================================
// C9  stbds_rand_seed + the seed LCG advance, observed through table->seed
// ===========================================================================

#[test]
fn c9_rand_seed_lcg_advance() {
    for start in [
        0usize,
        1,
        0x31415926,
        usize::MAX,
        0xDEAD_BEEF_CAFE_BABE,
        1 << 63,
    ] {
        let s = session(start);
        // Each shmode_func -> make_hash_index(8, NULL) consumes one LCG step,
        // storing the pre-advance seed in table->seed.
        let mut cseeds = Vec::new();
        let mut rseeds = Vec::new();
        unsafe {
            for _ in 0..40 {
                let ct = (s.c().shmode_func)(16, SH_ARENA);
                let rt = (s.r().shmode_func)(16, SH_ARENA);
                let cm = Map { t: ct, elemsize: 16 };
                let rm = Map { t: rt, elemsize: 16 };
                cseeds.push((*cm.table()).seed);
                rseeds.push((*rm.table()).seed);
                (s.c().hmfree_func)(cm.raw(), 16);
                (s.r().hmfree_func)(rm.raw(), 16);
            }
        }
        assert_eq!(cseeds, rseeds, "seed LCG chain from {start:#x}");
        assert_eq!(cseeds[0], start, "first table must use the seeded value");
        // Sanity: the chain actually moves.
        assert_ne!(cseeds[0], cseeds[1]);
    }
}

// ===========================================================================
// C10 / E45  strkey
// ===========================================================================

#[test]
fn c10_strkey() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0x5EED_0010);
    let mut ns: Vec<c_int> = vec![
        0,
        1,
        -1,
        9,
        10,
        -9,
        -10,
        99,
        100,
        -100,
        999,
        1000,
        12345,
        -12345,
        c_int::MAX,
        c_int::MIN,
        c_int::MIN + 1,
    ];
    for _ in 0..500 {
        ns.push(rng.next_u64() as c_int);
    }
    unsafe {
        let mut cbuf: Option<usize> = None;
        let mut rbuf: Option<usize> = None;
        for n in ns {
            let cp = (s.c().strkey)(n);
            let rp = (s.r().strkey)(n);
            let cb = cstr_bytes(cp);
            let rb = cstr_bytes(rp);
            assert_eq!(
                cb,
                rb,
                "strkey({n}): C={:?} Rust={:?}",
                String::from_utf8_lossy(&cb),
                String::from_utf8_lossy(&rb)
            );
            assert_eq!(cb, format!("test_{n}").into_bytes(), "strkey({n}) content");
            // Both must keep returning the same static buffer.
            let cu = cp as usize;
            let ru = rp as usize;
            assert_eq!(*cbuf.get_or_insert(cu), cu, "C strkey buffer moved");
            assert_eq!(*rbuf.get_or_insert(ru), ru, "Rust strkey buffer moved");
        }
    }
}

// ===========================================================================
// C11–C14 / E26–E29  stbds_arrgrowf & stbds_arrfreef
// ===========================================================================

unsafe fn arr_state(a: *mut c_void) -> (bool, usize, usize, isize, bool) {
    if a.is_null() {
        return (true, 0, 0, 0, false);
    }
    let h = (a as *mut ArrayHeader).wrapping_sub(1);
    (
        false,
        (*h).length,
        (*h).capacity,
        (*h).temp,
        (*h).hash_table.is_null(),
    )
}

/// C11 + E28 (NULL/0 no-op) + E27 (min_cap<4 clamp) + E29 (elemsize 0).
#[test]
fn c11_arrgrowf_from_null() {
    let s = session(0x31415926);
    for elemsize in [0usize, 1, 2, 4, 8, 12, 16, 24, 33, 40] {
        for addlen in [0usize, 1, 2, 3, 4, 5, 8, 17, 100] {
            for min_cap in [0usize, 1, 2, 3, 4, 5, 8, 17, 100] {
                unsafe {
                    let ca = (s.c().arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let ra = (s.r().arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let cs = arr_state(ca);
                    let rs = arr_state(ra);
                    assert_eq!(
                        cs, rs,
                        "arrgrowf(NULL, es={elemsize}, addlen={addlen}, min_cap={min_cap})"
                    );
                    // E28: both must decline to allocate for (0,0).
                    if addlen == 0 && min_cap == 0 {
                        assert!(ca.is_null() && ra.is_null(), "E28: expected NULL");
                    } else {
                        assert!(!ca.is_null() && !ra.is_null());
                        // E27: fresh array clamps to at least 4.
                        assert!(cs.2 >= 4.min(cs.2.max(4)));
                        (s.c().arrfreef)(ca);
                        (s.r().arrfreef)(ra);
                    }
                }
            }
        }
    }
}

/// C12 + C13 + E26: repeated growth (doubling clamp) and the already-fits no-op.
#[test]
fn c12_arrgrowf_repeated_growth() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xA11C_0012);
    for elemsize in [1usize, 4, 8, 16, 24] {
        for _round in 0..6 {
            unsafe {
                let mut ca = (s.c().arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
                let mut ra = (s.r().arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
                assert_eq!(arr_state(ca), arr_state(ra), "initial grow es={elemsize}");
                // 12 steps keeps the doubling within a few MB; stb_ds does not
                // check `realloc` for NULL, so exhausting memory would just
                // crash both libraries identically (untestable in-process).
                for step in 0..12 {
                    // Drive length forward like arrput does, then grow again.
                    let h = (ca as *mut ArrayHeader).wrapping_sub(1);
                    let cap = (*h).capacity;
                    (*h).length = cap;
                    let hr = (ra as *mut ArrayHeader).wrapping_sub(1);
                    (*hr).length = cap;

                    let (addlen, min_cap) = match step % 5 {
                        0 => (1usize, 0usize),
                        1 => (0, cap),           // E26: already fits -> no-op
                        2 => (0, cap + 1),
                        3 => (rng.range(1, 9), 0),
                        _ => (0, rng.range(1, 4 * cap + 8)),
                    };
                    let cbefore = ca;
                    let rbefore = ra;
                    // E26: the function returns `a` untouched iff
                    // max(min_cap, length+addlen) <= capacity.  Outside that
                    // case a realloc happens and may legitimately return the
                    // same address, so pointer identity is not comparable.
                    let effective = min_cap.max(cap + addlen);
                    ca = (s.c().arrgrowf)(ca, elemsize, addlen, min_cap);
                    ra = (s.r().arrgrowf)(ra, elemsize, addlen, min_cap);
                    if effective <= cap {
                        assert!(
                            ca == cbefore && ra == rbefore,
                            "E26: expected no-op (es={elemsize} step={step} addlen={addlen} min_cap={min_cap})"
                        );
                    }
                    assert_eq!(
                        arr_state(ca),
                        arr_state(ra),
                        "arrgrowf state differs (es={elemsize} step={step} addlen={addlen} min_cap={min_cap})"
                    );
                }
                (s.c().arrfreef)(ca);
                (s.r().arrfreef)(ra);
            }
        }
    }
}

/// C14: payload survives realloc identically.
#[test]
fn c14_arrgrowf_payload_preserved() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0xA11C_0014);
    let elemsize = 8usize;
    unsafe {
        let mut ca = (s.c().arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
        let mut ra = (s.r().arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
        let mut written: Vec<u64> = Vec::new();
        for i in 0..300usize {
            let v = rng.next_u64();
            let ch = (ca as *mut ArrayHeader).wrapping_sub(1);
            let _rh = (ra as *mut ArrayHeader).wrapping_sub(1);
            if (*ch).length + 1 > (*ch).capacity {
                ca = (s.c().arrgrowf)(ca, elemsize, 1, 0);
                ra = (s.r().arrgrowf)(ra, elemsize, 1, 0);
            }
            let ch = (ca as *mut ArrayHeader).wrapping_sub(1);
            let rh = (ra as *mut ArrayHeader).wrapping_sub(1);
            *((ca as *mut u64).add(i)) = v;
            *((ra as *mut u64).add(i)) = v;
            (*ch).length = i + 1;
            (*rh).length = i + 1;
            written.push(v);
            assert_eq!((*ch).capacity, (*rh).capacity, "capacity at i={i}");
        }
        for (i, v) in written.iter().enumerate() {
            assert_eq!(*((ca as *mut u64).add(i)), *v);
            assert_eq!(*((ra as *mut u64).add(i)), *v);
        }
        (s.c().arrfreef)(ca);
        (s.r().arrfreef)(ra);
    }
}

// ===========================================================================
// C48–C52 / E22–E25  stbds_stralloc & stbds_strreset
// ===========================================================================

unsafe fn chain_len(a: &StringArena) -> usize {
    let mut n = 0usize;
    let mut x = a.storage;
    while !x.is_null() {
        n += 1;
        x = (*x).next;
        assert!(n < 100_000, "cycle in arena block chain");
    }
    n
}

/// Mirror of the C branch decision, so the offset of the returned pointer can
/// be checked against a known block base on *both* code paths.
fn stralloc_expect_dedicated(a: &StringArena, len: usize) -> bool {
    if len <= a.remaining {
        return false;
    }
    let blocksize = 512usize << ((a.block as usize >> 1) & 63);
    len > blocksize
}

struct ArenaPair {
    c: StringArena,
    r: StringArena,
}

impl ArenaPair {
    fn new() -> ArenaPair {
        ArenaPair {
            c: StringArena::zeroed(),
            r: StringArena::zeroed(),
        }
    }

    unsafe fn alloc(&mut self, s: &Session, text: &mut Vec<u8>, ctx: &str) {
        let len = text.len(); // includes NUL
        let dedicated_c = stralloc_expect_dedicated(&self.c, len);
        let dedicated_r = stralloc_expect_dedicated(&self.r, len);
        assert_eq!(
            dedicated_c, dedicated_r,
            "{ctx}: predicted branch differs (arena state diverged)"
        );
        let before_c = self.c.storage;
        let before_r = self.r.storage;

        let cp = (s.c().stralloc)(&mut self.c, text.as_mut_ptr() as *mut c_char);
        let rp = (s.r().stralloc)(&mut self.r, text.as_mut_ptr() as *mut c_char);

        assert!(!cp.is_null() && !rp.is_null(), "{ctx}: NULL return");
        let cb = cstr_bytes(cp);
        let rb = cstr_bytes(rp);
        assert_eq!(cb, &text[..len - 1], "{ctx}: C stored the wrong bytes");
        assert_eq!(rb, &text[..len - 1], "{ctx}: Rust stored the wrong bytes");

        assert_eq!(
            self.c.remaining, self.r.remaining,
            "{ctx}: arena.remaining differs"
        );
        assert_eq!(self.c.block, self.r.block, "{ctx}: arena.block differs");
        assert_eq!(self.c.mode, self.r.mode, "{ctx}: arena.mode differs");
        assert_eq!(
            chain_len(&self.c),
            chain_len(&self.r),
            "{ctx}: block-chain length differs"
        );
        assert_eq!(
            self.c.storage == before_c,
            self.r.storage == before_r,
            "{ctx}: head-of-chain replacement differs"
        );

        // Offset of the returned pointer inside its block must match exactly.
        let (cbase, rbase) = if dedicated_c {
            if before_c.is_null() {
                (self.c.storage as usize, self.r.storage as usize)
            } else {
                (
                    (*self.c.storage).next as usize,
                    (*self.r.storage).next as usize,
                )
            }
        } else {
            (self.c.storage as usize, self.r.storage as usize)
        };
        assert_eq!(
            cp as usize - cbase,
            rp as usize - rbase,
            "{ctx}: offset within block differs (dedicated={dedicated_c})"
        );
        if dedicated_c {
            assert_eq!(cp as usize - cbase, 8, "{ctx}: E23 dedicated block offset");
        }
    }

    unsafe fn reset(&mut self, s: &Session, ctx: &str) {
        (s.c().strreset)(&mut self.c);
        (s.r().strreset)(&mut self.r);
        assert_eq!(self.c.remaining, self.r.remaining, "{ctx}: reset remaining");
        assert_eq!(self.c.block, self.r.block, "{ctx}: reset block");
        assert_eq!(self.c.mode, self.r.mode, "{ctx}: reset mode");
        assert!(self.c.storage.is_null() && self.r.storage.is_null(), "{ctx}");
    }
}

/// C48 + E22: boundary lengths on a fresh arena.
#[test]
fn c48_stralloc_boundary_lengths() {
    let s = session(0x31415926);
    for content_len in [0usize, 1, 2, 15, 16, 510, 511, 512, 513, 1023, 1024, 1025] {
        unsafe {
            let mut ap = ArenaPair::new();
            let mut text: Vec<u8> = vec![b'x'; content_len];
            text.push(0);
            ap.alloc(&s, &mut text, &format!("fresh len={content_len}"));
            ap.reset(&s, "after fresh");
        }
    }
}

/// C49 + E24: `block` must saturate at 22 and never overflow the shift.
#[test]
fn c49_stralloc_block_saturation() {
    let s = session(0x31415926);
    unsafe {
        let mut ap = ArenaPair::new();
        // Each allocation of ~half a block forces frequent new blocks.
        for i in 0..4000usize {
            let n = 400 + (i % 7);
            let mut text: Vec<u8> = vec![b'a' + (i % 26) as u8; n];
            text.push(0);
            ap.alloc(&s, &mut text, &format!("saturate i={i}"));
        }
        assert_eq!(ap.c.block, ap.r.block);
        assert_eq!(ap.c.block, 22, "E24: block must saturate at 22");
        ap.reset(&s, "after saturation");
        assert_eq!(ap.c.block, 0, "E25: reset zeroes block");
    }
}

/// C50 + E23: the dedicated-block path, on an empty and on a non-empty arena.
#[test]
fn c50_stralloc_dedicated_block() {
    let s = session(0x31415926);
    unsafe {
        // empty arena, huge string
        let mut ap = ArenaPair::new();
        let mut big: Vec<u8> = vec![b'Z'; 100_000];
        big.push(0);
        ap.alloc(&s, &mut big, "dedicated/empty");
        assert_eq!(ap.c.remaining, 0, "E23: remaining stays 0");
        // now a small one goes into a fresh regular block
        let mut small = b"hi\0".to_vec();
        ap.alloc(&s, &mut small, "small after dedicated");
        // and another huge one, this time with existing storage
        let mut big2: Vec<u8> = vec![b'Y'; 70_000];
        big2.push(0);
        ap.alloc(&s, &mut big2, "dedicated/non-empty");
        ap.reset(&s, "after dedicated");
    }
}

/// C51: randomized size mix, thousands of calls.
#[test]
fn c51_stralloc_random_mix() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0x5712_A110);
    unsafe {
        for round in 0..6 {
            let mut ap = ArenaPair::new();
            for i in 0..900usize {
                let n = match rng.below(10) {
                    0 => 0,
                    1 => rng.range(1, 8),
                    2 => rng.range(500, 520),
                    3 => rng.range(1000, 1100),
                    4 => rng.range(4090, 4100),
                    5 => rng.range(60_000, 70_000),
                    _ => rng.range(1, 400),
                };
                let mut text = rng.cstring(n);
                ap.alloc(&s, &mut text, &format!("round={round} i={i} n={n}"));
            }
            ap.reset(&s, &format!("round={round}"));
        }
    }
}

/// C52 + E25: reset on a virgin arena, and twice in a row.
#[test]
fn c52_strreset_idempotent() {
    let s = session(0x31415926);
    unsafe {
        let mut ap = ArenaPair::new();
        ap.reset(&s, "virgin");
        ap.reset(&s, "virgin twice");
        let mut t = b"abc\0".to_vec();
        ap.alloc(&s, &mut t, "one alloc");
        ap.reset(&s, "after one");
        ap.reset(&s, "after one, twice");
    }
}

// ===========================================================================
// C53 / E21 / E44  sh_puts  — stdout compared byte-for-byte
// ===========================================================================

/// Child-process runner used by `c53_sh_puts_stdout`; inert during a normal run.
#[test]
#[ignore]
fn sh_puts_child_runner() {
    common::sh_puts_child_main();
}

#[test]
fn c53_sh_puts_stdout() {
    let mut nums: Vec<c_int> = vec![
        0,
        1,
        2,
        3,
        7,
        8,
        9,
        100,
        1000,
        5000,
        -1,
        -2,
        -1000,
        c_int::MIN,
        c_int::MIN + 1,
    ];
    let mut rng = Rng::new(0x5450_0053);
    for _ in 0..12 {
        nums.push(rng.range(0, 300) as c_int);
    }
    for n in nums {
        let cout = sh_puts_stdout(n, "c", "sh_puts_child_runner");
        let rout = sh_puts_stdout(n, "rust", "sh_puts_child_runner");
        assert_eq!(
            String::from_utf8_lossy(&cout),
            String::from_utf8_lossy(&rout),
            "sh_puts({n}) stdout differs"
        );
        assert_eq!(
            cout,
            format!("a {n}\n").into_bytes(),
            "sh_puts({n}) unexpected output"
        );
    }
}
