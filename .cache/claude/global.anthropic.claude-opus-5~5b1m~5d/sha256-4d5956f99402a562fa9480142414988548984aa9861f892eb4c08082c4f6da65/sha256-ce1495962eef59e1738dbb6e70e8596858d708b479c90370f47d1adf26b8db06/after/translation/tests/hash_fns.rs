//! Phase B — CONFIGS.md rows 1..13, 16..17
//! `stbds_hash_bytes`, `stbds_hash_string`, `strkey`

mod common;
use common::*;
use std::ffi::c_char;
use std::ffi::c_void;

fn seeds(rng: &mut Rng, n: usize) -> Vec<usize> {
    let mut v = vec![0usize, 1, DEFAULT_SEED, usize::MAX, usize::MAX - 1, 2, 0x8000_0000_0000_0000];
    for _ in 0..n {
        v.push(rng.next_u64() as usize);
    }
    v
}

unsafe fn hb(buf: &mut [u8], len: usize, seed: usize) -> (usize, usize) {
    let p = pair();
    let ptr = if buf.is_empty() {
        std::ptr::null_mut()
    } else {
        buf.as_mut_ptr() as *mut c_void
    };
    ((p.c.hash_bytes)(ptr, len, seed), (p.r.hash_bytes)(ptr, len, seed))
}

unsafe fn hs(s: &mut [u8], seed: usize) -> (usize, usize) {
    let p = pair();
    (
        (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
        (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
    )
}

// --------------------------------------------------------------------- row 1
#[test]
fn c01_hash_bytes_len_zero() {
    let mut rng = Rng::new(0x0101);
    for s in seeds(&mut rng, 64) {
        unsafe {
            // NULL pointer with len 0 must never be dereferenced
            let p = pair();
            let a = (p.c.hash_bytes)(std::ptr::null_mut(), 0, s);
            let b = (p.r.hash_bytes)(std::ptr::null_mut(), 0, s);
            assert_eq!(a, b, "hash_bytes(NULL,0,{:#x})", s);
            // and a real buffer with len 0 gives the same answer
            let mut buf = [0xAAu8; 8];
            let (c, d) = hb(&mut buf, 0, s);
            assert_eq!(c, d);
            assert_eq!(a, c, "len-0 hash must not depend on the buffer");
        }
    }
}

// --------------------------------------------------------------------- row 2
#[test]
fn c02_hash_bytes_tail_cases() {
    let mut rng = Rng::new(0x0202);
    for len in 1..=7usize {
        for s in seeds(&mut rng, 40) {
            for _ in 0..40 {
                let mut buf = rng.bytes(len);
                unsafe {
                    let (a, b) = hb(&mut buf, len, s);
                    assert_eq!(a, b, "len={} seed={:#x} buf={:02x?}", len, s, buf);
                }
            }
        }
    }
}

// --------------------------------------------------------------------- row 3
#[test]
fn c03_hash_bytes_tail_sign_extension() {
    // `case 4: data |= (d[3] << 24);` makes a *negative int* that sign-extends
    // into size_t.  Force the top byte high for every tail length.
    let mut rng = Rng::new(0x0303);
    for len in 1..=7usize {
        for s in seeds(&mut rng, 16) {
            for _ in 0..64 {
                let mut buf = rng.bytes(len);
                for b in buf.iter_mut() {
                    *b |= 0x80;
                }
                unsafe {
                    let (a, b) = hb(&mut buf, len, s);
                    assert_eq!(a, b, "highbit len={} seed={:#x} buf={:02x?}", len, s, buf);
                }
                // and specifically only d[3] high
                if len >= 4 {
                    let mut buf2 = rng.bytes(len);
                    for (i, b) in buf2.iter_mut().enumerate() {
                        if i == 3 {
                            *b |= 0x80;
                        } else {
                            *b &= 0x7f;
                        }
                    }
                    unsafe {
                        let (a, b) = hb(&mut buf2, len, s);
                        assert_eq!(a, b, "d3 high len={} seed={:#x}", len, s);
                    }
                }
            }
        }
    }
}

// --------------------------------------------------------------------- row 4
#[test]
fn c04_hash_bytes_whole_blocks() {
    let mut rng = Rng::new(0x0404);
    for len in [8usize, 16, 24, 32, 40, 48, 56, 64, 72, 80] {
        for s in seeds(&mut rng, 24) {
            for _ in 0..24 {
                let mut buf = rng.bytes(len);
                unsafe {
                    let (a, b) = hb(&mut buf, len, s);
                    assert_eq!(a, b, "len={} seed={:#x}", len, s);
                }
            }
        }
    }
}

// --------------------------------------------------------------------- row 5
#[test]
fn c05_hash_bytes_blocks_plus_tail() {
    let mut rng = Rng::new(0x0505);
    for len in 9..80usize {
        for s in seeds(&mut rng, 8) {
            for _ in 0..12 {
                let mut buf = rng.bytes(len);
                unsafe {
                    let (a, b) = hb(&mut buf, len, s);
                    assert_eq!(a, b, "len={} seed={:#x}", len, s);
                }
            }
        }
    }
}

// --------------------------------------------------------------------- row 6
#[test]
fn c06_hash_bytes_extreme_content() {
    let mut rng = Rng::new(0x0606);
    for len in 0..=40usize {
        for fill in [0x00u8, 0xFF, 0x80, 0x7F, 0x01] {
            let mut buf = vec![fill; len.max(1)];
            for s in seeds(&mut rng, 8) {
                unsafe {
                    let (a, b) = hb(&mut buf, len, s);
                    assert_eq!(a, b, "fill={:#x} len={} seed={:#x}", fill, len, s);
                }
            }
        }
    }
}

// --------------------------------------------------------------------- row 7
#[test]
fn c07_hash_bytes_unaligned() {
    let mut rng = Rng::new(0x0707);
    let mut backing = rng.bytes(256);
    for off in 0..8usize {
        for len in [0usize, 1, 3, 7, 8, 9, 15, 16, 33, 64] {
            for s in seeds(&mut rng, 8) {
                unsafe {
                    let p = pair();
                    let ptr = backing.as_mut_ptr().add(off) as *mut c_void;
                    let a = (p.c.hash_bytes)(ptr, len, s);
                    let b = (p.r.hash_bytes)(ptr, len, s);
                    assert_eq!(a, b, "off={} len={} seed={:#x}", off, len, s);
                }
            }
        }
    }
}

// --------------------------------------------------------------------- row 8
#[test]
fn c08_hash_bytes_large() {
    let mut rng = Rng::new(0x0808);
    for len in [1024usize, 4095, 4096, 4097] {
        let mut buf = rng.bytes(len);
        for s in seeds(&mut rng, 8) {
            unsafe {
                let (a, b) = hb(&mut buf, len, s);
                assert_eq!(a, b, "len={} seed={:#x}", len, s);
            }
        }
    }
}

// --------------------------------------------------------------------- row 9
#[test]
fn c09_hash_string_empty() {
    let mut rng = Rng::new(0x0909);
    for s in seeds(&mut rng, 128) {
        let mut e = vec![0u8];
        unsafe {
            let (a, b) = hs(&mut e, s);
            assert_eq!(a, b, "hash_string(\"\", {:#x})", s);
        }
    }
}

// -------------------------------------------------------------------- row 10
#[test]
fn c10_hash_string_ascii() {
    let mut rng = Rng::new(0x1010);
    for len in 1..=64usize {
        for s in seeds(&mut rng, 8) {
            for _ in 0..16 {
                let mut k = rng.cstring(len, ASCII);
                unsafe {
                    let (a, b) = hs(&mut k, s);
                    assert_eq!(a, b, "len={} seed={:#x} k={:?}", len, s, show(&k));
                }
            }
        }
    }
}

// -------------------------------------------------------------------- row 11
#[test]
fn c11_hash_string_high_bytes() {
    // `(unsigned char) *str++` must not sign-extend.
    let mut rng = Rng::new(0x1111);
    let high: Vec<u8> = (0x80u8..=0xFFu8).collect();
    for len in 1..=64usize {
        for s in seeds(&mut rng, 8) {
            for _ in 0..16 {
                let mut k = rng.cstring(len, &high);
                unsafe {
                    let (a, b) = hs(&mut k, s);
                    assert_eq!(a, b, "len={} seed={:#x}", len, s);
                }
            }
        }
        // deterministic worst case: all 0xFF
        let mut k: Vec<u8> = vec![0xFF; len];
        k.push(0);
        for s in seeds(&mut rng, 4) {
            unsafe {
                let (a, b) = hs(&mut k, s);
                assert_eq!(a, b, "all-FF len={} seed={:#x}", len, s);
            }
        }
    }
}

// -------------------------------------------------------------------- row 12
#[test]
fn c12_hash_string_long() {
    let mut rng = Rng::new(0x1212);
    for len in [255usize, 256, 511, 512, 1024, 4096] {
        for _ in 0..8 {
            let mut k = rng.cstring(len, HIGHBYTES);
            for s in seeds(&mut rng, 4) {
                unsafe {
                    let (a, b) = hs(&mut k, s);
                    assert_eq!(a, b, "len={} seed={:#x}", len, s);
                }
            }
        }
    }
}

// -------------------------------------------------------------------- row 13
#[test]
fn c13_hash_string_strkey_shapes() {
    let mut rng = Rng::new(0x1313);
    let ss = seeds(&mut rng, 6);
    for n in 0..2000i32 {
        let mut k = format!("test_{}\0", n).into_bytes();
        for &s in &ss {
            unsafe {
                let (a, b) = hs(&mut k, s);
                assert_eq!(a, b, "n={} seed={:#x}", n, s);
            }
        }
    }
}

// -------------------------------------------------------------------- row 16
#[test]
fn c16_strkey_values() {
    // `strkey` writes into a shared `static char buffer[256]` inside each .so
    let _g = globals_guard();
    let p = pair();
    let mut rng = Rng::new(0x1616);
    let mut ns: Vec<i32> = vec![0, 1, -1, 9, 10, 99, 100, 999, 1000, i32::MAX, i32::MIN, -999999];
    for _ in 0..256 {
        ns.push(rng.next_u32() as i32);
    }
    for n in ns {
        unsafe {
            let a = read_cstr((p.c.strkey)(n));
            let b = read_cstr((p.r.strkey)(n));
            assert_eq!(
                show(&a),
                show(&b),
                "strkey({}) diverged: C={:?} Rust={:?}",
                n,
                show(&a),
                show(&b)
            );
            assert_eq!(show(&a), format!("test_{}", n));
        }
    }
}

// -------------------------------------------------------------------- row 17
#[test]
fn c17_strkey_static_buffer_semantics() {
    let _g = globals_guard();
    let p = pair();
    unsafe {
        let p1c = (p.c.strkey)(1);
        let p2c = (p.c.strkey)(22222);
        let p1r = (p.r.strkey)(1);
        let p2r = (p.r.strkey)(22222);
        // same static buffer is reused: both calls return the same pointer ...
        assert_eq!(p1c, p2c, "C strkey must reuse its static buffer");
        assert_eq!(p1r, p2r, "Rust strkey must reuse its static buffer");
        // ... and the earlier content is gone
        assert_eq!(show(&read_cstr(p1c)), "test_22222");
        assert_eq!(show(&read_cstr(p1r)), "test_22222");
        // the two libraries have *independent* buffers (separate .so statics)
        assert_ne!(p1c, p1r, "the two .so files must not share the static buffer");
    }
}
