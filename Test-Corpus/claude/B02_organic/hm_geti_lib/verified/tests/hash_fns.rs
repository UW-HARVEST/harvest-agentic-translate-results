//! Phase B — CONFIGS.md rows 1..12: the pure/lowest-level entry points
//! (`stbds_hash_bytes`, `stbds_hash_string`, `stbds_rand_seed`, `strkey`).

mod common;
use common::*;
use std::ffi::{c_char, c_void};

const SEEDS: [usize; 6] = [0, 1, DEFAULT_HASH_SEED, usize::MAX, 0xdead_beef_cafe_babe, 2];

// --------------------------------------------------------------- row 1
#[test]
fn row01_hash_bytes_len_zero() {
    let (c, r) = load_both();
    unsafe {
        for &seed in &SEEDS {
            let mut buf = [0u8; 16];
            let cv = (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            let rv = (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            diff_eq_val(&format!("hash_bytes(valid,0,{seed:#x})"), cv, rv);

            // len == 0 dereferences nothing, so NULL is a legal argument.
            let cv = (c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let rv = (r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            diff_eq_val(&format!("hash_bytes(NULL,0,{seed:#x})"), cv, rv);
        }
    }
}

// ---------------------------------------------------------- rows 2..7
fn hash_bytes_sweep(name: &str, lens: &[usize], gen: impl Fn(&mut Rng, usize) -> Vec<u8>, iters: usize) {
    let (c, r) = load_both();
    let mut rng = Rng::new(0xA11CE ^ name.len() as u64);
    unsafe {
        for &len in lens {
            for it in 0..iters {
                let mut buf = gen(&mut rng, len);
                // over-allocate so a hypothetical over-read is at least mapped
                buf.resize(len + 32, 0xAA);
                let seed = if it % 3 == 0 {
                    SEEDS[it % SEEDS.len()]
                } else {
                    rng.next_usize()
                };
                let cv = (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                let rv = (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                diff_eq_val(
                    &format!("{name}: hash_bytes(len={len}, seed={seed:#x}, it={it})"),
                    cv,
                    rv,
                );
            }
        }
    }
}

#[test]
fn row02_hash_bytes_tail_only() {
    // len 1..7 -> switch cases 1..7, no main-loop iteration
    hash_bytes_sweep("row02", &[1, 2, 3, 4, 5, 6, 7], |g, n| g.bytes(n), 512);
}

#[test]
fn row03_hash_bytes_one_block_exact() {
    hash_bytes_sweep("row03", &[8], |g, n| g.bytes(n), 4096);
}

#[test]
fn row04_hash_bytes_one_block_plus_tail() {
    hash_bytes_sweep("row04", &[9, 10, 11, 12, 13, 14, 15], |g, n| g.bytes(n), 512);
}

#[test]
fn row05_hash_bytes_multi_block() {
    let lens: Vec<usize> = (1..=32).map(|k| k * 8).collect();
    hash_bytes_sweep("row05", &lens, |g, n| g.bytes(n), 128);
}

#[test]
fn row06_hash_bytes_high_bit_bytes() {
    // Every byte >= 0x80 forces `d[3] << 24` / `d[7] << 24` negative in `int`
    // arithmetic and thus sign-extension into size_t.
    let lens: Vec<usize> = (0..=64).collect();
    hash_bytes_sweep(
        "row06",
        &lens,
        |g, n| (0..n).map(|_| 0x80 | (g.byte() & 0x7f)).collect(),
        64,
    );
}

#[test]
fn row07_hash_bytes_boundary_bytes() {
    const CHOICES: [u8; 5] = [0x00, 0x01, 0x7f, 0x80, 0xff];
    let lens: Vec<usize> = (0..=40).collect();
    hash_bytes_sweep(
        "row07",
        &lens,
        |g, n| (0..n).map(|_| CHOICES[(g.below(5)) as usize]).collect(),
        64,
    );
}

// ---------------------------------------------------------- rows 8..10
fn hash_string_sweep(name: &str, gen: impl Fn(&mut Rng, usize) -> Vec<u8>, lens: &[usize], iters: usize) {
    let (c, r) = load_both();
    let mut rng = Rng::new(0xB0B ^ name.len() as u64);
    unsafe {
        for &len in lens {
            for it in 0..iters {
                let mut s = gen(&mut rng, len);
                s.push(0); // NUL terminate
                let seed = if it % 3 == 0 {
                    SEEDS[it % SEEDS.len()]
                } else {
                    rng.next_usize()
                };
                let cv = (c.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                let rv = (r.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                diff_eq_val(
                    &format!("{name}: hash_string(len={len}, seed={seed:#x}, it={it})"),
                    cv,
                    rv,
                );
            }
        }
    }
}

#[test]
fn row08_hash_string_ascii() {
    let lens = [0usize, 1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 32, 63, 64, 65, 127, 200, 255];
    hash_string_sweep(
        "row08",
        |g, n| (0..n).map(|_| 0x21 + (g.below(0x5e) as u8)).collect(),
        &lens,
        128,
    );
}

#[test]
fn row09_hash_string_high_bytes() {
    // bytes 0x80..0xff: `char` is signed on x86-64, the C code casts to
    // `(unsigned char)` before adding.
    let lens: Vec<usize> = (0..=48).collect();
    hash_string_sweep(
        "row09",
        |g, n| (0..n).map(|_| 0x80 | (g.byte() & 0x7f)).collect(),
        &lens,
        64,
    );
}

#[test]
fn row10_hash_string_and_bytes_reserved_hash_values() {
    // Search for inputs whose raw hash is 0 or 1 (the reserved
    // STBDS_HASH_EMPTY / STBDS_HASH_DELETED values that get the `+= 2` fixup)
    // and make sure both libraries agree on the raw value everywhere.
    let (c, r) = load_both();
    let mut rng = Rng::new(0xC0FFEE);
    let mut low = 0usize;
    unsafe {
        for _ in 0..200_000 {
            let n = 1 + rng.below(12) as usize;
            let mut b = rng.bytes(n);
            let seed = rng.next_usize();
            let cv = (c.hash_bytes)(b.as_mut_ptr() as *mut c_void, n, seed);
            let rv = (r.hash_bytes)(b.as_mut_ptr() as *mut c_void, n, seed);
            diff_eq_val("row10 hash_bytes", cv, rv);
            if cv < 2 {
                low += 1;
            }
            let mut s: Vec<u8> = (0..n).map(|_| 1 + (rng.byte() % 255)).collect();
            s.push(0);
            let cv = (c.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
            let rv = (r.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
            diff_eq_val("row10 hash_string", cv, rv);
            if cv < 2 {
                low += 1;
            }
        }
    }
    // (informational — the `hash < 2` fixup itself is covered by row 28 of
    // ERRORS.md through the map tests)
    println!("row10: {low} raw hashes below 2 observed");
}

// --------------------------------------------------------------- row 11
#[test]
fn row11_rand_seed_sequence() {
    // `stbds_rand_seed` sets the global seed; every `make_hash_index` consumes
    // it and advances it with an LCG.  Creating N tables must produce the exact
    // same seed sequence in both libraries.
    let _g = global_lock();
    let (c, r) = load_both();
    let cfg = MapCfg::binary(8, 4);
    let mut rng = Rng::new(11);
    unsafe {
        let mut seeds: Vec<usize> = SEEDS.to_vec();
        for _ in 0..8 {
            seeds.push(rng.next_usize());
        }
        for &s in &seeds {
            (c.rand_seed)(s);
            (r.rand_seed)(s);
            let mut cts: Vec<*mut c_void> = Vec::new();
            let mut rts: Vec<*mut c_void> = Vec::new();
            for t in 0..12 {
                let key = (t as u32).to_le_bytes();
                let ct = map_put_binary(&c, std::ptr::null_mut(), &cfg, &key, &[1, 2, 3, 4]);
                let rt = map_put_binary(&r, std::ptr::null_mut(), &cfg, &key, &[1, 2, 3, 4]);
                diff_eq(
                    &format!("row11 seed={s:#x} table#{t}"),
                    &snapshot_map(ct, cfg.elemsize, cfg.kind),
                    &snapshot_map(rt, cfg.elemsize, cfg.kind),
                );
                cts.push(ct);
                rts.push(rt);
            }
            for (a, b) in cts.iter().zip(rts.iter()) {
                map_free(&c, *a, cfg.elemsize);
                map_free(&r, *b, cfg.elemsize);
            }
        }
    }
}

// --------------------------------------------------------------- row 12
#[test]
fn row12_strkey() {
    // `strkey` writes into a process-global `static char buffer[256]`.
    let _g = global_lock();
    let (c, r) = load_both();
    let mut rng = Rng::new(12);
    let mut ns: Vec<i32> = vec![
        0,
        1,
        -1,
        9,
        10,
        99,
        100,
        999,
        1000,
        12345,
        -12345,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    for _ in 0..512 {
        ns.push(rng.i32());
    }
    unsafe {
        for n in ns {
            let cp = (c.strkey)(n);
            let rp = (r.strkey)(n);
            diff_eq_val(&format!("strkey({n})"), cstr(cp), cstr(rp));
            // the returned pointer must be the *same* static buffer every call
            let cp2 = (c.strkey)(n);
            let rp2 = (r.strkey)(n);
            diff_eq_val(&format!("strkey({n}) buffer identity"), cp == cp2, rp == rp2);
        }
    }
}
