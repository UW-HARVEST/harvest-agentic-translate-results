//! Phase B — string-keyed hash map: `STBDS_HM_STRING` combined with every
//! `STBDS_SH_*` key-storage mode, plus `stbds_shmode_func`.
//! Rows C28–C36 of CONFIGS.md.
mod common;
use common::*;

/// element layouts for `struct { char *key; V value; }`
const SH_SHAPES: &[usize] = &[8, 16, 24, 32, 48];
/// `stbds_shput` always passes `sizeof (t)->key` == sizeof(char*)
const KEYSIZE: usize = 8;

fn keyset(rng: &mut Rng, n: usize) -> Vec<Vec<u8>> {
    (0..n)
        .map(|i| match i % 6 {
            0 => rng.cstring_range(1, 9, ASCII),
            1 => rng.cstring_range(8, 32, ASCII),
            2 => {
                let mut v = b"test_".to_vec();
                v.extend_from_slice(format!("{}", i).as_bytes());
                v.push(0);
                v
            }
            3 => rng.cstring_range(1, 5, b"ab"),
            4 => rng.cstring_range(1, 11, HIGHBYTES),
            _ => {
                let mut v = vec![b'x'; 1 + (i % 40)];
                v.push(0);
                v
            }
        })
        .collect()
}

// --- C28 : implicit SH_DEFAULT table (created by hmput_key) -----------------
#[test]
fn c28_sh_default_implicit() {
    let p = fresh_pair(0x28);
    for &elemsize in SH_SHAPES {
        let mut rng = Rng::new(0x28 ^ elemsize as u64);
        let mut m = DiffMap::lazy(&p, elemsize, KEYSIZE, HM_STRING, KeyRepr::CharPtr);
        let mut ka = KeyArena::new();
        let mut keys = Vec::new();
        for (i, kb) in keyset(&mut rng, 60).into_iter().enumerate() {
            let k = ka.add(&kb);
            keys.push(k);
            let (tc, tr) = m.put(k, &rng.bytes(elemsize));
            same_val(&format!("c28 elemsize={elemsize} put#{i} temp"), tc, tr);
            m.check(&format!("c28 elemsize={elemsize} put#{i}"));
            // SH_DEFAULT stores the CALLER's pointer verbatim: both libraries
            // must have recorded the exact same address.
            unsafe {
                same_val(
                    &format!("c28 temp_key ptr #{i}"),
                    temp_key_ptr(m.ct, elemsize) as usize,
                    temp_key_ptr(m.rt, elemsize) as usize,
                );
            }
        }
        for (i, k) in keys.iter().enumerate() {
            let (gc, gr) = m.get(*k);
            same_val(&format!("c28 get#{i} temp"), gc, gr);
            m.check(&format!("c28 get#{i}"));
        }
        for (i, k) in keys.iter().enumerate() {
            let (dc, dr) = m.del(*k, 0);
            same_val(&format!("c28 del#{i} temp"), dc, dr);
            m.check(&format!("c28 del#{i}"));
        }
        m.free();
        std::mem::forget(ka);
    }
}

// --- C32 / E27 : explicit SH_DEFAULT (shmode_func) --------------------------
#[test]
fn c32_sh_default_explicit() {
    sh_pipeline(0x32, SH_DEFAULT, "c32");
}

// --- C29 / E28 / E41 : SH_STRDUP --------------------------------------------
#[test]
fn c29_sh_strdup_pipeline() {
    sh_pipeline(0x29, SH_STRDUP, "c29");
}

// --- C30 / E29 : SH_ARENA ----------------------------------------------------
#[test]
fn c30_sh_arena_pipeline() {
    sh_pipeline(0x30, SH_ARENA, "c30");
}

fn sh_pipeline(seed: usize, shmode: std::os::raw::c_int, tag: &str) {
    let p = fresh_pair(seed);
    for &elemsize in SH_SHAPES {
        let mut rng = Rng::new(seed as u64 ^ elemsize as u64);
        let mut m = DiffMap::shmode(&p, elemsize, KEYSIZE, HM_STRING, shmode, KeyRepr::CharPtr);
        m.check(&format!("{tag} fresh shmode={shmode} elemsize={elemsize}"));
        let mut ka = KeyArena::new();
        let mut live = Vec::new();
        let mut dead: Vec<*mut u8> = Vec::new();
        let kbs = keyset(&mut rng, 200);
        let mut next = 0usize;
        for step in 0..300 {
            let ctx = format!("{tag} shmode={shmode} elemsize={elemsize} step={step}");
            match rng.below(10) {
                0..=3 => {
                    if next < kbs.len() {
                        let k = ka.add(&kbs[next]);
                        next += 1;
                        live.push(k);
                        let (a, b) = m.put(k, &rng.bytes(elemsize));
                        same_val(&format!("{ctx} put temp"), a, b);
                    }
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
                    m.put_default(&rng.bytes(elemsize));
                }
            }
            m.check(&ctx);
        }
        m.free();
        // SH_DEFAULT keeps the caller's pointers; keep the arena alive for the
        // whole process to avoid a use-after-free while the map still exists.
        std::mem::forget(ka);
    }
}

// --- C31 / E64 / E30 : SH_NONE table driven in STRING mode ------------------
#[test]
fn c31_sh_none_string_mode() {
    let p = fresh_pair(0x31);
    // element layout: [key bytes: 8][value]
    for &elemsize in &[8usize, 16, 24] {
        let mut rng = Rng::new(0x31 ^ elemsize as u64);
        let sk = SelfKeys::new(60);
        let mut m = DiffMap::shmode(&p, elemsize, KEYSIZE, HM_STRING, SH_NONE, KeyRepr::Inline);
        m.check(&format!("c31 fresh elemsize={elemsize}"));
        for (i, &k) in sk.keys.iter().enumerate() {
            let (tc, tr) = m.put(k, &rng.bytes(elemsize));
            same_val(&format!("c31 put#{i} temp"), tc, tr);
            m.check(&format!("c31 elemsize={elemsize} put#{i}"));
        }
        // re-put (exercises the string-compare-on-memcpy'd-bytes quirk)
        for (i, &k) in sk.keys.iter().enumerate() {
            let (tc, tr) = m.put(k, &rng.bytes(elemsize));
            same_val(&format!("c31 re-put#{i} temp"), tc, tr);
            m.check(&format!("c31 re-put#{i}"));
        }
        for (i, &k) in sk.keys.iter().enumerate() {
            let (gc, gr) = m.get(k);
            same_val(&format!("c31 get#{i} temp"), gc, gr);
            let (tc2, tr2) = m.get_ts(k);
            same_val(&format!("c31 get_ts#{i} temp"), tc2, tr2);
            m.check(&format!("c31 get#{i}"));
        }
        for (i, &k) in sk.keys.iter().enumerate() {
            let (dc, dr) = m.del(k, 0);
            same_val(&format!("c31 del#{i} temp"), dc, dr);
            m.check(&format!("c31 del#{i}"));
        }
        m.free();
    }
}

#[test]
fn e64_string_mode_none_table() {
    let p = fresh_pair(0x64);
    let elemsize = 16usize;
    let sk = SelfKeys::new(12);
    let mut m = DiffMap::shmode(&p, elemsize, KEYSIZE, HM_STRING, SH_NONE, KeyRepr::Inline);
    unsafe {
        // string.mode must be 0 in both
        same_val(
            "e64 string.mode",
            table_of(m.ct, elemsize).unwrap().string.mode,
            table_of(m.rt, elemsize).unwrap().string.mode,
        );
        same_val(
            "e64 string.mode == 0",
            table_of(m.ct, elemsize).unwrap().string.mode,
            0u8,
        );
    }
    for (i, &k) in sk.keys.iter().enumerate() {
        m.put(k, &[i as u8; 48][..elemsize - 8]);
        m.check(&format!("e64 put#{i}"));
    }
    m.free();
}

// --- C33 : key shapes --------------------------------------------------------
#[test]
fn c33_sh_key_shapes() {
    let p = fresh_pair(0x33);
    let elemsize = 16usize;
    for &shmode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let mut m = DiffMap::shmode(&p, elemsize, KEYSIZE, HM_STRING, shmode, KeyRepr::CharPtr);
        let mut ka = KeyArena::new();
        let mut shapes: Vec<Vec<u8>> = vec![
            vec![0],                    // ""
            b"a\0".to_vec(),            // 1 char
            b"abcdefgh\0".to_vec(),     // exactly 8
            vec![b'z'; 64]
                .into_iter()
                .chain(std::iter::once(0))
                .collect(), // 64 chars
            vec![0x80, 0xff, 0x81, 0],  // high bytes
            vec![0xff; 40]
                .into_iter()
                .chain(std::iter::once(0))
                .collect(),
        ];
        // common-prefix keys from the library's own strkey()
        unsafe {
            for n in 0..40 {
                let s = (p.c.strkey)(n);
                let mut v = Vec::new();
                let mut q = s as *const u8;
                while *q != 0 {
                    v.push(*q);
                    q = q.add(1);
                }
                v.push(0);
                shapes.push(v);
            }
        }
        let mut keys = Vec::new();
        for (i, kb) in shapes.iter().enumerate() {
            let k = ka.add(kb);
            keys.push(k);
            let (tc, tr) = m.put(k, &[(i as u8); 8]);
            same_val(&format!("c33 shmode={shmode} put#{i} temp"), tc, tr);
            m.check(&format!("c33 shmode={shmode} put#{i}"));
        }
        for (i, k) in keys.iter().enumerate() {
            let (gc, gr) = m.get(*k);
            same_val(&format!("c33 get#{i} temp"), gc, gr);
            m.check(&format!("c33 get#{i}"));
        }
        for (i, k) in keys.iter().enumerate() {
            let (dc, dr) = m.del(*k, 0);
            same_val(&format!("c33 del#{i} temp"), dc, dr);
            m.check(&format!("c33 del#{i}"));
        }
        m.free();
        std::mem::forget(ka);
    }
}

// --- C34 : 0..40 distinct keys, every rehash boundary -----------------------
#[test]
fn c34_sh_many_keys() {
    let p = fresh_pair(0x34);
    for &shmode in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for n in 0usize..41 {
            let elemsize = 16usize;
            let keyrepr = if shmode == SH_NONE {
                KeyRepr::Inline
            } else {
                KeyRepr::CharPtr
            };
            let mut m = DiffMap::shmode(&p, elemsize, KEYSIZE, HM_STRING, shmode, keyrepr);
            let mut ka = KeyArena::new();
            let sk = if shmode == SH_NONE {
                Some(SelfKeys::new(n.max(1)))
            } else {
                None
            };
            let mut rng = Rng::new(0x34 ^ (n as u64) << 8 ^ shmode as u64);
            for i in 0..n {
                let k = match &sk {
                    Some(s) => s.keys[i],
                    None => {
                        let kb = rng.cstring_range(1, 21, ASCII);
                        ka.add(&kb)
                    }
                };
                m.put(k, &rng.bytes(elemsize));
                m.check(&format!("c34 shmode={shmode} n={n} put#{i}"));
            }
            m.check(&format!("c34 shmode={shmode} n={n} final"));
            m.free();
            std::mem::forget(ka);
        }
    }
}

// --- C35 : long random pipeline per SH mode ---------------------------------
#[test]
fn c35_sh_random_pipeline() {
    for (i, &shmode) in [SH_DEFAULT, SH_STRDUP, SH_ARENA].iter().enumerate() {
        sh_pipeline(0x350 + i, shmode, "c35");
    }
    // SH_NONE needs the self-referential key trick
    let p = fresh_pair(0x35f);
    let elemsize = 24usize;
    let mut rng = Rng::new(0x35f);
    let sk = SelfKeys::new(80);
    let mut m = DiffMap::shmode(&p, elemsize, KEYSIZE, HM_STRING, SH_NONE, KeyRepr::Inline);
    let mut live: Vec<*mut u8> = Vec::new();
    let mut next = 0usize;
    for step in 0..400 {
        let ctx = format!("c35 SH_NONE step={step}");
        match rng.below(8) {
            0..=3 => {
                if next < sk.keys.len() {
                    let k = sk.keys[next];
                    next += 1;
                    live.push(k);
                    let (a, b) = m.put(k, &rng.bytes(elemsize));
                    same_val(&format!("{ctx} put temp"), a, b);
                }
            }
            4 => {
                if !live.is_empty() {
                    let k = live[rng.below(live.len())];
                    let (a, b) = m.get(k);
                    same_val(&format!("{ctx} get temp"), a, b);
                }
            }
            5 => {
                if !live.is_empty() {
                    let k = live[rng.below(live.len())];
                    let (a, b) = m.get_ts(k);
                    same_val(&format!("{ctx} get_ts temp"), a, b);
                }
            }
            6 => {
                if !live.is_empty() {
                    let i = rng.below(live.len());
                    let k = live.remove(i);
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
}

// --- C36 / E07..E09 : hmfree_func across all modes --------------------------
#[test]
fn c36_hmfree_all_modes() {
    let p = fresh_pair(0x36);
    let elemsize = 16usize;
    // 1. binary map
    {
        let mut m = DiffMap::lazy(&p, elemsize, 8, HM_BINARY, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        let mut rng = Rng::new(1);
        for _ in 0..20 {
            let k = ka.add(&rng.bytes(8));
            m.put(k, &rng.bytes(elemsize));
        }
        m.check("c36 binary before free");
        m.free();
    }
    // 2. each SH mode
    for &shmode in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let keyrepr = if shmode == SH_NONE {
            KeyRepr::Inline
        } else {
            KeyRepr::CharPtr
        };
        let mut m = DiffMap::shmode(&p, elemsize, KEYSIZE, HM_STRING, shmode, keyrepr);
        let sk = SelfKeys::new(20);
        let mut ka = KeyArena::new();
        let mut rng = Rng::new(shmode as u64 + 7);
        for i in 0..20 {
            let k = if shmode == SH_NONE {
                sk.keys[i]
            } else {
                let kb = rng.cstring_range(3, 23, ASCII);
                ka.add(&kb)
            };
            m.put(k, &rng.bytes(elemsize));
        }
        m.check(&format!("c36 shmode={shmode} before free"));
        m.free();
        std::mem::forget(ka);
    }
    // 3. table-less array turned into a "map" pointer (E08)
    unsafe {
        let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
        let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
        (p.c.hmfree_func)(ca, elemsize);
        (p.r.hmfree_func)(ra, elemsize);
    }
    // 4. NULL (E07)
    unsafe {
        (p.c.hmfree_func)(std::ptr::null_mut(), elemsize);
        (p.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        (p.c.hmfree_func)(std::ptr::null_mut(), 0);
        (p.r.hmfree_func)(std::ptr::null_mut(), 0);
    }
}

// --- E45 : shmode_func with elemsize 0 --------------------------------------
#[test]
fn e45_shmode_elemsize_zero() {
    let p = fresh_pair(0x45f);
    unsafe {
        for shmode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            let ct = (p.c.shmode_func)(0, shmode);
            let rt = (p.r.shmode_func)(0, shmode);
            same(
                &format!("e45 shmode_func(0,{shmode})"),
                &snap_map(ct, 0, KeyRepr::Inline),
                &snap_map(rt, 0, KeyRepr::Inline),
            );
            (p.c.hmfree_func)(ct, 0);
            (p.r.hmfree_func)(rt, 0);
        }
    }
}

// --- E21 : first table has slot_count 8 and the right implicit string.mode ---
#[test]
fn e21_hmput_key_first_table() {
    let p = fresh_pair(0x21f);
    let elemsize = 16usize;
    // binary -> string.mode 0
    {
        let mut m = DiffMap::lazy(&p, elemsize, 8, HM_BINARY, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        let k = ka.add(&[1u8, 2, 3, 4, 5, 6, 7, 8]);
        m.put(k, &[0u8; 8]);
        m.check("e21 binary first table");
        unsafe {
            let tc = table_of(m.ct, elemsize).unwrap();
            let tr = table_of(m.rt, elemsize).unwrap();
            same_val("e21 binary slot_count", tc.slot_count, 8usize);
            same_val("e21 binary string.mode", tc.string.mode, tr.string.mode);
            same_val("e21 binary string.mode == 0", tc.string.mode, 0u8);
        }
        m.free();
    }
    // string -> string.mode SH_DEFAULT(1)
    {
        let mut m = DiffMap::lazy(&p, elemsize, 8, HM_STRING, KeyRepr::CharPtr);
        let mut ka = KeyArena::new();
        let k = ka.add(b"hello\0");
        m.put(k, &[0u8; 8]);
        m.check("e21 string first table");
        unsafe {
            let tc = table_of(m.ct, elemsize).unwrap();
            let tr = table_of(m.rt, elemsize).unwrap();
            same_val("e21 string slot_count", tc.slot_count, 8usize);
            same_val("e21 string string.mode", tc.string.mode, tr.string.mode);
            same_val("e21 string string.mode == SH_DEFAULT", tc.string.mode, 1u8);
        }
        m.free();
        std::mem::forget(ka);
    }
}

// --- E22 / E26 : growth thresholds ------------------------------------------
#[test]
fn e22_hmput_key_grow_threshold() {
    let p = fresh_pair(0x22f);
    let elemsize = 16usize;
    let mut m = DiffMap::lazy(&p, elemsize, 8, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let mut rng = Rng::new(0x22f);
    let mut seen = Vec::new();
    for i in 0..100 {
        let k = ka.add(&rng.bytes(8));
        m.put(k, &rng.bytes(elemsize));
        m.check(&format!("e22 put#{i}"));
        unsafe {
            let tc = table_of(m.ct, elemsize).unwrap();
            let tr = table_of(m.rt, elemsize).unwrap();
            same_val(&format!("e22 slot_count#{i}"), tc.slot_count, tr.slot_count);
            same_val(&format!("e22 used_count#{i}"), tc.used_count, tr.used_count);
            same_val(
                &format!("e22 used_thr#{i}"),
                tc.used_count_threshold,
                tr.used_count_threshold,
            );
            seen.push(tc.slot_count);
        }
    }
    seen.dedup();
    assert_eq!(
        seen,
        vec![8usize, 16, 32, 64, 128, 256],
        "growth ladder must double from 8"
    );
    m.free();
}

#[test]
fn e26_hmput_array_regrow() {
    // the element array and the hash index grow independently; check both
    let p = fresh_pair(0x26f);
    let elemsize = 24usize;
    let mut m = DiffMap::lazy(&p, elemsize, 8, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let mut rng = Rng::new(0x26f);
    for i in 0..300 {
        let k = ka.add(&rng.bytes(8));
        m.put(k, &rng.bytes(elemsize));
        m.check(&format!("e26 put#{i}"));
        unsafe {
            same_val(
                &format!("e26 array cap#{i}"),
                map_header(m.ct, elemsize).capacity,
                map_header(m.rt, elemsize).capacity,
            );
        }
    }
    m.free();
}
