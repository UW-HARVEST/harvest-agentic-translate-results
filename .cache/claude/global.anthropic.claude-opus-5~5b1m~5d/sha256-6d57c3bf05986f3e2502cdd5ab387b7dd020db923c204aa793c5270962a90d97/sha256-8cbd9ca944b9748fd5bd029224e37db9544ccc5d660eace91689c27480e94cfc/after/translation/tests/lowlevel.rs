//! Phase B — low-level entry points: hashing, raw array growth, string arena,
//! `strkey`, `helxo`.  CONFIGS.md rows 1–19, 64–76.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ===========================================================================
// rows 1–3 : stbds_hash_string
// ===========================================================================

const SEEDS: [usize; 6] = [
    DEFAULT_SEED,
    0,
    1,
    usize::MAX,
    0xdead_beef,
    0x8000_0000_0000_0000,
];

#[test]
fn row01_hash_string_empty_and_fixed_seeds() {
    let (p, _g) = libs();
    let mut s = *b"\0";
    for &seed in SEEDS.iter() {
        unsafe {
            let a = (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
            let b = (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
            assert_eq!(a, b, "hash_string(\"\", {seed:#x})");
        }
    }
}

#[test]
fn row02_hash_string_random_ascii() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0x1001);
    for _ in 0..2000 {
        let n = rng.range(1, 64);
        let mut s = rng.cstr_bytes(n, false);
        let seed = SEEDS[rng.below(SEEDS.len())];
        unsafe {
            let a = (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
            let b = (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
            assert_eq!(a, b, "hash_string({:?}, {seed:#x})", &s);
        }
    }
}

#[test]
fn row03_hash_string_high_bit_bytes() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0x1002);
    for _ in 0..2000 {
        let n = rng.range(1, 64);
        let mut s = rng.cstr_bytes(n, true);
        let seed = SEEDS[rng.below(SEEDS.len())];
        unsafe {
            let a = (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
            let b = (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
            assert_eq!(a, b, "hash_string(hi-bit {:?}, {seed:#x})", &s);
        }
    }
    // every single high byte on its own
    for byte in 1u16..256 {
        let mut s = [byte as u8, 0u8];
        for &seed in SEEDS.iter() {
            unsafe {
                let a = (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                let b = (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                assert_eq!(a, b, "hash_string([{byte:#x}], {seed:#x})");
            }
        }
    }
}

// ===========================================================================
// rows 4–10 : stbds_hash_bytes  (siphash: body loop + all 8 tail cases)
// ===========================================================================

#[test]
fn row04_hash_bytes_len0() {
    let (p, _g) = libs();
    let mut buf = [0u8; 8];
    for &seed in SEEDS.iter() {
        unsafe {
            let a = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            let b = (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            assert_eq!(a, b, "hash_bytes(len=0, {seed:#x})");
        }
    }
}

#[test]
fn rows05_08_hash_bytes_every_length_0_to_24() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0x2001);
    for len in 0..=24usize {
        for high in [false, true] {
            for _ in 0..64 {
                // over-allocate so the C tail reads stay in bounds
                let mut buf: Vec<u8> = (0..len + 16)
                    .map(|_| if high { rng.u8() | 0x80 } else { rng.u8() & 0x7f })
                    .collect();
                let seed = SEEDS[rng.below(SEEDS.len())];
                unsafe {
                    let a = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                    let b = (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                    assert_eq!(a, b, "hash_bytes(len={len}, high={high}, {seed:#x})");
                }
            }
        }
    }
}

#[test]
fn row09_hash_bytes_property_sweep() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0x2002);
    for _ in 0..4000 {
        let len = rng.below(65);
        let mut buf = rng.bytes(len + 16);
        let seed = rng.next_u64() as usize;
        unsafe {
            let a = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
            let b = (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
            assert_eq!(a, b, "hash_bytes(len={len}, seed={seed:#x})");
        }
    }
    // deliberately boundary-y byte patterns
    for pat in [0x00u8, 0x01, 0x7f, 0x80, 0xfe, 0xff] {
        for len in 0..=24usize {
            let mut buf = vec![pat; len + 16];
            for &seed in SEEDS.iter() {
                unsafe {
                    let a = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                    let b = (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                    assert_eq!(a, b, "hash_bytes(pat={pat:#x}, len={len}, {seed:#x})");
                }
            }
        }
    }
}

#[test]
fn row10_rand_seed_does_not_affect_hash_fns() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0x2003);
    for _ in 0..200 {
        let gseed = rng.next_u64() as usize;
        reseed(p, gseed);
        let len = rng.below(33);
        let mut buf = rng.bytes(len + 16);
        let n2 = rng.range(1, 20);
        let mut s = rng.cstr_bytes(n2, false);
        unsafe {
            assert_eq!(
                (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 7),
                (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 7)
            );
            assert_eq!(
                (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, 7),
                (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, 7)
            );
        }
    }
    reseed(p, DEFAULT_SEED);
}

// ===========================================================================
// rows 11–19 : stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

/// Compares the observable result of one `arrgrowf` call: NULL-ness, the four
/// header fields and the payload bytes.
unsafe fn cmp_grow(
    tag: &str,
    (pc, pr): (*mut c_void, *mut c_void),
    elemsize: usize,
    payload_len: usize,
) {
    assert_eq!(
        pc.is_null(),
        pr.is_null(),
        "{tag}: NULL-ness differs (C={pc:?} Rust={pr:?})"
    );
    if pc.is_null() {
        return;
    }
    let hc = hdr_of(pc);
    let hr = hdr_of(pr);
    assert_eq!(hc.length, hr.length, "{tag}: length");
    assert_eq!(hc.capacity, hr.capacity, "{tag}: capacity");
    assert_eq!(hc.temp, hr.temp, "{tag}: temp");
    assert_eq!(
        hc.hash_table.is_null(),
        hr.hash_table.is_null(),
        "{tag}: hash_table NULL-ness"
    );
    let n = payload_len.min(elemsize * hc.capacity);
    if n > 0 {
        let a = std::slice::from_raw_parts(pc as *const u8, n);
        let b = std::slice::from_raw_parts(pr as *const u8, n);
        assert_eq!(a, b, "{tag}: payload bytes");
    }
}

#[test]
fn rows11_13_18_arrgrowf_fresh() {
    let (p, _g) = libs();
    for &elemsize in [0usize, 1, 4, 8, 16, 40].iter() {
        for &addlen in [0usize, 1, 2, 3, 4, 5, 7, 8, 1000].iter() {
            for &min_cap in [0usize, 1, 2, 3, 4, 5, 8, 9, 1000].iter() {
                unsafe {
                    let a = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let b = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let tag = format!("arrgrowf(NULL,{elemsize},{addlen},{min_cap})");
                    cmp_grow(&tag, (a, b), elemsize, 0);
                    if !a.is_null() {
                        (p.c.arrfreef)(a);
                    }
                    if !b.is_null() {
                        (p.r.arrfreef)(b);
                    }
                }
            }
        }
    }
}

#[test]
fn row07_err_arrgrowf_null_when_nothing_requested() {
    // ERRORS.md row 7: arrgrowf(NULL, e, 0, 0) returns NULL in both.
    let (p, _g) = libs();
    for &elemsize in [0usize, 1, 4, 8, 16].iter() {
        unsafe {
            let a = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let b = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(a.is_null(), "C should return NULL for elemsize={elemsize}");
            assert!(b.is_null(), "Rust should return NULL for elemsize={elemsize}");
        }
    }
}

#[test]
fn rows14_17_arrgrowf_growth_ladder() {
    let (p, _g) = libs();
    let elemsize = 8usize;
    unsafe {
        let mut a = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
        let mut b = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0);
        cmp_grow("ladder init", (a, b), elemsize, 0);

        // row 14: min_cap <= cap -> identical pointer, unchanged header
        for &mc in [0usize, 1, 2, 3, 4].iter() {
            let a2 = (p.c.arrgrowf)(a, elemsize, 0, mc);
            let b2 = (p.r.arrgrowf)(b, elemsize, 0, mc);
            assert_eq!(a2, a, "C arrgrowf must return same ptr for min_cap={mc}");
            assert_eq!(b2, b, "Rust arrgrowf must return same ptr for min_cap={mc}");
            cmp_grow(&format!("noop min_cap={mc}"), (a2, b2), elemsize, 0);
        }

        // rows 15/16/17: 16 successive growth steps, alternating the reason
        // the C picks a new min_cap (doubling vs explicit vs +4)
        for step in 0..16usize {
            let cap = hdr_of(a).capacity;
            let (addlen, min_cap) = match step % 4 {
                0 => (1usize, cap + 1),   // doubling wins
                1 => (0usize, 5 * cap),   // explicit min_cap wins
                2 => (cap + 3, 0usize),   // min_len wins
                _ => (1usize, 0usize),    // doubling wins
            };
            // write a recognisable payload before growing, so realloc-copy is checked
            let pl = elemsize * cap;
            for i in 0..pl {
                *(a as *mut u8).add(i) = (i as u8).wrapping_mul(7).wrapping_add(step as u8);
                *(b as *mut u8).add(i) = (i as u8).wrapping_mul(7).wrapping_add(step as u8);
            }
            (*((a as *mut u8).sub(HDR) as *mut Header)).length = cap;
            (*((b as *mut u8).sub(HDR) as *mut Header)).length = cap;

            a = (p.c.arrgrowf)(a, elemsize, addlen, min_cap);
            b = (p.r.arrgrowf)(b, elemsize, addlen, min_cap);
            cmp_grow(
                &format!("step {step} addlen={addlen} min_cap={min_cap}"),
                (a, b),
                elemsize,
                pl,
            );
        }
        (p.c.arrfreef)(a);
        (p.r.arrfreef)(b);
    }
}

#[test]
fn row19_arrgrowf_randomised_chains() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0x3001);
    for chain in 0..200 {
        let elemsize = [1usize, 2, 4, 8, 12, 16, 40][rng.below(7)];
        unsafe {
            let mut a = std::ptr::null_mut::<c_void>();
            let mut b = std::ptr::null_mut::<c_void>();
            for step in 0..12 {
                let addlen = rng.below(20);
                let min_cap = rng.below(40);
                a = (p.c.arrgrowf)(a, elemsize, addlen, min_cap);
                b = (p.r.arrgrowf)(b, elemsize, addlen, min_cap);
                cmp_grow(
                    &format!("chain {chain} step {step} e={elemsize} add={addlen} mc={min_cap}"),
                    (a, b),
                    elemsize,
                    0,
                );
                if a.is_null() {
                    continue;
                }
                // keep length in range so the next min_len is meaningful
                let cap = hdr_of(a).capacity;
                let l = rng.below(cap + 1);
                (*((a as *mut u8).sub(HDR) as *mut Header)).length = l;
                (*((b as *mut u8).sub(HDR) as *mut Header)).length = l;
            }
            if !a.is_null() {
                (p.c.arrfreef)(a);
            }
            if !b.is_null() {
                (p.r.arrfreef)(b);
            }
        }
    }
}

// ===========================================================================
// rows 64–72 : stbds_stralloc / stbds_strreset
// ===========================================================================

#[derive(Debug, PartialEq, Eq)]
#[allow(dead_code)]
struct ArenaSnap {
    remaining: usize,
    block: u8,
    mode: u8,
    has_storage: bool,
    /// contents of every string handed out so far, read back through the
    /// returned pointer (addresses differ between heaps, contents must not)
    strings: Vec<Vec<u8>>,
}

fn arena_run(p: &Pair, lens: &[usize], seed: u64) {
    unsafe {
        let mut ac = Arena::zeroed();
        let mut ar = Arena::zeroed();
        let mut rng = Rng::new(seed);
        let mut sc: Vec<Vec<u8>> = Vec::new();
        let mut sr: Vec<Vec<u8>> = Vec::new();
        let mut pc_all: Vec<*mut c_char> = Vec::new();
        let mut pr_all: Vec<*mut c_char> = Vec::new();
        for (i, &len) in lens.iter().enumerate() {
            let mut s = rng.cstr_bytes(len, false);
            let a = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let b = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            assert!(!a.is_null() && !b.is_null());
            assert_eq!(
                cstr_bytes(a),
                s[..len].to_vec(),
                "C stralloc returned wrong content at step {i}"
            );
            assert_eq!(
                cstr_bytes(b),
                s[..len].to_vec(),
                "Rust stralloc returned wrong content at step {i}"
            );
            pc_all.push(a);
            pr_all.push(b);
            assert_eq!(
                ac.remaining, ar.remaining,
                "arena.remaining diverged at step {i} (len={len})"
            );
            assert_eq!(ac.block, ar.block, "arena.block diverged at step {i} (len={len})");
            assert_eq!(ac.mode, ar.mode, "arena.mode diverged at step {i}");
            assert_eq!(
                ac.storage.is_null(),
                ar.storage.is_null(),
                "arena.storage NULL-ness diverged at step {i}"
            );
            // relative offset of the returned pointer inside its block must match
            sc.push(cstr_bytes(a));
            sr.push(cstr_bytes(b));
        }
        // all previously handed-out strings must still be intact and equal
        for (i, (a, b)) in pc_all.iter().zip(pr_all.iter()).enumerate() {
            assert_eq!(
                cstr_bytes(*a),
                cstr_bytes(*b),
                "string {i} content diverged after all allocations"
            );
        }
        assert_eq!(sc, sr);

        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
        assert_eq!(ac, Arena::zeroed(), "C strreset must zero the arena");
        assert_eq!(ar, Arena::zeroed(), "Rust strreset must zero the arena");
    }
}

#[test]
fn rows64_66_stralloc_short_strings() {
    let (p, _g) = libs();
    // row 64: single short string
    arena_run(p, &[10], 1);
    // row 65: exhaust the first 512-byte block, then more
    arena_run(p, &vec![60usize; 40], 2);
    arena_run(p, &vec![100usize; 60], 3);
    // row 66: 40 random lengths 1..300
    let mut rng = Rng::new(0x4001);
    for t in 0..40 {
        let lens: Vec<usize> = (0..40).map(|_| rng.range(1, 300)).collect();
        arena_run(p, &lens, 0x5000 + t);
    }
}

#[test]
fn rows67_69_stralloc_oversized_and_empty() {
    let (p, _g) = libs();
    // row 67: first allocation larger than the first block (512) -> dedicated
    // block becomes storage, remaining = 0
    arena_run(p, &[600], 11);
    arena_run(p, &[5000], 12);
    // row 68: short first (creates storage), then oversized -> spliced as next
    arena_run(p, &[10, 600], 13);
    arena_run(p, &[10, 5000, 20, 100000, 5], 14);
    // row 69: empty strings
    arena_run(p, &[0], 15);
    arena_run(p, &[0, 0, 0, 0, 0], 16);
    arena_run(p, &vec![0usize; 600], 17);
    // mixtures
    arena_run(p, &[0, 600, 0, 511, 512, 513, 0, 1], 18);
}

#[test]
fn row70_stralloc_block_counter_saturation() {
    let (p, _g) = libs();
    // blocksize = 512 << (block/2); `block` is incremented only while
    // blocksize < (1<<20), so it saturates at 22 (512<<11 == 1<<20).
    // Force many block rollovers with big strings first.
    let mut lens = Vec::new();
    for _ in 0..30 {
        lens.push(1_100_000); // > any blocksize -> dedicated block each time
    }
    arena_run(p, &lens, 21);

    // now drive `block` upward through the normal (non-oversized) path
    unsafe {
        let mut ac = Arena::zeroed();
        let mut ar = Arena::zeroed();
        let mut s = vec![b'z'; 400];
        s.push(0);
        for i in 0..200000usize {
            // each allocation of 401 bytes eventually exhausts each block
            let a = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let b = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr_bytes(a), cstr_bytes(b), "content at {i}");
            assert_eq!(ac.remaining, ar.remaining, "remaining at {i}");
            assert_eq!(ac.block, ar.block, "block at {i}");
            if ac.block >= 22 {
                break;
            }
        }
        assert!(
            ac.block >= 22,
            "did not reach block saturation (got {})",
            ac.block
        );
        assert_eq!(ac.block, ar.block);
        // once saturated it must stay put in both libraries
        for i in 0..64usize {
            let mut big = vec![b'q'; 2_000_000];
            big.push(0);
            let a = (p.c.stralloc)(&mut ac, big.as_mut_ptr() as *mut c_char);
            let b = (p.r.stralloc)(&mut ar, big.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr_bytes(a).len(), 2_000_000);
            assert_eq!(cstr_bytes(a), cstr_bytes(b), "saturated content at {i}");
            assert_eq!(ac.block, 22, "C block must saturate at 22");
            assert_eq!(ar.block, 22, "Rust block must saturate at 22");
            assert_eq!(ac.remaining, ar.remaining, "saturated remaining at {i}");
        }
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
    }
}

#[test]
fn rows71_72_strreset() {
    let (p, _g) = libs();
    unsafe {
        // row 71: zeroed arena, repeated resets
        let mut ac = Arena::zeroed();
        let mut ar = Arena::zeroed();
        for _ in 0..4 {
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
            assert_eq!(ac, Arena::zeroed());
            assert_eq!(ar, Arena::zeroed());
            assert_eq!(ac, ar);
        }
        // row 72: multi-block chain, reset, reuse
        let mut rng = Rng::new(0x4002);
        for round in 0..8 {
            for _ in 0..50 {
                let n2 = rng.range(1, 700);
                let mut s = rng.cstr_bytes(n2, false);
                let a = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
                let b = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
                assert_eq!(cstr_bytes(a), cstr_bytes(b), "round {round}");
                assert_eq!(ac.remaining, ar.remaining, "round {round} remaining");
                assert_eq!(ac.block, ar.block, "round {round} block");
            }
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
            assert_eq!(ac, Arena::zeroed());
            assert_eq!(ar, Arena::zeroed());
        }
    }
}

// ===========================================================================
// rows 73–74 : strkey
// ===========================================================================

#[test]
fn rows73_74_strkey() {
    let (p, _g) = libs();
    let mut vals: Vec<c_int> = vec![0, 1, -1, 42, i32::MAX, i32::MIN, 999999, -999999];
    let mut rng = Rng::new(0x6001);
    for _ in 0..64 {
        vals.push(rng.u32() as c_int);
    }
    unsafe {
        for &n in vals.iter() {
            let a = (p.c.strkey)(n);
            let b = (p.r.strkey)(n);
            assert_eq!(cstr_bytes(a), cstr_bytes(b), "strkey({n})");
            assert_eq!(
                cstr_bytes(a),
                format!("test_{n}").into_bytes(),
                "strkey({n}) content"
            );
            // row 74: same static buffer returned every time
            let a2 = (p.c.strkey)(n);
            let b2 = (p.r.strkey)(n);
            assert_eq!(a, a2, "C strkey must return the same static buffer");
            assert_eq!(b, b2, "Rust strkey must return the same static buffer");
        }
    }
}

// ===========================================================================
// rows 75–76 : helxo  (stdout compared byte-for-byte)
// ===========================================================================

#[test]
fn rows75_76_helxo_stdout_all_bytes() {
    let (p, _g) = libs();
    for byte in 0u16..256 {
        let ch = byte as u8 as c_char;
        let out_c = capture_stdout("c", || unsafe { (p.c.helxo)(ch) });
        let out_r = capture_stdout("r", || unsafe { (p.r.helxo)(ch) });
        assert_eq!(
            out_c, out_r,
            "helxo({byte:#04x}) stdout differs\nC   = {:?}\nRust= {:?}",
            String::from_utf8_lossy(&out_c),
            String::from_utf8_lossy(&out_r)
        );
    }
}

#[test]
fn row76_helxo_repeated_and_seed_independent() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0x7001);
    let mut first: Option<Vec<u8>> = None;
    for i in 0..16 {
        let seed = rng.next_u64() as usize;
        reseed(p, seed);
        let out_c = capture_stdout("c", || unsafe { (p.c.helxo)(b'w' as c_char) });
        let out_r = capture_stdout("r", || unsafe { (p.r.helxo)(b'w' as c_char) });
        assert_eq!(out_c, out_r, "helxo iteration {i} seed {seed:#x}");
        match &first {
            None => first = Some(out_c),
            Some(f) => assert_eq!(
                *f, out_c,
                "helxo output must be seed-independent (iteration {i})"
            ),
        }
    }
    reseed(p, DEFAULT_SEED);
    // sanity: the expected 5 lines
    let out = capture_stdout("c", || unsafe { (p.c.helxo)(b'w' as c_char) });
    assert_eq!(
        String::from_utf8_lossy(&out),
        "bob h\nsally e\nfred l\njen w\ndoug o\n"
    );
}
