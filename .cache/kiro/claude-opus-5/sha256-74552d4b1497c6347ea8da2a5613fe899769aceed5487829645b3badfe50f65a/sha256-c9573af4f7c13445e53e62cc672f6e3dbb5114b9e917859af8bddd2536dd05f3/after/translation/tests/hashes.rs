//! Phase B rows 1–10: the two pure hash primitives, plus the seed plumbing.

mod common;
use common::*;
use std::ffi::c_void;

fn both<T: PartialEq + std::fmt::Debug>(row: &str, ctx: &str, c: T, r: T) {
    assert_eq!(c, r, "CONFIGS row {row}: divergence at {ctx}");
}

// --- rows 1..7: stbds_hash_bytes -----------------------------------------

fn sweep_hash_bytes(row: &str, seed: usize, mk: &mut dyn FnMut(usize) -> Vec<u8>, lens: &[usize]) {
    let (c, r) = libs();
    for &len in lens {
        for rep in 0..8 {
            let mut buf = mk(len);
            // guarantee len bytes are addressable even for len == 0
            buf.resize(len.max(1), 0);
            let p = buf.as_mut_ptr() as *mut c_void;
            let hc = unsafe { (c.hash_bytes)(p, len, seed) };
            let hr = unsafe { (r.hash_bytes)(p, len, seed) };
            both(row, &format!("len={len} rep={rep} seed={seed:#x} bytes={:x?}", &buf[..len.min(buf.len())]), hc, hr);
        }
    }
}

#[test]
fn row01_hash_bytes_random_lengths_0_64() {
    let mut rng = Rng::new(0xC0FFEE01);
    sweep_hash_bytes(
        "1",
        0x31415926,
        &mut |n| rng.bytes(n),
        &(0..=64).collect::<Vec<_>>(),
    );
}

#[test]
fn row02_hash_bytes_high_bit_bytes() {
    let mut rng = Rng::new(0xC0FFEE02);
    // every byte >= 0x80 -> exercises the `d[3] << 24` / `d[7] << 24` int
    // sign-extension in both the main loop and the switch tail
    sweep_hash_bytes(
        "2",
        0x31415926,
        &mut |n| (0..n).map(|_| 0x80 | (rng.byte() & 0x7f)).collect(),
        &(0..=64).collect::<Vec<_>>(),
    );
    // and specifically: high bit only at offsets 3 and 7 of each block
    let (c, r) = libs();
    for len in 0usize..=40 {
        for &hi_at in &[3usize, 7, 11, 15] {
            let mut buf: Vec<u8> = (0..len.max(1)).map(|i| (i as u8) & 0x7f).collect();
            if hi_at < len {
                buf[hi_at] = 0xff;
            }
            let p = buf.as_mut_ptr() as *mut c_void;
            let hc = unsafe { (c.hash_bytes)(p, len, 0x31415926) };
            let hr = unsafe { (r.hash_bytes)(p, len, 0x31415926) };
            both("2", &format!("len={len} hi_at={hi_at}"), hc, hr);
        }
    }
}

#[test]
fn row03_hash_bytes_all_zero_all_ff() {
    sweep_hash_bytes("3", 0x31415926, &mut |n| vec![0u8; n], &(0..=64).collect::<Vec<_>>());
    sweep_hash_bytes("3", 0x31415926, &mut |n| vec![0xffu8; n], &(0..=64).collect::<Vec<_>>());
}

#[test]
fn row04_hash_bytes_tail_only() {
    let mut rng = Rng::new(0xC0FFEE04);
    sweep_hash_bytes("4", 0x31415926, &mut |n| rng.bytes(n), &[0, 1, 2, 3, 4, 5, 6, 7]);
    let mut rng2 = Rng::new(0xC0FFEE44);
    sweep_hash_bytes(
        "4",
        0,
        &mut |n| (0..n).map(|_| 0x80 | (rng2.byte() & 0x7f)).collect(),
        &[0, 1, 2, 3, 4, 5, 6, 7],
    );
}

#[test]
fn row05_hash_bytes_exact_multiples() {
    let mut rng = Rng::new(0xC0FFEE05);
    sweep_hash_bytes("5", 0x31415926, &mut |n| rng.bytes(n), &[8, 16, 24, 32, 40, 48, 56, 64]);
}

#[test]
fn row06_hash_bytes_large() {
    let mut rng = Rng::new(0xC0FFEE06);
    sweep_hash_bytes("6", 0x31415926, &mut |n| rng.bytes(n), &[255, 256, 257, 1023, 1024, 4096]);
}

#[test]
fn row07_hash_bytes_seed_matrix() {
    let seeds: [usize; 8] = [
        0,
        1,
        usize::MAX,
        0x31415926,
        0x8000_0000_0000_0000,
        0xdead_beef_dead_beef,
        0xffff_ffff,
        0x1234_5678_9abc_def0,
    ];
    for &s in &seeds {
        let mut rng = Rng::new(0xC0FFEE07 ^ (s as u64));
        sweep_hash_bytes("7", s, &mut |n| rng.bytes(n), &(0..=33).collect::<Vec<_>>());
    }
}

// --- rows 8..10: stbds_hash_string ---------------------------------------

fn sweep_hash_string(row: &str, seed: usize, alphabet: &[u8], lens: &[usize], rounds: usize) {
    let (c, r) = libs();
    let mut rng = Rng::new(0xBEEF_0000 ^ (seed as u64) ^ (alphabet.len() as u64));
    for &len in lens {
        for _ in 0..rounds {
            let mut s = rng.cstring(len, alphabet);
            let p = s.as_mut_ptr() as *mut std::ffi::c_char;
            let hc = unsafe { (c.hash_string)(p, seed) };
            let hr = unsafe { (r.hash_string)(p, seed) };
            both(row, &format!("len={len} seed={seed:#x} s={:x?}", &s), hc, hr);
        }
    }
}

#[test]
fn row08_hash_string_ascii() {
    sweep_hash_string("8", 0x31415926, ASCII, &(0..=64).collect::<Vec<_>>(), 8);
    // explicit empty string
    let (c, r) = libs();
    let mut e = [0i8; 1];
    let hc = unsafe { (c.hash_string)(e.as_mut_ptr(), 0x31415926) };
    let hr = unsafe { (r.hash_string)(e.as_mut_ptr(), 0x31415926) };
    both("8", "empty string", hc, hr);
}

#[test]
fn row09_hash_string_high_bit() {
    sweep_hash_string("9", 0x31415926, HIGHBIT, &(1..=64).collect::<Vec<_>>(), 8);
    // every single high byte on its own
    let (c, r) = libs();
    for b in 1u16..=255 {
        let mut s = [b as i8, 0];
        let hc = unsafe { (c.hash_string)(s.as_mut_ptr(), 0) };
        let hr = unsafe { (r.hash_string)(s.as_mut_ptr(), 0) };
        both("9", &format!("byte={b:#x}"), hc, hr);
    }
}

#[test]
fn row10_hash_string_seed_matrix() {
    for &s in &[0usize, 1, usize::MAX, 0x31415926, 0xdead_beef_dead_beef] {
        sweep_hash_string("10", s, ASCII, &(0..=32).collect::<Vec<_>>(), 4);
        sweep_hash_string("10", s, HIGHBIT, &(1..=32).collect::<Vec<_>>(), 4);
    }
}

// --- row 11: rand_seed / table seed plumbing -----------------------------

#[test]
fn row11_rand_seed_and_lcg_advance() {
    let (c, r) = libs();
    for &seed in &[0usize, 1, 0x31415926, usize::MAX, 0xabcd_ef01_2345_6789] {
        unsafe {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            // Create 6 fresh tables in each library; every table captures the
            // then-current global seed and advances it by the same LCG.
            let mut cs = Vec::new();
            let mut rs = Vec::new();
            for _ in 0..6 {
                let tc = (c.shmode_func)(16, SH_DEFAULT);
                let tr = (r.shmode_func)(16, SH_DEFAULT);
                cs.push(snapshot(tc, 16).seed);
                rs.push(snapshot(tr, 16).seed);
                (c.hmfree_func)((tc as *mut u8).sub(16) as *mut c_void, 16);
                (r.hmfree_func)((tr as *mut u8).sub(16) as *mut c_void, 16);
            }
            assert_eq!(cs, rs, "CONFIGS row 11: table seed sequence for seed={seed:#x}");
            assert_eq!(cs[0], seed, "first table must capture the seed verbatim");
            // the sequence must actually move (proves the LCG ran)
            assert!(cs.windows(2).any(|w| w[0] != w[1]));
        }
    }
    // restore the default so test ordering cannot matter
    unsafe {
        (c.rand_seed)(0x31415926);
        (r.rand_seed)(0x31415926);
    }
}
