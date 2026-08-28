//! Phase B — CONFIGS.md rows C1..C13: the leaf/pure entry points.
//!
//! `stbds_hash_bytes` (siphash-2-4), `stbds_hash_string`, `stbds_rand_seed`'s
//! global-seed ladder, and `strkey`.  Every row uses many randomized inputs from
//! a fixed-seed PRNG.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

const SEEDS: &[usize] = &[
    0,
    1,
    2,
    DEFAULT_SEED,
    usize::MAX,
    usize::MAX - 1,
    0x8000_0000_0000_0000,
    0x0000_0000_8000_0000,
    0xdead_beef_cafe_babe,
];

fn hb(l: &Lib, buf: &[u8], len: usize, seed: usize) -> usize {
    unsafe { (l.hash_bytes)(buf.as_ptr() as *mut c_void, len, seed) }
}

// ---------------------------------------------------------------------------
// C1 — len == 0, p == NULL (must not dereference)
// ---------------------------------------------------------------------------
#[test]
fn c1_hash_bytes_len0_null() {
    let _g = lock();
    let (c, r) = both();
    for &seed in SEEDS {
        unsafe {
            let cv = (c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let rv = (r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(cv, rv, "hash_bytes(NULL, 0, {seed:#x})");
        }
    }
    // also non-NULL pointer with len 0
    let buf = [0xAAu8; 8];
    for &seed in SEEDS {
        assert_eq!(hb(c, &buf, 0, seed), hb(r, &buf, 0, seed));
    }
}

// ---------------------------------------------------------------------------
// C2 — len 1..7: every `switch (len - i)` fall-through case
// C3 — same, with d[3] >= 0x80 (negative-int sign-extension into size_t)
// ---------------------------------------------------------------------------
#[test]
fn c2_c3_hash_bytes_tail_1_to_7() {
    let _g = lock();
    let (c, r) = both();
    let mut rng = Rng::new(0xC2C3_0001);

    // exhaustive-ish sweep over the interesting byte values at each position
    for len in 1..=7usize {
        for _ in 0..400 {
            let mut buf = rng.bytes(8);
            // C3: force the high bit of d[3] (and d[6]) to exercise both the
            // `(d[3] << 24)` int-overflow path and the `(size_t)d[6] << 48` path.
            if rng.next_u64() & 1 == 0 {
                if len > 3 {
                    buf[3] |= 0x80;
                }
                if len > 6 {
                    buf[6] |= 0x80;
                }
            }
            for &seed in SEEDS {
                assert_eq!(
                    hb(c, &buf, len, seed),
                    hb(r, &buf, len, seed),
                    "len={len} seed={seed:#x} buf={buf:02x?}"
                );
            }
        }
    }

    // Deterministic corner values at every tail position.
    for len in 1..=7usize {
        for pos in 0..len {
            for &v in &[0x00u8, 0x01, 0x7f, 0x80, 0xfe, 0xff] {
                let mut buf = vec![0u8; 8];
                buf[pos] = v;
                for &seed in SEEDS {
                    assert_eq!(
                        hb(c, &buf, len, seed),
                        hb(r, &buf, len, seed),
                        "len={len} pos={pos} v={v:#x} seed={seed:#x}"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C4/C5/C6/C7 — the block loop: len 8, 9..15, 16,17,31,32,33,64,255
// ---------------------------------------------------------------------------
#[test]
fn c4_c7_hash_bytes_block_loop() {
    let _g = lock();
    let (c, r) = both();
    let mut rng = Rng::new(0xC4C7_0002);

    let lens: Vec<usize> = (8..=17)
        .chain([23, 24, 25, 31, 32, 33, 47, 48, 63, 64, 65, 127, 128, 255, 256])
        .collect();

    for &len in &lens {
        for _ in 0..120 {
            let mut buf = rng.bytes(len + 8);
            // C7: sign-extension sites *inside* the block loop.
            match rng.below(4) {
                0 => {}
                1 => {
                    for i in (3..len).step_by(8) {
                        buf[i] |= 0x80;
                    }
                }
                2 => {
                    for i in (7..len).step_by(8) {
                        buf[i] |= 0x80;
                    }
                }
                _ => {
                    for b in buf.iter_mut() {
                        *b = 0xff;
                    }
                }
            }
            for &seed in SEEDS {
                assert_eq!(
                    hb(c, &buf, len, seed),
                    hb(r, &buf, len, seed),
                    "len={len} seed={seed:#x}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C8 — unaligned p
// ---------------------------------------------------------------------------
#[test]
fn c8_hash_bytes_unaligned() {
    let _g = lock();
    let (c, r) = both();
    let mut rng = Rng::new(0xC800_0003);
    let backing = rng.bytes(512);
    for off in 0..16usize {
        for len in 0..64usize {
            let p = unsafe { backing.as_ptr().add(off) } as *mut c_void;
            for &seed in &[0usize, DEFAULT_SEED, usize::MAX] {
                unsafe {
                    assert_eq!(
                        (c.hash_bytes)(p, len, seed),
                        (r.hash_bytes)(p, len, seed),
                        "off={off} len={len}"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C9 — `data = len << (64-8)` truncation for len with bits above 8
// ---------------------------------------------------------------------------
#[test]
fn c9_hash_bytes_len_shift_truncation() {
    let _g = lock();
    let (c, r) = both();
    let mut rng = Rng::new(0xC900_0004);
    let buf = rng.bytes(4096);
    for &len in &[
        255usize, 256, 257, 258, 263, 264, 511, 512, 513, 1000, 1023, 1024, 1025, 2048, 4095, 4096,
    ] {
        for &seed in SEEDS {
            assert_eq!(hb(c, &buf, len, seed), hb(r, &buf, len, seed), "len={len}");
        }
    }
}

// ---------------------------------------------------------------------------
// C10/C11 — stbds_hash_string
// ---------------------------------------------------------------------------
#[test]
fn c10_c11_hash_string() {
    let _g = lock();
    let (c, r) = both();
    let mut rng = Rng::new(0xC0A0_0005);

    let mut fixed: Vec<Vec<u8>> = vec![
        b"\0".to_vec(),
        b"a\0".to_vec(),
        b"ab\0".to_vec(),
        b"abcdefg\0".to_vec(),
        b"abcdefgh\0".to_vec(),
        b"abcdefghi\0".to_vec(),
        b"test_0\0".to_vec(),
        b"test_2147483647\0".to_vec(),
        b"\x80\0".to_vec(),
        b"\xff\xff\xff\xff\xff\xff\xff\xff\0".to_vec(),
    ];
    // random strings of many lengths, incl. bytes >= 0x80 (the
    // `(unsigned char) *str` cast) and a 1000-char one.
    for len in [0usize, 1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 63, 64, 127, 255, 1000] {
        for _ in 0..40 {
            fixed.push(rng.cstring(len));
        }
    }

    for s in &fixed {
        for &seed in SEEDS {
            unsafe {
                let cv = (c.hash_string)(s.as_ptr() as *mut c_char, seed);
                let rv = (r.hash_string)(s.as_ptr() as *mut c_char, seed);
                assert_eq!(cv, rv, "hash_string({:?}, {seed:#x})", show(s));
            }
        }
    }

    // random seeds too
    for _ in 0..2000 {
        let n = (rng.next_u64() % 40) as usize;
        let s = rng.cstring(n);
        let seed = rng.next_u64() as usize;
        unsafe {
            assert_eq!(
                (c.hash_string)(s.as_ptr() as *mut c_char, seed),
                (r.hash_string)(s.as_ptr() as *mut c_char, seed)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C12 — the global seed ladder: seed_{n+1} = seed_n * 0x27bb2ee687b0b0fd
//                                          + 0xb504f32d
// Observed through `table->seed` of consecutive fresh tables.
// ---------------------------------------------------------------------------
#[test]
fn c12_global_seed_ladder() {
    let _g = lock();
    let (c, r) = both();
    for &start in &[0usize, 1, DEFAULT_SEED, usize::MAX, 0x1234_5678_9abc_def0] {
        sync_seed(start);
        let mut cseeds = Vec::new();
        let mut rseeds = Vec::new();
        unsafe {
            for _ in 0..40 {
                let ct = (c.shmode_func)(16, SH_NONE);
                let rt = (r.shmode_func)(16, SH_NONE);
                cseeds
                    .push((*((*header(hash_to_arr(ct, 16))).hash_table as *mut HashIndex)).seed);
                rseeds
                    .push((*((*header(hash_to_arr(rt, 16))).hash_table as *mut HashIndex)).seed);
                (c.hmfree_func)(hash_to_arr(ct, 16), 16);
                (r.hmfree_func)(hash_to_arr(rt, 16), 16);
            }
        }
        assert_eq!(cseeds, rseeds, "seed ladder from {start:#x}");
        // and it really is the documented LCG
        let a: usize = 0x27bb_2ee6_87b0_b0fd;
        let b: usize = 0xb504_f32d;
        let mut expect = start;
        for (i, &got) in cseeds.iter().enumerate() {
            assert_eq!(got, expect, "ladder step {i}");
            expect = expect.wrapping_mul(a).wrapping_add(b);
        }
    }
    sync_seed(DEFAULT_SEED);
}

// ---------------------------------------------------------------------------
// C13 — strkey
// ---------------------------------------------------------------------------
#[test]
fn c13_strkey() {
    let _g = lock();
    let (c, r) = both();
    let mut vals: Vec<i32> = vec![
        0,
        1,
        -1,
        9,
        10,
        -9,
        -10,
        99,
        100,
        -99,
        -100,
        999,
        1000,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        -2147483647,
    ];
    let mut rng = Rng::new(0xC130_0006);
    for _ in 0..3000 {
        vals.push(rng.next_u64() as i32);
    }
    for &n in &vals {
        unsafe {
            let cp = (c.strkey)(n);
            let cs = cstr_opt(cp);
            let rp = (r.strkey)(n);
            let rs = cstr_opt(rp);
            assert_eq!(cs, rs, "strkey({n})");
            assert_eq!(cs, format!("{:?}", format!("test_{n}")), "strkey({n}) value");
            // the returned pointer must be stable (a static buffer)
            assert_eq!(cp, (c.strkey)(n));
            assert_eq!(rp, (r.strkey)(n));
        }
    }
}
