//! Phase B rows 1-11: `stbds_hash_bytes`, `stbds_hash_string`,
//! `stbds_rand_seed`.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_char;

const SEED: u64 = 0x5EED_1234;

fn seeds(rng: &mut Rng) -> Vec<usize> {
    let mut v = vec![0usize, 1, 2, usize::MAX, usize::MAX - 1, 0x3141_5926, 0x8000_0000_0000_0000];
    for _ in 0..8 {
        v.push(rng.next_u64() as usize);
    }
    v
}

fn cmp_hash_bytes(c: &Lib, r: &Lib, data: &mut [u8], len: usize, seed: usize, label: &str) {
    let p = data.as_mut_ptr() as *mut c_void;
    let hc = unsafe { (c.hash_bytes)(p, len, seed) };
    let hr = unsafe { (r.hash_bytes)(p, len, seed) };
    assert_eq!(hc, hr, "hash_bytes mismatch [{}] len={} seed={:#x} data={:?}", label, len, seed, &data[..len.min(data.len())]);
}

fn cmp_hash_string(c: &Lib, r: &Lib, s: &mut Vec<u8>, seed: usize, label: &str) {
    if !s.ends_with(&[0]) {
        s.push(0);
    }
    let p = s.as_mut_ptr() as *mut c_char;
    let hc = unsafe { (c.hash_string)(p, seed) };
    let hr = unsafe { (r.hash_string)(p, seed) };
    assert_eq!(hc, hr, "hash_string mismatch [{}] seed={:#x} s={:?}", label, seed, s);
}

#[test]
fn cfg_01_hash_bytes_len0() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED);
    let mut buf = rng.bytes(64);
    for s in seeds(&mut rng) {
        cmp_hash_bytes(&c, &r, &mut buf, 0, s, "len0");
    }
}

#[test]
fn cfg_02_hash_bytes_tail_arms() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 2);
    for len in 1usize..=7 {
        for _ in 0..400 {
            let mut buf = rng.bytes(64);
            for s in [0usize, 1, usize::MAX, rng.next_u64() as usize] {
                cmp_hash_bytes(&c, &r, &mut buf, len, s, "tail");
            }
        }
    }
}

#[test]
fn cfg_03_hash_bytes_len8() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..2000 {
        let mut buf = rng.bytes(16);
        let s = rng.next_u64() as usize;
        cmp_hash_bytes(&c, &r, &mut buf, 8, s, "len8");
    }
    // exhaustive-ish: every value of the sign-critical bytes d[3] and d[7]
    for hi3 in 0u16..=255 {
        for hi7 in [0u8, 1, 0x7f, 0x80, 0xff] {
            let mut buf = vec![0u8; 8];
            buf[3] = hi3 as u8;
            buf[7] = hi7;
            cmp_hash_bytes(&c, &r, &mut buf, 8, 0x3141_5926, "len8-signbytes");
        }
    }
}

#[test]
fn cfg_04_hash_bytes_word_plus_tail() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 4);
    for len in 9usize..=15 {
        for _ in 0..400 {
            let mut buf = rng.bytes(32);
            let s = rng.next_u64() as usize;
            cmp_hash_bytes(&c, &r, &mut buf, len, s, "word+tail");
        }
    }
}

#[test]
fn cfg_05_hash_bytes_multiword() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 5);
    for len in [16usize, 17, 23, 24, 31, 32, 63, 64, 65, 127, 128, 255, 256, 1000] {
        for _ in 0..200 {
            let mut buf = rng.bytes(len + 8);
            let s = rng.next_u64() as usize;
            cmp_hash_bytes(&c, &r, &mut buf, len, s, "multiword");
        }
    }
}

#[test]
fn cfg_06_hash_bytes_high_bit() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 6);
    // Force every byte to have the high bit set: exercises the `int`
    // sign-extension in both the word loop and the tail switch.
    for len in 1usize..=64 {
        for _ in 0..40 {
            let mut buf: Vec<u8> = rng.bytes(len + 8).iter().map(|b| b | 0x80).collect();
            let s = rng.next_u64() as usize;
            cmp_hash_bytes(&c, &r, &mut buf, len, s, "highbit-all");
        }
        // and one high-bit byte at each single position
        for pos in 0..len {
            let mut buf = vec![0u8; len + 8];
            buf[pos] = 0xFF;
            cmp_hash_bytes(&c, &r, &mut buf, len, 0, "highbit-single");
            let mut buf2 = vec![0xFFu8; len + 8];
            buf2[pos] = 0x00;
            cmp_hash_bytes(&c, &r, &mut buf2, len, usize::MAX, "lowbit-single");
        }
    }
}

#[test]
fn cfg_07_hash_bytes_seeds() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 7);
    let all = seeds(&mut rng);
    for len in [0usize, 1, 4, 7, 8, 9, 16, 33] {
        let mut buf = rng.bytes(64);
        for &s in &all {
            cmp_hash_bytes(&c, &r, &mut buf, len, s, "seeds");
        }
        // also every power-of-two seed
        for bit in 0..64 {
            cmp_hash_bytes(&c, &r, &mut buf, len, 1usize << bit, "seed-bit");
        }
    }
}

#[test]
fn cfg_08_hash_string_ascii() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 8);
    for len in 0usize..=64 {
        for _ in 0..60 {
            let mut s: Vec<u8> = (0..len).map(|_| 0x21 + (rng.byte() % 0x5e)).collect();
            let sd = rng.next_u64() as usize;
            cmp_hash_string(&c, &r, &mut s, sd, "ascii");
        }
    }
}

#[test]
fn cfg_09_hash_string_high_bytes() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 9);
    for len in 1usize..=64 {
        for _ in 0..40 {
            // bytes 0x80..=0xff only (never 0, which would terminate)
            let mut s: Vec<u8> = (0..len).map(|_| 0x80 | (rng.byte() & 0x7f)).collect();
            let sd = rng.next_u64() as usize;
            cmp_hash_string(&c, &r, &mut s, sd, "high");
        }
        // one high byte at each position, rest 'a'
        for pos in 0..len {
            let mut s: Vec<u8> = vec![b'a'; len];
            s[pos] = 0xFF;
            cmp_hash_string(&c, &r, &mut s, 0, "high-single");
        }
    }
}

#[test]
fn cfg_10_hash_string_seeds() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 10);
    let all = seeds(&mut rng);
    for &sd in &all {
        let mut empty: Vec<u8> = Vec::new();
        cmp_hash_string(&c, &r, &mut empty, sd, "empty");
        let mut long: Vec<u8> = (0..256).map(|_| 1 + (rng.byte() % 0xff)).collect();
        cmp_hash_string(&c, &r, &mut long, sd, "long");
    }
    for bit in 0..64 {
        let mut s = b"test_12345".to_vec();
        cmp_hash_string(&c, &r, &mut s, 1usize << bit, "seed-bit");
    }
}

#[test]
fn cfg_11_seed_sequence() {
    // `stbds_rand_seed` sets the global LCG state; every fresh hash index
    // consumes it and advances it. The whole sequence must match.
    let (c, r) = load_pair();
    let mut rng = Rng::new(SEED ^ 11);
    for start in [0usize, 1, 0x3141_5926, usize::MAX, rng.next_u64() as usize] {
        unsafe {
            (c.rand_seed)(start);
            (r.rand_seed)(start);
        }
        let mut cs = String::new();
        let mut rs = String::new();
        for _ in 0..40 {
            // stbds_shmode_func always builds a fresh index -> consumes a seed
            let mc = unsafe { (c.shmode_func)(16, SH_ARENA) };
            let mr = unsafe { (r.shmode_func)(16, SH_ARENA) };
            unsafe {
                let hc = &*header_of((mc as *mut u8).sub(16) as *mut c_void);
                let hr = &*header_of((mr as *mut u8).sub(16) as *mut c_void);
                cs.push_str(&format!("{:#x};", (*(hc.hash_table as *const HashIndex)).seed));
                rs.push_str(&format!("{:#x};", (*(hr.hash_table as *const HashIndex)).seed));
                (c.hmfree_func)((mc as *mut u8).sub(16) as *mut c_void, 16);
                (r.hmfree_func)((mr as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
        assert_eq!(cs, rs, "seed sequence mismatch for start={:#x}", start);
    }
}
