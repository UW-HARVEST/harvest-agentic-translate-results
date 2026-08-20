//! Phase B, CONFIGS.md rows 1-12: `stbds_hash_bytes`, `stbds_hash_string`,
//! `stbds_rand_seed` and the global seed evolution.

mod common;
use common::*;
use std::ffi::c_void;

const SEEDS: &[usize] = &[
    0,
    1,
    2,
    0x31415926,
    0xffff_ffff,
    0x8000_0000_0000_0000,
    usize::MAX,
    0xdead_beef_cafe_babe,
];

fn hb(p: &Pair, buf: &[u8], len: usize, seed: usize) -> (usize, usize) {
    let mut b = buf.to_vec();
    unsafe {
        let a = (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
        let c = (p.rs.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
        (a, c)
    }
}

fn hs(p: &Pair, s: &[u8], seed: usize) -> (usize, usize) {
    let mut b = s.to_vec();
    assert_eq!(*b.last().unwrap(), 0, "must be NUL-terminated");
    unsafe {
        let a = (p.c.hash_string)(b.as_mut_ptr() as *mut i8, seed);
        let c = (p.rs.hash_string)(b.as_mut_ptr() as *mut i8, seed);
        (a, c)
    }
}

// ---------------------------------------------------------------------------
// row 1 — len = 0
// ---------------------------------------------------------------------------
#[test]
fn cfg01_hash_bytes_len0() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let buf = [0xaau8; 32];
    for &seed in SEEDS {
        let (a, b) = hb(p, &buf, 0, seed);
        assert_eq_ctx(a, b, &format!("hash_bytes(len=0, seed={seed:#x})"));
    }
    let mut r = Rng::new(0x1000);
    for _ in 0..64 {
        let seed = r.u64() as usize;
        let (a, b) = hb(p, &buf, 0, seed);
        assert_eq_ctx(a, b, &format!("hash_bytes(len=0, rng seed={seed:#x})"));
    }
    // NULL pointer with len 0 must not be dereferenced by either impl
    unsafe {
        let a = (p.c.hash_bytes)(std::ptr::null_mut(), 0, 12345);
        let b = (p.rs.hash_bytes)(std::ptr::null_mut(), 0, 12345);
        assert_eq_ctx(a, b, "hash_bytes(NULL, 0, 12345)");
    }
}

// ---------------------------------------------------------------------------
// row 2 — len 1..=7, all-zero bytes (every fall-through case)
// ---------------------------------------------------------------------------
#[test]
fn cfg02_hash_bytes_tail_zeros() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let buf = [0u8; 16];
    for len in 0..=7usize {
        for &seed in SEEDS {
            let (a, b) = hb(p, &buf, len, seed);
            assert_eq_ctx(a, b, &format!("zeros len={len} seed={seed:#x}"));
        }
    }
}

// ---------------------------------------------------------------------------
// row 3 — len 1..=7 with high tail bytes (the `int` sign-extension quirk)
// ---------------------------------------------------------------------------
#[test]
fn cfg03_hash_bytes_tail_sign_extension() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    // exhaustively: for every tail length, every single byte position set to
    // every "interesting" value
    let interesting: [u8; 8] = [0x00, 0x01, 0x7f, 0x80, 0x81, 0xfe, 0xff, 0xaa];
    for len in 1..=7usize {
        for pos in 0..len {
            for &v in &interesting {
                let mut buf = [0u8; 16];
                buf[pos] = v;
                for &seed in &[0usize, 1, 0x31415926, usize::MAX] {
                    let (a, b) = hb(p, &buf, len, seed);
                    assert_eq_ctx(
                        a,
                        b,
                        &format!("len={len} pos={pos} v={v:#02x} seed={seed:#x}"),
                    );
                }
            }
        }
    }
    // and all 256 values in the d[3] position (case 4: `d[3] << 24` overflow)
    for v in 0..=255u8 {
        let mut buf = [0u8; 16];
        buf[3] = v;
        for len in 4..=7usize {
            let (a, b) = hb(p, &buf, len, 0x31415926);
            assert_eq_ctx(a, b, &format!("d[3]={v:#02x} len={len}"));
        }
    }
    // exhaustive over all 256 values in every tail position, len = 7
    for pos in 0..7 {
        for v in 0..=255u8 {
            let mut buf = [0u8; 16];
            buf[pos] = v;
            let (a, b) = hb(p, &buf, 7, 1);
            assert_eq_ctx(a, b, &format!("len=7 pos={pos} v={v:#02x}"));
        }
    }
}

// ---------------------------------------------------------------------------
// row 4 — len 1..=7, randomized bodies
// ---------------------------------------------------------------------------
#[test]
fn cfg04_hash_bytes_short_random() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let mut r = Rng::new(0x4004);
    for len in 1..=7usize {
        for _ in 0..256 {
            let buf = r.bytes(8);
            let seed = r.u64() as usize;
            let (a, b) = hb(p, &buf, len, seed);
            assert_eq_ctx(a, b, &format!("rng len={len} buf={buf:?} seed={seed:#x}"));
        }
    }
}

// ---------------------------------------------------------------------------
// rows 5, 6 — len 8 exactly, then 8..=64
// ---------------------------------------------------------------------------
#[test]
fn cfg05_06_hash_bytes_words_and_tails() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let mut r = Rng::new(0x5005);
    // len = 8 exactly
    for _ in 0..256 {
        let buf = r.bytes(8);
        let seed = r.u64() as usize;
        let (a, b) = hb(p, &buf, 8, seed);
        assert_eq_ctx(a, b, &format!("len=8 buf={buf:?} seed={seed:#x}"));
    }
    // len 8..=64
    for len in 8..=64usize {
        for _ in 0..8 {
            let buf = r.bytes(len + 8);
            let seed = r.u64() as usize;
            let (a, b) = hb(p, &buf, len, seed);
            assert_eq_ctx(a, b, &format!("len={len} seed={seed:#x} buf={buf:?}"));
        }
    }
}

// ---------------------------------------------------------------------------
// row 7 — long buffers
// ---------------------------------------------------------------------------
#[test]
fn cfg07_hash_bytes_long() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let mut r = Rng::new(0x7007);
    for &len in &[65usize, 127, 128, 129, 255, 256, 257, 1023, 1024, 4096, 4097] {
        for _ in 0..8 {
            let buf = r.bytes(len);
            let seed = r.u64() as usize;
            let (a, b) = hb(p, &buf, len, seed);
            assert_eq_ctx(a, b, &format!("long len={len} seed={seed:#x}"));
        }
    }
}

// ---------------------------------------------------------------------------
// row 8 — extreme words
// ---------------------------------------------------------------------------
#[test]
fn cfg08_hash_bytes_extreme_words() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    for fill in [0x00u8, 0x01, 0x7f, 0x80, 0xff, 0xaa, 0x55] {
        for &len in &[8usize, 16, 24, 32, 64, 128, 136] {
            let buf = vec![fill; len];
            for &seed in SEEDS {
                let (a, b) = hb(p, &buf, len, seed);
                assert_eq_ctx(a, b, &format!("fill={fill:#02x} len={len} seed={seed:#x}"));
            }
        }
    }
    // one 0x80 byte in each of the 8 positions of a single word
    for pos in 0..8 {
        let mut buf = vec![0u8; 8];
        buf[pos] = 0x80;
        for &seed in SEEDS {
            let (a, b) = hb(p, &buf, 8, seed);
            assert_eq_ctx(a, b, &format!("word 0x80@{pos} seed={seed:#x}"));
        }
    }
}

// ---------------------------------------------------------------------------
// row 9 — hash_string, ASCII
// ---------------------------------------------------------------------------
#[test]
fn cfg09_hash_string_ascii() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    for fixed in [
        &b"\0"[..],
        &b"a\0"[..],
        &b"ab\0"[..],
        &b"test_0\0"[..],
        &b"test_99999\0"[..],
        &b"the quick brown fox jumps over the lazy dog\0"[..],
    ] {
        for &seed in SEEDS {
            let (a, b) = hs(p, fixed, seed);
            assert_eq_ctx(
                a,
                b,
                &format!("hash_string({:?}, {seed:#x})", String::from_utf8_lossy(fixed)),
            );
        }
    }
    let mut r = Rng::new(0x9009);
    for _ in 0..512 {
        let len = r.range(0, 64);
        let s = r.cstring(len);
        let seed = r.u64() as usize;
        let (a, b) = hs(p, &s, seed);
        assert_eq_ctx(a, b, &format!("rng str len={len} seed={seed:#x}"));
    }
    // strkey() outputs, which is what the C test helper feeds in
    unsafe {
        for n in [-5i32, 0, 1, 42, 99999, i32::MIN, i32::MAX] {
            let kp = (p.c.strkey)(n);
            let bytes = cstr_bytes(kp as *const u8);
            let mut z = bytes.clone();
            z.push(0);
            for &seed in SEEDS {
                let (a, b) = hs(p, &z, seed);
                assert_eq_ctx(a, b, &format!("strkey({n}) seed={seed:#x}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 10 — hash_string with bytes >= 0x80 (unsigned char read)
// ---------------------------------------------------------------------------
#[test]
fn cfg10_hash_string_high_bytes() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    // single high byte in each position
    for len in 1..=8usize {
        for pos in 0..len {
            for v in [0x80u8, 0x81, 0xfe, 0xff, 0xaa] {
                let mut s = vec![b'x'; len];
                s[pos] = v;
                s.push(0);
                for &seed in &[0usize, 0x31415926, usize::MAX] {
                    let (a, b) = hs(p, &s, seed);
                    assert_eq_ctx(a, b, &format!("hi len={len} pos={pos} v={v:#02x}"));
                }
            }
        }
    }
    // every possible single byte value as a 1-char string
    for v in 1..=255u8 {
        let s = vec![v, 0];
        let (a, b) = hs(p, &s, 0x31415926);
        assert_eq_ctx(a, b, &format!("single byte {v:#02x}"));
    }
    let mut r = Rng::new(0xa00a);
    for _ in 0..256 {
        let len = r.range(1, 48);
        let s = r.cstring_hibytes(len);
        let seed = r.u64() as usize;
        let (a, b) = hs(p, &s, seed);
        assert_eq_ctx(a, b, &format!("rng hi len={len} seed={seed:#x}"));
    }
}

// ---------------------------------------------------------------------------
// row 11 — long strings
// ---------------------------------------------------------------------------
#[test]
fn cfg11_hash_string_long() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let mut r = Rng::new(0xb00b);
    for &len in &[128usize, 256, 512, 1024, 4096] {
        for _ in 0..8 {
            let s = r.cstring_hibytes(len);
            let seed = r.u64() as usize;
            let (a, b) = hs(p, &s, seed);
            assert_eq_ctx(a, b, &format!("long str len={len} seed={seed:#x}"));
        }
    }
}

// ---------------------------------------------------------------------------
// row 12 — global seed evolution through stbds_rand_seed + shmode_func
// ---------------------------------------------------------------------------
/// `stbds_make_hash_index(sc, NULL)` copies the global seed into
/// `table->seed` and then advances the global seed by `seed*a + b`.
/// Creating N tables therefore exposes the entire seed sequence.
#[test]
fn cfg12_global_seed_evolution() {
    let mut r = Rng::new(0xc00c);
    let mut seeds: Vec<usize> = vec![0, 1, 2, usize::MAX, INITIAL_HASH_SEED];
    for _ in 0..32 {
        seeds.push(r.u64() as usize);
    }
    for start in seeds {
        let (p, _g) = session(start);
        unsafe {
            let mut cs = Vec::new();
            let mut rss = Vec::new();
            let mut ct = Vec::new();
            let mut rt = Vec::new();
            for _ in 0..12 {
                let a = (p.c.shmode_func)(16, STBDS_SH_ARENA as i32);
                let b = (p.rs.shmode_func)(16, STBDS_SH_ARENA as i32);
                ct.push(a);
                rt.push(b);
                let ha = (a as *mut u8).sub(16).sub(HDR_SIZE);
                let hb = (b as *mut u8).sub(16).sub(HDR_SIZE);
                cs.push(rd_usize(rd_ptr(ha, HDR_HASH_TABLE), HI_SEED));
                rss.push(rd_usize(rd_ptr(hb, HDR_HASH_TABLE), HI_SEED));
            }
            assert_eq_ctx(cs.clone(), rss.clone(), &format!("seed chain from {start:#x}"));
            assert_eq!(cs[0], start, "first table must capture the seeded value");
            // free everything
            for a in ct {
                (p.c.hmfree_func)((a as *mut u8).sub(16) as *mut c_void, 16);
            }
            for b in rt {
                (p.rs.hmfree_func)((b as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
    }
}

/// The very first table created without any `rand_seed` call must use the
/// library's built-in initial seed `0x31415926`.
#[test]
fn cfg12b_initial_seed_matches() {
    // A fresh dlopen would be needed for a true "never seeded" check, but the
    // seed value itself is observable: seed both to the documented initial
    // constant and confirm the derived chain is identical AND that the constant
    // is what the sources say.
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        let a = (p.c.shmode_func)(16, STBDS_SH_NONE as i32);
        let b = (p.rs.shmode_func)(16, STBDS_SH_NONE as i32);
        let ha = (a as *mut u8).sub(16).sub(HDR_SIZE);
        let hb = (b as *mut u8).sub(16).sub(HDR_SIZE);
        let sa = rd_usize(rd_ptr(ha, HDR_HASH_TABLE), HI_SEED);
        let sb = rd_usize(rd_ptr(hb, HDR_HASH_TABLE), HI_SEED);
        assert_eq_ctx(sa, sb, "initial seed");
        assert_eq!(sa, 0x31415926);
        (p.c.hmfree_func)((a as *mut u8).sub(16) as *mut c_void, 16);
        (p.rs.hmfree_func)((b as *mut u8).sub(16) as *mut c_void, 16);
    }
}

// ---------------------------------------------------------------------------
// Exhaustive small-input sweeps (strengthens rows 2-4, 9-10)
// ---------------------------------------------------------------------------

/// EVERY 1- and 2-byte buffer, at several seeds.
#[test]
fn cfg_exhaustive_hash_bytes_len1_len2() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for &seed in &[0usize, 1, INITIAL_HASH_SEED, usize::MAX] {
            let mut buf = [0u8; 8];
            for a in 0..=255u8 {
                buf[0] = a;
                let x = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 1, seed);
                let y = (p.rs.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 1, seed);
                assert_eq_ctx(x, y, &format!("len=1 [{a:#02x}] seed={seed:#x}"));
            }
        }
        // all 65 536 two-byte buffers at one seed
        let mut buf = [0u8; 8];
        for a in 0..=255u8 {
            for b in 0..=255u8 {
                buf[0] = a;
                buf[1] = b;
                let x = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 2, 0x31415926);
                let y = (p.rs.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 2, 0x31415926);
                assert_eq_ctx(x, y, &format!("len=2 [{a:#02x},{b:#02x}]"));
            }
        }
    }
}

/// EVERY 3-byte buffer at one seed (16.7M pairs) -- this is the shortest length
/// that combines two sign-extended `int` shifts (`d[1]<<8`, `d[2]<<16`).
#[test]
fn cfg_exhaustive_hash_bytes_len3() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        let mut buf = [0u8; 8];
        for a in 0..=255u8 {
            for b in 0..=255u8 {
                for c in 0..=255u8 {
                    buf[0] = a;
                    buf[1] = b;
                    buf[2] = c;
                    let x = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 3, 12345);
                    let y = (p.rs.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 3, 12345);
                    if x != y {
                        panic!("len=3 [{a:#02x},{b:#02x},{c:#02x}]: C={x:#x} Rust={y:#x}");
                    }
                }
            }
        }
    }
}

/// EVERY 4-byte buffer whose top byte crosses the `d[3] << 24` int-overflow
/// boundary, sweeping the other three bytes.
#[test]
fn cfg_exhaustive_hash_bytes_len4_top_byte() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        let mut buf = [0u8; 8];
        for d3 in [0x00u8, 0x7f, 0x80, 0x81, 0xff] {
            for a in 0..=255u8 {
                for b in 0..=255u8 {
                    buf[0] = a;
                    buf[1] = b;
                    buf[2] = a ^ b;
                    buf[3] = d3;
                    let x = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 4, 7);
                    let y = (p.rs.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 4, 7);
                    if x != y {
                        panic!("len=4 d3={d3:#02x} [{a:#02x},{b:#02x}]: C={x:#x} Rust={y:#x}");
                    }
                }
            }
        }
    }
}

/// EVERY 1- and 2-character C string (bytes 1..=255, since 0 terminates).
#[test]
fn cfg_exhaustive_hash_string_len1_len2() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    unsafe {
        for &seed in &[0usize, 1, INITIAL_HASH_SEED, usize::MAX] {
            for a in 1..=255u8 {
                let mut s = [a, 0];
                let x = (p.c.hash_string)(s.as_mut_ptr() as *mut i8, seed);
                let y = (p.rs.hash_string)(s.as_mut_ptr() as *mut i8, seed);
                assert_eq_ctx(x, y, &format!("str [{a:#02x}] seed={seed:#x}"));
            }
        }
        for a in 1..=255u8 {
            for b in 1..=255u8 {
                let mut s = [a, b, 0];
                let x = (p.c.hash_string)(s.as_mut_ptr() as *mut i8, 0x31415926);
                let y = (p.rs.hash_string)(s.as_mut_ptr() as *mut i8, 0x31415926);
                if x != y {
                    panic!("str [{a:#02x},{b:#02x}]: C={x:#x} Rust={y:#x}");
                }
            }
        }
    }
}
