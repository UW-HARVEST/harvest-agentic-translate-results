//! Phase B rows C30–C45: `stbds_hmdel_key` (all modes, all `string.mode`s,
//! shrink + rebuild paths, non-zero `keyoffset`), `stbds_hmput_default`, and
//! `stbds_hmfree_func`.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

// ---------------------------------------------------------------- C30 / C31
#[test]
fn c30_binary_delete_last_element() {
    let s = session(0x31415926);
    for (elemsize, keysize) in [(16usize, 4usize), (16, 8), (24, 8), (8, 8), (4, 4), (1, 1)] {
        let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
        let mut rng = Rng::new(0x30 ^ (elemsize * 7 + keysize) as u64);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..40usize {
            let k = rng.bytes(keysize);
            if keys.contains(&k) {
                continue;
            }
            d.hmput(&format!("C30 put#{i}"), &k, &rng.bytes(elemsize), HM_BINARY);
            keys.push(k);
        }
        d.check("C30 populated");
        // Repeatedly delete the LAST element: old_index == final_index, no memmove.
        while let Some(k) = keys.pop() {
            let ctx = format!("C30 e={elemsize} k={keysize} del-last len={}", keys.len());
            let t = d.hmdel(&ctx, &k, 0, HM_BINARY);
            assert_eq!(t, 1, "{ctx}: temp must be 1 for a successful delete");
            d.check(&ctx);
        }
        d.free();
    }
}

#[test]
fn c31_binary_delete_middle_element() {
    let s = session(0x31415926);
    for (elemsize, keysize) in [(16usize, 4usize), (24, 8), (8, 8)] {
        let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
        let mut rng = Rng::new(0x31 ^ (elemsize * 7 + keysize) as u64);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..60usize {
            let k = rng.bytes(keysize);
            if keys.contains(&k) {
                continue;
            }
            d.hmput(&format!("C31 put#{i}"), &k, &rng.bytes(elemsize), HM_BINARY);
            keys.push(k);
        }
        // Always delete from the FRONT: old_index != final_index -> memmove +
        // slot re-index of the element pulled in from the tail.
        while !keys.is_empty() {
            let k = keys.remove(0);
            let ctx = format!("C31 e={elemsize} del-front len={}", keys.len());
            let t = d.hmdel(&ctx, &k, 0, HM_BINARY);
            assert_eq!(t, 1, "{ctx}: temp must be 1");
            d.check(&ctx);
            // Every remaining key must still be findable at its new index.
            for (j, kk) in keys.iter().enumerate() {
                let c2 = format!("{ctx} verify#{j}");
                assert!(d.hmgeti(&c2, kk, HM_BINARY) >= 0, "{c2}");
                d.check(&c2);
            }
        }
        d.free();
    }
}

// ---------------------------------------------------------------- C32
#[test]
fn c32_binary_delete_all_random_order_shrink_and_rebuild() {
    let s = session(0x31415926);
    for n in [200usize, 300] {
        let elemsize = 16usize;
        let keysize = 8usize;
        let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
        let mut rng = Rng::new(0x32 + n as u64);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..n {
            let k = rng.bytes(keysize);
            if keys.contains(&k) {
                continue;
            }
            d.hmput(&format!("C32 put#{i}"), &k, &rng.bytes(elemsize), HM_BINARY);
            keys.push(k);
        }
        let mut max_slots = 0usize;
        let mut saw_shrink = false;
        let mut saw_rebuild = false;
        unsafe {
            max_slots = max_slots.max((*(header(d.c_raw()).hash_table as *mut HashIndex)).slot_count);
        }
        let start_slots = max_slots;
        while !keys.is_empty() {
            let i = rng.below(keys.len());
            let k = keys.remove(i);
            let before = unsafe {
                let t = header(d.c_raw()).hash_table as *mut HashIndex;
                ((*t).slot_count, (*t).tombstone_count)
            };
            let ctx = format!("C32 n={n} del len={}", keys.len());
            assert_eq!(d.hmdel(&ctx, &k, 0, HM_BINARY), 1, "{ctx}");
            d.check(&ctx);
            let after = unsafe {
                let t = header(d.c_raw()).hash_table as *mut HashIndex;
                ((*t).slot_count, (*t).tombstone_count)
            };
            if after.0 < before.0 {
                saw_shrink = true;
            }
            if after.0 == before.0 && after.1 == 0 && before.1 > 0 {
                saw_rebuild = true;
            }
        }
        assert!(start_slots >= 256, "C32: expected several grows, got {start_slots}");
        assert!(saw_shrink, "C32: never exercised the shrink path (lib.c:854)");
        assert!(saw_rebuild, "C32: never exercised the rebuild path (lib.c:858)");
        d.check("C32 emptied");
        d.free();
    }
}

// ---------------------------------------------------------------- C33
#[test]
fn c33_binary_interleaved_random_ops() {
    let s = session(0x31415926);
    for (elemsize, keysize, seed) in [
        (16usize, 4usize, 0x330u64),
        (16, 8, 0x331),
        (24, 8, 0x332),
        (16, 2, 0x333),
        (16, 1, 0x334),
        (8, 8, 0x335),
    ] {
        let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
        let mut rng = Rng::new(seed);
        let mut live: Vec<Vec<u8>> = Vec::new();
        for op in 0..2000usize {
            let ctx = format!("C33 e={elemsize} k={keysize} op#{op}");
            match rng.below(10) {
                0..=4 => {
                    let k = rng.bytes(keysize);
                    d.hmput(&ctx, &k, &rng.bytes(elemsize), HM_BINARY);
                    if !live.contains(&k) {
                        live.push(k);
                    }
                }
                5..=6 => {
                    if !live.is_empty() {
                        let k = live[rng.below(live.len())].clone();
                        d.hmgeti(&ctx, &k, HM_BINARY);
                    } else {
                        d.hmgeti(&ctx, &rng.bytes(keysize), HM_BINARY);
                    }
                }
                7 => {
                    let k = rng.bytes(keysize);
                    d.hmgeti_ts(&ctx, &k, HM_BINARY);
                }
                _ => {
                    if !live.is_empty() {
                        let i = rng.below(live.len());
                        let k = live.remove(i);
                        d.hmdel(&ctx, &k, 0, HM_BINARY);
                    } else {
                        d.hmdel(&ctx, &rng.bytes(keysize), 0, HM_BINARY);
                    }
                }
            }
            d.check(&ctx);
        }
        // Every live key must still resolve.
        for (i, k) in live.iter().enumerate() {
            let ctx = format!("C33 e={elemsize} final-verify#{i}");
            assert!(d.hmgeti(&ctx, k, HM_BINARY) >= 0, "{ctx}");
            d.check(&ctx);
        }
        d.free();
    }
}

// ---------------------------------------------------------------- C34/C35/C36
fn string_delete_row(row: &str, mode: i32, elemsize: usize, n: usize, seed: u64) {
    let s = session(0x31415926);
    let mut d = Driver::shmode(&s, elemsize, 8, KeyKind::StringAt(0), mode);
    let mut rng = Rng::new(seed);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..n {
        let k = format!("{row}-key-{i}-{}", rng.below(1 << 20)).into_bytes();
        if keys.contains(&k) {
            continue;
        }
        d.shput(&format!("{row} put#{i}"), &k, &rng.bytes(elemsize), HM_STRING);
        keys.push(k);
    }
    d.check(&format!("{row} populated"));

    // delete the last element
    let k = keys.pop().unwrap();
    let ctx = format!("{row} del-last");
    assert_eq!(d.shdel(&ctx, &k, 0, HM_STRING), 1, "{ctx}");
    d.check(&ctx);

    // delete a middle element (forces the memmove + re-lookup)
    let k = keys.remove(keys.len() / 2);
    let ctx = format!("{row} del-middle");
    assert_eq!(d.shdel(&ctx, &k, 0, HM_STRING), 1, "{ctx}");
    d.check(&ctx);

    // delete an absent key
    let ctx = format!("{row} del-absent");
    assert_eq!(d.shdel(&ctx, b"--nope--", 0, HM_STRING), 0, "{ctx}");
    d.check(&ctx);

    // drain in random order (shrink + rebuild)
    while !keys.is_empty() {
        let i = rng.below(keys.len());
        let k = keys.remove(i);
        let ctx = format!("{row} drain len={}", keys.len());
        assert_eq!(d.shdel(&ctx, &k, 0, HM_STRING), 1, "{ctx}");
        d.check(&ctx);
        for (j, kk) in keys.iter().enumerate().take(8) {
            let c2 = format!("{ctx} verify#{j}");
            assert!(d.shgeti(&c2, kk, HM_STRING) >= 0, "{c2}");
            d.check(&c2);
        }
    }
    d.check(&format!("{row} emptied"));
    d.free();
}

#[test]
fn c34_string_default_delete() {
    string_delete_row("C34", SH_DEFAULT, 16, 200, 0x34);
    string_delete_row("C34b", SH_DEFAULT, 24, 90, 0x35);
}

#[test]
fn c35_string_strdup_delete() {
    string_delete_row("C35", SH_STRDUP, 16, 200, 0x36);
    string_delete_row("C35b", SH_STRDUP, 32, 90, 0x37);
}

#[test]
fn c36_string_arena_delete() {
    string_delete_row("C36", SH_ARENA, 16, 200, 0x38);
    string_delete_row("C36b", SH_ARENA, 24, 90, 0x39);
}

#[test]
fn c36c_string_interleaved_random_ops() {
    let s = session(0x31415926);
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let mut d = Driver::shmode(&s, 16, 8, KeyKind::StringAt(0), mode);
        let mut rng = Rng::new(0x36C + mode as u64);
        let mut live: Vec<Vec<u8>> = Vec::new();
        for op in 0..1200usize {
            let ctx = format!("C36c m={mode} op#{op}");
            match rng.below(10) {
                0..=4 => {
                    let k = format!("k{}", rng.below(400)).into_bytes();
                    d.shput(&ctx, &k, &rng.bytes(16), HM_STRING);
                    if !live.contains(&k) {
                        live.push(k);
                    }
                }
                5..=6 => {
                    let k = if live.is_empty() {
                        b"zz".to_vec()
                    } else {
                        live[rng.below(live.len())].clone()
                    };
                    d.shgeti(&ctx, &k, HM_STRING);
                }
                7 => {
                    let k = format!("k{}", rng.below(600)).into_bytes();
                    d.shgeti_ts(&ctx, &k, HM_STRING);
                }
                _ => {
                    if !live.is_empty() {
                        let i = rng.below(live.len());
                        let k = live.remove(i);
                        d.shdel(&ctx, &k, 0, HM_STRING);
                    } else {
                        d.shdel(&ctx, b"absent", 0, HM_STRING);
                    }
                }
            }
            d.check(&ctx);
        }
        for (i, k) in live.iter().enumerate() {
            let ctx = format!("C36c m={mode} final#{i}");
            assert!(d.shgeti(&ctx, k, HM_STRING) >= 0, "{ctx}");
        }
        d.check("C36c final");
        d.free();
    }
}

// ---------------------------------------------------------------- C37
#[test]
fn c37_binary_delete_nonzero_keyoffset() {
    // `stbds_hmdel_key` is the ONLY entry point taking `keyoffset`; the put/get
    // pair hard-codes 0.  A non-zero offset therefore compares the key against
    // the element's VALUE bytes.  Both libraries must agree on the outcome.
    let s = session(0x31415926);
    let elemsize = 24usize;
    let keysize = 8usize;
    for keyoffset in [8usize, 16] {
        let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
        let mut rng = Rng::new(0x37 + keyoffset as u64);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..50usize {
            let k = rng.bytes(keysize);
            if keys.contains(&k) {
                continue;
            }
            // Value bytes deliberately mirror the key at `keyoffset`, so the
            // offset lookup CAN succeed for some keys.
            let mut v = vec![0u8; elemsize];
            v[keyoffset - keysize..keyoffset - keysize + keysize].copy_from_slice(&k);
            d.hmput(&format!("C37 put#{i}"), &k, &v, HM_BINARY);
            keys.push(k);
        }
        d.check(&format!("C37 off={keyoffset} populated"));
        for (i, k) in keys.clone().iter().enumerate() {
            let ctx = format!("C37 off={keyoffset} del#{i}");
            d.hmdel(&ctx, k, keyoffset, HM_BINARY);
            d.check(&ctx);
        }
        d.free();
    }
}

// ---------------------------------------------------------------- C38
#[test]
fn c38_string_delete_nonzero_keyoffset() {
    // `keyoffset = 8` with `mode = STBDS_HM_STRING`: `is_key_equal` reads
    // `*(char **)(elem + 8)`.  The elements are built so that offset 8 really
    // does hold a valid `char *` (the same key string), which is the only way
    // this configuration is well-defined in the C.
    let s = session(0x31415926);
    let elemsize = 24usize;
    for mode in [SH_DEFAULT, SH_ARENA] {
        let mut d = Driver::shmode(&s, elemsize, 8, KeyKind::StringAtPair(0, 8), mode);
        let mut rng = Rng::new(0x38 + mode as u64);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..40usize {
            let k = format!("off-key-{i}").into_bytes();
            let ctx = format!("C38 m={mode} put#{i}");
            let idx = d.shput(&ctx, &k, &rng.bytes(elemsize), HM_STRING);
            unsafe {
                // element[8..16] := the pointer the library stored at [0..8]
                for m in [d.cm, d.rm] {
                    let e = (m as *mut u8).offset(idx * elemsize as isize);
                    let stored = *(e as *mut *mut c_char);
                    *(e.add(8) as *mut *mut c_char) = stored;
                    // zero the remaining tail so every compared byte is defined
                    std::ptr::write_bytes(e.add(16), 0, elemsize - 16);
                }
            }
            keys.push(k);
        }
        d.check(&format!("C38 m={mode} populated"));
        for (i, k) in keys.iter().enumerate() {
            let ctx = format!("C38 m={mode} del#{i}");
            d.shdel(&ctx, k, 8, HM_STRING);
            d.check(&ctx);
        }
        d.free();
    }
}

// ---------------------------------------------------------------- C39/C40/C41
#[test]
fn c39_hmput_default_bootstrap() {
    let s = session(0x31415926);
    let mut rng = Rng::new(0x39);
    for elemsize in [16usize, 24, 8, 4, 1] {
        let mut d = Driver::lazy(&s, elemsize, 8, KeyKind::Binary);
        let ctx = format!("C39 e={elemsize} bootstrap");
        d.hmdefault(&ctx, &rng.bytes(elemsize));
        unsafe {
            assert_eq!(header(d.c_raw()).length, 1, "{ctx}: C length");
            assert_eq!(header(d.r_raw()).length, 1, "{ctx}: Rust length");
        }
        d.check(&ctx);
        // Idempotent second call (length != 0 -> pass-through).
        let before_c = d.cm;
        let before_r = d.rm;
        d.hmdefault(&format!("{ctx} again"), &rng.bytes(elemsize));
        assert_eq!(d.cm, before_c, "{ctx}: C pointer changed on pass-through");
        assert_eq!(d.rm, before_r, "{ctx}: Rust pointer changed on pass-through");
        d.check(&format!("{ctx} again"));
        unsafe {
            (s.c.arrfreef)(d.c_raw());
            (s.r.arrfreef)(d.r_raw());
        }
        d.cm = std::ptr::null_mut();
        d.rm = std::ptr::null_mut();
    }
}

#[test]
fn c40_hmput_default_on_populated_map() {
    let s = session(0x31415926);
    let elemsize = 16usize;
    let mut d = Driver::lazy(&s, elemsize, 8, KeyKind::Binary);
    let mut rng = Rng::new(0x40);
    for i in 0..30usize {
        let k = rng.bytes(8);
        d.hmput(&format!("C40 put#{i}"), &k, &rng.bytes(elemsize), HM_BINARY);
    }
    let before_c = d.cm;
    let before_r = d.rm;
    d.hmdefault("C40 default", &rng.bytes(elemsize));
    assert_eq!(d.cm, before_c, "C40: C must return the map unchanged");
    assert_eq!(d.rm, before_r, "C40: Rust must return the map unchanged");
    d.check("C40 default");
    d.free();
}

#[test]
fn c41_hmput_default_on_zero_length_array() {
    // `hmput_default` second clause: a != NULL but header(raw)->length == 0.
    // Build exactly that with `arrgrowf` and then hand the hash-side pointer in.
    let s = session(0x31415926);
    for elemsize in [16usize, 24, 8] {
        unsafe {
            let ca = (s.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let ra = (s.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            assert_eq!(header(ca).length, 0);
            assert_eq!(header(ra).length, 0);
            let cm = (s.c.hmput_default)((ca as *mut u8).add(elemsize) as *mut c_void, elemsize);
            let rm = (s.r.hmput_default)((ra as *mut u8).add(elemsize) as *mut c_void, elemsize);
            let ctx = format!("C41 e={elemsize}");
            let cs = map_snap(cm, elemsize, KeyKind::Binary);
            let rs = map_snap(rm, elemsize, KeyKind::Binary);
            assert_snap_eq(&ctx, &cs, &rs);
            assert_eq!(cs.length, 1, "{ctx}: length must be 1");
            (s.c.arrfreef)(raw_of(cm, elemsize));
            (s.r.arrfreef)(raw_of(rm, elemsize));
        }
    }
}

// ---------------------------------------------------------------- C42-C45
#[test]
fn c42_to_c45_hmfree_all_table_kinds() {
    let s = session(0x31415926);
    let elemsize = 16usize;

    // C45: binary table (string.mode == 0)
    {
        let mut d = Driver::lazy(&s, elemsize, 8, KeyKind::Binary);
        let mut rng = Rng::new(0x45);
        for i in 0..80usize {
            d.hmput(&format!("C45 put#{i}"), &rng.bytes(8), &rng.bytes(elemsize), HM_BINARY);
        }
        unsafe {
            let t = header(d.c_raw()).hash_table as *mut HashIndex;
            assert_eq!((*t).string.mode, 0, "C45: binary table string.mode must be 0");
        }
        d.check("C45 before free");
        d.free();
    }

    // C42/C43/C44: DEFAULT / STRDUP / ARENA
    for (row, mode) in [("C42", SH_DEFAULT), ("C43", SH_STRDUP), ("C44", SH_ARENA)] {
        for n in [0usize, 1, 7, 80, 400] {
            let mut d = Driver::shmode(&s, elemsize, 8, KeyKind::StringAt(0), mode);
            let mut rng = Rng::new(0x4200 + n as u64 + mode as u64);
            for i in 0..n {
                let k = format!("{row}-{i}-{}", rng.below(1 << 20)).into_bytes();
                d.shput(&format!("{row} put#{i}"), &k, &rng.bytes(elemsize), HM_STRING);
            }
            let ctx = format!("{row} n={n} before free");
            d.check(&ctx);
            unsafe {
                let ct = header(d.c_raw()).hash_table as *mut HashIndex;
                let rt = header(d.r_raw()).hash_table as *mut HashIndex;
                assert_eq!((*ct).string.mode, mode as u8, "{ctx}: C string.mode");
                assert_eq!((*rt).string.mode, mode as u8, "{ctx}: Rust string.mode");
                assert_eq!(
                    (*ct).string.storage.is_null(),
                    (*rt).string.storage.is_null(),
                    "{ctx}: arena chain presence"
                );
                assert_eq!((*ct).string.block, (*rt).string.block, "{ctx}: arena block");
                assert_eq!(
                    (*ct).string.remaining,
                    (*rt).string.remaining,
                    "{ctx}: arena remaining"
                );
            }
            d.free();
        }
    }

    // hmfree on a table-less array (E7 configuration) and on NULL (E6).
    unsafe {
        let ca = (s.c.arrgrowf)(std::ptr::null_mut(), elemsize, 3, 0);
        let ra = (s.r.arrgrowf)(std::ptr::null_mut(), elemsize, 3, 0);
        (s.c.hmfree_func)(ca, elemsize);
        (s.r.hmfree_func)(ra, elemsize);
        (s.c.hmfree_func)(std::ptr::null_mut(), elemsize);
        (s.r.hmfree_func)(std::ptr::null_mut(), elemsize);
    }
}
