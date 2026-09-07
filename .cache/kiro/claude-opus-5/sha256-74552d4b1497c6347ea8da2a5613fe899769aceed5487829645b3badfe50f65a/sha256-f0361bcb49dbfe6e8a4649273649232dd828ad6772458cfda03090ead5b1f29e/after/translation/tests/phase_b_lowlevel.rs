//! Phase B — valid-path differential tests for the low-level, self-contained
//! entry points: hashing, array growth, and the string arena.
//!
//! CONFIGS.md rows 1-13 and 45-50.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

fn pair() -> Pair {
    load_pair()
}

// ---------------------------------------------------------------------------
// Rows 1-6: stbds_hash_bytes
// ---------------------------------------------------------------------------

fn hash_bytes_over(p: &Pair, seeds: &[usize], lens: impl Iterator<Item = usize>, tag: &str) {
    let mut rng = Rng::new(0xB17E5);
    for len in lens {
        for _rep in 0..24 {
            let mut buf = rng.bytes(len);
            let ptr = if len == 0 {
                std::ptr::null_mut()
            } else {
                buf.as_mut_ptr() as *mut c_void
            };
            for &s in seeds {
                let hc = unsafe { (p.c.hash_bytes)(ptr, len, s) };
                let hr = unsafe { (p.r.hash_bytes)(ptr, len, s) };
                assert_eq!(
                    hc, hr,
                    "{}: hash_bytes(len={}, seed={:#x}, buf={:02x?}) C={:#x} RUST={:#x}",
                    tag, len, s, buf, hc, hr
                );
            }
        }
    }
}

#[test]
fn row01_hash_bytes_default_seed() {
    let p = pair();
    hash_bytes_over(&p, &[0x31415926], 0..=64, "row01");
}

#[test]
fn row02_hash_bytes_seed_zero() {
    let p = pair();
    hash_bytes_over(&p, &[0], 0..=64, "row02");
}

#[test]
fn row03_hash_bytes_seed_max() {
    let p = pair();
    hash_bytes_over(&p, &[usize::MAX], 0..=64, "row03");
}

#[test]
fn row04_hash_bytes_random_seeds() {
    let p = pair();
    let mut sr = Rng::new(0x5EED);
    let seeds: Vec<usize> = (0..32).map(|_| sr.next_u64() as usize).collect();
    hash_bytes_over(&p, &seeds, 0..=40, "row04");
}

#[test]
fn row05_hash_bytes_long_buffers() {
    let p = pair();
    hash_bytes_over(&p, &[0x31415926, 0, 7], (65..=600).step_by(7), "row05");
}

#[test]
fn row06_hash_bytes_null_zero_len() {
    let p = pair();
    for &s in &[0usize, 1, 0x31415926, usize::MAX, 0xdead_beef_cafe_babe] {
        let hc = unsafe { (p.c.hash_bytes)(std::ptr::null_mut(), 0, s) };
        let hr = unsafe { (p.r.hash_bytes)(std::ptr::null_mut(), 0, s) };
        assert_eq!(hc, hr, "hash_bytes(NULL,0,{:#x})", s);
    }
}

#[test]
fn row08_hash_bytes_high_bit_bytes_every_remainder() {
    // Targets the `d[3] << 24` / `d[6] << 24 << 24` sign-extension chain.
    let p = pair();
    for len in 1..=24usize {
        for pos in 0..len {
            for &hi in &[0x80u8, 0xff, 0xfe, 0x7f] {
                let mut buf = vec![0u8; len];
                buf[pos] = hi;
                for &s in &[0usize, 0x31415926, usize::MAX] {
                    let hc = unsafe { (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, s) };
                    let hr = unsafe { (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, s) };
                    assert_eq!(hc, hr, "len={} pos={} hi={:#x} seed={:#x}", len, pos, hi, s);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 7-8: stbds_hash_string
// ---------------------------------------------------------------------------

#[test]
fn row07_hash_string_random() {
    let p = pair();
    let mut rng = Rng::new(0x57A1);
    for len in 0..=64usize {
        for _ in 0..24 {
            let mut s = rng.cstring(len);
            for &seed in &[0usize, 1, 0x31415926, usize::MAX] {
                let hc = unsafe { (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed) };
                let hr = unsafe { (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed) };
                assert_eq!(hc, hr, "hash_string(len={},seed={:#x})", len, seed);
            }
        }
    }
}

#[test]
fn row08_hash_string_random_seeds_and_high_bytes() {
    let p = pair();
    let mut rng = Rng::new(0x9911);
    let seeds: Vec<usize> = (0..24).map(|_| rng.next_u64() as usize).collect();
    // explicit high-bit strings (`(unsigned char) *str` — no sign extension)
    let mut fixed: Vec<Vec<u8>> = vec![
        vec![0],
        vec![0x80, 0],
        vec![0xff, 0],
        vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0],
        vec![0x01, 0x80, 0x7f, 0xfe, 0],
    ];
    for len in [1usize, 3, 8, 9, 33, 100] {
        fixed.push(rng.cstring(len));
    }
    for s in fixed.iter_mut() {
        for &seed in &seeds {
            let hc = unsafe { (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed) };
            let hr = unsafe { (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed) };
            assert_eq!(hc, hr, "hash_string({:02x?},{:#x})", s, seed);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9: stbds_rand_seed + the seed baked into / advanced by a fresh index
// ---------------------------------------------------------------------------

#[test]
fn row09_rand_seed_and_seed_advance() {
    let p = pair();
    for &seed in &[
        0usize,
        1,
        0x31415926,
        usize::MAX,
        0xdead_beef,
        0x0123_4567_89ab_cdef,
    ] {
        let mut out_c = String::new();
        let mut out_r = String::new();
        for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
            unsafe {
                (api.rand_seed)(seed);
                // three successive fresh indices: each bakes in the current
                // global seed and then advances it.
                let mut tables = Vec::new();
                for _ in 0..3 {
                    let t = (api.shmode_func)(16, STBDS_SH_DEFAULT);
                    out.push_str(&dump_hash(t, 16, KeyKind::StringPtr));
                    out.push('\n');
                    tables.push(t);
                }
                for t in tables {
                    (api.hmfree_func)((t as *mut u8).sub(16) as *mut c_void, 16);
                }
            }
        }
        diff(&format!("row09 seed={:#x}", seed), &out_c, &out_r);
    }
}

// ---------------------------------------------------------------------------
// Rows 10-13: stbds_arrgrowf / stbds_arrfreef
// ---------------------------------------------------------------------------

#[test]
fn row10_arrgrowf_from_null() {
    let p = pair();
    let mut out_c = String::new();
    let mut out_r = String::new();
    for elemsize in [1usize, 4, 8, 16, 24] {
        for addlen in [0usize, 1, 3, 7] {
            for min_cap in [0usize, 1, 2, 3, 4, 5, 9, 100] {
                for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
                    unsafe {
                        let a = (api.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                        out.push_str(&format!(
                            "e={} add={} min={} -> {}\n",
                            elemsize,
                            addlen,
                            min_cap,
                            dump_arr(a, elemsize)
                        ));
                        if !a.is_null() {
                            (api.arrfreef)(a);
                        }
                    }
                }
            }
        }
    }
    diff("row10", &out_c, &out_r);
}

#[test]
fn row11_arrgrowf_growth_chain() {
    let p = pair();
    for elemsize in [1usize, 4, 8, 16, 24] {
        let mut out_c = String::new();
        let mut out_r = String::new();
        for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
            unsafe {
                // emulate `arrput` 300 times: maybegrow(1) then length++
                let mut a: *mut c_void = std::ptr::null_mut();
                for i in 0..300usize {
                    let need = if a.is_null() {
                        true
                    } else {
                        (*header_of_arr(a)).length + 1 > (*header_of_arr(a)).capacity
                    };
                    if need {
                        a = (api.arrgrowf)(a, elemsize, 1, 0);
                    }
                    let h = header_of_arr(a);
                    let e = (a as *mut u8).add((*h).length * elemsize);
                    for b in 0..elemsize {
                        *e.add(b) = (i as u8).wrapping_add(b as u8);
                    }
                    (*h).length += 1;
                    (*h).temp = i as isize;
                    out.push_str(&format!("{} ", (*h).capacity));
                }
                out.push_str(&dump_arr(a, elemsize));
                (api.arrfreef)(a);
            }
        }
        diff(&format!("row11 elemsize={}", elemsize), &out_c, &out_r);
    }
}

#[test]
fn row12_arrgrowf_early_return_identity() {
    let p = pair();
    let mut out_c = String::new();
    let mut out_r = String::new();
    for elemsize in [1usize, 8, 24] {
        for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
            unsafe {
                let a = (api.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
                let h = header_of_arr(a);
                // deterministic element bytes: the C never initialises the
                // payload, so the test must.
                std::ptr::write_bytes(a as *mut u8, 0xAB, elemsize * (*h).capacity);
                (*h).length = 5;
                (*h).temp = -7;
                let cap = (*h).capacity;
                for min_cap in 0..=cap {
                    let b = (api.arrgrowf)(a, elemsize, 0, min_cap);
                    out.push_str(&format!(
                        "min={} same_ptr={} {}\n",
                        min_cap,
                        b == a,
                        dump_arr(b, elemsize)
                    ));
                }
                // addlen keeps min_len <= cap -> still an early return
                for addlen in 0..=(cap - 5) {
                    let b = (api.arrgrowf)(a, elemsize, addlen, 0);
                    out.push_str(&format!(
                        "add={} same_ptr={} {}\n",
                        addlen,
                        b == a,
                        dump_arr(b, elemsize)
                    ));
                }
                (api.arrfreef)(a);
            }
        }
    }
    diff("row12", &out_c, &out_r);
}

#[test]
fn row13_arrgrowf_random_triples() {
    let p = pair();
    let mut rng = Rng::new(0xA22C);
    let mut out_c = String::new();
    let mut out_r = String::new();
    for _ in 0..400 {
        let elemsize = 1 + (rng.below(32) as usize);
        let addlen = match rng.below(3) {
            0 => 0,
            1 => rng.below(10) as usize,
            _ => rng.below(1000) as usize,
        };
        let min_cap = match rng.below(3) {
            0 => 0,
            1 => rng.below(10) as usize,
            _ => rng.below(4096) as usize,
        };
        for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
            unsafe {
                let a = (api.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                out.push_str(&format!(
                    "e={} add={} min={} -> null={} cap={}\n",
                    elemsize,
                    addlen,
                    min_cap,
                    a.is_null(),
                    if a.is_null() {
                        0
                    } else {
                        (*header_of_arr(a)).capacity
                    }
                ));
                if !a.is_null() {
                    (api.arrfreef)(a);
                }
            }
        }
    }
    diff("row13", &out_c, &out_r);
}

// ---------------------------------------------------------------------------
// Rows 45-50: stbds_stralloc / stbds_strreset
// ---------------------------------------------------------------------------

/// Canonical, pointer-independent description of a `stralloc` result.
unsafe fn stralloc_repr(a: *const StringArena, p: *const c_char) -> String {
    // chain of blocks
    let mut blocks: Vec<*mut u8> = Vec::new();
    let mut x = (*a).storage as *mut u8;
    while !x.is_null() && blocks.len() < 4096 {
        blocks.push(x);
        x = *(x as *mut *mut u8);
    }
    // which block does `p` live in?  the one with the greatest storage base <= p
    let mut best: Option<(usize, isize)> = None;
    for (i, b) in blocks.iter().enumerate() {
        let base = b.add(8);
        let off = (p as *const u8).offset_from(base);
        if off >= 0 {
            if best.map(|(_, bo)| off < bo).unwrap_or(true) {
                best = Some((i, off));
            }
        }
    }
    format!(
        "{} chain={} block_idx={:?} content={}",
        dump_arena(a),
        blocks.len(),
        best,
        cstr_repr(p)
    )
}

#[test]
fn row45_stralloc_random_sequence() {
    let p = pair();
    for mode in [
        STBDS_SH_NONE,
        STBDS_SH_DEFAULT,
        STBDS_SH_STRDUP,
        STBDS_SH_ARENA,
    ] {
        let mut rng = Rng::new(0x5A11u64.wrapping_add(mode as u32 as u64));
        let lens: Vec<usize> = (0..300)
            .map(|_| match rng.below(5) {
                0 => 0,
                1 => rng.below(16) as usize,
                2 => rng.below(600) as usize,
                3 => rng.below(2000) as usize,
                _ => 500 + rng.below(80) as usize,
            })
            .collect();
        let strs: Vec<Vec<u8>> = {
            let mut r2 = Rng::new(0xBEEFu64.wrapping_add(mode as u32 as u64));
            lens.iter().map(|&l| r2.ascii_cstring(l)).collect()
        };

        let mut out_c = String::new();
        let mut out_r = String::new();
        for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
            unsafe {
                let mut arena = StringArena {
                    storage: std::ptr::null_mut(),
                    remaining: 0,
                    block: 0,
                    mode: mode as u8,
                };
                for s in strs.iter() {
                    let mut sc = s.clone();
                    let q = (api.stralloc)(&mut arena, sc.as_mut_ptr() as *mut c_char);
                    out.push_str(&stralloc_repr(&arena, q));
                    out.push('\n');
                }
                (api.strreset)(&mut arena);
                out.push_str(&dump_arena(&arena));
            }
        }
        diff(&format!("row45 mode={}", mode), &out_c, &out_r);
    }
}

#[test]
fn row46_stralloc_preset_block() {
    let p = pair();
    let mut out_c = String::new();
    let mut out_r = String::new();
    // block values whose blocksize stays allocatable (see CONFIGS.md A11)
    for block in [0u8, 1, 2, 3, 10, 21, 22, 23, 110, 111, 128, 255] {
        for slen in [0usize, 1, 7, 511, 512, 513, 1000] {
            for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
                unsafe {
                    let mut arena = StringArena {
                        storage: std::ptr::null_mut(),
                        remaining: 0,
                        block,
                        mode: STBDS_SH_ARENA as u8,
                    };
                    let mut s = vec![b'z'; slen];
                    s.push(0);
                    let q = (api.stralloc)(&mut arena, s.as_mut_ptr() as *mut c_char);
                    out.push_str(&format!(
                        "block={} slen={} -> {}\n",
                        block,
                        slen,
                        stralloc_repr(&arena, q)
                    ));
                    (api.strreset)(&mut arena);
                }
            }
        }
    }
    diff("row46", &out_c, &out_r);
}

#[test]
fn row47_row48_stralloc_preset_remaining_and_oversized() {
    let p = pair();
    let mut out_c = String::new();
    let mut out_r = String::new();
    for first_len in [0usize, 5, 100, 511] {
        for second_len in [0usize, 1, 5, 100, 511, 512, 513, 5000] {
            for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
                unsafe {
                    let mut arena = StringArena {
                        storage: std::ptr::null_mut(),
                        remaining: 0,
                        block: 0,
                        mode: STBDS_SH_ARENA as u8,
                    };
                    let mut s1 = vec![b'a'; first_len];
                    s1.push(0);
                    let q1 = (api.stralloc)(&mut arena, s1.as_mut_ptr() as *mut c_char);
                    out.push_str(&format!("  1st {}\n", stralloc_repr(&arena, q1)));
                    // walk `remaining` across the len boundary
                    for delta in [-1isize, 0, 1] {
                        let mut a2 = arena;
                        let want = (second_len + 1) as isize + delta;
                        if want < 0 {
                            continue;
                        }
                        if (want as usize) > a2.remaining {
                            continue;
                        }
                        a2.remaining = want as usize;
                        let mut s2 = vec![b'b'; second_len];
                        s2.push(0);
                        let q2 = (api.stralloc)(&mut a2, s2.as_mut_ptr() as *mut c_char);
                        out.push_str(&format!(
                            "  rem={} 2nd {}\n",
                            want,
                            stralloc_repr(&a2, q2)
                        ));
                        arena.storage = a2.storage;
                    }
                    // oversized with a live head block (`sb` chains after head)
                    let mut s3 = vec![b'c'; 4096];
                    s3.push(0);
                    let q3 = (api.stralloc)(&mut arena, s3.as_mut_ptr() as *mut c_char);
                    out.push_str(&format!("  over {}\n", stralloc_repr(&arena, q3)));
                    (api.strreset)(&mut arena);
                    out.push_str(&format!("  reset {}\n", dump_arena(&arena)));
                }
            }
        }
    }
    diff("row47_48", &out_c, &out_r);
}

#[test]
fn row48_stralloc_oversized_null_storage() {
    let p = pair();
    let mut out_c = String::new();
    let mut out_r = String::new();
    for slen in [512usize, 513, 1000, 4096, 100000] {
        for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
            unsafe {
                let mut arena = StringArena {
                    storage: std::ptr::null_mut(),
                    remaining: 0,
                    block: 0,
                    mode: STBDS_SH_ARENA as u8,
                };
                let mut s = vec![b'q'; slen];
                s.push(0);
                let q = (api.stralloc)(&mut arena, s.as_mut_ptr() as *mut c_char);
                out.push_str(&format!(
                    "slen={} -> {}\n",
                    slen,
                    stralloc_repr(&arena, q)
                ));
                (api.strreset)(&mut arena);
            }
        }
    }
    diff("row48", &out_c, &out_r);
}

#[test]
fn row49_strreset_shapes() {
    let p = pair();
    let mut out_c = String::new();
    let mut out_r = String::new();
    for nstr in [0usize, 1, 2, 10, 50] {
        for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
            unsafe {
                let mut arena = StringArena {
                    storage: std::ptr::null_mut(),
                    remaining: 0,
                    block: 7,
                    mode: STBDS_SH_ARENA as u8,
                };
                let mut r = Rng::new(0xC0FEu64.wrapping_add(nstr as u64));
                for _ in 0..nstr {
                    let n = r.below(900) as usize;
                    let mut s = r.ascii_cstring(n);
                    (api.stralloc)(&mut arena, s.as_mut_ptr() as *mut c_char);
                }
                out.push_str(&format!("nstr={} before={}", nstr, dump_arena(&arena)));
                (api.strreset)(&mut arena);
                out.push_str(&format!(" after={}\n", dump_arena(&arena)));
                // idempotent second reset
                (api.strreset)(&mut arena);
                out.push_str(&format!(" again={}\n", dump_arena(&arena)));
            }
        }
    }
    diff("row49", &out_c, &out_r);
}

// ---------------------------------------------------------------------------
// Row 50: strkey
// ---------------------------------------------------------------------------

#[test]
fn row50_strkey() {
    let p = pair();
    let mut rng = Rng::new(0x517E);
    let mut vals: Vec<c_int> = vec![0, 1, -1, 7, -7, i32::MIN, i32::MAX, 100000, -100000];
    for _ in 0..200 {
        vals.push(rng.next_u64() as i32);
    }
    let mut out_c = String::new();
    let mut out_r = String::new();
    for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
        unsafe {
            let mut prev: *mut c_char = std::ptr::null_mut();
            for &n in &vals {
                let q = (api.strkey)(n);
                out.push_str(&format!(
                    "{} -> {} stable={}\n",
                    n,
                    cstr_repr(q),
                    prev.is_null() || prev == q
                ));
                prev = q;
            }
            // repeated calls share the static buffer
            let a = (api.strkey)(1234);
            let b = (api.strkey)(-9);
            out.push_str(&format!(
                "shared={} a={} b={}\n",
                a == b,
                cstr_repr(a),
                cstr_repr(b)
            ));
        }
    }
    diff("row50", &out_c, &out_r);
}

/// Row 13 (continued): `size_t` wrap-around in `arrgrowf`'s size arithmetic.
///
/// The values are crafted so that `elemsize * min_cap` wraps to a *small*
/// number, i.e. the `realloc` that the C performs still succeeds and the header
/// write stays in bounds.  (Combinations whose wrapped size is < 32 bytes make
/// the C itself overflow its own allocation, which is UB in both
/// implementations and therefore not differentially observable.)
#[test]
fn row13b_arrgrowf_size_wraparound() {
    let p = pair();
    let mut out_c = String::new();
    let mut out_r = String::new();

    // elemsize * min_cap == 2^64 + 4096  ->  wraps to 4096
    let cases: [(usize, usize, usize); 6] = [
        (16, 0, (1usize << 60) + 256),          // min_cap wraps the product
        (16, (1usize << 60) + 256, 0),          // addlen -> min_len wraps the product
        (8, 0, (1usize << 61) + 512),
        (8, (1usize << 61) + 512, 0),
        (32, 0, (1usize << 59) + 128),
        (4, (1usize << 62) + 1024, 0),
    ];
    for (elemsize, addlen, min_cap) in cases {
        for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
            unsafe {
                let a = (api.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                out.push_str(&format!(
                    "e={} add={} min={} -> null={} len={} cap={} temp={} ht={}\n",
                    elemsize,
                    addlen,
                    min_cap,
                    a.is_null(),
                    (*header_of_arr(a)).length,
                    (*header_of_arr(a)).capacity,
                    (*header_of_arr(a)).temp,
                    (*header_of_arr(a)).hash_table.is_null(),
                ));
                (api.arrfreef)(a);
            }
        }
    }

    // `min_len = arrlen + addlen` wraps to 0 on an existing array -> early return
    for elemsize in [1usize, 8, 24] {
        for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
            unsafe {
                let a = (api.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
                std::ptr::write_bytes(a as *mut u8, 0xCD, elemsize * (*header_of_arr(a)).capacity);
                (*header_of_arr(a)).length = 5;
                (*header_of_arr(a)).temp = 42;
                let b = (api.arrgrowf)(a, elemsize, usize::MAX - 4, 0);
                out.push_str(&format!(
                    "wrap-min_len e={} same={} {}\n",
                    elemsize,
                    a == b,
                    dump_arr(b, elemsize)
                ));
                (api.arrfreef)(a);
            }
        }
    }
    diff("row13b", &out_c, &out_r);
}
