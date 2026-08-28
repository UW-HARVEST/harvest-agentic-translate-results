//! Phase B — long randomized property runs that mix every entry point.
//!
//! The per-row tests each pin one configuration; these runs interleave them so
//! that composed states (rehash-of-a-rehashed-table, tombstone reuse across a
//! shrink, arena growth while a map is churning, ...) get hit as well. Every
//! op is compared byte-for-byte immediately.
mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

// --- long hash-function sweep, including big buffers -------------------------
#[test]
fn stress_hash_long_buffers() {
    let p = fresh_pair(1);
    let mut rng = Rng::new(0xFEED_0001);
    for _ in 0..1500 {
        let len = match rng.below(4) {
            0 => rng.below(8),
            1 => rng.below(64),
            2 => rng.below(512),
            _ => rng.below(4096),
        };
        let mut b = rng.bytes(len + 8);
        let seed = match rng.below(4) {
            0 => 0,
            1 => usize::MAX,
            2 => 1usize << rng.below(64),
            _ => rng.next_u64() as usize,
        };
        unsafe {
            same_val(
                &format!("stress hash_bytes len={len} seed={seed:#x}"),
                (p.c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed),
                (p.r.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed),
            );
        }
        // and as a string (NUL-terminate a copy so lengths vary too)
        let mut s: Vec<u8> = b[..len].iter().map(|&x| if x == 0 { 1 } else { x }).collect();
        s.push(0);
        unsafe {
            same_val(
                &format!("stress hash_string len={len} seed={seed:#x}"),
                (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
                (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
            );
        }
    }
}

// --- long array-macro pipeline over many element sizes -----------------------
#[test]
fn stress_array_pipeline() {
    let p = fresh_pair(2);
    for &elemsize in &[1usize, 2, 3, 5, 8, 13, 16, 24, 33, 64] {
        let mut rng = Rng::new(0xFEED_0002 ^ elemsize as u64);
        let mut a = DiffArr::new(&p, elemsize);
        for step in 0..600 {
            let len = unsafe { DiffArr::len(a.ca) };
            let ctx = format!("stress-arr elemsize={elemsize} step={step} len={len}");
            match rng.below(10) {
                0..=3 => a.put(&rng.bytes(elemsize)),
                4 => {
                    if len > 0 {
                        let (c, r) = a.pop();
                        same_val(&format!("{ctx} pop"), c, r);
                    }
                }
                5 => {
                    let (c, r) = a.addn(rng.below(17));
                    same_val(&format!("{ctx} addn"), c, r);
                }
                6 => {
                    let i = if len == 0 { 0 } else { rng.below(len + 1) };
                    a.ins(i, &rng.bytes(elemsize));
                }
                7 => {
                    if len > 0 {
                        let i = rng.below(len);
                        a.deln(i, 1 + rng.below(len - i));
                    }
                }
                8 => {
                    if len > 0 {
                        a.delswap(rng.below(len));
                    }
                }
                _ => a.setcap(rng.below(300)),
            }
            a.check(&ctx);
        }
        a.free();
    }
}

// --- very long map pipeline, all modes, all storage modes -------------------
fn map_stress(tag: &str, seed: u64, elemsize: usize, mode: c_int, shmode: Option<c_int>, steps: usize) {
    let p = fresh_pair(seed as usize);
    let string_side = mode >= HM_STRING;
    let keysize = if string_side { 8 } else { 8 };
    let keyrepr = KeyRepr::Auto;
    let mut m = match shmode {
        Some(sm) => DiffMap::shmode(&p, elemsize, keysize, mode, sm, keyrepr),
        None => DiffMap::lazy(&p, elemsize, keysize, mode, keyrepr),
    };
    // A SH_NONE-style table combined with string mode needs self-referential
    // keys so that `stbds_is_key_equal` can dereference the memcpy'd bytes.
    let needs_selfkeys = string_side
        && match shmode {
            None => false,
            Some(sm) => !matches!((sm as u32 & 0xff) as u8, 1 | 2 | 3),
        };
    let sk = if needs_selfkeys { Some(SelfKeys::new(200)) } else { None };

    let mut rng = Rng::new(seed);
    let mut ka = KeyArena::new();
    let mut live: Vec<*mut u8> = Vec::new();
    let mut dead: Vec<*mut u8> = Vec::new();
    let mut next = 0usize;
    for step in 0..steps {
        let ctx = format!("{tag} elemsize={elemsize} mode={mode} shmode={shmode:?} step={step}");
        match rng.below(12) {
            0..=4 => {
                // fresh insert (unique key so the reverse-delete rule holds)
                let k = match &sk {
                    Some(s) => {
                        if next >= s.keys.len() {
                            continue;
                        }
                        let k = s.keys[next];
                        next += 1;
                        k
                    }
                    None => {
                        let kb = if string_side {
                            let mut v = format!("s{next}_").into_bytes();
                            let t = rng.cstring_range(0, 12, ASCII);
                            v.extend_from_slice(&t[..t.len() - 1]);
                            v.push(0);
                            v
                        } else {
                            let mut v = rng.bytes(keysize);
                            v[..4].copy_from_slice(&(next as u32).to_le_bytes());
                            v
                        };
                        next += 1;
                        ka.add(&kb)
                    }
                };
                live.push(k);
                let (a, b) = m.put(k, &rng.bytes(elemsize));
                same_val(&format!("{ctx} put temp"), a, b);
            }
            5 => {
                if !live.is_empty() {
                    let k = live[rng.below(live.len())];
                    let (a, b) = m.put(k, &rng.bytes(elemsize));
                    same_val(&format!("{ctx} re-put temp"), a, b);
                }
            }
            6 | 7 => {
                if !live.is_empty() {
                    let k = live[rng.below(live.len())];
                    let (a, b) = m.get(k);
                    same_val(&format!("{ctx} get temp"), a, b);
                }
            }
            8 => {
                if !dead.is_empty() {
                    let k = dead[rng.below(dead.len())];
                    let (a, b) = m.get_ts(k);
                    same_val(&format!("{ctx} get_ts(dead) temp"), a, b);
                }
            }
            9 | 10 => {
                // delete the most recently inserted live key: keeps
                // old_index == final_index, which is required for mode >= 2
                // (see the E67 note in ERRORS.md).
                if let Some(k) = live.pop() {
                    dead.push(k);
                    let (a, b) = m.del(k, 0);
                    same_val(&format!("{ctx} del temp"), a, b);
                }
            }
            _ => {
                m.put_default(&rng.bytes(elemsize));
            }
        }
        m.check(&ctx);
    }
    m.free();
    std::mem::forget(ka);
}

#[test]
fn stress_map_binary() {
    for (i, &elemsize) in [8usize, 16, 24, 40].iter().enumerate() {
        for &mode in &[HM_BINARY, -1, c_int::MIN] {
            map_stress(
                "stress-bin",
                0xFEED_0100u64
                    .wrapping_add(i as u64)
                    .wrapping_add(mode as u32 as u64),
                elemsize,
                mode,
                None,
                700,
            );
        }
    }
}

#[test]
fn stress_map_string_lazy() {
    for (i, &elemsize) in [8usize, 16, 24, 40].iter().enumerate() {
        for &mode in &[HM_STRING, 2, c_int::MAX] {
            map_stress(
                "stress-str-lazy",
                0xFEED_0200u64
                    .wrapping_add(i as u64)
                    .wrapping_add(mode as u32 as u64),
                elemsize,
                mode,
                None,
                700,
            );
        }
    }
}

#[test]
fn stress_map_string_shmodes() {
    for (i, &shmode) in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA, 7, 255].iter().enumerate() {
        for &elemsize in &[8usize, 16, 32] {
            for &mode in &[HM_STRING, 2] {
                map_stress(
                    "stress-str-sh",
                    0xFEED_0300u64
                        .wrapping_add(i as u64 * 7)
                        .wrapping_add(elemsize as u64),
                    elemsize,
                    mode,
                    Some(shmode),
                    400,
                );
            }
        }
    }
}

// --- arena churn interleaved with map churn ---------------------------------
#[test]
fn stress_arena_and_map_interleaved() {
    let p = fresh_pair(9);
    let elemsize = 24usize;
    let mut rng = Rng::new(0xFEED_0400);
    let mut m = DiffMap::shmode(&p, elemsize, 8, HM_STRING, SH_ARENA, KeyRepr::CharPtr);
    let mut ka = KeyArena::new();
    let mut live: Vec<*mut u8> = Vec::new();
    let mut ca = Box::new(CArena::zeroed());
    let mut ra = Box::new(CArena::zeroed());
    let mut next = 0usize;
    for step in 0..800 {
        let ctx = format!("stress-mix step={step}");
        if rng.below(3) == 0 {
            // independent arena op
            let n = rng.below(900);
            let mut s = rng.cstring(n, ASCII);
            unsafe {
                let cp = (p.c.stralloc)(&mut *ca, s.as_mut_ptr() as *mut c_char);
                let rp = (p.r.stralloc)(&mut *ra, s.as_mut_ptr() as *mut c_char);
                same_val(&format!("{ctx} stralloc content"), cstr(cp), cstr(rp));
                same_val(
                    &format!("{ctx} arena scalars"),
                    (ca.remaining, ca.block, ca.mode),
                    (ra.remaining, ra.block, ra.mode),
                );
            }
            if rng.below(30) == 0 {
                unsafe {
                    (p.c.strreset)(&mut *ca);
                    (p.r.strreset)(&mut *ra);
                }
            }
        } else if rng.below(4) != 0 || live.is_empty() {
            let mut kb = format!("m{next}_").into_bytes();
            let t = rng.cstring_range(0, 20, ASCII);
            kb.extend_from_slice(&t[..t.len() - 1]);
            kb.push(0);
            next += 1;
            let k = ka.add(&kb);
            live.push(k);
            let (a, b) = m.put(k, &rng.bytes(elemsize));
            same_val(&format!("{ctx} put temp"), a, b);
        } else if let Some(k) = live.pop() {
            let (a, b) = m.del(k, 0);
            same_val(&format!("{ctx} del temp"), a, b);
        }
        m.check(&ctx);
    }
    unsafe {
        (p.c.strreset)(&mut *ca);
        (p.r.strreset)(&mut *ra);
    }
    m.free();
    std::mem::forget(ka);
}

// --- rand_seed re-seeding in the middle of a workload -----------------------
#[test]
fn stress_reseed_midflight() {
    let p = pair();
    let elemsize = 16usize;
    let mut rng = Rng::new(0xFEED_0500);
    unsafe {
        (p.c.rand_seed)(0xAAAA);
        (p.r.rand_seed)(0xAAAA);
    }
    let mut maps: Vec<DiffMap> = Vec::new();
    let mut arenas: Vec<KeyArena> = Vec::new();
    for round in 0..14 {
        // reseed between map creations: each new table captures the *current*
        // global seed, then advances it by the LCG step
        let s = rng.next_u64() as usize;
        unsafe {
            (p.c.rand_seed)(s);
            (p.r.rand_seed)(s);
        }
        let mut m = DiffMap::lazy(&p, elemsize, 8, HM_BINARY, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        for i in 0..40 {
            let mut kb = rng.bytes(8);
            kb[..4].copy_from_slice(&(i as u32).to_le_bytes());
            let k = ka.add(&kb);
            m.put(k, &rng.bytes(elemsize));
            m.check(&format!("stress-reseed round={round} put#{i}"));
        }
        unsafe {
            same_val(
                &format!("stress-reseed round={round} captured seed"),
                table_of(m.ct, elemsize).unwrap().seed,
                table_of(m.rt, elemsize).unwrap().seed,
            );
        }
        maps.push(m);
        arenas.push(ka);
    }
    // interleave operations across all the live maps
    for step in 0..600 {
        let i = rng.below(maps.len());
        let mut kb = rng.bytes(8);
        kb[..4].copy_from_slice(&(rng.next_u32() % 40).to_le_bytes());
        let k = arenas[i].add(&kb);
        match rng.below(3) {
            0 => {
                let v = rng.bytes(elemsize);
                let (a, b) = maps[i].put(k, &v);
                same_val(&format!("stress-reseed x step={step} put"), a, b);
            }
            1 => {
                let (a, b) = maps[i].get(k);
                same_val(&format!("stress-reseed x step={step} get"), a, b);
            }
            _ => {
                let (a, b) = maps[i].get_ts(k);
                same_val(&format!("stress-reseed x step={step} get_ts"), a, b);
            }
        }
        maps[i].check(&format!("stress-reseed x step={step} map#{i}"));
    }
    for mut m in maps {
        m.free();
    }
    for ka in arenas {
        std::mem::forget(ka);
    }
}
