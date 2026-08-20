//! Phase B — CONFIGS.md rows 9-19, 69, 70, 74.
//! Hash functions, the global seed LCG, `strkey` and `arr_ins`.
mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const SEEDS: [usize; 8] = [
    0,
    1,
    2,
    0x3141_5926,
    usize::MAX,
    usize::MAX - 1,
    0x8000_0000_0000_0000,
    0x0102_0304_0506_0708,
];

fn both_hash_bytes(s: &Session, buf: &mut [u8], len: usize, seed: usize) -> (usize, usize) {
    unsafe {
        let p = buf.as_mut_ptr() as *mut c_void;
        ((s.c.hash_bytes)(p, len, seed), (s.rust.hash_bytes)(p, len, seed))
    }
}

fn both_hash_string(s: &Session, buf: &mut [u8], seed: usize) -> (usize, usize) {
    unsafe {
        let p = buf.as_mut_ptr() as *mut c_char;
        (
            (s.c.hash_string)(p, seed),
            (s.rust.hash_string)(p, seed),
        )
    }
}

// --- row 9: hash_bytes, len == 0 -------------------------------------------
#[test]
fn cfg_09_hash_bytes_zero_length() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 9);
    for &seed in SEEDS.iter() {
        let mut buf = rng.bytes(64);
        let (a, b) = both_hash_bytes(&s, &mut buf, 0, seed);
        assert_eq!(a, b, "hash_bytes(len=0, seed={:#x})", seed);
    }
    for _ in 0..500 {
        let seed = rng.next_usize();
        let mut buf = rng.bytes(64);
        let (a, b) = both_hash_bytes(&s, &mut buf, 0, seed);
        assert_eq!(a, b, "hash_bytes(len=0, seed={:#x})", seed);
    }
}

// --- row 10: hash_bytes, len 1..7 (every switch fall-through) --------------
#[test]
fn cfg_10_hash_bytes_short_lengths() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 10);
    for len in 1..=7usize {
        for &seed in SEEDS.iter() {
            for _ in 0..200 {
                let mut buf = rng.bytes(len.max(8));
                let (a, b) = both_hash_bytes(&s, &mut buf, len, seed);
                assert_eq!(a, b, "hash_bytes(len={}, seed={:#x}) buf={:?}", len, seed, buf);
            }
        }
    }
}

// --- row 11: hash_bytes, len == 8 exactly ---------------------------------
#[test]
fn cfg_11_hash_bytes_exactly_one_block() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 11);
    for &seed in SEEDS.iter() {
        for _ in 0..500 {
            let mut buf = rng.bytes(8);
            let (a, b) = both_hash_bytes(&s, &mut buf, 8, seed);
            assert_eq!(a, b, "hash_bytes(len=8, seed={:#x}) buf={:?}", seed, buf);
        }
    }
}

// --- row 12: hash_bytes, len 9..71 (blocks x remainders) ------------------
#[test]
fn cfg_12_hash_bytes_blocks_and_remainders() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 12);
    for len in 9..=71usize {
        for _ in 0..40 {
            let seed = rng.next_usize();
            let mut buf = rng.bytes(len);
            let (a, b) = both_hash_bytes(&s, &mut buf, len, seed);
            assert_eq!(a, b, "hash_bytes(len={}, seed={:#x})", len, seed);
        }
    }
}

// --- row 13: sign-extension traps ----------------------------------------
#[test]
fn cfg_13_hash_bytes_sign_extension_patterns() {
    let s = session();
    let patterns: [u8; 6] = [0x00, 0xFF, 0x80, 0x7F, 0x81, 0xAA];
    for len in 0..=40usize {
        for &p in patterns.iter() {
            for &seed in SEEDS.iter() {
                let mut buf = vec![p; len.max(8)];
                let (a, b) = both_hash_bytes(&s, &mut buf, len, seed);
                assert_eq!(
                    a, b,
                    "hash_bytes(len={}, fill={:#02x}, seed={:#x})",
                    len, p, seed
                );
            }
        }
    }
    // one 0x80 byte at every position of an 8-byte block
    for pos in 0..8usize {
        for &seed in SEEDS.iter() {
            let mut buf = vec![0u8; 16];
            buf[pos] = 0x80;
            buf[pos + 8] = 0xFF;
            for len in 0..=16usize {
                let (a, b) = both_hash_bytes(&s, &mut buf, len, seed);
                assert_eq!(a, b, "hash_bytes(len={}, hi at {}, seed={:#x})", len, pos, seed);
            }
        }
    }
}

// --- row 14: large buffers -----------------------------------------------
#[test]
fn cfg_14_hash_bytes_large_buffers() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 14);
    for len in [255usize, 256, 257, 1023, 1024, 4096, 4097] {
        for _ in 0..20 {
            let seed = rng.next_usize();
            let mut buf = rng.bytes(len);
            let (a, b) = both_hash_bytes(&s, &mut buf, len, seed);
            assert_eq!(a, b, "hash_bytes(len={}, seed={:#x})", len, seed);
        }
    }
}

// --- row 15/17: hash_string, ASCII ---------------------------------------
#[test]
fn cfg_15_hash_string_ascii() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 15);
    // empty string
    for &seed in SEEDS.iter() {
        let mut e = vec![0u8; 1];
        let (a, b) = both_hash_string(&s, &mut e, seed);
        assert_eq!(a, b, "hash_string(\"\", {:#x})", seed);
    }
    for len in 1..=64usize {
        for _ in 0..30 {
            let seed = rng.next_usize();
            let mut buf = rng.cstring(len);
            let (a, b) = both_hash_string(&s, &mut buf, seed);
            assert_eq!(a, b, "hash_string(len={}, seed={:#x})", len, seed);
        }
    }
    for len in [256usize, 1024, 4096] {
        for &seed in SEEDS.iter() {
            let mut buf = rng.cstring(len);
            let (a, b) = both_hash_string(&s, &mut buf, seed);
            assert_eq!(a, b, "hash_string(len={}, seed={:#x})", len, seed);
        }
    }
}

// --- row 16: hash_string, high-bit bytes ---------------------------------
#[test]
fn cfg_16_hash_string_high_bit_bytes() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 16);
    // every single byte value 1..=255 as a 1-char string
    for byte in 1u8..=255 {
        for &seed in SEEDS.iter() {
            let mut buf = vec![byte, 0];
            let (a, b) = both_hash_string(&s, &mut buf, seed);
            assert_eq!(a, b, "hash_string([{:#02x}], {:#x})", byte, seed);
        }
    }
    for len in 1..=48usize {
        for _ in 0..30 {
            let seed = rng.next_usize();
            let mut buf = rng.cstring_full(len);
            let (a, b) = both_hash_string(&s, &mut buf, seed);
            assert_eq!(a, b, "hash_string(full-range len={}, seed={:#x})", len, seed);
        }
    }
    // control characters
    for len in 1..=16usize {
        let mut buf: Vec<u8> = (0..len).map(|i| (i as u8 % 31) + 1).collect();
        buf.push(0);
        for &seed in SEEDS.iter() {
            let (a, b) = both_hash_string(&s, &mut buf, seed);
            assert_eq!(a, b, "hash_string(ctl len={}, {:#x})", len, seed);
        }
    }
}

// --- rows 18 / 74: rand_seed + the private hash_seed LCG -----------------
#[test]
fn cfg_18_74_rand_seed_and_lcg_lockstep() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 18);
    let lay = L_I2I;

    let mut seeds: Vec<usize> = vec![0, 1, 2, usize::MAX, INITIAL_HASH_SEED];
    for _ in 0..40 {
        seeds.push(rng.next_usize());
    }

    for &sd in seeds.iter() {
        unsafe {
            (s.c.rand_seed)(sd);
            (s.rust.rand_seed)(sd);
            // create 8 fresh indices in a row: index #0 gets `sd`, then the
            // private LCG must advance identically in both libraries.
            let mut cs = String::new();
            let mut rs = String::new();
            let mut cps = Vec::new();
            let mut rps = Vec::new();
            for _ in 0..8 {
                let cp = (s.c.shmode_func)(lay.elemsize, SH_ARENA);
                let rp = (s.rust.shmode_func)(lay.elemsize, SH_ARENA);
                cs.push_str(&dump_map(cp, DumpOpts::strptr(lay.elemsize)));
                rs.push_str(&dump_map(rp, DumpOpts::strptr(lay.elemsize)));
                cps.push(cp);
                rps.push(rp);
            }
            assert_same(&format!("rand_seed({:#x}) -> 8 fresh indices", sd), &cs, &rs);
            for (cp, rp) in cps.into_iter().zip(rps) {
                map_free(s.c, cp, lay);
                map_free(s.rust, rp, lay);
            }
        }
    }
}

// --- row 19: seed feeds the bucket layout --------------------------------
#[test]
fn cfg_19_seed_drives_bucket_layout() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 19);
    let lay = L_I2I;

    for trial in 0..30 {
        let sd = if trial == 0 { 0 } else { rng.next_usize() };
        unsafe {
            (s.c.rand_seed)(sd);
            (s.rust.rand_seed)(sd);
            let mut cp: *mut c_void = std::ptr::null_mut();
            let mut rp: *mut c_void = std::ptr::null_mut();
            for i in 0..25i32 {
                let key = i.wrapping_mul(7).wrapping_add(3).to_ne_bytes();
                let val = (i as u32 ^ 0xDEAD).to_ne_bytes();
                cp = map_put_binary(s.c, cp, lay, &key, &val, HM_BINARY);
                rp = map_put_binary(s.rust, rp, lay, &key, &val, HM_BINARY);
                assert_same(
                    &format!("seed={:#x} after put #{}", sd, i),
                    &dump_map(cp, DumpOpts::raw(lay.elemsize)),
                    &dump_map(rp, DumpOpts::raw(lay.elemsize)),
                );
            }
            map_free(s.c, cp, lay);
            map_free(s.rust, rp, lay);
        }
    }
}

// --- row 69: strkey ------------------------------------------------------
#[test]
fn cfg_69_strkey_full_buffer() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 69);

    let mut inputs: Vec<c_int> = vec![
        0,
        1,
        -1,
        9,
        10,
        11,
        99,
        100,
        101,
        999,
        1000,
        12345,
        -12345,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        -2147483647,
    ];
    for _ in 0..500 {
        inputs.push(rng.next_u32() as i32);
    }
    for n in inputs {
        unsafe {
            let cp = (s.c.strkey)(n);
            let rp = (s.rust.strkey)(n);
            assert!(!cp.is_null() && !rp.is_null());
            // compare the whole 256-byte static buffer, not just the string
            let cb = std::slice::from_raw_parts(cp as *const u8, 256);
            let rb = std::slice::from_raw_parts(rp as *const u8, 256);
            assert_eq!(
                cb,
                rb,
                "strkey({}) buffer mismatch:\n C   = {:?}\n RUST= {:?}",
                n,
                String::from_utf8_lossy(&cb[..cb.iter().position(|&b| b == 0).unwrap_or(256)]),
                String::from_utf8_lossy(&rb[..rb.iter().position(|&b| b == 0).unwrap_or(256)]),
            );
        }
    }
}

// --- row 70: arr_ins ----------------------------------------------------
#[test]
fn cfg_70_arr_ins() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 70);
    let mut inputs: Vec<c_int> = vec![0, 1, 2, 3, 4, 5, -1, i32::MAX, i32::MIN];
    for _ in 0..2000 {
        inputs.push(rng.next_u32() as i32);
    }
    for n in inputs {
        unsafe {
            (s.c.arr_ins)(n);
            (s.rust.arr_ins)(n);
        }
    }
}
