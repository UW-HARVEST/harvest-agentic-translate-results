//! Phase B — lowest-level entry points: `stbds_rand_seed`, `stbds_hash_bytes`,
//! `stbds_hash_string`. Rows C01–C08 of CONFIGS.md.
mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_char;

const SEEDS: &[usize] = &[
    0,
    1,
    2,
    0x31415926,
    0xffff_ffff,
    0x1_0000_0000,
    usize::MAX,
    usize::MAX - 1,
    0x8000_0000_0000_0000,
    0xdead_beef_cafe_babe,
];

fn hb(p: &Pair, buf: &mut [u8], len: usize, seed: usize) -> (usize, usize) {
    unsafe {
        let cp = buf.as_mut_ptr() as *mut c_void;
        ((p.c.hash_bytes)(cp, len, seed), (p.r.hash_bytes)(cp, len, seed))
    }
}

fn hs(p: &Pair, s: &mut [u8], seed: usize) -> (usize, usize) {
    unsafe {
        let cp = s.as_mut_ptr() as *mut c_char;
        ((p.c.hash_string)(cp, seed), (p.r.hash_string)(cp, seed))
    }
}

// --- C01 -------------------------------------------------------------------
#[test]
fn c01_hash_bytes_len0() {
    let p = fresh_pair(0x31415926);
    let mut rng = Rng::new(0xC01);
    let mut buf = [0u8; 8];
    for &s in SEEDS {
        let (c, r) = hb(&p, &mut buf, 0, s);
        same_val(&format!("hash_bytes(len=0, seed={s:#x})"), c, r);
    }
    for _ in 0..500 {
        let s = rng.next_u64() as usize;
        let mut b = rng.bytes(16);
        let (c, r) = hb(&p, &mut b, 0, s);
        same_val(&format!("hash_bytes(len=0, seed={s:#x})"), c, r);
    }
}

// --- C02 : every tail switch case (len-i == 1..7) ---------------------------
#[test]
fn c02_hash_bytes_short_tails() {
    let p = fresh_pair(7);
    let mut rng = Rng::new(0xC02);
    for len in 1usize..8 {
        for &s in SEEDS {
            for _ in 0..64 {
                let mut b = rng.bytes(len + 8);
                let (c, r) = hb(&p, &mut b, len, s);
                same_val(
                    &format!("hash_bytes(len={len}, seed={s:#x}, buf={})", hex(&b[..len])),
                    c,
                    r,
                );
            }
        }
    }
}

// --- C03 : body loop only, no tail -----------------------------------------
#[test]
fn c03_hash_bytes_aligned() {
    let p = fresh_pair(1);
    let mut rng = Rng::new(0xC03);
    for k in 1usize..=16 {
        let len = k * 8;
        for &s in SEEDS {
            for _ in 0..24 {
                let mut b = rng.bytes(len);
                let (c, r) = hb(&p, &mut b, len, s);
                same_val(
                    &format!("hash_bytes(len={len}, seed={s:#x}, buf={})", hex(&b)),
                    c,
                    r,
                );
            }
        }
    }
}

// --- C04 : body loop + tail -------------------------------------------------
#[test]
fn c04_hash_bytes_body_plus_tail() {
    let p = fresh_pair(2);
    let mut rng = Rng::new(0xC04);
    for len in 9usize..72 {
        if len % 8 == 0 {
            continue;
        }
        for &s in &SEEDS[..5] {
            for _ in 0..16 {
                let mut b = rng.bytes(len);
                let (c, r) = hb(&p, &mut b, len, s);
                same_val(
                    &format!("hash_bytes(len={len}, seed={s:#x}, buf={})", hex(&b)),
                    c,
                    r,
                );
            }
        }
    }
}

// --- C05 : sign-extension quirk (`d[3] << 24`, `d[7] << 24`) ---------------
#[test]
fn c05_hash_bytes_high_bit_bytes() {
    let p = fresh_pair(3);
    let mut rng = Rng::new(0xC05);
    for len in 1usize..40 {
        for pattern in 0..6 {
            for _ in 0..12 {
                let mut b = rng.bytes(len.max(8));
                match pattern {
                    0 => b.iter_mut().for_each(|x| *x = 0xff),
                    1 => b.iter_mut().for_each(|x| *x = 0x80),
                    2 => {
                        for (i, x) in b.iter_mut().enumerate() {
                            if i % 4 == 3 {
                                *x = 0xff
                            }
                        }
                    }
                    3 => {
                        for (i, x) in b.iter_mut().enumerate() {
                            if i % 8 == 7 {
                                *x = 0x80
                            }
                        }
                    }
                    4 => b.iter_mut().for_each(|x| *x = 0x00),
                    _ => {
                        for (i, x) in b.iter_mut().enumerate() {
                            *x |= if i % 2 == 0 { 0x80 } else { 0x00 }
                        }
                    }
                }
                for &s in &SEEDS[..4] {
                    let (c, r) = hb(&p, &mut b, len, s);
                    same_val(
                        &format!(
                            "hash_bytes(len={len}, pat={pattern}, seed={s:#x}, buf={})",
                            hex(&b[..len])
                        ),
                        c,
                        r,
                    );
                }
            }
        }
    }
}

// --- C06 : seed extremes ----------------------------------------------------
#[test]
fn c06_hash_bytes_seeds() {
    let p = fresh_pair(4);
    let mut rng = Rng::new(0xC06);
    for _ in 0..2000 {
        let seed = match rng.below(6) {
            0 => 0,
            1 => 1,
            2 => usize::MAX,
            3 => 0x31415926,
            4 => 1usize << rng.below(64),
            _ => rng.next_u64() as usize,
        };
        let len = rng.below(48);
        let mut b = rng.bytes(len + 8);
        let (c, r) = hb(&p, &mut b, len, seed);
        same_val(
            &format!("hash_bytes(len={len}, seed={seed:#x}, buf={})", hex(&b[..len])),
            c,
            r,
        );
    }
}

// --- C56 (E56) : NULL pointer with len 0 ------------------------------------
#[test]
fn e56_hash_bytes_null_len0() {
    let p = fresh_pair(5);
    unsafe {
        for &s in SEEDS {
            let c = (p.c.hash_bytes)(std::ptr::null_mut(), 0, s);
            let r = (p.r.hash_bytes)(std::ptr::null_mut(), 0, s);
            same_val(&format!("hash_bytes(NULL,0,{s:#x})"), c, r);
        }
    }
}

// --- C07 : hash_string ASCII ------------------------------------------------
#[test]
fn c07_hash_string_ascii() {
    let p = fresh_pair(6);
    let mut rng = Rng::new(0xC07);
    // empty string first (E57)
    let mut empty = vec![0u8];
    for &s in SEEDS {
        let (c, r) = hs(&p, &mut empty, s);
        same_val(&format!("hash_string(\"\", {s:#x})"), c, r);
    }
    for len in 0usize..65 {
        for &s in &SEEDS[..5] {
            for _ in 0..12 {
                let mut b = rng.cstring(len, ASCII);
                let (c, r) = hs(&p, &mut b, s);
                same_val(
                    &format!("hash_string(len={len}, seed={s:#x}, {})", hex(&b)),
                    c,
                    r,
                );
            }
        }
    }
}

// --- C08 / E58 : high bytes -------------------------------------------------
#[test]
fn c08_hash_string_high_bytes() {
    let p = fresh_pair(8);
    let mut rng = Rng::new(0xC08);
    for len in 1usize..48 {
        for &s in &SEEDS[..5] {
            for _ in 0..12 {
                let mut b = rng.cstring(len, HIGHBYTES);
                let (c, r) = hs(&p, &mut b, s);
                same_val(
                    &format!("hash_string(high len={len}, seed={s:#x}, {})", hex(&b)),
                    c,
                    r,
                );
            }
        }
    }
    // explicit 0x80..0xff runs
    for b0 in 0x80u8..=0xff {
        let mut b = vec![b0, b0.wrapping_add(1), 0xff, 0x80, 0];
        for &s in &SEEDS[..3] {
            let (c, r) = hs(&p, &mut b, s);
            same_val(&format!("hash_string({}, {s:#x})", hex(&b)), c, r);
        }
    }
}

#[test]
fn e57_hash_string_empty() {
    let p = fresh_pair(9);
    let mut empty = vec![0u8];
    for &s in SEEDS {
        let (c, r) = hs(&p, &mut empty, s);
        same_val(&format!("hash_string empty seed={s:#x}"), c, r);
    }
}

#[test]
fn e53_hash_bytes_len0() {
    let p = fresh_pair(10);
    let mut b = vec![0xffu8; 8];
    for &s in SEEDS {
        let (c, r) = hb(&p, &mut b, 0, s);
        same_val(&format!("len0 seed={s:#x}"), c, r);
    }
}

#[test]
fn e54_hash_bytes_tails() {
    let p = fresh_pair(11);
    let mut rng = Rng::new(0xE54);
    for len in 0usize..=7 {
        for _ in 0..200 {
            let mut b = rng.bytes(8);
            let s = rng.next_u64() as usize;
            let (c, r) = hb(&p, &mut b, len, s);
            same_val(&format!("tail len={len} {}", hex(&b)), c, r);
        }
    }
}

#[test]
fn e55_hash_bytes_high_bit() {
    let p = fresh_pair(12);
    // every single-byte value at every tail position
    for len in 1usize..=8 {
        for pos in 0..len {
            for v in [0x00u8, 0x01, 0x7f, 0x80, 0x81, 0xfe, 0xff] {
                let mut b = vec![0u8; 8];
                b[pos] = v;
                for &s in &SEEDS[..4] {
                    let (c, r) = hb(&p, &mut b, len, s);
                    same_val(&format!("len={len} pos={pos} v={v:#x} seed={s:#x}"), c, r);
                }
            }
        }
    }
}

#[test]
fn b06_seed_extremes() {
    let p = pair();
    let mut rng = Rng::new(0xB06);
    for &s in &[0usize, 1, usize::MAX, 0x31415926, usize::MAX / 2, 2, usize::MAX - 1] {
        unsafe {
            (p.c.rand_seed)(s);
            (p.r.rand_seed)(s);
        }
        // rand_seed only affects newly created hash tables; verify through the
        // per-table `seed` captured by make_hash_index and the resulting layout
        let mut m = DiffMap::lazy(&p, 8, 4, HM_BINARY, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        for i in 0..24u32 {
            let k = ka.add(&rng.next_u32().to_le_bytes());
            let v = (i as u32).to_le_bytes();
            let (tc, tr) = m.put(k, &v);
            same_val(&format!("b06 seed={s:#x} put temp"), tc, tr);
            m.check(&format!("b06 seed={s:#x}"));
        }
        m.free();
    }
}

// --- exhaustive small-input sweeps (cheap and complete) ---------------------

/// Every single byte value at every position, for every length 1..=9.
#[test]
fn exhaustive_hash_bytes_single_byte() {
    let p = fresh_pair(0x1111);
    for len in 1usize..=9 {
        for pos in 0..len {
            for v in 0u8..=255 {
                let mut b = vec![0u8; 16];
                b[pos] = v;
                for &s in &[0usize, 1, 0x31415926, usize::MAX] {
                    let (c, r) = hb(&p, &mut b, len, s);
                    same_val(&format!("exh1 len={len} pos={pos} v={v:#04x} seed={s:#x}"), c, r);
                }
            }
        }
    }
}

/// Every 1- and 2-byte string (255 + 255*255 = 65 280 inputs).
#[test]
fn exhaustive_hash_string_short() {
    let p = fresh_pair(0x2222);
    for a in 1u8..=255 {
        let mut s1 = vec![a, 0];
        for &sd in &[0usize, 0x31415926, usize::MAX] {
            let (c, r) = hs(&p, &mut s1, sd);
            same_val(&format!("exh-str1 {a:#04x} seed={sd:#x}"), c, r);
        }
    }
    for a in 1u8..=255 {
        for b in 1u8..=255 {
            let mut s2 = vec![a, b, 0];
            let (c, r) = hs(&p, &mut s2, 0x31415926);
            same_val(&format!("exh-str2 {a:#04x}{b:#04x}"), c, r);
        }
    }
}

/// Every 8-byte buffer that is a single bit set (64 inputs) plus every
/// all-ones prefix, over lengths 1..=16 — targets the shift/sign-extension math.
#[test]
fn exhaustive_hash_bytes_bit_patterns() {
    let p = fresh_pair(0x3333);
    for bit in 0..128usize {
        let mut b = vec![0u8; 16];
        b[bit / 8] = 1u8 << (bit % 8);
        for len in 1usize..=16 {
            for &s in &[0usize, 1, usize::MAX] {
                let (c, r) = hb(&p, &mut b, len, s);
                same_val(&format!("exh-bit {bit} len={len} seed={s:#x}"), c, r);
            }
        }
    }
    for n in 0usize..=16 {
        let mut b = vec![0u8; 16];
        for i in 0..n {
            b[i] = 0xff;
        }
        for len in 0usize..=16 {
            for &s in &[0usize, 0x31415926, usize::MAX] {
                let (c, r) = hb(&p, &mut b, len, s);
                same_val(&format!("exh-ones n={n} len={len} seed={s:#x}"), c, r);
            }
        }
    }
}

/// Every `int` value that `strkey` can be handed, in a strided sweep plus all
/// the decimal-digit boundaries and both extremes.
#[test]
fn exhaustive_strkey_boundaries() {
    let p = fresh_pair(0x4444);
    let mut ns: Vec<i32> = Vec::new();
    for e in 0..10u32 {
        let base = 10i64.pow(e);
        for d in -2i64..=2 {
            let v = base + d;
            if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                ns.push(v as i32);
            }
            let v = -base + d;
            if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
                ns.push(v as i32);
            }
        }
    }
    ns.extend([i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0, -0]);
    // strided sweep across the whole range
    let mut v: i64 = i32::MIN as i64;
    while v <= i32::MAX as i64 {
        ns.push(v as i32);
        v += 1_048_573; // prime-ish stride -> 4096 samples
    }
    unsafe {
        for n in ns {
            let c = cstr((p.c.strkey)(n));
            let r = cstr((p.r.strkey)(n));
            same_val(&format!("strkey({n})"), c, r);
        }
    }
}
