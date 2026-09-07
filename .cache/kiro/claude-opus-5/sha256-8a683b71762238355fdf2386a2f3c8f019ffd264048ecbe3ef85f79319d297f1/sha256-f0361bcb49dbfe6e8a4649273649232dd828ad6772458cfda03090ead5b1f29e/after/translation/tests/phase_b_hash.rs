//! Phase B — CONFIGS.md rows 1-9: hash functions and the global seed.

mod common;
use common::*;
use std::ffi::c_void;

#[test]
fn row01_hash_bytes_len0() {
    let (p, _g) = libs();
    for seed in [0usize, 1, DEFAULT_SEED, usize::MAX, 0xdead_beef_cafe_babe] {
        let mut buf = [0u8; 1];
        unsafe {
            let a = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            let b = (p.rs.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            assert_eq!(a, b, "hash_bytes(len=0, seed={seed:#x})");
        }
        // and with a genuinely NULL pointer (row 58 of ERRORS.md)
        unsafe {
            let a = (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let b = (p.rs.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(a, b, "hash_bytes(NULL, 0, {seed:#x})");
        }
    }
}

#[test]
fn row02_hash_bytes_tail_only() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE);
    for len in 1usize..=7 {
        for _ in 0..2000 {
            let mut b = rng.bytes(len);
            let seed = rng.next_u64() as usize;
            unsafe {
                let x = (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                let y = (p.rs.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                assert_eq!(x, y, "hash_bytes({b:02x?}, {len}, {seed:#x})");
            }
        }
    }
}

#[test]
fn row03_hash_bytes_len8() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 3);
    for _ in 0..5000 {
        let mut b = rng.bytes(8);
        let seed = rng.next_u64() as usize;
        unsafe {
            let x = (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, 8, seed);
            let y = (p.rs.hash_bytes)(b.as_mut_ptr() as *mut c_void, 8, seed);
            assert_eq!(x, y, "hash_bytes({b:02x?}, 8, {seed:#x})");
        }
    }
}

#[test]
fn row04_hash_bytes_body_plus_tail() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 4);
    for len in 9usize..=64 {
        for _ in 0..400 {
            let mut b = rng.bytes(len);
            let seed = rng.next_u64() as usize;
            unsafe {
                let x = (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                let y = (p.rs.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                assert_eq!(x, y, "hash_bytes(len={len}, seed={seed:#x}) bytes={b:02x?}");
            }
        }
    }
}

#[test]
fn row05_hash_bytes_high_bit_bytes() {
    // Every byte >= 0x80: maximises the `(d[3] << 24)` int-overflow
    // sign-extension quirk in both the body loop and the tail switch.
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 5);
    for len in 1usize..=64 {
        for _ in 0..400 {
            let mut b = rng.high_bytes(len);
            let seed = rng.next_u64() as usize;
            unsafe {
                let x = (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                let y = (p.rs.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                assert_eq!(x, y, "hash_bytes(hi, len={len}, seed={seed:#x}) {b:02x?}");
            }
        }
    }
    // exhaustive over the byte value at every tail position
    for len in 1usize..=7 {
        for pos in 0..len {
            for v in 0u16..=255 {
                let mut b = vec![0x11u8; len];
                b[pos] = v as u8;
                unsafe {
                    let x = (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, DEFAULT_SEED);
                    let y = (p.rs.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, DEFAULT_SEED);
                    assert_eq!(x, y, "tail len={len} pos={pos} v={v:#02x}");
                }
            }
        }
    }
    // exhaustive over the byte value at every body position (len == 8)
    for pos in 0..8 {
        for v in 0u16..=255 {
            let mut b = vec![0x11u8; 8];
            b[pos] = v as u8;
            unsafe {
                let x = (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, 8, DEFAULT_SEED);
                let y = (p.rs.hash_bytes)(b.as_mut_ptr() as *mut c_void, 8, DEFAULT_SEED);
                assert_eq!(x, y, "body pos={pos} v={v:#02x}");
            }
        }
    }
}

#[test]
fn row06_hash_bytes_seeds() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 6);
    let mut seeds: Vec<usize> = vec![0, 1, 2, usize::MAX, usize::MAX - 1, 1 << 63, DEFAULT_SEED];
    for _ in 0..500 {
        seeds.push(rng.next_u64() as usize);
    }
    for len in [0usize, 1, 3, 7, 8, 15, 16, 31, 32] {
        let mut b = rng.bytes(len.max(1));
        for &seed in &seeds {
            unsafe {
                let x = (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                let y = (p.rs.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
                assert_eq!(x, y, "hash_bytes(len={len}, seed={seed:#x})");
            }
        }
    }
}

#[test]
fn row07_hash_string_shapes() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 7);

    // empty string (ERRORS.md row 52)
    let k = CKey::new(b"");
    unsafe {
        assert_eq!(
            (p.c.hash_string)(k.ptr(), DEFAULT_SEED),
            (p.rs.hash_string)(k.ptr(), DEFAULT_SEED)
        );
    }

    // every single byte value 1..=255 (ERRORS.md row 53: `(unsigned char)` cast)
    for v in 1u16..=255 {
        let k = CKey::new(&[v as u8]);
        for seed in [0usize, DEFAULT_SEED, usize::MAX] {
            unsafe {
                let a = (p.c.hash_string)(k.ptr(), seed);
                let b = (p.rs.hash_string)(k.ptr(), seed);
                assert_eq!(a, b, "hash_string([{v:#02x}], {seed:#x})");
            }
        }
    }

    for len in 0usize..=64 {
        for _ in 0..300 {
            let bytes = if rng.next_u64() & 1 == 0 {
                rng.ascii(len)
            } else {
                rng.nonnul(len)
            };
            let k = CKey::new(&bytes);
            let seed = rng.next_u64() as usize;
            unsafe {
                let a = (p.c.hash_string)(k.ptr(), seed);
                let b = (p.rs.hash_string)(k.ptr(), seed);
                assert_eq!(a, b, "hash_string({bytes:02x?}, {seed:#x})");
            }
        }
    }
}

#[test]
fn row08_hash_string_seeds() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 8);
    let mut seeds: Vec<usize> = vec![0, 1, usize::MAX, 1 << 63, DEFAULT_SEED];
    for _ in 0..500 {
        seeds.push(rng.next_u64() as usize);
    }
    for n in [0usize, 1, 5, 17, 40] {
        let k = CKey::new(&rng.nonnul(n));
        for &seed in &seeds {
            unsafe {
                assert_eq!(
                    (p.c.hash_string)(k.ptr(), seed),
                    (p.rs.hash_string)(k.ptr(), seed),
                    "hash_string(len={n}, seed={seed:#x})"
                );
            }
        }
    }
}

#[test]
fn row09_rand_seed_and_table_seed_evolution() {
    // `stbds_make_hash_index(_, NULL)` copies the global seed into the table
    // and then advances the global with `seed*a + b`.  Creating 8 tables in a
    // row makes that whole chain observable through `table->seed`.
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 9);
    let mut seeds: Vec<usize> = vec![0, 1, DEFAULT_SEED, usize::MAX, 1 << 63];
    for _ in 0..40 {
        seeds.push(rng.next_u64() as usize);
    }
    for &s in &seeds {
        reset_seed(p, s);
        for i in 0..8 {
            unsafe {
                let ch = (p.c.shmode_func)(16, SH_NONE);
                let rh = (p.rs.shmode_func)(16, SH_NONE);
                let cs = snap_hash(ch, 16, ElemFmt::Raw);
                let rs = snap_hash(rh, 16, ElemFmt::Raw);
                eq_snap(&format!("rand_seed({s:#x}) table #{i}"), &cs, &rs);
                assert_eq!(
                    cs.table.as_ref().unwrap().seed,
                    rs.table.as_ref().unwrap().seed
                );
                (p.c.hmfree_func)((ch as *mut u8).sub(16) as *mut c_void, 16);
                (p.rs.hmfree_func)((rh as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
    }
}
