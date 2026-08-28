//! Phase B — binary-mode hash map (`STBDS_HM_BINARY`).
//! Rows C17–C27, C41–C46 of CONFIGS.md, driven through the *macro-level*
//! protocol (`stbds_hmput` / `stbds_hmgeti` / `stbds_hmgeti_ts` / `stbds_hmdel`
//! / `stbds_hmdefault`) so that every element byte is deterministic.
mod common;
use common::*;
use std::ffi::c_void;

/// (elemsize, keysize) shapes a real consumer produces with
/// `struct { K key; V value; }`.
const SHAPES: &[(usize, usize)] = &[
    (2, 1),
    (4, 2),
    (8, 4),
    (16, 8),
    (24, 8),
    (12, 4),
    (32, 16),
    (8, 8),
    (4, 4),
    (1, 1),
];

// --- C17 / E17 / E18 / E19 : hmput_default ----------------------------------
#[test]
fn c17_hmput_default() {
    let p = fresh_pair(0x17);
    let mut rng = Rng::new(0x17);
    for &(elemsize, keysize) in SHAPES {
        let mut m = DiffMap::lazy(&p, elemsize, keysize, HM_BINARY, KeyRepr::Inline);
        // E17: a == NULL
        let v = rng.bytes(elemsize);
        m.put_default(&v);
        m.check(&format!("c17 default#1 elemsize={elemsize}"));
        // E19: a != NULL, length != 0 -> no-op
        let v2 = rng.bytes(elemsize);
        m.put_default(&v2);
        m.check(&format!("c17 default#2 elemsize={elemsize}"));
        // then real inserts still work
        let mut ka = KeyArena::new();
        for i in 0..8u32 {
            let k = ka.add(&i.to_le_bytes()[..keysize.min(4)].to_vec());
            let val = rng.bytes(elemsize);
            let (tc, tr) = m.put(k, &val);
            same_val("c17 put temp", tc, tr);
            m.check(&format!("c17 elemsize={elemsize} after put {i}"));
        }
        m.free();
    }
}

#[test]
fn e18_hmput_default_len0() {
    let p = fresh_pair(0x18);
    let elemsize = 8usize;
    unsafe {
        // build a raw array with length 0 and turn it into a "map" pointer
        let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
        let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
        let ct = (ca as usize + elemsize) as *mut c_void;
        let rt = (ra as usize + elemsize) as *mut c_void;
        let c2 = (p.c.hmput_default)(ct, elemsize);
        let r2 = (p.r.hmput_default)(rt, elemsize);
        same(
            "e18 hmput_default on length==0",
            &snap_map(c2, elemsize, KeyRepr::Inline),
            &snap_map(r2, elemsize, KeyRepr::Inline),
        );
        (p.c.hmfree_func)((c2 as usize - elemsize) as *mut c_void, elemsize);
        (p.r.hmfree_func)((r2 as usize - elemsize) as *mut c_void, elemsize);
    }
}

#[test]
fn e19_hmput_default_noop() {
    let p = fresh_pair(0x19);
    let elemsize = 16usize;
    unsafe {
        let c1 = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
        let r1 = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
        // scribble row 0 then call again: must be untouched
        for t in [c1, r1] {
            let raw = (t as usize - elemsize) as *mut u8;
            for i in 0..elemsize {
                *raw.add(i) = 0x5A;
            }
        }
        let c2 = (p.c.hmput_default)(c1, elemsize);
        let r2 = (p.r.hmput_default)(r1, elemsize);
        same_val("e19 same pointer", c2 == c1, r2 == r1);
        same(
            "e19 row 0 untouched",
            &snap_map(c2, elemsize, KeyRepr::Inline),
            &snap_map(r2, elemsize, KeyRepr::Inline),
        );
        (p.c.hmfree_func)((c2 as usize - elemsize) as *mut c_void, elemsize);
        (p.r.hmfree_func)((r2 as usize - elemsize) as *mut c_void, elemsize);
    }
}

// --- C18 : single key --------------------------------------------------------
#[test]
fn c18_hm_binary_one() {
    let p = fresh_pair(0x18a);
    let mut rng = Rng::new(0x18a);
    for &(elemsize, keysize) in SHAPES {
        for _ in 0..30 {
            let mut m = DiffMap::lazy(&p, elemsize, keysize, HM_BINARY, KeyRepr::Inline);
            let mut ka = KeyArena::new();
            let k = ka.add(&rng.bytes(keysize));
            let v = rng.bytes(elemsize);
            let (tc, tr) = m.put(k, &v);
            same_val("c18 put temp", tc, tr);
            m.check(&format!("c18 elemsize={elemsize} keysize={keysize}"));
            let (gc, gr) = m.get(k);
            same_val("c18 get temp", gc, gr);
            m.check("c18 after get");
            m.free();
        }
    }
}

// --- C19 / E21 / E22 : many keys, crosses every rehash boundary --------------
#[test]
fn c19_hm_binary_many() {
    let p = fresh_pair(0x19a);
    for &(elemsize, keysize) in SHAPES {
        let mut rng = Rng::new(0x19a ^ elemsize as u64);
        let mut m = DiffMap::lazy(&p, elemsize, keysize, HM_BINARY, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        let mut keys = Vec::new();
        for i in 0..120u32 {
            let kb = rng.bytes(keysize);
            let k = ka.add(&kb);
            keys.push((k, kb));
            let v = rng.bytes(elemsize);
            let (tc, tr) = m.put(k, &v);
            same_val(
                &format!("c19 elemsize={elemsize} put#{i} temp"),
                tc,
                tr,
            );
            m.check(&format!("c19 elemsize={elemsize} put#{i}"));
        }
        // look every key up again
        for (i, (k, _)) in keys.iter().enumerate() {
            let (gc, gr) = m.get(*k);
            same_val(&format!("c19 get#{i} temp"), gc, gr);
            m.check(&format!("c19 get#{i}"));
        }
        m.free();
    }
}

// --- C20 / E23 / E24 : duplicates -------------------------------------------
#[test]
fn c20_hm_binary_duplicates() {
    let p = fresh_pair(0x20);
    let (elemsize, keysize) = (8usize, 4usize);
    let mut rng = Rng::new(0x20);
    let mut m = DiffMap::lazy(&p, elemsize, keysize, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let mut keys = Vec::new();
    for _ in 0..40 {
        let k = ka.add(&rng.bytes(keysize));
        keys.push(k);
        m.put(k, &rng.bytes(elemsize));
    }
    m.check("c20 initial fill");
    // re-put every key many times (hits both the in-bucket and the wrap-around
    // duplicate scan depending on `pos & 7`)
    for round in 0..5 {
        for (i, k) in keys.iter().enumerate() {
            let v = rng.bytes(elemsize);
            let (tc, tr) = m.put(*k, &v);
            same_val(&format!("c20 dup round={round} i={i} temp"), tc, tr);
            m.check(&format!("c20 dup round={round} i={i}"));
        }
    }
    m.free();
}

// --- C21 : put/get matrix ----------------------------------------------------
#[test]
fn c21_hm_binary_put_get_matrix() {
    let p = fresh_pair(0x21);
    for &(elemsize, keysize) in SHAPES {
        let mut rng = Rng::new(0x21 ^ ((elemsize * 31 + keysize) as u64));
        let mut m = DiffMap::lazy(&p, elemsize, keysize, HM_BINARY, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        let mut present: Vec<*mut u8> = Vec::new();
        for step in 0..200 {
            if rng.below(2) == 0 || present.is_empty() {
                let k = ka.add(&rng.bytes(keysize));
                present.push(k);
                let (tc, tr) = m.put(k, &rng.bytes(elemsize));
                same_val(&format!("c21 s={step} put temp"), tc, tr);
            } else if rng.below(2) == 0 {
                let k = present[rng.below(present.len())];
                let (tc, tr) = m.get(k);
                same_val(&format!("c21 s={step} get(hit) temp"), tc, tr);
            } else {
                // miss
                let mut miss = ka.add(&rng.bytes(keysize));
                if keysize > 0 {
                    unsafe { *miss = 0xEE };
                }
                let (tc, tr) = m.get(miss);
                same_val(&format!("c21 s={step} get(miss) temp"), tc, tr);
                miss = std::ptr::null_mut();
                let _ = miss;
            }
            m.check(&format!("c21 elemsize={elemsize} keysize={keysize} s={step}"));
        }
        m.free();
    }
}

// --- C22 : hmget_key_ts ------------------------------------------------------
#[test]
fn c22_hm_binary_get_ts() {
    let p = fresh_pair(0x22);
    for &(elemsize, keysize) in SHAPES {
        let mut rng = Rng::new(0x22 ^ (elemsize as u64));
        let mut m = DiffMap::lazy(&p, elemsize, keysize, HM_BINARY, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        // E11: on a NULL map the very first call goes through hmget_key_ts
        let k0 = ka.add(&rng.bytes(keysize));
        let (tc, tr) = m.get_ts(k0);
        same_val("c22 first get_ts temp", tc, tr);
        m.check("c22 after first get_ts");
        let mut keys = vec![k0];
        for step in 0..150 {
            if rng.below(3) == 0 {
                let k = ka.add(&rng.bytes(keysize));
                keys.push(k);
                m.put(k, &rng.bytes(elemsize));
            } else {
                let k = keys[rng.below(keys.len())];
                let (tc, tr) = m.get_ts(k);
                same_val(&format!("c22 s={step} get_ts temp"), tc, tr);
            }
            m.check(&format!("c22 elemsize={elemsize} s={step}"));
        }
        m.free();
    }
}

// --- C23 / E35 / E36 / E37 : deletes ----------------------------------------
#[test]
fn c23_hm_binary_del() {
    let p = fresh_pair(0x23);
    for &(elemsize, keysize) in SHAPES {
        let mut rng = Rng::new(0x23 ^ (elemsize as u64));
        let mut m = DiffMap::lazy(&p, elemsize, keysize, HM_BINARY, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        let mut keys = Vec::new();
        for _ in 0..30 {
            let k = ka.add(&rng.bytes(keysize));
            keys.push(k);
            m.put(k, &rng.bytes(elemsize));
        }
        m.check("c23 filled");
        // E36: delete the last inserted element (old_index == final_index)
        let last = keys.pop().unwrap();
        let (dc, dr) = m.del(last, 0);
        same_val("c23 del last temp", dc, dr);
        m.check("c23 after del last");
        // E37: delete a middle element (triggers the swap-in + slot patch)
        while !keys.is_empty() {
            let i = rng.below(keys.len());
            let k = keys.remove(i);
            let (dc, dr) = m.del(k, 0);
            same_val(&format!("c23 del i={i} temp"), dc, dr);
            m.check(&format!("c23 after del i={i} remaining={}", keys.len()));
        }
        m.free();
    }
}

// --- C24 / E38 / E39 : shrink -----------------------------------------------
#[test]
fn c24_hm_binary_del_shrink() {
    let p = fresh_pair(0x24);
    let (elemsize, keysize) = (8usize, 4usize);
    let mut rng = Rng::new(0x24);
    let mut m = DiffMap::lazy(&p, elemsize, keysize, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let mut keys = Vec::new();
    // grow well past slot_count 64
    for _ in 0..200 {
        let k = ka.add(&rng.bytes(keysize));
        keys.push(k);
        m.put(k, &rng.bytes(elemsize));
    }
    let sc = unsafe { table_of(m.ct, elemsize).unwrap().slot_count };
    assert!(sc >= 256, "expected a big table, got {sc}");
    m.check("c24 grown");
    // delete everything: crosses every shrink threshold down to 8
    while !keys.is_empty() {
        let k = keys.pop().unwrap();
        let (dc, dr) = m.del(k, 0);
        same_val("c24 del temp", dc, dr);
        m.check(&format!("c24 after del, {} left", keys.len()));
    }
    // E39: never shrinks below 8
    let sc2 = unsafe { table_of(m.ct, elemsize).unwrap().slot_count };
    same_val("c24 min slot_count is 8", sc2, 8usize);
    m.free();
}

// --- C25 / E40 : tombstone rebuild ------------------------------------------
#[test]
fn c25_hm_binary_del_rebuild() {
    let p = fresh_pair(0x25);
    let (elemsize, keysize) = (8usize, 4usize);
    let mut rng = Rng::new(0x25);
    let mut m = DiffMap::lazy(&p, elemsize, keysize, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let mut live = Vec::new();
    // add/remove churn keeps used_count above the shrink threshold while
    // tombstones accumulate past tombstone_count_threshold
    for step in 0..600 {
        if live.len() < 40 || rng.below(3) != 0 {
            let k = ka.add(&rng.bytes(keysize));
            live.push(k);
            let (tc, tr) = m.put(k, &rng.bytes(elemsize));
            same_val(&format!("c25 s={step} put temp"), tc, tr);
        } else {
            let i = rng.below(live.len());
            let k = live.remove(i);
            let (dc, dr) = m.del(k, 0);
            same_val(&format!("c25 s={step} del temp"), dc, dr);
        }
        m.check(&format!("c25 s={step} live={}", live.len()));
    }
    m.free();
}

// --- C26 / E43 : keyoffset != 0 ---------------------------------------------
#[test]
fn c26_hm_binary_keyoffset() {
    let p = fresh_pair(0x26);
    // element layout: [pad:4][key:4] -> keyoffset 4
    let elemsize = 8usize;
    let keysize = 4usize;
    let keyoffset = 4usize;
    let mut rng = Rng::new(0x26);
    unsafe {
        let mut ct: *mut c_void = std::ptr::null_mut();
        let mut rt: *mut c_void = std::ptr::null_mut();
        let mut keys: Vec<[u8; 4]> = Vec::new();
        // hmput_key always uses keyoffset 0 internally, so build the map with
        // the key at offset 0 ... then delete with a non-zero keyoffset, which is
        // exactly what stbds_hmdel/stbds_shdel do via STBDS_OFFSETOF.
        for i in 0..40u32 {
            let mut k = rng.next_u32().to_le_bytes();
            keys.push(k);
            ct = (p.c.hmput_key)(ct, elemsize, k.as_mut_ptr() as *mut c_void, keysize, HM_BINARY);
            rt = (p.r.hmput_key)(rt, elemsize, k.as_mut_ptr() as *mut c_void, keysize, HM_BINARY);
            // macro: t[temp].value = v  (value at offset keysize..elemsize)
            for t in [ct, rt] {
                let temp = map_header(t, elemsize).temp;
                let ep = (t as usize + elemsize * temp as usize) as *mut u8;
                for b in 0..4 {
                    *ep.add(4 + b) = (i as u8).wrapping_add(b as u8);
                }
            }
            same(
                &format!("c26 put#{i}"),
                &snap_map(ct, elemsize, KeyRepr::Inline),
                &snap_map(rt, elemsize, KeyRepr::Inline),
            );
        }
        // now delete using keyoffset 4: the "key" the library compares is the
        // element's *second* word, i.e. the value we wrote. Feed it those bytes.
        for i in 0..40u32 {
            let mut want = [
                (i as u8),
                (i as u8).wrapping_add(1),
                (i as u8).wrapping_add(2),
                (i as u8).wrapping_add(3),
            ];
            ct = (p.c.hmdel_key)(
                ct,
                elemsize,
                want.as_mut_ptr() as *mut c_void,
                keysize,
                keyoffset,
                HM_BINARY,
            );
            rt = (p.r.hmdel_key)(
                rt,
                elemsize,
                want.as_mut_ptr() as *mut c_void,
                keysize,
                keyoffset,
                HM_BINARY,
            );
            same(
                &format!("c26 del#{i} keyoffset={keyoffset}"),
                &snap_map(ct, elemsize, KeyRepr::Inline),
                &snap_map(rt, elemsize, KeyRepr::Inline),
            );
            same_val(
                &format!("c26 del#{i} temp"),
                map_header(ct, elemsize).temp,
                map_header(rt, elemsize).temp,
            );
        }
        (p.c.hmfree_func)((ct as usize - elemsize) as *mut c_void, elemsize);
        (p.r.hmfree_func)((rt as usize - elemsize) as *mut c_void, elemsize);
        let _ = keys;
    }
}

#[test]
fn e43_hmdel_keyoffset() {
    // covered in depth by c26; here: keyoffset that makes the lookup MISS
    let p = fresh_pair(0x43);
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut rng = Rng::new(0x43);
    unsafe {
        let mut ct: *mut c_void = std::ptr::null_mut();
        let mut rt: *mut c_void = std::ptr::null_mut();
        for _ in 0..10 {
            let mut k = rng.next_u64().to_le_bytes();
            ct = (p.c.hmput_key)(ct, elemsize, k.as_mut_ptr() as *mut c_void, keysize, HM_BINARY);
            rt = (p.r.hmput_key)(rt, elemsize, k.as_mut_ptr() as *mut c_void, keysize, HM_BINARY);
            for t in [ct, rt] {
                let temp = map_header(t, elemsize).temp;
                let ep = (t as usize + elemsize * temp as usize) as *mut u8;
                for b in 0..8 {
                    *ep.add(8 + b) = 0x77;
                }
            }
        }
        for off in [0usize, 8] {
            let mut probe = [0x77u8; 8];
            let c2 = (p.c.hmdel_key)(
                ct,
                elemsize,
                probe.as_mut_ptr() as *mut c_void,
                keysize,
                off,
                HM_BINARY,
            );
            let r2 = (p.r.hmdel_key)(
                rt,
                elemsize,
                probe.as_mut_ptr() as *mut c_void,
                keysize,
                off,
                HM_BINARY,
            );
            ct = c2;
            rt = r2;
            same(
                &format!("e43 del keyoffset={off}"),
                &snap_map(ct, elemsize, KeyRepr::Inline),
                &snap_map(rt, elemsize, KeyRepr::Inline),
            );
        }
        (p.c.hmfree_func)((ct as usize - elemsize) as *mut c_void, elemsize);
        (p.r.hmfree_func)((rt as usize - elemsize) as *mut c_void, elemsize);
    }
}

// --- C27 : random pipeline ---------------------------------------------------
#[test]
fn c27_hm_binary_random_pipeline() {
    let p = fresh_pair(0x27);
    for &(elemsize, keysize) in SHAPES {
        let mut rng = Rng::new(0x27 ^ ((elemsize as u64) << 8) ^ keysize as u64);
        let mut m = DiffMap::lazy(&p, elemsize, keysize, HM_BINARY, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        let mut live: Vec<*mut u8> = Vec::new();
        let mut dead: Vec<*mut u8> = Vec::new();
        for step in 0..400 {
            let ctx = format!("c27 elemsize={elemsize} keysize={keysize} step={step}");
            match rng.below(10) {
                0..=3 => {
                    let k = ka.add(&rng.bytes(keysize));
                    live.push(k);
                    let (a, b) = m.put(k, &rng.bytes(elemsize));
                    same_val(&format!("{ctx} put temp"), a, b);
                }
                4 => {
                    if !live.is_empty() {
                        let k = live[rng.below(live.len())];
                        let (a, b) = m.put(k, &rng.bytes(elemsize));
                        same_val(&format!("{ctx} re-put temp"), a, b);
                    }
                }
                5 | 6 => {
                    if !live.is_empty() {
                        let k = live[rng.below(live.len())];
                        let (a, b) = m.get(k);
                        same_val(&format!("{ctx} get temp"), a, b);
                    }
                }
                7 => {
                    if !dead.is_empty() {
                        let k = dead[rng.below(dead.len())];
                        let (a, b) = m.get_ts(k);
                        same_val(&format!("{ctx} get_ts(dead) temp"), a, b);
                    }
                }
                8 => {
                    if !live.is_empty() {
                        let i = rng.below(live.len());
                        let k = live.remove(i);
                        dead.push(k);
                        let (a, b) = m.del(k, 0);
                        same_val(&format!("{ctx} del temp"), a, b);
                    }
                }
                _ => {
                    let v = rng.bytes(elemsize);
                    m.put_default(&v);
                }
            }
            m.check(&ctx);
        }
        m.free();
    }
}

// --- C41 : rand_seed LCG progression ----------------------------------------
#[test]
fn c41_seed_lcg_progression() {
    let p = pair();
    let mut rng = Rng::new(0x41);
    for &s in &[0usize, 1, 2, 0x31415926, usize::MAX, 0xdeadbeef] {
        unsafe {
            (p.c.rand_seed)(s);
            (p.r.rand_seed)(s);
        }
        // each freshly created table consumes one step of the global LCG
        let mut seeds_c = Vec::new();
        let mut seeds_r = Vec::new();
        let mut maps: Vec<DiffMap> = Vec::new();
        for _ in 0..12 {
            let mut m = DiffMap::lazy(&p, 8, 4, HM_BINARY, KeyRepr::Inline);
            let mut ka = KeyArena::new();
            let k = ka.add(&rng.bytes(4));
            m.put(k, &rng.bytes(8));
            m.check(&format!("c41 seed={s:#x}"));
            unsafe {
                seeds_c.push(table_of(m.ct, 8).unwrap().seed);
                seeds_r.push(table_of(m.rt, 8).unwrap().seed);
            }
            // keep the key arena alive by leaking it (binary mode copies keys,
            // so this is only about the map itself)
            std::mem::forget(ka);
            maps.push(m);
        }
        same_val(&format!("c41 seed sequence for {s:#x}"), seeds_c, seeds_r);
        for mut m in maps {
            m.free();
        }
    }
}

// --- C42 : default seed (never call rand_seed) -------------------------------
#[test]
fn c42_default_seed() {
    // A separate process is required to observe the pristine 0x31415926 default,
    // because other tests in this binary have already advanced the global seed.
    // Reset both libraries to the documented default instead.
    let p = fresh_pair(0x31415926);
    let mut m = DiffMap::lazy(&p, 8, 4, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let k = ka.add(&[1u8, 2, 3, 4]);
    m.put(k, &[9u8; 8]);
    m.check("c42 default seed");
    unsafe {
        let ti = table_of(m.ct, 8).unwrap();
        same_val("c42 captured seed", ti.seed, 0x31415926usize);
        same_val(
            "c42 captured seed rust",
            table_of(m.rt, 8).unwrap().seed,
            0x31415926usize,
        );
    }
    m.free();
}

// --- C43 / E31 : keysize == 0 ------------------------------------------------
#[test]
fn c43_keysize_zero() {
    let p = fresh_pair(0x43a);
    let mut rng = Rng::new(0x43a);
    for &elemsize in &[1usize, 4, 8, 16] {
        let mut m = DiffMap::lazy(&p, elemsize, 0, HM_BINARY, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        for i in 0..20 {
            let k = ka.add(&rng.bytes(4));
            let (tc, tr) = m.put(k, &rng.bytes(elemsize));
            same_val(&format!("c43 keysize=0 put#{i} temp"), tc, tr);
            m.check(&format!("c43 elemsize={elemsize} put#{i}"));
        }
        // every key compares equal -> the map must hold exactly one entry
        let len = unsafe { map_header(m.ct, elemsize).length };
        same_val("c43 exactly one live element", len, 2usize);
        let k = ka.add(&[0u8; 4]);
        let (dc, dr) = m.del(k, 0);
        same_val("c43 del temp", dc, dr);
        m.check("c43 after del");
        m.free();
    }
}

#[test]
fn e31_put_keysize_zero() {
    let p = fresh_pair(0x31a);
    let elemsize = 8usize;
    let mut m = DiffMap::lazy(&p, elemsize, 0, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    for i in 0..5u8 {
        let k = ka.add(&[i, i, i, i]);
        m.put(k, &[i; 8]);
        m.check(&format!("e31 put#{i}"));
    }
    m.free();
}

// --- C44 / E44-ish : elemsize == 0 ------------------------------------------
#[test]
fn c44_elemsize_zero() {
    let p = fresh_pair(0x44);
    // keysize 0 as well, so nothing is written past the (zero-sized) element
    let mut m = DiffMap::lazy(&p, 0, 0, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    for i in 0..6u8 {
        let k = ka.add(&[i; 4]);
        let (tc, tr) = m.put(k, &[]);
        same_val(&format!("c44 put#{i} temp"), tc, tr);
        m.check(&format!("c44 elemsize=0 put#{i}"));
    }
    let k = ka.add(&[0u8; 4]);
    let (dc, dr) = m.del(k, 0);
    same_val("c44 del temp", dc, dr);
    m.check("c44 after del");
    m.free();
}

// --- C45 : engineered bucket overflow (multi-bucket probing) -----------------
#[test]
fn c45_forced_collisions() {
    let p = fresh_pair(0x45);
    let (elemsize, keysize) = (8usize, 4usize);
    let mut rng = Rng::new(0x45);
    let mut m = DiffMap::lazy(&p, elemsize, keysize, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    // grow the table to 64 slots (8 buckets) with random keys
    let mut all: Vec<*mut u8> = Vec::new();
    for _ in 0..30 {
        let k = ka.add(&rng.bytes(keysize));
        all.push(k);
        m.put(k, &rng.bytes(elemsize));
    }
    let (seed, slot_count) = unsafe {
        let ti = table_of(m.ct, elemsize).unwrap();
        (ti.seed, ti.slot_count)
    };
    assert_eq!(slot_count, 64, "expected 64 slots after 30 inserts");
    // fill bucket 0 completely, then overflow it
    for target in [0usize, 3, 7] {
        let mut counter = 1u32;
        let mut added = 0;
        while added < 12 {
            let kb = unsafe { key_in_bucket_bin(&p.c, seed, slot_count, target, &mut counter) };
            let k = ka.add(&kb);
            all.push(k);
            let (tc, tr) = m.put(k, &rng.bytes(elemsize));
            same_val(&format!("c45 target={target} put temp"), tc, tr);
            m.check(&format!("c45 target={target} added={added}"));
            added += 1;
            if unsafe { table_of(m.ct, elemsize).unwrap().slot_count } != slot_count {
                break; // table grew, engineering no longer valid
            }
        }
    }
    // look them all up (exercises multi-bucket probing in stbds_hm_find_slot)
    for (i, k) in all.iter().enumerate() {
        let (gc, gr) = m.get(*k);
        same_val(&format!("c45 get#{i} temp"), gc, gr);
        m.check(&format!("c45 get#{i}"));
    }
    // and delete them all (exercises the re-find after swap-in)
    for (i, k) in all.iter().enumerate() {
        let (dc, dr) = m.del(*k, 0);
        same_val(&format!("c45 del#{i} temp"), dc, dr);
        m.check(&format!("c45 del#{i}"));
    }
    m.free();
}

// --- C46 : grow / shrink / grow again ---------------------------------------
#[test]
fn c46_grow_shrink_grow() {
    let p = fresh_pair(0x46);
    let (elemsize, keysize) = (16usize, 8usize);
    let mut rng = Rng::new(0x46);
    let mut m = DiffMap::lazy(&p, elemsize, keysize, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    for round in 0..4 {
        let mut keys = Vec::new();
        for _ in 0..(60 + round * 40) {
            let k = ka.add(&rng.bytes(keysize));
            keys.push(k);
            m.put(k, &rng.bytes(elemsize));
        }
        m.check(&format!("c46 round={round} grown"));
        while !keys.is_empty() {
            let i = rng.below(keys.len());
            let k = keys.remove(i);
            m.del(k, 0);
            m.check(&format!("c46 round={round} shrinking {}", keys.len()));
        }
        m.check(&format!("c46 round={round} emptied"));
    }
    m.free();
}

// --- C47 : the two hash functions on identical bytes ------------------------
#[test]
fn c47_hash_fn_cross_mode() {
    let p = fresh_pair(0x47);
    let mut rng = Rng::new(0x47);
    for _ in 0..300 {
        let n = 1 + rng.below(20);
        let mut s = rng.cstring(n, ASCII);
        let seed = rng.next_u64() as usize;
        unsafe {
            let cb = (p.c.hash_bytes)(s.as_mut_ptr() as *mut c_void, s.len() - 1, seed);
            let rb = (p.r.hash_bytes)(s.as_mut_ptr() as *mut c_void, s.len() - 1, seed);
            same_val("c47 hash_bytes", cb, rb);
            let cs = (p.c.hash_string)(s.as_mut_ptr() as *mut std::os::raw::c_char, seed);
            let rs = (p.r.hash_string)(s.as_mut_ptr() as *mut std::os::raw::c_char, seed);
            same_val("c47 hash_string", cs, rs);
        }
    }
}
